// sha256.hpp — the SHA-256 cryptographic hash (FIPS 180-4), in pure ISO C++17
// (header-only). A faithful port of the Rust `sha256` crate.
// ===========================================================================
//
// Maps any byte sequence to a fixed 32-byte digest, processing 512-bit blocks
// through 64 rounds. Standard algorithm — output matches the published FIPS test
// vectors. Offers a streaming `hasher` plus one-shot `sha256` / `sha256_hex`.
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef SHA256_HPP
#define SHA256_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <string>

namespace ca {

using sha256_digest = std::array<std::uint8_t, 32>;

class sha256_hasher {
public:
    sha256_hasher() { reset(); }

    void reset() {
        state_ = {0x6a09e667u, 0xbb67ae85u, 0x3c6ef372u, 0xa54ff53au,
                  0x510e527fu, 0x9b05688cu, 0x1f83d9abu, 0x5be0cd19u};
        bit_length_ = 0;
        buffer_len_ = 0;
    }

    void update(const void *data, std::size_t len) {
        const std::uint8_t *bytes = static_cast<const std::uint8_t *>(data);
        for (std::size_t i = 0; i < len; i++) {
            buffer_[buffer_len_++] = bytes[i];
            if (buffer_len_ == 64) {
                transform(buffer_.data());
                bit_length_ += 512;
                buffer_len_ = 0;
            }
        }
    }
    void update(const std::string &s) { update(s.data(), s.size()); }

    // Finalise (on a copy, so the hasher stays usable) and return the digest.
    sha256_digest digest() const {
        sha256_hasher copy(*this);
        return copy.finalise();
    }

    std::string hex_digest() const {
        static const char hex[] = "0123456789abcdef";
        sha256_digest d = digest();
        std::string out;
        out.reserve(64);
        for (std::uint8_t byte : d) {
            out.push_back(hex[byte >> 4]);
            out.push_back(hex[byte & 0x0f]);
        }
        return out;
    }

private:
    std::array<std::uint32_t, 8> state_;
    std::uint64_t bit_length_;
    std::array<std::uint8_t, 64> buffer_;
    std::size_t buffer_len_;

    static std::uint32_t rotr(std::uint32_t x, unsigned n) {
        return ((x >> n) | (x << (32 - n))) & 0xffffffffu;
    }

