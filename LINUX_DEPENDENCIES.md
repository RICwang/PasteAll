# Linux平台依赖说明

在Linux平台上运行PasteAll需要安装以下依赖：

## 必要依赖

- pkg-config - 用于编译过程中查找库
- libdbus-1-dev - DBus开发库 (如果使用linux-clipboard特性)
- xclip - 命令行剪贴板工具 (如果使用linux-clipboard特性)

## 安装依赖

### Ubuntu/Debian系统

```bash
sudo apt update
sudo apt install pkg-config libdbus-1-dev xclip
```

### Fedora/RHEL/CentOS系统

```bash
sudo dnf install pkgconf-pkg-config dbus-devel xclip
# 或者在较旧的系统上使用:
# sudo yum install pkgconf-pkg-config dbus-devel xclip
```

### Arch Linux

```bash
sudo pacman -S pkg-config libdbus xclip
```

## 配置

如果不想安装这些依赖，可以在不使用linux-clipboard特性的情况下构建：

```bash
cargo build --no-default-features --features="clipboard-watcher,device-discovery"
```

## CI环境

在CI环境中，如果是Linux系统，需要确保安装了上述依赖。可以参考项目中的`.github/workflows/ci.yml`文件来了解如何在GitHub Actions中安装这些依赖。
