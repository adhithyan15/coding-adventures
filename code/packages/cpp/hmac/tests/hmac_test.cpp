// Tests for the C++ HMAC, using the iso_test.h harness. HMAC-SHA256 checked
// against RFC 4231 vectors (the sibling header-only `sha256` package supplies
// the hash). Also exercises key-longer-than-block and constant-time verify.
#include "iso_test.h"

#include <cstdint>
#include <string>
#include <vector>

#include "hmac.hpp"
#include "sha256.hpp" // sibling package (header-only); include path via run.sh

// HMAC-SHA256 as a hash-agnostic instantiation (block size 64).
static std::vector<std::uint8_t> hmac_sha256(const std::vector<std::uint8_t> &key,
                                             const std::vector<std::uint8_t> &msg) {
    auto hash = [](const std::vector<std::uint8_t> &d) {
        ca::sha256_digest h = ca::sha256(d.data(), d.size());
        return std::vector<std::uint8_t>(h.begin(), h.end());
    };
    return ca::hmac(hash, 64, key, msg);
}

static std::string to_hex(const std::vector<std::uint8_t> &d) {
    static const char hex[] = "0123456789abcdef";
    std::string s;
    for (std::uint8_t b : d) {
        s.push_back(hex[b >> 4]);
        s.push_back(hex[b & 0x0f]);
    }
    return s;
}

static std::vector<std::uint8_t> bytes(const std::string &s) {
    return std::vector<std::uint8_t>(s.begin(), s.end());
}

int main() {
    // RFC 4231 Test Case 1.
    ISO_CHECK_STR_EQ(
        to_hex(hmac_sha256(std::vector<std::uint8_t>(20, 0x0b), bytes("Hi There")))
            .c_str(),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7");

    // RFC 4231 Test Case 2 (also as raw bytes).
    {
        auto mac =
            hmac_sha256(bytes("Jefe"), bytes("what do ya want for nothing?"));
        const std::uint8_t expected[32] = {
            0x5b, 0xdc, 0xc1, 0x46, 0xbf, 0x60, 0x75, 0x4e, 0x6a, 0x04, 0x24,
            0x26, 0x08, 0x95, 0x75, 0xc7, 0x5a, 0x00, 0x3f, 0x08, 0x9d, 0x27,
            0x39, 0x83, 0x9d, 0xec, 0x58, 0xb9, 0x64, 0xec, 0x38, 0x43};
        ISO_CHECK_EQ_UINT(mac.size(), 32);
        ISO_CHECK_MEM_EQ(mac.data(), expected, 32);
    }

    // RFC 4231 Test Case 6 (key longer than the block → hashed first).
    ISO_CHECK_STR_EQ(
        to_hex(hmac_sha256(
                   std::vector<std::uint8_t>(131, 0xaa),
                   bytes("Test Using Larger Than Block-Size Key - Hash Key First")))
            .c_str(),
        "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54");

    // Constant-time verify.
    {
        std::vector<std::uint8_t> a{1, 2, 3, 4}, b{1, 2, 3, 4}, c{1, 2, 3, 5};
        ISO_CHECK(ca::hmac_verify(a, b));
        ISO_CHECK(!ca::hmac_verify(a, c));
    }

    return ISO_TEST_RESULT();
}
