#!/bin/sh
# Builds the in-browser extractor (crates/conker-pack-web) into web/dist/: extract.js + extract_bg.wasm,
# which pack-worker.js runs on the player's ROM (pack.js, README.md "Asset pack in the browser").
# Usage: web/build-pack.sh
# Needs: rustup target wasm32-unknown-unknown, wasm-bindgen-cli 0.2.129 (= the crate version).
# Optional: wasm-opt (binaryen; on PATH or WASM_OPT=path) makes it faster with -O3.
set -e
cd "$(dirname "$0")/.."
cargo build -p conker-pack-web --target wasm32-unknown-unknown --release
wasm-bindgen --target web --no-typescript --out-name extract --out-dir web/dist \
  "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/conker_pack_web.wasm"
wasm_opt=${WASM_OPT:-$(command -v wasm-opt || true)}
if [ -n "$wasm_opt" ]; then
  "$wasm_opt" -O3 --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
    --enable-mutable-globals --enable-reference-types --enable-multivalue \
    web/dist/extract_bg.wasm -o web/dist/extract_bg.wasm
fi
ls -l web/dist/extract*
