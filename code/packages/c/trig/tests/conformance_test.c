/*
 * conformance_test.c -- bit-exact conformance of `trig` against the shared
 * TRIG-REF-1 vectors (PHY00a sections 3 and 6).
 * ===========================================================================
 *
 * The vectors live in code/specs/fixtures/trig-reference-v1 (four .tsv files).  Every row
 * is five tab-separated fields:
 *
 *     op <TAB> x <TAB> y <TAB> cr <TAB> flags
 *
 * `x`, `y` and `cr` are IEEE-754 binary64 BIT PATTERNS written as 16 lowercase
 * hex digits -- never decimals, because a decimal hides the last digits of a
 * double and one value must have exactly one spelling.  `atan2` has two inputs
 * and writes `x` as "<y>,<x>".
 *
 * What this test does, per PHY00a 6.5:
 *   (a) decodes each input by a BIT CAST (memcpy from a uint64_t), never by
 *       parsing a decimal;
 *   (b) calls the PUBLIC function (trig_sin, trig_cos, ...);
 *   (c) re-encodes the result and compares it to `y` as an INTEGER.  The one
 *       relaxation is NaN: `7ff8000000000000` matches ANY NaN (VER04p `same`);
 *       `error` means "sqrt of a negative: TRIG_ERR_DOMAIN";
 *   (d) checks accuracy against `cr`, the correctly rounded value: sin and cos
 *       within 3 steps, radians and degrees within 1.  `tan` is a NAMED
 *       no-claim op (PHY00a 3.8: it is sin/cos plus a division and a pinned
 *       pole sentinel).  Any OTHER op that has a `cr` value but no limit is a
 *       test failure, so a future op cannot be skipped silently;
 *   (e) asserts the bits of the decimal literals the library writes in code
 *       (1e-15, 1e308, 1e-17, 1e-300), because "the literal parses to the
 *       nearest double" is a property of the compiler, not of the package.
 *
 * C needs no bit assert for the TRIG-REF-1 constants themselves: they are
 * written as hex float literals (`0x1.45f306dc9c883p-1`), which ARE the bits.
 * The vectors, which include directed witnesses for every coefficient, would
 * catch a mistyped one anyway.
 *
 * Pure ISO C17: stdio for reading the files, string.h memcpy for the bit cast.
 * The test runs from the package directory (tools/run.sh does `cd` there), so
 * the corpus is found by walking up to code/specs.
 */
#include "iso_test.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "trig.h"

/* ---------------------------------------------------------------------------
 *  Bit casts
 * ------------------------------------------------------------------------- */

static double bits_to_double(uint64_t b) {
    double d;
    memcpy(&d, &b, sizeof d);
    return d;
}

static uint64_t double_to_bits(double d) {
    uint64_t b;
    memcpy(&b, &d, sizeof b);
    return b;
}

/* Parse exactly 16 lowercase hex digits.  Returns 0 on a malformed field. */
static int parse_hex16(const char *s, uint64_t *out) {
    uint64_t v = 0;
    int i;
    for (i = 0; i < 16; i++) {
        char c = s[i];
        int d;
        if (c >= '0' && c <= '9') {
            d = c - '0';
        } else if (c >= 'a' && c <= 'f') {
            d = c - 'a' + 10;
        } else {
            return 0;
        }
        v = (v << 4) | (uint64_t)d;
    }
    *out = v;
    return 1;
}

static int is_nan(double d) { return d != d; }

/* Order doubles so that adjacent representable values differ by 1: map the
 * sign-magnitude bit pattern onto a signed integer line.  The distance between
 * two results in "steps" (units in the last place) is then |a - b|. */
static long long ordered(uint64_t bits) {
    long long mag = (long long)(bits & 0x7FFFFFFFFFFFFFFFULL);
    return (bits >> 63) ? -mag : mag;
}

/* ---------------------------------------------------------------------------
 *  Operations
 * ------------------------------------------------------------------------- */

enum { OP_SIN, OP_COS, OP_TAN, OP_RADIANS, OP_DEGREES, OP_SQRT, OP_ATAN,
       OP_ATAN2, OP_COUNT };

static const char *const OP_NAMES[OP_COUNT] = {
    "sin", "cos", "tan", "radians", "degrees", "sqrt", "atan", "atan2"};

static int op_index(const char *s, size_t len) {
    int i;
    for (i = 0; i < OP_COUNT; i++) {
        if (strlen(OP_NAMES[i]) == len && memcmp(OP_NAMES[i], s, len) == 0) {
            return i;
        }
    }
    return -1;
}

/* Accuracy limit in steps for the ops that make an accuracy claim, or -1 for
 * an op that makes none.  `tan` and Part II (sqrt/atan/atan2) are the NAMED
 * no-claim ops; the fail-closed check below refuses anything else. */
