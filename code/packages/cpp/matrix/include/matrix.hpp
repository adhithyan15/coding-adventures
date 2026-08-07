// matrix.hpp — a small, dependency-free 2D matrix of `double`, header-only in
// pure ISO C++17 (namespace ca::matrix). A faithful port of the Rust `matrix`
// crate.
// ===========================================================================
//
// The Rust type stores elements as `Vec<Vec<f64>>` — a vector of row vectors.
// This port keeps exactly that shape: `std::vector<std::vector<double>> data`,
// with cached `rows` and `cols`. Every method returns a *new* Matrix and never
// mutates `*this` (Rust's immutable-by-default design).
//
// DIVERGENCE FROM RUST. Rust returns `Result<Matrix, _>` for dimension
// mismatches and out-of-bounds access; the idiomatic C++ equivalent is to
// throw — this port throws std::invalid_argument (dimension mismatch) and
// std::out_of_range (bad index), carrying the same intent.
//
// NO libm / <cmath>. The pure-ISO build links no math library, so `sqrt` and a
// general `pow` are computed from scratch below; they reproduce the Rust f64
// results to ~1e-12 relative, well inside every test tolerance.
//
// PORTABILITY. Pure ISO C++17, no <cmath>, no compiler extensions; compiles
// clean under GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
// warnings-as-errors.
#ifndef CA_MATRIX_HPP
#define CA_MATRIX_HPP

#include <cstddef>
#include <functional>
#include <stdexcept>
#include <vector>

namespace ca {
namespace matrix {

namespace detail {

inline double d_abs(double x) { return x < 0.0 ? -x : x; }

// Newton–Raphson sqrt. sqrt(x<=0) is defined as 0.0 (Rust f64::sqrt(x<0) would
// be NaN, but the crate only square-roots non-negative data and producing NaN
// without <cmath> risks UB).
inline double d_sqrt(double x) {
    if (x <= 0.0) return 0.0;
    double guess = x >= 1.0 ? x : 1.0;
    for (int i = 0; i < 100; i++) {
        double next = 0.5 * (guess + x / guess);
        if (d_abs(next - guess) <= 1e-15 * guess) return next;
        guess = next;
    }
    return guess;
}

inline double pow2i(int k) {
    double result = 1.0;
    double base = k < 0 ? 0.5 : 2.0;
    int n = k < 0 ? -k : k;
    while (n > 0) {
        if (n & 1) result *= base;
        base *= base;
        n >>= 1;
    }
    return result;
}

// e^x via Cody–Waite range reduction (guards precede the (int) cast).
inline double d_exp(double x) {
    if (x != x) return x;
    if (x == 0.0) return 1.0;
    if (x > 709.782712893384) return 1.7976931348623157e308;
    if (x < -745.13321910194) return 0.0;
    constexpr double INV_LN2 = 1.4426950408889634;
    constexpr double C1 = 0.693359375;
    constexpr double C2 = -2.1219444005469058277e-4;
    double kf = x * INV_LN2;
    int k = static_cast<int>(kf >= 0.0 ? kf + 0.5 : kf - 0.5);
    double r = (x - static_cast<double>(k) * C1) - static_cast<double>(k) * C2;
    double term = 1.0, sum = 1.0;
    for (int i = 1; i <= 17; i++) {
        term *= r / static_cast<double>(i);
        sum += term;
    }
    return sum * pow2i(k);
}

// ln(x) with top guards so the reduction loops never spin on non-finite /
// non-positive input.
inline double d_ln(double x) {
    if (x != x) return x;
    if (x <= 0.0) return -1.7976931348623157e308;
    if (x > 1.7976931348623157e308) return 1.7976931348623157e308;
    int e = 0;
    double m = x;
    while (m < 1.0) { m *= 2.0; e--; }
    while (m >= 2.0) { m *= 0.5; e++; }
    double u = (m - 1.0) / (m + 1.0);
    double u2 = u * u;
    double term = u, sum = u;
    for (int n = 1; n <= 60; n++) {
        term *= u2;
        double add = term / static_cast<double>(2 * n + 1);
        sum += add;
        if (d_abs(add) < 1e-17) break;
    }
    constexpr double LN2 = 0.6931471805599453;
    return static_cast<double>(e) * LN2 + 2.0 * sum;
}

// x^y matching Rust f64::powf on this crate's inputs (integer-exponent fast
// path is exact and sign-correct; positive base uses exp(y*ln x); a fractional
// power of a negative base — which the crate never takes — returns 0.0).
inline double d_pow(double x, double y) {
    if (y == 0.0) return 1.0;
    if (x == 0.0) return y > 0.0 ? 0.0 : 1.7976931348623157e308;
    if (d_abs(y) < 1e15) {
        double ry = y < 0.0 ? -static_cast<double>(static_cast<long long>(-y))
                            : static_cast<double>(static_cast<long long>(y));
        if (ry == y) {
            long long n = static_cast<long long>(ry);
            bool neg = n < 0;
            unsigned long long k =
                static_cast<unsigned long long>(neg ? -n : n);
            double result = 1.0, base = x;
            while (k > 0) {
                if (k & 1ULL) result *= base;
                base *= base;
                k >>= 1;
            }
            return neg ? 1.0 / result : result;
        }
    }
    if (x > 0.0) return d_exp(y * d_ln(x));
    return 0.0;
}

}  // namespace detail

class Matrix {
public:
    std::vector<std::vector<double>> data;
    std::size_t rows;
    std::size_t cols;

