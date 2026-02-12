use rusqlite::{Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

// 数据库路径
pub fn get_db_path() -> PathBuf {
    // 优先使用环境变量指定的数据共享目录
    if let Ok(data_share) = std::env::var("TRIM_DATA_SHARE_PATHS") {
        let first_path = data_share.split(':').next().unwrap_or("");
        if !first_path.is_empty() {
            return PathBuf::from(first_path).join("clamav.db");
        }
    }
    // 开发环境回退
    PathBuf::from("/tmp/clamav.db")
}

// 初始化数据库
pub fn init_db() -> SqliteResult<()> {
    let db_path = get_db_path();

    // 确保目录存在
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).ok();
    }

    let conn = Connection::open(&db_path)?;

    // 创建扫描历史表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS scan_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scan_id TEXT UNIQUE NOT NULL,
            scan_type TEXT NOT NULL,
            paths TEXT NOT NULL,
            status TEXT NOT NULL,
            start_time INTEGER NOT NULL,
            end_time INTEGER,
            total_files INTEGER DEFAULT 0,
            scanned_files INTEGER DEFAULT 0,
            threats_found INTEGER DEFAULT 0,
            error_message TEXT,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;

    // 创建索引
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_scan_history_start_time ON scan_history(start_time DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_scan_history_status ON scan_history(status)",
        [],
    )?;

    // 创建威胁记录表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS threat_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scan_id TEXT NOT NULL,
            file_path TEXT NOT NULL,
            virus_name TEXT NOT NULL,
            action_taken TEXT,
            action_time INTEGER,
            original_location TEXT,
            file_hash TEXT,
            discovered_at INTEGER NOT NULL,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_threat_records_scan_id ON threat_records(scan_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_threat_records_action_time ON threat_records(action_time DESC)",
        [],
    )?;

    // 创建更新历史表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS update_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            start_time INTEGER NOT NULL,
            end_time INTEGER,
            result TEXT NOT NULL,
            old_version TEXT,
            new_version TEXT,
            error_message TEXT,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_update_history_start_time ON update_history(start_time DESC)",
        [],
    )?;

    // 创建隔离记录表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS quarantine_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            threat_id INTEGER NOT NULL,
            quarantine_path TEXT NOT NULL,
            original_path TEXT NOT NULL,
            quarantined_time INTEGER NOT NULL,
            file_size INTEGER NOT NULL,
            restored INTEGER DEFAULT 0,
            restored_time INTEGER,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (threat_id) REFERENCES threat_records(id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_quarantine_quarantined_time ON quarantine_records(quarantined_time DESC)",
        [],
    )?;

    // 创建配置表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;

    // 插入默认配置
    let default_settings: &[(&str, &str)] = &[
        ("scan.max_file_size_mb", "100"),
        ("scan.scan_archives", "true"),
        ("threat.action", "quarantine"),
        ("threat.auto_action", "false"),
        ("update.frequency", "daily"),
        ("update.schedule_time", "03:30"),
        ("history.retention_days", "90"),
    ];

    let now = chrono::Utc::now().timestamp();
    let now_str = now.to_string();
    for (key, value) in default_settings {
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
            [*key, *value, now_str.as_str()],
        )?;
    }

    Ok(())
}

// 扫描历史记录
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanHistory {
    pub id: i64,
    pub scan_id: String,
    pub scan_type: String,
    pub paths: String,
    pub status: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub total_files: i64,
    pub scanned_files: i64,
    pub threats_found: i64,
    pub error_message: Option<String>,
}

// 威胁记录
#[derive(Debug, Serialize, Deserialize)]
pub struct ThreatRecord {
    pub id: i64,
    pub scan_id: String,
    pub file_path: String,
    pub virus_name: String,
    pub action_taken: Option<String>,
    pub action_time: Option<i64>,
    pub original_location: Option<String>,
    pub file_hash: Option<String>,
    pub discovered_at: i64,
}

// 更新历史记录
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateHistory {
    pub id: i64,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub result: String,
    pub old_version: Option<String>,
    pub new_version: Option<String>,
    pub error_message: Option<String>,
}

// 隔离记录
#[derive(Debug, Serialize, Deserialize)]
pub struct QuarantineRecord {
    pub id: i64,
    pub threat_id: i64,
    pub quarantine_path: String,
    pub original_path: String,
    pub quarantined_time: i64,
    pub file_size: i64,
    pub restored: bool,
    pub restored_time: Option<i64>,
}

