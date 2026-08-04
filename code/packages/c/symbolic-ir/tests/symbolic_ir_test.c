/*
 * Tests for the C symbolic-ir library, using the header-only iso_test.h harness
 * (pure ISO). Vectors mirror the Rust crate's own tests.
 */
#include "iso_test.h"

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "symbolic_ir.h"

/* Build a Rational and assert it collapsed/reduced to the expected node. */
static void expect_rational(int64_t nn, int64_t dd, SirKind kind, int64_t en,
                            int64_t ed) {
    SirNode *r = NULL;
    ISO_CHECK(sir_rational(nn, dd, &r) == SIR_OK);
    ISO_CHECK(sir_kind(r) == kind);
    if (kind == SIR_INTEGER) {
        ISO_CHECK(sir_integer_value(r) == en);
    } else {
        int64_t gn, gd;
        sir_rational_parts(r, &gn, &gd);
        ISO_CHECK(gn == en && gd == ed);
    }
    sir_free(r);
}

/* Assert sir_to_string(n) == expected, then free n. */
static void expect_display(SirNode *n, const char *expected) {
    char *s = sir_to_string(n);
    ISO_CHECK(s != NULL);
    if (s) {
        ISO_CHECK_STR_EQ(s, expected);
    }
    free(s);
    sir_free(n);
}

