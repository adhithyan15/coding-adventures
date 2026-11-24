#!/bin/sh
# Build and run the logic-gates tests with every available C++ compiler
# (clang++, g++), under strict warnings-as-errors, then again under
# AddressSanitizer + UBSan (MANDATORY; opt out only with LG_ALLOW_NO_SANITIZER=1),
# (-O1), -O0 and -O2 plain runs, plus a macro-hygiene TU and compile-must-fail tests.  If llvm-cov is
# available, also reports header line coverage (informational; must be 100%).
set -eu
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"
OUT="$SELF/_build"
mkdir -p "$OUT"

WARN="-std=c++17 -Wall -Wextra -Wpedantic -Wconversion -Wsign-conversion -Werror"
SAN="-fsanitize=address,undefined -fno-sanitize-recover=all"
TESTS="tests/logic_gates_test.cpp"
HYGIENE="tests/macro_hygiene_test.cpp"

# Sanitizers are MANDATORY: a toolchain that cannot build+run an ASan/UBSan
# program is a FAILURE, not a skip, unless the operator opts out explicitly.
san_unavailable() {
    if [ "${LG_ALLOW_NO_SANITIZER:-}" = "1" ]; then
        echo "!!!!!!!! WARNING: $1: sanitizers UNAVAILABLE; running WITHOUT ASan/UBSan because LG_ALLOW_NO_SANITIZER=1 !!!!!!!!" >&2
        return 0
    fi
    echo "FAIL: $1: cannot build/run -fsanitize=address,undefined. Install sanitizer runtimes, or set LG_ALLOW_NO_SANITIZER=1 to opt out (loudly)." >&2
    exit 1
}

# must_fail CXX NAME NEEDLE BODY: BODY must NOT compile; the diagnostics must
# mention NEEDLE (the gate under test), so an unrelated error cannot pass.
must_fail() {
    cxx=$1; name=$2; needle=$3; body=$4
    {
        echo '#include "logic_gates/logic_gates.hpp"'
        echo 'using namespace logic_gates;'
        echo 'enum Unscoped { kTwo = 2 };'
        echo "void probe() { $body }"
    } > "$OUT/nc-$name.cpp"
    if "$cxx" -std=c++17 -fsyntax-only -Iinclude "$OUT/nc-$name.cpp" > "$OUT/nc-$name.log" 2>&1; then
        echo "FAIL: [$cxx] negative-compile '$name' COMPILED but must be rejected: $body" >&2
        exit 1
    fi
    if ! grep -q "$needle" "$OUT/nc-$name.log"; then
        echo "FAIL: [$cxx] negative-compile '$name' failed for the wrong reason (no mention of $needle)" >&2
        exit 1
    fi
}

