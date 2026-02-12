use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 隔离文件列表响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineListResponse {
    pub total: u32,
    pub total_size_bytes: u64,
    pub items: Vec<QuarantineItem>,
}

/// 隔离文件项目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineItem {
    pub uuid: String,
    pub original_path: String,
    pub original_name: String,
    pub file_size: u64,
    pub virus_name: String,
    pub quarantined_at: i64,
    pub scan_id: String,
}

/// 隔离文件元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineMetadata {
    pub uuid: String,
    pub original_path: String,
    pub original_name: String,
    pub file_size: u64,
    pub file_hash: Option<String>,
    pub quarantined_at: i64,
    pub virus_name: String,
    pub scan_id: String,
}

/// 恢复响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineRestoreResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restored_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 清理响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineCleanupResponse {
    pub success: bool,
    pub cleaned_count: u32,
    pub freed_bytes: u64,
}

impl QuarantineMetadata {
    pub fn new(
        original_path: String,
        virus_name: String,
        scan_id: String,
        file_size: u64,
    ) -> Self {
        let uuid = Uuid::new_v4().to_string();
        let original_name = original_path
            .rsplit('/')
            .next()
            .unwrap_or("unknown")
            .to_string();

        Self {
            uuid,
            original_path,
            original_name,
            file_size,
            file_hash: None,
            quarantined_at: chrono::Utc::now().timestamp(),
            virus_name,
            scan_id,
        }
    }
}
