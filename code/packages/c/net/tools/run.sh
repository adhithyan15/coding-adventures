#!/bin/sh
# run.sh — build & run the net tests on Unix (macOS + Linux) via platform-harness.
#
# BSD sockets live in libc on modern macOS/glibc, so no extra library is linked;
# the socket declarations need the extension feature macros (_DEFAULT_SOURCE on
# glibc, _DARWIN_C_SOURCE on Darwin — each harmless on the other). The Windows
# half (run.ps1, driven by BUILD_windows) links ws2_32 instead.
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"

# Walk up to the repo dir that holds code/packages/c (harness + os-platform +
# iso_test.h all live there).
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

# Our headers, os-platform's (for os_platform/status.h), and iso_test.h.
PLATFORM_INCLUDE="$SELF/include $OSP/include $ISO/include"
export PLATFORM_INCLUDE

PLATFORM_DEFINES="_DEFAULT_SOURCE _DARWIN_C_SOURCE"
export PLATFORM_DEFINES

if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    PLATFORM_REQUIRE="gcc clang"
    export PLATFORM_REQUIRE
fi

. "$HARNESS/lib/platform-lib.sh"

echo "net ($(platform_os)): C compilers: $(platform_compilers c)"

rc=0

# tcp — POSIX backend (BSD sockets). No extra OS library.
PLATFORM_LIBS=""
export PLATFORM_LIBS
platform_build_and_run c net-tests tests/net_test.c src/net_posix.c || rc=1

exit "$rc"