    void transform(const std::uint8_t *block) {
        static const std::uint32_t K[64] = {
            0x428a2f98u, 0x71374491u, 0xb5c0fbcfu, 0xe9b5dba5u, 0x3956c25bu,
            0x59f111f1u, 0x923f82a4u, 0xab1c5ed5u, 0xd807aa98u, 0x12835b01u,
            0x243185beu, 0x550c7dc3u, 0x72be5d74u, 0x80deb1feu, 0x9bdc06a7u,
            0xc19bf174u, 0xe49b69c1u, 0xefbe4786u, 0x0fc19dc6u, 0x240ca1ccu,
            0x2de92c6fu, 0x4a7484aau, 0x5cb0a9dcu, 0x76f988dau, 0x983e5152u,
            0xa831c66du, 0xb00327c8u, 0xbf597fc7u, 0xc6e00bf3u, 0xd5a79147u,
            0x06ca6351u, 0x14292967u, 0x27b70a85u, 0x2e1b2138u, 0x4d2c6dfcu,
            0x53380d13u, 0x650a7354u, 0x766a0abbu, 0x81c2c92eu, 0x92722c85u,
            0xa2bfe8a1u, 0xa81a664bu, 0xc24b8b70u, 0xc76c51a3u, 0xd192e819u,
            0xd6990624u, 0xf40e3585u, 0x106aa070u, 0x19a4c116u, 0x1e376c08u,
            0x2748774cu, 0x34b0bcb5u, 0x391c0cb3u, 0x4ed8aa4au, 0x5b9cca4fu,
            0x682e6ff3u, 0x748f82eeu, 0x78a5636fu, 0x84c87814u, 0x8cc70208u,
            0x90befffau, 0xa4506cebu, 0xbef9a3f7u, 0xc67178f2u};
        std::uint32_t w[64];
        for (unsigned i = 0; i < 16; i++) {
            w[i] = (static_cast<std::uint32_t>(block[i * 4]) << 24) |
                   (static_cast<std::uint32_t>(block[i * 4 + 1]) << 16) |
                   (static_cast<std::uint32_t>(block[i * 4 + 2]) << 8) |
                   (static_cast<std::uint32_t>(block[i * 4 + 3]));
        }
        for (unsigned i = 16; i < 64; i++) {
            std::uint32_t s0 =
                rotr(w[i - 15], 7) ^ rotr(w[i - 15], 18) ^ (w[i - 15] >> 3);
            std::uint32_t s1 =
                rotr(w[i - 2], 17) ^ rotr(w[i - 2], 19) ^ (w[i - 2] >> 10);
            w[i] = (w[i - 16] + s0 + w[i - 7] + s1) & 0xffffffffu;
        }
        std::uint32_t a = state_[0], b = state_[1], c = state_[2], d = state_[3];
        std::uint32_t e = state_[4], f = state_[5], g = state_[6], h = state_[7];
        for (unsigned i = 0; i < 64; i++) {
            std::uint32_t s1 = rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25);
            std::uint32_t ch = (e & f) ^ (~e & g);
            std::uint32_t t1 = (h + s1 + ch + K[i] + w[i]) & 0xffffffffu;
            std::uint32_t s0 = rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22);
            std::uint32_t maj = (a & b) ^ (a & c) ^ (b & c);
            std::uint32_t t2 = (s0 + maj) & 0xffffffffu;
            h = g; g = f; f = e;
            e = (d + t1) & 0xffffffffu;
            d = c; c = b; b = a;
            a = (t1 + t2) & 0xffffffffu;
        }
        state_[0] = (state_[0] + a) & 0xffffffffu;
        state_[1] = (state_[1] + b) & 0xffffffffu;
        state_[2] = (state_[2] + c) & 0xffffffffu;
        state_[3] = (state_[3] + d) & 0xffffffffu;
        state_[4] = (state_[4] + e) & 0xffffffffu;
        state_[5] = (state_[5] + f) & 0xffffffffu;
        state_[6] = (state_[6] + g) & 0xffffffffu;
        state_[7] = (state_[7] + h) & 0xffffffffu;
    }

    sha256_digest finalise() {
        std::uint64_t total_bits = bit_length_ + static_cast<std::uint64_t>(buffer_len_) * 8;
        buffer_[buffer_len_++] = 0x80;
        if (buffer_len_ > 56) {
            while (buffer_len_ < 64) {
                buffer_[buffer_len_++] = 0;
            }
            transform(buffer_.data());
            buffer_len_ = 0;
        }
        while (buffer_len_ < 56) {
            buffer_[buffer_len_++] = 0;
        }
        for (unsigned i = 0; i < 8; i++) {
            buffer_[56 + i] = static_cast<std::uint8_t>(total_bits >> (56 - i * 8));
        }
        transform(buffer_.data());
        sha256_digest out{};
        for (unsigned i = 0; i < 8; i++) {
            out[i * 4] = static_cast<std::uint8_t>(state_[i] >> 24);
            out[i * 4 + 1] = static_cast<std::uint8_t>(state_[i] >> 16);
            out[i * 4 + 2] = static_cast<std::uint8_t>(state_[i] >> 8);
            out[i * 4 + 3] = static_cast<std::uint8_t>(state_[i]);
        }
        return out;
    }
};

// One-shot helpers.
inline sha256_digest sha256(const void *data, std::size_t len) {
    sha256_hasher h;
    h.update(data, len);
    return h.digest();
}
inline sha256_digest sha256(const std::string &s) {
    return sha256(s.data(), s.size());
}
inline std::string sha256_hex(const void *data, std::size_t len) {
    sha256_hasher h;
    h.update(data, len);
    return h.hex_digest();
}
inline std::string sha256_hex(const std::string &s) {
    return sha256_hex(s.data(), s.size());
}

} // namespace ca

#endif // SHA256_HPP
