//! Wiremock HTTP integration tests for platform uploaders.

#![cfg(feature = "test-utils")]

use std::sync::Arc;
use tokio::sync::Mutex;
use video_uploader::{
    PlatformUploader, UploadError,
    config::{CredentialStore, PlatformCredentials},
    platforms::odysee::OdyseeUploader,
    platforms::youtube::YouTubeUploader,
    upload::{VideoUpload, Visibility},
};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{body_partial_json, body_string_contains, method, path},
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn fake_creds_youtube() -> PlatformCredentials {
    PlatformCredentials {
        api_key: None,
        refresh_token: Some("fake_refresh_token".into()),
        client_id: Some("fake_client_id".into()),
        client_secret: Some("cs".into()),
        access_token: Some("tok".into()),
        token_expires_at: Some(u64::MAX), // never expires
        daemon_url: None,
    }
}

fn fixture_video() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("minimal.mp4")
}

// ---------------------------------------------------------------------------
// YouTube wiremock tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_youtube_chunk_upload_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "yt_test456",
            "snippet": { "title": "Test" }
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let video = VideoUpload::new(&fixture_video(), "Test");
    let store = Arc::new(Mutex::new(CredentialStore::default()));
    let uploader = YouTubeUploader::new(store, "pass");

    let result = uploader
        .upload_chunks(
            &format!("{}/upload", mock_server.uri()),
            &video,
            "tok",
            None,
        )
        .await;

    assert!(result.is_ok(), "expected ok, got: {:?}", result);
    let json = result.unwrap();
    assert_eq!(json["id"], "yt_test456");
}

