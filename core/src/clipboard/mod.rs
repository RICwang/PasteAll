//! 剪贴板操作模块，提供跨平台的剪贴板监听和操作功能

use crate::error::{Error, Result};
use arboard;
use log::{error, info, warn};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::{self, Duration};

// 导入文件路径相关功能
mod file_paths;
pub use file_paths::{get_clipboard_file_paths, set_clipboard_file_paths};

// 导入测试环境使用的模拟剪贴板
#[cfg(feature = "ci")]
mod mock_clipboard;
#[cfg(feature = "ci")]
pub use mock_clipboard::{set_mock_clipboard, get_mock_clipboard, clear_mock_clipboard};

// 导入历史记录功能
mod history;
pub use history::{ClipboardHistory, HistoryEntry};

/// 剪贴板内容类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardContent {
    /// 文本内容
    Text(String),
    /// 图片内容
    Image(Vec<u8>),
    /// 文件路径列表
    Files(Vec<String>),
    /// 空内容
    Empty,
}

/// 剪贴板事件
#[derive(Debug, Clone)]
pub struct ClipboardEvent {
    /// 内容类型
    pub content: ClipboardContent,
    /// 时间戳（毫秒）
    pub timestamp: u64,
}

/// 剪贴板监听器回调函数类型
pub type ClipboardCallback = Box<dyn Fn(ClipboardEvent) + Send + Sync + 'static>;

/// 剪贴板监听器
pub struct ClipboardWatcher {
    /// 停止信号发送端
    stop_tx: Option<mpsc::Sender<()>>,
    /// 上次检测到的剪贴板内容
    last_content: Arc<Mutex<Option<ClipboardContent>>>,
    /// 剪贴板实例
    clipboard: Arc<Mutex<arboard::Clipboard>>,
    /// 剪贴板历史记录
    history: Option<Arc<ClipboardHistory>>,
}

impl ClipboardWatcher {
    /// 创建新的剪贴板监听器
    pub fn new() -> Result<Self> {
        let clipboard = match arboard::Clipboard::new() {
            Ok(cb) => Arc::new(Mutex::new(cb)),
            Err(e) => {
                error!("创建剪贴板实例失败: {e:?}");
                return Err(Error::Clipboard("创建剪贴板实例失败".to_string()));
            }
        };

        Ok(Self {
            stop_tx: None,
            last_content: Arc::new(Mutex::new(None)),
            clipboard,
            history: None,
        })
    }
    
    /// 创建带有历史记录功能的剪贴板监听器
    pub fn with_history(max_history: usize, persistence_enabled: bool) -> Result<Self> {
        let mut watcher = Self::new()?;
        let history = Arc::new(ClipboardHistory::new(max_history, persistence_enabled));
        watcher.history = Some(history);
        Ok(watcher)
    }

