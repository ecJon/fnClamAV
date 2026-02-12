// ClamAV 引擎单例和生命周期管理
//
// 设计要点:
// - 引擎单例模式，全局唯一实例
// - 线程安全访问 (Arc<Mutex<Engine>>)
// - 支持病毒库热重载（重新初始化）
// - 服务启动时初始化，关闭时释放
// - 引擎异常时自动恢复机制
//
// 引擎状态:
// - Uninitialized: 未初始化
// - Initializing: 正在初始化
// - Ready: 已就绪，可以执行扫描
// - Error: 引擎错误状态
// - Failed: 引擎失败，等待恢复

use std::sync::{Arc, Mutex};

use super::ffi::{ClamAVEngine, ScanOptions, ScanResult, ClamAVError};
use crate::models::config::ClamAVConfig;

/// 引擎状态
#[derive(Debug, Clone, PartialEq)]
pub enum EngineState {
    Uninitialized,
    Initializing,
    Ready,
    Error(String),
    Failed,
}

impl EngineState {
    pub fn is_ready(&self) -> bool {
        matches!(self, EngineState::Ready)
    }

    pub fn is_operational(&self) -> bool {
        matches!(self, EngineState::Ready)
    }
}

/// ClamAV 引擎单例管理器
pub struct EngineManager {
    engine: Arc<Mutex<Option<Arc<ClamAVEngine>>>>,
    state: Arc<Mutex<EngineState>>,
    config: ClamAVConfig,
}

impl EngineManager {
    /// 创建新的引擎管理器
    pub fn new(config: ClamAVConfig) -> Self {
        Self {
            engine: Arc::new(Mutex::new(None)),
            state: Arc::new(Mutex::new(EngineState::Uninitialized)),
            config,
        }
    }

    /// 初始化引擎
    ///
    /// # 参数
    /// - db_dir: 病毒库目录路径
    ///
    /// # 返回
    /// - Ok(()) 或 Err(错误信息)
    pub fn initialize(&self) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();

        // 检查是否已经初始化
        if matches!(*state, EngineState::Ready) {
            tracing::info!("Engine already initialized");
            return Ok(());
        }

        // 更新状态为初始化中
        *state = EngineState::Initializing;
        drop(state);

        // 准备证书目录路径
        let certs_dir = self.config.certs_dir.as_deref();
        tracing::info!("Initializing ClamAV engine with db_dir={}, certs_dir={:?}",
                       self.config.database_dir, certs_dir);

        // 创建新引擎
        let engine = match ClamAVEngine::initialize(&self.config.database_dir, certs_dir) {
            Ok(e) => {
                tracing::info!("ClamAV engine initialized successfully");
                e
            }
            Err(e) => {
                tracing::error!("Failed to initialize ClamAV engine: {}", e);
                // 更新状态为错误
                let mut state = self.state.lock().unwrap();
                *state = EngineState::Error(e.to_string());
                return Err(e.to_string());
            }
        };

        // 保存引擎实例
        *self.engine.lock().unwrap() = Some(Arc::new(engine));

        // 更新状态为就绪
        let mut state = self.state.lock().unwrap();
        *state = EngineState::Ready;
        drop(state);

        tracing::info!("Engine state: Ready");
        Ok(())
    }

    /// 获取引擎实例（用于执行扫描）
    ///
    /// 如果引擎未初始化或处于错误状态，返回错误
    pub fn get_engine(&self) -> Result<Arc<ClamAVEngine>, String> {
        let state = self.state.lock().unwrap();

        if !state.is_operational() {
            Err(format!("Engine not operational: {:?}", state))
        } else {
            Ok(self.engine.lock().unwrap().as_ref().unwrap().clone())
        }
    }

    /// 释放引擎资源
    ///
    /// 可以用于热重载病毒库后重新初始化
    pub fn shutdown(&self) {
        tracing::info!("Shutting down ClamAV engine");

        // 释放引擎
        {
            let mut engine = self.engine.lock().unwrap();
            if let Some(e) = engine.take() {
                // Arc 会自动处理引用计数
                drop(e);
            }
        }

        // 重置状态
        *self.state.lock().unwrap() = EngineState::Uninitialized;

        tracing::info!("Engine shutdown complete");
    }

    /// 获取当前引擎状态
    pub fn get_state(&self) -> EngineState {
        self.state.lock().unwrap().clone()
    }

    /// 引擎健康检查
    ///
    /// 定期调用以检测引擎是否正常工作
    pub fn health_check(&self) -> bool {
        let state = self.get_state();
        state.is_operational()
    }

    /// 病毒库热重载
    ///
    /// 重新加载病毒库，用于 freshclam 更新后调用
    pub fn reload(&self) -> Result<(), String> {
        tracing::info!("Reloading ClamAV engine with new database");

        // 先关闭当前引擎
        self.shutdown();

        // 重新初始化
        self.initialize()?;

        tracing::info!("Engine reloaded successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_manager_initialization() {
        let manager = EngineManager::new(ClamAVConfig {
            database_dir: "/test/db".to_string(),
            ..Default::default()
        });

        // 测试未初始化时获取引擎应该失败
        let result = manager.get_engine();
        assert!(result.is_err());

        // 测试状态
        assert_eq!(manager.get_state(), EngineState::Uninitialized);
    }

    #[test]
    fn test_engine_manager_shutdown() {
        let manager = EngineManager::new(ClamAVConfig {
            database_dir: "/test/db".to_string(),
            ..Default::default()
        });

        // 先初始化
        // 注意：由于 ClamAV FFI 可能不存在，这个测试可能失败
        // 实际实现时需要 mock 或条件编译
        let _init_result = manager.initialize();

        // 检查关闭
        manager.shutdown();

        // 验证状态
        assert_eq!(manager.get_state(), EngineState::Uninitialized);

        // 验证引擎已释放
        assert!(manager.engine.lock().unwrap().is_none());
    }
}
