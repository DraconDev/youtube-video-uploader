//! Odysee/LBRY uploader using the local lbrynet daemon JSON-RPC API.
//!
//! Requires `lbrynet` daemon running at localhost:5279 (or configured daemon_url).
//! See <https://lbry.tech/api/sdk> for API documentation.
//!
//! SECURITY: The daemon is expected to run on localhost with no authentication.
//! Only connect to daemons you trust and that have local file system access.

use crate::{
    PlatformUploader, ProgressListener, UploadError, UploadResult, VideoUpload,
    auth::urls::odysee_default_daemon_url,
    is_private_ip,
    net::{self, retry},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const DEFAULT_TIMEOUT_SECS: u64 = 600; // 10 minutes for large video uploads
const MAX_RETRIES: u32 = 3;

pub fn validate_daemon_url(url_str: &str) -> Result<String, UploadError> {
    let parsed = url::Url::parse(url_str)
        .map_err(|e| UploadError::Config(format!("Invalid Odysee daemon URL: {e}")))?;
    let scheme = parsed.scheme();
    let host = parsed
        .host_str()
        .ok_or_else(|| UploadError::Config("Odysee daemon URL has no host".into()))?;

    if scheme != "http" {
        return Err(UploadError::Config(format!(
            "Odysee daemon URL scheme must be http, got: {}",
            scheme
        )));
    }
    if !is_private_ip(host) {
        if std::env::var("ODYSEE_ALLOW_REMOTE_DAEMON").as_deref() == Ok("1") {
            return Ok(url_str.to_string());
        }
        return Err(UploadError::Config(
            "Odysee daemon URL must be localhost or private IP. Set ODYSEE_ALLOW_REMOTE_DAEMON=1 to override (not recommended for production).".into(),
        ));
    }

    Ok(url_str.to_string())
}

/// Response from lbrynet publish command.
#[derive(Debug, Deserialize)]
struct PublishResponse {
    #[serde(rename = "txid")]
    txid: Option<String>,
    #[serde(rename = "claim")]
    claim: Option<ClaimInfo>,
}

/// Simplified claim info from publish response.
#[derive(Debug, Deserialize)]
struct ClaimInfo {
    #[serde(rename = "claimId")]
    claim_id: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    name: Option<String>,
    #[serde(rename = "permanentUrl")]
    permanent_url: Option<String>,
}

/// Generic JSON-RPC response wrapper.
#[derive(Deserialize)]
struct JsonRpcResponse<T> {
    #[serde(rename = "result")]
    result: Option<T>,
    #[serde(rename = "error")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    #[serde(rename = "code")]
    code: i32,
    #[serde(rename = "message")]
    message: String,
}

/// Request for JSON-RPC calls.
#[derive(Serialize)]
struct JsonRpcRequest {
    #[serde(rename = "jsonrpc")]
    jsonrpc: &'static str,
    #[serde(rename = "id")]
    id: i32,
    #[serde(rename = "method")]
    method: String,
    #[serde(rename = "params")]
    params: serde_json::Value,
}

/// Convert visibility to LBRY privacy setting.
fn privacy_value(visibility: crate::upload::Visibility) -> &'static str {
    match visibility {
        crate::upload::Visibility::Public => "public",
        crate::upload::Visibility::Unlisted => "unlisted",
        crate::upload::Visibility::Private => "private",
    }
}

/// Generate a claim name from video title.
pub fn generate_claim_name(title: &str) -> String {
    // LBRY claim names must be lowercase, alphanumeric + hyphens, 1-63 chars
    let sanitized: String = title
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .take(63)
        .collect();
    sanitized.trim_matches('-').to_lowercase()
}

#[derive(Clone)]
pub struct OdyseeUploader {
    client: reqwest::Client,
    daemon_url: String,
    channel_name: Option<String>,
}

