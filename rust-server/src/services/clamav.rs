use crate::env::FnosEnv;
use std::process::{Command, Stdio, Child};
use std::io::{BufRead, BufReader};
use std::sync::Arc;
use tokio::sync::RwLock;

/// ClamAV 服务
#[derive(Clone)]
pub struct ClamavService {
    env: FnosEnv,
    current_scan_pid: Arc<RwLock<Option<u32>>>,
}

impl ClamavService {
    pub fn new(env: FnosEnv) -> Self {
        Self {
            env,
            current_scan_pid: Arc::new(RwLock::new(None)),
        }
    }

    /// 执行扫描
    pub async fn scan(
        &self,
        scan_id: String,
        paths: Vec<String>,
        progress_callback: Arc<dyn Fn(String, i32, u32, Option<String>) + Send + Sync>,
    ) -> Result<ScanResult, String> {
        let clamscan_bin = self.env.clamscan_bin();

        // 检查二进制文件是否存在
        if !std::path::Path::new(&clamscan_bin).exists() {
            return Err(format!("ClamAV binary not found: {}", clamscan_bin));
        }

        // 构建命令 - 移除 --suppress-ok-results 以获取所有文件的扫描进度
        let mut cmd = Command::new(&clamscan_bin);
        // 移除 --infected 以获取所有文件的扫描进度
        // 注意：这会产生大量输出，但能实时显示扫描进度
        cmd.env("TRIM_APPDEST", &self.env.app_dest)
            .env("TRIM_DATA_SHARE_PATHS", &self.env.data_dir())
            .arg("--recursive")
            .arg("--stdout");

        // 添加扫描路径
        for path in &paths {
            cmd.arg(path);
        }

        tracing::info!("Starting scan: {:?}", cmd);

        // 执行命令并读取输出
        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start clamscan: {}", e))?;

        // 保存进程ID用于停止扫描
        let pid = child.id();
        *self.current_scan_pid.write().await = Some(pid as u32);
        tracing::info!("Started scan with PID: {}", pid);

        let stdout = child.stdout.take().ok_or("Failed to get stdout")?;
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        let mut total_files = 0u32;
        let mut threats = Vec::new();
        let mut current_file: Option<String> = None;

        while let Some(Ok(line)) = lines.next() {
            tracing::debug!("Scan output: {}", line);

            // 解析输出行 - ClamAV 输出格式：
            // /path/to/file: OK
            // /path/to/file: VirusName FOUND
            if let Some((file_path, status)) = self.parse_scan_line(&line) {
                current_file = Some(file_path.clone());
                total_files += 1;

                if status == "FOUND" {
                    // 提取病毒名称
                    if let Some((_, virus_name)) = self.parse_threat_line(&line) {
                        threats.push((file_path, virus_name));
                    }
                }

                // 每扫描一个文件就报告进度
                progress_callback(
                    scan_id.clone(),
                    total_files as i32,
                    threats.len() as u32,
                    current_file.clone(),
                );
            }
        }

        // 等待进程结束（无论正常完成还是被杀死）
        let status = child.wait().map_err(|e| format!("Failed to wait for clamscan: {}", e))?;

        // 清除PID
        *self.current_scan_pid.write().await = None;
        let status = child.wait().map_err(|e| format!("Failed to wait for clamscan: {}", e))?;

        // ClamAV 退出码含义:
        // 0 = 无病毒发现
        // 1 = 发现病毒
        // 其他 = 真正的错误
        let exit_code = status.code().unwrap_or(-1);
        let success = exit_code == 0 || exit_code == 1;  // 0和1都是成功扫描

        Ok(ScanResult {
            scan_id,
            success,
            total_files,
            threats_found: threats.len() as u32,
            threats,
            error_message: if !success {
                Some(format!("扫描失败，错误代码: {}", exit_code))
            } else {
                None
            },
        })
    }

    fn parse_threat_line(&self, line: &str) -> Option<(String, String)> {
        // 格式: /path/to/file: VirusName FOUND
        let parts: Vec<&str> = line.rsplitn(3, ' ').collect();
        if parts.len() >= 3 {
            let virus_name = parts[1].to_string();
            let file_path = parts[2].trim_end_matches(':').to_string();
            Some((file_path, virus_name))
        } else {
            None
        }
    }

