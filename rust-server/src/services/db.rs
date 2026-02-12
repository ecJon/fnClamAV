use rusqlite::{Connection, Result as SqliteResult};
use std::path::Path;

pub fn init_db(db_path: &str) -> SqliteResult<()> {
    let mut conn = Connection::open(db_path)?;

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
            error_message TEXT
        )",
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
            file_hash TEXT
        )",
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
            error_message TEXT
        )",
        [],
    )?;

    // 创建隔离记录表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS quarantine_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid TEXT NOT NULL UNIQUE,
            threat_id INTEGER,
            quarantine_path TEXT NOT NULL,
            original_path TEXT NOT NULL,
            quarantined_time INTEGER NOT NULL,
            file_size INTEGER NOT NULL,
            restored BOOLEAN DEFAULT 0,
            restored_time INTEGER
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
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_threat_records_scan_id ON threat_records(scan_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_update_history_start_time ON update_history(start_time DESC)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_quarantine_quarantined_time ON quarantine_records(quarantined_time DESC)",
        [],
    )?;

    // 数据库迁移：添加 current_file 字段（如果不存在）
    // SQLite 不支持 IF NOT EXISTS for ALTER TABLE，所以需要检查列是否存在
    let has_current_file: SqliteResult<bool> = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('scan_history') WHERE name='current_file'",
        [],
        |row| row.get(0).map(|count: i64| count > 0),
    );

    if has_current_file.unwrap_or(false) {
        // 字段已存在，不需要迁移
    } else {
        // 添加 current_file 字段
        conn.execute(
            "ALTER TABLE scan_history ADD COLUMN current_file TEXT",
            [],
        )?;
    }

    Ok(())
}

#[derive(Clone)]
pub struct Database {
    db_path: String,
}

impl Database {
    pub fn new(db_path: &str) -> Self {
        // 确保数据库已初始化
        let _ = init_db(db_path);
        Self {
            db_path: db_path.to_string(),
        }
    }

    fn get_conn(&self) -> SqliteResult<Connection> {
        Connection::open(&self.db_path)
    }

    // === 扫描历史 ===

    pub fn create_scan(&self, scan_id: &str, scan_type: &str, paths: &[String]) -> SqliteResult<i64> {
        let conn = self.get_conn()?;
        let paths_json = serde_json::to_string(paths).unwrap();
        let start_time = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO scan_history (scan_id, scan_type, paths, status, start_time)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            [scan_id, scan_type, &paths_json, "scanning", &start_time.to_string()],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_scan_progress(&self, scan_id: &str, scanned_files: i32, current_file: Option<&str>) -> SqliteResult<()> {
        let conn = self.get_conn()?;
        if let Some(file) = current_file {
            conn.execute(
                "UPDATE scan_history SET scanned_files = ?1, current_file = ?2 WHERE scan_id = ?3",
                [scanned_files.to_string(), file.to_string(), scan_id.to_string()],
            )?;
        } else {
            conn.execute(
                "UPDATE scan_history SET scanned_files = ?1 WHERE scan_id = ?2",
                [scanned_files.to_string(), scan_id.to_string()],
            )?;
        }
        Ok(())
    }

    pub fn finish_scan(
        &self,
        scan_id: &str,
        status: &str,
        total_files: i32,
        error_message: Option<&str>,
    ) -> SqliteResult<()> {
        let conn = self.get_conn()?;
        let end_time = chrono::Utc::now().timestamp();

        conn.execute(
            "UPDATE scan_history SET status = ?1, end_time = ?2, total_files = ?3, error_message = ?4
             WHERE scan_id = ?5",
            [
                status,
                &end_time.to_string(),
                &total_files.to_string(),
                error_message.unwrap_or(""),
                scan_id,
            ],
        )?;
        Ok(())
    }

    pub fn get_current_scan(&self) -> SqliteResult<Option<ScanRecord>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, scan_id, scan_type, paths, status, start_time, end_time,
                    total_files, scanned_files, threats_found, current_file, error_message
             FROM scan_history WHERE status = 'scanning' LIMIT 1"
        )?;

        let mut rows = stmt.query([])?;

