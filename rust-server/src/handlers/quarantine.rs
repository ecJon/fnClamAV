use axum::{extract::{Path, State}, response::Json};
use serde_json::json;
use crate::services::{AppState, QuarantineService};
use crate::models::quarantine::*;

pub async fn list_quarantine(
    State(state): State<AppState>,
) -> Json<QuarantineListResponse> {
    let service = QuarantineService::new(state.env.clone());

    match service.list_files() {
        Ok(items) => {
            let total_size: u64 = items.iter().map(|i| i.file_size).sum();

            Json(QuarantineListResponse {
                total: items.len() as u32,
                total_size_bytes: total_size,
                items,
            })
        }
        Err(e) => Json(QuarantineListResponse {
            total: 0,
            total_size_bytes: 0,
            items: vec![],
        }),
    }
}

pub async fn restore_quarantine(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
) -> Json<QuarantineRestoreResponse> {
    let service = QuarantineService::new(state.env.clone());

    match service.restore_file(&uuid) {
        Ok(restored_to) => {
            // 更新数据库
            let _ = state.db.mark_quarantine_restored(&uuid);

            Json(QuarantineRestoreResponse {
                success: true,
                restored_to: Some(restored_to),
                error: None,
            })
        }
        Err(e) => Json(QuarantineRestoreResponse {
            success: false,
            restored_to: None,
            error: Some(e),
        }),
    }
}

pub async fn delete_quarantine(
    State(state): State<AppState>,
    Path(uuid): Path<String>,
) -> Json<serde_json::Value> {
    let service = QuarantineService::new(state.env.clone());

    match service.delete_file(&uuid) {
        Ok(()) => {
            // 更新数据库
            let _ = state.db.delete_quarantine_record(&uuid);

            Json(json!({
                "success": true,
                "uuid": uuid
            }))
        }
        Err(e) => Json(json!({
            "success": false,
            "error": e
        })),
    }
}

pub async fn cleanup_quarantine(
    State(state): State<AppState>,
) -> Json<QuarantineCleanupResponse> {
    let service = QuarantineService::new(state.env.clone());

    // 清理 90 天以上的隔离文件
    match service.cleanup_old(90) {
        Ok((count, bytes)) => Json(QuarantineCleanupResponse {
            success: true,
            cleaned_count: count,
            freed_bytes: bytes,
        }),
        Err(e) => Json(QuarantineCleanupResponse {
            success: false,
            cleaned_count: 0,
            freed_bytes: 0,
        }),
    }
}
