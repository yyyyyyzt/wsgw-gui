# Midscene Tauri 自动化客户端框架

这是为你搭建的轻量自动化客户端框架，基于 Tauri + Midscene.js 开发，专门适配windows自动化应用场景。

## 核心特性

✅ 自动检测Chrome远程调试端口，一键连接  
✅ 本地CDP模式执行，完全复用用户的登录状态、UKey、内网网络  
✅ 轻量打包，最终Windows安装包仅10MB左右，远小于Electron方案  
✅ 内置测试任务：打开百度整理今日热点新闻，直接验证功能

## 开发准备

### 环境要求（建议）

- Node.js 20 LTS 及以上
- npm 10 及以上
- Rust stable（建议 1.77+）
- Tauri 开发依赖（按官方文档安装系统依赖）

### 阶段 0：从文档仓库到可运行工程

1. 初始化前端与 Tauri 工程骨架（对应 `progress.md` 里程碑 A1） ✅ 已完成
2. 已生成并提交以下基础文件：
   - `package.json`（含 `tauri:dev`、`tauri:build`）
   - `src-tauri/Cargo.toml`
   - `src-tauri/tauri.conf.json`
   - 前端入口与最小页面（`index.html`、`src/main.ts`、`src/styles.css`）
3. 后续进入业务开发前仍需执行本地验证：
   - `npm run typecheck`
   - `npm run tauri:dev`
4. 每次阶段完成后同步更新：
   - `progress.md` 当前迭代任务
   - 本 README 的“开发与运行”命令（若有变更）

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

## 边开发边补文档（必做）

1. 每次开工前先更新 `progress.md` 的“当前迭代任务”。
2. 每完成一个可验证子功能，立刻更新任务状态与一句产出说明。
3. 若命令、目录、配置发生变化，必须在同次提交内同步更新 README。
4. 提交前检查“文档命令可执行、文档路径真实存在”。

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
