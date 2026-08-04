// md5.hpp — the MD5 hash (RFC 1321), in pure ISO C++17 (header-only). A faithful
// port of the Rust `md5` crate.
// ===========================================================================
//
// Maps any byte sequence to a fixed 16-byte digest via 512-bit blocks and 64
// rounds; little-endian message words and output. (MD5 is broken for collision
// resistance — do not use for security; still useful for checksums / interop.)
// Output matches the RFC 1321 test suite. Streaming `hasher` plus one-shot
// helpers.
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef MD5_HPP
#define MD5_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <string>

namespace ca {

using md5_digest = std::array<std::uint8_t, 16>;

class md5_hasher {
public:
    md5_hasher() { reset(); }

    void reset() {
        state_ = {0x67452301u, 0xefcdab89u, 0x98badcfeu, 0x10325476u};
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

    md5_digest digest() const {
        md5_hasher copy(*this);
        return copy.finalise();
    }

    std::string hex_digest() const {
        static const char hex[] = "0123456789abcdef";
        md5_digest d = digest();
        std::string out;
        out.reserve(32);
        for (std::uint8_t byte : d) {
            out.push_back(hex[byte >> 4]);
            out.push_back(hex[byte & 0x0f]);
        }
        return out;
    }

private:
    std::array<std::uint32_t, 4> state_;
    std::uint64_t bit_length_;
    std::array<std::uint8_t, 64> buffer_;
    std::size_t buffer_len_;

    static std::uint32_t rotl(std::uint32_t x, unsigned n) {
        return ((x << n) | (x >> (32 - n))) & 0xffffffffu;
    }

    void transform(const std::uint8_t *block) {
        static const unsigned S[64] = {
            7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
            5, 9,  14, 20, 5, 9,  14, 20, 5, 9,  14, 20, 5, 9,  14, 20,
            4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
            6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21};
        static const std::uint32_t K[64] = {
            0xd76aa478u, 0xe8c7b756u, 0x242070dbu, 0xc1bdceeeu, 0xf57c0fafu,
            0x4787c62au, 0xa8304613u, 0xfd469501u, 0x698098d8u, 0x8b44f7afu,
            0xffff5bb1u, 0x895cd7beu, 0x6b901122u, 0xfd987193u, 0xa679438eu,
            0x49b40821u, 0xf61e2562u, 0xc040b340u, 0x265e5a51u, 0xe9b6c7aau,
            0xd62f105du, 0x02441453u, 0xd8a1e681u, 0xe7d3fbc8u, 0x21e1cde6u,
            0xc33707d6u, 0xf4d50d87u, 0x455a14edu, 0xa9e3e905u, 0xfcefa3f8u,
            0x676f02d9u, 0x8d2a4c8au, 0xfffa3942u, 0x8771f681u, 0x6d9d6122u,
            0xfde5380cu, 0xa4beea44u, 0x4bdecfa9u, 0xf6bb4b60u, 0xbebfbc70u,
            0x289b7ec6u, 0xeaa127fau, 0xd4ef3085u, 0x04881d05u, 0xd9d4d039u,
            0xe6db99e5u, 0x1fa27cf8u, 0xc4ac5665u, 0xf4292244u, 0x432aff97u,
            0xab9423a7u, 0xfc93a039u, 0x655b59c3u, 0x8f0ccc92u, 0xffeff47du,
            0x85845dd1u, 0x6fa87e4fu, 0xfe2ce6e0u, 0xa3014314u, 0x4e0811a1u,
            0xf7537e82u, 0xbd3af235u, 0x2ad7d2bbu, 0xeb86d391u};
        std::uint32_t m[16];
        for (unsigned i = 0; i < 16; i++) {
            m[i] = (static_cast<std::uint32_t>(block[i * 4])) |
                   (static_cast<std::uint32_t>(block[i * 4 + 1]) << 8) |
                   (static_cast<std::uint32_t>(block[i * 4 + 2]) << 16) |
                   (static_cast<std::uint32_t>(block[i * 4 + 3]) << 24);
        }
        std::uint32_t a = state_[0], b = state_[1], c = state_[2], d = state_[3];
        for (unsigned i = 0; i < 64; i++) {
            std::uint32_t f;
            unsigned g;
            if (i < 16) {
                f = (b & c) | (~b & d);
                g = i;
            } else if (i < 32) {
                f = (d & b) | (~d & c);
                g = (5 * i + 1) % 16;
            } else if (i < 48) {
                f = b ^ c ^ d;
                g = (3 * i + 5) % 16;
            } else {
                f = c ^ (b | ~d);
                g = (7 * i) % 16;
            }
            f = (f + a + K[i] + m[g]) & 0xffffffffu;
            a = d;
            d = c;
            c = b;
            b = (b + rotl(f, S[i])) & 0xffffffffu;
        }
        state_[0] = (state_[0] + a) & 0xffffffffu;
        state_[1] = (state_[1] + b) & 0xffffffffu;
        state_[2] = (state_[2] + c) & 0xffffffffu;
        state_[3] = (state_[3] + d) & 0xffffffffu;
    }

    md5_digest finalise() {
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
            buffer_[56 + i] = static_cast<std::uint8_t>(total_bits >> (i * 8));
        }
        transform(buffer_.data());
        md5_digest out{};
        for (unsigned i = 0; i < 4; i++) {
            out[i * 4] = static_cast<std::uint8_t>(state_[i]);
            out[i * 4 + 1] = static_cast<std::uint8_t>(state_[i] >> 8);
            out[i * 4 + 2] = static_cast<std::uint8_t>(state_[i] >> 16);
            out[i * 4 + 3] = static_cast<std::uint8_t>(state_[i] >> 24);
        }
        return out;
    }
};

inline md5_digest md5(const void *data, std::size_t len) {
    md5_hasher h;
    h.update(data, len);
    return h.digest();
}
inline md5_digest md5(const std::string &s) { return md5(s.data(), s.size()); }
inline std::string md5_hex(const void *data, std::size_t len) {
    md5_hasher h;
    h.update(data, len);
    return h.hex_digest();
}
inline std::string md5_hex(const std::string &s) {
    return md5_hex(s.data(), s.size());
}

} // namespace ca

#endif // MD5_HPP
