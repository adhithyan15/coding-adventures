// wide_int.hpp — portable 128-bit integers from two 64-bit halves
// (header-only, ISO C++17).
// ---------------------------------------------------------------------------
//
// Rust has native u128/i128; C++ does not. GCC and Clang offer `__int128` as an
// extension, but MSVC has no 128-bit integer type and `__int128` is rejected
// under -pedantic-errors. So a *portable* 128-bit integer must be synthesised
// from two std::uint64_t halves using only standard 64-bit arithmetic — which is
// what `ca::wide_int::u128` and `i128` do, with idiomatic C++ operators.
//
// Representation: value = hi * 2^64 + lo. `i128` shares the layout and reuses
// u128's add/sub/mul/bitwise (two's complement), differing only in division,
// comparison, shift-right, and formatting. Every operation is total and
// well-defined (no undefined shifts, no `__int128`), identical across GCC,
// Clang, and MSVC.
#ifndef CA_WIDE_INT_HPP
#define CA_WIDE_INT_HPP

#include <cstdint>
#include <string>
#include <utility>

namespace ca {
namespace wide_int {

class u128 {
public:
    constexpr u128() noexcept : hi_(0), lo_(0) {}
    constexpr u128(std::uint64_t lo) noexcept : hi_(0), lo_(lo) {}
    constexpr u128(std::uint64_t hi, std::uint64_t lo) noexcept : hi_(hi), lo_(lo) {}

    constexpr std::uint64_t hi() const noexcept { return hi_; }
    constexpr std::uint64_t lo() const noexcept { return lo_; }
    // Truncate to 64 bits (explicit, to avoid silent narrowing).
    constexpr std::uint64_t to_u64() const noexcept { return lo_; }
    constexpr bool is_zero() const noexcept { return hi_ == 0 && lo_ == 0; }

    static constexpr u128 max() noexcept { return u128(~std::uint64_t(0), ~std::uint64_t(0)); }

    // ── arithmetic (modulo 2^128) ──────────────────────────────────
    friend constexpr u128 operator+(u128 a, u128 b) noexcept {
        std::uint64_t lo = a.lo_ + b.lo_;
        std::uint64_t carry = (lo < a.lo_) ? 1u : 0u;
        return u128(a.hi_ + b.hi_ + carry, lo);
    }
    friend constexpr u128 operator-(u128 a, u128 b) noexcept {
        std::uint64_t lo = a.lo_ - b.lo_;
        std::uint64_t borrow = (a.lo_ < b.lo_) ? 1u : 0u;
        return u128(a.hi_ - b.hi_ - borrow, lo);
    }
    friend constexpr u128 operator*(u128 a, u128 b) noexcept {
        u128 ll = mul_u64(a.lo_, b.lo_);
        std::uint64_t mid = a.lo_ * b.hi_ + a.hi_ * b.lo_;
        return u128(ll.hi_ + mid, ll.lo_);
    }
    friend constexpr u128 operator/(u128 a, u128 b) noexcept { return divmod(a, b).first; }
    friend constexpr u128 operator%(u128 a, u128 b) noexcept { return divmod(a, b).second; }

    u128& operator+=(u128 b) noexcept { return *this = *this + b; }
    u128& operator-=(u128 b) noexcept { return *this = *this - b; }
    u128& operator*=(u128 b) noexcept { return *this = *this * b; }
    u128& operator/=(u128 b) noexcept { return *this = *this / b; }
    u128& operator%=(u128 b) noexcept { return *this = *this % b; }

    // Exact widening 64x64 -> 128 multiply — the core primitive.
    static constexpr u128 mul_u64(std::uint64_t a, std::uint64_t b) noexcept {
        std::uint64_t a_lo = a & 0xFFFFFFFFu, a_hi = a >> 32;
        std::uint64_t b_lo = b & 0xFFFFFFFFu, b_hi = b >> 32;
        std::uint64_t ll = a_lo * b_lo, lh = a_lo * b_hi, hl = a_hi * b_lo, hh = a_hi * b_hi;
        std::uint64_t cross = (ll >> 32) + (lh & 0xFFFFFFFFu) + (hl & 0xFFFFFFFFu);
        std::uint64_t lo = (ll & 0xFFFFFFFFu) | (cross << 32);
        std::uint64_t hi = hh + (lh >> 32) + (hl >> 32) + (cross >> 32);
        return u128(hi, lo);
    }

