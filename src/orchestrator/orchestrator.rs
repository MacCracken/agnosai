use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::{CrewSpec, CrewState, CrewStatus, ResourceBudget, Result};
use crate::server::sse::EventBus;

use crate::orchestrator::crew_runner::CrewRunner;
use crate::orchestrator::scheduler::Scheduler;

pub struct OrchestratorState {
    pub scheduler: Scheduler,
    pub active_crews: Vec<CrewState>,
}

pub struct Orchestrator {
    state: Arc<RwLock<OrchestratorState>>,
    budget: ResourceBudget,
    events: Option<EventBus>,
}

impl Orchestrator {
    pub async fn new(budget: ResourceBudget) -> Result<Self> {
        let state = OrchestratorState {
            scheduler: Scheduler::new(),
            active_crews: Vec::new(),
        };

        Ok(Self {
            state: Arc::new(RwLock::new(state)),
            budget,
            events: None,
        })
    }

    /// Attach an event bus for SSE streaming.
    pub fn with_events(mut self, events: EventBus) -> Self {
        self.events = Some(events);
        self
    }

    pub async fn run_crew(&self, spec: CrewSpec) -> Result<CrewState> {
        let crew_id = spec.id;

        // Register as pending.
        {
            let mut state = self.state.write().await;
            state.active_crews.push(CrewState {
                crew_id,
                status: CrewStatus::Pending,
                results: Vec::new(),
            });
        }

        // Delegate to CrewRunner for the actual lifecycle.
        let mut runner = CrewRunner::new(spec);
        if let Some(ref events) = self.events {
            runner = runner.with_events(events.sender(crew_id));
        }
        let crew_state = runner.run().await?;

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

    pub async fn cancel_crew(&self, crew_id: crate::core::crew::CrewId) -> Result<()> {
        let mut state = self.state.write().await;
        if let Some(crew) = state.active_crews.iter_mut().find(|c| c.crew_id == crew_id) {
            crew.status = CrewStatus::Cancelled;
            Ok(())
        } else {
            Err(crate::core::AgnosaiError::CrewNotFound(crew_id.to_string()))
        }
    }

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
