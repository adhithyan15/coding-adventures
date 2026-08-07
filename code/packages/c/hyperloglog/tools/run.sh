#!/bin/sh
# Build and run the hyperloglog tests under EVERY available C/C++ compiler (pure
# ISO), via the shared iso-harness (code/packages/c/iso-harness). This crate is
# pure-ISO but COMPOSES two other pure-ISO packages — it links nothing, but it
# compiles their sources in: hf_fnv1a_64 from c/hash-functions and the from-scratch
# elementary math (fm_*) from c/float-math (so no libm is ever needed).
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
HASH="$d/code/packages/c/hash-functions"
FMATH="$d/code/packages/c/float-math"

# Our headers, the harness's (iso_test.h), and the two composed packages'.
ISO_INCLUDE="include $HARNESS/include $HASH/include $FMATH/include"
export ISO_INCLUDE

# On Linux CI both gcc and clang are installed — require both so the pure-ISO
# guarantee is firm rather than best-effort. Locally, use whatever is present.
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    ISO_REQUIRE="gcc clang"
    export ISO_REQUIRE
fi

. "$HARNESS/lib/iso-lib.sh"
iso_build_and_run c hyperloglog-tests \
    tests/hyperloglog_test.c \
    src/hyperloglog.c \
    "$HASH/src/hash_functions.c" \
    "$FMATH/src/float_math.c"
