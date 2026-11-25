/* Tests for the C logic-gates package, using the iso_test.h harness.
 *
 * Four families of test:
 *   1. literal truth tables, written out by hand (no formula to share a bug with),
 *   2. each NAND-built gate agrees with its direct gate,
 *   3. exhaustive rejection of every non-bit input we can reasonably enumerate,
 *   4. no valid input ever yields LG_INVALID.
 */
#include "iso_test.h"

#include <limits.h>

#include "logic_gates.h"

/* One row of a two-input truth table: a, b, expected output. */
struct row2 {
    int a, b, want;
};

typedef int (*gate2)(int, int);
typedef int (*gate1)(int);

/* Check a whole two-input table (exactly 4 rows) against one gate. */
static void check_table2(gate2 g, const struct row2 t[4]) {
    int i;
    for (i = 0; i < 4; i++) {
        ISO_CHECK_EQ_INT(g(t[i].a, t[i].b), t[i].want);
    }
}

/* The six direct two-input gates, rows in order 00, 01, 10, 11. */
static const struct row2 T_AND[4]  = {{0,0,0},{0,1,0},{1,0,0},{1,1,1}};
static const struct row2 T_OR[4]   = {{0,0,0},{0,1,1},{1,0,1},{1,1,1}};
static const struct row2 T_XOR[4]  = {{0,0,0},{0,1,1},{1,0,1},{1,1,0}};
static const struct row2 T_NAND[4] = {{0,0,1},{0,1,1},{1,0,1},{1,1,0}};
static const struct row2 T_NOR[4]  = {{0,0,1},{0,1,0},{1,0,0},{1,1,0}};
static const struct row2 T_XNOR[4] = {{0,0,1},{0,1,0},{1,0,0},{1,1,1}};

/* The invalid values we probe: [-1024, 1024] minus {0,1}, plus the extremes.
 * probe(i) is the i-th such value for i in [0, PROBES). */
#define PROBES (2049 - 2 + 4)
static int probe(int i) {
    int v;
    if (i >= 2047) {
        switch (i - 2047) {
        case 0:  return INT_MIN;
        case 1:  return INT_MIN + 1;
        case 2:  return INT_MAX - 1;
        default: return INT_MAX;
        }
    }
    v = -1024 + i; /* i in [0, 2046] */
    if (v >= 0) {
        v += 2; /* skip 0 and 1 */
    }
    return v;
}

static int is_bit(int v) { return v == 0 || v == 1; }

int main(void) {
    int a, b, i, j, k;
    int probes_ok = 1;
    gate1 g1[2];
    gate2 g2[9];

    /* 1. literal truth tables */
    ISO_CHECK_EQ_INT(lg_not(0), 1);
    ISO_CHECK_EQ_INT(lg_not(1), 0);
    ISO_CHECK_EQ_INT(lg_nand_not(0), 1);
    ISO_CHECK_EQ_INT(lg_nand_not(1), 0);
    check_table2(lg_and, T_AND);
    check_table2(lg_or, T_OR);
    check_table2(lg_xor, T_XOR);
    check_table2(lg_nand, T_NAND);
    check_table2(lg_nor, T_NOR);
    check_table2(lg_xnor, T_XNOR);
    check_table2(lg_nand_and, T_AND);
    check_table2(lg_nand_or, T_OR);
    check_table2(lg_nand_xor, T_XOR);

    /* 2. NAND-built == direct, on all valid inputs */
    for (a = 0; a <= 1; a++) {
        ISO_CHECK_EQ_INT(lg_nand_not(a), lg_not(a));
        for (b = 0; b <= 1; b++) {
            ISO_CHECK_EQ_INT(lg_nand_and(a, b), lg_and(a, b));
            ISO_CHECK_EQ_INT(lg_nand_or(a, b), lg_or(a, b));
            ISO_CHECK_EQ_INT(lg_nand_xor(a, b), lg_xor(a, b));
        }
    }

    /* 3. exhaustive rejection. The probe list must itself contain no bits. */
    for (i = 0; i < PROBES; i++) {
        if (is_bit(probe(i))) {
            probes_ok = 0;
        }
    }
    ISO_CHECK_MSG(probes_ok, "probe list must exclude 0 and 1");

    g1[0] = lg_not;      g1[1] = lg_nand_not;
    g2[0] = lg_and;      g2[1] = lg_or;       g2[2] = lg_xor;
    g2[3] = lg_nand;     g2[4] = lg_nor;      g2[5] = lg_xnor;
    g2[6] = lg_nand_and; g2[7] = lg_nand_or;  g2[8] = lg_nand_xor;

    for (i = 0; i < PROBES; i++) {
        int bad = probe(i);
        for (k = 0; k < 2; k++) {
            ISO_CHECK(g1[k](bad) == LG_INVALID);
        }
        for (k = 0; k < 9; k++) {
            /* bad in either position, other argument valid ... */
            ISO_CHECK(g2[k](bad, 0) == LG_INVALID);
            ISO_CHECK(g2[k](bad, 1) == LG_INVALID);
            ISO_CHECK(g2[k](0, bad) == LG_INVALID);
            ISO_CHECK(g2[k](1, bad) == LG_INVALID);
            /* ... and with the other argument also invalid. */
            for (j = 0; j < PROBES; j += 61) {
                ISO_CHECK(g2[k](bad, probe(j)) == LG_INVALID);
                ISO_CHECK(g2[k](probe(j), bad) == LG_INVALID);
            }
            ISO_CHECK(g2[k](bad, bad) == LG_INVALID);
        }
    }

    /* 4. no valid input returns LG_INVALID; every result is a bit. */
    for (a = 0; a <= 1; a++) {
        for (k = 0; k < 2; k++) {
            ISO_CHECK(is_bit(g1[k](a)));
        }
        for (b = 0; b <= 1; b++) {
            for (k = 0; k < 9; k++) {
                ISO_CHECK(is_bit(g2[k](a, b)));
                ISO_CHECK(g2[k](a, b) != LG_INVALID);
            }
        }
    }

    ISO_CHECK_EQ_INT(LG_INVALID, -1);
    return ISO_TEST_RESULT();
}
