use crate::{
    ProgressListener, UploadError, UploadResult, VideoUpload,
    auth::refresh_token::{is_token_expired, now_secs, refresh_access_token},
    auth::urls::{youtube_api_url, youtube_upload_endpoint},
    config::CredentialStore,
    net::{build_http_client_with_timeout, retry},
};
use reqwest::header::{AUTHORIZATION, CONTENT_RANGE, CONTENT_TYPE, LOCATION};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::Mutex;
use tracing::instrument;
use zeroize::Zeroizing;

const YOUTUBE_API_PART: &str = "snippet,status,recordingDetails";
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

/// YouTube resumable uploader.
///
/// Handles the full YouTube Data API v3 resumable upload flow:
/// 1. Authenticate via stored OAuth2 credentials
/// 2. Initiate a resumable upload session
/// 3. Upload video in 8 MiB chunks with automatic retries
/// 4. Return the uploaded video ID and watch URL
///
/// # Examples
///
/// ```no_run
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
/// use video_uploader::{CredentialStore, YouTubeUploader, VideoUpload};
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let store = Arc::new(Mutex::new(CredentialStore::load("my-passphrase")?));
/// let uploader = YouTubeUploader::new(store, "my-passphrase", "youtube");
///
/// let video = VideoUpload::new("/path/to/video.mp4", "My Video");
/// let result = uploader.upload(&video, None).await?;
/// println!("Uploaded: {}", result.url);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct YouTubeUploader {
    client: reqwest::Client,
    credential_store: Arc<Mutex<CredentialStore>>,
    passphrase: Zeroizing<String>,
    workspace: String,
}

impl YouTubeUploader {
    pub fn new(
        credential_store: Arc<Mutex<CredentialStore>>,
        passphrase: impl AsRef<str>,
        workspace: impl Into<String>,
    ) -> Self {
        Self {
            client: build_http_client_with_timeout(60),
            credential_store,
            passphrase: Zeroizing::new(passphrase.as_ref().to_string()),
            workspace: workspace.into(),
        }
    }

    /// Extract resumable upload state from an `Interrupted` error.
    ///
    /// Returns `Some(UploadState)` if the error is `Interrupted`, allowing
    /// the caller to save state and resume later. Returns `None` for other errors.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use video_uploader::{YouTubeUploader, VideoUpload};
    ///
    /// # async fn example(uploader: &YouTubeUploader, video: &VideoUpload) {
    /// match uploader.upload(video, None).await {
    ///     Ok(result) => println!("Uploaded: {}", result.url),
    ///     Err(e) => {
    ///         if let Some(state) = YouTubeUploader::extract_resume_state(&e) {
    ///             println!("Interrupted at {} bytes", state.uploaded_bytes);
    ///             state.save().ok();
    ///         }
    ///     }
    /// }
    /// # }
    /// ```
    pub fn extract_resume_state(error: &UploadError) -> Option<crate::UploadState> {
        match error {
            UploadError::Interrupted { uploaded, total } => Some(crate::UploadState {
                upload_url: String::new(), // URL is not available from the error alone
                uploaded_bytes: *uploaded,
                total_size: *total,
                file_path: PathBuf::new(),
                title: String::new(),
                workspace: String::new(),
            }),
            _ => None,
        }
    }