    // ─── Constructors ────────────────────────────────────────────────────
    explicit Matrix(std::vector<std::vector<double>> d)
        : data(std::move(d)),
          rows(data.size()),
          cols(data.empty() ? 0 : data[0].size()) {
        // `cols` is taken from row 0; require every other row to match so that
        // the unchecked `data[i][j]` accesses throughout the class can never
        // read past a short row (a ragged input would otherwise be a
        // memory-safety footgun the flat-buffer C port cannot express).
        for (const auto& row : data)
            if (row.size() != cols)
                throw std::invalid_argument("Matrix rows must be equal length");
    }

    // One-row matrix from a 1D vector (Rust new_1d).
    static Matrix new_1d(std::vector<double> d) {
        std::size_t c = d.size();
        std::vector<std::vector<double>> v;
        v.push_back(std::move(d));
        Matrix m(std::move(v));
        m.cols = c;
        return m;
    }

    // 1x1 matrix (Rust new_scalar).
    static Matrix new_scalar(double val) {
        return Matrix({std::vector<double>{val}});
    }

    // rows x cols of zeros.
    static Matrix zeros(std::size_t r, std::size_t c) {
        return Matrix(std::vector<std::vector<double>>(
            r, std::vector<double>(c, 0.0)));
    }

    // n x n identity.
    static Matrix identity(std::size_t n) {
        Matrix m = zeros(n, n);
        for (std::size_t i = 0; i < n; i++) m.data[i][i] = 1.0;
        return m;
    }

    // n x n diagonal from `values`.
    static Matrix from_diagonal(const std::vector<double>& values) {
        std::size_t n = values.size();
        Matrix m = zeros(n, n);
        for (std::size_t i = 0; i < n; i++) m.data[i][i] = values[i];
        return m;
    }

    // ─── Basic arithmetic ────────────────────────────────────────────────
    Matrix add(const Matrix& other) const {
        require_same_shape(other, "Matrix addition dimensions mismatch");
        Matrix c = zeros(rows, cols);
        for (std::size_t i = 0; i < rows; i++)
            for (std::size_t j = 0; j < cols; j++)
                c.data[i][j] = data[i][j] + other.data[i][j];
        return c;
    }

    Matrix add_scalar(double scalar) const {
        return map([scalar](double v) { return v + scalar; });
    }

    Matrix subtract(const Matrix& other) const {
        require_same_shape(other, "Matrix subtraction dimensions mismatch");
        Matrix c = zeros(rows, cols);
        for (std::size_t i = 0; i < rows; i++)
            for (std::size_t j = 0; j < cols; j++)
                c.data[i][j] = data[i][j] - other.data[i][j];
        return c;
    }

