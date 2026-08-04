// Tests for the C++ SHA-256, using the iso_test.h harness. Pinned to the FIPS
// 180-4 test vectors, plus streaming and boundary checks.
#include "iso_test.h"

#include <array>
#include <cstdint>
#include <string>

#include "sha256.hpp"

int main() {
    // Empty string.
    ISO_CHECK_STR_EQ(
        ca::sha256_hex(std::string("")).c_str(),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");

    // "abc".
    ISO_CHECK_STR_EQ(
        ca::sha256_hex(std::string("abc")).c_str(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");

    // Raw digest via ISO_CHECK_MEM_EQ.
    {
        const std::uint8_t expected[32] = {
            0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40,
            0xde, 0x5d, 0xae, 0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17,
            0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61, 0xf2, 0x00, 0x15, 0xad};
        ca::sha256_digest d = ca::sha256(std::string("abc"));
        ISO_CHECK_MEM_EQ(d.data(), expected, 32);
    }

    // 56-byte padding-boundary vector.
    ISO_CHECK_STR_EQ(
        ca::sha256_hex(std::string(
                           "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"))
            .c_str(),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");

    // Streaming equals one-shot.
    {
        ca::sha256_hasher h;
        h.update(std::string("ab"));
        h.update(std::string("c"));
        ISO_CHECK_STR_EQ(
            h.hex_digest().c_str(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        // digest() is repeatable (does not consume the hasher).
        ISO_CHECK(h.digest() == ca::sha256(std::string("abc")));
    }

    // 64 'a' characters (padding spills into a second block).
    ISO_CHECK_STR_EQ(
        ca::sha256_hex(std::string(64, 'a')).c_str(),
        "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb");

    return ISO_TEST_RESULT();
}
