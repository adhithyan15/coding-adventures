#!/bin/sh
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"
d="$SELF"
while [ "$d" != "/" ] && [ ! -d "$d/code/packages/c/iso-harness" ]; do d=$(dirname "$d"); done
HARNESS="$d/code/packages/c/iso-harness"
if [ ! -f "$HARNESS/lib/iso-lib.sh" ]; then echo "iso-harness not found" >&2; exit 1; fi
ISO_INCLUDE="include ../bounded-json/include ../iso-harness/include"
export ISO_INCLUDE
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then ISO_REQUIRE="gcc clang"; export ISO_REQUIRE; fi
. "$HARNESS/lib/iso-lib.sh"
iso_build_and_run c hdd-simulator-tests tests/hdd_simulator_test.c ../bounded-json/src/bounded_json.c
iso_build_and_run c hdd-simulator-public-smoke tests/hdd_public_smoke.c src/hdd_simulator.c
