// bignum_rational.hpp — an exact fraction (BigRational), built on BigInteger,
// in pure ISO C++17, header-only, in namespace ca. A faithful port of the
// `rational` module of the Rust `bignum-core` crate.
// ===========================================================================
//
// WHAT IT IS. A BigRational is an EXACT rational: an arbitrary-precision integer
// numerator over an arbitrary-precision integer denominator, always canonical —
// lowest terms (divided through by the gcd), the sign on the numerator (the
// denominator is always positive), and every zero collapsed to 0/1. So `==` and
// `<` compare by value and equal values print identically.
//
// WHY. `double` cannot represent 1/3, and `0.1 + 0.2` is not `0.3`. A
// BigRational holds `1/3` and `3/10` exactly; arithmetic never rounds.
//
// ERRORS. Value semantics throughout. Where Rust returns Option/Result, this
// port offers BOTH a throwing form (`make`/`div`/`recip` throw std::domain_error
// on a zero denominator/divisor; `parse` throws ParseRatioError; `try_pow`
// throws ca::PowTooLargeError) and a non-throwing `checked_*` / `try_parse`
// returning std::optional.
//
// DIVERGENCE. `to_f64` narrows to the nearest double; the Rust crate routes this
// through its `BigDouble` float rung, whereas this port (without that rung)
// computes the same correctly-rounded result through the exact base-10
// `BigDecimal` division and `std::strtod` — correct for every rational of
// practical magnitude, saturating to ±inf / 0 beyond double's range. No i128
// conversions (pure ISO C++ has no 128-bit integer).
//
// PORTABILITY. Pure ISO C++17 — no <cmath>/libm. Compiles clean under GCC,
// Clang, and MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
#ifndef CA_BIGNUM_RATIONAL_HPP
#define CA_BIGNUM_RATIONAL_HPP

#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <utility>

#include "bignum_core.hpp"
#include "bignum_decimal.hpp"

namespace ca {

// Thrown by BigRational::parse on a malformed fraction literal.
class ParseRatioError : public std::runtime_error {
public:
    enum class Kind { Empty, TooManySlashes, InvalidInteger, ZeroDenominator };

    explicit ParseRatioError(Kind k)
        : std::runtime_error(message(k)), kind_(k) {}

    Kind kind() const { return kind_; }

private:
    Kind kind_;
    static std::string message(Kind k) {
        switch (k) {
            case Kind::Empty: return "empty numerator or denominator";
            case Kind::TooManySlashes: return "more than one '/' in ratio";
            case Kind::InvalidInteger: return "invalid integer in ratio";
            case Kind::ZeroDenominator: return "denominator is zero";
        }
        return "ratio parse error";
    }
};

class BigRational {
public:
    // ---- construction --------------------------------------------------
    BigRational() : num_(BigInteger::zero()), den_(BigInteger::one()) {}
    static BigRational zero() { return BigRational(); }
    static BigRational one() {
        return BigRational(BigInteger::one(), BigInteger::one());
    }
    static BigRational from_i64(std::int64_t n) {
        return BigRational(BigInteger::from_i64(n), BigInteger::one());
    }
    static BigRational from_u64(std::uint64_t n) {
        return BigRational(BigInteger::from_u64(n), BigInteger::one());
    }
    static BigRational from_integer(const BigInteger& n) {
        return BigRational(n, BigInteger::one());
    }
    static BigRational from_ints(std::int64_t num, std::int64_t den) {
        return make(BigInteger::from_i64(num), BigInteger::from_i64(den));
    }

    // Build num/den, reduced to canonical form. Throws std::domain_error if
    // `den` is zero (the Rust `new` panic).
    static BigRational make(BigInteger num, BigInteger den) {
        std::optional<BigRational> r =
            checked_make(std::move(num), std::move(den));
        if (!r) throw std::domain_error("BigRational denominator must be non-zero");
        return std::move(*r);
    }
    // Non-throwing form: std::nullopt if `den` is zero.
    static std::optional<BigRational> checked_make(BigInteger num,
                                                   BigInteger den) {
        if (den.is_zero()) return std::nullopt;
        // Force the denominator positive, carrying the sign to the numerator.
        if (den.is_negative()) {
            num = num.neg();
            den = den.neg();
        }
        // Reduce to lowest terms. gcd is non-negative and gcd(0,d)==d, so a zero
        // numerator divides through to the canonical 0/1; den>0 stays positive.
        BigInteger g = num.gcd(den);
        return BigRational(num / g, den / g);
    }