static int accuracy_limit(int op) {
    switch (op) {
    case OP_SIN:
    case OP_COS:
        return 3;
    case OP_RADIANS:
    case OP_DEGREES:
        return 1;
    default:
        return -1;
    }
}

static int is_named_no_claim(int op) {
    return op == OP_TAN || op == OP_SQRT || op == OP_ATAN || op == OP_ATAN2;
}

/* Call the public function.  Returns 0 normally, 1 when sqrt reports
 * TRIG_ERR_DOMAIN (the C port's idiomatic error). */
static int call_op(int op, double a, double b, double *out) {
    switch (op) {
    case OP_SIN: *out = trig_sin(a); return 0;
    case OP_COS: *out = trig_cos(a); return 0;
    case OP_TAN: *out = trig_tan(a); return 0;
    case OP_RADIANS: *out = trig_radians(a); return 0;
    case OP_DEGREES: *out = trig_degrees(a); return 0;
    case OP_SQRT: return trig_sqrt(a, out) == TRIG_ERR_DOMAIN ? 1 : 0;
    case OP_ATAN: *out = trig_atan(a); return 0;
    default: *out = trig_atan2(a, b); return 0; /* a = y, b = x */
    }
}

/* ---------------------------------------------------------------------------
 *  Locating the corpus
 * ------------------------------------------------------------------------- */

static const char *const GROUPS[] = {"edge", "sweep-binade", "sweep-cw",
                                     "sweep-prng"};
#define N_GROUPS 4

/* The fixture directory relative to a few plausible working directories: the
 * package dir (what tools/run.sh uses), the repo root, and code/packages. */
static const char *const FIXTURE_DIRS[] = {
    "../../../specs/fixtures/trig-reference-v1",
    "code/specs/fixtures/trig-reference-v1",
    "../../specs/fixtures/trig-reference-v1",
};

static FILE *open_group(const char *group, const char **dir_used) {
    char path[512];
    size_t i;
    for (i = 0; i < sizeof FIXTURE_DIRS / sizeof FIXTURE_DIRS[0]; i++) {
        FILE *f;
        sprintf(path, "%s/%s.tsv", FIXTURE_DIRS[i], group);
        f = fopen(path, "rb");
        if (f) {
            *dir_used = FIXTURE_DIRS[i];
            return f;
        }
    }
    return NULL;
}

/* ---------------------------------------------------------------------------
 *  The vectors
 * ------------------------------------------------------------------------- */

static long rows_seen[OP_COUNT];
static long rows_failed[OP_COUNT];
static long worst_steps[OP_COUNT];
static long accuracy_failures[OP_COUNT];

static void report_row(const char *group, long lineno, int op, const char *why,
                       const char *line) {
    /* Print the first few failures per op only; the totals are the report. */
    if (rows_failed[op] <= 3) {
        printf("  FAIL %s:%ld %s: %s\n    %s\n", group, lineno, OP_NAMES[op],
               why, line);
    }
}

