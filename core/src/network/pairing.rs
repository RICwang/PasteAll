//! 设备配对模块，负责设备间的安全配对与认证

use crate::{
    crypto,
    error::{Error, Result},
    types::{AuthRequestPacket, DeviceInfo, PairingStatus},
};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

/// 配对请求回调函数类型
pub type PairingRequestCallback = Box<dyn Fn(DeviceInfo, String) -> bool + Send + Sync + 'static>;

/// 配对状态变更回调函数类型
pub type PairingStatusCallback = Box<dyn Fn(DeviceInfo, PairingStatus) + Send + Sync + 'static>;

/// 配对响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PairingResponse {
    /// 是否接受配对请求
    accepted: bool,
    /// PIN码（如果启用PIN码验证）
    pin: String,
}

/// 设备配对管理器
pub struct PairingManager {
    /// 本地设备信息
    local_device: DeviceInfo,
    /// 已配对设备映射表 (设备ID -> 设备信息)
    paired_devices: Arc<Mutex<HashMap<String, DeviceInfo>>>,
    /// 配对请求回调
    pairing_request_callback: Option<Arc<PairingRequestCallback>>,
    /// 配对状态变更回调
    status_callback: Option<Arc<PairingStatusCallback>>,
    /// 是否在等待配对中
    awaiting_pairing: Arc<Mutex<HashMap<String, String>>>, // 设备ID -> PIN码
    /// 停止信号发送端
    stop_tx: Option<mpsc::Sender<()>>,
}

impl PairingManager {
    /// 创建新的配对管理器
    pub fn new(local_device: DeviceInfo) -> Self {
        Self {
            local_device,
            paired_devices: Arc::new(Mutex::new(HashMap::new())),
            pairing_request_callback: None,
            status_callback: None,
            awaiting_pairing: Arc::new(Mutex::new(HashMap::new())),
            stop_tx: None,
        }
    }

    /// 设置配对请求回调
    pub fn set_pairing_request_callback(&mut self, callback: PairingRequestCallback) {
        self.pairing_request_callback = Some(Arc::new(callback));
    }

    /// 设置配对状态变更回调
    pub fn set_status_callback(&mut self, callback: PairingStatusCallback) {
        self.status_callback = Some(Arc::new(callback));
    }