// 扫描状态
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanStatus {
    pub scan_id: Option<String>,
    pub status: String,  // idle, scanning, completed, stopped, error
    pub progress: Option<ScanProgress>,
    pub threats_found: i64,
    pub start_time: Option<i64>,
    pub current_scan: Option<ScanHistory>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanProgress {
    pub percent: f32,
    pub scanned: i64,
    pub estimated_total: i64,
    pub current_file: String,
}

// 数据库操作
pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> SqliteResult<Self> {
        let db_path = get_db_path();

        // 确保目录存在
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path)?;
        Ok(Database { conn })
    }

    // 创建新的扫描记录
    pub fn create_scan(
        &self,
        scan_id: &str,
        scan_type: &str,
        paths: &[String],
    ) -> SqliteResult<i64> {
        let paths_json = serde_json::to_string(paths).unwrap_or_default();
        let now = chrono::Utc::now().timestamp();

        self.conn.execute(
            "INSERT INTO scan_history (scan_id, scan_type, paths, status, start_time, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            [scan_id, scan_type, &paths_json, "scanning", &now.to_string(), &now.to_string()],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    // 更新扫描进度
    pub fn update_scan_progress(
        &self,
        scan_id: &str,
        scanned_files: i64,
        threats_found: i64,
    ) -> SqliteResult<()> {
        self.conn.execute(
            "UPDATE scan_history SET scanned_files = ?1, threats_found = ?2 WHERE scan_id = ?3",
            (scanned_files, threats_found, scan_id),
        )?;
        Ok(())
    }

    // 完成扫描
    pub fn finish_scan(
        &self,
        scan_id: &str,
        status: &str,
        total_files: i64,
        error_message: Option<&str>,
    ) -> SqliteResult<()> {
        let now = chrono::Utc::now().timestamp();

        self.conn.execute(
            "UPDATE scan_history SET status = ?1, end_time = ?2, total_files = ?3, error_message = ?4
             WHERE scan_id = ?5",
            [
                status,
                &now.to_string(),
                &total_files.to_string(),
                &error_message.unwrap_or(""),
                scan_id,
            ],
        )?;
        Ok(())
    }

    // 获取当前扫描状态
    pub fn get_current_scan(&self) -> SqliteResult<Option<ScanHistory>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM scan_history WHERE status = 'scanning' ORDER BY start_time DESC LIMIT 1"
        )?;

        let result = stmt.query_row([], |row| {
            Ok(ScanHistory {
                id: row.get(0)?,
                scan_id: row.get(1)?,
                scan_type: row.get(2)?,
                paths: row.get(3)?,
                status: row.get(4)?,
                start_time: row.get(5)?,
                end_time: row.get(6)?,
                total_files: row.get(7)?,
                scanned_files: row.get(8)?,
                threats_found: row.get(9)?,
                error_message: row.get(10)?,
            })
        });

        match result {
            Ok(scan) => Ok(Some(scan)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // 添加威胁记录
    pub fn add_threat(
        &self,
        scan_id: &str,
        file_path: &str,
        virus_name: &str,
    ) -> SqliteResult<i64> {
        let now = chrono::Utc::now().timestamp();

        self.conn.execute(
            "INSERT INTO threat_records (scan_id, file_path, virus_name, discovered_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            [scan_id, file_path, virus_name, &now.to_string(), &now.to_string()],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    // 获取扫描历史列表
    pub fn get_scan_history(&self, limit: usize) -> SqliteResult<Vec<ScanHistory>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM scan_history ORDER BY start_time DESC LIMIT ?1"
        )?;

        let scans = stmt.query_map([limit], |row| {
            Ok(ScanHistory {
                id: row.get(0)?,
                scan_id: row.get(1)?,
                scan_type: row.get(2)?,
                paths: row.get(3)?,
                status: row.get(4)?,
                start_time: row.get(5)?,
                end_time: row.get(6)?,
                total_files: row.get(7)?,
                scanned_files: row.get(8)?,
                threats_found: row.get(9)?,
                error_message: row.get(10)?,
            })
        })?;

        let mut result = Vec::new();
        for scan in scans {
            result.push(scan?);
        }
        Ok(result)
    }
}
