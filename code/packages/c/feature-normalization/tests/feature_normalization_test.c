/*
 * Tests for the C feature-normalization library, using the header-only
 * iso_test.h harness (pure ISO). Vectors mirror the Rust crate's own tests.
 */
#include "iso_test.h"

#include "feature_normalization.h"

int main(void) {
    const double eps = 1e-9;

    /* The 3x3 training matrix from the Rust tests (row-major flat). */
    double data[9] = {
        1000.0, 3.0, 1.0, /* row 0 */
        1500.0, 4.0, 0.0, /* row 1 */
        2000.0, 5.0, 1.0  /* row 2 */
    };

    /* ── StandardScaler: centers and scales each column ──────────────────── */
    {
        FnStandardScaler s;
        ISO_CHECK(fn_fit_standard_scaler(data, 3, 3, &s) == FN_OK);
        ISO_CHECK_EQ_UINT(s.width, 3u);
        ISO_CHECK_EQ_DBL(s.means[0], 1500.0, eps);
        ISO_CHECK_EQ_DBL(s.means[1], 4.0, eps);
        /* Column 2 is {1,0,1}: mean 2/3, population stddev sqrt(2/9). */
        ISO_CHECK_EQ_DBL(s.means[2], 2.0 / 3.0, eps);

        double out[9];
        ISO_CHECK(fn_transform_standard(data, 3, 3, &s, out) == FN_OK);
        /* Column 0 z-scores for {1000,1500,2000}. */
        ISO_CHECK_EQ_DBL(out[0], -1.224744871391589, eps);
        ISO_CHECK_EQ_DBL(out[3], 0.0, eps);
        ISO_CHECK_EQ_DBL(out[6], 1.224744871391589, eps);
        fn_standard_scaler_free(&s);
    }

    /* ── MinMaxScaler: maps each column to [0, 1] ────────────────────────── */
    {
        FnMinMaxScaler s;
        ISO_CHECK(fn_fit_min_max_scaler(data, 3, 3, &s) == FN_OK);
        ISO_CHECK_EQ_DBL(s.minimums[0], 1000.0, eps);
        ISO_CHECK_EQ_DBL(s.maximums[0], 2000.0, eps);

        double out[9];
        ISO_CHECK(fn_transform_min_max(data, 3, 3, &s, out) == FN_OK);
        /* Expected: [[0,0,1],[0.5,0.5,0],[1,1,1]]. */
        ISO_CHECK_EQ_DBL(out[0], 0.0, eps);
        ISO_CHECK_EQ_DBL(out[1], 0.0, eps);
        ISO_CHECK_EQ_DBL(out[2], 1.0, eps);
        ISO_CHECK_EQ_DBL(out[3], 0.5, eps);
        ISO_CHECK_EQ_DBL(out[4], 0.5, eps);
        ISO_CHECK_EQ_DBL(out[5], 0.0, eps);
        ISO_CHECK_EQ_DBL(out[6], 1.0, eps);
        ISO_CHECK_EQ_DBL(out[7], 1.0, eps);
        ISO_CHECK_EQ_DBL(out[8], 1.0, eps);
        fn_min_max_scaler_free(&s);
    }

    /* ── constant columns map to zero (no divide-by-zero) ────────────────── */
    {
        double cdata[4] = {1.0, 7.0, 2.0, 7.0}; /* col 1 constant = 7 */
        FnStandardScaler ss;
        FnMinMaxScaler ms;
        ISO_CHECK(fn_fit_standard_scaler(cdata, 2, 2, &ss) == FN_OK);
        ISO_CHECK(fn_fit_min_max_scaler(cdata, 2, 2, &ms) == FN_OK);
        ISO_CHECK_EQ_DBL(ss.standard_deviations[1], 0.0, eps);

        double so[4], mo[4];
        ISO_CHECK(fn_transform_standard(cdata, 2, 2, &ss, so) == FN_OK);
        ISO_CHECK(fn_transform_min_max(cdata, 2, 2, &ms, mo) == FN_OK);
        ISO_CHECK_EQ_DBL(so[1], 0.0, eps);
        ISO_CHECK_EQ_DBL(mo[1], 0.0, eps);
        /* Column 0 still scales normally. */
        ISO_CHECK_EQ_DBL(mo[0], 0.0, eps);
        ISO_CHECK_EQ_DBL(mo[2], 1.0, eps);
        fn_standard_scaler_free(&ss);
        fn_min_max_scaler_free(&ms);
    }

    /* ── error cases: empty matrix and width mismatch ────────────────────── */
    {
        FnStandardScaler s;
        ISO_CHECK(fn_fit_standard_scaler(data, 0, 3, &s) == FN_ERR_EMPTY);
        ISO_CHECK(fn_fit_standard_scaler(data, 3, 0, &s) == FN_ERR_EMPTY);

        FnMinMaxScaler m;
        ISO_CHECK(fn_fit_min_max_scaler(data, 0, 0, &m) == FN_ERR_EMPTY);

        /* Fit width 3, then transform a width-2 matrix -> mismatch. */
        ISO_CHECK(fn_fit_standard_scaler(data, 3, 3, &s) == FN_OK);
        double twowide[4] = {1.0, 2.0, 3.0, 4.0};
        double out[4];
        ISO_CHECK(fn_transform_standard(twowide, 2, 2, &s, out) ==
                  FN_ERR_WIDTH_MISMATCH);
        fn_standard_scaler_free(&s);
    }

    return ISO_TEST_RESULT();
}
