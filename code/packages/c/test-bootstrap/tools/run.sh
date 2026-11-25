#!/bin/sh
# Build and run the test-bootstrap tests under EVERY available C compiler (pure ISO),
# via the shared iso-harness (code/packages/c/iso-harness), then once more under
# AddressSanitizer + UndefinedBehaviorSanitizer when a compiler can link it.
#
# Several test executables (one `main` each) share the library sources; every one
# must pass under every compiler.
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"

# Locate the iso-harness by walking up to the repo dir that contains it.
d="$SELF"
while [ "$d" != "/" ] && [ ! -d "$d/code/packages/c/iso-harness" ]; do
    d=$(dirname "$d")
done
HARNESS="$d/code/packages/c/iso-harness"
if [ ! -f "$HARNESS/lib/iso-lib.sh" ]; then
    echo "iso-harness not found (searched upward from $SELF)" >&2
    exit 1
fi

# This package's headers, its internal header (tests use the allocator), and the harness's iso_test.h.
ISO_INCLUDE="include src tests $HARNESS/include"
export ISO_INCLUDE

# On Linux CI both gcc and clang are installed - require both so the pure-ISO
# guarantee is firm rather than best-effort. Locally, use whatever is present.
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    ISO_REQUIRE="gcc clang"
    export ISO_REQUIRE
fi

. "$HARNESS/lib/iso-lib.sh"

LIB_SRCS="src/pbt_big.c src/pbt_prng.c src/pbt_val.c src/pbt_budget.c src/pbt_shrink.c src/pbt_gen.c src/pbt_runner.c src/pbt_json.c src/pbt_tfv.c src/pbt_corpus.c src/pbt_hooks.c"
TESTS="test_big test_prng test_json_tfv test_val test_budget test_engine test_oom test_corpus"

for t in $TESTS; do
    echo "== $t"
    # shellcheck disable=SC2086  # word-splitting of the source list is intended
    iso_build_and_run c "pbt-$t" "tests/$t.c" tests/test_util.c $LIB_SRCS
done

# ---- ASan + UBSan pass --------------------------------------------------------
# The harness has no sanitizer mode, so build each test directly with the harness's strict
# flags plus the sanitizers.  A compiler that cannot link the runtime skips (and says so).
san_flags="-fsanitize=address,undefined -fno-sanitize-recover=all -fno-omit-frame-pointer -g"
inc=""
for dir in $ISO_INCLUDE; do inc="$inc -I$dir"; done
mkdir -p "$ISO_BUILD_DIR"
printf 'int main(void){return 0;}\n' > "$ISO_BUILD_DIR/san_probe.c"
san_ran=0
for cc in $(iso_compilers c); do
    # shellcheck disable=SC2086
    if "$cc" $san_flags "$ISO_BUILD_DIR/san_probe.c" -o "$ISO_BUILD_DIR/san_probe" >/dev/null 2>&1 && "$ISO_BUILD_DIR/san_probe" >/dev/null 2>&1; then
        for t in $TESTS; do
            echo "iso-harness: [$cc +asan+ubsan] $t"
            # shellcheck disable=SC2086
            "$cc" -std="$ISO_CSTD" -pedantic-errors -Wall -Wextra -Werror $san_flags $inc "tests/$t.c" tests/test_util.c $LIB_SRCS -o "$ISO_BUILD_DIR/san-$cc-$t"
            "$ISO_BUILD_DIR/san-$cc-$t"
        done
        san_ran=1
    else
        echo "iso-harness: [$cc] cannot link the sanitizer runtime; skipping the ASan+UBSan pass for it"
    fi
done
if [ "$san_ran" = "0" ]; then
    echo "iso-harness: WARNING no compiler could run the ASan+UBSan pass" >&2
fi
