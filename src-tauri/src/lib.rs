mod env_bootstrap;

use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};
use tauri::path::BaseDirectory;
use tauri::Manager;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MidsceneMinimalOk {
  ok: bool,
  url: Option<String>,
  cdp_source: Option<String>,
  error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigSummary {
  pub wsgw_debug_port: String,
  pub wsgw_cdp_ws_url_configured: bool,
  pub wsgw_demo_url_configured: bool,
}

/// 供前端展示当前生效配置（不含密钥；仅布尔与端口字符串）。
#[tauri::command]
fn get_runtime_config_summary() -> RuntimeConfigSummary {
  let ws_set = std::env::var("WSGW_CDP_WS_URL")
    .map(|s| !s.trim().is_empty())
    .unwrap_or(false);
  let port = std::env::var("WSGW_DEBUG_PORT").unwrap_or_else(|_| "9222".into());
  let demo = std::env::var("WSGW_DEMO_URL")
    .map(|s| !s.trim().is_empty())
    .unwrap_or(false);
  RuntimeConfigSummary {
    wsgw_debug_port: port.trim().to_string(),
    wsgw_cdp_ws_url_configured: ws_set,
    wsgw_demo_url_configured: demo,
  }
}

fn resolve_midscene_script_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
  if let Ok(override_path) = std::env::var("WSGW_MIDSCENE_SCRIPT") {
    let p = PathBuf::from(override_path.trim());
    if p.is_file() {
      return Ok(p);
    }
    return Err(format!(
      "WSGW_MIDSCENE_SCRIPT 已设置但文件不存在：{}",
      p.display()
    ));
  }

  if cfg!(debug_assertions) {
    let dev_candidate =
      PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/midscene-minimal.mjs");
    if dev_candidate.is_file() {
      return Ok(dev_candidate);
    }
  }

  let bundled = app
    .path()
    .resolve("midscene-minimal.mjs", BaseDirectory::Resource)
    .map_err(|e| format!("无法解析内置脚本路径：{e}"))?;
  if bundled.is_file() {
    return Ok(bundled);
  }

  Err(
    "未找到 midscene-minimal.mjs。开发环境请先执行 npm run bundle:midscene-worker；发布包请确认 tauri.conf.json 的 bundle.resources 已包含该文件。"
      .into(),
  )
}

fn find_node_binary() -> &'static str {
  if cfg!(target_os = "windows") {
    "node.exe"
  } else {
    "node"
  }
}

/// 用户点击后执行：对本机调试端口做 TCP 探测（不发起 HTTP 请求）。
#[tauri::command]
async fn check_cdp_reachable() -> Result<String, String> {
  tauri::async_runtime::spawn_blocking(|| env_bootstrap::check_debug_port_tcp())
    .await
    .map_err(|e| format!("后台任务 Join 失败：{e}"))?
}

/// 用户点击后执行：请求 `/json/version` 并校验 `webSocketDebuggerUrl`（里程碑 B1）。
#[tauri::command]
async fn check_cdp_devtools_json() -> Result<String, String> {
  tauri::async_runtime::spawn_blocking(|| env_bootstrap::check_debug_port_http_json())
    .await
    .map_err(|e| format!("后台任务 Join 失败：{e}"))?
}

/// 用户点击界面后由前端调用：在子进程中运行打包后的 Midscene 最小探活脚本（不自动执行）。
#[tauri::command]
async fn run_midscene_minimal(app: tauri::AppHandle) -> Result<String, String> {
  env_bootstrap::validate_cdp_settings_for_child()?;

  let script = resolve_midscene_script_path(&app)?;
  let node = find_node_binary().to_string();

  let cdp_ws = std::env::var("WSGW_CDP_WS_URL")
    .ok()
    .filter(|s| !s.trim().is_empty());
  let debug_port = std::env::var("WSGW_DEBUG_PORT")
    .ok()
    .filter(|s| !s.trim().is_empty());
  let demo_url = std::env::var("WSGW_DEMO_URL")
    .ok()
    .filter(|s| !s.trim().is_empty());

  tauri::async_runtime::spawn_blocking(move || {
    let mut cmd = Command::new(&node);
    cmd.arg(&script);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    if let Some(v) = cdp_ws {
      cmd.env("WSGW_CDP_WS_URL", v);
    }
    if let Some(v) = debug_port {
      cmd.env("WSGW_DEBUG_PORT", v);
    }
    if let Some(v) = demo_url {
      cmd.env("WSGW_DEMO_URL", v);
    }

    let mut child = cmd.spawn().map_err(|e| {
      format!("无法启动 Node 子进程（{node}）：{e}。请确认已安装 Node.js 且已加入 PATH。")
    })?;

    let stdout = child
      .stdout
      .take()
      .ok_or_else(|| "子进程未提供 stdout".to_string())?;
    let stderr = child.stderr.take();

    let stderr_thread = std::thread::spawn(move || -> String {
      let Some(mut err) = stderr else {
        return String::new();
      };
      let mut buf = String::new();
      let _ = std::io::Read::read_to_string(&mut err, &mut buf);
      buf
    });

    let mut last_json_line: Option<String> = None;
    for line in BufReader::new(stdout).lines() {
      let line = line.map_err(|e| format!("读取子进程输出失败：{e}"))?;
      let t = line.trim();
      if t.starts_with('{') {
        last_json_line = Some(t.to_string());
      }
    }

    let status = child
      .wait()
      .map_err(|e| format!("等待子进程结束失败：{e}"))?;
    let stderr_body = stderr_thread.join().unwrap_or_default();

    if !status.success() {
      let hint = last_json_line.clone().unwrap_or_default();
      return Err(format!(
        "Midscene 子进程退出码 {:?}。stderr：{}。最后一行 JSON：{}",
        status.code(),
        stderr_body.trim(),
        hint
      ));
    }

    let line = last_json_line.ok_or_else(|| {
      format!(
        "子进程未输出结果 JSON。stderr：{}",
        stderr_body.trim()
      )
    })?;

    let parsed: MidsceneMinimalOk =
      serde_json::from_str(&line).map_err(|e| format!("解析子进程结果失败（{e}）：{line}"))?;

    if parsed.ok {
      let url = parsed.url.unwrap_or_default();
      let src = parsed.cdp_source.unwrap_or_else(|| "unknown".into());
      Ok(format!(
        "Midscene 探活成功：当前页 URL = {url}（CDP 来源：{src}）"
      ))
    } else {
      let err = parsed.error.unwrap_or_else(|| "未知错误".into());
      Err(err)
    }
  })
  .await
  .map_err(|e| format!("后台任务 Join 失败：{e}"))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  env_bootstrap::load_dotenv_files();
  env_bootstrap::apply_default_debug_port();

  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      run_midscene_minimal,
      check_cdp_reachable,
      check_cdp_devtools_json,
      get_runtime_config_summary
    ])
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
