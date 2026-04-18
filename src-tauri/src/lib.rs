use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde::Deserialize;
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
    let dev_candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/midscene-minimal.mjs");
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

/// 用户点击界面后由前端调用：在子进程中运行打包后的 Midscene 最小探活脚本（不自动执行）。
#[tauri::command]
async fn run_midscene_minimal(app: tauri::AppHandle) -> Result<String, String> {
  let script = resolve_midscene_script_path(&app)?;
  let node = find_node_binary().to_string();

  let cdp_ws = std::env::var("WSGW_CDP_WS_URL").ok().filter(|s| !s.trim().is_empty());
  let debug_port = std::env::var("WSGW_DEBUG_PORT").ok().filter(|s| !s.trim().is_empty());
  let demo_url = std::env::var("WSGW_DEMO_URL").ok().filter(|s| !s.trim().is_empty());

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
      format!(
        "无法启动 Node 子进程（{node}）：{e}。请确认已安装 Node.js 且已加入 PATH。"
      )
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
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![run_midscene_minimal])
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
