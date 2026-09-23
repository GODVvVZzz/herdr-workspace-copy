use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CopyPlan {
    pub source: PathBuf,
    pub dest: PathBuf,
    pub label: String,
    pub exclude: Vec<String>,
    pub include_git: bool,
}

#[derive(Debug)]
pub struct CopyStats {
    pub files_copied: u64,
    pub dirs_created: u64,
    pub bytes_copied: u64,
    pub skipped_entries: u64,
}

fn is_excluded(name: &str, plan: &CopyPlan) -> bool {
    if name == ".git" {
        return !plan.include_git;
    }
    plan.exclude.iter().any(|e| e == name)
}

fn copy_dir_inner(
    src: &Path,
    dest: &Path,
    plan: &CopyPlan,
    stats: &mut CopyStats,
) -> Result<(), String> {
    let entries = fs::read_dir(src)
        .map_err(|e| format!("read_dir {}: {e}", src.display()))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("read_dir entry in {}: {e}", src.display()))?;
        let file_type = entry
            .file_type()
            .map_err(|e| format!("file_type {}: {e}", entry.path().display()))?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_string();
        let from = entry.path();
        let to = dest.join(&name);

        if is_excluded(&name_str, plan) {
            stats.skipped_entries += 1;
            continue;
        }

        if file_type.is_dir() {
            // Skip symlinked dirs' recursive walk by treating as file copy target if symlink
            if file_type.is_symlink() {
                copy_symlink_or_file(&from, &to, stats)?;
                continue;
            }
            fs::create_dir_all(&to)
                .map_err(|e| format!("create_dir {}: {e}", to.display()))?;
            stats.dirs_created += 1;
            copy_dir_inner(&from, &to, plan, stats)?;
        } else if file_type.is_symlink() {
            copy_symlink_or_file(&from, &to, stats)?;
        } else if file_type.is_file() {
            fs::copy(&from, &to)
                .map_err(|e| format!("copy {} -> {}: {e}", from.display(), to.display()))?;
            stats.files_copied += 1;
            if let Ok(meta) = fs::metadata(&from) {
                stats.bytes_copied += meta.len();
            }
        } else {
            stats.skipped_entries += 1;
        }
    }

    Ok(())
}

fn copy_symlink_or_file(from: &Path, to: &Path, stats: &mut CopyStats) -> Result<(), String> {
    // Recreate symlinks verbatim (cp -a style): fs::copy would follow the link
    // and fail on links pointing to directories or dangling targets.
    let meta = fs::symlink_metadata(from)
        .map_err(|e| format!("symlink_metadata {}: {e}", from.display()))?;
    if meta.file_type().is_symlink() {
        let target = fs::read_link(from)
            .map_err(|e| format!("read_link {}: {e}", from.display()))?;
        if fs::symlink_metadata(to).is_ok() {
            fs::remove_file(to).map_err(|e| format!("remove {}: {e}", to.display()))?;
        }
        std::os::unix::fs::symlink(&target, to)
            .map_err(|e| format!("symlink {} -> {}: {e}", to.display(), target.display()))?;
        stats.files_copied += 1;
        return Ok(());
    }
    match fs::copy(from, to) {
        Ok(n) => {
            stats.files_copied += 1;
            stats.bytes_copied += n;
            Ok(())
        }
        Err(e) => Err(format!("copy symlink/file {} -> {}: {e}", from.display(), to.display())),
    }
}

pub fn validate_plan(plan: &CopyPlan) -> Result<(), String> {
    if !plan.source.exists() {
        return Err(format!("source does not exist: {}", plan.source.display()));
    }
    if !plan.source.is_dir() {
        return Err(format!("source is not a directory: {}", plan.source.display()));
    }
    if plan.dest.exists() {
        return Err(format!(
            "target already exists: {}",
            plan.dest.display()
        ));
    }

    let source = plan.source.canonicalize().unwrap_or_else(|_| plan.source.clone());
    let dest_parent = plan
        .dest
        .parent()
        .ok_or_else(|| "target has no parent directory".to_string())?;
    // dest may not exist yet; canonicalize parent when possible
    let dest_abs = if dest_parent.exists() {
        dest_parent
            .canonicalize()
            .map(|p| p.join(plan.dest.file_name().unwrap_or_default()))
            .unwrap_or_else(|_| plan.dest.clone())
    } else {
        plan.dest.clone()
    };

    if dest_abs.starts_with(&source) {
        return Err(format!(
            "target {} is inside source {}; refusing recursive copy",
            dest_abs.display(),
            source.display()
        ));
    }

    if plan.label.trim().is_empty() {
        return Err("label must not be empty".into());
    }

    Ok(())
}

/// Copy source folder into dest. On failure, best-effort removes dest if we created it.
pub fn copy_workspace_folder(plan: &CopyPlan) -> Result<CopyStats, String> {
    validate_plan(plan)?;

    let dest = &plan.dest;
    let parent = dest
        .parent()
        .ok_or_else(|| "target has no parent directory".to_string())?;
    if !parent.exists() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create parent {}: {e}", parent.display()))?;
    }

    fs::create_dir_all(dest).map_err(|e| format!("create target {}: {e}", dest.display()))?;

    let mut stats = CopyStats {
        files_copied: 0,
        dirs_created: 1,
        bytes_copied: 0,
        skipped_entries: 0,
    };

    match copy_dir_inner(&plan.source, dest, plan, &mut stats) {
        Ok(()) => Ok(stats),
        Err(err) => {
            let _ = fs::remove_dir_all(dest);
            Err(format!("{err} (partial target cleaned up)"))
        }
    }
}
