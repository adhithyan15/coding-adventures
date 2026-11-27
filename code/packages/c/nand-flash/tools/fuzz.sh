#!/bin/sh
# Optional Clang/libFuzzer campaigns; deliberately separate from BUILD fronts.
set -eu

SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"
CLANG=${CLANG:-clang}
LLVM_PROFDATA=${LLVM_PROFDATA:-llvm-profdata}
LLVM_COV=${LLVM_COV:-llvm-cov}
FUZZ_WORK_PARENT=${FUZZ_WORK_PARENT:-${TMPDIR:-/tmp}}
mkdir -p "$FUZZ_WORK_PARENT"
FUZZ_WORK_DIR=$(mktemp -d "$FUZZ_WORK_PARENT/db-e12-c-nand.XXXXXX")
mkdir -p "$FUZZ_WORK_DIR"/fixture/corpus "$FUZZ_WORK_DIR"/fixture/artifacts \
    "$FUZZ_WORK_DIR"/fixture/profiles "$FUZZ_WORK_DIR"/history/corpus \
    "$FUZZ_WORK_DIR"/history/artifacts "$FUZZ_WORK_DIR"/history/profiles

COMMON_FLAGS="-std=c17 -O1 -g -Wall -Wextra -Werror -pedantic -fno-omit-frame-pointer -fsanitize=fuzzer,address,undefined -fprofile-instr-generate -fcoverage-mapping"
SOURCES="tests/nand_flash_fuzz.c tests/nand_flash_fuzz_support.c src/nand_flash.c ../bounded-json/src/bounded_json.c"

# The tracked corpus is copied into a fresh writable directory. libFuzzer must
# never receive a tracked seed directory as its first corpus argument.
cp -R tests/fuzz-corpus/fixture-vectors/. "$FUZZ_WORK_DIR/fixture/corpus/"
cp ../../../specs/fixtures/database-media-v1/nand-flash.json \
    "$FUZZ_WORK_DIR/fixture/corpus/nand-flash.json"
cp -R tests/fuzz-corpus/command-histories/. "$FUZZ_WORK_DIR/history/corpus/"

"$CLANG" $COMMON_FLAGS -DNAND_FUZZ_MODE=1 \
    -I include -I ../bounded-json/include \
    $SOURCES -o "$FUZZ_WORK_DIR/nand_fixture_fuzz"
"$CLANG" $COMMON_FLAGS -DNAND_FUZZ_MODE=2 \
    -I include -I ../bounded-json/include \
    $SOURCES -o "$FUZZ_WORK_DIR/nand_history_fuzz"

{
    echo "clang: $("$CLANG" --version | sed -n '1p')"
    echo "llvm-profdata: $("$LLVM_PROFDATA" --version | sed -n '1p')"
    echo "llvm-cov: $("$LLVM_COV" --version | sed -n '1p')"
    echo "source revision: $(git -C "$SELF" rev-parse HEAD)"
    echo "runtime, adapter, driver, parser, and fixture sha256:"
    shasum -a 256 src/nand_flash.c tests/nand_flash_fuzz_support.c \
        tests/nand_flash_fuzz_support.h tests/nand_flash_fuzz.c \
        tests/nand_flash_test.c ../bounded-json/include/bounded_json.h \
        ../bounded-json/src/bounded_json.c \
        ../../../specs/fixtures/database-media-v1/nand-flash.json
    echo "fixture corpus sha256:"
    find "$FUZZ_WORK_DIR/fixture/corpus" -type f | LC_ALL=C sort | while IFS= read -r seed; do shasum -a 256 "$seed"; done
    echo "history corpus sha256:"
    find "$FUZZ_WORK_DIR/history/corpus" -type f | LC_ALL=C sort | while IFS= read -r seed; do shasum -a 256 "$seed"; done
} > "$FUZZ_WORK_DIR/toolchain-and-seeds.txt"

LLVM_PROFILE_FILE="$FUZZ_WORK_DIR/fixture/profiles/profile-%p.profraw" \
    "$FUZZ_WORK_DIR/nand_fixture_fuzz" -seed=12001 -max_len=65536 \
    -timeout=10 -runs=100000 \
    -artifact_prefix="$FUZZ_WORK_DIR/fixture/artifacts/" \
    "$FUZZ_WORK_DIR/fixture/corpus" \
    > "$FUZZ_WORK_DIR/fixture/fuzzer.log" 2>&1
tail -n 20 "$FUZZ_WORK_DIR/fixture/fuzzer.log"
LLVM_PROFILE_FILE="$FUZZ_WORK_DIR/history/profiles/profile-%p.profraw" \
    "$FUZZ_WORK_DIR/nand_history_fuzz" -seed=12002 -max_len=514 \
    -timeout=10 -runs=100000 \
    -artifact_prefix="$FUZZ_WORK_DIR/history/artifacts/" \
    "$FUZZ_WORK_DIR/history/corpus" \
    > "$FUZZ_WORK_DIR/history/fuzzer.log" 2>&1
tail -n 20 "$FUZZ_WORK_DIR/history/fuzzer.log"

"$LLVM_PROFDATA" merge -sparse \
    "$FUZZ_WORK_DIR"/fixture/profiles/*.profraw \
    -o "$FUZZ_WORK_DIR/fixture/coverage.profdata"
"$LLVM_PROFDATA" merge -sparse \
    "$FUZZ_WORK_DIR"/history/profiles/*.profraw \
    -o "$FUZZ_WORK_DIR/history/coverage.profdata"
"$LLVM_COV" report -show-branch-summary \
    -instr-profile="$FUZZ_WORK_DIR/fixture/coverage.profdata" \
    "$FUZZ_WORK_DIR/nand_fixture_fuzz" \
    --sources=src/nand_flash.c,tests/nand_flash_fuzz_support.c,tests/nand_flash_fuzz.c,../bounded-json/src/bounded_json.c \
    > "$FUZZ_WORK_DIR/fixture/coverage.txt"
"$LLVM_COV" report -show-branch-summary \
    -instr-profile="$FUZZ_WORK_DIR/history/coverage.profdata" \
    "$FUZZ_WORK_DIR/nand_history_fuzz" \
    --sources=src/nand_flash.c,tests/nand_flash_fuzz_support.c,tests/nand_flash_fuzz.c,../bounded-json/src/bounded_json.c \
    > "$FUZZ_WORK_DIR/history/coverage.txt"

echo "DB-E12 NAND scratch evidence: $FUZZ_WORK_DIR"
cat "$FUZZ_WORK_DIR/toolchain-and-seeds.txt"
echo "fixture coverage:"
cat "$FUZZ_WORK_DIR/fixture/coverage.txt"
echo "history coverage:"
cat "$FUZZ_WORK_DIR/history/coverage.txt"
