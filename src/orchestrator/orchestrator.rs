use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::{CrewSpec, CrewState, CrewStatus, ResourceBudget, Result};

use crate::orchestrator::crew_runner::CrewRunner;
use crate::orchestrator::scheduler::Scheduler;

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
        let crew_state = runner.run().await?;

        // Update stored state.
        {
            let mut state = self.state.write().await;
            if let Some(entry) = state.active_crews.iter_mut().find(|c| c.crew_id == crew_id) {
                *entry = crew_state.clone();
            }
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
