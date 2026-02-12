use crate::env::FnosEnv;
use crate::services::{Database, ClamavService};
use crate::models::config::ClamAVConfig;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 更新服务
pub struct UpdateService {
    db: Arc<Database>,
    clamav: ClamavService,
    current_update: Arc<RwLock<Option<String>>>,
}

impl UpdateService {
    pub fn new(db: Arc<Database>, env: FnosEnv) -> Self {
        // 从 FnosEnv 构造 ClamAVConfig
        let config = ClamAVConfig {
            database_dir: env.clamav_db_dir(),
            lib_path: Some(format!("{}/lib/libclamav.so", env.app_dest)),
            ..Default::default()
        };
        let clamav = ClamavService::new(config);

        Self {
            db,
            clamav,
            current_update: Arc::new(RwLock::new(None)),
        }
    }

    /// 开始更新
    pub async fn start_update(&self) -> Result<UpdateResultInner, String> {
        // 检查是否已有更新在进行
        let current = self.current_update.read().await;
        if current.is_some() {
            return Err("Update already in progress".to_string());
        }
        drop(current);

        // 设置更新中状态
        let update_id = chrono::Utc::now().timestamp().to_string();
        let mut current = self.current_update.write().await;
        *current = Some(update_id.clone());
        drop(current);

        // 执行更新 - TODO: 实现 FFI 更新功能
        let result: Result<UpdateResultInner, String> = Ok(UpdateResultInner {
            success: true,
            old_version: Some("1.0".to_string()),
            new_version: Some("1.0".to_string()),
        });

        // 记录到数据库
        let db_result = if result.is_ok() {
            let r = result.as_ref().unwrap();
            self.db.add_update_history(
                r.old_version.as_deref(),
                r.new_version.as_deref(),
                if r.success { "success" } else { "failed" },
                result.as_ref().err().map(|e| e.as_str()),
            )
        } else {
            self.db.add_update_history(
                None,
                None,
                "failed",
                result.as_ref().err().map(|e| e.as_str()),
            )
        };

        // 清除更新状态
        let mut current = self.current_update.write().await;
        *current = None;
        drop(current);

        match result {
            Ok(r) => match db_result {
                Ok(_) => Ok(UpdateResultInner {
                    success: r.success,
                    new_version: r.new_version,
                    old_version: r.old_version,
                }),
                Err(e) => Err(format!("Database error: {}", e)),
            }
            Err(e) => Err(e),
        }
    }

    /// 获取更新状态
    pub async fn get_status(&self) -> UpdateStatus {
        let current = self.current_update.read().await;

        UpdateStatus {
            is_updating: current.is_some(),
            current_update_id: current.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateResultInner {
    pub success: bool,
    pub old_version: Option<String>,
    pub new_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateStatus {
    pub is_updating: bool,
    pub current_update_id: Option<String>,
}
