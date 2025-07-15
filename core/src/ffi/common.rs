//! 通用FFI接口
//!
//! 本模块定义了与各平台应用程序交互的通用FFI接口。
//! 主要功能包括：
//! - 核心库初始化和配置
//! - 设备发现与配对
//! - 剪贴板内容传输
//! - 回调函数注册
//!
//! 所有功能都通过C风格接口导出，确保跨语言兼容性。

use std::ffi::{c_char, CStr, CString};
use std::sync::Mutex;

use ffi_support::ByteBuffer;
use once_cell::sync::Lazy;
use log::error;

use crate::error::{Error, Result};
use crate::types::Config;
use crate::clipboard;
use crate::PasteAll;

// 定义错误码
const ERROR_SUCCESS: i32 = 0;
const ERROR_INVALID_PARAMETER: i32 = 1;
const ERROR_INIT_FAILED: i32 = 2;
const ERROR_CLIPBOARD: i32 = 3;
const ERROR_NETWORK: i32 = 4;
const ERROR_CRYPTO: i32 = 5;
const ERROR_STORAGE: i32 = 6;
#[allow(dead_code)]
const ERROR_PAIRING: i32 = 7;
const ERROR_UNKNOWN: i32 = 100;

// FFI接口使用的回调函数类型
type DeviceDiscoveryCallback = extern "C" fn(device_json: *const c_char);
type ClipboardContentCallback = extern "C" fn(content_type: i32, content_json: *const c_char);
type PairingStatusCallback = extern "C" fn(device_json: *const c_char, status: i32);
type TransferProgressCallback = extern "C" fn(device_id: *const c_char, file_path: *const c_char, progress: f32);
type ErrorCallback = extern "C" fn(error_code: i32, error_msg: *const c_char);

// 全局PasteAll实例
static INSTANCE: Lazy<Mutex<Option<PasteAll>>> = Lazy::new(|| Mutex::new(None));

// 全局回调函数
static DEVICE_DISCOVERY_CALLBACK: Lazy<Mutex<Option<DeviceDiscoveryCallback>>> = Lazy::new(|| Mutex::new(None));
static CLIPBOARD_CONTENT_CALLBACK: Lazy<Mutex<Option<ClipboardContentCallback>>> = Lazy::new(|| Mutex::new(None));
static PAIRING_STATUS_CALLBACK: Lazy<Mutex<Option<PairingStatusCallback>>> = Lazy::new(|| Mutex::new(None));
static TRANSFER_PROGRESS_CALLBACK: Lazy<Mutex<Option<TransferProgressCallback>>> = Lazy::new(|| Mutex::new(None));
static ERROR_CALLBACK: Lazy<Mutex<Option<ErrorCallback>>> = Lazy::new(|| Mutex::new(None));

// 错误码和错误信息映射
fn map_error_code(error: &Error) -> i32 {
    match error {
        Error::InvalidArgument(_) => ERROR_INVALID_PARAMETER,
        Error::Initialization(_) => ERROR_INIT_FAILED,
        Error::Clipboard(_) => ERROR_CLIPBOARD,
        Error::Network(_) => ERROR_NETWORK,
        Error::Crypto(_) => ERROR_CRYPTO,
        Error::Database(_) => ERROR_STORAGE,
        Error::Serialization(_) => ERROR_UNKNOWN,
        Error::Io(_) => ERROR_UNKNOWN,
        Error::Storage(_) => ERROR_STORAGE,
        Error::Unknown(_) => ERROR_UNKNOWN,
        _ => ERROR_UNKNOWN,
    }
}

// ByteBuffer结构体析构器
ffi_support::define_string_destructor!(pasteall_string_free);

/// ByteBuffer返回值帮助函数
#[allow(dead_code)]
fn error_to_byte_buffer(error: &Error) -> ByteBuffer {
    let error_string = error.to_string();
    ByteBuffer::from_vec(error_string.into_bytes())
}

/// 将Result转为i32状态码
fn result_to_status_code<T>(result: crate::error::Result<T>) -> i32 {
    match result {
        Ok(_) => ERROR_SUCCESS,
        Err(e) => {
            let code = map_error_code(&e);
            if let Some(callback) = *ERROR_CALLBACK.lock().unwrap() {
                let error_string = CString::new(e.to_string()).unwrap_or_default();
                callback(code, error_string.as_ptr());
            }
            code
        }
    }
}