    // Division with remainder (binary long division). By-zero yields (0, a) —
    // callers guard against it; see the note in README.
    static constexpr std::pair<u128, u128> divmod(u128 a, u128 b) noexcept {
        if (b.is_zero()) {
            return {u128(0), a};
        }
        u128 q(0), r(0);
        for (int i = 127; i >= 0; --i) {
            r = r << 1;
            r.lo_ |= a.bit(static_cast<unsigned>(i));
            if (r >= b) {
                r = r - b;
                q.set_bit(static_cast<unsigned>(i));
            }
        }
        return {q, r};
    }

    // ── bitwise / shifts ───────────────────────────────────────────
    friend constexpr u128 operator&(u128 a, u128 b) noexcept { return u128(a.hi_ & b.hi_, a.lo_ & b.lo_); }
    friend constexpr u128 operator|(u128 a, u128 b) noexcept { return u128(a.hi_ | b.hi_, a.lo_ | b.lo_); }
    friend constexpr u128 operator^(u128 a, u128 b) noexcept { return u128(a.hi_ ^ b.hi_, a.lo_ ^ b.lo_); }
    constexpr u128 operator~() const noexcept { return u128(~hi_, ~lo_); }

    friend constexpr u128 operator<<(u128 a, unsigned n) noexcept {
        if (n == 0) return a;
        if (n >= 128) return u128(0);
        if (n >= 64) return u128(a.lo_ << (n - 64), 0);
        return u128((a.hi_ << n) | (a.lo_ >> (64 - n)), a.lo_ << n);
    }
    friend constexpr u128 operator>>(u128 a, unsigned n) noexcept {
        if (n == 0) return a;
        if (n >= 128) return u128(0);
        if (n >= 64) return u128(0, a.hi_ >> (n - 64));
        return u128(a.hi_ >> n, (a.lo_ >> n) | (a.hi_ << (64 - n)));
    }

    // ── comparison ─────────────────────────────────────────────────
    friend constexpr bool operator==(u128 a, u128 b) noexcept { return a.hi_ == b.hi_ && a.lo_ == b.lo_; }
    friend constexpr bool operator!=(u128 a, u128 b) noexcept { return !(a == b); }
    friend constexpr bool operator<(u128 a, u128 b) noexcept {
        return a.hi_ != b.hi_ ? a.hi_ < b.hi_ : a.lo_ < b.lo_;
    }
    friend constexpr bool operator>(u128 a, u128 b) noexcept { return b < a; }
    friend constexpr bool operator<=(u128 a, u128 b) noexcept { return !(b < a); }
    friend constexpr bool operator>=(u128 a, u128 b) noexcept { return !(a < b); }

    // ── formatting ─────────────────────────────────────────────────
    std::string to_hex() const {
        static const char* D = "0123456789abcdef";
        std::string s(32, '0');
        for (int i = 0; i < 16; ++i) s[static_cast<std::size_t>(i)] = D[(hi_ >> (60 - i * 4)) & 0xF];
        for (int i = 0; i < 16; ++i) s[static_cast<std::size_t>(16 + i)] = D[(lo_ >> (60 - i * 4)) & 0xF];
        return s;
    }
    std::string to_string() const {
        if (is_zero()) return "0";
        std::string s;
        u128 v = *this;
        u128 ten(10);
        while (!v.is_zero()) {
            std::pair<u128, u128> qr = divmod(v, ten);
            s.push_back(static_cast<char>('0' + static_cast<int>(qr.second.lo_)));
            v = qr.first;
        }
        std::string out(s.rbegin(), s.rend());
        return out;
    }

private:
    constexpr std::uint64_t bit(unsigned i) const noexcept {
        return i >= 64 ? (hi_ >> (i - 64)) & 1u : (lo_ >> i) & 1u;
    }
    constexpr void set_bit(unsigned i) noexcept {
        if (i >= 64) hi_ |= std::uint64_t(1) << (i - 64);
        else lo_ |= std::uint64_t(1) << i;
    }

