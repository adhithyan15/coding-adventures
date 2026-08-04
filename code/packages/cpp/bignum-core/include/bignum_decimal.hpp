// bignum_decimal.hpp — an exact base-10 number (BigDecimal), built on
// BigInteger, in pure ISO C++17, header-only, in namespace ca. A faithful port
// of the `decimal` module of the Rust `bignum-core` crate.
// ===========================================================================
//
// WHAT IT IS. A BigDecimal is `mantissa × 10^(-scale)`: an arbitrary-precision
// integer mantissa (a ca::BigInteger, carrying the sign) scaled by a power of
// ten. `123.45` is `(12345, 2)`; `100` is `(1, -2)`. Everything is held in
// CANONICAL FORM — the mantissa never ends in a `0` digit (unless the value is
// exactly zero, pinned to `(0, 0)`) — so `==` and the total order `<` compare by
// value, and equal values hash and print identically.
//
// WHY. `double` cannot represent `0.1`; money, tax, and dosing need exact
// base-10 arithmetic. `+ - *` here are EXACT (they never round); only division
// — the one base-10 operation that need not terminate (10/3) — rounds, and only
// then do you say to how many places and how (a RoundingMode). `to_f64` is the
// single labelled lossy exit.
//
// ERRORS. Value semantics throughout (copy/move like an int). Where Rust returns
// `Option`/`Result`, this port offers BOTH: a throwing form (`from_parts`
// throws std::out_of_range past the ceiling; `div_round` throws
// std::domain_error on a zero divisor; `parse` throws ParseDecimalError) and a
// non-throwing `checked_*` / `try_*` form returning std::optional.
//
// PORTABILITY. Pure ISO C++17 — no <cmath>/libm (float export goes through
// std::strtod), no compiler extensions. Compiles clean under GCC, Clang, and
// MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
#ifndef CA_BIGNUM_DECIMAL_HPP
#define CA_BIGNUM_DECIMAL_HPP

#include <cerrno>
#include <climits>
#include <cstdint>
#include <cstdlib>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>

#include "bignum_core.hpp"

namespace ca {

// How to round when a value cannot be represented exactly at a requested scale.
// The Half* modes decide only the exact-halfway (…5) case.
enum class RoundingMode {
    Down,     // toward zero (truncate)
    Up,       // away from zero
    Floor,    // toward -infinity
    Ceiling,  // toward +infinity
    HalfUp,   // nearest; ties away from zero
    HalfDown, // nearest; ties toward zero
    HalfEven  // nearest; ties to even ("banker's")
};

// Thrown by BigDecimal::parse on a malformed literal.
class ParseDecimalError : public std::runtime_error {
public:
    enum class Kind { Empty, InvalidDigit, MalformedShape, ExponentOverflow };

    explicit ParseDecimalError(Kind k)
        : std::runtime_error(message(k)), kind_(k) {}

