#!/bin/sh
set -e
cd "$(dirname "$0")/.."
root=$(git rev-parse --show-toplevel)
export ISO_INCLUDE="include $root/code/packages/c/iso-harness/include"
# nextafter is provided by libm on Linux. Keep it after the test/library
# sources so traditional linkers can resolve the test's symbol references.
export ISO_LINK_ARGS="-lm"
. "$root/code/packages/c/iso-harness/lib/iso-lib.sh"
iso_build_and_run c rank-statistics tests/rank_test.c src/rank_statistics.c
