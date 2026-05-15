#![cfg(feature = "live-test")]

mod live_helpers;

use video_uploader::{
    platforms::odysee::OdyseeUploader,
    upload::{VideoUpload, Visibility},
};

#[tokio::test]
async fn test_odysee_upload_and_abandon() {
    live_helpers::load_test_env();

    let daemon_url = std::env::var("ODYSEE_TEST_DAEMON_URL")
        .unwrap_or_else(|_| "http://localhost:5279".to_string());
    let channel_name = std::env::var("ODYSEE_TEST_CHANNEL_NAME").ok();

    let uploader =
        OdyseeUploader::new(&daemon_url, channel_name).expect("Odysee uploader should initialize");

    let fixture = live_helpers::fixture_video();
    assert!(
        fixture.exists(),
        "fixture minimal.mp4 not found at {:?}",
        fixture
    );

    let video = VideoUpload::new(&fixture, &live_helpers::unique_title("odysee"))
        .description("Live test upload")
        .visibility(Visibility::Private);

    let result = uploader
        .upload(&video, None)
        .await
        .expect("Odysee upload should succeed");

    assert_eq!(result.platform, "odysee", "platform should be odysee");
    assert!(
        !result.platform_id.is_empty(),
        "platform_id should not be empty"
    );

    uploader
        .abandon_claim(&result.platform_id)
        .await
        .expect("Odysee abandon_claim should succeed");
}

#[tokio::test]
async fn test_odysee_upload_public() {
    live_helpers::load_test_env();

    let daemon_url = std::env::var("ODYSEE_TEST_DAEMON_URL")
        .unwrap_or_else(|_| "http://localhost:5279".to_string());
    let channel_name = std::env::var("ODYSEE_TEST_CHANNEL_NAME").ok();

    let uploader =
        OdyseeUploader::new(&daemon_url, channel_name).expect("Odysee uploader should initialize");

    let video = VideoUpload::new(
        live_helpers::fixture_video(),
        &live_helpers::unique_title("odysee-public"),
    )
    .visibility(Visibility::Public);

    let result = uploader
        .upload(&video, None)
        .await
        .expect("Odysee public upload should succeed");

    uploader
        .abandon_claim(&result.platform_id)
        .await
        .expect("abandon_claim should succeed");
}
