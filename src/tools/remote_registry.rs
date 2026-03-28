//! Remote WASM tool registry — download and register tool packages from URLs.
//!
//! Fetches `.agpkg` ZIP bundles or raw WASM modules from a remote URL,
//! validates them, and registers the contained tools in the `ToolRegistry`.

use std::sync::OnceLock;

use tracing::info;

/// Maximum download size for a remote tool package (10 MB).
const MAX_DOWNLOAD_SIZE: usize = 10 * 1024 * 1024;

/// Shared HTTP client for remote tool fetches.
fn http_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client for remote registry")
    })
}

/// Result of a remote tool fetch operation.
#[derive(Debug)]
#[non_exhaustive]
pub enum FetchResult {
    /// Successfully downloaded and validated.
    Ok {
        /// Number of bytes downloaded.
        size_bytes: usize,
        /// Content type from the response.
        content_type: Option<String>,
        /// The raw bytes of the package.
        data: Vec<u8>,
    },
    /// Download failed.
    Error(String),
}

/// Fetch a tool package from a remote URL.
///
/// Validates the URL against SSRF rules and enforces a size limit.
pub async fn fetch_package(url: &str) -> FetchResult {
    // SSRF check.
    if !crate::server::ssrf::is_safe_url(url) {
        return FetchResult::Error("URL rejected: private/internal network".into());
    }

    info!(url, "fetching remote tool package");

    let response = match http_client().get(url).send().await {
        Ok(r) => r,
        Err(e) => return FetchResult::Error(format!("download failed: {e}")),
    };

    if !response.status().is_success() {
        return FetchResult::Error(format!("HTTP {}", response.status()));
    }

    // Check content-length before downloading.
    if let Some(len) = response.content_length()
        && len as usize > MAX_DOWNLOAD_SIZE
    {
        return FetchResult::Error(format!(
            "package too large: {len} bytes (max {MAX_DOWNLOAD_SIZE})"
        ));
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    match response.bytes().await {
        Ok(bytes) => {
            if bytes.len() > MAX_DOWNLOAD_SIZE {
                return FetchResult::Error(format!(
                    "package too large: {} bytes (max {MAX_DOWNLOAD_SIZE})",
                    bytes.len()
                ));
            }
            info!(
                url,
                size_bytes = bytes.len(),
                "remote tool package downloaded"
            );
            FetchResult::Ok {
                size_bytes: bytes.len(),
                content_type,
                data: bytes.to_vec(),
            }
        }
        Err(e) => FetchResult::Error(format!("failed to read response body: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_download_size_is_reasonable() {
        assert_eq!(MAX_DOWNLOAD_SIZE, 10 * 1024 * 1024);
    }

    #[tokio::test]
    async fn fetch_rejects_private_url() {
        let result = fetch_package("http://192.168.1.1/tool.wasm").await;
        assert!(matches!(result, FetchResult::Error(ref e) if e.contains("private")));
    }

    #[tokio::test]
    async fn fetch_rejects_localhost() {
        let result = fetch_package("http://localhost:8080/tool.wasm").await;
        assert!(matches!(result, FetchResult::Error(ref e) if e.contains("private")));
    }
}
