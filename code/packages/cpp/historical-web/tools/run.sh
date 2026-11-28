#!/bin/sh
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
case "$SELF" in
    *[[:space:]]*) echo "historical-web: package path contains whitespace: $SELF" >&2; exit 1 ;;
esac
cd "$SELF"
d="$SELF"
while [ "$d" != "/" ] && [ ! -d "$d/code/packages/c/iso-harness" ]; do d=$(dirname "$d"); done
HARNESS="$d/code/packages/c/iso-harness"
CENG="$d/code/packages/c/test-bootstrap"
if [ ! -f "$HARNESS/lib/iso-lib.sh" ] || [ ! -f "$CENG/include/pbt.h" ]; then
    echo "historical-web: iso-harness or c/test-bootstrap not found" >&2; exit 1
fi
CPP_BOOT="$d/code/packages/cpp/test-bootstrap"
ISO_INCLUDE="include tests $CPP_BOOT/include $CENG/include $HARNESS/include ../bounded-json/include"
export ISO_INCLUDE
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    ISO_REQUIRE="g++ clang++ gcc clang"; export ISO_REQUIRE
fi
. "$HARNESS/lib/iso-lib.sh"
iso__require_check cpp || exit 1
LIB="pbt_big pbt_prng pbt_val pbt_budget pbt_shrink pbt_gen pbt_runner pbt_json pbt_tfv pbt_hooks fuzz_hooks fuzz_mutate fuzz_bitmap fuzz_sig fuzz_engine"
THREAD_FLAGS=""
if [ "$(uname)" = "Linux" ]; then THREAD_FLAGS="-pthread"; fi
CFLAGS_ISO="$(iso__strict_flags c)"
CXXFLAGS_ISO="$(iso__strict_flags cpp)"
INC="$(iso__include_flags)"
SAN="-fsanitize=address,undefined -fno-sanitize-recover=all -fno-omit-frame-pointer -g"
mkdir -p "$ISO_BUILD_DIR"
c_partner() { case "$1" in g++) echo gcc ;; clang++) echo clang ;; *) echo cc ;; esac; }
run_pass() {
    _label="$1"; _cxx="$2"; _extra="$3"; _cc=$(c_partner "$_cxx")
    _objdir="$ISO_BUILD_DIR/$_label-obj"; mkdir -p "$_objdir"; _objs=""
    for _f in $LIB; do
        # shellcheck disable=SC2086
        "$_cc" $CFLAGS_ISO $_extra $INC -c "$CENG/src/$_f.c" -o "$_objdir/$_f.o"
        _objs="$_objs $_objdir/$_f.o"
    done
    echo "historical-web: [$_label] assurance suite"
    # shellcheck disable=SC2086
    "$_cxx" $CXXFLAGS_ISO $_extra $THREAD_FLAGS $INC tests/historical_web_test.cpp src/historical_web.cpp $_objs -lm -o "$ISO_BUILD_DIR/$_label-historical-web"
    "$ISO_BUILD_DIR/$_label-historical-web"
}
for cxx in $(iso_compilers cpp); do run_pass "$cxx" "$cxx" ""; done
printf 'int main(){return 0;}\n' > "$ISO_BUILD_DIR/san_probe.cpp"
san_ran=0
for cxx in $(iso_compilers cpp); do
    # shellcheck disable=SC2086
    if "$cxx" $SAN "$ISO_BUILD_DIR/san_probe.cpp" -o "$ISO_BUILD_DIR/san_probe" >/dev/null 2>&1 && "$ISO_BUILD_DIR/san_probe" >/dev/null 2>&1; then
        run_pass "$cxx+asan+ubsan" "$cxx" "$SAN"; san_ran=1
    else
        echo "historical-web: [$cxx] sanitizer runtime unavailable; skipping sanitizer pass"
    fi
done
if [ "$san_ran" = "0" ]; then echo "historical-web: warning: no sanitizer pass ran" >&2; fi
