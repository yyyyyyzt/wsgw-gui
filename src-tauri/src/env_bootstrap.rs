use std::path::PathBuf;

use crate::cdp_session;

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

  let info = cdp_session::fetch_devtools_version_with_retry(port)
    .map_err(|e| format!("http://127.0.0.1:{port}/json/version：{e}"))?;
  Ok(cdp_session::format_http_check_ok(port, &info))
}
