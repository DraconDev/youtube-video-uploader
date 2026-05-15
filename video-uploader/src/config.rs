use crate::UploadError;
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use zeroize::{Zeroize, ZeroizeOnDrop};

const CREDENTIALS_FILE: &str = "video-uploader/credentials.enc";
pub const NONCE_SIZE: usize = 12;
pub const SALT_SIZE: usize = 16;
const PBKDF2_ITERATIONS: u32 = 100_000;
pub const MIN_PASSPHRASE_LEN: usize = 8;

pub const FORMAT_MAGIC: &[u8] = b"VU";
pub const FORMAT_VERSION_V2: u8 = 0x02;

/// Per-platform credential storage.
#[derive(Clone, Serialize, Deserialize, Default, Zeroize, ZeroizeOnDrop)]
pub struct PlatformCredentials {
    pub api_key: Option<String>,
    pub refresh_token: Option<String>,
    pub access_token: Option<String>,
    #[zeroize(skip)]
    pub token_expires_at: Option<u64>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    #[zeroize(skip)]
    pub daemon_url: Option<String>,
}

impl std::fmt::Debug for PlatformCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformCredentials")
            .field("api_key", &self.api_key.as_ref().map(|_| "[REDACTED]"))
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "[REDACTED]"),
            )
            .field(
                "access_token",
                &self.access_token.as_ref().map(|_| "[REDACTED]"),
            )
            .field("token_expires_at", &self.token_expires_at)
            .field("client_id", &self.client_id.as_ref().map(|_| "[REDACTED]"))
            .field(
                "client_secret",
                &self.client_secret.as_ref().map(|_| "[REDACTED]"),
            )
            .field("daemon_url", &self.daemon_url)
            .finish()
    }
}

/// The encrypted on-disk credential store.
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct CredentialStore {
    #[serde(flatten)]
    platforms: HashMap<String, PlatformCredentials>,
}

