use std::env;

const DEFAULT_GOOGLE_DEVICE_CODE_URL: &str = "https://oauth2.googleapis.com/device/code";
const DEFAULT_GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const DEFAULT_YOUTUBE_UPLOAD_ENDPOINT: &str = "https://www.googleapis.com/upload/youtube/v3/videos";
const DEFAULT_YOUTUBE_UPLOAD_SCOPE: &str = "https://www.googleapis.com/auth/youtube";
const DEFAULT_YOUTUBE_API_URL: &str = "https://www.googleapis.com/youtube/v3";

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
    }

    // Env-var override testing is covered by wiremock integration tests
    // (tests/wiremock.rs) which set env vars before the process starts.
    // Using unsafe env::set_var in unit tests causes race conditions
    // under concurrent test execution and is no longer safe in Rust 1.82+.
}
