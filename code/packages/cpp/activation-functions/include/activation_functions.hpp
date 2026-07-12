// activation_functions.hpp — neural-network activation functions and their
// derivatives, header-only in pure ISO C++17 (namespace ca::activation_functions).
// A faithful port of the Rust `activation-functions` crate.
// ===========================================================================
//
// The activation function is the nonlinearity a neuron applies to its weighted
// input; its derivative is what backpropagation multiplies through. This library
// provides the classic set, each as a pair (function, derivative):
//
//   linear      f(x) = x                        f'(x) = 1
//   sigmoid     f(x) = 1 / (1 + e^-x)           f'(x) = f(x)(1 - f(x))
//   relu        f(x) = max(0, x)                f'(x) = x > 0 ? 1 : 0
//   leaky_relu  f(x) = x>0 ? x : 0.01x          f'(x) = x > 0 ? 1 : 0.01
//   tanh        f(x) = tanh(x)                  f'(x) = 1 - tanh(x)^2
//   softplus    f(x) = ln(1 + e^x)              f'(x) = sigmoid(x)
//
// NO libm / <cmath>. The transcendental helpers (e^x, tanh, ln(1+x)) are
// computed from scratch — range-reduced Taylor/Newton series — so the header
// pulls in no math library. Results match std::exp / std::tanh / std::log1p to
// within about 1e-12 (the tolerance the Rust crate's own tests use).
//
// All functions are total; there is nothing to fail, exactly like Rust.
//
// PORTABILITY. Pure ISO C++17, no <cmath>, no compiler extensions. Compiles
// clean under GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
// warnings-as-errors.
#ifndef CA_ACTIVATION_FUNCTIONS_HPP
#define CA_ACTIVATION_FUNCTIONS_HPP

namespace ca {
namespace activation_functions {

// The negative-side slope of leaky ReLU (matches the Rust constant).
inline constexpr double LEAKY_RELU_SLOPE = 0.01;

namespace detail {

inline double d_abs(double x) { return x < 0.0 ? -x : x; }

// 2^k for an integer k, by exact binary exponentiation (powers of two).
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

// e^x via Cody-Waite range reduction: x = k*ln2 + r, |r| <= ln2/2, so
// e^x = 2^k * e^r with e^r a fast-converging Taylor series.
inline double d_exp(double x) {
    if (x != x) return x;  // NaN propagates (and stays out of the int cast)
    if (x == 0.0) return 1.0;
    if (x > 709.782712893384) return 1.7976931348623157e308;  // overflow (+inf)
    // Below this e^x underflows to 0; this also bounds |x| before the (int)
    // cast (softplus may pass e.g. d_exp(-1e300), an out-of-int-range value).
    if (x < -745.13321910194) return 0.0;

    constexpr double INV_LN2 = 1.4426950408889634;
    constexpr double C1 = 0.693359375;             // exact; C1 + C2 == ln2
    constexpr double C2 = -2.1219444005469058277e-4;

    double kf = x * INV_LN2;
    int k = static_cast<int>(kf >= 0.0 ? kf + 0.5 : kf - 0.5);  // round nearest
    double r = (x - static_cast<double>(k) * C1) - static_cast<double>(k) * C2;

    double term = 1.0;
    double sum = 1.0;
    for (int n = 1; n <= 17; n++) {
        term *= r / static_cast<double>(n);
        sum += term;
    }
    return sum * pow2i(k);
}

// ln(1 + y) for y >= 0 via 2*atanh(u), u = y/(2+y) (no near-1 cancellation).
inline double d_ln1p(double y) {
    double u = y / (2.0 + y);
    double u2 = u * u;
    double term = u;
    double sum = u;
    for (int n = 1; n <= 60; n++) {
        term *= u2;
        double add = term / static_cast<double>(2 * n + 1);
        sum += add;
        if (d_abs(add) < 1e-18) break;
    }
    return 2.0 * sum;
}

// tanh(x) = (1 - e^-2|x|) / (1 + e^-2|x|), odd-extended and saturated.
inline double d_tanh(double x) {
    if (x == 0.0) return 0.0;
    bool neg = x < 0.0;
    double ax = neg ? -x : x;
    if (ax > 20.0) return neg ? -1.0 : 1.0;
    double em2 = d_exp(-2.0 * ax);
    double t = (1.0 - em2) / (1.0 + em2);
    return neg ? -t : t;
}

}  // namespace detail

// ── Activations ──────────────────────────────────────────────────────────────

inline double linear(double x) { return x; }
inline double linear_derivative(double) { return 1.0; }

inline double sigmoid(double x) {
    if (x < -709.0) return 0.0;
    if (x > 709.0) return 1.0;
    return 1.0 / (1.0 + detail::d_exp(-x));
}
inline double sigmoid_derivative(double x) {
    double s = sigmoid(x);
    return s * (1.0 - s);
}

inline double relu(double x) { return x > 0.0 ? x : 0.0; }
inline double relu_derivative(double x) { return x > 0.0 ? 1.0 : 0.0; }

inline double leaky_relu(double x) {
    return x > 0.0 ? x : LEAKY_RELU_SLOPE * x;
}
inline double leaky_relu_derivative(double x) {
    return x > 0.0 ? 1.0 : LEAKY_RELU_SLOPE;
}

inline double tanh(double x) { return detail::d_tanh(x); }
inline double tanh_derivative(double x) {
    double t = detail::d_tanh(x);
    return 1.0 - t * t;
}

inline double softplus(double x) {
    double max0 = x > 0.0 ? x : 0.0;
    return detail::d_ln1p(detail::d_exp(-detail::d_abs(x))) + max0;
}
inline double softplus_derivative(double x) { return sigmoid(x); }

}  // namespace activation_functions
}  // namespace ca

#endif  // CA_ACTIVATION_FUNCTIONS_HPP
