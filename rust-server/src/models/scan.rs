use serde::{Deserialize, Serialize};

/// 扫描状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanStatus {
    Idle,
    Scanning,
    Completed,
    Stopped,
    Error,
}

/// 扫描类型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScanType {
    Full,
    Custom,
}

/// 扫描请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRequest {
    pub scan_type: ScanType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<String>>,
}

/// 扫描响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 扫描状态响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStatusResponse {
    pub scan_id: Option<String>,
    pub status: String,
    pub progress: Option<ScanProgress>,
    pub threats: Option<ThreatsInfo>,
    pub start_time: Option<i64>,
    pub elapsed_seconds: Option<u64>,
}

/// 扫描进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub percent: f32,
    pub scanned: u64,
    pub estimated_total: u64,
    pub current_file: String,
    /// 已发现的文件数（两线程模式：发现线程持续更新）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovered: Option<u64>,
    /// 扫描速率（文件/秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scan_rate: Option<f32>,
}

/// 威胁信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatsInfo {
    pub count: u32,
    pub files: Vec<ThreatFile>,
}

/// 威胁文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatFile {
    pub path: String,
    pub virus: String,
    pub action: String,
}

/// 扫描历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanHistory {
    pub id: i64,
    pub scan_id: String,
    pub scan_type: String,
    pub paths: String,
    pub status: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub total_files: i32,
    pub scanned_files: i32,
    pub threats_found: i32,
    pub error_message: Option<String>,
}

/// 威胁记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatRecord {
    pub id: i64,
    pub scan_id: String,
    pub file_path: String,
    pub virus_name: String,
    pub action_taken: Option<String>,
    pub action_time: Option<i64>,
    pub original_location: Option<String>,
    pub file_hash: Option<String>,
}
