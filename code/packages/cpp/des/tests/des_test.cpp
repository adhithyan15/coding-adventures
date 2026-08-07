// Tests for the C++ DES, using the iso_test.h harness. Pinned to the FIPS 46 and
// NIST SP 800-20 known-answer vectors, plus round-trips, ECB, and Triple DES.
#include "iso_test.h"

#include <array>
#include <cstdint>
#include <optional>
#include <vector>

#include "des.hpp"

using block_t = ca::des::block_t;

int main() {
    // FIPS 46 worked example.
    {
        block_t key = {0x13, 0x34, 0x57, 0x79, 0x9B, 0xBC, 0xDF, 0xF1};
        block_t plain = {0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF};
        block_t ct = {0x85, 0xE8, 0x13, 0x54, 0x0F, 0x0A, 0xB4, 0x05};
        ISO_CHECK(ca::des::encrypt_block(plain, key) == ct);
        ISO_CHECK(ca::des::decrypt_block(ct, key) == plain);
    }

    // NIST SP 800-20 Table 1 (plaintext variable, key = all 0x01).
    {
        block_t key = {1, 1, 1, 1, 1, 1, 1, 1};
        struct {
            block_t pt, ct;
        } v[3] = {
            {{0x95, 0xF8, 0xA5, 0xE5, 0xDD, 0x31, 0xD9, 0x00},
             {0x80, 0, 0, 0, 0, 0, 0, 0}},
            {{0xDD, 0x7F, 0x12, 0x1C, 0xA5, 0x01, 0x56, 0x19},
             {0x40, 0, 0, 0, 0, 0, 0, 0}},
            {{0x2E, 0x86, 0x53, 0x10, 0x4F, 0x38, 0x34, 0xEA},
             {0x20, 0, 0, 0, 0, 0, 0, 0}},
        };
        for (auto &pair : v) {
            ISO_CHECK(ca::des::encrypt_block(pair.pt, key) == pair.ct);
        }
    }

    // NIST SP 800-20 Table 2 (key variable, plaintext = all zero).
    {
        block_t pt = {0, 0, 0, 0, 0, 0, 0, 0};
        struct {
            block_t key, ct;
        } v[2] = {
            {{0x80, 1, 1, 1, 1, 1, 1, 1},
             {0x95, 0xA8, 0xD7, 0x28, 0x13, 0xDA, 0xA9, 0x4D}},
            {{0x40, 1, 1, 1, 1, 1, 1, 1},
             {0x0E, 0xEC, 0x14, 0x87, 0xDD, 0x8C, 0x26, 0xD5}},
        };
        for (auto &pair : v) {
            ISO_CHECK(ca::des::encrypt_block(pt, pair.key) == pair.ct);
        }
    }

    // Round-trips across every byte value.
    {
        block_t key = {0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10};
        bool ok = true;
        for (int start = 0; start <= 248; start += 8) {
            block_t block{};
            for (int i = 0; i < 8; i++) {
                block[static_cast<std::size_t>(i)] =
                    static_cast<std::uint8_t>(start + i);
            }
            if (ca::des::decrypt_block(ca::des::encrypt_block(block, key), key) !=
                block) {
                ok = false;
            }
        }
        ISO_CHECK_MSG(ok, "encrypt/decrypt must round-trip for all blocks");
    }

    // ECB with PKCS#7 padding.
    {
        block_t key = {0x01, 0x33, 0x45, 0x77, 0x99, 0xBB, 0xCD, 0xFF};
        std::vector<std::uint8_t> msg = {'h', 'e', 'l', 'l', 'o'};
        auto ct = ca::des::ecb_encrypt(msg, key);
        ISO_CHECK_EQ_UINT(ct.size(), 8);
        auto pt = ca::des::ecb_decrypt(ct, key);
        ISO_CHECK(pt.has_value() && pt.value() == msg);

        // Block-aligned input → a full extra padding block.
        std::vector<std::uint8_t> eight = {1, 2, 3, 4, 5, 6, 7, 8};
        auto c2 = ca::des::ecb_encrypt(eight, key);
        ISO_CHECK_EQ_UINT(c2.size(), 16);
        auto p2 = ca::des::ecb_decrypt(c2, key);
        ISO_CHECK(p2.has_value() && p2.value() == eight);

        // Bad length rejected.
        std::vector<std::uint8_t> junk(7, 0);
        ISO_CHECK(!ca::des::ecb_decrypt(junk, key).has_value());
    }

    // Triple DES round-trip and single-DES reduction.
    {
        block_t k1 = {0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF};
        block_t k2 = {0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01};
        block_t k3 = {0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23};
        block_t plain = {0x6B, 0xC1, 0xBE, 0xE2, 0x2E, 0x40, 0x9F, 0x96};
        auto ct = ca::des::tdea_encrypt_block(plain, k1, k2, k3);
        ISO_CHECK(ca::des::tdea_decrypt_block(ct, k1, k2, k3) == plain);
        // K1 = K2 = K3 → 3DES == single DES.
        ISO_CHECK(ca::des::tdea_encrypt_block(plain, k1, k1, k1) ==
                  ca::des::encrypt_block(plain, k1));
    }

    return ISO_TEST_RESULT();
}
