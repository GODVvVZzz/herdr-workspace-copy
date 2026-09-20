#!/bin/sh
# Minimal fake Herdr CLI for local smoke tests of workspace-copy.
# Not used at runtime when a real herdr binary is on PATH / HERDR_BIN_PATH.
set -e
cmd="${1:-}"
sub="${2:-}"

case "$cmd $sub" in
  "pane get"|"pane list")
    cwd="${SMOKE_PANE_CWD:-/tmp}"
    printf '{"id":"smoke","result":{"pane":{"pane_id":"w1:p1","cwd":"%s","focused":true}}}\n' "$cwd"
    ;;
  "workspace create")
    printf '{"id":"smoke","result":{"workspace":{"workspace_id":"wSMOKE"},"type":"workspace_created"}}\n'
    ;;
  *)
    printf '{"id":"smoke","result":{}}\n'
    ;;
esac
