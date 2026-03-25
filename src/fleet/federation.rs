//! Multi-cluster federation with coordinator election and discovery.
//!
//! Implements a lightweight federation protocol for connecting multiple
//! AgnosAI clusters:
//!
//! - **Coordinator election** — Raft-inspired leader election with term numbers
//! - **Cluster discovery** — static seed list (DNS-SD/mDNS pluggable later)
//! - **Health tracking** — per-cluster heartbeat with configurable TTL

use std::collections::HashMap;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Unique identifier for a federated cluster.
pub type ClusterId = String;

/// Role of a cluster in the federation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum FederationRole {
    /// This cluster is the coordinator (leader).
    Coordinator,
    /// This cluster is a follower.
    Follower,
    /// Election in progress, no leader yet.
    Candidate,
}

/// Status of a federated cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ClusterStatus {
    Online,
    Suspect,
    Offline,
}

/// Information about a federated cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ClusterInfo {
    pub id: ClusterId,
    /// gRPC/HTTP endpoint for the cluster's control plane.
    pub endpoint: String,
    pub status: ClusterStatus,
    pub role: FederationRole,
    pub node_count: usize,
    pub last_heartbeat: DateTime<Utc>,
    /// Monotonic instant of last heartbeat (not serialized).
    #[serde(skip, default = "Instant::now")]
    pub last_heartbeat_instant: Instant,
}

/// Election state for coordinator selection.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ElectionState {
    /// Current election term.
    pub term: u64,
    /// Current coordinator (None during election).
    pub coordinator: Option<ClusterId>,
    /// When the current term started.
    pub term_started: Instant,
}

/// Configuration for the federation manager.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct FederationConfig {
    /// This cluster's ID.
    pub cluster_id: ClusterId,
    /// This cluster's control-plane endpoint.
    pub endpoint: String,
    /// Seed endpoints for discovery.
    pub seeds: Vec<String>,
    /// Time without heartbeat before marking a cluster suspect.
    pub suspect_timeout: Duration,
    /// Time without heartbeat before marking a cluster offline.
    pub offline_timeout: Duration,
    /// Minimum election timeout (randomized up to 2x this value).
    pub election_timeout: Duration,
}

impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            cluster_id: String::new(),
            endpoint: String::new(),
            seeds: Vec::new(),
            suspect_timeout: Duration::from_secs(10),
            offline_timeout: Duration::from_secs(30),
            election_timeout: Duration::from_secs(5),
        }
    }
}

/// Manages multi-cluster federation, coordinator election, and health tracking.
pub struct FederationManager {
    config: FederationConfig,
    role: FederationRole,
    clusters: HashMap<ClusterId, ClusterInfo>,
    election: ElectionState,
}

impl FederationManager {
    pub fn new(config: FederationConfig) -> Self {
        info!(
            cluster_id = %config.cluster_id,
            seeds = config.seeds.len(),
            "federation manager initialized"
        );
        Self {
            role: FederationRole::Follower,
            clusters: HashMap::new(),
            election: ElectionState {
                term: 0,
                coordinator: None,
                term_started: Instant::now(),
            },
            config,
        }
    }

    /// Register or update a peer cluster from a heartbeat.
    pub fn heartbeat(&mut self, cluster_id: ClusterId, endpoint: String, node_count: usize) {
        let now = Instant::now();
        let entry = self.clusters.entry(cluster_id.clone()).or_insert_with(|| {
            info!(cluster = %cluster_id, "new cluster joined federation");
            ClusterInfo {
                id: cluster_id.clone(),
                endpoint: endpoint.clone(),
                status: ClusterStatus::Online,
                role: FederationRole::Follower,
                node_count: 0,
                last_heartbeat: Utc::now(),
                last_heartbeat_instant: now,
            }
        });

        entry.endpoint = endpoint;
        entry.node_count = node_count;
        entry.status = ClusterStatus::Online;
        entry.last_heartbeat = Utc::now();
        entry.last_heartbeat_instant = now;

        debug!(cluster = %cluster_id, node_count, "heartbeat received");
    }

    /// Check all clusters for liveness and update statuses.
    pub fn check_liveness(&mut self) {
        let now = Instant::now();
        for cluster in self.clusters.values_mut() {
            let elapsed = now.duration_since(cluster.last_heartbeat_instant);
            if elapsed > self.config.offline_timeout && cluster.status != ClusterStatus::Offline {
                warn!(cluster = %cluster.id, "cluster marked offline");
                cluster.status = ClusterStatus::Offline;
            } else if elapsed > self.config.suspect_timeout
                && cluster.status == ClusterStatus::Online
            {
                warn!(cluster = %cluster.id, "cluster marked suspect");
                cluster.status = ClusterStatus::Suspect;
            }
        }
    }