/// 将JSON字符串转为ByteBuffer
fn json_to_buffer<T: serde::Serialize>(value: &T) -> ByteBuffer {
    match serde_json::to_string(value) {
        Ok(json) => ByteBuffer::from_vec(json.into_bytes()),
        Err(e) => {
            error!("序列化JSON失败: {}", e);
            ByteBuffer::new_with_size(0)
        }
    }
}

/// 从字符串指针创建Rust字符串
unsafe fn cstr_to_string(ptr: *const c_char) -> Result<String> {
    if ptr.is_null() {
        return Err(Error::InvalidArgument("指针为空".to_string()));
    }

    CStr::from_ptr(ptr)
        .to_str()
        .map(|s| s.to_owned())
        .map_err(|e| Error::InvalidArgument(format!("无效的UTF-8字符串: {}", e)))
}

/// 从JSON字符串解析到指定类型
unsafe fn parse_json<T: for<'de> serde::Deserialize<'de>>(json_ptr: *const c_char) -> Result<T> {
    let json_str = cstr_to_string(json_ptr)?;
    serde_json::from_str::<T>(&json_str)
        .map_err(|e| Error::InvalidArgument(format!("无效的JSON格式: {}", e)))
}

#[no_mangle]
/// 初始化PasteAll核心库
///
/// # 参数
///
/// * `config_json` - 包含配置信息的JSON字符串
///
/// # 返回
///
/// * `i32` - 错误码，0表示成功
pub extern "C" fn pasteall_init(config_json: *const c_char) -> i32 {
    let result = unsafe {
        if config_json.is_null() {
            return ERROR_INVALID_PARAMETER;
        }
        
        // 解析配置
        let config: Config = match parse_json(config_json) {
            Ok(config) => config,
            Err(e) => {
                error!("解析配置失败: {}", e);
                return ERROR_INVALID_PARAMETER;
            }
        };
        
        // 初始化日志
        crate::init_logger();
        
        // 创建PasteAll实例
        let pasteall = PasteAll::new(config);
        
        // 存储到全局变量
        match INSTANCE.lock() {
            Ok(mut instance) => {
                *instance = Some(pasteall);
                Ok(())
            },
            Err(e) => {
                error!("获取实例锁失败: {}", e);
                Err(Error::Initialization("获取实例锁失败".to_string()))
            }
        }
    };
    
    result_to_status_code(result)
}

#[no_mangle]
/// 启动PasteAll服务
///
/// # 返回
///
/// * `i32` - 错误码，0表示成功
pub extern "C" fn pasteall_start() -> i32 {
    let _result = async move {
        let instance = match INSTANCE.lock() {
            Ok(instance) => instance,
            Err(e) => {
                error!("获取实例锁失败: {}", e);
                return Err(Error::Initialization("获取实例锁失败".to_string()));
            }
        };
        
        let pasteall = match &*instance {
            Some(pasteall) => pasteall,
            None => return Err(Error::Initialization("PasteAll未初始化".to_string())),
        };
        
        pasteall.start().await
    };
    
    // 在实际应用中，这里需要使用适当的方式执行异步代码
    // 例如在JavaScript/TypeScript中可以返回Promise，在Dart中可以使用Future
    // 这里简化处理，返回成功
    ERROR_SUCCESS
}

#[no_mangle]
/// 停止PasteAll服务
///
/// # 返回
///
/// * `i32` - 错误码，0表示成功
pub extern "C" fn pasteall_stop() -> i32 {
    let _result = async move {
        let instance = match INSTANCE.lock() {
            Ok(instance) => instance,
            Err(e) => {
                error!("获取实例锁失败: {}", e);
                return Err(Error::Initialization("获取实例锁失败".to_string()));
            }
        };
        
        let pasteall = match &*instance {
            Some(pasteall) => pasteall,
            None => return Err(Error::Initialization("PasteAll未初始化".to_string())),
        };
        
        pasteall.stop().await
    };
    
    // 同上，简化处理
    ERROR_SUCCESS
}

