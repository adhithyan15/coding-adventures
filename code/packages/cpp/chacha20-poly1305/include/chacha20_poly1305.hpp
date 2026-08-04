// chacha20_poly1305.hpp — the ChaCha20 stream cipher and the ChaCha20-Poly1305
// AEAD (RFC 8439), in pure ISO C++17 (header-only). A faithful port of the Rust
// `chacha20-poly1305` crate.
// ===========================================================================
//
// ChaCha20 is a fast stream cipher: a 256-bit key + 96-bit nonce + 32-bit block
// counter produce a keystream that is XORed with the data. Poly1305 is a
// one-time authenticator producing a 16-byte tag. Together they form an AEAD
// (Authenticated Encryption with Associated Data): encryption plus a tag that
// detects tampering with the ciphertext or the associated data.
//
// Poly1305 needs 130-bit modular arithmetic; since ISO C++ has no 128-bit
// integer, this port uses the "poly1305-donna" representation (five 26-bit
// limbs, products taken in std::uint64_t). Output matches the RFC 8439 vectors.
//
//   chacha20_encrypt(...)  — raw stream cipher (also decrypts: XOR again)
//   poly1305_mac(...)      — one-time authenticator
//   aead_encrypt(...)      — returns {ciphertext, tag}
//   aead_decrypt(...)      — returns std::optional<plaintext>; empty if the tag
//                            fails (constant-time comparison)
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef CHACHA20_POLY1305_HPP
#define CHACHA20_POLY1305_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace ca {

