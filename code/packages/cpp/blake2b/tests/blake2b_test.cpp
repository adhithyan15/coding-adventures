// Tests for the C++ BLAKE2b, using the iso_test.h harness. Pinned to the RFC
// 7693 / reference test vectors, plus digest-size, streaming, and keyed checks.
#include "iso_test.h"

#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

#include "blake2b.hpp"

int main() {
    // BLAKE2b-512 of empty and "abc".
    ISO_CHECK_STR_EQ(
        ca::blake2b_hex(std::string("")).c_str(),
        "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419"
        "d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce");
    ISO_CHECK_STR_EQ(
        ca::blake2b_hex(std::string("abc")).c_str(),
        "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1"
        "7d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923");

    // Raw digest via ISO_CHECK_MEM_EQ.
    {
        const std::uint8_t expected[64] = {
            0xba, 0x80, 0xa5, 0x3f, 0x98, 0x1c, 0x4d, 0x0d, 0x6a, 0x27, 0x97,
            0xb6, 0x9f, 0x12, 0xf6, 0xe9, 0x4c, 0x21, 0x2f, 0x14, 0x68, 0x5a,
            0xc4, 0xb7, 0x4b, 0x12, 0xbb, 0x6f, 0xdb, 0xff, 0xa2, 0xd1, 0x7d,
            0x87, 0xc5, 0x39, 0x2a, 0xab, 0x79, 0x2d, 0xc2, 0x52, 0xd5, 0xde,
            0x45, 0x33, 0xcc, 0x95, 0x18, 0xd3, 0x8a, 0xa8, 0xdb, 0xf1, 0x92,
            0x5a, 0xb9, 0x23, 0x86, 0xed, 0xd4, 0x00, 0x99, 0x23};
        auto d = ca::blake2b(std::string("abc"));
        ISO_CHECK_EQ_UINT(d.size(), 64);
        ISO_CHECK_MEM_EQ(d.data(), expected, 64);
    }

    // BLAKE2b-256 of "abc".
    ISO_CHECK_STR_EQ(
        ca::blake2b_hex(std::string("abc"), 32).c_str(),
        "bddd813c634239723171ef3fee98579b94964e3bb1cb3e427262c8c068d52319");

    // Streaming equals one-shot.
    {
        ca::blake2b_hasher h(64);
        h.update(std::string("ab"));
        h.update(std::string("c"));
        ISO_CHECK_STR_EQ(
            h.hex_digest().c_str(),
            "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1"
            "7d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923");
        ISO_CHECK(h.digest() == ca::blake2b(std::string("abc")));
    }

    // Keyed: deterministic and different from unkeyed.
    {
        std::vector<std::uint8_t> key{1, 2, 3, 4, 5, 6, 7, 8};
        ca::blake2b_hasher a(64, key);
        a.update(std::string("message"));
        ca::blake2b_hasher b(64, key);
        b.update(std::string("message"));
        ISO_CHECK(a.digest() == b.digest());
        ISO_CHECK(a.digest() != ca::blake2b(std::string("message")));
    }

    // Invalid parameters throw.
    {
        bool threw = false;
        try {
            ca::blake2b_hasher bad(0);
        } catch (const std::invalid_argument &) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    return ISO_TEST_RESULT();
}
