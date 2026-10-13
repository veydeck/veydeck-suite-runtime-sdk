//! Runtime 描述驱动的 Agent HTTP 客户端。

use reqwest::{Method, Response};
use serde::Serialize;
use std::{env, path::Path, time::Duration};
use tokio::time::sleep;

use crate::{Error, OperationEvent, Result, RuntimeDescriptor, RuntimeEndpoint};

const DEFAULT_RUNTIME_PATH: &str = "/run/veydeck-agent/runtime.json";
const OPERATION_EVENT_PATH: &str = "/api/v1/agent/suite-runtime/operation-events";
const MAX_ATTEMPTS: usize = 3;

/// 已认证的套件 Runtime 客户端。
pub struct RuntimeClient {
    descriptor: RuntimeDescriptor,
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl RuntimeClient {
    /// 从环境变量或默认位置加载 Runtime 描述。
    pub async fn from_environment(required_capability: &str) -> Result<Self> {
        let path =
            env::var("VEYDECK_AGENT_RUNTIME").unwrap_or_else(|_| DEFAULT_RUNTIME_PATH.into());
        Self::from_path(Path::new(&path), required_capability).await
    }

    /// 从指定描述文件创建客户端，便于测试和嵌入式运行。
    pub async fn from_path(path: &Path, required_capability: &str) -> Result<Self> {
        let descriptor = RuntimeDescriptor::load(path).await?;
        if let (Ok(suite_id), Ok(instance_id)) = (
            env::var("VEYDECK_SUITE_ID"),
            env::var("VEYDECK_SUITE_INSTANCE_ID"),
        ) {
            descriptor.require_identity(&suite_id, &instance_id)?;
        }
        descriptor.require_capability(required_capability)?;
        let token = tokio::fs::read_to_string(&descriptor.credential.token_path).await?;
        let token = token.trim().to_string();
        if token.is_empty() {
            return Err(Error::InvalidCredential("token is empty".to_string()));
        }
        let (http, base_url) = build_transport(&descriptor.endpoint).await?;
        Ok(Self {
            descriptor,
            http,
            base_url,
            token,
        })
    }

    pub fn descriptor(&self) -> &RuntimeDescriptor {
        &self.descriptor
    }

    /// 发送带套件令牌的通用 Agent 请求。
    pub async fn request<B: Serialize + ?Sized>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<Response> {
        let mut request = self
            .http
            .request(method, self.url(path))
            .bearer_auth(&self.token);
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().await?;
        ensure_success(response).await
    }

    /// 提交语义操作事件；重试始终复用事件内的 UUIDv7。
    pub async fn submit_operation_event(&self, event: &OperationEvent) -> Result<()> {
        event.validate()?;
        for attempt in 0..MAX_ATTEMPTS {
            let result = self
                .http
                .post(self.url(OPERATION_EVENT_PATH))
                .bearer_auth(&self.token)
                .json(event)
                .send()
                .await;
            match result {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(response) if response.status().is_client_error() => {
                    return Err(agent_error(response).await);
                }
                Ok(response) if attempt + 1 == MAX_ATTEMPTS => {
                    return Err(agent_error(response).await);
                }
                Err(error) if attempt + 1 == MAX_ATTEMPTS => return Err(Error::Transport(error)),
                Ok(_) | Err(_) => sleep(Duration::from_millis(100 * (1 << attempt))).await,
            }
        }
        unreachable!("bounded retry loop always returns")
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }
}

async fn build_transport(endpoint: &RuntimeEndpoint) -> Result<(reqwest::Client, String)> {
    match endpoint {
        RuntimeEndpoint::Unix {
            socket_path,
            base_url,
        } => Ok((
            reqwest::Client::builder()
                .unix_socket(socket_path.as_path())
                .no_proxy()
                .build()?,
            base_url.clone(),
        )),
        RuntimeEndpoint::Https {
            base_url,
            ca_path,
            client_cert_path,
            client_key_path,
        } => {
            let ca = reqwest::Certificate::from_pem(&tokio::fs::read(ca_path).await?)?;
            let mut identity = tokio::fs::read(client_cert_path).await?;
            identity.extend_from_slice(&tokio::fs::read(client_key_path).await?);
            let identity = reqwest::Identity::from_pem(&identity)?;
            Ok((
                reqwest::Client::builder()
                    .add_root_certificate(ca)
                    .identity(identity)
                    .https_only(true)
                    .build()?,
                base_url.clone(),
            ))
        }
    }
}

async fn ensure_success(response: Response) -> Result<Response> {
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(agent_error(response).await)
    }
}

async fn agent_error(response: Response) -> Error {
    let status = response.status().as_u16();
    let message = response.text().await.unwrap_or_default();
    Error::Agent {
        status,
        message: message
            .replace(['\r', '\n'], " ")
            .chars()
            .take(512)
            .collect(),
    }
}
