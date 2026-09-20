use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, Default)]
pub struct PluginContext {
    pub workspace_id: Option<String>,
    pub workspace_label: Option<String>,
    pub workspace_cwd: Option<PathBuf>,
    pub focused_pane_cwd: Option<PathBuf>,
    pub focused_pane_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContextJson {
    #[serde(default)]
    workspace_id: Option<String>,
    #[serde(default)]
    workspace_label: Option<String>,
    #[serde(default)]
    workspace_cwd: Option<String>,
    #[serde(default)]
    focused_pane_cwd: Option<String>,
    #[serde(default)]
    focused_pane_id: Option<String>,
}

pub fn load_from_env() -> PluginContext {
    let mut ctx = PluginContext {
        workspace_id: std::env::var("HERDR_WORKSPACE_ID").ok().filter(|s| !s.is_empty()),
        workspace_label: None,
        workspace_cwd: None,
        focused_pane_cwd: None,
        focused_pane_id: std::env::var("HERDR_PANE_ID").ok().filter(|s| !s.is_empty()),
    };

    if let Ok(raw) = std::env::var("HERDR_PLUGIN_CONTEXT_JSON") {
        if let Ok(parsed) = serde_json::from_str::<ContextJson>(&raw) {
            if parsed.workspace_id.is_some() {
                ctx.workspace_id = parsed.workspace_id;
            }
            ctx.workspace_label = parsed.workspace_label;
            ctx.workspace_cwd = parsed.workspace_cwd.map(PathBuf::from);
            ctx.focused_pane_cwd = parsed.focused_pane_cwd.map(PathBuf::from);
            if parsed.focused_pane_id.is_some() {
                ctx.focused_pane_id = parsed.focused_pane_id;
            }
        }
    }

    ctx
}

pub fn default_exclude_names() -> Vec<String> {
    if let Ok(raw) = std::env::var("WORKSPACE_FORK_EXCLUDE") {
        let list: Vec<String> = raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !list.is_empty() {
            return list;
        }
    }

    [
        "node_modules",
        ".venv",
        "venv",
        "target",
        "dist",
        "build",
        ".cache",
        ".tmp",
        "__pycache__",
        ".next",
        ".turbo",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

pub fn include_git() -> bool {
    match std::env::var("WORKSPACE_FORK_INCLUDE_GIT") {
        Ok(v) => !matches!(v.as_str(), "0" | "false" | "FALSE" | "no"),
        Err(_) => true,
    }
}

pub fn name_template_suffix() -> String {
    std::env::var("WORKSPACE_FORK_NAME_SUFFIX").unwrap_or_else(|_| "-copy".into())
}

pub fn suggested_name(source: &Path) -> String {
    let base = source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "workspace".into());
    format!("{base}{}", name_template_suffix())
}
