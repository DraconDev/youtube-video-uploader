use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use youtube_uploader::{
    ProgressListener, UploadError, Zeroizing,
    config::{CredentialStore, PlatformCredentials},
    upload::{VideoUpload, Visibility},
};

#[tokio::test]
async fn test_youtube_uploader_requires_refresh_token() {
    let store = Arc::new(Mutex::new(CredentialStore::default()));
    let yt = youtube_uploader::YouTubeUploader::new(store, "passphrase", "youtube");

    let video = VideoUpload::new("tests/fixtures/video.mp4", "Test");
    let result = yt.upload(&video, None).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, UploadError::Auth(_)));
}

#[test]
fn test_credential_store_empty_by_default() {
    let store = CredentialStore::default();
    assert!(store.get("youtube").is_none());
}

#[test]
fn test_credential_store_remove() {
    let mut store = CredentialStore::default();
    store.set("youtube", PlatformCredentials::default());
    assert!(store.get("youtube").is_some());

    let removed = store.remove("youtube");
    assert!(removed.is_some());
    assert!(store.get("youtube").is_none());
}

#[test]
fn test_credential_store_workspaces_iterator() {
    let mut store = CredentialStore::default();
    store.set("youtube", PlatformCredentials::default());
    store.set("youtube_alt", PlatformCredentials::default());

    let mut workspaces: Vec<_> = store.workspaces().cloned().collect();
    workspaces.sort();
    assert_eq!(workspaces, vec!["youtube", "youtube_alt"]);
}

#[test]
fn test_credential_store_multiple_workspaces() {
    let mut store = CredentialStore::default();

    let yt_creds = PlatformCredentials::new(
        Some("refresh".to_string()),
        None,
        Some("client_id".to_string()),
        Some("client_secret".to_string()),
    );

    store.set("youtube", yt_creds);

    let yt = store.get("youtube").expect("youtube should exist");
    assert_eq!(
        yt.refresh_token.as_ref().map(|z| z.as_str()),
        Some("refresh")
    );
    assert_eq!(yt.client_id.as_ref().map(|z| z.as_str()), Some("client_id"));
}

#[test]
fn test_visibility_serde_roundtrip() {
    for visibility in &[
        Visibility::Public,
        Visibility::Unlisted,
        Visibility::Private,
    ] {
        let json = serde_json::to_string(visibility).unwrap();
        let parsed: Visibility = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, *visibility);
    }
}

#[test]
fn test_visibility_display() {
    assert_eq!(Visibility::Public.to_string(), "public");
    assert_eq!(Visibility::Unlisted.to_string(), "unlisted");
    assert_eq!(Visibility::Private.to_string(), "private");
}

#[test]
fn test_upload_error_is_retryable() {
    // HTTP errors are retryable
    let err = UploadError::Config("other".into());
    assert!(!err.is_retryable());

    // Interrupted is retryable
    let err = UploadError::Interrupted {
        uploaded: 1,
        total: 100,
    };
    assert!(err.is_retryable());
}

#[test]
fn test_upload_error_is_not_retryable() {
    for err in [
        UploadError::Auth("bad".into()),
        UploadError::Config("cfg".into()),
        UploadError::Encryption("enc".into()),
        UploadError::Config("other".into()),
        UploadError::UnsupportedFormat("fmt".into()),
    ] {
        assert!(!err.is_retryable(), " {:?} should not be retryable", err);
    }
}

#[test]
fn test_platform_api_error_retryable_statuses() {
    for (status, expected) in [
        (429, true),
        (500, true),
        (502, true),
        (503, true),
        (504, true),
        (400, false),
        (401, false),
        (403, false),
        (404, false),
    ] {
        let err = UploadError::PlatformApi {
            status,
            message: "test".into(),
        };
        assert_eq!(
            err.is_retryable(),
            expected,
            "status {} should be retryable={}",
            status,
            expected
        );
    }
}

#[test]
fn test_video_upload_file_path() {
    let video = VideoUpload::new("/tmp/video.mp4", "Title");
    assert_eq!(video.file_path(), std::path::Path::new("/tmp/video.mp4"));
}

