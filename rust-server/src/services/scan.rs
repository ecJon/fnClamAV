use crate::env::FnosEnv;
use crate::services::{Database, ClamavService};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

/// 扫描服务
pub struct ScanService {
    db: Arc<Database>,
    clamav: ClamavService,
    current_scan: Arc<RwLock<Option<CurrentScan>>>,
}

struct CurrentScan {
    scan_id: String,
    task_handle: JoinHandle<Result<crate::services::clamav::ScanResult, String>>,
    _stop_tx: tokio::sync::broadcast::Sender<()>,
}

impl ScanService {
    pub fn new(db: Arc<Database>, env: FnosEnv) -> Self {
        let clamav = ClamavService::new(env);

        Self {
            db,
            clamav,
            current_scan: Arc::new(RwLock::new(None)),
        }
    }

    /// 开始扫描
    pub async fn start_scan(
        &self,
        scan_id: String,
        paths: Vec<String>,
    ) -> Result<(), String> {
        // 检查是否已有正在运行的扫描
        let current = self.current_scan.read().await;
        if current.is_some() {
            return Err("Scan already in progress".to_string());
        }
        drop(current);

        // 创建停止通道
        let (stop_tx, _stop_rx) = tokio::sync::broadcast::channel(1);

        // 克隆必要的引用
        let db = self.db.clone();
        let clamav = self.clamav.clone();
        let current_scan = self.current_scan.clone();
        let scan_id_clone = scan_id.clone();

        // 创建进度回调
        let progress_callback = Arc::new({
            let db_for_callback = db.clone();
            move |sid: String, scanned: i32, _threats: u32, current_file: Option<String>| {
                let db = db_for_callback.clone();
                tokio::spawn(async move {
                    let _ = db.update_scan_progress(&sid, scanned, current_file.as_deref());
                });
            }
        });

        // 启动后台扫描任务
        let task_handle = tokio::spawn(async move {
            let result = clamav.scan(scan_id_clone.clone(), paths, progress_callback).await;

            // 更新扫描状态
            // ClamAV 退出码: 0=无病毒, 1=发现病毒, 其他=错误
            // 只在真正失败时标记为 error，发现病毒仍标记为 completed
            let (status, error_msg) = match &result {
                Ok(r) => {
                    if r.threats_found > 0 {
                        ("completed", Some(format!("发现 {} 个威胁", r.threats_found)))
                    } else {
                        ("completed", Some("扫描完成，未发现威胁".to_string()))
                    }
                }
                Err(e) => ("error", Some(e.clone()))
            };

            let total_files = result.as_ref().map(|r| r.total_files as i32).unwrap_or(0);

            let _ = db.finish_scan(&scan_id_clone, status, total_files, error_msg.as_deref());

            // 清除当前扫描
            let mut current = current_scan.write().await;
            *current = None;

            result
        });

        // 保存当前扫描信息
        let mut current = self.current_scan.write().await;
        *current = Some(CurrentScan {
            scan_id: scan_id.clone(),
            task_handle,
            _stop_tx: stop_tx,
        });

        Ok(())
    }

    /// 停止扫描
    pub async fn stop_scan(&self, scan_id: &str) -> Result<(), String> {
        // 首先停止 ClamAV 进程
        let clamav_result = self.clamav.stop_scan(scan_id).await;

        // 然后清理任务状态
        let mut current = self.current_scan.write().await;

        if let Some(scan) = current.as_ref() {
            if scan.scan_id == scan_id {
                // 发送停止信号
                let _ = scan._stop_tx.send(());

                // 取消任务
                scan.task_handle.abort();

                // 更新数据库状态
                let _ = self.db.finish_scan(scan_id, "stopped", 0, Some("Stopped by user"));

                *current = None;

                // 返回 ClamAV 停止结果（如果有错误仍然报告）
                clamav_result
            } else {
                Err(format!("Scan ID mismatch: expected {}, got {}", scan.scan_id, scan_id))
            }
        } else {
            Err("No scan in progress".to_string())
        }
    }

    /// 获取当前扫描状态
    pub async fn get_current_scan_id(&self) -> Option<String> {
        let current = self.current_scan.read().await;
        current.as_ref().map(|s| s.scan_id.clone())
    }

    /// 检查是否有扫描正在进行
    pub async fn is_scanning(&self) -> bool {
        let current = self.current_scan.read().await;
        current.is_some()
    }
}
