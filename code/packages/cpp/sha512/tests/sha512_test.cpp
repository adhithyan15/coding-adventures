// Tests for the C++ SHA-512, using the iso_test.h harness. Pinned to the FIPS
// 180-4 test vectors, plus streaming and padding-boundary checks.
#include "iso_test.h"

#include <cstdint>
#include <string>

#include "sha512.hpp"

int main() {
    ISO_CHECK_STR_EQ(
        ca::sha512_hex(std::string("")).c_str(),
        "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce"
        "47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e");

    ISO_CHECK_STR_EQ(
        ca::sha512_hex(std::string("abc")).c_str(),
        "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a"
        "2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f");

    {
        const std::uint8_t expected[64] = {
            0xdd, 0xaf, 0x35, 0xa1, 0x93, 0x61, 0x7a, 0xba, 0xcc, 0x41, 0x73,
            0x49, 0xae, 0x20, 0x41, 0x31, 0x12, 0xe6, 0xfa, 0x4e, 0x89, 0xa9,
            0x7e, 0xa2, 0x0a, 0x9e, 0xee, 0xe6, 0x4b, 0x55, 0xd3, 0x9a, 0x21,
            0x92, 0x99, 0x2a, 0x27, 0x4f, 0xc1, 0xa8, 0x36, 0xba, 0x3c, 0x23,
            0xa3, 0xfe, 0xeb, 0xbd, 0x45, 0x4d, 0x44, 0x23, 0x64, 0x3c, 0xe8,
            0x0e, 0x2a, 0x9a, 0xc9, 0x4f, 0xa5, 0x4c, 0xa4, 0x9f};
        ca::sha512_digest d = ca::sha512(std::string("abc"));
        ISO_CHECK_MEM_EQ(d.data(), expected, 64);
    }

    // 112-byte padding-boundary vector.
    ISO_CHECK_STR_EQ(
        ca::sha512_hex(std::string(
                           "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmn"
                           "hijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu"))
            .c_str(),
        "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018"
        "501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909");

    {
        ca::sha512_hasher h;
        h.update(std::string("ab"));
        h.update(std::string("c"));
        ISO_CHECK_STR_EQ(
            h.hex_digest().c_str(),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a"
            "2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f");
        ISO_CHECK(h.digest() == ca::sha512(std::string("abc")));
    }

    return ISO_TEST_RESULT();
}
