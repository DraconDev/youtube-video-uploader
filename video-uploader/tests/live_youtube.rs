#![cfg(feature = "live-test")]

mod live_helpers;

use std::sync::Arc;
use video_uploader::{
    PlatformUploader,
    auth::refresh_token::refresh_access_token,
    config::{CredentialStore, PlatformCredentials},
    platforms::youtube::YouTubeUploader,
    upload::{VideoUpload, Visibility},
};

#[tokio::test]
async fn test_youtube_refresh_access_token() {
    live_helpers::load_test_env();

    let refresh_token = live_helpers::require_env("YOUTUBE_TEST_REFRESH_TOKEN");
    let client_id = live_helpers::require_env("YOUTUBE_TEST_CLIENT_ID");
    let client_secret = live_helpers::require_env("YOUTUBE_TEST_CLIENT_SECRET");

    let token = refresh_access_token(&refresh_token, &client_id, &client_secret)
        .await
        .expect("token refresh should succeed with real credentials");

    assert!(
        !token.access_token.is_empty(),
        "access_token should not be empty"
    );
    assert!(token.expires_in > 0, "expires_in should be positive");
    assert_eq!(token.token_type, "Bearer", "token_type should be Bearer");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_youtube_upload_and_delete() {
    live_helpers::load_test_env();

    let refresh_token = live_helpers::require_env("YOUTUBE_TEST_REFRESH_TOKEN");
    let client_id = live_helpers::require_env("YOUTUBE_TEST_CLIENT_ID");
    let client_secret = live_helpers::require_env("YOUTUBE_TEST_CLIENT_SECRET");

    let passphrase = "test-live-youtube";

    let mut store = CredentialStore::default();
    let mut creds = PlatformCredentials::default();
    creds.refresh_token = Some(refresh_token);
    creds.client_id = Some(client_id);
    creds.client_secret = Some(client_secret);
    store.set("youtube", creds);

    let store = Arc::new(tokio::sync::Mutex::new(store));
    let uploader = YouTubeUploader::new(store, passphrase);

    let fixture = live_helpers::fixture_video();
    assert!(
        fixture.exists(),
        "fixture minimal.mp4 not found at {:?}",
        fixture
    );

    let video = VideoUpload::new(&fixture, &live_helpers::unique_title("yt"))
        .visibility(Visibility::Private);

    let result = uploader
        .upload(&video, None)
        .await
        .expect("YouTube upload should succeed");

    assert_eq!(result.platform, "youtube", "platform should be youtube");
    assert!(
        !result.platform_id.is_empty(),
        "platform_id should not be empty"
    );
    assert!(
        result.url.contains(&result.platform_id),
        "url should contain platform_id"
    );

    let video_id = result.platform_id.clone();

    uploader
        .delete_video(&video_id)
        .await
        .expect("YouTube delete should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_youtube_upload_with_tags() {
    live_helpers::load_test_env();

    let refresh_token = live_helpers::require_env("YOUTUBE_TEST_REFRESH_TOKEN");
    let client_id = live_helpers::require_env("YOUTUBE_TEST_CLIENT_ID");
    let client_secret = live_helpers::require_env("YOUTUBE_TEST_CLIENT_SECRET");

    let passphrase = "test-live-youtube";

    let mut store = CredentialStore::default();
    let mut creds = PlatformCredentials::default();
    creds.refresh_token = Some(refresh_token);
    creds.client_id = Some(client_id);
    creds.client_secret = Some(client_secret);
    store.set("youtube", creds);

    let store = Arc::new(tokio::sync::Mutex::new(store));
    let uploader = YouTubeUploader::new(store, passphrase);

    let video = VideoUpload::new(
        live_helpers::fixture_video(),
        &live_helpers::unique_title("yt-tags"),
    )
    .description("Test video with tags")
    .tags(vec!["live-test".to_string(), "automated".to_string()])
    .visibility(Visibility::Private);

    let result = uploader
        .upload(&video, None)
        .await
        .expect("YouTube upload with tags should succeed");

    let video_id = result.platform_id.clone();
    uploader
        .delete_video(&video_id)
        .await
        .expect("delete should succeed");
}
