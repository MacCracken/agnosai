//! Server-Side Request Forgery (SSRF) protection utilities.
//!
//! Validates URLs to prevent requests to private networks, loopback addresses,
//! cloud metadata endpoints, and other internal resources.

/// Check whether a URL is safe for outbound HTTP requests.
///
/// Rejects:
/// - Non-HTTP(S) schemes
/// - Localhost and common internal hostnames (`.local`, `.internal`, `.localhost`)
/// - Private IPv4 ranges (10.x, 172.16-31.x, 192.168.x, 127.x, 169.254.x, 0.x)
/// - Private IPv6 ranges (fc00::/7, fe80::/10, ::1, ::ffff:private)
/// - Cloud metadata IPs (169.254.169.254)
#[must_use]
pub fn is_safe_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };

    // Only allow HTTP(S).
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return false;
    }

    let Some(host) = parsed.host_str() else {
        return false;
    };

    // Block localhost and common internal names.
    let lower = host.to_ascii_lowercase();
    if lower == "localhost"
        || lower == "127.0.0.1"
        || lower == "::1"
        || lower == "[::1]"
        || lower.ends_with(".local")
        || lower.ends_with(".internal")
        || lower.ends_with(".localhost")
    {
        return false;
    }

    // Block private/link-local IP ranges. Use parsed.host() to correctly
    // handle both IPv4 and bracketed IPv6 addresses.
    match parsed.host() {
        Some(url::Host::Ipv4(v4)) => {
            if is_private_ipv4(v4) {
                return false;
            }
        }
        Some(url::Host::Ipv6(v6)) => {
            if is_private_ip(std::net::IpAddr::V6(v6)) {
                return false;
            }
        }
        _ => {}
    }

    true
}

/// Check whether an IP address is in a private, loopback, or link-local range.
///
/// Covers IPv4 private ranges, IPv6 private/link-local ranges, and
/// IPv6-mapped IPv4 addresses (e.g. `::ffff:10.0.0.1`).
#[must_use]
pub fn is_private_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => is_private_ipv4(v4),
        std::net::IpAddr::V6(v6) => {
            if v6.is_loopback() {
                return true;
            }
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_private_ipv4(mapped);
            }
            let segments = v6.segments();
            // fc00::/7 — unique local addresses
            if segments[0] & 0xfe00 == 0xfc00 {
                return true;
            }
            // fe80::/10 — link-local
            if segments[0] & 0xffc0 == 0xfe80 {
                return true;
            }
            false
        }
    }
}

