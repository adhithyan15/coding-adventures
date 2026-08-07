#!/bin/sh
# run.sh — build & run the event-loop test on Unix. event-loop is OS-agnostic;
# it composes os-platform's thread backend (osp_mutex, guarding the stop flag) and
# clock backend (osp_sleep_ns, the idle nap). The test also spawns a worker thread
# (osp_thread) to stop the loop from outside. Links the OS thread library (-pthread).
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
# _POSIX_C_SOURCE exposes pthreads (thread backend) AND clock_gettime/nanosleep
# (clock backend) on both glibc and Darwin — the same define both backends use.
PLATFORM_DEFINES="_POSIX_C_SOURCE=200809L"
export PLATFORM_DEFINES
echo "event-loop ($(platform_os)): C compilers: $(platform_compilers c)"
rc=0
PLATFORM_LIBS="-pthread"; export PLATFORM_LIBS
platform_build_and_run c event-loop-tests \
    tests/event_loop_test.c src/event_loop.c \
    "$OSP/src/thread_posix.c" "$OSP/src/clock_posix.c" || rc=1
exit "$rc"
