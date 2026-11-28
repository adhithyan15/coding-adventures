#!/bin/sh
# Build the HWEB00 public API with the shared ISO C17 multi-compiler harness.
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"

BOOT="$SELF/../test-bootstrap"
JSON="$SELF/../bounded-json"
HARNESS="$SELF/../iso-harness"
if [ ! -f "$HARNESS/lib/iso-lib.sh" ]; then
    echo "historical-web: shared c/iso-harness was not found" >&2
    exit 1
fi

ISO_INCLUDE="include $BOOT/include $BOOT/src $BOOT/tests $JSON/include $HARNESS/include"
export ISO_INCLUDE
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    ISO_REQUIRE="gcc clang"
    export ISO_REQUIRE
fi
. "$HARNESS/lib/iso-lib.sh"

TEST_BOOTSTRAP_SOURCES="$BOOT/src/pbt_big.c $BOOT/src/pbt_prng.c $BOOT/src/pbt_val.c $BOOT/src/pbt_budget.c $BOOT/src/pbt_shrink.c $BOOT/src/pbt_gen.c $BOOT/src/pbt_runner.c $BOOT/src/pbt_json.c $BOOT/src/pbt_tfv.c $BOOT/src/pbt_corpus.c $BOOT/src/pbt_hooks.c $BOOT/src/fuzz_hooks.c $BOOT/src/fuzz_mutate.c $BOOT/src/fuzz_bitmap.c $BOOT/src/fuzz_sig.c $BOOT/src/fuzz_engine.c $BOOT/src/fuzz_corpus.c"
SOURCES="tests/historical_web_test.c $BOOT/tests/test_util.c src/html_tags_1992.c src/http09.c $TEST_BOOTSTRAP_SOURCES $JSON/src/bounded_json.c"
# Intentional word splitting expands source paths and the platform linker flag.
# shellcheck disable=SC2086
iso_build_and_run c historical-web-tests $SOURCES -lm
