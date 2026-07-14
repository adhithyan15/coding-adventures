#!/bin/sh
# run.sh — build & run the os-platform tests on Unix (macOS + Linux).
#
# This is the POSIX half of the package. It compiles each primitive's test
# together with that primitive's POSIX backend, under EVERY available C compiler,
# via the sibling platform-harness (which keeps -Wall -Wextra -Werror but drops
# -pedantic-errors, because this code deliberately talks to the OS). The Windows
# half lives in run.ps1 and is driven by BUILD_windows.
#
# Adding a primitive later = add one `platform_build_and_run …` line below.
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"

# Walk up to the repo directory that holds code/packages/c, so the harness and
# iso_test.h are found whether run from a worktree, CI checkout, or install.
d="$SELF"
while [ "$d" != "/" ] && [ ! -d "$d/code/packages/c/platform-harness" ]; do
    d=$(dirname "$d")
done
HARNESS="$d/code/packages/c/platform-harness"
ISO="$d/code/packages/c/iso-harness"
if [ ! -f "$HARNESS/lib/platform-lib.sh" ]; then
    echo "platform-harness not found (searched upward from $SELF)" >&2
    exit 1
fi

# Our own headers plus iso-harness's (for iso_test.h).
PLATFORM_INCLUDE="$SELF/include $ISO/include"
export PLATFORM_INCLUDE

# clock_gettime / nanosleep are gated behind this feature-test macro.
PLATFORM_DEFINES="_POSIX_C_SOURCE=200809L"
export PLATFORM_DEFINES

# On Linux CI both gcc and clang are installed — require both so portability is
# firm rather than best-effort. Locally, use whatever compilers are present.
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    PLATFORM_REQUIRE="gcc clang"
    export PLATFORM_REQUIRE
fi

. "$HARNESS/lib/platform-lib.sh"

echo "os-platform ($(platform_os)): C compilers: $(platform_compilers c)"

rc=0

# clock — POSIX backend (clock_gettime + nanosleep). No extra OS library.
PLATFORM_LIBS=""
export PLATFORM_LIBS
platform_build_and_run c clock-tests tests/clock_test.c src/clock_posix.c || rc=1

# thread — POSIX backend (pthreads). Links the OS thread library.
PLATFORM_LIBS="-pthread"
export PLATFORM_LIBS
platform_build_and_run c thread-tests tests/thread_test.c src/thread_posix.c || rc=1

exit "$rc"
