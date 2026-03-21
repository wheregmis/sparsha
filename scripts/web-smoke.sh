#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_DIR="${SPARSH_ARTIFACT_DIR:-$ROOT_DIR/artifacts}"
LOG_DIR="$ARTIFACT_DIR/web-smoke"
SHOWCASE_PORT="${SPARSH_SHOWCASE_PORT:-4176}"
SERVER_PIDS=()
export PLAYWRIGHT_BROWSERS_PATH="${PLAYWRIGHT_BROWSERS_PATH:-$ROOT_DIR/.playwright-browsers}"

mkdir -p "$LOG_DIR"

cleanup() {
  for pid in "${SERVER_PIDS[@]:-}"; do
    kill "$pid" >/dev/null 2>&1 || true
  done
}
trap cleanup EXIT

if [[ ! -d "$ROOT_DIR/node_modules/@playwright/test" ]]; then
  echo "Playwright dependencies are missing. Run 'npm install' from the repo root." >&2
  exit 1
fi

if [[ ! -d "$PLAYWRIGHT_BROWSERS_PATH" && -z "${CHROME_PATH:-}" ]]; then
  echo "No Playwright browser install found at $PLAYWRIGHT_BROWSERS_PATH." >&2
  echo "Run 'npm run web:install' or set CHROME_PATH to an existing browser binary." >&2
  exit 1
fi

"$ROOT_DIR/scripts/web-build-example.sh" showcase
"$ROOT_DIR/scripts/web-serve-dist.sh" showcase "$SHOWCASE_PORT" >"$LOG_DIR/showcase.log" 2>&1 &
SERVER_PIDS+=("$!")

for url in "http://127.0.0.1:${SHOWCASE_PORT}/"; do
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
done

export SPARSH_SHOWCASE_URL="http://127.0.0.1:${SHOWCASE_PORT}"

cd "$ROOT_DIR"
PLAYWRIGHT_TARGET=("tests/playwright/showcase.spec.ts")
if [[ "$#" -gt 0 ]]; then
  PLAYWRIGHT_TARGET=("$@")
fi

npx playwright test --config playwright.config.mjs "${PLAYWRIGHT_TARGET[@]}"
