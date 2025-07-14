/**
 * 与Rust核心库的桥接模块
 */

const ffi = require('ffi-napi');
const path = require('path');
const log = require('./logger');

// 错误码映射
const ERROR_CODES = {
  0: '成功',
  1: '参数错误',
  2: '初始化错误',
  3: '剪贴板错误',
  4: '网络错误',
  5: '加密错误',
  6: '存储错误',
  7: '配对错误',
  100: '未知错误',
};

/**
 * PasteAll核心库桥接类
 */
class PasteAllCore {
  constructor() {
    this.lib = null;
    this.handle = 0;
    this.clipboardCallback = null;
    
    this._loadLibrary();
  }
  
  /**
   * 加载Rust核心库
   * @private
   */
  _loadLibrary() {
    try {
      // 库路径根据平台确定
      const libraryPath = this._getLibraryPath();
      
      // 定义FFI接口
      this.lib = ffi.Library(libraryPath, {
        // 初始化库
        'pasteall_init': ['uint64', ['pointer', 'pointer']],
        // 启动服务
        'pasteall_start': ['bool', ['uint64', 'pointer']],
        // 停止服务
        'pasteall_stop': ['bool', ['uint64', 'pointer']],
        // 释放资源
        'pasteall_destroy': ['void', ['uint64']],
        // 获取版本
        'pasteall_get_version': ['pointer', []],
        // 获取设备列表
        'pasteall_get_devices': ['pointer', ['uint64', 'pointer']],
        // 发送内容到设备
        'pasteall_send_to_device': ['bool', ['uint64', 'string', 'uint8', 'pointer', 'pointer']],
        // 其他接口可以根据需要添加
      });
      
      log.info('Rust核心库加载成功');
    } catch (err) {
      log.error(`加载Rust核心库失败: ${err.message}`);
      throw new Error(`无法加载PasteAll核心库: ${err.message}`);
    }
  }
  
  /**
   * 获取库路径
   * @private
   */
  _getLibraryPath() {
    // 这里简化处理，实际应用中需要根据不同平台和开发/生产环境提供正确的路径
    const platform = process.platform;
    let libName;
    
    if (platform === 'win32') {
      libName = 'pasteall_core.dll';
    } else if (platform === 'darwin') {
      libName = 'libpasteall_core.dylib';
    } else {
      libName = 'libpasteall_core.so';
    }
    
    // 开发环境下通常库在不同位置
    const isDev = process.env.NODE_ENV === 'development';
    const libPath = isDev
      ? path.join(__dirname, '../../core/target/debug', libName)
      : path.join(__dirname, '../native', libName);
    
    log.info(`使用库路径: ${libPath}`);
    return libPath;
  }
  
  /**
   * 初始化核心库
   * @param {Object} config 配置对象
   * @param {string} config.deviceName 设备名称
   * @param {number} config.deviceType 设备类型 (0:桌面, 1:移动)
   * @param {string} config.storagePath 存储路径
   */
  async init(config) {
    return new Promise((resolve, reject) => {
      if (this.handle !== 0) {
        return reject(new Error('核心库已经初始化'));
      }
      
      // 这里需要创建与Rust对应的结构体
      // 简化实现，实际应用需要正确设置内存布局
      const configStruct = Buffer.alloc(100); // 假设足够大
      
      // 写入设备名称
      const nameBuffer = Buffer.from(config.deviceName + '\0', 'utf8');
      nameBuffer.copy(configStruct, 0);
      
      // 写入设备类型
      configStruct[50] = config.deviceType || 0;
      
      // 写入存储路径
      const pathBuffer = Buffer.from(config.storagePath + '\0', 'utf8');
      pathBuffer.copy(configStruct, 51);
      
      // 错误处理结构
      const errorStruct = Buffer.alloc(256);
      
      try {
        this.handle = this.lib.pasteall_init(configStruct, errorStruct);
        
        if (this.handle === 0) {
          const errorCode = errorStruct.readInt16LE(0);
          const errorMessage = ERROR_CODES[errorCode] || '未知错误';
          return reject(new Error(`初始化失败: ${errorMessage}`));
        }
        
        log.info(`核心库初始化成功, 句柄: ${this.handle}`);
        resolve();
      } catch (err) {
        log.error(`调用pasteall_init失败: ${err.message}`);
        reject(new Error(`初始化失败: ${err.message}`));
      }
    });
  }
  
  /**
   * 启动服务
   */
  async start() {
    return new Promise((resolve, reject) => {
      if (this.handle === 0) {
        return reject(new Error('核心库尚未初始化'));
      }
      
      const errorStruct = Buffer.alloc(256);
      
      try {
        const success = this.lib.pasteall_start(this.handle, errorStruct);
        
        if (!success) {
          const errorCode = errorStruct.readInt16LE(0);
          const errorMessage = ERROR_CODES[errorCode] || '未知错误';
          return reject(new Error(`启动失败: ${errorMessage}`));
        }
        
        log.info('核心库服务启动成功');
        resolve();
      } catch (err) {
        log.error(`调用pasteall_start失败: ${err.message}`);
        reject(new Error(`启动失败: ${err.message}`));
      }
    });
  }
  
