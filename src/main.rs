mod context;
mod copy;
mod herdr_cli;
mod ui;

use std::path::PathBuf;

use crate::copy::CopyPlan;

fn print_help() {
    println!(
        "workspace-copy — duplicate a Herdr workspace folder to a sibling path

USAGE:
    workspace-copy duplicate [--yes] [--name NAME] [--dry-run]
    workspace-copy help

BEHAVIOR:
    Resolves the current workspace folder, asks for a new name (default: <folder>-copy),
    copies it to the same parent directory, then runs:
        herdr workspace create --cwd <target> --label <name> --focus

ENV:
    HERDR_BIN_PATH            Herdr CLI (default: herdr)
    HERDR_WORKSPACE_ID        Current workspace id (injected by Herdr plugins)
    HERDR_PLUGIN_CONTEXT_JSON Plugin context JSON (injected by Herdr plugins)
    WORKSPACE_FORK_EXCLUDE    Comma-separated dir names to skip
    WORKSPACE_FORK_INCLUDE_GIT  1/0, default 1 (copy .git)
    WORKSPACE_FORK_NAME_SUFFIX    Default suffix, default \"-copy\"

NOTES:
    Plugin id: workspace-copy
    Action:    workspace-copy.duplicate-workspace
    v0.1 does not show popup path details; paths are printed in the confirm log.
"
    );
}

fn build_plan(name_override: Option<String>, yes: bool) -> Result<(CopyPlan, String, context::PluginContext), String> {
    let ctx = context::load_from_env();
    let (source, via) = herdr_cli::resolve_workspace_path(&ctx)?;
    let source = source
        .canonicalize()
        .unwrap_or(source);

    let had_override = name_override.is_some();
    let mut suggested = name_override.unwrap_or_else(|| context::suggested_name(&source));
    // Plugin actions can't prompt for a new name; when the default target already
    // exists, bump the suffix (…-copy-2, …-copy-3) instead of failing validation.
    if !had_override && ui::running_as_plugin_action() {
        let base = source
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "workspace".into());
        let suffix = context::name_template_suffix();
        let mut n = 2;
        while sibling_dest(&source, &suggested).exists() {
            suggested = format!("{base}{suffix}-{n}");
            n += 1;
        }
    }
    let preview_dest = sibling_dest(&source, &suggested);
    let preview_plan = CopyPlan {
        source: source.clone(),
        dest: preview_dest.clone(),
        label: suggested.clone(),
        exclude: context::default_exclude_names(),
        include_git: context::include_git(),
    };

    let label = if yes {
        ui::print_plan(
            ctx.workspace_id.as_deref(),
            ctx.workspace_label.as_deref(),
            &preview_plan,
            &via,
        );
        suggested
    } else {
        ui::print_plan(
            ctx.workspace_id.as_deref(),
            ctx.workspace_label.as_deref(),
            &preview_plan,
            &via,
        );

        match ui::confirm_new_name(&suggested) {
            Ok(Some(name)) => name,
            Ok(None) => {
                ui::print_cancelled();
                std::process::exit(0);
            }
            Err(e) => return Err(format!("confirmation failed: {e}")),
        }
    };

    let dest = sibling_dest(&source, &label);
    let plan = CopyPlan {
        source,
        dest,
        label,
        exclude: context::default_exclude_names(),
        include_git: context::include_git(),
    };
    Ok((plan, via, ctx))
}

fn sibling_dest(source: &std::path::Path, new_name: &str) -> PathBuf {
    let parent = source
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    parent.join(new_name)
}

fn run_duplicate(args: &[String]) -> Result<(), String> {
    let mut yes = false;
    let mut dry_run = false;
    let mut name: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--yes" | "-y" => {
                yes = true;
                i += 1;
            }
            "--dry-run" | "-n" => {
                dry_run = true;
                i += 1;
            }
            "--name" => {
                let Some(v) = args.get(i + 1) else {
                    return Err("--name requires a value".into());
                };
                name = Some(v.clone());
                i += 2;
            }
            other => return Err(format!("unknown option: {other}")),
        }
    }

    if yes && name.is_none() {
        // non-interactive path still needs a name; use default suggested later
    }

    let (plan, via, ctx) = build_plan(name, yes)?;
    copy::validate_plan(&plan)?;

    if !yes {
        // Re-print final plan after name resolution
        ui::print_plan(
            ctx.workspace_id.as_deref(),
            ctx.workspace_label.as_deref(),
            &plan,
            &via,
        );
        if !dry_run {
            match ui::confirm_yes_no("Proceed with copy & open workspace?") {
                Ok(true) => {}
                Ok(false) => {
                    ui::print_cancelled();
                    return Ok(());
                }
                Err(e) => return Err(format!("confirmation failed: {e}")),
            }
        }
    } else {
        ui::print_plan(
            ctx.workspace_id.as_deref(),
            ctx.workspace_label.as_deref(),
            &plan,
            &via,
        );
    }

    if dry_run {
        println!("dry-run: would copy {} -> {}", plan.source.display(), plan.dest.display());
        println!("dry-run: would run herdr workspace create --cwd {} --label {} --focus",
            plan.dest.display(), plan.label);
        return Ok(());
    }

    println!("Copying…");
    let stats = copy::copy_workspace_folder(&plan)?;
    ui::print_stats(&stats);

    println!("Creating Herdr workspace…");
    let workspace_id = herdr_cli::create_workspace(&plan.dest, &plan.label, true)?;
    ui::print_result(&workspace_id, &plan.source, &plan.dest);
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(|s| s.as_str()).unwrap_or("duplicate");
    let rest = if args.is_empty() { &args[..] } else { &args[1..] };

    let result = match cmd {
        "duplicate" => run_duplicate(rest),
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => {
            eprintln!("workspace-copy: unknown command `{other}`");
            print_help();
            std::process::exit(2);
        }
    };

    if let Err(err) = result {
        eprintln!("workspace-copy: {err}");
        std::process::exit(1);
    }
}
