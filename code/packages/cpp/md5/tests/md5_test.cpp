// Tests for the C++ MD5, using the iso_test.h harness. Pinned to the RFC 1321
// test suite, plus streaming and padding-boundary checks.
#include "iso_test.h"

#include <cstdint>
#include <string>

#include "md5.hpp"

int main() {
    ISO_CHECK_STR_EQ(ca::md5_hex(std::string("")).c_str(),
                     "d41d8cd98f00b204e9800998ecf8427e");
    ISO_CHECK_STR_EQ(ca::md5_hex(std::string("a")).c_str(),
                     "0cc175b9c0f1b6a831c399e269772661");
    ISO_CHECK_STR_EQ(ca::md5_hex(std::string("abc")).c_str(),
                     "900150983cd24fb0d6963f7d28e17f72");
    ISO_CHECK_STR_EQ(ca::md5_hex(std::string("message digest")).c_str(),
                     "f96b697d7cb7938d525a2f31aaf161d0");

    {
        const std::uint8_t expected[16] = {0x90, 0x01, 0x50, 0x98, 0x3c, 0xd2,
                                           0x4f, 0xb0, 0xd6, 0x96, 0x3f, 0x7d,
                                           0x28, 0xe1, 0x7f, 0x72};
        ca::md5_digest d = ca::md5(std::string("abc"));
        ISO_CHECK_MEM_EQ(d.data(), expected, 16);
    }

    ISO_CHECK_STR_EQ(
        ca::md5_hex(std::string(
                        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"))
            .c_str(),
        "d174ab98d277d9f5a5611c2c9f419d9f");

    {
        ca::md5_hasher h;
        h.update(std::string("ab"));
        h.update(std::string("c"));
        ISO_CHECK_STR_EQ(h.hex_digest().c_str(),
                         "900150983cd24fb0d6963f7d28e17f72");
        ISO_CHECK(h.digest() == ca::md5(std::string("abc")));
    }

    return ISO_TEST_RESULT();
}
