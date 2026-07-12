// bignum_core.hpp — arbitrary-precision signed integers (BigInteger), in pure
// ISO C++17, header-only, in namespace ca. A faithful port of the `BigInteger`
// core of the Rust `bignum-core` crate.
// ===========================================================================
//
// Sign-magnitude arbitrary-precision integer: a sign (-1 / 0 / +1) plus a
// magnitude stored as little-endian base-2^32 limbs with no trailing zero limb
// (zero is the empty magnitude — never a "-0"). All arithmetic uses 32-bit limbs
// and a 64-bit accumulator, so no 128-bit integers are needed.
//
// Add/subtract are column methods, multiply is schoolbook O(n·m), and division
// is Knuth's Algorithm D (TAOCP §4.3.1) — long division in base 2^32. Division
// truncates toward zero and the remainder takes the dividend's sign, matching
// C++ integer `/` and `%`.
//
// Errors: division by zero throws std::domain_error; parse failures throw
// ca::ParseBigIntError; try_pow throws ca::PowTooLargeError.
//
// This ports the integer core; the crate's decimal / float rungs build on it.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_BIGNUM_CORE_HPP
#define CA_BIGNUM_CORE_HPP

#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace ca {

// Thrown by parse_radix on a malformed string or bad radix.
class ParseBigIntError : public std::runtime_error {
public:
    enum class Kind { Empty, InvalidDigit, InvalidRadix };
    Kind kind;
    char bad_char;     // valid for InvalidDigit
    std::uint32_t radix;  // valid for InvalidRadix
    explicit ParseBigIntError(Kind k, char c = 0, std::uint32_t r = 0)
        : std::runtime_error(message(k, c, r)), kind(k), bad_char(c), radix(r) {}

private:
    static std::string message(Kind k, char c, std::uint32_t r) {
        switch (k) {
            case Kind::Empty:
                return "cannot parse an empty integer string";
            case Kind::InvalidDigit:
                return std::string("invalid digit '") + c +
                       "' for the requested radix";
            default:
                return "invalid radix " + std::to_string(r) +
                       ": must be in 2..=36";
        }
    }
};

// Thrown by try_pow when the projected result would exceed the ceiling.
class PowTooLargeError : public std::runtime_error {
public:
    std::uint64_t projected_bits;
    std::uint64_t max_bits;
    PowTooLargeError(std::uint64_t projected, std::uint64_t max)
        : std::runtime_error("pow result would be ~" +
                             std::to_string(projected) + " bits, exceeding the " +
                             std::to_string(max) + "-bit ceiling"),
          projected_bits(projected), max_bits(max) {}
};

