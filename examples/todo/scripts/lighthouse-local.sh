#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST_DIR="$ROOT_DIR/dist"
REPORT_DIR="$ROOT_DIR/lighthouse"
PORT="${PORT:-4173}"
URL="http://127.0.0.1:${PORT}/"

SERVER_PID=""
cleanup() {
  if [[ -n "$SERVER_PID" ]]; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

cd "$ROOT_DIR"
mkdir -p "$REPORT_DIR"

echo "[1/4] Building release bundle"
NO_COLOR=true trunk build --release

echo "[2/4] Starting static server on ${URL}"
python3 -m http.server "$PORT" --bind 127.0.0.1 --directory "$DIST_DIR" >/tmp/sparsh-todo-lighthouse-server.log 2>&1 &
SERVER_PID=$!

for _ in {1..20}; do
  if curl -sSf "$URL" >/dev/null 2>&1; then
    break
  fi
  sleep 0.5
done

if ! curl -sSf "$URL" >/dev/null 2>&1; then
  echo "Server did not become ready at ${URL}" >&2
  exit 1
fi

if command -v lighthouse >/dev/null 2>&1; then
  LH_CMD=(lighthouse)
else
  export NPM_CONFIG_CACHE="${NPM_CONFIG_CACHE:-$ROOT_DIR/.npm-cache}"
  mkdir -p "$NPM_CONFIG_CACHE"
  LH_CMD=(npx --yes lighthouse)
fi

CHROME_PATH="${CHROME_PATH:-}"
if [[ -z "$CHROME_PATH" ]]; then
  for candidate in \
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" \
    "/Applications/Chromium.app/Contents/MacOS/Chromium" \
    "$(command -v google-chrome || true)" \
    "$(command -v chromium || true)" \
    "$(command -v chromium-browser || true)"; do
    if [[ -n "$candidate" && -x "$candidate" ]]; then
      CHROME_PATH="$candidate"
      break
    fi
  done
fi

TIMESTAMP="$(date +"%Y%m%d-%H%M%S")"
REPORT_BASE="$REPORT_DIR/todo-$TIMESTAMP"

echo "[3/4] Running Lighthouse (mobile, performance+seo)"
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
)

if [[ -n "$CHROME_PATH" ]]; then
  LH_ARGS+=(--chrome-path="$CHROME_PATH")
fi

"${LH_CMD[@]}" "${LH_ARGS[@]}"

echo "[4/4] Collecting report outputs"
HTML_REPORT="$(ls -t "$REPORT_DIR"/todo-"$TIMESTAMP"*.html 2>/dev/null | head -n1 || true)"
JSON_REPORT="$(ls -t "$REPORT_DIR"/todo-"$TIMESTAMP"*.json 2>/dev/null | head -n1 || true)"

if [[ -z "$HTML_REPORT" || -z "$JSON_REPORT" ]]; then
  echo "Could not locate Lighthouse report files under $REPORT_DIR" >&2
  exit 1
fi

node -e 'const fs=require("fs"); const p=process.argv[1]; const r=JSON.parse(fs.readFileSync(p, "utf8")); const perf=Math.round((r.categories.performance.score ?? 0)*100); const seo=Math.round((r.categories.seo.score ?? 0)*100); console.log(`Performance: ${perf}`); console.log(`SEO: ${seo}`);' "$JSON_REPORT"

echo "HTML report: $HTML_REPORT"
echo "JSON report: $JSON_REPORT"
