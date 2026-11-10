#!/usr/bin/env bash
#
# build-demo-wasm.sh — build the WebAssembly engines the browser demos use.
#
# Generated binaries are built, never committed (see code/specs/BUILD-001-
# generated-wasm-artifacts.md). After a fresh clone, or after changing any Rust
# crate an engine depends on, run this once and the demos work:
#
#   bash code/scripts/build-demo-wasm.sh              # everything below
#   bash code/scripts/build-demo-wasm.sh spreadsheet  # just one target
#   bash code/scripts/build-demo-wasm.sh --list       # print the targets
#
# Targets, and what each one produces (all ignored by version control):
#
#   spreadsheet  code/packages/rust/spreadsheet-wasm/pkg/spreadsheet_engine.wasm
#   visicalc     code/programs/typescript/visicalc-html/vendor/spreadsheet-engine-wasm.js
#                (the base64 bundle that visicalc-html, visicalc-webcomp and
#                visicalc-deno load; builds `spreadsheet` first if needed)
#   task         code/packages/rust/task-wasm/pkg/task_engine.wasm
#   engram       code/packages/rust/engram-wasm/pkg/engram_engine.wasm
#
# Each crate keeps its own build-wasm.sh as the one place that knows how to
# compile it; this script only adds the missing target and calls them in order.
# `visicalc` always rebuilds the bundle from the current .wasm.
#
# Tooling: cargo (and rustup, if the wasm32 target must be installed) and bash.
# Run through `mise exec -- bash code/scripts/build-demo-wasm.sh` if your
# toolchain comes from mise.

set -euo pipefail

if [ -d "$HOME/.cargo/bin" ] && ! command -v cargo >/dev/null 2>&1; then
  export PATH="$HOME/.cargo/bin:$PATH"
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CODE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RUST="$CODE_DIR/packages/rust"
VISICALC_HTML="$CODE_DIR/programs/typescript/visicalc-html"

ALL_TARGETS="spreadsheet visicalc task engram"

usage() {
  sed -n '3,/^set -euo/p' "${BASH_SOURCE[0]}" | sed '$d' | sed 's/^# \{0,1\}//'
}

case "${1:-}" in
  -h|--help) usage; exit 0 ;;
  --list) printf '%s\n' $ALL_TARGETS; exit 0 ;;
esac

targets=("$@")
if [ ${#targets[@]} -eq 0 ]; then
  # shellcheck disable=SC2206 # word-splitting the constant list is the point
  targets=($ALL_TARGETS)
fi
for t in "${targets[@]}"; do
  case " $ALL_TARGETS " in
    *" $t "*) ;;
    *) echo "build-demo-wasm.sh: unknown target '$t' (expected one of: $ALL_TARGETS)" >&2; exit 2 ;;
  esac
done

# Install the browser target when rustup is around; if it is not (a toolchain
# managed some other way), the cargo build below reports the missing target.
if command -v rustup >/dev/null 2>&1; then
  rustup target add wasm32-unknown-unknown
fi

built_spreadsheet=0
build_spreadsheet() {
  [ "$built_spreadsheet" -eq 1 ] && return 0
  echo "==> spreadsheet-wasm"
  bash "$RUST/spreadsheet-wasm/build-wasm.sh"
  built_spreadsheet=1
}

for t in "${targets[@]}"; do
  case "$t" in
    spreadsheet) build_spreadsheet ;;
    visicalc)
      build_spreadsheet
      echo "==> visicalc-html vendor bundle"
      bash "$VISICALC_HTML/scripts/bundle-wasm-engine.sh"
      ;;
    task)
      echo "==> task-wasm"
      bash "$RUST/task-wasm/build-wasm.sh"
      ;;
    engram)
      echo "==> engram-wasm"
      bash "$RUST/engram-wasm/build-wasm.sh"
      ;;
  esac
done

echo "Done: ${targets[*]}"
