// ClamAV 扫描引擎
//
// 此模块实现扫描引擎的核心功能：
// - 单文件扫描
// - 目录扫描
// - 实时进度回调
// - 暂停/恢复控制
// - 扫描任务管理

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use anyhow::Result;
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};

use super::types::*;
use super::ffi::{ClamAVEngine, ScanOptions, ClamAVError};

// 为 ClamAVEngine 实现 Send 和 Sync
// 因为 ClamAVEngine 内部使用原生指针，需要 unsafe 实现
unsafe impl Send for ClamAVEngine {}
unsafe impl Sync for ClamAVEngine {}

/// 扫描任务 ID
pub type TaskId = String;

/// 扫描任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// 扫描目标
#[derive(Debug, Clone)]
pub enum ScanTarget {
    File(PathBuf),
    Directory(PathBuf),
}

impl ScanTarget {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref();
        if path.is_dir() {
            Self::Directory(path.to_path_buf())
        } else {
            Self::File(path.to_path_buf())
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            Self::File(p) => p,
            Self::Directory(p) => p,
        }
    }
}

/// 进度回调类型
pub type ProgressCallback = Arc<dyn Fn(ScanProgress) + Send + Sync>;

/// 完成回调类型 (task_id, result，使用引用因为 anyhow::Error 不实现 Clone)
pub type CompletionCallback = Arc<dyn Fn(&str, &Result<ScanOutcome>) + Send + Sync>;

/// 扫描任务状态
#[derive(Debug, Clone, PartialEq)]
pub enum TaskState {
    Pending,
    Running,
    Paused,
    Completed,
    Failed(String),
    Cancelled,
}

/// 扫描任务
#[derive(Debug, Clone)]
pub struct ScanTask {
    pub id: TaskId,
    pub target: ScanTarget,
    pub priority: TaskPriority,
    pub state: TaskState,
    pub options: ScanOptions,
    pub created_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub progress: ScanProgress,
}

impl ScanTask {
    pub fn new(target: ScanTarget, priority: TaskPriority, options: ScanOptions) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            target,
            priority,
            state: TaskState::Pending,
            options,
            created_at: SystemTime::now(),
            started_at: None,
            completed_at: None,
            progress: ScanProgress::new(),
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
}

/// 任务队列
#[derive(Debug)]
pub struct TaskQueue {
    queue: VecDeque<ScanTask>,
    current_task: Option<ScanTask>,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            current_task: None,
        }
    }

    pub fn push(&mut self, task: ScanTask) {
        // 按优先级插入
        let mut insert_idx = self.queue.len();
        for (i, t) in self.queue.iter().enumerate() {
            if task.priority > t.priority {
                insert_idx = i;
                break;
            }
        }
        self.queue.insert(insert_idx, task);
    }

    pub fn pop(&mut self) -> Option<ScanTask> {
        self.queue.pop_front()
    }

    pub fn peek(&self) -> Option<&ScanTask> {
        self.queue.front()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn set_current(&mut self, task: ScanTask) {
        self.current_task = Some(task);
    }

    pub fn current(&self) -> Option<&ScanTask> {
        self.current_task.as_ref()
    }

    pub fn take_current(&mut self) -> Option<ScanTask> {
        self.current_task.take()
    }

    pub fn cancel(&mut self, task_id: &str) -> bool {
        if let Some(current) = &self.current_task {
            if current.id == task_id {
                return false; // 正在运行的任务不能直接取消
            }
        }
        self.queue.retain(|t| t.id != task_id);
        true
    }
}

impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// 扫描引擎命令
#[derive(Debug)]
pub enum EngineCommand {
    SubmitTask {
        task: ScanTask,
        reply: oneshot::Sender<Result<TaskId>>,
    },
    CancelTask {
        task_id: TaskId,
        reply: oneshot::Sender<Result<bool>>,
    },
    PauseTask {
        task_id: TaskId,
        reply: oneshot::Sender<Result<bool>>,
    },
    ResumeTask {
        task_id: TaskId,
        reply: oneshot::Sender<Result<bool>>,
    },
    GetTask {
        task_id: TaskId,
        reply: oneshot::Sender<Result<ScanTask>>,
    },
    ListTasks {
        reply: oneshot::Sender<Result<Vec<ScanTask>>>,
    },
    Shutdown {
        reply: oneshot::Sender<()>,
    },
}

