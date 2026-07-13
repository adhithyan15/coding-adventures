// x25519.hpp — X25519 (Curve25519 ECDH), in pure ISO C++17, header-only, in
// namespace ca::x25519. A faithful port of the Rust `x25519` crate.
// ===========================================================================
//
// X25519 (RFC 7748) is the elliptic-curve Diffie-Hellman function on
// Curve25519: a constant-time Montgomery-ladder scalar multiplication over
// GF(2^255 - 19). It underpins TLS 1.3, Signal, WireGuard, and SSH.
//
// Field elements use radix-2^51 (five 51-bit limbs); the multiply accumulates
// 102-bit partial products. Since pure ISO C++17 has no `__int128` (it would be
// rejected under -pedantic-errors / /permissive-), this port carries a small,
// contained 128-bit emulation — identical to the C sibling, and verified
// against the RFC 7748 test vectors.
//
// API. Keys are 32-byte little-endian arrays. `x25519` returns std::nullopt
// (where the Rust returns `Err`) when the output is all zeros — a low-order
// point input, which a Diffie-Hellman caller must reject.
//
// PORTABILITY. Pure ISO C++17 — no `__int128`, standard library only. Compiles
// clean under GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
// warnings-as-errors.
#ifndef CA_X25519_HPP
#define CA_X25519_HPP

#include <array>
#include <cstdint>
#include <cstring>
#include <optional>

