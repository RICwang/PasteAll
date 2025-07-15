#!/bin/bash
# 修复PasteAll项目的编译错误和Clippy警告

# 1. 格式化字符串警告修复 - uninlined_format_args

# 剪贴板模块
find ./core/src/clipboard -name "*.rs" -exec sed -i '' 's/error!("获取剪贴板锁失败: {:?}", e);/error!("获取剪贴板锁失败: {e:?}");/g' {} \;
find ./core/src/clipboard -name "*.rs" -exec sed -i '' 's/error!("设置剪贴板文本失败: {:?}", e);/error!("设置剪贴板文本失败: {e:?}");/g' {} \;
find ./core/src/clipboard -name "*.rs" -exec sed -i '' 's/error!("设置剪贴板文件路径失败: {:?}", e);/error!("设置剪贴板文件路径失败: {e:?}");/g' {} \;
find ./core/src/clipboard -name "*.rs" -exec sed -i '' 's/error!("获取上次内容锁失败: {:?}", e);/error!("获取上次内容锁失败: {e:?}");/g' {} \;

# 加密模块
find ./core/src/crypto -name "*.rs" -exec sed -i '' 's/error!("初始化sodiumoxide失败: {:?}", e);/error!("初始化sodiumoxide失败: {e:?}");/g' {} \;
find ./core/src/crypto -name "*.rs" -exec sed -i '' 's/format!("解析公钥Base64编码失败: {}", e)/format!("解析公钥Base64编码失败: {e}")/g' {} \;
find ./core/src/crypto -name "*.rs" -exec sed -i '' 's/error!("获取共享密钥锁失败: {:?}", e);/error!("获取共享密钥锁失败: {e:?}");/g' {} \;
find ./core/src/crypto -name "*.rs" -exec sed -i '' 's/format!("未找到设备的共享密钥: {}", device_id)/format!("未找到设备的共享密钥: {device_id}")/g' {} \;

# 网络发现模块
find ./core/src/network -name "discovery.rs" -exec sed -i '' 's/error!("绑定广播套接字失败: {:?}", e);/error!("绑定广播套接字失败: {e:?}");/g' {} \;
find ./core/src/network -name "discovery.rs" -exec sed -i '' 's/error!("设置广播套接字选项失败: {:?}", e);/error!("设置广播套接字选项失败: {e:?}");/g' {} \;
find ./core/src/network -name "discovery.rs" -exec sed -i '' 's/debug!("发送广播包: {}", packet_json);/debug!("发送广播包: {packet_json}");/g' {} \;
find ./core/src/network -name "discovery.rs" -exec sed -i '' 's/error!("发送广播包失败: {:?}", e);/error!("发送广播包失败: {e:?}");/g' {} \;
find ./core/src/network -name "discovery.rs" -exec sed -i '' 's/format!("0.0.0.0:{}", listen_port)/format!("0.0.0.0:{listen_port}")/g' {} \;
find ./core/src/network -name "discovery.rs" -exec sed -i '' 's/debug!("收到数据包: {} 来自: {}", packet_str, addr);/debug!("收到数据包: {packet_str} 来自: {addr}");/g' {} \;
find ./core/src/network -name "discovery.rs" -exec sed -i '' 's/error!("获取设备列表锁失败: {:?}", e);/error!("获取设备列表锁失败: {e:?}");/g' {} \;

# 网络传输模块
find ./core/src/network -name "transport.rs" -exec sed -i '' 's/format!("0.0.0.0:{}", listen_port)/format!("0.0.0.0:{listen_port}")/g' {} \;
find ./core/src/network -name "transport.rs" -exec sed -i '' 's/error!("绑定TCP监听器失败: {:?}", e);/error!("绑定TCP监听器失败: {e:?}");/g' {} \;
find ./core/src/network -name "transport.rs" -exec sed -i '' 's/info!("TCP监听器启动在 {}", addr);/info!("TCP监听器启动在 {addr}");/g' {} \;
find ./core/src/network -name "transport.rs" -exec sed -i '' 's/info!("接受新连接: {}", addr);/info!("接受新连接: {addr}");/g' {} \;
find ./core/src/network -name "transport.rs" -exec sed -i '' 's/error!("读取数据头部失败: {:?}", e);/error!("读取数据头部失败: {e:?}");/g' {} \;
find ./core/src/network -name "transport.rs" -exec sed -i '' 's/error!("读取数据体失败: {:?}", e);/error!("读取数据体失败: {e:?}");/g' {} \;
find ./core/src/network -name "transport.rs" -exec sed -i '' 's/error!("接受连接失败: {:?}", e);/error!("接受连接失败: {e:?}");/g' {} \;
find ./core/src/network -name "transport.rs" -exec sed -i '' 's/error!("发送停止信号失败: {:?}", e);/error!("发送停止信号失败: {e:?}");/g' {} \;
find ./core/src/network -name "transport.rs" -exec sed -i '' 's/error!("连接到目标设备失败: {:?}", e);/error!("连接到目标设备失败: {e:?}");/g' {} \;
find ./core/src/network -name "transport.rs" -exec sed -i '' 's/error!("发送数据长度失败: {:?}", e);/error!("发送数据长度失败: {e:?}");/g' {} \;
find ./core/src/network -name "transport.rs" -exec sed -i '' 's/error!("发送数据体失败: {:?}", e);/error!("发送数据体失败: {e:?}");/g' {} \;

