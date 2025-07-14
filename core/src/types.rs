//! 基础类型和常量定义模块
//!
//! PasteAll 核心类型模块提供了整个应用所需的基本数据结构和类型定义。
//!
//! 本模块包含以下主要类型：
//!
//! - 设备相关：`DeviceType`, `DeviceInfo`, `DeviceCapabilities`
//! - 内容相关：`ContentType`, `ContentPacket`, `ContentMetadata`
//! - 配对相关：`PairingStatus`, `ConnectionStatus`, `AuthRequestPacket`
//! - 传输相关：`TransferStatus`, `TransferProgress`, `FileTransfer`
//! - 发现相关：`DiscoveryPacket`
//! - 配置相关：`Config`, `ConfigOptions`, `SecurityPolicy`
//! - 通知相关：`NotificationType`, `Notification`, `NotificationAction`
//! - 消息相关：`Message`, `MessageType`
//!
//! 所有类型均实现了 `Serialize` 和 `Deserialize` trait，方便在网络传输和存储中使用。
//!
//! # 示例
//!
//! ```
//! use pasteall_core::types::{DeviceInfo, DeviceType};
//!
//! // 创建一个新的设备信息
//! let device = DeviceInfo::new(
//!     "我的设备",
//!     DeviceType::Desktop,
//!     "base64_encoded_public_key"
//! );
//!
//! assert_eq!(device.name, "我的设备");
//! assert_eq!(device.device_type, DeviceType::Desktop);
//! ```

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 设备类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    /// 桌面设备（Windows/macOS）
    Desktop,
    /// 移动设备（iOS/Android）
    Mobile,
    /// 未知设备类型
    Unknown,
}

impl Default for DeviceType {
    fn default() -> Self {
        Self::Unknown
    }
}

/// 内容类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentType {
    /// 文本内容
    Text(String),
    /// 文件内容
    File {
        /// 文件名
        name: String,
        /// MIME类型
        mime_type: String,
        /// 文件大小（字节）
        size: u64,
    },
    /// 图片内容
    Image {
        /// 图片格式
        format: String,
        /// 图片宽度
        width: u32,
        /// 图片高度
        height: u32,
        /// 图片大小（字节）
        size: u64,
    },
}

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// 设备ID
    pub id: String,
    /// 设备名称
    pub name: String,
    /// 设备类型
    pub device_type: DeviceType,
    /// 公钥（Base64编码）
    pub public_key: String,
    /// 设备是否在线
    pub online: bool,
    /// 设备IP地址
    pub ip_address: Option<String>,
    /// 设备系统版本
    pub system_version: Option<String>,
    /// 设备应用版本
    pub app_version: Option<String>,
    /// 设备能力
    #[serde(default)]
    pub capabilities: DeviceCapabilities,
    /// 最后一次在线时间（时间戳）
    pub last_seen: Option<u64>,
    /// 配对状态
    #[serde(default)]
    pub pairing_status: PairingStatus,
    /// 设备描述（可选）
    pub description: Option<String>,
    /// 是否为受信任设备
    #[serde(default)]
    pub trusted: bool,
}

impl DeviceInfo {
    /// 创建新的设备信息
    pub fn new(name: &str, device_type: DeviceType, public_key: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            device_type,
            public_key: public_key.to_string(),
            online: true,
            ip_address: None,
            system_version: None,
            app_version: None,
            capabilities: DeviceCapabilities::default(),
            last_seen: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            ),
            pairing_status: PairingStatus::Unpaired,
            description: None,
            trusted: false,
        }
    }

    /// 创建带有详细信息的设备信息
    pub fn with_details(
        name: &str,
        device_type: DeviceType,
        public_key: &str,
        ip_address: Option<String>,
        system_version: Option<String>,
        app_version: Option<String>,
        capabilities: DeviceCapabilities,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            device_type,
            public_key: public_key.to_string(),
            online: true,
            ip_address,
            system_version,
            app_version,
            capabilities,
            last_seen: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            ),
            pairing_status: PairingStatus::Unpaired,
            description: None,
            trusted: false,
        }
    }

    /// 设置设备在线状态并更新最后在线时间
    pub fn set_online(&mut self, online: bool) {
        self.online = online;
        if online {
            self.last_seen = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
        }
    }

    /// 更新设备信息
    pub fn update_from(&mut self, other: &DeviceInfo) {
        self.name = other.name.clone();
        self.device_type = other.device_type;
        self.online = other.online;

        if let Some(ip) = &other.ip_address {
            self.ip_address = Some(ip.clone());
        }

        if let Some(sv) = &other.system_version {
            self.system_version = Some(sv.clone());
        }

        if let Some(av) = &other.app_version {
            self.app_version = Some(av.clone());
        }

        self.capabilities = other.capabilities;

        if let Some(ls) = other.last_seen {
            self.last_seen = Some(ls);
        }

        if let Some(desc) = &other.description {
            self.description = Some(desc.clone());
        }
    }
}

