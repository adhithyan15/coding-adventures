// sha1.hpp — the SHA-1 hash (FIPS 180-4), in pure ISO C++17 (header-only). A
// faithful port of the Rust `sha1` crate.
// ===========================================================================
//
// Maps any byte sequence to a fixed 20-byte digest via 512-bit blocks and 80
// rounds. (SHA-1 is broken for collision resistance — do not use for security;
// still useful for checksums / Git object IDs / interop.) Output matches the
// published FIPS test vectors. Streaming `hasher` plus one-shot helpers.
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef SHA1_HPP
#define SHA1_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <string>

namespace ca {

using sha1_digest = std::array<std::uint8_t, 20>;

class sha1_hasher {
public:
    sha1_hasher() { reset(); }

    void reset() {
        state_ = {0x67452301u, 0xefcdab89u, 0x98badcfeu, 0x10325476u,
                  0xc3d2e1f0u};
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

    sha1_digest digest() const {
        sha1_hasher copy(*this);
        return copy.finalise();
    }

    std::string hex_digest() const {
        static const char hex[] = "0123456789abcdef";
        sha1_digest d = digest();
        std::string out;
        out.reserve(40);
        for (std::uint8_t byte : d) {
            out.push_back(hex[byte >> 4]);
            out.push_back(hex[byte & 0x0f]);
        }
        return out;
    }

private:
    std::array<std::uint32_t, 5> state_;
    std::uint64_t bit_length_;
    std::array<std::uint8_t, 64> buffer_;
    std::size_t buffer_len_;

    static std::uint32_t rotl(std::uint32_t x, unsigned n) {
        return ((x << n) | (x >> (32 - n))) & 0xffffffffu;
    }

    void transform(const std::uint8_t *block) {
        std::uint32_t w[80];
        for (unsigned i = 0; i < 16; i++) {
            w[i] = (static_cast<std::uint32_t>(block[i * 4]) << 24) |
                   (static_cast<std::uint32_t>(block[i * 4 + 1]) << 16) |
                   (static_cast<std::uint32_t>(block[i * 4 + 2]) << 8) |
                   (static_cast<std::uint32_t>(block[i * 4 + 3]));
        }
        for (unsigned i = 16; i < 80; i++) {
            w[i] = rotl(w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16], 1);
        }
        std::uint32_t a = state_[0], b = state_[1], c = state_[2], d = state_[3],
                      e = state_[4];
        for (unsigned i = 0; i < 80; i++) {
            std::uint32_t f, k;
            if (i < 20) {
                f = (b & c) | (~b & d);
                k = 0x5a827999u;
            } else if (i < 40) {
                f = b ^ c ^ d;
                k = 0x6ed9eba1u;
            } else if (i < 60) {
                f = (b & c) | (b & d) | (c & d);
                k = 0x8f1bbcdcu;
            } else {
                f = b ^ c ^ d;
                k = 0xca62c1d6u;
            }
            std::uint32_t tmp = (rotl(a, 5) + f + e + k + w[i]) & 0xffffffffu;
            e = d;
            d = c;
            c = rotl(b, 30);
            b = a;
            a = tmp;
        }
        state_[0] = (state_[0] + a) & 0xffffffffu;
        state_[1] = (state_[1] + b) & 0xffffffffu;
        state_[2] = (state_[2] + c) & 0xffffffffu;
        state_[3] = (state_[3] + d) & 0xffffffffu;
        state_[4] = (state_[4] + e) & 0xffffffffu;
    }

    sha1_digest finalise() {
        std::uint64_t total_bits =
            bit_length_ + static_cast<std::uint64_t>(buffer_len_) * 8;
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
            buffer_[56 + i] =
                static_cast<std::uint8_t>(total_bits >> (56 - i * 8));
        }
        transform(buffer_.data());
        sha1_digest out{};
        for (unsigned i = 0; i < 5; i++) {
            out[i * 4] = static_cast<std::uint8_t>(state_[i] >> 24);
            out[i * 4 + 1] = static_cast<std::uint8_t>(state_[i] >> 16);
            out[i * 4 + 2] = static_cast<std::uint8_t>(state_[i] >> 8);
            out[i * 4 + 3] = static_cast<std::uint8_t>(state_[i]);
        }
        return out;
    }
};

inline sha1_digest sha1(const void *data, std::size_t len) {
    sha1_hasher h;
    h.update(data, len);
    return h.digest();
}
inline sha1_digest sha1(const std::string &s) {
    return sha1(s.data(), s.size());
}
inline std::string sha1_hex(const void *data, std::size_t len) {
    sha1_hasher h;
    h.update(data, len);
    return h.hex_digest();
}
inline std::string sha1_hex(const std::string &s) {
    return sha1_hex(s.data(), s.size());
}

} // namespace ca

#endif // SHA1_HPP
