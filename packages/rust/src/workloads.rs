//! 套件受控工作负载与流量取证 API。

use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Result, RuntimeClient};

const WORKLOADS_PATH: &str = "/api/v1/agent/suite-runtime/workloads";

/// 工作负载端点使用的传输协议。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkloadTransport {
    Tcp,
    Udp,
}

impl WorkloadTransport {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tcp => "tcp",
            Self::Udp => "udp",
        }
    }
}

/// 工作负载公开的命名端点。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkloadPort {
    pub endpoint_id: String,
    pub host_port: u16,
    pub container_port: u16,
    pub protocol: WorkloadTransport,
}

/// 工作负载资源限制。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkloadResources {
    pub memory_mb: Option<u32>,
    pub cpu_shares: Option<u32>,
}

/// 创建受控工作负载。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartWorkloadRequest {
    pub workload_kind: String,
    pub workload_name: String,
    pub image: String,
    #[serde(default)]
    pub ports: Vec<WorkloadPort>,
    #[serde(default)]
    pub env: Value,
    #[serde(default)]
    pub config_json: Value,
    #[serde(default)]
    pub resources: WorkloadResources,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartWorkloadResponse {
    pub workload_id: String,
    pub container_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkloadSummary {
    pub workload_id: String,
    pub suite_id: String,
    pub suite_instance_id: String,
    pub workload_kind: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub container_id: String,
}

/// 抓包实际覆盖的工作负载端点。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureEndpoint {
    pub endpoint_id: String,
    pub host_port: u16,
    pub protocol: WorkloadTransport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartCaptureResponse {
    pub capture_id: String,
    pub status: String,
    pub endpoints: Vec<CaptureEndpoint>,
}

impl RuntimeClient {
    /// 创建属于当前套件实例的受控工作负载。
    pub async fn start_workload(
        &self,
        payload: &StartWorkloadRequest,
    ) -> Result<StartWorkloadResponse> {
        self.request(Method::POST, WORKLOADS_PATH, Some(payload))
            .await?
            .json()
            .await
            .map_err(Into::into)
    }

    /// 列出属于当前套件实例的工作负载。
    pub async fn list_workloads(&self) -> Result<Vec<WorkloadSummary>> {
        self.request::<Value>(Method::GET, WORKLOADS_PATH, None)
            .await?
            .json()
            .await
            .map_err(Into::into)
    }

    /// 删除属于当前套件实例的工作负载。
    pub async fn delete_workload(&self, workload_id: &str) -> Result<()> {
        self.request::<Value>(
            Method::DELETE,
            &format!("{WORKLOADS_PATH}/{workload_id}"),
            None,
        )
        .await?;
        Ok(())
    }

    /// 为工作负载的全部公开端点启动一次抓包。
    pub async fn start_capture(&self, workload_id: &str) -> Result<StartCaptureResponse> {
        self.request::<Value>(
            Method::POST,
            &format!("{WORKLOADS_PATH}/{workload_id}/captures"),
            None,
        )
        .await?
        .json()
        .await
        .map_err(Into::into)
    }

    /// 停止抓包并返回 PCAP 字节。
    pub async fn finish_capture(&self, workload_id: &str, capture_id: &str) -> Result<Vec<u8>> {
        Ok(self
            .request::<Value>(
                Method::POST,
                &format!("{WORKLOADS_PATH}/{workload_id}/captures/{capture_id}/finish"),
                None,
            )
            .await?
            .bytes()
            .await?
            .to_vec())
    }
}
