use crate::{
    PlatformUploader, ProgressListener, UploadError, UploadResult, VideoUpload,
    auth::refresh_token::{is_token_expired, now_secs, refresh_access_token},
    auth::urls::youtube_upload_endpoint,
    config::CredentialStore,
    net::{build_http_client_with_timeout, retry},
};
use async_trait::async_trait;
use reqwest::header::{AUTHORIZATION, CONTENT_RANGE, CONTENT_TYPE, LOCATION};
use serde_json::json;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::Mutex;
use zeroize::Zeroizing;

const YOUTUBE_API_PART: &str = "snippet,status";
const CHUNK_SIZE: usize = 8 * 1024 * 1024; // 8 MiB
const MAX_RETRIES: u32 = 3;

const GOOGLE_APIS_HOST: &str = "googleapis.com";

pub(crate) fn validate_upload_url(url: &str) -> Result<String, UploadError> {
    let parsed = url::Url::parse(url).map_err(|e| UploadError::PlatformApi {
        status: 500,
        message: format!("Invalid upload URL: {e}"),
    })?;
    if parsed.scheme() != "https" {
        return Err(UploadError::PlatformApi {
            status: 500,
            message: format!("Upload URL scheme must be https, got: {}", parsed.scheme()),
        });
    }
    let host = parsed.host_str().ok_or_else(|| UploadError::PlatformApi {
        status: 500,
        message: "Upload URL has no host".into(),
    })?;
    if host != GOOGLE_APIS_HOST && !host.ends_with(&format!(".{GOOGLE_APIS_HOST}")) {
        return Err(UploadError::PlatformApi {
            status: 500,
            message: format!(
                "Upload URL host must end with {}, got: {}",
                GOOGLE_APIS_HOST, host
            ),
        });
    }
    Ok(url.to_string())
}

#[derive(Clone)]
pub struct YouTubeUploader {
    client: reqwest::Client,
    credential_store: Arc<Mutex<CredentialStore>>,
    passphrase: Zeroizing<String>,
}

impl YouTubeUploader {
    pub fn new(
        credential_store: Arc<Mutex<CredentialStore>>,
        passphrase: impl AsRef<str>,
    ) -> Self {
        Self {
            client: build_http_client_with_timeout(60),
            credential_store,
            passphrase: Zeroizing::new(passphrase.as_ref().to_string()),
        }
    }

