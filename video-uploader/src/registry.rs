use crate::{
    OdyseeUploader, PlatformUploader, ProgressListener, UploadError,
    YouTubeUploader,
    auth::urls::odysee_default_daemon_url,
    config::CredentialStore,
    upload::{UploadResult, VideoUpload},
};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use zeroize::Zeroizing;

/// Builder for constructing a `UploaderRegistry` with custom settings.
impl Default for UploaderRegistryBuilder {
    fn default() -> Self {
        Self { max_concurrency: 4 }
    }
}

//
pub struct UploaderRegistryBuilder {
    max_concurrency: usize,
}

impl UploaderRegistryBuilder {
    /// Maximum number of concurrent uploads across platforms.
    /// By default, 4 uploads run simultaneously.
    pub fn max_concurrency(mut self, n: usize) -> Self {
        if n == 0 {
            tracing::warn!("max_concurrency must be >= 1, using 1");
            self.max_concurrency = 1;
        } else {
            self.max_concurrency = n;
        }
        self
    }

    /// Build the registry from a loaded credential store.
    pub fn build(self, store: CredentialStore, passphrase: impl Into<String>) -> UploaderRegistry {
        UploaderRegistry {
            store: Arc::new(Mutex::new(store)),
            passphrase: Zeroizing::new(passphrase.into()),
            max_concurrency: self.max_concurrency,
        }
    }
}

/// Loads credentials from the encrypted store and constructs the appropriate
/// uploader for each platform. Uploads can be dispatched to one platform or
/// all configured platforms concurrently.
#[derive(Clone)]
pub struct UploaderRegistry {
    store: Arc<Mutex<CredentialStore>>,
    passphrase: Zeroizing<String>,
    max_concurrency: usize,
}

