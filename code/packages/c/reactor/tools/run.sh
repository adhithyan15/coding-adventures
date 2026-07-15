#!/bin/sh
# run.sh — build & run the reactor tests on Unix (macOS + Linux) via
# platform-harness. poll()/socketpair live in libc; no extra library. The
# Windows half (run.ps1, BUILD_windows) links ws2_32 for WSAPoll.
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"
d="$SELF"
while [ "$d" != "/" ] && [ ! -d "$d/code/packages/c/platform-harness" ]; do
    d=$(dirname "$d")
done
HARNESS="$d/code/packages/c/platform-harness"
ISO="$d/code/packages/c/iso-harness"
OSP="$d/code/packages/c/os-platform"
if [ ! -f "$HARNESS/lib/platform-lib.sh" ]; then
    echo "platform-harness not found (searched upward from $SELF)" >&2
    exit 1
fi
PLATFORM_INCLUDE="$SELF/include $OSP/include $ISO/include"
export PLATFORM_INCLUDE
PLATFORM_DEFINES="_DEFAULT_SOURCE _DARWIN_C_SOURCE"
export PLATFORM_DEFINES
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    PLATFORM_REQUIRE="gcc clang"; export PLATFORM_REQUIRE
fi
. "$HARNESS/lib/platform-lib.sh"
echo "reactor ($(platform_os)): C compilers: $(platform_compilers c)"
rc=0
PLATFORM_LIBS=""; export PLATFORM_LIBS
platform_build_and_run c reactor-tests tests/reactor_test.c src/reactor_posix.c || rc=1
exit "$rc"
