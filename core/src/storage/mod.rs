//! 存储模块，负责保存配对设备信息、共享密钥和历史记录

use crate::{
    error::{Error, Result},
    types::{DeviceInfo, DeviceType},
};
use log::error;
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::{Arc, Mutex};

/// 存储管理器
pub struct Storage {
    /// 数据库连接
    conn: Arc<Mutex<Connection>>,
}

impl Storage {
    /// 创建新的存储管理器
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = match Connection::open(db_path) {
            Ok(conn) => conn,
            Err(e) => {
                error!("打开数据库失败: {:?}", e);
                return Err(Error::Storage(format!("打开数据库失败: {}", e)));
            }
        };

        // 初始化数据库
        let instance = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        instance.init_db()?;

        Ok(instance)
    }

    /// 初始化数据库表结构
    fn init_db(&self) -> Result<()> {
        let conn = match self.conn.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("获取数据库连接锁失败: {:?}", e);
                return Err(Error::Storage("获取数据库连接锁失败".to_string()));
            }
        };

        // 创建设备表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                device_type INTEGER NOT NULL,
                public_key TEXT NOT NULL,
                last_seen INTEGER NOT NULL
            )",
            [],
        )
        .map_err(Error::Database)?;

        // 创建密钥表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS keys (
                device_id TEXT PRIMARY KEY,
                shared_key BLOB NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY(device_id) REFERENCES devices(id)
            )",
            [],
        )
        .map_err(Error::Database)?;

        // 创建历史记录表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device_id TEXT NOT NULL,
                content_type TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                metadata TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY(device_id) REFERENCES devices(id)
            )",
            [],
        )
        .map_err(Error::Database)?;

        Ok(())
    }

    /// 保存设备信息
    pub fn save_device(&self, device: &DeviceInfo) -> Result<()> {
        let conn = match self.conn.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("获取数据库连接锁失败: {:?}", e);
                return Err(Error::Storage("获取数据库连接锁失败".to_string()));
            }
        };

        let device_type = match device.device_type {
            DeviceType::Desktop => 0,
            DeviceType::Mobile => 1,
            DeviceType::Unknown => 2,
        };

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        conn.execute(
            "INSERT OR REPLACE INTO devices (id, name, device_type, public_key, last_seen)
             VALUES (?, ?, ?, ?, ?)",
            params![
                device.id,
                device.name,
                device_type,
                device.public_key,
                timestamp
            ],
        )
        .map_err(Error::Database)?;

        Ok(())
    }

    /// 获取设备信息
    pub fn get_device(&self, device_id: &str) -> Result<Option<DeviceInfo>> {
        let conn = match self.conn.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("获取数据库连接锁失败: {:?}", e);
                return Err(Error::Storage("获取数据库连接锁失败".to_string()));
            }
        };

        let result = conn
            .query_row(
                "SELECT id, name, device_type, public_key FROM devices WHERE id = ?",
                params![device_id],
                |row| {
                    let id: String = row.get(0)?;
                    let name: String = row.get(1)?;
                    let device_type_int: i64 = row.get(2)?;
                    let public_key: String = row.get(3)?;

                    let device_type = match device_type_int {
                        0 => DeviceType::Desktop,
                        1 => DeviceType::Mobile,
                        _ => DeviceType::Unknown,
                    };

                    Ok(DeviceInfo {
                        id,
                        name,
                        device_type,
                        public_key,
                        online: false, // 从数据库中加载的设备默认为离线状态
                        ip_address: None,
                        system_version: None,
                        app_version: None,
                        capabilities: crate::types::DeviceCapabilities::default(),
                        last_seen: None,
                        pairing_status: crate::types::PairingStatus::default(),
                        description: None,
                        trusted: false,
                    })
                },
            )
            .optional()
            .map_err(Error::Database)?;

        Ok(result)
    }

    /// 获取所有已配对设备
    pub fn get_all_devices(&self) -> Result<Vec<DeviceInfo>> {
        let conn = match self.conn.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("获取数据库连接锁失败: {:?}", e);
                return Err(Error::Storage("获取数据库连接锁失败".to_string()));
            }
        };

        let mut stmt = conn
            .prepare("SELECT id, name, device_type, public_key FROM devices")
            .map_err(Error::Database)?;

        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let device_type_int: i64 = row.get(2)?;
                let public_key: String = row.get(3)?;

                let device_type = match device_type_int {
                    0 => DeviceType::Desktop,
                    1 => DeviceType::Mobile,
                    _ => DeviceType::Unknown,
                };

                Ok(DeviceInfo {
                    id,
                    name,
                    device_type,
                    public_key,
                    online: false, // 从数据库中加载的设备默认为离线状态
                    ip_address: None,
                    system_version: None,
                    app_version: None,
                    capabilities: crate::types::DeviceCapabilities::default(),
                    last_seen: None,
                    pairing_status: crate::types::PairingStatus::default(),
                    description: None,
                    trusted: false,
                })
            })
            .map_err(Error::Database)?;

        let mut devices = Vec::new();
        for device_result in rows {
            match device_result {
                Ok(device) => devices.push(device),
                Err(e) => error!("获取设备信息失败: {:?}", e),
            }
        }

        Ok(devices)
    }

    /// 删除设备
    pub fn delete_device(&self, device_id: &str) -> Result<()> {
        let conn = match self.conn.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("获取数据库连接锁失败: {:?}", e);
                return Err(Error::Storage("获取数据库连接锁失败".to_string()));
            }
        };

        conn.execute("DELETE FROM keys WHERE device_id = ?", params![device_id])
            .map_err(Error::Database)?;

        conn.execute("DELETE FROM devices WHERE id = ?", params![device_id])
            .map_err(Error::Database)?;

        Ok(())
    }

    /// 保存共享密钥
    pub fn save_shared_key(&self, device_id: &str, key_data: &[u8]) -> Result<()> {
        let conn = match self.conn.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("获取数据库连接锁失败: {:?}", e);
                return Err(Error::Storage("获取数据库连接锁失败".to_string()));
            }
        };

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        conn.execute(
            "INSERT OR REPLACE INTO keys (device_id, shared_key, created_at)
             VALUES (?, ?, ?)",
            params![device_id, key_data, timestamp],
        )
        .map_err(Error::Database)?;

        Ok(())
    }

    /// 获取共享密钥
    pub fn get_shared_key(&self, device_id: &str) -> Result<Option<Vec<u8>>> {
        let conn = match self.conn.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("获取数据库连接锁失败: {:?}", e);
                return Err(Error::Storage("获取数据库连接锁失败".to_string()));
            }
        };

        let result = conn
            .query_row(
                "SELECT shared_key FROM keys WHERE device_id = ?",
                params![device_id],
                |row| {
                    let key: Vec<u8> = row.get(0)?;
                    Ok(key)
                },
            )
            .optional()
            .map_err(Error::Database)?;

        Ok(result)
    }

    /// 添加历史记录
    pub fn add_history(
        &self,
        device_id: &str,
        content_type: &str,
        content_hash: &str,
        metadata: &str,
    ) -> Result<()> {
        let conn = match self.conn.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("获取数据库连接锁失败: {:?}", e);
                return Err(Error::Storage("获取数据库连接锁失败".to_string()));
            }
        };

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        conn.execute(
            "INSERT INTO history (device_id, content_type, content_hash, metadata, timestamp)
             VALUES (?, ?, ?, ?, ?)",
            params![device_id, content_type, content_hash, metadata, timestamp],
        )
        .map_err(Error::Database)?;

        Ok(())
    }

    /// 获取历史记录
    pub fn get_history(&self, limit: usize) -> Result<Vec<HistoryEntry>> {
        let conn = match self.conn.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("获取数据库连接锁失败: {:?}", e);
                return Err(Error::Storage("获取数据库连接锁失败".to_string()));
            }
        };

        let mut stmt = conn
            .prepare(
                "SELECT h.id, h.device_id, d.name, h.content_type, h.metadata, h.timestamp 
                 FROM history h
                 JOIN devices d ON h.device_id = d.id
                 ORDER BY h.timestamp DESC
                 LIMIT ?",
            )
            .map_err(Error::Database)?;

        let rows = stmt
            .query_map([limit as i64], |row| {
                let id: i64 = row.get(0)?;
                let device_id: String = row.get(1)?;
                let device_name: String = row.get(2)?;
                let content_type: String = row.get(3)?;
                let metadata: String = row.get(4)?;
                let timestamp: i64 = row.get(5)?;

                Ok(HistoryEntry {
                    id: id as u64,
                    device_id,
                    device_name,
                    content_type,
                    metadata,
                    timestamp: timestamp as u64,
                })
            })
            .map_err(Error::Database)?;

        let mut entries = Vec::new();
        for entry_result in rows {
            match entry_result {
                Ok(entry) => entries.push(entry),
                Err(e) => error!("获取历史记录失败: {e:?}"),
            }
        }

        Ok(entries)
    }

    /// 清除历史记录
    pub fn clear_history(&self) -> Result<()> {
        let conn = match self.conn.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("获取数据库连接锁失败: {:?}", e);
                return Err(Error::Storage("获取数据库连接锁失败".to_string()));
            }
        };

        conn.execute("DELETE FROM history", [])
            .map_err(Error::Database)?;

        Ok(())
    }
}

