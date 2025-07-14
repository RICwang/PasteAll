//! PasteAll核心库 - 跨平台近距离设备复制粘贴工具的核心功能实现
//! 
//! 本模块包含以下核心功能：
//! - 剪贴板监听与操作
//! - 加密和安全
//! - 设备发现与配对
//! - 网络通信协议
//! - 数据存储

#![warn(missing_docs)]
#![allow(clippy::new_without_default)]

/// 剪贴板操作相关模块
pub mod clipboard;
/// 加密与安全相关模块
pub mod crypto;
/// 错误处理相关模块
pub mod error;
/// 网络通信协议相关模块
pub mod network;
/// 存储相关模块
pub mod storage;
/// 基础通用类型和常量
pub mod types;
/// FFI绑定相关模块
pub mod ffi;

use log::info;

/// PasteAll核心库的入口点
pub struct PasteAll {
    /// PasteAll的配置信息
    config: types::Config,
}

impl PasteAll {
    /// 创建PasteAll实例
    pub fn new(config: types::Config) -> Self {
        info!("PasteAll核心库初始化");
        Self { config }
    }

    /// 启动PasteAll服务
    pub async fn start(&self) -> Result<(), error::Error> {
        info!("启动PasteAll核心服务");
        
        // 初始化剪贴板监听
        let _clipboard_watcher = clipboard::ClipboardWatcher::new()?;
        
        // 初始化设备发现服务
        let _device_discovery = network::discovery::DeviceDiscovery::new(&self.config)?;
        
        // 初始化存储
        let _storage = storage::Storage::new(&self.config.storage_path)?;
        
        // 初始化加密模块
        crypto::init();
        
        info!("PasteAll核心服务启动完成");
        Ok(())
    }

    /// 停止PasteAll服务
    pub async fn stop(&self) -> Result<(), error::Error> {
        info!("停止PasteAll核心服务");
        // 清理资源
        Ok(())
    }
}

/// 初始化日志系统
pub fn init_logger() {
    env_logger::init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_pasteall_instance() {
        let config = types::Config {
            device_name: "Test Device".to_string(),
            device_type: types::DeviceType::Desktop,
            storage_path: ":memory:".to_string(),
            discovery_port: 5678,
            listen_port: 5679,
            device_id: uuid::Uuid::new_v4().to_string(),
            capabilities: types::DeviceCapabilities::default(),
            options: types::ConfigOptions::default(),
        };

        let pasteall = PasteAll::new(config);
        assert_eq!(pasteall.config.device_name, "Test Device");
    }
}