    /// 开始监听剪贴板变化
    pub async fn start(&mut self, callback: ClipboardCallback) -> Result<()> {
        if self.stop_tx.is_some() {
            warn!("剪贴板监听器已经在运行中");
            return Ok(());
        }

        info!("开始监听剪贴板变化");

        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
        self.stop_tx = Some(stop_tx);

        let clipboard = self.clipboard.clone();
        let last_content = self.last_content.clone();
        let history = self.history.clone();

        // 启动监听任务
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(500));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = Self::check_arboard_clipboard_change(clipboard.clone(), last_content.clone(), &callback, history.clone()).await {
                            error!("检查剪贴板变化出错: {e:?}");
                        }
                    }
                    _ = stop_rx.recv() => {
                        info!("停止剪贴板监听");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// 停止监听剪贴板变化
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(stop_tx) = self.stop_tx.take() {
            if let Err(e) = stop_tx.send(()).await {
                error!("发送停止信号失败: {e:?}");
                return Err(Error::Clipboard("停止剪贴板监听器失败".to_string()));
            }
        }

        Ok(())
    }

    /// 获取当前剪贴板内容
    pub fn get_content(&self) -> Result<ClipboardContent> {
        let mut clipboard = match self.clipboard.lock() {
            Ok(cb) => cb,
            Err(e) => {
                error!("获取剪贴板锁失败: {e:?}");
                return Err(Error::Clipboard("获取剪贴板锁失败".to_string()));
            }
        };

        // 1. 首先尝试获取文件路径列表（平台特定实现）
        if let Some(file_paths) = get_clipboard_file_paths() {
            return Ok(ClipboardContent::Files(file_paths));
        }

        // 2. 尝试读取文本
        match clipboard.get_text() {
            Ok(text) => Ok(ClipboardContent::Text(text)),
            Err(_) => {
                // 3. 文本获取失败，尝试读取图片
                match clipboard.get_image() {
                    Ok(image) => {
                        let mut buffer = Vec::new();
                        // 转换图片数据为RGBA字节数组
                        for y in 0..image.height {
                            for x in 0..image.width {
                                let i = y * image.width + x;
                                buffer.extend_from_slice(&image.bytes[i * 4..(i + 1) * 4]);
                            }
                        }
                        Ok(ClipboardContent::Image(buffer))
                    }
                    Err(_) => {
                        // 4. 没有找到任何内容
                        Ok(ClipboardContent::Empty)
                    }
                }
            }
        }
    }

    /// 设置剪贴板内容
    pub fn set_content(&mut self, content: &ClipboardContent) -> Result<()> {
        let mut clipboard = match self.clipboard.lock() {
            Ok(cb) => cb,
            Err(e) => {
                error!("获取剪贴板锁失败: {e:?}");
                return Err(Error::Clipboard("获取剪贴板锁失败".to_string()));
            }
        };

        match content {
            ClipboardContent::Text(text) => {
                if let Err(e) = clipboard.set_text(text) {
                    error!("设置剪贴板文本失败: {e:?}");
                    return Err(Error::Clipboard("设置剪贴板文本失败".to_string()));
                }
            }
            ClipboardContent::Image(_data) => {
                // 设置图片需要转换格式，这里简化处理
                warn!("设置图片内容暂未实现");
                return Err(Error::Clipboard("设置图片内容暂未实现".to_string()));
            }
            ClipboardContent::Files(paths) => {
                if let Err(e) = set_clipboard_file_paths(paths) {
                    error!("设置剪贴板文件路径失败: {e:?}");
                    return Err(Error::Clipboard("设置剪贴板文件路径失败".to_string()));
                }
            }
            ClipboardContent::Empty => {
                warn!("清空剪贴板内容暂未实现");
                return Err(Error::Clipboard("清空剪贴板内容暂未实现".to_string()));
            }
        }

        // 更新最后内容
        if let Ok(mut last) = self.last_content.lock() {
            *last = Some(content.clone());
        }

        Ok(())
    }
    
    /// 获取剪贴板历史记录
    pub fn get_history(&self) -> Option<Arc<ClipboardHistory>> {
        self.history.clone()
    }
    
    /// 获取最近的历史记录条目
    pub fn get_recent_history(&self, count: usize) -> Result<Vec<HistoryEntry>> {
        if let Some(history) = &self.history {
            let mut entries = history.get_all()?;
            entries.truncate(count);
            Ok(entries)
        } else {
            Ok(Vec::new())
        }
    }

    /// 检查剪贴板变化（使用arboard）
    async fn check_arboard_clipboard_change(
        clipboard: Arc<Mutex<arboard::Clipboard>>,
        last_content: Arc<Mutex<Option<ClipboardContent>>>,
        callback: &ClipboardCallback,
        history: Option<Arc<ClipboardHistory>>,
    ) -> Result<()> {
        let current_content = {
            let mut clipboard_guard = match clipboard.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    error!("获取剪贴板锁失败: {e:?}");
                    return Err(Error::Clipboard("获取剪贴板锁失败".to_string()));
                }
            };

            // 1. 首先尝试获取文件路径列表
            if let Some(file_paths) = get_clipboard_file_paths() {
                ClipboardContent::Files(file_paths)
            } else {
                // 2. 尝试获取文本
                match clipboard_guard.get_text() {
                    Ok(text) if !text.is_empty() => ClipboardContent::Text(text),
                    _ => {
                        // 3. 尝试获取图片
                        match clipboard_guard.get_image() {
                            Ok(image) => ClipboardContent::Image(image.bytes.to_vec()),
                            Err(_) => {
                                // 4. 没有找到任何内容
                                ClipboardContent::Empty
                            }
                        }
                    }
                }
            }
        };
        
        // 检查是否与上次不同
        let different = {
            let mut last = match last_content.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    error!("获取上次内容锁失败: {e:?}");
                    return Err(Error::Clipboard("获取上次内容锁失败".to_string()));
                }
            };
            
            let is_different = match (&*last, &current_content) {
                (Some(ClipboardContent::Text(last_text)), ClipboardContent::Text(current_text)) => {
                    last_text != current_text
                },
                (Some(ClipboardContent::Files(last_files)), ClipboardContent::Files(current_files)) => {
                    last_files != current_files
                },
                (Some(ClipboardContent::Image(last_image)), ClipboardContent::Image(current_image)) => {
                    last_image != current_image
                },
                (Some(ClipboardContent::Empty), ClipboardContent::Empty) => false,
                _ => true,
            };
            
            if is_different {
                *last = Some(current_content.clone());
            }
            
            is_different
        };
        
        // 如果内容不同，调用回调
        if different && current_content != ClipboardContent::Empty {
            let event = ClipboardEvent {
                content: current_content.clone(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            };
            
            // 调用回调
            callback(event);
            
            // 如果启用了历史记录功能，添加到历史记录
            if let Some(history) = history {
                if let Err(e) = history.add(current_content) {
                    warn!("添加到剪贴板历史记录失败: {e:?}");
                }
            }
        }
        
        Ok(())
    }
}

