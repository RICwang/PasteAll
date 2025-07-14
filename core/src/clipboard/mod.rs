//! 剪贴板操作模块，提供跨平台的剪贴板监听和操作功能

use crate::error::{Error, Result};
use arboard::Clipboard;
use log::{debug, error, info, warn};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::{self, Duration};

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
    clipboard: Arc<Mutex<Clipboard>>,
}

impl ClipboardWatcher {
    /// 创建新的剪贴板监听器
    pub fn new() -> Result<Self> {
        let clipboard = match Clipboard::new() {
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
        })
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

        // 启动监听任务
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_millis(500));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = Self::check_clipboard_change(clipboard.clone(), last_content.clone(), &callback).await {
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
                error!("获取剪贴板锁失败: {:?}", e);
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
                                let i = (y * image.width + x) as usize;
                                buffer.extend_from_slice(&image.bytes[i * 4..(i + 1) * 4]);
                            }
                        }
                        return Ok(ClipboardContent::Image(buffer));
                    }
                    Err(_) => {
                        // 4. 没有找到任何内容
                        return Ok(ClipboardContent::Empty);
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
                error!("获取剪贴板锁失败: {:?}", e);
                return Err(Error::Clipboard("获取剪贴板锁失败".to_string()));
            }
        };

        match content {
            ClipboardContent::Text(text) => {
                if let Err(e) = clipboard.set_text(text) {
                    error!("设置剪贴板文本失败: {:?}", e);
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
                    error!("设置剪贴板文件路径失败: {:?}", e);
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

    /// 检查剪贴板变化
    async fn check_clipboard_change(
        clipboard: Arc<Mutex<Clipboard>>,
        last_content: Arc<Mutex<Option<ClipboardContent>>>,
        callback: &ClipboardCallback,
    ) -> Result<()> {
        let current_content = {
            let mut clipboard_guard = match clipboard.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    error!("获取剪贴板锁失败: {:?}", e);
                    return Err(Error::Clipboard("获取剪贴板锁失败".to_string()));
                }
            };

            // 1. 首先尝试获取文件路径列表
            if let Some(file_paths) = get_clipboard_file_paths() {
                ClipboardContent::Files(file_paths)
            } else {
                // 2. 尝试获取文本
                match clipboard_guard.get_text() {
                    Ok(text) => ClipboardContent::Text(text),
                    Err(_) => {
                        // 3. 尝试获取图片
                        match clipboard_guard.get_image() {
                            Ok(image) => {
                                let mut buffer = Vec::new();
                                // 转换图片数据
                                for y in 0..image.height {
                                    for x in 0..image.width {
                                        let i = (y * image.width + x) as usize;
                                        buffer.extend_from_slice(&image.bytes[i * 4..(i + 1) * 4]);
                                    }
                                }
                                ClipboardContent::Image(buffer)
                            }
                            Err(_) => ClipboardContent::Empty,
                        }
                    }
                }
            }
        };

        // 检查内容是否变化
        let content_changed = {
            let last = match last_content.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    error!("获取上次内容锁失败: {:?}", e);
                    return Err(Error::Clipboard("获取上次内容锁失败".to_string()));
                }
            };

            match &*last {
                Some(prev) => *prev != current_content,
                None => true,
            }
        };

        if content_changed {
            debug!("检测到剪贴板内容变化");
            
            // 更新最后内容
            if let Ok(mut last) = last_content.lock() {
                *last = Some(current_content.clone());
            }

            // 触发回调
            let event = ClipboardEvent {
                content: current_content,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            };

            callback(event);
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

/// 获取剪贴板中的文件路径列表
#[cfg(target_os = "macos")]
fn get_clipboard_file_paths() -> Option<Vec<String>> {
    use std::process::Command;
    
    // 在macOS上使用osascript获取剪贴板中的文件路径
    let output = Command::new("osascript")
        .arg("-e")
        .arg(r#"
            use framework "Foundation"
            use framework "AppKit"
            set thePasteboard to current application's NSPasteboard's generalPasteboard()
            set theFiles to thePasteboard's pasteboardItems()
            set filePaths to {}
            
            repeat with theFile in theFiles
                set theURL to theFile's stringForType:"public.file-url"
                if theURL is not missing value then
                    set theURLObj to current application's NSURL's URLWithString:theURL
                    set thePath to theURLObj's |path|() as string
                    copy thePath to end of filePaths
                end if
            end repeat
            
            return filePaths
        "#)
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    // 处理输出结果
    let output_str = String::from_utf8_lossy(&output.stdout);
    if output_str.trim().is_empty() {
        return None;
    }
    
    // 解析路径列表
    let paths: Vec<String> = output_str
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    if paths.is_empty() {
        None
    } else {
        Some(paths)
    }
}

/// 获取剪贴板中的文件路径列表
#[cfg(target_os = "windows")]
fn get_clipboard_file_paths() -> Option<Vec<String>> {
    // 注意：Windows平台需要使用win32 API
    // 由于需要使用FFI，这里只提供实现思路
    // TODO: 使用windows-rs crate实现文件路径获取
    None
}

/// 获取剪贴板中的文件路径列表
#[cfg(target_os = "linux")]
fn get_clipboard_file_paths() -> Option<Vec<String>> {
    // 注意：Linux平台需要使用X11或Wayland的剪贴板API
    // TODO: 使用x11-clipboard或类似的crate实现
    None
}

/// 获取剪贴板中的文件路径列表（默认实现）
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn get_clipboard_file_paths() -> Option<Vec<String>> {
    None
}

/// 设置文件路径到剪贴板
#[cfg(target_os = "macos")]
fn set_clipboard_file_paths(paths: &[String]) -> Result<()> {
    use std::process::Command;
    
    if paths.is_empty() {
        return Ok(());
    }
    
    // 构建AppleScript命令
    let mut script = String::from(r#"
        use framework "Foundation"
        use framework "AppKit"
        set thePasteboard to current application's NSPasteboard's generalPasteboard()
        thePasteboard's clearContents()
        
        set theURLs to {}
    "#);
    
    // 添加每个文件路径
    for path in paths {
        // 转义路径中的双引号
        let escaped_path = path.replace("\"", "\\\"");
        script.push_str(&format!(r#"
            set fileURL to current application's NSURL's fileURLWithPath:"{}"
            copy fileURL to end of theURLs
        "#, escaped_path));
    }
    
    // 写入剪贴板
    script.push_str(r#"
        set theItems to current application's NSArray's array()
        set thePasteboardItem to current application's NSPasteboardItem's alloc()'s init()
        thePasteboardItem's setPropertyList:theURLs forType:"NSFilenamesPboardType"
        thePasteboard's writeObjects:{thePasteboardItem}
    "#);
    
    // 执行AppleScript
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output();
    
    match output {
        Ok(output) => {
            if output.status.success() {
                Ok(())
            } else {
                let error_msg = String::from_utf8_lossy(&output.stderr).to_string();
                Err(Error::Clipboard(format!("设置文件路径失败: {}", error_msg)))
            }
        }
        Err(e) => Err(Error::Clipboard(format!("执行AppleScript失败: {}", e))),
    }
}

/// 设置文件路径到剪贴板
#[cfg(target_os = "windows")]
fn set_clipboard_file_paths(_paths: &[String]) -> Result<()> {
    // TODO: 使用Windows API实现
    Err(Error::Clipboard("Windows平台暂未实现设置文件路径功能".to_string()))
}

/// 设置文件路径到剪贴板
#[cfg(target_os = "linux")]
fn set_clipboard_file_paths(_paths: &[String]) -> Result<()> {
    // TODO: 使用X11或Wayland API实现
    Err(Error::Clipboard("Linux平台暂未实现设置文件路径功能".to_string()))
}

/// 设置文件路径到剪贴板（默认实现）
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
fn set_clipboard_file_paths(_paths: &[String]) -> Result<()> {
    Err(Error::Clipboard("当前平台不支持设置文件路径功能".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn test_clipboard_content_equality() {
        let text1 = ClipboardContent::Text("Hello".to_string());
        let text2 = ClipboardContent::Text("Hello".to_string());
        let text3 = ClipboardContent::Text("World".to_string());
        
        assert_eq!(text1, text2);
        assert_ne!(text1, text3);
        
        let files1 = ClipboardContent::Files(vec!["path1.txt".to_string(), "path2.txt".to_string()]);
        let files2 = ClipboardContent::Files(vec!["path1.txt".to_string(), "path2.txt".to_string()]);
        let files3 = ClipboardContent::Files(vec!["path3.txt".to_string()]);
        
        assert_eq!(files1, files2);
        assert_ne!(files1, files3);
    }

    #[tokio::test]
    async fn test_clipboard_watcher_creation() {
        let watcher = ClipboardWatcher::new();
        assert!(watcher.is_ok());
    }
    
    #[tokio::test]
    async fn test_clipboard_watcher_start_stop() {
        let mut watcher = ClipboardWatcher::new().unwrap();
        
        let (tx, _rx) = mpsc::channel();
        
        // 创建回调函数
        let callback = Box::new(move |event: ClipboardEvent| {
            let _ = tx.send(event);
        });
        
        // 开始监听
        let result = watcher.start(callback).await;
        assert!(result.is_ok());
        
        // 停止监听
        let result = watcher.stop().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_clipboard_content_get_set() {
        let mut watcher = ClipboardWatcher::new().unwrap();
        
        // 设置文本内容
        let text = "Test clipboard content";
        let content = ClipboardContent::Text(text.to_string());
        let result = watcher.set_content(&content);
        
        // 注意：在CI环境中，剪贴板操作可能会失败，所以这里我们需要宽容一些
        if result.is_ok() {
            // 读取内容
            let read_content = watcher.get_content();
            assert!(read_content.is_ok());
            
            // 由于可能有多种类型的内容（文本、文件等），我们检查是否包含我们设置的文本
            if let Ok(ClipboardContent::Text(read_text)) = read_content {
                assert_eq!(read_text, text);
            }
        }
    }
}