# 存储模块
find ./core/src/storage -name "*.rs" -exec sed -i '' 's/error!("打开数据库失败: {:?}", e);/error!("打开数据库失败: {e:?}");/g' {} \;
find ./core/src/storage -name "*.rs" -exec sed -i '' 's/format!("打开数据库失败: {}", e)/format!("打开数据库失败: {e}")/g' {} \;
find ./core/src/storage -name "*.rs" -exec sed -i '' 's/error!("获取数据库连接锁失败: {:?}", e);/error!("获取数据库连接锁失败: {e:?}");/g' {} \;
find ./core/src/storage -name "*.rs" -exec sed -i '' 's/error!("获取设备信息失败: {:?}", e),/error!("获取设备信息失败: {e:?}"),/g' {} \;

# 2. 不必要的return语句修复 - needless_return
find ./core/src/clipboard -name "*.rs" -exec sed -i '' 's/return Ok(ClipboardContent::Image(buffer));/Ok(ClipboardContent::Image(buffer))/g' {} \;
find ./core/src/clipboard -name "*.rs" -exec sed -i '' 's/return Ok(ClipboardContent::Empty);/Ok(ClipboardContent::Empty)/g' {} \;

# 3. 不必要的类型转换修复 - unnecessary_cast
find ./core/src/clipboard -name "*.rs" -exec sed -i '' 's/(y \* image.width + x) as usize/(y * image.width + x)/g' {} \;

# 4. clone_on_copy修复
sed -i '' 's/remote_public_key.clone()/*remote_public_key/g' ./core/src/crypto/mod.rs
find ./core/src/network -name "discovery.rs" -exec sed -i '' 's/local_device.capabilities.clone()/local_device.capabilities/g' {} \;

# 5. 测试中结构体初始化修复
cat > ./core/src/ffi/android.rs << 'EOF'
//! Android平台特定的FFI实现

use crate::error::Result;

/// 初始化Android平台特定功能
pub fn init() -> Result<()> {
    // Android平台的特定初始化代码
    // 目前是一个存根实现
    Ok(())
}
EOF

cat > ./core/src/ffi/ios.rs << 'EOF'
//! iOS平台特定的FFI实现

use crate::error::Result;

/// 初始化iOS平台特定功能
pub fn init() -> Result<()> {
    // iOS平台的特定初始化代码
    // 目前是一个存根实现
    Ok(())
}
EOF

# 运行cargo检查
cd ./core
cargo fmt
cargo check
cargo clippy

# CI构建检查 - 不同特性组合
echo "检查不同特性组合的构建情况"
cargo check --no-default-features
cargo check --features="clipboard-watcher"
cargo check --features="device-discovery"
cargo check --features="windows-clipboard"
# 在Linux下，如果使用linux-clipboard特性，确保安装了libdbus-1-dev包
if [ -f /etc/os-release ] && grep -q -i "ubuntu\|debian" /etc/os-release; then
  echo "在Ubuntu/Debian系统上检查是否安装了libdbus-1-dev"
  if ! dpkg -l | grep -q libdbus-1-dev; then
    echo "缺少libdbus-1-dev包，跳过linux-clipboard特性测试"
  else
    cargo check --features="linux-clipboard"
  fi
elif [ -f /etc/os-release ] && grep -q -i "fedora\|rhel\|centos" /etc/os-release; then
  echo "在Fedora/RHEL/CentOS系统上检查是否安装了dbus-devel"
  if ! rpm -qa | grep -q dbus-devel; then
    echo "缺少dbus-devel包，跳过linux-clipboard特性测试"
  else
    cargo check --features="linux-clipboard"
  fi
else
  # 在CI中，如果环境变量CI=true，则跳过linux-clipboard特性测试
  if [ "$CI" = "true" ]; then
    echo "在CI环境中跳过linux-clipboard特性测试"
  else
    cargo check --features="linux-clipboard"
  fi
fi

# 检查完成
cd ..

echo "修复完成，请检查是否还有其他错误"
