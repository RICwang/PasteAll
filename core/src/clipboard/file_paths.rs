#[cfg(all(feature = "linux-clipboard", target_os = "linux"))]
use percent_encoding;

use crate::error::{Error, Result};

/// 获取剪贴板中的文件路径
pub fn get_clipboard_file_paths() -> Option<Vec<String>> {
    #[cfg(target_os = "windows")]
    {
        // Windows实现
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use std::ptr;
        use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
        use windows::Win32::System::DataExchange::{CF_HDROP, CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard};
        use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
        use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};

        // 定义一个宏用于确保资源在作用域结束时释放
        macro_rules! defer {
            ($($body:tt)*) => {
                let _guard = scopeguard::guard((), |_| { $($body)* });
            };
        }

        unsafe {
            // 尝试打开剪贴板
            if OpenClipboard(ptr::null_mut()) == false {
                return None;
            }

            // 确保在返回前关闭剪贴板
            defer! {
                CloseClipboard();
            }

            // 检查是否有文件拖放数据
            if IsClipboardFormatAvailable(CF_HDROP) == false {
                return None;
            }

            let result = (|| {
                // 获取剪贴板数据
                let h_drop = GetClipboardData(CF_HDROP) as HDROP;
                if h_drop.is_null() {
                    return None;
                }

                // 锁定全局内存
                let h_drop = GlobalLock(h_drop as *mut _) as HDROP;
                if h_drop.is_null() {
                    return None;
                }

                // 确保在返回前解锁内存
                defer! {
                    GlobalUnlock(h_drop as *mut _);
                };

                // 获取文件数量
                let file_count = DragQueryFileW(h_drop, 0xFFFFFFFF, ptr::null_mut(), 0);
                if file_count == 0 {
                    return None;
                }

                let mut paths = Vec::with_capacity(file_count as usize);

                // 获取每个文件的路径
                for i in 0..file_count {
                    // 获取文件名需要的缓冲区大小
                    let len = DragQueryFileW(h_drop, i, ptr::null_mut(), 0);
                    if len == 0 {
                        continue;
                    }

                    // 为文件名分配缓冲区
                    let mut buffer = Vec::<u16>::with_capacity((len + 1) as usize);
                    buffer.resize((len + 1) as usize, 0);

                    // 获取文件名
                    let len = DragQueryFileW(h_drop, i, buffer.as_mut_ptr(), (len + 1));
                    if len == 0 {
                        continue;
                    }

                    // 转换为OsString然后为String
                    let os_string = OsString::from_wide(&buffer[..len as usize]);
                    match os_string.into_string() {
                        Ok(path) => paths.push(path),
                        Err(_) => continue,
                    }
                }

                Some(paths)
            })();

            result
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS使用pasteboard接口
        // 需要使用系统原生API，这里仅做示意
        use std::process::Command;
        
        let output = Command::new("osascript")
            .arg("-e")
            .arg(r#"tell application "Finder" to set theClipboard to (get clipboard) as text"#)
            .output();
            
        match output {
            Ok(out) => {
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                if text.trim().starts_with("file://") {
                    let paths: Vec<String> = text
                        .lines()
                        .filter(|line| line.starts_with("file://"))
                        .map(|line| {
                            let path = line.trim_start_matches("file://");
                            path.to_string()
                        })
                        .collect();
                    
                    if !paths.is_empty() {
                        return Some(paths);
                    }
                }
            }
            Err(_) => {}
        }
        
        None
    }

    #[cfg(all(target_os = "linux", feature = "linux-clipboard"))]
    {
        // Linux通常使用xclip或其他X11工具
        use std::process::Command;
        
        let output = Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .arg("-o")
            .arg("-t")
            .arg("text/uri-list")
            .output();
            
        match output {
            Ok(out) => {
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                if !text.is_empty() {
                    let paths: Vec<String> = text
                        .lines()
                        .filter(|line| line.starts_with("file://"))
                        .map(|line| {
                            let path = line.trim_start_matches("file://");
                            percent_encoding::percent_decode_str(path).decode_utf8_lossy().to_string()
                        })
                        .collect();
                    
                    if !paths.is_empty() {
                        return Some(paths);
                    }
                }
            }
            Err(_) => {}
        }
        
        None
    }

    #[cfg(all(target_os = "linux", not(feature = "linux-clipboard")))]
    {
        None
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

/// 设置剪贴板中的文件路径
pub fn set_clipboard_file_paths(paths: &[String]) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        // Windows实现
        // 需要使用COM接口操作剪贴板
        use std::ptr;
        use windows::Win32::System::DataExchange::{EmptyClipboard, OpenClipboard, CloseClipboard};
        
        unsafe {
            if OpenClipboard(ptr::null_mut()) == false {
                return Err(Error::Clipboard("打开剪贴板失败".to_string()));
            }
            
            // 清空剪贴板
            EmptyClipboard();
            
            // 实现文件拖放需要复杂的COM操作
            // 这里只是示意，实际需要完整的COM实现
            
            CloseClipboard();
        }
        
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        // macOS实现
        use std::process::Command;
        
        let mut script = String::from(r#"tell application "Finder" to set the clipboard to {"#);
        
        for (i, path) in paths.iter().enumerate() {
            if i > 0 {
                script.push_str(", ");
            }
            script.push_str(&format!(r#"POSIX file "{}""#, path));
        }
        
        script.push_str("}");
        
        let status = Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .status();
            
        match status {
            Ok(exit_status) if exit_status.success() => Ok(()),
            _ => Err(Error::Clipboard("设置剪贴板文件路径失败".to_string()))
        }
    }

    #[cfg(all(target_os = "linux", feature = "linux-clipboard"))]
    {
        // Linux实现
        use std::process::{Command, Stdio};
        use std::io::Write;
        
        let mut uri_list = String::new();
        
        for path in paths {
            uri_list.push_str(&format!("file://{}\n", path));
        }
        
        let mut child = Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .arg("-t")
            .arg("text/uri-list")
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| Error::Clipboard(format!("启动xclip失败: {}", e)))?;
            
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(uri_list.as_bytes())
                .map_err(|e| Error::Clipboard(format!("写入xclip失败: {}", e)))?;
        }
        
        let status = child.wait()
            .map_err(|e| Error::Clipboard(format!("等待xclip完成失败: {}", e)))?;
            
        if status.success() {
            Ok(())
        } else {
            Err(Error::Clipboard("设置剪贴板文件路径失败".to_string()))
        }
    }

    #[cfg(all(target_os = "linux", not(feature = "linux-clipboard")))]
    {
        Err(Error::Clipboard("Linux剪贴板功能未启用".to_string()))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err(Error::Clipboard("当前平台不支持设置剪贴板文件路径".to_string()))
    }
}
