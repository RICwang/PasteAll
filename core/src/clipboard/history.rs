//! 剪贴板历史记录模块
//! 
//! 提供剪贴板历史记录的存储和管理功能，支持查询历史记录、导出/导入历史记录等。

use crate::clipboard::ClipboardContent;
use crate::error::{Error, Result};
use crate::storage;
use log::{debug, error, warn};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// 历史记录条目，包含剪贴板内容和时间戳
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// 唯一标识符
    pub id: String,
    /// 剪贴板内容
    pub content: ClipboardContent,
    /// 创建时间（Unix时间戳，毫秒）
    pub timestamp: u64,
    /// 可选的自定义标签
    pub tags: Vec<String>,
    /// 是否标记为收藏
    pub is_favorite: bool,
}

impl HistoryEntry {
    /// 创建新的历史记录条目
    pub fn new(content: ClipboardContent) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
            
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            timestamp: now,
            tags: Vec::new(),
            is_favorite: false,
        }
    }
    
    /// 添加标签
    pub fn add_tag(&mut self, tag: &str) {
        if !self.tags.contains(&tag.to_string()) {
            self.tags.push(tag.to_string());
        }
    }
    
    /// 移除标签
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
    }
    
    /// 切换收藏状态
    pub fn toggle_favorite(&mut self) {
        self.is_favorite = !self.is_favorite;
    }
}

/// 剪贴板历史记录管理器
pub struct ClipboardHistory {
    /// 历史记录列表
    entries: Arc<Mutex<VecDeque<HistoryEntry>>>,
    /// 最大历史记录条数
    max_entries: usize,
    /// 是否启用持久化存储
    persistence_enabled: bool,
}