    // ---- accessors -----------------------------------------------------
    const BigInteger& numerator() const { return num_; }   // carries the sign
    const BigInteger& denominator() const { return den_; } // always > 0

    // ---- predicates & sign ---------------------------------------------
    bool is_zero() const { return num_.is_zero(); }
    bool is_integer() const { return den_ == BigInteger::one(); }
    bool is_negative() const { return num_.is_negative(); }
    bool is_positive() const { return num_.is_positive(); }
    int signum() const { return num_.signum(); }
    BigRational abs() const { return BigRational(num_.abs(), den_); }

    // The reciprocal 1/self. Throws std::domain_error if self is zero.
    BigRational recip() const {
        std::optional<BigRational> r = checked_recip();
        if (!r) throw std::domain_error("cannot take the reciprocal of zero");
        return std::move(*r);
    }
    std::optional<BigRational> checked_recip() const {
        if (is_zero()) return std::nullopt;
        return checked_make(den_, num_); // swap; sign may move off the denominator
    }

    // ---- exact arithmetic ----------------------------------------------
    // a/b + c/d = (a·d + c·b)/(b·d); the plain common denominator, then reduced.
    BigRational add(const BigRational& o) const {
        return make(num_ * o.den_ + o.num_ * den_, den_ * o.den_);
    }
    BigRational sub(const BigRational& o) const {
        return make(num_ * o.den_ - o.num_ * den_, den_ * o.den_);
    }
    BigRational mul(const BigRational& o) const {
        return make(num_ * o.num_, den_ * o.den_);
    }
    // a/b ÷ c/d = (a·d)/(b·c). Throws std::domain_error if `o` is zero.
    BigRational div(const BigRational& o) const {
        std::optional<BigRational> r = checked_div(o);
        if (!r) throw std::domain_error("division by zero");
        return std::move(*r);
    }
    std::optional<BigRational> checked_div(const BigRational& o) const {
        if (o.is_zero()) return std::nullopt;
        // den here can be negative (if `o` was negative); make() fixes the sign.
        return checked_make(num_ * o.den_, den_ * o.num_);
    }

    // Raise to an integer power (a negative exponent takes the reciprocal).
    // Throws std::domain_error on a negative power of zero (that is 1/0).
    // UNBOUNDED in result size — use try_pow for an untrusted exponent.
    BigRational pow(std::int32_t exp) const {
        if (exp == 0) return one();
        std::uint32_t n = exp_abs_u32(exp);
        BigInteger num_pow = num_.pow(n);
        BigInteger den_pow = den_.pow(n);
        if (exp > 0) {
            // Coprime because num,den are; den^n > 0. Already canonical.
            return BigRational(std::move(num_pow), std::move(den_pow));
        }
        // Reciprocal: den^n / num^n. make() fixes the sign / reports 1/0.
        return make(std::move(den_pow), std::move(num_pow));
    }
    // DoS-safe pow: throws ca::PowTooLargeError up front (before allocating) if
    // either part of the result would exceed `max_bits` bits.
    BigRational try_pow(std::int32_t exp, std::uint64_t max_bits) const {
        if (exp == 0) return one();
        std::uint32_t n = exp_abs_u32(exp);
        BigInteger num_pow = num_.try_pow(n, max_bits);
        BigInteger den_pow = den_.try_pow(n, max_bits);
        if (exp > 0) {
            return BigRational(std::move(num_pow), std::move(den_pow));
        }
        return make(std::move(den_pow), std::move(num_pow));
    }

    // ---- ordering ------------------------------------------------------
    // a/b vs c/d compare as their cross-products a·d vs c·b (both denominators
    // positive by canonical form).
    int cmp(const BigRational& o) const {
        return (num_ * o.den_).cmp(o.num_ * den_);
    }
    bool operator==(const BigRational& o) const { return cmp(o) == 0; }
    bool operator!=(const BigRational& o) const { return cmp(o) != 0; }
    bool operator<(const BigRational& o) const { return cmp(o) < 0; }
    bool operator>(const BigRational& o) const { return cmp(o) > 0; }
    bool operator<=(const BigRational& o) const { return cmp(o) <= 0; }
    bool operator>=(const BigRational& o) const { return cmp(o) >= 0; }