namespace detail {

inline std::uint32_t load32(const std::uint8_t *p) {
    return static_cast<std::uint32_t>(p[0]) |
           (static_cast<std::uint32_t>(p[1]) << 8) |
           (static_cast<std::uint32_t>(p[2]) << 16) |
           (static_cast<std::uint32_t>(p[3]) << 24);
}
inline void store32(std::uint8_t *p, std::uint32_t v) {
    p[0] = static_cast<std::uint8_t>(v);
    p[1] = static_cast<std::uint8_t>(v >> 8);
    p[2] = static_cast<std::uint8_t>(v >> 16);
    p[3] = static_cast<std::uint8_t>(v >> 24);
}
inline std::uint32_t rotl32(std::uint32_t x, unsigned n) {
    return (x << n) | (x >> (32 - n));
}

// One ChaCha20 block: 64 keystream bytes for the given key/nonce/counter.
inline void chacha20_block(const std::uint8_t key[32],
                           const std::uint8_t nonce[12], std::uint32_t counter,
                           std::uint8_t out[64]) {
    std::array<std::uint32_t, 16> state{};
    state[0] = 0x61707865u;
    state[1] = 0x3320646eu;
    state[2] = 0x79622d32u;
    state[3] = 0x6b206574u;
    for (unsigned i = 0; i < 8; i++) {
        state[4 + i] = load32(key + i * 4);
    }
    state[12] = counter;
    state[13] = load32(nonce + 0);
    state[14] = load32(nonce + 4);
    state[15] = load32(nonce + 8);

    std::array<std::uint32_t, 16> w = state;
    auto qr = [&w](unsigned a, unsigned b, unsigned c, unsigned d) {
        w[a] += w[b];
        w[d] = rotl32(w[d] ^ w[a], 16);
        w[c] += w[d];
        w[b] = rotl32(w[b] ^ w[c], 12);
        w[a] += w[b];
        w[d] = rotl32(w[d] ^ w[a], 8);
        w[c] += w[d];
        w[b] = rotl32(w[b] ^ w[c], 7);
    };
    for (unsigned i = 0; i < 10; i++) {
        qr(0, 4, 8, 12);
        qr(1, 5, 9, 13);
        qr(2, 6, 10, 14);
        qr(3, 7, 11, 15);
        qr(0, 5, 10, 15);
        qr(1, 6, 11, 12);
        qr(2, 7, 8, 13);
        qr(3, 4, 9, 14);
    }
    for (unsigned i = 0; i < 16; i++) {
        store32(out + i * 4, w[i] + state[i]);
    }
}

// Poly1305 (poly1305-donna, 32-bit): 16-byte authenticator of `msg` under the
// one-time 32-byte `key`.
inline std::array<std::uint8_t, 16> poly1305(const std::uint8_t *msg,
                                             std::size_t len,
                                             const std::uint8_t key[32]) {
    std::uint32_t t0 = load32(key + 0), t1 = load32(key + 4),
                  t2 = load32(key + 8), t3 = load32(key + 12);
    const std::uint32_t r0 = t0 & 0x3ffffffu;
    const std::uint32_t r1 = ((t0 >> 26) | (t1 << 6)) & 0x3ffff03u;
    const std::uint32_t r2 = ((t1 >> 20) | (t2 << 12)) & 0x3ffc0ffu;
    const std::uint32_t r3 = ((t2 >> 14) | (t3 << 18)) & 0x3f03fffu;
    const std::uint32_t r4 = (t3 >> 8) & 0x00fffffu;
    const std::uint32_t s1 = r1 * 5, s2 = r2 * 5, s3 = r3 * 5, s4 = r4 * 5;

    std::uint32_t h0 = 0, h1 = 0, h2 = 0, h3 = 0, h4 = 0;
    std::uint32_t c;
    std::size_t offset = 0;
    while (offset < len) {
        std::array<std::uint8_t, 16> buf{};
        std::uint32_t hibit;
        std::size_t take = len - offset;
        if (take >= 16) {
            take = 16;
            for (std::size_t i = 0; i < 16; i++) {
                buf[i] = msg[offset + i];
            }
            hibit = 1u << 24;
        } else {
            for (std::size_t i = 0; i < take; i++) {
                buf[i] = msg[offset + i];
            }
            buf[take] = 1;
            hibit = 0;
        }
        offset += take;

        t0 = load32(buf.data() + 0);
        t1 = load32(buf.data() + 4);
        t2 = load32(buf.data() + 8);
        t3 = load32(buf.data() + 12);
        h0 += t0 & 0x3ffffffu;
        h1 += ((t0 >> 26) | (t1 << 6)) & 0x3ffffffu;
        h2 += ((t1 >> 20) | (t2 << 12)) & 0x3ffffffu;
        h3 += ((t2 >> 14) | (t3 << 18)) & 0x3ffffffu;
        h4 += (t3 >> 8) | hibit;

        std::uint64_t d0 = static_cast<std::uint64_t>(h0) * r0 +
                           static_cast<std::uint64_t>(h1) * s4 +
                           static_cast<std::uint64_t>(h2) * s3 +
                           static_cast<std::uint64_t>(h3) * s2 +
                           static_cast<std::uint64_t>(h4) * s1;
        std::uint64_t d1 = static_cast<std::uint64_t>(h0) * r1 +
                           static_cast<std::uint64_t>(h1) * r0 +
                           static_cast<std::uint64_t>(h2) * s4 +
                           static_cast<std::uint64_t>(h3) * s3 +
                           static_cast<std::uint64_t>(h4) * s2;
        std::uint64_t d2 = static_cast<std::uint64_t>(h0) * r2 +
                           static_cast<std::uint64_t>(h1) * r1 +
                           static_cast<std::uint64_t>(h2) * r0 +
                           static_cast<std::uint64_t>(h3) * s4 +
                           static_cast<std::uint64_t>(h4) * s3;
        std::uint64_t d3 = static_cast<std::uint64_t>(h0) * r3 +
                           static_cast<std::uint64_t>(h1) * r2 +
                           static_cast<std::uint64_t>(h2) * r1 +
                           static_cast<std::uint64_t>(h3) * r0 +
                           static_cast<std::uint64_t>(h4) * s4;
        std::uint64_t d4 = static_cast<std::uint64_t>(h0) * r4 +
                           static_cast<std::uint64_t>(h1) * r3 +
                           static_cast<std::uint64_t>(h2) * r2 +
                           static_cast<std::uint64_t>(h3) * r1 +
                           static_cast<std::uint64_t>(h4) * r0;

        c = static_cast<std::uint32_t>(d0 >> 26);
        h0 = static_cast<std::uint32_t>(d0) & 0x3ffffffu;
        d1 += c;
        c = static_cast<std::uint32_t>(d1 >> 26);
        h1 = static_cast<std::uint32_t>(d1) & 0x3ffffffu;
        d2 += c;
        c = static_cast<std::uint32_t>(d2 >> 26);
        h2 = static_cast<std::uint32_t>(d2) & 0x3ffffffu;
        d3 += c;
        c = static_cast<std::uint32_t>(d3 >> 26);
        h3 = static_cast<std::uint32_t>(d3) & 0x3ffffffu;
        d4 += c;
        c = static_cast<std::uint32_t>(d4 >> 26);
        h4 = static_cast<std::uint32_t>(d4) & 0x3ffffffu;
        h0 += c * 5;
        c = h0 >> 26;
        h0 &= 0x3ffffffu;
        h1 += c;
    }

    c = h1 >> 26;
    h1 &= 0x3ffffffu;
    h2 += c;
    c = h2 >> 26;
    h2 &= 0x3ffffffu;
    h3 += c;
    c = h3 >> 26;
    h3 &= 0x3ffffffu;
    h4 += c;
    c = h4 >> 26;
    h4 &= 0x3ffffffu;
    h0 += c * 5;
    c = h0 >> 26;
    h0 &= 0x3ffffffu;
    h1 += c;

    std::uint32_t g0 = h0 + 5;
    c = g0 >> 26;
    g0 &= 0x3ffffffu;
    std::uint32_t g1 = h1 + c;
    c = g1 >> 26;
    g1 &= 0x3ffffffu;
    std::uint32_t g2 = h2 + c;
    c = g2 >> 26;
    g2 &= 0x3ffffffu;
    std::uint32_t g3 = h3 + c;
    c = g3 >> 26;
    g3 &= 0x3ffffffu;
    std::uint32_t g4 = h4 + c - (1u << 26);

    std::uint32_t mask = (g4 >> 31) - 1;
    g0 &= mask;
    g1 &= mask;
    g2 &= mask;
    g3 &= mask;
    g4 &= mask;
    mask = ~mask;
    h0 = (h0 & mask) | g0;
    h1 = (h1 & mask) | g1;
    h2 = (h2 & mask) | g2;
    h3 = (h3 & mask) | g3;
    h4 = (h4 & mask) | g4;

    h0 = (h0 | (h1 << 26)) & 0xffffffffu;
    h1 = ((h1 >> 6) | (h2 << 20)) & 0xffffffffu;
    h2 = ((h2 >> 12) | (h3 << 14)) & 0xffffffffu;
    h3 = ((h3 >> 18) | (h4 << 8)) & 0xffffffffu;

    std::uint64_t f = static_cast<std::uint64_t>(h0) + load32(key + 16);
    h0 = static_cast<std::uint32_t>(f);
    f = static_cast<std::uint64_t>(h1) + load32(key + 20) + (f >> 32);
    h1 = static_cast<std::uint32_t>(f);
    f = static_cast<std::uint64_t>(h2) + load32(key + 24) + (f >> 32);
    h2 = static_cast<std::uint32_t>(f);
    f = static_cast<std::uint64_t>(h3) + load32(key + 28) + (f >> 32);
    h3 = static_cast<std::uint32_t>(f);

    std::array<std::uint8_t, 16> tag{};
    store32(tag.data() + 0, h0);
    store32(tag.data() + 4, h1);
    store32(tag.data() + 8, h2);
    store32(tag.data() + 12, h3);
    return tag;
}

inline std::size_t pad16(std::size_t n) { return (n % 16 == 0) ? 0 : 16 - n % 16; }

// aad || pad || ciphertext || pad || le64(aad_len) || le64(ct_len)
inline std::vector<std::uint8_t> build_mac_data(const std::uint8_t *aad,
                                                std::size_t aad_len,
                                                const std::uint8_t *ct,
                                                std::size_t ct_len) {
    std::vector<std::uint8_t> data;
    data.reserve(aad_len + pad16(aad_len) + ct_len + pad16(ct_len) + 16);
    data.insert(data.end(), aad, aad + aad_len);
    data.insert(data.end(), pad16(aad_len), 0);
    data.insert(data.end(), ct, ct + ct_len);
    data.insert(data.end(), pad16(ct_len), 0);
    for (unsigned i = 0; i < 8; i++) {
        data.push_back(static_cast<std::uint8_t>(
            static_cast<std::uint64_t>(aad_len) >> (i * 8)));
    }
    for (unsigned i = 0; i < 8; i++) {
        data.push_back(static_cast<std::uint8_t>(
            static_cast<std::uint64_t>(ct_len) >> (i * 8)));
    }
    return data;
}

} // namespace detail

