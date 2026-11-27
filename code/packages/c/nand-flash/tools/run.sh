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
iso_build_and_run c nand-flash-tests tests/nand_flash_test.c tests/nand_flash_fuzz_support.c src/nand_flash.c ../bounded-json/src/bounded_json.c