    /// 启动配对监听服务
    pub async fn start_listening(&mut self, port: u16) -> Result<()> {
        if self.stop_tx.is_some() {
            warn!("配对监听服务已经在运行中");
            return Ok(());
        }

        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
        self.stop_tx = Some(stop_tx);

        let awaiting_pairing = self.awaiting_pairing.clone();
        let paired_devices = self.paired_devices.clone();
        let local_device_id = self.local_device.id.clone();
        let status_callback = self.status_callback.clone();

        // 启动TCP监听服务，接收配对请求
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await
            .map_err(|e| {
                error!("绑定配对监听套接字失败: {e:?}");
                Error::Network(format!("无法绑定地址: 0.0.0.0:{port}"))
            })?;

        info!("配对服务启动在端口: {port}");

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop_rx.recv() => {
                        info!("停止配对监听服务");
                        break;
                    }
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok((socket, addr)) => {
                                info!("接受新的配对连接: {addr}");
                                let awaiting = awaiting_pairing.clone();
                                let devices = paired_devices.clone();
                                let local_id = local_device_id.clone();
                                let callback = status_callback.clone();
                                
                                tokio::spawn(async move {
                                    if let Err(e) = Self::handle_pairing_connection(socket, awaiting, devices, local_id, callback).await {
                                        error!("处理配对连接失败: {e:?}");
                                    }
                                });
                            }
                            Err(e) => {
                                error!("接受配对连接失败: {e:?}");
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// 停止配对监听服务
    pub async fn stop_listening(&mut self) -> Result<()> {
        if let Some(stop_tx) = self.stop_tx.take() {
            if let Err(e) = stop_tx.send(()).await {
                error!("发送停止信号失败: {e:?}");
                return Err(Error::Network("停止配对监听服务失败".to_string()));
            }
        }
        Ok(())
    }

    /// 请求与设备配对
    pub async fn request_pairing(&self, device: &DeviceInfo) -> Result<()> {
        // 检查是否已经配对
        {
            let devices = self.paired_devices.lock().unwrap();
            if devices.contains_key(&device.id) {
                return Err(Error::Pairing("设备已配对".to_string()));
            }
        }

        // 生成随机PIN码
        let pin = Self::generate_pin();

        // 存储等待配对设备
        {
            let mut awaiting = self.awaiting_pairing.lock().unwrap();
            awaiting.insert(device.id.clone(), pin.clone());
        }

        // 连接到目标设备
        if let Some(ip) = &device.ip_address {
            let addr = format!("{}:45680", ip); // 假设目标设备在45680端口监听配对请求
            let mut stream = TcpStream::connect(&addr).await
                .map_err(|e| {
                    error!("连接到目标设备失败: {e:?}");
                    Error::Network(format!("无法连接到 {addr}"))
                })?;

            // 创建配对请求包
            let request = AuthRequestPacket {
                r#type: "pairing_request".to_string(),
                device_id: self.local_device.id.clone(),
                nonce: uuid::Uuid::new_v4().to_string(), // 使用随机UUID作为nonce
                signature: crypto::sign(&self.local_device.id, &self.local_device.public_key)?,
            };

            // 序列化并发送请求
            let request_json = serde_json::to_string(&request)
                .map_err(|e| Error::Serialization(e))?;
            
            let len_bytes = (request_json.len() as u32).to_be_bytes();
            stream.write_all(&len_bytes).await
                .map_err(|e| Error::Network(format!("发送请求长度失败: {e}")))?;
            
            stream.write_all(request_json.as_bytes()).await
                .map_err(|e| Error::Network(format!("发送配对请求失败: {e}")))?;

            // 读取响应
            let mut len_bytes = [0u8; 4];
            stream.read_exact(&mut len_bytes).await
                .map_err(|e| Error::Network(format!("读取响应长度失败: {e}")))?;
            
            let len = u32::from_be_bytes(len_bytes) as usize;
            let mut buffer = vec![0u8; len];
            
            stream.read_exact(&mut buffer).await
                .map_err(|e| Error::Network(format!("读取响应数据失败: {e}")))?;

            // 解析响应
            let response: PairingResponse = serde_json::from_slice(&buffer)
                .map_err(|e| Error::Serialization(e))?;

            // 处理响应结果
            if response.accepted {
                // 验证PIN码
                if response.pin == pin {
                    // 配对成功，保存设备
                    let mut device_info = device.clone();
                    device_info.pairing_status = PairingStatus::Paired;
                    device_info.trusted = true;
                    
                    // 添加到配对设备列表
                    {
                        let mut devices = self.paired_devices.lock().unwrap();
                        devices.insert(device_info.id.clone(), device_info.clone());
                    }
                    
                    // 通知状态变更
                    if let Some(callback) = &self.status_callback {
                        callback(device_info, PairingStatus::Paired);
                    }
                    
                    return Ok(());
                } else {
                    return Err(Error::Pairing("PIN码验证失败".to_string()));
                }
            } else {
                return Err(Error::Pairing("对方拒绝配对请求".to_string()));
            }
        } else {
            return Err(Error::Pairing("设备IP地址未知，无法发起配对".to_string()));
        }
    }

    /// 处理配对连接
    async fn handle_pairing_connection(
        mut socket: TcpStream,
        awaiting_pairing: Arc<Mutex<HashMap<String, String>>>,
        paired_devices: Arc<Mutex<HashMap<String, DeviceInfo>>>,
        _local_device_id: String,
        status_callback: Option<Arc<PairingStatusCallback>>,
    ) -> Result<()> {
        // 读取请求长度
        let mut len_bytes = [0u8; 4];
        socket.read_exact(&mut len_bytes).await
            .map_err(|e| Error::Network(format!("读取请求长度失败: {e}")))?;
        
        let len = u32::from_be_bytes(len_bytes) as usize;
        let mut buffer = vec![0u8; len];
        
        // 读取请求数据
        socket.read_exact(&mut buffer).await
            .map_err(|e| Error::Network(format!("读取请求数据失败: {e}")))?;

        // 解析请求
        let request: AuthRequestPacket = serde_json::from_slice(&buffer)
            .map_err(|e| Error::Serialization(e))?;

        // 检查是否已经配对
        let already_paired = {
            let devices = paired_devices.lock().unwrap();
            devices.contains_key(&request.device_id)
        };

        // 准备响应
        let mut response = PairingResponse {
            accepted: false,
            pin: String::new(),
        };

        // 如果已配对，直接接受
        if already_paired {
            response.accepted = true;
        } else {
            // 获取PIN码
            let pin = {
                let awaiting = awaiting_pairing.lock().unwrap();
                awaiting.get(&request.device_id).cloned()
            };

            // 如果有PIN码，表示我们期望这个设备配对
            if let Some(pin) = pin {
                response.accepted = true;
                response.pin = pin;
            }
        }

        // 序列化并发送响应
        let response_json = serde_json::to_string(&response)
            .map_err(|e| Error::Serialization(e))?;
        
        let len_bytes = (response_json.len() as u32).to_be_bytes();
        socket.write_all(&len_bytes).await
            .map_err(|e| Error::Network(format!("发送响应长度失败: {e}")))?;
        
        socket.write_all(response_json.as_bytes()).await
            .map_err(|e| Error::Network(format!("发送响应失败: {e}")))?;

        // 如果配对成功，更新设备状态
        if response.accepted && !already_paired {
            // TODO: 创建设备信息，目前简化处理
            let device_info = DeviceInfo::new("远程设备", crate::types::DeviceType::Unknown, "placeholder");
            
            // 添加到配对设备列表
            {
                let mut devices = paired_devices.lock().unwrap();
                devices.insert(device_info.id.clone(), device_info.clone());
            }
            
            // 通知状态变更
            if let Some(callback) = status_callback {
                callback(device_info, PairingStatus::Paired);
            }
        }

        Ok(())
    }

    /// 生成6位数PIN码
    fn generate_pin() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        format!("{:06}", rng.gen_range(0..1000000))
    }
    
    /// 获取已配对设备列表
    pub fn get_paired_devices(&self) -> Vec<DeviceInfo> {
        let devices = self.paired_devices.lock().unwrap();
        devices.values().cloned().collect()
    }
}
