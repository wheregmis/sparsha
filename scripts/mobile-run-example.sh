#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXAMPLE="${1:-}"
PLATFORM="${2:-}"
ACTION="${3:-run}"

if [[ -z "$EXAMPLE" || -z "$PLATFORM" ]]; then
  echo "usage: $0 <example> <android|ios> [cargo-mobile2-action]" >&2
  echo "example: $0 kitchen-sink android run" >&2
  exit 1
fi

EXAMPLE_DIR="$ROOT_DIR/examples/$EXAMPLE"
if [[ ! -d "$EXAMPLE_DIR" ]]; then
  echo "unknown example: $EXAMPLE" >&2
  exit 1
fi

case "$PLATFORM" in
  android)
    if ! cargo android --help >/dev/null 2>&1; then
      echo "cargo-mobile2 is required. Install it with:" >&2
      echo "  cargo install --git https://github.com/tauri-apps/cargo-mobile2" >&2
      exit 1
    fi
    MOBILE_CMD=(cargo android "$ACTION")
    ;;
  ios)
    if ! cargo apple --help >/dev/null 2>&1; then
      echo "cargo-mobile2 is required. Install it with:" >&2
      echo "  cargo install --git https://github.com/tauri-apps/cargo-mobile2" >&2
      exit 1
    fi
    if [[ "$(uname -s)" != "Darwin" ]]; then
      echo "iOS builds require macOS (Darwin)." >&2
      exit 1
    fi
    MOBILE_CMD=(cargo apple "$ACTION")
    ;;
  *)
    echo "unsupported platform: $PLATFORM (expected android or ios)" >&2
    exit 1
    ;;
esac

(
  cd "$EXAMPLE_DIR"
  "${MOBILE_CMD[@]}"
)