#[no_mangle]
/// 注册设备发现回调函数
///
/// # 参数
///
/// * `callback` - 设备发现回调函数
///
/// # 返回
///
/// * `i32` - 错误码，0表示成功
pub extern "C" fn pasteall_register_device_discovery_callback(callback: DeviceDiscoveryCallback) -> i32 {
    let result = match DEVICE_DISCOVERY_CALLBACK.lock() {
        Ok(mut cb) => {
            *cb = Some(callback);
            Ok(())
        },
        Err(e) => {
            error!("获取回调函数锁失败: {}", e);
            Err(Error::Initialization("获取回调函数锁失败".to_string()))
        }
    };
    
    result_to_status_code(result)
}

#[no_mangle]
/// 注册剪贴板内容变更回调函数
///
/// # 参数
///
/// * `callback` - 剪贴板内容变更回调函数
///
/// # 返回
///
/// * `i32` - 错误码，0表示成功
pub extern "C" fn pasteall_register_clipboard_content_callback(callback: ClipboardContentCallback) -> i32 {
    let result = match CLIPBOARD_CONTENT_CALLBACK.lock() {
        Ok(mut cb) => {
            *cb = Some(callback);
            Ok(())
        },
        Err(e) => {
            error!("获取回调函数锁失败: {}", e);
            Err(Error::Initialization("获取回调函数锁失败".to_string()))
        }
    };
    
    result_to_status_code(result)
}

#[no_mangle]
/// 注册配对状态变更回调函数
///
/// # 参数
///
/// * `callback` - 配对状态变更回调函数
///
/// # 返回
///
/// * `i32` - 错误码，0表示成功
pub extern "C" fn pasteall_register_pairing_status_callback(callback: PairingStatusCallback) -> i32 {
    let result = match PAIRING_STATUS_CALLBACK.lock() {
        Ok(mut cb) => {
            *cb = Some(callback);
            Ok(())
        },
        Err(e) => {
            error!("获取回调函数锁失败: {}", e);
            Err(Error::Initialization("获取回调函数锁失败".to_string()))
        }
    };
    
    result_to_status_code(result)
}

#[no_mangle]
/// 注册传输进度回调函数
///
/// # 参数
///
/// * `callback` - 传输进度回调函数
///
/// # 返回
///
/// * `i32` - 错误码，0表示成功
pub extern "C" fn pasteall_register_transfer_progress_callback(callback: TransferProgressCallback) -> i32 {
    let result = match TRANSFER_PROGRESS_CALLBACK.lock() {
        Ok(mut cb) => {
            *cb = Some(callback);
            Ok(())
        },
        Err(e) => {
            error!("获取回调函数锁失败: {}", e);
            Err(Error::Initialization("获取回调函数锁失败".to_string()))
        }
    };
    
    result_to_status_code(result)
}

#[no_mangle]
/// 注册错误回调函数
///
/// # 参数
///
/// * `callback` - 错误回调函数
///
/// # 返回
///
/// * `i32` - 错误码，0表示成功
pub extern "C" fn pasteall_register_error_callback(callback: ErrorCallback) -> i32 {
    let result = match ERROR_CALLBACK.lock() {
        Ok(mut cb) => {
            *cb = Some(callback);
            Ok(())
        },
        Err(e) => {
            error!("获取回调函数锁失败: {}", e);
            Err(Error::Initialization("获取回调函数锁失败".to_string()))
        }
    };
    
    result_to_status_code(result)
}

#[no_mangle]
/// 获取设备列表
///
/// # 返回
///
/// * `ByteBuffer` - 包含设备列表的JSON字符串
pub extern "C" fn pasteall_get_devices() -> ByteBuffer {
    // TODO: 实现设备列表获取
    ByteBuffer::new_with_size(0)
}

#[no_mangle]
/// 开始配对流程
///
/// # 参数
///
/// * `pin_enabled` - 是否启用PIN码验证
///
/// # 返回
///
/// * `ByteBuffer` - 包含配对信息的JSON字符串，如QR码内容
pub extern "C" fn pasteall_start_pairing(_pin_enabled: bool) -> ByteBuffer {
    // TODO: 实现配对流程
    ByteBuffer::new_with_size(0)
}

