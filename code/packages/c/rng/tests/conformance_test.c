/*
 * conformance_test.c — the shared rng corpus (spec rng.md, "Conformance
 * corpus"), read from its flat form code/specs/fixtures/rng-v1/cases.txt.
 *
 * The corpus pins exact outputs every language's rng package must reproduce;
 * its results come from the independent reference
 * code/scripts/rng_conformance.py, which also generates the flat form so that
 * a lane with no JSON reader can use it. Each line is whitespace-separated:
 *
 *     case <id> <generator> <seed>
 *     next_u32 <result>
 *     next_u64 <result>
 *     next_float <16 hex digits of the IEEE-754 bits>
 *     next_int_in_range <min> <max> <result | error>
 *     end
 *
 * The test runs from the package directory (tools/run.sh cds there).
 */
/* fopen and fscanf are ISO C17; MSVC's _s alternatives are not, and its
 * deprecation warning would be an error under /W4 /WX. */
#ifdef _MSC_VER
#define _CRT_SECURE_NO_WARNINGS
#endif
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

#include "rng.h"

#define CORPUS "../../../specs/fixtures/rng-v1/cases.txt"
#define TOKEN 128

/* One generator of any of the three kinds, so the driver can be generic. */
typedef struct {
    int kind; /* 0 lcg, 1 xorshift64, 2 pcg32 */
    rng_lcg lcg;
    rng_xorshift64 xs;
    rng_pcg32 pcg;
} any_rng;

static int start(any_rng *g, const char *name, uint64_t seed) {
    if (strcmp(name, "lcg") == 0) {
        g->kind = 0;
        rng_lcg_init(&g->lcg, seed);
    } else if (strcmp(name, "xorshift64") == 0) {
        g->kind = 1;
        rng_xorshift64_init(&g->xs, seed);
    } else if (strcmp(name, "pcg32") == 0) {
        g->kind = 2;
        rng_pcg32_init(&g->pcg, seed);
    } else {
        return -1;
    }
    return 0;
}

static uint32_t u32(any_rng *g) {
    return g->kind == 0   ? rng_lcg_next_u32(&g->lcg)
           : g->kind == 1 ? rng_xorshift64_next_u32(&g->xs)
                          : rng_pcg32_next_u32(&g->pcg);
}

static uint64_t u64(any_rng *g) {
    return g->kind == 0   ? rng_lcg_next_u64(&g->lcg)
           : g->kind == 1 ? rng_xorshift64_next_u64(&g->xs)
                          : rng_pcg32_next_u64(&g->pcg);
}

static double f64(any_rng *g) {
    return g->kind == 0   ? rng_lcg_next_float(&g->lcg)
           : g->kind == 1 ? rng_xorshift64_next_float(&g->xs)
                          : rng_pcg32_next_float(&g->pcg);
}

static int in_range(any_rng *g, int64_t min, int64_t max, int64_t *out) {
    return g->kind == 0   ? rng_lcg_try_int_in_range(&g->lcg, min, max, out)
           : g->kind == 1 ? rng_xorshift64_try_int_in_range(&g->xs, min, max, out)
                          : rng_pcg32_try_int_in_range(&g->pcg, min, max, out);
}

/* Read one whitespace-separated token; 0 at end of file. */
static int token(FILE *f, char *buf) {
    return fscanf(f, "%127s", buf) == 1;
}

/* Skip the '#' comment lines at the top of the file. */
static void skip_comments(FILE *f) {
    int c;
    while ((c = fgetc(f)) == '#') {
        while ((c = fgetc(f)) != '\n' && c != EOF) {
        }
    }
    if (c != EOF) {
        ungetc(c, f);
    }
}

int main(void) {
    char op[TOKEN], id[TOKEN], name[TOKEN], a[TOKEN], b[TOKEN], c[TOKEN];
    int cases = 0, calls = 0;
    any_rng g;
    FILE *f = fopen(CORPUS, "r");
    ISO_CHECK_MSG(f != NULL, "open " CORPUS);
    if (f == NULL) {
        return ISO_TEST_RESULT();
    }
    skip_comments(f);
    while (token(f, op)) {
        ISO_CHECK_STR_EQ(op, "case");
        if (!token(f, id) || !token(f, name) || !token(f, a) ||
            start(&g, name, strtoull(a, NULL, 10)) != 0) {
            ISO_CHECK_MSG(0, "malformed case line");
            break;
        }
        cases++;
        while (token(f, op) && strcmp(op, "end") != 0) {
            calls++;
            if (strcmp(op, "next_u32") == 0 && token(f, a)) {
                ISO_CHECK_EQ_UINT(u32(&g), strtoul(a, NULL, 10));
            } else if (strcmp(op, "next_u64") == 0 && token(f, a)) {
                ISO_CHECK_MSG(u64(&g) == strtoull(a, NULL, 10), id);
            } else if (strcmp(op, "next_float") == 0 && token(f, a)) {
                double d = f64(&g);
                uint64_t bits;
                memcpy(&bits, &d, sizeof bits);
                ISO_CHECK_MSG(bits == strtoull(a, NULL, 16), id);
            } else if (strcmp(op, "next_int_in_range") == 0 && token(f, a) &&
                       token(f, b) && token(f, c)) {
                int64_t value = 0;
                int status = in_range(&g, strtoll(a, NULL, 10), strtoll(b, NULL, 10),
                                      &value);
                if (strcmp(c, "error") == 0) {
                    ISO_CHECK_MSG(status == -1, id);
                } else {
                    ISO_CHECK_MSG(status == 0 && value == strtoll(c, NULL, 10), id);
                }
            } else {
                ISO_CHECK_MSG(0, "malformed call line");
                break;
            }
        }
    }
    fclose(f);
    /* Guard against a truncated or empty file passing vacuously. */
    ISO_CHECK_MSG(cases >= 27, "the corpus has at least 27 cases");
    ISO_CHECK_MSG(calls >= 150, "the corpus has at least 150 calls");
    return ISO_TEST_RESULT();
}
