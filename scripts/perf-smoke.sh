#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_DIR="${SPARSH_ARTIFACT_DIR:-$ROOT_DIR/artifacts}"
PERF_DIR="$ARTIFACT_DIR/perf"
LOG_FILE="$PERF_DIR/rust-perf-smoke.log"

mkdir -p "$PERF_DIR"
: > "$LOG_FILE"

run_perf_test() {
  local crate="$1"
  local filter="$2"
  echo "[perf-smoke] cargo test -p ${crate} --release ${filter}" | tee -a "$LOG_FILE"
  cargo test -p "$crate" --release "$filter" -- --ignored --nocapture --test-threads=1 | tee -a "$LOG_FILE"
}

cd "$ROOT_DIR"

run_perf_test sparsha-layout perf_smoke
run_perf_test sparsha-text perf_smoke
run_perf_test sparsha-render perf_smoke

echo "[perf-smoke] web startup smoke" | tee -a "$LOG_FILE"
"$ROOT_DIR/scripts/web-perf-smoke.sh" | tee -a "$LOG_FILE"