    Kind kind() const { return kind_; }

private:
    Kind kind_;
    static std::string message(Kind k) {
        switch (k) {
            case Kind::Empty: return "empty decimal literal";
            case Kind::InvalidDigit: return "invalid character in decimal literal";
            case Kind::MalformedShape: return "malformed decimal literal (stray '.' or 'e')";
            case Kind::ExponentOverflow: return "exponent out of range";
        }
        return "decimal parse error";
    }
};

namespace decimal_detail {

// The unsigned magnitude of an i64 without the UB of negating INT64_MIN.
inline std::uint64_t abs_u64(std::int64_t s) {
    return s < 0 ? (0u - static_cast<std::uint64_t>(s))
                 : static_cast<std::uint64_t>(s);
}

// Checked i64 arithmetic (C++17 has no i128 guarantee; test overflow by hand).
inline bool add_checked(std::int64_t a, std::int64_t b, std::int64_t& out) {
    if (b > 0 && a > INT64_MAX - b) return false;
    if (b < 0 && a < INT64_MIN - b) return false;
    out = a + b;
    return true;
}
inline bool sub_checked(std::int64_t a, std::int64_t b, std::int64_t& out) {
    if (b < 0) {
        if (a > INT64_MAX + b) return false;
    } else {
        if (a < INT64_MIN + b) return false;
    }
    out = a - b;
    return true;
}
inline bool mul_checked(std::int64_t a, std::int64_t b, std::int64_t& out) {
    if (a == 0 || b == 0) {
        out = 0;
        return true;
    }
    // Test BEFORE multiplying: forming an overflowing product is signed-overflow
    // UB, which an optimizer may exploit to delete a divide-back check after it.
    if (a == -1 && b == INT64_MIN) return false;
    if (b == -1 && a == INT64_MIN) return false;
    if (a > 0) {
        if (b > 0 ? a > INT64_MAX / b : b < INT64_MIN / a) return false;
    } else { // a < 0
        if (b > 0 ? a < INT64_MIN / b : b < INT64_MAX / a) return false;
    }
    out = a * b;
    return true;
}

// 10^n as a BigInteger.
inline BigInteger ten_pow(std::uint32_t n) {
    return BigInteger::from_i64(10).pow(n);
}

// Round the exact quotient n/d (d != 0) to the nearest integer under `mode`.
// Decided from the truncating quotient q and remainder r of the magnitudes:
// |n| = q·|d| + r with 0 ≤ r < |d|. The result's sign is sign(n·d).
inline BigInteger round_div(const BigInteger& n, const BigInteger& d,
                            RoundingMode mode) {
    int sign = n.signum() * d.signum();
    if (sign == 0) return BigInteger::zero(); // n == 0
    BigInteger na = n.abs();
    BigInteger da = d.abs();
    std::pair<BigInteger, BigInteger> qr = na.div_rem(da);
    const BigInteger& q = qr.first;
    const BigInteger& r = qr.second;
    if (r.is_zero()) return sign < 0 ? q.neg() : q; // exact

    BigInteger two = BigInteger::from_i64(2);
    int half_cmp = (r * two).cmp(da); // r vs d/2: <0 below, 0 at, >0 above
    bool q_is_odd = !(q % two).is_zero();

    bool round_away;
    switch (mode) {
        case RoundingMode::Down: round_away = false; break;
        case RoundingMode::Up: round_away = true; break;
        case RoundingMode::Floor: round_away = sign < 0; break;
        case RoundingMode::Ceiling: round_away = sign > 0; break;
        case RoundingMode::HalfUp: round_away = half_cmp >= 0; break;
        case RoundingMode::HalfDown: round_away = half_cmp > 0; break;
        case RoundingMode::HalfEven:
            round_away = half_cmp > 0 || (half_cmp == 0 && q_is_odd);
            break;
        default: round_away = false; break;
    }
    BigInteger magnitude = round_away ? q + BigInteger::one() : q;
    return sign < 0 ? magnitude.neg() : magnitude;
}

}  // namespace decimal_detail

class BigDecimal {
public:
    // The largest scale magnitude accepted from untrusted input (`parse`); a
    // security budget that bounds any power-of-ten materialization to under a
    // megabyte. The internal ceiling is wider so ordinary arithmetic on
    // parse-budget operands never trips it.
    static constexpr std::int64_t MAX_SCALE = 1000000;
    static constexpr std::int64_t INTERNAL_SCALE_LIMIT = 8000000;

    // ---- construction --------------------------------------------------
    BigDecimal() : mant_(BigInteger::zero()), scale_(0) {}
    static BigDecimal zero() { return BigDecimal(); }
    static BigDecimal one() { return from_i64(1); }
    static BigDecimal from_i64(std::int64_t n) {
        return from_parts(BigInteger::from_i64(n), 0);
    }
    static BigDecimal from_integer(const BigInteger& n) {
        return from_parts(n, 0);
    }

    // Build `mant × 10^(-scale)`, canonicalize, enforce the internal ceiling.
    // Throws std::out_of_range past the ceiling (Rust's panicking `from_parts`).
    static BigDecimal from_parts(BigInteger mant, std::int64_t scale) {
        std::optional<BigDecimal> d = checked_from_parts(std::move(mant), scale);
        if (!d) {
            throw std::out_of_range(
                "BigDecimal scale magnitude exceeds the internal ceiling");
        }
        return std::move(*d);
    }
    // Non-throwing form: std::nullopt if the canonical scale magnitude exceeds
    // the internal ceiling.
    static std::optional<BigDecimal> checked_from_parts(BigInteger mant,
                                                        std::int64_t scale) {
        BigDecimal d(std::move(mant), scale);
        d.normalize();
        if (decimal_detail::abs_u64(d.scale_) >
            static_cast<std::uint64_t>(INTERNAL_SCALE_LIMIT)) {
            return std::nullopt;
        }
        return d;
    }

    // ---- accessors -----------------------------------------------------
    const BigInteger& mantissa() const { return mant_; }
    std::int64_t scale() const { return scale_; }