/// PasteAll配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 设备名称
    pub device_name: String,
    /// 设备类型
    pub device_type: DeviceType,
    /// 存储路径
    pub storage_path: String,
    /// 设备能力
    #[serde(default)]
    pub capabilities: DeviceCapabilities,
    /// 监听端口
    #[serde(default = "default_listen_port")]
    pub listen_port: u16,
    /// 发现端口
    #[serde(default = "default_discovery_port")]
    pub discovery_port: u16,
    /// 设备ID（UUID）
    #[serde(default = "generate_uuid")]
    pub device_id: String,
    /// 附加选项
    #[serde(default)]
    pub options: ConfigOptions,
}

/// 默认监听端口
fn default_listen_port() -> u16 {
    45680
}

/// 默认发现端口
fn default_discovery_port() -> u16 {
    45678
}

/// 生成UUID
fn generate_uuid() -> String {
    Uuid::new_v4().to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            device_name: "未命名设备".to_string(),
            device_type: DeviceType::Unknown,
            storage_path: "pasteall.db".to_string(),
            capabilities: DeviceCapabilities::default(),
            listen_port: default_listen_port(),
            discovery_port: default_discovery_port(),
            device_id: generate_uuid(),
            options: ConfigOptions::default(),
        }
    }
}

/// 设备发现包
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryPacket {
    /// 包类型
    pub r#type: String,
    /// 设备ID
    pub device_id: String,
    /// 设备名称
    pub device_name: String,
    /// 设备公钥
    pub public_key: String,
    /// 时间戳
    pub timestamp: u64,
    /// 设备类型
    pub device_type: DeviceType,
    /// 监听端口
    pub port: u16,
    /// IP地址（可选）
    pub ip_address: Option<String>,
    /// 设备能力
    #[serde(default)]
    pub capabilities: DeviceCapabilities,
    /// 应用版本
    pub app_version: Option<String>,
    /// 系统版本
    pub system_version: Option<String>,
    /// 协议版本
    pub protocol_version: String,
}

/// 认证请求包
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequestPacket {
    /// 包类型
    pub r#type: String,
    /// 设备ID
    pub device_id: String,
    /// 随机数（用于防重放攻击）
    pub nonce: String,
    /// 签名
    pub signature: String,
}

/// 内容传输包
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPacket {
    /// 包类型
    pub r#type: String,
    /// 设备ID
    pub device_id: String,
    /// 内容类型
    pub content_type: String,
    /// Base64编码的内容
    pub content: String,
    /// 元数据
    pub metadata: ContentMetadata,
    /// 时间戳
    pub timestamp: u64,
}

/// 内容元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentMetadata {
    /// 可选的文件名
    pub filename: Option<String>,
    /// 内容大小
    pub size: u64,
    /// MIME类型
    pub mime_type: String,
}

/// 配对状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PairingStatus {
    /// 未配对
    Unpaired,
    /// 配对请求已发送
    RequestSent,
    /// 配对请求已接收
    RequestReceived,
    /// 已配对
    Paired,
}

impl Default for PairingStatus {
    fn default() -> Self {
        Self::Unpaired
    }
}

/// 连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    /// 未连接
    Disconnected,
    /// 正在连接
    Connecting,
    /// 已连接
    Connected,
}

/// 传输进度更新
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    /// 传输ID
    pub id: String,
    /// 已传输字节数
    pub bytes_transferred: u64,
    /// 总字节数
    pub total_bytes: u64,
    /// 传输状态
    pub status: TransferStatus,
    /// 设备ID
    pub device_id: String,
}

impl TransferProgress {
    /// 计算传输进度百分比
    pub fn progress_percentage(&self) -> f32 {
        if self.total_bytes == 0 {
            return 0.0;
        }
        (self.bytes_transferred as f32 / self.total_bytes as f32) * 100.0
    }
}

/// 传输状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferStatus {
    /// 初始化中
    Initializing,
    /// 进行中
    InProgress,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Canceled,
}

