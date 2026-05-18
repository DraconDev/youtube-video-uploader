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
use zeroize::Zeroize;
use zeroize::Zeroizing;

const CREDENTIALS_FILE: &str = "video-uploader/credentials.enc";
pub const NONCE_SIZE: usize = 12;
pub const SALT_SIZE: usize = 16;
const PBKDF2_ITERATIONS: u32 = 100_000;
pub const MIN_PASSPHRASE_LEN: usize = 8;

pub const FORMAT_MAGIC: &[u8] = b"VU";
pub const FORMAT_VERSION_V2: u8 = 0x02;

/// Per-workspace credential storage.
///
/// Each workspace holds a set of OAuth2 credentials for a single YouTube account.
/// Workspaces are stored in a single AES-256-GCM encrypted file (`credentials.enc`)
/// protected by a user passphrase with PBKDF2 key derivation (100K iterations).
///
/// # Workspace format (v0.2+)
///
/// ```toml
/// default_workspace = "youtube"
///
/// [workspaces.youtube]
/// refresh_token = "..."
/// client_id = "..."
/// ```
///
/// Old flat format (`[youtube]` at top level) is auto-migrated on load.
/// Per-workspace credential storage.
///
/// Each workspace holds a set of OAuth2 credentials for a single YouTube account.
/// Workspaces are stored in a single AES-256-GCM encrypted file (`credentials.enc`)
/// protected by a user passphrase with PBKDF2 key derivation (100K iterations).
///
/// # Workspace format (v0.2+)
///
/// ```toml
/// default_workspace = "youtube"
///
/// [workspaces.youtube]
/// refresh_token = "..."
/// client_id = "..."
/// ```
///
/// Old flat format (`[youtube]` at top level) is auto-migrated on load.
#[derive(Serialize, Deserialize, Default, Zeroize)]
pub struct PlatformCredentials {
    pub api_key: Option<Zeroizing<String>>,
    pub refresh_token: Option<Zeroizing<String>>,
    pub access_token: Option<Zeroizing<String>>,
    #[zeroize(skip)]
    pub token_expires_at: Option<u64>,
    pub client_id: Option<Zeroizing<String>>,
    pub client_secret: Option<Zeroizing<String>>,
    /// YouTube channel ID (fetched after auth via channels.list?mine=true).
    #[zeroize(skip)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    /// YouTube channel title (fetched after auth via channels.list?mine=true).
    #[zeroize(skip)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_name: Option<String>,
}

impl PlatformCredentials {
    /// Create credentials with the given OAuth2 fields.
    ///
    /// All string fields are wrapped in `Zeroizing<String>` for secure memory handling.
    /// Create credentials with the given OAuth2 fields.
    ///
    /// All string fields are wrapped in `Zeroizing<String>` for secure memory handling.
    /// `channel_id` and `channel_name` are left as `None` (fetched later via `fetch_channel_info`).
    pub fn new(
        refresh_token: Option<String>,
        access_token: Option<String>,
        client_id: Option<String>,
        client_secret: Option<String>,
    ) -> Self {
        Self {
            api_key: None,
            refresh_token: refresh_token.map(Zeroizing::new),
            access_token: access_token.map(Zeroizing::new),
            token_expires_at: None,
            client_id: client_id.map(Zeroizing::new),
            client_secret: client_secret.map(Zeroizing::new),
            channel_id: None,
            channel_name: None,
        }
    }
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
            .field("channel_id", &self.channel_id)
            .field("channel_name", &self.channel_name)
            .finish()
    }
}

impl Drop for PlatformCredentials {
    fn drop(&mut self) {
        self.zeroize();
    }
}

