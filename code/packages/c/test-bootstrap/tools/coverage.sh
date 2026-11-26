#!/bin/sh
# On-demand line/region coverage report for the test-bootstrap engine (NOT part of BUILD:
# no package in the C lane gates coverage today, and CI has no C coverage step).
#
# Builds every test with clang's source-based coverage, runs them all, merges the
# profiles and prints llvm-cov's per-file report for src/ only.
#
#     sh tools/coverage.sh            # report (clang + llvm-cov must be installed)
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"
d="$SELF"
while [ "$d" != "/" ] && [ ! -d "$d/code/packages/c/iso-harness" ]; do d=$(dirname "$d"); done
HARNESS="$d/code/packages/c/iso-harness"

CLANG=${CLANG:-clang}
if [ -x /usr/bin/xcrun ] && xcrun --find llvm-cov >/dev/null 2>&1; then
    LLVM_COV="xcrun llvm-cov"
    LLVM_PROFDATA="xcrun llvm-profdata"
else
    LLVM_COV=${LLVM_COV:-llvm-cov}
    LLVM_PROFDATA=${LLVM_PROFDATA:-llvm-profdata}
fi
OUT=${COVERAGE_DIR:-_coverage}
rm -rf "$OUT"
mkdir -p "$OUT"
LIB_SRCS="src/pbt_big.c src/pbt_prng.c src/pbt_val.c src/pbt_budget.c src/pbt_shrink.c src/pbt_gen.c src/pbt_runner.c src/pbt_json.c src/pbt_tfv.c src/pbt_corpus.c src/pbt_hooks.c src/fuzz_hooks.c src/fuzz_mutate.c src/fuzz_bitmap.c src/fuzz_sig.c src/fuzz_engine.c src/fuzz_corpus.c"
TESTS="test_big test_prng test_json_tfv test_val test_budget test_engine test_oom test_corpus test_fuzz_corpus test_fuzz_vectors test_fuzz_pinned test_fuzz_engine test_fuzz_oom"
OBJS=""
for t in $TESTS; do
    "$CLANG" -std=c17 -O0 -g -fprofile-instr-generate -fcoverage-mapping -Iinclude -Isrc -Itests -I"$HARNESS/include" \
        "tests/$t.c" tests/test_util.c $LIB_SRCS -lm -o "$OUT/$t"
    LLVM_PROFILE_FILE="$OUT/$t.profraw" "$OUT/$t" >/dev/null
    OBJS="$OBJS -object $OUT/$t"
done
$LLVM_PROFDATA merge -sparse "$OUT"/*.profraw -o "$OUT/all.profdata"
# shellcheck disable=SC2086
set -- $OBJS
first=$2
shift 2
# shellcheck disable=SC2086
$LLVM_COV report "$first" "$@" -instr-profile="$OUT/all.profdata" $(for f in $LIB_SRCS; do printf '%s ' "$f"; done)
