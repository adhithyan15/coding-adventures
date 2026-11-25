#!/bin/sh
# Keep strict compiler tests separate from install-consumer packaging checks.
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"
d="$SELF"
while [ "$d" != / ] && [ ! -d "$d/code/packages/c/iso-harness" ]; do d=$(dirname "$d"); done
HARNESS="$d/code/packages/c/iso-harness"
if [ ! -f "$HARNESS/lib/iso-lib.sh" ]; then echo 'iso-harness not found' >&2; exit 1; fi
ISO_INCLUDE="include ../rank-statistics/include ../bounded-json/include $HARNESS/include"
export ISO_INCLUDE
if [ "${CI:-}" = true ] && [ "$(uname)" = Linux ]; then ISO_REQUIRE='gcc clang'; export ISO_REQUIRE; fi
. "$HARNESS/lib/iso-lib.sh"
iso_build_and_run cpp microbenchmark-tests tests/microbenchmark_test.cpp src/microbenchmark.cpp src/array_workload.cpp
iso_build_and_run cpp array-microbenchmark-smoke src/array_driver.cpp src/microbenchmark.cpp src/array_workload.cpp

tmp=$(mktemp -d "${TMPDIR:-/tmp}/ca-microbenchmark-cpp.XXXXXX")
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
prefix="$tmp/install"
cmake -S "$d/code/packages/cpp/bounded-json" -B "$tmp/bounded-json" -DCMAKE_INSTALL_PREFIX="$prefix" -DCMAKE_BUILD_TYPE=Release
cmake --build "$tmp/bounded-json" --config Release
cmake --install "$tmp/bounded-json" --config Release
cmake -S "$SELF/../rank-statistics" -B "$tmp/rank-statistics" -DCMAKE_INSTALL_PREFIX="$prefix" -DCMAKE_BUILD_TYPE=Release
cmake --build "$tmp/rank-statistics" --config Release
cmake --install "$tmp/rank-statistics" --config Release
cmake -S "$SELF" -B "$tmp/microbenchmark" -DCMAKE_INSTALL_PREFIX="$prefix" -DCMAKE_PREFIX_PATH="$prefix" -DCMAKE_BUILD_TYPE=Release
cmake --build "$tmp/microbenchmark" --config Release
ctest --test-dir "$tmp/microbenchmark" -C Release --output-on-failure
cmake --install "$tmp/microbenchmark" --config Release
cmake -S "$SELF/tests/consumer" -B "$tmp/consumer" -DCMAKE_PREFIX_PATH="$prefix" -DCMAKE_BUILD_TYPE=Release
cmake --build "$tmp/consumer" --config Release
"$tmp/consumer/ca_microbenchmark_consumer"
