// argon2d.hpp — Argon2d, data-dependent memory-hard password hashing (RFC 9106),
// in pure ISO C++17 (header-only), in namespace ca. A faithful port of the Rust
// `argon2d` crate.
// ===========================================================================
//
// Argon2 fills a large memory matrix (memory_cost KiB) with a BLAKE2b-derived
// compression function, reading it back in a data-dependent order so an attacker
// cannot trade memory for speed. The *d* variant derives every reference index
// from the previous block's first 64 bits — maximal GPU/ASIC resistance, but a
// timing side channel, so it suits only threat models without side-channel
// attackers. Prefer Argon2id for password hashing.
//
//   H0        = BLAKE2b(params || pass || salt || key || ad)
//   B[i][0/1] = H'(H0 || 0/1 || i)
//   B[i][j]   = G(B[i][j-1], B[l'][z'])       (XOR into place after pass 0)
//   tag       = H'(XOR of the last column across lanes)
//
// Built on the sibling header-only `blake2b` package. Invalid parameters throw
// std::invalid_argument.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_ARGON2D_HPP
#define CA_ARGON2D_HPP

#include <cstddef>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

#include "blake2b.hpp"  // sibling header-only package (include path via run.sh)

namespace ca {
namespace argon2d_detail {

constexpr std::uint64_t mask32 = 0xFFFFFFFFull;
constexpr std::size_t block_size = 1024;
constexpr std::size_t block_words = 128;  // block_size / 8
constexpr std::size_t sync_points = 4;
constexpr std::uint32_t type_d = 0;

inline std::uint64_t rotr64(std::uint64_t x, unsigned n) {
    return (x >> n) | (x << (64 - n));
}

inline void g_b(std::uint64_t* v, std::size_t a, std::size_t b, std::size_t c,
                std::size_t d) {
    std::uint64_t va = v[a], vb = v[b], vc = v[c], vd = v[d];
    va = va + vb + 2ull * (va & mask32) * (vb & mask32);
    vd = rotr64(vd ^ va, 32);
    vc = vc + vd + 2ull * (vc & mask32) * (vd & mask32);
    vb = rotr64(vb ^ vc, 24);
    va = va + vb + 2ull * (va & mask32) * (vb & mask32);
    vd = rotr64(vd ^ va, 16);
    vc = vc + vd + 2ull * (vc & mask32) * (vd & mask32);
    vb = rotr64(vb ^ vc, 63);
    v[a] = va;
    v[b] = vb;
    v[c] = vc;
    v[d] = vd;
}

inline void permutation_p(std::uint64_t* v) {
    g_b(v, 0, 4, 8, 12);
    g_b(v, 1, 5, 9, 13);
    g_b(v, 2, 6, 10, 14);
    g_b(v, 3, 7, 11, 15);
    g_b(v, 0, 5, 10, 15);
    g_b(v, 1, 6, 11, 12);
    g_b(v, 2, 7, 8, 13);
    g_b(v, 3, 4, 9, 14);
}

// Compression G(x, y); `out` must be distinct from x and y (block_words words).
inline void compress(const std::uint64_t* x, const std::uint64_t* y,
                     std::uint64_t* out) {
    std::uint64_t r[block_words];
    std::uint64_t q[block_words];
    std::uint64_t col[16];
    for (std::size_t i = 0; i < block_words; ++i) {
        r[i] = x[i] ^ y[i];
        q[i] = r[i];
    }
    for (std::size_t i = 0; i < 8; ++i) {
        permutation_p(q + i * 16);
    }
    for (std::size_t c = 0; c < 8; ++c) {
        for (std::size_t row = 0; row < 8; ++row) {
            col[2 * row] = q[row * 16 + 2 * c];
            col[2 * row + 1] = q[row * 16 + 2 * c + 1];
        }
        permutation_p(col);
        for (std::size_t row = 0; row < 8; ++row) {
            q[row * 16 + 2 * c] = col[2 * row];
            q[row * 16 + 2 * c + 1] = col[2 * row + 1];
        }
    }
    for (std::size_t i = 0; i < block_words; ++i) {
        out[i] = r[i] ^ q[i];
    }
}

inline void put_le32(std::vector<std::uint8_t>& v, std::uint32_t n) {
    v.push_back(static_cast<std::uint8_t>(n & 0xFF));
    v.push_back(static_cast<std::uint8_t>((n >> 8) & 0xFF));
    v.push_back(static_cast<std::uint8_t>((n >> 16) & 0xFF));
    v.push_back(static_cast<std::uint8_t>((n >> 24) & 0xFF));
}

inline std::uint64_t load_le64(const std::uint8_t* p) {
    std::uint64_t w = 0;
    for (int i = 0; i < 8; ++i) {
        w |= static_cast<std::uint64_t>(p[i]) << (8 * i);
    }
    return w;
}

// H' variable-length BLAKE2b extender (RFC 9106 §3.3): t bytes of H'(x).
inline std::vector<std::uint8_t> blake2b_long(std::uint32_t t,
                                              const std::vector<std::uint8_t>& x) {
    if (t == 0) {
        throw std::invalid_argument("argon2d: H' output length must be positive");
    }
    std::vector<std::uint8_t> input;
    put_le32(input, t);
    input.insert(input.end(), x.begin(), x.end());
    if (t <= 64) {
        return ca::blake2b(input.data(), input.size(), t);
    }
    std::uint32_t r = (t + 31u) / 32u - 2u;
    std::vector<std::uint8_t> v = ca::blake2b(input.data(), input.size(), 64);
    std::vector<std::uint8_t> out;
    out.insert(out.end(), v.begin(), v.begin() + 32);
    for (std::uint32_t k = 1; k < r; ++k) {
        v = ca::blake2b(v.data(), v.size(), 64);
        out.insert(out.end(), v.begin(), v.begin() + 32);
    }
    std::uint32_t final_size = t - 32u * r;
    std::vector<std::uint8_t> last = ca::blake2b(v.data(), v.size(), final_size);
    out.insert(out.end(), last.begin(), last.end());
    return out;
}

inline std::size_t index_alpha(std::uint64_t j1, std::size_t r, std::size_t sl,
                               std::size_t c, bool same_lane, std::size_t q,
                               std::size_t sl_len) {
    std::size_t w;
    std::size_t start;
    if (r == 0 && sl == 0) {
        w = c - 1;
        start = 0;
    } else if (r == 0) {
        w = same_lane ? sl * sl_len + c - 1
                      : (c == 0 ? sl * sl_len - 1 : sl * sl_len);
        start = 0;
    } else {
        w = same_lane ? q - sl_len + c - 1
                      : (c == 0 ? q - sl_len - 1 : q - sl_len);
        start = ((sl + 1) * sl_len) % q;
    }
    std::uint64_t x = (j1 * j1) >> 32;
    std::uint64_t y = (static_cast<std::uint64_t>(w) * x) >> 32;
    std::int64_t rel = static_cast<std::int64_t>(w) - 1 - static_cast<std::int64_t>(y);
    std::int64_t res = static_cast<std::int64_t>(start) + rel;
    res %= static_cast<std::int64_t>(q);
    if (res < 0) {
        res += static_cast<std::int64_t>(q);
    }
    return static_cast<std::size_t>(res);
}

}  // namespace argon2d_detail

// Optional inputs for argon2d (key / associated data / version override).
struct Argon2dOptions {
    std::vector<std::uint8_t> key;
    std::vector<std::uint8_t> associated_data;
    std::optional<std::uint32_t> version;
};

constexpr std::uint32_t argon2d_version = 0x13;

// argon2d — compute the Argon2d tag (RFC 9106). Throws std::invalid_argument on
// any invalid parameter.
inline std::vector<std::uint8_t> argon2d(const std::vector<std::uint8_t>& password,
                                         const std::vector<std::uint8_t>& salt,
                                         std::uint32_t time_cost,
                                         std::uint32_t memory_cost,
                                         std::uint32_t parallelism,
                                         std::uint32_t tag_length,
                                         const Argon2dOptions& opts = {}) {
    namespace d = argon2d_detail;
    const std::vector<std::uint8_t>& key = opts.key;
    const std::vector<std::uint8_t>& ad = opts.associated_data;
    std::uint32_t version = opts.version.value_or(argon2d_version);

    // Validation (RFC 9106 §3.1).
    if (static_cast<std::uint64_t>(password.size()) > 0xFFFFFFFFull) {
        throw std::invalid_argument("argon2d: password length must fit in 32 bits");
    }
    if (salt.size() < 8) {
        throw std::invalid_argument("argon2d: salt must be at least 8 bytes");
    }
    if (static_cast<std::uint64_t>(salt.size()) > 0xFFFFFFFFull) {
        throw std::invalid_argument("argon2d: salt length must fit in 32 bits");
    }
    if (static_cast<std::uint64_t>(key.size()) > 0xFFFFFFFFull) {
        throw std::invalid_argument("argon2d: key length must fit in 32 bits");
    }
    if (static_cast<std::uint64_t>(ad.size()) > 0xFFFFFFFFull) {
        throw std::invalid_argument("argon2d: associated data must fit in 32 bits");
    }
    if (tag_length < 4) {
        throw std::invalid_argument("argon2d: tag_length must be >= 4");
    }
    if (parallelism < 1 || parallelism > 0xFFFFFF) {
        throw std::invalid_argument("argon2d: parallelism must be in [1, 2^24-1]");
    }
    if (memory_cost < 8 * parallelism) {
        throw std::invalid_argument("argon2d: memory_cost must be >= 8*parallelism");
    }
    if (time_cost < 1) {
        throw std::invalid_argument("argon2d: time_cost must be >= 1");
    }
    if (version != argon2d_version) {
        throw std::invalid_argument("argon2d: only Argon2 v1.3 (0x13) is supported");
    }

    std::size_t p = parallelism;
    std::size_t t = time_cost;
    std::size_t segment_length = memory_cost / (d::sync_points * parallelism);
    std::size_t m_prime = segment_length * d::sync_points * p;
    std::size_t q = m_prime / p;
    std::size_t sl_len = segment_length;

    // H0 = BLAKE2b(params || pass || salt || key || ad).
    std::vector<std::uint8_t> h0_in;
    d::put_le32(h0_in, static_cast<std::uint32_t>(p));
    d::put_le32(h0_in, tag_length);
    d::put_le32(h0_in, memory_cost);
    d::put_le32(h0_in, static_cast<std::uint32_t>(t));
    d::put_le32(h0_in, version);
    d::put_le32(h0_in, d::type_d);
    d::put_le32(h0_in, static_cast<std::uint32_t>(password.size()));
    h0_in.insert(h0_in.end(), password.begin(), password.end());
    d::put_le32(h0_in, static_cast<std::uint32_t>(salt.size()));
    h0_in.insert(h0_in.end(), salt.begin(), salt.end());
    d::put_le32(h0_in, static_cast<std::uint32_t>(key.size()));
    h0_in.insert(h0_in.end(), key.begin(), key.end());
    d::put_le32(h0_in, static_cast<std::uint32_t>(ad.size()));
    h0_in.insert(h0_in.end(), ad.begin(), ad.end());
    std::vector<std::uint8_t> h0 = ca::blake2b(h0_in.data(), h0_in.size(), 64);

    // Working matrix: p*q blocks of block_words words.
    std::vector<std::uint64_t> memory(m_prime * d::block_words, 0);
    auto block_at = [&](std::size_t lane, std::size_t col) -> std::uint64_t* {
        return memory.data() + (lane * q + col) * d::block_words;
    };

    // First two columns of each lane.
    for (std::size_t i = 0; i < p; ++i) {
        for (std::uint32_t col = 0; col < 2; ++col) {
            std::vector<std::uint8_t> in0 = h0;
            d::put_le32(in0, col);
            d::put_le32(in0, static_cast<std::uint32_t>(i));
            std::vector<std::uint8_t> b =
                d::blake2b_long(static_cast<std::uint32_t>(d::block_size), in0);
            std::uint64_t* dst = block_at(i, col);
            for (std::size_t w = 0; w < d::block_words; ++w) {
                dst[w] = d::load_le64(b.data() + w * 8);
            }
        }
    }

    // t passes over 4 segments over p lanes (data-dependent addressing).
    std::uint64_t newb[d::block_words];
    for (std::size_t r = 0; r < t; ++r) {
        for (std::size_t sl = 0; sl < d::sync_points; ++sl) {
            for (std::size_t lane = 0; lane < p; ++lane) {
                std::size_t starting_c = (r == 0 && sl == 0) ? 2 : 0;
                for (std::size_t i = starting_c; i < sl_len; ++i) {
                    std::size_t col = sl * sl_len + i;
                    std::size_t prev_col = (col == 0) ? (q - 1) : (col - 1);
                    const std::uint64_t* prev_block = block_at(lane, prev_col);
                    std::uint64_t pseudo = prev_block[0];
                    std::uint64_t j1 = pseudo & d::mask32;
                    std::uint64_t j2 = (pseudo >> 32) & d::mask32;
                    std::size_t l_prime =
                        (r == 0 && sl == 0)
                            ? lane
                            : static_cast<std::size_t>(j2 % static_cast<std::uint64_t>(p));
                    std::size_t z_prime = d::index_alpha(j1, r, sl, i,
                                                         l_prime == lane, q, sl_len);
                    const std::uint64_t* ref_block = block_at(l_prime, z_prime);
                    std::uint64_t* dst = block_at(lane, col);
                    d::compress(prev_block, ref_block, newb);
                    if (r == 0) {
                        for (std::size_t w = 0; w < d::block_words; ++w) {
                            dst[w] = newb[w];
                        }
                    } else {
                        for (std::size_t w = 0; w < d::block_words; ++w) {
                            dst[w] ^= newb[w];
                        }
                    }
                }
            }
        }
    }

    // Final block = XOR of the last column across lanes.
    std::uint64_t final_block[d::block_words];
    const std::uint64_t* first = block_at(0, q - 1);
    for (std::size_t w = 0; w < d::block_words; ++w) {
        final_block[w] = first[w];
    }
    for (std::size_t lane = 1; lane < p; ++lane) {
        const std::uint64_t* lb = block_at(lane, q - 1);
        for (std::size_t w = 0; w < d::block_words; ++w) {
            final_block[w] ^= lb[w];
        }
    }
    std::vector<std::uint8_t> final_bytes(d::block_size);
    for (std::size_t w = 0; w < d::block_words; ++w) {
        for (int b = 0; b < 8; ++b) {
            final_bytes[w * 8 + static_cast<std::size_t>(b)] =
                static_cast<std::uint8_t>((final_block[w] >> (8 * b)) & 0xFF);
        }
    }
    return d::blake2b_long(tag_length, final_bytes);
}

// argon2d_hex — like argon2d but returns a lowercase hex string.
inline std::string argon2d_hex(const std::vector<std::uint8_t>& password,
                               const std::vector<std::uint8_t>& salt,
                               std::uint32_t time_cost, std::uint32_t memory_cost,
                               std::uint32_t parallelism, std::uint32_t tag_length,
                               const Argon2dOptions& opts = {}) {
    std::vector<std::uint8_t> tag = argon2d(password, salt, time_cost, memory_cost,
                                            parallelism, tag_length, opts);
    static const char* digits = "0123456789abcdef";
    std::string s;
    s.reserve(tag.size() * 2);
    for (std::uint8_t b : tag) {
        s.push_back(digits[b >> 4]);
        s.push_back(digits[b & 0x0F]);
    }
    return s;
}

}  // namespace ca

#endif  // CA_ARGON2D_HPP
