//! Wi-Fi数据传输模块，专门用于高效的文件和大数据传输

use crate::error::{Error, Result};
use crate::types::{DeviceInfo, TransferProgress, TransferStatus};
use log::{error, info, warn};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

/// 进度回调函数类型
pub type ProgressCallback = Arc<dyn Fn(TransferProgress) + Send + Sync + 'static>;

/// 块大小：256KB
const BLOCK_SIZE: usize = 262144;

/// Wi-Fi传输服务
pub struct WiFiTransport {
    /// 本地设备信息
    local_device: DeviceInfo,
    /// 停止信号发送端
    stop_tx: Option<mpsc::Sender<()>>,
    /// 监听端口
    port: u16,
    /// 当前传输进度
    progress: Arc<Mutex<HashMap<String, TransferProgress>>>,
}

impl WiFiTransport {
    /// 创建新的Wi-Fi传输服务
    pub fn new(local_device: DeviceInfo, port: u16) -> Self {
        Self {
            local_device,
            stop_tx: None,
            port,
            progress: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 启动服务器端
    pub async fn start_server(&mut self) -> Result<()> {
        if self.stop_tx.is_some() {
            warn!("Wi-Fi传输服务已经在运行中");
            return Ok(());
        }

        let addr = format!("0.0.0.0:{}", self.port);
        let listener = TcpListener::bind(&addr).await.map_err(|e| {
            error!("绑定TCP监听器失败: {e:?}");
            Error::Network(format!("无法绑定地址 {addr}"))
        })?;

        info!("Wi-Fi传输服务启动在 {addr}");

        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
        self.stop_tx = Some(stop_tx);

        let progress = self.progress.clone();

        // 启动监听任务
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop_rx.recv() => {
                        info!("停止Wi-Fi传输服务");
                        break;
                    }
                    accept_result = listener.accept() => {
                        match accept_result {
                            Ok((socket, addr)) => {
                                info!("接受新的传输连接: {addr}");
                                let progress_clone = progress.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = Self::handle_incoming(socket, progress_clone).await {
                                        error!("处理传输连接失败: {e:?}");
                                    }
                                });
                            }
                            Err(e) => {
                                error!("接受连接失败: {e:?}");
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// 停止服务器
    pub async fn stop_server(&mut self) -> Result<()> {
        if let Some(stop_tx) = self.stop_tx.take() {
            if let Err(e) = stop_tx.send(()).await {
                error!("发送停止信号失败: {e:?}");
                return Err(Error::Network("停止Wi-Fi传输服务失败".to_string()));
            }
        }
        Ok(())
    }

    /// 发送文件
    pub async fn send_file<P: AsRef<Path>>(
        &self,
        device_info: &DeviceInfo,
        file_path: P,
        callback: Option<ProgressCallback>,
    ) -> Result<()> {
        let file_path = file_path.as_ref();
        let file_name = file_path
            .file_name()
            .ok_or_else(|| Error::Network("无效的文件路径".to_string()))?
            .to_string_lossy()
            .to_string();

        let file_size = tokio::fs::metadata(file_path)
            .await
            .map_err(|e| Error::Network(format!("获取文件元数据失败: {e}")))?
            .len();

        // 生成传输ID
        let transfer_id = uuid::Uuid::new_v4().to_string();

        // 添加到进度跟踪
        {
            let mut progress_guard = self.progress.lock().unwrap();
            progress_guard.insert(
                transfer_id.clone(),
                TransferProgress {
                    id: transfer_id.clone(),
                    file_name: file_name.clone(),
                    total_bytes: file_size,
                    transferred_bytes: 0,
                    status: TransferStatus::Starting,
                },
            );
        }

        // 更新回调
        if let Some(cb) = &callback {
            let progress = {
                let progress_guard = self.progress.lock().unwrap();
                progress_guard.get(&transfer_id).cloned().unwrap()
            };
            cb(progress);
        }

        // 连接到目标设备
        let ip = device_info.ip_address.as_ref().ok_or_else(|| Error::Network("设备IP地址未知".to_string()))?;
        let addr = format!("{}:{}", ip, self.port);
        let mut stream = match TcpStream::connect(&addr).await {
            Ok(stream) => stream,
            Err(e) => {
                error!("连接到目标设备失败: {e:?}");
                self.update_progress(
                    &transfer_id,
                    0,
                    TransferStatus::Failed("连接失败".to_string()),
                    &callback,
                );
                return Err(Error::Network(format!("无法连接到 {addr}")));
            }
        };

        // 发送文件头信息
        let header = serde_json::to_string(&FileHeader {
            transfer_id: transfer_id.clone(),
            file_name,
            file_size,
        })
        .unwrap();

        let header_len = header.len() as u32;
        if let Err(e) = stream.write_all(&header_len.to_be_bytes()).await {
            error!("发送头部长度失败: {e:?}");
            self.update_progress(
                &transfer_id,
                0,
                TransferStatus::Failed("发送文件信息失败".to_string()),
                &callback,
            );
            return Err(Error::Network("发送文件头信息失败".to_string()));
        }

        if let Err(e) = stream.write_all(header.as_bytes()).await {
            error!("发送头部内容失败: {e:?}");
            self.update_progress(
                &transfer_id,
                0,
                TransferStatus::Failed("发送文件信息失败".to_string()),
                &callback,
            );
            return Err(Error::Network("发送文件头信息失败".to_string()));
        }

        // 开始传输文件
        self.update_progress(&transfer_id, 0, TransferStatus::InProgress, &callback);

        let file = match File::open(file_path).await {
            Ok(file) => file,
            Err(e) => {
                error!("打开文件失败: {e:?}");
                self.update_progress(
                    &transfer_id,
                    0,
                    TransferStatus::Failed("无法打开文件".to_string()),
                    &callback,
                );
                return Err(Error::Network(format!("打开文件失败: {e}")));
            }
        };

        let mut reader = BufReader::new(file);
        let mut buffer = vec![0u8; BLOCK_SIZE];
        let mut transferred = 0u64;

        loop {
            let n = match reader.read(&mut buffer).await {
                Ok(0) => break, // 文件结束
                Ok(n) => n,
                Err(e) => {
                    error!("读取文件失败: {e:?}");
                    self.update_progress(
                        &transfer_id,
                        transferred,
                        TransferStatus::Failed("读取文件失败".to_string()),
                        &callback,
                    );
                    return Err(Error::Network(format!("读取文件失败: {e}")));
                }
            };

            if let Err(e) = stream.write_all(&buffer[..n]).await {
                error!("发送文件数据失败: {e:?}");
                self.update_progress(
                    &transfer_id,
                    transferred,
                    TransferStatus::Failed("发送数据失败".to_string()),
                    &callback,
                );
                return Err(Error::Network("发送文件数据失败".to_string()));
            }

            transferred += n as u64;
            self.update_progress(&transfer_id, transferred, TransferStatus::InProgress, &callback);
        }

        // 传输完成
        self.update_progress(
            &transfer_id,
            file_size,
            TransferStatus::Completed,
            &callback,
        );

        info!("文件传输完成: {}", file_path.display());
        Ok(())
    }

    /// 更新传输进度
    fn update_progress(
        &self,
        transfer_id: &str,
        transferred_bytes: u64,
        status: TransferStatus,
        callback: &Option<ProgressCallback>,
    ) {
        let progress = {
            let mut progress_guard = self.progress.lock().unwrap();
            if let Some(p) = progress_guard.get_mut(transfer_id) {
                p.transferred_bytes = transferred_bytes;
                p.status = status;
                p.clone()
            } else {
                return;
            }
        };

        if let Some(cb) = callback {
            cb(progress);
        }
    }

    /// 处理入站连接
    async fn handle_incoming(
        mut socket: TcpStream,
        progress: Arc<Mutex<HashMap<String, TransferProgress>>>,
    ) -> Result<()> {
        // 读取头部长度
        let mut header_len_bytes = [0u8; 4];
        socket.read_exact(&mut header_len_bytes).await.map_err(|e| {
            error!("读取头部长度失败: {e:?}");
            Error::Network("读取文件头长度失败".to_string())
        })?;

        let header_len = u32::from_be_bytes(header_len_bytes);

        // 读取头部
        let mut header_bytes = vec![0u8; header_len as usize];
        socket.read_exact(&mut header_bytes).await.map_err(|e| {
            error!("读取头部内容失败: {e:?}");
            Error::Network("读取文件头内容失败".to_string())
        })?;

        let header: FileHeader = serde_json::from_slice(&header_bytes).map_err(|e| {
            error!("解析文件头失败: {e:?}");
            Error::Network("解析文件头失败".to_string())
        })?;

        info!(
            "接收文件传输: {} ({} 字节)",
            header.file_name, header.file_size
        );

        // 添加到进度跟踪
        {
            let mut progress_guard = progress.lock().unwrap();
            progress_guard.insert(
                header.transfer_id.clone(),
                TransferProgress {
                    id: header.transfer_id.clone(),
                    file_name: header.file_name.clone(),
                    total_bytes: header.file_size,
                    transferred_bytes: 0,
                    status: TransferStatus::Starting,
                },
            );
        }

        // 创建文件
        let downloads_dir = dirs::download_dir().unwrap_or_else(|| Path::new(".").to_path_buf());
        let file_path = downloads_dir.join(&header.file_name);

        let file = File::create(&file_path).await.map_err(|e| {
            error!("创建文件失败: {e:?}");
            Error::Network(format!("创建文件失败: {e}"))
        })?;

        let mut writer = BufWriter::new(file);
        let mut buffer = vec![0u8; BLOCK_SIZE];
        let mut received = 0u64;

        // 更新状态
        {
            let mut progress_guard = progress.lock().unwrap();
            if let Some(p) = progress_guard.get_mut(&header.transfer_id) {
                p.status = TransferStatus::InProgress;
            }
        }

        // 接收文件数据
        loop {
            let n = match socket.read(&mut buffer).await {
                Ok(0) => break, // 连接关闭
                Ok(n) => n,
                Err(e) => {
                    error!("读取文件数据失败: {e:?}");
                    // 更新状态为失败
                    {
                        let mut progress_guard = progress.lock().unwrap();
                        if let Some(p) = progress_guard.get_mut(&header.transfer_id) {
                            p.status = TransferStatus::Failed("读取数据失败".to_string());
                        }
                    }
                    return Err(Error::Network(format!("读取文件数据失败: {e}")));
                }
            };

            writer.write_all(&buffer[..n]).await.map_err(|e| {
                error!("写入文件数据失败: {e:?}");
                // 更新状态为失败
                {
                    let mut progress_guard = progress.lock().unwrap();
                    if let Some(p) = progress_guard.get_mut(&header.transfer_id) {
                        p.status = TransferStatus::Failed("写入数据失败".to_string());
                    }
                }
                Error::Network(format!("写入文件失败: {e}"))
            })?;

            received += n as u64;

            // 更新进度
            {
                let mut progress_guard = progress.lock().unwrap();
                if let Some(p) = progress_guard.get_mut(&header.transfer_id) {
                    p.transferred_bytes = received;
                }
            }
        }

        // 确保所有数据都写入磁盘
        writer.flush().await.map_err(|e| {
            error!("刷新文件数据失败: {e:?}");
            Error::Network(format!("确保文件写入失败: {e}"))
        })?;

        // 更新状态为完成
        {
            let mut progress_guard = progress.lock().unwrap();
            if let Some(p) = progress_guard.get_mut(&header.transfer_id) {
                p.transferred_bytes = received;
                p.status = TransferStatus::Completed;
            }
        }

        info!("文件接收完成: {}", file_path.display());
        Ok(())
    }
}

/// 文件头信息
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct FileHeader {
    /// 传输ID
    transfer_id: String,
    /// 文件名
    file_name: String,
    /// 文件大小（字节）
    file_size: u64,
}

use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DeviceType;
    use std::path::PathBuf;
    use tokio::fs;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn test_wifi_transport_creation() {
        let device = DeviceInfo::new("Test Device", DeviceType::Desktop, "dummy_key");
        let transport = WiFiTransport::new(device, 45681);
        
        assert_eq!(transport.port, 45681);
    }

    #[tokio::test]
    async fn test_wifi_transport_server() {
        let device = DeviceInfo::new("Test Device", DeviceType::Desktop, "dummy_key");
        let mut transport = WiFiTransport::new(device, 0); // 使用随机端口

        let result = transport.start_server().await;
        assert!(result.is_ok());

        let result = transport.stop_server().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // 这个测试需要手动运行
    async fn test_file_transfer() {
        // 创建临时测试文件
        let temp_dir = tempfile::tempdir().unwrap();
        let source_path = temp_dir.path().join("source.txt");
        let mut source_file = fs::File::create(&source_path).await.unwrap();
        
        // 写入1MB的随机数据
        let data = vec![b'A'; 1024 * 1024];
        source_file.write_all(&data).await.unwrap();
        source_file.flush().await.unwrap();
        drop(source_file);

        // 服务端
        let server_device = DeviceInfo::new("Server", DeviceType::Desktop, "server_key");
        let mut server = WiFiTransport::new(server_device.clone(), 45682);
        server.start_server().await.unwrap();

        // 客户端
        let client_device = DeviceInfo::new("Client", DeviceType::Desktop, "client_key");
        let client_device_with_ip = DeviceInfo {
            last_ip: "127.0.0.1".to_string(),
            ..client_device
        };
        let client = WiFiTransport::new(client_device, 45683);

        // 发送文件
        let progress_callback = Arc::new(|progress: TransferProgress| {
            println!(
                "传输进度: {}/{} 字节 ({}%)",
                progress.transferred_bytes,
                progress.total_bytes,
                progress.transferred_bytes * 100 / progress.total_bytes
            );
        });

        let result = client
            .send_file(&server_device_with_ip, &source_path, Some(progress_callback))
            .await;

        // 清理
        server.stop_server().await.unwrap();
        temp_dir.close().unwrap();
        
        assert!(result.is_ok());
    }
}
