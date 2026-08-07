// Tests for chacha20-poly1305, using the header-only iso_test.h harness (pure
// ISO). Pinned to the RFC 8439 test vectors: Poly1305 (§2.5.2) and the full
// ChaCha20-Poly1305 AEAD (§2.8.2), plus a round-trip and tamper checks.
#include "iso_test.h"

#include <array>
#include <cstdint>
#include <cstring>
#include <string>
#include <vector>

#include "chacha20_poly1305.hpp"

int main() {
    // ── ChaCha20 round-trip ────────────────────────────────────────────────
    {
        std::uint8_t key[32];
        for (unsigned i = 0; i < 32; i++) {
            key[i] = static_cast<std::uint8_t>(i);
        }
        std::uint8_t nonce[12] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0};
        std::string msg = "The quick brown fox jumps over the lazy dog.";
        std::vector<std::uint8_t> data(msg.begin(), msg.end());
        auto ct = ca::chacha20_encrypt(data, key, nonce, 1);
        ISO_CHECK(ct != data);
        auto pt = ca::chacha20_encrypt(ct, key, nonce, 1);
        ISO_CHECK(pt == data);
    }

    // ── Poly1305 (RFC 8439 §2.5.2) ─────────────────────────────────────────
    {
        std::uint8_t key[32] = {
            0x85, 0xd6, 0xbe, 0x78, 0x57, 0x55, 0x6d, 0x33, 0x7f, 0x44, 0x52,
            0xfe, 0x42, 0xd5, 0x06, 0xa8, 0x01, 0x03, 0x80, 0x8a, 0xfb, 0x0d,
            0xb2, 0xfd, 0x4a, 0xbf, 0xf6, 0xaf, 0x41, 0x49, 0xf5, 0x1b};
        std::string msg = "Cryptographic Forum Research Group";
        std::uint8_t expected[16] = {0xa8, 0x06, 0x1d, 0xc1, 0x30, 0x51,
                                     0x36, 0xc6, 0xc2, 0x2b, 0x8b, 0xaf,
                                     0x0c, 0x01, 0x27, 0xa9};
        auto tag = ca::poly1305_mac(
            reinterpret_cast<const std::uint8_t *>(msg.data()), msg.size(), key);
        ISO_CHECK_MEM_EQ(tag.data(), expected, 16);
    }

    // ── Full AEAD (RFC 8439 §2.8.2) ────────────────────────────────────────
    {
        std::uint8_t key[32] = {
            0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87, 0x88, 0x89, 0x8a,
            0x8b, 0x8c, 0x8d, 0x8e, 0x8f, 0x90, 0x91, 0x92, 0x93, 0x94, 0x95,
            0x96, 0x97, 0x98, 0x99, 0x9a, 0x9b, 0x9c, 0x9d, 0x9e, 0x9f};
        std::uint8_t nonce[12] = {0x07, 0x00, 0x00, 0x00, 0x40, 0x41,
                                  0x42, 0x43, 0x44, 0x45, 0x46, 0x47};
        std::vector<std::uint8_t> aad = {0x50, 0x51, 0x52, 0x53, 0xc0, 0xc1,
                                         0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7};
        std::string plaintext =
            "Ladies and Gentlemen of the class of '99: If I could offer you "
            "only one tip for the future, sunscreen would be it.";
        std::vector<std::uint8_t> pt(plaintext.begin(), plaintext.end());
        std::uint8_t expected_ct[114] = {
            0xd3, 0x1a, 0x8d, 0x34, 0x64, 0x8e, 0x60, 0xdb, 0x7b, 0x86, 0xaf,
            0xbc, 0x53, 0xef, 0x7e, 0xc2, 0xa4, 0xad, 0xed, 0x51, 0x29, 0x6e,
            0x08, 0xfe, 0xa9, 0xe2, 0xb5, 0xa7, 0x36, 0xee, 0x62, 0xd6, 0x3d,
            0xbe, 0xa4, 0x5e, 0x8c, 0xa9, 0x67, 0x12, 0x82, 0xfa, 0xfb, 0x69,
            0xda, 0x92, 0x72, 0x8b, 0x1a, 0x71, 0xde, 0x0a, 0x9e, 0x06, 0x0b,
            0x29, 0x05, 0xd6, 0xa5, 0xb6, 0x7e, 0xcd, 0x3b, 0x36, 0x92, 0xdd,
            0xbd, 0x7f, 0x2d, 0x77, 0x8b, 0x8c, 0x98, 0x03, 0xae, 0xe3, 0x28,
            0x09, 0x1b, 0x58, 0xfa, 0xb3, 0x24, 0xe4, 0xfa, 0xd6, 0x75, 0x94,
            0x55, 0x85, 0x80, 0x8b, 0x48, 0x31, 0xd7, 0xbc, 0x3f, 0xf4, 0xde,
            0xf0, 0x8e, 0x4b, 0x7a, 0x9d, 0xe5, 0x76, 0xd2, 0x65, 0x86, 0xce,
            0xc6, 0x4b, 0x61, 0x16};
        std::array<std::uint8_t, 16> expected_tag = {
            0x1a, 0xe1, 0x0b, 0x59, 0x4f, 0x09, 0xe2, 0x6a,
            0x7e, 0x90, 0x2e, 0xcb, 0xd0, 0x60, 0x06, 0x91};

        ISO_CHECK_EQ_UINT(pt.size(), 114);
        auto enc = ca::aead_encrypt(pt, key, nonce, aad);
        ISO_CHECK_EQ_UINT(enc.ciphertext.size(), 114);
        ISO_CHECK_MEM_EQ(enc.ciphertext.data(), expected_ct, 114);
        ISO_CHECK(enc.tag == expected_tag);

        // Decrypt recovers the plaintext.
        auto dec = ca::aead_decrypt(enc.ciphertext, key, nonce, aad, enc.tag);
        ISO_CHECK(dec.has_value());
        ISO_CHECK(dec.value() == pt);

        // Tampered ciphertext → no plaintext.
        auto bad_ct = enc.ciphertext;
        bad_ct[0] ^= 0x01;
        ISO_CHECK(!ca::aead_decrypt(bad_ct, key, nonce, aad, enc.tag).has_value());

        // Tampered AAD → no plaintext.
        auto bad_aad = aad;
        bad_aad[0] ^= 0x01;
        ISO_CHECK(
            !ca::aead_decrypt(enc.ciphertext, key, nonce, bad_aad, enc.tag)
                 .has_value());
    }

    // ── AEAD with empty AAD ────────────────────────────────────────────────
    {
        std::uint8_t key[32];
        for (unsigned i = 0; i < 32; i++) {
            key[i] = static_cast<std::uint8_t>(i);
        }
        std::uint8_t nonce[12] = {9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 1, 2};
        std::string msg = "no aad here";
        std::vector<std::uint8_t> pt(msg.begin(), msg.end());
        std::vector<std::uint8_t> no_aad;
        auto enc = ca::aead_encrypt(pt, key, nonce, no_aad);
        auto dec = ca::aead_decrypt(enc.ciphertext, key, nonce, no_aad, enc.tag);
        ISO_CHECK(dec.has_value());
        ISO_CHECK(dec.value() == pt);
    }

    return ISO_TEST_RESULT();
}
