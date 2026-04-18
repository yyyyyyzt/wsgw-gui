type StatusType = "idle" | "working" | "done";

const state = {
  status: "idle" as StatusType,
  logs: ["项目骨架已初始化，等待执行任务。"],
};

import "./styles.css";

const statusMap: Record<StatusType, string> = {
  idle: "未开始",
  working: "执行中",
  done: "已完成",
};

function renderLayout(): void {
  const app = document.querySelector<HTMLElement>("#app");
  if (!app) return;
  app.innerHTML = `
    <main class="layout">
      <section class="card">
        <h1 class="title">WSGW GUI</h1>
        <p class="subtitle">Tauri + Midscene 自动化客户端骨架（里程碑 A1）</p>
      </section>
      <section class="card">
        <div class="status">
          <span class="dot"></span>
          当前状态：<strong id="status-value"></strong>
        </div>
        <div class="actions" style="margin-top: 12px;">
          <button id="run-task">执行占位任务</button>
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

async function runMockTask(): Promise<void> {
  setStatus("working");
  pushLog("开始执行内置占位任务...");
  await new Promise((resolve) => {
    window.setTimeout(resolve, 800);
  });
  pushLog("占位任务执行完成。下一步将接入 Midscene 与 CDP 逻辑。");
  setStatus("done");
}

function bindEvents(): void {
  const runBtn = document.querySelector<HTMLButtonElement>("#run-task");
  if (!runBtn) return;
  runBtn.addEventListener("click", () => {
    void runMockTask();
  });
}

function bootstrap(): void {
  renderLayout();
  renderStatus();
  renderLogs();
  bindEvents();
}

bootstrap();
