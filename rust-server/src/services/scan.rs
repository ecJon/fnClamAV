// 扫描服务
//
// 此服务提供扫描任务的高级接口：
// - 扫描任务创建和管理
// - 进度跟踪和数据库同步
// - 暂停/恢复/取消操作

use std::sync::Arc;
use std::collections::HashMap;
use anyhow::{Result, Context};
use tokio::sync::RwLock;

use crate::services::Database;
use crate::services::clamav::{ClamavService, ScanRequest};
use crate::clamav::engine::{ScanTarget, TaskPriority};
use crate::clamav::ScanOptions;
use crate::clamav::ScanProgress;

/// 扫描服务
pub struct ScanService {
    db: Arc<Database>,
    pub clamav: ClamavService,
    active_scans: Arc<RwLock<HashMap<String, ActiveScan>>>,
}

/// 活跃的扫描任务
#[derive(Clone)]
pub struct ActiveScan {
    pub scan_id: String,
    pub task_id: String,
    pub paths: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    // 实时进度数据（用于前端查询，避免异步数据库更新导致的时序问题）
    pub scanned_files: u32,
    pub total_files: u32,
    pub current_file: Option<String>,
    pub threats_found: u32,
    pub status: String,  // "scanning", "completed", "failed", "paused"
}

impl ScanService {
    /// 创建新的扫描服务
    pub fn new(db: Arc<Database>, clamav: ClamavService) -> Self {
        Self {
            db,
            clamav,
            active_scans: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 开始扫描
    pub async fn start_scan(
        &self,
        scan_id: String,
        paths: Vec<String>,
        priority: TaskPriority,
        options: ScanOptions,
    ) -> Result<String> {
        tracing::info!("Starting scan {} with paths: {:?}", scan_id, paths);

        // 创建扫描请求
        let request = ScanRequest::new(paths.clone())
            .with_priority(priority)
            .with_options(options);

        // 转换为扫描目标
        let targets = request.to_targets();
        if targets.is_empty() {
            tracing::error!("No valid paths to scan from: {:?}", paths);
            return Err(anyhow::anyhow!("No valid paths to scan").into());
        }

        // 对于多路径扫描，我们为每个路径创建一个任务
        // 这里简化处理：只处理第一个目标
        let target = targets.into_iter().next().unwrap();
        tracing::info!("Scan target: {:?}", target);

        // 注意：数据库记录由 handler 层负责创建，这里不需要重复创建
        // handlers/scan.rs 已经在调用 start_scan 之前创建了记录并传入了正确的 scan_type

        // 记录活跃扫描
        let active_scan = ActiveScan {
            scan_id: scan_id.clone(),
            task_id: String::new(), // 临时值，下面会更新
            paths,
            created_at: chrono::Utc::now(),
            scanned_files: 0,
            total_files: 0,
            current_file: None,
            threats_found: 0,
            status: "scanning".to_string(),
        };

        let mut scans = self.active_scans.write().await;
        scans.insert(scan_id.clone(), active_scan);
        drop(scans);

        // 设置进度回调（在提交任务之前）
        let db = self.db.clone();
        let active_scans_for_progress = self.active_scans.clone();
        let scan_id_for_callback = scan_id.clone();
        self.clamav.set_progress_callback(move |progress| {
            let db = db.clone();
            let active_scans = active_scans_for_progress.clone();
            let scan_id = scan_id_for_callback.clone();
            let scanned = progress.scanned_files.0;
            let total = progress.total_files.0;
            let percent = progress.percent.0;
            let current_file = progress.current_file.as_ref().map(|f| f.0.clone());
            let threats = progress.threats_found.0;
            tracing::debug!("Progress update: scan_id={}, scanned={}/{}, percent={}, current_file={:?}, threats={}",
                          scan_id, scanned, total, percent, current_file, threats);

            // 同步更新内存中的实时状态（确保前端获取最新数据）
            let scans = active_scans.try_read();
            if let Ok(scans) = scans {
                if let Some(scan) = scans.get(&scan_id) {
                    // 注意：这里使用 block_in_place 在同步上下文中获取写锁
                    let _ = tokio::task::block_in_place(|| {
                        let mut scans_w = active_scans.try_write();
                        if let Ok(mut scans) = scans_w {
                            if let Some(s) = scans.get_mut(&scan_id) {
                                s.scanned_files = scanned;
                                s.total_files = total;
                                s.current_file = current_file.clone();
                                s.threats_found = threats;
                            }
                        }
                    });
                }
            }

            // 只有未完成时才更新数据库进度（避免覆盖完成状态）
            // 当 scanned >= total 时，说明扫描已完成，不需要再更新进度
            if scanned < total {
                tokio::spawn(async move {
                    let _ = db.update_scan_progress(
                        &scan_id,
                        scanned as i32,
                        total as i32,
                        current_file.as_deref(),
                    );
                });
            } else {
                tracing::debug!("Skipping progress update for scan_id={}, scan already completed ({} >= {})", scan_id, scanned, total);
            }
        }).await;

        // 设置完成回调（接收 task_id，根据 task_id 查找对应的 scan_id）
        let db_clone = self.db.clone();
        let active_scans_clone = self.active_scans.clone();
        self.clamav.set_completion_callback(move |task_id, result| {
            let db = db_clone.clone();
            let active_scans = active_scans_clone.clone();

            // 根据 task_id 查找对应的 scan_id（使用 block_in_place 在同步上下文中执行异步操作）
            let scan_id = tokio::task::block_in_place(|| {
                let scans = active_scans.try_read();
                if let Ok(scans) = scans {
                    scans.iter()
                        .find(|(_, s)| s.task_id == task_id)
                        .map(|(id, _)| id.clone())
                } else {
                    None
                }
            });

            let scan_id = match scan_id {
                Some(id) => id,
                None => {
                    tracing::error!("Cannot find scan_id for task_id={}, skipping completion callback", task_id);
                    return;
                }
            };

            match result {
                Ok(outcome) => {
                    tracing::info!("Scan completed successfully: scan_id={}, task_id={}, total={}, scanned={}, threats={}",
                                  scan_id, task_id, outcome.total_files, outcome.scanned_files, outcome.threats.len());

                    // 同步更新数据库，确保完成状态最后写入（避免与异步进度更新产生竞态条件）
                    let total = outcome.total_files as i32;
                    let threats_count = outcome.threats.len();
                    let status = "completed";
                    let error_msg = if threats_count == 0 {
                        "扫描完成，未发现威胁"
                    } else {
                        "扫描完成，发现威胁"
                    };

                    // 直接同步调用，不使用 tokio::spawn
                    let _ = db.finish_scan(&scan_id, status, total, Some(error_msg));
                }
                Err(e) => {
                    tracing::error!("Scan failed: scan_id={}, task_id={}, error={:?}", scan_id, task_id, e);
                    // 扫描失败时也要更新数据库
                    let error_msg = e.to_string();
                    // 直接同步调用，不使用 tokio::spawn
                    let _ = db.finish_scan(&scan_id, "failed", 0, Some(error_msg.as_str()));
                }
            }

            // 从 active_scans 移除完成的扫描
            tokio::spawn(async move {
                let mut scans = active_scans.write().await;
                scans.remove(&scan_id);
                tracing::debug!("Removed scan {} from active_scans after completion", scan_id);
            });
        }).await;

        // 提交扫描任务
        tracing::info!("Submitting scan task to engine...");
        let task_id = self.clamav.submit_scan(target, priority, options).await?;
        tracing::info!("Scan task submitted with task_id={}", task_id);

        // 更新活跃扫描的 task_id
        let mut scans = self.active_scans.write().await;
        if let Some(scan) = scans.get_mut(&scan_id) {
            scan.task_id = task_id.clone();
        }
        drop(scans);

        Ok(task_id)
    }

    /// 停止扫描
    pub async fn stop_scan(&self, scan_id: &str) -> Result<()> {
        let scans = self.active_scans.read().await;
        let active = scans.get(scan_id)
            .ok_or_else(|| anyhow::anyhow!("Scan not found: {}", scan_id))?;
        let task_id = active.task_id.clone();
        drop(scans);

        // 取消任务
        self.clamav.cancel_scan(&task_id).await?;

        // 更新数据库
        let _ = self.db.finish_scan(scan_id, "stopped", 0, Some("Stopped by user"));

        // 移除活跃扫描
        let mut scans = self.active_scans.write().await;
        scans.remove(scan_id);

        Ok(())
    }

    /// 暂停扫描
    pub async fn pause_scan(&self, scan_id: &str) -> Result<()> {
        let scans = self.active_scans.read().await;
        let active = scans.get(scan_id)
            .ok_or_else(|| anyhow::anyhow!("Scan not found: {}", scan_id))?;
        let task_id = active.task_id.clone();
        drop(scans);

        self.clamav.pause_scan(&task_id).await?;

        // 更新数据库状态
        let _ = self.db.update_scan_status(scan_id, "paused");

        Ok(())
    }

    /// 恢复扫描
    pub async fn resume_scan(&self, scan_id: &str) -> Result<()> {
        let scans = self.active_scans.read().await;
        let active = scans.get(scan_id)
            .ok_or_else(|| anyhow::anyhow!("Scan not found: {}", scan_id))?;
        let task_id = active.task_id.clone();
        drop(scans);

        self.clamav.resume_scan(&task_id).await?;

        // 更新数据库状态
        let _ = self.db.update_scan_status(scan_id, "running");

        Ok(())
    }

    /// 获取扫描状态
    pub async fn get_scan_status(&self, scan_id: &str) -> Result<ScanStatus> {
        let scans = self.active_scans.read().await;
        let active = scans.get(scan_id);
        let task_id = active.as_ref().map(|a| a.task_id.clone()).ok_or_else(|| {
            anyhow::anyhow!("Scan not found: {}", scan_id)
        })?;
        drop(scans);

        // 获取任务状态
        let task = self.clamav.get_task(&task_id).await?;

        Ok(ScanStatus {
            scan_id: scan_id.to_string(),
            task_id: task.id.clone(),
            status: format!("{:?}", task.state),
            percent: task.progress.percent.0,
            scanned_files: task.progress.scanned_files.0,
            threats_found: task.progress.threats_found.0,
            current_file: task.progress.current_file.map(|f| f.0),
        })
    }

    /// 获取当前扫描ID
    pub async fn get_current_scan_id(&self) -> Option<String> {
        let scans = self.active_scans.read().await;
        scans.keys().next().cloned()
    }

    /// 获取当前扫描的实时进度（从内存中读取，避免数据库异步更新导致的时序问题）
    pub async fn get_current_scan_progress(&self) -> Option<ActiveScan> {
        let scans = self.active_scans.read().await;
        scans.values().next().cloned()
    }

    /// 检查是否有扫描正在进行
    pub async fn is_scanning(&self) -> bool {
        let scans = self.active_scans.read().await;
        !scans.is_empty()
    }

    /// 完成扫描（由回调触发）
    pub async fn complete_scan(
        &self,
        scan_id: &str,
        result: &crate::clamav::ScanOutcome,
    ) -> Result<()> {
        let status = match result.status {
            crate::clamav::ScanStatus::Completed => "completed",
            crate::clamav::ScanStatus::Failed(_) => "failed",
            _ => "unknown",
        };

        let threats_count = result.threats.len();
        let error_msg = match &result.status {
            crate::clamav::ScanStatus::Failed(msg) => Some(msg.as_str()),
            _ if threats_count == 0 => Some("扫描完成，未发现威胁"),
            _ => Some("扫描完成，发现威胁"),
        };

        self.db.finish_scan(
            scan_id,
            status,
            result.total_files as i32,
            error_msg,
        );

        // 移除活跃扫描
        let mut scans = self.active_scans.write().await;
        scans.remove(scan_id);

        Ok(())
    }
}

/// 扫描状态
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanStatus {
    pub scan_id: String,
    pub task_id: String,
    pub status: String,
    pub percent: u8,
    pub scanned_files: u32,
    pub threats_found: u32,
    pub current_file: Option<String>,
}

/// 扫描结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct ScanResult {
    pub scan_id: String,
    pub success: bool,
    pub total_files: u32,
    pub scanned_files: u32,
    pub threats_found: u32,
    pub threats: Vec<ThreatInfo>,
    pub error_message: Option<String>,
}

/// 威胁信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct ThreatInfo {
    pub file_path: String,
    pub virus_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_status_default() {
        let status = ScanStatus {
            scan_id: "test".to_string(),
            task_id: "task-1".to_string(),
            status: "running".to_string(),
            percent: 50,
            scanned_files: 100,
            threats_found: 2,
            current_file: Some("/test/file.txt".to_string()),
        };

        assert_eq!(status.scan_id, "test");
        assert_eq!(status.percent, 50);
        assert_eq!(status.threats_found, 2);
    }
}
