//! CDP 会话：解析 `webSocketDebuggerUrl`、带重试、进程内缓存，供多次探活复用同一浏览器上下文（里程碑 B2）。

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

/// 从 `/json/version` 拉取并解析出的 DevTools 信息。
#[derive(Debug, Clone)]
pub struct DevToolsVersionInfo {
  pub browser: Option<String>,
  pub web_socket_debugger_url: String,
}

fn sleep_ms(ms: u64) {
  std::thread::sleep(Duration::from_millis(ms));
}

fn env_u64(name: &str, default: u64) -> u64 {
  std::env::var(name)
    .ok()
    .and_then(|s| s.trim().parse().ok())
    .filter(|&n| n > 0)
    .unwrap_or(default)
}

/// 对 `http://127.0.0.1:<port>/json/version` 发起单次 HTTP/1.1 GET，返回 JSON 根对象。
pub fn fetch_json_version_once(port: u16) -> Result<Value, String> {
  if port == 0 {
    return Err("端口不能为 0".into());
  }

  let url = format!("http://127.0.0.1:{port}/json/version");
  let addr = format!("127.0.0.1:{port}");
  let socket_addr: std::net::SocketAddr = addr
    .parse()
    .map_err(|e| format!("内部错误：无法解析地址 {addr}：{e}"))?;

  use std::io::{Read, Write};
  use std::net::TcpStream;

  let connect_timeout = Duration::from_secs(3);
  let mut stream = TcpStream::connect_timeout(&socket_addr, connect_timeout).map_err(|e| {
    format!("无法连接 {addr}（{e}）。请确认 Chrome 已用 --remote-debugging-port={port} 启动。")
  })?;
  let _ = stream.set_read_timeout(Some(Duration::from_secs(4)));
  let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));

  let req = format!(
    "GET /json/version HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nAccept: */*\r\n\r\n"
  );
  stream
    .write_all(req.as_bytes())
    .map_err(|e| format!("向 {url} 发送 HTTP 请求失败：{e}"))?;

  let mut raw = Vec::<u8>::new();
  stream
    .read_to_end(&mut raw)
    .map_err(|e| format!("读取 {url} 响应失败：{e}"))?;

  let text = String::from_utf8_lossy(&raw).into_owned();
  let lower = text.to_ascii_lowercase();
  let body_start = lower
    .find("\r\n\r\n")
    .ok_or_else(|| format!("{url} 响应格式异常：未找到 HTTP 头结束标记。"))?
    + 4;

  let (head, body) = text.split_at(body_start);
  let status_line = head.lines().next().unwrap_or("").trim();
  let status_code = status_line
    .split_whitespace()
    .nth(1)
    .and_then(|s| s.parse::<u16>().ok())
    .unwrap_or(0);

  if !(200..300).contains(&status_code) {
    return Err(format!(
      "请求 {url} 返回非成功状态：{status_line}。远程调试端口可能未由浏览器正确暴露。"
    ));
  }

  let body_trim = body.trim();
  serde_json::from_str(body_trim).map_err(|e| {
    format!(
      "{url} 的响应体不是合法 JSON（{e}）。响应片段：{}",
      body_trim.chars().take(120).collect::<String>()
    )
  })
}

pub fn parse_devtools_version(json: &Value) -> Result<DevToolsVersionInfo, String> {
  let ws = json
    .get("webSocketDebuggerUrl")
    .and_then(|v| v.as_str())
    .unwrap_or("")
    .trim()
    .to_string();

  if ws.is_empty() {
    return Err(
      "JSON 中缺少 webSocketDebuggerUrl。请确认该端口由 Chrome DevTools 协议提供。".into(),
    );
  }

  let browser = json
    .get("Browser")
    .or_else(|| json.get("browser"))
    .and_then(|v| v.as_str())
    .map(|s| s.to_string());

  Ok(DevToolsVersionInfo {
    browser,
    web_socket_debugger_url: ws,
  })
}

/// 带重试地拉取 `/json/version`（应对 Chrome 刚启动时尚未就绪的竞态）。
pub fn fetch_devtools_version_with_retry(port: u16) -> Result<DevToolsVersionInfo, String> {
  let max_attempts = env_u64("WSGW_CDP_RESOLVE_RETRIES", 5).min(20) as usize;
  let delay_ms = env_u64("WSGW_CDP_RESOLVE_DELAY_MS", 400).min(5000);

  let mut last_err: Option<String> = None;
  for attempt in 1..=max_attempts {
    match fetch_json_version_once(port).and_then(|j| parse_devtools_version(&j)) {
      Ok(info) => {
        if attempt > 1 {
          log::info!(
            "cdp_session: resolved webSocketDebuggerUrl on attempt {}/{}",
            attempt,
            max_attempts
          );
        }
        return Ok(info);
      }
      Err(e) => {
        last_err = Some(e.clone());
        log::warn!("cdp_session: attempt {}/{} failed: {}", attempt, max_attempts, e);
        if attempt < max_attempts {
          sleep_ms(delay_ms);
        }
      }
    }
  }

  Err(last_err.unwrap_or_else(|| "解析 CDP WebSocket 失败（未知原因）".into()))
}

