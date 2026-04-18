# Midscene Tauri 自动化客户端框架

这是为你搭建的轻量自动化客户端框架，基于 Tauri + Midscene.js 开发，专门适配 Windows 自动化应用场景。

## 核心特性

✅ 通过 Chrome 远程调试（CDP）连接本机已打开的浏览器，复用登录态与内网访问能力  
✅ Midscene + `puppeteer-core` 在独立 Node 子进程中运行，由用户点击触发（符合本地自动化、不后台自启的约束）  
✅ 轻量打包方向：相比 Electron 体积更小（具体体积随依赖与资源而定）  
⏳ 内置业务任务（如百度新闻整理）计划在后续里程碑（B3）补齐

## 开发准备

### 环境要求（建议）

- Node.js 20 LTS 及以上
- npm 10 及以上
- Rust **1.88.0**（本仓库通过根目录 `rust-toolchain.toml` 固定；满足上游依赖对 Cargo `edition2024` 的要求）
- 系统已安装 **Node.js** 且 `node` 在 PATH 中（运行 Midscene 探活子进程需要；与是否安装 Chromium 无关，本客户端使用 `puppeteer-core` 仅通过 CDP 连接）
- Tauri 在 Linux/macOS 上按官方文档安装系统依赖；**Windows 为当前主要目标平台**

### 阶段 0：从文档仓库到可运行工程

1. 初始化前端与 Tauri 工程骨架（对应 `progress.md` 里程碑 A1）✅ 已完成  
2. 接入 Midscene 最小探活（对应 `progress.md` 里程碑 A2）✅ 已完成  
3. 每次阶段完成后同步更新 `progress.md` 与本文档

## 开发与运行

```bash
# 1. 安装依赖
npm install

# 2.（可选）在项目根目录复制环境变量模板并按需填写
cp .env.example .env

# 3. 启动开发模式（predev 会生成 src-tauri/resources/midscene-minimal.mjs）
npm run tauri:dev
```

### Midscene 最小探活（里程碑 A2）

1. 按下文「开启 Chrome 远程调试」用 `--remote-debugging-port=9222`（或自定义端口）启动 Chrome。  
2. 在项目根目录配置环境变量（可复制 `.env.example` 为 `.env`；子进程通过 `dotenv/config` 读取）：
   - **推荐**：`WSGW_DEBUG_PORT=9222`（脚本会访问 `http://127.0.0.1:<端口>/json/version` 解析 `webSocketDebuggerUrl`）  
   - **或**：`WSGW_CDP_WS_URL=ws://...`（完整 CDP WebSocket 地址）  
   - **可选**：`WSGW_DEMO_URL=https://...`（若设置，探活前会导航到该 URL；须用户显式配置，默认不导航）  
3. 在客户端窗口点击「运行 Midscene 最小探活」。成功时日志会显示当前活动页 URL。

说明：

- 打包产物 `src-tauri/resources/midscene-minimal.mjs` 由 `npm run bundle:midscene-worker` 生成，已加入 `.gitignore`；`npm run build` / `npm run dev` 会通过 `prebuild` / `predev` 自动生成。  
- 若开发时脚本路径异常，可设置 `WSGW_MIDSCENE_SCRIPT` 指向本地 bundle 的绝对路径（见 `.env.example`）。

## 其他 npm 脚本

| 命令 | 用途 |
|------|------|
| `npm run bundle:midscene-worker` | 将 `scripts/run-minimal-midscene.mts` 打包为 `src-tauri/resources/midscene-minimal.mjs`（供 Tauri 子进程执行） |
| `npm run typecheck` | TypeScript 类型检查 |
| `npm run build` | 前端 `tsc` + Vite 构建（会先执行 `bundle:midscene-worker`） |

## 打包 Windows EXE 安装包

```bash
npm run tauri:build
```

打包完成后，安装包会生成在 `src-tauri/target/release/bundle/nsis/` 目录下（在 Windows 上执行该目标时）。

## 边开发边补文档（必做）

1. 每次开工前先更新 `progress.md` 的「当前迭代任务」。  
2. 每完成一个可验证子功能，立刻更新任务状态与一句产出说明。  
3. 若命令、目录、配置发生变化，必须在同次提交内同步更新 README。  
4. 提交前检查「文档命令可执行、文档路径真实存在」。

## 使用说明

### 首次使用：开启 Chrome 远程调试

1. 关闭所有已打开的 Chrome/Edge 窗口  
2. 右键 Chrome 快捷方式 → 选择「属性」  
3. 在「目标」输入框的最后，添加参数：` --remote-debugging-port=9222`  
   > 注意：参数前有一个空格，例如  
   > `C:\Program Files\Google\Chrome\Application\chrome.exe --remote-debugging-port=9222`  
4. 点击确定，通过这个快捷方式启动 Chrome  
5. 正常登录你的内网业务系统；本客户端通过 CDP 连接后复用该浏览器上下文

### 执行自动化（当前阶段）

1. 启动本客户端  
2. 配置好 `WSGW_DEBUG_PORT` 或 `WSGW_CDP_WS_URL`（见上文）  
3. 点击「运行 Midscene 最小探活」；后续里程碑将在此通道上扩展正式业务任务（见 `progress.md` 里程碑 B）
