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
