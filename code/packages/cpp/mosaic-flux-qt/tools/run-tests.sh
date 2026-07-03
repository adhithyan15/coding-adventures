#!/bin/sh
# run-tests.sh — POSIX shell runner for the mosaic-flux-qt test
# suite.  Invoked from `BUILD` as a single command because the
# repo's build-tool feeds each BUILD line to its own `sh -c`
# process — multi-line `if`/`for` constructs in BUILD itself fall
# apart.  This script encapsulates the multi-step logic so BUILD
# stays a one-liner.
#
# No cmake dependency: we compile each test_*.cpp directly with
# whatever system compiler is available.  cmake is the documented
# end-user build path (see CMakeLists.txt), but CI runners may
# not have it installed and shouldn't need it just to run tests.
#
# Output dir is `_build/`, NOT `build/`, because macOS HFS+ is
# case-insensitive and `build/` collides with the `BUILD` script
# itself.  See lessons.md.
set -e

SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"

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

mkdir -p _build
FLAGS="-std=c++17 -Wall -Wextra -Wpedantic -Werror -I include -I tests"
FAILED=0
TOTAL=0
PASSED=0

for src in tests/test_action.cpp tests/test_store.cpp \
           tests/test_middleware.cpp tests/test_selector.cpp \
           tests/test_devtools.cpp; do
    name=$(basename "$src" .cpp)
    echo "==> Compiling $name"
    "$COMPILER" $FLAGS "$src" -o "_build/$name"
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
