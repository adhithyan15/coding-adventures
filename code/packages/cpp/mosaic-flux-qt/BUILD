# BUILD — build + test mosaic-flux-qt (POSIX sh, no cmake required).
#
# The build-tool runs this script via /bin/sh, so we avoid bashisms
# (no `set -euo pipefail`, no `[[ ]]`, no arrays).  CMake is the
# preferred build path for end users (see CMakeLists.txt + README),
# but CI runners may not have it installed.  This script just
# invokes the system compiler directly on each test_*.cpp.
#
# NOTE: build artefacts go to `_build/`, NOT `build/`, because
# macOS HFS+ is case-insensitive and `build/` collides with this
# `BUILD` script — `rm -rf build` would delete the script itself.
# This is the same trap that bit the mosaic-flux-compose Gradle
# setup (see repo-root lessons.md).
set -e

SELF_DIR=$(cd "$(dirname "$0")" && pwd)
cd "$SELF_DIR"

# Pick the first compiler that exists.
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
