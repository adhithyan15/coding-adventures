#!/bin/sh
# Build the Conduit C ABI, compile the conduit-hello demo + its smoke test, and
# run the smoke test. Output dir _build/ (not build/) to avoid the BUILD collision.
set -e
SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"

PKG=$(cd ../../../packages/cpp/conduit && pwd)
CAPI_DIR=$(cd ../../../packages/rust/conduit-capi && pwd)
TARGET_REL=$(cd ../../../packages/rust && pwd)/target/release

echo "==> Building conduit-capi"
# CARGO_TERM_COLOR=never so the rustc note isn't colorized (CI forces color) —
# otherwise a trailing ANSI reset gets captured into the link line.
ESC=$(printf '\033')
NATIVE_LIBS=$(cd "$CAPI_DIR" && CARGO_TERM_COLOR=never cargo rustc --release \
    --crate-type staticlib -- --print native-static-libs 2>&1 \
    | sed -n 's/.*native-static-libs: //p' | tail -1 | sed "s/${ESC}\\[[0-9;]*m//g")

if [ -n "${CXX:-}" ] && command -v "$CXX" >/dev/null 2>&1; then COMPILER="$CXX";
elif command -v clang++ >/dev/null 2>&1; then COMPILER=clang++;
elif command -v g++ >/dev/null 2>&1; then COMPILER=g++;
else echo "no C++ compiler found" >&2; exit 1; fi

mkdir -p _build
FLAGS="-std=c++17 -Wall -Wextra -Wpedantic -Werror -pthread \
    -I src -I $PKG/include -I $CAPI_DIR/include"
# Link the static archive by full path so Linux's ld doesn't prefer the sibling
# .so (which then fails to load at runtime). Portable across clang + g++.
LINK="$TARGET_REL/libconduit_capi.a $NATIVE_LIBS -pthread"

echo "==> Compiling demo binary"
# shellcheck disable=SC2086
"$COMPILER" $FLAGS src/main.cpp -o _build/conduit-hello $LINK

echo "==> Compiling + running smoke test"
# shellcheck disable=SC2086
"$COMPILER" $FLAGS tests/smoke.cpp -o _build/smoke $LINK
_build/smoke
