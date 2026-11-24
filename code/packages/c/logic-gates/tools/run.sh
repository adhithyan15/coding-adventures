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

SUMMARY="harness"
SAN_RAN=""
SAN_SKIPPED=""
NODISCARD_RAN=""
NODISCARD_SKIPPED=""

# Negative control (LGE-C2, CWE-252): a TU that DISCARDS a gate result must be
# rejected under -Werror, proving LG_NODISCARD is live. Compilers without the
# attribute are skipped with a printed reason.
printf '#include "logic_gates.h"\nint main(void){ lg_and(1, 1); return 0; }\n' > "$OUT/discard.c"
printf '#include "logic_gates.h"\nint main(void){ return lg_and(1, 1) == 1 ? 0 : 1; }\n' > "$OUT/use.c"

for cc in $(iso_compilers c); do
    echo "logic-gates: [$cc] strict-warnings build"
    # shellcheck disable=SC2086
    "$cc" $STRICT $INC tests/logic_gates_test.c src/logic_gates.c -o "$OUT/$cc-strict"
    "$OUT/$cc-strict"
    SUMMARY="$SUMMARY strict:$cc"

    # nodiscard negative control. The positive twin must compile, so a failure
    # of discard.c is attributable to the ignored result and nothing else.
    # shellcheck disable=SC2086
    if "$cc" -std=c17 -Wall -Wextra -Werror $INC -c "$OUT/use.c" -o "$OUT/use-$cc.o" >/dev/null 2>&1; then
        # shellcheck disable=SC2086
        if "$cc" -std=c17 -Wall -Wextra -Werror $INC -c "$OUT/discard.c" -o "$OUT/discard-$cc.o" >/dev/null 2>&1; then
            case "$cc" in
                gcc|clang|cc|*-gcc|*-clang)
                    echo "logic-gates: [$cc] FAIL: discarding a result compiled; LG_NODISCARD is not effective" >&2
                    exit 1 ;;
            esac
            echo "logic-gates: [$cc] SKIPPED nodiscard check (compiler has no warn_unused_result support)"
            NODISCARD_SKIPPED="$NODISCARD_SKIPPED $cc"
        else
            echo "logic-gates: [$cc] nodiscard negative control: discarded result rejected (ok)"
            NODISCARD_RAN="$NODISCARD_RAN $cc"
        fi
    else
        echo "logic-gates: [$cc] FAIL: positive twin of nodiscard check did not compile" >&2
        exit 1
    fi

    # Probe: can this compiler build AND run a sanitized program here?
    printf 'int main(void){return 0;}\n' > "$OUT/san-probe.c"
    # shellcheck disable=SC2086
    if "$cc" -std=c17 $SAN "$OUT/san-probe.c" -o "$OUT/$cc-san-probe" >/dev/null 2>&1 \
        && "$OUT/$cc-san-probe" >/dev/null 2>&1; then
        echo "logic-gates: [$cc] sanitizer (address,undefined) build"
        # shellcheck disable=SC2086
        "$cc" $STRICT $SAN -g $INC tests/logic_gates_test.c src/logic_gates.c -o "$OUT/$cc-san"
        "$OUT/$cc-san"
        SAN_RAN="$SAN_RAN $cc"
    else
        echo "logic-gates: [$cc] SKIPPED sanitizer run (compiler cannot build/run -fsanitize=address,undefined here)"
        SAN_SKIPPED="$SAN_SKIPPED $cc"
    fi
done

# Fail closed in CI (LGE-C3): a sanitizer run that silently vanishes is a
# silently weaker gate. Require at least one compiler to have run it.
if [ -z "$SAN_RAN" ]; then
    if [ -n "${CI:-}" ]; then
        echo "logic-gates: FAIL: CI is set but no compiler could build and run ASan+UBSan (skipped:$SAN_SKIPPED)" >&2
        exit 1
    fi
    echo "logic-gates: *** WARNING: NO SANITIZER RUN HAPPENED (set CI=1 to make this fatal) ***"
fi

echo "logic-gates: SUMMARY builds ran: $SUMMARY | sanitizers ran:${SAN_RAN:- none} skipped:${SAN_SKIPPED:- none} | nodiscard ran:${NODISCARD_RAN:- none} skipped:${NODISCARD_SKIPPED:- none}"
