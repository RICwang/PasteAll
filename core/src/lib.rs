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
#![allow(clippy::empty_line_after_doc_comments)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

/// 剪贴板操作相关模块
pub mod clipboard;
/// 加密与安全相关模块
pub mod crypto;
/// 错误处理相关模块
pub mod error;
/// FFI绑定相关模块
pub mod ffi;
/// 网络通信协议相关模块
pub mod network;
/// 存储相关模块
pub mod storage;
/// 基础通用类型和常量
pub mod types;

use log::{info, warn};

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

        // 初始化加密模块
        crypto::init();
        
        // 初始化剪贴板监听
        let _clipboard_watcher = clipboard::ClipboardWatcher::new()?;
        
        // 创建本地设备信息
        let local_device = types::DeviceInfo::new(
            &self.config.device_name,
            self.config.device_type,
            &crypto::get_public_key()?
        );
        
        // 初始化设备发现服务
        let mut discovery = network::discovery::DeviceDiscovery::new(&self.config)?;
        
        // 尝试初始化BLE设备发现
        let ble_discovery_result = network::ble_discovery::BleDiscovery::new(&self.config).await;
        
        if let Ok(mut ble_discovery) = ble_discovery_result {
            info!("BLE设备发现服务已初始化");
            
            // 启动BLE设备发现（示例回调）
            let ble_callback = Box::new(|device: types::DeviceInfo| {
                info!("发现BLE设备: {} ({})", device.name, device.id);
            });
            
            if let Err(e) = ble_discovery.start(ble_callback).await {
                info!("BLE设备发现启动失败: {e:?}，将使用UDP广播");
            }
        } else {
            info!("BLE设备发现不可用，将使用UDP广播");
        }
        
        // 启动UDP设备发现（示例回调）
        let udp_callback = Box::new(|device: types::DeviceInfo| {
            info!("发现UDP设备: {} ({})", device.name, device.id);
        });
        
        discovery.start(udp_callback).await?;
        
        // 初始化配对管理器
        let mut pairing_manager = network::pairing::PairingManager::new(local_device.clone());
        
        // 设置配对请求回调
        pairing_manager.set_pairing_request_callback(Box::new(|device, pin| {
            info!("收到配对请求: {} ({}), PIN: {}", device.name, device.id, pin);
            // 实际应用中，这里应该弹出UI让用户确认
            true // 测试环境下自动接受
        }));
        
        // 设置配对状态变更回调
        pairing_manager.set_status_callback(Box::new(|device, status| {
            info!("设备 {} ({}) 配对状态更新为: {:?}", device.name, device.id, status);
        }));
        
        // 启动配对监听服务
        if let Err(e) = pairing_manager.start_listening(self.config.listen_port).await {
            warn!("启动配对监听服务失败: {e:?}");
        }

        // 初始化存储
        let _storage = storage::Storage::new(&self.config.storage_path)?;

        // 初始化Wi-Fi传输服务
        let mut wifi_transport = network::wifi_transport::WiFiTransport::new(
            local_device,
            self.config.listen_port + 1 // 使用listen_port+1作为文件传输端口
        );
        
        // 启动Wi-Fi传输服务
        if let Err(e) = wifi_transport.start_server().await {
            warn!("启动Wi-Fi传输服务失败: {e:?}");
        }

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
