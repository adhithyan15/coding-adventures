#!/bin/sh
# Build and run the http1 tests under EVERY available C/C++ compiler (pure ISO),
# via the shared iso-harness. Composes c/http-core for the head vocabulary —
# links nothing, but compiles that package's source in.
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"

d="$SELF"
while [ "$d" != "/" ] && [ ! -d "$d/code/packages/c/iso-harness" ]; do
    d=$(dirname "$d")
done
HARNESS="$d/code/packages/c/iso-harness"
if [ ! -f "$HARNESS/lib/iso-lib.sh" ]; then
    echo "iso-harness not found (searched upward from $SELF)" >&2
    exit 1
fi
HTTPCORE="$d/code/packages/c/http-core"

ISO_INCLUDE="include $HARNESS/include $HTTPCORE/include"
export ISO_INCLUDE

if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    ISO_REQUIRE="gcc clang"
    export ISO_REQUIRE
fi

. "$HARNESS/lib/iso-lib.sh"
iso_build_and_run c http1-tests \
    tests/http1_test.c \
    src/http1.c \
    "$HTTPCORE/src/http_core.c"