/// The encrypted on-disk credential store.
///
/// Workspaces are named sets of credentials (e.g. `"youtube"`, `"cooking-channel"`).
/// One workspace may be marked as the default.
///
/// # Usage
///
/// ```no_run
/// use video_uploader::CredentialStore;
///
/// let store = CredentialStore::load("my-passphrase").unwrap();
/// let creds = store.get("youtube").expect("youtube workspace not found");
/// println!("Refresh token: {:?}", creds.refresh_token);
/// ```
///
/// # Multi-channel setup
///
/// ```no_run
/// use video_uploader::config::PlatformCredentials;
/// use video_uploader::CredentialStore;
///
/// let mut store = CredentialStore::default();
/// store.set("gaming", PlatformCredentials::default());
/// store.set_default("gaming");
/// store.save("my-passphrase").unwrap();
/// ```
/// The encrypted on-disk credential store.
///
/// Workspaces are named sets of credentials (e.g. `"youtube"`, `"cooking-channel"`).
/// One workspace may be marked as the default.
///
/// # Usage
///
/// ```no_run
/// use video_uploader::CredentialStore;
///
/// let store = CredentialStore::load("my-passphrase").unwrap();
/// let creds = store.get("youtube").expect("youtube workspace not found");
/// println!("Refresh token: {:?}", creds.refresh_token);
/// ```
///
/// # Multi-channel setup
///
/// ```no_run
/// use video_uploader::config::PlatformCredentials;
/// use video_uploader::CredentialStore;
///
/// let mut store = CredentialStore::default();
/// store.set("gaming", PlatformCredentials::default());
/// store.set_default("gaming");
/// store.save("my-passphrase").unwrap();
/// ```
#[derive(Serialize, Deserialize, Default)]
pub struct CredentialStore {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    default_workspace: Option<String>,
    #[serde(default)]
    workspaces: HashMap<String, PlatformCredentials>,
}

