// Tests for the C++ SHA-1, using the iso_test.h harness. Pinned to the FIPS test
// vectors, plus streaming and padding-boundary checks.
#include "iso_test.h"

#include <cstdint>
#include <string>

#include "sha1.hpp"

int main() {
    ISO_CHECK_STR_EQ(ca::sha1_hex(std::string("")).c_str(),
                     "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    ISO_CHECK_STR_EQ(ca::sha1_hex(std::string("abc")).c_str(),
                     "a9993e364706816aba3e25717850c26c9cd0d89d");

    {
        const std::uint8_t expected[20] = {
            0xa9, 0x99, 0x3e, 0x36, 0x47, 0x06, 0x81, 0x6a, 0xba, 0x3e,
            0x25, 0x71, 0x78, 0x50, 0xc2, 0x6c, 0x9c, 0xd0, 0xd8, 0x9d};
        ca::sha1_digest d = ca::sha1(std::string("abc"));
        ISO_CHECK_MEM_EQ(d.data(), expected, 20);
    }

    ISO_CHECK_STR_EQ(
        ca::sha1_hex(std::string(
                         "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"))
            .c_str(),
        "84983e441c3bd26ebaae4aa1f95129e5e54670f1");

    {
        ca::sha1_hasher h;
        h.update(std::string("ab"));
        h.update(std::string("c"));
        ISO_CHECK_STR_EQ(h.hex_digest().c_str(),
                         "a9993e364706816aba3e25717850c26c9cd0d89d");
        ISO_CHECK(h.digest() == ca::sha1(std::string("abc")));
    }

    return ISO_TEST_RESULT();
}
