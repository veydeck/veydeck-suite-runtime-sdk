//! 套件语义操作事件构建与校验。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::{Error, OperationImpact, OperationOutcome, OperationTarget, ParameterValue, Result};

pub const OPERATION_CONTEXT_HEADER: &str = "x-veydeck-operation-context";

/// 从代理请求头值提取可信操作上下文 ID。
pub fn operation_context_from_header(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .map(ToOwned::to_owned)
}

/// 动态事件双语名称。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationEventLabel {
    pub zh_cn: String,
    pub en_us: String,
}

/// 套件提交给 Agent 的操作事件。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationEvent {
    pub event_id: String,
    pub operation_context_id: Option<String>,
    pub event_code: String,
    pub event_label: OperationEventLabel,
    pub outcome: OperationOutcome,
    pub impact: OperationImpact,
    pub target: Option<OperationTarget>,
    pub task_id: Option<String>,
    pub parameters: BTreeMap<String, ParameterValue>,
    pub error_code: Option<String>,
    pub error_summary: Option<String>,
}

/// 逐步构建并在提交前校验事件。
pub struct OperationEventBuilder(OperationEvent);

impl OperationEvent {
    /// 创建一个带 UUIDv7 幂等键的事件构建器。
    pub fn builder(
        event_code: impl Into<String>,
        zh_cn: impl Into<String>,
        en_us: impl Into<String>,
        outcome: OperationOutcome,
        impact: OperationImpact,
    ) -> OperationEventBuilder {
        OperationEventBuilder(Self {
            event_id: Uuid::now_v7().to_string(),
            operation_context_id: None,
            event_code: event_code.into(),
            event_label: OperationEventLabel {
                zh_cn: zh_cn.into(),
                en_us: en_us.into(),
            },
            outcome,
            impact,
            target: None,
            task_id: None,
            parameters: BTreeMap::new(),
            error_code: None,
            error_summary: None,
        })
    }

    /// 校验 Agent 接口公开约束。
    pub fn validate(&self) -> Result<()> {
        let id = Uuid::parse_str(&self.event_id)
            .map_err(|_| Error::InvalidEvent("eventId must be a UUIDv7".to_string()))?;
        if id.get_version_num() != 7 {
            return Err(Error::InvalidEvent("eventId must be a UUIDv7".to_string()));
        }
        if self
            .operation_context_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty() || value.len() > 128)
        {
            return Err(Error::InvalidEvent(
                "operationContextId must contain 1 to 128 characters".to_string(),
            ));
        }
        let code = self.event_code.as_bytes();
        if !(3..=64).contains(&code.len())
            || !code[0].is_ascii_lowercase()
            || !code
                .iter()
                .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || *value == b'_')
        {
            return Err(Error::InvalidEvent(
                "eventCode must use lower snake case".to_string(),
            ));
        }
        for label in [&self.event_label.zh_cn, &self.event_label.en_us] {
            if label.trim().is_empty() || label.chars().count() > 128 {
                return Err(Error::InvalidEvent(
                    "event labels must contain 1 to 128 characters".to_string(),
                ));
            }
        }
        if self.parameters.len() > 32 || self.parameters.keys().any(|key| sensitive_key(key)) {
            return Err(Error::InvalidEvent(
                "operation parameters contain unsupported fields".to_string(),
            ));
        }
        Ok(())
    }
}

impl OperationEventBuilder {
    pub fn operation_context_id(mut self, value: impl Into<String>) -> Self {
        self.0.operation_context_id = Some(value.into());
        self
    }

    pub fn target(mut self, target: OperationTarget) -> Self {
        self.0.target = Some(target);
        self
    }

    pub fn task_id(mut self, value: impl Into<String>) -> Self {
        self.0.task_id = Some(value.into());
        self
    }

    pub fn parameter(mut self, key: impl Into<String>, value: ParameterValue) -> Self {
        self.0.parameters.insert(key.into(), value);
        self
    }

    pub fn error(mut self, code: impl Into<String>, summary: impl AsRef<str>) -> Self {
        self.0.error_code = Some(code.into().chars().take(128).collect());
        self.0.error_summary = Some(redact_error(summary.as_ref()));
        self
    }

    pub fn build(self) -> Result<OperationEvent> {
        self.0.validate()?;
        Ok(self.0)
    }
}

fn sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    [
        "password",
        "token",
        "authorization",
        "secret",
        "cookie",
        "command",
        "environment",
    ]
    .iter()
    .any(|value| key.contains(value))
}

fn redact_error(value: &str) -> String {
    let mut value = value.replace(['\r', '\n'], " ");
    for marker in ["Bearer ", "token=", "password="] {
        if let Some(index) = value
            .to_ascii_lowercase()
            .find(&marker.to_ascii_lowercase())
        {
            value.truncate(index);
            value.push_str("[REDACTED]");
        }
    }
    value.chars().take(512).collect()
}
