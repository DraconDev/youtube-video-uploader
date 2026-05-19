use crate::auth::urls::google_token_url;
use crate::net::build_http_client;
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub scope: Option<String>,
}

async fn refresh_access_token_with_url(
    client: &reqwest::Client,
    token_url: &str,
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<RefreshTokenResponse, crate::UploadError> {
    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];

    let response = client.post(token_url).form(&params).send().await?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(crate::UploadError::TokenRefresh(format!(
            "Google token refresh failed: {text}"
        )));
    }

    let token: RefreshTokenResponse = response.json().await.map_err(|e| {
        crate::UploadError::TokenRefresh(format!("Failed to parse refresh response: {e}"))
    })?;

    Ok(token)
}

pub async fn refresh_access_token(
    client: &reqwest::Client,
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<RefreshTokenResponse, crate::UploadError> {
    refresh_access_token_with_url(
        client,
        google_token_url().as_str(),
        refresh_token,
        client_id,
        client_secret,
    )
    .await
}

#[cfg(feature = "test-utils")]
pub async fn refresh_access_token_url(
    client: &reqwest::Client,
    token_url: &str,
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<RefreshTokenResponse, crate::UploadError> {
    refresh_access_token_with_url(client, token_url, refresh_token, client_id, client_secret).await
}

/// Standalone refresh that creates its own HTTP client.
/// Used only by the device code flow (which doesn't have a `YouTubeUploader` yet).
pub async fn refresh_access_token_standalone(
    refresh_token: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<RefreshTokenResponse, crate::UploadError> {
    let client = build_http_client();
    refresh_access_token(&client, refresh_token, client_id, client_secret).await
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn is_token_expired(expires_at: u64) -> bool {
    now_secs() + 60 >= expires_at
}

#[cfg(test)]
mod tests {
    use super::{is_token_expired, now_secs};

    #[test]
    fn test_token_not_expired_with_far_future_expiry() {
        let future = now_secs() + 3600;
        assert!(!is_token_expired(future));
    }

    #[test]
    fn test_token_expired_with_past_expiry() {
        let past = now_secs().saturating_sub(60);
        assert!(is_token_expired(past));
    }

    #[test]
    fn test_token_expired_within_60_second_buffer() {
        let near = now_secs() + 30;
        assert!(is_token_expired(near));
    }

    #[test]
    fn test_now_secs_returns_reasonable_unix_timestamp() {
        let now = now_secs();
        assert!(now > 1_700_000_000);
        assert!(now < 2_000_000_000);
    }

    #[test]
    fn test_is_token_expired_zero_expiry() {
        assert!(is_token_expired(0));
    }
}
