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
    let scan_id = scan_service.get_current_scan_id().await;
    let is_scanning = scan_service.is_scanning().await;
    drop(scan_service);

    Json(json!({
        "status": "running",
        "version": "1.0.0",
        "service": "clamav-daemon",
        "scan_in_progress": is_scanning,
        "current_scan_id": scan_id
    }))
}
