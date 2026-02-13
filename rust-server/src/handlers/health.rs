use axum::{response::Json, extract::State};
use serde_json::json;
use crate::services::AppState;

pub async fn health_check() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "message": "ClamAV Daemon is running",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

pub async fn status(State(state): State<AppState>) -> Json<serde_json::Value> {
    let scan_service = state.scan_service.read().await;
    let clamav_service = &state.clamav;

    // 获取引擎状态，检查是否已就绪
    let engine_state = clamav_service.get_engine_state().await;
    let is_engine_ready = engine_state.is_ready();

    // 只有引擎就绪时才检查扫描状态
    let (scan_id, is_scanning) = if is_engine_ready {
        let sid = scan_service.get_current_scan_id().await;
        let scanning = scan_service.is_scanning().await;
        drop(scan_service);
        (sid, scanning)
    } else {
        drop(scan_service);
        (None, false)
    };

    // 根据引擎状态返回状态
    let service_status = if is_engine_ready {
        "running"
    } else {
        match &engine_state {
            crate::clamav::EngineState::Initializing => "initializing",
            crate::clamav::EngineState::Error(_) => "error",
            crate::clamav::EngineState::Failed => "failed",
            _ => "starting",
        }
    };

    Json(json!({
        "status": service_status,
        "version": "1.0.0",
        "service": "clamav-daemon",
        "scan_in_progress": is_scanning,
        "current_scan_id": scan_id,
        "engine_ready": is_engine_ready
    }))
}