impl std::fmt::Debug for CredentialStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialStore")
            .field("platforms", &self.platforms.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl CredentialStore {
    fn decrypt_store(passphrase: &str, ciphertext: &[u8]) -> Result<(Self, bool), UploadError> {
        if passphrase.len() < MIN_PASSPHRASE_LEN {
            return Err(UploadError::Encryption(format!(
                "Passphrase must be at least {} characters",
                MIN_PASSPHRASE_LEN
            )));
        }
        if ciphertext.len() < NONCE_SIZE {
            return Err(UploadError::Encryption(
                "Credential file appears corrupted or tampered with (too short)".into(),
            ));
        }

        let (store, needs_migration) = if ciphertext.starts_with(FORMAT_MAGIC)
            && ciphertext.len() > FORMAT_MAGIC.len()
            && ciphertext[FORMAT_MAGIC.len()] == FORMAT_VERSION_V2
        {
            let plaintext = String::from_utf8(Self::decrypt_v2(passphrase, ciphertext)?)
                .map_err(|e| {
                    UploadError::Encryption(format!(
                        "Credentials file corrupted (invalid UTF-8): {}",
                        e
                    ))
                })?;
            let store: CredentialStore = toml::from_str(&plaintext)
                .map_err(|e| {
                    UploadError::Encryption(format!("Failed to parse credentials: {e}"))
                })?;
            (store, false)
        } else {
            let plaintext = String::from_utf8(Self::decrypt_v1(passphrase, ciphertext)?)
                .map_err(|e| {
                    UploadError::Encryption(format!(
                        "Credentials file corrupted (invalid UTF-8): {}",
                        e
                    ))
                })?;
            let store: CredentialStore = toml::from_str(&plaintext)
                .map_err(|e| {
                    UploadError::Encryption(format!("Failed to parse credentials: {e}"))
                })?;
            (store, true)
        };

        Ok((store, needs_migration))
    }

    pub fn load(passphrase: &str) -> Result<Self, UploadError> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let mut file = fs::File::open(&path).map_err(UploadError::Io)?;
        let mut ciphertext = Vec::new();
        file.read_to_end(&mut ciphertext).map_err(UploadError::Io)?;

        let (store, needs_migration) = Self::decrypt_store(passphrase, &ciphertext)?;

        if needs_migration {
            tracing::warn!(
                "Credentials are in legacy V1 format. Re-encrypting as V2 for improved security."
            );
            store.save(passphrase)?;
        }

        Ok(store)
    }

    pub fn load_from_path(passphrase: &str, path: &std::path::Path) -> Result<Self, UploadError> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let mut file = fs::File::open(path).map_err(UploadError::Io)?;
        let mut ciphertext = Vec::new();
        file.read_to_end(&mut ciphertext).map_err(UploadError::Io)?;

        let (store, needs_migration) = Self::decrypt_store(passphrase, &ciphertext)?;

        if needs_migration {
            tracing::warn!(
                "Credentials are in legacy V1 format. Re-encrypting as V2 for improved security."
            );
            store.save_to_path(passphrase, path)?;
        }

        Ok(store)
    }

    fn decrypt_v2(passphrase: &str, data: &[u8]) -> Result<Vec<u8>, UploadError> {
        // Format: "VU" [0x02] [salt(16)] [nonce(12)] [ciphertext]
        let header_len = FORMAT_MAGIC.len() + 1 + SALT_SIZE + NONCE_SIZE;
        if data.len() < header_len {
            return Err(UploadError::Encryption(
                "Credential file too short for v2 format".into(),
            ));
        }

        let salt = &data[FORMAT_MAGIC.len() + 1..FORMAT_MAGIC.len() + 1 + SALT_SIZE];
        let nonce = Nonce::from_slice(
            &data[FORMAT_MAGIC.len() + 1 + SALT_SIZE
                ..FORMAT_MAGIC.len() + 1 + SALT_SIZE + NONCE_SIZE],
        );
        let encrypted = &data[header_len..];

        let key = derive_key_pbkdf2(passphrase, salt);
        let cipher = Aes256Gcm::new(&key);
        cipher
            .decrypt(nonce, encrypted)
            .map_err(|e| UploadError::Encryption(format!("Decrypt failed: {e}")))
    }

    #[allow(deprecated)]
    fn decrypt_v1(passphrase: &str, data: &[u8]) -> Result<Vec<u8>, UploadError> {
        // Legacy format: [nonce(12)] [ciphertext]
        let nonce = Nonce::from_slice(&data[..NONCE_SIZE]);
        let encrypted = &data[NONCE_SIZE..];

        let key = derive_key_legacy(passphrase);
        let cipher = Aes256Gcm::new(&key);
        cipher
            .decrypt(nonce, encrypted)
            .map_err(|e| UploadError::Encryption(format!("Decrypt failed: {e}")))
    }

    pub fn save(&self, passphrase: &str) -> Result<(), UploadError> {
        let path = Self::path()?;
        self.save_to_path(passphrase, &path)
    }

    pub fn save_to_path(
        &self,
        passphrase: &str,
        path: &std::path::Path,
    ) -> Result<(), UploadError> {
        if passphrase.len() < MIN_PASSPHRASE_LEN {
            return Err(UploadError::Encryption(format!(
                "Passphrase must be at least {} characters",
                MIN_PASSPHRASE_LEN
            )));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(UploadError::Io)?;
        }

        let plaintext = toml::to_string(self).map_err(|e| {
            UploadError::Encryption(format!("Failed to serialize credentials: {e}"))
        })?;

        let salt: [u8; SALT_SIZE] = rand::random();
        let key = derive_key_pbkdf2(passphrase, &salt);
        let cipher = Aes256Gcm::new(&key);
        let nonce_bytes: [u8; NONCE_SIZE] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| UploadError::Encryption(format!("Encrypt failed: {e}")))?;

        let mut file = fs::File::create(path).map_err(UploadError::Io)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o600);
            file.set_permissions(permissions).map_err(UploadError::Io)?;
        }
        #[cfg(windows)]
        {
            tracing::warn!(
                "Credential file permissions not enforced on Windows. \
                Ensure the file is protected by NTFS ACLs (restrict to current user)."
            );
        }

        // Write v2 format: "VU" [0x02] [salt] [nonce] [ciphertext]
        file.write_all(FORMAT_MAGIC).map_err(UploadError::Io)?;
        file.write_all(&[FORMAT_VERSION_V2])
            .map_err(UploadError::Io)?;
        file.write_all(&salt).map_err(UploadError::Io)?;
        file.write_all(&nonce_bytes).map_err(UploadError::Io)?;
        file.write_all(&ciphertext).map_err(UploadError::Io)?;
        file.sync_all().map_err(UploadError::Io)?;

        Ok(())
    }

    pub fn get(&self, platform: &str) -> Option<&PlatformCredentials> {
        self.platforms.get(platform)
    }

    pub fn get_mut(&mut self, platform: &str) -> Option<&mut PlatformCredentials> {
        self.platforms.get_mut(platform)
    }

    pub fn set(&mut self, platform: impl Into<String>, creds: PlatformCredentials) {
        self.platforms.insert(platform.into(), creds);
    }

    pub fn remove(&mut self, platform: &str) -> Option<PlatformCredentials> {
        self.platforms.remove(platform)
    }

    pub fn platforms(&self) -> impl Iterator<Item = &String> {
        self.platforms.keys()
    }

    fn path() -> Result<PathBuf, UploadError> {
        dirs::config_dir()
            .ok_or_else(|| UploadError::Config("No config directory found".into()))
            .map(|d| d.join(CREDENTIALS_FILE))
    }
}

