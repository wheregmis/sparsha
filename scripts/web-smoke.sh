#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
KITCHEN_PORT="${SPARSH_KITCHEN_SINK_PORT:-4173}"
HYBRID_PORT="${SPARSH_HYBRID_OVERLAY_PORT:-4174}"
TODO_PORT="${SPARSH_TODO_PORT:-4175}"
SERVER_PIDS=()
export PLAYWRIGHT_BROWSERS_PATH="${PLAYWRIGHT_BROWSERS_PATH:-$ROOT_DIR/.playwright-browsers}"

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

"$ROOT_DIR/scripts/web-build-all.sh"

"$ROOT_DIR/scripts/web-serve-dist.sh" kitchen-sink "$KITCHEN_PORT" >/tmp/sparsh-kitchen-sink.log 2>&1 &
SERVER_PIDS+=("$!")
"$ROOT_DIR/scripts/web-serve-dist.sh" hybrid-overlay "$HYBRID_PORT" >/tmp/sparsh-hybrid-overlay.log 2>&1 &
SERVER_PIDS+=("$!")
"$ROOT_DIR/scripts/web-serve-dist.sh" todo "$TODO_PORT" >/tmp/sparsh-todo.log 2>&1 &
SERVER_PIDS+=("$!")

for url in \
  "http://127.0.0.1:${KITCHEN_PORT}/" \
  "http://127.0.0.1:${HYBRID_PORT}/" \
  "http://127.0.0.1:${TODO_PORT}/"; do
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

export SPARSH_KITCHEN_SINK_URL="http://127.0.0.1:${KITCHEN_PORT}"
export SPARSH_HYBRID_OVERLAY_URL="http://127.0.0.1:${HYBRID_PORT}"
export SPARSH_TODO_URL="http://127.0.0.1:${TODO_PORT}"

cd "$ROOT_DIR"
npx playwright test --config playwright.config.mjs "$@"