    // ---- operator overloads --------------------------------------------
    BigRational operator+(const BigRational& o) const { return add(o); }
    BigRational operator-(const BigRational& o) const { return sub(o); }
    BigRational operator*(const BigRational& o) const { return mul(o); }
    BigRational operator/(const BigRational& o) const { return div(o); }
    BigRational operator-() const { return BigRational(num_.neg(), den_); }

    // ---- formatting ----------------------------------------------------
    // "numerator/denominator", or just "numerator" for a whole number.
    std::string to_string() const {
        if (is_integer()) return num_.to_string();
        return num_.to_string() + "/" + den_.to_string();
    }

    // A lossy narrowing to the nearest double (round-half-even). Values beyond
    // double's range saturate to ±inf; tiny ones to 0.
    double to_f64() const {
        if (num_.is_zero()) return 0.0;
        std::uint64_t nbits = num_.bit_len();
        std::uint64_t dbits = den_.bit_len();
        // Fractional places to capture the leading significant digit plus a
        // 45-digit guard; beyond ~400 places every value underflows to 0.
        std::int64_t scale = 45;
        if (dbits > nbits) {
            std::uint64_t lead = ((dbits - nbits) * 30103u) / 100000u;
            if (lead > 400) lead = 400;
            scale = static_cast<std::int64_t>(lead) + 45;
        }
        BigDecimal q = BigDecimal::from_integer(num_).div_round(
            BigDecimal::from_integer(den_), scale, RoundingMode::HalfEven);
        return q.to_f64();
    }

    // ---- parsing -------------------------------------------------------
    // Throwing form: "num/den" or a bare integer "num" (base 10; n → n/1).
    // Whitespace is not trimmed.
    static BigRational parse(const std::string& s) {
        std::optional<BigRational> r;
        ParseRatioError::Kind err = ParseRatioError::Kind::Empty;
        if (!parse_impl(s, r, err)) throw ParseRatioError(err);
        return std::move(*r);
    }
    // Non-throwing form.
    static std::optional<BigRational> try_parse(const std::string& s) {
        std::optional<BigRational> r;
        ParseRatioError::Kind err = ParseRatioError::Kind::Empty;
        if (!parse_impl(s, r, err)) return std::nullopt;
        return r;
    }

private:
    BigInteger num_;
    BigInteger den_;

    BigRational(BigInteger num, BigInteger den)
        : num_(std::move(num)), den_(std::move(den)) {}

    // The magnitude of an i32 exponent as a u32, without UB on INT32_MIN.
    static std::uint32_t exp_abs_u32(std::int32_t exp) {
        return exp < 0 ? (0u - static_cast<std::uint32_t>(exp))
                       : static_cast<std::uint32_t>(exp);
    }

    static bool parse_impl(const std::string& s, std::optional<BigRational>& out,
                           ParseRatioError::Kind& err) {
        using Kind = ParseRatioError::Kind;
        std::size_t slash = s.find('/');
        if (slash != std::string::npos &&
            s.find('/', slash + 1) != std::string::npos) {
            err = Kind::TooManySlashes;
            return false;
        }
        std::string num_str =
            slash == std::string::npos ? s : s.substr(0, slash);
        if (num_str.empty()) {
            err = Kind::Empty;
            return false;
        }
        BigInteger num;
        try {
            num = BigInteger::parse_radix(num_str, 10);
        } catch (const ParseBigIntError&) {
            err = Kind::InvalidInteger;
            return false;
        }
        if (slash == std::string::npos) {
            out = from_integer(num); // bare integer → n/1
            return true;
        }
        std::string den_str = s.substr(slash + 1);
        if (den_str.empty()) {
            err = Kind::Empty;
            return false;
        }
        BigInteger den;
        try {
            den = BigInteger::parse_radix(den_str, 10);
        } catch (const ParseBigIntError&) {
            err = Kind::InvalidInteger;
            return false;
        }
        std::optional<BigRational> r =
            checked_make(std::move(num), std::move(den));
        if (!r) {
            err = Kind::ZeroDenominator;
            return false;
        }
        out = std::move(r);
        return true;
    }
};

}  // namespace ca

#endif  // CA_BIGNUM_RATIONAL_HPP
