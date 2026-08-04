// feature_normalization.hpp — column-wise feature scaling, header-only in pure
// ISO C++17 (namespace ca::feature_normalization). A faithful port of the Rust
// `feature-normalization` crate.
// ===========================================================================
//
// Two classic scalers that put a data matrix's columns on comparable scales:
//
//   StandardScaler (z-score)    z = (x - mean) / stddev      per column
//   MinMaxScaler   (unit range) u = (x - min)  / (max - min) per column
//
// Each is a two-step fit/transform: `fit_*` learns per-column statistics from a
// training matrix; `transform_*` applies them. A column with zero spread
// (stddev == 0, or max == min) maps to 0.0, exactly as in the Rust crate.
//
// Matrices are `std::vector<std::vector<double>>` — the same shape as the Rust
// crate — so ragged rows are representable and validated.
//
// DIVERGENCE FROM RUST. Rust returns `Result<_, &'static str>`; this port
// throws `std::invalid_argument` carrying the same message. The population
// standard deviation (divide by n, not n-1) matches the Rust crate. `sqrt` is
// computed by Newton's method (no <cmath>).
//
// PORTABILITY. Pure ISO C++17 — no <cmath>, no compiler extensions. Compiles
// clean under GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
// warnings-as-errors.
#ifndef CA_FEATURE_NORMALIZATION_HPP
#define CA_FEATURE_NORMALIZATION_HPP

#include <cstddef>
#include <stdexcept>
#include <utility>
#include <vector>

namespace ca {
namespace feature_normalization {

using Matrix = std::vector<std::vector<double>>;

// Per-column mean and (population) standard deviation.
struct StandardScaler {
    std::vector<double> means;
    std::vector<double> standard_deviations;
};

// Per-column minimum and maximum.
struct MinMaxScaler {
    std::vector<double> minimums;
    std::vector<double> maximums;
};

namespace detail {

// Square root via Newton's method — no <cmath>. Callers pass a non-negative
// variance, so no domain check is needed.
inline double newton_sqrt(double x) {
    if (x <= 0.0) return 0.0;
    double guess = x >= 1.0 ? x : 1.0;
    for (int i = 0; i < 60; i++) {
        double next = (guess + x / guess) / 2.0;
        double diff = next - guess;
        if (diff < 0.0) diff = -diff;
        if (diff < 1e-15 * guess + 1e-300) return next;
        guess = next;
    }
    return guess;
}

// Validate a matrix: at least one row and column, and all rows equal width.
// Returns the width, or throws std::invalid_argument (mirroring the Rust Err).
inline std::size_t validate_matrix(const Matrix& rows) {
    if (rows.empty() || rows[0].empty()) {
        throw std::invalid_argument(
            "matrix must have at least one row and one column");
    }
    std::size_t width = rows[0].size();
    for (const auto& row : rows) {
        if (row.size() != width) {
            throw std::invalid_argument(
                "all rows must have the same number of columns");
        }
    }
    return width;
}

}  // namespace detail

// ── StandardScaler ───────────────────────────────────────────────────────────

inline StandardScaler fit_standard_scaler(const Matrix& rows) {
    std::size_t width = detail::validate_matrix(rows);
    std::vector<double> means(width, 0.0);
    for (const auto& row : rows)
        for (std::size_t c = 0; c < width; c++) means[c] += row[c];
    for (auto& m : means) m /= static_cast<double>(rows.size());

    std::vector<double> sds(width, 0.0);
    for (const auto& row : rows)
        for (std::size_t c = 0; c < width; c++) {
            double diff = row[c] - means[c];
            sds[c] += diff * diff;
        }
    for (auto& sd : sds)
        sd = detail::newton_sqrt(sd / static_cast<double>(rows.size()));

    return StandardScaler{means, sds};
}

inline Matrix transform_standard(const Matrix& rows,
                                 const StandardScaler& scaler) {
    std::size_t width = detail::validate_matrix(rows);
    if (width != scaler.means.size() ||
        width != scaler.standard_deviations.size()) {
        throw std::invalid_argument("matrix width must match scaler width");
    }
    Matrix out;
    out.reserve(rows.size());
    for (const auto& row : rows) {
        std::vector<double> scaled(width);
        for (std::size_t c = 0; c < width; c++) {
            // A column with no spread maps to 0 (avoids divide-by-zero).
            scaled[c] = scaler.standard_deviations[c] == 0.0
                            ? 0.0
                            : (row[c] - scaler.means[c]) /
                                  scaler.standard_deviations[c];
        }
        out.push_back(std::move(scaled));
    }
    return out;
}

// ── MinMaxScaler ─────────────────────────────────────────────────────────────

inline MinMaxScaler fit_min_max_scaler(const Matrix& rows) {
    std::size_t width = detail::validate_matrix(rows);
    std::vector<double> minimums = rows[0];
    std::vector<double> maximums = rows[0];
    for (std::size_t r = 1; r < rows.size(); r++) {
        for (std::size_t c = 0; c < width; c++) {
            double v = rows[r][c];
            if (v < minimums[c]) minimums[c] = v;
            if (v > maximums[c]) maximums[c] = v;
        }
    }
    return MinMaxScaler{minimums, maximums};
}

inline Matrix transform_min_max(const Matrix& rows,
                                const MinMaxScaler& scaler) {
    std::size_t width = detail::validate_matrix(rows);
    if (width != scaler.minimums.size() || width != scaler.maximums.size()) {
        throw std::invalid_argument("matrix width must match scaler width");
    }
    Matrix out;
    out.reserve(rows.size());
    for (const auto& row : rows) {
        std::vector<double> scaled(width);
        for (std::size_t c = 0; c < width; c++) {
            double span = scaler.maximums[c] - scaler.minimums[c];
            scaled[c] =
                span == 0.0 ? 0.0 : (row[c] - scaler.minimums[c]) / span;
        }
        out.push_back(std::move(scaled));
    }
    return out;
}

}  // namespace feature_normalization
}  // namespace ca

#endif  // CA_FEATURE_NORMALIZATION_HPP
