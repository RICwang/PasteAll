/**
 * PasteAll桌面端渲染进程的主JavaScript文件
 */

// 全局状态
const state = {
  devices: [],
  selectedDeviceId: null,
  clipboardContent: null,
  clipboardContentType: null,
  settings: {
    deviceName: '',
    autoStart: false,
    minimizeToTray: true,
    notifyOnReceive: true
  }
};

// DOM元素引用
const elements = {
  // 设备相关
  devicesContainer: document.getElementById('devices-container'),
  refreshDevicesBtn: document.getElementById('refresh-devices-btn'),
  addDeviceBtn: document.getElementById('add-device-btn'),
  startPairingBtn: document.getElementById('start-pairing-btn'),
  
  // 剪贴板相关
  clipboardPreview: document.getElementById('clipboard-preview'),
  clipboardStatus: document.getElementById('clipboard-status'),
  sendClipboardBtn: document.getElementById('send-clipboard-btn'),
  clearClipboardBtn: document.getElementById('clear-clipboard-btn'),
  
  // 设置相关
  settingsBtn: document.getElementById('settings-btn'),
  settingsModal: document.getElementById('settings-modal'),
  closeSettingsBtn: document.getElementById('close-settings-btn'),
  saveSettingsBtn: document.getElementById('save-settings-btn'),
  deviceNameInput: document.getElementById('device-name'),
  autoStartCheck: document.getElementById('auto-start'),
  minimizeToTrayCheck: document.getElementById('minimize-to-tray'),
  notifyOnReceiveCheck: document.getElementById('notify-on-receive'),
  
  // 配对相关
  pairingModal: document.getElementById('pairing-modal'),
  closePairingBtn: document.getElementById('close-pairing-btn'),
  stepQrcode: document.getElementById('step-qrcode'),
  stepPin: document.getElementById('step-pin'),
  stepComplete: document.getElementById('step-complete'),
  pinCode: document.getElementById('pin-code'),
  qrcodeContainer: document.getElementById('qrcode-container'),
  confirmPinBtn: document.getElementById('confirm-pin-btn'),
  cancelPinBtn: document.getElementById('cancel-pin-btn'),
  completePairingBtn: document.getElementById('complete-pairing-btn'),
  
  // 设备菜单相关
  deviceMenu: document.getElementById('device-menu'),
  menuSend: document.getElementById('menu-send'),
  menuRename: document.getElementById('menu-rename'),
  menuDelete: document.getElementById('menu-delete')
};

// 初始化函数
async function init() {
  // 加载设置
  await loadSettings();
  
  // 加载设备列表
  await refreshDevices();
  
  // 设置事件监听器
  setupEventListeners();
  
  // 设置IPC事件监听器
  setupIPCListeners();
}

// 加载设置
async function loadSettings() {
  try {
    state.settings = await window.pasteall.getSettings();
    
    // 更新设置表单
    elements.deviceNameInput.value = state.settings.deviceName;
    elements.autoStartCheck.checked = state.settings.autoStart;
    elements.minimizeToTrayCheck.checked = state.settings.minimizeToTray;
    elements.notifyOnReceiveCheck.checked = state.settings.notifyOnReceive;
  } catch (err) {
    console.error('加载设置失败:', err);
    showErrorStatus('加载设置失败');
  }
}

// 保存设置
async function saveSettings() {
  try {
    const newSettings = {
      deviceName: elements.deviceNameInput.value,
      autoStart: elements.autoStartCheck.checked,
      minimizeToTray: elements.minimizeToTrayCheck.checked,
      notifyOnReceive: elements.notifyOnReceiveCheck.checked
    };
    
    await window.pasteall.saveSettings(newSettings);
    state.settings = newSettings;
    
    closeSettingsModal();
    showSuccessStatus('设置已保存');
  } catch (err) {
    console.error('保存设置失败:', err);
    showErrorStatus('保存设置失败');
  }
}

