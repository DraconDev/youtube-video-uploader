use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn load_test_env() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent() // video-uploader/
        .and_then(|p| p.parent()) // workspace root (video-uploader/)
        .expect("CARGO_MANIFEST_DIR should have at least two parent dirs");
    let env_test = workspace_root.join(".env.test");
    dotenvy::from_path(env_test).ok();
}

pub fn require_env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| {
        panic!(
            "{name} not set in .env.test or environment. See .env.test.example for required variables.",
            name = name
        )
    })
}

pub fn fixture_video() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("minimal.mp4")
}

pub fn unique_title(prefix: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before UNIX epoch")
        .as_secs();
    format!("{prefix} test {now}")
}

pub async fn delete_video(
    platform: &str,
    platform_id: &str,
) -> Result<(), video_uploader::UploadError> {
    use video_uploader::{OdyseeUploader, YouTubeUploader, config::CredentialStore};

    match platform {
        "youtube" => {
            let passphrase = "test-live-cleanup";
            let mut store = CredentialStore::default();
            let mut creds = video_uploader::config::PlatformCredentials::default();
            creds.refresh_token = std::env::var("YOUTUBE_TEST_REFRESH_TOKEN").ok();
            creds.client_id = std::env::var("YOUTUBE_TEST_CLIENT_ID").ok();
            creds.client_secret = std::env::var("YOUTUBE_TEST_CLIENT_SECRET").ok();
            store.set("youtube", creds);
            let store = std::sync::Arc::new(tokio::sync::Mutex::new(store));
            let uploader = YouTubeUploader::new(store, passphrase);
            uploader.delete_video(platform_id).await
        }
        "odysee" => {
            let daemon_url = std::env::var("ODYSEE_TEST_DAEMON_URL")
                .unwrap_or_else(|_| "http://localhost:5279".to_string());
            let channel_name = std::env::var("ODYSEE_TEST_CHANNEL_NAME").ok();
            let uploader = OdyseeUploader::new(&daemon_url, channel_name).map_err(|e| {
                video_uploader::UploadError::Config(format!("Odysee init failed: {e}"))
            })?;
            uploader.abandon_claim(platform_id).await
        }
        _ => Ok(()),
    }
}

pub struct VideoGuard {
    platform: String,
    platform_id: String,
    deleted: std::sync::atomic::AtomicBool,
}

impl VideoGuard {
    pub fn new(platform: &str, platform_id: &str) -> Self {
        Self {
            platform: platform.to_string(),
            platform_id: platform_id.to_string(),
            deleted: std::sync::atomic::AtomicBool::new(false),
        }
    }
}

impl Drop for VideoGuard {
    fn drop(&mut self) {
        if !self.deleted.load(std::sync::atomic::Ordering::SeqCst) {
            self.deleted
                .store(true, std::sync::atomic::Ordering::SeqCst);
            eprintln!(
                "[VideoGuard] WARNING: {} video {} was not explicitly deleted. \
                 Manual cleanup may be required.",
                self.platform, self.platform_id
            );
        }
    }
}

impl VideoGuard {
    pub async fn delete(self) {
        self.deleted
            .store(true, std::sync::atomic::Ordering::SeqCst);
        if let Err(e) = delete_video(&self.platform, &self.platform_id).await {
            eprintln!(
                "[VideoGuard] Cleanup failed for {} video {}: {}",
                self.platform, self.platform_id, e
            );
        }
    }
}
