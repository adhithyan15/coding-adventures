#!/bin/sh
# Build and run the logic-gates tests under EVERY available C compiler (pure ISO),
# via the shared iso-harness (code/packages/c/iso-harness), then repeat with the
# extra-strict conversion warnings, and again under ASan+UBSan when supported.
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

# Include both this package's headers and the harness's (for iso_test.h).
ISO_INCLUDE="include $HARNESS/include"
export ISO_INCLUDE

# On Linux CI both gcc and clang are installed — require both so the pure-ISO
# guarantee is firm rather than best-effort. Locally, use whatever is present.
if [ "${CI:-}" = "true" ] && [ "$(uname)" = "Linux" ]; then
    ISO_REQUIRE="gcc clang"
    export ISO_REQUIRE
fi

. "$HARNESS/lib/iso-lib.sh"

# Pass 0: the standard ISO harness build (-pedantic-errors -Wall -Wextra -Werror).
iso_build_and_run c logic_gates-tests tests/logic_gates_test.c src/logic_gates.c

# Pass 1 and 2: stricter warnings, then sanitizers. Same compilers the harness found.
STRICT="-std=c17 -Wall -Wextra -Wpedantic -Wconversion -Wsign-conversion -Werror"
INC="-Iinclude -I$HARNESS/include"
OUT="$ISO_BUILD_DIR"
mkdir -p "$OUT"
SAN="-fsanitize=address,undefined -fno-sanitize-recover=all"

for cc in $(iso_compilers c); do
    echo "logic-gates: [$cc] strict-warnings build"
    # shellcheck disable=SC2086
    "$cc" $STRICT $INC tests/logic_gates_test.c src/logic_gates.c -o "$OUT/$cc-strict"
    "$OUT/$cc-strict"

    # Probe: can this compiler build AND run a sanitized program here?
    printf 'int main(void){return 0;}\n' > "$OUT/san-probe.c"
    # shellcheck disable=SC2086
    if "$cc" -std=c17 $SAN "$OUT/san-probe.c" -o "$OUT/$cc-san-probe" >/dev/null 2>&1 \
        && "$OUT/$cc-san-probe" >/dev/null 2>&1; then
        echo "logic-gates: [$cc] sanitizer (address,undefined) build"
        # shellcheck disable=SC2086
        "$cc" $STRICT $SAN -g $INC tests/logic_gates_test.c src/logic_gates.c -o "$OUT/$cc-san"
        "$OUT/$cc-san"
    else
        echo "logic-gates: [$cc] SKIPPED sanitizer run (compiler cannot build/run -fsanitize=address,undefined here)"
    fi
done
