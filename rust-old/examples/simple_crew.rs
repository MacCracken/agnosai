use agnosai::core::{CrewSpec, Task};
use agnosai::orchestrator::Orchestrator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let orchestrator = Orchestrator::new(Default::default()).await?;

    let mut crew = CrewSpec::new("example-crew");
    crew.tasks.push(Task::new("Analyze the project structure"));

    let result = orchestrator.run_crew(crew).await?;
    println!("Crew completed with status: {:?}", result.status);

    Ok(())
}
