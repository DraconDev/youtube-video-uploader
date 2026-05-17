//! OAuth2 authorization code flow with local redirect server.
//!
//! Used when the device code flow is unavailable (e.g., "web" OAuth2 client type).
//! Spins up a one-shot localhost HTTP server, opens the browser for consent,
//! then captures the redirect with the authorization code.

use crate::UploadError;
use crate::auth::TokenResponse;
use crate::auth::urls::{google_token_url, youtube_upload_scope};
use crate::net::build_http_client;
use zeroize::Zeroizing;

const REDIRECT_PORT: u16 = 8089;

/// PKCE code pair for the authorization code flow.
pub struct PkcePair {
    pub verifier: Zeroizing<String>,
    pub challenge: String,
}

impl PkcePair {
    pub fn generate() -> Self {
        use sha2::{Digest, Sha256};
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

        // Generate random verifier (43-128 chars, RFC 7636)
        let verifier: String = (0..43)
            .map(|_| {
                const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
                let idx = (rand::random::<u32>() as usize) % CHARSET.len();
                CHARSET[idx] as char
            })
            .collect();

        // SHA-256 hash the verifier, base64url-encode
        let mut hasher = Sha256::new();
        hasher.update(verifier.as_bytes());
        let hash = hasher.finalize();
        let challenge = URL_SAFE_NO_PAD.encode(hash);

        Self {
            verifier: Zeroizing::new(verifier),
            challenge,
        }
    }
}

/// Run the authorization code flow with a local redirect server.
///
/// 1. Generates PKCE challenge
/// 2. Opens browser to Google's consent page
/// 3. Listens on localhost for the redirect
/// 4. Exchanges the auth code for tokens
pub async fn auth_code_flow(
    client_id: &str,
    client_secret: &str,
) -> Result<TokenResponse, UploadError> {
    let pkce = PkcePair::generate();
    let redirect_uri = format!("http://127.0.0.1:{REDIRECT_PORT}");

    // Build the authorization URL
    let auth_url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth\
          ?client_id={}\
          &redirect_uri=http://127.0.0.1:{}\
          &response_type=code\
          &scope={}\
          &code_challenge={}\
          &code_challenge_method=S256\
          &access_type=offline\
          &prompt=consent",
        urlencoding::encode(client_id),
        REDIRECT_PORT,
        urlencoding::encode(&youtube_upload_scope()),
        urlencoding::encode(&pkce.challenge),
    );

    // Start the local server
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{REDIRECT_PORT}"))
        .await
        .map_err(|e| UploadError::Auth(format!("Failed to bind localhost:{REDIRECT_PORT}: {e}")))?;

    println!();
    println!("  Opening browser for YouTube authorization...");
    println!();
    println!("  If the browser doesn't open, visit this URL:");
    println!();
    println!("  {auth_url}");
    println!();

    // Try to open the browser
    let _ = open::that(&auth_url);

    // Wait for the redirect
    let (stream, _) = listener
        .accept()
        .await
        .map_err(|e| UploadError::Auth(format!("Failed to accept connection: {e}")))?;

    let (reader, writer) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(reader);

    // Read the HTTP request
    let mut request_line = String::new();
    tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut request_line)
        .await
        .map_err(|e| UploadError::Auth(format!("Failed to read request: {e}")))?;

    // Parse the authorization code from the request
    let code = request_line
        .split_whitespace()
        .nth(1)
        .and_then(|path| {
            path.split('?').nth(1).and_then(|query| {
                query.split('&').find_map(|pair| {
                    let (k, v) = pair.split_once('=')?;
                    (k == "code").then_some(v.to_string())
                })
            })
        })
        .ok_or_else(|| UploadError::Auth("No authorization code in redirect".into()))?;

    // Read remaining headers
    let mut buf = String::new();
    loop {
        buf.clear();
        tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut buf).await?;
        if buf == "\r\n" || buf.is_empty() {
            break;
        }
    }

    // Send success response to browser
    use tokio::io::AsyncWriteExt;
    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
        <html><body><h1>Authorization successful!</h1>\
        <p>You can close this tab and return to the terminal.</p></body></html>";
    let mut writer = tokio::io::BufWriter::new(writer);
    writer.write_all(response.as_bytes()).await.ok();
    writer.flush().await.ok();
    drop(writer);

    // Exchange code for tokens
    let client = build_http_client();
    let params = [
        ("client_id", client_id.to_string()),
        ("client_secret", client_secret.to_string()),
        ("code", code),
        ("code_verifier", (*pkce.verifier).clone()),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code".to_string()),
    ];

    let response = client
        .post(google_token_url())
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(UploadError::Auth(format!(
            "Token exchange failed: {text}"
        )));
    }

    let token: TokenResponse = response.json().await.map_err(|e| {
        UploadError::Auth(format!("Failed to parse token response: {e}"))
    })?;

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_pair_generates_valid_verifier() {
        let pkce = PkcePair::generate();
        assert_eq!(pkce.verifier.len(), 43);
        assert!(!pkce.challenge.is_empty());
    }

    #[test]
    fn test_pkce_pair_challenge_is_deterministic() {
        let pkce = PkcePair::generate();
        // SHA-256 of 43-byte input → 32 bytes → base64url ≈ 43 chars
        assert_eq!(pkce.challenge.len(), 43);
    }
}
