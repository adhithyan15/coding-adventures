// Tests for the C++ argon2d, using the header-only iso_test.h harness (pure
// ISO). The primary vector is RFC 9106 §5.1 (Argon2d), matching the Rust crate.
#include "iso_test.h"

#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

#include "argon2d.hpp"

using Bytes = std::vector<std::uint8_t>;

static Bytes filled(std::size_t n, std::uint8_t b) { return Bytes(n, b); }

int main() {
    // ── RFC 9106 §5.1 Argon2d known-answer ──────────────────────────────────
    {
        ca::Argon2dOptions opts;
        opts.key = filled(8, 0x03);
        opts.associated_data = filled(12, 0x04);
        std::string h = ca::argon2d_hex(filled(32, 0x01), filled(16, 0x02), 3, 32,
                                        4, 32, opts);
        ISO_CHECK(h ==
                  "512b391b6f1162975371d30919734294f868e3be3984f3c1a13a4db9fabe4acb");
    }

    const Bytes pw = {'p', 'a', 's', 's', 'w', 'o', 'r', 'd'};
    const Bytes salt = {'s', 'o', 'm', 'e', 's', 'a', 'l', 't'};

    // ── determinism, sensitivity, hex matches bytes ─────────────────────────
    {
        Bytes a = ca::argon2d(pw, salt, 1, 8, 1, 32);
        Bytes b = ca::argon2d(pw, salt, 1, 8, 1, 32);
        ISO_CHECK(a == b);  // deterministic
        ISO_CHECK_EQ_UINT(a.size(), 32u);
        // hex matches bytes.
        std::string hex = ca::argon2d_hex(pw, salt, 1, 8, 1, 32);
        static const char* digits = "0123456789abcdef";
        std::string expect;
        for (std::uint8_t byte : a) {
            expect.push_back(digits[byte >> 4]);
            expect.push_back(digits[byte & 0x0F]);
        }
        ISO_CHECK(hex == expect);
        // sensitivity.
        ISO_CHECK(ca::argon2d({'p', '1'}, salt, 1, 8, 1, 32,
                              ca::Argon2dOptions{}) !=
                  ca::argon2d({'p', '2'}, salt, 1, 8, 1, 32));
        ISO_CHECK(ca::argon2d(pw, {'s', 'a', 'l', 't', 's', 'a', 'l', 't'}, 1, 8,
                              1, 32) !=
                  ca::argon2d(pw, {'s', 'a', 'l', 't', 's', 'a', 'l', '2'}, 1, 8,
                              1, 32));
        ISO_CHECK(ca::argon2d(pw, salt, 1, 8, 1, 32) !=
                  ca::argon2d(pw, salt, 2, 8, 1, 32));  // more passes
    }

    // ── key / associated data bind ──────────────────────────────────────────
    {
        Bytes base = ca::argon2d(pw, salt, 1, 8, 1, 32);
        ca::Argon2dOptions ko;
        ko.key = {'s', 'e', 'c', 'r', 'e', 't', '!', '!'};
        ISO_CHECK(ca::argon2d(pw, salt, 1, 8, 1, 32, ko) != base);
        ca::Argon2dOptions ao;
        ao.associated_data = {'a', 'd'};
        ISO_CHECK(ca::argon2d(pw, salt, 1, 8, 1, 32, ao) != base);
    }

    // ── tag length variants (including > 64, exercising H') ─────────────────
    {
        for (std::uint32_t tl : {4u, 16u, 32u, 64u, 65u, 128u}) {
            Bytes tag = ca::argon2d(pw, salt, 1, 8, 1, tl);
            ISO_CHECK_EQ_UINT(tag.size(), tl);
        }
    }

    // ── multi-lane parameters ───────────────────────────────────────────────
    {
        Bytes tag = ca::argon2d(filled(32, 0x01), filled(16, 0x02), 3, 32, 4, 32);
        ISO_CHECK_EQ_UINT(tag.size(), 32u);
    }

    // ── parameter validation (throws std::invalid_argument) ─────────────────
    {
        auto throws = [](auto fn) {
            try {
                fn();
            } catch (const std::invalid_argument&) {
                return true;
            }
            return false;
        };
        ISO_CHECK(throws([&] { ca::argon2d(pw, {'s', 'h', 'o', 'r', 't'}, 1, 8, 1, 32); }));
        ISO_CHECK(throws([&] { ca::argon2d(pw, salt, 1, 8, 1, 3); }));   // tag < 4
        ISO_CHECK(throws([&] { ca::argon2d(pw, salt, 1, 1, 1, 32); }));  // memory
        ISO_CHECK(throws([&] { ca::argon2d(pw, salt, 0, 8, 1, 32); }));  // time 0
        ISO_CHECK(throws([&] { ca::argon2d(pw, salt, 1, 8, 0, 32); }));  // p 0
        ca::Argon2dOptions badver;
        badver.version = 0x10;
        ISO_CHECK(throws([&] { ca::argon2d(pw, salt, 1, 8, 1, 32, badver); }));
    }

    return ISO_TEST_RESULT();
}
