#!/bin/sh
# run.sh — build & run the http-server test on Unix. http-server is OS-agnostic;
# compiled with tcp-runtime, net's POSIX backend, the reactor backend for this OS
# (kqueue/epoll), and http-core (pure ISO C).
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
TCPRT="$d/code/packages/c/tcp-runtime"
HTTP="$d/code/packages/c/http-core"
if [ ! -f "$HARNESS/lib/platform-lib.sh" ]; then
    echo "platform-harness not found (searched upward from $SELF)" >&2
    exit 1
fi
PLATFORM_INCLUDE="$SELF/include $TCPRT/include $NET/include $REACTOR/include $HTTP/include $OSP/include $ISO/include"
export PLATFORM_INCLUDE
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    PLATFORM_REQUIRE="gcc clang"; export PLATFORM_REQUIRE
fi
. "$HARNESS/lib/platform-lib.sh"
if [ "$(platform_os)" = "mac" ]; then
    REACTOR_SRC="$REACTOR/src/reactor_mac.c"
    PLATFORM_DEFINES="_DARWIN_C_SOURCE"
else
    REACTOR_SRC="$REACTOR/src/reactor_linux.c"
    PLATFORM_DEFINES="_GNU_SOURCE"
fi
export PLATFORM_DEFINES
echo "http-server ($(platform_os)): C compilers: $(platform_compilers c)"
rc=0
# tcp-runtime's mailbox uses os-platform's thread mutex, so this consumer links
# the thread backend and the OS thread library too (-pthread).
PLATFORM_LIBS="-pthread"; export PLATFORM_LIBS
platform_build_and_run c http-server-tests \
    tests/http_server_test.c src/http_server.c \
    "$TCPRT/src/tcp_runtime.c" "$NET/src/net_posix.c" "$REACTOR_SRC" \
    "$HTTP/src/http_core.c" "$OSP/src/thread_posix.c" || rc=1
exit "$rc"
