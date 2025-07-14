import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'providers/device_provider.dart';
import 'providers/clipboard_provider.dart';
import 'providers/settings_provider.dart';

import 'screens/home_screen.dart';
import 'screens/pairing_screen.dart';
import 'screens/settings_screen.dart';

import 'utils/theme.dart';

void main() async {
  // 确保Flutter绑定初始化
  WidgetsFlutterBinding.ensureInitialized();
  
  // 初始化核心库
  // TODO: 初始化Rust核心库
  
  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider(create: (_) => DeviceProvider()),
        ChangeNotifierProvider(create: (_) => ClipboardProvider()),
        ChangeNotifierProvider(create: (_) => SettingsProvider()),
      ],
      child: const MyApp(),
    ),
  );
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'PasteAll',
      theme: AppTheme.lightTheme,
      darkTheme: AppTheme.darkTheme,
      themeMode: ThemeMode.system,
      initialRoute: '/',
      routes: {
        '/': (context) => const HomeScreen(),
        '/pairing': (context) => const PairingScreen(),
        '/settings': (context) => const SettingsScreen(),
      },
    );
  }
}
