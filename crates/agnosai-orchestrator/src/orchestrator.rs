use std::sync::Arc;
use tokio::sync::RwLock;

use agnosai_core::{CrewSpec, CrewState, CrewStatus, ResourceBudget, Result};

use crate::scheduler::Scheduler;

pub struct OrchestratorState {
    pub scheduler: Scheduler,
    pub active_crews: Vec<CrewState>,
}

pub struct Orchestrator {
    state: Arc<RwLock<OrchestratorState>>,
    budget: ResourceBudget,
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
        })
    }

    pub async fn run_crew(&self, spec: CrewSpec) -> Result<CrewState> {
        let crew_state = CrewState {
            crew_id: spec.id,
            status: CrewStatus::Pending,
            results: Vec::new(),
        };

        {
            let mut state = self.state.write().await;
            state.active_crews.push(crew_state.clone());
        }

        // TODO: Phase 1 — implement crew lifecycle:
        // 1. Resolve task DAG (topological sort)
        // 2. Score and assign agents
        // 3. Execute tasks respecting dependencies
        // 4. Aggregate results

        Ok(crew_state)
    }

    pub async fn cancel_crew(&self, crew_id: agnosai_core::crew::CrewId) -> Result<()> {
        let mut state = self.state.write().await;
        if let Some(crew) = state
            .active_crews
            .iter_mut()
            .find(|c| c.crew_id == crew_id)
        {
            crew.status = CrewStatus::Cancelled;
            Ok(())
        } else {
            Err(agnosai_core::AgnosaiError::CrewNotFound(
                crew_id.to_string(),
            ))
        }
    }

    pub fn budget(&self) -> &ResourceBudget {
        &self.budget
    }
}
