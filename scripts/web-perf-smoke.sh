#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ARTIFACT_DIR="${SPARSH_ARTIFACT_DIR:-$ROOT_DIR/artifacts}"
REPORT_DIR="$ARTIFACT_DIR/lighthouse"
PORT="${SPARSH_WEB_PERF_PORT:-4180}"
URL="http://127.0.0.1:${PORT}/"
PLAYWRIGHT_BROWSERS_PATH="${PLAYWRIGHT_BROWSERS_PATH:-$ROOT_DIR/.playwright-browsers}"

SERVER_PID=""
cleanup() {
  if [[ -n "$SERVER_PID" ]]; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

find_chrome_path() {
  local candidates=(
    "${CHROME_PATH:-}"
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
    "/Applications/Chromium.app/Contents/MacOS/Chromium"
    "$(command -v google-chrome 2>/dev/null || true)"
    "$(command -v chromium 2>/dev/null || true)"
    "$(command -v chromium-browser 2>/dev/null || true)"
  )

  for candidate in "${candidates[@]}"; do
    if [[ -n "$candidate" && -x "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  if [[ -d "$PLAYWRIGHT_BROWSERS_PATH" ]]; then
    while IFS= read -r candidate; do
      if [[ -x "$candidate" ]]; then
        printf '%s\n' "$candidate"
        return 0
      fi
    done < <(find "$PLAYWRIGHT_BROWSERS_PATH" -type f \( \
      -path "*/chrome-linux/chrome" -o \
      -path "*/chrome-mac/Chromium.app/Contents/MacOS/Chromium" -o \
      -path "*/chrome-mac/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing" \
    \) 2>/dev/null | sort)
  fi

  return 1
}

cd "$ROOT_DIR"
mkdir -p "$REPORT_DIR"

echo "[web-perf-smoke] building todo web bundle"
"$ROOT_DIR/scripts/web-build-example.sh" todo release

echo "[web-perf-smoke] starting static server on ${URL}"
"$ROOT_DIR/scripts/web-serve-dist.sh" todo "$PORT" >"$REPORT_DIR/todo-server.log" 2>&1 &
SERVER_PID=$!

for _ in {1..20}; do
  if curl -sSf "$URL" >/dev/null 2>&1; then
    break
  fi
  sleep 0.5
done

if ! curl -sSf "$URL" >/dev/null 2>&1; then
  echo "server did not become ready at ${URL}" >&2
  exit 1
fi

if command -v lighthouse >/dev/null 2>&1; then
  LH_CMD=(lighthouse)
else
  export NPM_CONFIG_CACHE="${NPM_CONFIG_CACHE:-$ROOT_DIR/.npm-cache}"
  mkdir -p "$NPM_CONFIG_CACHE"
  LH_CMD=(npx --yes lighthouse)
fi

CHROME_PATH="$(find_chrome_path || true)"
if [[ -z "$CHROME_PATH" ]]; then
  echo "could not locate a Chromium/Chrome binary; set CHROME_PATH or install Playwright browsers" >&2
  exit 1
fi

TIMESTAMP="$(date +"%Y%m%d-%H%M%S")"
REPORT_BASE="$REPORT_DIR/todo-$TIMESTAMP"

echo "[web-perf-smoke] running Lighthouse against ${URL}"
LH_ARGS=(
  "$URL"
  --preset=perf
  --form-factor=mobile
  --screenEmulation.mobile=true
  --throttling-method=simulate
  --only-categories=performance,seo
  --chrome-flags="--headless=new --disable-gpu --no-sandbox"
  --output=html
  --output=json
  --output-path="$REPORT_BASE"
  --chrome-path="$CHROME_PATH"
)

"${LH_CMD[@]}" "${LH_ARGS[@]}"

HTML_REPORT="$(ls -t "$REPORT_DIR"/todo-"$TIMESTAMP"*.html 2>/dev/null | head -n1 || true)"
JSON_REPORT="$(ls -t "$REPORT_DIR"/todo-"$TIMESTAMP"*.json 2>/dev/null | head -n1 || true)"

if [[ -z "$HTML_REPORT" || -z "$JSON_REPORT" ]]; then
  echo "could not locate Lighthouse report files under $REPORT_DIR" >&2
  exit 1
fi

node -e 'const fs=require("fs"); const p=process.argv[1]; const r=JSON.parse(fs.readFileSync(p, "utf8")); const perf=Math.round((r.categories.performance.score ?? 0)*100); const seo=Math.round((r.categories.seo.score ?? 0)*100); console.log(`Performance: ${perf}`); console.log(`SEO: ${seo}`);' "$JSON_REPORT"

echo "HTML report: $HTML_REPORT"
echo "JSON report: $JSON_REPORT"
