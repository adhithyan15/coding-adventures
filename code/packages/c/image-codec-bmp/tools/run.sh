#!/bin/sh
# Build and run the image-codec-bmp tests under EVERY available C/C++ compiler
# (pure ISO), via the shared iso-harness (code/packages/c/iso-harness). Composes
# c/pixel-container — links nothing, but compiles that package's source in.
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
PIXEL="$d/code/packages/c/pixel-container"

ISO_INCLUDE="include $HARNESS/include $PIXEL/include"
export ISO_INCLUDE

if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    ISO_REQUIRE="gcc clang"
    export ISO_REQUIRE
fi

. "$HARNESS/lib/iso-lib.sh"
iso_build_and_run c image_codec_bmp-tests \
    tests/bmp_codec_test.c \
    src/bmp_codec.c \
    "$PIXEL/src/pixel_container.c"
