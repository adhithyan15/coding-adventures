#!/bin/sh
# Build and run the logic-gates tests under EVERY available C compiler (pure ISO),
# via the shared iso-harness (code/packages/c/iso-harness), then repeat with the
# extra-strict conversion warnings, check LG_NODISCARD, and run under
# ASan+UBSan when supported.
set -e
SELF=$(cd "$(dirname "$0")/.." && pwd)
cd "$SELF"

# One CI rule everywhere: CI is true iff $CI is set to something other than
# empty, "false" or "0". Under CI every skip below is FATAL (fail closed).
is_ci() {
    case "${CI:-}" in
        ""|false|0) return 1 ;;
        *) return 0 ;;
    esac
}

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
# Every compiler found (so, under CI, every required one) must then also run
# the nodiscard control and the sanitizer pass below.
if is_ci && [ "$(uname)" = "Linux" ]; then
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
# rejected, and the rejection must be FOR THAT REASON: with only
# -Werror=unused-result (no other warning noise) the compiler's stderr must
# mention unused-result / warn_unused_result / nodiscard. A compile failure
# without that text is a FAIL (it could be any unrelated error). The positive
# twin (uses the result) must compile under the same flags. Run once per
# standard: c17 exercises the __attribute__ branch, c23 the [[nodiscard]] one.
printf '#include "logic_gates.h"\nint main(void){ lg_and(1, 1); return 0; }\n' > "$OUT/discard.c"
printf '#include "logic_gates.h"\nint main(void){ return lg_and(1, 1) == 1 ? 0 : 1; }\n' > "$OUT/use.c"

# nodiscard_control <cc> <std>: returns 0 if it ran OK, 1 if skipped (printed);
# exits non-zero on a real failure.
nodiscard_control() {
    nd_cc=$1
    nd_std=$2
    # shellcheck disable=SC2086
    if ! "$nd_cc" -std="$nd_std" -Werror=unused-result $INC -c "$OUT/use.c" -o "$OUT/use-$nd_cc-$nd_std.o" >/dev/null 2>&1; then
        echo "logic-gates: [$nd_cc] SKIPPED nodiscard check at -std=$nd_std (compiler rejects that standard)"
        return 1
    fi
    # shellcheck disable=SC2086
    if "$nd_cc" -std="$nd_std" -Werror=unused-result $INC -c "$OUT/discard.c" -o "$OUT/discard-$nd_cc-$nd_std.o" >"$OUT/nd.log" 2>&1; then
        echo "logic-gates: [$nd_cc] FAIL: discarding a result compiled at -std=$nd_std; LG_NODISCARD is not effective" >&2
        exit 1
    fi
    if ! grep -E 'unused-result|warn_unused_result|nodiscard' "$OUT/nd.log" >/dev/null 2>&1; then
        echo "logic-gates: [$nd_cc] FAIL: discard rejected at -std=$nd_std but not for the unused-result reason:" >&2
        cat "$OUT/nd.log" >&2
        exit 1
    fi
    echo "logic-gates: [$nd_cc] nodiscard negative control (-std=$nd_std): discarded result rejected for the right reason (ok)"
    return 0
}

for cc in $(iso_compilers c); do
    echo "logic-gates: [$cc] strict-warnings build"
    # shellcheck disable=SC2086
    "$cc" $STRICT $INC tests/logic_gates_test.c src/logic_gates.c -o "$OUT/$cc-strict"
    "$OUT/$cc-strict"
    SUMMARY="$SUMMARY strict:$cc"

    nd_ok=""
    for std in c17 c23; do
        if nodiscard_control "$cc" "$std"; then
            nd_ok="$nd_ok $std"
        else
            NODISCARD_SKIPPED="$NODISCARD_SKIPPED $cc/$std"
        fi
    done
    if [ -n "$nd_ok" ]; then
        NODISCARD_RAN="$NODISCARD_RAN $cc($nd_ok )"
    fi
    # c17 (the attribute branch) is mandatory under CI for every compiler;
    # c23 is best-effort because older compilers lack -std=c23.
    case "$nd_ok" in
        *c17*) ;;
        *)
            if is_ci; then
                echo "logic-gates: FAIL: CI requires the nodiscard control for $cc" >&2
                exit 1
            fi
            ;;
    esac

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
        if is_ci; then
            echo "logic-gates: FAIL: CI requires the sanitizer run for every compiler; $cc could not run it" >&2
            exit 1
        fi
    fi
done

# Locally (not CI), a missing sanitizer run is skipped, but loudly.
if [ -z "$SAN_RAN" ]; then
    echo "logic-gates: *** WARNING: NO SANITIZER RUN HAPPENED (set CI=1 to make this fatal) ***"
fi

echo "logic-gates: SUMMARY builds ran: $SUMMARY | sanitizers ran:${SAN_RAN:- none} skipped:${SAN_SKIPPED:- none} | nodiscard ran:${NODISCARD_RAN:- none} skipped:${NODISCARD_SKIPPED:- none}"
