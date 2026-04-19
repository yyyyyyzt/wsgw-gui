import { invoke } from "@tauri-apps/api/core";

import "./styles.css";

/** 与 progress 里程碑 A4 对齐的 UI 状态 */
type UiStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "executing"
  | "completed"
  | "failed";

type RuntimeConfigSummary = {
  wsgwDebugPort: string;
  wsgwCdpWsUrlConfigured: boolean;
  wsgwDemoUrlConfigured: boolean;
};

const state: {
  uiStatus: UiStatus;
  logs: string[];
  busy: boolean;
} = {
  uiStatus: "disconnected",
  logs: [],
  busy: false,
};

const statusLabel: Record<UiStatus, string> = {
  disconnected: "未连接",
  connecting: "连接中",
  connected: "已连接",
  executing: "执行中",
  completed: "完成",
  failed: "失败",
};

const statusCssClass: Record<UiStatus, string> = {
  disconnected: "status-disconnected",
  connecting: "status-connecting",
  connected: "status-connected",
  executing: "status-executing",
  completed: "status-completed",
  failed: "status-failed",
};

function renderLayout(): void {
  const app = document.querySelector<HTMLElement>("#app");
  if (!app) return;
  app.innerHTML = `
    <main class="layout">
      <section class="card">
        <h1 class="title">WSGW GUI</h1>
        <p class="subtitle">Tauri + Midscene Windows 自动化客户端（里程碑 A3/A4：配置与状态）</p>
      </section>
      <section class="card">
        <div id="status-pill" class="status-pill">
          <span class="dot"></span>
          <span>当前状态：</span>
          <strong id="status-value"></strong>
        </div>
        <p class="config-hint" id="config-hint"></p>
        <div class="actions" style="margin-top: 12px;">
          <button type="button" id="btn-check">检测 CDP（TCP）</button>
          <button type="button" id="btn-run">运行 Midscene 最小探活</button>
        </div>
        <ol id="logs" class="log"></ol>
      </section>
    </main>
  `;
}

function renderStatus(): void {
  const pill = document.querySelector<HTMLElement>("#status-pill");
  const statusEl = document.querySelector<HTMLElement>("#status-value");
  if (!pill || !statusEl) return;
  pill.className = `status-pill ${statusCssClass[state.uiStatus]}`;
  statusEl.textContent = statusLabel[state.uiStatus];
}

function renderConfigHint(text: string): void {
  const el = document.querySelector<HTMLElement>("#config-hint");
  if (!el) return;
  el.textContent = text;
}

function renderLogs(): void {
  const logsEl = document.querySelector<HTMLOListElement>("#logs");
  if (!logsEl) return;
  logsEl.innerHTML = "";
  state.logs.forEach((line) => {
    const item = document.createElement("li");
    item.textContent = line;
    logsEl.appendChild(item);
  });
}

function pushLog(line: string): void {
  state.logs.push(line);
  renderLogs();
}

function setUiStatus(s: UiStatus): void {
  state.uiStatus = s;
  renderStatus();
  syncButtonDisabled();
}

function syncButtonDisabled(): void {
  const checkBtn = document.querySelector<HTMLButtonElement>("#btn-check");
  const runBtn = document.querySelector<HTMLButtonElement>("#btn-run");
  const disabled = state.busy;
  if (checkBtn) checkBtn.disabled = disabled;
  if (runBtn) runBtn.disabled = disabled;
}

function setBusy(busy: boolean): void {
  state.busy = busy;
  syncButtonDisabled();
}

function formatInvokeError(raw: unknown): string {
  if (typeof raw === "string") return raw;
  if (raw instanceof Error) return raw.message;
  return JSON.stringify(raw);
}

async function loadConfigSummary(): Promise<void> {
  try {
    const cfg = await invoke<RuntimeConfigSummary>("get_runtime_config_summary");
    const mode = cfg.wsgwCdpWsUrlConfigured
      ? "已配置 WSGW_CDP_WS_URL（优先于端口）"
      : `使用本机端口 WSGW_DEBUG_PORT=${cfg.wsgwDebugPort}（未配置 URL 时由脚本请求 /json/version）`;
    const demo = cfg.wsgwDemoUrlConfigured
      ? "；已配置 WSGW_DEMO_URL（探活前会导航）"
      : "；未配置 WSGW_DEMO_URL（不自动导航）";
    renderConfigHint(`${mode}${demo}。配置来自应用目录或开发仓库根目录的 .env（可选 WSGW_ENV_FILE）。");
    pushLog(`[config] ${mode}${demo}`);
  } catch (raw) {
    renderConfigHint("无法读取运行时配置摘要，请查看日志。");
    pushLog(`[config] 读取配置摘要失败：${formatInvokeError(raw)}`);
  }
}

async function onCheckCdp(): Promise<void> {
  setBusy(true);
  setUiStatus("connecting");
  pushLog("[cdp] 正在检测本机调试端口（TCP，约 2 秒超时）…");
  try {
    const msg = await invoke<string>("check_cdp_reachable");
    pushLog(`[cdp] ${msg}`);
    setUiStatus("connected");
  } catch (raw) {
    pushLog(`[cdp] ${formatInvokeError(raw)}`);
    setUiStatus("failed");
  } finally {
    setBusy(false);
  }
}

async function onRunMidscene(): Promise<void> {
  setBusy(true);
  setUiStatus("executing");
  pushLog("[automation] 正在通过 Tauri 调用 Node 子进程（Midscene + puppeteer-core + CDP）…");
  try {
    const message = await invoke<string>("run_midscene_minimal");
    pushLog(`[automation] ${message}`);
    setUiStatus("completed");
  } catch (raw) {
    pushLog(`[automation] 失败：${formatInvokeError(raw)}`);
    setUiStatus("failed");
  } finally {
    setBusy(false);
  }
}

function bindEvents(): void {
  document.querySelector("#btn-check")?.addEventListener("click", () => {
    void onCheckCdp();
  });
  document.querySelector("#btn-run")?.addEventListener("click", () => {
    void onRunMidscene();
  });
}

async function bootstrap(): Promise<void> {
  renderLayout();
  renderStatus();
  renderLogs();
  bindEvents();
  pushLog("就绪：请先配置 .env（或使用默认端口 9222），建议先点「检测 CDP」再运行探活。");
  await loadConfigSummary();
}

void bootstrap();
