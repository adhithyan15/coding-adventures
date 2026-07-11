/*
 * iso_test.h — a tiny, dependency-free unit-test harness in pure ISO C/C++.
 * ===========================================================================
 *
 * WHY A HAND-ROLLED HEADER?
 * -------------------------
 * The whole point of the ISO lane is portability: a package here must compile
 * under GCC, Clang, and MSVC with zero non-standard extensions. A third-party
 * test framework would drag in its own portability assumptions (and an external
 * dependency the repo forbids for these packages). So the test harness is a
 * single header of the *intersection* of ISO C17 and ISO C++17 — it compiles,
 * unmodified, as either language.
 *
 * HOW IT WORKS
 * ------------
 * There is no test registration and no constructors (pure C has neither). A test
 * file is just a `main()` that runs checks and returns ISO_TEST_RESULT():
 *
 *     #include "iso_test.h"
 *
 *     int main(void) {
 *         ISO_CHECK(1 + 1 == 2);
 *         ISO_CHECK_EQ_INT(ring_size(&r), 3);
 *         ISO_CHECK_MSG(ptr != NULL, "allocation must succeed");
 *         return ISO_TEST_RESULT();   // 0 if all passed, 1 otherwise
 *     }
 *
 * The counters have internal linkage (`static`), so the header is meant to be
 * included in exactly one translation unit per test executable — which is how
 * the harness compiles each test file on its own.
 *
 * Everything here is C89-clean apart from the C99/C++ `//`-free comments, so it
 * is comfortably inside C17 and C++17 with -pedantic-errors / /permissive-.
 */
#ifndef ISO_TEST_H
#define ISO_TEST_H

#include <stdio.h>

/* Test bookkeeping. `static` gives these internal linkage so including the
 * header in a single-file test never causes multiple-definition errors. */
static int iso_test_checks_run = 0;
static int iso_test_checks_failed = 0;

/* ISO_CHECK(cond) — record a boolean assertion. On failure it prints the file,
 * line, and the stringized condition, then keeps going so one run reports every
 * failure rather than stopping at the first. */
#define ISO_CHECK(cond)                                                        \
    do {                                                                       \
        iso_test_checks_run++;                                                 \
        if (!(cond)) {                                                         \
            iso_test_checks_failed++;                                          \
            printf("  FAIL %s:%d  ISO_CHECK(%s)\n",                            \
                   __FILE__, __LINE__, #cond);                                 \
        }                                                                      \
    } while (0)

/* ISO_CHECK_MSG(cond, msg) — like ISO_CHECK but with a human-readable note. */
#define ISO_CHECK_MSG(cond, msg)                                               \
    do {                                                                       \
        iso_test_checks_run++;                                                 \
        if (!(cond)) {                                                         \
            iso_test_checks_failed++;                                          \
            printf("  FAIL %s:%d  %s\n", __FILE__, __LINE__, (msg));           \
        }                                                                      \
    } while (0)

/* ISO_CHECK_EQ_INT(a, b) — integer equality with both values printed on
 * failure. Values are widened to `long` for a single portable printf format. */
#define ISO_CHECK_EQ_INT(a, b)                                                 \
    do {                                                                       \
        long iso_a_ = (long)(a);                                               \
        long iso_b_ = (long)(b);                                               \
        iso_test_checks_run++;                                                 \
        if (iso_a_ != iso_b_) {                                                \
            iso_test_checks_failed++;                                          \
            printf("  FAIL %s:%d  ISO_CHECK_EQ_INT(%s, %s): %ld != %ld\n",     \
                   __FILE__, __LINE__, #a, #b, iso_a_, iso_b_);                \
        }                                                                      \
    } while (0)

/* ISO_TEST_RESULT() — print a one-line summary and yield a process exit code:
 * 0 when every check passed, 1 otherwise. Use it as `return ISO_TEST_RESULT();`
 * at the end of main(). */
#define ISO_TEST_RESULT()                                                      \
    (printf("  %d checks, %d failed\n",                                        \
            iso_test_checks_run, iso_test_checks_failed),                      \
     iso_test_checks_failed == 0 ? 0 : 1)

#endif /* ISO_TEST_H */