impl Drop for ClipboardWatcher {
    fn drop(&mut self) {
        if let Some(tx) = self.stop_tx.take() {
            // 阻塞发送停止信号
            let _ = tx.blocking_send(());
        }
    }
}

/// 剪贴板操作封装
pub struct Clipboard {
    inner: arboard::Clipboard,
}

impl Clipboard {
    /// 创建新的剪贴板实例
    pub fn new() -> Result<Self> {
        match arboard::Clipboard::new() {
            Ok(inner) => Ok(Self { inner }),
            Err(e) => {
                error!("创建剪贴板实例失败: {e:?}");
                Err(Error::Clipboard("创建剪贴板实例失败".to_string()))
            }
        }
    }

    /// 获取文本内容
    pub fn get_text(&mut self) -> Result<String> {
        match self.inner.get_text() {
            Ok(text) => Ok(text),
            Err(e) => {
                error!("获取剪贴板文本失败: {e:?}");
                Err(Error::Clipboard("获取剪贴板文本失败".to_string()))
            }
        }
    }

    /// 设置文本内容
    pub fn set_text(&mut self, text: &str) -> Result<()> {
        match self.inner.set_text(text) {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("设置剪贴板文本失败: {e:?}");
                Err(Error::Clipboard("设置剪贴板文本失败".to_string()))
            }
        }
    }

    /// 获取图片内容
    pub fn get_image(&mut self) -> Result<Vec<u8>> {
        match self.inner.get_image() {
            Ok(image) => {
                // 转换图片数据为RGBA字节数组
                let mut buffer = Vec::with_capacity((image.width * image.height * 4) as usize);
                for y in 0..image.height {
                    for x in 0..image.width {
                        let i = y * image.width + x;
                        buffer.extend_from_slice(&image.bytes[i * 4..(i + 1) * 4]);
                    }
                }
                Ok(buffer)
            },
            Err(e) => {
                error!("获取剪贴板图片失败: {e:?}");
                Err(Error::Clipboard("获取剪贴板图片失败".to_string()))
            }
        }
    }

    /// 获取文件路径列表
    pub fn get_file_paths(&self) -> Result<Option<Vec<String>>> {
        Ok(get_clipboard_file_paths())
    }

    /// 设置文件路径列表
    pub fn set_file_paths(&mut self, paths: &[String]) -> Result<()> {
        set_clipboard_file_paths(paths)
    }
}
