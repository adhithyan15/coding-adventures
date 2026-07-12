// Tests for the C++ aes-modes, using the header-only iso_test.h harness (pure
// ISO). Vectors are NIST SP 800-38A (ECB/CBC) and the classic GCM test cases,
// matching the Rust crate's own tests.
#include "iso_test.h"

#include <array>
#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

#include "aes_modes.hpp"

namespace am = ca::aes_modes;
using Bytes = std::vector<std::uint8_t>;

static Bytes from_hex(const std::string& hex) {
    Bytes out;
    for (std::size_t i = 0; i + 1 < hex.size(); i += 2) {
        auto nib = [](char c) {
            return c <= '9' ? c - '0' : (c | 0x20) - 'a' + 10;
        };
        out.push_back(static_cast<std::uint8_t>((nib(hex[i]) << 4) |
                                                nib(hex[i + 1])));
    }
    return out;
}

int main() {
    const Bytes key = from_hex("2b7e151628aed2a6abf7158809cf4f3c");
    const Bytes pt = from_hex(
        "6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e51"
        "30c81c46a35ce411e5fbc1191a0a52eff69f2445df4f9b17ad2b417be66c3710");

    // ── PKCS#7 ──────────────────────────────────────────────────────────────
    {
        Bytes aligned(16, 0xAA);
        Bytes padded = am::pkcs7_pad(aligned);
        ISO_CHECK_EQ_UINT(padded.size(), 32u);
        for (std::size_t i = 16; i < 32; ++i) {
            ISO_CHECK(padded[i] == 16);
        }
        Bytes thirteen(13, 0xBB);
        Bytes p13 = am::pkcs7_pad(thirteen);
        ISO_CHECK_EQ_UINT(p13.size(), 16u);
        ISO_CHECK(p13[13] == 3 && p13[15] == 3);
        Bytes five = {1, 2, 3, 4, 5};
        ISO_CHECK(am::pkcs7_unpad(am::pkcs7_pad(five)) == five);
        auto throws = [](auto fn) {
            try {
                fn();
            } catch (const std::invalid_argument&) {
                return true;
            }
            return false;
        };
        Bytes bad(16, 0);
        bad[15] = 0;
        ISO_CHECK(throws([&] { am::pkcs7_unpad(bad); }));
        bad[15] = 2;
        bad[14] = 3;
        ISO_CHECK(throws([&] { am::pkcs7_unpad(bad); }));
    }

    // ── ECB (NIST SP 800-38A) ───────────────────────────────────────────────
    {
        Bytes ct = am::ecb_encrypt(pt, key);
        ISO_CHECK_EQ_UINT(ct.size(), 80u);
        ISO_CHECK(Bytes(ct.begin(), ct.begin() + 16) ==
                  from_hex("3ad77bb40d7a3660a89ecaf32466ef97"));
        ISO_CHECK(Bytes(ct.begin() + 16, ct.begin() + 32) ==
                  from_hex("f5d3d58503b9699de785895a96fdbaaf"));
        ISO_CHECK(am::ecb_decrypt(ct, key) == pt);
    }

    // ── CBC (NIST SP 800-38A) ───────────────────────────────────────────────
    {
        Bytes iv = from_hex("000102030405060708090a0b0c0d0e0f");
        Bytes ct = am::cbc_encrypt(pt, key, iv);
        ISO_CHECK_EQ_UINT(ct.size(), 80u);
        ISO_CHECK(Bytes(ct.begin(), ct.begin() + 16) ==
                  from_hex("7649abac8119b246cee98e9b12e9197d"));
        ISO_CHECK(Bytes(ct.begin() + 16, ct.begin() + 32) ==
                  from_hex("5086cb9b507219ee95db113a917678b2"));
        ISO_CHECK(am::cbc_decrypt(ct, key, iv) == pt);
        bool threw = false;
        try {
            am::cbc_encrypt(pt, key, Bytes(8, 0));
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK(threw);  // wrong IV length
    }

    // ── CTR (round trip; no padding) ────────────────────────────────────────
    {
        Bytes nonce = from_hex("f0f1f2f3f4f5f6f7f8f9fafb");
        Bytes ct = am::ctr_encrypt(pt, key, nonce);
        ISO_CHECK_EQ_UINT(ct.size(), pt.size());  // no padding
        ISO_CHECK(am::ctr_decrypt(ct, key, nonce) == pt);
        bool threw = false;
        try {
            am::ctr_encrypt(Bytes{0}, key, Bytes(16, 0));
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK(threw);  // wrong nonce length
    }

    // ── GCM test case 3 (64-byte plaintext, empty AAD) ──────────────────────
    {
        Bytes gkey = from_hex("feffe9928665731c6d6a8f9467308308");
        Bytes iv = from_hex("cafebabefacedbaddecaf888");
        Bytes gpt = from_hex(
            "d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a72"
            "1c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b391aafd255");
        Bytes want_ct = from_hex(
            "42831ec2217774244b7221b784d0d49ce3aa212f2c02a4e035c17e2329aca12e"
            "21d514b25466931c7d8f6a5aac84aa051ba30b396a0aac973d58e091473f5985");
        Bytes want_tag_v = from_hex("4d5c2af327cd64a62cf35abd2ba6fab4");
        am::Block want_tag;
        for (std::size_t i = 0; i < 16; ++i) {
            want_tag[i] = want_tag_v[i];
        }

        auto [ct, tag] = am::gcm_encrypt(gpt, gkey, iv, Bytes{});
        ISO_CHECK(ct == want_ct);
        ISO_CHECK(tag == want_tag);
        ISO_CHECK(am::gcm_decrypt(ct, gkey, iv, Bytes{}, tag) == gpt);

        // tampered ciphertext / tag rejected.
        auto auth_fails = [&](Bytes c, am::Block t) {
            try {
                am::gcm_decrypt(c, gkey, iv, Bytes{}, t);
            } catch (const am::AuthenticationError&) {
                return true;
            }
            return false;
        };
        Bytes tampered = ct;
        tampered[0] ^= 0x01;
        ISO_CHECK(auth_fails(tampered, tag));
        am::Block bad_tag = tag;
        bad_tag[0] ^= 0x01;
        ISO_CHECK(auth_fails(ct, bad_tag));
    }

    // ── GCM empty pt / empty aad (tag known-answer) ─────────────────────────
    {
        Bytes gkey(16, 0);
        Bytes iv(12, 0);
        auto [ct, tag] = am::gcm_encrypt(Bytes{}, gkey, iv, Bytes{});
        ISO_CHECK_EQ_UINT(ct.size(), 0u);
        Bytes want = from_hex("58e2fccefa7e3061367f1d57a4e7455a");
        ISO_CHECK(Bytes(tag.begin(), tag.end()) == want);
    }

    // ── GCM with AAD round trip; wrong AAD rejected ─────────────────────────
    {
        Bytes gkey = from_hex("feffe9928665731c6d6a8f9467308308");
        Bytes iv = from_hex("cafebabefacedbaddecaf888");
        Bytes gpt = from_hex("d9313225f88406e5a55909c5aff5269a");
        Bytes aad = from_hex("feedfacedeadbeeffeedfacedeadbeef");
        auto [ct, tag] = am::gcm_encrypt(gpt, gkey, iv, aad);
        ISO_CHECK(am::gcm_decrypt(ct, gkey, iv, aad, tag) == gpt);
        bool threw = false;
        try {
            am::gcm_decrypt(ct, gkey, iv, from_hex("deadbeeffeedface"), tag);
        } catch (const am::AuthenticationError&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    return ISO_TEST_RESULT();
}
