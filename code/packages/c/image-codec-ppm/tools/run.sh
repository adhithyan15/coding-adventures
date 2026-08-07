#!/bin/sh
# Build and run the image-codec-ppm tests under EVERY available C/C++ compiler
# (pure ISO), via the shared iso-harness (code/packages/c/iso-harness). This crate
# is pure-ISO but COMPOSES c/pixel-container — it links nothing, but compiles that
# package's source (the RGBA8 PixelContainer) in.
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"

# Locate the repo root (the dir containing code/packages/c/iso-harness).
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

# Our headers, the harness's (iso_test.h), and pixel-container's.
ISO_INCLUDE="include $HARNESS/include $PIXEL/include"
export ISO_INCLUDE

# On Linux CI both gcc and clang are installed — require both so the pure-ISO
# guarantee is firm rather than best-effort. Locally, use whatever is present.
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    ISO_REQUIRE="gcc clang"
    export ISO_REQUIRE
fi

. "$HARNESS/lib/iso-lib.sh"
iso_build_and_run c image_codec_ppm-tests \
    tests/ppm_codec_test.c \
    src/ppm_codec.c \
    "$PIXEL/src/pixel_container.c"