    Matrix scale(double scalar) const {
        return map([scalar](double v) { return v * scalar; });
    }

    Matrix transpose() const {
        if (rows == 0) return zeros(0, 0);
        Matrix c = zeros(cols, rows);
        for (std::size_t i = 0; i < rows; i++)
            for (std::size_t j = 0; j < cols; j++) c.data[j][i] = data[i][j];
        return c;
    }

    // Matrix multiplication: (m x k)·(k x n) = (m x n).
    Matrix dot(const Matrix& other) const {
        if (cols != other.rows)
            throw std::invalid_argument("Matrix dot inner dimensions mismatch");
        Matrix c = zeros(rows, other.cols);
        for (std::size_t i = 0; i < rows; i++)
            for (std::size_t j = 0; j < other.cols; j++) {
                double acc = 0.0;
                for (std::size_t k = 0; k < cols; k++)
                    acc += data[i][k] * other.data[k][j];
                c.data[i][j] = acc;
            }
        return c;
    }

    // ─── Element access ──────────────────────────────────────────────────
    double get(std::size_t row, std::size_t col) const {
        if (row >= rows || col >= cols)
            throw std::out_of_range("index out of bounds");
        return data[row][col];
    }

    // Immutable set: returns a copy with (row, col) replaced.
    Matrix set(std::size_t row, std::size_t col, double value) const {
        if (row >= rows || col >= cols)
            throw std::out_of_range("index out of bounds");
        Matrix c = *this;
        c.data[row][col] = value;
        return c;
    }

    // ─── Reductions ──────────────────────────────────────────────────────
    double sum() const {
        double total = 0.0;
        for (const auto& row : data)
            for (double v : row) total += v;
        return total;
    }

    Matrix sum_rows() const {
        std::vector<std::vector<double>> out;
        out.reserve(rows);
        for (const auto& row : data) {
            double s = 0.0;
            for (double v : row) s += v;
            out.push_back(std::vector<double>{s});
        }
        return Matrix(std::move(out));
    }

    Matrix sum_cols() const {
        std::vector<double> sums(cols, 0.0);
        for (std::size_t i = 0; i < rows; i++)
            for (std::size_t j = 0; j < cols; j++) sums[j] += data[i][j];
        return new_1d(std::move(sums));
    }

    double mean() const {
        std::size_t n = rows * cols;
        if (n == 0) return 0.0;
        return sum() / static_cast<double>(n);
    }

    // min/max return 0.0 for an empty matrix (Rust yields +/-inf; the crate
    // never reduces an empty matrix, and 0.0 avoids fabricating an infinity
    // without <cmath>).
    double min_val() const {
        bool seen = false;
        double best = 0.0;
        for (const auto& row : data)
            for (double v : row) {
                if (!seen || v < best) {
                    best = v;
                    seen = true;
                }
            }
        return best;
    }

    double max_val() const {
        bool seen = false;
        double best = 0.0;
        for (const auto& row : data)
            for (double v : row) {
                if (!seen || v > best) {
                    best = v;
                    seen = true;
                }
            }
        return best;
    }

    std::pair<std::size_t, std::size_t> argmin() const {
        std::pair<std::size_t, std::size_t> pos(0, 0);
        bool seen = false;
        double best = 0.0;
        for (std::size_t i = 0; i < rows; i++)
            for (std::size_t j = 0; j < cols; j++) {
                double v = data[i][j];
                if (!seen || v < best) {
                    best = v;
                    pos = {i, j};
                    seen = true;
                }
            }
        return pos;
    }

    std::pair<std::size_t, std::size_t> argmax() const {
        std::pair<std::size_t, std::size_t> pos(0, 0);
        bool seen = false;
        double best = 0.0;
        for (std::size_t i = 0; i < rows; i++)
            for (std::size_t j = 0; j < cols; j++) {
                double v = data[i][j];
                if (!seen || v > best) {
                    best = v;
                    pos = {i, j};
                    seen = true;
                }
            }
        return pos;
    }

