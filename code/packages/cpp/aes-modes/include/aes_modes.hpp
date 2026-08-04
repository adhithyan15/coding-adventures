// aes_modes.hpp — AES modes of operation (ECB, CBC, CTR, GCM) with PKCS#7
// padding, in pure ISO C++17 (header-only), in namespace ca::aes_modes. A
// faithful port of the Rust `aes-modes` crate.
// ===========================================================================
//
// AES is a 128-bit (16-byte) block cipher; a *mode of operation* chains block
// calls to encrypt arbitrary-length messages. Built on the raw block cipher of
// the sibling header-only `aes` package.
//
//   ECB — each block independently. INSECURE (identical blocks leak); teaching
//         only. PKCS#7 padded.
//   CBC — C[i] = E(P[i] XOR C[i-1]); 16-byte IV. PKCS#7 padded.
//   CTR — stream cipher: keystream = E(nonce||counter); 12-byte nonce, 32-bit
//         big-endian counter from 1. No padding; enc == dec.
//   GCM — CTR encryption + a GHASH authentication tag (AEAD). 12-byte IV.
//         Decryption verifies the tag before returning.
//
// GHASH multiplies in GF(2^128) with reducing polynomial x^128+x^7+x^2+x+1
// (byte-wise — no 128-bit integers).
//
// Validation errors throw std::invalid_argument; a GCM tag mismatch throws
// ca::aes_modes::AuthenticationError.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_AES_MODES_HPP
#define CA_AES_MODES_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <utility>
#include <vector>

#include "aes.hpp"  // sibling header-only package (include path via run.sh)