/// 设备连接信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    /// 设备ID
    pub device_id: String,
    /// 连接状态
    pub status: ConnectionStatus,
    /// IP地址
    pub ip_address: Option<String>,
    /// 端口
    pub port: Option<u16>,
    /// BLE连接标识符
    pub ble_identifier: Option<String>,
    /// 连接时间戳
    pub connected_at: Option<u64>,
    /// 最后活跃时间戳
    pub last_active: Option<u64>,
}

/// 消息类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    /// 配对请求
    PairingRequest,
    /// 配对响应
    PairingResponse {
        /// 是否接受配对请求
        accepted: bool,
    },
    /// 文本传输
    TextTransfer {
        /// 待传输的文本内容
        text: String,
    },
    /// 文件传输开始
    FileTransferStart {
        /// 文件唯一标识符
        file_id: String,
        /// 文件名称
        filename: String,
        /// 文件大小（字节）
        size: u64,
        /// 文件MIME类型
        mime_type: String,
    },
    /// 文件传输数据块
    FileTransferChunk {
        /// 文件唯一标识符
        file_id: String,
        /// 数据块索引
        chunk_index: u32,
        /// 数据块内容
        data: Vec<u8>,
    },
    /// 文件传输完成
    FileTransferComplete {
        /// 文件唯一标识符
        file_id: String,
        /// 文件校验和
        checksum: String,
    },
    /// 设备状态更新
    StatusUpdate {
        /// 设备是否在线
        online: bool,
        /// 电池电量百分比
        battery_level: Option<u8>,
        /// 是否正在充电
        is_charging: Option<bool>,
    },
    /// 传输取消
    TransferCancel {
        /// 传输唯一标识符
        transfer_id: String,
    },
    /// 心跳包
    Heartbeat,
    /// 错误消息
    Error {
        /// 错误代码
        code: u16,
        /// 错误消息描述
        message: String,
    },
}

/// 通用消息结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// 消息ID
    pub id: String,
    /// 消息类型
    pub message_type: MessageType,
    /// 发送者ID
    pub sender_id: String,
    /// 接收者ID
    pub receiver_id: Option<String>,
    /// 时间戳
    pub timestamp: u64,
    /// 是否需要确认
    pub require_ack: bool,
    /// 关联消息ID（如果是响应消息）
    pub related_message_id: Option<String>,
    /// 消息过期时间（Unix时间戳，0表示不过期）
    pub expires_at: u64,
}

impl Message {
    /// 创建新的消息
    pub fn new(
        sender_id: &str,
        message_type: MessageType,
        require_ack: bool,
        receiver_id: Option<&str>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message_type,
            sender_id: sender_id.to_string(),
            receiver_id: receiver_id.map(|s| s.to_string()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            require_ack,
            related_message_id: None,
            expires_at: 0,
        }
    }

    /// 设置消息过期时间
    pub fn with_expiry(mut self, seconds_from_now: u64) -> Self {
        if seconds_from_now > 0 {
            self.expires_at = self.timestamp + seconds_from_now;
        }
        self
    }

    /// 创建响应消息
    pub fn create_response(&self, message_type: MessageType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message_type,
            sender_id: self
                .receiver_id
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            receiver_id: Some(self.sender_id.clone()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            require_ack: false,
            related_message_id: Some(self.id.clone()),
            expires_at: 0,
        }
    }

    /// 检查消息是否已过期
    pub fn is_expired(&self) -> bool {
        if self.expires_at == 0 {
            return false;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.expires_at < now
    }
}

/// 设备能力标志
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    /// 支持文件传输
    pub supports_files: bool,
    /// 支持图片传输
    pub supports_images: bool,
    /// 支持BLE发现
    pub supports_ble: bool,
    /// 支持WiFi直连
    pub supports_wifi_direct: bool,
    /// 支持NFC
    pub supports_nfc: bool,
    /// 支持后台运行
    pub supports_background: bool,
    /// 最大支持文件大小(MB)
    pub max_file_size: u32,
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            supports_files: true,
            supports_images: true,
            supports_ble: false,
            supports_wifi_direct: true,
            supports_nfc: false,
            supports_background: true,
            max_file_size: 1024, // 1GB
        }
    }
}

/// 设备安全策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityPolicy {
    /// 自动接受所有请求
    AutoAcceptAll,
    /// 仅接受已配对设备
    AcceptPairedOnly,
    /// 总是询问
    AlwaysAsk,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self::AlwaysAsk
    }
}

