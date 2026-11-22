#!/bin/sh
# Build and run the dsp-filters tests under EVERY available C++ compiler (pure ISO),
# via the shared iso-harness (code/packages/c/iso-harness).
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

ISO_INCLUDE="include $HARNESS/include"
export ISO_INCLUDE

if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    ISO_REQUIRE="g++ clang++"
    export ISO_REQUIRE
fi

. "$HARNESS/lib/iso-lib.sh"
iso_build_and_run cpp dsp_filters-tests tests/dsp_filters_test.cpp
