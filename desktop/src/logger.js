/**
 * 日志模块
 */

const winston = require('winston');
const path = require('path');
const { app } = require('electron');

// 确定日志文件路径
const logPath = app 
  ? path.join(app.getPath('userData'), 'logs')
  : path.join(process.cwd(), 'logs');

// 创建日志实例
const logger = winston.createLogger({
  level: process.env.NODE_ENV === 'development' ? 'debug' : 'info',
  format: winston.format.combine(
    winston.format.timestamp({
      format: 'YYYY-MM-DD HH:mm:ss'
    }),
    winston.format.errors({ stack: true }),
    winston.format.splat(),
    winston.format.json()
  ),
  defaultMeta: { service: 'pasteall-desktop' },
  transports: [
    // 写入所有日志到文件
    new winston.transports.File({ 
      filename: path.join(logPath, 'error.log'), 
      level: 'error' 
    }),
    new winston.transports.File({ 
      filename: path.join(logPath, 'combined.log') 
    })
  ]
});

// 在开发环境下也输出到控制台
if (process.env.NODE_ENV === 'development') {
  logger.add(new winston.transports.Console({
    format: winston.format.combine(
      winston.format.colorize(),
      winston.format.simple()
    )
  }));
}

module.exports = logger;
