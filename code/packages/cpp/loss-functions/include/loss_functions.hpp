// loss_functions.hpp — machine-learning loss functions and their gradients,
// header-only in pure ISO C++17 (namespace ca::loss_functions). A faithful port
// of the Rust `loss-functions` crate.
// ===========================================================================
//
// A loss function scores how far a model's predictions `y_pred` are from the
// ground truth `y_true`; its derivative (gradient) is what training descends.
// This library provides the four classics, each as a scalar loss and a
// per-element gradient:
//
//   MSE  mean squared error        (1/n) Σ (t - p)^2
//   MAE  mean absolute error       (1/n) Σ |t - p|
//   BCE  binary cross-entropy      -(1/n) Σ [t·ln p + (1-t)·ln(1-p)]
//   CCE  categorical cross-entropy -(1/n) Σ [t·ln p]
//
// Cross-entropy clamps each prediction to [EPSILON, 1-EPSILON] before the log
// (EPSILON = 1e-7, matching the Rust crate), so ln(0) = -inf never occurs.
//
// NO libm / <cmath>: the one transcendental (ln) is computed from scratch.
//
// DIVERGENCE FROM RUST. Rust returns `Result<_, &'static str>`; this port throws
// `std::invalid_argument` when the two vectors differ in length or are empty.
// The scalar losses return `double`; the gradients return `std::vector<double>`.
//
// PORTABILITY. Pure ISO C++17, no <cmath>, no compiler extensions.
#ifndef CA_LOSS_FUNCTIONS_HPP
#define CA_LOSS_FUNCTIONS_HPP

#include <cstddef>
#include <stdexcept>
#include <vector>

namespace ca {
namespace loss_functions {

// Clamp bound for predictions before a logarithm (matches the Rust constant).
inline constexpr double EPSILON = 1e-7;

namespace detail {

inline double d_abs(double x) { return x < 0.0 ? -x : x; }

inline double d_clamp(double x, double lo, double hi) {
    if (x < lo) return lo;
    if (x > hi) return hi;
    return x;
}

// Natural log for x > 0: reduce x = m*2^e (m in [1,2)), ln(x) = e*ln2 + ln(m)
// with ln(m) = 2*atanh((m-1)/(m+1)). Callers only pass x in [EPSILON, 1].
inline double d_ln(double x) {
    int e = 0;
    double m = x;
    while (m < 1.0) {
        m *= 2.0;
        e--;
    }
    while (m >= 2.0) {
        m *= 0.5;
        e++;
    }
    double u = (m - 1.0) / (m + 1.0);
    double u2 = u * u;
    double term = u;
    double sum = u;
    for (int n = 1; n <= 40; n++) {
        term *= u2;
        double add = term / static_cast<double>(2 * n + 1);
        sum += add;
        if (d_abs(add) < 1e-17) break;
    }
    constexpr double LN2 = 0.6931471805599453;
    return static_cast<double>(e) * LN2 + 2.0 * sum;
}

// Equal, non-zero lengths — mirrors the Rust check; throws otherwise.
inline void check_lengths(const std::vector<double>& a,
                          const std::vector<double>& b) {
    if (a.size() != b.size() || a.empty()) {
        throw std::invalid_argument(
            "Slices must have the same non-zero length");
    }
}

}  // namespace detail

// ── Scalar losses ────────────────────────────────────────────────────────────

inline double mse(const std::vector<double>& y_true,
                  const std::vector<double>& y_pred) {
    detail::check_lengths(y_true, y_pred);
    double sum = 0.0;
    for (std::size_t i = 0; i < y_true.size(); i++) {
        double diff = y_true[i] - y_pred[i];
        sum += diff * diff;
    }
    return sum / static_cast<double>(y_true.size());
}

inline double mae(const std::vector<double>& y_true,
                  const std::vector<double>& y_pred) {
    detail::check_lengths(y_true, y_pred);
    double sum = 0.0;
    for (std::size_t i = 0; i < y_true.size(); i++)
        sum += detail::d_abs(y_true[i] - y_pred[i]);
    return sum / static_cast<double>(y_true.size());
}

inline double bce(const std::vector<double>& y_true,
                  const std::vector<double>& y_pred) {
    detail::check_lengths(y_true, y_pred);
    double sum = 0.0;
    for (std::size_t i = 0; i < y_true.size(); i++) {
        double p = detail::d_clamp(y_pred[i], EPSILON, 1.0 - EPSILON);
        sum += y_true[i] * detail::d_ln(p) +
               (1.0 - y_true[i]) * detail::d_ln(1.0 - p);
    }
    return -sum / static_cast<double>(y_true.size());
}

inline double cce(const std::vector<double>& y_true,
                  const std::vector<double>& y_pred) {
    detail::check_lengths(y_true, y_pred);
    double sum = 0.0;
    for (std::size_t i = 0; i < y_true.size(); i++) {
        double p = detail::d_clamp(y_pred[i], EPSILON, 1.0 - EPSILON);
        sum += y_true[i] * detail::d_ln(p);
    }
    return -sum / static_cast<double>(y_true.size());
}

// ── Gradients ────────────────────────────────────────────────────────────────

inline std::vector<double> mse_derivative(const std::vector<double>& y_true,
                                          const std::vector<double>& y_pred) {
    detail::check_lengths(y_true, y_pred);
    double n = static_cast<double>(y_true.size());
    std::vector<double> res;
    res.reserve(y_true.size());
    for (std::size_t i = 0; i < y_true.size(); i++)
        res.push_back((2.0 / n) * (y_pred[i] - y_true[i]));
    return res;
}

inline std::vector<double> mae_derivative(const std::vector<double>& y_true,
                                          const std::vector<double>& y_pred) {
    detail::check_lengths(y_true, y_pred);
    double n = static_cast<double>(y_true.size());
    std::vector<double> res;
    res.reserve(y_true.size());
    for (std::size_t i = 0; i < y_true.size(); i++) {
        if (y_pred[i] > y_true[i])
            res.push_back(1.0 / n);
        else if (y_pred[i] < y_true[i])
            res.push_back(-1.0 / n);
        else
            res.push_back(0.0);
    }
    return res;
}

inline std::vector<double> bce_derivative(const std::vector<double>& y_true,
                                          const std::vector<double>& y_pred) {
    detail::check_lengths(y_true, y_pred);
    double n = static_cast<double>(y_true.size());
    std::vector<double> res;
    res.reserve(y_true.size());
    for (std::size_t i = 0; i < y_true.size(); i++) {
        double p = detail::d_clamp(y_pred[i], EPSILON, 1.0 - EPSILON);
        res.push_back((1.0 / n) * ((p - y_true[i]) / (p * (1.0 - p))));
    }
    return res;
}

inline std::vector<double> cce_derivative(const std::vector<double>& y_true,
                                          const std::vector<double>& y_pred) {
    detail::check_lengths(y_true, y_pred);
    double n = static_cast<double>(y_true.size());
    std::vector<double> res;
    res.reserve(y_true.size());
    for (std::size_t i = 0; i < y_true.size(); i++) {
        double p = detail::d_clamp(y_pred[i], EPSILON, 1.0 - EPSILON);
        res.push_back((-1.0 / n) * (y_true[i] / p));
    }
    return res;
}

}  // namespace loss_functions
}  // namespace ca

#endif  // CA_LOSS_FUNCTIONS_HPP
