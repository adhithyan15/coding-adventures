#!/bin/sh
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"
HARNESS="$SELF/../iso-harness"
ISO_INCLUDE="include ../matrix/include ../float-math/include ../bounded-json/include $HARNESS/include"
export ISO_INCLUDE
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    ISO_REQUIRE="gcc clang"
    export ISO_REQUIRE
fi
. "$HARNESS/lib/iso-lib.sh"
# Trusted build-tool override is useful on hosts whose Python is named python,
# while Unix CI normally supplies python3. Never derived from checkpoint bytes.
PYTHON="${PYTHON:-python3}"
iso_build_and_run c neural-framework-tests tests/framework_test.c src/neural_framework.c ../matrix/src/matrix.c ../float-math/src/float_math.c ../bounded-json/src/bounded_json.c
iso_build_and_run c native-xor examples/xor.c src/neural_framework.c ../matrix/src/matrix.c ../float-math/src/float_math.c ../bounded-json/src/bounded_json.c
# Compilation/discovery stays with the shared harness. Exercise every binary it
# produced, not just the first available compiler's output.
for compiler in $(iso_compilers c); do
    executable="$(cd "$ISO_BUILD_DIR" && pwd)/$compiler-native-xor"
    "$PYTHON" tests/command_test.py "$executable"
    "$PYTHON" ../../../scripts/validate_neural_framework_c.py --executable "$executable"
done
"$PYTHON" ../../../scripts/tests/test_neural_framework_c.py
