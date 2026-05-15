pub mod auth;
pub mod config;
pub mod error;
pub mod net;
pub mod platforms;
pub mod progress;
pub mod registry;
pub mod upload;
pub mod validation;

pub use config::{CredentialStore, PlatformCredentials};
pub use error::UploadError;
pub use net::is_private_ip;
pub use platforms::odysee::{OdyseeUploader, validate_daemon_url};
pub use platforms::youtube::YouTubeUploader;
pub use progress::{NoopProgressListener, ProgressListener, StderrProgressListener};
pub use registry::UploaderRegistry;
pub use upload::{UploadResult, VideoUpload, Visibility};

use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait PlatformUploader: Send + Sync {
    fn platform_name(&self) -> &'static str;

    async fn upload(
        &self,
        video: &VideoUpload,
        progress: Option<Arc<dyn ProgressListener>>,
    ) -> Result<UploadResult, UploadError>;

    fn supports_resumable(&self) -> bool {
        false
    }
}
