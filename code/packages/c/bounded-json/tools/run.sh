#!/bin/sh
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"
HARNESS="$SELF/../iso-harness"
ISO_INCLUDE="include $HARNESS/include"
export ISO_INCLUDE
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    ISO_REQUIRE="gcc clang"
    export ISO_REQUIRE
fi
. "$HARNESS/lib/iso-lib.sh"
iso_build_and_run c bounded-json-tests tests/json_test.c src/bounded_json.c