  /**
   * 停止服务
   */
  async stop() {
    return new Promise((resolve, reject) => {
      if (this.handle === 0) {
        return resolve(); // 没有初始化，直接返回成功
      }
      
      const errorStruct = Buffer.alloc(256);
      
      try {
        const success = this.lib.pasteall_stop(this.handle, errorStruct);
        
        if (!success) {
          const errorCode = errorStruct.readInt16LE(0);
          const errorMessage = ERROR_CODES[errorCode] || '未知错误';
          return reject(new Error(`停止失败: ${errorMessage}`));
        }
        
        // 释放资源
        this.lib.pasteall_destroy(this.handle);
        this.handle = 0;
        
        log.info('核心库服务停止成功');
        resolve();
      } catch (err) {
        log.error(`调用pasteall_stop失败: ${err.message}`);
        reject(new Error(`停止失败: ${err.message}`));
      }
    });
  }
  
  /**
   * 获取设备列表
   */
  async getDevices() {
    return new Promise((resolve, reject) => {
      if (this.handle === 0) {
        return reject(new Error('核心库尚未初始化'));
      }
      
      const errorStruct = Buffer.alloc(256);
      
      try {
        const resultBuffer = this.lib.pasteall_get_devices(this.handle, errorStruct);
        
        if (!resultBuffer) {
          const errorCode = errorStruct.readInt16LE(0);
          const errorMessage = ERROR_CODES[errorCode] || '未知错误';
          return reject(new Error(`获取设备列表失败: ${errorMessage}`));
        }
        
        // 解析返回的JSON字符串
        const jsonStr = resultBuffer.readCString(0);
        const devices = JSON.parse(jsonStr);
        
        resolve(devices);
      } catch (err) {
        log.error(`调用pasteall_get_devices失败: ${err.message}`);
        reject(new Error(`获取设备列表失败: ${err.message}`));
      }
    });
  }
  
  /**
   * 发送内容到设备
   * @param {string} deviceId 目标设备ID
   * @param {number} contentType 内容类型 (0:文本, 1:文件, 2:图片)
   * @param {Buffer|string} content 内容数据
   */
  async sendToDevice(deviceId, contentType, content) {
    return new Promise((resolve, reject) => {
      if (this.handle === 0) {
        return reject(new Error('核心库尚未初始化'));
      }
      
      const errorStruct = Buffer.alloc(256);
      
      try {
        // 转换内容为Buffer
        let contentBuffer;
        if (typeof content === 'string') {
          contentBuffer = Buffer.from(content, 'utf8');
        } else if (Buffer.isBuffer(content)) {
          contentBuffer = content;
        } else {
          return reject(new Error('内容必须是字符串或Buffer'));
        }
        
        // 创建FFI可以接受的Buffer对象
        const contentFFI = {
          data: contentBuffer,
          len: contentBuffer.length
        };
        
        const success = this.lib.pasteall_send_to_device(
          this.handle,
          deviceId,
          contentType,
          contentFFI,
          errorStruct
        );
        
        if (!success) {
          const errorCode = errorStruct.readInt16LE(0);
          const errorMessage = ERROR_CODES[errorCode] || '未知错误';
          return reject(new Error(`发送内容失败: ${errorMessage}`));
        }
        
        log.info(`成功发送内容到设备: ${deviceId}`);
        resolve(true);
      } catch (err) {
        log.error(`调用pasteall_send_to_device失败: ${err.message}`);
        reject(new Error(`发送内容失败: ${err.message}`));
      }
    });
  }
  
  /**
   * 设置剪贴板变化回调
   * @param {Function} callback 剪贴板内容变化时的回调函数
   */
  onClipboardChanged(callback) {
    this.clipboardCallback = callback;
    // 在实际实现中，这里需要通过FFI注册回调函数到Rust
    log.info('注册剪贴板变化回调');
  }
  
  /**
   * 获取库版本
   */
  getVersion() {
    try {
      const resultBuffer = this.lib.pasteall_get_version();
      return resultBuffer.readCString(0);
    } catch (err) {
      log.error(`获取版本失败: ${err.message}`);
      return '未知版本';
    }
  }
  
  /**
   * 资源清理
   */
  destroy() {
    if (this.handle !== 0) {
      try {
        this.lib.pasteall_destroy(this.handle);
        this.handle = 0;
        log.info('核心库资源已释放');
      } catch (err) {
        log.error(`释放核心库资源失败: ${err.message}`);
      }
    }
  }
}

module.exports = { PasteAllCore };
