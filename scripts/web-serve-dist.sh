#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXAMPLE="${1:-}"
PORT="${2:-4173}"

if [[ -z "$EXAMPLE" ]]; then
  echo "usage: $0 <example> [port]" >&2
  exit 1
fi

DIST_DIR="$ROOT_DIR/examples/$EXAMPLE/dist"
if [[ ! -d "$DIST_DIR" ]]; then
  echo "missing built dist for example '$EXAMPLE' at $DIST_DIR" >&2
  echo "run ./scripts/web-build-example.sh $EXAMPLE first" >&2
  exit 1
fi

exec python3 -m http.server "$PORT" --bind 127.0.0.1 --directory "$DIST_DIR"
