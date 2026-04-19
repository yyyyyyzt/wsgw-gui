import "dotenv/config";

/**
 * 由 Tauri 后端在用户点击时拉起的最小 Midscene + Puppeteer 探活脚本。
 * 仅连接用户已开启远程调试的 Chrome（CDP），实例化 PuppeteerAgent，读取当前页 URL 后退出。
 * 不发起任何外网请求（除非用户通过 WSGW_DEMO_URL 显式配置）。
 */
import puppeteer from "puppeteer-core";
import { PuppeteerAgent } from "@midscene/web/puppeteer";

type ResultPayload =
  | { ok: true; url: string; cdpSource: "WSGW_CDP_WS_URL" | "WSGW_DEBUG_PORT" }
  | { ok: false; error: string };

function fail(message: string): never {
  const payload: ResultPayload = { ok: false, error: message };
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  process.exit(1);
}

function ok(
  url: string,
  cdpSource: "WSGW_CDP_WS_URL" | "WSGW_DEBUG_PORT",
): void {
  const payload: ResultPayload = { ok: true, url, cdpSource };
  process.stdout.write(`${JSON.stringify(payload)}\n`);
}

async function resolveBrowserWsEndpoint(): Promise<{
  ws: string;
  source: "WSGW_CDP_WS_URL" | "WSGW_DEBUG_PORT";
}> {
  const direct = process.env.WSGW_CDP_WS_URL?.trim();
  if (direct) {
    return { ws: direct, source: "WSGW_CDP_WS_URL" };
  }

  const portRaw = process.env.WSGW_DEBUG_PORT?.trim();
  if (!portRaw) {
    fail(
      "未配置 CDP：请设置 WSGW_CDP_WS_URL（完整 ws 地址），或设置 WSGW_DEBUG_PORT（如 9222）以从本机 http://127.0.0.1:<端口>/json/version 自动解析。",
    );
  }

  const port = Number(portRaw);
  if (!Number.isInteger(port) || port <= 0 || port > 65535) {
    fail(`WSGW_DEBUG_PORT 无效：${portRaw}`);
  }

  const versionUrl = `http://127.0.0.1:${port}/json/version`;
  let res: Response;
  try {
    res = await fetch(versionUrl);
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    fail(
      `无法连接本机调试端口 ${port}（${versionUrl}）：${msg}。请确认 Chrome 已用 --remote-debugging-port=${port} 启动。`,
    );
  }

  if (!res.ok) {
    fail(
      `请求 ${versionUrl} 失败：HTTP ${res.status}。请确认远程调试端口已开启。`,
    );
  }

  let body: unknown;
  try {
    body = await res.json();
  } catch {
    fail(`解析 ${versionUrl} 的 JSON 响应失败。`);
  }

  const ws =
    typeof body === "object" &&
    body !== null &&
    "webSocketDebuggerUrl" in body &&
    typeof (body as { webSocketDebuggerUrl?: unknown }).webSocketDebuggerUrl ===
      "string"
      ? (body as { webSocketDebuggerUrl: string }).webSocketDebuggerUrl
      : "";

  if (!ws) {
    fail(
      `响应中缺少 webSocketDebuggerUrl。请确认 ${versionUrl} 返回 Chrome DevTools 版本信息。`,
    );
  }

  return { ws, source: "WSGW_DEBUG_PORT" };
}

async function main(): Promise<void> {
  const { ws, source } = await resolveBrowserWsEndpoint();
  const demoUrl = process.env.WSGW_DEMO_URL?.trim();

  let browser: Awaited<ReturnType<typeof puppeteer.connect>> | undefined;
  let agent: PuppeteerAgent | undefined;

  try {
    browser = await puppeteer.connect({
      browserWSEndpoint: ws,
      defaultViewport: null,
    });

    const pages = await browser.pages();
    const page = pages[0] ?? (await browser.newPage());

    agent = new PuppeteerAgent(page, {
      generateReport: false,
      persistExecutionDump: false,
      autoPrintReportMsg: false,
      groupName: "WSGW Minimal Ping",
      waitForNetworkIdleTimeout: 0,
    });

    if (demoUrl) {
      await agent.page.navigate(demoUrl);
    }

    const url = await agent.page.url();
    await agent.destroy();
    agent = undefined;

    await browser.disconnect();
    browser = undefined;

    ok(url, source);
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    try {
      if (agent) await agent.destroy();
    } catch {
      /* ignore */
    }
    try {
      if (browser) await browser.disconnect();
    } catch {
      /* ignore */
    }
    fail(`Midscene 最小探活失败：${message}`);
  }
}

void main();