#[no_mangle]
/// 验证配对PIN码
///
/// # 参数
///
/// * `device_id` - 设备ID
/// * `pin` - PIN码
///
/// # 返回
///
/// * `i32` - 错误码，0表示成功
pub extern "C" fn pasteall_verify_pin(_device_id: *const c_char, _pin: *const c_char) -> i32 {
    // TODO: 实现PIN码验证
    ERROR_SUCCESS
}

#[no_mangle]
/// 发送剪贴板内容到指定设备
///
/// # 参数
///
/// * `device_id` - 目标设备ID
///
/// # 返回
///
/// * `i32` - 错误码，0表示成功
pub extern "C" fn pasteall_send_clipboard_content(_device_id: *const c_char) -> i32 {
    // TODO: 实现剪贴板内容发送
    ERROR_SUCCESS
}

#[no_mangle]
/// 设置文本到剪贴板
///
/// # 参数
///
/// * `text` - 文本内容
///
/// # 返回
///
/// * `i32` - 错误码，0表示成功
pub extern "C" fn pasteall_set_text_to_clipboard(text: *const c_char) -> i32 {
    let result = unsafe {
        match cstr_to_string(text) {
            Ok(text_str) => {
                match clipboard::Clipboard::new() {
                    Ok(mut clipboard) => clipboard.set_text(&text_str),
                    Err(e) => Err(e)
                }
            },
            Err(e) => Err(e)
        }
    };
    
    result_to_status_code(result)
}

#[no_mangle]
/// 设置文件路径到剪贴板
///
/// # 参数
///
/// * `paths_json` - 文件路径数组的JSON字符串
///
/// # 返回
///
/// * `i32` - 错误码，0表示成功
pub extern "C" fn pasteall_set_files_to_clipboard(paths_json: *const c_char) -> i32 {
    let result = unsafe {
        match cstr_to_string(paths_json) {
            Ok(paths_str) => {
                match serde_json::from_str::<Vec<String>>(&paths_str)
                    .map_err(|e| Error::InvalidArgument(format!("无效的JSON格式: {}", e))) {
                    Ok(paths) => {
                        match clipboard::Clipboard::new() {
                            Ok(mut clipboard) => clipboard.set_file_paths(&paths),
                            Err(e) => Err(e)
                        }
                    },
                    Err(e) => Err(e)
                }
            },
            Err(e) => Err(e)
        }
    };
    
    result_to_status_code(result)
}

#[no_mangle]
/// 获取当前剪贴板内容
///
/// # 返回
///
/// * `ByteBuffer` - 包含剪贴板内容的JSON字符串
pub extern "C" fn pasteall_get_clipboard_content() -> ByteBuffer {
    // TODO: 实现剪贴板内容获取
    ByteBuffer::new_with_size(0)
}

#[no_mangle]
/// 获取配置
///
/// # 返回
///
/// * `ByteBuffer` - 包含配置的JSON字符串
pub extern "C" fn pasteall_get_config() -> ByteBuffer {
    let result = match INSTANCE.lock() {
        Ok(instance) => {
            match &*instance {
                Some(pasteall) => {
                    Ok(json_to_buffer(&pasteall.config))
                },
                None => Err(Error::Initialization("PasteAll未初始化".to_string())),
            }
        },
        Err(e) => {
            error!("获取实例锁失败: {}", e);
            Err(Error::Initialization("获取实例锁失败".to_string()))
        }
    };
    
    match result {
        Ok(buffer) => buffer,
        Err(e) => {
            error!("获取配置失败: {}", e);
            ByteBuffer::new_with_size(0)
        }
    }
}

#[no_mangle]
/// 更新配置
///
/// # 参数
///
/// * `config_json` - 包含新配置的JSON字符串
///
/// # 返回
///
/// * `i32` - 错误码，0表示成功
pub extern "C" fn pasteall_update_config(_config_json: *const c_char) -> i32 {
    // TODO: 实现配置更新
    ERROR_SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    
    #[test]
    fn test_cstr_to_string() {
        let text = "测试文本";
        let c_string = CString::new(text).unwrap();
        let result = unsafe { cstr_to_string(c_string.as_ptr()) };
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), text);
    }
    
    #[test]
    fn test_json_to_buffer() {
        let device = DeviceInfo::new("测试设备", DeviceType::Desktop, "dummy_key");
        let buffer = json_to_buffer(&device);
        assert!(buffer.len() > 0);
    }
}

