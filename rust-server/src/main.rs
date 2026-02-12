mod clamav;
mod env;
mod models;
mod services;
mod handlers;

use std::net::SocketAddr;
use std::path::PathBuf;
use axum::{
    routing::{get, post},
    Router,
    http::StatusCode,
    response::Json,
};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use models::config::ConfigResponse;

use handlers::{scan, update, config, threat, quarantine, health};

// 服务器端口
const SERVER_PORT: u16 = 8899;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "clamav_daemon=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting ClamAV Daemon...");

    // 初始化环境
    let env = env::FnosEnv::from_env().map_err(|e| anyhow::anyhow!(e))?;
    tracing::info!("Environment loaded: data_dir={}", env.data_dir());

    // 确保必要的目录存在
    std::fs::create_dir_all(env.data_dir())?;
    std::fs::create_dir_all(env.clamav_db_dir())?;
    std::fs::create_dir_all(env.quarantine_dir())?;
    std::fs::create_dir_all(&format!("{}/metadata", env.quarantine_dir()))?;
    std::fs::create_dir_all(&format!("{}/files", env.quarantine_dir()))?;
    std::fs::create_dir_all(&env.pkg_var)?;
    std::fs::create_dir_all(&env.pkg_etc)?;

    // 初始化病毒数据库：如果目标目录为空，从预装的数据库复制
    initialize_virus_db(&env)?;

    // 初始化数据库
    services::db::init_db(&env.history_db())?;

    // 构建应用状态
    let app_state = services::AppState::new(env.clone());

    // 初始化 ClamAV 引擎（FFI 版本）
    tracing::info!("Initializing ClamAV engine...");

    // 初始化引擎
    {
        let scan_service = app_state.scan_service.read().await;
        if let Err(e) = scan_service.clamav.initialize().await {
            tracing::error!("Failed to initialize ClamAV engine: {}", e);
            return Err(anyhow::anyhow!("ClamAV engine initialization failed: {}", e));
        }
        tracing::info!("ClamAV engine initialized successfully");

        // 启动扫描引擎
        if let Err(e) = scan_service.clamav.start_scan_engine().await {
            tracing::error!("Failed to start scan engine: {}", e);
            return Err(anyhow::anyhow!("Scan engine start failed: {}", e));
        }
        tracing::info!("Scan engine started successfully");
    }

    // 构建路由
    let app = Router::new()
        // 健康检查
        .route("/health", get(health::health_check))
        .route("/api/status", get(health::status))

        // 扫描相关
        .route("/api/scan/start", post(scan::start_scan))
        .route("/api/scan/stop", post(scan::stop_scan))
        .route("/api/scan/status", get(scan::scan_status))
        .route("/api/scan/history", get(scan::scan_history))

        // 更新相关
        .route("/api/update/start", post(update::start_update))
        .route("/api/update/status", get(update::update_status))
        .route("/api/update/version", get(update::update_version))
        .route("/api/update/history", get(update::update_history))

        // 威胁处理
        .route("/api/threats", get(threat::list_threats))
        .route("/api/threats/:id/handle", post(threat::handle_threat))

        // 隔离区管理
        .route("/api/quarantine", get(quarantine::list_quarantine))
        .route("/api/quarantine/:uuid/restore", post(quarantine::restore_quarantine))
        .route("/api/quarantine/:uuid", axum::routing::delete(quarantine::delete_quarantine))
        .route("/api/quarantine/cleanup", post(quarantine::cleanup_quarantine))

        // 配置管理
        .route("/api/config", get(config::get_config).put(config::update_config))

        // CORS 支持
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())

        // 注入应用状态
       .with_state(app_state);

    // 绑定地址
    let addr = SocketAddr::from(([127, 0, 0, 1], SERVER_PORT));
    tracing::info!("Server listening on http://{}", addr);

    // 写入 PID 文件
    // let pid = std::process::id();
    // std::fs::write(&format!("{}/daemon.pid", env.pkg_var()), pid.to_string())?;

    // 启动服务器
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// 初始化病毒数据库
/// 如果目标数据库目录为空，从预装的数据库复制
fn initialize_virus_db(env: &env::FnosEnv) -> anyhow::Result<()> {
    let target_dir = &env.clamav_db_dir();

    // 检查目标目录是否已有数据库文件
    let has_db = std::fs::read_dir(target_dir)?
        .filter_map(|e| e.ok())
        .any(|entry| {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            name_str.ends_with(".cvd") || name_str.ends_with(".cld") || name_str == "daily.cvd" || name_str == "main.cvd"
        });

    if has_db {
        tracing::info!("Virus database already exists in {}", target_dir);
        return Ok(());
    }

    // 尝试从预装位置复制数据库
    let source_dirs = vec![
        format!("{}/share/clamav", env.app_dest),  // 预装位置
        format!("{}/../share/clamav", env.app_dest),  // 开发环境
        "/var/lib/clamav".to_string(),  // 系统位置
    ];

    for source_dir in source_dirs {
        if let Ok(mut entries) = std::fs::read_dir(&source_dir) {
            let db_files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|entry| {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    name_str.ends_with(".cvd") || name_str.ends_with(".cld") || name_str == "freshclam.dat"
                })
                .collect();

            if !db_files.is_empty() {
                tracing::info!("Copying virus database from {} to {}", source_dir, target_dir);
                for entry in db_files {
                    let src = entry.path();
                    let dest_name = entry.file_name();
                    let dest = std::path::PathBuf::from(target_dir).join(&dest_name);

                    if let Err(e) = std::fs::copy(&src, &dest) {
                        tracing::warn!("Failed to copy {:?} to {:?}: {}", src, dest, e);
                    } else {
                        tracing::info!("Copied {:?}", dest_name);
                    }
                }
                return Ok(());
            }
        }
    }

    tracing::warn!("No pre-installed virus database found. Please run freshclam to update.");
    Ok(())
}