/// Derives an AES-256-GCM key from a passphrase using PBKDF2 with SHA-256.
///
/// # Security
/// This function zeroizes the key buffer after extracting the key.
fn derive_key_pbkdf2(passphrase: &str, salt: &[u8]) -> aes_gcm::Key<Aes256Gcm> {
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(passphrase.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
    let result = *aes_gcm::Key::<Aes256Gcm>::from_slice(&key);
    key.zeroize();
    result
}

/// Derives an AES-256-GCM key from a passphrase using SHA-256 digest (V1 format).
///
/// # Security
/// **DO NOT USE for new encryption.** This function exists only to decrypt legacy
/// V1 credential files that used a simple SHA-256-based key derivation with no salt
/// and no iterations. New credentials always use `derive_key_pbkdf2` with a random
/// salt and 100,000 PBKDF2 iterations. If you accidentally use this for new data,
/// credentials will be incompatible with the V2 format and users will lose access
/// to their stored tokens.
#[deprecated(
    since = "0.1.0",
    note = "legacy V1 decryption only — use derive_key_pbkdf2 for new encryption"
)]
fn derive_key_legacy(passphrase: &str) -> aes_gcm::Key<Aes256Gcm> {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(passphrase.as_bytes());
    let result = *aes_gcm::Key::<Aes256Gcm>::from_slice(&hash);
    let mut key_bytes: [u8; 32] = hash.into();
    key_bytes.zeroize();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_credentials_default() {
        let creds = PlatformCredentials::default();
        assert!(creds.api_key.is_none());
        assert!(creds.refresh_token.is_none());
        assert!(creds.access_token.is_none());
        assert!(creds.token_expires_at.is_none());
        assert!(creds.client_id.is_none());
        assert!(creds.client_secret.is_none());
    }

    #[test]
    fn test_credential_store_roundtrip() {
        let mut store = CredentialStore::default();
        store.set(
            "test-platform",
            PlatformCredentials {
                api_key: Some("secret-key".into()),
                refresh_token: None,
                access_token: None,
                token_expires_at: None,
                client_id: None,
                client_secret: None,
                daemon_url: None,
            },
        );

        let passphrase = "test-passphrase";

        // Save to a temp file
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();
        {
            let mut file = fs::File::create(&temp_path).unwrap();
            let salt: [u8; SALT_SIZE] = rand::random();
            let key = derive_key_pbkdf2(passphrase, &salt);
            let cipher = Aes256Gcm::new(&key);
            let nonce_bytes: [u8; NONCE_SIZE] = rand::random();
            let nonce = Nonce::from_slice(&nonce_bytes);
            let plaintext = toml::to_string(&store).unwrap();
            let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes()).unwrap();
            file.write_all(FORMAT_MAGIC).unwrap();
            file.write_all(&[FORMAT_VERSION_V2]).unwrap();
            file.write_all(&salt).unwrap();
            file.write_all(&nonce_bytes).unwrap();
            file.write_all(&ciphertext).unwrap();
        }

        // Read back using load_from_path
        let loaded = CredentialStore::load_from_path(passphrase, &temp_path).unwrap();
        assert_eq!(
            loaded.get("test-platform").unwrap().api_key.as_deref(),
            Some("secret-key")
        );

        let _ = fs::remove_file(&temp_path);
    }

    #[test]
    fn test_derive_key_pbkdf2_deterministic() {
        let passphrase = "test-passphrase";
        let salt = [0u8; SALT_SIZE];
        let key1 = derive_key_pbkdf2(passphrase, &salt);
        let key2 = derive_key_pbkdf2(passphrase, &salt);
        assert_eq!(key1.as_slice(), key2.as_slice());

        let salt2 = [1u8; SALT_SIZE];
        let key3 = derive_key_pbkdf2(passphrase, &salt2);
        assert_ne!(key1.as_slice(), key3.as_slice());

        let key4 = derive_key_pbkdf2("different-passphrase", &salt);
        assert_ne!(key1.as_slice(), key4.as_slice());
    }

    #[test]
    fn test_credential_store_v1_to_v2_auto_migration() {
        #[allow(deprecated)]
        let key = {
            let passphrase = "migration-test-pass";
            derive_key_legacy(passphrase)
        };
        let cipher = Aes256Gcm::new(&key);
        let nonce_bytes: [u8; NONCE_SIZE] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut store = CredentialStore::default();
        store.set(
            "youtube",
            PlatformCredentials {
                api_key: Some("test-api-key".into()),
                refresh_token: Some("test-refresh".into()),
                access_token: None,
                token_expires_at: None,
                client_id: Some("test-client".into()),
                client_secret: Some("test-secret".into()),
                daemon_url: None,
            },
        );

        let plaintext = toml::to_string(&store).unwrap();
        let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes()).unwrap();

        let mut v1_data = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        v1_data.extend_from_slice(&nonce_bytes);
        v1_data.extend_from_slice(&ciphertext);

        let temp_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), &v1_data).unwrap();

        #[allow(deprecated)]
        let loaded =
            CredentialStore::load_from_path("migration-test-pass", temp_file.path()).unwrap();

        let creds = loaded
            .get("youtube")
            .expect("youtube platform should be migrated");
        assert_eq!(creds.api_key.as_deref(), Some("test-api-key"));
        assert_eq!(creds.refresh_token.as_deref(), Some("test-refresh"));
        assert_eq!(creds.client_id.as_deref(), Some("test-client"));

        let _ = fs::remove_file(temp_file.path());
    }

    #[test]
    fn test_credential_store_v1_wrong_passphrase() {
        #[allow(deprecated)]
        let key = {
            let passphrase = "correct-pass";
            derive_key_legacy(passphrase)
        };
        let cipher = Aes256Gcm::new(&key);
        let nonce_bytes: [u8; NONCE_SIZE] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut store = CredentialStore::default();
        store.set("test", PlatformCredentials::default());

        let plaintext = toml::to_string(&store).unwrap();
        let ciphertext = cipher.encrypt(nonce, plaintext.as_bytes()).unwrap();

        let mut v1_data = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        v1_data.extend_from_slice(&nonce_bytes);
        v1_data.extend_from_slice(&ciphertext);

        let temp_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), &v1_data).unwrap();

        let result = CredentialStore::load_from_path("wrong-passphrase", temp_file.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UploadError::Encryption(_)));

        let _ = fs::remove_file(temp_file.path());
    }

    #[test]
    fn test_derive_key_legacy_deterministic() {
        let key1 = derive_key_legacy("same-passphrase");
        let key2 = derive_key_legacy("same-passphrase");
        assert_eq!(key1.as_slice(), key2.as_slice());
    }

    #[test]
    fn test_pbkdf2_vs_legacy_different() {
        let salt = [0u8; SALT_SIZE];
        let pbkdf2_key = derive_key_pbkdf2("test", &salt);
        let legacy_key = derive_key_legacy("test");
        assert_ne!(pbkdf2_key.as_slice(), legacy_key.as_slice());
    }
}
