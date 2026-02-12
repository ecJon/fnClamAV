use std::path::PathBuf;
use std::process::Command;
use serde::{Deserialize, Serialize};
use crate::db::Database;

// ClamAV 二进制路径（注意：使用 .bin 后缀，包装脚本会处理路径）
pub fn get_clamscan_path() -> PathBuf {
    if let Ok(app_dest) = std::env::var("TRIM_APPDEST") {
        return PathBuf::from(app_dest).join("bin/clamscan.bin");
    }
    PathBuf::from("/usr/bin/clamscan")
}

pub fn get_freshclam_path() -> PathBuf {
    if let Ok(app_dest) = std::env::var("TRIM_APPDEST") {
        return PathBuf::from(app_dest).join("bin/freshclam.bin");
    }
    PathBuf::from("/usr/bin/freshclam")
}

// 病毒库目录
pub fn get_db_dir() -> PathBuf {
    if let Ok(data_share) = std::env::var("TRIM_DATA_SHARE_PATHS") {
        let first_path = data_share.split(':').next().unwrap_or("");
        if !first_path.is_empty() {
            return PathBuf::from(first_path).join("clamav");
        }
    }
    PathBuf::from("/var/lib/clamav")
}

// 扫描结果
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResult {
    pub scan_id: String,
    pub status: String,
    pub total_files: i64,
    pub scanned_files: i64,
    pub threats_found: i64,
    pub threats: Vec<ThreatInfo>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreatInfo {
    pub file_path: String,
    pub virus_name: String,
}

// 更新结果
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateResult {
    pub success: bool,
    pub old_version: Option<VirusVersion>,
    pub new_version: Option<VirusVersion>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VirusVersion {
    pub daily: String,
    pub main: String,
    pub bytecode: String,
}

// 生成扫描 ID
pub fn generate_scan_id() -> String {
    let now = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    format!("scan_{}", now)
}

// 扫描指定路径
pub fn scan_path(scan_id: &str, paths: &[String], db: &Database) -> ScanResult {
    let db_dir = get_db_dir();
    let clamscan_path = get_clamscan_path();

    // 检查 clamscan 是否存在
    if !clamscan_path.exists() {
        return ScanResult {
            scan_id: scan_id.to_string(),
            status: "error".to_string(),
            total_files: 0,
            scanned_files: 0,
            threats_found: 0,
            threats: Vec::new(),
            error_message: Some("ClamAV 扫描程序不存在".to_string()),
        };
    }

    let mut all_threats = Vec::new();
    let mut total_scanned = 0i64;

    // 扫描每个路径
    for path in paths {
        match scan_single_path(path, &db_dir, scan_id, db) {
            Ok(result) => {
                total_scanned += result.scanned_files;
                all_threats.extend(result.threats);
            }
            Err(e) => {
                return ScanResult {
                    scan_id: scan_id.to_string(),
                    status: "error".to_string(),
                    total_files: 0,
                    scanned_files: total_scanned,
                    threats_found: all_threats.len() as i64,
                    threats: all_threats,
                    error_message: Some(e.to_string()),
                };
            }
        }
    }

    ScanResult {
        scan_id: scan_id.to_string(),
        status: "completed".to_string(),
        total_files: total_scanned,
        scanned_files: total_scanned,
        threats_found: all_threats.len() as i64,
        threats: all_threats,
        error_message: None,
    }
}

// 扫描单个路径
fn scan_single_path(
    path: &str,
    db_dir: &PathBuf,
    scan_id: &str,
    db: &Database,
) -> Result<ScanResult, Box<dyn std::error::Error>> {
    let clamscan_path = get_clamscan_path();

    // 使用 output() 方法获取完整输出
    let output = Command::new(&clamscan_path)
        .arg(format!("--database={}", db_dir.display()))
        .arg("--recursive")
        .arg("--infected")
        .arg("--suppress-ok-results")
        .arg("--stdout")
        .arg(path)
        .output()?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    let mut threats = Vec::new();
    let scanned_count = 0i64;  // 由于使用 output()，无法实时统计

    // 解析输出
    for line in stdout_str.lines() {
        if line.contains("FOUND") {
            // 威胁文件格式: /path/to/file: VirusName FOUND
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() >= 2 {
                let file_path = parts[0].trim();
                let virus_part = parts[1].trim();
                let virus_name = virus_part
                    .split_whitespace()
                    .next()
                    .unwrap_or("Unknown");

                // 记录到数据库
                if let Ok(_threat_id) = db.add_threat(scan_id, file_path, virus_name) {
                    threats.push(ThreatInfo {
                        file_path: file_path.to_string(),
                        virus_name: virus_name.to_string(),
                    });
                }
            }
        }
        // 统计扫描的文件（通过 "-" 开头的行，这是 clamscan 的输出格式）
        // 或者简单地按输出行数估算
    }

    // 检查是否成功
    if !output.status.success() {
        return Ok(ScanResult {
            scan_id: scan_id.to_string(),
            status: "error".to_string(),
            total_files: scanned_count,
            scanned_files: scanned_count,
            threats_found: threats.len() as i64,
            threats,
            error_message: Some(format!("Scan failed: {}", stderr_str)),
        });
    }

    Ok(ScanResult {
        scan_id: scan_id.to_string(),
        status: "completed".to_string(),
        total_files: scanned_count,
        scanned_files: scanned_count,
        threats_found: threats.len() as i64,
        threats,
        error_message: None,
    })
}

// 更新病毒库
pub fn update_signatures() -> Result<UpdateResult, Box<dyn std::error::Error>> {
    let db_dir = get_db_dir();
    let freshclam_path = get_freshclam_path();

    if !freshclam_path.exists() {
        return Ok(UpdateResult {
            success: false,
            old_version: None,
            new_version: None,
            error_message: Some("freshclam 程序不存在".to_string()),
        });
    }

    // 获取当前版本
    let old_version = get_current_version(&db_dir)?;

    // 运行 freshclam
    let output = Command::new(&freshclam_path)
        .arg(format!("--datadir={}", db_dir.display()))
        .arg("--stdout")
        .output()?;

    if output.status.success() {
        let new_version = get_current_version(&db_dir)?;
        Ok(UpdateResult {
            success: true,
            old_version: Some(old_version),
            new_version: Some(new_version),
            error_message: None,
        })
    } else {
        let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
        Ok(UpdateResult {
            success: false,
            old_version: Some(old_version),
            new_version: None,
            error_message: Some(error_msg),
        })
    }
}

// 获取当前病毒库版本
fn get_current_version(db_dir: &PathBuf) -> Result<VirusVersion, Box<dyn std::error::Error>> {
    let daily = read_version_file(db_dir, "daily.cvd");
    let main = read_version_file(db_dir, "main.cvd");
    let bytecode = read_version_file(db_dir, "bytecode.cvd");

    Ok(VirusVersion {
        daily: daily.unwrap_or_else(|_| "unknown".to_string()),
        main: main.unwrap_or_else(|_| "unknown".to_string()),
        bytecode: bytecode.unwrap_or_else(|_| "unknown".to_string()),
    })
}

// 读取病毒库版本文件
fn read_version_file(db_dir: &PathBuf, filename: &str) -> Result<String, Box<dyn std::error::Error>> {
    let path = db_dir.join(filename);

    // 如果文件不存在，检查 .cld 文件
    let path = if !path.exists() {
        let alt_name = filename.replace(".cvd", ".cld");
        db_dir.join(&alt_name)
    } else {
        path
    };

    if !path.exists() {
        return Ok("not_found".to_string());
    }

    // 读取文件头获取版本号
    let content = std::fs::read(&path)?;
    if content.len() < 100 {
        return Ok("unknown".to_string());
    }

    // ClamAV 版本文件格式: ClamAV-VDB:DATE VERSION...
    let header = String::from_utf8_lossy(&content[..100]);
    if let Some(start) = header.find("ClamAV-VDB:") {
        let after = &header[start + 11..];
        let parts: Vec<&str> = after.split_whitespace().collect();
        if parts.len() >= 2 {
            return Ok(parts[1].to_string());
        }
    }

    Ok("unknown".to_string())
}
