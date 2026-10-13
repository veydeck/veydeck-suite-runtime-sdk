//! Agent 注入的套件 Runtime 描述。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// 套件 Runtime 描述。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDescriptor {
    pub schema_version: u32,
    pub platform_version: String,
    pub suite_id: String,
    pub instance_id: String,
    pub endpoint: RuntimeEndpoint,
    pub credential: RuntimeCredential,
    pub capabilities: Vec<String>,
}

/// Agent 连接端点。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
pub enum RuntimeEndpoint {
    Unix {
        socket_path: PathBuf,
        base_url: String,
    },
    Https {
        base_url: String,
        ca_path: PathBuf,
        client_cert_path: PathBuf,
        client_key_path: PathBuf,
    },
}

/// 套件实例令牌位置。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCredential {
    pub token_path: PathBuf,
}

impl RuntimeDescriptor {
    /// 读取并校验 Runtime 描述。
    pub async fn load(path: &Path) -> Result<Self> {
        let value = tokio::fs::read_to_string(path).await?;
        let descriptor: Self = serde_json::from_str(&value)?;
        descriptor.validate()?;
        Ok(descriptor)
    }

    /// 要求 Agent 授予指定能力。
    pub fn require_capability(&self, capability: &str) -> Result<()> {
        if self.capabilities.iter().any(|value| value == capability) {
            Ok(())
        } else {
            Err(Error::CapabilityDenied(capability.to_string()))
        }
    }

    /// 校验调用进程声明的套件和实例身份与 Agent 描述一致。
    pub fn require_identity(&self, suite_id: &str, instance_id: &str) -> Result<()> {
        if self.suite_id == suite_id && self.instance_id == instance_id {
            Ok(())
        } else {
            Err(Error::InvalidDescriptor(
                "runtime descriptor identity does not match suite instance".to_string(),
            ))
        }
    }

    fn validate(&self) -> Result<()> {
        if self.schema_version != 1
            || semver::Version::parse(&self.platform_version).is_err()
            || self.suite_id.is_empty()
            || self.instance_id.is_empty()
        {
            return Err(Error::InvalidDescriptor(
                "schema version and instance identity are required".to_string(),
            ));
        }
        if self
            .capabilities
            .iter()
            .any(|value| value.trim().is_empty())
        {
            return Err(Error::InvalidDescriptor(
                "capabilities must not contain empty values".to_string(),
            ));
        }
        Ok(())
    }
}
