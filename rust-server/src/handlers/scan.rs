use axum::{extract::State, response::Json};
use serde_json::json;
use crate::services::AppState;
use crate::models::scan::*;
use crate::clamav::engine::TaskPriority;
use crate::clamav::ScanOptions;

pub async fn start_scan(
    State(state): State<AppState>,
    Json(req): Json<ScanRequest>,
) -> Json<ScanResponse> {
    let scan_id = format!("scan_{:}", chrono::Utc::now().format("%Y%m%d_%H%M%S"));

    // 确定扫描路径
    let paths = if req.scan_type == ScanType::Full {
        // 全盘扫描：从 /proc/mounts 获取挂载点
        get_full_scan_paths()
    } else {
        req.paths.unwrap_or_default()
    };

    if paths.is_empty() {
        return Json(ScanResponse {
            success: false,
            scan_id: None,
            status: None,
            error: Some("No valid scan paths".to_string()),
        });
    }

    // 创建数据库记录
    let scan_type_str = match req.scan_type {
        ScanType::Full => "full",
        ScanType::Custom => "custom",
    };

    if let Err(e) = state.db.create_scan(&scan_id, scan_type_str, &paths) {
        return Json(ScanResponse {
            success: false,
            scan_id: None,
            status: None,
            error: Some(format!("Failed to create scan: {}", e)),
        });
    }

    // 启动后台扫描
    let result = state.scan_service.write().await
        .start_scan(
            scan_id.clone(),
            paths.clone(),
            TaskPriority::Normal,
            ScanOptions::default(),
        ).await;

    match result {
        Ok(task_id) => Json(ScanResponse {
            success: true,
            scan_id: Some(scan_id),
            status: Some("scanning".to_string()),
            error: None,
        }),
        Err(e) => Json(ScanResponse {
            success: false,
            scan_id: None,
            status: None,
            error: Some(e.to_string()),
        }),
    }
}

pub async fn stop_scan(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let scan_service = state.scan_service.read().await;
    let scan_id = scan_service.get_current_scan_id().await;

    match scan_id {
        Some(sid) => {
            let result = scan_service.stop_scan(&sid).await;
            match result {
                Ok(()) => Json(json!({
                    "success": true,
                    "scan_id": sid,
                    "status": "stopped"
                })),
                Err(e) => Json(json!({
                    "success": false,
                    "error": e.to_string()
                })),
            }
        }
        None => Json(json!({
            "success": false,
            "error": "No scan in progress"
        })),
    }
}

pub async fn scan_status(
    State(state): State<AppState>,
) -> Json<ScanStatusResponse> {
    // 首先检查是否有活跃的扫描（从内存中获取实时数据）
    let scan_service = state.scan_service.read().await;
    let current_scan_id = scan_service.get_current_scan_id().await;

    // 优先从内存获取实时进度数据
    let realtime_progress = scan_service.get_current_scan_progress().await;
    drop(scan_service);

    // 如果有活跃扫描且有实时进度数据，优先返回实时数据
    if let (Some(scan_id), Some(progress)) = (current_scan_id, realtime_progress) {
        let elapsed = chrono::Utc::now().timestamp() - progress.created_at.timestamp();
        let scan_status = progress.status.clone();
        let is_scanning = scan_status == "scanning";

        // 使用 discovered_files 计算进度（更准确的实时总数）
        let effective_total = if progress.discovered_files > 0 {
            progress.discovered_files
        } else {
            progress.total_files
        };

        return Json(ScanStatusResponse {
            scan_id: Some(progress.scan_id.clone()),
            status: scan_status.clone(),
            progress: if is_scanning || scan_status == "completed" {
                Some(ScanProgress {
                    percent: if effective_total > 0 {
                        (progress.scanned_files as f32 / effective_total as f32) * 100.0
                    } else {
                        0.0
                    },
                    scanned: progress.scanned_files as u64,
                    estimated_total: effective_total as u64,
                    current_file: progress.current_file.clone().unwrap_or_default(),
                    discovered: if progress.discovered_files > 0 {
                        Some(progress.discovered_files as u64)
                    } else {
                        None
                    },
                    scan_rate: if progress.scan_rate > 0.0 {
                        Some(progress.scan_rate)
                    } else {
                        None
                    },
                })
            } else {
                None
            },
            threats: Some(ThreatsInfo {
                count: progress.threats_found as u32,
                files: vec![],
            }),
            start_time: Some(progress.created_at.timestamp()),
            elapsed_seconds: Some(elapsed.max(0) as u64),
        });
    }

    // 没有实时进度数据，回退到从数据库获取最近的状态
    match state.db.get_current_scan() {
        Ok(Some(scan)) => {
            let elapsed = chrono::Utc::now().timestamp() - scan.start_time;
            let scan_status = scan.status.clone();
            let is_scanning = scan_status == "scanning";

            Json(ScanStatusResponse {
                scan_id: Some(scan.scan_id),
                status: scan_status,
                progress: if is_scanning || scan.status == "completed" {
                    Some(ScanProgress {
                        percent: if scan.total_files > 0 {
                            (scan.scanned_files as f32 / scan.total_files as f32) * 100.0
                        } else {
                            0.0
                        },
                        scanned: scan.scanned_files as u64,
                        estimated_total: scan.total_files as u64,
                        current_file: scan.current_file.unwrap_or_default(),
                        discovered: None,
                        scan_rate: None,
                    })
                } else {
                    None
                },
                threats: Some(ThreatsInfo {
                    count: scan.threats_found as u32,
                    files: vec![],
                }),
                start_time: Some(scan.start_time),
                elapsed_seconds: Some(elapsed.max(0) as u64),
            })
        }
        Ok(None) | Err(_) => {
            Json(ScanStatusResponse {
                scan_id: None,
                status: "idle".to_string(),
                progress: None,
                threats: None,
                start_time: None,
                elapsed_seconds: None,
            })
        }
    }
}

