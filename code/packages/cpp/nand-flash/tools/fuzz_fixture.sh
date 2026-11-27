#!/bin/sh
set -eu
NAND_PACKAGE=$(cd "$(dirname "$0")/.." && pwd)
cd "$NAND_PACKAGE"
NAND_FUZZ_COMPILER=${FUZZ_CXX:-clang++}
NAND_FUZZ_RUNS=${FUZZ_RUNS:-100000}
case "$NAND_FUZZ_RUNS" in
    ''|*[!0-9]*) echo 'FUZZ_RUNS must be a positive integer' >&2; exit 2 ;;
esac
while [ "${NAND_FUZZ_RUNS#0}" != "$NAND_FUZZ_RUNS" ]; do
    NAND_FUZZ_RUNS=${NAND_FUZZ_RUNS#0}
done
if [ -z "$NAND_FUZZ_RUNS" ] || [ "${#NAND_FUZZ_RUNS}" -gt 10 ] ||
   [ "$NAND_FUZZ_RUNS" -gt 2147483647 ]; then
    echo 'FUZZ_RUNS must be a positive integer no greater than 2147483647' >&2
    exit 2
fi
NAND_FUZZ_ROOT=${FUZZ_OUTPUT_DIR:-_build/nand-fixture-fuzz}
case "$NAND_FUZZ_ROOT" in
    /*) ;;
    *) NAND_FUZZ_ROOT="$NAND_PACKAGE/$NAND_FUZZ_ROOT" ;;
esac
NAND_FUZZ_CORPUS="$NAND_FUZZ_ROOT/corpus"
NAND_FUZZ_ARTIFACTS="$NAND_FUZZ_ROOT/artifacts"
NAND_FUZZ_BINARY="$NAND_FUZZ_ROOT/nand-fixture-fuzzer"
mkdir -p "$NAND_FUZZ_CORPUS" "$NAND_FUZZ_ARTIFACTS"
python3 tools/generate_fixture_fuzz_seeds.py \
    ../../../specs/fixtures/database-media-v1/nand-flash.json "$NAND_FUZZ_CORPUS"
"$NAND_FUZZ_COMPILER" -std=c++17 -O1 -g -fsanitize=fuzzer,address,undefined \
    -fno-omit-frame-pointer -Iinclude -Isrc -Itests \
    -I../bounded-json/include -I../../c/iso-harness/include \
    tests/fuzz_nand_fixture.cpp src/nand_flash.cpp -o "$NAND_FUZZ_BINARY"
"$NAND_FUZZ_BINARY" -runs="$NAND_FUZZ_RUNS" -max_len=65537 \
    -seed=20261127 -print_final_stats=1 \
    -artifact_prefix="$NAND_FUZZ_ARTIFACTS/" "$NAND_FUZZ_CORPUS"
