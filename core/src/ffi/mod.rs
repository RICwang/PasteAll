//! FFI模块，负责与各平台集成

#[cfg(feature = "android-integration")]
pub mod android;

#[cfg(feature = "ios-integration")]
pub mod ios;

/// 通用FFI接口
pub mod common;
