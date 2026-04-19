use std::path::PathBuf;

/// 在进程启动时加载 `.env`，不覆盖已存在的环境变量（与 shell 导出行为一致）。
pub fn load_dotenv_files() {
  if let Ok(override_path) = std::env::var("WSGW_ENV_FILE") {
    let p = PathBuf::from(override_path.trim());
    if p.is_file() {
      let _ = dotenvy::from_path_override(&p);
      return;
    }
  }

  let candidates = dotenv_candidate_paths();
  for p in candidates {
    if p.is_file() {
      let _ = dotenvy::from_path(&p);
      break;
    }
  }
}

fn dotenv_candidate_paths() -> Vec<PathBuf> {
  let mut out = Vec::new();

  if let Ok(cwd) = std::env::current_dir() {
    out.push(cwd.join(".env"));
  }

  if let Ok(exe) = std::env::current_exe() {
    if let Some(dir) = exe.parent() {
      out.push(dir.join(".env"));
    }
  }

  // 开发：仓库根目录（src-tauri 上一级）
  let dev_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join(".env");
  out.push(dev_root);

  out
}

/// 若未配置 CDP WebSocket 且未配置端口，则默认使用 9222（与 README 一致）。
pub fn apply_default_debug_port() {
  let ws_empty = std::env::var("WSGW_CDP_WS_URL")
    .map(|s| s.trim().is_empty())
    .unwrap_or(true);
  let port_empty = std::env::var("WSGW_DEBUG_PORT")
    .map(|s| s.trim().is_empty())
    .unwrap_or(true);

  if ws_empty && port_empty {
    std::env::set_var("WSGW_DEBUG_PORT", "9222");
  }
}

pub fn validate_cdp_settings_for_child() -> Result<(), String> {
  let ws = std::env::var("WSGW_CDP_WS_URL").unwrap_or_default();
  if !ws.trim().is_empty() {
    return Ok(());
  }

  let port_raw = std::env::var("WSGW_DEBUG_PORT").unwrap_or_else(|_| "9222".into());
  let port_raw = port_raw.trim();
  if port_raw.is_empty() {
    return Err(
      "未配置 CDP：请设置 WSGW_CDP_WS_URL，或设置 WSGW_DEBUG_PORT（1–65535）。"
        .into(),
    );
  }

  let port: u16 = port_raw
    .parse()
    .map_err(|_| format!("WSGW_DEBUG_PORT 无效：{port_raw}（须为 1–65535 的整数）"))?;

  if port == 0 {
    return Err("WSGW_DEBUG_PORT 不能为 0".into());
  }

  Ok(())
}

pub fn check_debug_port_tcp() -> Result<String, String> {
  if let Ok(ws) = std::env::var("WSGW_CDP_WS_URL") {
    if !ws.trim().is_empty() {
      return Ok(
        "已配置 WSGW_CDP_WS_URL：将直接使用该 WebSocket 连接 CDP，未做本机 TCP 端口探测。"
          .into(),
      );
    }
  }

  let port_raw = std::env::var("WSGW_DEBUG_PORT").unwrap_or_else(|_| "9222".into());
  let port: u16 = port_raw.trim().parse().map_err(|_| {
    format!(
      "WSGW_DEBUG_PORT 无效：{}。请在 .env 或系统环境中设置为整数端口。",
      port_raw.trim()
    )
  })?;

  if port == 0 {
    return Err("WSGW_DEBUG_PORT 不能为 0".into());
  }

  let addr = format!("127.0.0.1:{port}");
  let socket_addr: std::net::SocketAddr = addr
    .parse()
    .map_err(|e| format!("内部错误：无法解析地址 {addr}：{e}"))?;

  use std::net::TcpStream;
  use std::time::Duration;

  TcpStream::connect_timeout(&socket_addr, Duration::from_secs(2)).map_err(|e| {
    format!(
      "本机 {addr} 无响应（{e}）。请确认已用带 --remote-debugging-port={port} 的快捷方式启动 Chrome，或检查防火墙/端口是否被占用。"
    )
  })?;

  Ok(format!(
    "本机调试端口 {port} 可建立 TCP 连接（Chrome 远程调试可能已开启）。"
  ))
}

/// 请求 `http://127.0.0.1:<port>/json/version`，确认返回 DevTools JSON 且含 `webSocketDebuggerUrl`。
/// 使用最小 HTTP/1.1 实现，避免引入额外 HTTP 客户端依赖。
pub fn check_debug_port_http_json() -> Result<String, String> {
  if let Ok(ws) = std::env::var("WSGW_CDP_WS_URL") {
    if !ws.trim().is_empty() {
      return Ok(
        "已配置 WSGW_CDP_WS_URL：将直接使用该 WebSocket 连接 CDP，未请求 /json/version。"
          .into(),
      );
    }
  }

  let port_raw = std::env::var("WSGW_DEBUG_PORT").unwrap_or_else(|_| "9222".into());
  let port: u16 = port_raw.trim().parse().map_err(|_| {
    format!(
      "WSGW_DEBUG_PORT 无效：{}。请在 .env 或系统环境中设置为整数端口。",
      port_raw.trim()
    )
  })?;

  if port == 0 {
    return Err("WSGW_DEBUG_PORT 不能为 0".into());
  }

  let url = format!("http://127.0.0.1:{port}/json/version");
  let addr = format!("127.0.0.1:{port}");
  let socket_addr: std::net::SocketAddr = addr
    .parse()
    .map_err(|e| format!("内部错误：无法解析地址 {addr}：{e}"))?;

  use std::io::{Read, Write};
  use std::net::TcpStream;
  use std::time::Duration;

  let mut stream = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(3)).map_err(
    |e| {
      format!(
        "无法连接 {addr}（{e}）。请先通过「检测 CDP（TCP）」确认端口开放，或检查 Chrome 是否以 --remote-debugging-port={port} 启动。"
      )
    },
  )?;
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
  let json: serde_json::Value =
    serde_json::from_str(body_trim).map_err(|e| {
      format!(
        "{url} 的响应体不是合法 JSON（{e}）。若端口被其他程序占用，请更换 WSGW_DEBUG_PORT。响应片段：{}",
        body_trim.chars().take(120).collect::<String>()
      )
    })?;

  let ws = json
    .get("webSocketDebuggerUrl")
    .and_then(|v| v.as_str())
    .unwrap_or("");

  if ws.is_empty() {
    return Err(format!(
      "{url} 返回的 JSON 中缺少 webSocketDebuggerUrl。请确认该端口由 Chrome DevTools 协议提供，而非普通 HTTP 服务。"
    ));
  }

  let browser = json
    .get("Browser")
    .or_else(|| json.get("browser"))
    .and_then(|v| v.as_str())
    .unwrap_or("");

  if browser.is_empty() {
    Ok(format!(
      "HTTP/JSON 检测通过：{url} 返回 DevTools 版本信息，webSocketDebuggerUrl 已就绪（长度 {}）。",
      ws.len()
    ))
  } else {
    Ok(format!(
      "HTTP/JSON 检测通过：{url} 对应 {browser}，webSocketDebuggerUrl 已就绪。"
    ))
  }
}
