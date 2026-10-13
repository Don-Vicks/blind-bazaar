#!/usr/bin/env bash
# Build the browser verifier (packages/bb-verify-wasm) and place it under apps/web/public/wasm.
# Needs: rustup target add wasm32-unknown-unknown; cargo install wasm-bindgen-cli
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -p bb-verify-wasm --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir apps/web/public/wasm --out-name bb_verify_wasm \
  target/wasm32-unknown-unknown/release/bb_verify_wasm.wasm
ls -la apps/web/public/wasm
