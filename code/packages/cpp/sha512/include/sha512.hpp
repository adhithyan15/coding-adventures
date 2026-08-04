// sha512.hpp — the SHA-512 hash (FIPS 180-4), in pure ISO C++17 (header-only). A
// faithful port of the Rust `sha512` crate.
// ===========================================================================
//
// Like SHA-256 but in 64-bit words: eight of them, 1024-bit blocks, 80 rounds,
// and a 128-bit length field. Output matches the published FIPS test vectors.
// Streaming `hasher` plus one-shot helpers.
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef SHA512_HPP
#define SHA512_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <string>

namespace ca {

using sha512_digest = std::array<std::uint8_t, 64>;

class sha512_hasher {
public:
    sha512_hasher() { reset(); }

    void reset() {
        state_ = {0x6a09e667f3bcc908u, 0xbb67ae8584caa73bu, 0x3c6ef372fe94f82bu,
                  0xa54ff53a5f1d36f1u, 0x510e527fade682d1u, 0x9b05688c2b3e6c1fu,
                  0x1f83d9abfb41bd6bu, 0x5be0cd19137e2179u};
        length_low_ = 0;
        length_high_ = 0;
        buffer_len_ = 0;
    }

    void update(const void *data, std::size_t len) {
        const std::uint8_t *bytes = static_cast<const std::uint8_t *>(data);
        for (std::size_t i = 0; i < len; i++) {
            buffer_[buffer_len_++] = bytes[i];
            if (buffer_len_ == 128) {
                transform(buffer_.data());
                if (length_low_ > 0xffffffffffffffffu - 1024u) {
                    length_high_++;
                }
                length_low_ += 1024u;
                buffer_len_ = 0;
            }
        }
    }
    void update(const std::string &s) { update(s.data(), s.size()); }

    sha512_digest digest() const {
        sha512_hasher copy(*this);
        return copy.finalise();
    }

    std::string hex_digest() const {
        static const char hex[] = "0123456789abcdef";
        sha512_digest d = digest();
        std::string out;
        out.reserve(128);
        for (std::uint8_t byte : d) {
            out.push_back(hex[byte >> 4]);
            out.push_back(hex[byte & 0x0f]);
        }
        return out;
    }

private:
    std::array<std::uint64_t, 8> state_;
    std::uint64_t length_low_;
    std::uint64_t length_high_;
    std::array<std::uint8_t, 128> buffer_;
    std::size_t buffer_len_;

    static std::uint64_t rotr(std::uint64_t x, unsigned n) {
        return (x >> n) | (x << (64 - n));
    }

