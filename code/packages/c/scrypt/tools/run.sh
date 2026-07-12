#!/bin/sh
# Build and run the scrypt tests under EVERY available C/C++ compiler (pure ISO),
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

# Include this package's headers, the sibling KDF/hash chain (pbkdf2 -> hmac ->
# sha1/256/512), and the harness's (for iso_test.h).
ISO_INCLUDE="include ../pbkdf2/include ../hmac/include ../sha1/include ../sha256/include ../sha512/include $HARNESS/include"
export ISO_INCLUDE

# On Linux CI both gcc and clang are installed — require both so the pure-ISO
# guarantee is firm rather than best-effort. Locally, use whatever is present.
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    ISO_REQUIRE="gcc clang"
    export ISO_REQUIRE
fi

. "$HARNESS/lib/iso-lib.sh"
# The test links this package plus the pbkdf2 + hmac + SHA-1/256/512 sources.
iso_build_and_run c scrypt-tests tests/scrypt_test.c src/scrypt.c \
    ../pbkdf2/src/pbkdf2.c ../hmac/src/hmac.c ../sha1/src/sha1.c \
    ../sha256/src/sha256.c ../sha512/src/sha512.c
