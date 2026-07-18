#!/bin/sh
# Build and run the mosfet-models tests under EVERY available C/C++ compiler (pure
# ISO), via the shared iso-harness. Composes c/device-physics (thermal voltage)
# and c/float-math (sqrt/exp) — links nothing, but compiles their sources in.
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
DEVPHYS="$d/code/packages/c/device-physics"
FMATH="$d/code/packages/c/float-math"

ISO_INCLUDE="include $HARNESS/include $DEVPHYS/include $FMATH/include"
export ISO_INCLUDE

if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    ISO_REQUIRE="gcc clang"
    export ISO_REQUIRE
fi

. "$HARNESS/lib/iso-lib.sh"
iso_build_and_run c mosfet_models-tests \
    tests/mosfet_models_test.c \
    src/mosfet_models.c \
    "$DEVPHYS/src/device_physics.c" \
    "$FMATH/src/float_math.c"
