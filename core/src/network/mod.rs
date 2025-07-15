//! 网络通信模块，负责设备发现和数据传输

/// 基于UDP广播的设备发现模块
pub mod discovery;
/// 蓝牙低功耗(BLE)设备发现与连接模块
pub mod ble_discovery;
/// 设备配对与认证模块
pub mod pairing;
/// 基本传输协议相关模块
pub mod transport;
/// 高效Wi-Fi文件传输模块
pub mod wifi_transport;
