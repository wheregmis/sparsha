#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXAMPLE="${1:-}"
PLATFORM="${2:-}"
ACTION="${3:-run}"
TEMPLATE_PACK="${4:-winit}"
MOBILE2_GIT_URL="https://github.com/tauri-apps/cargo-mobile2"

if [[ -z "$EXAMPLE" || -z "$PLATFORM" ]]; then
  echo "usage: $0 <example> <android|ios> [action] [winit|wgpu]" >&2
  echo "example: $0 kitchen-sink android run" >&2
  echo "note: action is forwarded directly to cargo-mobile2 (for example: run, build, open)." >&2
  exit 1
fi

EXAMPLE_DIR="$ROOT_DIR/examples/$EXAMPLE"
if [[ ! -d "$EXAMPLE_DIR" ]]; then
  echo "unknown example: $EXAMPLE" >&2
  exit 1
fi

case "$TEMPLATE_PACK" in
  winit|wgpu) ;;
  *)
    echo "unsupported template pack: $TEMPLATE_PACK (expected winit or wgpu)" >&2
    exit 1
    ;;
esac

ensure_mobile2_subcommand() {
  local subcommand="$1"
  if ! cargo "$subcommand" --help >/dev/null 2>&1; then
    echo "cargo-mobile2 is required. Install it with:" >&2
    echo "  cargo install --git $MOBILE2_GIT_URL" >&2
    exit 1
  fi
}

ensure_mobile_config_template() {
  local config_file="$EXAMPLE_DIR/cargo-mobile2.toml"
  if [[ ! -f "$config_file" ]]; then
    local identifier_suffix
    identifier_suffix="${EXAMPLE//-/_}"
    cat >"$config_file" <<EOF
[app]
name = "$EXAMPLE"
identifier = "com.example.${identifier_suffix}"
template-pack = "$TEMPLATE_PACK"
EOF
    return
  fi

  if grep -q '^[[:space:]]*\[app\][[:space:]]*$' "$config_file"; then
    awk -v template="$TEMPLATE_PACK" '
      BEGIN { in_app = 0; has_template = 0 }
      /^[[:space:]]*\[/ {
        if (in_app && !has_template) {
          print "template-pack = \"" template "\""
        }
        in_app = ($0 ~ /^[[:space:]]*\[app\][[:space:]]*$/)
      }
      {
        if (in_app && $0 ~ /^[[:space:]]*template-pack[[:space:]]*=/) {
          sub(/=.*/, "= \"" template "\"")
          has_template = 1
        }
        print
      }
      END {
        if (in_app && !has_template) {
          print "template-pack = \"" template "\""
        }
      }
    ' "$config_file" >"$config_file.tmp"
    mv "$config_file.tmp" "$config_file"
  else
    echo "warning: could not find [app] in cargo-mobile2.toml; keeping existing template-pack settings." >&2
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
    ensure_mobile_config_template
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
