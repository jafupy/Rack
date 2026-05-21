#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

cargo build \
  --manifest-path "$ROOT_DIR/Cargo.toml" \
  --release \
  --target wasm32-wasip1

cp "$ROOT_DIR/target/wasm32-wasip1/release/rack_hello_route.wasm" \
  "$ROOT_DIR/functions.wasm"

echo "built $ROOT_DIR/functions.wasm"

