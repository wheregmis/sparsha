#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v wasm-bindgen-test-runner >/dev/null 2>&1; then
  echo "wasm-bindgen-test-runner is required." >&2
  echo "Install it with 'cargo install wasm-bindgen-cli --version 0.2.114 --locked'." >&2
  exit 1
fi

if [[ -z "${CHROME_PATH:-}" ]]; then
  for candidate in chromium google-chrome chromium-browser; do
    if command -v "$candidate" >/dev/null 2>&1; then
      export CHROME_PATH="$(command -v "$candidate")"
      break
    fi
  done
fi

if [[ -z "${CHROME_PATH:-}" && -d "$ROOT_DIR/node_modules/playwright" ]]; then
  export CHROME_PATH="$(node -e 'const { chromium } = require("playwright"); process.stdout.write(chromium.executablePath())')"
fi

if [[ -z "${CHROMEDRIVER:-}" ]]; then
  if command -v chromedriver >/dev/null 2>&1; then
    export CHROMEDRIVER="$(command -v chromedriver)"
  fi
fi

if [[ -z "${CHROME_PATH:-}" ]]; then
  echo "No Chromium/Chrome executable found." >&2
  echo "Set CHROME_PATH or install Chromium (or npm install Playwright browsers)." >&2
  exit 1
fi

export CI="${CI:-1}"
export CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="${CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER:-wasm-bindgen-test-runner}"

echo "[wasm-browser-tests] CHROME_PATH=${CHROME_PATH}"
if [[ -n "${CHROMEDRIVER:-}" ]]; then
  echo "[wasm-browser-tests] CHROMEDRIVER=${CHROMEDRIVER}"
fi

cargo test -p sparsha --target wasm32-unknown-unknown --lib --tests "$@"
