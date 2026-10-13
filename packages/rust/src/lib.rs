//! Veydeck 套件后端访问 Agent 受信 Runtime API 的客户端。

mod client;
mod descriptor;
mod error;
mod operation_logs;
mod types;
mod workloads;

pub use client::RuntimeClient;
pub use descriptor::{RuntimeCredential, RuntimeDescriptor, RuntimeEndpoint};
pub use error::{Error, Result};
pub use operation_logs::{
    OPERATION_CONTEXT_HEADER, OperationEvent, OperationEventBuilder, operation_context_from_header,
};
pub use types::{OperationImpact, OperationOutcome, OperationTarget, ParameterValue};
pub use workloads::{
    CaptureEndpoint, StartCaptureResponse, StartWorkloadRequest, StartWorkloadResponse,
    WorkloadPort, WorkloadResources, WorkloadSummary, WorkloadTransport,
};