impl std::fmt::Debug for CredentialStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CredentialStore")
            .field("default_workspace", &self.default_workspace)
            .field("workspaces", &self.workspaces.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Zeroize for CredentialStore {
    fn zeroize(&mut self) {
        // Clear drops each PlatformCredentials, which zeroizes via Drop
        self.workspaces.clear();
        self.default_workspace.zeroize();
    }
}

impl Drop for CredentialStore {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl CredentialStore {
    #[cfg(feature = "test-utils")]
    pub fn decrypt_store_for_testing(
        passphrase: &str,
        ciphertext: &[u8],
    ) -> Result<(Self, bool), UploadError> {
        Self::decrypt_store(passphrase, ciphertext)
    }

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

        let (plaintext, needs_encryption_migration) =
            if ciphertext.starts_with(FORMAT_MAGIC)
                && ciphertext.len() > FORMAT_MAGIC.len()
                && ciphertext[FORMAT_MAGIC.len()] == FORMAT_VERSION_V2
            {
                let plaintext =
                    String::from_utf8(Self::decrypt_v2(passphrase, ciphertext)?).map_err(|e| {
                        UploadError::Encryption(format!(
                            "Credentials file corrupted (invalid UTF-8): {}",
                            e
                        ))
                    })?;
                (plaintext, false)
            } else {
                let plaintext =
                    String::from_utf8(Self::decrypt_v1(passphrase, ciphertext)?).map_err(|e| {
                        UploadError::Encryption(format!(
                            "Credentials file corrupted (invalid UTF-8): {}",
                            e
                        ))
                    })?;
                (plaintext, true)
            };

        let (store, needs_format_migration) = Self::try_parse_toml(&plaintext)?;

        Ok((store, needs_encryption_migration || needs_format_migration))
    }

    /// Parse decrypted TOML, auto-detecting v0.1 (flat) vs v0.2 (workspace) format.
    fn try_parse_toml(plaintext: &str) -> Result<(Self, bool), UploadError> {
        let value: toml::Value = toml::from_str(plaintext)
            .map_err(|e| UploadError::Encryption(format!("Invalid TOML: {e}")))?;

        let table = value.as_table().ok_or_else(|| {
            UploadError::Encryption("Credential file is not a TOML table".into())
        })?;

        // Empty store
        if table.is_empty() {
            return Ok((Self::default(), false));
        }

        // New format indicators
        if table.contains_key("workspaces")
            || table.contains_key("default_workspace")
        {
            let store: CredentialStore = toml::from_str(plaintext).map_err(|e| {
                UploadError::Encryption(format!("Failed to parse credentials: {e}"))
            })?;
            return Ok((store, false));
        }

        // Try old flat format (v0.1): top-level keys are workspace names
        let old_map: HashMap<String, PlatformCredentials> = toml::from_str(plaintext)
            .map_err(|e| UploadError::Encryption(format!("Failed to parse credentials: {e}")))?;

        if old_map.is_empty() {
            return Ok((Self::default(), false));
        }

        let default = old_map
            .contains_key("youtube")
            .then(|| "youtube".to_string())
            .or_else(|| old_map.keys().next().cloned());

        tracing::warn!("Credential store migrated from v0.1 flat format to v0.2 workspace format");

        Ok((
            Self {
                default_workspace: default,
                workspaces: old_map,
            },
            true,
        ))
    }

    /// Load the credential store from the default path using a passphrase.
    ///
    /// Automatically detects and migrates v0.1 (flat) format to v0.2 (workspace) format.
    /// Re-encrypts the file if migration was needed.
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
            tracing::warn!("Credential store migrated to latest format. Re-encrypting.");
            store.save(passphrase)?;
        }

        Ok(store)
    }

    /// Load the credential store from a specific path using a passphrase.
    ///
    /// Useful for testing or custom credential file locations.
    pub fn load_from_path(passphrase: &str, path: &std::path::Path) -> Result<Self, UploadError> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let mut file = fs::File::open(path).map_err(UploadError::Io)?;
        let mut ciphertext = Vec::new();
        file.read_to_end(&mut ciphertext).map_err(UploadError::Io)?;

        let (store, needs_migration) = Self::decrypt_store(passphrase, &ciphertext)?;

        if needs_migration {
            tracing::warn!("Credential store migrated to latest format. Re-encrypting.");
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

    /// Save the credential store to the default path, encrypting with the passphrase.
    pub fn save(&self, passphrase: &str) -> Result<(), UploadError> {
        let path = Self::path()?;
        self.save_to_path(passphrase, &path)
    }

    /// Save the credential store to a specific path, encrypting with the passphrase.
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

    /// Get a reference to the credentials for a workspace.
    pub fn get(&self, name: &str) -> Option<&PlatformCredentials> {
        self.workspaces.get(name)
    }

    /// Get a mutable reference to the credentials for a workspace.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut PlatformCredentials> {
        self.workspaces.get_mut(name)
    }

    /// Set (insert or replace) credentials for a workspace.
    pub fn set(&mut self, name: impl Into<String>, creds: PlatformCredentials) {
        self.workspaces.insert(name.into(), creds);
    }

    /// Remove a workspace's credentials, returning them if they existed.
    pub fn remove(&mut self, name: &str) -> Option<PlatformCredentials> {
        self.workspaces.remove(name)
    }

    /// Iterate over all workspace names.
    pub fn workspaces(&self) -> impl Iterator<Item = &String> {
        self.workspaces.keys()
    }

    /// Get the name of the default workspace, if one is set.
    pub fn default_workspace(&self) -> Option<&str> {
        self.default_workspace.as_deref()
    }

    /// Clear the default workspace marker.
    pub fn clear_default(&mut self) {
        self.default_workspace = None;
    }

    /// Set the default workspace by name.
    pub fn set_default(&mut self, name: &str) {
        self.default_workspace = Some(name.to_string());
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
        assert!(creds.channel_id.is_none());
        assert!(creds.channel_name.is_none());
    }

    #[test]
    fn test_credential_store_roundtrip() {
        let mut store = CredentialStore::default();
        store.set(
            "test-workspace",
            PlatformCredentials::new(None, None, None, None),
        );
        store.get_mut("test-workspace").unwrap().api_key = Some(Zeroizing::new("secret-key".to_string()));

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
            loaded.get("test-workspace").unwrap().api_key.as_ref().map(|z| z.as_str()),
            Some("secret-key")
        );

        let _ = fs::remove_file(&temp_path);
    }

    #[test]
    fn test_credential_store_default_workspace() {
        let mut store = CredentialStore::default();
        store.set("alpha", PlatformCredentials::default());
        store.set("beta", PlatformCredentials::default());

        assert!(store.default_workspace().is_none());
        store.set_default("alpha");
        assert_eq!(store.default_workspace(), Some("alpha"));
    }

    #[test]
    fn test_credential_store_multi_workspace() {
        let mut store = CredentialStore::default();
        store.set(
            "youtube",
            PlatformCredentials::new(Some("yt-token".to_string()), None, None, None),
        );
        store.set(
            "cooking",
            PlatformCredentials::new(Some("cook-token".to_string()), None, None, None),
        );

        let names: Vec<_> = store.workspaces().collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&&"youtube".to_string()));
        assert!(names.contains(&&"cooking".to_string()));

        assert_eq!(
            store.get("youtube").unwrap().refresh_token.as_ref().map(|z| z.as_str()),
            Some("yt-token")
        );
        assert_eq!(
            store.get("cooking").unwrap().refresh_token.as_ref().map(|z| z.as_str()),
            Some("cook-token")
        );
        assert!(store.get("missing").is_none());
    }

    #[test]
    fn test_credential_store_v01_to_v02_format_migration() {
        // Build a v0.1 flat-format encrypted file
        #[allow(deprecated)]
        let key = {
            let passphrase = "migration-test-pass";
            derive_key_legacy(passphrase)
        };
        let cipher = Aes256Gcm::new(&key);
        let nonce_bytes: [u8; NONCE_SIZE] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut old_store = CredentialStore::default();
        old_store.set(
            "youtube",
            PlatformCredentials::new(
                Some("test-refresh".to_string()),
                None,
                Some("test-client".to_string()),
                Some("test-secret".to_string()),
            ),
        );
        old_store.get_mut("youtube").unwrap().api_key = Some(Zeroizing::new("test-api-key".to_string()));

        // Serialize the OLD way (flat HashMap without workspaces wrapper)
        let old_plaintext = toml::to_string(&old_store.workspaces).unwrap();
        let ciphertext = cipher.encrypt(nonce, old_plaintext.as_bytes()).unwrap();

        let mut v1_data = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        v1_data.extend_from_slice(&nonce_bytes);
        v1_data.extend_from_slice(&ciphertext);

        let temp_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), &v1_data).unwrap();

        #[allow(deprecated)]
        let loaded =
            CredentialStore::load_from_path("migration-test-pass", temp_file.path()).unwrap();

        // Should have migrated to workspace format
        assert_eq!(loaded.default_workspace(), Some("youtube"));
        let creds = loaded
            .get("youtube")
            .expect("youtube workspace should be migrated");
        assert_eq!(creds.api_key.as_ref().map(|z| z.as_str()), Some("test-api-key"));
        assert_eq!(creds.refresh_token.as_ref().map(|z| z.as_str()), Some("test-refresh"));
        assert_eq!(creds.client_id.as_ref().map(|z| z.as_str()), Some("test-client"));

        let _ = fs::remove_file(temp_file.path());
    }

    #[test]
    fn test_credential_store_v1_encryption_v01_format_migration() {
        // V1 encryption + v0.1 flat format — both migrations at once
        #[allow(deprecated)]
        let key = {
            let passphrase = "combo-test-pass";
            derive_key_legacy(passphrase)
        };
        let cipher = Aes256Gcm::new(&key);
        let nonce_bytes: [u8; NONCE_SIZE] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut old_store = CredentialStore::default();
        old_store.set(
            "youtube",
            PlatformCredentials::new(Some("combo-refresh".to_string()), None, None, None),
        );

        let old_plaintext = toml::to_string(&old_store.workspaces).unwrap();
        let ciphertext = cipher.encrypt(nonce, old_plaintext.as_bytes()).unwrap();

        let mut v1_data = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        v1_data.extend_from_slice(&nonce_bytes);
        v1_data.extend_from_slice(&ciphertext);

        let temp_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), &v1_data).unwrap();

        #[allow(deprecated)]
        let loaded =
            CredentialStore::load_from_path("combo-test-pass", temp_file.path()).unwrap();

        assert_eq!(loaded.default_workspace(), Some("youtube"));
        assert_eq!(
            loaded.get("youtube").unwrap().refresh_token.as_ref().map(|z| z.as_str()),
            Some("combo-refresh")
        );

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
    fn test_credential_store_corrupted_file_too_short() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"VU\x02short").unwrap();
        let result = CredentialStore::load_from_path("anypass", temp_file.path());
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UploadError::Encryption(_)));
    }

    #[test]
    fn test_credential_store_corrupted_file_garbage_data() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            temp_file.path(),
            b"VU\x02garbage_data_that_is_not_valid_encrypted_content",
        )
        .unwrap();
        let result = CredentialStore::load_from_path("anypass", temp_file.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, UploadError::Encryption(_)));
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
    #[allow(deprecated)]
    fn test_derive_key_legacy_deterministic() {
        let key1 = derive_key_legacy("same-passphrase");
        let key2 = derive_key_legacy("same-passphrase");
        assert_eq!(key1.as_slice(), key2.as_slice());
    }

    #[test]
    #[allow(deprecated)]
    fn test_pbkdf2_vs_legacy_different() {
        let salt = [0u8; SALT_SIZE];
        let pbkdf2_key = derive_key_pbkdf2("test", &salt);
        let legacy_key = derive_key_legacy("test");
        assert_ne!(pbkdf2_key.as_slice(), legacy_key.as_slice());
    }
}
