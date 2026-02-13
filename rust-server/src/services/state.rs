use crate::env::FnosEnv;
use crate::services::{Database, ClamavService, ScanService, UpdateService};
use crate::models::config::ClamAVConfig;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 应用共享状态
#[derive(Clone)]
pub struct AppState {
    pub env: FnosEnv,
    pub db: Arc<Database>,
    pub clamav: Arc<ClamavService>,
    pub scan_service: Arc<tokio::sync::RwLock<ScanService>>,
    pub update_service: Arc<tokio::sync::RwLock<UpdateService>>,
}

impl AppState {
    pub fn new(env: FnosEnv) -> Self {
        let db = Arc::new(Database::new(&env.history_db()));

        // 创建 ClamAV 配置
        let clamav_config = ClamAVConfig {
            database_dir: env.clamav_db_dir(),
            certs_dir: Some(format!("{}/certs", env.app_dest)),
            lib_path: Some(format!("{}/lib/libclamav.so", env.app_dest)),
            ..Default::default()
        };

        // 创建 ClamAV 服务
        let clamav = Arc::new(ClamavService::new(clamav_config));

        let scan_service = Arc::new(tokio::sync::RwLock::new(
            ScanService::new(db.clone(), (*clamav).clone())
        ));

        let update_service = Arc::new(tokio::sync::RwLock::new(
            UpdateService::new(db.clone(), env.clone())
        ));

        Self {
            env,
            db,
            clamav,
            scan_service,
            update_service,
        }
    }
}
