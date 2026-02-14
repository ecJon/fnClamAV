// ClamAV 相关类型定义
//
// 此文件定义了 ClamAV FFI 中使用的各种数据结构

use std::fmt;

/// 病毒名称
#[derive(Debug, Clone, PartialEq)]
pub struct VirusName(pub String);

impl fmt::Display for VirusName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 扫描的文件路径
#[derive(Debug, Clone, PartialEq)]
pub struct FilePath(pub String);

impl fmt::Display for FilePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 扫描进度百分比 (0-100)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressPercent(pub u8);

impl fmt::Display for ProgressPercent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", self.0)
    }
}

/// 扫描的文件数量
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScannedFiles(pub u32);

impl fmt::Display for ScannedFiles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 文件总数
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TotalFiles(pub u32);

impl fmt::Display for TotalFiles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 发现的威胁数量
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThreatsFound(pub u32);

impl fmt::Display for ThreatsFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 扫描状态
#[derive(Debug, Clone, PartialEq)]
pub enum ScanStatus {
    Idle,
    Scanning,
    Paused,
    Stopping,
    Completed,
    Failed(String),
}

impl fmt::Display for ScanStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScanStatus::Idle => write!(f, "空闲"),
            ScanStatus::Scanning => write!(f, "扫描中"),
            ScanStatus::Paused => write!(f, "已暂停"),
            ScanStatus::Stopping => write!(f, "停止中"),
            ScanStatus::Completed => write!(f, "已完成"),
            ScanStatus::Failed(ref msg) => write!(f, "失败: {}", msg),
        }
    }
}

/// 发现的文件数量（用于两线程模式的进度显示）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiscoveredFiles(pub u32);

impl fmt::Display for DiscoveredFiles {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 扫描速率（文件/秒）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScanRate(pub f32);

impl fmt::Display for ScanRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} 文件/秒", self.0)
    }
}

/// 扫描进度信息
#[derive(Debug, Clone)]
pub struct ScanProgress {
    pub percent: ProgressPercent,
    pub scanned_files: ScannedFiles,
    pub total_files: TotalFiles,
    pub threats_found: ThreatsFound,
    pub current_file: Option<FilePath>,
    /// 已发现的文件数（两线程模式：发现线程持续更新）
    pub discovered_files: DiscoveredFiles,
    /// 扫描速率（文件/秒，基于 EMA 计算）
    pub scan_rate: Option<ScanRate>,
}

impl ScanProgress {
    pub fn new() -> Self {
        Self {
            percent: ProgressPercent(0),
            scanned_files: ScannedFiles(0),
            total_files: TotalFiles(0),
            threats_found: ThreatsFound(0),
            current_file: None,
            discovered_files: DiscoveredFiles(0),
            scan_rate: None,
        }
    }
}

/// 扫描结果
#[derive(Debug, Clone)]
pub struct ScanOutcome {
    pub total_files: u32,
    pub scanned_files: u32,
    pub threats: Vec<(FilePath, VirusName)>,
    pub status: ScanStatus,
    pub error_message: Option<String>,
}

impl ScanOutcome {
    pub fn success(total_files: u32, scanned_files: u32, threats: Vec<(FilePath, VirusName)>) -> Self {
        Self {
            total_files,
            scanned_files,
            threats,
            status: ScanStatus::Completed,
            error_message: None,
        }
    }

    pub fn failed(message: String) -> Self {
        Self {
            total_files: 0,
            scanned_files: 0,
            threats: vec![],
            status: ScanStatus::Failed(message.clone()),
            error_message: Some(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_default() {
        let progress = ScanProgress::new();
        assert_eq!(progress.percent.0, 0);
        assert_eq!(progress.scanned_files.0, 0);
        assert_eq!(progress.threats_found.0, 0);
        assert!(progress.current_file.is_none());
        assert_eq!(progress.discovered_files.0, 0);
        assert!(progress.scan_rate.is_none());
    }

    #[test]
    fn test_progress_update() {
        let mut progress = ScanProgress::new();
        progress.percent = ProgressPercent(50);
        progress.scanned_files = ScannedFiles(100);
        progress.threats_found = ThreatsFound(2);
        progress.current_file = Some(FilePath("/test/file.txt".to_string()));
        progress.discovered_files = DiscoveredFiles(200);
        progress.scan_rate = Some(ScanRate(45.5));

        assert_eq!(progress.percent, ProgressPercent(50));
        assert_eq!(progress.scanned_files, ScannedFiles(100));
        assert_eq!(progress.threats_found, ThreatsFound(2));
        assert_eq!(progress.current_file, Some(FilePath("/test/file.txt".to_string())));
        assert_eq!(progress.discovered_files, DiscoveredFiles(200));
        assert_eq!(progress.scan_rate, Some(ScanRate(45.5)));
    }

    #[test]
    fn test_scan_status_display() {
        assert_eq!(format!("{}", ScanStatus::Idle), "空闲");
        assert_eq!(format!("{}", ScanStatus::Scanning), "扫描中");
        assert_eq!(format!("{}", ScanStatus::Paused), "已暂停");
        assert_eq!(format!("{}", ScanStatus::Completed), "已完成");
        assert_eq!(format!("{}", ScanStatus::Failed("error".to_string())), "失败: error");
    }

    #[test]
    fn test_virus_name_display() {
        let name = VirusName("Eicar-Test-Signature".to_string());
        assert_eq!(format!("{}", name), "Eicar-Test-Signature");
    }
}