pub fn format_http_check_ok(port: u16, info: &DevToolsVersionInfo) -> String {
  let url = format!("http://127.0.0.1:{port}/json/version");
  match &info.browser {
    Some(b) if !b.is_empty() => format!(
      "HTTP/JSON 检测通过：{url} 对应 {b}，webSocketDebuggerUrl 已就绪。"
    ),
    _ => format!(
      "HTTP/JSON 检测通过：{url} 返回 DevTools 版本信息，webSocketDebuggerUrl 已就绪（长度 {}）。",
      info.web_socket_debugger_url.len()
    ),
  }
}

#[derive(Debug, Clone)]
struct CachedEndpoint {
  web_socket_debugger_url: String,
  /// `env` 表示来自 WSGW_CDP_WS_URL；`http` 表示来自本机端口解析
  source: &'static str,
}

pub struct CdpSessionCache {
  inner: Mutex<Option<CachedEndpoint>>,
}

impl CdpSessionCache {
  pub const fn new() -> Self {
    Self {
      inner: Mutex::new(None),
    }
  }

  pub fn clear(&self) {
    let mut g = self.inner.lock().expect("cdp cache mutex poisoned");
    *g = None;
  }

  /// `force_refresh` 为 true 时忽略缓存并重新解析（或重新读取环境变量中的 URL）。
  pub fn resolve_endpoint(
    &self,
    force_refresh: bool,
  ) -> Result<(String, &'static str, bool), String> {
    if force_refresh {
      self.clear();
    }

    let env_ws = std::env::var("WSGW_CDP_WS_URL").unwrap_or_default();
    let env_ws = env_ws.trim();
    if !env_ws.is_empty() {
      let mut g = self.inner.lock().expect("cdp cache mutex poisoned");
      *g = Some(CachedEndpoint {
        web_socket_debugger_url: env_ws.to_string(),
        source: "env",
      });
      return Ok((env_ws.to_string(), "env", force_refresh));
    }

    if !force_refresh {
      let g = self.inner.lock().expect("cdp cache mutex poisoned");
      if let Some(c) = g.as_ref() {
        return Ok((
          c.web_socket_debugger_url.clone(),
          c.source,
          false,
        ));
      }
    }

    let port_raw = std::env::var("WSGW_DEBUG_PORT").unwrap_or_else(|_| "9222".into());
    let port: u16 = port_raw.trim().parse().map_err(|_| {
      format!(
        "WSGW_DEBUG_PORT 无效：{}。",
        port_raw.trim()
      )
    })?;
    if port == 0 {
      return Err("WSGW_DEBUG_PORT 不能为 0".into());
    }

    let info = fetch_devtools_version_with_retry(port)?;
    let ws = info.web_socket_debugger_url.clone();
    {
      let mut g = self.inner.lock().expect("cdp cache mutex poisoned");
      *g = Some(CachedEndpoint {
        web_socket_debugger_url: ws.clone(),
        source: "http",
      });
    }
    Ok((ws, "http", true))
  }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CdpSessionStatus {
  pub has_cached_ws: bool,
  pub prefers_env_ws_url: bool,
}

/// 与 CDP 端点做一次 WebSocket 握手后立即关闭（验证 URL 可用，不保留长连接）。
pub fn try_cdp_websocket_handshake(ws_url: &str) -> Result<(), String> {
  let url = url::Url::parse(ws_url).map_err(|e| format!("WebSocket URL 无法解析：{e}"))?;
  let scheme = url.scheme();
  if scheme != "ws" && scheme != "wss" {
    return Err("CDP WebSocket 地址须为 ws:// 或 wss://".into());
  }
  if scheme == "wss" {
    return Err(
      "当前版本仅校验本机 ws:// CDP；若你使用 wss，请确认证书与网络策略后再扩展实现。"
        .into(),
    );
  }

  let (mut socket, _resp) = tungstenite::connect(&url).map_err(|e| {
    format!(
      "WebSocket 握手失败：{e}。请确认 Chrome 仍以远程调试模式运行，且 webSocketDebuggerUrl 未过期。"
    )
  })?;

  let _ = socket.close(None);
  let _ = socket.flush();
  Ok(())
}

pub fn session_status_snapshot(cache: &CdpSessionCache) -> CdpSessionStatus {
  let has = cache
    .inner
    .lock()
    .expect("cdp cache mutex poisoned")
    .is_some();
  let prefers = std::env::var("WSGW_CDP_WS_URL")
    .map(|s| !s.trim().is_empty())
    .unwrap_or(false);
  CdpSessionStatus {
    has_cached_ws: has,
    prefers_env_ws_url: prefers,
  }
}