/// 扫描引擎
pub struct ScanEngine {
    engine: Arc<ClamAVEngine>,
    task_queue: Arc<AsyncMutex<TaskQueue>>,
    command_tx: mpsc::UnboundedSender<EngineCommand>,
    progress_callback: Arc<AsyncMutex<Option<ProgressCallback>>>,
    completion_callback: Arc<AsyncMutex<Option<CompletionCallback>>>,
    cancel_flag: Arc<AsyncMutex<bool>>,
}

impl ScanEngine {
    /// 创建新的扫描引擎
    pub fn new(clamav_engine: Arc<ClamAVEngine>) -> Self {
        let (command_tx, mut command_rx) = mpsc::unbounded_channel();

        let engine = clamav_engine;
        let task_queue = Arc::new(AsyncMutex::new(TaskQueue::new()));
        let progress_callback = Arc::new(AsyncMutex::new(None));
        let completion_callback = Arc::new(AsyncMutex::new(None));
        let cancel_flag = Arc::new(AsyncMutex::new(false));

        // 启动任务处理循环
        let engine_clone = engine.clone();
        let queue_clone = task_queue.clone();
        let progress_clone = progress_callback.clone();
        let completion_clone = completion_callback.clone();
        let cancel_clone = cancel_flag.clone();

        tokio::spawn(async move {
            Self::run_task_loop(
                engine_clone,
                queue_clone,
                progress_clone,
                completion_clone,
                cancel_clone,
                &mut command_rx,
            ).await;
        });

        Self {
            engine,
            task_queue,
            command_tx,
            progress_callback,
            completion_callback,
            cancel_flag,
        }
    }

    /// 设置进度回调
    pub async fn set_progress_callback(&self, callback: ProgressCallback) {
        let mut cb = self.progress_callback.lock().await;
        *cb = Some(callback);
    }

    /// 设置完成回调
    pub async fn set_completion_callback(&self, callback: CompletionCallback) {
        let mut cb = self.completion_callback.lock().await;
        *cb = Some(callback);
    }

    /// 提交扫描任务
    pub async fn submit_task(&self, target: ScanTarget, priority: TaskPriority, options: ScanOptions) -> Result<TaskId> {
        let task = ScanTask::new(target, priority, options);
        let task_id = task.id.clone();

        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx.send(EngineCommand::SubmitTask {
            task,
            reply: reply_tx,
        })?;

        reply_rx.await?
    }

    /// 取消扫描任务
    pub async fn cancel_task(&self, task_id: &str) -> Result<bool> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx.send(EngineCommand::CancelTask {
            task_id: task_id.to_string(),
            reply: reply_tx,
        })?;

