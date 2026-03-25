use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, info, warn};

use crate::core::crew::CrewId;
use crate::core::{CrewSpec, CrewState, CrewStatus, ResourceBudget, Result};
use crate::llm::{AuditChain, CostTracker, HooshClient, ResponseCache};
use crate::server::sse::EventBus;
use dashmap::DashMap;

use crate::orchestrator::crew_runner::CrewRunner;
use crate::orchestrator::scheduler::Scheduler;

/// Maximum number of completed crews retained in memory.
const MAX_RETAINED_CREWS: usize = 1000;

/// Internal mutable state of the orchestrator.
pub(crate) struct OrchestratorState {
    /// Task scheduler instance (used by future fleet integration).
    #[allow(dead_code)]
    pub(crate) scheduler: Scheduler,
    /// Crews currently tracked (active and recently completed).
    pub(crate) active_crews: Vec<CrewState>,
}

/// Top-level orchestrator managing crew lifecycles and scheduling.
pub struct Orchestrator {
    state: Arc<RwLock<OrchestratorState>>,
    budget: ResourceBudget,
    events: Option<EventBus>,
    /// Lazily-initialised LLM client. Created on first crew execution that
    /// needs inference, so startup stays fast when no LLM work is pending.
    llm: OnceLock<Arc<HooshClient>>,
    /// Base URL for the hoosh inference gateway (e.g. `http://localhost:8088`).
    /// When `None`, tasks fall back to placeholder execution.
    llm_url: Option<String>,
    /// Shared inference response cache across all crew executions.
    cache: Arc<ResponseCache>,
    /// Shared cost tracker for inference cost accounting.
    cost_tracker: Arc<CostTracker>,
    /// Semaphore limiting concurrent crew executions.
    crew_semaphore: Arc<Semaphore>,
    /// Per-crew cancellation tokens. When set to `true`, the crew runner
    /// will stop scheduling new tasks and return early.
    cancel_tokens: Arc<DashMap<CrewId, Arc<AtomicBool>>>,
    /// Cryptographic audit chain for tamper-proof crew/task event logging.
    audit: Arc<AuditChain>,
}

impl Orchestrator {
    /// Create a new orchestrator with the given resource budget.
    pub async fn new(budget: ResourceBudget) -> Result<Self> {
        let max_concurrent = budget.max_concurrent_tasks.unwrap_or(10);
        let state = OrchestratorState {
            scheduler: Scheduler::new(),
            active_crews: Vec::new(),
        };

        // Generate a cryptographically random signing key for the audit chain.
        let mut audit_key = [0u8; 32];
        rand::Rng::fill(&mut rand::rng(), &mut audit_key);

        Ok(Self {
            state: Arc::new(RwLock::new(state)),
            budget,
            events: None,
            llm: OnceLock::new(),
            llm_url: None,
            cache: Arc::new(ResponseCache::new(Default::default())),
            cost_tracker: Arc::new(CostTracker::new()),
            crew_semaphore: Arc::new(Semaphore::new(max_concurrent)),
            cancel_tokens: Arc::new(DashMap::new()),
            audit: Arc::new(AuditChain::new(&audit_key, 10_000)),
        })
    }

    /// Attach an event bus for SSE streaming.
    pub fn with_events(mut self, events: EventBus) -> Self {
        self.events = Some(events);
        self
    }

    /// Attach a pre-built audit chain (overrides the random-key default).
    pub fn with_audit(mut self, audit: Arc<AuditChain>) -> Self {
        self.audit = audit;
        self
    }

    /// Get a reference to the audit chain.
    pub fn audit(&self) -> &Arc<AuditChain> {
        &self.audit
    }

    /// Set the hoosh URL for lazy LLM client initialisation.
    ///
    /// The actual [`HooshClient`] is created on the first crew execution that
    /// requires inference, keeping server startup fast.
    pub fn with_llm_url(mut self, url: impl Into<String>) -> Self {
        self.llm_url = Some(url.into());
        self
    }

    /// Attach a pre-built LLM client (skips lazy init).
    pub fn with_llm(self, client: Arc<HooshClient>) -> Self {
        let _ = self.llm.set(client);
        self
    }

    /// Get or lazily create the LLM client. Returns `None` when no URL was
    /// configured (placeholder mode).
    fn llm_client(&self) -> Option<&Arc<HooshClient>> {
        if let Some(client) = self.llm.get() {
            return Some(client);
        }
        let url = self.llm_url.as_deref()?;
        Some(self.llm.get_or_init(|| {
            info!(hoosh_url = %url, "LLM client initialised (lazy)");
            Arc::new(HooshClient::new(url))
        }))
    }