#[test]
fn test_video_upload_file_path_owned() {
    let path = std::path::PathBuf::from("/tmp/video.mp4");
    let video = VideoUpload::new(path.clone(), "Title");
    assert_eq!(video.file_path(), path.as_path());
}

// ---------------------------------------------------------------------------
// Credential encryption edge case tests
// ---------------------------------------------------------------------------

// Helper functions for encryption tests (to avoid orphan rule issues)
fn encrypt_store_to_file(
    store: &CredentialStore,
    passphrase: &str,
    path: &PathBuf,
) -> Result<(), youtube_uploader::UploadError> {
    use aes_gcm::{
        Aes256Gcm, Nonce,
        aead::{Aead, Key, KeyInit},
    };
    use pbkdf2::pbkdf2_hmac;
    use sha2::Sha256;
    use std::io::Write;
    use youtube_uploader::config::{FORMAT_MAGIC, FORMAT_VERSION_V2, NONCE_SIZE, SALT_SIZE};

    let plaintext = toml::to_string(store).map_err(|e| {
        youtube_uploader::UploadError::Encryption(format!("Failed to serialize: {e}"))
    })?;

    let salt: [u8; SALT_SIZE] = rand::random();
    let mut key_bytes = [0u8; 32];
    pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), &salt, 100_000, &mut key_bytes);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce_bytes: [u8; NONCE_SIZE] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| youtube_uploader::UploadError::Encryption(format!("Encrypt failed: {e}")))?;

    let mut file = std::fs::File::create(path).map_err(youtube_uploader::UploadError::Io)?;
    file.write_all(FORMAT_MAGIC)
        .map_err(youtube_uploader::UploadError::Io)?;
    file.write_all(&[FORMAT_VERSION_V2])
        .map_err(youtube_uploader::UploadError::Io)?;
    file.write_all(&salt)
        .map_err(youtube_uploader::UploadError::Io)?;
    file.write_all(&nonce_bytes)
        .map_err(youtube_uploader::UploadError::Io)?;
    file.write_all(&ciphertext)
        .map_err(youtube_uploader::UploadError::Io)?;
    file.sync_all().map_err(youtube_uploader::UploadError::Io)?;

    Ok(())
}

fn decrypt_store_from_file(
    passphrase: &str,
    path: &PathBuf,
) -> Result<CredentialStore, youtube_uploader::UploadError> {
    use aes_gcm::{
        Aes256Gcm, Nonce,
        aead::{Aead, Key, KeyInit},
    };
    use pbkdf2::pbkdf2_hmac;
    use sha2::Sha256;
    use std::io::Read;
    use youtube_uploader::config::{FORMAT_MAGIC, NONCE_SIZE, SALT_SIZE};

    let mut file = std::fs::File::open(path).map_err(youtube_uploader::UploadError::Io)?;
    let mut ciphertext = Vec::new();
    file.read_to_end(&mut ciphertext)
        .map_err(youtube_uploader::UploadError::Io)?;

    let header_len = FORMAT_MAGIC.len() + 1 + SALT_SIZE + NONCE_SIZE;
    if ciphertext.len() < header_len {
        return Err(youtube_uploader::UploadError::Encryption(
            "File too short".into(),
        ));
    }

    let salt = &ciphertext[FORMAT_MAGIC.len() + 1..FORMAT_MAGIC.len() + 1 + SALT_SIZE];
    let nonce = Nonce::from_slice(
        &ciphertext
            [FORMAT_MAGIC.len() + 1 + SALT_SIZE..FORMAT_MAGIC.len() + 1 + SALT_SIZE + NONCE_SIZE],
    );
    let encrypted = &ciphertext[header_len..];

    let mut key_bytes = [0u8; 32];
    pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), salt, 100_000, &mut key_bytes);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let plaintext = cipher
        .decrypt(nonce, encrypted)
        .map_err(|e| youtube_uploader::UploadError::Encryption(format!("Decrypt failed: {e}")))?;

    let store: CredentialStore = toml::from_str(&String::from_utf8_lossy(&plaintext))
        .map_err(|e| youtube_uploader::UploadError::Encryption(format!("Parse failed: {e}")))?;

    Ok(store)
}

