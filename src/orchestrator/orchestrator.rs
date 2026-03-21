use std::sync::{Arc, OnceLock};
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, info, warn};

use crate::core::{CrewSpec, CrewState, CrewStatus, ResourceBudget, Result};
use crate::llm::{CostTracker, HooshClient, ResponseCache};
use crate::server::sse::EventBus;

use crate::orchestrator::crew_runner::CrewRunner;
use crate::orchestrator::scheduler::Scheduler;

/// Maximum number of completed crews retained in memory.
const MAX_RETAINED_CREWS: usize = 1000;

/// Internal mutable state of the orchestrator.
pub struct OrchestratorState {
    /// Task scheduler instance.
    pub scheduler: Scheduler,
    /// Crews currently tracked (active and recently completed).
    pub active_crews: Vec<CrewState>,
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
}

impl Orchestrator {
    /// Create a new orchestrator with the given resource budget.
    pub async fn new(budget: ResourceBudget) -> Result<Self> {
        let max_concurrent = budget.max_concurrent_tasks.unwrap_or(10);
        let state = OrchestratorState {
            scheduler: Scheduler::new(),
            active_crews: Vec::new(),
        };

        Ok(Self {
            state: Arc::new(RwLock::new(state)),
            budget,
            events: None,
            llm: OnceLock::new(),
            llm_url: None,
            cache: Arc::new(ResponseCache::new(Default::default())),
            cost_tracker: Arc::new(CostTracker::new()),
            crew_semaphore: Arc::new(Semaphore::new(max_concurrent)),
        })
    }

    /// Attach an event bus for SSE streaming.
    pub fn with_events(mut self, events: EventBus) -> Self {
        self.events = Some(events);
        self
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

        // Delegate to CrewRunner for the actual lifecycle.
        let mut runner = CrewRunner::new(spec)
            .with_cache(Arc::clone(&self.cache))
            .with_cost_tracker(Arc::clone(&self.cost_tracker));
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

        // Update stored state.
        {
            let mut state = self.state.write().await;
            if let Some(entry) = state.active_crews.iter_mut().find(|c| c.crew_id == crew_id) {
                *entry = crew_state.clone();
            }
        }

        // Clean up the event channel now that the crew is done.
        if let Some(ref events) = self.events {
            events.remove(crew_id);
        }

        Ok(crew_state)
    }

    /// Cancel a running crew by ID.
    pub async fn cancel_crew(&self, crew_id: crate::core::crew::CrewId) -> Result<()> {
        let mut state = self.state.write().await;
        if let Some(crew) = state.active_crews.iter_mut().find(|c| c.crew_id == crew_id) {
            crew.status = CrewStatus::Cancelled;
            info!(crew_id = %crew_id, "crew cancelled");
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
}
