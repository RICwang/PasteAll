# PasteAll - 跨平台近距离设备复制粘贴工具

PasteAll是一个跨平台应用程序，支持Windows、macOS桌面端和iOS、Android移动端，实现近距离（10米内）设备间的文件和文本内容无缝复制粘贴。

## 项目概述

本应用提供直观的用户界面，确保首次配对认证后自动同步，无需重复验证。采用多层次的安全机制，确保数据传输安全可靠。

## 技术架构

- **核心功能**：使用Rust编写，通过FFI与各平台集成
- **桌面端**：使用Electron框架（支持Windows/macOS）
- **移动端**：使用Flutter框架（支持iOS/Android）
- **近距离通信**：蓝牙低功耗(BLE)用于设备发现，Wi-Fi直连/点对点Wi-Fi用于数据传输
- **加密与安全**：端到端加密，使用NaCl库（libsodium）

## 功能特性

- 自动监听系统剪贴板，捕获文本和文件复制事件
- 检测附近已配对设备并显示在列表中
- 一键将复制内容推送到选定设备
- 接收端自动或手动将内容写入剪贴板
- 设备发现与配对界面，支持配对设备管理
- 首次配对采用QR码扫描+PIN码验证，后续自动连接

## 项目结构

```plaintext
core/               - 核心功能实现
  clipboard/        - 剪贴板操作抽象
  crypto/           - 加密与认证
  network/          - 网络通信协议
  storage/          - 数据存储
desktop/            - 桌面端实现
  windows/          - Windows 特定实现
  macos/            - macOS 特定实现
mobile/             - 移动端实现
  android/          - Android 特定实现
  ios/              - iOS 特定实现
ui/                 - 用户界面实现
  components/       - 共享 UI 组件
  screens/          - 应用界面
```

## 当前开发状态

- [x] 项目基础结构搭建
- [x] 核心库基础代码框架
- [x] 基本数据类型和错误处理
- [x] 核心库编译通过
- [x] 剪贴板监听完善（多平台支持）
- [x] BLE设备发现与连接
- [x] 设备配对与认证流程
- [x] CI/CD管道配置
- [ ] Wi-Fi数据传输（基本功能已实现，需优化）
- [ ] 桌面端UI实现（基础框架已完成）
- [ ] 移动端UI实现（基础框架已完成）
- [ ] FFI接口完善（基本接口已实现，需优化）
- [ ] 单元测试和集成测试（部分实现）
- [ ] 首个可用版本发布

## 开发环境设置

### 安装必要工具

1. 安装Rust工具链

    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```

2. 安装Node.js和npm（用于Electron开发）

    ```bash
    # 使用nvm安装Node.js
    curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
    nvm install 16
    ```

3. 安装Flutter SDK

    ```bash
    # 请访问 https://flutter.dev/docs/get-started/install 获取安装说明
    ```

### 构建与运行

#### 核心库

```bash
cd core
cargo build
```

#### 桌面客户端

```bash
cd desktop
npm install
npm start
```

#### 移动应用

```bash
cd mobile
flutter pub get
flutter run
```

### 平台特定依赖

#### Linux

在Linux平台上运行PasteAll需要安装额外的依赖，详见 [LINUX_DEPENDENCIES.md](./LINUX_DEPENDENCIES.md)。

### 特殊构建选项

#### CI环境

在持续集成(CI)环境中构建时，可以使用ci特性避免对系统库的依赖：

```bash
cd core
cargo build --features="ci"
```

#### 禁用特定平台功能

可以通过特性标记控制启用哪些平台特定功能：

```bash
# 不启用Linux剪贴板功能
cargo build --no-default-features --features="clipboard-watcher,device-discovery,windows-clipboard"

# 只启用基本发现功能
cargo build --no-default-features --features="device-discovery"
```

详细的特性配置请参考`Cargo.toml`。

## 贡献指南

欢迎提交Pull Request或Issue来帮助改进项目。详细的贡献流程和代码规范请参考 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 开发计划

1. **第一阶段**：完成核心功能库，包括基本通信、剪贴板操作和加密模块
2. **第二阶段**：开发桌面端和移动端的基本界面，实现设备发现和配对
3. **第三阶段**：完成跨平台文本和文件传输功能
4. **第四阶段**：优化用户体验、性能调优和增加附加功能
5. **第五阶段**：发布测试版并收集反馈

## 许可证

[MIT License](LICENSE)
