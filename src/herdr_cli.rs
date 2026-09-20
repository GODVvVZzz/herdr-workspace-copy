use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use serde_json::Value;

use crate::context::PluginContext;

#[derive(Debug, Deserialize)]
struct Envelope {
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<Value>,
}

fn herdr_bin() -> String {
    std::env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".into())
}

fn run_herdr(args: &[&str]) -> Result<String, String> {
    let bin = herdr_bin();
    let output = Command::new(&bin)
        .args(args)
        .output()
        .map_err(|e| format!("failed to spawn `{bin} {}`: {e}", args.join(" ")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        let mut msg = format!(
            "`{bin} {}` exited with {}",
            args.join(" "),
            output.status
        );
        if !stderr.trim().is_empty() {
            msg.push_str(&format!("\nstderr: {}", stderr.trim()));
        }
        if !stdout.trim().is_empty() {
            msg.push_str(&format!("\nstdout: {}", stdout.trim()));
        }
        return Err(msg);
    }

    if stdout.trim().is_empty() {
        return Err(format!("`{bin} {}` returned empty stdout", args.join(" ")));
    }
    Ok(stdout)
}

fn parse_envelope(raw: &str) -> Result<Value, String> {
    let env: Envelope =
        serde_json::from_str(raw).map_err(|e| format!("invalid herdr JSON response: {e}"))?;
    if let Some(err) = env.error {
        return Err(format!("herdr API error: {err}"));
    }
    env.result
        .ok_or_else(|| "herdr response missing result".to_string())
}

fn path_from_value(value: &Value) -> Option<PathBuf> {
    value
        .as_str()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

fn extract_pane_cwd(result: &Value) -> Option<PathBuf> {
    // pane.get → result.pane.{cwd,focused_pane_cwd,foreground_cwd}
    // also tolerate result.{cwd,...} or result.panes[0]
    let candidates = [
        result.pointer("/pane/cwd"),
        result.pointer("/pane/foreground_cwd"),
        result.pointer("/cwd"),
        result.pointer("/foreground_cwd"),
        result.pointer("/panes/0/cwd"),
        result.pointer("/panes/0/foreground_cwd"),
    ];
    for c in candidates {
        if let Some(path) = c.and_then(path_from_value) {
            return Some(path);
        }
    }

    // pane.list → result.panes[] prefer focused
    if let Some(panes) = result.get("panes").and_then(|p| p.as_array()) {
        let focused = panes.iter().find(|p| p.get("focused").and_then(|f| f.as_bool()) == Some(true));
        let pane = focused.or_else(|| panes.first())?;
        return path_from_value(pane.get("cwd").unwrap_or(&Value::Null))
            .or_else(|| path_from_value(pane.get("foreground_cwd").unwrap_or(&Value::Null)));
    }

    None
}

/// Resolve the workspace folder path using plugin context first, then Herdr CLI.
pub fn resolve_workspace_path(ctx: &PluginContext) -> Result<(PathBuf, String), String> {
    if let Some(cwd) = ctx
        .focused_pane_cwd
        .as_ref()
        .or(ctx.workspace_cwd.as_ref())
    {
        if cwd.is_absolute() {
            return Ok((cwd.clone(), "plugin context".into()));
        }
    }

    if let Some(pane_id) = ctx.focused_pane_id.as_deref() {
        let raw = run_herdr(&["pane", "get", pane_id])?;
        let result = parse_envelope(&raw)?;
        if let Some(path) = extract_pane_cwd(&result) {
            return Ok((path, format!("herdr pane get {pane_id}")));
        }
    }

    let raw = if let Some(ws) = ctx.workspace_id.as_deref() {
        run_herdr(&["pane", "list", "--workspace", ws])?
    } else {
        run_herdr(&["pane", "get", "--current"])?
    };
    let result = parse_envelope(&raw)?;
    if let Some(path) = extract_pane_cwd(&result) {
        return Ok((
            path,
            if ctx.workspace_id.is_some() {
                "herdr pane list --workspace".into()
            } else {
                "herdr pane get --current".into()
            },
        ));
    }

    Err(
        "could not resolve workspace folder path; focus a workspace and ensure Herdr is running"
            .into(),
    )
}

pub fn create_workspace(cwd: &Path, label: &str, focus: bool) -> Result<String, String> {
    let cwd_str = cwd.to_string_lossy().to_string();
    let mut args = vec![
        "workspace",
        "create",
        "--cwd",
        cwd_str.as_str(),
        "--label",
        label,
    ];
    if focus {
        args.push("--focus");
    } else {
        args.push("--no-focus");
    }
    let raw = run_herdr(&args)?;
    let result = parse_envelope(&raw)?;
    let workspace_id = result
        .pointer("/workspace/workspace_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "(unknown)".into());
    Ok(workspace_id)
}
