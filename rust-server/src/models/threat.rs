use serde::{Deserialize, Serialize};

/// 威胁列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatsListResponse {
    pub total: u32,
    pub items: Vec<ThreatItem>,
}

/// 威胁项目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatItem {
    pub id: i64,
    pub scan_id: String,
    pub file_path: String,
    pub virus_name: String,
    /// 检测时间（使用 action_time 或当前时间）
    pub detected_time: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_taken: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quarantine_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_time: Option<i64>,
}

/// 威胁处理请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatHandleRequest {
    pub action: String,  // "quarantine" | "delete" | "ignore"
}

/// 威胁处理响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatHandleResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threat: Option<ThreatItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
