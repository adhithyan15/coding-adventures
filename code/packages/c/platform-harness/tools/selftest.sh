#!/bin/sh
# selftest.sh — verify the platform-harness builds and runs OS-dependent C/C++.
#
# Invoked from BUILD as a single line (`sh tools/selftest.sh`) because the
# build-tool feeds each BUILD line to its own `sh -c`, so real multi-line logic
# lives here.
#
# It proves, on every compiler present, that the harness can compile POSIX
# thread code under -Wall -Wextra -Werror (WITHOUT -pedantic-errors) and link the
# OS-provided pthread library (via PLATFORM_LIBS), then run the result. This is
# the platform-harness analogue of iso-harness's self-test — except the whole
# point here is that OS-dependent code is allowed.
#
# Output goes to _build/ (NOT build/ — collides with the BUILD script on
# case-insensitive filesystems, see lessons.md).
set -e

SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"

# Reuse iso-harness's iso_test.h (sibling package), and link the OS thread lib.
PLATFORM_INCLUDE="$SELF_DIR/../iso-harness/include"
export PLATFORM_INCLUDE
PLATFORM_LIBS="-pthread"
export PLATFORM_LIBS

. lib/platform-lib.sh

echo "platform-harness self-test ($(platform_os))"
echo "  C   compilers: $(platform_compilers c)"
echo "  C++ compilers: $(platform_compilers cpp)"

rc=0

echo "== POSIX threads: C =="
platform_build_and_run c posix-selftest selftest/posix_selftest.c || rc=1

echo "== POSIX threads: C++ (std::thread) =="
platform_build_and_run cpp posix-selftest selftest/posix_selftest.cpp || rc=1

echo ""
if [ "$rc" -eq 0 ]; then
    echo "platform-harness self-test: PASS"
else
    echo "platform-harness self-test: FAIL" >&2
fi
exit "$rc"