impl ClipboardHistory {
    /// 创建新的历史记录管理器
    pub fn new(max_entries: usize, persistence_enabled: bool) -> Self {
        let history = Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(max_entries))),
            max_entries,
            persistence_enabled,
        };
        
        // 如果启用持久化存储，从数据库加载历史记录
        if persistence_enabled {
            if let Err(e) = history.load_from_storage() {
                warn!("从存储加载历史记录失败: {e:?}");
            }
        }
        
        history
    }
    
    /// 添加新的历史记录
    pub fn add(&self, content: ClipboardContent) -> Result<()> {
        // 忽略空内容
        if matches!(content, ClipboardContent::Empty) {
            return Ok(());
        }
        
        let entry = HistoryEntry::new(content);
        
        // 获取锁并添加记录
        let mut entries = self.entries.lock().map_err(|e| 
            Error::Other(format!("获取历史记录锁失败: {e:?}"))
        )?;
        
        // 检查是否已存在相同内容（避免重复）
        let is_duplicate = entries.iter().any(|e| e.content == entry.content);
        
        if !is_duplicate {
            // 添加新记录
            entries.push_front(entry.clone());
            
            // 如果超出最大条数，移除最旧的记录
            while entries.len() > self.max_entries {
                entries.pop_back();
            }
            
            // 如果启用持久化，保存到存储
            if self.persistence_enabled {
                drop(entries); // 释放锁后再保存
                if let Err(e) = self.save_entry(&entry) {
                    warn!("保存历史记录条目失败: {e:?}");
                }
            }
        }
        
        Ok(())
    }
    
    /// 获取所有历史记录
    pub fn get_all(&self) -> Result<Vec<HistoryEntry>> {
        let entries = self.entries.lock().map_err(|e| 
            Error::Other(format!("获取历史记录锁失败: {e:?}"))
        )?;
        
        Ok(entries.iter().cloned().collect())
    }
    
    /// 获取指定标签的历史记录
    pub fn get_by_tag(&self, tag: &str) -> Result<Vec<HistoryEntry>> {
        let entries = self.entries.lock().map_err(|e| 
            Error::Other(format!("获取历史记录锁失败: {e:?}"))
        )?;
        
        Ok(entries
            .iter()
            .filter(|entry| entry.tags.contains(&tag.to_string()))
            .cloned()
            .collect())
    }
    
    /// 获取收藏的历史记录
    pub fn get_favorites(&self) -> Result<Vec<HistoryEntry>> {
        let entries = self.entries.lock().map_err(|e| 
            Error::Other(format!("获取历史记录锁失败: {e:?}"))
        )?;
        
        Ok(entries
            .iter()
            .filter(|entry| entry.is_favorite)
            .cloned()
            .collect())
    }
    
    /// 按ID查找历史记录条目
    pub fn find_by_id(&self, id: &str) -> Result<Option<HistoryEntry>> {
        let entries = self.entries.lock().map_err(|e| 
            Error::Other(format!("获取历史记录锁失败: {e:?}"))
        )?;
        
        Ok(entries
            .iter()
            .find(|entry| entry.id == id)
            .cloned())
    }
    
    /// 删除指定ID的历史记录
    pub fn remove(&self, id: &str) -> Result<bool> {
        let mut entries = self.entries.lock().map_err(|e| 
            Error::Other(format!("获取历史记录锁失败: {e:?}"))
        )?;
        
        let initial_len = entries.len();
        entries.retain(|entry| entry.id != id);
        
        let removed = entries.len() < initial_len;
        
        // 如果启用持久化且成功移除了记录，则从存储中也删除
        if removed && self.persistence_enabled {
            drop(entries); // 释放锁后再操作数据库
            if let Err(e) = self.remove_from_storage(id) {
                warn!("从存储中删除历史记录失败: {e:?}");
            }
        }
        
        Ok(removed)
    }
    
    /// 清空历史记录
    pub fn clear(&self) -> Result<()> {
        let mut entries = self.entries.lock().map_err(|e| 
            Error::Other(format!("获取历史记录锁失败: {e:?}"))
        )?;
        
        entries.clear();
        
        // 如果启用持久化，清空存储中的记录
        if self.persistence_enabled {
            drop(entries); // 释放锁后再操作数据库
            if let Err(e) = self.clear_storage() {
                warn!("清空历史记录存储失败: {e:?}");
            }
        }
        
        Ok(())
    }
    
    /// 切换条目的收藏状态
    pub fn toggle_favorite(&self, id: &str) -> Result<bool> {
        let mut entries = self.entries.lock().map_err(|e| 
            Error::Other(format!("获取历史记录锁失败: {e:?}"))
        )?;
        
        if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
            entry.toggle_favorite();
            
            // 如果启用持久化，更新存储
            if self.persistence_enabled {
                let entry_clone = entry.clone();
                drop(entries); // 释放锁后再操作数据库
                if let Err(e) = self.update_entry(&entry_clone) {
                    warn!("更新历史记录条目失败: {e:?}");
                }
            }
            
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// 为条目添加标签
    pub fn add_tag(&self, id: &str, tag: &str) -> Result<bool> {
        let mut entries = self.entries.lock().map_err(|e| 
            Error::Other(format!("获取历史记录锁失败: {e:?}"))
        )?;
        
        if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
            entry.add_tag(tag);
            
            // 如果启用持久化，更新存储
            if self.persistence_enabled {
                let entry_clone = entry.clone();
                drop(entries); // 释放锁后再操作数据库
                if let Err(e) = self.update_entry(&entry_clone) {
                    warn!("更新历史记录条目失败: {e:?}");
                }
            }
            
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// 从条目中移除标签
    pub fn remove_tag(&self, id: &str, tag: &str) -> Result<bool> {
        let mut entries = self.entries.lock().map_err(|e| 
            Error::Other(format!("获取历史记录锁失败: {e:?}"))
        )?;
        
        if let Some(entry) = entries.iter_mut().find(|e| e.id == id) {
            entry.remove_tag(tag);
            
            // 如果启用持久化，更新存储
            if self.persistence_enabled {
                let entry_clone = entry.clone();
                drop(entries); // 释放锁后再操作数据库
                if let Err(e) = self.update_entry(&entry_clone) {
                    warn!("更新历史记录条目失败: {e:?}");
                }
            }
            
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    // 以下是内部持久化存储相关方法
    
    /// 从存储中加载历史记录
    fn load_from_storage(&self) -> Result<()> {
        debug!("从存储中加载历史记录");
        let db = storage::get_connection()?;
        
        // 确保表存在
        db.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_history (
                id TEXT PRIMARY KEY,
                content BLOB NOT NULL,
                content_type INTEGER NOT NULL,
                timestamp INTEGER NOT NULL,
                tags TEXT,
                is_favorite INTEGER NOT NULL
            )",
            [],
        ).map_err(Error::Database)?;
        
        // 查询所有记录，按时间戳降序排列
        let mut stmt = db.prepare(
            "SELECT id, content, content_type, timestamp, tags, is_favorite 
             FROM clipboard_history 
             ORDER BY timestamp DESC
             LIMIT ?",
        ).map_err(Error::Database)?;
        
        let rows = stmt.query_map([self.max_entries], |row| {
            let id: String = row.get(0)?;
            let content_blob: Vec<u8> = row.get(1)?;
            let content_type: i32 = row.get(2)?;
            let timestamp: i64 = row.get(3)?;
            let tags_str: Option<String> = row.get(4)?;
            let is_favorite: bool = row.get(5)?;
            
            // 解析内容
            let content = match content_type {
                0 => {
                    // 文本
                    let text = String::from_utf8(content_blob)
                        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                            0, 
                            rusqlite::types::Type::Blob, 
                            Box::new(e)
                        ))?;
                    ClipboardContent::Text(text)
                },
                1 => {
                    // 图片
                    ClipboardContent::Image(content_blob)
                },
                2 => {
                    // 文件路径
                    let text = String::from_utf8(content_blob)
                        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                            0, 
                            rusqlite::types::Type::Blob, 
                            Box::new(e)
                        ))?;
                    let paths: Vec<String> = serde_json::from_str(&text)
                        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                            0, 
                            rusqlite::types::Type::Blob, 
                            Box::new(e)
                        ))?;
                    ClipboardContent::Files(paths)
                },
                _ => ClipboardContent::Empty
            };
            
            // 解析标签
            let tags = if let Some(tags_str) = tags_str {
                serde_json::from_str(&tags_str).unwrap_or_default()
            } else {
                Vec::new()
            };
            
            Ok(HistoryEntry {
                id,
                content,
                timestamp: timestamp as u64,
                tags,
                is_favorite,
            })
        }).map_err(Error::Database)?;
        
        // 添加到内存中的历史记录
        let mut entries = self.entries.lock().map_err(|e| 
            Error::Other(format!("获取历史记录锁失败: {e:?}"))
        )?;
        
        entries.clear();
        for row_result in rows {
            if let Ok(entry) = row_result {
                entries.push_back(entry);
            }
        }
        
        Ok(())
    }
    
    /// 保存单个历史记录条目到存储
    fn save_entry(&self, entry: &HistoryEntry) -> Result<()> {
        let db = storage::get_connection()?;
        
        // 确保表存在
        db.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_history (
                id TEXT PRIMARY KEY,
                content BLOB NOT NULL,
                content_type INTEGER NOT NULL,
                timestamp INTEGER NOT NULL,
                tags TEXT,
                is_favorite INTEGER NOT NULL
            )",
            [],
        ).map_err(Error::Database)?;
        
        // 准备内容和类型
        let (content_blob, content_type) = match &entry.content {
            ClipboardContent::Text(text) => {
                (text.as_bytes().to_vec(), 0)
            },
            ClipboardContent::Image(data) => {
                (data.clone(), 1)
            },
            ClipboardContent::Files(paths) => {
                let json = serde_json::to_string(paths).map_err(|e| 
                    Error::Other(format!("序列化文件路径失败: {e:?}"))
                )?;
                (json.as_bytes().to_vec(), 2)
            },
            ClipboardContent::Empty => {
                (Vec::new(), 3)
            },
        };
        
        // 序列化标签
        let tags_json = serde_json::to_string(&entry.tags).map_err(|e| 
            Error::Other(format!("序列化标签失败: {e:?}"))
        )?;
        
        // 插入或更新记录
        db.execute(
            "INSERT OR REPLACE INTO clipboard_history 
             (id, content, content_type, timestamp, tags, is_favorite) 
             VALUES (?, ?, ?, ?, ?, ?)",
            rusqlite::params![
                &entry.id,
                content_blob,
                content_type,
                entry.timestamp as i64,
                tags_json,
                entry.is_favorite,
            ],
        ).map_err(Error::Database)?;
        
        Ok(())
    }
    
    /// 更新历史记录条目
    fn update_entry(&self, entry: &HistoryEntry) -> Result<()> {
        // 重用保存方法
        self.save_entry(entry)
    }
    
    /// 从存储中删除历史记录
    fn remove_from_storage(&self, id: &str) -> Result<()> {
        let db = storage::get_connection()?;
        
        db.execute(
            "DELETE FROM clipboard_history WHERE id = ?",
            [id],
        ).map_err(Error::Database)?;
        
        Ok(())
    }
    
    /// 清空历史记录存储
    fn clear_storage(&self) -> Result<()> {
        let db = storage::get_connection()?;
        
        db.execute(
            "DELETE FROM clipboard_history",
            [],
        ).map_err(Error::Database)?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_history_entry() {
        let content = ClipboardContent::Text("测试文本".to_string());
        let mut entry = HistoryEntry::new(content);
        
        // 测试添加标签
        entry.add_tag("工作");
        entry.add_tag("重要");
        assert_eq!(entry.tags.len(), 2);
        assert!(entry.tags.contains(&"工作".to_string()));
        
        // 测试重复添加标签
        entry.add_tag("工作");
        assert_eq!(entry.tags.len(), 2);
        
        // 测试移除标签
        entry.remove_tag("工作");
        assert_eq!(entry.tags.len(), 1);
        assert!(!entry.tags.contains(&"工作".to_string()));
        
        // 测试切换收藏
        assert!(!entry.is_favorite);
        entry.toggle_favorite();
        assert!(entry.is_favorite);
        entry.toggle_favorite();
        assert!(!entry.is_favorite);
    }
    
    #[test]
    fn test_clipboard_history_in_memory() {
        // 创建不启用持久化的历史记录管理器
        let history = ClipboardHistory::new(3, false);
        
        // 添加记录
        history.add(ClipboardContent::Text("第一条".to_string())).unwrap();
        history.add(ClipboardContent::Text("第二条".to_string())).unwrap();
        history.add(ClipboardContent::Text("第三条".to_string())).unwrap();
        
        // 测试获取所有记录
        let entries = history.get_all().unwrap();
        assert_eq!(entries.len(), 3);
        
        // 测试限制条数
        history.add(ClipboardContent::Text("第四条".to_string())).unwrap();
        let entries = history.get_all().unwrap();
        assert_eq!(entries.len(), 3);
        
        // 第一条应该被移除了
        let texts: Vec<String> = entries.iter()
            .filter_map(|e| if let ClipboardContent::Text(text) = &e.content {
                Some(text.clone())
            } else {
                None
            })
            .collect();
        assert!(!texts.contains(&"第一条".to_string()));
        assert!(texts.contains(&"第二条".to_string()));
        assert!(texts.contains(&"第三条".to_string()));
        assert!(texts.contains(&"第四条".to_string()));
        
        // 测试标记收藏
        let id = entries[0].id.clone();
        history.toggle_favorite(&id).unwrap();
        let favorites = history.get_favorites().unwrap();
        assert_eq!(favorites.len(), 1);
        
        // 测试添加标签
        history.add_tag(&id, "重要").unwrap();
        let tagged = history.get_by_tag("重要").unwrap();
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0].id, id);
        
        // 测试移除记录
        history.remove(&id).unwrap();
        let entries = history.get_all().unwrap();
        assert_eq!(entries.len(), 2);
        
        // 测试清空
        history.clear().unwrap();
        let entries = history.get_all().unwrap();
        assert_eq!(entries.len(), 0);
    }
}
