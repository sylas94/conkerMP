#!/bin/sh
# The GitHub Pages site: deploy/build.sh's pages for a static host that serves no asset pack. Each
# player makes the pack from their own ROM in the browser (web/pack.js, crates/conker-pack-web), and
# the landing calls the game servers' web server on another host (API, CORS: README.md "GitHub Pages").
#   index.html, play.html, play.js, voice.js, chat.js, pack.js, pack-worker.js, sw.js, landing/*
#   v/<hash>/client.js, client_bg.wasm      the client, named by its content
#   x/<hash>/extract.js, extract_bg.wasm    the extractor, named by its content (it names the pack)
# Usage: API=https://game.example.org/ [WASM_OPT=/path/to/wasm-opt] [SITE=dir] sh deploy/pages.sh
# Needs: what web/build.sh needs, wasm-opt (binaryen), bun.
set -e
cd "$(dirname "$0")/.."
: "${API:?API=https://game.example.org/ : where the game servers' web server answers /api/}"
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-target-net}
wasm_opt=${WASM_OPT:-$(command -v wasm-opt || true)}
[ -x "$wasm_opt" ] || { echo "needs wasm-opt (binaryen): put it on PATH or set WASM_OPT" >&2; exit 1; }
command -v bun > /dev/null || { echo "needs bun" >&2; exit 1; }
WASM_OPT=$wasm_opt sh web/build.sh web
WASM_OPT=$wasm_opt sh web/build-pack.sh
(cd web/app && bun install --frozen-lockfile && VITE_API=$API bun run build)   # -> web/index.html, web/landing/
site=${SITE:-$CARGO_TARGET_DIR/pages}
sum() { cat "$@" | { sha256sum 2> /dev/null || shasum -a 256; } | cut -c1-12; }
client=$(sum web/dist/client.js web/dist/client_bg.wasm)
extract=$(sum web/dist/extract.js web/dist/extract_bg.wasm)
rm -rf "$site"
mkdir -p "$site/v/$client" "$site/x/$extract"
cp web/dist/client.js web/dist/client_bg.wasm "$site/v/$client/"
cp web/dist/extract.js web/dist/extract_bg.wasm "$site/x/$extract/"
cp -R web/index.html web/play.html web/voice.js web/chat.js web/pack-worker.js web/sw.js web/landing "$site/"
# As deploy/build.sh: every "./dist/client.js" moves to the client's folder, "./dist/extract.js" to
# the extractor's (pack.js names its pack's cache by it, so a new extractor makes the pack again).
sed "s|\"./dist/client.js\"|\"./v/$client/client.js\"|g" web/play.js > "$site/play.js"
sed "s|\"./dist/extract.js\"|\"./x/$extract/extract.js\"|g" web/pack.js > "$site/pack.js"
grep -q "\"./v/$client/client.js\"" "$site/play.js" || { echo "play.js: import not rewritten" >&2; exit 1; }
grep -q "\"./x/$extract/extract.js\"" "$site/pack.js" || { echo "pack.js: extractor not rewritten" >&2; exit 1; }
! grep -q '"./dist/' "$site/play.js" "$site/pack.js" || { echo "a dist/ path left" >&2; exit 1; }
# GitHub Pages: no Jekyll (it would hide nothing here, but it's slower), and no asset pack ever.
touch "$site/.nojekyll"
[ ! -e "$site/assets" ] || { echo "an assets/ folder in the site: the pack must never be published" >&2; exit 1; }
ls -lR "$site" | head -40
