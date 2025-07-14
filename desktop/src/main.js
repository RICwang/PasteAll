/**
 * PasteAll桌面端主进程
 * 负责应用的生命周期管理、系统托盘集成和与Rust核心库的交互
 */

const { app, BrowserWindow, Tray, Menu, ipcMain, dialog, clipboard, shell } = require('electron');
const path = require('path');
const { PasteAllCore } = require('./core-bridge');
const Store = require('electron-store');
const log = require('./logger');

// 全局变量
let mainWindow;
let tray;
let pasteallCore;
let store;
let isQuiting = false;

/**
 * 创建主窗口
 */
function createMainWindow() {
  mainWindow = new BrowserWindow({
    width: 800,
    height: 600,
    show: false,
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      nodeIntegration: false,
      contextIsolation: true,
    },
    icon: path.join(__dirname, '../assets/icons/icon.png'),
  });

  mainWindow.loadFile(path.join(__dirname, '../renderer/index.html'));

  // 窗口准备好后显示
  mainWindow.once('ready-to-show', () => {
    mainWindow.show();
  });

  // 处理窗口关闭事件
  mainWindow.on('close', (event) => {
    if (!isQuiting) {
      event.preventDefault();
      mainWindow.hide();
      return false;
    }
    return true;
  });

  // 开发环境下打开开发者工具
  if (process.env.NODE_ENV === 'development') {
    mainWindow.webContents.openDevTools();
  }
}

/**
 * 创建系统托盘
 */
function createTray() {
  tray = new Tray(path.join(__dirname, '../assets/icons/tray.png'));
  
  const contextMenu = Menu.buildFromTemplate([
    { label: '显示PasteAll', click: () => mainWindow.show() },
    { type: 'separator' },
    { label: '退出', click: () => {
      isQuiting = true;
      app.quit();
    }},
  ]);
  
  tray.setToolTip('PasteAll - 跨平台设备复制粘贴工具');
  tray.setContextMenu(contextMenu);
  
  // 点击托盘图标显示主窗口
  tray.on('click', () => {
    if (mainWindow.isVisible()) {
      mainWindow.hide();
    } else {
      mainWindow.show();
    }
  });
}

/**
 * 初始化Rust核心库
 */
async function initCore() {
  try {
    pasteallCore = new PasteAllCore();
    
    const deviceName = store.get('deviceName') || app.getName();
    const deviceType = process.platform === 'darwin' ? 'macOS' : 'Windows';
    
    log.info(`初始化核心库，设备名称: ${deviceName}, 设备类型: ${deviceType}`);
    
    await pasteallCore.init({
      deviceName,
      deviceType: deviceType === 'macOS' ? 0 : 0, // 桌面端都是0
      storagePath: path.join(app.getPath('userData'), 'pasteall.db')
    });
    
    await pasteallCore.start();
    log.info('核心库初始化成功');
    
    // 定期检查设备状态
    setInterval(async () => {
      try {
        const devices = await pasteallCore.getDevices();
        mainWindow.webContents.send('devices-updated', devices);
      } catch (err) {
        log.error(`获取设备列表失败: ${err.message}`);
      }
    }, 5000);
    
    // 监听剪贴板变化
    pasteallCore.onClipboardChanged((content) => {
      mainWindow.webContents.send('clipboard-changed', content);
    });
    
  } catch (err) {
    log.error(`初始化核心库失败: ${err.message}`);
    dialog.showErrorBox('初始化失败', `无法初始化PasteAll核心库: ${err.message}`);
  }
}

/**
 * 设置IPC通信处理程序
 */
function setupIPC() {
  // 发送内容到设备
  ipcMain.handle('send-to-device', async (event, deviceId, contentType, content) => {
    try {
      return await pasteallCore.sendToDevice(deviceId, contentType, content);
    } catch (err) {
      log.error(`发送内容到设备失败: ${err.message}`);
      throw err;
    }
  });
  
  // 获取设备列表
  ipcMain.handle('get-devices', async () => {
    try {
      return await pasteallCore.getDevices();
    } catch (err) {
      log.error(`获取设备列表失败: ${err.message}`);
      throw err;
    }
  });
  
  // 开始设备配对
  ipcMain.handle('start-pairing', async () => {
    try {
      return await pasteallCore.startPairing();
    } catch (err) {
      log.error(`开始设备配对失败: ${err.message}`);
      throw err;
    }
  });
  
  // 更新设备名称
  ipcMain.handle('rename-device', async (event, deviceId, newName) => {
    try {
      await pasteallCore.renameDevice(deviceId, newName);
      return true;
    } catch (err) {
      log.error(`更新设备名称失败: ${err.message}`);
      throw err;
    }
  });
  
  // 删除设备
  ipcMain.handle('delete-device', async (event, deviceId) => {
    try {
      await pasteallCore.deleteDevice(deviceId);
      return true;
    } catch (err) {
      log.error(`删除设备失败: ${err.message}`);
      throw err;
    }
  });
  
  // 获取本机设备信息
  ipcMain.handle('get-device-info', () => {
    return {
      name: store.get('deviceName') || app.getName(),
      platform: process.platform,
      version: app.getVersion()
    };
  });
  
  // 保存设置
  ipcMain.handle('save-settings', (event, settings) => {
    for (const [key, value] of Object.entries(settings)) {
      store.set(key, value);
    }
    return true;
  });
  
  // 获取设置
  ipcMain.handle('get-settings', () => {
    return {
      deviceName: store.get('deviceName') || app.getName(),
      autoStart: store.get('autoStart') || false,
      minimizeToTray: store.get('minimizeToTray') !== false, // 默认为true
      notifyOnReceive: store.get('notifyOnReceive') !== false, // 默认为true
    };
  });
}

/**
 * 应用程序启动
 */
app.whenReady().then(() => {
  // 初始化存储
  store = new Store();
  
  // 创建主窗口和托盘
  createMainWindow();
  createTray();
  
  // 设置IPC处理程序
  setupIPC();
  
  // 初始化核心库
  initCore();
  
  // macOS特定处理：当应用被激活但没有窗口时，创建一个新窗口
  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createMainWindow();
    } else {
      mainWindow.show();
    }
  });
});

/**
 * 应用程序退出前清理
 */
app.on('before-quit', async () => {
  isQuiting = true;
  
  // 停止核心库
  if (pasteallCore) {
    try {
      await pasteallCore.stop();
      log.info('核心库已停止');
    } catch (err) {
      log.error(`停止核心库失败: ${err.message}`);
    }
  }
});

/**
 * 处理所有窗口关闭事件
 */
app.on('window-all-closed', () => {
  // 在macOS上，应用程序及其菜单栏通常保持活动状态，直到用户使用Cmd+Q明确退出
  if (process.platform !== 'darwin') {
    app.quit();
  }
});
