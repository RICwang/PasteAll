//! 蓝牙低功耗(BLE)设备发现与连接模块

use crate::error::{Error, Result};
use crate::types::{Config, DeviceInfo};
use btleplug::api::{
    Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use futures::stream::StreamExt;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time;
use uuid::Uuid;

/// 服务UUID - 用于识别PasteAll设备
const PASTEALL_SERVICE_UUID: Uuid = Uuid::from_u128(0x00001000_0000_1000_8000_00805f9b34fb);
/// 设备信息特性UUID
const DEVICE_INFO_CHAR_UUID: Uuid = Uuid::from_u128(0x00001001_0000_1000_8000_00805f9b34fb);
/// 配对请求特性UUID
const PAIRING_REQ_CHAR_UUID: Uuid = Uuid::from_u128(0x00001002_0000_1000_8000_00805f9b34fb);
/// 配对响应特性UUID
const PAIRING_RESP_CHAR_UUID: Uuid = Uuid::from_u128(0x00001003_0000_1000_8000_00805f9b34fb);

/// 设备发现回调函数类型
pub type BleDeviceDiscoveryCallback = Box<dyn Fn(DeviceInfo) + Send + Sync + 'static>;

/// BLE设备发现服务
pub struct BleDiscovery {
    /// 本地设备信息
    local_device: DeviceInfo,
    /// 发现的设备列表
    devices: Arc<Mutex<HashMap<String, DeviceInfo>>>,
    /// 停止信号发送端
    stop_tx: Option<mpsc::Sender<()>>,
    /// 蓝牙适配器
    adapter: Option<Adapter>,
}

impl BleDiscovery {
    /// 创建新的BLE设备发现服务
    pub async fn new(config: &Config) -> Result<Self> {
        // 获取蓝牙适配器
        let manager = Manager::new().await.map_err(|e| {
            error!("无法创建蓝牙管理器: {e:?}");
            Error::Network("无法初始化蓝牙管理器".to_string())
        })?;

        let adapters = manager.adapters().await.map_err(|e| {
            error!("无法获取蓝牙适配器: {e:?}");
            Error::Network("无法获取蓝牙适配器列表".to_string())
        })?;

        if adapters.is_empty() {
            return Err(Error::Network("未找到蓝牙适配器".to_string()));
        }

        // 使用第一个适配器
        let adapter = adapters.into_iter().next().unwrap();
        info!("使用蓝牙适配器: {}", adapter.adapter_info().await.unwrap());

        let local_device = DeviceInfo::new(&config.device_name, config.device_type, "dummy_key");

        Ok(Self {
            local_device,
            devices: Arc::new(Mutex::new(HashMap::new())),
            stop_tx: None,
            adapter: Some(adapter),
        })
    }

    /// 启动BLE设备发现
    pub async fn start(&mut self, callback: BleDeviceDiscoveryCallback) -> Result<()> {
        if self.stop_tx.is_some() {
            warn!("BLE设备发现已经在运行中");
            return Ok(());
        }

        let adapter = match &self.adapter {
            Some(adapter) => adapter.clone(),
            None => return Err(Error::Network("蓝牙适配器未初始化".to_string())),
        };

        // 创建停止通道
        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
        self.stop_tx = Some(stop_tx);

        // 保存设备列表的引用
        let devices = self.devices.clone();
        
        // 开始扫描
        adapter.start_scan(ScanFilter::default()).await.map_err(|e| {
            error!("启动蓝牙扫描失败: {e:?}");
            Error::Network("无法启动蓝牙扫描".to_string())
        })?;
        
        info!("开始BLE设备扫描");

        // 创建事件监听器
        let mut events = adapter.events().await.map_err(|e| {
            error!("无法获取蓝牙事件: {e:?}");
            Error::Network("无法监听蓝牙事件".to_string())
        })?;

        // 启动事件处理任务
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop_rx.recv() => {
                        info!("停止BLE设备扫描");
                        let _ = adapter.stop_scan().await;
                        break;
                    }
                    event = events.next() => {
                        if let Some(CentralEvent::DeviceDiscovered(id)) = event {
                            // 尝试连接设备并检查是否为PasteAll设备
                            if let Ok(peripheral) = adapter.peripheral(&id).await {
                                if let Err(e) = Self::process_discovered_device(&peripheral, devices.clone(), &callback).await {
                                    error!("处理发现的设备失败: {e:?}");
                                }
                            }
                        }
                    }
                    _ = time::sleep(Duration::from_secs(30)) => {
                        // 每30秒重新扫描一次
                        debug!("刷新BLE设备扫描");
                        let _ = adapter.stop_scan().await;
                        let _ = time::sleep(Duration::from_millis(100)).await;
                        let _ = adapter.start_scan(ScanFilter::default()).await;
                    }
                }
            }
        });

        Ok(())
    }

    /// 停止BLE设备发现
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(stop_tx) = self.stop_tx.take() {
            if let Err(e) = stop_tx.send(()).await {
                error!("发送停止信号失败: {e:?}");
                return Err(Error::Network("停止BLE设备发现失败".to_string()));
            }
        }

        if let Some(adapter) = &self.adapter {
            if let Err(e) = adapter.stop_scan().await {
                error!("停止蓝牙扫描失败: {e:?}");
                return Err(Error::Network("无法停止蓝牙扫描".to_string()));
            }
        }

        Ok(())
    }

    /// 获取已发现的设备列表
    pub fn get_devices(&self) -> Result<Vec<DeviceInfo>> {
        let devices = match self.devices.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("获取设备列表锁失败: {e:?}");
                return Err(Error::Network("获取设备列表失败".to_string()));
            }
        };

        Ok(devices.values().cloned().collect())
    }

    /// 处理发现的设备
    async fn process_discovered_device(
        peripheral: &Peripheral,
        devices: Arc<Mutex<HashMap<String, DeviceInfo>>>,
        callback: &BleDeviceDiscoveryCallback,
    ) -> Result<()> {
        // 连接到设备
        if let Err(e) = peripheral.connect().await {
            debug!("连接到设备失败: {e:?}");
            return Ok(()); // 不是错误，只是这个设备不是我们要找的
        }

        // 发现服务
        if let Err(e) = peripheral.discover_services().await {
            debug!("发现服务失败: {e:?}");
            let _ = peripheral.disconnect().await;
            return Ok(());
        }

        // 检查是否有PasteAll服务
        let services = peripheral.services();
        if !services.iter().any(|s| s.uuid == PASTEALL_SERVICE_UUID) {
            let _ = peripheral.disconnect().await;
            return Ok(());
        }

        // 读取设备信息
        let characteristics = peripheral.characteristics();
        let device_info_char = characteristics
            .iter()
            .find(|c| c.uuid == DEVICE_INFO_CHAR_UUID)
            .ok_or_else(|| Error::Network("设备信息特性不存在".to_string()))?;

        let data = peripheral
            .read(device_info_char)
            .await
            .map_err(|e| {
                error!("读取设备信息失败: {e:?}");
                Error::Network("无法读取设备信息".to_string())
            })?;

        // 解析设备信息
        let device_info: DeviceInfo = match serde_json::from_slice(&data) {
            Ok(info) => info,
            Err(e) => {
                error!("解析设备信息失败: {e:?}");
                let _ = peripheral.disconnect().await;
                return Ok(());
            }
        };

        // 断开连接，节省电池
        let _ = peripheral.disconnect().await;

        // 保存设备信息
        {
            let mut devices_guard = match devices.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    error!("获取设备列表锁失败: {e:?}");
                    return Err(Error::Network("获取设备列表锁失败".to_string()));
                }
            };

            devices_guard.insert(device_info.id.clone(), device_info.clone());
        }

        // 触发回调
        callback(device_info);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DeviceType;

    #[tokio::test]
    #[ignore] // 需要硬件支持，默认忽略
    async fn test_ble_discovery_creation() {
        let config = Config {
            device_name: "Test Device".to_string(),
            device_type: DeviceType::Desktop,
            listen_port: 45679,
        };

        let ble_discovery = BleDiscovery::new(&config).await;
        assert!(ble_discovery.is_ok());
    }
}
