import 'package:flutter/foundation.dart';
import '../models/device.dart';
import '../services/device_service.dart';

/// 设备提供者，管理设备状态
class DeviceProvider with ChangeNotifier {
  final DeviceService _deviceService = DeviceService();
  
  List<Device> _devices = [];
  Device? _selectedDevice;
  bool _isLoading = false;
  String? _error;
  
  /// 设备列表
  List<Device> get devices => _devices;
  
  /// 当前选中的设备
  Device? get selectedDevice => _selectedDevice;
  
  /// 是否正在加载
  bool get isLoading => _isLoading;
  
  /// 错误信息
  String? get error => _error;
  
  /// 设备提供者构造函数
  DeviceProvider() {
    refreshDevices();
  }
  
  /// 刷新设备列表
  Future<void> refreshDevices() async {
    _isLoading = true;
    _error = null;
    notifyListeners();
    
    try {
      _devices = await _deviceService.getDevices();
      
      // 如果当前选中的设备不在列表中，清除选择
      if (_selectedDevice != null && 
          !_devices.any((d) => d.id == _selectedDevice!.id)) {
        _selectedDevice = null;
      }
      
      _isLoading = false;
      notifyListeners();
    } catch (e) {
      _isLoading = false;
      _error = e.toString();
      notifyListeners();
    }
  }
  
  /// 选择设备
  void selectDevice(Device? device) {
    _selectedDevice = device;
    notifyListeners();
  }
  
  /// 开始设备配对
  Future<String> startPairing() async {
    _isLoading = true;
    _error = null;
    notifyListeners();
    
    try {
      final pairingCode = await _deviceService.startPairing();
      _isLoading = false;
      notifyListeners();
      return pairingCode;
    } catch (e) {
      _isLoading = false;
      _error = e.toString();
      notifyListeners();
      rethrow;
    }
  }
  
  /// 确认PIN码
  Future<bool> confirmPin(String pin) async {
    _isLoading = true;
    _error = null;
    notifyListeners();
    
    try {
      final result = await _deviceService.confirmPin(pin);
      _isLoading = false;
      
      if (result) {
        await refreshDevices();
      } else {
        _error = '配对失败: PIN码不匹配';
        notifyListeners();
      }
      
      return result;
    } catch (e) {
      _isLoading = false;
      _error = e.toString();
      notifyListeners();
      return false;
    }
  }
  
  /// 重命名设备
  Future<bool> renameDevice(String deviceId, String newName) async {
    _isLoading = true;
    _error = null;
    notifyListeners();
    
    try {
      final result = await _deviceService.renameDevice(deviceId, newName);
      
      if (result) {
        // 更新本地设备列表
        final index = _devices.indexWhere((d) => d.id == deviceId);
        if (index != -1) {
          _devices[index] = _devices[index].copyWith(name: newName);
        }
      }
      
      _isLoading = false;
      notifyListeners();
      return result;
    } catch (e) {
      _isLoading = false;
      _error = e.toString();
      notifyListeners();
      return false;
    }
  }
  
  /// 删除设备
  Future<bool> deleteDevice(String deviceId) async {
    _isLoading = true;
    _error = null;
    notifyListeners();
    
    try {
      final result = await _deviceService.deleteDevice(deviceId);
      
      if (result) {
        // 从本地列表移除
        _devices.removeWhere((d) => d.id == deviceId);
        
        // 如果删除的是当前选中的设备，清除选择
        if (_selectedDevice?.id == deviceId) {
          _selectedDevice = null;
        }
      }
      
      _isLoading = false;
      notifyListeners();
      return result;
    } catch (e) {
      _isLoading = false;
      _error = e.toString();
      notifyListeners();
      return false;
    }
  }
}