impl UploaderRegistry {
    /// Create a new registry from a loaded credential store.
    /// Uses a default `max_concurrency` of 4.
    pub fn new(store: CredentialStore, passphrase: impl Into<String>) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            passphrase: Zeroizing::new(passphrase.into()),
            max_concurrency: 4,
        }
    }

    /// Returns the maximum number of concurrent uploads configured for this registry.
    pub fn max_concurrency(&self) -> usize {
        self.max_concurrency
    }

    /// Load the registry from disk using the given passphrase.
    pub fn load(passphrase: impl Into<String>) -> Result<Self, UploadError> {
        let passphrase_str = passphrase.into();
        let store = CredentialStore::load(&passphrase_str)?;
        Ok(Self::new(store, passphrase_str))
    }

    /// Returns a builder for constructing a registry with custom settings.
    pub fn builder() -> UploaderRegistryBuilder {
        UploaderRegistryBuilder::default()
    }

    /// Returns true if the given platform has credentials configured.
    pub async fn is_configured(&self, platform: &str) -> bool {
        let store = self.store.lock().await;
        store.get(platform).is_some()
    }

    /// Returns a list of configured platform names.
    pub async fn configured_platforms(&self) -> Vec<String> {
        let store = self.store.lock().await;
        store.platforms().cloned().collect()
    }

    /// Build an uploader for a specific platform, if credentials exist.
    pub async fn get_uploader(&self, platform: &str) -> Option<Arc<dyn PlatformUploader>> {
        let store = self.store.lock().await;
        let creds = store.get(platform)?.clone();
        drop(store);

        match platform {
            "youtube" => {
                if creds.refresh_token.is_some()
                    && creds.client_id.is_some()
                    && creds.client_secret.is_some()
                {
                    Some(Arc::new(YouTubeUploader::new(
                        Arc::clone(&self.store),
                        self.passphrase.as_str(),
                    )))
                } else {
                    None
                }
            }
            "odysee" => {
                let daemon_url = creds
                    .daemon_url
                    .clone()
                    .unwrap_or_else(odysee_default_daemon_url);
                if let Err(e) = crate::platforms::odysee::validate_daemon_url(&daemon_url) {
                    tracing::error!("Invalid Odysee daemon URL: {e}");
                    return None;
                }
                let channel_name = None;
                match OdyseeUploader::new(daemon_url, channel_name) {
                    Ok(u) => Some(Arc::new(u) as Arc<dyn PlatformUploader>),
                    Err(e) => {
                        tracing::error!("Invalid Odysee daemon URL: {e}");
                        None
                    }
                }
            }
            _ => None,
        }
    }

    /// Upload a video to a single platform.
    pub async fn upload_to(
        &self,
        platform: &str,
        video: &VideoUpload,
        progress: Option<Arc<dyn ProgressListener>>,
    ) -> Result<UploadResult, UploadError> {
        let uploader = self
            .get_uploader(platform)
            .await
            .ok_or_else(|| UploadError::NotConfigured(platform.into()))?;

        crate::validation::validate(video, platform).await?;
        uploader.upload(video, progress).await
    }

    /// Upload a video to all configured platforms concurrently.
    /// Uses a semaphore to limit concurrent uploads to `max_concurrency`.
    pub async fn upload_to_all(
        &self,
        video: &VideoUpload,
        progress: Option<Arc<dyn ProgressListener>>,
    ) -> Vec<(String, Result<UploadResult, UploadError>)> {
        let platforms = self.configured_platforms().await;
        let semaphore = Arc::new(Semaphore::new(self.max_concurrency));

        let mut handles = Vec::with_capacity(platforms.len());

        for platform in platforms {
            if let Err(e) = crate::validation::validate(video, &platform).await {
                handles.push(tokio::spawn(async move { (platform, Err(e)) }));
                continue;
            }

            let semaphore = Arc::clone(&semaphore);
            let registry = self.clone();
            let video = video.clone();
            let progress = progress.clone();
            handles.push(tokio::spawn(async move {
                let _permit = semaphore
                    .acquire()
                    .await
                    .expect("semaphore closed unexpectedly");
                let result = registry.upload_to(&platform, &video, progress).await;
                (platform, result)
            }));
        }

        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok((platform, result)) => results.push((platform, result)),
                Err(e) => results.push((
                    "unknown".into(),
                    Err(UploadError::Other(format!("Task panicked: {e}"))),
                )),
            }
        }
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PlatformCredentials;

    #[tokio::test]
    async fn test_registry_empty_by_default() {
        let registry = UploaderRegistry::new(CredentialStore::default(), "test");
        assert!(!registry.is_configured("youtube").await);
        assert!(!registry.is_configured("odysee").await);
        assert!(registry.configured_platforms().await.is_empty());
    }

    #[tokio::test]
    async fn test_registry_detects_configured_platforms() {
        let mut store = CredentialStore::default();
        store.set(
            "youtube",
            PlatformCredentials {
                refresh_token: Some("rt".into()),
                access_token: None,
                api_key: None,
                token_expires_at: None,
                client_id: Some("cid".into()),
                client_secret: Some("cs".into()),
                daemon_url: None,
            },
        );
        let registry = UploaderRegistry::new(store, "test");
        let platforms = registry.configured_platforms().await;
        assert_eq!(platforms.len(), 1);
        assert!(registry.is_configured("youtube").await);
    }

    #[tokio::test]
    async fn test_registry_get_youtube_uploader() {
        let mut store = CredentialStore::default();
        store.set(
            "youtube",
            PlatformCredentials {
                refresh_token: Some("rt".into()),
                access_token: None,
                api_key: None,
                token_expires_at: None,
                client_id: Some("cid".into()),
                client_secret: Some("cs".into()),
                daemon_url: None,
            },
        );

        let registry = UploaderRegistry::new(store, "test");
        let uploader = registry.get_uploader("youtube").await;
        assert!(uploader.is_some());
        assert_eq!(uploader.unwrap().platform_name(), "youtube");
    }

    #[tokio::test]
    async fn test_registry_missing_credentials_returns_none() {
        let mut store = CredentialStore::default();
        store.set(
            "youtube",
            PlatformCredentials {
                client_id: Some("cid".into()),
                client_secret: Some("cs".into()),
                refresh_token: None,
                access_token: None,
                api_key: None,
                token_expires_at: None,
                daemon_url: None,
            },
        );

        let registry = UploaderRegistry::new(store, "test");
        assert!(registry.get_uploader("youtube").await.is_none());
    }

    #[tokio::test]
    async fn test_registry_upload_to_unconfigured_fails() {
        let registry = UploaderRegistry::new(CredentialStore::default(), "test");
        let video = VideoUpload::new("/tmp/fake.mp4", "Test");
        let result = registry.upload_to("youtube", &video, None).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UploadError::NotConfigured(_)));
    }

    #[tokio::test]
    async fn test_registry_builder_default_max_concurrency() {
        let store = CredentialStore::default();
        let registry = UploaderRegistry::builder().build(store, "pass");
        assert_eq!(registry.max_concurrency(), 4);
    }

    #[tokio::test]
    async fn test_registry_builder_custom_max_concurrency() {
        let store = CredentialStore::default();
        let registry = UploaderRegistry::builder()
            .max_concurrency(8)
            .build(store, "pass");
        assert_eq!(registry.max_concurrency(), 8);
    }
}
