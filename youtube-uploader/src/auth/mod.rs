pub mod auth_code;
pub mod device_code;
pub mod refresh_token;
pub mod urls;

pub use refresh_token::now_secs;

/// OAuth2 token response from Google's token endpoint.
#[derive(Debug, serde::Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
    pub token_type: String,
}
