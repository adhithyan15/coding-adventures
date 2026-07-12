// Tests for the C++ scrypt, using the header-only iso_test.h harness (pure ISO).
// Vectors are the published RFC 7914 §12 test vectors, matching the Rust
// crate's own tests.
#include "iso_test.h"

#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

#include "scrypt.hpp"

using Bytes = std::vector<std::uint8_t>;

static Bytes bytes(const char* s, std::size_t n) {
    return Bytes(reinterpret_cast<const std::uint8_t*>(s),
                 reinterpret_cast<const std::uint8_t*>(s) + n);
}

int main() {
    // ── RFC 7914 §12 vectors ───────────────────────────────────────────────
    ISO_CHECK(ca::scrypt_hex(bytes("", 0), bytes("", 0), 16, 1, 1, 64) ==
              "77d6576238657b203b19ca42c18a0497"
              "f16b4844e3074ae8dfdffa3fede21442"
              "fcd0069ded0948f8326a753a0fc81f17"
              "e8d3e0fb2e0d3628cf35e20c38d18906");
    ISO_CHECK(ca::scrypt_hex(bytes("password", 8), bytes("NaCl", 4), 1024, 8, 16,
                             64) ==
              "fdbabe1c9d3472007856e7190d01e9fe"
              "7c6ad7cbc8237830e77376634b373162"
              "2eaf30d92e22a3886ff109279d9830da"
              "c727afb94a83ee6d8360cbdfa2cc0640");
    ISO_CHECK(ca::scrypt_hex(bytes("pleaseletmein", 13),
                             bytes("SodiumChloride", 14), 16384, 8, 1, 64) ==
              "7023bdcb3afd7348461c06cd81fd38eb"
              "fda8fbba904f8e3ea9b543f6545da1f2"
              "d5432955613f0fcf62d49705242a9af9"
              "e61e85dc0d651e40dfcf017b45575887");

    // ── output properties ──────────────────────────────────────────────────
    {
        for (std::size_t len : {std::size_t(1), std::size_t(16), std::size_t(32),
                                std::size_t(64), std::size_t(100)}) {
            Bytes dk = ca::scrypt(bytes("pw", 2), bytes("salt", 4), 16, 1, 1,
                                  len);
            ISO_CHECK_EQ_UINT(dk.size(), static_cast<unsigned>(len));
        }
        // deterministic
        ISO_CHECK(ca::scrypt(bytes("password", 8), bytes("salt", 4), 16, 1, 1,
                             32) ==
                  ca::scrypt(bytes("password", 8), bytes("salt", 4), 16, 1, 1,
                             32));
        // password / salt / N sensitivity
        ISO_CHECK(ca::scrypt(bytes("password1", 9), bytes("salt", 4), 16, 1, 1,
                             32) !=
                  ca::scrypt(bytes("password2", 9), bytes("salt", 4), 16, 1, 1,
                             32));
        ISO_CHECK(ca::scrypt(bytes("password", 8), bytes("salt1", 5), 16, 1, 1,
                             32) !=
                  ca::scrypt(bytes("password", 8), bytes("salt2", 5), 16, 1, 1,
                             32));
        ISO_CHECK(ca::scrypt(bytes("password", 8), bytes("salt", 4), 16, 1, 1,
                             32) !=
                  ca::scrypt(bytes("password", 8), bytes("salt", 4), 32, 1, 1,
                             32));
    }

    // ── parameter validation (throw std::invalid_argument) ─────────────────
    {
        auto throws = [](auto fn) {
            try {
                fn();
            } catch (const std::invalid_argument&) {
                return true;
            }
            return false;
        };
        ISO_CHECK(throws(
            [] { ca::scrypt(bytes("p", 1), bytes("s", 1), 1, 1, 1, 32); }));
        ISO_CHECK(throws(
            [] { ca::scrypt(bytes("p", 1), bytes("s", 1), 0, 1, 1, 32); }));
        ISO_CHECK(throws(
            [] { ca::scrypt(bytes("p", 1), bytes("s", 1), 3, 1, 1, 32); }));
        ISO_CHECK(throws([] {
            ca::scrypt(bytes("p", 1), bytes("s", 1), ca::scrypt_max_n + 1, 1, 1,
                       32);
        }));
        ISO_CHECK(throws(
            [] { ca::scrypt(bytes("p", 1), bytes("s", 1), 2, 0, 1, 32); }));
        ISO_CHECK(throws(
            [] { ca::scrypt(bytes("p", 1), bytes("s", 1), 2, 1, 0, 32); }));
        ISO_CHECK(throws(
            [] { ca::scrypt(bytes("p", 1), bytes("s", 1), 2, 1, 1, 0); }));
        ISO_CHECK(throws([] {
            ca::scrypt(bytes("p", 1), bytes("s", 1), 2, 1, 1,
                       ca::scrypt_max_dk_len + 1);
        }));
        ISO_CHECK(throws([] {
            ca::scrypt(bytes("p", 1), bytes("s", 1), 2, std::size_t(1) << 15,
                       std::size_t(1) << 16, 32);
        }));  // p*r > 2^30
        ISO_CHECK(throws([] {
            ca::scrypt(bytes("p", 1), bytes("s", 1), 2, std::size_t(1) << 24, 1,
                       32);
        }));  // p*128*r > 2^30
    }

    // ── empty password / salt allowed ──────────────────────────────────────
    {
        ISO_CHECK_EQ_UINT(
            ca::scrypt(bytes("", 0), bytes("", 0), 16, 1, 1, 32).size(), 32u);
        ISO_CHECK_EQ_UINT(
            ca::scrypt(bytes("password", 8), bytes("", 0), 16, 1, 1, 32).size(),
            32u);
    }

    return ISO_TEST_RESULT();
}
