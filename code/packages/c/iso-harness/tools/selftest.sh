#!/bin/sh
# selftest.sh — verify the iso-harness actually enforces pure ISO C/C++.
#
# Invoked from BUILD as a single line (`sh tools/selftest.sh`) because the
# build-tool feeds each BUILD line to its own `sh -c`, so real multi-line logic
# lives here in a sibling script.
#
# It proves two things, on every compiler present on this machine:
#   1. POSITIVE — the conforming C and C++ fixtures compile *and run* cleanly
#      under the strict flags.
#   2. NEGATIVE — the non-conforming fixtures (which use a GNU statement
#      expression) are REJECTED. This is the harness testing itself: if a
#      compiler accepted the extension, the strict flags would not be enforcing
#      ISO conformance and the self-test fails.
#
# Output goes to _build/ (NOT build/ — case-insensitive filesystems collide with
# the BUILD script, see lessons.md).
set -e

SELF_DIR=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF_DIR"

# Make iso_test.h reachable from the fixtures and load the harness library.
ISO_INCLUDE="include"
export ISO_INCLUDE
. lib/iso-lib.sh

echo "iso-harness self-test"
echo "  C   compilers: $(iso_compilers c)"
echo "  C++ compilers: $(iso_compilers cpp)"

rc=0

# ── 1. Positive: conforming fixtures must build and run everywhere ───────────
echo "== positive: conforming C =="
iso_build_and_run c conforming selftest/conforming.c || rc=1

echo "== positive: conforming C++ =="
iso_build_and_run cpp conforming selftest/conforming.cpp || rc=1

# ── 2. Negative: non-conforming fixtures must be rejected everywhere ─────────
echo "== negative: non-conforming C must be rejected =="
iso_expect_compile_fail c selftest/nonconforming.c || rc=1

echo "== negative: non-conforming C++ must be rejected =="
iso_expect_compile_fail cpp selftest/nonconforming.cpp || rc=1

echo ""
if [ "$rc" -eq 0 ]; then
    echo "iso-harness self-test: PASS"
else
    echo "iso-harness self-test: FAIL" >&2
fi
exit "$rc"
