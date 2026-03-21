#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_DIR="${SPARSH_ARTIFACT_DIR:-$ROOT_DIR/artifacts}"

mkdir -p "$ARTIFACT_DIR"

cd "$ROOT_DIR"

echo "[release-readiness] foundation verification"
"$ROOT_DIR/scripts/verify-foundation.sh"

echo "[release-readiness] browser smoke suite"
"$ROOT_DIR/scripts/web-smoke.sh"

echo "[release-readiness] performance and startup smoke suite"
"$ROOT_DIR/scripts/perf-smoke.sh"