    // ─── Element-wise math ───────────────────────────────────────────────
    Matrix map(const std::function<double(double)>& f) const {
        std::vector<std::vector<double>> out;
        out.reserve(rows);
        for (const auto& row : data) {
            std::vector<double> r;
            r.reserve(row.size());
            for (double v : row) r.push_back(f(v));
            out.push_back(std::move(r));
        }
        return Matrix(std::move(out));
    }

    Matrix sqrt() const { return map(detail::d_sqrt); }
    Matrix abs_val() const { return map(detail::d_abs); }
    Matrix pow_val(double exp) const {
        return map([exp](double v) { return detail::d_pow(v, exp); });
    }

    // ─── Shape operations ────────────────────────────────────────────────
    Matrix flatten() const {
        std::vector<double> flat;
        flat.reserve(rows * cols);
        for (std::size_t i = 0; i < rows; i++)
            for (std::size_t j = 0; j < cols; j++) flat.push_back(data[i][j]);
        return new_1d(std::move(flat));
    }

    Matrix reshape(std::size_t r, std::size_t c) const {
        // Guard the size_t product against overflow BEFORE the element-count
        // comparison (the C sibling does the same). Otherwise an overflowed
        // r*c could wrap to alias the true count, slip past the check, and
        // drive the iterator arithmetic below out of bounds.
        if (r != 0 && c > static_cast<std::size_t>(-1) / r)
            throw std::invalid_argument("reshape dimensions overflow");
        if (r * c != rows * cols)
            throw std::invalid_argument("reshape size mismatch");
        Matrix flat = flatten();
        using diff_t = std::vector<double>::difference_type;
        std::vector<std::vector<double>> out;
        out.reserve(r);
        for (std::size_t i = 0; i < r; i++) {
            std::vector<double> row(
                flat.data[0].begin() + static_cast<diff_t>(i * c),
                flat.data[0].begin() + static_cast<diff_t>((i + 1) * c));
            out.push_back(std::move(row));
        }
        return Matrix(std::move(out));
    }

    Matrix row(std::size_t i) const {
        if (i >= rows) throw std::out_of_range("row index out of bounds");
        return new_1d(data[i]);
    }

    Matrix col(std::size_t j) const {
        if (j >= cols) throw std::out_of_range("column index out of bounds");
        std::vector<std::vector<double>> out;
        out.reserve(rows);
        for (const auto& r : data) out.push_back(std::vector<double>{r[j]});
        return Matrix(std::move(out));
    }

    // Half-open sub-matrix [r0:r1, c0:c1).
    Matrix slice(std::size_t r0, std::size_t r1, std::size_t c0,
                 std::size_t c1) const {
        if (r0 >= r1 || c0 >= c1 || r1 > rows || c1 > cols)
            throw std::out_of_range("invalid slice range");
        using diff_t = std::vector<double>::difference_type;
        std::vector<std::vector<double>> out;
        out.reserve(r1 - r0);
        for (std::size_t i = r0; i < r1; i++) {
            std::vector<double> row(data[i].begin() + static_cast<diff_t>(c0),
                                    data[i].begin() + static_cast<diff_t>(c1));
            out.push_back(std::move(row));
        }
        return Matrix(std::move(out));
    }

    // ─── Comparison ──────────────────────────────────────────────────────
    bool equals(const Matrix& other) const {
        if (rows != other.rows || cols != other.cols) return false;
        for (std::size_t i = 0; i < rows; i++)
            for (std::size_t j = 0; j < cols; j++)
                if (data[i][j] != other.data[i][j]) return false;
        return true;
    }

    bool close(const Matrix& other, double tolerance) const {
        if (rows != other.rows || cols != other.cols) return false;
        for (std::size_t i = 0; i < rows; i++)
            for (std::size_t j = 0; j < cols; j++)
                if (detail::d_abs(data[i][j] - other.data[i][j]) > tolerance)
                    return false;
        return true;
    }

private:
    void require_same_shape(const Matrix& other, const char* msg) const {
        if (rows != other.rows || cols != other.cols)
            throw std::invalid_argument(msg);
    }
};

}  // namespace matrix
}  // namespace ca

#endif  // CA_MATRIX_HPP