/// Check whether an IPv4 address is private, loopback, link-local, or metadata.
#[inline]
#[must_use]
pub fn is_private_ipv4(v4: std::net::Ipv4Addr) -> bool {
    v4.is_private()
        || v4.is_loopback()
        || v4.is_link_local()
        || (v4.octets()[0] == 169 && v4.octets()[1] == 254) // metadata
        || v4.is_unspecified()                                // 0.0.0.0
        || (v4.octets()[0] == 0) // 0.0.0.0/8
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    // ── is_safe_url ──────────────────────────────────────────────

    #[test]
    fn safe_url_accepts_public_https() {
        assert!(is_safe_url("https://example.com"));
        assert!(is_safe_url("https://example.com/path?q=1"));
    }

    #[test]
    fn safe_url_accepts_public_http() {
        assert!(is_safe_url("http://example.com"));
    }

    #[test]
    fn safe_url_rejects_non_http_schemes() {
        assert!(!is_safe_url("ftp://example.com"));
        assert!(!is_safe_url("file:///etc/passwd"));
        assert!(!is_safe_url("gopher://example.com"));
        assert!(!is_safe_url("ssh://example.com"));
    }

    #[test]
    fn safe_url_rejects_invalid_url() {
        assert!(!is_safe_url("not a url"));
        assert!(!is_safe_url(""));
    }

    #[test]
    fn safe_url_rejects_localhost() {
        assert!(!is_safe_url("http://localhost"));
        assert!(!is_safe_url("http://localhost:8080"));
        assert!(!is_safe_url("http://127.0.0.1"));
        assert!(!is_safe_url("http://127.0.0.1:3000"));
    }

    #[test]
    fn safe_url_rejects_ipv6_loopback() {
        assert!(!is_safe_url("http://[::1]"));
        assert!(!is_safe_url("http://[::1]:8080"));
    }

    #[test]
    fn safe_url_rejects_internal_hostnames() {
        assert!(!is_safe_url("http://myservice.local"));
        assert!(!is_safe_url("http://db.internal"));
        assert!(!is_safe_url("http://app.localhost"));
    }

    #[test]
    fn safe_url_rejects_private_ipv4_10() {
        assert!(!is_safe_url("http://10.0.0.1"));
        assert!(!is_safe_url("http://10.255.255.255"));
    }

    #[test]
    fn safe_url_rejects_private_ipv4_172() {
        assert!(!is_safe_url("http://172.16.0.1"));
        assert!(!is_safe_url("http://172.31.255.255"));
    }

    #[test]
    fn safe_url_rejects_private_ipv4_192() {
        assert!(!is_safe_url("http://192.168.0.1"));
        assert!(!is_safe_url("http://192.168.255.255"));
    }

    #[test]
    fn safe_url_rejects_metadata_ip() {
        assert!(!is_safe_url("http://169.254.169.254"));
        assert!(!is_safe_url("http://169.254.169.254/latest/meta-data"));
    }

    #[test]
    fn safe_url_rejects_zero_network() {
        assert!(!is_safe_url("http://0.0.0.0"));
        assert!(!is_safe_url("http://0.1.2.3"));
    }

    #[test]
    fn safe_url_rejects_private_ipv6() {
        // unique local fc00::/7
        assert!(!is_safe_url("http://[fc00::1]"));
        assert!(!is_safe_url("http://[fd12::1]"));
        // link-local fe80::/10
        assert!(!is_safe_url("http://[fe80::1]"));
    }

    // ── is_private_ip ────────────────────────────────────────────

    #[test]
    fn private_ip_ipv4_ranges() {
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))));
        assert!(is_private_ip(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))));
    }

    #[test]
    fn private_ip_ipv6_loopback() {
        assert!(is_private_ip(IpAddr::V6(Ipv6Addr::LOCALHOST)));
    }

    #[test]
    fn private_ip_ipv6_mapped_ipv4() {
        // ::ffff:10.0.0.1
        let mapped: Ipv6Addr = "::ffff:10.0.0.1".parse().unwrap();
        assert!(is_private_ip(IpAddr::V6(mapped)));
        // ::ffff:192.168.1.1
        let mapped2: Ipv6Addr = "::ffff:192.168.1.1".parse().unwrap();
        assert!(is_private_ip(IpAddr::V6(mapped2)));
    }

    #[test]
    fn private_ip_ipv6_unique_local() {
        let addr: Ipv6Addr = "fc00::1".parse().unwrap();
        assert!(is_private_ip(IpAddr::V6(addr)));
        let addr2: Ipv6Addr = "fd12:3456::1".parse().unwrap();
        assert!(is_private_ip(IpAddr::V6(addr2)));
    }

    #[test]
    fn private_ip_ipv6_link_local() {
        let addr: Ipv6Addr = "fe80::1".parse().unwrap();
        assert!(is_private_ip(IpAddr::V6(addr)));
    }

    #[test]
    fn private_ip_public_returns_false() {
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_private_ip(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34))));
        let public_v6: Ipv6Addr = "2001:db8::1".parse().unwrap();
        assert!(!is_private_ip(IpAddr::V6(public_v6)));
    }

    #[test]
    fn private_ip_mapped_public_returns_false() {
        let mapped: Ipv6Addr = "::ffff:8.8.8.8".parse().unwrap();
        assert!(!is_private_ip(IpAddr::V6(mapped)));
    }

    // ── is_private_ipv4 ─────────────────────────────────────────

    #[test]
    fn private_ipv4_10_range() {
        assert!(is_private_ipv4(Ipv4Addr::new(10, 0, 0, 0)));
        assert!(is_private_ipv4(Ipv4Addr::new(10, 255, 255, 255)));
    }

    #[test]
    fn private_ipv4_172_range() {
        assert!(is_private_ipv4(Ipv4Addr::new(172, 16, 0, 0)));
        assert!(is_private_ipv4(Ipv4Addr::new(172, 31, 255, 255)));
        // 172.15 is not private
        assert!(!is_private_ipv4(Ipv4Addr::new(172, 15, 0, 1)));
        // 172.32 is not private
        assert!(!is_private_ipv4(Ipv4Addr::new(172, 32, 0, 1)));
    }

    #[test]
    fn private_ipv4_192_range() {
        assert!(is_private_ipv4(Ipv4Addr::new(192, 168, 0, 0)));
        assert!(is_private_ipv4(Ipv4Addr::new(192, 168, 255, 255)));
    }

    #[test]
    fn private_ipv4_loopback() {
        assert!(is_private_ipv4(Ipv4Addr::new(127, 0, 0, 1)));
        assert!(is_private_ipv4(Ipv4Addr::new(127, 255, 255, 255)));
    }

    #[test]
    fn private_ipv4_link_local() {
        assert!(is_private_ipv4(Ipv4Addr::new(169, 254, 0, 1)));
        assert!(is_private_ipv4(Ipv4Addr::new(169, 254, 169, 254)));
    }

    #[test]
    fn private_ipv4_zero_network() {
        assert!(is_private_ipv4(Ipv4Addr::new(0, 0, 0, 0)));
        assert!(is_private_ipv4(Ipv4Addr::new(0, 1, 2, 3)));
    }

    #[test]
    fn private_ipv4_public_returns_false() {
        assert!(!is_private_ipv4(Ipv4Addr::new(8, 8, 8, 8)));
        assert!(!is_private_ipv4(Ipv4Addr::new(1, 1, 1, 1)));
        assert!(!is_private_ipv4(Ipv4Addr::new(93, 184, 216, 34)));
    }
}
