//! FFI接口测试

#[cfg(test)]
mod tests {
    use pasteall_core::ffi::common::*;
    use pasteall_core::types::{Config, ConfigOptions, DeviceCapabilities, DeviceType};
    use std::ffi::{CStr, CString};
    use std::ptr;
    
    /// 创建测试配置
    fn create_test_config() -> Config {
        Config {
            device_name: "Test Device".to_string(),
            device_type: DeviceType::Desktop,
            storage_path: ":memory:".to_string(),
            discovery_port: 5678,
            listen_port: 5679,
            device_id: uuid::Uuid::new_v4().to_string(),
            capabilities: DeviceCapabilities::default(),
            options: ConfigOptions::default(),
        }
    }
    
    /// 将配置转为JSON字符串，然后转为C字符串
    fn config_to_c_string(config: &Config) -> CString {
        let json = serde_json::to_string(config).expect("Failed to serialize config");
        CString::new(json).expect("Failed to create CString")
    }
    
    #[test]
    fn test_pasteall_init() {
        // 创建测试配置
        let config = create_test_config();
        let config_c_string = config_to_c_string(&config);
        
        // 调用初始化函数
        let result = unsafe { pasteall_init(config_c_string.as_ptr()) };
        
        // 验证结果
        assert_eq!(result, 0, "Initialization should succeed");
    }
    
    #[test]
    fn test_pasteall_init_null_config() {
        // 使用空指针调用初始化函数
        let result = unsafe { pasteall_init(ptr::null()) };
        
        // 验证结果
        assert_eq!(result, 1, "Initialization with null pointer should fail with ERROR_INVALID_PARAMETER");
    }
    
    #[test]
    fn test_pasteall_register_callbacks() {
        // 创建并初始化测试配置
        let config = create_test_config();
        let config_c_string = config_to_c_string(&config);
        let _ = unsafe { pasteall_init(config_c_string.as_ptr()) };
        
        // 测试回调函数
        extern "C" fn test_device_discovery_callback(_device_json: *const std::os::raw::c_char) {}
        extern "C" fn test_clipboard_content_callback(_content_type: i32, _content_json: *const std::os::raw::c_char) {}
        extern "C" fn test_pairing_status_callback(_device_json: *const std::os::raw::c_char, _status: i32) {}
        extern "C" fn test_transfer_progress_callback(_device_id: *const std::os::raw::c_char, _file_path: *const std::os::raw::c_char, _progress: f32) {}
        extern "C" fn test_error_callback(_error_code: i32, _error_msg: *const std::os::raw::c_char) {}
        
        // 注册回调函数
        let device_result = unsafe { pasteall_register_device_discovery_callback(test_device_discovery_callback) };
        let clipboard_result = unsafe { pasteall_register_clipboard_content_callback(test_clipboard_content_callback) };
        let pairing_result = unsafe { pasteall_register_pairing_status_callback(test_pairing_status_callback) };
        let transfer_result = unsafe { pasteall_register_transfer_progress_callback(test_transfer_progress_callback) };
        let error_result = unsafe { pasteall_register_error_callback(test_error_callback) };
        
        // 验证结果
        assert_eq!(device_result, 0, "Device discovery callback registration should succeed");
        assert_eq!(clipboard_result, 0, "Clipboard content callback registration should succeed");
        assert_eq!(pairing_result, 0, "Pairing status callback registration should succeed");
        assert_eq!(transfer_result, 0, "Transfer progress callback registration should succeed");
        assert_eq!(error_result, 0, "Error callback registration should succeed");
    }
    
    #[test]
    fn test_pasteall_set_text_to_clipboard() {
        // 创建测试文本
        let test_text = "测试文本";
        let text_c_string = CString::new(test_text).expect("Failed to create CString");
        
        // 调用设置剪贴板文本函数
        // 注意：这个测试在没有剪贴板的环境中可能会失败
        #[cfg(not(feature = "ci_tests"))]
        {
            let result = unsafe { pasteall_set_text_to_clipboard(text_c_string.as_ptr()) };
            // 验证结果
            assert_eq!(result, 0, "Setting text to clipboard should succeed");
        }
    }
    
    #[test]
    fn test_pasteall_get_config() {
        // 创建并初始化测试配置
        let config = create_test_config();
        let config_c_string = config_to_c_string(&config);
        let _ = unsafe { pasteall_init(config_c_string.as_ptr()) };
        
        // 获取配置
        let buffer = unsafe { pasteall_get_config() };
        
        // 验证结果
        assert!(buffer.len() > 0, "Config buffer should not be empty");
        
        // 将ByteBuffer转为字符串
        let config_json = unsafe { CStr::from_ptr(buffer.data) }
            .to_str()
            .expect("Failed to convert buffer to string");
            
        // 解析JSON
        let parsed_config: Config = serde_json::from_str(config_json).expect("Failed to parse config JSON");
        
        // 验证配置
        assert_eq!(parsed_config.device_name, config.device_name, "Device name should match");
        assert_eq!(parsed_config.device_type, config.device_type, "Device type should match");
        
        // 释放ByteBuffer
        unsafe { pasteall_string_free(buffer) };
    }
}
