#!/usr/bin/env bash
#
# bundle-engine.sh — bundle the generic spreadsheet engine to a single
# browser-ready IIFE for the *live* VisiCalc HTML demo.
#
# Why this exists
# ---------------
# index.html used to be a STATIC SNAPSHOT: a compiled Grid template hydrated
# with hard-coded sample data, frozen, clicking did nothing.  This script is
# the other half of making it LIVE — it bundles
#
#     code/packages/typescript/spreadsheet-engine
#         (+ its CAS / excel-parser dependency closure)
#
# into `vendor/spreadsheet-engine.js`, a self-contained IIFE that exposes the
# package's public API on the global `SpreadsheetEngine`.  index.html loads it
# with a plain <script> tag, so the demo still opens directly from disk via
# file:// — no server, no module/CORS dance.
#
# The output is committed (it lives in vendor/, not the git-ignored build/) so
# the demo runs out of the box.  Re-run this whenever the engine changes.
#
# Usage:
#   cd demo/visicalc-html
#   bash scripts/bundle-engine.sh
#
# Tooling note: this repo installs nothing globally — esbuild comes from mise.
# Run through `mise exec --` if esbuild isn't already on your PATH:
#   mise exec -- bash scripts/bundle-engine.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEMO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$DEMO_DIR/../.." && pwd)"

ENGINE_ENTRY="$REPO_ROOT/code/packages/typescript/spreadsheet-engine/src/index.ts"
OUT_DIR="$DEMO_DIR/vendor"
OUT="$OUT_DIR/spreadsheet-engine.js"

mkdir -p "$OUT_DIR"

BANNER='// AUTO-GENERATED — DO NOT EDIT.
// Browser bundle of @coding-adventures/spreadsheet-engine + its dependency
// closure (CAS + excel-parser), exposed on window.SpreadsheetEngine.
// Regenerate with: bash demo/visicalc-html/scripts/bundle-engine.sh'

echo "Bundling spreadsheet-engine -> $OUT"
npx --yes esbuild "$ENGINE_ENTRY" \
  --bundle \
  --format=iife \
  --global-name=SpreadsheetEngine \
  --platform=browser \
  --legal-comments=none \
  --banner:js="$BANNER" \
  --outfile="$OUT"

echo "Done:"
ls -la "$OUT"
