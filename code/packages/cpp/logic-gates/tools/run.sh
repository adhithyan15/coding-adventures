#!/bin/sh
# Build and run the logic-gates tests with every available C++ compiler
# (clang++, g++), under strict warnings-as-errors, then again under
# AddressSanitizer + UBSan when the toolchain supports it.  If llvm-cov is
# available, also reports header line coverage (informational; must be 100%).
set -eu
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"
OUT="$SELF/_build"
mkdir -p "$OUT"

WARN="-std=c++17 -Wall -Wextra -Wpedantic -Wconversion -Wsign-conversion -Werror"
ran=0
for cxx in clang++ g++; do
    if ! command -v "$cxx" >/dev/null 2>&1; then
        echo "[skip] $cxx not found"
        continue
    fi
    echo "== $cxx: strict warnings =="
    # shellcheck disable=SC2086
    "$cxx" $WARN -Iinclude tests/logic_gates_test.cpp -o "$OUT/test-$cxx"
    "$OUT/test-$cxx"
    ran=$((ran + 1))

    # Probe sanitizer support with a trivial program.
    echo 'int main() { return 0; }' > "$OUT/probe.cpp"
    if "$cxx" -fsanitize=address,undefined -fno-sanitize-recover=all \
        "$OUT/probe.cpp" -o "$OUT/probe-$cxx" >/dev/null 2>&1 && "$OUT/probe-$cxx" >/dev/null 2>&1; then
        echo "== $cxx: ASan + UBSan =="
        # shellcheck disable=SC2086
        "$cxx" $WARN -g -fsanitize=address,undefined -fno-sanitize-recover=all \
            -Iinclude tests/logic_gates_test.cpp -o "$OUT/test-san-$cxx"
        "$OUT/test-san-$cxx"
    else
        echo "[skip] $cxx: sanitizers unsupported on this toolchain"
    fi
done
if [ "$ran" -eq 0 ]; then
    echo "no C++ compiler found" >&2
    exit 1
fi

# Coverage (clang only, optional).
PROF=$(command -v llvm-profdata || xcrun -f llvm-profdata 2>/dev/null || true)
COV=$(command -v llvm-cov || xcrun -f llvm-cov 2>/dev/null || true)
if command -v clang++ >/dev/null 2>&1 && [ -n "$PROF" ] && [ -n "$COV" ]; then
    echo "== coverage (clang source-based) =="
    clang++ -std=c++17 -fprofile-instr-generate -fcoverage-mapping -Iinclude \
        tests/logic_gates_test.cpp -o "$OUT/test-cov"
    LLVM_PROFILE_FILE="$OUT/cov.profraw" "$OUT/test-cov" >/dev/null
    "$PROF" merge -sparse "$OUT/cov.profraw" -o "$OUT/cov.profdata"
    "$COV" report "$OUT/test-cov" -instr-profile="$OUT/cov.profdata" include/logic_gates/logic_gates.hpp
else
    echo "[skip] coverage: llvm-cov/llvm-profdata not found"
fi
echo "logic-gates: OK"
