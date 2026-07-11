// Tests for the C++ AES, using the iso_test.h harness. Pinned to the FIPS 197
// known-answer vectors (Appendices B and C) plus S-box properties.
#include "iso_test.h"

#include <array>
#include <cstdint>
#include <vector>

#include "aes.hpp"

using block_t = ca::aes::block_t;

int main() {
    // AES-128, FIPS 197 Appendix B.
    {
        std::vector<std::uint8_t> key = {0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae,
                                         0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88,
                                         0x09, 0xcf, 0x4f, 0x3c};
        block_t plain = {0x32, 0x43, 0xf6, 0xa8, 0x88, 0x5a, 0x30, 0x8d,
                         0x31, 0x31, 0x98, 0xa2, 0xe0, 0x37, 0x07, 0x34};
        block_t ct = {0x39, 0x25, 0x84, 0x1d, 0x02, 0xdc, 0x09, 0xfb,
                      0xdc, 0x11, 0x85, 0x97, 0x19, 0x6a, 0x0b, 0x32};
        ISO_CHECK(ca::aes::encrypt_block(plain, key) == ct);
        ISO_CHECK(ca::aes::decrypt_block(ct, key) == plain);
    }

    // AES-128, FIPS 197 Appendix C.1.
    {
        std::vector<std::uint8_t> key = {0, 1, 2,  3,  4,  5,  6,  7,
                                         8, 9, 10, 11, 12, 13, 14, 15};
        block_t plain = {0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
                         0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff};
        block_t ct = {0x69, 0xc4, 0xe0, 0xd8, 0x6a, 0x7b, 0x04, 0x30,
                      0xd8, 0xcd, 0xb7, 0x80, 0x70, 0xb4, 0xc5, 0x5a};
        ISO_CHECK(ca::aes::encrypt_block(plain, key) == ct);
        ISO_CHECK(ca::aes::decrypt_block(ct, key) == plain);
    }

    // AES-192, FIPS 197 Appendix C.2.
    {
        std::vector<std::uint8_t> key(24);
        for (int i = 0; i < 24; i++) {
            key[static_cast<std::size_t>(i)] = static_cast<std::uint8_t>(i);
        }
        block_t plain = {0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
                         0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff};
        block_t ct = {0xdd, 0xa9, 0x7c, 0xa4, 0x86, 0x4c, 0xdf, 0xe0,
                      0x6e, 0xaf, 0x70, 0xa0, 0xec, 0x0d, 0x71, 0x91};
        ISO_CHECK(ca::aes::encrypt_block(plain, key) == ct);
        ISO_CHECK(ca::aes::decrypt_block(ct, key) == plain);
    }

    // AES-256, FIPS 197 Appendix C.3.
    {
        std::vector<std::uint8_t> key(32);
        for (int i = 0; i < 32; i++) {
            key[static_cast<std::size_t>(i)] = static_cast<std::uint8_t>(i);
        }
        block_t plain = {0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
                         0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff};
        block_t ct = {0x8e, 0xa2, 0xb7, 0xca, 0x51, 0x67, 0x45, 0xbf,
                      0xea, 0xfc, 0x49, 0x90, 0x4b, 0x49, 0x60, 0x89};
        ISO_CHECK(ca::aes::encrypt_block(plain, key) == ct);
        ISO_CHECK(ca::aes::decrypt_block(ct, key) == plain);
    }

    // Invalid key length → nullopt.
    {
        block_t block{};
        std::vector<std::uint8_t> key(20, 0);
        ISO_CHECK(!ca::aes::encrypt_block(block, key).has_value());
        ISO_CHECK(!ca::aes::decrypt_block(block, key).has_value());
    }

    // S-box properties.
    {
        const auto &sb = ca::aes::sbox();
        const auto &isb = ca::aes::inv_sbox();
        ISO_CHECK_EQ_UINT(sb[0x00], 0x63);
        ISO_CHECK_EQ_UINT(sb[0x01], 0x7c);
        ISO_CHECK_EQ_UINT(sb[0xff], 0x16);
        std::array<bool, 256> seen{};
        bool inverse_ok = true;
        for (int i = 0; i < 256; i++) {
            seen[sb[static_cast<std::size_t>(i)]] = true;
            if (isb[sb[static_cast<std::size_t>(i)]] != static_cast<std::uint8_t>(i)) {
                inverse_ok = false;
            }
        }
        bool bijection = true;
        for (int i = 0; i < 256; i++) {
            if (!seen[static_cast<std::size_t>(i)]) {
                bijection = false;
            }
        }
        ISO_CHECK_MSG(bijection, "S-box must be a permutation");
        ISO_CHECK_MSG(inverse_ok, "inv_sbox[sbox[b]] == b");
    }

    return ISO_TEST_RESULT();
}