namespace bignum_detail {

using Mag = std::vector<std::uint32_t>;  // little-endian, no trailing zeros

inline unsigned clz32(std::uint32_t x) {
    unsigned n = 0;
    if (x == 0) {
        return 32;
    }
    while (!(x & 0x80000000u)) {
        x <<= 1;
        ++n;
    }
    return n;
}

inline void normalize(Mag& m) {
    while (!m.empty() && m.back() == 0) {
        m.pop_back();
    }
}

inline int mag_cmp(const Mag& a, const Mag& b) {
    if (a.size() != b.size()) {
        return a.size() < b.size() ? -1 : 1;
    }
    for (std::size_t i = a.size(); i > 0; --i) {
        if (a[i - 1] != b[i - 1]) {
            return a[i - 1] < b[i - 1] ? -1 : 1;
        }
    }
    return 0;
}

inline Mag mag_add(const Mag& a, const Mag& b) {
    const Mag& lo = a.size() >= b.size() ? a : b;
    const Mag& sh = a.size() >= b.size() ? b : a;
    Mag r;
    r.reserve(lo.size() + 1);
    std::uint64_t carry = 0;
    for (std::size_t i = 0; i < lo.size(); ++i) {
        std::uint64_t sum = (std::uint64_t)lo[i] + carry +
                            (i < sh.size() ? sh[i] : 0);
        r.push_back((std::uint32_t)sum);
        carry = sum >> 32;
    }
    if (carry) {
        r.push_back((std::uint32_t)carry);
    }
    return r;
}

// Requires a >= b.
inline Mag mag_sub(const Mag& a, const Mag& b) {
    Mag r;
    r.reserve(a.size());
    std::int64_t borrow = 0;
    for (std::size_t i = 0; i < a.size(); ++i) {
        std::int64_t diff = (std::int64_t)a[i] -
                            (i < b.size() ? (std::int64_t)b[i] : 0) - borrow;
        if (diff < 0) {
            diff += (std::int64_t)1 << 32;
            borrow = 1;
        } else {
            borrow = 0;
        }
        r.push_back((std::uint32_t)diff);
    }
    normalize(r);
    return r;
}

inline Mag mag_mul(const Mag& a, const Mag& b) {
    if (a.empty() || b.empty()) {
        return {};
    }
    Mag r(a.size() + b.size(), 0);
    for (std::size_t i = 0; i < a.size(); ++i) {
        std::uint64_t ai = a[i];
        std::uint64_t carry = 0;
        for (std::size_t j = 0; j < b.size(); ++j) {
            std::uint64_t cur = (std::uint64_t)r[i + j] + ai * b[j] + carry;
            r[i + j] = (std::uint32_t)cur;
            carry = cur >> 32;
        }
        r[i + b.size()] = (std::uint32_t)((std::uint64_t)r[i + b.size()] + carry);
    }
    normalize(r);
    return r;
}

inline Mag mag_mul_small(const Mag& a, std::uint32_t factor) {
    if (factor == 0 || a.empty()) {
        return {};
    }
    Mag r;
    r.reserve(a.size() + 1);
    std::uint64_t carry = 0, f = factor;
    for (std::uint32_t limb : a) {
        std::uint64_t cur = (std::uint64_t)limb * f + carry;
        r.push_back((std::uint32_t)cur);
        carry = cur >> 32;
    }
    if (carry) {
        r.push_back((std::uint32_t)carry);
    }
    return r;
}

inline Mag mag_add_small(const Mag& a, std::uint32_t addend) {
    Mag r = a;
    std::uint64_t carry = addend;
    std::size_t i = 0;
    while (carry != 0) {
        if (i < r.size()) {
            std::uint64_t cur = (std::uint64_t)r[i] + carry;
            r[i] = (std::uint32_t)cur;
            carry = cur >> 32;
            ++i;
        } else {
            r.push_back((std::uint32_t)carry);
            carry = 0;
        }
    }
    return r;
}

inline std::pair<Mag, std::uint32_t> mag_divmod_small(const Mag& a,
                                                     std::uint32_t divisor) {
    Mag q(a.size(), 0);
    std::uint64_t d = divisor, rem = 0;
    for (std::size_t i = a.size(); i > 0; --i) {
        std::uint64_t cur = (rem << 32) | a[i - 1];
        q[i - 1] = (std::uint32_t)(cur / d);
        rem = cur % d;
    }
    normalize(q);
    return {q, (std::uint32_t)rem};
}

inline Mag shl_small(const Mag& a, unsigned bits) {
    if (bits == 0) {
        return a;
    }
    Mag r;
    r.reserve(a.size() + 1);
    std::uint32_t carry = 0;
    for (std::uint32_t limb : a) {
        std::uint64_t v = ((std::uint64_t)limb << bits) | carry;
        r.push_back((std::uint32_t)v);
        carry = (std::uint32_t)(v >> 32);
    }
    if (carry) {
        r.push_back(carry);
    }
    return r;
}

inline Mag shr_small(const Mag& a, unsigned bits) {
    if (bits == 0) {
        Mag r = a;
        normalize(r);
        return r;
    }
    Mag r(a.size(), 0);
    std::uint32_t carry = 0;
    for (std::size_t i = a.size(); i > 0; --i) {
        std::uint32_t cur = a[i - 1];
        r[i - 1] = (cur >> bits) | carry;
        carry = cur << (32 - bits);
    }
    normalize(r);
    return r;
}

// Knuth Algorithm D. v normalized, non-empty. Returns (quotient, remainder).
inline std::pair<Mag, Mag> mag_divmod(const Mag& u_in, const Mag& v_in) {
    if (mag_cmp(u_in, v_in) < 0) {
        Mag r = u_in;
        normalize(r);
        return {Mag{}, r};
    }
    std::size_t n = v_in.size();
    if (n == 1) {
        auto qr = mag_divmod_small(u_in, v_in[0]);
        Mag r;
        if (qr.second != 0) {
            r.push_back(qr.second);
        }
        return {qr.first, r};
    }

    std::uint64_t base = (std::uint64_t)1 << 32;
    unsigned shift = clz32(v_in[n - 1]);
    Mag v = shl_small(v_in, shift);
    v.resize(n);  // shift keeps length n
    Mag u = shl_small(u_in, shift);
    u.resize(u_in.size() + 1, 0);
    std::size_t m = u_in.size() - n;

    Mag q(m + 1, 0);
    for (std::size_t j = m + 1; j > 0; --j) {
        std::size_t jj = j - 1;
        std::uint64_t dividend = ((std::uint64_t)u[jj + n] << 32) | u[jj + n - 1];
        std::uint64_t qhat = dividend / v[n - 1];
        std::uint64_t rhat = dividend % v[n - 1];
        for (;;) {
            if (qhat >= base ||
                qhat * v[n - 2] > (rhat << 32) + u[jj + n - 2]) {
                qhat -= 1;
                rhat += v[n - 1];
                if (rhat < base) {
                    continue;
                }
            }
            break;
        }
        std::int64_t k = 0;
        for (std::size_t i = 0; i < n; ++i) {
            std::uint64_t p = qhat * v[i];
            std::int64_t t = (std::int64_t)u[jj + i] - k -
                             (std::int64_t)(std::uint32_t)p;
            u[jj + i] = (std::uint32_t)t;
            k = (std::int64_t)(p >> 32) - (t >> 32);
        }
        std::int64_t t = (std::int64_t)u[jj + n] - k;
        u[jj + n] = (std::uint32_t)t;
        if (t < 0) {
            qhat -= 1;
            std::uint64_t carry = 0;
            for (std::size_t i = 0; i < n; ++i) {
                std::uint64_t sum = (std::uint64_t)u[jj + i] + v[i] + carry;
                u[jj + i] = (std::uint32_t)sum;
                carry = sum >> 32;
            }
            u[jj + n] = (std::uint32_t)((std::uint64_t)u[jj + n] + carry);
        }
        q[jj] = (std::uint32_t)qhat;
    }
    normalize(q);
    Mag u_low(u.begin(), u.begin() + (std::ptrdiff_t)n);
    Mag r = shr_small(u_low, shift);
    return {q, r};
}

}  // namespace bignum_detail

