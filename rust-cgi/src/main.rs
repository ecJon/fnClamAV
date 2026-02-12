use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

mod db;
mod clamav;

use db::Database;
use clamav::{generate_scan_id, scan_path, update_signatures};

// MIME 类型映射
fn get_mime_type(filename: &str) -> &'static str {
    let ext = Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext.to_lowercase().as_str() {
        "html" | "htm" => "text/html; charset=UTF-8",
        "css" => "text/css; charset=UTF-8",
        "js" => "application/javascript; charset=UTF-8",
        "json" => "application/json; charset=UTF-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "svg" => "image/svg+xml",
        "txt" | "log" => "text/plain; charset=UTF-8",
        _ => "application/octet-stream",
    }
}

// API 响应结构
#[derive(serde::Serialize)]
struct ApiResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ApiError>,
}

#[derive(serde::Serialize)]
struct ApiError {
    code: String,
    message: String,
}

// 调试日志
fn debug_log(msg: &str) {
    if let Ok(mut file) = File::create("/tmp/rust_cgi_debug.log") {
        use std::io::Write;
        let _ = file.write_all(msg.as_bytes());
    }
}

// 发送 JSON 响应（成功）
fn send_json_response(data: ApiResponse) {
    match serde_json::to_string(&data) {
        Ok(json) => {
            debug_log(&format!("Sending JSON response: {}\n", json));
            println!("Content-Type: application/json; charset=UTF-8");
            println!("Content-Length: {}", json.len());
            println!();
            print!("{}", json);
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
        Err(e) => {
            debug_log(&format!("JSON serialization error: {}\n", e));
            send_error_response(500, "Internal Server Error", "Failed to marshal JSON");
        }
    }
}

// 发送错误响应
fn send_error_response(status_code: u32, status_message: &str, message: &str) {
    let error = ApiError {
        code: status_code.to_string(),
        message: message.to_string(),
    };
    let response = ApiResponse {
        success: false,
        data: None,
        error: Some(error),
    };

    match serde_json::to_string(&response) {
        Ok(json) => {
            debug_log(&format!("Sending error response: {} {}\n", status_code, status_message));
            println!("Status: {} {}", status_code, status_message);
            println!("Content-Type: application/json; charset=UTF-8");
            println!("Content-Length: {}", json.len());
            println!();
            print!("{}", json);
            use std::io::Write;
            let _ = std::io::stdout().flush();
        }
        Err(_) => {
            println!("Status: 500 Internal Server Error");
            println!("Content-Type: text/plain; charset=UTF-8");
            println!();
            println!("{}", message);
        }
    }
}

// 路由处理：/api/status
fn handle_api_status() -> ApiResponse {
    let mut data = serde_json::Map::new();
    data.insert("status".to_string(), serde_json::json!("running"));
    data.insert("version".to_string(), serde_json::json!("1.0.0"));
    data.insert("service".to_string(), serde_json::json!("App.Native.HelloFnosAppCenter"));
    data.insert("rust_cgi".to_string(), serde_json::json!("active"));

    ApiResponse {
        success: true,
        data: Some(serde_json::Value::Object(data)),
        error: None,
    }
}

// 路由处理：/api/health
fn handle_api_health() -> ApiResponse {
    let mut data = serde_json::Map::new();
    data.insert("status".to_string(), serde_json::json!("ok"));
    data.insert("message".to_string(), serde_json::json!("Hello from Rust CGI API!"));
    data.insert("app".to_string(), serde_json::json!("App.Native.HelloFnosAppCenter"));

    ApiResponse {
        success: true,
        data: Some(serde_json::Value::Object(data)),
        error: None,
    }
}

// 路由处理：/api/scan/start
fn handle_api_scan_start(request_body: &str) -> ApiResponse {
    // 解析请求体
    let request: serde_json::Value = match serde_json::from_str(request_body) {
        Ok(v) => v,
        Err(_) => {
            return ApiResponse {
                success: false,
                data: None,
                error: Some(ApiError {
                    code: "INVALID_REQUEST".to_string(),
                    message: "Invalid JSON request body".to_string(),
                }),
            };
        }
    };

    let scan_type = request["scan_type"].as_str().unwrap_or("custom");
    let paths = request["paths"].as_array();

    // 确定扫描路径
    let scan_paths = if scan_type == "full" {
        // 全盘扫描：获取所有挂载点
        vec!["/".to_string()]
    } else if let Some(user_paths) = paths {
        // 自定义路径
        user_paths
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect()
    } else {
        return ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: "INVALID_PATHS".to_string(),
                message: "No scan paths specified".to_string(),
            }),
        };
    };

    // 初始化数据库
    let db = match Database::new() {
        Ok(d) => d,
        Err(e) => {
            return ApiResponse {
                success: false,
                data: None,
                error: Some(ApiError {
                    code: "DATABASE_ERROR".to_string(),
                    message: format!("Failed to connect to database: {}", e),
                }),
            };
        }
    };

    // 检查是否有正在运行的扫描
    if let Ok(Some(current)) = db.get_current_scan() {
        return ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: "SCAN_ALREADY_RUNNING".to_string(),
                message: format!("Scan {} is already running", current.scan_id),
            }),
        };
    }

    // 生成扫描 ID
    let scan_id = generate_scan_id();

    // 创建扫描记录
    if db.create_scan(&scan_id, scan_type, &scan_paths).is_err() {
        return ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: "DATABASE_ERROR".to_string(),
                message: "Failed to create scan record".to_string(),
            }),
        };
    }

    // 执行扫描
    let result = scan_path(&scan_id, &scan_paths, &db);

    // 更新扫描状态
    let status = if result.error_message.is_some() {
        "error"
    } else {
        "completed"
    };
    let _ = db.finish_scan(&scan_id, status, result.total_files, result.error_message.as_deref());

    // 构建响应
    let mut data = serde_json::Map::new();
    data.insert("scan_id".to_string(), serde_json::json!(scan_id));
    data.insert("status".to_string(), serde_json::json!(result.status));
    data.insert("total_files".to_string(), serde_json::json!(result.total_files));
    data.insert("threats_found".to_string(), serde_json::json!(result.threats_found));

    ApiResponse {
        success: result.error_message.is_none(),
        data: Some(serde_json::Value::Object(data)),
        error: result.error_message.map(|msg| ApiError {
            code: "SCAN_ERROR".to_string(),
            message: msg,
        }),
    }
}

