use serde::{Deserialize, Serialize};

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub scan: ScanConfig,
    pub threat: ThreatConfig,
    pub update: UpdateConfig,
    pub history: HistoryConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            scan: ScanConfig::default(),
            threat: ThreatConfig::default(),
            update: UpdateConfig::default(),
            history: HistoryConfig::default(),
        }
    }
}

/// 扫描配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub default_scan_type: String,
    pub exclude_paths: Vec<String>,
    pub max_file_size_mb: u32,
    pub scan_archives: bool,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            default_scan_type: "full".to_string(),
            exclude_paths: vec![
                "/proc".to_string(),
                "/sys".to_string(),
                "/dev".to_string(),
                "/run".to_string(),
            ],
            max_file_size_mb: 100,
            scan_archives: true,
        }
    }
}

/// 威胁处理配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatConfig {
    pub action: String,  // "quarantine" | "delete" | "none"
    pub auto_action: bool,
}

impl Default for ThreatConfig {
    fn default() -> Self {
        Self {
            action: "quarantine".to_string(),
            auto_action: false,
        }
    }
}

/// 更新配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    pub frequency: String,  // "daily" | "weekly" | "manual"
    pub schedule_time: String,  // "HH:MM"
    pub timezone: String,
    pub auto_check: bool,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            frequency: "daily".to_string(),
            schedule_time: "03:30".to_string(),
            timezone: "Asia/Shanghai".to_string(),
            auto_check: true,
        }
    }
}

/// 历史配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    pub retention_days: u32,
    pub max_records: u32,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            retention_days: 90,
            max_records: 1000,
        }
    }
}

/// 配置响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub scan: ScanConfig,
    pub threat: ThreatConfig,
    pub update: UpdateConfig,
    pub history: HistoryConfig,
}

/// ClamAV 引擎配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClamAVConfig {
    /// 病毒库目录
    pub database_dir: String,
    /// 证书目录
    pub certs_dir: Option<String>,
    /// libclamav.so 路径
    pub lib_path: Option<String>,
    /// 最大扫描线程数
    pub max_threads: u32,
    /// 扫描超时时间（秒）
    pub scan_timeout: u32,
    /// 是否启用启发式扫描
    pub heuristic_scan: bool,
}

impl Default for ClamAVConfig {
    fn default() -> Self {
        Self {
            database_dir: "/var/lib/clamav".to_string(),
            certs_dir: None,
            lib_path: None,
            max_threads: 4,
            scan_timeout: 300,
            heuristic_scan: true,
        }
    }
}
