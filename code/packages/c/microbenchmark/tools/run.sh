#!/bin/sh
set -e
cd "$(dirname "$0")/.."
root=$(git rev-parse --show-toplevel)
cmp testdata/microbenchmark-pairs-v1.json "$root/code/specs/fixtures/microbenchmark-pairs-v1.json"
export ISO_INCLUDE="include $root/code/packages/c/rank-statistics/include $root/code/packages/c/bounded-json/include $root/code/packages/c/test-bootstrap/include $root/code/packages/c/iso-harness/include"
. "$root/code/packages/c/iso-harness/lib/iso-lib.sh"
pbt="$root/code/packages/c/test-bootstrap/src"
iso_build_and_run c microbenchmark-tests tests/microbenchmark_test.c src/microbenchmark.c src/native_clock.c src/report_json.c "$root/code/packages/c/rank-statistics/src/rank_statistics.c" "$root/code/packages/c/bounded-json/src/bounded_json.c" \
  "$pbt/pbt_big.c" "$pbt/pbt_prng.c" "$pbt/pbt_val.c" "$pbt/pbt_budget.c" "$pbt/pbt_shrink.c" "$pbt/pbt_gen.c" "$pbt/pbt_runner.c" "$pbt/pbt_json.c" "$pbt/pbt_tfv.c" "$pbt/pbt_corpus.c" "$pbt/pbt_hooks.c"
sh tools/install-test.sh
