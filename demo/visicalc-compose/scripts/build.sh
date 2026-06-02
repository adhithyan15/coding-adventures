#!/usr/bin/env bash
# build.sh — placeholder for the eventual Compose emitter wire-up.
#
# Today there is no `mosaic-compile --backend compose`, so this
# script is a no-op that documents the path.  When the Compose
# emitter lands, this will mirror the sibling backends' build.sh:
# run `mosaic-compile --backend compose` against the Mosaic
# component sources and drop the generated Kotlin into
# `src/main/kotlin/generated/`.
#
# Usage:
#   bash scripts/build.sh
#
# Then to run the demo:
#   ./gradlew run

set -euo pipefail

echo "build.sh — no-op for v0.1.0."
echo ""
echo "  mosaic-emit-compose does not yet exist; FormulaBar.kt and"
echo "  Grid.kt under src/main/kotlin/ are hand-written placeholders."
echo "  Follow the cross-backend demo plan to see what the generated"
echo "  output is expected to look like."
echo ""
echo "To run the demo:"
echo "  ./gradlew run"
