# Workspace Copy

[Herdr](https://herdr.dev) community plugin: copy a workspace folder to a sibling path and open it as a new Herdr workspace — without git worktree.

| | |
|---|---|
| Plugin id | `workspace-copy` |
| Action id | `duplicate-workspace` |
| Qualified action | `workspace-copy.duplicate-workspace` |
| Platforms | macOS, Linux |
| License | MIT |
| Herdr | ≥ 0.8.0 |

## Why

Herdr can open parallel contexts with git worktree. Sometimes you want something simpler: a full folder copy of the same project, as its own workspace, so agents or you can work without sharing one working tree.

This plugin is a small, single-purpose tool. It does **not** decide when you should copy a project — you or your agent decide.

## What it does

```text
focused workspace (e.g. folder workspace-a)
  → workspace-copy.duplicate-workspace
  → default new name: workspace-a-copy
  → copy to the same parent directory
  → herdr workspace create --cwd <target> --label <name> --focus
  → new workspace opens
```

Absolute `source` and `target` paths are printed before the copy. v0.1 does not add sidebar hover UI or a workspace context-menu item (plugin API limitation).

## Install

```bash
herdr plugin install <owner>/herdr-workspace-copy --yes
herdr plugin enable workspace-copy
```

Replace `<owner>` with the GitHub account that hosts this repository.

### Marketplace listing

After the repository is public:

1. Add the GitHub topic `herdr-plugin`
2. Keep a valid `herdr-plugin.toml` on the default branch  
3. Wait for the index refresh (~30 minutes) — see [Herdr plugins marketplace](https://herdr.dev/plugins/)

Listing is automatic and is **not** a security review.

### Local development

```bash
cargo build --release
herdr plugin link /absolute/path/to/herdr-workspace-copy
herdr plugin list
herdr plugin action list --plugin workspace-copy
```

`plugin link` does not run `[[build]]`; build yourself first. GitHub `plugin install` runs the manifest build commands after confirmation.

## Usage

Focus a workspace in Herdr, then:

```bash
herdr plugin action invoke workspace-copy.duplicate-workspace
```

Optional keybind (user config):

```toml
[[keys.command]]
key = "prefix+d"
type = "plugin_action"
command = "workspace-copy.duplicate-workspace"
description = "duplicate workspace folder"
```

Direct binary:

```bash
workspace-copy duplicate [--yes] [--name NAME] [--dry-run]
workspace-copy help
```

| Flag | Meaning |
|------|---------|
| `--yes` / `-y` | Non-interactive; use default (or `--name`) |
| `--name NAME` | Folder / workspace label for the copy |
| `--dry-run` | Print the plan; do not copy or create |

### Non-interactive behavior

Herdr plugin actions are spawned without a TTY. In that case (or when `HERDR_PLUGIN_ID` is set) the tool **auto-accepts the default name** and continues, and logs that choice. Use `--name` when you need a custom name from a script.

## Defaults

| Setting | Default |
|---------|---------|
| New folder name | `{source_folder_name}-copy` |
| Target path | `parent(source) / new_name` |
| Excluded directory names | `node_modules`, `.venv`, `venv`, `target`, `dist`, `build`, `.cache`, `.tmp`, `__pycache__`, `.next`, `.turbo` |
| `.git` | included |
| Existing target | refused |

### Environment

| Variable | Effect |
|----------|--------|
| `HERDR_BIN_PATH` | Herdr CLI (default: `herdr` on `PATH`) |
| `HERDR_WORKSPACE_ID` / `HERDR_PLUGIN_CONTEXT_JSON` | Injected by Herdr when run as a plugin action |
| `WORKSPACE_FORK_EXCLUDE` | Comma-separated directory names to skip |
| `WORKSPACE_FORK_INCLUDE_GIT` | `0` = do not copy `.git` (default: copy) |
| `WORKSPACE_FORK_NAME_SUFFIX` | Default suffix (default: `-copy`) |

## Safety

- Runs as your user; can read/write disk and call the full Herdr CLI.
- Review `herdr-plugin.toml` and the source before installing any plugin.
- Refuses to overwrite an existing target.
- Refuses a target path inside the source directory.
- On copy failure, tries to remove the partial target directory.
- Does not merge changes back into the original tree.

## Out of scope (v0.1)

- Sidebar hover / path tooltips  
- Native workspace context-menu items  
- Automatic merge-back  
- Scenario policy (“when you should fork”)  
- Windows-specific copy backend  

## Development notes

Smoke-test helper (optional): `scripts/herdr-stub.sh` — a tiny fake `herdr` CLI for local dry runs without a live server. Not required at runtime.

```bash
export HERDR_BIN_PATH="$PWD/scripts/herdr-stub.sh"
export HERDR_PLUGIN_CONTEXT_JSON='{"workspace_id":"w1","focused_pane_cwd":"/tmp/demo-project"}'
./target/release/workspace-copy duplicate --yes --dry-run
```

## License

MIT — see [LICENSE](LICENSE).