        reply_rx.await?
    }

    /// 暂停扫描任务
    pub async fn pause_task(&self, task_id: &str) -> Result<bool> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx.send(EngineCommand::PauseTask {
            task_id: task_id.to_string(),
            reply: reply_tx,
        })?;

        reply_rx.await?
    }

    /// 恢复扫描任务
    pub async fn resume_task(&self, task_id: &str) -> Result<bool> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx.send(EngineCommand::ResumeTask {
            task_id: task_id.to_string(),
            reply: reply_tx,
        })?;

        reply_rx.await?
    }

    /// 获取任务信息
    pub async fn get_task(&self, task_id: &str) -> Result<ScanTask> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx.send(EngineCommand::GetTask {
            task_id: task_id.to_string(),
            reply: reply_tx,
        })?;

        reply_rx.await?
    }

    /// 列出所有任务
    pub async fn list_tasks(&self) -> Result<Vec<ScanTask>> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx.send(EngineCommand::ListTasks {
            reply: reply_tx,
        })?;

        reply_rx.await?
    }

    /// 关闭引擎
    pub async fn shutdown(&self) -> Result<()> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.command_tx.send(EngineCommand::Shutdown {
            reply: reply_tx,
        })?;

        reply_rx.await;
        Ok(())
    }

    /// 任务处理循环
    async fn run_task_loop(
        engine: Arc<ClamAVEngine>,
        task_queue: Arc<AsyncMutex<TaskQueue>>,
        progress_callback: Arc<AsyncMutex<Option<ProgressCallback>>>,
        completion_callback: Arc<AsyncMutex<Option<CompletionCallback>>>,
        cancel_flag: Arc<AsyncMutex<bool>>,
        command_rx: &mut mpsc::UnboundedReceiver<EngineCommand>,
    ) {
        while let Some(cmd) = command_rx.recv().await {
            match cmd {
                EngineCommand::SubmitTask { task, reply } => {
                    let mut queue = task_queue.lock().await;
                    queue.push(task.clone());
                    let _ = reply.send(Ok(task.id));
                    drop(queue);

                    // 如果没有当前任务，开始处理新任务
                    Self::process_next_task(
                        engine.clone(),
                        task_queue.clone(),
                        progress_callback.clone(),
                        completion_callback.clone(),
                        cancel_flag.clone(),
                    ).await;
                }

                EngineCommand::CancelTask { task_id, reply } => {
                    // 首先设置取消标志
                    {
                        let mut flag = cancel_flag.lock().await;
                        *flag = true;
                        tracing::info!("Set cancel flag for task: {}", task_id);
                    }

                    let mut queue = task_queue.lock().await;
                    let current = queue.current();

                    // 检查是否是当前正在运行的任务
                    let is_current = match current {
                        Some(task) if task.id == task_id => {
                            // 立即清理 current_task，不等待后台任务完成
                            queue.take_current();
                            tracing::info!("Cleared current task: {}", task_id);
                            true
                        }
                        _ => false,
                    };

                    // 从队列中取消任务
                    let result = queue.cancel(&task_id);
                    let _ = reply.send(Ok(result || is_current));
                }

                EngineCommand::PauseTask { task_id, reply } => {
                    let queue = task_queue.lock().await;
                    let current = queue.current();
                    let result = match current {
                        Some(task) if task.id == task_id => {
                            // 暂停当前任务
                            let mut flag = cancel_flag.lock().await;
                            *flag = true;
                            true
                        }
                        _ => false,
                    };
                    let _ = reply.send(Ok(result));
                }

                EngineCommand::ResumeTask { task_id, reply } => {
                    let mut flag = cancel_flag.lock().await;
                    *flag = false;
                    let _ = reply.send(Ok(true));

                    // 重新开始任务处理
                    Self::process_next_task(
                        engine.clone(),
                        task_queue.clone(),
                        progress_callback.clone(),
                        completion_callback.clone(),
                        cancel_flag.clone(),
                    ).await;
                }

                EngineCommand::GetTask { task_id, reply } => {
                    let queue = task_queue.lock().await;
                    let task = queue.current()
                        .or_else(|| queue.peek())
                        .filter(|t| t.id == task_id)
                        .cloned();
                    let _ = reply.send(
                        task.ok_or_else(|| anyhow::anyhow!("Task not found"))
                    );
                }

                EngineCommand::ListTasks { reply } => {
                    let queue = task_queue.lock().await;
                    let mut tasks: Vec<ScanTask> = queue.queue.iter().cloned().collect();
                    if let Some(current) = &queue.current_task {
                        tasks.push(current.clone());
                    }
                    let _ = reply.send(Ok(tasks));
                }

                EngineCommand::Shutdown { reply } => {
                    let _ = reply.send(());
                    break;
                }
            }
        }
    }

    /// 处理下一个任务
    async fn process_next_task(
        engine: Arc<ClamAVEngine>,
        task_queue: Arc<AsyncMutex<TaskQueue>>,
        progress_callback: Arc<AsyncMutex<Option<ProgressCallback>>>,
        completion_callback: Arc<AsyncMutex<Option<CompletionCallback>>>,
        cancel_flag: Arc<AsyncMutex<bool>>,
    ) {
        let mut queue = task_queue.lock().await;

        // 如果已有任务在运行，跳过
        if queue.current().is_some() {
            tracing::debug!("Task already running, skipping");
            return;
        }

        // 获取下一个任务
        let task = match queue.pop() {
            Some(t) => t,
            None => {
                tracing::debug!("No tasks in queue");
                return;
            }
        };

        tracing::info!("Processing scan task: id={}, target={:?}", task.id, task.target);
        queue.set_current(task.clone());
        let task_id = task.id.clone();
        let target = task.target.clone();
        let options = task.options.clone();
        drop(queue);

        // 重置取消标志
        {
            let mut flag = cancel_flag.lock().await;
            *flag = false;
        }

        // 在独立的 tokio task 中执行扫描，避免阻塞命令循环
        tokio::spawn(async move {
            tracing::info!("Starting scan execution for task {} in background", task_id);
            let result = Self::execute_scan(
                engine,
                &target,
                &options,
                progress_callback,
                cancel_flag.clone(),
            ).await;

            // 更新任务状态
            tracing::info!("Scan task {} completed with result: {:?}", task_id, result.is_ok());
            let mut queue = task_queue.lock().await;
            queue.take_current();
            drop(queue);

            // 调用完成回调
            let cb = completion_callback.lock().await;
            if let Some(ref callback) = *cb {
                tracing::info!("Calling completion callback for task {}", task_id);
                callback(&task_id, &result);
            }
        });
    }

    /// 执行扫描
    async fn execute_scan(
        engine: Arc<ClamAVEngine>,
        target: &ScanTarget,
        options: &ScanOptions,
        progress_callback: Arc<AsyncMutex<Option<ProgressCallback>>>,
        cancel_flag: Arc<AsyncMutex<bool>>,
    ) -> Result<ScanOutcome> {
        let path = target.path();

        // 检查路径是否存在
        if !path.exists() {
            let error = format!("Path does not exist: {}", path.display());
            tracing::error!("{}", error);
            return Ok(ScanOutcome::failed(error));
        }

        tracing::info!("Executing scan for target: {:?}, path: {}", target, path.display());

        match target {
            ScanTarget::File(_) => {
                Self::scan_file(engine, path, options, progress_callback, cancel_flag).await
            }
            ScanTarget::Directory(_) => {
                Self::scan_directory(engine, path, options, progress_callback, cancel_flag).await
            }
        }
    }

    /// 扫描单个文件
    async fn scan_file(
        engine: Arc<ClamAVEngine>,
        path: &Path,
        options: &ScanOptions,
        progress_callback: Arc<AsyncMutex<Option<ProgressCallback>>>,
        cancel_flag: Arc<AsyncMutex<bool>>,
    ) -> Result<ScanOutcome> {
        let path = path.to_path_buf();
        let options = *options;

        // 更新进度
        Self::update_progress(
            &progress_callback,
            ScanProgress {
                percent: ProgressPercent(0),
                scanned_files: ScannedFiles(0),
                total_files: TotalFiles(1),
                threats_found: ThreatsFound(0),
                current_file: Some(FilePath(path.display().to_string())),
            },
        ).await;

        // 检查取消标志
        if *cancel_flag.lock().await {
            return Ok(ScanOutcome::failed("Scan cancelled".to_string()));
        }

        // 在 spawn_blocking 中执行同步扫描
        let engine_clone = engine.clone();
        let path_str = path.to_string_lossy().to_string();
        let result = tokio::task::spawn_blocking(move || {
            engine_clone.scan_file(&path_str, options)
        }).await??;

        // 更新进度
        let is_infected = result.is_infected;
        Self::update_progress(
            &progress_callback,
            ScanProgress {
                percent: ProgressPercent(100),
                scanned_files: ScannedFiles(1),
                total_files: TotalFiles(1),
                threats_found: ThreatsFound(if is_infected { 1 } else { 0 }),
                current_file: None,
            },
        ).await;

        let threats = if is_infected {
            vec![(
                FilePath(result.filename),
                VirusName(result.virus_name.unwrap_or_else(|| "Unknown".to_string()))
            )]
        } else {
            vec![]
        };

        Ok(ScanOutcome::success(
            1,
            1,
            threats,
        ))
    }

    /// 扫描目录
    async fn scan_directory(
        engine: Arc<ClamAVEngine>,
        path: &Path,
        options: &ScanOptions,
        progress_callback: Arc<AsyncMutex<Option<ProgressCallback>>>,
        cancel_flag: Arc<AsyncMutex<bool>>,
    ) -> Result<ScanOutcome> {
        tracing::info!("Starting directory scan: {}", path.display());

        // 检查取消标志
        if *cancel_flag.lock().await {
            return Ok(ScanOutcome::failed("Scan cancelled".to_string()));
        }

        // 收集所有文件（带取消检查）
        let files = Self::collect_files_with_cancel(path, cancel_flag.clone()).await?;
        let total = files.len() as u32;
        tracing::info!("Found {} files to scan", total);

        let mut scanned = 0u32;
        let mut all_threats = Vec::new();

        for (idx, file) in files.iter().enumerate() {
            // 检查取消标志
            if *cancel_flag.lock().await {
                tracing::info!("Scan cancelled at file {}", idx);
                return Ok(ScanOutcome::failed("Scan cancelled".to_string()));
            }

            // 更新进度
            let percent = if files.len() > 0 {
                ((idx + 1) * 100 / files.len()) as u8
            } else {
                100
            };
            Self::update_progress(
                &progress_callback,
                ScanProgress {
                    percent: ProgressPercent(percent),
                    scanned_files: ScannedFiles(scanned),
                    total_files: TotalFiles(total),
                    threats_found: ThreatsFound(all_threats.len() as u32),
                    current_file: Some(FilePath(file.display().to_string())),
                },
            ).await;

            // 扫描文件
            let engine_clone = engine.clone();
            let file_clone = file.to_path_buf();
            let options_copy = *options;
            let file_str = file.display().to_string();

            tracing::debug!("Scanning file {}/{}: {}", idx + 1, total, file_str);

            match tokio::task::spawn_blocking(move || {
                engine_clone.scan_file(&file_clone.to_string_lossy(), options_copy)
            }).await? {
                Ok(result) => {
                    scanned += 1;
                    if result.is_infected {
                        tracing::warn!("THREAT FOUND in {}: {:?}", result.filename, result.virus_name);
                        all_threats.push((
                            FilePath(result.filename),
                            VirusName(result.virus_name.unwrap_or_else(|| "Unknown".to_string()))
                        ));
                    } else {
                        tracing::trace!("File clean: {}", file_str);
                    }
                }
                Err(e) => {
                    // 记录错误但继续扫描
                    tracing::error!("Error scanning {}: {}", file.display(), e);
                }
            }
        }

        tracing::info!("Directory scan complete: {}/{} files scanned, {} threats found",
                      scanned, total, all_threats.len());

        // 最终进度更新
        Self::update_progress(
            &progress_callback,
            ScanProgress {
                percent: ProgressPercent(100),
                scanned_files: ScannedFiles(scanned),
                total_files: TotalFiles(total),
                threats_found: ThreatsFound(all_threats.len() as u32),
                current_file: None,
            },
        ).await;

        Ok(ScanOutcome::success(
            total,
            scanned,
            all_threats,
        ))
    }

    /// 收集目录中的所有文件
    /// 收集目录中的所有文件（带取消检查）
    async fn collect_files_with_cancel(
        path: &Path,
        cancel_flag: Arc<AsyncMutex<bool>>,
    ) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let mut dir_queue = vec![path.to_path_buf()];

        // 使用迭代方式而不是递归，避免栈溢出
        while let Some(dir) = dir_queue.pop() {
            // 每处理一个目录检查一次取消标志
            {
                let flag = cancel_flag.lock().await;
                if *flag {
                    tracing::info!("File collection cancelled, collected {} files so far", files.len());
                    return Err(anyhow::anyhow!("Scan cancelled during file collection"));
                }
            }

            let entries = match std::fs::read_dir(&dir) {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("Failed to read directory {}: {}", dir.display(), e);
                    continue;
                }
            };

            for entry in entries {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::warn!("Failed to read entry: {}", e);
                        continue;
                    }
                };
                let path = entry.path();

                if path.is_dir() {
                    dir_queue.push(path);
                } else if path.is_file() {
                    files.push(path);
                }

                // 每 1000 个文件检查一次取消标志（使用 try_lock 避免频繁阻塞）
                if files.len() % 1000 == 0 {
                    if let Ok(flag) = cancel_flag.try_lock() {
                        if *flag {
                            tracing::info!("File collection cancelled at {} files", files.len());
                            return Err(anyhow::anyhow!("Scan cancelled during file collection"));
                        }
                    }
                }
            }
        }

        tracing::info!("Collected {} files from {}", files.len(), path.display());
        Ok(files)
    }

    /// 更新进度回调
    async fn update_progress(
        callback: &Arc<AsyncMutex<Option<ProgressCallback>>>,
        progress: ScanProgress,
    ) {
        let cb = callback.lock().await;
        if let Some(ref f) = *cb {
            f(progress);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_queue_priority() {
        let mut queue = TaskQueue::new();

        let low_task = ScanTask::new(
            ScanTarget::File(PathBuf::from("/tmp/test1.txt")),
            TaskPriority::Low,
            ScanOptions::default(),
        );

        let high_task = ScanTask::new(
            ScanTarget::File(PathBuf::from("/tmp/test2.txt")),
            TaskPriority::High,
            ScanOptions::default(),
        );

        queue.push(low_task);
        queue.push(high_task);

        // 高优先级应该先出队
        let next = queue.pop().unwrap();
        assert_eq!(next.priority, TaskPriority::High);
    }

    #[test]
    fn test_scan_target_from_path() {
        // 测试文件路径
        let target = ScanTarget::from_path("/tmp/test.txt");
        assert!(matches!(target, ScanTarget::File(_)));

        // 测试目录路径
        let target = ScanTarget::from_path("/tmp");
        assert!(matches!(target, ScanTarget::Directory(_)));
    }
}
