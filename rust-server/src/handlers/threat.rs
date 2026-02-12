use axum::{extract::{Path, State}, response::Json};
use serde_json::json;
use crate::services::AppState;
use crate::models::threat::*;

pub async fn list_threats(
    State(state): State<AppState>,
) -> Json<ThreatsListResponse> {
    match state.db.get_threats(None, 100) {
        Ok(threats) => {
            let items: Vec<ThreatItem> = threats.into_iter().map(|t| ThreatItem {
                id: t.id,
                scan_id: t.scan_id,
                file_path: t.file_path,
                virus_name: t.virus_name,
                action_taken: t.action_taken,
                quarantine_uuid: t.original_location,
                action_time: t.action_time,
            }).collect();

            Json(ThreatsListResponse {
                total: items.len() as u32,
                items,
            })
        }
        Err(_) => Json(ThreatsListResponse {
            total: 0,
            items: vec![],
        }),
    }
}

pub async fn handle_threat(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<ThreatHandleRequest>,
) -> Json<ThreatHandleResponse> {
    // 获取威胁记录
    let threat = match state.db.get_threat_by_id(id) {
        Ok(Some(t)) => t,
        Ok(None) => {
            return Json(ThreatHandleResponse {
                success: false,
                threat: None,
                error: Some("Threat not found".to_string()),
            });
        }
        Err(e) => {
            return Json(ThreatHandleResponse {
                success: false,
                threat: None,
                error: Some(e.to_string()),
            });
        }
    };

    match req.action.as_str() {
        "quarantine" => {
            // 隔离文件
            let quarantine_service = crate::services::QuarantineService::new(state.env.clone());

            match quarantine_service.quarantine_file(
                &threat.file_path,
                &threat.virus_name,
                &threat.scan_id,
                0,  // file_size - TODO: 获取实际文件大小
            ) {
                Ok(uuid) => {
                    // 更新威胁记录
                    let _ = state.db.update_threat_action(id, "quarantined", Some(&uuid));

                    Json(ThreatHandleResponse {
                        success: true,
                        threat: Some(ThreatItem {
                            id: threat.id,
                            scan_id: threat.scan_id,
                            file_path: threat.file_path.clone(),
                            virus_name: threat.virus_name,
                            action_taken: Some("quarantined".to_string()),
                            quarantine_uuid: Some(uuid),
                            action_time: Some(chrono::Utc::now().timestamp()),
                        }),
                        error: None,
                    })
                }
                Err(e) => Json(ThreatHandleResponse {
                    success: false,
                    threat: None,
                    error: Some(format!("Failed to quarantine: {}", e)),
                }),
            }
        }
        "delete" => {
            // 删除文件
            match std::fs::remove_file(&threat.file_path) {
                Ok(()) => {
                    // 更新威胁记录
                    let _ = state.db.update_threat_action(id, "deleted", None);

                    Json(ThreatHandleResponse {
                        success: true,
                        threat: Some(ThreatItem {
                            id: threat.id,
                            scan_id: threat.scan_id,
                            file_path: threat.file_path.clone(),
                            virus_name: threat.virus_name,
                            action_taken: Some("deleted".to_string()),
                            quarantine_uuid: None,
                            action_time: Some(chrono::Utc::now().timestamp()),
                        }),
                        error: None,
                    })
                }
                Err(e) => Json(ThreatHandleResponse {
                    success: false,
                    threat: None,
                    error: Some(format!("Failed to delete: {}", e)),
                }),
            }
        }
        "ignore" => {
            // 忽略威胁
            let _ = state.db.update_threat_action(id, "ignored", None);

            Json(ThreatHandleResponse {
                success: true,
                threat: Some(ThreatItem {
                    id: threat.id,
                    scan_id: threat.scan_id,
                    file_path: threat.file_path.clone(),
                    virus_name: threat.virus_name,
                    action_taken: Some("ignored".to_string()),
                    quarantine_uuid: None,
                    action_time: Some(chrono::Utc::now().timestamp()),
                }),
                error: None,
            })
        }
        _ => Json(ThreatHandleResponse {
            success: false,
            threat: None,
            error: Some("Invalid action".to_string()),
        }),
    }
}
