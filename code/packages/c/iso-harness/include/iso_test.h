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
#include <string.h> /* strcmp, memcmp — for ISO_CHECK_STR_EQ / ISO_CHECK_MEM_EQ */

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

/* ISO_CHECK_EQ_UINT(a, b) — unsigned integer equality (e.g. size_t results).
 * Both sides widen to `unsigned long` for one portable printf format. */
#define ISO_CHECK_EQ_UINT(a, b)                                                \
    do {                                                                       \
        unsigned long iso_a_ = (unsigned long)(a);                            \
        unsigned long iso_b_ = (unsigned long)(b);                            \
        iso_test_checks_run++;                                                 \
        if (iso_a_ != iso_b_) {                                                \
            iso_test_checks_failed++;                                          \
            printf("  FAIL %s:%d  ISO_CHECK_EQ_UINT(%s, %s): %lu != %lu\n",    \
                   __FILE__, __LINE__, #a, #b, iso_a_, iso_b_);                \
        }                                                                      \
    } while (0)

/* ISO_CHECK_STR_EQ(a, b) — NUL-terminated C-string equality via strcmp. Prints
 * both strings on failure.
 *
 * Temporary-safe by design: it never stores the char pointers across statements.
 * That matters in C++, where the common idiom passes `temporary.c_str()` — if we
 * saved that pointer in a local it would dangle the moment the temporary string
 * died (GCC/Clang catch this as -Wdangling-gsl under -Werror). Instead each of
 * `a` and `b` is evaluated inside the strcmp/printf full-expression, so any
 * temporary lives exactly as long as the call that reads it.
 *
 * Trade-off: on the FAILURE path `a` and `b` are evaluated a second time (for
 * the printf), so pass pure expressions — which test assertions always are. */
#define ISO_CHECK_STR_EQ(a, b)                                                 \
    do {                                                                       \
        iso_test_checks_run++;                                                 \
        if (strcmp((a), (b)) != 0) {                                           \
            iso_test_checks_failed++;                                          \
            printf("  FAIL %s:%d  ISO_CHECK_STR_EQ(%s, %s): \"%s\" != \"%s\"\n",\
                   __FILE__, __LINE__, #a, #b, (a), (b));                      \
        }                                                                      \
    } while (0)

/* ISO_CHECK_MEM_EQ(a, b, n) — byte-wise equality of two buffers of length `n`
 * via memcmp. Ideal for hash digests, cipher output, and serialized bytes. On
 * failure it prints the index and the two differing byte values. */
#define ISO_CHECK_MEM_EQ(a, b, n)                                              \
    do {                                                                       \
        const unsigned char *iso_a_ = (const unsigned char *)(a);             \
        const unsigned char *iso_b_ = (const unsigned char *)(b);             \
        size_t iso_n_ = (size_t)(n);                                           \
        iso_test_checks_run++;                                                 \
        if (memcmp(iso_a_, iso_b_, iso_n_) != 0) {                            \
            size_t iso_i_ = 0;                                                 \
            while (iso_i_ < iso_n_ && iso_a_[iso_i_] == iso_b_[iso_i_]) {      \
                iso_i_++;                                                      \
            }                                                                  \
            iso_test_checks_failed++;                                          \
            printf("  FAIL %s:%d  ISO_CHECK_MEM_EQ(%s, %s): byte %lu: "        \
                   "0x%02x != 0x%02x\n",                                       \
                   __FILE__, __LINE__, #a, #b, (unsigned long)iso_i_,          \
                   iso_a_[iso_i_], iso_b_[iso_i_]);                            \
        }                                                                      \
    } while (0)

/* ISO_CHECK_EQ_DBL(a, b, eps) — floating-point equality within a tolerance.
 * The absolute difference is computed inline (no <math.h> / no -lm needed). */
#define ISO_CHECK_EQ_DBL(a, b, eps)                                            \
    do {                                                                       \
        double iso_a_ = (double)(a);                                           \
        double iso_b_ = (double)(b);                                           \
        double iso_d_ = iso_a_ - iso_b_;                                       \
        if (iso_d_ < 0) {                                                      \
            iso_d_ = -iso_d_;                                                   \
        }                                                                      \
        iso_test_checks_run++;                                                 \
        if (iso_d_ > (double)(eps)) {                                          \
            iso_test_checks_failed++;                                          \
            printf("  FAIL %s:%d  ISO_CHECK_EQ_DBL(%s, %s): %g != %g "         \
                   "(|d|=%g > %g)\n",                                          \
                   __FILE__, __LINE__, #a, #b, iso_a_, iso_b_, iso_d_,         \
                   (double)(eps));                                            \
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
