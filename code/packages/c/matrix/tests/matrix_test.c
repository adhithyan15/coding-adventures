/*
 * Tests for the C matrix library, using the header-only iso_test.h harness
 * (pure ISO). The expected values are taken directly from the Rust crate's
 * own unit tests, so this suite verifies the port is faithful.
 */
#include "iso_test.h"

#include "matrix.h"

/* Convenience: check that element (i,j) of m equals v within an absolute eps. */
static int at_eq(const Mat *m, size_t i, size_t j, double v, double eps) {
    double got;
    if (mat_get(m, i, j, &got) != MAT_OK) return 0;
    double d = got - v;
    if (d < 0) d = -d;
    return d <= eps;
}

int main(void) {
    /* ── zeros ────────────────────────────────────────────────────────── */
    {
        Mat z;
        ISO_CHECK(mat_zeros(2, 3, &z) == MAT_OK);
        ISO_CHECK_EQ_UINT((unsigned)z.rows, 2u);
        ISO_CHECK_EQ_UINT((unsigned)z.cols, 3u);
        ISO_CHECK(at_eq(&z, 1, 2, 0.0, 0.0));
        mat_free(&z);
    }

    /* ── add / subtract ───────────────────────────────────────────────── */
    {
        double av[] = {1, 2, 3, 4}, bv[] = {5, 6, 7, 8};
        Mat a, b, c, d;
        ISO_CHECK(mat_new(2, 2, av, &a) == MAT_OK);
        ISO_CHECK(mat_new(2, 2, bv, &b) == MAT_OK);
        ISO_CHECK(mat_add(&a, &b, &c) == MAT_OK);
        ISO_CHECK(at_eq(&c, 0, 0, 6, 0) && at_eq(&c, 0, 1, 8, 0));
        ISO_CHECK(at_eq(&c, 1, 0, 10, 0) && at_eq(&c, 1, 1, 12, 0));
        ISO_CHECK(mat_subtract(&b, &a, &d) == MAT_OK);
        ISO_CHECK(at_eq(&d, 0, 0, 4, 0) && at_eq(&d, 1, 1, 4, 0));
        /* dimension mismatch -> MAT_ERR_DIM, out left empty (safe to free) */
        double sv[] = {1, 2, 3};
        Mat s, bad;
        ISO_CHECK(mat_new(1, 3, sv, &s) == MAT_OK);
        ISO_CHECK(mat_add(&a, &s, &bad) == MAT_ERR_DIM);
        mat_free(&bad);
        mat_free(&a);
        mat_free(&b);
        mat_free(&c);
        mat_free(&d);
        mat_free(&s);
    }

    /* ── dot ──────────────────────────────────────────────────────────── */
    {
        double av[] = {1, 2, 3, 4}, bv[] = {5, 6, 7, 8};
        Mat a, b, c;
        ISO_CHECK(mat_new(2, 2, av, &a) == MAT_OK);
        ISO_CHECK(mat_new(2, 2, bv, &b) == MAT_OK);
        ISO_CHECK(mat_dot(&a, &b, &c) == MAT_OK);
        ISO_CHECK(at_eq(&c, 0, 0, 19, 0) && at_eq(&c, 0, 1, 22, 0));
        ISO_CHECK(at_eq(&c, 1, 0, 43, 0) && at_eq(&c, 1, 1, 50, 0));
        mat_free(&a);
        mat_free(&b);
        mat_free(&c);
    }

    /* ── identity / from_diagonal / identity*M == M ───────────────────── */
    {
        Mat i3;
        ISO_CHECK(mat_identity(3, &i3) == MAT_OK);
        ISO_CHECK(at_eq(&i3, 0, 0, 1, 0) && at_eq(&i3, 1, 1, 1, 0) &&
                  at_eq(&i3, 2, 2, 1, 0));
        ISO_CHECK(at_eq(&i3, 0, 1, 0, 0) && at_eq(&i3, 2, 0, 0, 0));

        double mv[] = {1, 2, 3, 4, 5, 6, 7, 8, 9};
        Mat m, prod;
        ISO_CHECK(mat_new(3, 3, mv, &m) == MAT_OK);
        ISO_CHECK(mat_dot(&i3, &m, &prod) == MAT_OK);
        ISO_CHECK(mat_equals(&prod, &m));
        mat_free(&i3);
        mat_free(&m);
        mat_free(&prod);

        double dv[] = {2, 3};
        Mat d;
        ISO_CHECK(mat_from_diagonal(dv, 2, &d) == MAT_OK);
        ISO_CHECK(at_eq(&d, 0, 0, 2, 0) && at_eq(&d, 0, 1, 0, 0) &&
                  at_eq(&d, 1, 0, 0, 0) && at_eq(&d, 1, 1, 3, 0));
        mat_free(&d);
    }

    /* ── get / set (immutability) ─────────────────────────────────────── */
    {
        double mv[] = {1, 2, 3, 4};
        Mat m, m2;
        double got;
        ISO_CHECK(mat_new(2, 2, mv, &m) == MAT_OK);
        ISO_CHECK(mat_get(&m, 0, 0, &got) == MAT_OK && got == 1.0);
        ISO_CHECK(mat_get(&m, 1, 1, &got) == MAT_OK && got == 4.0);
        ISO_CHECK(mat_get(&m, 2, 0, &got) == MAT_ERR_BOUNDS);
        ISO_CHECK(mat_set(&m, 0, 0, 99.0, &m2) == MAT_OK);
        ISO_CHECK(mat_get(&m2, 0, 0, &got) == MAT_OK && got == 99.0);
        ISO_CHECK(mat_get(&m, 0, 0, &got) == MAT_OK && got == 1.0); /* orig */
        Mat bad;
        ISO_CHECK(mat_set(&m, 5, 0, 1.0, &bad) == MAT_ERR_BOUNDS);
        mat_free(&bad);
        mat_free(&m);
        mat_free(&m2);
    }

    /* ── reductions ───────────────────────────────────────────────────── */
    {
        double mv[] = {1, 2, 3, 4};
        Mat m, sr, sc;
        ISO_CHECK(mat_new(2, 2, mv, &m) == MAT_OK);
        ISO_CHECK_EQ_DBL(mat_sum(&m), 10.0, 0.0);
        ISO_CHECK_EQ_DBL(mat_mean(&m), 2.5, 0.0);
        ISO_CHECK_EQ_DBL(mat_min_val(&m), 1.0, 0.0);
        ISO_CHECK_EQ_DBL(mat_max_val(&m), 4.0, 0.0);
        ISO_CHECK(mat_sum_rows(&m, &sr) == MAT_OK);
        ISO_CHECK(at_eq(&sr, 0, 0, 3, 0) && at_eq(&sr, 1, 0, 7, 0));
        ISO_CHECK(mat_sum_cols(&m, &sc) == MAT_OK);
        ISO_CHECK(at_eq(&sc, 0, 0, 4, 0) && at_eq(&sc, 0, 1, 6, 0));
        mat_free(&m);
        mat_free(&sr);
        mat_free(&sc);

        /* argmin/argmax + ties */
        size_t r, cc;
        double mv2[] = {1, 2, 3, 4};
        Mat n;
        ISO_CHECK(mat_new(2, 2, mv2, &n) == MAT_OK);
        mat_argmin(&n, &r, &cc);
        ISO_CHECK_EQ_UINT((unsigned)r, 0u);
        ISO_CHECK_EQ_UINT((unsigned)cc, 0u);
        mat_argmax(&n, &r, &cc);
        ISO_CHECK_EQ_UINT((unsigned)r, 1u);
        ISO_CHECK_EQ_UINT((unsigned)cc, 1u);
        mat_free(&n);

        double tv[] = {5, 5, 5, 5};
        Mat t;
        ISO_CHECK(mat_new(2, 2, tv, &t) == MAT_OK);
        mat_argmin(&t, &r, &cc);
        ISO_CHECK(r == 0 && cc == 0);
        mat_argmax(&t, &r, &cc);
        ISO_CHECK(r == 0 && cc == 0); /* first occurrence wins */
        mat_free(&t);

        /* larger reductions */
        double lv[] = {1, 2, 3, 4, 5, 6};
        Mat l, lr, lc;
        ISO_CHECK(mat_new(2, 3, lv, &l) == MAT_OK);
        ISO_CHECK_EQ_DBL(mat_sum(&l), 21.0, 0.0);
        ISO_CHECK_EQ_DBL(mat_mean(&l), 3.5, 0.0);
        ISO_CHECK(mat_sum_rows(&l, &lr) == MAT_OK);
        ISO_CHECK(at_eq(&lr, 0, 0, 6, 0) && at_eq(&lr, 1, 0, 15, 0));
        ISO_CHECK(mat_sum_cols(&l, &lc) == MAT_OK);
        ISO_CHECK(at_eq(&lc, 0, 0, 5, 0) && at_eq(&lc, 0, 1, 7, 0) &&
                  at_eq(&lc, 0, 2, 9, 0));
        mat_free(&l);
        mat_free(&lr);
        mat_free(&lc);
    }

    /* ── element-wise math ────────────────────────────────────────────── */
    {
        double mv[] = {1, 4, 9, 16};
        Mat m, s;
        ISO_CHECK(mat_new(2, 2, mv, &m) == MAT_OK);
        ISO_CHECK(mat_sqrt(&m, &s) == MAT_OK);
        ISO_CHECK(at_eq(&s, 0, 0, 1, 1e-9) && at_eq(&s, 0, 1, 2, 1e-9) &&
                  at_eq(&s, 1, 0, 3, 1e-9) && at_eq(&s, 1, 1, 4, 1e-9));
        mat_free(&m);
        mat_free(&s);

        double nv[] = {-1, 2, -3, 4};
        Mat n, a;
        ISO_CHECK(mat_new(2, 2, nv, &n) == MAT_OK);
        ISO_CHECK(mat_abs(&n, &a) == MAT_OK);
        ISO_CHECK(at_eq(&a, 0, 0, 1, 0) && at_eq(&a, 1, 0, 3, 0));
        mat_free(&n);
        mat_free(&a);

        double pv[] = {1, 2, 3, 4};
        Mat p, sq, half, cube;
        ISO_CHECK(mat_new(2, 2, pv, &p) == MAT_OK);
        ISO_CHECK(mat_pow(&p, 2.0, &sq) == MAT_OK);
        ISO_CHECK(at_eq(&sq, 0, 1, 4, 1e-9) && at_eq(&sq, 1, 1, 16, 1e-9));
        /* general (non-integer) exponent path: x^0.5 == sqrt(x) */
        ISO_CHECK(mat_pow(&p, 0.5, &half) == MAT_OK);
        ISO_CHECK(at_eq(&half, 1, 1, 2.0, 1e-9)); /* 4^0.5 == 2 */
        /* integer fast path, larger exponent */
        ISO_CHECK(mat_pow(&p, 3.0, &cube) == MAT_OK);
        ISO_CHECK(at_eq(&cube, 1, 1, 64, 1e-9)); /* 4^3 */
        /* close(m, sqrt(m).pow(2)) within 1e-9 */
        Mat rs, rp;
        ISO_CHECK(mat_sqrt(&p, &rs) == MAT_OK);
        ISO_CHECK(mat_pow(&rs, 2.0, &rp) == MAT_OK);
        ISO_CHECK(mat_close(&p, &rp, 1e-9));
        mat_free(&p);
        mat_free(&sq);
        mat_free(&half);
        mat_free(&cube);
        mat_free(&rs);
        mat_free(&rp);
    }

    /* ── shape operations ─────────────────────────────────────────────── */
    {
        double mv[] = {1, 2, 3, 4};
        Mat m, f, rt;
        ISO_CHECK(mat_new(2, 2, mv, &m) == MAT_OK);
        ISO_CHECK(mat_flatten(&m, &f) == MAT_OK);
        ISO_CHECK_EQ_UINT((unsigned)f.rows, 1u);
        ISO_CHECK_EQ_UINT((unsigned)f.cols, 4u);
        ISO_CHECK(at_eq(&f, 0, 0, 1, 0) && at_eq(&f, 0, 3, 4, 0));
        ISO_CHECK(mat_reshape(&f, 2, 2, &rt) == MAT_OK);
        ISO_CHECK(mat_equals(&rt, &m));
        mat_free(&f);
        mat_free(&rt);

        Mat bad;
        ISO_CHECK(mat_reshape(&m, 3, 3, &bad) == MAT_ERR_DIM);
        mat_free(&bad);
        mat_free(&m);

        double flv[] = {1, 2, 3, 4, 5, 6};
        Mat flat, resh;
        ISO_CHECK(mat_new_1d(flv, 6, &flat) == MAT_OK);
        ISO_CHECK(mat_reshape(&flat, 2, 3, &resh) == MAT_OK);
        ISO_CHECK(at_eq(&resh, 0, 2, 3, 0) && at_eq(&resh, 1, 0, 4, 0));
        mat_free(&flat);
        mat_free(&resh);
    }

    /* ── row / col / slice ────────────────────────────────────────────── */
    {
        double mv[] = {1, 2, 3, 4};
        Mat m, r0, c1, sl, bad;
        ISO_CHECK(mat_new(2, 2, mv, &m) == MAT_OK);
        ISO_CHECK(mat_row(&m, 0, &r0) == MAT_OK);
        ISO_CHECK(at_eq(&r0, 0, 0, 1, 0) && at_eq(&r0, 0, 1, 2, 0));
        ISO_CHECK(mat_row(&m, 2, &bad) == MAT_ERR_BOUNDS);
        mat_free(&bad);
        ISO_CHECK(mat_col(&m, 1, &c1) == MAT_OK);
        ISO_CHECK(at_eq(&c1, 0, 0, 2, 0) && at_eq(&c1, 1, 0, 4, 0));
        ISO_CHECK(mat_slice(&m, 0, 2, 0, 1, &sl) == MAT_OK);
        ISO_CHECK(at_eq(&sl, 0, 0, 1, 0) && at_eq(&sl, 1, 0, 3, 0));
        mat_free(&r0);
        mat_free(&c1);
        mat_free(&sl);

        Mat bad2;
        ISO_CHECK(mat_slice(&m, 0, 3, 0, 1, &bad2) == MAT_ERR_BOUNDS);
        mat_free(&bad2);
        ISO_CHECK(mat_slice(&m, 1, 0, 0, 1, &bad2) == MAT_ERR_BOUNDS);
        mat_free(&bad2);
        mat_free(&m);

        double bv[] = {1, 2, 3, 4, 5, 6, 7, 8, 9};
        Mat big, sl2;
        ISO_CHECK(mat_new(3, 3, bv, &big) == MAT_OK);
        ISO_CHECK(mat_slice(&big, 0, 2, 1, 3, &sl2) == MAT_OK);
        ISO_CHECK(at_eq(&sl2, 0, 0, 2, 0) && at_eq(&sl2, 0, 1, 3, 0) &&
                  at_eq(&sl2, 1, 0, 5, 0) && at_eq(&sl2, 1, 1, 6, 0));
        mat_free(&big);
        mat_free(&sl2);
    }

    /* ── equality / closeness ─────────────────────────────────────────── */
    {
        double av[] = {1, 2, 3, 4}, cv[] = {1, 2, 3, 5};
        Mat a, b, c;
        ISO_CHECK(mat_new(2, 2, av, &a) == MAT_OK);
        ISO_CHECK(mat_new(2, 2, av, &b) == MAT_OK);
        ISO_CHECK(mat_new(2, 2, cv, &c) == MAT_OK);
        ISO_CHECK(mat_equals(&a, &b));
        ISO_CHECK(!mat_equals(&a, &c));
        mat_free(&a);
        mat_free(&b);
        mat_free(&c);

        /* different shapes are never equal / close */
        double dv[] = {1, 2, 3};
        Mat p, q;
        ISO_CHECK(mat_new(2, 2, av, &p) == MAT_OK);
        ISO_CHECK(mat_new_1d(dv, 3, &q) == MAT_OK);
        ISO_CHECK(!mat_equals(&p, &q));
        ISO_CHECK(!mat_close(&p, &q, 1e-9));
        mat_free(&p);
        mat_free(&q);

        Mat s1, s2;
        ISO_CHECK(mat_new_scalar(1.0000000001, &s1) == MAT_OK);
        ISO_CHECK(mat_new_scalar(1.0, &s2) == MAT_OK);
        ISO_CHECK(mat_close(&s1, &s2, 1e-9));
        mat_free(&s1);
        mat_free(&s2);

        Mat t1, t2;
        ISO_CHECK(mat_new_scalar(1.1, &t1) == MAT_OK);
        ISO_CHECK(mat_new_scalar(1.0, &t2) == MAT_OK);
        ISO_CHECK(!mat_close(&t1, &t2, 0.01));
        mat_free(&t1);
        mat_free(&t2);
    }

    /* ── transpose / scale / add_scalar ───────────────────────────────── */
    {
        double mv[] = {1, 2, 3, 4, 5, 6};
        Mat m, t, sc, as;
        ISO_CHECK(mat_new(2, 3, mv, &m) == MAT_OK); /* 2x3 */
        ISO_CHECK(mat_transpose(&m, &t) == MAT_OK); /* 3x2 */
        ISO_CHECK_EQ_UINT((unsigned)t.rows, 3u);
        ISO_CHECK_EQ_UINT((unsigned)t.cols, 2u);
        ISO_CHECK(at_eq(&t, 0, 0, 1, 0) && at_eq(&t, 2, 1, 6, 0) &&
                  at_eq(&t, 1, 0, 2, 0));
        ISO_CHECK(mat_scale(&m, 2.0, &sc) == MAT_OK);
        ISO_CHECK(at_eq(&sc, 0, 0, 2, 0) && at_eq(&sc, 1, 2, 12, 0));
        ISO_CHECK(mat_add_scalar(&m, 10.0, &as) == MAT_OK);
        ISO_CHECK(at_eq(&as, 0, 0, 11, 0) && at_eq(&as, 1, 2, 16, 0));
        mat_free(&m);
        mat_free(&t);
        mat_free(&sc);
        mat_free(&as);
    }

    return ISO_TEST_RESULT();
}