    pub async fn delete_video(&self, video_id: &str) -> Result<(), UploadError> {
        use reqwest::header::AUTHORIZATION;
        let access_token = self.get_access_token().await?;
        let url = format!("{}?id={}", crate::auth::urls::youtube_api_url(), video_id);
        let resp = self
            .client
            .delete(&url)
            .header(AUTHORIZATION, format!("Bearer {}", access_token))
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(UploadError::PlatformApi {
                status: status.as_u16(),
                message: format!("Failed to delete YouTube video ({}): {}", status, body),
            });
        }
        Ok(())
    }

    async fn get_access_token(&self) -> Result<String, UploadError> {
        let (_access_token, refresh_tok, client_id, client_secret) = {
            let store = self.credential_store.lock().await;
            let creds = store
                .get("youtube")
                .ok_or_else(|| UploadError::Auth("YouTube not configured".into()))?;
            let token_expired = creds.token_expires_at.map(is_token_expired).unwrap_or(true);
            if !token_expired && let Some(ref tok) = creds.access_token {
                return Ok(tok.clone());
            }
            (
                creds.access_token.clone(),
                creds.refresh_token.clone(),
                creds.client_id.clone(),
                creds.client_secret.clone(),
            )
        };

        let refresh_tok =
            refresh_tok.ok_or_else(|| UploadError::Auth("No refresh token".into()))?;
        let client_id = client_id.ok_or_else(|| UploadError::Auth("No client ID".into()))?;
        let client_secret =
            client_secret.ok_or_else(|| UploadError::Auth("No client secret".into()))?;

        tracing::info!("Refreshing YouTube access token");
        let token = refresh_access_token(&refresh_tok, &client_id, &client_secret).await?;
        let access_token = token.access_token.clone();

        {
            let mut store = self.credential_store.lock().await;
            if let Some(creds) = store.get_mut("youtube") {
                creds.access_token = Some(token.access_token);
                creds.token_expires_at = Some(now_secs() + token.expires_in);
                if let Err(e) = store.save(&self.passphrase) {
                    tracing::error!("Failed to persist refreshed token: {e}");
                }
            }
        }
        Ok(access_token)
    }

    fn mime_type(&self, video: &VideoUpload) -> String {
        mime_guess::from_path(&video.file_path)
            .first_or_octet_stream()
            .to_string()
    }

    async fn initiate_resumable(
        &self,
        video: &VideoUpload,
        access_token: &str,
    ) -> Result<String, UploadError> {
        let total_size = video.file_size().await?;

        let category_id = video.category_id.clone().unwrap_or_else(|| {
            tracing::warn!(
                "No category specified for YouTube upload, defaulting to People & Blogs (22)"
            );
            "22".to_string()
        });

        let metadata = json!({
            "snippet": {
                "title": video.title,
                "description": video.description.as_deref().unwrap_or(""),
                "tags": video.tags,
                "categoryId": category_id,
            },
            "status": { "privacyStatus": video.visibility.to_string() }
        });

        let response = self
            .client
            .post(format!("{}/videos", youtube_upload_endpoint()))
            .query(&[("uploadType", "resumable"), ("part", YOUTUBE_API_PART)])
            .header(AUTHORIZATION, format!("Bearer {access_token}"))
            .header(CONTENT_TYPE, "application/json; charset=UTF-8")
            .header("X-Upload-Content-Type", self.mime_type(video))
            .header("X-Upload-Content-Length", total_size.to_string())
            .body(metadata.to_string())
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(UploadError::PlatformApi {
                status: response.status().as_u16(),
                message: format!(
                    "Failed to initiate upload: {}",
                    response.text().await.unwrap_or_default()
                ),
            });
        }

        let location = response
            .headers()
            .get(LOCATION)
            .ok_or_else(|| UploadError::PlatformApi {
                status: 500,
                message: "No Location header in resumable upload response".into(),
            })?
            .to_str()
            .map_err(|e| UploadError::PlatformApi {
                status: 500,
                message: format!("Invalid Location header: {e}"),
            })?
            .to_string();

        validate_upload_url(&location)?;

        Ok(location)
    }

    async fn initiate_resumable_with_retry(
        &self,
        video: &VideoUpload,
        access_token: &str,
    ) -> Result<String, UploadError> {
        retry(|| self.initiate_resumable(video, access_token), MAX_RETRIES).await
    }

    pub async fn upload_chunks(
        &self,
        upload_url: &str,
        video: &VideoUpload,
        access_token: &str,
        progress: Option<Arc<dyn ProgressListener>>,
    ) -> Result<serde_json::Value, UploadError> {
        let total_size = video.file_size().await?;
        let mut file = File::open(&video.file_path).await?;
        let mut uploaded: u64 = 0;
        let mut chunk_buf = vec![0u8; CHUNK_SIZE];
        let mime = self.mime_type(video);

        while uploaded < total_size {
            let bytes_read = file.read(&mut chunk_buf).await?;
            if bytes_read == 0 {
                break;
            }

            let end = uploaded + bytes_read as u64 - 1;
            let chunk = &chunk_buf[..bytes_read];

            let response = self
                .client
                .put(upload_url)
                .header(AUTHORIZATION, format!("Bearer {access_token}"))
                .header(CONTENT_TYPE, &mime)
                .header(
                    CONTENT_RANGE,
                    format!("bytes {}-{}/{}", uploaded, end, total_size),
                )
                .body(chunk.to_vec())
                .send()
                .await
                .map_err(|e| {
                    if e.is_timeout() || e.is_connect() {
                        if let Some(p) = &progress {
                            p.on_progress(uploaded, total_size);
                        }
                        UploadError::Interrupted {
                            uploaded,
                            total: total_size,
                        }
                    } else {
                        UploadError::Http(e)
                    }
                })?;

            let status = response.status();

            if status.as_u16() == 308 {
                if let Some(range) = response.headers().get("range") {
                    let range_str = range.to_str().unwrap_or("");
                    if let Some(pos) = range_str.rfind('-') {
                        if let Ok(next_byte) = range_str[pos + 1..].parse::<u64>() {
                            uploaded = next_byte + 1;
                            file.seek(std::io::SeekFrom::Start(uploaded)).await?;
                        } else {
                            uploaded = end + 1;
                        }
                    } else {
                        uploaded = end + 1;
                    }
                } else {
                    uploaded = end + 1;
                }
                continue;
            }

            if status.is_success() {
                let body = response.text().await?;
                if let Some(p) = progress {
                    p.on_progress(uploaded.min(total_size), total_size);
                }
                return serde_json::from_str(&body).map_err(|e| UploadError::PlatformApi {
                    status: status.as_u16(),
                    message: format!("Failed to parse response: {e}"),
                });
            }

            return Err(UploadError::PlatformApi {
                status: status.as_u16(),
                message: format!(
                    "YouTube chunk upload failed: {}",
                    response.text().await.unwrap_or_default()
                ),
            });
        }

        Err(UploadError::PlatformApi {
            status: 500,
            message: "Upload completed without success response".into(),
        })
    }

    pub async fn upload_with_retry(
        &self,
        upload_url: &str,
        video: &VideoUpload,
        access_token: &str,
        progress: Option<Arc<dyn ProgressListener>>,
    ) -> Result<serde_json::Value, UploadError> {
        retry(
            || self.upload_chunks(upload_url, video, access_token, progress.clone()),
            MAX_RETRIES,
        )
        .await
    }
}

