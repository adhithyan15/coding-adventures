#!/bin/sh
set -eu
NAND_FUZZ_PACKAGE=$(cd "$(dirname "$0")/.." && pwd)
cd "$NAND_FUZZ_PACKAGE"
NAND_FUZZ_COMPILER=${FUZZ_CXX:-clang++}
NAND_FUZZ_DURATION=${FUZZ_SECONDS:-30}
case "$NAND_FUZZ_DURATION" in
  ''|*[!0-9]*) echo 'FUZZ_SECONDS must be a positive integer' >&2; exit 2 ;;
esac
while [ "${NAND_FUZZ_DURATION#0}" != "$NAND_FUZZ_DURATION" ]; do
  NAND_FUZZ_DURATION=${NAND_FUZZ_DURATION#0}
done
if [ -z "$NAND_FUZZ_DURATION" ] || [ "${#NAND_FUZZ_DURATION}" -gt 10 ] ||
   [ "$NAND_FUZZ_DURATION" -gt 2147483647 ]; then
  echo 'FUZZ_SECONDS must be a positive integer no greater than 2147483647' >&2
  exit 2
fi
NAND_FUZZ_OUTPUT=${FUZZ_BINARY:-_build/fuzz-nand-flash}
case "$NAND_FUZZ_OUTPUT" in
  /*) ;;
  *) NAND_FUZZ_OUTPUT="$NAND_FUZZ_PACKAGE/$NAND_FUZZ_OUTPUT" ;;
esac
NAND_FUZZ_OUTPUT_DIR=$(dirname "$NAND_FUZZ_OUTPUT")
NAND_FUZZ_ARTIFACT_DIR="$NAND_FUZZ_OUTPUT_DIR/fuzzer-artifacts"
NAND_FUZZ_SEED=20261126
mkdir -p "$NAND_FUZZ_OUTPUT_DIR" "$NAND_FUZZ_ARTIFACT_DIR"
"$NAND_FUZZ_COMPILER" -std=c++17 -O1 -g -fsanitize=fuzzer,address,undefined \
  -fno-omit-frame-pointer -Iinclude -Isrc -Itests \
  tests/fuzz_nand_flash.cpp src/nand_flash.cpp -o "$NAND_FUZZ_OUTPUT"
"$NAND_FUZZ_OUTPUT" -max_total_time="$NAND_FUZZ_DURATION" -max_len=128 \
  -seed="$NAND_FUZZ_SEED" -artifact_prefix="$NAND_FUZZ_ARTIFACT_DIR/"
