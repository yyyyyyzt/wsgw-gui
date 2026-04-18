# Midscene Tauri 自动化客户端框架

这是为你搭建的轻量自动化客户端框架，基于 Tauri + Midscene.js 开发，专门适配windows自动化应用场景。

## 核心特性

✅ 自动检测Chrome远程调试端口，一键连接  
✅ 本地CDP模式执行，完全复用用户的登录状态、UKey、内网网络  
✅ 轻量打包，最终Windows安装包仅10MB左右，远小于Electron方案  
✅ 内置测试任务：打开百度整理今日热点新闻，直接验证功能

## 开发准备

待补充

## 开发与运行

```bash
# 1. 安装依赖
npm install

# 2. 启动开发模式（会同时启动前端和Tauri壳）
npm run tauri:dev
```

## 打包Windows EXE安装包

```bash
npm run tauri:build
```

打包完成后，安装包会生成在 `src-tauri/target/release/bundle/nsis/` 目录下，可直接分发。

## 使用说明

### 首次使用：开启Chrome远程调试

1. 关闭所有已打开的Chrome/Edge窗口
2. 右键Chrome快捷方式 → 选择「属性」
3. 在「目标」输入框的最后，添加参数：` --remote-debugging-port=9222`
   > 注意：参数前有一个空格，比如原本的目标是 `C:\Program Files\Google\Chrome\Application\chrome.exe`，修改后是：
   > `C:\Program Files\Google\Chrome\Application\chrome.exe --remote-debugging-port=9222`
4. 点击确定，通过这个快捷方式启动Chrome
5. 正常登录你的内网业务系统即可，客户端会自动复用这个浏览器的所有状态

### 执行自动化任务

1. 启动本客户端，客户端会自动检测Chrome的调试端口
2. 连接成功后，点击「执行测试任务」即可自动运行百度新闻整理的测试流程
3. 你可以基于这个框架，快速开发自己的业务自动化任务
