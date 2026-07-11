// polynomial.hpp — coefficient-array polynomial arithmetic over doubles, in pure
// ISO C++17 (header-only). A faithful port of the Rust `polynomial` crate, in
// namespace `ca::polynomial`.
// ===========================================================================
//
// A polynomial a0 + a1*x + a2*x^2 + ... is a std::vector<double> in little-endian
// order (`p[i]` is the coefficient of x^i); the zero polynomial is the empty
// vector.
//
//   normalize / degree            — canonical form and degree
//   add / subtract / multiply     — arithmetic
//   divmod / divide / modulo      — long division (throws on a zero divisor)
//   evaluate                      — Horner evaluation
//   gcd                           — Euclidean GCD
//
// A coefficient with magnitude <= DBL_EPSILON*1e6 is treated as zero.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. No extensions.
#ifndef POLYNOMIAL_HPP
#define POLYNOMIAL_HPP

#include <cfloat>
#include <cstddef>
#include <stdexcept>
#include <utility>
#include <vector>

namespace ca {
namespace polynomial {

using poly = std::vector<double>;

namespace detail {
constexpr double ZERO_THRESHOLD = DBL_EPSILON * 1e6;
inline double dabs(double x) { return x < 0.0 ? -x : x; }
} // namespace detail

// Strip trailing (high-degree) near-zero coefficients.
inline poly normalize(const poly &p) {
    std::size_t len = p.size();
    while (len > 0 && detail::dabs(p[len - 1]) <= detail::ZERO_THRESHOLD) {
        len--;
    }
    return poly(p.begin(), p.begin() + static_cast<std::ptrdiff_t>(len));
}

// Index of the highest non-zero coefficient (0 for the zero polynomial).
inline std::size_t degree(const poly &p) {
    poly n = normalize(p);
    return n.empty() ? 0 : n.size() - 1;
}

inline poly zero() { return poly{0.0}; }
inline poly one() { return poly{1.0}; }

inline poly add(const poly &a, const poly &b) {
    std::size_t len = a.size() > b.size() ? a.size() : b.size();
    poly result(len, 0.0);
    for (std::size_t i = 0; i < len; i++) {
        double ai = i < a.size() ? a[i] : 0.0;
        double bi = i < b.size() ? b[i] : 0.0;
        result[i] = ai + bi;
    }
    return normalize(result);
}

inline poly subtract(const poly &a, const poly &b) {
    std::size_t len = a.size() > b.size() ? a.size() : b.size();
    poly result(len, 0.0);
    for (std::size_t i = 0; i < len; i++) {
        double ai = i < a.size() ? a[i] : 0.0;
        double bi = i < b.size() ? b[i] : 0.0;
        result[i] = ai - bi;
    }
    return normalize(result);
}

inline poly multiply(const poly &a, const poly &b) {
    if (a.empty() || b.empty()) {
        return poly{};
    }
    poly result(a.size() + b.size() - 1, 0.0);
    for (std::size_t i = 0; i < a.size(); i++) {
        for (std::size_t j = 0; j < b.size(); j++) {
            result[i + j] += a[i] * b[j];
        }
    }
    return normalize(result);
}

// Long division: returns (quotient, remainder). Throws on a zero divisor.
inline std::pair<poly, poly> divmod(const poly &dividend, const poly &divisor) {
    poly nb = normalize(divisor);
    if (nb.empty()) {
        throw std::invalid_argument("polynomial division by zero");
    }
    poly na = normalize(dividend);
    if (na.size() < nb.size()) {
        return {poly{}, na};
    }
    std::size_t deg_a = na.size() - 1;
    std::size_t deg_b = nb.size() - 1;
    poly rem = na;
    poly quot(deg_a - deg_b + 1, 0.0);
    double lead_b = nb[deg_b];
    std::size_t deg_rem = deg_a;
    for (;;) {
        if (deg_rem < deg_b) {
            break;
        }
        double lead_rem = rem[deg_rem];
        double coeff = lead_rem / lead_b;
        std::size_t power = deg_rem - deg_b;
        quot[power] = coeff;
        for (std::size_t j = 0; j <= deg_b; j++) {
            rem[power + j] -= coeff * nb[j];
        }
        std::ptrdiff_t sd = static_cast<std::ptrdiff_t>(deg_rem) - 1;
        while (sd >= 0 &&
               detail::dabs(rem[static_cast<std::size_t>(sd)]) <=
                   detail::ZERO_THRESHOLD) {
            sd--;
        }
        if (sd < 0) {
            break;
        }
        deg_rem = static_cast<std::size_t>(sd);
    }
    return {normalize(quot), normalize(rem)};
}

inline poly divide(const poly &a, const poly &b) { return divmod(a, b).first; }
inline poly modulo(const poly &a, const poly &b) { return divmod(a, b).second; }

// Value of the polynomial at x (Horner's method).
inline double evaluate(const poly &p, double x) {
    poly n = normalize(p);
    double acc = 0.0;
    for (std::size_t i = n.size(); i > 0; i--) {
        acc = acc * x + n[i - 1];
    }
    return acc;
}

// Euclidean GCD.
inline poly gcd(const poly &a, const poly &b) {
    poly u = normalize(a);
    poly v = normalize(b);
    while (!v.empty()) {
        poly r = modulo(u, v);
        u = v;
        v = r;
    }
    return normalize(u);
}

} // namespace polynomial
} // namespace ca

#endif // POLYNOMIAL_HPP
