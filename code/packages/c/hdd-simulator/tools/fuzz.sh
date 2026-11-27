#!/bin/sh
# Optional two-lane Clang/libFuzzer campaigns; BUILD does not depend on LLVM.
set -eu

SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"
CLANG=${CLANG:-clang}
LLVM_PROFDATA=${LLVM_PROFDATA:-llvm-profdata}
LLVM_COV=${LLVM_COV:-llvm-cov}
FUZZ_WORK_PARENT=${FUZZ_WORK_PARENT:-${TMPDIR:-/tmp}}
mkdir -p "$FUZZ_WORK_PARENT"
WORK=$(mktemp -d "$FUZZ_WORK_PARENT/db-e12-c-hdd.XXXXXX")
mkdir -p "$WORK/fixture/corpus" "$WORK/fixture/artifacts" "$WORK/fixture/profiles" \
    "$WORK/history/corpus" "$WORK/history/artifacts" "$WORK/history/profiles"

COMMON="-std=c17 -O1 -g -Wall -Wextra -Werror -pedantic -fno-omit-frame-pointer -fsanitize=fuzzer,address,undefined -fprofile-instr-generate -fcoverage-mapping"
SOURCES="tests/hdd_simulator_fuzz.c tests/hdd_simulator_test.c ../bounded-json/src/bounded_json.c"

cp -R tests/fuzz-corpus/fixture-vectors/. "$WORK/fixture/corpus/"
cp ../../../specs/fixtures/database-media-v1/hdd-simulator.json "$WORK/fixture/corpus/hdd-simulator.json"
cp -R tests/fuzz-corpus/command-histories/. "$WORK/history/corpus/"

"$CLANG" $COMMON -DHDD_FUZZ_BUILD -DHDD_FUZZ_MODE=1 -I include -I ../bounded-json/include -I ../iso-harness/include \
    $SOURCES -o "$WORK/hdd_fixture_fuzz"
"$CLANG" $COMMON -DHDD_FUZZ_BUILD -DHDD_FUZZ_MODE=2 -I include -I ../bounded-json/include -I ../iso-harness/include \
    $SOURCES -o "$WORK/hdd_history_fuzz"

{
    echo "clang: $("$CLANG" --version | sed -n '1p')"
    echo "llvm-profdata: $("$LLVM_PROFDATA" --version | sed -n '1p')"
    echo "llvm-cov: $("$LLVM_COV" --version | sed -n '1p')"
    echo "source revision: $(git -C "$SELF" rev-parse HEAD)"
    echo "runtime, adapter, driver, parser, and fixture sha256:"
    shasum -a 256 src/hdd_simulator.c tests/hdd_simulator_test.c tests/hdd_simulator_fuzz.c \
        ../bounded-json/include/bounded_json.h ../bounded-json/src/bounded_json.c \
        ../../../specs/fixtures/database-media-v1/hdd-simulator.json
    echo "fixture corpus sha256:"
    find "$WORK/fixture/corpus" -type f | LC_ALL=C sort | while IFS= read -r seed; do shasum -a 256 "$seed"; done
    echo "history corpus sha256:"
    find "$WORK/history/corpus" -type f | LC_ALL=C sort | while IFS= read -r seed; do shasum -a 256 "$seed"; done
} > "$WORK/toolchain-and-seeds.txt"

LLVM_PROFILE_FILE="$WORK/fixture/profiles/profile-%p.profraw" \
    "$WORK/hdd_fixture_fuzz" -seed=12021 -max_len=65536 -timeout=10 -runs=100000 \
    -artifact_prefix="$WORK/fixture/artifacts/" "$WORK/fixture/corpus" > "$WORK/fixture/fuzzer.log" 2>&1
tail -n 20 "$WORK/fixture/fuzzer.log"
LLVM_PROFILE_FILE="$WORK/history/profiles/profile-%p.profraw" \
    "$WORK/hdd_history_fuzz" -seed=12022 -max_len=523 -timeout=10 -runs=100000 \
    -artifact_prefix="$WORK/history/artifacts/" "$WORK/history/corpus" > "$WORK/history/fuzzer.log" 2>&1
tail -n 20 "$WORK/history/fuzzer.log"

"$LLVM_PROFDATA" merge -sparse "$WORK"/fixture/profiles/*.profraw -o "$WORK/fixture/coverage.profdata"
"$LLVM_PROFDATA" merge -sparse "$WORK"/history/profiles/*.profraw -o "$WORK/history/coverage.profdata"
"$LLVM_COV" report -show-branch-summary -instr-profile="$WORK/fixture/coverage.profdata" \
    "$WORK/hdd_fixture_fuzz" --sources=src/hdd_simulator.c,tests/hdd_simulator_test.c,tests/hdd_simulator_fuzz.c,../bounded-json/src/bounded_json.c > "$WORK/fixture/coverage.txt"
"$LLVM_COV" report -show-branch-summary -instr-profile="$WORK/history/coverage.profdata" \
    "$WORK/hdd_history_fuzz" --sources=src/hdd_simulator.c,tests/hdd_simulator_test.c,tests/hdd_simulator_fuzz.c,../bounded-json/src/bounded_json.c > "$WORK/history/coverage.txt"

echo "DB-E12 C HDD scratch evidence: $WORK"
cat "$WORK/toolchain-and-seeds.txt"
echo "fixture coverage:"
cat "$WORK/fixture/coverage.txt"
echo "history coverage:"
cat "$WORK/history/coverage.txt"
