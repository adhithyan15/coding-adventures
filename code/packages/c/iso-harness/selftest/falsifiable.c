/*
 * falsifiable.c — proves every iso_test.h check can FAIL.
 *
 * A check that never fails passes every test it guards. conforming.c only
 * shows the checks accept good values; this file feeds each one a value it
 * must reject and counts the failures. The program exits 0 only when exactly
 * the expected checks failed, so it is a positive fixture: build and run it
 * like conforming.c. The FAIL lines it prints are expected.
 *
 * NaN and infinity are made at run time from a volatile zero: a constant
 * 0.0 / 0.0 is a compile error on MSVC, and the harness forbids <math.h>.
 */
#include "iso_test.h"

int main(void) {
    volatile double zero = 0.0;
    const double not_a_number = zero / zero;
    const double infinity = 1.0 / zero;
    const unsigned char lhs[2] = {0x01, 0x02};
    const unsigned char rhs[2] = {0x01, 0x03};
    int must_fail = 0;
    int failed_after_passes;
    int eps_evaluations = 0;

    /* --- these must PASS (edge cases of ISO_CHECK_EQ_DBL) --- */
    ISO_CHECK_EQ_DBL(not_a_number, not_a_number, 0.0);
    ISO_CHECK_EQ_DBL(infinity, infinity, 0.0);
    ISO_CHECK_EQ_DBL(-infinity, -infinity, 0.0);
    ISO_CHECK_EQ_DBL(0.3, 0.3, not_a_number);
    ISO_CHECK_EQ_DBL(1.0, 1.0 + 1e-12, 1e-9);
    /* Each argument is evaluated exactly once, even when the exact-equality
     * shortcut decides the check before the tolerance is needed. */
    ISO_CHECK_EQ_DBL(1.0, 1.0, (eps_evaluations++, 1e-9));
    ISO_CHECK_EQ_INT(eps_evaluations, 1);
    failed_after_passes = iso_test_checks_failed;

    /* --- these must FAIL --- */
    printf("  (the FAIL lines below are expected)\n");
    ISO_CHECK(0); must_fail++;
    ISO_CHECK_MSG(0, "expected failure"); must_fail++;
    ISO_CHECK_EQ_INT(1, 2); must_fail++;
    ISO_CHECK_EQ_UINT(1u, 2u); must_fail++;
    ISO_CHECK_STR_EQ("a", "b"); must_fail++;
    ISO_CHECK_MEM_EQ(lhs, rhs, 2); must_fail++;
    ISO_CHECK_EQ_DBL(0.3, 0.4, 1e-9); must_fail++;
    ISO_CHECK_EQ_DBL(not_a_number, 0.3, 1e-9); must_fail++;
    ISO_CHECK_EQ_DBL(0.3, not_a_number, 1e-9); must_fail++;
    ISO_CHECK_EQ_DBL(not_a_number, 0.3, 1e300); must_fail++;
    ISO_CHECK_EQ_DBL(infinity, -infinity, 1e300); must_fail++;
    ISO_CHECK_EQ_DBL(infinity, 1.0, 1e300); must_fail++;
    ISO_CHECK_EQ_DBL(0.3, 0.31, not_a_number); must_fail++;

    if (failed_after_passes != 0 || iso_test_checks_failed != must_fail) {
        printf("  falsifiable: %d edge-case passes failed; %d of %d expected "
               "failures seen\n",
               failed_after_passes, iso_test_checks_failed, must_fail);
        return 1;
    }
    printf("  falsifiable: all %d expected failures seen\n", must_fail);
    return 0;
}
