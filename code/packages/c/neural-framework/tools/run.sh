#!/bin/sh
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"
HARNESS="$SELF/../iso-harness"
ISO_INCLUDE="include ../matrix/include ../float-math/include ../bounded-json/include $HARNESS/include"
export ISO_INCLUDE
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    ISO_REQUIRE="gcc clang"
    export ISO_REQUIRE
fi
. "$HARNESS/lib/iso-lib.sh"
iso_build_and_run c neural-framework-tests tests/framework_test.c src/neural_framework.c ../matrix/src/matrix.c ../float-math/src/float_math.c ../bounded-json/src/bounded_json.c
