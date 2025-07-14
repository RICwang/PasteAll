//! 数据传输协议实现

use crate::{
    error::{Error, Result},
    types::{ContentPacket, DeviceInfo},
};
use log::{error, info, warn};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

/// 传输服务状态回调函数类型
pub type TransportCallback = Arc<dyn Fn(DeviceInfo, Vec<u8>) + Send + Sync + 'static>;

/// 数据传输服务
pub struct TransportService {
    /// 本地设备信息
    local_device: DeviceInfo,
    /// 已连接设备会话
    sessions: Arc<Mutex<HashMap<String, TcpStream>>>,
    /// 停止信号发送端
    stop_tx: Option<mpsc::Sender<()>>,
    /// 监听端口
    listen_port: u16,
}

impl TransportService {
    /// 创建新的数据传输服务
    pub fn new(local_device: DeviceInfo) -> Self {
        Self {
            local_device,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            stop_tx: None,
            listen_port: 45680,
        }
    }

    /// 启动数据传输服务
    pub async fn start(&mut self, callback: TransportCallback) -> Result<()> {
        if self.stop_tx.is_some() {
            warn!("数据传输服务已经在运行中");
            return Ok(());
        }

        info!("启动数据传输服务");

        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
        self.stop_tx = Some(stop_tx);

        let sessions = self.sessions.clone();
        let _local_device = self.local_device.clone();
        let listen_port = self.listen_port;

        // 启动监听任务
        tokio::spawn(async move {
            let addr = format!("0.0.0.0:{listen_port}");
            let listener = match TcpListener::bind(&addr).await {
                Ok(l) => l,
                Err(e) => {
                    error!("绑定TCP监听器失败: {e:?}");
                    return;
                }
            };

            info!("TCP监听器启动在 {addr}");

            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((socket, addr)) => {
                                info!("接受新连接: {addr}");

                                // 处理新连接
                                let _sessions_clone = sessions.clone();
                                // 使用Arc而不是clone回调
                                let callback = Arc::clone(&callback);
                                tokio::spawn(async move {
                                    // 这里应该进行认证
                                    // 简化起见，直接接受连接

                                    // 读取数据
                                    let mut buffer = Vec::new();
                                    let socket = socket;

                                    // 读取头部长度
                                    let mut header_buf = [0u8; 4];
                                    if let Err(e) = socket.try_read(&mut header_buf) {
                                        error!("读取数据头部失败: {e:?}");
                                        return;
                                    }

                                    let length = u32::from_be_bytes(header_buf) as usize;
                                    buffer.resize(length, 0);

                                    // 读取数据体
                                    if let Err(e) = socket.try_read(&mut buffer) {
                                        error!("读取数据体失败: {e:?}");
                                        return;
                                    }

                                    // 解析内容包
                                    if let Ok(packet) = serde_json::from_slice::<ContentPacket>(&buffer) {
                                        // 获取设备信息
                                        let device = DeviceInfo {
                                            id: packet.device_id,
                                            name: "远程设备".to_string(), // 这里应从会话中获取
                                            device_type: crate::types::DeviceType::Unknown,
                                            public_key: "".to_string(), // 这里应从会话中获取
                                            online: true,
                                            ip_address: None,
                                            system_version: None,
                                            app_version: None,
                                            capabilities: crate::types::DeviceCapabilities::default(),
                                            last_seen: Some(std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap_or_default()
                                                .as_secs()),
                                            pairing_status: crate::types::PairingStatus::default(),
                                            description: None,
                                            trusted: false,
                                        };

                                        // 触发回调
                                        callback(device, buffer);
                                    }
                                });
                            }
                            Err(e) => {
                                error!("接受连接失败: {e:?}");
                            }
                        }
                    }
                    _ = stop_rx.recv() => {
                        info!("停止数据传输服务");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// 停止数据传输服务
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(stop_tx) = self.stop_tx.take() {
            if let Err(e) = stop_tx.send(()).await {
                error!("发送停止信号失败: {e:?}");
                return Err(Error::Network("停止数据传输服务失败".to_string()));
            }
        }

        Ok(())
    }

    /// 发送数据到指定设备
    pub async fn send_data(&self, device: &DeviceInfo, data: &[u8]) -> Result<()> {
        let addr = format!("{}:{}", device.id, self.listen_port);

        // 连接到目标设备
        let stream = match TcpStream::connect(addr).await {
            Ok(s) => s,
            Err(e) => {
                error!("连接到目标设备失败: {e:?}");
                return Err(Error::Network("连接到目标设备失败".to_string()));
            }
        };

        // 发送数据长度头部
        let length = data.len() as u32;
        let length_bytes = length.to_be_bytes();

        if let Err(e) = stream.try_write(&length_bytes) {
            error!("发送数据长度失败: {e:?}");
            return Err(Error::Network("发送数据长度失败".to_string()));
        }

        // 发送数据体
        if let Err(e) = stream.try_write(data) {
            error!("发送数据体失败: {e:?}");
            return Err(Error::Network("发送数据体失败".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DeviceType;

    #[test]
    fn test_transport_service_creation() {
        let device = DeviceInfo::new("Test Device", DeviceType::Desktop, "test_public_key");

        let transport = TransportService::new(device);
        assert_eq!(transport.listen_port, 45680);
    }
}
