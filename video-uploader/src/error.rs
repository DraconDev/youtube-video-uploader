/// Errors that can occur during video upload.
#[derive(thiserror::Error, Debug)]
pub enum UploadError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Token refresh failed: {0}")]
    TokenRefresh(String),

    #[error("Platform API error ({status}): {message}")]
    PlatformApi { status: u16, message: String },

    #[error("Upload interrupted after {uploaded} of {total} bytes")]
    Interrupted { uploaded: u64, total: u64 },

    #[error("File too large: {size} bytes (max {max})")]
    FileTooLarge { size: u64, max: u64 },

    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),

    #[error("Platform '{0}' is not configured")]
    NotConfigured(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("{0}")]
    Other(String),
}

impl UploadError {
    pub fn is_retryable(&self) -> bool {
        match self {
            UploadError::Http(_) => true,
            UploadError::Interrupted { .. } => true,
            UploadError::PlatformApi { status, .. } => matches!(status, 500..=504 | 429),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupted_error_is_retryable() {
        let err = UploadError::Interrupted {
            uploaded: 1000,
            total: 5000,
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_platform_api_5xx_is_retryable() {
        let err = UploadError::PlatformApi {
            status: 500,
            message: "Server error".into(),
        };
        assert!(err.is_retryable());

        let err = UploadError::PlatformApi {
            status: 502,
            message: "Bad gateway".into(),
        };
        assert!(err.is_retryable());

        let err = UploadError::PlatformApi {
            status: 503,
            message: "Service unavailable".into(),
        };
        assert!(err.is_retryable());

        let err = UploadError::PlatformApi {
            status: 429,
            message: "Rate limited".into(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_platform_api_4xx_not_retryable() {
        let err = UploadError::PlatformApi {
            status: 400,
            message: "Bad request".into(),
        };
        assert!(!err.is_retryable());

        let err = UploadError::PlatformApi {
            status: 401,
            message: "Unauthorized".into(),
        };
        assert!(!err.is_retryable());

        let err = UploadError::PlatformApi {
            status: 404,
            message: "Not found".into(),
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_auth_error_not_retryable() {
        let err = UploadError::Auth("Invalid credentials".into());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_io_error_not_retryable() {
        let err = UploadError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        ));
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_encryption_error_not_retryable() {
        let err = UploadError::Encryption("failed to decrypt".into());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_other_error_not_retryable() {
        let err = UploadError::Other("Something went wrong".into());
        assert!(!err.is_retryable());
    }
}
