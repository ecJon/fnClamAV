use axum::{extract::State, response::Json};
use serde_json::json;
use crate::services::AppState;
use crate::models::update::*;

pub async fn start_update(
    State(state): State<AppState>,
) -> Json<UpdateResponse> {
    let result = state.update_service.write().await
        .start_update().await;

    let start_time = chrono::Utc::now().timestamp();

    match result {
        Ok(r) => Json(UpdateResponse {
            success: r.success,
            status: if r.success { "success".to_string() } else { "failed".to_string() },
            start_time: Some(start_time),
            error: None,
        }),
        Err(e) => Json(UpdateResponse {
            success: false,
            status: "error".to_string(),
            start_time: Some(start_time),
            error: Some(e),
        }),
    }
}

pub async fn update_status(
    State(state): State<AppState>,
) -> Json<UpdateStatusResponse> {
    let status = state.update_service.read().await.get_status().await;

    // 获取最新更新历史
    let last_update = state.db.get_update_history(1).ok()
        .and_then(|h| h.into_iter().next());

    let last_update_time = last_update.as_ref().and_then(|h| {
        if h.end_time.is_some() {
            h.end_time
        } else {
            Some(h.start_time)
        }
    });

    let (old_version, new_version) = last_update.as_ref()
        .map(|h| (h.old_version.clone(), h.new_version.clone()))
        .unwrap_or((None, None));

    Json(UpdateStatusResponse {
        status: if status.is_updating { "updating" } else { "idle" }.to_string(),
        current_version: VirusVersion {
            daily: new_version.as_ref().or(old_version.as_ref()).cloned(),
            main: None,
            bytecode: None,
        },
        last_update: last_update_time,
        next_scheduled: None,  // TODO: 从配置计算
        update_frequency: "daily".to_string(),  // TODO: 从配置读取
    })
}

pub async fn update_version(
    State(_state): State<AppState>,
) -> Json<serde_json::Value> {
    // 读取病毒库目录获取版本信息
    let db_dir = std::env::var("TRIM_DATA_SHARE_PATHS")
        .unwrap_or_else(|_| "/tmp/clamav_data".to_string());
    let db_dir = format!("{}/clamav", db_dir.split(':').next().unwrap());

    let mut daily_version = None;
    let mut main_version = None;
    let mut bytecode_version = None;

    // 尝试读取 freshclam.dat 获取版本
    let freshclam_dat = format!("{}/freshclam.dat", db_dir);
    if let Ok(content) = std::fs::read_to_string(&freshclam_dat) {
        // 解析版本信息
        for line in content.lines() {
            if line.starts_with("Daily:") {
                daily_version = Some(line.split(':').nth(1).unwrap_or("unknown").trim().to_string());
            } else if line.starts_with("Main:") {
                main_version = Some(line.split(':').nth(1).unwrap_or("unknown").trim().to_string());
            } else if line.starts_with("Bytecode:") {
                bytecode_version = Some(line.split(':').nth(1).unwrap_or("unknown").trim().to_string());
            }
        }
    }

    // 备选方案：检查 .cvd/.cld 文件的修改时间
    if daily_version.is_none() {
        if let Ok(entries) = std::fs::read_dir(&db_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.contains("daily") {
                    if let Ok(meta) = entry.metadata() {
                        if let Ok(mtime) = meta.modified() {
                            let age_days = chrono::Utc::now().timestamp() - mtime.elapsed().unwrap().as_secs() as i64;
                            daily_version = Some(format!("{} days old", age_days / 86400));
                        }
                    }
                }
            }
        }
    }

    Json(json!({
        "version": {
            "daily": daily_version,
            "main": main_version,
            "bytecode": bytecode_version
        },
        "age_days": None::<Option<f64>>  // TODO: 计算
    }))
}

pub async fn update_history(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    match state.db.get_update_history(50) {
        Ok(history) => {
            let items: Vec<serde_json::Value> = history.into_iter().map(|h| {
                json!({
                    "id": h.id,
                    "time": h.start_time,
                    "result": h.result,
                    "old_version": h.old_version,
                    "new_version": h.new_version,
                    "duration_seconds": h.end_time.unwrap_or(h.start_time) - h.start_time
                })
            }).collect();

            Json(json!({
                "success": true,
                "records": items,
                "total": items.len()
            }))
        }
        Err(e) => Json(json!({
            "success": false,
            "error": e.to_string()
        }))
    }
}
