use std::env;

const DEFAULT_GOOGLE_DEVICE_CODE_URL: &str = "https://oauth2.googleapis.com/device/code";
const DEFAULT_GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const DEFAULT_YOUTUBE_UPLOAD_ENDPOINT: &str = "https://www.googleapis.com/upload/youtube/v3/videos";
const DEFAULT_YOUTUBE_UPLOAD_SCOPE: &str = "https://www.googleapis.com/auth/youtube.upload";
const DEFAULT_YOUTUBE_API_URL: &str = "https://www.googleapis.com/youtube/v3";
const DEFAULT_ODYSEE_DAEMON_URL: &str = "http://localhost:5279";

pub fn google_device_code_url() -> String {
    env::var("GOOGLE_DEVICE_CODE_URL")
        .unwrap_or_else(|_| DEFAULT_GOOGLE_DEVICE_CODE_URL.to_string())
}

pub fn google_token_url() -> String {
    env::var("GOOGLE_TOKEN_URL").unwrap_or_else(|_| DEFAULT_GOOGLE_TOKEN_URL.to_string())
}

pub fn youtube_upload_endpoint() -> String {
    env::var("YOUTUBE_UPLOAD_ENDPOINT")
        .unwrap_or_else(|_| DEFAULT_YOUTUBE_UPLOAD_ENDPOINT.to_string())
}

pub fn youtube_upload_scope() -> String {
    env::var("YOUTUBE_UPLOAD_SCOPE").unwrap_or_else(|_| DEFAULT_YOUTUBE_UPLOAD_SCOPE.to_string())
}

pub fn youtube_api_url() -> String {
    env::var("YOUTUBE_API_URL").unwrap_or_else(|_| DEFAULT_YOUTUBE_API_URL.to_string())
}

pub fn odysee_default_daemon_url() -> String {
    env::var("ODYSEE_DAEMON_URL").unwrap_or_else(|_| DEFAULT_ODYSEE_DAEMON_URL.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        assert_eq!(google_device_code_url(), DEFAULT_GOOGLE_DEVICE_CODE_URL);
        assert_eq!(google_token_url(), DEFAULT_GOOGLE_TOKEN_URL);
        assert_eq!(youtube_upload_endpoint(), DEFAULT_YOUTUBE_UPLOAD_ENDPOINT);
        assert_eq!(youtube_upload_scope(), DEFAULT_YOUTUBE_UPLOAD_SCOPE);
        assert_eq!(youtube_api_url(), DEFAULT_YOUTUBE_API_URL);
        assert_eq!(odysee_default_daemon_url(), DEFAULT_ODYSEE_DAEMON_URL);
    }

    #[cfg(feature = "test-utils")]
    #[test]
    fn test_env_override() {
        unsafe {
            env::set_var("GOOGLE_TOKEN_URL", "https://custom.example.com/token");
            assert_eq!(google_token_url(), "https://custom.example.com/token");
            env::remove_var("GOOGLE_TOKEN_URL");

            env::set_var(
                "YOUTUBE_UPLOAD_ENDPOINT",
                "https://custom.example.com/upload",
            );
            assert_eq!(
                youtube_upload_endpoint(),
                "https://custom.example.com/upload"
            );
            env::remove_var("YOUTUBE_UPLOAD_ENDPOINT");

            env::set_var("YOUTUBE_API_URL", "https://custom.example.com/api");
            assert_eq!(youtube_api_url(), "https://custom.example.com/api");
            env::remove_var("YOUTUBE_API_URL");
        }
    }
}
