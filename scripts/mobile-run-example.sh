#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXAMPLE="${1:-}"
PLATFORM="${2:-}"
ACTION="${3:-run}"
MOBILE2_GIT_URL="https://github.com/tauri-apps/cargo-mobile2"

if [[ -z "$EXAMPLE" || -z "$PLATFORM" ]]; then
  echo "usage: $0 <example> <android|ios> [action]" >&2
  echo "example: $0 kitchen-sink android run" >&2
  echo "note: action is forwarded directly to cargo-mobile2 (for example: run, build, open)." >&2
  exit 1
fi

EXAMPLE_DIR="$ROOT_DIR/examples/$EXAMPLE"
if [[ ! -d "$EXAMPLE_DIR" ]]; then
  echo "unknown example: $EXAMPLE" >&2
  exit 1
fi

ensure_mobile2_subcommand() {
  local subcommand="$1"
  if ! cargo "$subcommand" --help >/dev/null 2>&1; then
    echo "cargo-mobile2 is required. Install it with:" >&2
    echo "  cargo install --git $MOBILE2_GIT_URL" >&2
    exit 1
  fi
}

ensure_mobile_project_initialized() {
  local platform="$1"
  local gen_dir
  case "$platform" in
    android) gen_dir="$EXAMPLE_DIR/gen/android" ;;
    ios) gen_dir="$EXAMPLE_DIR/gen/apple" ;;
    *)
      echo "unsupported platform: $platform (expected android or ios)" >&2
      exit 1
      ;;
  esac

  if [[ ! -d "$gen_dir" ]]; then
    echo "Initializing cargo-mobile2 project for $EXAMPLE..." >&2
    (
      cd "$EXAMPLE_DIR"
      cargo mobile init --non-interactive --skip-dev-tools
    )
  fi
}

case "$PLATFORM" in
  android)
    ensure_mobile2_subcommand mobile
    ensure_mobile2_subcommand android
    ensure_mobile_project_initialized android
    MOBILE_CMD=(cargo android "$ACTION")
    ;;
  ios)
    if [[ "$(uname -s)" != "Darwin" ]]; then
      echo "iOS builds require macOS (Darwin)." >&2
      exit 1
    fi
    ensure_mobile2_subcommand mobile
    ensure_mobile2_subcommand apple
    ensure_mobile_project_initialized ios
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