    // ---- predicates & sign ---------------------------------------------
    bool is_zero() const { return mant_.is_zero(); }
    bool is_negative() const { return mant_.is_negative(); }
    bool is_positive() const { return mant_.is_positive(); }
    int signum() const { return mant_.signum(); }
    BigDecimal abs() const { return BigDecimal(mant_.abs(), scale_); }

    // ---- exact arithmetic ----------------------------------------------
    BigDecimal add(const BigDecimal& other) const {
        std::int64_t scale;
        std::pair<BigInteger, BigInteger> ab = aligned_mantissas(other, scale);
        return from_parts(ab.first + ab.second, scale);
    }
    BigDecimal sub(const BigDecimal& other) const {
        std::int64_t scale;
        std::pair<BigInteger, BigInteger> ab = aligned_mantissas(other, scale);
        return from_parts(ab.first - ab.second, scale);
    }
    BigDecimal mul(const BigDecimal& other) const {
        std::int64_t scale;
        if (!decimal_detail::add_checked(scale_, other.scale_, scale)) {
            throw std::overflow_error("scale overflow in multiplication");
        }
        return from_parts(mant_ * other.mant_, scale);
    }
    BigDecimal pow(std::uint32_t exp) const {
        std::int64_t scale;
        if (!decimal_detail::mul_checked(scale_, static_cast<std::int64_t>(exp),
                                         scale)) {
            throw std::overflow_error("scale overflow in pow");
        }
        return from_parts(mant_.pow(exp), scale);
    }

    // Divide, rounding to exactly `target_scale` places with `mode`. Throws
    // std::domain_error if `other` is zero.
    BigDecimal div_round(const BigDecimal& other, std::int64_t target_scale,
                         RoundingMode mode) const {
        std::optional<BigDecimal> r =
            checked_div_round(other, target_scale, mode);
        if (!r) throw std::domain_error("division by zero");
        return std::move(*r);
    }
    // Non-throwing form: std::nullopt if `other` is zero.
    std::optional<BigDecimal> checked_div_round(const BigDecimal& other,
                                               std::int64_t target_scale,
                                               RoundingMode mode) const {
        if (other.is_zero()) return std::nullopt;
        // R = round( m1 · 10^(s2 - s1 + target_scale) / m2 ). Apply the exponent
        // e = target_scale + s2 - s1 to whichever side keeps both integers.
        std::int64_t e;
        if (!decimal_detail::add_checked(target_scale, other.scale_, e) ||
            !decimal_detail::sub_checked(e, scale_, e)) {
            throw std::overflow_error("scale overflow in division");
        }
        BigInteger rounded;
        if (e >= 0) {
            rounded = decimal_detail::round_div(
                mant_ * decimal_detail::ten_pow(scale_diff_u32(e)), other.mant_,
                mode);
        } else {
            rounded = decimal_detail::round_div(
                mant_,
                other.mant_ * decimal_detail::ten_pow(scale_diff_u32(-e)), mode);
        }
        return from_parts(std::move(rounded), target_scale);
    }

    // Round to `target_scale` places with `mode`. Increasing the scale is exact.
    BigDecimal round_to_scale(std::int64_t target_scale,
                              RoundingMode mode) const {
        if (target_scale >= scale_) return *this;
        std::int64_t drop;
        if (!decimal_detail::sub_checked(scale_, target_scale, drop)) {
            throw std::overflow_error("scale overflow in round_to_scale");
        }
        BigInteger rounded = decimal_detail::round_div(
            mant_, decimal_detail::ten_pow(scale_diff_u32(drop)), mode);
        return from_parts(std::move(rounded), target_scale);
    }

    // ---- ordering ------------------------------------------------------
    int cmp(const BigDecimal& other) const {
        std::int64_t scale;
        std::pair<BigInteger, BigInteger> ab = aligned_mantissas(other, scale);
        return ab.first.cmp(ab.second);
    }
    bool operator==(const BigDecimal& o) const { return cmp(o) == 0; }
    bool operator!=(const BigDecimal& o) const { return cmp(o) != 0; }
    bool operator<(const BigDecimal& o) const { return cmp(o) < 0; }
    bool operator>(const BigDecimal& o) const { return cmp(o) > 0; }
    bool operator<=(const BigDecimal& o) const { return cmp(o) <= 0; }
    bool operator>=(const BigDecimal& o) const { return cmp(o) >= 0; }

    // ---- operator overloads --------------------------------------------
    BigDecimal operator+(const BigDecimal& o) const { return add(o); }
    BigDecimal operator-(const BigDecimal& o) const { return sub(o); }
    BigDecimal operator*(const BigDecimal& o) const { return mul(o); }
    BigDecimal operator-() const { return BigDecimal(mant_.neg(), scale_); }

