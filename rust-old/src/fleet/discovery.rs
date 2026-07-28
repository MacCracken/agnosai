//! Multi-node fleet discovery backends.
//!
//! Provides a `DiscoveryBackend` trait for pluggable node discovery, with
//! `StaticDiscovery` for fixed lists and `DnsDiscovery` as a stub for
//! DNS SRV-based discovery.

use std::collections::HashMap;

/// A node discovered by a `DiscoveryBackend`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DiscoveredNode {
    /// Network address (hostname or IP).
    pub address: String,
    /// Port number.
    pub port: u16,
    /// Arbitrary metadata (e.g. region, zone, GPU type).
    pub metadata: HashMap<String, String>,
}

impl DiscoveredNode {
    /// Create a new discovered node with no metadata.
    pub fn new(address: impl Into<String>, port: u16) -> Self {
        Self {
            address: address.into(),
            port,
            metadata: HashMap::new(),
        }
    }

    /// Add a metadata key-value pair.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Return the `address:port` socket string.
    #[must_use]
    #[inline]
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }
}

/// Trait for pluggable fleet discovery backends.
///
/// Implementations resolve a set of fleet nodes that the coordinator can
/// register and monitor.
pub trait DiscoveryBackend: Send + Sync {
    /// Discover available nodes.
    ///
    /// Implementations should return all currently known nodes. The coordinator
    /// handles deduplication and liveness tracking.
    fn discover(&self) -> impl std::future::Future<Output = Vec<DiscoveredNode>> + Send;
}

/// A discovery backend that returns a fixed, pre-configured list of nodes.
///
/// Useful for development, testing, and static infrastructure deployments.
#[derive(Debug, Clone)]
pub struct StaticDiscovery {
    nodes: Vec<DiscoveredNode>,
}

impl StaticDiscovery {
    /// Create a new static discovery backend with the given nodes.
    pub fn new(nodes: Vec<DiscoveredNode>) -> Self {
        Self { nodes }
    }
}

impl DiscoveryBackend for StaticDiscovery {
    async fn discover(&self) -> Vec<DiscoveredNode> {
        tracing::debug!(
            count = self.nodes.len(),
            "static discovery returning fixed node list"
        );
        self.nodes.clone()
    }
}

/// DNS SRV-based fleet discovery (stub).
///
/// Resolves a DNS SRV record to discover fleet nodes. This is a structural
/// placeholder — actual DNS resolution requires a DNS client dependency
/// (e.g. `trust-dns-resolver`) which is not included in the base crate.
#[derive(Debug, Clone)]
pub struct DnsDiscovery {
    /// The SRV record name to resolve (e.g. `_agnosai._tcp.fleet.example.com`).
    pub srv_name: String,
}

impl DnsDiscovery {
    /// Create a new DNS discovery backend for the given SRV record.
    pub fn new(srv_name: impl Into<String>) -> Self {
        Self {
            srv_name: srv_name.into(),
        }
    }
}

impl DiscoveryBackend for DnsDiscovery {
    async fn discover(&self) -> Vec<DiscoveredNode> {
        tracing::warn!(
            srv_name = %self.srv_name,
            "DNS SRV discovery is a stub — no DNS client dependency available"
        );
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovered_node_new() {
        let node = DiscoveredNode::new("10.0.0.1", 8080);
        assert_eq!(node.address, "10.0.0.1");
        assert_eq!(node.port, 8080);
        assert!(node.metadata.is_empty());
    }

    #[test]
    fn discovered_node_with_metadata() {
        let node = DiscoveredNode::new("10.0.0.1", 8080)
            .with_metadata("region", "us-east-1")
            .with_metadata("gpu", "A100");
        assert_eq!(node.metadata.len(), 2);
        assert_eq!(node.metadata.get("region").unwrap(), "us-east-1");
        assert_eq!(node.metadata.get("gpu").unwrap(), "A100");
    }

    #[test]
    fn discovered_node_socket_addr() {
        let node = DiscoveredNode::new("10.0.0.1", 8080);
        assert_eq!(node.socket_addr(), "10.0.0.1:8080");
    }

    #[tokio::test]
    async fn static_discovery_returns_all_nodes() {
        let nodes = vec![
            DiscoveredNode::new("10.0.0.1", 8080),
            DiscoveredNode::new("10.0.0.2", 8080),
            DiscoveredNode::new("10.0.0.3", 9090),
        ];
        let backend = StaticDiscovery::new(nodes);
        let discovered = backend.discover().await;
        assert_eq!(discovered.len(), 3);
        assert_eq!(discovered[0].address, "10.0.0.1");
        assert_eq!(discovered[1].address, "10.0.0.2");
        assert_eq!(discovered[2].port, 9090);
    }

    #[tokio::test]
    async fn static_discovery_empty() {
        let backend = StaticDiscovery::new(vec![]);
        let discovered = backend.discover().await;
        assert!(discovered.is_empty());
    }

    #[tokio::test]
    async fn dns_discovery_stub_returns_empty() {
        let backend = DnsDiscovery::new("_agnosai._tcp.fleet.example.com");
        let discovered = backend.discover().await;
        assert!(discovered.is_empty());
    }

    #[test]
    fn dns_discovery_stores_srv_name() {
        let backend = DnsDiscovery::new("_agnosai._tcp.fleet.example.com");
        assert_eq!(backend.srv_name, "_agnosai._tcp.fleet.example.com");
    }
}
