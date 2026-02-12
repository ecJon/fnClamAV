// ClamAV FFI 服务
//
// 此服务提供 ClamAV 引擎的高级接口：
// - 引擎初始化和生命周期管理
// - 扫描任务管理
// - 进度回调处理
// - 病毒库更新

use std::sync::Arc;
use std::path::PathBuf;
use anyhow::{Result, Context};
use tokio::sync::RwLock;

use crate::models::ClamAVConfig;
use crate::clamav::{
    ClamAVEngine, EngineManager,
};
use crate::clamav::engine::{ScanTarget, TaskPriority, ScanEngine as ClamAVScanEngine, CompletionCallback};
use crate::clamav::{ScanOptions, ScanProgress, ScanOutcome, VirusName, FilePath};

/// 类型别名
type ScanEngine = ClamAVScanEngine;

/// ClamAV FFI 服务
#[derive(Clone)]
pub struct ClamavService {
    engine_manager: Arc<EngineManager>,
    scan_engine: Arc<RwLock<Option<Arc<ScanEngine>>>>,
    config: ClamAVConfig,
}

impl ClamavService {
    /// 创建新的 ClamAV 服务
    pub fn new(config: ClamAVConfig) -> Self {
        let engine_manager = Arc::new(EngineManager::new(config.clone()));

        Self {
            engine_manager,
            scan_engine: Arc::new(RwLock::new(None)),
            config,
        }
    }

    /// 初始化 ClamAV 引擎
    pub async fn initialize(&self) -> Result<()> {
        self.engine_manager.initialize()
            .map_err(|e| anyhow::anyhow!("Failed to initialize ClamAV engine: {}", e))?;
        Ok(())
    }

    /// 启动扫描引擎
    pub async fn start_scan_engine(&self) -> Result<()> {
        let engine = self.engine_manager.get_engine()
            .map_err(|e| anyhow::anyhow!("Failed to get engine: {}", e))?;

        let scan_engine = Arc::new(ScanEngine::new(engine));

        let mut se = self.scan_engine.write().await;
        *se = Some(scan_engine);
        drop(se);

        Ok(())
    }

    /// 获取扫描引擎
    async fn get_scan_engine(&self) -> Result<Arc<ScanEngine>> {
        let se: tokio::sync::RwLockReadGuard<'_, Option<Arc<ScanEngine>>> = self.scan_engine.read().await;
        se.as_ref()
            .map(Arc::clone)
            .ok_or_else(|| anyhow::anyhow!("Scan engine not started"))
    }

    /// 提交扫描任务
    pub async fn submit_scan(
        &self,
        target: ScanTarget,
        priority: TaskPriority,
        options: ScanOptions,
    ) -> Result<String> {
        let scan_engine: Arc<ScanEngine> = self.get_scan_engine().await?;
        scan_engine.submit_task(target, priority, options).await
    }

    /// 取消扫描任务
    pub async fn cancel_scan(&self, task_id: &str) -> Result<bool> {
        let scan_engine: Arc<ScanEngine> = self.get_scan_engine().await?;
        scan_engine.cancel_task(task_id).await
    }

    /// 暂停扫描任务
    pub async fn pause_scan(&self, task_id: &str) -> Result<bool> {
        let scan_engine: Arc<ScanEngine> = self.get_scan_engine().await?;
        scan_engine.pause_task(task_id).await
    }

    /// 恢复扫描任务
    pub async fn resume_scan(&self, task_id: &str) -> Result<bool> {
        let scan_engine: Arc<ScanEngine> = self.get_scan_engine().await?;
        scan_engine.resume_task(task_id).await
    }

    /// 获取任务状态
    pub async fn get_task(&self, task_id: &str) -> Result<crate::clamav::engine::ScanTask> {
        let scan_engine: Arc<ScanEngine> = self.get_scan_engine().await?;
        scan_engine.get_task(task_id).await
    }

    /// 列出所有任务
    pub async fn list_tasks(&self) -> Result<Vec<crate::clamav::engine::ScanTask>> {
        let scan_engine: Arc<ScanEngine> = self.get_scan_engine().await?;
        scan_engine.list_tasks().await
    }