class BigInteger {
public:
    BigInteger() : sign_(0) {}

    static BigInteger zero() { return BigInteger(); }
    static BigInteger one() { return from_u64(1); }

    static BigInteger from_u64(std::uint64_t value) {
        return from_u64_signed(value, false);
    }
    static BigInteger from_i64(std::int64_t value) {
        if (value < 0) {
            std::uint64_t mag =
                (std::uint64_t)(-(value + 1)) + 1;  // handles INT64_MIN
            return from_u64_signed(mag, true);
        }
        return from_u64_signed((std::uint64_t)value, false);
    }

    // Queries.
    bool is_zero() const { return sign_ == 0; }
    bool is_negative() const { return sign_ < 0; }
    bool is_positive() const { return sign_ > 0; }
    int signum() const { return sign_; }
    std::size_t num_limbs() const { return mag_.size(); }
    std::uint64_t bit_len() const {
        if (mag_.empty()) {
            return 0;
        }
        return (std::uint64_t)(mag_.size() - 1) * 32 +
               (32 - bignum_detail::clz32(mag_.back()));
    }

    // Three-way compare: -1 / 0 / +1.
    int cmp(const BigInteger& o) const {
        if (sign_ != o.sign_) {
            return sign_ < o.sign_ ? -1 : 1;
        }
        if (sign_ == 0) {
            return 0;
        }
        if (sign_ > 0) {
            return bignum_detail::mag_cmp(mag_, o.mag_);
        }
        return bignum_detail::mag_cmp(o.mag_, mag_);
    }