    // ---- formatting ----------------------------------------------------
    // Plain decimal notation, never scientific: "100", "1.23", "0.001", "-0.5".
    std::string to_string() const {
        if (mant_.is_zero()) return "0";
        bool neg = mant_.is_negative();
        std::string digits = mant_.abs().to_string(); // base-10, no sign
        std::size_t dlen = digits.size();
        std::string out;
        if (scale_ <= 0) {
            // Whole number with |scale| trailing zeros appended.
            out = digits;
            out.append(static_cast<std::size_t>(decimal_detail::abs_u64(scale_)),
                       '0');
        } else {
            std::size_t s = static_cast<std::size_t>(scale_);
            if (dlen > s) {
                out = digits.substr(0, dlen - s) + "." + digits.substr(dlen - s);
            } else {
                out = "0.";
                out.append(s - dlen, '0');
                out += digits;
            }
        }
        return neg ? "-" + out : out;
    }

    // A lossy narrowing to the nearest double, through the value's own decimal
    // string and the correctly-rounded std::strtod. Out-of-range magnitudes
    // saturate to ±inf / 0 exactly as strtod does.
    double to_f64() const {
        std::string s = to_string();
        return std::strtod(s.c_str(), nullptr);
    }

    // ---- parsing -------------------------------------------------------
    // Throwing form: parses plain and scientific notation, enforcing the strict
    // MAX_SCALE budget on the stored scale.
    static BigDecimal parse(const std::string& s) {
        std::optional<BigDecimal> d;
        ParseDecimalError::Kind err = ParseDecimalError::Kind::Empty;
        if (!parse_impl(s, d, err)) throw ParseDecimalError(err);
        return std::move(*d);
    }
    // Non-throwing form: std::nullopt on any malformed or out-of-budget input.
    static std::optional<BigDecimal> try_parse(const std::string& s) {
        std::optional<BigDecimal> d;
        ParseDecimalError::Kind err = ParseDecimalError::Kind::Empty;
        if (!parse_impl(s, d, err)) return std::nullopt;
        return d;
    }

private:
    BigInteger mant_;
    std::int64_t scale_;

    BigDecimal(BigInteger mant, std::int64_t scale)
        : mant_(std::move(mant)), scale_(scale) {}

    // Strip every trailing zero digit (lowering the scale, value-preserving),
    // and pin zero to (0, 0).
    void normalize() {
        if (mant_.is_zero()) {
            mant_ = BigInteger::zero();
            scale_ = 0;
            return;
        }
        BigInteger ten = BigInteger::from_i64(10);
        for (;;) {
            std::pair<BigInteger, BigInteger> qr = mant_.div_rem(ten);
            if (!qr.second.is_zero()) break;
            mant_ = qr.first;
            // saturating decrement: a scale near INT64_MIN is past every ceiling
            // anyway, so we only need to avoid the underflow itself.
            if (scale_ != INT64_MIN) scale_ -= 1;
        }
    }

    // Re-express both mantissas at max(this.scale, other.scale) — exact.
    std::pair<BigInteger, BigInteger> aligned_mantissas(
        const BigDecimal& other, std::int64_t& scale_out) const {
        std::int64_t target = scale_ > other.scale_ ? scale_ : other.scale_;
        scale_out = target;
        BigInteger a =
            mant_ * decimal_detail::ten_pow(scale_diff_u32(target - scale_));
        BigInteger b = other.mant_ *
                       decimal_detail::ten_pow(scale_diff_u32(target - other.scale_));
        return {std::move(a), std::move(b)};
    }

    // Narrow a non-negative scale difference to the u32 that ten_pow needs.
    // Bounded at 3× the internal ceiling: alignment differences are ≤ 2× the
    // ceiling, and any div/round exponent that could yield a REPRESENTABLE
    // result (canonical scale ≤ the ceiling) stays within 3×. A larger value is
    // always doomed by the ceiling, so we reject it here rather than materialize
    // a multi-hundred-megabyte power of ten first (Rust bounds only at u32::MAX;
    // this makes the always-rejected case cheap). 3·8e6 caps ten_pow at ~24 MB.
    static std::uint32_t scale_diff_u32(std::int64_t diff) {
        if (diff < 0 || diff > 3 * INTERNAL_SCALE_LIMIT) {
            throw std::out_of_range(
                "scale difference too large to materialize");
        }
        return static_cast<std::uint32_t>(diff);
    }