// 刷新设备列表
async function refreshDevices() {
  try {
    showLoadingStatus('正在加载设备...');
    
    const devices = await window.pasteall.getDevices();
    state.devices = devices;
    
    renderDevicesList();
    showReadyStatus();
  } catch (err) {
    console.error('加载设备失败:', err);
    showErrorStatus('加载设备失败');
  }
}

// 渲染设备列表
function renderDevicesList() {
  if (!state.devices || state.devices.length === 0) {
    elements.devicesContainer.innerHTML = `
      <div class="empty-state">
        <p>没有发现设备</p>
        <button id="empty-start-pairing-btn" class="btn">配对新设备</button>
      </div>
    `;
    
    // 重新绑定按钮事件
    document.getElementById('empty-start-pairing-btn').addEventListener('click', startPairing);
    
    // 禁用发送按钮
    elements.sendClipboardBtn.disabled = true;
    return;
  }
  
  let html = '';
  
  for (const device of state.devices) {
    const isSelected = device.id === state.selectedDeviceId;
    const deviceType = device.device_type === 1 ? 'mobile' : 'desktop';
    const statusClass = device.online ? '' : 'offline';
    
    html += `
      <div class="device-item ${isSelected ? 'selected' : ''}" data-device-id="${device.id}">
        <div class="device-info">
          <div class="device-icon ${deviceType} ${statusClass}">
            ${deviceType === 'mobile' ? 'M' : 'D'}
          </div>
          <div class="device-details">
            <h3>${device.name}</h3>
            <p>${deviceType === 'mobile' ? '移动设备' : '桌面设备'}</p>
          </div>
        </div>
        <span class="device-status ${statusClass}">
          ${device.online ? '在线' : '离线'}
        </span>
      </div>
    `;
  }
  
  elements.devicesContainer.innerHTML = html;
  
  // 添加设备点击事件
  const deviceItems = document.querySelectorAll('.device-item');
  deviceItems.forEach(item => {
    item.addEventListener('click', () => selectDevice(item.dataset.deviceId));
    item.addEventListener('contextmenu', (e) => showDeviceMenu(e, item.dataset.deviceId));
  });
  
  // 如果当前选中的设备不在列表中，清除选择
  if (state.selectedDeviceId && !state.devices.find(d => d.id === state.selectedDeviceId)) {
    state.selectedDeviceId = null;
    updateSendButtonState();
  }
}

// 选择设备
function selectDevice(deviceId) {
  state.selectedDeviceId = deviceId === state.selectedDeviceId ? null : deviceId;
  renderDevicesList();
  updateSendButtonState();
}

// 更新发送按钮状态
function updateSendButtonState() {
  elements.sendClipboardBtn.disabled = !state.selectedDeviceId || !state.clipboardContent;
}

// 显示设备右键菜单
function showDeviceMenu(event, deviceId) {
  event.preventDefault();
  
  // 存储当前右键菜单的设备ID
  elements.deviceMenu.dataset.deviceId = deviceId;
  
  // 定位菜单
  elements.deviceMenu.style.top = `${event.clientY}px`;
  elements.deviceMenu.style.left = `${event.clientX}px`;
  
  // 显示菜单
  elements.deviceMenu.classList.add('active');
  
  // 添加全局点击事件以关闭菜单
  document.addEventListener('click', hideDeviceMenu);
}

// 隐藏设备右键菜单
function hideDeviceMenu() {
  elements.deviceMenu.classList.remove('active');
  document.removeEventListener('click', hideDeviceMenu);
}

// 发送剪贴板内容到选中设备
async function sendClipboardContent() {
  if (!state.selectedDeviceId || !state.clipboardContent) {
    return;
  }
  
  try {
    showLoadingStatus('正在发送...');
    
    await window.pasteall.sendToDevice(
      state.selectedDeviceId,
      state.clipboardContentType,
      state.clipboardContent
    );
    
    showSuccessStatus('发送成功');
  } catch (err) {
    console.error('发送失败:', err);
    showErrorStatus('发送失败');
  }
}

