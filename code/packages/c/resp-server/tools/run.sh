#!/bin/sh
# run.sh — build & run the resp-server test on Unix. resp-server is OS-agnostic;
# it is compiled with tcp-runtime, net's POSIX backend, the reactor backend for
# this OS (kqueue/epoll), and resp-protocol (pure ISO C).
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
RESP="$d/code/packages/c/resp-protocol"
if [ ! -f "$HARNESS/lib/platform-lib.sh" ]; then
    echo "platform-harness not found (searched upward from $SELF)" >&2
    exit 1
fi
PLATFORM_INCLUDE="$SELF/include $TCPRT/include $NET/include $REACTOR/include $RESP/include $OSP/include $ISO/include"
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
echo "resp-server ($(platform_os)): C compilers: $(platform_compilers c)"
rc=0
PLATFORM_LIBS=""; export PLATFORM_LIBS
platform_build_and_run c resp-server-tests \
    tests/resp_server_test.c src/resp_server.c \
    "$TCPRT/src/tcp_runtime.c" "$NET/src/net_posix.c" "$REACTOR_SRC" \
    "$RESP/src/resp_protocol.c" || rc=1
exit "$rc"