    /// 解析扫描输出行，返回 (文件路径, 状态)
    /// 状态可能是 "OK" 或 "FOUND"
    fn parse_scan_line(&self, line: &str) -> Option<(String, String)> {
        // 跳过空行和非扫描结果行
        if line.is_empty()
            || line.starts_with("---")
            || line.starts_with("LibClamAV")
            || line.starts_with("Known viruses")
            || line.starts_with("Engine version")
            || line.starts_with("Scanned directories")
            || line.starts_with("Scanned files")
            || line.starts_with("Infected files")
            || line.starts_with("Data scanned")
            || line.starts_with("Data read")
            || line.starts_with("Time:")
            || line.starts_with("Start Date")
            || line.starts_with("End Date")
            || line.contains("ERROR")
        {
            return None;
        }

        // 格式: /path/to/file: OK 或 /path/to/file: VirusName FOUND
        if let Some(colon_pos) = line.rfind(':') {
            let file_path = line[..colon_pos].trim().to_string();
            let status_part = line[colon_pos + 1..].trim();

            if status_part.ends_with("OK") {
                return Some((file_path, "OK".to_string()));
            } else if status_part.ends_with("FOUND") {
                return Some((file_path, "FOUND".to_string()));
            }
        }

        None
    }

    /// 更新病毒库
    pub async fn update(&self) -> Result<UpdateResult, String> {
        let db_dir = self.env.clamav_db_dir();
        let freshclam_bin = self.env.freshclam_bin();

        // 检查二进制文件是否存在
        if !std::path::Path::new(&freshclam_bin).exists() {
            return Err(format!("Freshclam binary not found: {}", freshclam_bin));
        }

        // 构建命令
        let output = Command::new(&freshclam_bin)
            .env("TRIM_APPDEST", &self.env.app_dest)
            .env("TRIM_DATA_SHARE_PATHS", &self.env.data_dir())
            .arg("--stdout")
            .arg("--no-warnings")
            .output()
            .map_err(|e| format!("Failed to run freshclam: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        tracing::info!("Freshclam output: {}", stdout);
        if !stderr.is_empty() {
            tracing::warn!("Freshclam stderr: {}", stderr);
        }

        // 解析输出
        let (old_version, new_version) = self.parse_update_output(&stdout);

        Ok(UpdateResult {
            success: output.status.success(),
            old_version,
            new_version,
            error_message: if !output.status.success() {
                Some(format!("Exit code: {:?}", output.status.code()))
            } else {
                None
            },
        })
    }

    fn parse_update_output(&self, output: &str) -> (Option<String>, Option<String>) {
        // 简化版本解析，实际需要更复杂的解析
        let old_version = None;
        let new_version = None;

        // TODO: 解析 freshclam 输出获取版本信息

        (old_version, new_version)
    }

    /// 停止扫描（通过 PID）
    pub async fn stop_scan(&self, scan_id: &str) -> Result<(), String> {
        // 读取当前扫描的PID
        let pid = self.current_scan_pid.read().await;

        if let Some(scan_pid) = *pid {
            tracing::info!("Stopping scan {} with PID: {}", scan_id, scan_pid);

            // 使用标准库的 kill 命令来停止进程
            #[cfg(unix)]
            {
                use std::process::Command;

                // 先尝试 SIGTERM（优雅退出）
                let term_result = Command::new("kill")
                    .arg("-TERM")
                    .arg(scan_pid.to_string())
                    .output();

                match &term_result {
                    Ok(output) => {
                        if !output.status.success() {
                            tracing::warn!("SIGTERM failed, trying SIGKILL");
                            // SIGTERM 失败，使用 SIGKILL（强制退出）
                            let _ = Command::new("kill")
                                .arg("-KILL")
                                .arg(scan_pid.to_string())
                                .status();
                        } else {
                            tracing::info!("Sent SIGTERM to PID {}", scan_pid);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to execute kill command: {}", e);
                    }
                }
            }

            // 清除PID
            drop(pid);
            *self.current_scan_pid.write().await = None;

            Ok(())
        } else {
            Err("No scan process found".to_string())
        }
    }
}

/// 扫描结果
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub scan_id: String,
    pub success: bool,
    pub total_files: u32,
    pub threats_found: u32,
    pub threats: Vec<(String, String)>,  // (file_path, virus_name)
    pub error_message: Option<String>,
}

/// 更新结果
#[derive(Debug, Clone)]
pub struct UpdateResult {
    pub success: bool,
    pub old_version: Option<String>,
    pub new_version: Option<String>,
    pub error_message: Option<String>,
}