// 开始设备配对
async function startPairing() {
  try {
    showPairingModal();
    
    // 这里需要实际实现配对逻辑
    // 暂时使用模拟实现
    const qrCodeUrl = 'https://api.qrserver.com/v1/create-qr-code/?size=200x200&data=pasteall:pair:mock123456';
    
    // 显示QR码
    elements.qrcodeContainer.innerHTML = `<img src="${qrCodeUrl}" alt="配对码">`;
    
    // 等待扫描（实际应用中应该由IPC事件触发）
    setTimeout(() => {
      // 显示PIN码步骤
      elements.stepQrcode.style.display = 'none';
      elements.stepPin.style.display = 'block';
      
      // 生成随机PIN码
      const pin = Math.floor(100000 + Math.random() * 900000);
      elements.pinCode.textContent = pin;
    }, 3000);
  } catch (err) {
    console.error('启动配对失败:', err);
    showErrorStatus('启动配对失败');
    closePairingModal();
  }
}

// 确认PIN码
function confirmPin() {
  // 显示完成步骤
  elements.stepPin.style.display = 'none';
  elements.stepComplete.style.display = 'block';
}

// 取消PIN码确认
function cancelPin() {
  closePairingModal();
}

// 完成配对
async function completePairing() {
  closePairingModal();
  await refreshDevices();
}

// 重命名设备
async function renameDevice(deviceId) {
  const device = state.devices.find(d => d.id === deviceId);
  if (!device) return;
  
  const newName = prompt('请输入新的设备名称:', device.name);
  if (!newName || newName === device.name) return;
  
  try {
    showLoadingStatus('正在更新...');
    
    await window.pasteall.renameDevice(deviceId, newName);
    
    // 更新本地数据
    device.name = newName;
    renderDevicesList();
    
    showSuccessStatus('重命名成功');
  } catch (err) {
    console.error('重命名失败:', err);
    showErrorStatus('重命名失败');
  }
}

// 删除设备
async function deleteDevice(deviceId) {
  const device = state.devices.find(d => d.id === deviceId);
  if (!device) return;
  
  const confirmed = confirm(`确定要删除设备 "${device.name}" 吗？`);
  if (!confirmed) return;
  
  try {
    showLoadingStatus('正在删除...');
    
    await window.pasteall.deleteDevice(deviceId);
    
    // 从本地列表移除
    state.devices = state.devices.filter(d => d.id !== deviceId);
    
    // 如果删除的是当前选中的设备，清除选择
    if (state.selectedDeviceId === deviceId) {
      state.selectedDeviceId = null;
    }
    
    renderDevicesList();
    showSuccessStatus('删除成功');
  } catch (err) {
    console.error('删除失败:', err);
    showErrorStatus('删除失败');
  }
}

// 显示设置弹窗
function showSettingsModal() {
  elements.settingsModal.classList.add('active');
}

// 关闭设置弹窗
function closeSettingsModal() {
  elements.settingsModal.classList.remove('active');
}

// 显示配对弹窗
function showPairingModal() {
  // 重置配对状态
  elements.stepQrcode.style.display = 'block';
  elements.stepPin.style.display = 'none';
  elements.stepComplete.style.display = 'none';
  
  // 显示弹窗
  elements.pairingModal.classList.add('active');
}

// 关闭配对弹窗
function closePairingModal() {
  elements.pairingModal.classList.remove('active');
}

// 显示"加载中"状态
function showLoadingStatus(message) {
  elements.clipboardStatus.textContent = message || '加载中...';
  elements.clipboardStatus.className = 'status-badge warning';
}

// 显示"就绪"状态
function showReadyStatus() {
  elements.clipboardStatus.textContent = '就绪';
  elements.clipboardStatus.className = 'status-badge';
}