    /// Submit and execute a crew, returning the final state.
    #[tracing::instrument(skip(self, spec), fields(crew_id = %spec.id, crew_name = %spec.name, task_count = spec.tasks.len()))]
    pub async fn run_crew(&self, spec: CrewSpec) -> Result<CrewState> {
        // Enforce concurrent crew limit.
        let _permit =
            self.crew_semaphore.acquire().await.map_err(|_| {
                crate::core::AgnosaiError::Scheduling("crew semaphore closed".into())
            })?;

        let crew_id = spec.id;
        let crew_name = spec.name.clone();
        let task_count = spec.tasks.len();

        info!(crew_id = %crew_id, name = %crew_name, tasks = task_count, "crew accepted");

        // Audit: crew accepted.
        self.audit.record(
            "crew_accepted",
            "info",
            &crew_name,
            None,
            None,
            Some(serde_json::json!({
                "crew_id": crew_id.to_string(),
                "task_count": task_count,
            })),
        );

        // Register as pending, evicting oldest completed crews if at capacity.
        {
            let mut state = self.state.write().await;
            if state.active_crews.len() >= MAX_RETAINED_CREWS {
                let before = state.active_crews.len();
                state.active_crews.retain(|c| {
                    !matches!(
                        c.status,
                        CrewStatus::Completed | CrewStatus::Failed | CrewStatus::Cancelled
                    )
                });
                debug!(
                    evicted = before - state.active_crews.len(),
                    "evicted completed crews"
                );
            }
            state.active_crews.push(CrewState {
                crew_id,
                status: CrewStatus::Pending,
                results: Vec::new(),
                profile: None,
            });
        }

        // Create cancellation token for this crew.
        let cancel_token = Arc::new(AtomicBool::new(false));
        self.cancel_tokens
            .insert(crew_id, Arc::clone(&cancel_token));

        // Delegate to CrewRunner for the actual lifecycle.
        let mut runner = CrewRunner::new(spec)
            .with_cache(Arc::clone(&self.cache))
            .with_cost_tracker(Arc::clone(&self.cost_tracker))
            .with_cancel_token(cancel_token)
            .with_audit(Arc::clone(&self.audit));
        if let Some(llm) = self.llm_client() {
            runner = runner.with_llm(Arc::clone(llm));
        }
        if let Some(ref events) = self.events {
            runner = runner.with_events(events.sender(crew_id));
        }

        // Enforce execution timeout from budget.
        let timeout_dur = self
            .budget
            .max_duration_secs
            .map(std::time::Duration::from_secs)
            .unwrap_or(std::time::Duration::from_secs(3600)); // 1 hour default

        let crew_state = match tokio::time::timeout(timeout_dur, runner.run()).await {
            Ok(result) => result?,
            Err(_) => {
                warn!(crew_id = %crew_id, timeout_secs = timeout_dur.as_secs(), "crew execution timed out");
                CrewState {
                    crew_id,
                    status: CrewStatus::Failed,
                    results: Vec::new(),
                    profile: None,
                }
            }
        };

        info!(
            crew_id = %crew_id,
            status = ?crew_state.status,
            results = crew_state.results.len(),
            "crew finished"
        );

        // Audit: crew finished.
        {
            let status_str = format!("{:?}", crew_state.status);
            let cost_usd = crew_state.profile.as_ref().map_or(0.0, |p| p.cost_usd);
            let wall_ms = crew_state.profile.as_ref().map_or(0, |p| p.wall_ms);
            let level = match crew_state.status {
                CrewStatus::Completed => "info",
                CrewStatus::Failed => "error",
                CrewStatus::Cancelled => "warn",
                _ => "info",
            };
            self.audit.record(
                "crew_finished",
                level,
                &crew_name,
                None,
                None,
                Some(serde_json::json!({
                    "crew_id": crew_id.to_string(),
                    "status": status_str,
                    "task_count": crew_state.results.len(),
                    "wall_ms": wall_ms,
                    "cost_usd": cost_usd,
                })),
            );
        }

        // Update stored state.
        {
            let mut state = self.state.write().await;
            if let Some(entry) = state.active_crews.iter_mut().find(|c| c.crew_id == crew_id) {
                *entry = crew_state.clone();
            }
        }

        // Clean up cancellation token and event channel.
        self.cancel_tokens.remove(&crew_id);
        if let Some(ref events) = self.events {
            events.remove(crew_id);
        }

        Ok(crew_state)
    }