    BigInteger abs() const {
        return from_parts(sign_ == 0 ? 0 : 1, mag_);
    }
    BigInteger neg() const { return from_parts(-sign_, mag_); }

    // Arithmetic.
    BigInteger add(const BigInteger& o) const {
        if (sign_ == 0) return o;
        if (o.sign_ == 0) return *this;
        if (sign_ == o.sign_) {
            return from_parts(sign_, bignum_detail::mag_add(mag_, o.mag_));
        }
        int c = bignum_detail::mag_cmp(mag_, o.mag_);
        if (c == 0) return zero();
        if (c > 0) {
            return from_parts(sign_, bignum_detail::mag_sub(mag_, o.mag_));
        }
        return from_parts(o.sign_, bignum_detail::mag_sub(o.mag_, mag_));
    }
    BigInteger sub(const BigInteger& o) const { return add(o.neg()); }
    BigInteger mul(const BigInteger& o) const {
        if (sign_ == 0 || o.sign_ == 0) return zero();
        return from_parts(sign_ == o.sign_ ? 1 : -1,
                          bignum_detail::mag_mul(mag_, o.mag_));
    }

    std::pair<BigInteger, BigInteger> div_rem(const BigInteger& o) const {
        if (o.sign_ == 0) {
            throw std::domain_error("BigInteger division by zero");
        }
        if (sign_ == 0) {
            return {zero(), zero()};
        }
        auto qr = bignum_detail::mag_divmod(mag_, o.mag_);
        return {from_parts(sign_ == o.sign_ ? 1 : -1, qr.first),
                from_parts(sign_, qr.second)};
    }
    BigInteger div(const BigInteger& o) const { return div_rem(o).first; }
    BigInteger rem(const BigInteger& o) const { return div_rem(o).second; }

    BigInteger pow(std::uint32_t exp) const {
        BigInteger result = one();
        BigInteger base = *this;
        std::uint32_t e = exp;
        while (e > 0) {
            if (e & 1u) {
                result = result.mul(base);
            }
            e >>= 1;
            if (e > 0) {
                base = base.mul(base);
            }
        }
        return result;
    }

    BigInteger try_pow(std::uint32_t exp, std::uint64_t max_bits) const {
        std::uint64_t bl = bit_len();
        std::uint64_t projected;
        if (exp == 0 || sign_ == 0 || bl <= 1) {
            projected = 1;
        } else {
            std::uint64_t e = exp;
            projected = (bl > (std::uint64_t)-1 / e) ? (std::uint64_t)-1 : bl * e;
        }
        if (projected > max_bits) {
            throw PowTooLargeError(projected, max_bits);
        }
        return pow(exp);
    }

    BigInteger gcd(const BigInteger& o) const {
        BigInteger a = abs();
        BigInteger b = o.abs();
        while (!b.is_zero()) {
            BigInteger r = a.div_rem(b).second;
            a = b;
            b = r;
        }
        return a;
    }

