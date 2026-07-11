// gf256.hpp — Galois Field GF(2^8) arithmetic, in pure ISO C++17 (header-only).
// A faithful port of the Rust `gf256` crate, in namespace `ca::gf256`.
// ===========================================================================
//
// GF(2^8) is the finite field of 256 elements — the bytes 0..255 with field
// arithmetic. Addition is XOR (characteristic 2, so subtraction is the same),
// and multiplication reduces the product modulo an irreducible degree-8
// polynomial. This field underlies Reed-Solomon codes, QR codes, and AES.
//
// Two interfaces, matching the crate:
//   - Free functions fixed to the Reed-Solomon polynomial 0x11D, using log /
//     antilog tables (built once, thread-safe via a function-local static).
//   - `ca::gf256::Field` — parameterised by any primitive polynomial (e.g.
//     AES's 0x11B), using table-free Russian-peasant multiplication.
//
// Degenerate cases (division by zero, inverse of zero) return 0 (the Rust crate
// panics).
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. No extensions.
#ifndef GF256_HPP
#define GF256_HPP

#include <array>
#include <cstdint>

namespace ca {
namespace gf256 {

constexpr std::uint8_t ZERO = 0;
constexpr std::uint8_t ONE = 1;
constexpr std::uint16_t PRIMITIVE_POLYNOMIAL = 0x11d;

namespace detail {

struct tables {
    std::array<std::uint16_t, 256> log;
    std::array<std::uint8_t, 256> alog;
};

inline const tables &get_tables() {
    // Function-local static: initialised once, thread-safely (C++11).
    static const tables t = [] {
        tables tt{};
        std::uint16_t val = 1;
        for (int i = 0; i < 255; i++) {
            tt.alog[static_cast<std::size_t>(i)] = static_cast<std::uint8_t>(val);
            tt.log[val] = static_cast<std::uint16_t>(i);
            val = static_cast<std::uint16_t>(val << 1);
            if (val >= 256) {
                val ^= PRIMITIVE_POLYNOMIAL;
            }
        }
        tt.alog[255] = 1;
        tt.log[0] = 0;
        return tt;
    }();
    return t;
}

} // namespace detail

// ── module-level operations (default field, polynomial 0x11D) ───────────────
inline std::uint8_t add(std::uint8_t a, std::uint8_t b) {
    return static_cast<std::uint8_t>(a ^ b);
}
inline std::uint8_t subtract(std::uint8_t a, std::uint8_t b) {
    return static_cast<std::uint8_t>(a ^ b);
}
inline std::uint8_t multiply(std::uint8_t a, std::uint8_t b) {
    if (a == 0 || b == 0) {
        return 0;
    }
    const detail::tables &t = detail::get_tables();
    unsigned exp = (static_cast<unsigned>(t.log[a]) +
                    static_cast<unsigned>(t.log[b])) % 255u;
    return t.alog[exp];
}
inline std::uint8_t divide(std::uint8_t a, std::uint8_t b) {
    if (b == 0 || a == 0) {
        return 0;
    }
    const detail::tables &t = detail::get_tables();
    int exp = ((static_cast<int>(t.log[a]) - static_cast<int>(t.log[b])) + 255) %
              255;
    return t.alog[static_cast<std::size_t>(exp)];
}
inline std::uint8_t power(std::uint8_t base, std::uint32_t exp) {
    if (base == 0) {
        return static_cast<std::uint8_t>(exp == 0 ? 1 : 0);
    }
    if (exp == 0) {
        return 1;
    }
    const detail::tables &t = detail::get_tables();
    unsigned e = static_cast<unsigned>(
        (static_cast<std::uint64_t>(t.log[base]) * exp) % 255u);
    return t.alog[e];
}
inline std::uint8_t inverse(std::uint8_t a) {
    if (a == 0) {
        return 0;
    }
    const detail::tables &t = detail::get_tables();
    return t.alog[static_cast<std::size_t>(255 - t.log[a])];
}

// ── parameterisable field (any primitive polynomial) ────────────────────────
class Field {
public:
    explicit Field(std::uint16_t primitive_poly)
        : primitive_polynomial(primitive_poly),
          reduce_(static_cast<std::uint8_t>(primitive_poly & 0xff)) {}

    std::uint16_t polynomial() const { return primitive_polynomial; }

    std::uint8_t add(std::uint8_t a, std::uint8_t b) const {
        return static_cast<std::uint8_t>(a ^ b);
    }
    std::uint8_t subtract(std::uint8_t a, std::uint8_t b) const {
        return static_cast<std::uint8_t>(a ^ b);
    }
    std::uint8_t multiply(std::uint8_t a, std::uint8_t b) const {
        return gf_mul(a, b);
    }
    std::uint8_t divide(std::uint8_t a, std::uint8_t b) const {
        if (b == 0) {
            return 0;
        }
        return gf_mul(a, gf_pow(b, 254));
    }
    std::uint8_t power(std::uint8_t base, std::uint32_t exp) const {
        return gf_pow(base, exp);
    }
    std::uint8_t inverse(std::uint8_t a) const {
        if (a == 0) {
            return 0;
        }
        return gf_pow(a, 254);
    }

    std::uint16_t primitive_polynomial;

private:
    std::uint8_t reduce_;

    std::uint8_t gf_mul(std::uint8_t a, std::uint8_t b) const {
        std::uint8_t result = 0, aa = a, bb = b;
        for (int i = 0; i < 8; i++) {
            if (bb & 1) {
                result = static_cast<std::uint8_t>(result ^ aa);
            }
            std::uint8_t hi = static_cast<std::uint8_t>(aa & 0x80);
            aa = static_cast<std::uint8_t>(aa << 1);
            if (hi) {
                aa = static_cast<std::uint8_t>(aa ^ reduce_);
            }
            bb = static_cast<std::uint8_t>(bb >> 1);
        }
        return result;
    }
    std::uint8_t gf_pow(std::uint8_t base, std::uint32_t exp) const {
        if (base == 0) {
            return static_cast<std::uint8_t>(exp == 0 ? 1 : 0);
        }
        if (exp == 0) {
            return 1;
        }
        std::uint8_t result = 1, b = base;
        std::uint32_t e = exp;
        while (e > 0) {
            if (e & 1) {
                result = gf_mul(result, b);
            }
            b = gf_mul(b, b);
            e >>= 1;
        }
        return result;
    }
};

} // namespace gf256
} // namespace ca

#endif // GF256_HPP
