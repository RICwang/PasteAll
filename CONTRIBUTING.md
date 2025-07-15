# 贡献指南

感谢你考虑为PasteAll项目做出贡献！以下是一些指南，帮助你了解如何参与到这个项目中。

## 开发流程

1. Fork本仓库
2. 创建你的特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交你的更改 (`git commit -m '添加一些很棒的功能'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 开启一个Pull Request

## 开发环境设置

### 通用环境

1. 安装Rust工具链：[rustup.rs](https://rustup.rs/)
2. 安装Node.js（用于桌面端）：[nodejs.org](https://nodejs.org/)
3. 安装Flutter SDK（用于移动端）：[flutter.dev](https://flutter.dev/docs/get-started/install)

### 平台特定依赖

#### Linux

在Linux平台上开发需要安装额外的依赖，具体请参考 [LINUX_DEPENDENCIES.md](./LINUX_DEPENDENCIES.md)。

#### macOS

macOS平台通常无需安装额外依赖。

#### Windows

Windows平台无需安装额外依赖，所有必要的库都会通过Cargo和npm自动安装。

### CI环境

在提交代码前，请确保它通过了CI检查。你可以在本地运行：

```bash
# Rust核心库测试
cd core
cargo fmt -- --check
cargo clippy -- -D warnings
cargo test

# 桌面端测试
cd ../desktop
npm run lint
npm test
```

## 代码规范

请确保你的代码符合项目的代码规范：

### Rust代码

- 使用`cargo fmt`格式化代码
- 使用`cargo clippy`检查潜在问题
- 所有公共API必须有文档注释
- 错误处理必须明确，不允许吞噬异常

### JavaScript/TypeScript代码（桌面端）

- 使用ESLint和Prettier格式化代码
- 遵循项目中的`.eslintrc`和`.prettierrc`配置

### Dart代码（移动端）

- 使用`dart format`格式化代码
- 遵循项目中的分析规则

## 提交消息规范

我们使用[约定式提交](https://www.conventionalcommits.org/)规范：

- `feat`: 新功能
- `fix`: 修复错误
- `docs`: 仅文档更改
- `style`: 不影响代码含义的更改（空白、格式、缺少分号等）
- `refactor`: 既不修复错误也不添加功能的代码更改
- `perf`: 改进性能的代码更改
- `test`: 添加缺失的测试或更正现有测试
- `build`: 影响构建系统或外部依赖项的更改
- `ci`: 对CI配置文件和脚本的更改
- `chore`: 其他不修改src或test文件的更改

示例：`feat(network): 添加设备自动发现功能`

## 测试

- 核心功能需要编写单元测试
- 加密算法必须有专门的测试
- 多设备通信需要集成测试
- UI交互需要端到端测试

请确保你的代码通过了所有测试。

## 安全注意事项

- 密码学相关代码只使用经过验证的库（如libsodium）
- 所有网络通信必须加密
- 敏感数据（如密钥）必须安全存储
- 遵循最小权限原则

## 跨平台兼容性

- 确保代码在所有目标平台上正常工作
- 针对特定平台的代码应明确隔离
- UI设计应适应不同的屏幕尺寸和操作系统风格

## 提问和讨论

如有任何问题或想法，欢迎在Issues中提出，或者参与相关的讨论。
