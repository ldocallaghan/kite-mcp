#!/usr/bin/env bash
# Run the evaluation questions through Claude Code, headless, with kite-mcp
# as its only MCP server and no built-in tools, then sanitise the transcripts
# and score them.
#
# Usage: eval/run.sh [question-id ...]     (default: all questions)
# EVAL_DIR picks the question set (default eval; the held-out set is eval/heldout).
# Needs: the Claude Code CLI (`claude`), and kite-mcp built (cargo build --release).
# KITE_MCP_COMMAND overrides the server command (default: target/release/kite-mcp).
set -euo pipefail
cd "$(dirname "$0")/.."
dir="${EVAL_DIR:-eval}"
server="${KITE_MCP_COMMAND:-$PWD/target/release/kite-mcp}"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
printf '{"mcpServers":{"kite":{"command":"%s","args":["serve"]}}}\n' "$server" > "$work/mcp.json"
tools="mcp__kite__list_models,mcp__kite__describe_baseline,mcp__kite__generate_baseline,mcp__kite__run_scenario,mcp__kite__get_table,mcp__kite__compare_runs"
mkdir -p "$dir/transcripts"

cargo run -q --example score_eval -- questions "$dir" | while IFS=$'\t' read -r id question; do
  if [ "$#" -gt 0 ] && ! printf '%s\n' "$@" | grep -qx "$id"; then continue; fi
  echo "running $id"
  # an empty working directory, so that no project instructions are loaded
  (cd "$work" && claude -p "/mcp__kite__analyse_trade_policy_scenario $question" \
      --tools "" --mcp-config "$work/mcp.json" --strict-mcp-config \
      --allowedTools "$tools" --output-format stream-json --verbose \
      --no-session-persistence) > "$work/$id.raw.jsonl"
  cargo run -q --example score_eval -- sanitize "$work/$id.raw.jsonl" "$dir/transcripts/$id.jsonl"
done
cargo run -q --example score_eval -- score "$dir"
