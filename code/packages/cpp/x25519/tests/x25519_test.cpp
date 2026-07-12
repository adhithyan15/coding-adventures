// Tests for the C++ x25519 port, using the header-only iso_test.h harness (pure
// ISO). These are the authoritative RFC 7748 §5.2 / §6.1 test vectors.
#include "iso_test.h"

#include <array>
#include <cstdint>
#include <optional>

#include "x25519.hpp"

using ca::x25519::Key;

static Key hex32(const char* hex) {
    Key out{};
    for (int i = 0; i < 32; i++) {
        int hi = hex[2 * i], lo = hex[2 * i + 1];
        hi = (hi <= '9') ? hi - '0' : (hi | 0x20) - 'a' + 10;
        lo = (lo <= '9') ? lo - '0' : (lo | 0x20) - 'a' + 10;
        out[static_cast<std::size_t>(i)] =
            static_cast<std::uint8_t>((hi << 4) | lo);
    }
    return out;
}

static void check_x25519(const char* scalar, const char* u,
                         const char* expected) {
    auto r = ca::x25519::x25519(hex32(scalar), hex32(u));
    ISO_CHECK(r.has_value());
    if (r) ISO_CHECK(*r == hex32(expected));
}

int main() {
    // ── RFC 7748 §5.2 test vectors ───────────────────────────────────────
    check_x25519(
        "a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4",
        "e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c",
        "c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552");
    check_x25519(
        "4b66e9d4d1b4673c5ad22691957d6af5c11b6421e0ea01d42ca4169e7918ba0d",
        "e5210f12786811d3f4b7959d0538ae2c31dbe7106fc03c3efc4cd549c715a493",
        "95cbde9476e8907d7aade45cb4b873f88b595a68799fa152e6f8f7647aac7957");

    // ── RFC 7748 §6.1 Diffie-Hellman worked example ──────────────────────
    {
        Key alice_priv = hex32(
            "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a");
        Key bob_priv = hex32(
            "5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb");

        auto alice_pub = ca::x25519::x25519_base(alice_priv);
        ISO_CHECK(alice_pub.has_value());
        ISO_CHECK(alice_pub && *alice_pub ==
                                   hex32("8520f0098930a754748b7ddcb43ef75a0dbf3"
                                         "a0d26381af4eba4a98eaa9b4e6a"));
        auto bob_pub = ca::x25519::x25519_base(bob_priv);
        ISO_CHECK(bob_pub.has_value());
        ISO_CHECK(bob_pub && *bob_pub ==
                                 hex32("de9edb7d7b7dc1b4d35b61c2ece435373f8343c"
                                       "85b78674dadfc7e146f882b4f"));

        Key expected = hex32(
            "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742");
        auto shared_ab = ca::x25519::x25519(alice_priv, *bob_pub);
        auto shared_ba = ca::x25519::x25519(bob_priv, *alice_pub);
        ISO_CHECK(shared_ab && *shared_ab == expected);
        ISO_CHECK(shared_ba && *shared_ba == expected);

        auto kp = ca::x25519::generate_keypair(alice_priv);
        ISO_CHECK(kp && *kp == *alice_pub);
    }

    // ── the base point is u = 9 ──────────────────────────────────────────
    {
        Key nine{};
        nine[0] = 9;
        ISO_CHECK(ca::x25519::BASE_POINT == nine);
    }

    // ── RFC 7748 §5.2 iterated test — 1 and 1000 rounds ──────────────────
    {
        Key k{}, u{};
        k[0] = 9;
        u[0] = 9;
        for (int i = 0; i < 1000; i++) {
            auto nk = ca::x25519::x25519(k, u);
            ISO_CHECK(nk.has_value());
            if (i == 0) {
                ISO_CHECK(nk && *nk ==
                                    hex32("422c8e7a6227d7bca1350b3e2bb7279f7897b"
                                          "87bb6854b783c60e80311ae3079"));
            }
            u = k;
            k = *nk;
        }
        ISO_CHECK(k == hex32("684cf59ba83309552800ef566f2f4d3c1c3887c49360e3875f"
                             "2eb94d99532c51"));
    }

    return ISO_TEST_RESULT();
}
