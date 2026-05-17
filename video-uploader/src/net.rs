use crate::UploadError;
use rand::Rng;
use std::future::Future;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

pub fn build_http_client() -> reqwest::Client {
    build_http_client_with_timeout(30)
}

pub fn build_http_client_with_timeout(secs: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(secs))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("failed to build HTTP client — this should never happen in normal operation")
}

pub async fn retry<F, Fut, T>(operation: F, max_retries: u32) -> Result<T, UploadError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, UploadError>>,
{
    let mut last_error = None;

    for attempt in 0..max_retries {
        if attempt > 0 {
            let base_delay_secs = 2_u64.pow(attempt - 1) as f64;
            let jitter = base_delay_secs * 0.25;
            let delay_secs = base_delay_secs + rand::rng().random::<f64>() * 2.0 * jitter - jitter;
            let delay = Duration::from_secs_f64(delay_secs.max(0.0));
            tracing::warn!(
                "Retrying after {:?} (attempt {}/{})",
                delay,
                attempt + 1,
                max_retries
            );
            tokio::time::sleep(delay).await;
        }

        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if e.is_retryable() => {
                tracing::warn!("Retryable error: {}", e);
                last_error = Some(e);
            }
            Err(e) => return Err(e),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        UploadError::NoAttempts
    }))
}

pub fn is_private_ip(host: &str) -> bool {
    if host == "localhost" || host == "127.0.0.1" || host == "::1" {
        return true;
    }
    if let Some(ip_str) = host.strip_prefix("::ffff:")
        && let Ok(ip) = ip_str.parse::<IpAddr>()
    {
        return is_private_ip_addr(ip);
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        return is_private_ip_addr(ip);
    }
    false
}

fn ip_v4_from_v6segs(segments: [u16; 8]) -> Option<Ipv4Addr> {
    if segments[..6] != [0, 0, 0, 0, 0, 0xffff] {
        return None;
    }
    let b6 = segments[6].to_be_bytes();
    let b7 = segments[7].to_be_bytes();
    let combined = (u32::from(b6[0]) << 24)
        | (u32::from(b6[1]) << 16)
        | (u32::from(b7[0]) << 8)
        | u32::from(b7[1]);
    Some(Ipv4Addr::from(combined))
}

fn is_private_ip_addr(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            octets[0] == 0  // 0.0.0.0/8 — current network
                || (octets[0] == 100 && (64..=127).contains(&octets[1]))  // 100.64.0.0/10 — CGNAT
                || octets[0] == 10  // 10.0.0.0/8
                || (octets[0] == 172 && (16..=31).contains(&octets[1]))  // 172.16.0.0/12
                || (octets[0] == 192 && octets[1] == 168)  // 192.168.0.0/16
                || (octets[0] == 169 && octets[1] == 254)  // 169.254.0.0/16 — link-local
                || octets[0] == 127 // 127.0.0.0/8 — loopback
        }
        IpAddr::V6(v6) => {
            let segments = v6.segments();
            if segments == [0, 0, 0, 0, 0, 0, 0, 0] {
                return true; // :: — IPv6 unspecified, equivalent to 0.0.0.0
            }
            if let Some(v4) = ip_v4_from_v6segs(segments) {
                return is_private_ip_addr(IpAddr::V4(v4));
            }
            segments[0] & 0xffc0 == 0xfe80  // fe80::/10 — link-local
                || (segments[0] & 0xfe00 == 0xfc00) // fc00::/7 — unique local
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_private_v4() {
        assert!(is_private_ip("10.0.0.1"));
        assert!(is_private_ip("10.255.255.255"));
        assert!(is_private_ip("172.16.0.1"));
        assert!(is_private_ip("172.31.255.255"));
        assert!(is_private_ip("192.168.0.1"));
        assert!(is_private_ip("192.168.255.255"));
        assert!(is_private_ip("169.254.1.1"));
        assert!(is_private_ip("127.0.0.1"));
    }

    #[test]
    fn test_private_hostnames() {
        assert!(is_private_ip("localhost"));
        assert!(is_private_ip("::1"));
    }

    #[test]
    fn test_loopback_range() {
        assert!(is_private_ip("127.0.0.1"));
        assert!(is_private_ip("127.0.0.2"));
        assert!(is_private_ip("127.255.255.255"));
    }

    #[test]
    fn test_public_v4() {
        assert!(!is_private_ip("8.8.8.8"));
        assert!(!is_private_ip("1.1.1.1"));
        assert!(!is_private_ip("93.184.216.34"));
        assert!(!is_private_ip("100.0.0.1"));
    }

    #[test]
    fn test_non_ip_strings() {
        assert!(!is_private_ip("example.com"));
        assert!(!is_private_ip("google.com"));
        assert!(!is_private_ip("10.example.com"));
        assert!(!is_private_ip("framatube.org"));
    }

    #[test]
    fn test_private_v4_cgnat() {
        assert!(is_private_ip("100.64.0.1"));
        assert!(is_private_ip("100.127.255.255"));
        assert!(is_private_ip("100.100.0.1"));
        assert!(is_private_ip("100.64.0.0"));
    }

    #[test]
    fn test_private_v4_zero() {
        assert!(is_private_ip("0.0.0.0"));
        assert!(is_private_ip("0.255.255.255"));
        assert!(is_private_ip("0.0.0.1"));
    }

    #[test]
    fn test_private_ipv6_link_local() {
        assert!(is_private_ip("fe80::1"));
        assert!(is_private_ip("fe80::1:1"));
        assert!(is_private_ip("febf::1"));
    }

    #[test]
    fn test_private_ipv6_unique_local() {
        assert!(is_private_ip("fc00::1"));
        assert!(is_private_ip("fd00::1"));
    }

    #[test]
    fn test_private_ipv6_unspecified() {
        assert!(is_private_ip("::"));
    }

    #[test]
    fn test_public_ipv6() {
        assert!(!is_private_ip("2001:db8::1"));
        assert!(!is_private_ip("::ffff:8.8.8.8"));
    }

    #[test]
    fn test_ipv4_mapped_ipv6_private() {
        assert!(is_private_ip("::ffff:127.0.0.1"));
        assert!(is_private_ip("::ffff:10.0.0.1"));
        assert!(is_private_ip("::ffff:192.168.1.1"));
        assert!(is_private_ip("::ffff:172.16.0.1"));
    }

    #[test]
    fn test_ipv4_mapped_ipv6_public() {
        assert!(!is_private_ip("::ffff:8.8.8.8"));
        assert!(!is_private_ip("::ffff:93.184.216.34"));
    }

    #[test]
    fn test_ipv4_mapped_ipv6_cgnat() {
        assert!(is_private_ip("::ffff:100.64.0.1"));
        assert!(is_private_ip("::ffff:100.127.255.255"));
        assert!(is_private_ip("::ffff:100.64.0.0"));
    }
}
