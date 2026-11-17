#!/bin/sh
# Build and run the trig tests under EVERY available C/C++ compiler (pure ISO),
# via the shared iso-harness (code/packages/c/iso-harness).
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
iso_build_and_run cpp trig-tests tests/trig_test.cpp

# The bit-exact conformance test: every row of code/specs/fixtures/trig-reference-v1
# (PHY00a 6) must match bit for bit, and sin/cos/radians/degrees must stay within
# their ulp limits of the correctly rounded column. It runs from this directory.
iso_build_and_run cpp trig-conformance tests/conformance_test.cpp

# ---------------------------------------------------------------------------
# Fusion canary (PHY00a 5, TC-5). A header is compiled with the CONSUMER's flags,
# so trig.hpp guards itself against fused multiply-add. Two extra builds, with
# optimisation and FMA hardware flags the strict harness does not use, prove it:
#
#   1. guards ON, the compiler's default contraction, FMA enabled: the vectors
#      must PASS (the in-header guards hold);
#   2. guards compiled out (-DCA_TRIG_NO_FP_GUARDS), -ffp-contract=fast: the
#      vectors must FAIL (fusion really changes bits, so the vectors can see it,
#      and build 1 is not passing by accident).
#
# Skipped, loudly, on hosts where we cannot enable and run FMA code.
# ---------------------------------------------------------------------------
fma_flag=""
fma_ok=0
case "$(uname -m)" in
    x86_64|amd64)
        fma_flag="-mfma"
        if grep -qw fma /proc/cpuinfo 2>/dev/null; then fma_ok=1; fi ;;
    aarch64|arm64)
        fma_ok=1 ;;  # FMA is part of the base ISA
esac
if [ "$fma_ok" = 1 ]; then
    base_flags="$(iso__strict_flags cpp) $(iso__include_flags) -O2 $fma_flag"
    for cc in $(iso_compilers cpp); do
        echo "trig: [$cc] fusion canary 1/2: guards on, default contraction (must pass)"
        # shellcheck disable=SC2086
        "$cc" $base_flags tests/conformance_test.cpp -o "$ISO_BUILD_DIR/$cc-trig-guarded"
        "$ISO_BUILD_DIR/$cc-trig-guarded" >/dev/null || {
            echo "trig: [$cc] the in-header FMA guards did NOT hold" >&2
            exit 1
        }
        echo "trig: [$cc] fusion canary 2/2: guards off, -ffp-contract=fast (must fail)"
        # shellcheck disable=SC2086
        "$cc" $base_flags -ffp-contract=fast -DCA_TRIG_NO_FP_GUARDS \
            tests/conformance_test.cpp -o "$ISO_BUILD_DIR/$cc-trig-canary"
        if "$ISO_BUILD_DIR/$cc-trig-canary" >/dev/null; then
            echo "trig: [$cc] the fusion canary PASSED: the vectors cannot see fused arithmetic" >&2
            exit 1
        fi
    done
else
    echo "trig: fusion canary SKIPPED (no FMA-capable x86-64 or arm64 host)"
fi
