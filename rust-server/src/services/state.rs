use crate::env::FnosEnv;
use crate::services::Database;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 应用共享状态
#[derive(Clone)]
pub struct AppState {
    pub env: FnosEnv,
    pub db: Arc<Database>,
    pub scan_service: Arc<tokio::sync::RwLock<crate::services::ScanService>>,
    pub update_service: Arc<tokio::sync::RwLock<crate::services::UpdateService>>,
}

impl AppState {
    pub fn new(env: FnosEnv) -> Self {
        let db = Arc::new(Database::new(&env.history_db()));

        let scan_service = Arc::new(tokio::sync::RwLock::new(
            crate::services::ScanService::new(db.clone(), env.clone())
        ));

        let update_service = Arc::new(tokio::sync::RwLock::new(
            crate::services::UpdateService::new(db.clone(), env.clone())
        ));

        Self {
            env,
            db,
            scan_service,
            update_service,
        }
    }
}
