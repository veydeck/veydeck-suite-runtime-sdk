use veydeck_suite_runtime::{OperationEvent, OperationImpact, OperationOutcome, RuntimeClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = RuntimeClient::from_environment("operation-logs.write").await?;
    let event = OperationEvent::builder(
        "task_submitted",
        "提交任务",
        "Submit task",
        OperationOutcome::Success,
        OperationImpact::Info,
    )
    .build()?;
    client.submit_operation_event(&event).await?;
    Ok(())
}