static int check_row(const char *group, long lineno, char *line) {
    char *f[5];
    char *p = line;
    int i, op;
    uint64_t xa = 0, xb = 0, y = 0, cr = 0;
    int y_is_error, has_cr;
    double a, b, got = 0.0;
    int raised;

    /* Split on tabs into exactly five fields. */
    for (i = 0; i < 5; i++) {
        f[i] = p;
        p = strchr(p, '\t');
        if (i < 4) {
            if (!p) {
                printf("  FAIL %s:%ld malformed row\n", group, lineno);
                return 0;
            }
            *p++ = '\0';
        } else if (p) {
            printf("  FAIL %s:%ld too many fields\n", group, lineno);
            return 0;
        }
    }
    op = op_index(f[0], strlen(f[0]));
    if (op < 0) {
        printf("  FAIL %s:%ld unknown op %s\n", group, lineno, f[0]);
        return 0;
    }
    rows_seen[op]++;

    if (op == OP_ATAN2) {
        /* "<y>,<x>": 16 + 1 + 16 characters. */
        if (strlen(f[1]) != 33 || f[1][16] != ',' ||
            !parse_hex16(f[1], &xa) || !parse_hex16(f[1] + 17, &xb)) {
            printf("  FAIL %s:%ld malformed atan2 input\n", group, lineno);
            return 0;
        }
    } else if (strlen(f[1]) != 16 || !parse_hex16(f[1], &xa)) {
        printf("  FAIL %s:%ld malformed input\n", group, lineno);
        return 0;
    }
    y_is_error = strcmp(f[2], "error") == 0;
    if (!y_is_error && (strlen(f[2]) != 16 || !parse_hex16(f[2], &y))) {
        printf("  FAIL %s:%ld malformed result\n", group, lineno);
        return 0;
    }
    has_cr = strcmp(f[3], "-") != 0;
    if (has_cr && (strlen(f[3]) != 16 || !parse_hex16(f[3], &cr))) {
        printf("  FAIL %s:%ld malformed cr\n", group, lineno);
        return 0;
    }

    a = bits_to_double(xa);
    b = bits_to_double(xb);
    raised = call_op(op, a, b, &got);

    /* (c) bit-for-bit, NaN as a class, `error` as the port's own error. */
    if (y_is_error) {
        if (!raised) {
            rows_failed[op]++;
            report_row(group, lineno, op, "expected TRIG_ERR_DOMAIN", f[1]);
        }
        return 1;
    }
    if (raised) {
        rows_failed[op]++;
        report_row(group, lineno, op, "unexpected TRIG_ERR_DOMAIN", f[1]);
        return 1;
    }
    if (y == 0x7FF8000000000000ULL) {
        if (!is_nan(got)) {
            char why[64];
            sprintf(why, "expected NaN, got %016llx",
                    (unsigned long long)double_to_bits(got));
            rows_failed[op]++;
            report_row(group, lineno, op, why, f[1]);
        }
    } else if (double_to_bits(got) != y) {
        char why[80];
        sprintf(why, "expected %s, got %016llx", f[2],
                (unsigned long long)double_to_bits(got));
        rows_failed[op]++;
        report_row(group, lineno, op, why, f[1]);
    }

    /* (d) accuracy against the correctly rounded column. */
    if (has_cr) {
        int limit = accuracy_limit(op);
        if (limit < 0) {
            /* Fail closed: only a NAMED no-claim op may carry a cr value
             * without a limit. */
            if (!is_named_no_claim(op)) {
                accuracy_failures[op]++;
                printf("  FAIL %s:%ld op %s has a cr value but no limit\n",
                       group, lineno, OP_NAMES[op]);
            }
        } else if (is_nan(got)) {
            accuracy_failures[op]++;
        } else {
            long long d = ordered(double_to_bits(got)) - ordered(cr);
            long steps = (long)(d < 0 ? -d : d);
            if (steps > worst_steps[op]) {
                worst_steps[op] = steps;
            }
            if (steps > limit) {
                accuracy_failures[op]++;
            }
        }
    }
    return 1;
}

static void run_vectors(void) {
    int g, op;
    long total = 0;
    static char line[4096];

    for (g = 0; g < N_GROUPS; g++) {
        const char *dir = NULL;
        long lineno = 0, rows = 0;
        FILE *fp = open_group(GROUPS[g], &dir);
        ISO_CHECK_MSG(fp != NULL, "fixture group not found");
        if (!fp) {
            continue;
        }
        while (fgets(line, (int)sizeof line, fp)) {
            size_t n = strlen(line);
            lineno++;
            /* Rows end in exactly "\n" (the fixture is -text). */
            if (n == 0 || line[n - 1] != '\n') {
                ISO_CHECK_MSG(0, "row without a final newline");
                continue;
            }
            line[n - 1] = '\0';
            if (check_row(GROUPS[g], lineno, line)) {
                rows++;
            }
        }
        fclose(fp);
        ISO_CHECK_MSG(rows > 0, "fixture group is empty");
        total += rows;
    }
    printf("trig conformance: %ld rows\n", total);

    for (op = 0; op < OP_COUNT; op++) {
        printf("  %-8s rows=%-6ld failing=%-6ld accuracy-failing=%ld "
               "worst-steps=%ld\n",
               OP_NAMES[op], rows_seen[op], rows_failed[op],
               accuracy_failures[op], worst_steps[op]);
        ISO_CHECK_MSG(rows_seen[op] > 0, "op has no vectors");
        ISO_CHECK_MSG(rows_failed[op] == 0, "op has non-matching vectors");
        ISO_CHECK_MSG(accuracy_failures[op] == 0,
                      "op exceeds its accuracy limit against cr");
    }
}

/* ---------------------------------------------------------------------------
 *  Named contract points (PHY00a 3.3)
 * ------------------------------------------------------------------------- */