    /// Cancel a running crew by ID.
    ///
    /// Sets the cancellation token so the crew runner stops scheduling new
    /// tasks and returns early. Tasks already in flight will complete but no
    /// new tasks will be started.
    #[tracing::instrument(skip(self), fields(%crew_id))]
    pub async fn cancel_crew(&self, crew_id: CrewId) -> Result<()> {
        // Signal the cancellation token (if the crew is still running).
        if let Some(token) = self.cancel_tokens.get(&crew_id) {
            token.store(true, Ordering::Release);
        }

        let mut state = self.state.write().await;
        if let Some(crew) = state.active_crews.iter_mut().find(|c| c.crew_id == crew_id) {
            crew.status = CrewStatus::Cancelled;
            info!(crew_id = %crew_id, "crew cancelled");
            self.audit.record(
                "crew_cancelled",
                "warn",
                "crew cancelled by caller",
                None,
                None,
                Some(serde_json::json!({ "crew_id": crew_id.to_string() })),
            );
            Ok(())
        } else {
            warn!(crew_id = %crew_id, "cancel failed: crew not found");
            Err(crate::core::AgnosaiError::CrewNotFound(crew_id.to_string()))
        }
    }

    /// Get a reference to the resource budget.
    pub fn budget(&self) -> &ResourceBudget {
        &self.budget
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::AgentDefinition;
    use crate::core::crew::CrewSpec;
    use crate::core::task::{ProcessMode, Task};

    fn test_spec(name: &str) -> CrewSpec {
        let agent = AgentDefinition::new("agent-a", "tester", "test things");
        let task = Task::new("do something");
        CrewSpec::new(name)
            .with_agents(vec![agent])
            .with_tasks(vec![task])
            .with_process(ProcessMode::Sequential)
    }

    #[tokio::test]
    async fn new_creates_orchestrator() {
        let orch = Orchestrator::new(Default::default()).await.unwrap();
        // Default budget should have sensible defaults.
        assert!(orch.budget().max_concurrent_tasks.is_some());
    }

    #[tokio::test]
    async fn run_crew_completes_successfully() {
        let orch = Orchestrator::new(Default::default()).await.unwrap();
        let spec = test_spec("test-crew");
        let crew_id = spec.id;
        let state = orch.run_crew(spec).await.unwrap();
        assert_eq!(state.crew_id, crew_id);
        assert_eq!(state.status, CrewStatus::Completed);
        assert_eq!(state.results.len(), 1);
    }

    #[tokio::test]
    async fn run_crew_with_events() {
        let events = EventBus::new();
        let orch = Orchestrator::new(Default::default())
            .await
            .unwrap()
            .with_events(events.clone());

        let spec = test_spec("evented-crew");
        let crew_id = spec.id;
        let mut rx = events.subscribe(crew_id);

        let state = orch.run_crew(spec).await.unwrap();
        assert_eq!(state.status, CrewStatus::Completed);

        // Events should have been emitted.
        let mut event_types = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            event_types.push(ev.event_type);
        }
        assert!(event_types.contains(&"crew_started".to_string()));
        assert!(event_types.contains(&"crew_completed".to_string()));
    }

    #[tokio::test]
    async fn run_crew_cleans_up_event_channel() {
        let events = EventBus::new();
        let orch = Orchestrator::new(Default::default())
            .await
            .unwrap()
            .with_events(events.clone());

        let spec = test_spec("cleanup-crew");
        let crew_id = spec.id;
        orch.run_crew(spec).await.unwrap();

        // Channel should be removed after completion — new subscribe gets a fresh channel.
        let mut rx = events.subscribe(crew_id);
        // The old sender was removed, so this is a new channel with no messages.
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn cancel_existing_crew() {
        let orch = Orchestrator::new(Default::default()).await.unwrap();
        let spec = test_spec("cancel-crew");
        let crew_id = spec.id;

        // Run to completion first so crew is in active_crews.
        orch.run_crew(spec).await.unwrap();

        // Cancel should succeed (crew exists).
        orch.cancel_crew(crew_id).await.unwrap();
    }

    #[tokio::test]
    async fn cancel_nonexistent_crew_returns_error() {
        let orch = Orchestrator::new(Default::default()).await.unwrap();
        let fake_id = uuid::Uuid::new_v4();
        let result = orch.cancel_crew(fake_id).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("crew not found"));
    }

