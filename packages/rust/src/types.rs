//! 操作事件 wire 类型。

use serde::{Deserialize, Serialize};

/// 操作结果。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationOutcome {
    Success,
    Failure,
    Partial,
    Canceled,
    TimedOut,
}

/// 操作影响级别。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationImpact {
    Info,
    Warning,
    Error,
}

/// 可选业务目标。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationTarget {
    pub kind: String,
    pub id: String,
    pub display_name: Option<String>,
    pub ownership: Option<String>,
}

/// 允许进入审计事件的基础参数值。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParameterValue {
    String(String),
    Number(f64),
    Boolean(bool),
}
