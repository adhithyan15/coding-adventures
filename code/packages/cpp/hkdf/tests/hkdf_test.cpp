// Tests for the C++ HKDF, using the iso_test.h harness. HKDF-SHA256 checked
// against RFC 5869 vectors (the sibling header-only `hmac` + `sha256` packages
// supply the primitives; include paths via run.sh).
#include "iso_test.h"

#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

#include "hkdf.hpp"
#include "sha256.hpp"

static std::string to_hex(const std::vector<std::uint8_t> &d) {
    static const char hex[] = "0123456789abcdef";
    std::string s;
    for (std::uint8_t b : d) {
        s.push_back(hex[b >> 4]);
        s.push_back(hex[b & 0x0f]);
    }
    return s;
}

// A SHA-256 hash callable (block 64, digest 32).
static std::vector<std::uint8_t> sha(const std::vector<std::uint8_t> &d) {
    ca::sha256_digest h = ca::sha256(d.data(), d.size());
    return std::vector<std::uint8_t>(h.begin(), h.end());
}

template <typename Ex, typename F> static bool throws(F body) {
    try {
        body();
    } catch (const Ex &) {
        return true;
    } catch (...) {
        return false;
    }
    return false;
}

int main() {
    std::vector<std::uint8_t> ikm(22, 0x0b);

    // RFC 5869 Test Case 1.
    {
        std::vector<std::uint8_t> salt = {0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
                                          0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c};
        std::vector<std::uint8_t> info = {0xf0, 0xf1, 0xf2, 0xf3, 0xf4,
                                          0xf5, 0xf6, 0xf7, 0xf8, 0xf9};
        auto prk = ca::hkdf_extract(sha, 64, 32, salt, ikm);
        ISO_CHECK_STR_EQ(
            to_hex(prk).c_str(),
            "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5");
        auto okm = ca::hkdf(sha, 64, 32, salt, ikm, info, 42);
        ISO_CHECK_EQ_UINT(okm.size(), 42);
        ISO_CHECK_STR_EQ(to_hex(okm).c_str(),
                         "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d"
                         "56ecc4c5bf34007208d5b887185865");
    }

    // RFC 5869 Test Case 3: empty salt and info.
    {
        auto okm = ca::hkdf(sha, 64, 32, {}, ikm, {}, 42);
        ISO_CHECK_STR_EQ(to_hex(okm).c_str(),
                         "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e"
                         "5f3c738d2d9d201395faa4b61a96c8");
    }

    // Error paths throw.
    ISO_CHECK(throws<std::invalid_argument>(
        [&] { (void)ca::hkdf(sha, 64, 32, {}, ikm, {}, 0); }));
    ISO_CHECK(throws<std::invalid_argument>([&] {
        std::vector<std::uint8_t> prk(32, 0);
        (void)ca::hkdf_expand(sha, 64, 32, prk, {}, static_cast<std::size_t>(255) * 32 + 1);
    }));

    return ISO_TEST_RESULT();
}
