//! 测试环境使用的模拟剪贴板实现
//! 
//! 此模块提供了一个简化的剪贴板模拟实现，主要用于CI环境中的测试。
//! 不依赖系统剪贴板功能，避免在无图形界面的CI环境中出现问题。

use crate::clipboard::ClipboardContent;
use crate::error::{Error, Result};
use once_cell::sync::Mutex;
use std::sync::Arc;

// 使用静态变量模拟剪贴板
static MOCK_CLIPBOARD: once_cell::sync::Lazy<Mutex<Option<ClipboardContent>>> = 
    once_cell::sync::Lazy::new(|| Mutex::new(None));

/// 设置模拟剪贴板内容
pub fn set_mock_clipboard(content: ClipboardContent) -> Result<()> {
    let mut clipboard = MOCK_CLIPBOARD.lock().map_err(|_| Error::Clipboard("获取模拟剪贴板锁失败".to_string()))?;
    *clipboard = Some(content);
    Ok(())
}

/// 获取模拟剪贴板内容
pub fn get_mock_clipboard() -> Result<Option<ClipboardContent>> {
    let clipboard = MOCK_CLIPBOARD.lock().map_err(|_| Error::Clipboard("获取模拟剪贴板锁失败".to_string()))?;
    Ok(clipboard.clone())
}

/// 清除模拟剪贴板内容
pub fn clear_mock_clipboard() -> Result<()> {
    let mut clipboard = MOCK_CLIPBOARD.lock().map_err(|_| Error::Clipboard("获取模拟剪贴板锁失败".to_string()))?;
    *clipboard = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mock_clipboard() {
        // 初始应该为空
        assert!(get_mock_clipboard().unwrap().is_none());
        
        // 设置文本内容
        let text = "测试文本".to_string();
        set_mock_clipboard(ClipboardContent::Text(text.clone())).unwrap();
        
        // 验证内容
        if let Some(ClipboardContent::Text(content)) = get_mock_clipboard().unwrap() {
            assert_eq!(content, text);
        } else {
            panic!("剪贴板内容类型不匹配");
        }
        
        // 清除并验证
        clear_mock_clipboard().unwrap();
        assert!(get_mock_clipboard().unwrap().is_none());
    }
}