    static bool all_ascii_digits(const std::string& s, std::size_t from,
                                 std::size_t to) {
        for (std::size_t i = from; i < to; i++) {
            if (s[i] < '0' || s[i] > '9') return false;
        }
        return true;
    }

    // Shared parse core. Returns true and sets `out` on success; returns false
    // and sets `err` on failure.
    static bool parse_impl(const std::string& s, std::optional<BigDecimal>& out,
                           ParseDecimalError::Kind& err) {
        using Kind = ParseDecimalError::Kind;
        if (s.empty()) {
            err = Kind::Empty;
            return false;
        }
        std::size_t i = 0;
        bool negative = false;
        if (s[0] == '+' || s[0] == '-') {
            negative = s[0] == '-';
            i = 1;
        }
        // Split off an optional exponent at the first 'e'/'E'.
        std::size_t epos = std::string::npos;
        for (std::size_t k = i; k < s.size(); k++) {
            if (s[k] == 'e' || s[k] == 'E') {
                epos = k;
                break;
            }
        }
        std::size_t digits_begin = i;
        std::size_t digits_end = epos == std::string::npos ? s.size() : epos;

        // Integer and fractional groups around at most one '.'.
        std::size_t dot = std::string::npos;
        for (std::size_t k = digits_begin; k < digits_end; k++) {
            if (s[k] == '.') {
                if (dot != std::string::npos) {
                    err = Kind::MalformedShape;
                    return false;
                }
                dot = k;
            }
        }
        std::size_t int_begin = digits_begin;
        std::size_t int_end = dot == std::string::npos ? digits_end : dot;
        std::size_t frac_begin = dot == std::string::npos ? digits_end : dot + 1;
        std::size_t frac_end = digits_end;
        std::size_t int_len = int_end - int_begin;
        std::size_t frac_len = frac_end - frac_begin;

        if (int_len == 0 && frac_len == 0) {
            err = Kind::Empty;
            return false;
        }
        if (!all_ascii_digits(s, int_begin, int_end) ||
            !all_ascii_digits(s, frac_begin, frac_end)) {
            err = Kind::InvalidDigit;
            return false;
        }

        // The exponent, if present. `s` is NUL-terminated via c_str(), and the
        // exponent runs to the end, so strtoll can consume it directly.
        std::int64_t exp = 0;
        if (epos != std::string::npos) {
            std::size_t exp_begin = epos + 1;
            std::size_t exp_len = s.size() - exp_begin;
            if (exp_len == 0) {
                err = Kind::InvalidDigit;
                return false;
            }
            std::size_t sign_off = 0;
            if (s[exp_begin] == '+' || s[exp_begin] == '-') sign_off = 1;
            if (sign_off == exp_len ||
                !all_ascii_digits(s, exp_begin + sign_off, s.size())) {
                err = Kind::InvalidDigit;
                return false;
            }
            errno = 0;
            const char* start = s.c_str() + exp_begin;
            char* endp = nullptr;
            long long v = std::strtoll(start, &endp, 10);
            if (errno == ERANGE) {
                err = Kind::ExponentOverflow;
                return false;
            }
#if LLONG_MAX > INT64_MAX
            if (v > INT64_MAX || v < INT64_MIN) {
                err = Kind::ExponentOverflow;
                return false;
            }
#endif
            (void)endp; // shape already validated
            exp = static_cast<std::int64_t>(v);
        }

        // Assemble the mantissa integer: [-] int_digits frac_digits.
        std::string all;
        all.reserve(int_len + frac_len + 1);
        if (negative) all.push_back('-');
        all.append(s, int_begin, int_len);
        all.append(s, frac_begin, frac_len);

        BigInteger mant = BigInteger::zero();
        if (!all.empty() && all != "-") {
            mant = BigInteger::parse_radix(all, 10); // digits pre-validated
        }

        // scale = frac_len - exp.
        std::int64_t scale;
        if (!decimal_detail::sub_checked(static_cast<std::int64_t>(frac_len), exp,
                                         scale)) {
            err = Kind::ExponentOverflow;
            return false;
        }

        // Canonicalize (can change the stored scale) THEN enforce MAX_SCALE.
        std::optional<BigDecimal> d =
            checked_from_parts(std::move(mant), scale);
        if (!d || decimal_detail::abs_u64(d->scale_) >
                      static_cast<std::uint64_t>(MAX_SCALE)) {
            err = Kind::ExponentOverflow;
            return false;
        }
        out = std::move(d);
        return true;
    }
};

}  // namespace ca

#endif  // CA_BIGNUM_DECIMAL_HPP
