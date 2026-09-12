#!/usr/bin/env bash
# Render VisiCalc on Compose Desktop to PNGs.
#
# VisiCalc is gated on swiftui and qt in CI and has no Compose lane, so there is
# no generated Compose project lying around to drop a test into -- unlike
# Trestle, whose screenshot harness (#14799) piggybacks on the project its CI
# lane already builds. This script does the whole thing: build the Rust
# runtime, generate the project, copy the harness in, run it.
#
# It exists because the alternative is doing those four steps by hand, which is
# how UI60 (#14828) was found and is not a thing that happens twice.
#
#   scripts/render-compose.sh [OUTPUT_DIR]
#
# Writes PNGs to OUTPUT_DIR (default: ./compose-shots) and prints their paths.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
app="$(dirname "$here")"
repo="$(cd "$app/../../../.." && pwd)"
shots="${1:-$app/compose-shots}"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

case "$(uname -s)" in
  Darwin) lib=libvisicalc_mosaic_app.dylib; resources=macos ;;
  *)      lib=libvisicalc_mosaic_app.so;    resources=linux ;;
esac

cargo build --manifest-path "$repo/code/packages/rust/Cargo.toml" -p visicalc-mosaic-app
runtime="$repo/code/packages/rust/target/debug/$lib"
test -f "$runtime"

cargo run --manifest-path "$repo/code/packages/rust/mosaic-compile/Cargo.toml" -- \
  pkg "$app" --backend compose --output "$work" --emit-project \
  --runtime-library "$runtime"

# VisiCalc is native-complete on Compose as of UI73 (#14843): the four
# `table-cell-role` entries were false positives (#14954) and the last real
# gap, wheel routing, is implemented. Asserting nativeComplete rather than
# pinning a count -- the pin existed only because there was something to pin.
python3 -c "
import json
d=json.load(open('$work/compose/mosaic-degradations.json'))
n=len(d['degradations'])
assert n == 0, f'expected VisiCalc to be native-complete on Compose, got {n}: ' + repr([x[\"code\"] for x in d['degradations']])
assert d.get('nativeComplete') is True, 'report does not claim nativeComplete'
"

mkdir -p "$work/compose/src/test/kotlin"
cp "$app/conformance/compose/VisiCalcScreenshots.kt" "$work/compose/src/test/kotlin/"

MOSAIC_SHOT_DIR="$shots" \
  JAVA_TOOL_OPTIONS="-Dcompose.application.resources.dir=$work/compose/app-resources/$resources" \
  gradle --no-daemon --rerun-tasks -p "$work/compose" test --tests VisiCalcScreenshots

ls -1 "$shots"/*.png