#[test]
fn test_credential_store_wrong_passphrase_returns_error() {
    use aes_gcm::{
        Aes256Gcm, Nonce,
        aead::{Aead, Key, KeyInit},
    };
    use pbkdf2::pbkdf2_hmac;
    use sha2::Sha256;
    use std::io::Write;
    use youtube_uploader::config::{FORMAT_MAGIC, FORMAT_VERSION_V2, NONCE_SIZE, SALT_SIZE};

    // Create and save a store with one passphrase
    let mut store = CredentialStore::default();
    store.set(
        "test",
        PlatformCredentials {
            api_key: Some(Zeroizing::new("secret".to_string())),
            refresh_token: None,
            access_token: None,
            token_expires_at: None,
            client_id: None,
            client_secret: None,
            channel_id: None,
            channel_name: None,
        },
    );

    // Save to temp file with correct passphrase
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();

    // Manually write encrypted data with "correct_pass"
    let correct_pass = "correct_pass";
    let salt: [u8; SALT_SIZE] = rand::random();
    let mut key_bytes = [0u8; 32];
    pbkdf2_hmac::<Sha256>(correct_pass.as_bytes(), &salt, 100_000, &mut key_bytes);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce_bytes: [u8; NONCE_SIZE] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext = toml::to_string(&store).unwrap();
    let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes()).unwrap();

    {
        let mut file = std::fs::File::create(&temp_path).unwrap();
        file.write_all(FORMAT_MAGIC).unwrap();
        file.write_all(&[FORMAT_VERSION_V2]).unwrap();
        file.write_all(&salt).unwrap();
        file.write_all(&nonce_bytes).unwrap();
        file.write_all(&ciphertext).unwrap();
    }

    // Verify different passphrases produce different keys
    let wrong_key = {
        let salt = [0u8; SALT_SIZE];
        let mut key_bytes = [0u8; 32];
        pbkdf2_hmac::<Sha256>(b"wrong_pass", &salt, 100_000, &mut key_bytes);
        key_bytes
    };
    assert_ne!(key_bytes.as_slice(), &wrong_key);

    drop(temp_file);
}

#[test]
fn test_credential_store_empty_passphrase_works() {
    let mut store = CredentialStore::default();
    store.set(
        "test",
        PlatformCredentials {
            api_key: Some(Zeroizing::new("secret".to_string())),
            refresh_token: None,
            access_token: None,
            token_expires_at: None,
            client_id: None,
            client_secret: None,
            channel_id: None,
            channel_name: None,
        },
    );

    let passphrase = "";

    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();

    encrypt_store_to_file(&store, passphrase, &temp_path)
        .expect("save with empty passphrase should succeed");

    // Load back
    let loaded = decrypt_store_from_file(passphrase, &temp_path)
        .expect("load with empty passphrase should succeed");

    let creds = loaded.get("test").expect("test workspace should exist");
    assert_eq!(creds.api_key.as_ref().map(|z| z.as_str()), Some("secret"));

    drop(temp_file);
}

#[test]
fn test_credential_store_special_chars_in_passphrase() {
    let mut store = CredentialStore::default();
    store.set(
        "test",
        PlatformCredentials {
            api_key: Some(Zeroizing::new("secret".to_string())),
            refresh_token: None,
            access_token: None,
            token_expires_at: None,
            client_id: None,
            client_secret: None,
            channel_id: None,
            channel_name: None,
        },
    );

    // Passphrase with special characters, unicode, emojis
    let special_passphrases = vec![
        "p@$$w0rd!#$%^&*()",
        "пароль_日本語",
        "password🔐with🔑emojis",
        "   spaced   ",
        "tab\there",
    ];

    for pass in special_passphrases {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Save with special passphrase
        encrypt_store_to_file(&store, pass, &temp_path)
            .unwrap_or_else(|_| panic!("save with passphrase {:?} should succeed", pass));

        // Load back and verify
        let loaded = decrypt_store_from_file(pass, &temp_path)
            .unwrap_or_else(|_| panic!("load with passphrase {:?} should succeed", pass));

        let creds = loaded.get("test").expect("test workspace should exist");
        assert_eq!(
            creds.api_key.as_ref().map(|z| z.as_str()),
            Some("secret"),
            "passphrase {:?} should round-trip correctly",
            pass
        );

        drop(temp_file);
    }
}

