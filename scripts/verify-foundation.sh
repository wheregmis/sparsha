#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT_DIR"
cargo check --workspace
cargo test --workspace
cargo check -p kitchen-sink -p fractal-clock -p hybrid-overlay -p todo --target wasm32-unknown-unknown
