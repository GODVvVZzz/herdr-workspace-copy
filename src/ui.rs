use std::io::{self, BufRead, IsTerminal, Write};
use std::path::Path;

use crate::copy::{CopyPlan, CopyStats};

/// True when this process can prompt the user (real TTY stdin).
pub fn stdin_is_interactive() -> bool {
    io::stdin().is_terminal()
}

/// True when running as a Herdr plugin action (Herdr injects this).
pub fn running_as_plugin_action() -> bool {
    std::env::var_os("HERDR_PLUGIN_ACTION_ID").is_some()
        || std::env::var_os("HERDR_PLUGIN_ID").is_some()
}

pub fn print_plan(
    workspace_id: Option<&str>,
    label: Option<&str>,
    plan: &CopyPlan,
    source_via: &str,
) {
    println!("Workspace Copy · duplicate workspace folder");
    println!("────────────────────────────────────────────");
    println!(
        "workspace : {}",
        match (label, workspace_id) {
            (Some(l), Some(id)) => format!("{l} ({id})"),
            (Some(l), None) => l.to_string(),
            (None, Some(id)) => id.to_string(),
            (None, None) => "(current)".to_string(),
        }
    );
    println!("source    : {}", plan.source.display());
    println!("via       : {source_via}");
    println!("new name  : {}", plan.label);
    if plan.dest.as_os_str().is_empty() {
        // dest not resolved yet
    } else {
        println!("target    : {}", plan.dest.display());
    }
    println!(
        "rules     : exclude [{}]{}; {}",
        plan.exclude.join(", "),
        if plan.include_git {
            ""
        } else {
            " + .git"
        },
        if plan.include_git {
            "include .git"
        } else {
            "exclude .git"
        }
    );
    println!();
}

/// Interactive confirmation.
/// - Empty input → use suggested name
/// - `y` / `yes` → accept suggested name
/// - `n` / `no` / `q` → cancel
/// - Any other non-empty input → use as new folder name
/// Returns None if cancelled.
pub fn confirm_new_name(suggested: &str) -> io::Result<Option<String>> {
    // Herdr plugin actions are spawned without a TTY. Auto-accept the default
    // name so keybind/CLI action invoke works; custom names still work when
    // the binary is run interactively.
    if !stdin_is_interactive() || running_as_plugin_action() {
        println!(
            "non-interactive/plugin action: using default name `{suggested}`"
        );
        return Ok(Some(suggested.to_string()));
    }

    println!("New folder name [{suggested}]:");
    println!("  Enter = use default · y/yes = confirm · n/no = cancel · or type a new name");
    print!("> ");
    io::stdout().flush()?;

    let mut line = String::new();
    let n = io::stdin().lock().read_line(&mut line)?;
    if n == 0 {
        return Ok(Some(suggested.to_string()));
    }
    let input = line.trim();

    if input.is_empty() || input.eq_ignore_ascii_case("y") || input.eq_ignore_ascii_case("yes") {
        return Ok(Some(suggested.to_string()));
    }
    if input.eq_ignore_ascii_case("n")
        || input.eq_ignore_ascii_case("no")
        || input.eq_ignore_ascii_case("q")
        || input.eq_ignore_ascii_case("quit")
    {
        return Ok(None);
    }
    Ok(Some(input.to_string()))
}

pub fn confirm_yes_no(prompt: &str) -> io::Result<bool> {
    if !stdin_is_interactive() || running_as_plugin_action() {
        println!("{prompt} → auto-yes (non-interactive/plugin action)");
        return Ok(true);
    }
    println!("{prompt} [y/N]:");
    print!("> ");
    io::stdout().flush()?;
    let mut line = String::new();
    let n = io::stdin().lock().read_line(&mut line)?;
    if n == 0 {
        return Ok(false);
    }
    let input = line.trim();
    Ok(input.eq_ignore_ascii_case("y") || input.eq_ignore_ascii_case("yes"))
}

pub fn print_stats(stats: &CopyStats) {
    println!(
        "copied: {} files, {} dirs, {} skipped (~{} bytes)",
        stats.files_copied, stats.dirs_created, stats.skipped_entries, stats.bytes_copied
    );
}

pub fn print_result(workspace_id: &str, source: &Path, dest: &Path) {
    println!();
    println!("Workspace Copy · done");
    println!("─────────────────────");
    println!("new workspace_id : {workspace_id}");
    println!("source           : {}", source.display());
    println!("target           : {}", dest.display());
}

pub fn print_cancelled() {
    println!("Cancelled. Nothing copied.");
}