static void run_contract(void) {
    volatile double zero = 0.0;
    volatile double huge = 1.7976931348623157e308;
    double inf = huge * 10.0;
    double nan = zero / zero;
    double neg_zero = -zero;
    double just_over = 0x1p29 * (1.0 + 0x1p-52);

    /* The decimal literals the library writes in code, by bits. */
    ISO_CHECK(double_to_bits(1e-15) == 0x3CD203AF9EE75616ULL);
    ISO_CHECK(double_to_bits(1.0e308) == 0x7FE1CCF385EBC8A0ULL);
    ISO_CHECK(double_to_bits(1e-17) == 0x3C670EF54646D497ULL);
    ISO_CHECK(double_to_bits(1e-300) == 0x01A56E1FC2F8F359ULL);
    ISO_CHECK(double_to_bits(TRIG_PI) == 0x400921FB54442D18ULL);

    /* sin(-0) is -0; sin(+0) is +0; cos of either zero is 1. */
    ISO_CHECK(double_to_bits(trig_sin(neg_zero)) == 0x8000000000000000ULL);
    ISO_CHECK(double_to_bits(trig_sin(zero)) == 0);
    ISO_CHECK(trig_cos(neg_zero) == 1.0);
    ISO_CHECK(trig_cos(zero) == 1.0);

    /* Below TINY, sin returns its argument exactly and cos returns 1. */
    {
        double tiny = bits_to_double(0x3E3FFFFFFFFFFFFFULL);
        double sub = bits_to_double(1ULL);
        ISO_CHECK(trig_sin(tiny) == tiny);
        ISO_CHECK(trig_sin(sub) == sub);
        ISO_CHECK(trig_cos(tiny) == 1.0);
    }

    /* Outside the domain (NaN, infinities, |x| > 2^29): NaN. */
    ISO_CHECK(is_nan(trig_sin(nan)) && is_nan(trig_cos(nan)) &&
              is_nan(trig_tan(nan)));
    ISO_CHECK(is_nan(trig_sin(inf)) && is_nan(trig_cos(inf)) &&
              is_nan(trig_tan(inf)));
    ISO_CHECK(is_nan(trig_sin(-inf)) && is_nan(trig_cos(-inf)) &&
              is_nan(trig_tan(-inf)));
    ISO_CHECK(is_nan(trig_sin(1e20)) && is_nan(trig_cos(1e20)) &&
              is_nan(trig_tan(1e20)));
    ISO_CHECK(is_nan(trig_sin(just_over)) && is_nan(trig_cos(just_over)) &&
              is_nan(trig_tan(just_over)));
    ISO_CHECK(is_nan(trig_sin(-just_over)));
    ISO_CHECK(is_nan(trig_sin(huge)));

    /* The last in-domain value is finite and exact. */
    ISO_CHECK(double_to_bits(trig_sin(0x1p29)) == 0x3FD4E67C0E2622DFULL);
    ISO_CHECK(double_to_bits(trig_cos(0x1p29)) == 0xBFEE3EDD2DFEEF90ULL);

    /* radians and degrees are single multiplies by pinned constants. */
    ISO_CHECK(trig_radians(180.0) == TRIG_PI);
    ISO_CHECK(trig_degrees(TRIG_PI) == 180.0);
    ISO_CHECK(double_to_bits(trig_radians(neg_zero)) == 0x8000000000000000ULL);
    ISO_CHECK(double_to_bits(trig_degrees(neg_zero)) == 0x8000000000000000ULL);
    ISO_CHECK(trig_degrees(1.7e308) == inf);
    ISO_CHECK(is_nan(trig_radians(nan)));

    /* tan keeps its pinned pole sentinel. */
    ISO_CHECK(trig_tan(TRIG_PI / 2.0) == 1e308);
    ISO_CHECK(trig_tan(-TRIG_PI / 2.0) == -1e308);

    /* atan2 of a NaN is NaN (added by TRIG-REF-1), in every position. */
    ISO_CHECK(is_nan(trig_atan2(nan, 1.0)));
    ISO_CHECK(is_nan(trig_atan2(1.0, nan)));
    ISO_CHECK(is_nan(trig_atan2(-1.0, nan)));
    ISO_CHECK(is_nan(trig_atan2(0.0, nan)));
    ISO_CHECK(is_nan(trig_atan2(nan, nan)));
    ISO_CHECK(is_nan(trig_atan2(nan, -1.0)));
    ISO_CHECK(is_nan(trig_atan2(nan, 0.0)));

    /* The pinned atan2 conventions, which are not IEEE 754's. */
    ISO_CHECK(trig_atan2(neg_zero, -1.0) == TRIG_PI);
    ISO_CHECK(trig_atan2(0.0, 0.0) == 0.0);
    ISO_CHECK(is_nan(trig_atan2(inf, inf)));
    ISO_CHECK(trig_atan2(1e300, 1e-300) == TRIG_PI / 2.0);

    /* atan keeps the sign of zero. */
    ISO_CHECK(double_to_bits(trig_atan(neg_zero)) == 0x8000000000000000ULL);

    /* Quadrant arithmetic for negative k: "& 3" is the floor modulus, so
     * k = -1 and k = -5 land in quadrant 3. */
    ISO_CHECK(trig_sin(-TRIG_PI / 2.0) == -trig_sin(TRIG_PI / 2.0));
}

int main(void) {
    run_contract();
    run_vectors();
    return ISO_TEST_RESULT();
}
