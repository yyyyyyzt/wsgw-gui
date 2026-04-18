import { invoke } from "@tauri-apps/api/core";

import "./styles.css";

type StatusType = "idle" | "working" | "done" | "failed";

const state = {
  status: "idle" as StatusType,
  logs: ["项目已接入 Midscene 最小探活：请先按 README 开启 Chrome 远程调试并配置环境变量，再点击下方按钮。"],
};

const statusMap: Record<StatusType, string> = {
  idle: "未开始",
  working: "执行中",
  done: "已完成",
  failed: "失败",
};

function renderLayout(): void {
  const app = document.querySelector<HTMLElement>("#app");
  if (!app) return;
  app.innerHTML = `
    <main class="layout">
      <section class="card">
        <h1 class="title">WSGW GUI</h1>
        <p class="subtitle">Tauri + Midscene Windows 自动化客户端（里程碑 A2：最小探活）</p>
      </section>
      <section class="card">
        <div class="status">
          <span class="dot"></span>
          当前状态：<strong id="status-value"></strong>
        </div>
        <div class="actions" style="margin-top: 12px;">
          <button id="run-task">运行 Midscene 最小探活</button>
        </div>
        <ol id="logs" class="log"></ol>
      </section>
    </main>
  `;
}

function renderStatus(): void {
  const statusEl = document.querySelector<HTMLElement>("#status-value");
  if (!statusEl) return;
  statusEl.textContent = statusMap[state.status];
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

function setStatus(status: StatusType): void {
  state.status = status;
  renderStatus();
}

async function runMidsceneMinimalTask(): Promise<void> {
  setStatus("working");
  pushLog("[automation] 正在通过 Tauri 调用 Node 子进程（Midscene + puppeteer-core + CDP）…");

  try {
    const message = await invoke<string>("run_midscene_minimal");
    pushLog(`[automation] ${message}`);
    setStatus("done");
  } catch (raw) {
    const errText =
      typeof raw === "string"
        ? raw
        : raw instanceof Error
          ? raw.message
          : JSON.stringify(raw);
    pushLog(`[automation] 失败：${errText}`);
    setStatus("failed");
  }
}

function bindEvents(): void {
  const runBtn = document.querySelector<HTMLButtonElement>("#run-task");
  if (!runBtn) return;
  runBtn.addEventListener("click", () => {
    void runMidsceneMinimalTask();
  });
}

function bootstrap(): void {
  renderLayout();
  renderStatus();
  renderLogs();
  bindEvents();
}

bootstrap();
