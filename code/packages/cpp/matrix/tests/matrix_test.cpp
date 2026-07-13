// Tests for the C++ matrix library, using the header-only iso_test.h harness
// (pure ISO). Expected values come straight from the Rust crate's unit tests,
// so this suite verifies the port is faithful.
#include "iso_test.h"

#include <stdexcept>

#include "matrix.hpp"

using ca::matrix::Matrix;

// Returns true iff calling `fn` throws any std::exception (mirrors Rust
// `.is_err()`).
template <typename F>
static bool throws(F fn) {
    try {
        fn();
    } catch (const std::exception&) {
        return true;
    }
    return false;
}

int main() {
    // ── zeros ──────────────────────────────────────────────────────────────
    {
        Matrix z = Matrix::zeros(2, 3);
        ISO_CHECK_EQ_UINT(static_cast<unsigned>(z.rows), 2u);
        ISO_CHECK_EQ_UINT(static_cast<unsigned>(z.cols), 3u);
        ISO_CHECK_EQ_DBL(z.data[1][2], 0.0, 0.0);
    }

    // ── add / subtract ─────────────────────────────────────────────────────
    {
        Matrix a({{1, 2}, {3, 4}});
        Matrix b({{5, 6}, {7, 8}});
        Matrix c = a.add(b);
        ISO_CHECK_EQ_DBL(c.data[0][0], 6.0, 0.0);
        ISO_CHECK_EQ_DBL(c.data[1][1], 12.0, 0.0);
        Matrix d = b.subtract(a);
        ISO_CHECK_EQ_DBL(d.data[0][0], 4.0, 0.0);
        ISO_CHECK_EQ_DBL(d.data[1][1], 4.0, 0.0);
        ISO_CHECK(throws([&] { a.add(Matrix::new_1d({1, 2, 3})); }));
    }

    // ── dot ────────────────────────────────────────────────────────────────
    {
        Matrix a({{1, 2}, {3, 4}});
        Matrix b({{5, 6}, {7, 8}});
        Matrix c = a.dot(b);
        ISO_CHECK_EQ_DBL(c.data[0][0], 19.0, 0.0);
        ISO_CHECK_EQ_DBL(c.data[0][1], 22.0, 0.0);
        ISO_CHECK_EQ_DBL(c.data[1][0], 43.0, 0.0);
        ISO_CHECK_EQ_DBL(c.data[1][1], 50.0, 0.0);
        ISO_CHECK(throws([&] { a.dot(Matrix::new_1d({1, 2, 3})); }));
    }

    // ── identity / from_diagonal / identity·M == M ─────────────────────────
    {
        Matrix i3 = Matrix::identity(3);
        ISO_CHECK_EQ_DBL(i3.data[0][0], 1.0, 0.0);
        ISO_CHECK_EQ_DBL(i3.data[1][1], 1.0, 0.0);
        ISO_CHECK_EQ_DBL(i3.data[0][1], 0.0, 0.0);
        Matrix m({{1, 2, 3}, {4, 5, 6}, {7, 8, 9}});
        ISO_CHECK(i3.dot(m).equals(m));
        Matrix d = Matrix::from_diagonal({2, 3});
        ISO_CHECK_EQ_DBL(d.data[0][0], 2.0, 0.0);
        ISO_CHECK_EQ_DBL(d.data[0][1], 0.0, 0.0);
        ISO_CHECK_EQ_DBL(d.data[1][1], 3.0, 0.0);
    }

    // ── get / set (immutability) ───────────────────────────────────────────
    {
        Matrix m({{1, 2}, {3, 4}});
        ISO_CHECK_EQ_DBL(m.get(0, 0), 1.0, 0.0);
        ISO_CHECK_EQ_DBL(m.get(1, 1), 4.0, 0.0);
        ISO_CHECK(throws([&] { m.get(2, 0); }));
        Matrix m2 = m.set(0, 0, 99.0);
        ISO_CHECK_EQ_DBL(m2.get(0, 0), 99.0, 0.0);
        ISO_CHECK_EQ_DBL(m.get(0, 0), 1.0, 0.0);  // original unchanged
        ISO_CHECK(throws([&] { m.set(5, 0, 1.0); }));
    }

    // ── reductions ─────────────────────────────────────────────────────────
    {
        Matrix m({{1, 2}, {3, 4}});
        ISO_CHECK_EQ_DBL(m.sum(), 10.0, 0.0);
        ISO_CHECK_EQ_DBL(m.mean(), 2.5, 0.0);
        ISO_CHECK_EQ_DBL(m.min_val(), 1.0, 0.0);
        ISO_CHECK_EQ_DBL(m.max_val(), 4.0, 0.0);
        ISO_CHECK_EQ_DBL(m.sum_rows().data[0][0], 3.0, 0.0);
        ISO_CHECK_EQ_DBL(m.sum_rows().data[1][0], 7.0, 0.0);
        ISO_CHECK_EQ_DBL(m.sum_cols().data[0][0], 4.0, 0.0);
        ISO_CHECK_EQ_DBL(m.sum_cols().data[0][1], 6.0, 0.0);

        auto mn = m.argmin();
        ISO_CHECK(mn.first == 0 && mn.second == 0);
        auto mx = m.argmax();
        ISO_CHECK(mx.first == 1 && mx.second == 1);

        Matrix t({{5, 5}, {5, 5}});  // ties resolve to first occurrence (0,0)
        auto tmn = t.argmin();
        ISO_CHECK(tmn.first == 0 && tmn.second == 0);
        auto tmx = t.argmax();
        ISO_CHECK(tmx.first == 0 && tmx.second == 0);

        Matrix l({{1, 2, 3}, {4, 5, 6}});
        ISO_CHECK_EQ_DBL(l.sum(), 21.0, 0.0);
        ISO_CHECK_EQ_DBL(l.mean(), 3.5, 0.0);
        ISO_CHECK_EQ_DBL(l.sum_rows().data[1][0], 15.0, 0.0);
        ISO_CHECK_EQ_DBL(l.sum_cols().data[0][2], 9.0, 0.0);
    }

    // ── element-wise math ──────────────────────────────────────────────────
    {
        Matrix m({{1, 4}, {9, 16}});
        Matrix s = m.sqrt();
        ISO_CHECK_EQ_DBL(s.data[0][1], 2.0, 1e-9);
        ISO_CHECK_EQ_DBL(s.data[1][1], 4.0, 1e-9);

        Matrix n({{-1, 2}, {-3, 4}});
        Matrix a = n.abs_val();
        ISO_CHECK_EQ_DBL(a.data[0][0], 1.0, 0.0);
        ISO_CHECK_EQ_DBL(a.data[1][0], 3.0, 0.0);

        Matrix p({{1, 2}, {3, 4}});
        Matrix sq = p.pow_val(2.0);
        ISO_CHECK_EQ_DBL(sq.data[0][1], 4.0, 1e-9);
        ISO_CHECK_EQ_DBL(sq.data[1][1], 16.0, 1e-9);
        // general (non-integer) exponent path
        ISO_CHECK_EQ_DBL(p.pow_val(0.5).data[1][1], 2.0, 1e-9);  // 4^0.5
        // integer fast path, larger exponent
        ISO_CHECK_EQ_DBL(p.pow_val(3.0).data[1][1], 64.0, 1e-9);  // 4^3
        // custom map
        Matrix doubled = p.map([](double x) { return x * 2.0; });
        ISO_CHECK_EQ_DBL(doubled.data[1][1], 8.0, 0.0);
        // close(m, sqrt(m).pow(2))
        ISO_CHECK(p.close(p.sqrt().pow_val(2.0), 1e-9));
    }

    // ── shape operations ───────────────────────────────────────────────────
    {
        Matrix m({{1, 2}, {3, 4}});
        Matrix f = m.flatten();
        ISO_CHECK_EQ_UINT(static_cast<unsigned>(f.rows), 1u);
        ISO_CHECK_EQ_UINT(static_cast<unsigned>(f.cols), 4u);
        ISO_CHECK_EQ_DBL(f.data[0][3], 4.0, 0.0);
        ISO_CHECK(f.reshape(2, 2).equals(m));
        ISO_CHECK(throws([&] { m.reshape(3, 3); }));
        // ragged input is rejected at construction (memory-safety guard)
        ISO_CHECK(throws([] { Matrix({{1, 2}, {3}}); }));
        // an overflowing reshape product must not alias the true count
        ISO_CHECK(throws([&] {
            m.reshape(2, (static_cast<std::size_t>(-1) / 2) + 3);
        }));

        Matrix flat = Matrix::new_1d({1, 2, 3, 4, 5, 6});
        Matrix resh = flat.reshape(2, 3);
        ISO_CHECK_EQ_DBL(resh.data[0][2], 3.0, 0.0);
        ISO_CHECK_EQ_DBL(resh.data[1][0], 4.0, 0.0);
    }

    // ── row / col / slice ──────────────────────────────────────────────────
    {
        Matrix m({{1, 2}, {3, 4}});
        ISO_CHECK_EQ_DBL(m.row(0).data[0][1], 2.0, 0.0);
        ISO_CHECK_EQ_DBL(m.row(1).data[0][0], 3.0, 0.0);
        ISO_CHECK(throws([&] { m.row(2); }));
        ISO_CHECK_EQ_DBL(m.col(1).data[0][0], 2.0, 0.0);
        ISO_CHECK_EQ_DBL(m.col(1).data[1][0], 4.0, 0.0);
        ISO_CHECK(throws([&] { m.col(2); }));

        Matrix s = m.slice(0, 2, 0, 1);
        ISO_CHECK_EQ_DBL(s.data[0][0], 1.0, 0.0);
        ISO_CHECK_EQ_DBL(s.data[1][0], 3.0, 0.0);
        ISO_CHECK(throws([&] { m.slice(0, 3, 0, 1); }));
        ISO_CHECK(throws([&] { m.slice(1, 0, 0, 1); }));

        Matrix big({{1, 2, 3}, {4, 5, 6}, {7, 8, 9}});
        Matrix s2 = big.slice(0, 2, 1, 3);
        ISO_CHECK_EQ_DBL(s2.data[0][0], 2.0, 0.0);
        ISO_CHECK_EQ_DBL(s2.data[0][1], 3.0, 0.0);
        ISO_CHECK_EQ_DBL(s2.data[1][0], 5.0, 0.0);
        ISO_CHECK_EQ_DBL(s2.data[1][1], 6.0, 0.0);
    }

    // ── equality / closeness / transpose / scale / add_scalar ──────────────
    {
        Matrix a({{1, 2}, {3, 4}});
        Matrix b({{1, 2}, {3, 4}});
        Matrix c({{1, 2}, {3, 5}});
        ISO_CHECK(a.equals(b));
        ISO_CHECK(!a.equals(c));
        ISO_CHECK(!a.equals(Matrix::new_1d({1, 2, 3})));

        ISO_CHECK(Matrix::new_scalar(1.0000000001).close(Matrix::new_scalar(1.0),
                                                         1e-9));
        ISO_CHECK(!Matrix::new_scalar(1.1).close(Matrix::new_scalar(1.0), 0.01));
        ISO_CHECK(!Matrix::new_scalar(1.0).close(Matrix::new_1d({1, 2}), 1e-9));

        Matrix m({{1, 2, 3}, {4, 5, 6}});  // 2x3
        Matrix t = m.transpose();           // 3x2
        ISO_CHECK_EQ_UINT(static_cast<unsigned>(t.rows), 3u);
        ISO_CHECK_EQ_UINT(static_cast<unsigned>(t.cols), 2u);
        ISO_CHECK_EQ_DBL(t.data[2][1], 6.0, 0.0);
        ISO_CHECK_EQ_DBL(m.scale(2.0).data[1][2], 12.0, 0.0);
        ISO_CHECK_EQ_DBL(m.add_scalar(10.0).data[1][2], 16.0, 0.0);
    }

    return ISO_TEST_RESULT();
}