impl OdyseeUploader {
    /// Create a new Odysee uploader.
    ///
    /// # Arguments
    /// * `daemon_url` - LBRY daemon URL (e.g., "http://localhost:5279")
    /// * `channel_name` - Optional channel name for uploads (e.g., "@mychannel")
    pub fn new(
        daemon_url: impl Into<String>,
        channel_name: Option<String>,
    ) -> Result<Self, UploadError> {
        let daemon_url = daemon_url.into();
        let validated = validate_daemon_url(&daemon_url)?;
        Ok(Self {
            daemon_url: validated,
            channel_name,
            client: net::build_http_client_with_timeout(DEFAULT_TIMEOUT_SECS),
        })
    }

    /// Create with default daemon URL (localhost:5279).
    pub fn with_default_daemon() -> Result<Self, UploadError> {
        Self::new(odysee_default_daemon_url().as_str(), None)
    }

    pub async fn abandon_claim(&self, claim_id: &str) -> Result<(), UploadError> {
        let params = serde_json::json!({ "claim_id": claim_id });
        let _: serde_json::Value = self.make_request("claim_abandon", params).await?;
        Ok(())
    }

    async fn make_request<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T, UploadError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: method.to_string(),
            params,
        };

        let response = self
            .client
            .post(&self.daemon_url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        let rpc_response: JsonRpcResponse<T> = serde_json::from_str(&body)
            .map_err(|e| UploadError::Other(format!("Failed to parse JSON-RPC response: {}", e)))?;

        if let Some(err) = rpc_response.error {
            return Err(UploadError::PlatformApi {
                status: status.as_u16(),
                message: format!("Odysee RPC error {}: {}", err.code, err.message),
            });
        }

        rpc_response.result.ok_or_else(|| UploadError::PlatformApi {
            status: status.as_u16(),
            message: "No result in JSON-RPC response".to_string(),
        })
    }

    fn visibility_to_bid(visibility: crate::upload::Visibility) -> &'static str {
        match visibility {
            crate::upload::Visibility::Public => "0.01",
            crate::upload::Visibility::Unlisted => "0.001",
            crate::upload::Visibility::Private => "0.0001",
        }
    }

    async fn check_daemon(&self) -> Result<(), UploadError> {
        let _: serde_json::Value = self.make_request("status", serde_json::json!({})).await?;
        Ok(())
    }

    async fn upload_with_retry(
        &self,
        video: &VideoUpload,
        _progress: Option<Arc<dyn ProgressListener>>,
    ) -> Result<UploadResult, UploadError> {
        retry(|| async { self.upload_one(video).await }, MAX_RETRIES).await
    }

    async fn upload_one(&self, video: &VideoUpload) -> Result<UploadResult, UploadError> {
        let total_size = video.file_size().await.map_err(UploadError::Io)?;
        let file_path = &video.file_path;

        tracing::info!(
            target: "odysee",
            "Uploading {} ({} bytes) to Odysee",
            video.title,
            total_size
        );

        let claim_name = generate_claim_name(&video.title);
        if claim_name.is_empty() {
            return Err(UploadError::Config(
                "Video title produces an empty claim name after sanitization. Please use a title with at least one alphanumeric character.".into(),
            ));
        }
        let bid = Self::visibility_to_bid(video.visibility);

        let mut params = serde_json::json!({
            "name": claim_name,
            "bid": bid,
            "file_path": file_path.to_string_lossy().to_string(),
            "title": video.title,
            "tags": video.tags,
            "privacy": privacy_value(video.visibility),
        });

        if let Some(desc) = &video.description {
            params["description"] = serde_json::json!(desc);
        }

        if let Some(ref channel) = self.channel_name {
            params["channel_name"] = serde_json::json!(channel);
        }

        let publish_response: PublishResponse = self.make_request("publish", params).await?;

        let (claim_id, permanent_url) = if let Some(claim) = publish_response.claim {
            (
                claim.claim_id.unwrap_or_else(|| "unknown".to_string()),
                claim.permanent_url.unwrap_or_else(|| "unknown".to_string()),
            )
        } else {
            return Err(UploadError::PlatformApi {
                status: 500,
                message: format!(
                    "Publish succeeded but no claim returned. TXID: {:?}",
                    publish_response.txid
                ),
            });
        };

        Ok(UploadResult {
            platform: "odysee",
            platform_id: claim_id.clone(),
            url: permanent_url.clone(),
            title: video.title.clone(),
        })
    }
}

