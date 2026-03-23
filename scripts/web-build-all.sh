#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MODE="${1:-release}"
EXAMPLES=(
  counter
  layout-probe
  kitchen-sink
  fractal-clock
  hybrid-overlay
  showcase
  todo
)

for example in "${EXAMPLES[@]}"; do
  echo "[web-build-all] building ${example} (${MODE})"
  "$ROOT_DIR/scripts/web-build-example.sh" "$example" "$MODE"
done
