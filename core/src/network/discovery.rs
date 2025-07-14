//! 设备发现模块，负责在局域网内发现其他设备

use crate::{
    error::{Error, Result},
    types::{Config, DeviceInfo, DiscoveryPacket, PairingStatus},
};
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use tokio::net::UdpSocket;
use tokio::sync::mpsc;
use tokio::time::{self, Duration};

/// 设备发现回调函数类型
pub type DeviceDiscoveryCallback = Box<dyn Fn(DeviceInfo) + Send + Sync + 'static>;

/// 设备发现服务
pub struct DeviceDiscovery {
    /// 本地设备信息
    local_device: DeviceInfo,
    /// 发现的设备列表
    devices: Arc<Mutex<HashMap<String, DeviceInfo>>>,
    /// 停止信号发送端
    stop_tx: Option<mpsc::Sender<()>>,
    /// 广播端口
    broadcast_port: u16,
    /// 监听端口
    listen_port: u16,
}

impl DeviceDiscovery {
    /// 创建新的设备发现服务
    pub fn new(config: &Config) -> Result<Self> {
        // 这里使用UUID作为设备ID
        let _device_id = uuid::Uuid::new_v4().to_string();

        // 假设已经生成或加载了密钥对
        // 实际实现中应该从安全存储中加载或生成
        let public_key = "dummy_public_key_base64_encoded";

        let local_device = DeviceInfo::new(&config.device_name, config.device_type, public_key);

        Ok(Self {
            local_device,
            devices: Arc::new(Mutex::new(HashMap::new())),
            stop_tx: None,
            broadcast_port: 45678,
            listen_port: 45679,
        })
    }

    /// 开始设备发现服务
    pub async fn start(&mut self, callback: DeviceDiscoveryCallback) -> Result<()> {
        if self.stop_tx.is_some() {
            warn!("设备发现服务已经在运行中");
            return Ok(());
        }

        info!("开始设备发现服务");

        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
        self.stop_tx = Some(stop_tx);

        let devices = self.devices.clone();
        let _local_device_id = self.local_device.id.clone(); // 只克隆ID，避免移动问题
        let local_device_broadcast = self.local_device.clone();
        let local_device_listen = self.local_device.clone();
        let broadcast_port = self.broadcast_port;
        let listen_port = self.listen_port;

        // 启动广播任务
        let broadcast_task = tokio::spawn(async move {
            let local_device = local_device_broadcast; // 在任务内部使用本地变量
            let socket = match UdpSocket::bind("0.0.0.0:0").await {
                Ok(s) => s,
                Err(e) => {
                    error!("绑定广播套接字失败: {e:?}");
                    return;
                }
            };

            if let Err(e) = socket.set_broadcast(true) {
                error!("设置广播套接字选项失败: {e:?}");
                return;
            }

            let broadcast_addr = SocketAddr::new(
                IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255)),
                broadcast_port,
            );

            let mut interval = time::interval(Duration::from_secs(5));

            // 创建广播包
            let discovery_packet = DiscoveryPacket {
                r#type: "discovery".to_string(),
                device_id: local_device.id.clone(),
                device_name: local_device.name.clone(),
                public_key: local_device.public_key.clone(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                device_type: local_device.device_type,
                port: broadcast_port,
                ip_address: None, // 这里可以获取本地IP地址
                capabilities: local_device.capabilities,
                app_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                system_version: None,
                protocol_version: "1.0".to_string(),
            };

            let packet_json = match serde_json::to_string(&discovery_packet) {
                Ok(json) => json,
                Err(e) => {
                    error!("序列化发现包失败: {e:?}");
                    return;
                }
            };

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        debug!("发送广播包: {packet_json}");
                        if let Err(e) = socket.send_to(packet_json.as_bytes(), broadcast_addr).await {
                            error!("发送广播包失败: {e:?}");
                        }
                    }
                    _ = stop_rx.recv() => {
                        info!("停止设备发现广播");
                        break;
                    }
                }
            }
        });

        // 启动监听任务
        let listen_task = tokio::spawn(async move {
            let local_device = local_device_listen; // 在任务内部使用本地变量
            let socket = match UdpSocket::bind(format!("0.0.0.0:{listen_port}")).await {
                Ok(s) => s,
                Err(e) => {
                    error!("绑定监听套接字失败: {e:?}");
                    return;
                }
            };

            let mut buf = vec![0u8; 1024];

            loop {
                tokio::select! {
                    result = socket.recv_from(&mut buf) => {
                        match result {
                            Ok((len, addr)) => {
                                if let Ok(packet_str) = String::from_utf8(buf[..len].to_vec()) {
                                    debug!("收到数据包: {packet_str} 来自: {addr}");

                                    if let Ok(packet) = serde_json::from_str::<DiscoveryPacket>(&packet_str) {
                                        // 忽略自己发送的包
                                        if packet.device_id != local_device.id {
                                            let device = DeviceInfo {
                                                id: packet.device_id,
                                                name: packet.device_name,
                                                device_type: packet.device_type,
                                                public_key: packet.public_key,
                                                online: true,
                                                ip_address: packet.ip_address,
                                                system_version: packet.system_version,
                                                app_version: packet.app_version,
                                                capabilities: packet.capabilities,
                                                last_seen: Some(packet.timestamp),
                                                pairing_status: PairingStatus::default(),
                                                description: None,
                                                trusted: false,
                                            };

                                            // 更新设备列表
                                            {
                                                let mut devices_map = devices.lock().unwrap();
                                                devices_map.insert(device.id.clone(), device.clone());
                                            }

                                            // 触发回调
                                            callback(device);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("接收数据包失败: {e:?}");
                            }
                        }
                    }
                }
            }
        });

        // 等待任务完成
        // 实际应用中应该有更好的管理方式
        tokio::spawn(async move {
            let _ = broadcast_task.await;
            let _ = listen_task.await;
        });

        Ok(())
    }

    /// 停止设备发现服务
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(stop_tx) = self.stop_tx.take() {
            if let Err(e) = stop_tx.send(()).await {
                error!("发送停止信号失败: {e:?}");
                return Err(Error::Discovery("停止设备发现服务失败".to_string()));
            }
        }

        Ok(())
    }

    /// 获取发现的设备列表
    pub fn get_devices(&self) -> Vec<DeviceInfo> {
        let devices = match self.devices.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("获取设备列表锁失败: {e:?}");
                return Vec::new();
            }
        };

        devices.values().cloned().collect()
    }

    /// 通过ID获取设备
    pub fn get_device_by_id(&self, device_id: &str) -> Option<DeviceInfo> {
        let devices = match self.devices.lock() {
            Ok(guard) => guard,
            Err(e) => {
                error!("获取设备列表锁失败: {e:?}");
                return None;
            }
        };

        devices.get(device_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DeviceType;

    #[test]
    fn test_device_discovery_creation() {
        let config = Config {
            device_name: "Test Device".to_string(),
            device_type: DeviceType::Desktop,
            storage_path: ":memory:".to_string(),
            device_id: "test_id".to_string(),
            discovery_port: 8888,
            capabilities: crate::types::DeviceCapabilities::default(),
            listen_port: 8889,
            options: crate::types::ConfigOptions::default(),
        };

        let discovery = DeviceDiscovery::new(&config);
        assert!(discovery.is_ok());
    }
}
