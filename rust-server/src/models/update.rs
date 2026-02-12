use serde::{Deserialize, Serialize};

/// 更新状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateStatus {
    Idle,
    Updating,
    Error,
}

/// 更新频率
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateFrequency {
    Daily,
    Weekly,
    Manual,
}

/// 病毒库版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirusVersion {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub main: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytecode: Option<String>,
}

/// 更新请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force: Option<bool>,
}

/// 更新响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResponse {
    pub success: bool,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 更新状态响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStatusResponse {
    pub status: String,
    pub current_version: VirusVersion,
    pub last_update: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_scheduled: Option<i64>,
    pub update_frequency: String,
}

/// 更新历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateHistory {
    pub id: i64,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub result: String,
    pub old_version: Option<String>,
    pub new_version: Option<String>,
    pub error_message: Option<String>,
}