/// 配置附加选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOptions {
    /// 安全策略
    pub security_policy: SecurityPolicy,
    /// 自动清除历史记录的天数
    pub auto_clear_history_days: Option<u32>,
    /// 保存接收文件的默认目录
    pub default_download_dir: Option<String>,
    /// 启用通知
    pub enable_notifications: bool,
    /// 自动启动
    pub auto_start: bool,
    /// 开机自启动
    pub start_on_boot: bool,
}

impl Default for ConfigOptions {
    fn default() -> Self {
        Self {
            security_policy: SecurityPolicy::default(),
            auto_clear_history_days: None,
            default_download_dir: None,
            enable_notifications: true,
            auto_start: true,
            start_on_boot: false,
        }
    }
}

/// 文件传输描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTransfer {
    /// 传输ID
    pub id: String,
    /// 文件名
    pub filename: String,
    /// 文件大小
    pub size: u64,
    /// MIME类型
    pub mime_type: String,
    /// 发送者ID
    pub sender_id: String,
    /// 接收者ID
    pub receiver_id: String,
    /// 开始时间
    pub start_time: u64,
    /// 结束时间
    pub end_time: Option<u64>,
    /// 状态
    pub status: TransferStatus,
    /// 分块大小
    pub chunk_size: u32,
    /// 总分块数
    pub total_chunks: u32,
    /// 已完成分块数
    pub completed_chunks: u32,
    /// 校验和（SHA-256）
    pub checksum: Option<String>,
    /// 存储路径
    pub storage_path: Option<String>,
}

impl FileTransfer {
    /// 创建新的文件传输描述
    pub fn new(
        filename: &str,
        size: u64,
        mime_type: &str,
        sender_id: &str,
        receiver_id: &str,
        chunk_size: u32,
    ) -> Self {
        let total_chunks = ((size as f64) / (chunk_size as f64)).ceil() as u32;

        Self {
            id: Uuid::new_v4().to_string(),
            filename: filename.to_string(),
            size,
            mime_type: mime_type.to_string(),
            sender_id: sender_id.to_string(),
            receiver_id: receiver_id.to_string(),
            start_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            end_time: None,
            status: TransferStatus::Initializing,
            chunk_size,
            total_chunks,
            completed_chunks: 0,
            checksum: None,
            storage_path: None,
        }
    }

    /// 更新传输进度
    pub fn update_progress(&mut self, completed_chunks: u32) -> TransferProgress {
        self.completed_chunks = completed_chunks;

        // 如果全部完成，更新状态和结束时间
        if self.completed_chunks >= self.total_chunks {
            self.status = TransferStatus::Completed;
            self.end_time = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            );
        } else if self.status == TransferStatus::Initializing {
            self.status = TransferStatus::InProgress;
        }

        // 返回进度信息
        TransferProgress {
            id: self.id.clone(),
            bytes_transferred: (self.completed_chunks as u64) * (self.chunk_size as u64),
            total_bytes: self.size,
            status: self.status,
            device_id: self.receiver_id.clone(),
        }
    }

    /// 标记传输失败
    pub fn mark_failed(&mut self) -> TransferProgress {
        self.status = TransferStatus::Failed;
        self.end_time = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );

        TransferProgress {
            id: self.id.clone(),
            bytes_transferred: (self.completed_chunks as u64) * (self.chunk_size as u64),
            total_bytes: self.size,
            status: self.status,
            device_id: self.receiver_id.clone(),
        }
    }

    /// 标记传输取消
    pub fn mark_canceled(&mut self) -> TransferProgress {
        self.status = TransferStatus::Canceled;
        self.end_time = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );

        TransferProgress {
            id: self.id.clone(),
            bytes_transferred: (self.completed_chunks as u64) * (self.chunk_size as u64),
            total_bytes: self.size,
            status: self.status,
            device_id: self.receiver_id.clone(),
        }
    }

    /// 计算传输时间（秒）
    pub fn transfer_duration(&self) -> Option<u64> {
        self.end_time.map(|end| end - self.start_time)
    }

    /// 计算传输速度（字节/秒）
    pub fn transfer_speed(&self) -> Option<f64> {
        let bytes_transferred = (self.completed_chunks as u64) * (self.chunk_size as u64);

        // 如果没有传输字节或者没有结束时间，返回None
        if bytes_transferred == 0 {
            return None;
        }

        let duration = if let Some(end) = self.end_time {
            end - self.start_time
        } else {
            // 如果尚未结束，使用当前时间计算
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                - self.start_time
        };

        // 避免除以0
        if duration == 0 {
            return None;
        }

        Some(bytes_transferred as f64 / duration as f64)
    }
}

