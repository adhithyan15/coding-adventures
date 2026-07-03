#!/bin/sh
# run-tests.sh — build the Conduit C ABI, then compile & run the C++ tests.
#
# Invoked from `BUILD` as a single command (the build-tool feeds each BUILD line
# to its own `sh -c`, so multi-line constructs must live in this script).
#
# Steps:
#   1. cargo build the reusable conduit-capi static library.
#   2. Ask rustc for the platform's native-static-libs (the system libs a Rust
#      staticlib needs — e.g. -lSystem/-liconv on macOS, -lpthread/-ldl on Linux).
#   3. Compile each tests/test_*.cpp, linking libconduit_capi.a + those libs.
#
# Output dir is `_build/`, NOT `build/`: macOS HFS+ is case-insensitive and
# `build/` would collide with the `BUILD` script. See lessons.md.
set -e

SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"

CAPI_DIR=$(cd ../../rust/conduit-capi && pwd)
TARGET_REL=$(cd ../../rust && pwd)/target/release

# ── 1. Build the C ABI and capture its native-static-libs ────────────────────
# CARGO_TERM_COLOR=never: CI sets CARGO_TERM_COLOR=always, which makes rustc
# colorize the "native-static-libs:" note — the trailing ANSI reset would get
# captured into the link line (e.g. "-lm\033[0m" → ld: library 'm...' not found).
# The ESC-stripping sed is a defensive second layer.
echo "==> Building conduit-capi (release static lib)"
ESC=$(printf '\033')
NATIVE_LIBS=$(cd "$CAPI_DIR" && CARGO_TERM_COLOR=never cargo rustc --release \
    --crate-type staticlib -- --print native-static-libs 2>&1 \
    | sed -n 's/.*native-static-libs: //p' | tail -1 | sed "s/${ESC}\\[[0-9;]*m//g")
echo "==> native-static-libs: ${NATIVE_LIBS:-(none)}"

if [ ! -f "$TARGET_REL/libconduit_capi.a" ]; then
    echo "libconduit_capi.a not found at $TARGET_REL" >&2
    exit 1
fi

# ── 2. Pick a compiler ───────────────────────────────────────────────────────
if [ -n "${CXX:-}" ] && command -v "$CXX" >/dev/null 2>&1; then
    COMPILER="$CXX"
elif command -v clang++ >/dev/null 2>&1; then
    COMPILER=clang++
elif command -v g++ >/dev/null 2>&1; then
    COMPILER=g++
else
    echo "no C++ compiler found (need clang++ or g++)" >&2
    exit 1
fi
echo "==> Using compiler: $COMPILER"

mkdir -p _build
FLAGS="-std=c++17 -Wall -Wextra -Wpedantic -Werror -pthread \
    -I include -I tests -I $CAPI_DIR/include"
# Link the STATIC archive by full path (not -lconduit_capi): on Linux, ld prefers
# the sibling libconduit_capi.so (cdylib) over the .a when both sit in the search
# path, producing binaries that fail to load the .so at runtime. Naming the .a
# directly forces static linking portably (clang + g++). It must come after the
# source so left-to-right symbol resolution finds it.
LINK="$TARGET_REL/libconduit_capi.a $NATIVE_LIBS -pthread"

FAILED=0
TOTAL=0
PASSED=0
for src in tests/test_response.cpp tests/test_application.cpp tests/test_server.cpp; do
    name=$(basename "$src" .cpp)
    echo "==> Compiling $name"
    # shellcheck disable=SC2086
    "$COMPILER" $FLAGS "$src" -o "_build/$name" $LINK
    echo "==> Running $name"
    if "_build/$name"; then
        PASSED=$((PASSED + 1))
    else
        FAILED=$((FAILED + 1))
    fi
    TOTAL=$((TOTAL + 1))
done

echo ""
echo "Suites: $PASSED / $TOTAL passed, $FAILED failed"
[ "$FAILED" -eq 0 ] || exit 1