    /// Start a new election term. This cluster becomes a candidate.
    ///
    /// Returns the new term number.
    pub fn start_election(&mut self) -> u64 {
        self.election.term += 1;
        self.election.coordinator = None;
        self.election.term_started = Instant::now();
        self.role = FederationRole::Candidate;

        info!(
            term = self.election.term,
            cluster = %self.config.cluster_id,
            "starting election"
        );

        self.election.term
    }

    /// Declare a coordinator for the current term.
    ///
    /// Returns `true` if the declaration was accepted (term matches).
    pub fn declare_coordinator(&mut self, term: u64, coordinator_id: ClusterId) -> bool {
        if term < self.election.term {
            debug!(
                declared_term = term,
                current_term = self.election.term,
                "ignoring stale coordinator declaration"
            );
            return false;
        }

        info!(
            term,
            coordinator = %coordinator_id,
            "coordinator elected"
        );

        self.election.term = term;
        self.election.coordinator = Some(coordinator_id.clone());

        if coordinator_id == self.config.cluster_id {
            self.role = FederationRole::Coordinator;
        } else {
            self.role = FederationRole::Follower;
        }

        // Reset all clusters to Follower, then set the new coordinator.
        for cluster in self.clusters.values_mut() {
            cluster.role = FederationRole::Follower;
        }
        if let Some(cluster) = self.clusters.get_mut(&coordinator_id) {
            cluster.role = FederationRole::Coordinator;
        }

        true
    }

    /// Elect a coordinator based on the lowest cluster ID (deterministic tiebreak).
    ///
    /// Only considers online clusters. Returns the elected coordinator ID,
    /// or `None` if no clusters are online.
    pub fn elect_by_lowest_id(&mut self) -> Option<ClusterId> {
        let term = self.start_election();

        // Include ourselves and all online peers.
        let mut candidates: Vec<ClusterId> = self
            .clusters
            .values()
            .filter(|c| c.status == ClusterStatus::Online)
            .map(|c| c.id.clone())
            .collect();
        candidates.push(self.config.cluster_id.clone());
        candidates.sort();
        candidates.dedup();

        if let Some(winner) = candidates.first().cloned() {
            self.declare_coordinator(term, winner.clone());
            Some(winner)
        } else {
            None
        }
    }

    /// Get the current coordinator, if one is elected.
    pub fn coordinator(&self) -> Option<&str> {
        self.election.coordinator.as_deref()
    }

    /// Whether this cluster is the coordinator.
    pub fn is_coordinator(&self) -> bool {
        self.role == FederationRole::Coordinator
    }

    /// This cluster's current role.
    pub fn role(&self) -> FederationRole {
        self.role
    }

    /// Current election term.
    pub fn term(&self) -> u64 {
        self.election.term
    }

    /// List all known clusters.
    pub fn clusters(&self) -> Vec<&ClusterInfo> {
        self.clusters.values().collect()
    }

    /// List only online clusters.
    pub fn online_clusters(&self) -> Vec<&ClusterInfo> {
        self.clusters
            .values()
            .filter(|c| c.status == ClusterStatus::Online)
            .collect()
    }

    /// Number of known clusters (excluding self).
    pub fn cluster_count(&self) -> usize {
        self.clusters.len()
    }