// 路由处理：/api/scan/status
fn handle_api_scan_status() -> ApiResponse {
    let db = match Database::new() {
        Ok(d) => d,
        Err(e) => {
            return ApiResponse {
                success: false,
                data: None,
                error: Some(ApiError {
                    code: "DATABASE_ERROR".to_string(),
                    message: format!("Failed to connect to database: {}", e),
                }),
            };
        }
    };

    match db.get_current_scan() {
        Ok(Some(scan)) => {
            let mut data = serde_json::Map::new();
            data.insert("scan_id".to_string(), serde_json::json!(scan.scan_id));
            data.insert("status".to_string(), serde_json::json!(scan.status));
            data.insert("scanned_files".to_string(), serde_json::json!(scan.scanned_files));
            data.insert("threats_found".to_string(), serde_json::json!(scan.threats_found));
            data.insert("start_time".to_string(), serde_json::json!(scan.start_time));

            ApiResponse {
                success: true,
                data: Some(serde_json::Value::Object(data)),
                error: None,
            }
        }
        Ok(None) => {
            let mut data = serde_json::Map::new();
            data.insert("status".to_string(), serde_json::json!("idle"));
            data.insert("scan_id".to_string(), serde_json::json!(serde_json::Value::Null));

            ApiResponse {
                success: true,
                data: Some(serde_json::Value::Object(data)),
                error: None,
            }
        }
        Err(e) => ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: "DATABASE_ERROR".to_string(),
                message: e.to_string(),
            }),
        }
    }
}

