import "dotenv/config";

/**
 * 内置演示任务（里程碑 B3）：在用户点击后由 Tauri 拉起。
 * 连接 CDP 后打开新闻页，用 DOM 抓取整理热点标题（不依赖大模型，便于内网/无 Key 环境验收）。
 * 目标 URL 由 WSGW_NEWS_URL 配置，默认 https://news.baidu.com/ 。
 */
import puppeteer from "puppeteer-core";
import { PuppeteerAgent } from "@midscene/web/puppeteer";

type ResultPayload =
  | {
      ok: true;
      url: string;
      cdpSource: "WSGW_CDP_WS_URL" | "WSGW_DEBUG_PORT";
      headlines: string[];
      sourceUrl: string;
    }
  | { ok: false; error: string };

const DEFAULT_NEWS_URL = "https://news.baidu.com/";

function fail(message: string): never {
  const payload: ResultPayload = { ok: false, error: message };
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  process.exit(1);
}

function ok(
  url: string,
  cdpSource: "WSGW_CDP_WS_URL" | "WSGW_DEBUG_PORT",
  headlines: string[],
  sourceUrl: string,
): void {
  const payload: ResultPayload = {
    ok: true,
    url,
    cdpSource,
    headlines,
    sourceUrl,
  };
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
      "未配置 CDP：请设置 WSGW_CDP_WS_URL 或 WSGW_DEBUG_PORT（由主进程注入或 .env）。",
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
    fail(`无法连接本机调试端口 ${port}（${versionUrl}）：${msg}`);
  }

  if (!res.ok) {
    fail(`请求 ${versionUrl} 失败：HTTP ${res.status}`);
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
    fail(`响应中缺少 webSocketDebuggerUrl（${versionUrl}）。`);
  }

  return { ws, source: "WSGW_DEBUG_PORT" };
}

async function main(): Promise<void> {
  const { ws, source } = await resolveBrowserWsEndpoint();
  const newsUrl = (process.env.WSGW_NEWS_URL ?? DEFAULT_NEWS_URL).trim() || DEFAULT_NEWS_URL;

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
      groupName: "WSGW Builtin News",
      waitForNetworkIdleTimeout: 0,
    });

    await agent.page.navigate(newsUrl);
    await new Promise((r) => setTimeout(r, 1200));

    const headlines = await page.evaluate(() => {
      const seen = new Set<string>();
      const out: string[] = [];
      const push = (t: string | null | undefined) => {
        const s = (t ?? "").replace(/\s+/g, " ").trim();
        if (s.length < 4 || s.length > 120) return;
        if (seen.has(s)) return;
        seen.add(s);
        out.push(s);
      };

      document
        .querySelectorAll(
          "a[href*='news'], .hotnews a, .news-item a, li a, h2 a, h3 a, .title a",
        )
        .forEach((el) => push(el.textContent));

      if (out.length < 3) {
        document.querySelectorAll("a").forEach((el) => {
          const href = el.getAttribute("href") ?? "";
          if (href.includes("baijiahao") || href.includes("/article")) {
            push(el.textContent);
          }
        });
      }

      return out.slice(0, 12);
    });
    const finalUrl = await agent.page.url();

    await agent.destroy();
    agent = undefined;
    await browser.disconnect();
    browser = undefined;

    ok(finalUrl, source, headlines, newsUrl);
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
    fail(`内置新闻任务失败：${message}`);
  }
}

void main();
