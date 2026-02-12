use crate::env::FnosEnv;
use crate::models::quarantine::{QuarantineMetadata, QuarantineItem};
use std::fs;
use std::path::Path;

/// 隔离区服务
pub struct QuarantineService {
    env: FnosEnv,
}

impl QuarantineService {
    pub fn new(env: FnosEnv) -> Self {
        Self { env }
    }

    /// 隔离文件
    pub fn quarantine_file(
        &self,
        original_path: &str,
        virus_name: &str,
        scan_id: &str,
        file_size: u64,
    ) -> Result<String, String> {
        let metadata = QuarantineMetadata::new(
            original_path.to_string(),
            virus_name.to_string(),
            scan_id.to_string(),
            file_size,
        );

        let files_dir = format!("{}/files", self.env.quarantine_dir());
        let metadata_dir = format!("{}/metadata", self.env.quarantine_dir());

        // 确保目录存在
        fs::create_dir_all(&files_dir)
            .map_err(|e| format!("Failed to create files dir: {}", e))?;
        fs::create_dir_all(&metadata_dir)
            .map_err(|e| format!("Failed to create metadata dir: {}", e))?;

        // 移动文件到隔离区（使用 UUID 作为文件名）
        let quarantine_path = format!("{}/files/{}", self.env.quarantine_dir(), metadata.uuid);

        fs::rename(&original_path, &quarantine_path)
            .map_err(|e| format!("Failed to move file: {}", e))?;

        // 保存元数据
        let metadata_path = format!("{}/{}.json", metadata_dir, metadata.uuid);
        let metadata_json = serde_json::to_string_pretty(&metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;

        fs::write(&metadata_path, metadata_json)
            .map_err(|e| format!("Failed to write metadata: {}", e))?;

        Ok(metadata.uuid)
    }

    /// 恢复隔离文件
    pub fn restore_file(&self, uuid: &str) -> Result<String, String> {
        let files_dir = format!("{}/files", self.env.quarantine_dir());
        let metadata_dir = format!("{}/metadata", self.env.quarantine_dir());

        // 读取元数据
        let metadata_path = format!("{}/{}.json", metadata_dir, uuid);
        let metadata_json = fs::read_to_string(&metadata_path)
            .map_err(|e| format!("Failed to read metadata: {}", e))?;

        let metadata: QuarantineMetadata = serde_json::from_str(&metadata_json)
            .map_err(|e| format!("Failed to parse metadata: {}", e))?;

        // 检查原位置是否可用
        let original_dir = Path::new(&metadata.original_path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("/tmp")
            .to_string();

        if !Path::new(&original_dir).exists() {
            return Err(format!("Original directory not available: {}", original_dir));
        }

        // 恢复文件
        let quarantine_path = format!("{}/files/{}", self.env.quarantine_dir(), uuid);

        fs::rename(&quarantine_path, &metadata.original_path)
            .map_err(|e| format!("Failed to restore file: {}", e))?;

        // 删除元数据
        fs::remove_file(&metadata_path)
            .map_err(|e| format!("Failed to remove metadata: {}", e))?;

        Ok(metadata.original_path)
    }

    /// 删除隔离文件
    pub fn delete_file(&self, uuid: &str) -> Result<(), String> {
        let files_dir = format!("{}/files", self.env.quarantine_dir());
        let metadata_dir = format!("{}/metadata", self.env.quarantine_dir());

        // 删除文件
        let quarantine_path = format!("{}/files/{}", self.env.quarantine_dir(), uuid);
        fs::remove_file(&quarantine_path)
            .map_err(|e| format!("Failed to delete file: {}", e))?;

        // 删除元数据
        let metadata_path = format!("{}/{}.json", metadata_dir, uuid);
        fs::remove_file(&metadata_path)
            .map_err(|e| format!("Failed to delete metadata: {}", e))?;

        Ok(())
    }

    /// 列出隔离文件
    pub fn list_files(&self) -> Result<Vec<QuarantineItem>, String> {
        let metadata_dir = format!("{}/metadata", self.env.quarantine_dir());

        let mut items = Vec::new();

        let entries = fs::read_dir(&metadata_dir)
            .map_err(|e| format!("Failed to read metadata dir: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            let metadata_json = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read metadata: {}", e))?;

            let metadata: QuarantineMetadata = serde_json::from_str(&metadata_json)
                .map_err(|e| format!("Failed to parse metadata: {}", e))?;

            items.push(QuarantineItem {
                uuid: metadata.uuid,
                original_path: metadata.original_path.clone(),
                original_name: metadata.original_name,
                file_size: metadata.file_size,
                virus_name: metadata.virus_name,
                quarantined_at: metadata.quarantined_at,
                scan_id: metadata.scan_id,
            });
        }

        Ok(items)
    }

    /// 清理过期隔离文件
    pub fn cleanup_old(&self, days: u32) -> Result<(u32, u64), String> {
        let metadata_dir = format!("{}/metadata", self.env.quarantine_dir());
        let cutoff_time = chrono::Utc::now().timestamp() - (days as i64 * 86400);

        let mut cleaned_count = 0u32;
        let mut freed_bytes = 0u64;

        let entries = fs::read_dir(&metadata_dir)
            .map_err(|e| format!("Failed to read metadata dir: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            let metadata_json = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read metadata: {}", e))?;

            let metadata: QuarantineMetadata = serde_json::from_str(&metadata_json)
                .map_err(|e| format!("Failed to parse metadata: {}", e))?;

            if metadata.quarantined_at < cutoff_time {
                // 删除文件
                let uuid = &metadata.uuid;
                let quarantine_path = format!("{}/files/{}", self.env.quarantine_dir(), uuid);

                let file_size = fs::metadata(&quarantine_path)
                    .map(|m| m.len())
                    .unwrap_or(0);

                fs::remove_file(&quarantine_path)
                    .map_err(|e| format!("Failed to delete file: {}", e))?;

                fs::remove_file(&path)
                    .map_err(|e| format!("Failed to delete metadata: {}", e))?;

                cleaned_count += 1;
                freed_bytes += file_size;
            }
        }

        Ok((cleaned_count, freed_bytes))
    }
}
