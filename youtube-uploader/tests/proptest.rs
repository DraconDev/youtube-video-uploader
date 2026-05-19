//! Property-based tests using proptest.
//!
//! These verify invariants across a wide range of random inputs.

use proptest::prelude::*;

fn is_private_ip_reference(ip_str: &str) -> bool {
    use std::net::IpAddr;
    if ip_str == "localhost" || ip_str == "127.0.0.1" || ip_str == "::1" {
        return true;
    }
    let ip_str = if let Some(stripped) = ip_str.strip_prefix("::ffff:") {
        stripped
    } else {
        ip_str
    };
    if let Ok(ip) = ip_str.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(v4) => {
                let octets = v4.octets();
                // Loopback 127.0.0.0/8
                if octets[0] == 127 {
                    return true;
                }
                // Private 10.0.0.0/8
                if octets[0] == 10 {
                    return true;
                }
                // Private 172.16.0.0/12
                if octets[0] == 172 && (16..=31).contains(&octets[1]) {
                    return true;
                }
                // Private 192.168.0.0/16
                if octets[0] == 192 && octets[1] == 168 {
                    return true;
                }
                // 0.0.0.0/8 current network
                if octets[0] == 0 {
                    return true;
                }
                // 100.64.0.0/10 CGNAT
                if octets[0] == 100 && (64..=127).contains(&octets[1]) {
                    return true;
                }
                false
            }
            IpAddr::V6(v6) => {
                // Unspecified ::
                if v6.is_unspecified() {
                    return true;
                }
                // Loopback ::1
                if v6.is_loopback() {
                    return true;
                }
                // Link-local fe80::/10
                if v6.segments()[0] & 0xffc0 == 0xfe80 {
                    return true;
                }
                // Unique local fc00::/7
                let first = v6.segments()[0];
                if first & 0xfe00 == 0xfc00 {
                    return true;
                }
                false
            }
        }
    } else {
        false
    }
}

proptest! {
    #[test]
    fn proptest_is_private_ip_matches_reference(ip in "\\PC*") {
        // Filter to avoid testing every possible string (too many)
        let result = youtube_uploader::is_private_ip(&ip);
        let expected = is_private_ip_reference(&ip);
        prop_assert_eq!(
            result, expected,
            "is_private_ip({:?}) = {}, expected {}",
            ip, result, expected
        );
    }
}

#[test]
fn proptest_is_private_ip_public_ips_rejected() {
    let public_ips = [
        "8.8.8.8",
        "1.1.1.1",
        "142.250.80.46",
        "151.101.1.140",
        "54.239.28.85",
        "185.199.108.153",
        "140.82.112.4",
    ];
    for ip in public_ips {
        assert!(
            !youtube_uploader::is_private_ip(ip),
            "public IP {} should be rejected",
            ip
        );
    }
}

#[test]
fn proptest_is_private_ip_private_ips_accepted() {
    let private_ips = [
        ("127.0.0.1", true),
        ("127.0.0.2", true),
        ("127.255.255.255", true),
        ("10.0.0.1", true),
        ("10.255.255.255", true),
        ("172.16.0.1", true),
        ("172.31.255.255", true),
        ("192.168.0.1", true),
        ("192.168.255.255", true),
        ("::1", true),
        ("::", true),
        ("fe80::1", true),
        ("fc00::1", true),
        ("::ffff:127.0.0.1", true),
        ("::ffff:8.8.8.8", false), // IPv4-mapped of public
        ("0.0.0.1", true),         // 0.0.0.0/8
        ("100.64.0.1", true),      // CGNAT
        ("localhost", true),
    ];
    for (ip, expected) in private_ips {
        assert_eq!(
            youtube_uploader::is_private_ip(ip),
            expected,
            "is_private_ip({}) = {}, expected {}",
            ip,
            youtube_uploader::is_private_ip(ip),
            expected
        );
    }
}

// PKCE tests removed: device code flow does not use PKCE.
// Google's device code flow rejects code_verifier in the token exchange.
// PKCE is still correctly used in the auth_code (browser) flow.

#[test]
fn proptest_credential_store_roundtrip_random_data() {
    use youtube_uploader::UploadError;
    use youtube_uploader::config::{CredentialStore, PlatformCredentials};

    // Create a store with random data
    let mut store = CredentialStore::default();
    store.set(
        "youtube",
        PlatformCredentials::new(
            Some("test_refresh_token_123".to_string()),
            Some("test_access_token_456".to_string()),
            Some("client_id_abc".to_string()),
            Some("client_secret_xyz".to_string()),
        ),
    );
    store.get_mut("youtube").unwrap().token_expires_at = Some(1234567890);

    let passphrase = "TestPassphrase123";

    // Save to a temp file to avoid overwriting real credentials
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    let temp_path = temp_file.path();
    store.save_to_path(passphrase, temp_path).unwrap();

    // Load back from the same temp path
    let loaded = CredentialStore::load_from_path(passphrase, temp_path).unwrap();

    assert_eq!(
        loaded
            .get("youtube")
            .unwrap()
            .refresh_token
            .as_ref()
            .map(|z| z.as_str()),
        Some("test_refresh_token_123")
    );

    // Wrong passphrase should fail
    let wrong_load = CredentialStore::load_from_path("WrongPassphrase999", temp_path);
    assert!(wrong_load.is_err());
    let err = wrong_load.unwrap_err();
    assert!(
        matches!(err, UploadError::Encryption(_)),
        "expected Encryption error, got: {:?}",
        err
    );
}