    void transform(const std::uint8_t *block) {
        static const std::uint64_t K[80] = {
            0x428a2f98d728ae22u, 0x7137449123ef65cdu, 0xb5c0fbcfec4d3b2fu,
            0xe9b5dba58189dbbcu, 0x3956c25bf348b538u, 0x59f111f1b605d019u,
            0x923f82a4af194f9bu, 0xab1c5ed5da6d8118u, 0xd807aa98a3030242u,
            0x12835b0145706fbeu, 0x243185be4ee4b28cu, 0x550c7dc3d5ffb4e2u,
            0x72be5d74f27b896fu, 0x80deb1fe3b1696b1u, 0x9bdc06a725c71235u,
            0xc19bf174cf692694u, 0xe49b69c19ef14ad2u, 0xefbe4786384f25e3u,
            0x0fc19dc68b8cd5b5u, 0x240ca1cc77ac9c65u, 0x2de92c6f592b0275u,
            0x4a7484aa6ea6e483u, 0x5cb0a9dcbd41fbd4u, 0x76f988da831153b5u,
            0x983e5152ee66dfabu, 0xa831c66d2db43210u, 0xb00327c898fb213fu,
            0xbf597fc7beef0ee4u, 0xc6e00bf33da88fc2u, 0xd5a79147930aa725u,
            0x06ca6351e003826fu, 0x142929670a0e6e70u, 0x27b70a8546d22ffcu,
            0x2e1b21385c26c926u, 0x4d2c6dfc5ac42aedu, 0x53380d139d95b3dfu,
            0x650a73548baf63deu, 0x766a0abb3c77b2a8u, 0x81c2c92e47edaee6u,
            0x92722c851482353bu, 0xa2bfe8a14cf10364u, 0xa81a664bbc423001u,
            0xc24b8b70d0f89791u, 0xc76c51a30654be30u, 0xd192e819d6ef5218u,
            0xd69906245565a910u, 0xf40e35855771202au, 0x106aa07032bbd1b8u,
            0x19a4c116b8d2d0c8u, 0x1e376c085141ab53u, 0x2748774cdf8eeb99u,
            0x34b0bcb5e19b48a8u, 0x391c0cb3c5c95a63u, 0x4ed8aa4ae3418acbu,
            0x5b9cca4f7763e373u, 0x682e6ff3d6b2b8a3u, 0x748f82ee5defb2fcu,
            0x78a5636f43172f60u, 0x84c87814a1f0ab72u, 0x8cc702081a6439ecu,
            0x90befffa23631e28u, 0xa4506cebde82bde9u, 0xbef9a3f7b2c67915u,
            0xc67178f2e372532bu, 0xca273eceea26619cu, 0xd186b8c721c0c207u,
            0xeada7dd6cde0eb1eu, 0xf57d4f7fee6ed178u, 0x06f067aa72176fbau,
            0x0a637dc5a2c898a6u, 0x113f9804bef90daeu, 0x1b710b35131c471bu,
            0x28db77f523047d84u, 0x32caab7b40c72493u, 0x3c9ebe0a15c9bebcu,
            0x431d67c49c100d4cu, 0x4cc5d4becb3e42b6u, 0x597f299cfc657e2au,
            0x5fcb6fab3ad6faecu, 0x6c44198c4a475817u};
        std::uint64_t w[80];
        for (unsigned i = 0; i < 16; i++) {
            w[i] = (static_cast<std::uint64_t>(block[i * 8]) << 56) |
                   (static_cast<std::uint64_t>(block[i * 8 + 1]) << 48) |
                   (static_cast<std::uint64_t>(block[i * 8 + 2]) << 40) |
                   (static_cast<std::uint64_t>(block[i * 8 + 3]) << 32) |
                   (static_cast<std::uint64_t>(block[i * 8 + 4]) << 24) |
                   (static_cast<std::uint64_t>(block[i * 8 + 5]) << 16) |
                   (static_cast<std::uint64_t>(block[i * 8 + 6]) << 8) |
                   (static_cast<std::uint64_t>(block[i * 8 + 7]));
        }
        for (unsigned i = 16; i < 80; i++) {
            std::uint64_t s0 =
                rotr(w[i - 15], 1) ^ rotr(w[i - 15], 8) ^ (w[i - 15] >> 7);
            std::uint64_t s1 =
                rotr(w[i - 2], 19) ^ rotr(w[i - 2], 61) ^ (w[i - 2] >> 6);
            w[i] = w[i - 16] + s0 + w[i - 7] + s1;
        }
        std::uint64_t a = state_[0], b = state_[1], c = state_[2], d = state_[3];
        std::uint64_t e = state_[4], f = state_[5], g = state_[6], h = state_[7];
        for (unsigned i = 0; i < 80; i++) {
            std::uint64_t s1 = rotr(e, 14) ^ rotr(e, 18) ^ rotr(e, 41);
            std::uint64_t ch = (e & f) ^ (~e & g);
            std::uint64_t t1 = h + s1 + ch + K[i] + w[i];
            std::uint64_t s0 = rotr(a, 28) ^ rotr(a, 34) ^ rotr(a, 39);
            std::uint64_t maj = (a & b) ^ (a & c) ^ (b & c);
            std::uint64_t t2 = s0 + maj;
            h = g; g = f; f = e;
            e = d + t1;
            d = c; c = b; b = a;
            a = t1 + t2;
        }
        state_[0] += a; state_[1] += b; state_[2] += c; state_[3] += d;
        state_[4] += e; state_[5] += f; state_[6] += g; state_[7] += h;
    }

    sha512_digest finalise() {
        std::uint64_t bits_low = length_low_ + static_cast<std::uint64_t>(buffer_len_) * 8u;
        std::uint64_t bits_high = length_high_;
        if (bits_low < length_low_) {
            bits_high++;
        }
        buffer_[buffer_len_++] = 0x80;
        if (buffer_len_ > 112) {
            while (buffer_len_ < 128) {
                buffer_[buffer_len_++] = 0;
            }
            transform(buffer_.data());
            buffer_len_ = 0;
        }
        while (buffer_len_ < 112) {
            buffer_[buffer_len_++] = 0;
        }
        for (unsigned i = 0; i < 8; i++) {
            buffer_[112 + i] = static_cast<std::uint8_t>(bits_high >> (56 - i * 8));
        }
        for (unsigned i = 0; i < 8; i++) {
            buffer_[120 + i] = static_cast<std::uint8_t>(bits_low >> (56 - i * 8));
        }
        transform(buffer_.data());
        sha512_digest out{};
        for (unsigned i = 0; i < 8; i++) {
            for (unsigned s = 0; s < 8; s++) {
                out[i * 8 + s] = static_cast<std::uint8_t>(state_[i] >> (56 - s * 8));
            }
        }
        return out;
    }
};

inline sha512_digest sha512(const void *data, std::size_t len) {
    sha512_hasher h;
    h.update(data, len);
    return h.digest();
}
inline sha512_digest sha512(const std::string &s) {
    return sha512(s.data(), s.size());
}
inline std::string sha512_hex(const void *data, std::size_t len) {
    sha512_hasher h;
    h.update(data, len);
    return h.hex_digest();
}
inline std::string sha512_hex(const std::string &s) {
    return sha512_hex(s.data(), s.size());
}

} // namespace ca

#endif // SHA512_HPP
