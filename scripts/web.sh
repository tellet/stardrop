#!/usr/bin/env bash
# Builds the browser version into web/, ready to serve as static files.
#   ./scripts/web.sh
# Needs the wasm target and wasm-bindgen-cli:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version 0.2.128
set -euo pipefail
cd "$(dirname "$0")/.."

# A wasm module keeps the file path of every panic site as plain data, which
# `strip` does not touch, so an unremapped build publishes the paths of the
# machine that built it. Rewrite them to names that say nothing about it.
export RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=$HOME/.cargo/registry/src=/crates --remap-path-prefix=$HOME/.rustup/toolchains=/rust --remap-path-prefix=$HOME=/home/builder"

echo "==> building for the browser (release)"
cargo build --release --target wasm32-unknown-unknown

echo "==> wasm-bindgen"
wasm-bindgen \
    --target web \
    --no-typescript \
    --out-dir web \
    target/wasm32-unknown-unknown/release/stardrop.wasm

raw=$(wc -c < web/stardrop_bg.wasm | awk '{printf "%.1f MB", $1/1024/1024}')
gz=$(gzip -c web/stardrop_bg.wasm | wc -c | awk '{printf "%.1f MB", $1/1024/1024}')
echo "==> web/ is ready: $raw raw, about $gz over the wire"
echo "    Serve it with any static server, e.g."
echo "    python3 -m http.server --directory web 8080"
