// blake2b.hpp — the BLAKE2b hash (RFC 7693), in pure ISO C++17 (header-only). A
// faithful port of the Rust `blake2b` crate.
// ===========================================================================
//
// A fast cryptographic hash producing up to 64 bytes, with optional keying (a
// MAC), 16-byte salt, and 16-byte personalization. 64-bit words, 128-byte
// blocks, 12 rounds. Output matches the published RFC 7693 test vectors.
// Streaming `hasher` plus one-shot helpers.
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef BLAKE2B_HPP
#define BLAKE2B_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

namespace ca {

class blake2b_hasher {
public:
    static constexpr std::size_t block_size = 128;
    static constexpr std::size_t max_digest = 64;
    static constexpr std::size_t max_key = 64;

    // digest_size in 1..64; key up to 64 bytes (empty for unkeyed); salt and
    // personal are each exactly 16 bytes or empty. Throws std::invalid_argument
    // on a bad parameter.
    explicit blake2b_hasher(std::size_t digest_size = 64,
                            const std::vector<std::uint8_t> &key = {},
                            const std::vector<std::uint8_t> &salt = {},
                            const std::vector<std::uint8_t> &personal = {}) {
        if (digest_size < 1 || digest_size > max_digest ||
            key.size() > max_key || (!salt.empty() && salt.size() != 16) ||
            (!personal.empty() && personal.size() != 16)) {
            throw std::invalid_argument("blake2b: invalid parameter");
        }
        digest_size_ = digest_size;
        std::array<std::uint8_t, 64> p{};
        p[0] = static_cast<std::uint8_t>(digest_size);
        p[1] = static_cast<std::uint8_t>(key.size());
        p[2] = 1;
        p[3] = 1;
        for (std::size_t i = 0; i < salt.size(); i++) {
            p[32 + i] = salt[i];
        }
        for (std::size_t i = 0; i < personal.size(); i++) {
            p[48 + i] = personal[i];
        }
        for (unsigned i = 0; i < 8; i++) {
            state_[i] = iv()[i] ^ load64(&p[i * 8]);
        }
        count_low_ = count_high_ = 0;
        if (!key.empty()) {
            buffer_.fill(0);
            for (std::size_t i = 0; i < key.size(); i++) {
                buffer_[i] = key[i];
            }
            buffer_len_ = block_size;
        } else {
            buffer_len_ = 0;
        }
    }

    void update(const void *data, std::size_t len) {
        const std::uint8_t *bytes = static_cast<const std::uint8_t *>(data);
        while (len > 0) {
            if (buffer_len_ == block_size) {
                count_add(block_size);
                compress(count_low_, count_high_, false);
                buffer_len_ = 0;
            }
            std::size_t take = block_size - buffer_len_;
            if (take > len) {
                take = len;
            }
            for (std::size_t i = 0; i < take; i++) {
                buffer_[buffer_len_++] = bytes[i];
            }
            bytes += take;
            len -= take;
        }
    }
    void update(const std::string &s) { update(s.data(), s.size()); }

    std::vector<std::uint8_t> digest() const {
        blake2b_hasher copy(*this);
        return copy.finalise();
    }

    std::string hex_digest() const {
        static const char hex[] = "0123456789abcdef";
        std::vector<std::uint8_t> d = digest();
        std::string out;
        out.reserve(d.size() * 2);
        for (std::uint8_t byte : d) {
            out.push_back(hex[byte >> 4]);
            out.push_back(hex[byte & 0x0f]);
        }
        return out;
    }

private:
    std::array<std::uint64_t, 8> state_;
    std::array<std::uint8_t, 128> buffer_;
    std::size_t buffer_len_;
    std::uint64_t count_low_;
    std::uint64_t count_high_;
    std::size_t digest_size_;

    static const std::array<std::uint64_t, 8> &iv() {
        static const std::array<std::uint64_t, 8> v = {
            0x6a09e667f3bcc908u, 0xbb67ae8584caa73bu, 0x3c6ef372fe94f82bu,
            0xa54ff53a5f1d36f1u, 0x510e527fade682d1u, 0x9b05688c2b3e6c1fu,
            0x1f83d9abfb41bd6bu, 0x5be0cd19137e2179u};
        return v;
    }

