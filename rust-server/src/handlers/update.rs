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

    // 从 CVD/CLD 文件头解析版本信息
    // CVD 文件头格式: ClamAV-VDB:日期:版本号:...
    fn parse_cvd_version(path: &str) -> Option<String> {
        use std::io::Read;
        if let Ok(mut file) = std::fs::File::open(path) {
            let mut header = [0u8; 512];
            if file.read(&mut header).is_ok() {
                // 转换为字符串并查找版本号
                if let Ok(header_str) = std::str::from_utf8(&header) {
                    // 格式: ClamAV-VDB:10 Feb 2026 07-25 +0000:27908:...
                    // 版本号在第三个冒号分隔的位置
                    let parts: Vec<&str> = header_str.split(':').collect();
                    if parts.len() >= 3 {
                        let version = parts[2].trim();
                        // 验证是否为数字
                        if version.parse::<u32>().is_ok() {
                            return Some(version.to_string());
                        }
                    }
                }
            }
        }
        None
    }

    // 检查 daily.cvd 或 daily.cld
    let daily_path = format!("{}/daily.cvd", db_dir);
    let daily_path_cld = format!("{}/daily.cld", db_dir);
    daily_version = parse_cvd_version(&daily_path)
        .or_else(|| parse_cvd_version(&daily_path_cld));

    // 检查 main.cvd 或 main.cld
    let main_path = format!("{}/main.cvd", db_dir);
    let main_path_cld = format!("{}/main.cld", db_dir);
    main_version = parse_cvd_version(&main_path)
        .or_else(|| parse_cvd_version(&main_path_cld));

    // 检查 bytecode.cvd 或 bytecode.cld
    let bytecode_path = format!("{}/bytecode.cvd", db_dir);
    let bytecode_path_cld = format!("{}/bytecode.cld", db_dir);
    bytecode_version = parse_cvd_version(&bytecode_path)
        .or_else(|| parse_cvd_version(&bytecode_path_cld));

    Json(json!({
        "version": {
            "daily": daily_version,
            "main": main_version,
            "bytecode": bytecode_version
        },
        "age_days": None::<Option<f64>>
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