pub async fn scan_history(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    match state.db.get_scan_history(50) {
        Ok(history) => {
            let items: Vec<serde_json::Value> = history.into_iter().map(|h| {
                json!({
                    "id": h.id,
                    "scan_id": h.scan_id,
                    "scan_type": h.scan_type,
                    "paths": h.paths,
                    "status": h.status,
                    "start_time": h.start_time,
                    "end_time": h.end_time,
                    "total_files": h.total_files,
                    "scanned_files": h.scanned_files,
                    "threats_found": h.threats_found,
                    "error_message": h.error_message
                })
            }).collect();

            Json(json!({
                "success": true,
                "items": items,
                "total": items.len()
            }))
        }
        Err(e) => Json(json!({
            "success": false,
            "error": e.to_string()
        }))
    }
}

/// 删除单条扫描历史记录
pub async fn delete_scan_history(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Json<serde_json::Value> {
    match state.db.delete_scan_history(id) {
        Ok(()) => Json(json!({
            "success": true
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e.to_string()
        }))
    }
}

/// 清空所有扫描历史记录
pub async fn clear_scan_history(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    match state.db.clear_scan_history() {
        Ok(()) => Json(json!({
            "success": true
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e.to_string()
        }))
    }
}

// 获取全盘扫描路径
fn get_full_scan_paths() -> Vec<String> {
    let mut paths = Vec::new();

    // 全盘扫描策略：只扫描用户数据共享目录
    // 从 /proc/mounts 读取挂载点，只选择以下类型的挂载点：
    // 1. /vol1, /vol2 等数据卷
    // 2. /home, /root 等用户目录
    // 3. /data, /mnt 等常见数据挂载点
    // 排除：系统目录、Docker overlay、ZFS 快照、应用目录

    if let Ok(content) = std::fs::read_to_string("/proc/mounts") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let mount_point = parts[1];
                let fs_type = if parts.len() >= 3 { parts[2] } else { "" };

                // 排除系统路径
                if mount_point.starts_with("/proc")
                    || mount_point.starts_with("/sys")
                    || mount_point.starts_with("/dev")
                    || mount_point.starts_with("/run")
                    || mount_point == "/tmp"
                    || mount_point == "/snap"
                {
                    continue;
                }

                // 排除 ZFS 快照
                if mount_point.contains(".zfs/snapshot") {
                    continue;
                }

                // 排除 Docker overlay 文件系统
                if fs_type == "overlay" || mount_point.contains("overlay2/merged") {
                    continue;
                }

                // 排除应用中心目录
                if mount_point.contains("/@appcenter/") {
                    continue;
                }

                // 排除 proc 和 sysfs 类型的挂载
                if fs_type == "proc" || fs_type == "sysfs" || fs_type == "debugfs" || fs_type == "tracefs" {
                    continue;
                }

                // 排除特殊挂载点
                if mount_point.contains("/rpc_pipefs")
                    || mount_point.contains("/binfmt_misc")
                    || mount_point.contains("/nfsd")
                    || mount_point.contains("/fuse/connections")
                    || mount_point.contains("/bpf")
                    || mount_point.contains("/pstore")
                    || mount_point.contains("/efivars")
                {
                    continue;
                }

                // 扫描主要数据卷
                if mount_point == "/"
                    || mount_point.starts_with("/vol")
                    || mount_point.starts_with("/data")
                    || mount_point.starts_with("/mnt")
                    || mount_point.starts_with("/home")
                    || mount_point.starts_with("/root")
                {
                    paths.push(mount_point.to_string());
                }
            }
        }
    }

    // 去重并排序
    paths.sort();
    paths.dedup();

    // 如果仍然没有找到路径，使用根目录作为最后的备选
    if paths.is_empty() {
        paths.push("/".to_string());
    }

    paths
}
