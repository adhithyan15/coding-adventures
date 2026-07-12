// Tests for the C++ feature-normalization library, using the header-only
// iso_test.h harness (pure ISO). Vectors mirror the Rust crate's own tests.
#include "iso_test.h"

#include <stdexcept>
#include <vector>

#include "feature_normalization.hpp"

namespace fn = ca::feature_normalization;

int main() {
    const double eps = 1e-9;

    const fn::Matrix data = {
        {1000.0, 3.0, 1.0},
        {1500.0, 4.0, 0.0},
        {2000.0, 5.0, 1.0},
    };

    // ── StandardScaler centers and scales columns ─────────────────────────
    {
        fn::StandardScaler s = fn::fit_standard_scaler(data);
        ISO_CHECK_EQ_UINT(s.means.size(), 3u);
        ISO_CHECK_EQ_DBL(s.means[0], 1500.0, eps);
        ISO_CHECK_EQ_DBL(s.means[1], 4.0, eps);
        ISO_CHECK_EQ_DBL(s.means[2], 2.0 / 3.0, eps);

        fn::Matrix out = fn::transform_standard(data, s);
        ISO_CHECK_EQ_DBL(out[0][0], -1.224744871391589, eps);
        ISO_CHECK_EQ_DBL(out[1][0], 0.0, eps);
        ISO_CHECK_EQ_DBL(out[2][0], 1.224744871391589, eps);
    }

    // ── MinMaxScaler maps to the unit range ───────────────────────────────
    {
        fn::MinMaxScaler s = fn::fit_min_max_scaler(data);
        ISO_CHECK_EQ_DBL(s.minimums[0], 1000.0, eps);
        ISO_CHECK_EQ_DBL(s.maximums[0], 2000.0, eps);

        fn::Matrix out = fn::transform_min_max(data, s);
        const double expected[3][3] = {
            {0.0, 0.0, 1.0}, {0.5, 0.5, 0.0}, {1.0, 1.0, 1.0}};
        for (int r = 0; r < 3; r++)
            for (int c = 0; c < 3; c++)
                ISO_CHECK_EQ_DBL(out[static_cast<std::size_t>(r)]
                                    [static_cast<std::size_t>(c)],
                                 expected[r][c], eps);
    }

    // ── constant columns map to zero ──────────────────────────────────────
    {
        fn::Matrix cdata = {{1.0, 7.0}, {2.0, 7.0}};  // col 1 constant = 7
        fn::StandardScaler ss = fn::fit_standard_scaler(cdata);
        fn::MinMaxScaler ms = fn::fit_min_max_scaler(cdata);
        ISO_CHECK_EQ_DBL(ss.standard_deviations[1], 0.0, eps);

        fn::Matrix so = fn::transform_standard(cdata, ss);
        fn::Matrix mo = fn::transform_min_max(cdata, ms);
        ISO_CHECK_EQ_DBL(so[0][1], 0.0, eps);
        ISO_CHECK_EQ_DBL(mo[0][1], 0.0, eps);
        ISO_CHECK_EQ_DBL(mo[0][0], 0.0, eps);
        ISO_CHECK_EQ_DBL(mo[1][0], 1.0, eps);
    }

    // ── validation errors throw std::invalid_argument ─────────────────────
    {
        bool threw = false;
        try {
            (void)fn::fit_standard_scaler({});  // empty matrix
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK(threw);

        threw = false;
        try {
            (void)fn::fit_min_max_scaler({{1.0, 2.0}, {3.0}});  // ragged rows
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK(threw);

        threw = false;
        try {
            fn::StandardScaler s = fn::fit_standard_scaler(data);  // width 3
            (void)fn::transform_standard({{1.0, 2.0}}, s);         // width 2
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    return ISO_TEST_RESULT();
}