    static std::uint64_t load64(const std::uint8_t *p) {
        return static_cast<std::uint64_t>(p[0]) |
               (static_cast<std::uint64_t>(p[1]) << 8) |
               (static_cast<std::uint64_t>(p[2]) << 16) |
               (static_cast<std::uint64_t>(p[3]) << 24) |
               (static_cast<std::uint64_t>(p[4]) << 32) |
               (static_cast<std::uint64_t>(p[5]) << 40) |
               (static_cast<std::uint64_t>(p[6]) << 48) |
               (static_cast<std::uint64_t>(p[7]) << 56);
    }
    static std::uint64_t rotr(std::uint64_t x, unsigned n) {
        return (x >> n) | (x << (64 - n));
    }

    void count_add(std::uint64_t n) {
        std::uint64_t prev = count_low_;
        count_low_ += n;
        if (count_low_ < prev) {
            count_high_++;
        }
    }

    void compress(std::uint64_t t_low, std::uint64_t t_high, bool is_final) {
        static const std::uint8_t SIGMA[12][16] = {
            {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15},
            {14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3},
            {11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4},
            {7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8},
            {9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13},
            {2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9},
            {12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11},
            {13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10},
            {6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5},
            {10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0},
            {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15},
            {14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3}};
        std::uint64_t m[16];
        for (unsigned i = 0; i < 16; i++) {
            m[i] = load64(&buffer_[i * 8]);
        }
        std::uint64_t v[16];
        for (unsigned i = 0; i < 8; i++) {
            v[i] = state_[i];
            v[i + 8] = iv()[i];
        }
        v[12] ^= t_low;
        v[13] ^= t_high;
        if (is_final) {
            v[14] ^= 0xffffffffffffffffu;
        }
        auto mix = [&v](unsigned a, unsigned b, unsigned c, unsigned d,
                        std::uint64_t x, std::uint64_t y) {
            v[a] = v[a] + v[b] + x;
            v[d] = rotr(v[d] ^ v[a], 32);
            v[c] = v[c] + v[d];
            v[b] = rotr(v[b] ^ v[c], 24);
            v[a] = v[a] + v[b] + y;
            v[d] = rotr(v[d] ^ v[a], 16);
            v[c] = v[c] + v[d];
            v[b] = rotr(v[b] ^ v[c], 63);
        };
        for (unsigned i = 0; i < 12; i++) {
            const std::uint8_t *s = SIGMA[i];
            mix(0, 4, 8, 12, m[s[0]], m[s[1]]);
            mix(1, 5, 9, 13, m[s[2]], m[s[3]]);
            mix(2, 6, 10, 14, m[s[4]], m[s[5]]);
            mix(3, 7, 11, 15, m[s[6]], m[s[7]]);
            mix(0, 5, 10, 15, m[s[8]], m[s[9]]);
            mix(1, 6, 11, 12, m[s[10]], m[s[11]]);
            mix(2, 7, 8, 13, m[s[12]], m[s[13]]);
            mix(3, 4, 9, 14, m[s[14]], m[s[15]]);
        }
        for (unsigned i = 0; i < 8; i++) {
            state_[i] ^= v[i] ^ v[i + 8];
        }
    }

    std::vector<std::uint8_t> finalise() {
        std::uint64_t t_low = count_low_, t_high = count_high_;
        std::uint64_t prev = t_low;
        t_low += buffer_len_;
        if (t_low < prev) {
            t_high++;
        }
        for (std::size_t i = buffer_len_; i < block_size; i++) {
            buffer_[i] = 0;
        }
        compress(t_low, t_high, true);
        std::vector<std::uint8_t> out(digest_size_);
        std::array<std::uint8_t, 64> full{};
        for (unsigned i = 0; i < 8; i++) {
            for (unsigned s = 0; s < 8; s++) {
                full[i * 8 + s] = static_cast<std::uint8_t>(state_[i] >> (s * 8));
            }
        }
        for (std::size_t i = 0; i < digest_size_; i++) {
            out[i] = full[i];
        }
        return out;
    }
};

inline std::vector<std::uint8_t> blake2b(const void *data, std::size_t len,
                                         std::size_t digest_size = 64) {
    blake2b_hasher h(digest_size);
    h.update(data, len);
    return h.digest();
}
inline std::vector<std::uint8_t> blake2b(const std::string &s,
                                         std::size_t digest_size = 64) {
    return blake2b(s.data(), s.size(), digest_size);
}
inline std::string blake2b_hex(const std::string &s,
                               std::size_t digest_size = 64) {
    blake2b_hasher h(digest_size);
    h.update(s);
    return h.hex_digest();
}

} // namespace ca

#endif // BLAKE2B_HPP