// chacha20_encrypt — XOR `input` with the ChaCha20 keystream. Decryption is the
// same operation. Returns the ciphertext (same length as input).
inline std::vector<std::uint8_t> chacha20_encrypt(const std::uint8_t *input,
                                                  std::size_t len,
                                                  const std::uint8_t key[32],
                                                  const std::uint8_t nonce[12],
                                                  std::uint32_t counter) {
    std::vector<std::uint8_t> out(len);
    std::uint8_t block[64];
    std::size_t offset = 0;
    while (offset < len) {
        std::size_t take = len - offset;
        if (take > 64) {
            take = 64;
        }
        detail::chacha20_block(key, nonce, counter, block);
        counter++;
        for (std::size_t i = 0; i < take; i++) {
            out[offset + i] = input[offset + i] ^ block[i];
        }
        offset += take;
    }
    return out;
}
inline std::vector<std::uint8_t> chacha20_encrypt(
    const std::vector<std::uint8_t> &input, const std::uint8_t key[32],
    const std::uint8_t nonce[12], std::uint32_t counter) {
    return chacha20_encrypt(input.data(), input.size(), key, nonce, counter);
}

// poly1305_mac — 16-byte one-time authenticator of `message` under `key`.
inline std::array<std::uint8_t, 16> poly1305_mac(const std::uint8_t *message,
                                                 std::size_t len,
                                                 const std::uint8_t key[32]) {
    return detail::poly1305(message, len, key);
}