#[async_trait]
impl PlatformUploader for YouTubeUploader {
    fn platform_name(&self) -> &'static str {
        "youtube"
    }

    fn supports_resumable(&self) -> bool {
        true
    }

    async fn upload(
        &self,
        video: &VideoUpload,
        progress: Option<Arc<dyn ProgressListener>>,
    ) -> Result<UploadResult, UploadError> {
        let access_token = self.get_access_token().await?;
        let upload_url = self
            .initiate_resumable_with_retry(video, &access_token)
            .await?;

        let json = self
            .upload_with_retry(&upload_url, video, &access_token, progress.clone())
            .await
            .inspect_err(|e| {
                if let Some(p) = &progress {
                    p.on_error(e);
                }
            })?;

        let video_id = json["id"]
            .as_str()
            .ok_or_else(|| UploadError::PlatformApi {
                status: 500,
                message: "No video ID in upload response".into(),
            })?;

        let result = UploadResult {
            platform: "youtube",
            platform_id: video_id.to_string(),
            url: format!("https://www.youtube.com/watch?v={video_id}"),
            title: video.title.clone(),
        };

        if let Some(p) = progress {
            p.on_complete(&result);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CredentialStore;

    #[test]
    fn test_youtube_platform_name() {
        let store = Arc::new(Mutex::new(CredentialStore::default()));
        let yt = YouTubeUploader::new(store, "test");
        assert_eq!(yt.platform_name(), "youtube");
    }

    #[test]
    fn test_youtube_supports_resumable() {
        let store = Arc::new(Mutex::new(CredentialStore::default()));
        let yt = YouTubeUploader::new(store, "test");
        assert!(yt.supports_resumable());
    }

    #[test]
    fn test_validate_upload_url() {
        assert!(
            validate_upload_url("https://storage.googleapis.com/upload/mybucket/video.mp4").is_ok()
        );
        assert!(validate_upload_url("https://googleapis.com/storage/v1/upload").is_ok());
        assert!(
            validate_upload_url("http://storage.googleapis.com/upload/mybucket/video.mp4").is_err()
        );
        assert!(validate_upload_url("https://evil.com/googleapis.com/upload").is_err());
        assert!(validate_upload_url("https://notgoogle.com/upload").is_err());
        assert!(validate_upload_url("https://").is_err());
        assert!(validate_upload_url("ftp://googleapis.com/upload").is_err());
        assert!(validate_upload_url("").is_err());
    }

    #[test]
    fn test_validate_upload_url_rejects_evil_googleapis() {
        assert!(
            validate_upload_url("https://evilgoogleapis.com/upload.googleapis.com/upload").is_err()
        );
        assert!(validate_upload_url("https://evilgoogleapis.com/upload").is_err());
    }

    #[test]
    fn test_validate_upload_url_accepts_subdomain() {
        assert!(
            validate_upload_url("https://foo.googleapis.com/upload.googleapis.com/upload").is_ok()
        );
        assert!(
            validate_upload_url("https://storage.googleapis.com/upload/youtube/v3/videos").is_ok()
        );
    }

    #[test]
    fn test_validate_upload_url_accepts_apex() {
        assert!(validate_upload_url("https://googleapis.com/upload/youtube/v3/videos").is_ok());
    }
}