// ---------------------------------------------------------------------------
// Progress listener tests
// ---------------------------------------------------------------------------

struct TestProgressListener {
    progress_calls: std::sync::Mutex<Vec<(u64, u64)>>,
    complete_calls: std::sync::Mutex<Vec<youtube_uploader::UploadResult>>,
    error_count: std::sync::Mutex<usize>,
}

impl TestProgressListener {
    fn new() -> Self {
        Self {
            progress_calls: std::sync::Mutex::new(Vec::new()),
            complete_calls: std::sync::Mutex::new(Vec::new()),
            error_count: std::sync::Mutex::new(0),
        }
    }

    fn get_progress_calls(&self) -> Vec<(u64, u64)> {
        self.progress_calls.lock().unwrap().clone()
    }

    fn get_complete_calls(&self) -> Vec<youtube_uploader::UploadResult> {
        self.complete_calls.lock().unwrap().clone()
    }

    fn get_error_count(&self) -> usize {
        *self.error_count.lock().unwrap()
    }
}

impl youtube_uploader::ProgressListener for TestProgressListener {
    fn on_progress(&self, uploaded: u64, total: u64) {
        self.progress_calls.lock().unwrap().push((uploaded, total));
    }

    fn on_complete(&self, result: &youtube_uploader::UploadResult) {
        self.complete_calls.lock().unwrap().push(result.clone());
    }

    fn on_error(&self, _error: &youtube_uploader::UploadError) {
        *self.error_count.lock().unwrap() += 1;
    }
}

#[test]
fn test_progress_listener_on_progress_called() {
    let listener = TestProgressListener::new();

    // Simulate progress calls
    listener.on_progress(0, 1000);
    listener.on_progress(500, 1000);
    listener.on_progress(1000, 1000);

    let calls = listener.get_progress_calls();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0], (0, 1000));
    assert_eq!(calls[1], (500, 1000));
    assert_eq!(calls[2], (1000, 1000));
}

#[test]
fn test_progress_listener_on_complete_called() {
    let listener = TestProgressListener::new();

    let result = youtube_uploader::UploadResult::new(
        "youtube",
        "uuid123",
        "https://example.com/video",
        "Test Video",
    );

    listener.on_complete(&result);

    let calls = listener.get_complete_calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].workspace, "youtube");
    assert_eq!(calls[0].video_id, "uuid123");
}

#[test]
fn test_progress_listener_on_error_called() {
    let listener = TestProgressListener::new();

    let error = youtube_uploader::UploadError::PlatformApi {
        status: 500,
        message: "Server error".into(),
    };

    listener.on_error(&error);

    assert_eq!(listener.get_error_count(), 1);
}

#[test]
fn test_credential_store_wrong_passphrase_v2() {
    let mut store = CredentialStore::default();
    store.set(
        "test",
        PlatformCredentials {
            api_key: Some(Zeroizing::new("secret".to_string())),
            refresh_token: None,
            access_token: None,
            token_expires_at: None,
            client_id: None,
            client_secret: None,
            channel_id: None,
            channel_name: None,
        },
    );

    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let temp_path = temp_file.path();
    store
        .save_to_path("correct_pass", temp_path)
        .expect("save should succeed");

    let wrong_result = CredentialStore::load_from_path("wrong_pass", temp_path);
    assert!(wrong_result.is_err());
    assert!(
        matches!(wrong_result.unwrap_err(), UploadError::Encryption(_)),
        "wrong passphrase should return Encryption error"
    );
}
