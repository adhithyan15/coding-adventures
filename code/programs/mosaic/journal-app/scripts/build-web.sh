#!/usr/bin/env bash
# Build the Journal web host's generated inputs (J5c-2).
#
#   1. mosaic-compile emits JournalApp for React, in both themes, into
#      host/web/src/components/{light,dark} (git-ignored).
#   2. The Rust runtime (journal-mosaic-app) is built for wasm32 and copied
#      into host/web/public, where the page fetches it.
#
# JOURNAL_WASM_PROFILE=release builds the runtime optimised, for the published
# site (deploy-journal.yml, J5d). The default is debug: faster to build, and
# what the tests and the dev server use.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
WEB_DIR="$PACKAGE_DIR/host/web"
REPO_ROOT="$(cd "$PACKAGE_DIR/../../../.." && pwd)"
RUST="$REPO_ROOT/code/packages/rust"
OUT="$WEB_DIR/src/components"
mkdir -p "$OUT" "$WEB_DIR/public"
cargo build --manifest-path "$RUST/Cargo.toml" -p mosaic-compile
rustup target add wasm32-unknown-unknown
PROFILE="${JOURNAL_WASM_PROFILE:-debug}"
case "$PROFILE" in
  debug) PROFILE_FLAG=() ;;
  release) PROFILE_FLAG=(--release) ;;
  *) echo "JOURNAL_WASM_PROFILE must be debug or release, not '$PROFILE'" >&2; exit 2 ;;
esac
cargo build --manifest-path "$RUST/Cargo.toml" -p journal-mosaic-app --target wasm32-unknown-unknown ${PROFILE_FLAG[@]+"${PROFILE_FLAG[@]}"}
for THEME in light dark; do
  "$RUST/target/debug/mosaic-compile" pkg "$PACKAGE_DIR" \
    --backend react --theme "$THEME" --output "$OUT/$THEME"
done
cp "$RUST/target/wasm32-unknown-unknown/$PROFILE/journal_mosaic_app.wasm" "$WEB_DIR/public/journal_mosaic_app.wasm"
