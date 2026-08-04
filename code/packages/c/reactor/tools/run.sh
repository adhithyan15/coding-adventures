#!/bin/sh
# run.sh — build & run the reactor tests on Unix via platform-harness. The two
# Unix platforms use different scalable readiness mechanisms, so the source file
# and feature-test macros are chosen by OS: macOS → reactor_mac.c (kqueue),
# Linux → reactor_linux.c (epoll). Both plus socketpair live in libc; no extra
# library. The Windows half (run.ps1, BUILD_windows) links ws2_32 for WSAPoll.
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
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    PLATFORM_REQUIRE="gcc clang"; export PLATFORM_REQUIRE
fi
. "$HARNESS/lib/platform-lib.sh"

# Pick the backend + feature macros for this OS. macOS exposes kqueue and
# socketpair under _DARWIN_C_SOURCE; Linux needs _GNU_SOURCE for epoll_create1
# (and socketpair).
if [ "$(platform_os)" = "mac" ]; then
    SRC="src/reactor_mac.c"
    PLATFORM_DEFINES="_DARWIN_C_SOURCE"
else
    SRC="src/reactor_linux.c"
    PLATFORM_DEFINES="_GNU_SOURCE"
fi
export PLATFORM_DEFINES

echo "reactor ($(platform_os)): C compilers: $(platform_compilers c)"
rc=0
PLATFORM_LIBS=""; export PLATFORM_LIBS
platform_build_and_run c reactor-tests tests/reactor_test.c "$SRC" || rc=1
exit "$rc"