    /// Resume an upload from saved state.
    ///
    /// Continues uploading from `state.uploaded_bytes`, skipping already-transmitted
    /// chunks. The upload URL must still be valid (Google resumable URLs expire
    /// after ~7 days of inactivity).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use video_uploader::{YouTubeUploader, UploadState, VideoUpload};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let state = UploadState::load_for_file(std::path::Path::new("/path/to/video.mp4"))?.unwrap();
    /// let uploader = YouTubeUploader::new(
    ///     /* store */ unreachable!(), "pass", &state.workspace,
    /// );
    /// let video = VideoUpload::new(state.file_path, &state.title);
    /// let result = uploader.resume(&state, &video, None).await?;
    /// println!("Resumed upload: {}", result.url);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn resume(
        &self,
        state: &crate::UploadState,
        video: &VideoUpload,
        progress: Option<Arc<dyn ProgressListener>>,
    ) -> Result<UploadResult, UploadError> {
        let access_token = self.get_access_token().await?;
        let total_size = video.file_size().await?;

        let json = self
            .upload_with_retry(&state.upload_url, video, &access_token, total_size, progress.clone())
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

        let result = UploadResult::new(
            self.workspace.clone(),
            video_id.to_string(),
            format!("https://www.youtube.com/watch?v={video_id}"),
            video.title.clone(),
        );

        // Clean up saved state on success
        if let Err(e) = state.delete() {
            tracing::warn!("Failed to delete resume state after successful upload: {e}");
        }

        if let Some(p) = progress {
            p.on_complete(&result);
        }

        Ok(result)
    }

    pub async fn delete_video(&self, video_id: &str) -> Result<(), UploadError> {
        self.delete_video_with_url(&youtube_api_url(), video_id)
            .await
    }

    #[cfg(feature = "test-utils")]
    pub async fn delete_video_url(&self, api_url: &str, video_id: &str) -> Result<(), UploadError> {
        self.delete_video_with_url(api_url, video_id).await
    }

    async fn delete_video_with_url(
        &self,
        api_url: &str,
        video_id: &str,
    ) -> Result<(), UploadError> {
        use reqwest::header::AUTHORIZATION;
        let access_token = self.get_access_token().await?;
        let url = format!("{}/videos?id={}", api_url, video_id);
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
                .get(&self.workspace)
                .ok_or_else(|| UploadError::Auth(format!("Workspace '{}' not configured", self.workspace)))?;
            let token_expired = creds.token_expires_at.map(is_token_expired).unwrap_or(true);
            if !token_expired && let Some(ref tok) = creds.access_token {
                return Ok(tok.as_str().to_string());
            }
            (
                creds.access_token.clone(),
                creds.refresh_token.clone(),
                creds.client_id.clone(),
                creds.client_secret.clone(),
            )
        };

        let refresh_tok = refresh_tok
            .ok_or_else(|| UploadError::Auth("No refresh token".into()))?;
        let client_id = client_id
            .ok_or_else(|| UploadError::Auth("No client ID".into()))?;
        let client_secret =
            client_secret.ok_or_else(|| UploadError::Auth("No client secret".into()))?;

        tracing::info!("Refreshing YouTube access token");
        let token = refresh_access_token(&self.client, &refresh_tok, &client_id, &client_secret).await?;
        let access_token = token.access_token.clone();

        {
            let mut store = self.credential_store.lock().await;
            if let Some(creds) = store.get_mut(&self.workspace) {
                creds.access_token = Some(Zeroizing::new(token.access_token));
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

    /// Fetch the authenticated user's YouTube channel info via `channels.list?mine=true`.
    ///
    /// Returns `(channel_id, channel_title)` if found.
    /// Requires a valid access token (will refresh if needed).
    pub async fn fetch_channel_info(&self) -> Result<(String, String), UploadError> {
        let access_token = self.get_access_token().await?;

        let url = format!(
            "https://www.googleapis.com/youtube/v3/channels?mine=true&part=snippet"
        );

        let response = self.client
            .get(&url)
            .bearer_auth(&access_token)
            .send()
            .await
            .map_err(UploadError::Http)?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(UploadError::PlatformApi {
                status,
                message: format!("channels.list: {body}"),
            });
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(UploadError::Http)?;

        let items = body["items"].as_array().ok_or_else(|| {
            UploadError::Auth("channels.list returned no items — no YouTube channel found for this account".to_string())
        })?;

        if items.is_empty() {
            return Err(UploadError::Auth(
                "No YouTube channel found for this account".to_string(),
            ));
        }

        let channel = &items[0];
        let channel_id = channel["id"].as_str().unwrap_or("").to_string();
        let channel_title = channel["snippet"]["title"].as_str().unwrap_or("(unknown)").to_string();

        Ok((channel_id, channel_title))
    }

    /// Initiate a resumable upload session using the default YouTube upload endpoint.
    pub async fn initiate_resumable(
        &self,
        video: &VideoUpload,
        access_token: &str,
        total_size: u64,
    ) -> Result<String, UploadError> {
        self.initiate_resumable_inner(video, access_token, total_size).await
    }

    #[cfg(feature = "test-utils")]
    pub async fn initiate_resumable_url(
        &self,
        upload_url: &str,
        video: &VideoUpload,
        access_token: &str,
    ) -> Result<String, UploadError> {
        let total_size = video.file_size().await?;
        self.initiate_resumable_with_url(upload_url, video, access_token, total_size)
            .await
    }

    #[cfg(feature = "test-utils")]
    pub async fn initiate_resumable_url_with_retry(
        &self,
        upload_url: &str,
        video: &VideoUpload,
        access_token: &str,
    ) -> Result<String, UploadError> {
        let total_size = video.file_size().await?;
        self.initiate_resumable_with_url_with_retry(upload_url, video, access_token, total_size)
            .await
    }

    async fn initiate_resumable_inner(
        &self,
        video: &VideoUpload,
        access_token: &str,
        total_size: u64,
    ) -> Result<String, UploadError> {
        self.initiate_resumable_with_url(&youtube_upload_endpoint(), video, access_token, total_size)
            .await
    }

    async fn initiate_resumable_with_url(
        &self,
        upload_url: &str,
        video: &VideoUpload,
        access_token: &str,
        total_size: u64,
    ) -> Result<String, UploadError> {
        let category_id = video.category_id.clone().unwrap_or_else(|| {
            tracing::warn!(
                "No category specified for YouTube upload, defaulting to People & Blogs (22)"
            );
            "22".to_string()
        });

        let mut status = json!({ "privacyStatus": video.visibility.to_string() });
        if let Some(kids) = video.made_for_kids {
            status["selfDeclaredMadeForKids"] = json!(kids);
        }
        if let Some(license) = &video.license {
            status["license"] = json!(license.to_string());
        }
        if let Some(embeddable) = video.embeddable {
            status["embeddable"] = json!(embeddable);
        }
        if let Some(pub_stats) = video.public_stats_viewable {
            status["publicStatsViewable"] = json!(pub_stats);
        }
        if let Some(synthetic) = video.contains_synthetic_media {
            status["containsSyntheticMedia"] = json!(synthetic);
        }
        if let Some(ref publish_at) = video.publish_at {
            status["publishAt"] = json!(publish_at);
        }

        let mut snippet = json!({
            "title": video.title,
            "description": video.description.as_deref().unwrap_or(""),
            "tags": video.tags,
            "categoryId": category_id,
        });
        if let Some(ref lang) = video.language {
            snippet["defaultLanguage"] = json!(lang);
        }

        // Append description suffix if set
        if let Some(ref suffix) = video.description_suffix {
            let desc = snippet["description"].as_str().unwrap_or("");
            snippet["description"] = json!(format!("{desc}{suffix}"));
        }

        // Recording details (separate API object)
        let mut recording_details = json!({});
        if let Some(ref date) = video.recording_date {
            recording_details["recordingDate"] = json!(date);
        }

        let mut metadata = json!({
            "snippet": snippet,
            "status": status
        });
        if !recording_details.as_object().is_none_or(|o| o.is_empty()) {
            metadata["recordingDetails"] = recording_details;
        }

        let response = self
            .client
            .post(upload_url)
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

    #[cfg(feature = "test-utils")]
    pub fn validate_upload_url_for_testing(url: &str) -> Result<String, UploadError> {
        validate_upload_url(url)
    }

    #[cfg(feature = "test-utils")]
    async fn initiate_resumable_with_url_with_retry(
        &self,
        upload_url: &str,
        video: &VideoUpload,
        access_token: &str,
        total_size: u64,
    ) -> Result<String, UploadError> {
        retry(
            || self.initiate_resumable_with_url(upload_url, video, access_token, total_size),
            MAX_RETRIES,
        )
        .await
    }

    async fn initiate_resumable_with_retry(
        &self,
        video: &VideoUpload,
        access_token: &str,
        total_size: u64,
    ) -> Result<String, UploadError> {
        self.initiate_resumable_inner(video, access_token, total_size).await
    }

    #[instrument(skip(self, video, progress), fields(workspace = %self.workspace, title = %video.title))]
    pub async fn upload_chunks(
        &self,
        upload_url: &str,
        video: &VideoUpload,
        access_token: &str,
        total_size: u64,
        progress: Option<Arc<dyn ProgressListener>>,
    ) -> Result<serde_json::Value, UploadError> {
        let mut file = File::open(&video.file_path).await?;
        let mut uploaded: u64 = 0;
        let mut chunk_buf = vec![0u8; CHUNK_SIZE];
        let mime = self.mime_type(video);

        while uploaded < total_size {
            // Read a full chunk, handling partial reads from the OS.
            // `read()` may return fewer bytes than the buffer size even when
            // more data is available, so we loop until the buffer is full or EOF.
            let mut bytes_read = 0usize;
            while bytes_read < CHUNK_SIZE && (uploaded + bytes_read as u64) < total_size {
                let n = file.read(&mut chunk_buf[bytes_read..]).await?;
                if n == 0 {
                    break; // EOF
                }
                bytes_read += n;
            }
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
        total_size: u64,
        progress: Option<Arc<dyn ProgressListener>>,
    ) -> Result<serde_json::Value, UploadError> {
        retry(
            || self.upload_chunks(upload_url, video, access_token, total_size, progress.clone()),
            MAX_RETRIES,
        )
        .await
    }

    #[instrument(skip(self, video, progress), fields(workspace = %self.workspace, title = %video.title))]
    pub async fn upload(
        &self,
        video: &VideoUpload,
        progress: Option<Arc<dyn ProgressListener>>,
    ) -> Result<UploadResult, UploadError> {
        let access_token = self.get_access_token().await?;
        let total_size = video.file_size().await?;
        let upload_url = self
            .initiate_resumable_with_retry(video, &access_token, total_size)
            .await?;

        let json = self
            .upload_with_retry(&upload_url, video, &access_token, total_size, progress.clone())
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

        let result = UploadResult::new(
            self.workspace.clone(),
            video_id.to_string(),
            format!("https://www.youtube.com/watch?v={video_id}"),
            video.title.clone(),
        );

        if let Some(p) = progress {
            p.on_complete(&result);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
