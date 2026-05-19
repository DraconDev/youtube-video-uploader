use crate::UploadError;
use crate::auth::TokenResponse;
use crate::auth::urls::{google_device_code_url, google_token_url, youtube_upload_scope};
use crate::net::build_http_client;
use serde::Deserialize;
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_url: String,
    #[serde(default)]
    pub expires_in: u64,
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_interval() -> u64 {
    5
}

#[derive(Debug, Deserialize)]
struct TokenErrorResponse {
    error: String,
    #[serde(default)]
    error_description: String,
}

async fn start_device_code_with_url(
    device_code_url: &str,
    client_id: &str,
) -> Result<DeviceCodeResponse, UploadError> {
    let client = build_http_client();
    let params = [("client_id", client_id), ("scope", &youtube_upload_scope())];

    let response = client.post(device_code_url).form(&params).send().await?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(UploadError::Auth(format!(
            "Failed to request device code: {text}"
        )));
    }

    let resp: DeviceCodeResponse = response.json().await?;
    Ok(resp)
}

pub async fn start_device_code(client_id: &str) -> Result<DeviceCodeResponse, UploadError> {
    start_device_code_with_url(google_device_code_url().as_str(), client_id).await
}

#[cfg(feature = "test-utils")]
pub async fn start_device_code_url(
    device_code_url: &str,
    client_id: &str,
) -> Result<DeviceCodeResponse, UploadError> {
    start_device_code_with_url(device_code_url, client_id).await
}

#[cfg(feature = "test-utils")]
pub async fn poll_for_token_url(
    token_url: &str,
    device_code: &str,
    client_id: &str,
    client_secret: &str,
    expires_in_secs: u64,
    poll_interval_secs: u64,
) -> Result<TokenResponse, UploadError> {
    poll_for_token_with_url(
        token_url,
        device_code,
        client_id,
        client_secret,
        expires_in_secs,
        poll_interval_secs,
    )
    .await
}

pub async fn poll_for_token(
    device_code: &str,
    client_id: &str,
    client_secret: &str,
    expires_in_secs: u64,
    poll_interval_secs: u64,
) -> Result<TokenResponse, UploadError> {
    poll_for_token_with_url(
        google_token_url().as_str(),
        device_code,
        client_id,
        client_secret,
        expires_in_secs,
        poll_interval_secs,
    )
    .await
}

pub async fn poll_for_token_with_url(
    token_url: &str,
    device_code: &str,
    client_id: &str,
    client_secret: &str,
    expires_in_secs: u64,
    poll_interval_secs: u64,
) -> Result<TokenResponse, UploadError> {
    let client = build_http_client();
    let start = Instant::now();
    let expires_in = Duration::from_secs(expires_in_secs.max(60));
    let mut interval = Duration::from_secs(poll_interval_secs.max(5));

    loop {
        if start.elapsed() > expires_in {
            return Err(UploadError::Auth(
                "Device code expired before authorization".into(),
            ));
        }

        tokio::time::sleep(interval).await;

        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ];

        let response = client.post(token_url).form(&params).send().await?;

        let status = response.status();
        let body = response.text().await?;

        tracing::debug!(
            "Token poll response: status={}, body={}",
            status,
            &body[..body.len().min(200)]
        );

        if status.is_success() {
            let token: TokenResponse = serde_json::from_str(&body)
                .map_err(|e| UploadError::Auth(format!("Failed to parse token response: {e}")))?;
            return Ok(token);
        }

        let err: TokenErrorResponse = serde_json::from_str(&body).unwrap_or(TokenErrorResponse {
            error: "unknown".into(),
            error_description: body.clone(),
        });

        match err.error.as_str() {
            "authorization_pending" => continue,
            "slow_down" => {
                interval += Duration::from_secs(5);
                continue;
            }
            "expired_token" => {
                return Err(UploadError::Auth("Device code expired".into()));
            }
            // Google sometimes returns transient errors; retry a few times
            "internal_failure" | "server_error" => {
                tracing::warn!(
                    "Transient token error: {} - {}, retrying...",
                    err.error,
                    err.error_description
                );
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
            _ => {
                return Err(UploadError::Auth(format!(
                    "Token polling failed: {} - {}",
                    err.error, err.error_description
                )));
            }
        }
    }
}

pub async fn run_device_code_flow(
    client_id: &str,
    client_secret: &str,
    print_instructions: impl Fn(&DeviceCodeResponse),
) -> Result<TokenResponse, UploadError> {
    let device = start_device_code(client_id).await?;
    print_instructions(&device);
    let expires_in = if device.expires_in > 0 {
        device.expires_in
    } else {
        600
    };
    let poll_interval = if device.interval >= 5 {
        device.interval
    } else {
        5
    };
    poll_for_token(
        &device.device_code,
        client_id,
        client_secret,
        expires_in,
        poll_interval,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_code_response_deserialization() {
        let json = r#"{
            "device_code": "_device_123",
            "user_code": "ABCD-EFGH",
            "verification_url": "https://www.google.com/device",
            "expires_in": 1800,
            "interval": 5
        }"#;

        let resp: DeviceCodeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.device_code, "_device_123");
        assert_eq!(resp.user_code, "ABCD-EFGH");
        assert_eq!(resp.verification_url, "https://www.google.com/device");
        assert_eq!(resp.expires_in, 1800);
        assert_eq!(resp.interval, 5);
    }

    #[test]
    fn test_device_code_response_defaults() {
        let json = r#"{
            "device_code": "device_456",
            "user_code": "IJkl-MNOP",
            "verification_url": "https://google.com/device"
        }"#;

        let resp: DeviceCodeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.device_code, "device_456");
        assert_eq!(resp.expires_in, 0); // default
        assert_eq!(resp.interval, 5); // default
    }

    #[test]
    fn test_token_response_deserialization() {
        let json = r#"{
            "access_token": "ya29.access_token_value",
            "refresh_token": "1//refresh_token_value",
            "expires_in": 3600,
            "token_type": "Bearer"
        }"#;

        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.access_token, "ya29.access_token_value");
        assert_eq!(
            resp.refresh_token,
            Some("1//refresh_token_value".to_string())
        );
        assert_eq!(resp.expires_in, 3600);
        assert_eq!(resp.token_type, "Bearer");
    }

    #[test]
    fn test_token_response_without_refresh_token() {
        let json = r#"{
            "access_token": "ya29.access_token",
            "expires_in": 3600,
            "token_type": "Bearer"
        }"#;

        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.access_token, "ya29.access_token");
        assert!(resp.refresh_token.is_none());
        assert_eq!(resp.expires_in, 3600);
    }

    #[test]
    fn test_token_error_response_deserialization() {
        let json = r#"{
            "error": "authorization_pending",
            "error_description": "Authorization pending"
        }"#;

        let resp: TokenErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.error, "authorization_pending");
        assert_eq!(resp.error_description, "Authorization pending");
    }

    #[test]
    fn test_token_error_response_defaults() {
        let json = r#"{"error": "expired_token"}"#;

        let resp: TokenErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.error, "expired_token");
        assert_eq!(resp.error_description, "");
    }

    #[test]
    fn test_token_error_response_slow_down_deserialization() {
        let json = r#"{
            "error": "slow_down",
            "error_description": "Polling too fast, increase interval"
        }"#;

        let resp: TokenErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.error, "slow_down");
        assert_eq!(
            resp.error_description,
            "Polling too fast, increase interval"
        );
    }
}