#[async_trait]
impl PlatformUploader for OdyseeUploader {
    fn platform_name(&self) -> &'static str {
        "odysee"
    }

    async fn upload(
        &self,
        video: &VideoUpload,
        progress: Option<Arc<dyn ProgressListener>>,
    ) -> Result<UploadResult, UploadError> {
        if let Err(e) = self.check_daemon().await {
            let err = UploadError::PlatformApi {
                status: 503,
                message: format!(
                    "Odysee daemon not available at {}. Make sure lbrynet is running. Error: {}",
                    self.daemon_url, e
                ),
            };
            if let Some(p) = &progress {
                p.on_error(&err);
            }
            return Err(err);
        }

        let total_size = video.file_size().await.map_err(UploadError::Io)?;

        if let Some(p) = &progress {
            p.on_progress(0, total_size);
        }

        let result = self.upload_with_retry(video, progress.clone()).await.inspect_err(|e| {
            if let Some(p) = &progress {
                p.on_error(e);
            }
        })?;

        if let Some(p) = progress {
            p.on_complete(&result);
        }

        tracing::info!(target: "odysee", "Uploaded {} to {}", result.platform_id, result.url);

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_name() {
        let u = OdyseeUploader::with_default_daemon().unwrap();
        assert_eq!(u.platform_name(), "odysee");
    }

    #[test]
    fn test_generate_claim_name() {
        assert_eq!(generate_claim_name("My Video Title!"), "myvideotitle");
        assert_eq!(generate_claim_name("Test 123"), "test123");
        assert_eq!(
            generate_claim_name("Video with   spaces"),
            "videowithspaces"
        );
        assert_eq!(
            generate_claim_name("Special!@#$%Characters"),
            "specialcharacters"
        );
    }

    #[test]
    fn test_generate_claim_name_lowercase() {
        let name = generate_claim_name("UPPERCASE TITLE");
        assert_eq!(name, "uppercasetitle");
    }

    #[test]
    fn test_generate_claim_name_empty() {
        assert_eq!(generate_claim_name("!@#$"), "");
        assert_eq!(generate_claim_name("-_-"), "");
    }

    #[test]
    fn test_visibility_to_bid() {
        assert_eq!(
            OdyseeUploader::visibility_to_bid(crate::upload::Visibility::Public),
            "0.01"
        );
        assert_eq!(
            OdyseeUploader::visibility_to_bid(crate::upload::Visibility::Unlisted),
            "0.001"
        );
        assert_eq!(
            OdyseeUploader::visibility_to_bid(crate::upload::Visibility::Private),
            "0.0001"
        );
    }

    #[test]
    fn test_validate_daemon_url() {
        assert!(validate_daemon_url("http://localhost").is_ok());
        assert!(validate_daemon_url("http://localhost:5279").is_ok());
        assert!(validate_daemon_url("http://127.0.0.1").is_ok());
        assert!(validate_daemon_url("http://127.0.0.1:5279").is_ok());
        assert!(validate_daemon_url("http://192.168.1.1").is_ok());
        assert!(validate_daemon_url("http://10.0.0.1").is_ok());
        assert!(validate_daemon_url("http://172.16.0.1").is_ok());
        assert!(validate_daemon_url("https://localhost").is_err());
        assert!(validate_daemon_url("https://127.0.0.1").is_err());
        assert!(validate_daemon_url("http://public.example.com").is_err());
        assert!(validate_daemon_url("http://framatube.org").is_err());
        assert!(validate_daemon_url("ftp://localhost").is_err());
        assert!(validate_daemon_url("").is_err());
    }
}