// 路由处理：/api/scan/history
fn handle_api_scan_history() -> ApiResponse {
    let db = match Database::new() {
        Ok(d) => d,
        Err(e) => {
            return ApiResponse {
                success: false,
                data: None,
                error: Some(ApiError {
                    code: "DATABASE_ERROR".to_string(),
                    message: format!("Failed to connect to database: {}", e),
                }),
            };
        }
    };

    match db.get_scan_history(50) {
        Ok(history) => {
            ApiResponse {
                success: true,
                data: Some(serde_json::json!(history)),
                error: None,
            }
        }
        Err(e) => ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: "DATABASE_ERROR".to_string(),
                message: e.to_string(),
            }),
        }
    }
}

// 路由处理：/api/update/start
fn handle_api_update_start() -> ApiResponse {
    match update_signatures() {
        Ok(result) => {
            let mut data = serde_json::Map::new();
            data.insert("success".to_string(), serde_json::json!(result.success));
            if let Some(old_ver) = result.old_version {
                data.insert("old_version".to_string(), serde_json::json!(old_ver));
            }
            if let Some(new_ver) = result.new_version {
                data.insert("new_version".to_string(), serde_json::json!(new_ver));
            }

            ApiResponse {
                success: result.success,
                data: Some(serde_json::Value::Object(data)),
                error: result.error_message.map(|msg| ApiError {
                    code: "UPDATE_ERROR".to_string(),
                    message: msg,
                }),
            }
        }
        Err(e) => ApiResponse {
            success: false,
            data: None,
            error: Some(ApiError {
                code: "UPDATE_ERROR".to_string(),
                message: e.to_string(),
            }),
        }
    }
}

// 处理 API 请求
fn handle_api_request(path: &str, method: &str, body: &str) -> bool {
    // 简单路由匹配
    let parts: Vec<&str> = path.split('?').collect();
    let path = parts[0];

    match (method, path) {
        // GET /api/status
        ("GET", "/api/status") => {
            send_json_response(handle_api_status());
            true
        }
        // GET /api/health
        ("GET", "/api/health") => {
            send_json_response(handle_api_health());
            true
        }
        // POST /api/scan/start
        ("POST", "/api/scan/start") => {
            send_json_response(handle_api_scan_start(body));
            true
        }
        // GET /api/scan/status
        ("GET", "/api/scan/status") => {
            send_json_response(handle_api_scan_status());
            true
        }
        // GET /api/scan/history
        ("GET", "/api/scan/history") => {
            send_json_response(handle_api_scan_history());
            true
        }
        // POST /api/update/start
        ("POST", "/api/update/start") => {
            send_json_response(handle_api_update_start());
            true
        }
        _ => false,
    }
}

