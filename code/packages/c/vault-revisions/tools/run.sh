#!/bin/sh
# run.sh — build & run the vault-revisions test on Unix. vault-revisions is
# OS-agnostic; its only OS dependency is os-platform's thread backend (osp_mutex,
# guarding the store). The test spawns worker threads (osp_thread) for the
# concurrent-archive check. Links the OS thread library (-pthread).
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
# _POSIX_C_SOURCE exposes pthreads (the thread backend) on glibc and Darwin.
PLATFORM_DEFINES="_POSIX_C_SOURCE=200809L"
export PLATFORM_DEFINES
echo "vault-revisions ($(platform_os)): C compilers: $(platform_compilers c)"
rc=0
PLATFORM_LIBS="-pthread"; export PLATFORM_LIBS
platform_build_and_run c vault-revisions-tests \
    tests/vault_revisions_test.c src/vault_revisions.c \
    "$OSP/src/thread_posix.c" || rc=1
exit "$rc"