// The AEAD result: ciphertext plus its 16-byte authentication tag.
struct aead_result {
    std::vector<std::uint8_t> ciphertext;
    std::array<std::uint8_t, 16> tag;
};

// aead_encrypt — ChaCha20-Poly1305 AEAD (RFC 8439 §2.8).
inline aead_result aead_encrypt(const std::uint8_t *plaintext,
                                std::size_t plaintext_len,
                                const std::uint8_t key[32],
                                const std::uint8_t nonce[12],
                                const std::uint8_t *aad, std::size_t aad_len) {
    std::uint8_t block[64];
    detail::chacha20_block(key, nonce, 0, block);
    std::uint8_t poly_key[32];
    for (unsigned i = 0; i < 32; i++) {
        poly_key[i] = block[i];
    }
    aead_result r;
    r.ciphertext = chacha20_encrypt(plaintext, plaintext_len, key, nonce, 1);
    std::vector<std::uint8_t> mac_data = detail::build_mac_data(
        aad, aad_len, r.ciphertext.data(), r.ciphertext.size());
    r.tag = detail::poly1305(mac_data.data(), mac_data.size(), poly_key);
    return r;
}
inline aead_result aead_encrypt(const std::vector<std::uint8_t> &plaintext,
                                const std::uint8_t key[32],
                                const std::uint8_t nonce[12],
                                const std::vector<std::uint8_t> &aad) {
    return aead_encrypt(plaintext.data(), plaintext.size(), key, nonce,
                        aad.data(), aad.size());
}

// aead_decrypt — verify `tag` (constant-time) and decrypt. Returns the
// plaintext, or std::nullopt if the tag is invalid.
inline std::optional<std::vector<std::uint8_t>> aead_decrypt(
    const std::uint8_t *ciphertext, std::size_t ciphertext_len,
    const std::uint8_t key[32], const std::uint8_t nonce[12],
    const std::uint8_t *aad, std::size_t aad_len,
    const std::array<std::uint8_t, 16> &tag) {
    std::uint8_t block[64];
    detail::chacha20_block(key, nonce, 0, block);
    std::uint8_t poly_key[32];
    for (unsigned i = 0; i < 32; i++) {
        poly_key[i] = block[i];
    }
    std::vector<std::uint8_t> mac_data =
        detail::build_mac_data(aad, aad_len, ciphertext, ciphertext_len);
    std::array<std::uint8_t, 16> expected =
        detail::poly1305(mac_data.data(), mac_data.size(), poly_key);

    std::uint8_t diff = 0;
    for (unsigned i = 0; i < 16; i++) {
        diff = static_cast<std::uint8_t>(diff | (expected[i] ^ tag[i]));
    }
    if (diff != 0) {
        return std::nullopt;
    }
    return chacha20_encrypt(ciphertext, ciphertext_len, key, nonce, 1);
}
inline std::optional<std::vector<std::uint8_t>> aead_decrypt(
    const std::vector<std::uint8_t> &ciphertext, const std::uint8_t key[32],
    const std::uint8_t nonce[12], const std::vector<std::uint8_t> &aad,
    const std::array<std::uint8_t, 16> &tag) {
    return aead_decrypt(ciphertext.data(), ciphertext.size(), key, nonce,
                        aad.data(), aad.size(), tag);
}

} // namespace ca

#endif // CHACHA20_POLY1305_HPP