    /// Remove clusters that have been offline beyond the TTL.
    pub fn evict_offline(&mut self, ttl: Duration) -> Vec<ClusterId> {
        let now = Instant::now();
        let evicted: Vec<ClusterId> = self
            .clusters
            .iter()
            .filter(|(_, c)| {
                c.status == ClusterStatus::Offline
                    && now.duration_since(c.last_heartbeat_instant) > ttl
            })
            .map(|(id, _)| id.clone())
            .collect();

        for id in &evicted {
            info!(cluster = %id, "evicting offline cluster");
            self.clusters.remove(id);
        }

        evicted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(id: &str) -> FederationConfig {
        FederationConfig {
            cluster_id: id.into(),
            endpoint: format!("http://{id}:8080"),
            suspect_timeout: Duration::from_millis(100),
            offline_timeout: Duration::from_millis(200),
            ..Default::default()
        }
    }

    #[test]
    fn heartbeat_registers_cluster() {
        let mut fm = FederationManager::new(test_config("cluster-a"));
        fm.heartbeat("cluster-b".into(), "http://b:8080".into(), 5);

        assert_eq!(fm.cluster_count(), 1);
        let clusters = fm.online_clusters();
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].id, "cluster-b");
        assert_eq!(clusters[0].node_count, 5);
    }

    #[test]
    fn heartbeat_updates_existing() {
        let mut fm = FederationManager::new(test_config("cluster-a"));
        fm.heartbeat("cluster-b".into(), "http://b:8080".into(), 5);
        fm.heartbeat("cluster-b".into(), "http://b:8080".into(), 10);

        assert_eq!(fm.cluster_count(), 1);
        let b = fm
            .clusters
            .get("cluster-b")
            .expect("cluster-b should exist");
        assert_eq!(b.node_count, 10);
    }

    #[test]
    fn check_liveness_marks_suspect_and_offline() {
        let mut fm = FederationManager::new(test_config("a"));
        fm.heartbeat("b".into(), "http://b:8080".into(), 1);

        // Manually backdate the heartbeat instant.
        if let Some(c) = fm.clusters.get_mut("b") {
            c.last_heartbeat_instant = Instant::now() - Duration::from_millis(150);
        }
        fm.check_liveness();
        assert_eq!(fm.clusters["b"].status, ClusterStatus::Suspect);

        if let Some(c) = fm.clusters.get_mut("b") {
            c.last_heartbeat_instant = Instant::now() - Duration::from_millis(250);
        }
        fm.check_liveness();
        assert_eq!(fm.clusters["b"].status, ClusterStatus::Offline);
    }

    #[test]
    fn start_election_increments_term() {
        let mut fm = FederationManager::new(test_config("a"));
        assert_eq!(fm.term(), 0);
        let t1 = fm.start_election();
        assert_eq!(t1, 1);
        assert_eq!(fm.role(), FederationRole::Candidate);
        let t2 = fm.start_election();
        assert_eq!(t2, 2);
    }

    #[test]
    fn declare_coordinator() {
        let mut fm = FederationManager::new(test_config("a"));
        fm.start_election();
        assert!(fm.declare_coordinator(1, "a".into()));
        assert!(fm.is_coordinator());
        assert_eq!(fm.coordinator(), Some("a"));
    }

    #[test]
    fn declare_coordinator_stale_term() {
        let mut fm = FederationManager::new(test_config("a"));
        fm.start_election(); // term 1
        fm.start_election(); // term 2
        assert!(!fm.declare_coordinator(1, "b".into())); // stale
    }

    #[test]
    fn elect_by_lowest_id() {
        let mut fm = FederationManager::new(test_config("cluster-b"));
        fm.heartbeat("cluster-a".into(), "http://a:8080".into(), 1);
        fm.heartbeat("cluster-c".into(), "http://c:8080".into(), 1);

        let winner = fm.elect_by_lowest_id().expect("should elect");
        assert_eq!(winner, "cluster-a");
        assert_eq!(fm.coordinator(), Some("cluster-a"));
        assert_eq!(fm.role(), FederationRole::Follower);
    }

    #[test]
    fn elect_self_when_lowest() {
        let mut fm = FederationManager::new(test_config("aaa"));
        fm.heartbeat("bbb".into(), "http://b:8080".into(), 1);

        let winner = fm.elect_by_lowest_id().expect("should elect");
        assert_eq!(winner, "aaa");
        assert!(fm.is_coordinator());
    }

    #[test]
    fn evict_offline_clusters() {
        let mut fm = FederationManager::new(test_config("a"));
        fm.heartbeat("b".into(), "http://b:8080".into(), 1);

        // Backdate and mark offline.
        if let Some(c) = fm.clusters.get_mut("b") {
            c.status = ClusterStatus::Offline;
            c.last_heartbeat_instant = Instant::now() - Duration::from_secs(60);
        }

        let evicted = fm.evict_offline(Duration::from_secs(30));
        assert_eq!(evicted, vec!["b"]);
        assert_eq!(fm.cluster_count(), 0);
    }

    #[test]
    fn evict_does_not_remove_recent_offline() {
        let mut fm = FederationManager::new(test_config("a"));
        fm.heartbeat("b".into(), "http://b:8080".into(), 1);
        if let Some(c) = fm.clusters.get_mut("b") {
            c.status = ClusterStatus::Offline;
        }

        let evicted = fm.evict_offline(Duration::from_secs(300));
        assert!(evicted.is_empty());
        assert_eq!(fm.cluster_count(), 1);
    }

    #[test]
    fn coordinator_returns_none_initially() {
        let fm = FederationManager::new(test_config("a"));
        assert!(fm.coordinator().is_none());
        assert!(!fm.is_coordinator());
    }

    #[test]
    fn initial_role_is_follower() {
        let fm = FederationManager::new(test_config("a"));
        assert_eq!(fm.role(), FederationRole::Follower);
    }

    #[test]
    fn is_coordinator_reflects_election() {
        let mut fm = FederationManager::new(test_config("a"));
        assert!(!fm.is_coordinator());

        fm.start_election();
        fm.declare_coordinator(1, "a".into());
        assert!(fm.is_coordinator());

        // Declare a different coordinator — we become follower.
        fm.start_election();
        fm.declare_coordinator(2, "b".into());
        assert!(!fm.is_coordinator());
        assert_eq!(fm.role(), FederationRole::Follower);
    }

    #[test]
    fn role_changes_with_declare_coordinator() {
        let mut fm = FederationManager::new(test_config("x"));
        assert_eq!(fm.role(), FederationRole::Follower);

        fm.start_election();
        assert_eq!(fm.role(), FederationRole::Candidate);

        fm.declare_coordinator(1, "x".into());
        assert_eq!(fm.role(), FederationRole::Coordinator);

        fm.start_election();
        fm.declare_coordinator(2, "y".into());
        assert_eq!(fm.role(), FederationRole::Follower);
    }

    #[test]
    fn term_increments_with_elections() {
        let mut fm = FederationManager::new(test_config("a"));
        assert_eq!(fm.term(), 0);

        fm.start_election();
        assert_eq!(fm.term(), 1);
        fm.start_election();
        assert_eq!(fm.term(), 2);
        fm.start_election();
        assert_eq!(fm.term(), 3);
    }

    #[test]
    fn clusters_and_online_clusters_filtering() {
        let mut fm = FederationManager::new(test_config("a"));
        fm.heartbeat("b".into(), "http://b:8080".into(), 1);
        fm.heartbeat("c".into(), "http://c:8080".into(), 2);

        assert_eq!(fm.clusters().len(), 2);
        assert_eq!(fm.online_clusters().len(), 2);

        // Mark one offline.
        if let Some(c) = fm.clusters.get_mut("b") {
            c.status = ClusterStatus::Offline;
        }

        assert_eq!(fm.clusters().len(), 2, "clusters() returns all");
        assert_eq!(fm.online_clusters().len(), 1, "online_clusters() filters");
        assert_eq!(fm.online_clusters()[0].id, "c");
    }

    #[test]
    fn cluster_count_accuracy() {
        let mut fm = FederationManager::new(test_config("a"));
        assert_eq!(fm.cluster_count(), 0);

        fm.heartbeat("b".into(), "http://b:8080".into(), 1);
        assert_eq!(fm.cluster_count(), 1);

        fm.heartbeat("c".into(), "http://c:8080".into(), 2);
        assert_eq!(fm.cluster_count(), 2);

        // Heartbeat same cluster again — count stays the same.
        fm.heartbeat("b".into(), "http://b:8080".into(), 5);
        assert_eq!(fm.cluster_count(), 2);
    }

    #[test]
    fn evict_offline_removes_old_clusters() {
        let mut fm = FederationManager::new(test_config("a"));
        fm.heartbeat("b".into(), "http://b:8080".into(), 1);
        fm.heartbeat("c".into(), "http://c:8080".into(), 1);

        // Only mark "b" as old+offline.
        if let Some(c) = fm.clusters.get_mut("b") {
            c.status = ClusterStatus::Offline;
            c.last_heartbeat_instant = Instant::now() - Duration::from_secs(120);
        }

        let evicted = fm.evict_offline(Duration::from_secs(60));
        assert_eq!(evicted.len(), 1);
        assert_eq!(evicted[0], "b");
        assert_eq!(fm.cluster_count(), 1);
        // "c" should still be there.
        assert!(fm.clusters.contains_key("c"));
    }

    #[test]
    fn new_coordinator_resets_old_coordinator_role() {
        let mut fm = FederationManager::new(test_config("a"));
        fm.heartbeat("b".into(), "http://b:8080".into(), 1);
        fm.heartbeat("c".into(), "http://c:8080".into(), 1);

        // Elect "b" as coordinator.
        fm.start_election();
        fm.declare_coordinator(1, "b".into());
        assert_eq!(
            fm.clusters.get("b").unwrap().role,
            FederationRole::Coordinator
        );
        assert_eq!(fm.clusters.get("c").unwrap().role, FederationRole::Follower);

        // Now elect "c" — "b" should be reset to Follower.
        fm.start_election();
        fm.declare_coordinator(2, "c".into());
        assert_eq!(fm.clusters.get("b").unwrap().role, FederationRole::Follower);
        assert_eq!(
            fm.clusters.get("c").unwrap().role,
            FederationRole::Coordinator
        );
    }
}
