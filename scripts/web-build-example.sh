#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXAMPLE="${1:-}"
MODE="${2:-release}"
CARGO_HOME="${CARGO_HOME:-$ROOT_DIR/.cargo-home}"

if [[ -z "$EXAMPLE" ]]; then
  echo "usage: $0 <example> [release|debug]" >&2
  exit 1
fi

EXAMPLE_DIR="$ROOT_DIR/examples/$EXAMPLE"
if [[ ! -d "$EXAMPLE_DIR" ]]; then
  echo "unknown example: $EXAMPLE" >&2
  exit 1
fi

TRUNK_ARGS=(build)
if [[ "$MODE" != "debug" ]]; then
  TRUNK_ARGS+=(--release)
fi

mkdir -p "$CARGO_HOME"

(
  cd "$EXAMPLE_DIR"
  CARGO_HOME="$CARGO_HOME" NO_COLOR=true trunk "${TRUNK_ARGS[@]}"
)