    std::uint64_t hi_;
    std::uint64_t lo_;
};

// Signed 128-bit integer (two's complement over u128's bits).
class i128 {
public:
    constexpr i128() noexcept : bits_(0) {}
    constexpr i128(std::int64_t v) noexcept
        : bits_(v < 0 ? ~std::uint64_t(0) : std::uint64_t(0), static_cast<std::uint64_t>(v)) {}
    constexpr i128(std::uint64_t hi, std::uint64_t lo) noexcept : bits_(hi, lo) {}
    explicit constexpr i128(u128 bits) noexcept : bits_(bits) {}

    constexpr u128 bits() const noexcept { return bits_; }
    constexpr bool is_negative() const noexcept { return (bits_.hi() >> 63) != 0; }
    constexpr bool is_zero() const noexcept { return bits_.is_zero(); }

    friend constexpr i128 operator+(i128 a, i128 b) noexcept { return i128(a.bits_ + b.bits_); }
    friend constexpr i128 operator-(i128 a, i128 b) noexcept { return i128(a.bits_ - b.bits_); }
    friend constexpr i128 operator*(i128 a, i128 b) noexcept { return i128(a.bits_ * b.bits_); }
    constexpr i128 operator-() const noexcept { return i128((~bits_) + u128(1)); }

    friend constexpr i128 operator/(i128 a, i128 b) noexcept { return divmod(a, b).first; }
    friend constexpr i128 operator%(i128 a, i128 b) noexcept { return divmod(a, b).second; }

    // Truncating division toward zero; remainder takes the dividend's sign.
    static constexpr std::pair<i128, i128> divmod(i128 a, i128 b) noexcept {
        if (b.is_zero()) {
            return {i128(0), a};
        }
        bool na = a.is_negative(), nb = b.is_negative();
        u128 ua = (na ? (-a) : a).bits_;
        u128 ub = (nb ? (-b) : b).bits_;
        std::pair<u128, u128> qr = u128::divmod(ua, ub);
        i128 q(qr.first), r(qr.second);
        if (na != nb) q = -q;
        if (na) r = -r;
        return {q, r};
    }

    // Arithmetic shift right (sign-extending).
    friend constexpr i128 operator>>(i128 a, unsigned n) noexcept {
        u128 shifted = a.bits_ >> n;
        if (a.is_negative()) {
            u128 fill = (n >= 128) ? u128::max() : (u128::max() << (128 - n));
            shifted = shifted | fill;
        }
        return i128(shifted);
    }

    friend constexpr bool operator==(i128 a, i128 b) noexcept { return a.bits_ == b.bits_; }
    friend constexpr bool operator!=(i128 a, i128 b) noexcept { return !(a == b); }
    friend constexpr bool operator<(i128 a, i128 b) noexcept {
        bool na = a.is_negative(), nb = b.is_negative();
        if (na != nb) return na; // the negative one is smaller
        return a.bits_ < b.bits_;
    }
    friend constexpr bool operator>(i128 a, i128 b) noexcept { return b < a; }
    friend constexpr bool operator<=(i128 a, i128 b) noexcept { return !(b < a); }
    friend constexpr bool operator>=(i128 a, i128 b) noexcept { return !(a < b); }

    std::string to_string() const {
        if (is_negative()) {
            return "-" + (-(*this)).bits_.to_string();
        }
        return bits_.to_string();
    }

private:
    u128 bits_;
};

} // namespace wide_int
} // namespace ca

#endif // CA_WIDE_INT_HPP