int main(void) {
    ISO_CHECK_STR_EQ(SIR_VERSION, "0.2.0");

    /* ── rational: reduce, collapse, sign, zero numerator ────────────────── */
    expect_rational(2, 4, SIR_RATIONAL, 1, 2);
    expect_rational(6, 3, SIR_INTEGER, 2, 0);   /* collapses */
    expect_rational(10, 5, SIR_INTEGER, 2, 0);
    expect_rational(1, -2, SIR_RATIONAL, -1, 2); /* sign -> numerator */
    expect_rational(-3, -4, SIR_RATIONAL, 3, 4);
    expect_rational(0, 5, SIR_INTEGER, 0, 0);   /* 0/anything -> 0 */

    /* zero denominator is an error (the Rust panic). */
    {
        SirNode *r = NULL;
        ISO_CHECK(sir_rational(1, 0, &r) == SIR_ERR_ZERO_DENOM);
    }

    /* ── standard heads are the expected strings ─────────────────────────── */
    ISO_CHECK_STR_EQ(SIR_ADD, "Add");
    ISO_CHECK_STR_EQ(SIR_MUL, "Mul");
    ISO_CHECK_STR_EQ(SIR_POW, "Pow");
    ISO_CHECK_STR_EQ(SIR_SIN, "Sin");
    ISO_CHECK_STR_EQ(SIR_DEFINE, "Define");
    ISO_CHECK_STR_EQ(SIR_COTH, "Coth");
    ISO_CHECK_STR_EQ(SIR_SECH, "Sech");
    ISO_CHECK_STR_EQ(SIR_CSCH, "Csch");

    /* ── equality ────────────────────────────────────────────────────────── */
    {
        SirNode *x1 = sir_sym("x"), *x2 = sir_sym("x"), *xu = sir_sym("X");
        ISO_CHECK(sir_equals(x1, x2));      /* case-sensitive: equal */
        ISO_CHECK(!sir_equals(x1, xu));     /* x != X */
        sir_free(x1);
        sir_free(x2);
        sir_free(xu);

        SirNode *i1 = sir_int(42), *i2 = sir_int(42), *i3 = sir_int(1);
        ISO_CHECK(sir_equals(i1, i2));
        ISO_CHECK(!sir_equals(i1, i3));

        SirNode *f1 = sir_flt(1.0), *f2 = sir_flt(1.0), *f3 = sir_flt(2.0);
        ISO_CHECK(sir_equals(f1, f2));
        ISO_CHECK(!sir_equals(f1, f3));

        /* Different variants are never equal. */
        ISO_CHECK(!sir_equals(i3, f1));      /* Integer(1) != Float(1.0) */
        SirNode *s1 = sir_sym("1");
        ISO_CHECK(!sir_equals(s1, i3));      /* Symbol("1") != Integer(1) */

        sir_free(i1);
        sir_free(i2);
        sir_free(i3);
        sir_free(f1);
        sir_free(f2);
        sir_free(f3);
        sir_free(s1);
    }

    /* NaN with identical bits compares equal (bit-pattern equality). */
    {
        volatile double zero = 0.0;
        double nan = zero / zero;
        SirNode *n1 = sir_flt(nan), *n2 = sir_flt(nan);
        ISO_CHECK(sir_equals(n1, n2));
        sir_free(n1);
        sir_free(n2);
    }

    /* ── hash: equal nodes hash equal ────────────────────────────────────── */
    {
        SirNode *a = sir_sym("x"), *b = sir_sym("x");
        ISO_CHECK(sir_hash(a) == sir_hash(b));
        sir_free(a);
        sir_free(b);

        SirNode *i1 = sir_int(7), *i2 = sir_int(7);
        ISO_CHECK(sir_hash(i1) == sir_hash(i2));
        /* Different variant with the "same" payload hashes differently. */
        SirNode *sy = sir_sym("7");
        ISO_CHECK(sir_hash(i1) != sir_hash(sy));
        sir_free(i1);
        sir_free(i2);
        sir_free(sy);

        SirNode *r1 = NULL, *r2 = NULL;
        sir_rational(1, 2, &r1);
        sir_rational(1, 2, &r2);
        ISO_CHECK(sir_hash(r1) == sir_hash(r2));
        ISO_CHECK(sir_equals(r1, r2));
        sir_free(r1);
        sir_free(r2);
    }

    /* ── display ─────────────────────────────────────────────────────────── */
    expect_display(sir_sym("x"), "x");
    expect_display(sir_int(-7), "-7");
    {
        SirNode *r = NULL;
        sir_rational(1, 3, &r);
        expect_display(r, "1/3");
    }
    expect_display(sir_flt(1.5), "1.5");
    expect_display(sir_flt(3.0), "3.0"); /* integer-valued float keeps ".0" */
    expect_display(sir_str("hello"), "\"hello\"");

    /* Apply: Add(x, 1). */
    {
        SirNode *args[2] = {sir_sym("x"), sir_int(1)};
        SirNode *e = sir_apply(sir_sym(SIR_ADD), args, 2);
        expect_display(e, "Add(x, 1)");
    }
    /* Nested Apply: Pow(x, 2). */
    {
        SirNode *args[2] = {sir_sym("x"), sir_int(2)};
        SirNode *e = sir_apply(sir_sym(SIR_POW), args, 2);
        expect_display(e, "Pow(x, 2)");
    }
    /* Doubly-nested: Add(Pow(x, 2), 1) exercises recursion + equality. */
    {
        SirNode *pow_args[2] = {sir_sym("x"), sir_int(2)};
        SirNode *pow = sir_apply(sir_sym(SIR_POW), pow_args, 2);
        SirNode *add_args[2] = {pow, sir_int(1)};
        SirNode *e = sir_apply(sir_sym(SIR_ADD), add_args, 2);

        /* Build an identical tree and check structural equality. */
        SirNode *pow_args2[2] = {sir_sym("x"), sir_int(2)};
        SirNode *pow2 = sir_apply(sir_sym(SIR_POW), pow_args2, 2);
        SirNode *add_args2[2] = {pow2, sir_int(1)};
        SirNode *e2 = sir_apply(sir_sym(SIR_ADD), add_args2, 2);

        ISO_CHECK(sir_equals(e, e2));
        ISO_CHECK(sir_hash(e) == sir_hash(e2));
        ISO_CHECK_EQ_UINT(sir_apply_arity(e), 2u);
        ISO_CHECK(sir_kind(sir_apply_arg(e, 0)) == SIR_APPLY);

        char *s = sir_to_string(e);
        ISO_CHECK_STR_EQ(s, "Add(Pow(x, 2), 1)");
        free(s);
        sir_free(e);
        sir_free(e2);
    }

    return ISO_TEST_RESULT();
}