    /// 健康检查
    pub async fn health_check(&self) -> Result<bool> {
        Ok(self.engine_manager.health_check())
    }

    /// 重新加载引擎
    pub async fn reload_engine(&self) -> Result<()> {
        self.engine_manager.reload()
            .map_err(|e| anyhow::anyhow!("Reload failed: {}", e))
    }

    /// 关闭服务
    pub async fn shutdown(&self) -> Result<()> {
        // 关闭扫描引擎
        {
            let mut se: tokio::sync::RwLockWriteGuard<'_, Option<Arc<ScanEngine>>> = self.scan_engine.write().await;
            let scan_engine: Option<Arc<ScanEngine>> = se.take();
            drop(se);
            if let Some(engine) = scan_engine {
                ScanEngine::shutdown(&*engine).await?;
            }
        }

        // 关闭引擎管理器
        self.engine_manager.shutdown();

        Ok(())
    }

    /// 设置进度回调
    pub async fn set_progress_callback<F>(&self, callback: F)
    where
        F: Fn(ScanProgress) + Send + Sync + 'static,
    {
        match self.get_scan_engine().await {
            Ok(engine) => {
                ScanEngine::set_progress_callback(&*engine, std::sync::Arc::new(callback)).await;
            }
            Err(_) => {}
        }
    }

    /// 设置完成回调 (task_id, result)
    pub async fn set_completion_callback<F>(&self, callback: F)
    where
        F: Fn(&str, &Result<ScanOutcome>) + Send + Sync + 'static,
    {
        match self.get_scan_engine().await {
            Ok(engine) => {
                ScanEngine::set_completion_callback(&*engine, std::sync::Arc::new(callback)).await;
            }
            Err(_) => {}
        }
    }

    /// 获取引擎状态
    pub async fn get_engine_state(&self) -> crate::clamav::EngineState {
        self.engine_manager.get_state()
    }
}

/// 扫描任务请求
#[derive(Debug, Clone)]
pub struct ScanRequest {
    pub paths: Vec<String>,
    pub priority: TaskPriority,
    pub options: ScanOptions,
}

impl ScanRequest {
    pub fn new(paths: Vec<String>) -> Self {
        Self {
            paths,
            priority: TaskPriority::Normal,
            options: ScanOptions::default(),
        }
    }

    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_options(mut self, options: ScanOptions) -> Self {
        self.options = options;
        self
    }

    /// 将路径转换为扫描目标
    pub fn to_targets(&self) -> Vec<ScanTarget> {
        self.paths.iter()
            .filter_map(|p| {
                let path = PathBuf::from(p);
                if path.exists() {
                    Some(ScanTarget::from_path(path))
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Default for ScanRequest {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            priority: TaskPriority::Normal,
            options: ScanOptions::default(),
        }
    }
}

/// 扫描任务响应
#[derive(Debug, Clone)]
pub struct ScanResponse {
    pub task_id: String,
    pub status: String,
}

/// 扫描状态详情
#[derive(Debug, Clone)]
pub struct ScanStatusDetail {
    pub task_id: String,
    pub state: String,
    pub percent: u8,
    pub scanned_files: u32,
    pub threats_found: u32,
    pub current_file: Option<String>,
}

impl From<crate::clamav::engine::ScanTask> for ScanStatusDetail {
    fn from(task: crate::clamav::engine::ScanTask) -> Self {
        Self {
            task_id: task.id,
            state: format!("{:?}", task.state),
            percent: task.progress.percent.0,
            scanned_files: task.progress.scanned_files.0,
            threats_found: task.progress.threats_found.0,
            current_file: task.progress.current_file.map(|f| f.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_request_new() {
        let request = ScanRequest::new(vec!["/tmp/test".to_string()]);
        assert_eq!(request.paths.len(), 1);
        assert_eq!(request.priority, TaskPriority::Normal);
    }

    #[test]
    fn test_scan_request_with_priority() {
        let request = ScanRequest::new(vec![])
            .with_priority(TaskPriority::High);
        assert_eq!(request.priority, TaskPriority::High);
    }
}