namespace ca {
namespace x25519 {

using Key = std::array<std::uint8_t, 32>;

namespace detail {

// ── Emulated 128-bit unsigned integer ────────────────────────────────────────
struct u128 {
    std::uint64_t lo, hi;
};

inline u128 u128_from_u64(std::uint64_t v) { return u128{v, 0}; }

// 64×64 → 128 via four 32×32 products.
inline u128 u128_mul_64(std::uint64_t a, std::uint64_t b) {
    std::uint64_t aL = a & 0xFFFFFFFFu, aH = a >> 32;
    std::uint64_t bL = b & 0xFFFFFFFFu, bH = b >> 32;
    std::uint64_t ll = aL * bL, lh = aL * bH, hl = aH * bL, hh = aH * bH;
    std::uint64_t cross = (ll >> 32) + (lh & 0xFFFFFFFFu) + (hl & 0xFFFFFFFFu);
    u128 r;
    r.lo = (ll & 0xFFFFFFFFu) | (cross << 32);
    r.hi = hh + (lh >> 32) + (hl >> 32) + (cross >> 32);
    return r;
}

inline u128 u128_add(u128 a, u128 b) {
    u128 r;
    r.lo = a.lo + b.lo;
    r.hi = a.hi + b.hi + (r.lo < a.lo ? 1u : 0u);
    return r;
}

inline u128 u128_or(u128 a, u128 b) { return u128{a.lo | b.lo, a.hi | b.hi}; }

inline u128 u128_shl(u128 a, unsigned n) {
    if (n == 0) return a;
    if (n < 64) return u128{a.lo << n, (a.hi << n) | (a.lo >> (64 - n))};
    return u128{0, a.lo << (n - 64)};
}

inline u128 u128_shr(u128 a, unsigned n) {
    if (n == 0) return a;
    if (n < 64) return u128{(a.lo >> n) | (a.hi << (64 - n)), a.hi >> n};
    return u128{a.hi >> (n - 64), 0};
}

inline u128 u128_from_le16(const std::uint8_t* b) {
    u128 r{0, 0};
    for (int i = 0; i < 8; i++) r.lo |= static_cast<std::uint64_t>(b[i]) << (8 * i);
    for (int i = 0; i < 8; i++)
        r.hi |= static_cast<std::uint64_t>(b[8 + i]) << (8 * i);
    return r;
}

inline void u128_to_le16(u128 v, std::uint8_t* out) {
    for (int i = 0; i < 8; i++) out[i] = static_cast<std::uint8_t>(v.lo >> (8 * i));
    for (int i = 0; i < 8; i++)
        out[8 + i] = static_cast<std::uint8_t>(v.hi >> (8 * i));
}

// ── Field element in GF(2^255 - 19): five 51-bit limbs ───────────────────────
struct Fe {
    std::uint64_t v[5];
};

constexpr std::uint64_t MASK51 = (static_cast<std::uint64_t>(1) << 51) - 1;

inline Fe fe_zero() { return Fe{{0, 0, 0, 0, 0}}; }
inline Fe fe_one() { return Fe{{1, 0, 0, 0, 0}}; }

inline Fe fe_carry_propagate(Fe f) {
    std::uint64_t* l = f.v;
    l[1] += l[0] >> 51;
    l[0] &= MASK51;
    l[2] += l[1] >> 51;
    l[1] &= MASK51;
    l[3] += l[2] >> 51;
    l[2] &= MASK51;
    l[4] += l[3] >> 51;
    l[3] &= MASK51;
    std::uint64_t overflow = l[4] >> 51;
    l[4] &= MASK51;
    l[0] += overflow * 19;
    l[1] += l[0] >> 51;
    l[0] &= MASK51;
    return f;
}

inline Fe fe_add(Fe a, Fe b) {
    Fe r;
    for (int i = 0; i < 5; i++) r.v[i] = a.v[i] + b.v[i];
    return r;
}

inline Fe fe_sub(Fe a, Fe b) {
    Fe r;
    r.v[0] = (a.v[0] + (static_cast<std::uint64_t>(1) << 52) - 38) - b.v[0];
    r.v[1] = (a.v[1] + (static_cast<std::uint64_t>(1) << 52) - 2) - b.v[1];
    r.v[2] = (a.v[2] + (static_cast<std::uint64_t>(1) << 52) - 2) - b.v[2];
    r.v[3] = (a.v[3] + (static_cast<std::uint64_t>(1) << 52) - 2) - b.v[3];
    r.v[4] = (a.v[4] + (static_cast<std::uint64_t>(1) << 52) - 2) - b.v[4];
    return r;
}

inline Fe fe_carry_wide(u128 r[5]) {
    Fe out;
    u128 carry = u128_from_u64(0);
    for (int i = 0; i < 5; i++) {
        u128 sum = u128_add(r[i], carry);
        out.v[i] = sum.lo & MASK51;
        carry = u128_shr(sum, 51);
    }
    out.v[0] += carry.lo * 19;
    out.v[1] += out.v[0] >> 51;
    out.v[0] &= MASK51;
    return out;
}

inline Fe fe_mul(Fe fa, Fe fb) {
    std::uint64_t* a = fa.v;
    std::uint64_t* b = fb.v;
    std::uint64_t b19[5];
    for (int i = 0; i < 5; i++) b19[i] = b[i] * 19;

    u128 r[5];
    r[0] = u128_mul_64(a[0], b[0]);
    r[0] = u128_add(r[0], u128_mul_64(a[1], b19[4]));
    r[0] = u128_add(r[0], u128_mul_64(a[2], b19[3]));
    r[0] = u128_add(r[0], u128_mul_64(a[3], b19[2]));
    r[0] = u128_add(r[0], u128_mul_64(a[4], b19[1]));
    r[1] = u128_mul_64(a[0], b[1]);
    r[1] = u128_add(r[1], u128_mul_64(a[1], b[0]));
    r[1] = u128_add(r[1], u128_mul_64(a[2], b19[4]));
    r[1] = u128_add(r[1], u128_mul_64(a[3], b19[3]));
    r[1] = u128_add(r[1], u128_mul_64(a[4], b19[2]));
    r[2] = u128_mul_64(a[0], b[2]);
    r[2] = u128_add(r[2], u128_mul_64(a[1], b[1]));
    r[2] = u128_add(r[2], u128_mul_64(a[2], b[0]));
    r[2] = u128_add(r[2], u128_mul_64(a[3], b19[4]));
    r[2] = u128_add(r[2], u128_mul_64(a[4], b19[3]));
    r[3] = u128_mul_64(a[0], b[3]);
    r[3] = u128_add(r[3], u128_mul_64(a[1], b[2]));
    r[3] = u128_add(r[3], u128_mul_64(a[2], b[1]));
    r[3] = u128_add(r[3], u128_mul_64(a[3], b[0]));
    r[3] = u128_add(r[3], u128_mul_64(a[4], b19[4]));
    r[4] = u128_mul_64(a[0], b[4]);
    r[4] = u128_add(r[4], u128_mul_64(a[1], b[3]));
    r[4] = u128_add(r[4], u128_mul_64(a[2], b[2]));
    r[4] = u128_add(r[4], u128_mul_64(a[3], b[1]));
    r[4] = u128_add(r[4], u128_mul_64(a[4], b[0]));
    return fe_carry_wide(r);
}

inline Fe fe_square(Fe fa) {
    std::uint64_t* a = fa.v;
    std::uint64_t a0_2 = a[0] * 2, a1_2 = a[1] * 2, a2_2 = a[2] * 2,
                  a3_2 = a[3] * 2;
    std::uint64_t a3_19 = a[3] * 19, a4_19 = a[4] * 19;

    u128 r[5];
    r[0] = u128_mul_64(a[0], a[0]);
    r[0] = u128_add(r[0], u128_mul_64(a1_2, a4_19));
    r[0] = u128_add(r[0], u128_mul_64(a2_2, a3_19));
    r[1] = u128_mul_64(a0_2, a[1]);
    r[1] = u128_add(r[1], u128_mul_64(a2_2, a4_19));
    r[1] = u128_add(r[1], u128_mul_64(a[3], a3_19));
    r[2] = u128_mul_64(a0_2, a[2]);
    r[2] = u128_add(r[2], u128_mul_64(a[1], a[1]));
    r[2] = u128_add(r[2], u128_mul_64(a3_2, a4_19));
    r[3] = u128_mul_64(a0_2, a[3]);
    r[3] = u128_add(r[3], u128_mul_64(a1_2, a[2]));
    r[3] = u128_add(r[3], u128_mul_64(a[4], a4_19));
    r[4] = u128_mul_64(a0_2, a[4]);
    r[4] = u128_add(r[4], u128_mul_64(a1_2, a[3]));
    r[4] = u128_add(r[4], u128_mul_64(a[2], a[2]));
    return fe_carry_wide(r);
}

inline Fe fe_mul_small(Fe fa, std::uint64_t small) {
    u128 r[5];
    for (int i = 0; i < 5; i++) r[i] = u128_mul_64(fa.v[i], small);
    return fe_carry_wide(r);
}

inline Fe fe_invert(Fe a) {
    Fe t0 = fe_square(a);
    Fe t1 = fe_square(fe_square(t0));
    t1 = fe_mul(a, t1);
    t0 = fe_mul(t0, t1);
    Fe t2 = fe_square(t0);
    t1 = fe_mul(t1, t2);
    t2 = fe_square(t1);
    for (int i = 1; i < 5; i++) t2 = fe_square(t2);
    t1 = fe_mul(t2, t1);
    t2 = fe_square(t1);
    for (int i = 1; i < 10; i++) t2 = fe_square(t2);
    t2 = fe_mul(t2, t1);
    Fe t3 = fe_square(t2);
    for (int i = 1; i < 20; i++) t3 = fe_square(t3);
    t2 = fe_mul(t3, t2);
    t2 = fe_square(t2);
    for (int i = 1; i < 10; i++) t2 = fe_square(t2);
    t1 = fe_mul(t2, t1);
    t2 = fe_square(t1);
    for (int i = 1; i < 50; i++) t2 = fe_square(t2);
    t2 = fe_mul(t2, t1);
    t3 = fe_square(t2);
    for (int i = 1; i < 100; i++) t3 = fe_square(t3);
    t2 = fe_mul(t3, t2);
    t2 = fe_square(t2);
    for (int i = 1; i < 50; i++) t2 = fe_square(t2);
    t1 = fe_mul(t2, t1);
    t1 = fe_square(t1);
    t1 = fe_square(t1);
    t1 = fe_square(t1);
    t1 = fe_square(t1);
    t1 = fe_square(t1);
    return fe_mul(t1, t0);
}

inline void fe_to_bytes(Fe f, std::uint8_t out[32]) {
    Fe t = fe_carry_propagate(fe_carry_propagate(f));
    std::uint64_t* l = t.v;
    std::uint64_t q = (l[0] + 19) >> 51;
    q = (l[1] + q) >> 51;
    q = (l[2] + q) >> 51;
    q = (l[3] + q) >> 51;
    q = (l[4] + q) >> 51;
    l[0] += 19 * q;
    l[1] += l[0] >> 51;
    l[0] &= MASK51;
    l[2] += l[1] >> 51;
    l[1] &= MASK51;
    l[3] += l[2] >> 51;
    l[2] &= MASK51;
    l[4] += l[3] >> 51;
    l[3] &= MASK51;
    l[4] &= MASK51;

    u128 v0 = u128_from_u64(l[0]), v1 = u128_from_u64(l[1]),
         v2 = u128_from_u64(l[2]), v3 = u128_from_u64(l[3]),
         v4 = u128_from_u64(l[4]);
    u128 lo = u128_or(u128_or(v0, u128_shl(v1, 51)), u128_shl(v2, 102));
    u128 hi = u128_or(u128_or(u128_shr(v2, 26), u128_shl(v3, 25)),
                      u128_shl(v4, 76));
    u128_to_le16(lo, out);
    u128_to_le16(hi, out + 16);
}

inline Fe fe_from_bytes(const std::uint8_t bytes[32]) {
    u128 lo = u128_from_le16(bytes);
    u128 hi = u128_from_le16(bytes + 16);
    Fe f;
    f.v[0] = lo.lo & MASK51;
    f.v[1] = u128_shr(lo, 51).lo & MASK51;
    f.v[2] = (u128_shr(lo, 102).lo | (hi.lo << 26)) & MASK51;
    f.v[3] = u128_shr(hi, 25).lo & MASK51;
    f.v[4] = u128_shr(hi, 76).lo & MASK51;
    return f;
}

inline void fe_cswap(std::uint64_t swap, Fe& a, Fe& b) {
    std::uint64_t mask = static_cast<std::uint64_t>(0) - swap;
    for (int i = 0; i < 5; i++) {
        std::uint64_t dummy = mask & (a.v[i] ^ b.v[i]);
        a.v[i] ^= dummy;
        b.v[i] ^= dummy;
    }
}

inline Key clamp_scalar(const Key& k) {
    Key c = k;
    c[0] &= 248;
    c[31] &= 127;
    c[31] |= 64;
    return c;
}

inline Key montgomery_ladder(const Key& k_bytes, const Key& u_bytes) {
    Key k = clamp_scalar(k_bytes);
    Key u_masked = u_bytes;
    u_masked[31] &= 0x7F;
    Fe u = fe_from_bytes(u_masked.data());

    Fe x_1 = u;
    Fe x_2 = fe_one();
    Fe z_2 = fe_zero();
    Fe x_3 = u;
    Fe z_3 = fe_one();
    std::uint64_t swap = 0;

    for (int t = 254; t >= 0; t--) {
        std::uint64_t k_t = static_cast<std::uint64_t>((k[t / 8] >> (t % 8)) & 1);
        swap ^= k_t;
        fe_cswap(swap, x_2, x_3);
        fe_cswap(swap, z_2, z_3);
        swap = k_t;

        Fe a = fe_add(x_2, z_2);
        Fe aa = fe_square(a);
        Fe b = fe_sub(x_2, z_2);
        Fe bb = fe_square(b);
        Fe e = fe_sub(aa, bb);
        Fe c = fe_add(x_3, z_3);
        Fe d = fe_sub(x_3, z_3);
        Fe da = fe_mul(d, a);
        Fe cb = fe_mul(c, b);
        x_3 = fe_square(fe_add(da, cb));
        z_3 = fe_mul(x_1, fe_square(fe_sub(da, cb)));
        x_2 = fe_mul(aa, bb);
        z_2 = fe_mul(e, fe_add(bb, fe_mul_small(e, 121666)));
    }

    fe_cswap(swap, x_2, x_3);
    fe_cswap(swap, z_2, z_3);
    Fe result = fe_mul(x_2, fe_invert(z_2));
    Key out;
    fe_to_bytes(result, out.data());
    return out;
}

}  // namespace detail

// The standard Curve25519 base point (u = 9).
inline constexpr Key BASE_POINT = {9};

// Compute X25519. Returns std::nullopt when the output is all zeros (a low-order
// point input, which a Diffie-Hellman caller must reject).
inline std::optional<Key> x25519(const Key& scalar, const Key& u_coordinate) {
    Key out = detail::montgomery_ladder(scalar, u_coordinate);
    for (std::uint8_t byte : out) {
        if (byte != 0) return out;
    }
    return std::nullopt;
}

// X25519 against the base point (u = 9): derive a public key.
inline std::optional<Key> x25519_base(const Key& scalar) {
    return x25519(scalar, BASE_POINT);
}

// Generate a public key from a private key — an alias of x25519_base.
inline std::optional<Key> generate_keypair(const Key& private_key) {
    return x25519_base(private_key);
}

}  // namespace x25519
}  // namespace ca

#endif  // CA_X25519_HPP
