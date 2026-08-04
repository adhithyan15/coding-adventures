#!/bin/sh
# run.sh — build & run the tcp-runtime test on Unix. tcp-runtime is OS-agnostic;
# it is compiled with net's POSIX backend and the reactor backend for this OS
# (kqueue on macOS, epoll on Linux), which is where the per-OS code lives.
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
NET="$d/code/packages/c/net"
REACTOR="$d/code/packages/c/reactor"
if [ ! -f "$HARNESS/lib/platform-lib.sh" ]; then
    echo "platform-harness not found (searched upward from $SELF)" >&2
    exit 1
fi
PLATFORM_INCLUDE="$SELF/include $NET/include $REACTOR/include $OSP/include $ISO/include"
export PLATFORM_INCLUDE
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    PLATFORM_REQUIRE="gcc clang"; export PLATFORM_REQUIRE
fi
. "$HARNESS/lib/platform-lib.sh"
# net's POSIX backend serves both Unix OSes; the reactor backend + its feature
# macros differ (kqueue/_DARWIN_C_SOURCE vs epoll/_GNU_SOURCE).
if [ "$(platform_os)" = "mac" ]; then
    REACTOR_SRC="$REACTOR/src/reactor_mac.c"
    PLATFORM_DEFINES="_DARWIN_C_SOURCE"
else
    REACTOR_SRC="$REACTOR/src/reactor_linux.c"
    PLATFORM_DEFINES="_GNU_SOURCE"
fi
export PLATFORM_DEFINES
echo "tcp-runtime ($(platform_os)): C compilers: $(platform_compilers c)"
rc=0
# The mailbox mutex comes from os-platform's thread backend (pthreads on POSIX),
# so link its source and the OS thread library. _GNU_SOURCE / _DARWIN_C_SOURCE
# (set above for the reactor) already expose the pthreads declarations.
PLATFORM_LIBS="-pthread"; export PLATFORM_LIBS
platform_build_and_run c tcp-runtime-tests \
    tests/tcp_runtime_test.c src/tcp_runtime.c \
    "$NET/src/net_posix.c" "$REACTOR_SRC" "$OSP/src/thread_posix.c" || rc=1
exit "$rc"