/// 通知优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationPriority {
    /// 低优先级
    Low,
    /// 普通优先级
    Normal,
    /// 高优先级
    High,
    /// 紧急优先级
    Urgent,
}

/// 通知类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationType {
    /// 设备发现
    DeviceDiscovered,
    /// 配对请求
    PairingRequest,
    /// 配对成功
    PairingSuccess,
    /// 配对失败
    PairingFailure,
    /// 接收文本
    TextReceived,
    /// 文件传输请求
    FileTransferRequest,
    /// 文件传输开始
    FileTransferStarted,
    /// 文件传输完成
    FileTransferCompleted,
    /// 文件传输失败
    FileTransferFailed,
    /// 连接状态变化
    ConnectionChanged,
    /// 错误通知
    Error,
    /// 系统通知
    System,
}

/// 应用通知
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// 通知ID
    pub id: String,
    /// 通知类型
    pub notification_type: NotificationType,
    /// 通知标题
    pub title: String,
    /// 通知内容
    pub content: String,
    /// 通知优先级
    pub priority: NotificationPriority,
    /// 时间戳
    pub timestamp: u64,
    /// 关联的设备ID
    pub device_id: Option<String>,
    /// 关联的传输ID
    pub transfer_id: Option<String>,
    /// 通知是否已读
    pub is_read: bool,
    /// 通知操作
    pub actions: Vec<NotificationAction>,
}

/// 通知操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    /// 操作ID
    pub id: String,
    /// 操作标签
    pub label: String,
    /// 操作类型
    pub action_type: NotificationActionType,
}

/// 通知操作类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationActionType {
    /// 接受操作
    Accept,
    /// 拒绝操作
    Reject,
    /// 打开文件
    OpenFile {
        /// 文件路径
        path: String,
    },
    /// 打开URL
    OpenUrl {
        /// 网址链接
        url: String,
    },
    /// 查看详情
    ViewDetails {
        /// 详情类型
        details_type: String,
        /// 详情对象ID
        id: String,
    },
    /// 自定义操作
    Custom {
        /// 自定义命令
        command: String,
        /// 命令参数
        params: String,
    },
}

impl Notification {
    /// 创建新的通知
    pub fn new(
        notification_type: NotificationType,
        title: &str,
        content: &str,
        priority: NotificationPriority,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            notification_type,
            title: title.to_string(),
            content: content.to_string(),
            priority,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            device_id: None,
            transfer_id: None,
            is_read: false,
            actions: Vec::new(),
        }
    }

    /// 添加设备ID关联
    pub fn with_device_id(mut self, device_id: &str) -> Self {
        self.device_id = Some(device_id.to_string());
        self
    }

    /// 添加传输ID关联
    pub fn with_transfer_id(mut self, transfer_id: &str) -> Self {
        self.transfer_id = Some(transfer_id.to_string());
        self
    }

    /// 添加通知操作
    pub fn with_action(mut self, action: NotificationAction) -> Self {
        self.actions.push(action);
        self
    }

    /// 标记为已读
    pub fn mark_as_read(&mut self) {
        self.is_read = true;
    }
}

impl NotificationAction {
    /// 创建接受操作
    pub fn accept(label: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            label: label.to_string(),
            action_type: NotificationActionType::Accept,
        }
    }

    /// 创建拒绝操作
    pub fn reject(label: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            label: label.to_string(),
            action_type: NotificationActionType::Reject,
        }
    }

    /// 创建打开文件操作
    pub fn open_file(label: &str, path: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            label: label.to_string(),
            action_type: NotificationActionType::OpenFile {
                path: path.to_string(),
            },
        }
    }

    /// 创建打开URL操作
    pub fn open_url(label: &str, url: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            label: label.to_string(),
            action_type: NotificationActionType::OpenUrl {
                url: url.to_string(),
            },
        }
    }

    /// 创建查看详情操作
    pub fn view_details(label: &str, details_type: &str, id: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            label: label.to_string(),
            action_type: NotificationActionType::ViewDetails {
                details_type: details_type.to_string(),
                id: id.to_string(),
            },
        }
    }

    /// 创建自定义操作
    pub fn custom(label: &str, command: &str, params: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            label: label.to_string(),
            action_type: NotificationActionType::Custom {
                command: command.to_string(),
                params: params.to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info_creation() {
        let device = DeviceInfo::new("测试设备", DeviceType::Desktop, "base64_encoded_public_key");

        assert_eq!(device.name, "测试设备");
        assert_eq!(device.device_type, DeviceType::Desktop);
        assert_eq!(device.public_key, "base64_encoded_public_key");
        assert!(device.online);
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.device_name, "未命名设备");
        assert_eq!(config.device_type, DeviceType::Unknown);
    }

    #[test]
    fn test_transfer_progress() {
        let progress = TransferProgress {
            id: "test_id".to_string(),
            bytes_transferred: 50,
            total_bytes: 100,
            status: TransferStatus::InProgress,
            device_id: "device1".to_string(),
        };

        assert_eq!(progress.progress_percentage(), 50.0);
    }

    #[test]
    fn test_device_capabilities() {
        let caps = DeviceCapabilities::default();
        assert!(caps.supports_files);
        assert!(caps.supports_images);
    }
}

