#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_DIR="${SPARSH_ARTIFACT_DIR:-$ROOT_DIR/artifacts}"
LOG_DIR="$ARTIFACT_DIR/web-smoke"
SERVER_PIDS=()
export PLAYWRIGHT_BROWSERS_PATH="${PLAYWRIGHT_BROWSERS_PATH:-$ROOT_DIR/.playwright-browsers}"

mkdir -p "$LOG_DIR"

cleanup() {
  for pid in "${SERVER_PIDS[@]:-}"; do
    kill "$pid" >/dev/null 2>&1 || true
  done
}
trap cleanup EXIT

declare -A EXAMPLE_PORTS=(
  [showcase]="${SPARSH_SHOWCASE_PORT:-4176}"
  [todo]="${SPARSH_TODO_PORT:-4175}"
  [kitchen-sink]="${SPARSH_KITCHEN_SINK_PORT:-4173}"
  [hybrid-overlay]="${SPARSH_HYBRID_OVERLAY_PORT:-4174}"
  [counter]="${SPARSH_COUNTER_PORT:-4177}"
  [layout-probe]="${SPARSH_LAYOUT_PROBE_PORT:-4178}"
)

declare -A EXAMPLE_URL_VARS=(
  [showcase]="SPARSH_SHOWCASE_URL"
  [todo]="SPARSH_TODO_URL"
  [kitchen-sink]="SPARSH_KITCHEN_SINK_URL"
  [hybrid-overlay]="SPARSH_HYBRID_OVERLAY_URL"
  [counter]="SPARSH_COUNTER_URL"
  [layout-probe]="SPARSH_LAYOUT_PROBE_URL"
)

declare -A EXAMPLE_SPEC_FILES=(
  [showcase]="tests/playwright/showcase.spec.ts"
  [todo]="tests/playwright/todo.spec.ts"
  [kitchen-sink]="tests/playwright/kitchen-sink.spec.ts"
  [hybrid-overlay]="tests/playwright/hybrid-overlay.spec.ts"
  [counter]="tests/playwright/counter.spec.ts"
  [layout-probe]="tests/playwright/layout-probe.spec.ts"
)

if [[ ! -d "$ROOT_DIR/node_modules/@playwright/test" ]]; then
  echo "Playwright dependencies are missing. Run 'npm install' from the repo root." >&2
  exit 1
fi

if [[ ! -d "$PLAYWRIGHT_BROWSERS_PATH" && -z "${CHROME_PATH:-}" ]]; then
  echo "No Playwright browser install found at $PLAYWRIGHT_BROWSERS_PATH." >&2
  echo "Run 'npm run web:install' or set CHROME_PATH to an existing browser binary." >&2
  exit 1
fi

for example in showcase todo kitchen-sink hybrid-overlay counter layout-probe; do
  echo "[web-smoke] building ${example}"
  "$ROOT_DIR/scripts/web-build-example.sh" "$example"
  port="${EXAMPLE_PORTS[$example]}"
  echo "[web-smoke] serving ${example} on ${port}"
  "$ROOT_DIR/scripts/web-serve-dist.sh" "$example" "$port" >"$LOG_DIR/${example}.log" 2>&1 &
  SERVER_PIDS+=("$!")
done

for example in showcase todo kitchen-sink hybrid-overlay counter layout-probe; do
  port="${EXAMPLE_PORTS[$example]}"
  url="http://127.0.0.1:${port}/"
  for _ in {1..20}; do
    if curl -sSf "$url" >/dev/null 2>&1; then
      break
    fi
    sleep 0.5
  done
  if ! curl -sSf "$url" >/dev/null 2>&1; then
    echo "server did not become ready at $url" >&2
    exit 1
  fi
  export "${EXAMPLE_URL_VARS[$example]}"="$url"
done

cd "$ROOT_DIR"
PLAYWRIGHT_TARGET=("tests/playwright/showcase.spec.ts")
if [[ "$#" -gt 0 ]]; then
  PLAYWRIGHT_TARGET=("$@")
else
  PLAYWRIGHT_TARGET=(
    "${EXAMPLE_SPEC_FILES[showcase]}"
    "${EXAMPLE_SPEC_FILES[todo]}"
    "${EXAMPLE_SPEC_FILES[kitchen-sink]}"
    "${EXAMPLE_SPEC_FILES[hybrid-overlay]}"
    "${EXAMPLE_SPEC_FILES[counter]}"
    "${EXAMPLE_SPEC_FILES[layout-probe]}"
  )
fi

npx playwright test --config playwright.config.mjs "${PLAYWRIGHT_TARGET[@]}"
