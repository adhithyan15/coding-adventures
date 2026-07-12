// argon2id.hpp — Argon2id, hybrid memory-hard password hashing (RFC 9106), in
// pure ISO C++17 (header-only), in namespace ca. A faithful port of the Rust
// `argon2id` crate.
// ===========================================================================
//
// Argon2 fills a large memory matrix (memory_cost KiB) with a BLAKE2b-derived
// compression function. The *id* variant combines Argon2i and Argon2d: the first
// two slices of the first pass use data-INDEPENDENT addressing (an address
// stream), and everything after uses data-DEPENDENT addressing (the previous
// block). This gives some side-channel resistance for the early passes while
// keeping the GPU/ASIC resistance of the data-dependent mode — the RECOMMENDED
// variant for password hashing (RFC 9106 §4).
//
// The address stream (RFC 9106 §3.4.2) yields (J1, J2) pairs by running the
// compression function twice over a counter block.
//
// Built on the sibling header-only `blake2b` package. Invalid parameters throw
// std::invalid_argument.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_ARGON2ID_HPP
#define CA_ARGON2ID_HPP

#include <cstddef>
#include <cstdint>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

#include "blake2b.hpp"  // sibling header-only package (include path via run.sh)

namespace ca {
namespace argon2id_detail {

constexpr std::uint64_t mask32 = 0xFFFFFFFFull;
constexpr std::size_t block_size = 1024;
constexpr std::size_t block_words = 128;  // block_size / 8
constexpr std::size_t sync_points = 4;
constexpr std::size_t addresses_per_block = block_words;
constexpr std::uint32_t type_id = 2;

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

inline std::vector<std::uint8_t> blake2b_long(std::uint32_t t,
                                              const std::vector<std::uint8_t>& x) {
    if (t == 0) {
        throw std::invalid_argument("argon2id: H' output length must be positive");
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

// Data-independent (J1, J2) generator (RFC 9106 §3.4.2).
struct AddressStream {
    std::uint64_t r, lane, sl, m_prime, t, counter = 0;
    std::uint64_t buf[block_words];
    std::size_t idx = addresses_per_block;  // refill on first call

    AddressStream(std::size_t r_, std::size_t lane_, std::size_t sl_,
                  std::size_t m_prime_, std::size_t t_)
        : r(r_), lane(lane_), sl(sl_), m_prime(m_prime_), t(t_) {}

    std::uint64_t next() {
        if (idx >= addresses_per_block) {
            std::uint64_t zero[block_words] = {0};
            std::uint64_t input_block[block_words] = {0};
            std::uint64_t once[block_words];
            ++counter;
            input_block[0] = r;
            input_block[1] = lane;
            input_block[2] = sl;
            input_block[3] = m_prime;
            input_block[4] = t;
            input_block[5] = type_id;
            input_block[6] = counter;
            compress(zero, input_block, once);
            compress(zero, once, buf);
            idx = 0;
        }
        return buf[idx++];
    }
};

}  // namespace argon2id_detail

// Optional inputs for argon2id (key / associated data / version override).
struct Argon2idOptions {
    std::vector<std::uint8_t> key;
    std::vector<std::uint8_t> associated_data;
    std::optional<std::uint32_t> version;
};

constexpr std::uint32_t argon2id_version = 0x13;

// argon2id — compute the Argon2id tag (RFC 9106). Throws std::invalid_argument on
// any invalid parameter.
inline std::vector<std::uint8_t> argon2id(const std::vector<std::uint8_t>& password,
                                         const std::vector<std::uint8_t>& salt,
                                         std::uint32_t time_cost,
                                         std::uint32_t memory_cost,
                                         std::uint32_t parallelism,
                                         std::uint32_t tag_length,
                                         const Argon2idOptions& opts = {}) {
    namespace d = argon2id_detail;
    const std::vector<std::uint8_t>& key = opts.key;
    const std::vector<std::uint8_t>& ad = opts.associated_data;
    std::uint32_t version = opts.version.value_or(argon2id_version);

    if (static_cast<std::uint64_t>(password.size()) > 0xFFFFFFFFull) {
        throw std::invalid_argument("argon2id: password length must fit in 32 bits");
    }
    if (salt.size() < 8) {
        throw std::invalid_argument("argon2id: salt must be at least 8 bytes");
    }
    if (static_cast<std::uint64_t>(salt.size()) > 0xFFFFFFFFull) {
        throw std::invalid_argument("argon2id: salt length must fit in 32 bits");
    }
    if (static_cast<std::uint64_t>(key.size()) > 0xFFFFFFFFull) {
        throw std::invalid_argument("argon2id: key length must fit in 32 bits");
    }
    if (static_cast<std::uint64_t>(ad.size()) > 0xFFFFFFFFull) {
        throw std::invalid_argument("argon2id: associated data must fit in 32 bits");
    }
    if (tag_length < 4) {
        throw std::invalid_argument("argon2id: tag_length must be >= 4");
    }
    if (parallelism < 1 || parallelism > 0xFFFFFF) {
        throw std::invalid_argument("argon2id: parallelism must be in [1, 2^24-1]");
    }
    if (memory_cost < 8 * parallelism) {
        throw std::invalid_argument("argon2id: memory_cost must be >= 8*parallelism");
    }
    if (time_cost < 1) {
        throw std::invalid_argument("argon2id: time_cost must be >= 1");
    }
    if (version != argon2id_version) {
        throw std::invalid_argument("argon2id: only Argon2 v1.3 (0x13) is supported");
    }

    std::size_t p = parallelism;
    std::size_t t = time_cost;
    std::size_t segment_length = memory_cost / (d::sync_points * parallelism);
    std::size_t m_prime = segment_length * d::sync_points * p;
    std::size_t q = m_prime / p;
    std::size_t sl_len = segment_length;

    std::vector<std::uint8_t> h0_in;
    d::put_le32(h0_in, static_cast<std::uint32_t>(p));
    d::put_le32(h0_in, tag_length);
    d::put_le32(h0_in, memory_cost);
    d::put_le32(h0_in, static_cast<std::uint32_t>(t));
    d::put_le32(h0_in, version);
    d::put_le32(h0_in, d::type_id);
    d::put_le32(h0_in, static_cast<std::uint32_t>(password.size()));
    h0_in.insert(h0_in.end(), password.begin(), password.end());
    d::put_le32(h0_in, static_cast<std::uint32_t>(salt.size()));
    h0_in.insert(h0_in.end(), salt.begin(), salt.end());
    d::put_le32(h0_in, static_cast<std::uint32_t>(key.size()));
    h0_in.insert(h0_in.end(), key.begin(), key.end());
    d::put_le32(h0_in, static_cast<std::uint32_t>(ad.size()));
    h0_in.insert(h0_in.end(), ad.begin(), ad.end());
    std::vector<std::uint8_t> h0 = ca::blake2b(h0_in.data(), h0_in.size(), 64);

    std::vector<std::uint64_t> memory(m_prime * d::block_words, 0);
    auto block_at = [&](std::size_t lane, std::size_t col) -> std::uint64_t* {
        return memory.data() + (lane * q + col) * d::block_words;
    };

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

    std::uint64_t newb[d::block_words];
    for (std::size_t r = 0; r < t; ++r) {
        for (std::size_t sl = 0; sl < d::sync_points; ++sl) {
            for (std::size_t lane = 0; lane < p; ++lane) {
                std::size_t starting_c = (r == 0 && sl == 0) ? 2 : 0;
                // Argon2id: data-independent addressing for the first two slices
                // of the first pass, data-dependent everywhere else (§3.4).
                bool di = (r == 0 && sl < 2);
                d::AddressStream addr(r, lane, sl, m_prime, t);
                if (di) {
                    for (std::size_t skip = 0; skip < starting_c; ++skip) {
                        addr.next();  // keep the stream in phase
                    }
                }
                for (std::size_t i = starting_c; i < sl_len; ++i) {
                    std::size_t col = sl * sl_len + i;
                    std::size_t prev_col = (col == 0) ? (q - 1) : (col - 1);
                    const std::uint64_t* prev_block = block_at(lane, prev_col);
                    std::uint64_t pseudo = di ? addr.next() : prev_block[0];
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

// argon2id_hex — like argon2id but returns a lowercase hex string.
inline std::string argon2id_hex(const std::vector<std::uint8_t>& password,
                               const std::vector<std::uint8_t>& salt,
                               std::uint32_t time_cost, std::uint32_t memory_cost,
                               std::uint32_t parallelism, std::uint32_t tag_length,
                               const Argon2idOptions& opts = {}) {
    std::vector<std::uint8_t> tag = argon2id(password, salt, time_cost, memory_cost,
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

#endif  // CA_ARGON2ID_HPP