    #[tokio::test]
    async fn multiple_crews_tracked_independently() {
        let orch = Orchestrator::new(Default::default()).await.unwrap();
        let spec1 = test_spec("crew-1");
        let spec2 = test_spec("crew-2");
        let id1 = spec1.id;
        let id2 = spec2.id;

        let s1 = orch.run_crew(spec1).await.unwrap();
        let s2 = orch.run_crew(spec2).await.unwrap();

        assert_eq!(s1.crew_id, id1);
        assert_eq!(s2.crew_id, id2);
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn budget_accessor_returns_default() {
        let orch = Orchestrator::new(Default::default()).await.unwrap();
        let b = orch.budget();
        // Budget is accessible and non-empty.
        assert!(b.max_concurrent_tasks.is_some());
    }

    #[tokio::test]
    async fn cancel_token_signals_crew_runner() {
        let orch = Orchestrator::new(Default::default()).await.unwrap();

        // Create a spec with multiple sequential tasks.
        let agent = AgentDefinition::new("agent-a", "tester", "test things");
        let tasks: Vec<Task> = (0..5).map(|i| Task::new(format!("task {i}"))).collect();
        let spec = CrewSpec::new("cancel-test")
            .with_agents(vec![agent])
            .with_tasks(tasks)
            .with_process(ProcessMode::Sequential);
        let crew_id = spec.id;

        // Pre-cancel before running — runner should stop early.
        orch.cancel_tokens
            .insert(crew_id, Arc::new(AtomicBool::new(true)));

        // Manually create a runner with the cancel token to test the mechanism.
        let token = orch.cancel_tokens.get(&crew_id).unwrap().clone();
        let mut runner = CrewRunner::new(spec).with_cancel_token(token);
        let state = runner.run().await.unwrap();

        // Crew should be marked cancelled with fewer results than total tasks.
        assert_eq!(state.status, CrewStatus::Cancelled);
        assert_eq!(state.results.len(), 0, "no tasks should have run");
    }

    #[tokio::test]
    async fn audit_chain_records_crew_lifecycle() {
        let orch = Orchestrator::new(Default::default()).await.unwrap();
        let spec = test_spec("audit-crew");
        orch.run_crew(spec).await.unwrap();

        // Audit chain should have recorded crew_accepted and crew_finished events.
        let entries = orch.audit().recent(10);
        assert!(
            entries.len() >= 2,
            "expected at least 2 audit entries (accepted + finished), got {}",
            entries.len()
        );

        let events: Vec<&str> = entries.iter().map(|e| e.event.as_str()).collect();
        assert!(
            events.contains(&"crew_accepted"),
            "missing crew_accepted event"
        );
        assert!(
            events.contains(&"crew_finished"),
            "missing crew_finished event"
        );

        // Verify chain integrity.
        let (valid, err) = orch.audit().verify();
        assert!(valid, "audit chain integrity failed: {err:?}");
    }

    #[tokio::test]
    async fn audit_chain_records_task_events() {
        let orch = Orchestrator::new(Default::default()).await.unwrap();

        // Two tasks → should get two task_completed audit entries.
        let agent = AgentDefinition::new("agent-a", "tester", "test things");
        let spec = CrewSpec::new("audit-tasks")
            .with_agents(vec![agent])
            .with_tasks(vec![Task::new("task A"), Task::new("task B")])
            .with_process(ProcessMode::Sequential);
        orch.run_crew(spec).await.unwrap();

        let entries = orch.audit().recent(20);
        let task_events: Vec<_> = entries
            .iter()
            .filter(|e| e.event == "task_completed")
            .collect();
        assert_eq!(
            task_events.len(),
            2,
            "expected 2 task_completed audit entries, got {}",
            task_events.len()
        );

        // All task entries should have info level (successful).
        for entry in &task_events {
            assert_eq!(entry.level, "info");
        }
    }

    #[tokio::test]
    async fn audit_chain_records_cancel_event() {
        let orch = Orchestrator::new(Default::default()).await.unwrap();
        let spec = test_spec("audit-cancel");
        let crew_id = spec.id;
        orch.run_crew(spec).await.unwrap();

        orch.cancel_crew(crew_id).await.unwrap();

        let entries = orch.audit().recent(10);
        let cancel_events: Vec<_> = entries
            .iter()
            .filter(|e| e.event == "crew_cancelled")
            .collect();
        assert_eq!(cancel_events.len(), 1);
        assert_eq!(cancel_events[0].level, "warn");
    }
}