        if let Some(row) = rows.next()? {
            Ok(Some(ScanRecord {
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
                current_file: row.get(10)?,
                error_message: row.get(11)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_scan_history(&self, limit: i32) -> SqliteResult<Vec<ScanRecord>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, scan_id, scan_type, paths, status, start_time, end_time,
                    total_files, scanned_files, threats_found, current_file, error_message
             FROM scan_history ORDER BY start_time DESC LIMIT ?1"
        )?;

        let mut rows = stmt.query([limit])?;
        let mut results = Vec::new();

        while let Some(row) = rows.next()? {
            results.push(ScanRecord {
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
                current_file: row.get(10)?,
                error_message: row.get(11)?,
            });
        }

        Ok(results)
    }

    // === 威胁记录 ===

    pub fn add_threat(
        &self,
        scan_id: &str,
        file_path: &str,
        virus_name: &str,
    ) -> SqliteResult<i64> {
        let conn = self.get_conn()?;
        conn.execute(
            "INSERT INTO threat_records (scan_id, file_path, virus_name)
             VALUES (?1, ?2, ?3)",
            [scan_id, file_path, virus_name],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_threat_action(
        &self,
        threat_id: i64,
        action: &str,
        quarantine_uuid: Option<&str>,
    ) -> SqliteResult<()> {
        let conn = self.get_conn()?;
        let action_time = chrono::Utc::now().timestamp();

        if let Some(uuid) = quarantine_uuid {
            conn.execute(
                "UPDATE threat_records SET action_taken = ?1, action_time = ?2, original_location = uuid
                 WHERE id = ?3",
                [action, &action_time.to_string(), &threat_id.to_string()],
            )?;
        } else {
            conn.execute(
                "UPDATE threat_records SET action_taken = ?1, action_time = ?2 WHERE id = ?3",
                [action, &action_time.to_string(), &threat_id.to_string()],
            )?;
        }
        Ok(())
    }

    pub fn get_threats(&self, scan_id: Option<&str>, limit: i32) -> SqliteResult<Vec<ThreatRecord>> {
        let conn = self.get_conn()?;

        let sql = if scan_id.is_some() {
            "SELECT id, scan_id, file_path, virus_name, action_taken, action_time, original_location, file_hash
             FROM threat_records WHERE scan_id = ?1 ORDER BY id DESC LIMIT ?2"
        } else {
            "SELECT id, scan_id, file_path, virus_name, action_taken, action_time, original_location, file_hash
             FROM threat_records ORDER BY id DESC LIMIT ?1"
        };

        let mut stmt = conn.prepare(sql)?;

        let mut rows = if let Some(sid) = scan_id {
            stmt.query([&sid as &str, &(limit.to_string())])?
        } else {
            stmt.query([limit])?
        };

        let mut results = Vec::new();
        while let Some(row) = rows.next()? {
            results.push(ThreatRecord {
                id: row.get(0)?,
                scan_id: row.get(1)?,
                file_path: row.get(2)?,
                virus_name: row.get(3)?,
                action_taken: row.get(4)?,
                action_time: row.get(5)?,
                original_location: row.get(6)?,
                file_hash: row.get(7)?,
            });
        }

        Ok(results)
    }

    pub fn get_threat_by_id(&self, threat_id: i64) -> SqliteResult<Option<ThreatRecord>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, scan_id, file_path, virus_name, action_taken, action_time, original_location, file_hash
             FROM threat_records WHERE id = ?1"
        )?;

        let mut rows = stmt.query([threat_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(ThreatRecord {
                id: row.get(0)?,
                scan_id: row.get(1)?,
                file_path: row.get(2)?,
                virus_name: row.get(3)?,
                action_taken: row.get(4)?,
                action_time: row.get(5)?,
                original_location: row.get(6)?,
                file_hash: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    // === 更新历史 ===

    pub fn add_update_history(
        &self,
        old_version: Option<&str>,
        new_version: Option<&str>,
        result: &str,
        error_message: Option<&str>,
    ) -> SqliteResult<i64> {
        let conn = self.get_conn()?;
        let start_time = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO update_history (start_time, end_time, result, old_version, new_version, error_message)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            [
                &start_time.to_string(),
                &start_time.to_string(),
                result,
                old_version.unwrap_or(""),
                new_version.unwrap_or(""),
                error_message.unwrap_or(""),
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_update_history(&self, limit: i32) -> SqliteResult<Vec<UpdateRecord>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, start_time, end_time, result, old_version, new_version, error_message
             FROM update_history ORDER BY start_time DESC LIMIT ?1"
        )?;

        let mut rows = stmt.query([limit])?;
        let mut results = Vec::new();

        while let Some(row) = rows.next()? {
            results.push(UpdateRecord {
                id: row.get(0)?,
                start_time: row.get(1)?,
                end_time: row.get(2)?,
                result: row.get(3)?,
                old_version: row.get(4)?,
                new_version: row.get(5)?,
                error_message: row.get(6)?,
            });
        }

        Ok(results)
    }

    // === 隔离记录 ===

    pub fn add_quarantine_record(
        &self,
        uuid: &str,
        quarantine_path: &str,
        original_path: &str,
        file_size: i64,
    ) -> SqliteResult<i64> {
        let conn = self.get_conn()?;
        let quarantined_time = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO quarantine_records (uuid, quarantine_path, original_path, quarantined_time, file_size)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            [
                uuid,
                quarantine_path,
                original_path,
                &quarantined_time.to_string(),
                &file_size.to_string(),
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_quarantine_records(&self, limit: i32) -> SqliteResult<Vec<QuarantineRecord>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, uuid, quarantine_path, original_path, quarantined_time, file_size, restored
             FROM quarantine_records WHERE restored = 0 ORDER BY quarantined_time DESC LIMIT ?1"
        )?;

        let mut rows = stmt.query([limit])?;
        let mut results = Vec::new();

        while let Some(row) = rows.next()? {
            results.push(QuarantineRecord {
                id: row.get(0)?,
                uuid: row.get(1)?,
                quarantine_path: row.get(2)?,
                original_path: row.get(3)?,
                quarantined_time: row.get(4)?,
                file_size: row.get(5)?,
                restored: row.get::<_, i32>(6)? != 0,
                restored_time: None,
            });
        }

        Ok(results)
    }

    pub fn get_quarantine_by_uuid(&self, uuid: &str) -> SqliteResult<Option<QuarantineRecord>> {
        let conn = self.get_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, uuid, quarantine_path, original_path, quarantined_time, file_size, restored, restored_time
             FROM quarantine_records WHERE uuid = ?1"
        )?;

        let mut rows = stmt.query([uuid])?;

        if let Some(row) = rows.next()? {
            Ok(Some(QuarantineRecord {
                id: row.get(0)?,
                uuid: row.get(1)?,
                quarantine_path: row.get(2)?,
                original_path: row.get(3)?,
                quarantined_time: row.get(4)?,
                file_size: row.get(5)?,
                restored: row.get::<_, i32>(6)? != 0,
                restored_time: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn mark_quarantine_restored(&self, uuid: &str) -> SqliteResult<()> {
        let conn = self.get_conn()?;
        let restored_time = chrono::Utc::now().timestamp();

        conn.execute(
            "UPDATE quarantine_records SET restored = 1, restored_time = ?1 WHERE uuid = ?2",
            [&restored_time.to_string(), uuid],
        )?;
        Ok(())
    }

    pub fn delete_quarantine_record(&self, uuid: &str) -> SqliteResult<()> {
        let conn = self.get_conn()?;
        conn.execute("DELETE FROM quarantine_records WHERE uuid = ?1", [uuid])?;
        Ok(())
    }
}

// === 数据记录结构 ===

#[derive(Debug, Clone)]
pub struct ScanRecord {
    pub id: i64,
    pub scan_id: String,
    pub scan_type: String,
    pub paths: String,
    pub status: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub total_files: i32,
    pub scanned_files: i32,
    pub threats_found: i32,
    pub current_file: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ThreatRecord {
    pub id: i64,
    pub scan_id: String,
    pub file_path: String,
    pub virus_name: String,
    pub action_taken: Option<String>,
    pub action_time: Option<i64>,
    pub original_location: Option<String>,
    pub file_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateRecord {
    pub id: i64,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub result: String,
    pub old_version: Option<String>,
    pub new_version: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QuarantineRecord {
    pub id: i64,
    pub uuid: String,
    pub quarantine_path: String,
    pub original_path: String,
    pub quarantined_time: i64,
    pub file_size: i64,
    pub restored: bool,
    pub restored_time: Option<i64>,
}
