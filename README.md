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

### 配置（里程碑 A3）

1. 复制模板：`cp .env.example .env`（Windows 上可手动复制并重命名）。  
2. **Rust 主进程**在启动时会依次尝试加载 `.env`：当前工作目录 → 可执行文件所在目录 → 开发时仓库根目录（`src-tauri/../.env`）。也可用 **`WSGW_ENV_FILE`** 指向任意 `.env` 文件的绝对路径（会覆盖已存在的环境变量）。  
3. **CDP 连接信息**（二选一；同时配置时 **WSGW_CDP_WS_URL 优先**）：
   - `WSGW_CDP_WS_URL`：完整 WebSocket 地址（来自 `chrome://inspect` 或 `http://127.0.0.1:<端口>/json/version` 的 `webSocketDebuggerUrl`）。  
   - `WSGW_DEBUG_PORT`：仅端口号；若 **既未设置 URL 也未设置端口**，主进程会默认使用 **`9222`**（与 README 中 Chrome 快捷方式示例一致）。  
4. **Midscene 子进程**仍通过 `scripts/run-minimal-midscene.mts` 内的 `dotenv/config` 读取**当前工作目录**下的 `.env`；开发时通常与仓库根目录一致。若子进程未读到变量，请确认从项目根目录启动 `tauri dev`，或依赖主进程已通过环境变量传入的值（点击「运行探活」时由 Rust 注入 `WSGW_*`）。

### Midscene 最小探活与 CDP 检测（里程碑 A2 + A4）

1. 按下文「开启 Chrome 远程调试」用 `--remote-debugging-port=9222`（或自定义端口）启动 Chrome。  
2. 按上文配置 `.env`。  
3. 在客户端窗口先点击 **「检测 CDP（TCP）」**：仅检测本机 `127.0.0.1:<WSGW_DEBUG_PORT>` 是否能建立 TCP 连接（约 2 秒超时）；若已配置 `WSGW_CDP_WS_URL` 则跳过端口探测并提示将直接使用 WebSocket。  
4. 点击 **「运行 Midscene 最小探活」**：拉起 Node 子进程，完成 CDP 与 `PuppeteerAgent` 探活；成功时日志显示当前活动页 URL。  
5. 界面状态 pill 展示 **未连接 / 连接中 / 已连接 / 执行中 / 完成 / 失败**（与 `progress.md` 里程碑 A4 对齐）。

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
2. 配置好 `.env`（或使用默认端口 9222）  
3. 先「检测 CDP」再「运行 Midscene 最小探活」；后续里程碑将在此通道上扩展正式业务任务（见 `progress.md` 里程碑 B）