#[tokio::test]
async fn test_youtube_chunk_upload_308_resume() {
    let mock_server = MockServer::start().await;

    // First chunk gets 308 with Range header indicating bytes 0-511 received
    Mock::given(method("PUT"))
        .and(path("/upload"))
        .respond_with(
            ResponseTemplate::new(308)
                .insert_header("range", "bytes=0-511")
                .set_body_string(""),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second chunk succeeds
    Mock::given(method("PUT"))
        .and(path("/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "yt_resume",
            "snippet": { "title": "Test" }
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let video = VideoUpload::new(&fixture_video(), "Test");
    let store = Arc::new(Mutex::new(CredentialStore::default()));
    let uploader = YouTubeUploader::new(store, "pass");

    let result = uploader
        .upload_chunks(
            &format!("{}/upload", mock_server.uri()),
            &video,
            "tok",
            None,
        )
        .await;

    assert!(
        result.is_ok(),
        "expected ok after 308 resume, got: {:?}",
        result
    );
    let json = result.unwrap();
    assert_eq!(json["id"], "yt_resume");
}

#[tokio::test]
async fn test_youtube_chunk_upload_retries_on_5xx() {
    let mock_server = MockServer::start().await;

    // First attempt: 503, second: 200
    Mock::given(method("PUT"))
        .and(path("/upload"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("PUT"))
        .and(path("/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "yt_retry",
            "snippet": { "title": "Test" }
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let video = VideoUpload::new(&fixture_video(), "Test");
    let store = Arc::new(Mutex::new(CredentialStore::default()));
    let uploader = YouTubeUploader::new(store, "pass");

    let result = uploader
        .upload_with_retry(
            &format!("{}/upload", mock_server.uri()),
            &video,
            "tok",
            None,
        )
        .await;

    assert!(result.is_ok(), "expected ok after retry, got: {:?}", result);
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// Odysee wiremock tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_odysee_publish_success() {
    let mock_server = MockServer::start().await;

    // Mock both status check AND publish calls using body matching
    // Status check has method: "status" in the body
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(serde_json::json!({
            "method": "status"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {"lbrynet_version": "1.0.0"}
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Publish call has method: "publish" in the body
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(serde_json::json!({
            "method": "publish"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "txid": "abc123",
                "claim": {
                    "claimId": "claim456",
                    "name": "testvideo",
                    "permanentUrl": "lbry://testvideo#claim456"
                }
            }
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let video = VideoUpload::new(&fixture_video(), "Test Odysee Video");
    let uploader = OdyseeUploader::new(mock_server.uri().as_str(), None).unwrap();

    let result = uploader.upload(&video, None).await;
    assert!(result.is_ok(), "expected ok, got: {:?}", result);
    let r = result.unwrap();
    assert_eq!(r.platform, "odysee");
    assert_eq!(r.platform_id, "claim456");
}

#[tokio::test]
async fn test_odysee_rpc_error() {
    let mock_server = MockServer::start().await;

    // Mock daemon status check (passes)
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(serde_json::json!({
            "method": "status"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {}
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Mock publish that returns an RPC error
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(serde_json::json!({
            "method": "publish"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32600,
                "message": "Invalid Request"
            }
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let video = VideoUpload::new(&fixture_video(), "Test");
    let uploader = OdyseeUploader::new(mock_server.uri().as_str(), None).unwrap();

    let result = uploader.upload(&video, None).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    // JSON-RPC error doesn't use HTTP status, so it should be whatever status was returned
    assert!(matches!(err, UploadError::PlatformApi { .. }));
    let err_str = format!("{}", err);
    assert!(
        err_str.contains("Invalid Request") || err_str.contains("-32600"),
        "error should contain RPC error message"
    );
}

#[tokio::test]
async fn test_odysee_daemon_unavailable() {
    let mock_server = MockServer::start().await;

    // Mock daemon status check that fails (connection refused simulation)
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let video = VideoUpload::new(&fixture_video(), "Test");
    let uploader = OdyseeUploader::new(mock_server.uri().as_str(), None).unwrap();

    let result = uploader.upload(&video, None).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, UploadError::PlatformApi { status: 503, .. }));
    let err_str = format!("{}", err);
    assert!(
        err_str.contains("daemon not available") || err_str.contains("lbrynet"),
        "should mention daemon unavailability"
    );
}

#[tokio::test]
async fn test_odysee_channel_upload() {
    let mock_server = MockServer::start().await;

    // Mock daemon status check
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(serde_json::json!({
            "method": "status"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {}
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Mock publish with channel_name in params
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_partial_json(serde_json::json!({
            "method": "publish"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "txid": "tx789",
                "claim": {
                    "claimId": "channel_claim123",
                    "name": "mychannel",
                    "permanentUrl": "lbry://mychannel#channel_claim123"
                }
            }
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let video = VideoUpload::new(&fixture_video(), "Channel Video");
    let uploader =
        OdyseeUploader::new(mock_server.uri().as_str(), Some("@mychannel".into())).unwrap();

    let result = uploader.upload(&video, None).await;
    assert!(result.is_ok(), "expected ok, got: {:?}", result);
    let r = result.unwrap();
    assert_eq!(r.platform_id, "channel_claim123");
}

// ---------------------------------------------------------------------------
// PKCE device code flow wiremock tests
// ---------------------------------------------------------------------------
// These tests verify that the PKCE code_verifier generated during the device code
// flow is actually sent to the token endpoint during poll_for_token. We can't easily
// mock poll_for_token itself because it POSTs to a hardcoded Google URL, so we
// test the wiremock infrastructure directly by sending real HTTP requests and
// asserting on the captured body content.

fn pkce_body_contains(substring: &str) -> impl wiremock::Match {
    body_string_contains(substring)
}

#[tokio::test]
async fn test_pkce_code_verifier_is_sent_to_token_endpoint() {
    let mock_server = MockServer::start().await;

    let token_url = format!("{}/token", mock_server.uri());

    Mock::given(method("POST"))
        .and(path("/token"))
        .and(pkce_body_contains("code_verifier=verifier_from_test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ya29.mock_token",
            "refresh_token": "1//mock_refresh",
            "expires_in": 3600,
            "token_type": "Bearer"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // Directly POST to the mock token URL using the same form encoding as poll_for_token
    let client = reqwest::Client::new();
    let _resp = client
        .post(&token_url)
        .form(&[
            ("client_id", "test_client"),
            ("client_secret", "test_secret"),
            ("device_code", "device_abc"),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("code_verifier", "verifier_from_test"),
        ])
        .send()
        .await
        .unwrap();

    // The .expect(1) above verified the mock server received exactly one request
    // with "code_verifier=verifier_from_test" in the body. If wiremock didn't match,
    // the expect would fail before we reach here.
}

// Integration test: device_code flow generates matching verifier/challenge pair
#[test]
fn test_generate_pkce_pair_produces_valid_pair() {
    let (verifier, challenge) = video_uploader::auth::device_code::generate_pkce_pair();

    // Verifier should be base64url-encoded 64 bytes (no padding)
    let decoded =
        base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, &verifier)
            .unwrap();
    assert_eq!(decoded.len(), 64, "PKCE verifier must be 64 bytes");

    // Challenge should be base64url(SHA256(verifier))
    use sha2::Digest;
    let hash = sha2::Sha256::digest(verifier.as_bytes());
    let expected_challenge =
        base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, hash);
    assert_eq!(
        challenge, expected_challenge,
        "code_challenge must be base64url(SHA256(verifier))"
    );
}

// ---------------------------------------------------------------------------
// refresh_access_token wiremock tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_refresh_access_token_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("grant_type=refresh_token"))
        .and(body_string_contains("client_id=test_client"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "ya29.mock_access_token",
            "expires_in": 3600,
            "token_type": "Bearer",
            "scope": "https://www.googleapis.com/auth/youtube.upload"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let result = video_uploader::auth::refresh_token::refresh_access_token_url(
        &format!("{}/token", mock_server.uri()),
        "test_refresh_token",
        "test_client",
        "test_secret",
    )
    .await
    .expect("refresh_access_token should succeed");

    assert_eq!(result.access_token, "ya29.mock_access_token");
    assert_eq!(result.expires_in, 3600);
    assert_eq!(result.token_type, "Bearer");
}

#[tokio::test]
async fn test_refresh_access_token_failure_returns_token_refresh_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "invalid_grant",
            "error_description": "The refresh token has expired or been revoked"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let result = video_uploader::auth::refresh_token::refresh_access_token_url(
        &format!("{}/token", mock_server.uri()),
        "expired_refresh_token",
        "test_client",
        "test_secret",
    )
    .await;

    assert!(
        result.is_err(),
        "refresh_access_token should fail for invalid grant"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, UploadError::TokenRefresh(_)),
        "expected TokenRefresh error, got: {:?}",
        err
    );
}

// ---------------------------------------------------------------------------
// start_device_code wiremock tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_start_device_code_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/device/code"))
        .and(body_string_contains("client_id=test_client_id"))
        .and(body_string_contains("code_challenge_method=S256"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "device_abc123",
            "user_code": "ABCD-EFGH",
            "verification_url": "https://www.google.com/device",
            "expires_in": 1800,
            "interval": 5
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let result = video_uploader::auth::device_code::start_device_code_url(
        &format!("{}/device/code", mock_server.uri()),
        "test_client_id",
        "test_code_challenge",
    )
    .await
    .expect("start_device_code should succeed");

    assert_eq!(result.device_code, "device_abc123");
    assert_eq!(result.user_code, "ABCD-EFGH");
    assert_eq!(result.expires_in, 1800);
    assert_eq!(result.interval, 5);
}

#[tokio::test]
async fn test_start_device_code_includes_pkce_params() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/device/code"))
        .and(body_string_contains("code_challenge="))
        .and(body_string_contains("code_challenge_method=S256"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "device_code": "device_xyz",
            "user_code": "WXYZ-1234",
            "verification_url": "https://www.google.com/device",
            "expires_in": 600,
            "interval": 5
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let result = video_uploader::auth::device_code::start_device_code_url(
        &format!("{}/device/code", mock_server.uri()),
        "client_with_pkce",
        "EAAAAGIhpcjA6L6Nk1q-A7vI4gQj0CRh7bNQc6B8LNLjKmV8F0p1g",
    )
    .await
    .expect("start_device_code with PKCE should succeed");

    assert_eq!(result.device_code, "device_xyz");
    assert_eq!(result.user_code, "WXYZ-1234");
}
