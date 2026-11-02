#!/bin/sh
# Compiler selection remains owned by the shared ISO harness.
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"
HARNESS="$SELF/../../c/iso-harness"
ISO_INCLUDE="include ../matrix/include ../float-math/include ../bounded-json/include $HARNESS/include"
export ISO_INCLUDE
if [ "${CI:-}" = true ] && [ "$(uname)" = Linux ]; then ISO_REQUIRE='gcc clang'; export ISO_REQUIRE; fi
. "$HARNESS/lib/iso-lib.sh"
PYTHON="${PYTHON:-python3}"
iso_build_and_run cpp neural-framework-tests tests/network_test.cpp tests/allocation.cpp tests/allocation_hooks.cpp
iso_build_and_run cpp native-xor examples/xor.cpp > "$ISO_BUILD_DIR/native-xor-harness.log"
for compiler in $(iso_compilers cpp); do
    executable="$(cd "$ISO_BUILD_DIR" && pwd)/$compiler-native-xor"
    if [ -f "$executable.exe" ]; then executable="$executable.exe"; fi
    "$PYTHON" tests/command_test.py "$executable"
done
