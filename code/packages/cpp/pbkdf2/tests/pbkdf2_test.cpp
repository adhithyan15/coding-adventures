// Tests for the C++ pbkdf2, using the header-only iso_test.h harness (pure ISO).
// Vectors are the published RFC 6070 (PBKDF2-HMAC-SHA1) and RFC 7914
// (PBKDF2-HMAC-SHA256) test vectors, matching the Rust crate's own tests.
#include "iso_test.h"

#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

#include "pbkdf2.hpp"

using Bytes = std::vector<std::uint8_t>;

static Bytes bytes(const char* s, std::size_t n) {
    return Bytes(reinterpret_cast<const std::uint8_t*>(s),
                 reinterpret_cast<const std::uint8_t*>(s) + n);
}

int main() {
    // ── RFC 6070 PBKDF2-HMAC-SHA1 ──────────────────────────────────────────
    {
        ISO_CHECK(ca::pbkdf2_hmac_sha1_hex(bytes("password", 8), bytes("salt", 4),
                                           1, 20) ==
                  "0c60c80f961f0e71f3a9b524af6012062fe037a6");
        ISO_CHECK(ca::pbkdf2_hmac_sha1_hex(bytes("password", 8), bytes("salt", 4),
                                           4096, 20) ==
                  "4b007901b765489abead49d926f721d065a429c1");
        // Long password & salt, multi-block.
        ISO_CHECK(
            ca::pbkdf2_hmac_sha1_hex(
                bytes("passwordPASSWORDpassword", 24),
                bytes("saltSALTsaltSALTsaltSALTsaltSALTsalt", 36), 4096, 25) ==
            "3d2eec4fe41c849b80c8d83662c0e44a8b291a964cf2f07038");
        // Embedded NUL bytes.
        ISO_CHECK(ca::pbkdf2_hmac_sha1_hex(bytes("pass\x00word", 9),
                                           bytes("sa\x00lt", 5), 4096, 16) ==
                  "56fa6aa75548099dcc37d7f03425e0c3");
    }

    // ── RFC 7914 PBKDF2-HMAC-SHA256 ────────────────────────────────────────
    {
        ISO_CHECK(ca::pbkdf2_hmac_sha256_hex(bytes("passwd", 6), bytes("salt", 4),
                                             1, 64) ==
                  "55ac046e56e3089fec1691c22544b605"
                  "f94185216dde0465e68b9d57c20dacbc"
                  "49ca9cccf179b645991664b39d77ef31"
                  "7c71b845b1e30bd509112041d3a19783");
        // Truncation consistency.
        Bytes s16 = ca::pbkdf2_hmac_sha256(bytes("key", 3), bytes("salt", 4), 1,
                                           16);
        Bytes f32 = ca::pbkdf2_hmac_sha256(bytes("key", 3), bytes("salt", 4), 1,
                                           32);
        ISO_CHECK(s16 == Bytes(f32.begin(), f32.begin() + 16));
        // hex matches bytes for the same inputs.
        Bytes dk = ca::pbkdf2_hmac_sha256(bytes("passwd", 6), bytes("salt", 4), 1,
                                          32);
        ISO_CHECK(ca::pbkdf2_hmac_sha256_hex(bytes("passwd", 6), bytes("salt", 4),
                                             1, 32) == ca::to_hex(dk));
    }

    // ── SHA-512 sanity ─────────────────────────────────────────────────────
    {
        Bytes a = ca::pbkdf2_hmac_sha512(bytes("secret", 6), bytes("nacl", 4), 1,
                                         64);
        ISO_CHECK_EQ_UINT(a.size(), 64u);
        Bytes b = ca::pbkdf2_hmac_sha512(bytes("secret", 6), bytes("nacl", 4), 1,
                                         64);
        ISO_CHECK(a == b);  // deterministic
        Bytes half = ca::pbkdf2_hmac_sha512(bytes("secret", 6), bytes("nacl", 4),
                                            1, 32);
        ISO_CHECK(half == Bytes(a.begin(), a.begin() + 32));  // truncation
    }

    // ── validation / error paths (throw std::invalid_argument) ─────────────
    {
        auto throws = [](auto fn) {
            try {
                fn();
            } catch (const std::invalid_argument&) {
                return true;
            }
            return false;
        };
        ISO_CHECK(throws([] {
            ca::pbkdf2_hmac_sha256(Bytes{}, bytes("salt", 4), 1, 32);
        }));  // empty password
        ISO_CHECK(throws([] {
            ca::pbkdf2_hmac_sha256(bytes("pw", 2), bytes("salt", 4), 0, 32);
        }));  // zero iterations
        ISO_CHECK(throws([] {
            ca::pbkdf2_hmac_sha256(bytes("pw", 2), bytes("salt", 4), 1, 0);
        }));  // zero key length
        ISO_CHECK(throws([] {
            ca::pbkdf2_hmac_sha256(bytes("pw", 2), bytes("salt", 4), 1,
                                   ca::pbkdf2_max_key_length + 1);
        }));  // too large

        // empty password allowed with the flag.
        Bytes dk = ca::pbkdf2_hmac_sha256(Bytes{}, bytes("salt", 4), 1, 32, true);
        ISO_CHECK_EQ_UINT(dk.size(), 32u);
        // empty salt allowed.
        Bytes dk2 = ca::pbkdf2_hmac_sha256(bytes("password", 8), Bytes{}, 1, 32);
        ISO_CHECK_EQ_UINT(dk2.size(), 32u);
    }

    // ── different inputs give different keys ───────────────────────────────
    {
        ISO_CHECK(ca::pbkdf2_hmac_sha256(bytes("password", 8), bytes("salt1", 5),
                                         1, 32) !=
                  ca::pbkdf2_hmac_sha256(bytes("password", 8), bytes("salt2", 5),
                                         1, 32));
        ISO_CHECK(ca::pbkdf2_hmac_sha256(bytes("password1", 9), bytes("salt", 4),
                                         1, 32) !=
                  ca::pbkdf2_hmac_sha256(bytes("password2", 9), bytes("salt", 4),
                                         1, 32));
    }

    return ISO_TEST_RESULT();
}
