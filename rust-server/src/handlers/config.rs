use axum::{extract::State, response::Json};
use serde_json::json;
use crate::services::AppState;
use crate::models::config::*;

pub async fn get_config(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    // 尝试从文件读取配置
    let settings_file = state.env.settings_file();

    let config = if let Ok(content) = std::fs::read_to_string(&settings_file) {
        if let Ok(app_config) = serde_json::from_str::<AppConfig>(&content) {
            app_config
        } else {
            // 解析失败，返回默认配置
            AppConfig::default()
        }
    } else {
        // 文件不存在，返回默认配置
        AppConfig::default()
    };

    // 返回前端期望的格式
    Json(json!({
        "scan_paths": config.scan.exclude_paths,
        "auto_update": config.update.auto_check,
        "quarantine_enabled": config.threat.auto_action,
        "threat_action": config.threat.action
    }))
}

pub async fn update_config(
    State(state): State<AppState>,
    Json(partial): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    // 读取现有配置
    let settings_file = state.env.settings_file();

    // 确保配置目录存在
    if let Some(parent) = std::path::Path::new(&settings_file).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut config = if let Ok(content) = std::fs::read_to_string(&settings_file) {
        serde_json::from_str::<AppConfig>(&content).unwrap_or_default()
    } else {
        AppConfig::default()
    };

    // 支持前端发送的简化格式
    // 前端格式: { scan_paths, auto_update, quarantine_enabled, threat_action }
    if let Some(paths) = partial.get("scan_paths") {
        if let Some(paths_array) = paths.as_array() {
            config.scan.exclude_paths = paths_array.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
        } else if let Some(paths_str) = paths.as_str() {
            // 支持字符串格式的路径列表（换行分隔）
            config.scan.exclude_paths = paths_str
                .split('\n')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
    }

    if let Some(auto_update) = partial.get("auto_update").and_then(|v| v.as_bool()) {
        config.update.auto_check = auto_update;
    }

    if let Some(quarantine_enabled) = partial.get("quarantine_enabled").and_then(|v| v.as_bool()) {
        config.threat.auto_action = quarantine_enabled;
    }

    if let Some(threat_action) = partial.get("threat_action").and_then(|v| v.as_str()) {
        config.threat.action = threat_action.to_string();
    }

    // 也支持原有的嵌套格式
    if let Some(scan) = partial.get("scan").and_then(|v| v.as_object()) {
        if let Some(default_type) = scan.get("default_scan_type").and_then(|v| v.as_str()) {
            config.scan.default_scan_type = default_type.to_string();
        }
        if let Some(exclude) = scan.get("exclude_paths").and_then(|v| v.as_array()) {
            config.scan.exclude_paths = exclude.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
        }
        if let Some(max_size) = scan.get("max_file_size_mb").and_then(|v| v.as_u64()) {
            config.scan.max_file_size_mb = max_size as u32;
        }
        if let Some(archives) = scan.get("scan_archives").and_then(|v| v.as_bool()) {
            config.scan.scan_archives = archives;
        }
    }

    if let Some(threat) = partial.get("threat").and_then(|v| v.as_object()) {
        if let Some(action) = threat.get("action").and_then(|v| v.as_str()) {
            config.threat.action = action.to_string();
        }
        if let Some(auto) = threat.get("auto_action").and_then(|v| v.as_bool()) {
            config.threat.auto_action = auto;
        }
    }

    if let Some(update) = partial.get("update").and_then(|v| v.as_object()) {
        if let Some(freq) = update.get("frequency").and_then(|v| v.as_str()) {
            config.update.frequency = freq.to_string();
        }
        if let Some(time) = update.get("schedule_time").and_then(|v| v.as_str()) {
            config.update.schedule_time = time.to_string();
        }
        if let Some(tz) = update.get("timezone").and_then(|v| v.as_str()) {
            config.update.timezone = tz.to_string();
        }
        if let Some(check) = update.get("auto_check").and_then(|v| v.as_bool()) {
            config.update.auto_check = check;
        }
    }

    if let Some(history) = partial.get("history").and_then(|v| v.as_object()) {
        if let Some(days) = history.get("retention_days").and_then(|v| v.as_u64()) {
            config.history.retention_days = days as u32;
        }
        if let Some(max) = history.get("max_records").and_then(|v| v.as_u64()) {
            config.history.max_records = max as u32;
        }
    }

    // 保存配置
    let config_json = serde_json::to_string_pretty(&config).unwrap();
    match std::fs::write(&settings_file, config_json) {
        Ok(_) => {
            tracing::info!("Configuration saved to {}", settings_file);
            Json(json!({
                "success": true,
                "message": "配置已保存"
            }))
        }
        Err(e) => {
            tracing::error!("Failed to save configuration: {}", e);
            Json(json!({
                "success": false,
                "error": format!("保存配置失败: {}", e)
            }))
        }
    }
}

fn get_default_config() -> ConfigResponse {
    ConfigResponse {
        scan: crate::models::config::ScanConfig::default(),
        threat: crate::models::config::ThreatConfig::default(),
        update: crate::models::config::UpdateConfig::default(),
        history: crate::models::config::HistoryConfig::default(),
    }
}
