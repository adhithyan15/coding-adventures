/*
 * Tests for the C x25519 port, using the header-only iso_test.h harness (pure
 * ISO). These are the authoritative RFC 7748 §5.2 / §6.1 test vectors — the
 * gold-standard correctness proof for an X25519 implementation. Passing them
 * (including the 1000-iteration stress test) exercises every field operation,
 * the ladder, (de)serialization, and the constant-time swap.
 */
#include "iso_test.h"

#include <stdint.h>
#include <string.h>

#include "x25519.h"

/* Decode a 64-char hex string into 32 bytes. */
static void hex32(const char *hex, uint8_t out[32]) {
    for (int i = 0; i < 32; i++) {
        int hi = hex[2 * i], lo = hex[2 * i + 1];
        hi = (hi <= '9') ? hi - '0' : (hi | 0x20) - 'a' + 10;
        lo = (lo <= '9') ? lo - '0' : (lo | 0x20) - 'a' + 10;
        out[i] = (uint8_t)((hi << 4) | lo);
    }
}

/* Assert x25519(scalar, u) == expected. */
static void check_x25519(const char *scalar_hex, const char *u_hex,
                         const char *expected_hex) {
    uint8_t scalar[32], u[32], expected[32], out[32];
    hex32(scalar_hex, scalar);
    hex32(u_hex, u);
    hex32(expected_hex, expected);
    ISO_CHECK(x25519(out, scalar, u) == 0);
    ISO_CHECK_MEM_EQ(out, expected, 32);
}

int main(void) {
    /* ── RFC 7748 §5.2 test vectors ─────────────────────────────────────── */
    check_x25519(
        "a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4",
        "e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c",
        "c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552");
    check_x25519(
        "4b66e9d4d1b4673c5ad22691957d6af5c11b6421e0ea01d42ca4169e7918ba0d",
        "e5210f12786811d3f4b7959d0538ae2c31dbe7106fc03c3efc4cd549c715a493",
        "95cbde9476e8907d7aade45cb4b873f88b595a68799fa152e6f8f7647aac7957");

    /* ── RFC 7748 §6.1 Diffie-Hellman worked example ────────────────────── */
    {
        uint8_t alice_priv[32], bob_priv[32];
        uint8_t alice_pub[32], bob_pub[32];
        uint8_t shared_ab[32], shared_ba[32], expected[32];
        hex32("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a",
              alice_priv);
        hex32("5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb",
              bob_priv);

        ISO_CHECK(x25519_base(alice_pub, alice_priv) == 0);
        hex32("8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a",
              expected);
        ISO_CHECK_MEM_EQ(alice_pub, expected, 32);

        ISO_CHECK(x25519_base(bob_pub, bob_priv) == 0);
        hex32("de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f",
              expected);
        ISO_CHECK_MEM_EQ(bob_pub, expected, 32);

        ISO_CHECK(x25519(shared_ab, alice_priv, bob_pub) == 0);
        ISO_CHECK(x25519(shared_ba, bob_priv, alice_pub) == 0);
        hex32("4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742",
              expected);
        ISO_CHECK_MEM_EQ(shared_ab, expected, 32);
        ISO_CHECK_MEM_EQ(shared_ba, expected, 32); /* both sides agree */

        /* generate_keypair is an alias of x25519_base. */
        uint8_t kp[32];
        ISO_CHECK(x25519_generate_keypair(kp, alice_priv) == 0);
        ISO_CHECK_MEM_EQ(kp, alice_pub, 32);
    }

    /* ── the base point is u = 9 ────────────────────────────────────────── */
    {
        uint8_t nine[32];
        memset(nine, 0, 32);
        nine[0] = 9;
        ISO_CHECK_MEM_EQ(X25519_BASE_POINT, nine, 32);
    }

    /* ── RFC 7748 §5.2 iterated test — 1000 rounds of k' = X25519(k, u),
     *    then u := k, k := k'. The result after 1 and 1000 rounds is pinned. */
    {
        uint8_t k[32], u[32], new_k[32], expected[32];
        memset(k, 0, 32);
        memset(u, 0, 32);
        k[0] = 9;
        u[0] = 9;
        for (int i = 0; i < 1000; i++) {
            ISO_CHECK(x25519(new_k, k, u) == 0);
            if (i == 0) {
                hex32("422c8e7a6227d7bca1350b3e2bb7279f7897b87bb6854b783c60e803"
                      "11ae3079",
                      expected);
                ISO_CHECK_MEM_EQ(new_k, expected, 32); /* after 1 iteration */
            }
            memcpy(u, k, 32);
            memcpy(k, new_k, 32);
        }
        hex32("684cf59ba83309552800ef566f2f4d3c1c3887c49360e3875f2eb94d99532c51",
              expected);
        ISO_CHECK_MEM_EQ(k, expected, 32); /* after 1000 iterations */
    }

    return ISO_TEST_RESULT();
}
