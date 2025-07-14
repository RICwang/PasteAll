//! 错误处理模块，定义了PasteAll核心库中使用的所有错误类型

use thiserror::Error;

/// PasteAll核心库的错误类型
#[derive(Error, Debug)]
pub enum Error {
    /// 初始化错误
    #[error("初始化错误: {0}")]
    Initialization(String),

    /// 剪贴板操作错误
    #[error("剪贴板错误: {0}")]
    Clipboard(String),

    /// 网络错误
    #[error("网络错误: {0}")]
    Network(String),

    /// 加密错误
    #[error("加密错误: {0}")]
    Crypto(String),

    /// 存储错误
    #[error("存储错误: {0}")]
    Storage(String),

    /// 设备发现错误
    #[error("设备发现错误: {0}")]
    Discovery(String),

    /// 设备配对错误
    #[error("设备配对错误: {0}")]
    Pairing(String),

    /// 数据传输错误
    #[error("数据传输错误: {0}")]
    Transfer(String),

    /// 认证错误
    #[error("认证错误: {0}")]
    Authentication(String),

    /// 权限错误
    #[error("权限错误: {0}")]
    Permission(String),

    /// IO错误
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),

    /// 序列化/反序列化错误
    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    /// 数据库错误
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    /// 配置错误
    #[error("配置错误: {0}")]
    Configuration(String),

    /// 无效参数
    #[error("无效参数: {0}")]
    InvalidArgument(String),

    /// 连接错误
    #[error("连接错误: {0}")]
    Connection(String),

    /// 文件错误
    #[error("文件错误: {0}")]
    File(String),

    /// 通知错误
    #[error("通知错误: {0}")]
    Notification(String),

    /// 超时错误
    #[error("超时错误: {0}")]
    Timeout(String),

    /// 未知错误
    #[error("未知错误: {0}")]
    Unknown(String),
}

/// Result类型别名，用于简化错误处理
pub type Result<T> = std::result::Result<T, Error>;

/// 从字符串创建错误
pub fn from_str(s: &str, error_type: &str) -> Error {
    match error_type {
        "clipboard" => Error::Clipboard(s.to_string()),
        "network" => Error::Network(s.to_string()),
        "crypto" => Error::Crypto(s.to_string()),
        "storage" => Error::Storage(s.to_string()),
        "discovery" => Error::Discovery(s.to_string()),
        "pairing" => Error::Pairing(s.to_string()),
        "transfer" => Error::Transfer(s.to_string()),
        "authentication" => Error::Authentication(s.to_string()),
        "permission" => Error::Permission(s.to_string()),
        "initialization" => Error::Initialization(s.to_string()),
        "configuration" => Error::Configuration(s.to_string()),
        "invalid_argument" => Error::InvalidArgument(s.to_string()),
        "connection" => Error::Connection(s.to_string()),
        "file" => Error::File(s.to_string()),
        "notification" => Error::Notification(s.to_string()),
        "timeout" => Error::Timeout(s.to_string()),
        _ => Error::Unknown(s.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_from_str() {
        let err = from_str("测试错误", "clipboard");
        assert!(matches!(err, Error::Clipboard(_)));

        let err = from_str("测试错误", "unknown_type");
        assert!(matches!(err, Error::Unknown(_)));
    }
}
