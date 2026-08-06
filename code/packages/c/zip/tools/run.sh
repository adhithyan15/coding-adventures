#!/bin/sh
# Build and run the zip tests under EVERY available C/C++ compiler (pure ISO),
# via the shared iso-harness (code/packages/c/iso-harness).
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"

# Locate the iso-harness by walking up to the repo dir that contains it.
d="$SELF"
while [ "$d" != "/" ] && [ ! -d "$d/code/packages/c/iso-harness" ]; do
    d=$(dirname "$d")
done
HARNESS="$d/code/packages/c/iso-harness"
if [ ! -f "$HARNESS/lib/iso-lib.sh" ]; then
    echo "iso-harness not found (searched upward from $SELF)" >&2
    exit 1
fi

# Include this package's headers, the sibling deflate package's headers (RFC
# 1951 compress/decompress this package builds on), the sibling lzss
# package's headers (deflate.c's own dependency, needed to compile it in),
# and the harness's (for iso_test.h).
ISO_INCLUDE="include ../deflate/include ../lzss/include $HARNESS/include"
export ISO_INCLUDE

# On Linux CI both gcc and clang are installed — require both so the pure-ISO
# guarantee is firm rather than best-effort. Locally, use whatever is present.
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    ISO_REQUIRE="gcc clang"
    export ISO_REQUIRE
fi

. "$HARNESS/lib/iso-lib.sh"
# The test compiles the sibling deflate + lzss sources in directly (deflate
# for RFC 1951 compress/decompress; lzss because deflate.c depends on it).
iso_build_and_run c zip-tests tests/zip_test.c src/zip.c \
    ../deflate/src/deflate.c ../lzss/src/lzss.c