fn main() {
    // 初始化数据库
    if let Err(e) = db::init_db() {
        debug_log(&format!("Database init error: {}\n", e));
    }

    // 获取 CGI 环境变量
    let request_uri = match env::var("REQUEST_URI") {
        Ok(uri) => uri,
        Err(_) => {
            send_error_response(500, "Internal Server Error", "REQUEST_URI environment variable is not set.");
            return;
        }
    };

    let request_method = env::var("REQUEST_METHOD").unwrap_or_else(|_| "GET".to_string());

    debug_log(&format!("REQUEST_URI: {}\n", request_uri));
    debug_log(&format!("REQUEST_METHOD: {}\n", request_method));

    // 读取请求体（对于 POST 请求）
    let mut request_body = String::new();
    if request_method == "POST" {
        let content_length = env::var("CONTENT_LENGTH").ok();
        if let Some(len_str) = content_length {
            if let Ok(_len) = len_str.parse::<usize>() {
                let _ = io::stdin().read_to_string(&mut request_body);
                debug_log(&format!("REQUEST_BODY: {}\n", request_body));
            }
        }
    }

    // 解析路径
    let cgi_script_name = "/index.cgi";
    let target_path = if let Some(cgi_index) = request_uri.find(cgi_script_name) {
        &request_uri[cgi_index + cgi_script_name.len()..]
    } else {
        "/"
    };

    debug_log(&format!("target_path: '{}'\n", target_path));

    // 判断是否是 API 请求
    if target_path.starts_with("/api/") {
        debug_log(&format!("API request: {}\n", target_path));
        if handle_api_request(target_path, &request_method, &request_body) {
            return;
        }
        send_error_response(404, "Not Found", &format!("API endpoint not found: {}", target_path));
        return;
    }

    // 静态文件服务
    let script_filename = match env::var("SCRIPT_FILENAME") {
        Ok(path) => path,
        Err(_) => {
            send_error_response(500, "Internal Server Error", "SCRIPT_FILENAME environment variable is not set.");
            return;
        }
    };

    debug_log(&format!("SCRIPT_FILENAME: {}\n", script_filename));

    // 获取 web 根目录
    let ui_dir = Path::new(&script_filename).parent().unwrap_or_else(|| Path::new("/"));
    let target_dir = ui_dir.parent().unwrap_or_else(|| Path::new("/"));
    let web_root = target_dir.join("www");

    debug_log(&format!("web_root: {:?}\n", web_root));

    let target_file = if target_path == "/" || target_path.is_empty() {
        web_root.join("index.html")
    } else {
        let clean_path = target_path.trim_start_matches('/');
        web_root.join(clean_path)
    };

    debug_log(&format!("target_file: {:?}\n", target_file));

    // 安全检查：防止路径穿越
    let abs_target = match fs::canonicalize(&target_file) {
        Ok(path) => path,
        Err(_) => {
            debug_log(&format!("canonicalize target_file failed\n"));
            send_error_response(500, "Internal Server Error", "Failed to resolve file path.");
            return;
        }
    };

    let abs_web_root = match fs::canonicalize(&web_root) {
        Ok(path) => path,
        Err(_) => {
            debug_log(&format!("canonicalize web_root failed\n"));
            send_error_response(500, "Internal Server Error", "Failed to resolve web root path.");
            return;
        }
    };

    debug_log(&format!("abs_target: {:?}\n", abs_target));
    debug_log(&format!("abs_web_root: {:?}\n", abs_web_root));

    if !abs_target.starts_with(&abs_web_root) {
        debug_log(&format!("Path traversal detected\n"));
        send_error_response(403, "Forbidden", "Access denied. Path traversal attempt detected.");
        return;
    }

    // 检查文件是否存在
    let metadata = match fs::metadata(&abs_target) {
        Ok(m) => m,
        Err(_) => {
            debug_log(&format!("File not found: {:?}\n", abs_target));
            send_error_response(404, "Not Found", "The requested resource was not found on this server.");
            return;
        }
    };

    if metadata.is_dir() {
        debug_log(&format!("Is directory, not file: {:?}\n", abs_target));
        send_error_response(404, "Not Found", "The requested resource was not found on this server.");
        return;
    }

    // 读取并发送文件
    let mut file = match File::open(&abs_target) {
        Ok(f) => f,
        Err(_) => {
            debug_log(&format!("Failed to open file: {:?}\n", abs_target));
            send_error_response(404, "Not Found", "The requested resource was not found on this server or not readable.");
            return;
        }
    };

    let filename = abs_target
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let content_type = get_mime_type(filename);
    let file_size = metadata.len();

    // 发送 HTTP 响应头
    println!("Content-Type: {}", content_type);
    println!("Content-Length: {}", file_size);
    println!();

    // 发送文件内容
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();
    let mut buffer = vec![0u8; 8192];

    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                let _ = stdout_lock.write_all(&buffer[..n]);
            }
            Err(_) => break,
        }
    }

    let _ = stdout_lock.flush();
}