namespace ca {
namespace aes_modes {

using Bytes = std::vector<std::uint8_t>;
using Block = std::array<std::uint8_t, 16>;

constexpr std::size_t block_size = 16;

// Thrown when a GCM tag fails to verify. Returning unauthenticated plaintext
// enables attacks, so decryption reports this rather than a value.
class AuthenticationError : public std::runtime_error {
public:
    AuthenticationError()
        : std::runtime_error("aes-modes: GCM authentication failed") {}
};

namespace detail {

inline Block enc_block(const Block& in, const Bytes& key) {
    auto r = ca::aes::encrypt_block(in, key);
    if (!r) {
        throw std::invalid_argument("aes-modes: invalid AES key length");
    }
    return *r;
}

inline Block dec_block(const Block& in, const Bytes& key) {
    auto r = ca::aes::decrypt_block(in, key);
    if (!r) {
        throw std::invalid_argument("aes-modes: invalid AES key length");
    }
    return *r;
}

// GF(2^128) multiply with the GCM reducing polynomial (high byte 0xE1).
inline Block gf128_mul(const Block& x, const Block& y) {
    Block z{};
    Block v = y;
    for (int i = 0; i < 128; ++i) {
        int byte_idx = i / 8;
        int bit_idx = 7 - (i % 8);
        if ((x[static_cast<std::size_t>(byte_idx)] >> bit_idx) & 1) {
            for (int j = 0; j < 16; ++j) {
                z[static_cast<std::size_t>(j)] ^= v[static_cast<std::size_t>(j)];
            }
        }
        std::uint8_t carry = static_cast<std::uint8_t>(v[15] & 1);
        for (int j = 15; j >= 1; --j) {
            v[static_cast<std::size_t>(j)] = static_cast<std::uint8_t>(
                (v[static_cast<std::size_t>(j)] >> 1) |
                ((v[static_cast<std::size_t>(j - 1)] & 1) << 7));
        }
        v[0] >>= 1;
        if (carry) {
            v[0] ^= 0xe1;
        }
    }
    return z;
}

inline Block ghash(const Block& h, const Bytes& aad, const Bytes& ct) {
    Block y{};
    auto process = [&](const Bytes& data) {
        for (std::size_t off = 0; off < data.size(); off += block_size) {
            Block block{};
            std::size_t n = data.size() - off;
            if (n > block_size) {
                n = block_size;
            }
            for (std::size_t k = 0; k < n; ++k) {
                block[k] = data[off + k];
            }
            for (std::size_t j = 0; j < 16; ++j) {
                block[j] ^= y[j];
            }
            y = gf128_mul(block, h);
        }
    };
    if (!aad.empty()) {
        process(aad);
    }
    if (!ct.empty()) {
        process(ct);
    }
    Block len_block{};
    std::uint64_t aad_bits = static_cast<std::uint64_t>(aad.size()) * 8u;
    std::uint64_t ct_bits = static_cast<std::uint64_t>(ct.size()) * 8u;
    for (int j = 0; j < 8; ++j) {
        len_block[static_cast<std::size_t>(j)] =
            static_cast<std::uint8_t>((aad_bits >> (56 - 8 * j)) & 0xFF);
        len_block[static_cast<std::size_t>(8 + j)] =
            static_cast<std::uint8_t>((ct_bits >> (56 - 8 * j)) & 0xFF);
    }
    for (std::size_t j = 0; j < 16; ++j) {
        len_block[j] ^= y[j];
    }
    return gf128_mul(len_block, h);
}

// Increment the 32-bit big-endian counter in the last 4 bytes.
inline void increment_counter(Block& block) {
    for (int i = 15; i >= 12; --i) {
        block[static_cast<std::size_t>(i)] =
            static_cast<std::uint8_t>(block[static_cast<std::size_t>(i)] + 1);
        if (block[static_cast<std::size_t>(i)] != 0) {
            break;
        }
    }
}

}  // namespace detail

// ---- PKCS#7 -----------------------------------------------------------------

inline Bytes pkcs7_pad(const Bytes& data) {
    std::size_t pad_len = block_size - (data.size() % block_size);
    Bytes out = data;
    out.insert(out.end(), pad_len, static_cast<std::uint8_t>(pad_len));
    return out;
}

inline Bytes pkcs7_unpad(const Bytes& data) {
    if (data.empty() || data.size() % block_size != 0) {
        throw std::invalid_argument(
            "aes-modes: padded data must be a positive multiple of 16");
    }
    std::size_t pad_len = data.back();
    if (pad_len < 1 || pad_len > block_size) {
        throw std::invalid_argument("aes-modes: invalid PKCS#7 padding");
    }
    std::uint8_t diff = 0;
    for (std::size_t i = data.size() - pad_len; i < data.size(); ++i) {
        diff |= static_cast<std::uint8_t>(data[i] ^ static_cast<std::uint8_t>(pad_len));
    }
    if (diff != 0) {
        throw std::invalid_argument("aes-modes: invalid PKCS#7 padding");
    }
    return Bytes(data.begin(), data.end() - static_cast<std::ptrdiff_t>(pad_len));
}

// ---- ECB (INSECURE — educational) -------------------------------------------

inline Bytes ecb_encrypt(const Bytes& plaintext, const Bytes& key) {
    Bytes padded = pkcs7_pad(plaintext);
    Bytes out;
    out.reserve(padded.size());
    for (std::size_t off = 0; off < padded.size(); off += block_size) {
        Block block;
        for (std::size_t k = 0; k < 16; ++k) {
            block[k] = padded[off + k];
        }
        Block enc = detail::enc_block(block, key);
        out.insert(out.end(), enc.begin(), enc.end());
    }
    return out;
}

inline Bytes ecb_decrypt(const Bytes& ciphertext, const Bytes& key) {
    if (ciphertext.empty() || ciphertext.size() % block_size != 0) {
        throw std::invalid_argument(
            "aes-modes: ECB ciphertext must be a non-empty multiple of 16");
    }
    Bytes out;
    out.reserve(ciphertext.size());
    for (std::size_t off = 0; off < ciphertext.size(); off += block_size) {
        Block block;
        for (std::size_t k = 0; k < 16; ++k) {
            block[k] = ciphertext[off + k];
        }
        Block dec = detail::dec_block(block, key);
        out.insert(out.end(), dec.begin(), dec.end());
    }
    return pkcs7_unpad(out);
}

// ---- CBC --------------------------------------------------------------------

inline Bytes cbc_encrypt(const Bytes& plaintext, const Bytes& key,
                         const Bytes& iv) {
    if (iv.size() != block_size) {
        throw std::invalid_argument("aes-modes: CBC IV must be 16 bytes");
    }
    Bytes padded = pkcs7_pad(plaintext);
    Bytes out;
    out.reserve(padded.size());
    Block prev;
    for (std::size_t k = 0; k < 16; ++k) {
        prev[k] = iv[k];
    }
    for (std::size_t off = 0; off < padded.size(); off += block_size) {
        Block block;
        for (std::size_t k = 0; k < 16; ++k) {
            block[k] = static_cast<std::uint8_t>(padded[off + k] ^ prev[k]);
        }
        Block enc = detail::enc_block(block, key);
        out.insert(out.end(), enc.begin(), enc.end());
        prev = enc;
    }
    return out;
}

inline Bytes cbc_decrypt(const Bytes& ciphertext, const Bytes& key,
                         const Bytes& iv) {
    if (iv.size() != block_size) {
        throw std::invalid_argument("aes-modes: CBC IV must be 16 bytes");
    }
    if (ciphertext.empty() || ciphertext.size() % block_size != 0) {
        throw std::invalid_argument(
            "aes-modes: CBC ciphertext must be a non-empty multiple of 16");
    }
    Bytes out;
    out.reserve(ciphertext.size());
    Block prev;
    for (std::size_t k = 0; k < 16; ++k) {
        prev[k] = iv[k];
    }
    for (std::size_t off = 0; off < ciphertext.size(); off += block_size) {
        Block block;
        for (std::size_t k = 0; k < 16; ++k) {
            block[k] = ciphertext[off + k];
        }
        Block dec = detail::dec_block(block, key);
        for (std::size_t k = 0; k < 16; ++k) {
            out.push_back(static_cast<std::uint8_t>(dec[k] ^ prev[k]));
        }
        prev = block;
    }
    return pkcs7_unpad(out);
}

// ---- CTR --------------------------------------------------------------------

inline Bytes ctr_encrypt(const Bytes& input, const Bytes& key,
                         const Bytes& nonce) {
    if (nonce.size() != 12) {
        throw std::invalid_argument("aes-modes: CTR nonce must be 12 bytes");
    }
    Bytes out;
    out.reserve(input.size());
    std::uint32_t counter = 1;
    for (std::size_t off = 0; off < input.size(); off += block_size) {
        Block cb{};
        for (std::size_t k = 0; k < 12; ++k) {
            cb[k] = nonce[k];
        }
        cb[12] = static_cast<std::uint8_t>((counter >> 24) & 0xFF);
        cb[13] = static_cast<std::uint8_t>((counter >> 16) & 0xFF);
        cb[14] = static_cast<std::uint8_t>((counter >> 8) & 0xFF);
        cb[15] = static_cast<std::uint8_t>(counter & 0xFF);
        Block keystream = detail::enc_block(cb, key);
        std::size_t n = input.size() - off;
        if (n > block_size) {
            n = block_size;
        }
        for (std::size_t k = 0; k < n; ++k) {
            out.push_back(static_cast<std::uint8_t>(input[off + k] ^ keystream[k]));
        }
        counter = counter + 1;  // uint32 wraps (well-defined)
    }
    return out;
}

inline Bytes ctr_decrypt(const Bytes& input, const Bytes& key,
                         const Bytes& nonce) {
    return ctr_encrypt(input, key, nonce);
}

// ---- GCM --------------------------------------------------------------------

namespace detail {

// CTR keystream over `in` starting at inc32(J0) (incremented before each block).
inline Bytes gcm_ctr(const Block& j0, const Bytes& in, const Bytes& key) {
    Bytes out;
    out.reserve(in.size());
    Block counter = j0;
    for (std::size_t off = 0; off < in.size(); off += block_size) {
        increment_counter(counter);
        Block keystream = enc_block(counter, key);
        std::size_t n = in.size() - off;
        if (n > block_size) {
            n = block_size;
        }
        for (std::size_t k = 0; k < n; ++k) {
            out.push_back(static_cast<std::uint8_t>(in[off + k] ^ keystream[k]));
        }
    }
    return out;
}

inline Block gcm_setup_j0(const Bytes& iv) {
    Block j0{};
    for (std::size_t k = 0; k < 12; ++k) {
        j0[k] = iv[k];
    }
    j0[15] = 1;
    return j0;
}

inline Block gcm_tag(const Block& h, const Block& j0, const Bytes& aad,
                     const Bytes& ct, const Bytes& key) {
    Block gh = ghash(h, aad, ct);
    Block enc_j0 = enc_block(j0, key);
    Block tag;
    for (std::size_t i = 0; i < 16; ++i) {
        tag[i] = static_cast<std::uint8_t>(gh[i] ^ enc_j0[i]);
    }
    return tag;
}

}  // namespace detail

// gcm_encrypt — returns (ciphertext, 16-byte tag).
inline std::pair<Bytes, Block> gcm_encrypt(const Bytes& plaintext,
                                           const Bytes& key, const Bytes& iv,
                                           const Bytes& aad) {
    if (iv.size() != 12) {
        throw std::invalid_argument("aes-modes: GCM IV must be 12 bytes");
    }
    Block zero{};
    Block h = detail::enc_block(zero, key);
    Block j0 = detail::gcm_setup_j0(iv);
    Bytes ct = detail::gcm_ctr(j0, plaintext, key);
    Block tag = detail::gcm_tag(h, j0, aad, ct, key);
    return {std::move(ct), tag};
}

// gcm_decrypt — verifies the tag (constant-time), then returns the plaintext.
// Throws AuthenticationError on a tag mismatch.
inline Bytes gcm_decrypt(const Bytes& ciphertext, const Bytes& key,
                         const Bytes& iv, const Bytes& aad, const Block& tag) {
    if (iv.size() != 12) {
        throw std::invalid_argument("aes-modes: GCM IV must be 12 bytes");
    }
    Block zero{};
    Block h = detail::enc_block(zero, key);
    Block j0 = detail::gcm_setup_j0(iv);
    Block expected = detail::gcm_tag(h, j0, aad, ciphertext, key);
    std::uint8_t diff = 0;
    for (std::size_t i = 0; i < 16; ++i) {
        diff |= static_cast<std::uint8_t>(expected[i] ^ tag[i]);
    }
    if (diff != 0) {
        throw AuthenticationError();
    }
    return detail::gcm_ctr(j0, ciphertext, key);
}

}  // namespace aes_modes
}  // namespace ca

#endif  // CA_AES_MODES_HPP
