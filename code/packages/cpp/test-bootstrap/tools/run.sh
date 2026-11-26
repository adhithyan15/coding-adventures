#!/bin/sh
# Build and run the C++ test-bootstrap tests under EVERY available C++ compiler
# (pure ISO C++17), then once more under AddressSanitizer + UndefinedBehaviorSanitizer.
#
# The binding links the C engine of c/test-bootstrap (VER05i), so for each C++
# compiler the C sources are compiled with its C partner (g++ -> gcc, clang++ ->
# clang) in strict ISO C17, the tests in strict ISO C++17, and the C++ driver
# links both.  The harness's one-command helper would compile the .c files as C++,
# so this script drives the compilers itself, with the harness's own flags.
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
case "$SELF" in
    *[[:space:]]*)
        echo "tools/run.sh: the package path contains whitespace ($SELF); the harness splits include and source lists on spaces, so move the checkout to a path without spaces" >&2
        exit 1
        ;;
esac
cd "$SELF"

# Locate the repo root by walking up to the dir that contains the iso-harness.
d="$SELF"
while [ "$d" != "/" ] && [ ! -d "$d/code/packages/c/iso-harness" ]; do
    d=$(dirname "$d")
done
HARNESS="$d/code/packages/c/iso-harness"
CENG="$d/code/packages/c/test-bootstrap"
if [ ! -f "$HARNESS/lib/iso-lib.sh" ] || [ ! -f "$CENG/include/pbt.h" ]; then
    echo "iso-harness or c/test-bootstrap not found (searched upward from $SELF)" >&2
    exit 1
fi

# This package's header, its tests' shared header, the C engine's public and
# internal headers (the tests use the allocator seam) and the harness's iso_test.h.
ISO_INCLUDE="include tests $CENG/include $CENG/src $HARNESS/include"
export ISO_INCLUDE

# On Linux CI both toolchains are installed - require them so the guarantee is firm.
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    ISO_REQUIRE="g++ clang++ gcc clang"
    export ISO_REQUIRE
fi

. "$HARNESS/lib/iso-lib.sh"
iso__require_check cpp || exit 1

C_LIB="pbt_big pbt_prng pbt_val pbt_budget pbt_shrink pbt_gen pbt_runner pbt_json pbt_tfv pbt_hooks"
TESTS="test_api test_exceptions test_alloc_throw test_corpus"
CFLAGS_ISO="$(iso__strict_flags c)"
CXXFLAGS_ISO="$(iso__strict_flags cpp)"
INC="$(iso__include_flags)"
SAN="-fsanitize=address,undefined -fno-sanitize-recover=all -fno-omit-frame-pointer -g"
mkdir -p "$ISO_BUILD_DIR"

# The C partner of a C++ compiler.
c_partner() {
    case "$1" in
        g++) echo gcc ;;
        clang++) echo clang ;;
        *) echo cc ;;
    esac
}

# run_pass LABEL CXX EXTRA_FLAGS: compile the C engine and every test, run each test.
run_pass() {
    _label="$1"; _cxx="$2"; _extra="$3"
    _cc=$(c_partner "$_cxx")
    if ! command -v "$_cc" >/dev/null 2>&1; then
        echo "iso-harness: ERROR C partner $_cc of $_cxx not found" >&2
        return 1
    fi
    _objdir="$ISO_BUILD_DIR/$_label-obj"
    mkdir -p "$_objdir"
    _objs=""
    for _f in $C_LIB; do
        # shellcheck disable=SC2086  # word-splitting of the flag strings is intended
        "$_cc" $CFLAGS_ISO $_extra $INC -c "$CENG/src/$_f.c" -o "$_objdir/$_f.o"
        _objs="$_objs $_objdir/$_f.o"
    done
    for _t in $TESTS; do
        echo "iso-harness: [$_label] $_t"
        # shellcheck disable=SC2086
        "$_cxx" $CXXFLAGS_ISO $_extra $INC "tests/$_t.cpp" $_objs -o "$ISO_BUILD_DIR/$_label-$_t"
        "$ISO_BUILD_DIR/$_label-$_t"
    done
}

for cxx in $(iso_compilers cpp); do
    run_pass "$cxx" "$cxx" ""
done

# ---- ASan + UBSan pass ---------------------------------------------------------------
printf 'int main(){return 0;}\n' > "$ISO_BUILD_DIR/san_probe.cpp"
san_ran=0
for cxx in $(iso_compilers cpp); do
    # shellcheck disable=SC2086
    if "$cxx" $SAN "$ISO_BUILD_DIR/san_probe.cpp" -o "$ISO_BUILD_DIR/san_probe" >/dev/null 2>&1 && "$ISO_BUILD_DIR/san_probe" >/dev/null 2>&1; then
        run_pass "$cxx+asan+ubsan" "$cxx" "$SAN"
        san_ran=1
    else
        echo "iso-harness: [$cxx] cannot link the sanitizer runtime; skipping the ASan+UBSan pass for it"
    fi
done
if [ "$san_ran" = "0" ]; then
    echo "iso-harness: WARNING no compiler could run the ASan+UBSan pass" >&2
fi