// 显示"成功"状态
function showSuccessStatus(message) {
  elements.clipboardStatus.textContent = message || '成功';
  elements.clipboardStatus.className = 'status-badge';
  
  // 3秒后恢复就绪状态
  setTimeout(showReadyStatus, 3000);
}

// 显示"错误"状态
function showErrorStatus(message) {
  elements.clipboardStatus.textContent = message || '错误';
  elements.clipboardStatus.className = 'status-badge error';
  
  // 5秒后恢复就绪状态
  setTimeout(showReadyStatus, 5000);
}

// 设置事件监听器
function setupEventListeners() {
  // 设备相关
  elements.refreshDevicesBtn.addEventListener('click', refreshDevices);
  elements.addDeviceBtn.addEventListener('click', startPairing);
  elements.startPairingBtn.addEventListener('click', startPairing);
  
  // 剪贴板相关
  elements.sendClipboardBtn.addEventListener('click', sendClipboardContent);
  elements.clearClipboardBtn.addEventListener('click', () => {
    // 这里应该实现清空剪贴板的逻辑
    state.clipboardContent = null;
    state.clipboardContentType = null;
    elements.clipboardPreview.innerHTML = `
      <div class="empty-state">
        <p>当前剪贴板为空</p>
      </div>
    `;
    updateSendButtonState();
  });
  
  // 设置相关
  elements.settingsBtn.addEventListener('click', showSettingsModal);
  elements.closeSettingsBtn.addEventListener('click', closeSettingsModal);
  elements.saveSettingsBtn.addEventListener('click', saveSettings);
  
  // 配对相关
  elements.closePairingBtn.addEventListener('click', closePairingModal);
  elements.confirmPinBtn.addEventListener('click', confirmPin);
  elements.cancelPinBtn.addEventListener('click', cancelPin);
  elements.completePairingBtn.addEventListener('click', completePairing);
  
  // 设备菜单相关
  elements.menuSend.addEventListener('click', () => {
    const deviceId = elements.deviceMenu.dataset.deviceId;
    if (deviceId) {
      state.selectedDeviceId = deviceId;
      renderDevicesList();
      updateSendButtonState();
      sendClipboardContent();
    }
    hideDeviceMenu();
  });
  
  elements.menuRename.addEventListener('click', () => {
    const deviceId = elements.deviceMenu.dataset.deviceId;
    if (deviceId) {
      renameDevice(deviceId);
    }
    hideDeviceMenu();
  });
  
  elements.menuDelete.addEventListener('click', () => {
    const deviceId = elements.deviceMenu.dataset.deviceId;
    if (deviceId) {
      deleteDevice(deviceId);
    }
    hideDeviceMenu();
  });
}

// 设置IPC事件监听器
function setupIPCListeners() {
  // 监听设备列表更新
  window.pasteall.onDevicesUpdated(devices => {
    state.devices = devices;
    renderDevicesList();
  });
  
  // 监听剪贴板内容变化
  window.pasteall.onClipboardChanged(content => {
    // 更新状态
    state.clipboardContent = content.data;
    state.clipboardContentType = content.type;
    
    // 更新UI
    if (content.type === 'text') {
      elements.clipboardPreview.textContent = content.data;
    } else if (content.type === 'image') {
      elements.clipboardPreview.innerHTML = '<img src="data:image/png;base64,' + content.data + '" alt="剪贴板图片">';
    } else if (content.type === 'file') {
      elements.clipboardPreview.textContent = '文件: ' + content.name;
    } else {
      elements.clipboardPreview.textContent = '未知类型的剪贴板内容';
    }
    
    // 启用按钮
    elements.clearClipboardBtn.disabled = false;
    updateSendButtonState();
  });
}

// 清理函数
function cleanup() {
  // 移除所有事件监听器
  window.pasteall.removeAllListeners();
}

// 在页面卸载时调用清理函数
window.addEventListener('beforeunload', cleanup);

// 初始化应用
init();