    // Parsing / formatting.
    static BigInteger parse_radix(const std::string& s, std::uint32_t radix) {
        if (radix < 2 || radix > 36) {
            throw ParseBigIntError(ParseBigIntError::Kind::InvalidRadix, 0, radix);
        }
        if (s.empty()) {
            throw ParseBigIntError(ParseBigIntError::Kind::Empty);
        }
        std::size_t start = 0;
        bool negative = false;
        if (s[0] == '+') {
            start = 1;
        } else if (s[0] == '-') {
            negative = true;
            start = 1;
        }
        if (start >= s.size()) {
            throw ParseBigIntError(ParseBigIntError::Kind::Empty);
        }
        bignum_detail::Mag mag;
        for (std::size_t i = start; i < s.size(); ++i) {
            std::uint32_t digit;
            if (!digit_value(s[i], radix, digit)) {
                throw ParseBigIntError(ParseBigIntError::Kind::InvalidDigit, s[i]);
            }
            mag = bignum_detail::mag_add_small(
                bignum_detail::mag_mul_small(mag, radix), digit);
        }
        bignum_detail::normalize(mag);
        return from_parts(mag.empty() ? 0 : (negative ? -1 : 1), mag);
    }

    std::string to_str_radix(std::uint32_t radix) const {
        static const char* DIGITS = "0123456789abcdefghijklmnopqrstuvwxyz";
        if (radix < 2 || radix > 36) {
            throw std::invalid_argument("radix must be in 2..=36");
        }
        if (sign_ == 0) {
            return "0";
        }
        std::string digits;
        bignum_detail::Mag mag = mag_;
        while (!mag.empty()) {
            auto qr = bignum_detail::mag_divmod_small(mag, radix);
            digits.push_back(DIGITS[qr.second]);
            mag = qr.first;
        }
        std::string out;
        if (sign_ < 0) {
            out.push_back('-');
        }
        for (std::size_t i = digits.size(); i > 0; --i) {
            out.push_back(digits[i - 1]);
        }
        return out;
    }
    std::string to_string() const { return to_str_radix(10); }

    // Operators.
    BigInteger operator+(const BigInteger& o) const { return add(o); }
    BigInteger operator-(const BigInteger& o) const { return sub(o); }
    BigInteger operator*(const BigInteger& o) const { return mul(o); }
    BigInteger operator/(const BigInteger& o) const { return div(o); }
    BigInteger operator%(const BigInteger& o) const { return rem(o); }
    BigInteger operator-() const { return neg(); }
    bool operator==(const BigInteger& o) const { return cmp(o) == 0; }
    bool operator!=(const BigInteger& o) const { return cmp(o) != 0; }
    bool operator<(const BigInteger& o) const { return cmp(o) < 0; }
    bool operator>(const BigInteger& o) const { return cmp(o) > 0; }
    bool operator<=(const BigInteger& o) const { return cmp(o) <= 0; }
    bool operator>=(const BigInteger& o) const { return cmp(o) >= 0; }

private:
    int sign_;
    bignum_detail::Mag mag_;

    static BigInteger from_parts(int sign, bignum_detail::Mag mag) {
        bignum_detail::normalize(mag);
        BigInteger b;
        if (mag.empty()) {
            b.sign_ = 0;
        } else {
            b.sign_ = sign;
            b.mag_ = std::move(mag);
        }
        return b;
    }

    static BigInteger from_u64_signed(std::uint64_t value, bool negative) {
        BigInteger b;
        std::uint64_t x = value;
        while (x != 0) {
            b.mag_.push_back((std::uint32_t)x);
            x >>= 32;
        }
        b.sign_ = b.mag_.empty() ? 0 : (negative ? -1 : 1);
        return b;
    }

    static bool digit_value(char c, std::uint32_t radix, std::uint32_t& out) {
        std::uint32_t d;
        if (c >= '0' && c <= '9') {
            d = (std::uint32_t)(c - '0');
        } else if (c >= 'a' && c <= 'z') {
            d = (std::uint32_t)(c - 'a') + 10;
        } else if (c >= 'A' && c <= 'Z') {
            d = (std::uint32_t)(c - 'A') + 10;
        } else {
            return false;
        }
        if (d >= radix) {
            return false;
        }
        out = d;
        return true;
    }
};

}  // namespace ca

#endif  // CA_BIGNUM_CORE_HPP