negative_compile() {
    cxx=$1
    # Positive control: the same preamble compiles with valid ints, so the
    # failures below are about the offending snippet and not the harness.
    printf '#include "logic_gates/logic_gates.hpp"\nusing namespace logic_gates;\nint probe() { return and_gate(1, 1) + not_gate(0); }\n' > "$OUT/nc-control.cpp"
    "$cxx" -std=c++17 -fsyntax-only -Iinclude "$OUT/nc-control.cpp"
    n=0
    # Deleted overloads: every gate x every non-int argument type, in each position.
    for g in not_gate nand_not; do
        for v in 1.5 1.0f true "'a'" 1LL 1u kTwo nullptr '"x"' 1L; do
            n=$((n + 1)); must_fail "$cxx" "d$n" "$g" "(void)$g($v);"
        done
    done
    for g in and_gate or_gate xor_gate nand_gate nor_gate xnor_gate nand_and nand_or nand_xor; do
        for v in 1.5 1.0f true "'a'" 1LL 1u kTwo nullptr '"x"' 1L; do
            n=$((n + 1)); must_fail "$cxx" "d$n" "$g" "(void)$g($v, 1);"
            n=$((n + 1)); must_fail "$cxx" "d$n" "$g" "(void)$g(1, $v);"
        done
    done
    # Invalid constant expressions must not compile (a throw is not constant).
    for g in not_gate nand_not; do
        n=$((n + 1)); must_fail "$cxx" "c$n" "$g" "constexpr int x = $g(2); (void)x;"
    done
    for g in and_gate or_gate xor_gate nand_gate nor_gate xnor_gate nand_and nand_or nand_xor; do
        n=$((n + 1)); must_fail "$cxx" "c$n" "$g" "constexpr int x = $g(2, 1); (void)x;"
        n=$((n + 1)); must_fail "$cxx" "c$n" "$g" "constexpr int x = $g(1, -1); (void)x;"
    done
    # Braced-init-list arguments must not slip past the deleted overloads.
    decl='bool bv = true; char cv = 1; short sv = 1; Unscoped ev = kTwo; long lv = 1; (void)bv; (void)cv; (void)sv; (void)ev; (void)lv;'
    for g in not_gate nand_not; do
        for v in '{1}' '{true}' '{bv}' '{cv}' '{sv}' '{ev}' '{1.5}' '{lv}' "{'a'}"; do
            n=$((n + 1)); must_fail "$cxx" "b$n" "$g" "$decl (void)$g($v);"
        done
    done
    for g in and_gate or_gate xor_gate nand_gate nor_gate xnor_gate nand_and nand_or nand_xor; do
        for v in '{1}' '{true}' '{bv}' '{cv}' '{sv}' '{ev}' '{1.5}' '{lv}' "{'a'}"; do
            n=$((n + 1)); must_fail "$cxx" "b$n" "$g" "$decl (void)$g($v, 1);"
            n=$((n + 1)); must_fail "$cxx" "b$n" "$g" "$decl (void)$g(1, $v);"
            n=$((n + 1)); must_fail "$cxx" "b$n" "$g" "$decl (void)$g($v, $v);"
        done
    done
    # std::function cannot bind the overload set (only an explicit pointer type can).
    n=$((n + 1))
    printf '#include <functional>\n#include "logic_gates/logic_gates.hpp"\nstd::function<int(int,int)> g = logic_gates::and_gate;\n' > "$OUT/nc-stdfn.cpp"
    if "$cxx" -std=c++17 -fsyntax-only -Iinclude "$OUT/nc-stdfn.cpp" >/dev/null 2>&1; then
        echo "FAIL: [$cxx] std::function<int(int,int)> = and_gate compiled; README/header claim is wrong" >&2
        exit 1
    fi
    # Explicit pointer type / cast / lambda DO reach the int overload.
    printf '#include "logic_gates/logic_gates.hpp"\nint (*f)(int,int) = &logic_gates::and_gate;\nauto g = static_cast<int(*)(int,int)>(logic_gates::and_gate);\nauto h = [](int x,int y){ return logic_gates::and_gate(x,y); };\n' > "$OUT/nc-ptr-ok.cpp"
    "$cxx" -std=c++17 -fsyntax-only -Iinclude "$OUT/nc-ptr-ok.cpp"
    # [[nodiscard]]: discarding a result is a warning, an error under -Werror.
    n=$((n + 1))
    printf '#include "logic_gates/logic_gates.hpp"\nvoid probe() { logic_gates::and_gate(1, 1); }\n' > "$OUT/nc-nodiscard.cpp"
    if "$cxx" -std=c++17 -Werror -fsyntax-only -Iinclude "$OUT/nc-nodiscard.cpp" >/dev/null 2>&1; then
        echo "FAIL: [$cxx] discarding a gate result compiled under -Werror" >&2
        exit 1
    fi
    echo "   negative-compile: $n snippets rejected as required"
}

# A gate throwing out of a noexcept caller must terminate the process (the gates
# are deliberately not noexcept; documented in the header).  Child process.
noexcept_terminates() {
    cxx=$1
    cat > "$OUT/noexcept-death.cpp" <<'EOS'
#include "logic_gates/logic_gates.hpp"
int main() {
  auto f = []() noexcept { return logic_gates::and_gate(5, 1); };
  return f();
}
EOS
    "$cxx" -std=c++17 -Iinclude "$OUT/noexcept-death.cpp" -o "$OUT/noexcept-death-$cxx"
    if "$OUT/noexcept-death-$cxx" >/dev/null 2>&1; then
        echo "FAIL: [$cxx] noexcept caller of and_gate(5,1) did not terminate" >&2
        exit 1
    fi
    echo "   noexcept caller + invalid input terminates (as documented)"
}

ran=0
for cxx in clang++ g++; do
    if ! command -v "$cxx" >/dev/null 2>&1; then
        echo "[skip] $cxx not found"
        continue
    fi
    echo "== $cxx: strict warnings (-O0) =="
    # shellcheck disable=SC2086
    "$cxx" $WARN -O0 -Iinclude $TESTS -o "$OUT/test-$cxx"
    "$OUT/test-$cxx"
    echo "== $cxx: strict warnings (-O2) =="
    # shellcheck disable=SC2086
    "$cxx" $WARN -O2 -Iinclude $TESTS -o "$OUT/test-o2-$cxx"
    "$OUT/test-o2-$cxx"
    echo "== $cxx: macro hygiene (user macros defined before the include) =="
    # shellcheck disable=SC2086
    "$cxx" $WARN -Iinclude $HYGIENE -o "$OUT/hygiene-$cxx"
    "$OUT/hygiene-$cxx"
    echo "== $cxx: negative-compile =="
    negative_compile "$cxx"
    noexcept_terminates "$cxx"
    ran=$((ran + 1))

    # Probe sanitizer support with a trivial program.
    echo 'int main() { return 0; }' > "$OUT/probe.cpp"
    # shellcheck disable=SC2086
    if "$cxx" $SAN "$OUT/probe.cpp" -o "$OUT/probe-$cxx" >/dev/null 2>&1 && "$OUT/probe-$cxx" >/dev/null 2>&1; then
        echo "== $cxx: ASan + UBSan (-O1) =="
        # shellcheck disable=SC2086
        "$cxx" $WARN -O1 -g $SAN -Iinclude $TESTS -o "$OUT/test-san-$cxx"
        "$OUT/test-san-$cxx"
    else
        san_unavailable "$cxx"
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
