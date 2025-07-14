/**
 * 预加载脚本，负责创建主进程与渲染进程之间的通信桥梁
 * 在这里暴露安全的API给渲染进程
 */

const { contextBridge, ipcRenderer } = require('electron');

// 暴露安全的API给渲染进程
contextBridge.exposeInMainWorld('pasteall', {
  // 获取设备列表
  getDevices: () => ipcRenderer.invoke('get-devices'),
  
  // 发送内容到设备
  sendToDevice: (deviceId, contentType, content) =>
    ipcRenderer.invoke('send-to-device', deviceId, contentType, content),
  
  // 开始设备配对
  startPairing: () => ipcRenderer.invoke('start-pairing'),
  
  // 重命名设备
  renameDevice: (deviceId, newName) =>
    ipcRenderer.invoke('rename-device', deviceId, newName),
  
  // 删除设备
  deleteDevice: (deviceId) => ipcRenderer.invoke('delete-device', deviceId),
  
  // 获取本机设备信息
  getDeviceInfo: () => ipcRenderer.invoke('get-device-info'),
  
  // 保存设置
  saveSettings: (settings) => ipcRenderer.invoke('save-settings', settings),
  
  // 获取设置
  getSettings: () => ipcRenderer.invoke('get-settings'),
  
  // 事件监听
  onDevicesUpdated: (callback) => 
    ipcRenderer.on('devices-updated', (_, devices) => callback(devices)),
  
  onClipboardChanged: (callback) =>
    ipcRenderer.on('clipboard-changed', (_, content) => callback(content)),
  
  // 移除事件监听器
  removeAllListeners: () => {
    ipcRenderer.removeAllListeners('devices-updated');
    ipcRenderer.removeAllListeners('clipboard-changed');
  }
});