/// 历史记录条目
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// 条目ID
    pub id: u64,
    /// 设备ID
    pub device_id: String,
    /// 设备名称
    pub device_name: String,
    /// 内容类型
    pub content_type: String,
    /// 元数据（JSON格式）
    pub metadata: String,
    /// 时间戳
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_storage_init() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::new(temp_file.path().to_str().unwrap());
        assert!(storage.is_ok());
    }

    #[test]
    fn test_device_crud() {
        let temp_file = NamedTempFile::new().unwrap();
        let storage = Storage::new(temp_file.path().to_str().unwrap()).unwrap();

        let device = DeviceInfo {
            id: "test_id".to_string(),
            name: "Test Device".to_string(),
            device_type: DeviceType::Desktop,
            public_key: "test_key".to_string(),
            online: true,
            app_version: "1.0.0".to_string(),
            description: "Test Description".to_string(),
            capabilities: vec![],
            os_type: crate::types::OsType::MacOS,
            os_version: "1.0".to_string(),
            last_seen: 0,
            trusted: false,
            paired: false,
        };

        // 保存设备
        assert!(storage.save_device(&device).is_ok());

        // 获取设备
        let result = storage.get_device("test_id").unwrap();
        assert!(result.is_some());
        let retrieved = result.unwrap();
        assert_eq!(retrieved.id, "test_id");
        assert_eq!(retrieved.name, "Test Device");

        // 获取所有设备
        let devices = storage.get_all_devices().unwrap();
        assert_eq!(devices.len(), 1);

        // 删除设备
        assert!(storage.delete_device("test_id").is_ok());
        let result = storage.get_device("test_id").unwrap();
        assert!(result.is_none());
    }
}