#[cfg(test)]
mod type_tests {
    use super::*;

    #[test]
    fn test_message_creation_and_expiry() {
        let msg = Message::new(
            "sender1",
            MessageType::TextTransfer {
                text: "Hello".to_string(),
            },
            true,
            Some("receiver1"),
        );

        assert_eq!(msg.sender_id, "sender1");
        assert!(matches!(msg.message_type, MessageType::TextTransfer { .. }));
        assert_eq!(msg.receiver_id, Some("receiver1".to_string()));
        assert!(msg.require_ack);
        assert!(!msg.is_expired());

        // Test with expiry
        let msg_with_expiry = Message::new(
            "sender1",
            MessageType::TextTransfer {
                text: "Hello".to_string(),
            },
            true,
            Some("receiver1"),
        )
        .with_expiry(1); // 1 second expiry

        // Sleep for 2 seconds to ensure it's expired
        std::thread::sleep(std::time::Duration::from_secs(2));
        assert!(msg_with_expiry.is_expired());

        // Test response message
        let response = msg.create_response(MessageType::PairingResponse { accepted: true });
        assert_eq!(response.sender_id, "receiver1");
        assert_eq!(response.receiver_id, Some("sender1".to_string()));
        assert_eq!(response.related_message_id, Some(msg.id));
    }

    #[test]
    fn test_file_transfer() {
        let mut transfer =
            FileTransfer::new("test.txt", 1000, "text/plain", "sender1", "receiver1", 100);

        assert_eq!(transfer.filename, "test.txt");
        assert_eq!(transfer.size, 1000);
        assert_eq!(transfer.total_chunks, 10);
        assert_eq!(transfer.status, TransferStatus::Initializing);

        // Update progress
        let progress = transfer.update_progress(5);
        assert_eq!(transfer.status, TransferStatus::InProgress);
        assert_eq!(progress.bytes_transferred, 500);
        assert_eq!(progress.progress_percentage(), 50.0);

        // Complete transfer
        let _progress = transfer.update_progress(10);
        assert_eq!(transfer.status, TransferStatus::Completed);
        assert!(transfer.end_time.is_some());

        // Transfer duration should be available
        assert!(transfer.transfer_duration().is_some());
    }

    #[test]
    fn test_notification_creation() {
        let notification = Notification::new(
            NotificationType::TextReceived,
            "新消息",
            "您收到了一条新消息",
            NotificationPriority::Normal,
        )
        .with_device_id("device1")
        .with_action(NotificationAction::accept("接受"))
        .with_action(NotificationAction::reject("拒绝"));

        assert_eq!(
            notification.notification_type,
            NotificationType::TextReceived
        );
        assert_eq!(notification.title, "新消息");
        assert_eq!(notification.device_id, Some("device1".to_string()));
        assert_eq!(notification.actions.len(), 2);
        assert!(!notification.is_read);

        // Test action creation
        let open_action = NotificationAction::open_file("打开文件", "/tmp/file.txt");
        assert!(matches!(
            open_action.action_type,
            NotificationActionType::OpenFile { .. }
        ));
    }

    #[test]
    fn test_device_capabilities() {
        let caps = DeviceCapabilities::default();
        assert!(caps.supports_files);
        assert!(caps.supports_wifi_direct);
        assert_eq!(caps.max_file_size, 1024);

        // Test security policy default
        let policy = SecurityPolicy::default();
        assert_eq!(policy, SecurityPolicy::AlwaysAsk);
    }
}
