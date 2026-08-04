/* Tests for the C scrypt, using the header-only iso_test.h harness (pure ISO).
 * Vectors are the published RFC 7914 §12 test vectors, matching the Rust
 * crate's own tests. */
#include "iso_test.h"

#include <string.h> /* memcmp */

#include "scrypt.h"

/* Decode a lowercase hex string into `out` (out must hold strlen(hex)/2). */
static void from_hex(const char *hex, uint8_t *out) {
    size_t i;
    for (i = 0; hex[i] && hex[i + 1]; i += 2) {
        int hi = hex[i] <= '9' ? hex[i] - '0' : (hex[i] | 0x20) - 'a' + 10;
        int lo = hex[i + 1] <= '9' ? hex[i + 1] - '0'
                                   : (hex[i + 1] | 0x20) - 'a' + 10;
        out[i / 2] = (uint8_t)((hi << 4) | lo);
    }
}

int main(void) {
    /* ── RFC 7914 §12 vector 1: scrypt("", "", 16, 1, 1, 64) ────────────── */
    {
        uint8_t dk[64];
        uint8_t want[64];
        ISO_CHECK_EQ_INT(
            (int)scrypt((const uint8_t *)"", 0, (const uint8_t *)"", 0, 16, 1, 1,
                        64, dk),
            (int)SCRYPT_OK);
        from_hex("77d6576238657b203b19ca42c18a0497"
                 "f16b4844e3074ae8dfdffa3fede21442"
                 "fcd0069ded0948f8326a753a0fc81f17"
                 "e8d3e0fb2e0d3628cf35e20c38d18906",
                 want);
        ISO_CHECK_MEM_EQ(dk, want, 64);
    }

    /* ── vector 2: scrypt("password", "NaCl", 1024, 8, 16, 64) ──────────── */
    {
        uint8_t dk[64];
        uint8_t want[64];
        ISO_CHECK_EQ_INT(
            (int)scrypt((const uint8_t *)"password", 8, (const uint8_t *)"NaCl",
                        4, 1024, 8, 16, 64, dk),
            (int)SCRYPT_OK);
        from_hex("fdbabe1c9d3472007856e7190d01e9fe"
                 "7c6ad7cbc8237830e77376634b373162"
                 "2eaf30d92e22a3886ff109279d9830da"
                 "c727afb94a83ee6d8360cbdfa2cc0640",
                 want);
        ISO_CHECK_MEM_EQ(dk, want, 64);
    }

    /* ── vector 3: scrypt("pleaseletmein","SodiumChloride",16384,8,1,64) ── */
    {
        uint8_t dk[64];
        uint8_t want[64];
        ISO_CHECK_EQ_INT(
            (int)scrypt((const uint8_t *)"pleaseletmein", 13,
                        (const uint8_t *)"SodiumChloride", 14, 16384, 8, 1, 64,
                        dk),
            (int)SCRYPT_OK);
        from_hex("7023bdcb3afd7348461c06cd81fd38eb"
                 "fda8fbba904f8e3ea9b543f6545da1f2"
                 "d5432955613f0fcf62d49705242a9af9"
                 "e61e85dc0d651e40dfcf017b45575887",
                 want);
        ISO_CHECK_MEM_EQ(dk, want, 64);
    }

    /* ── output properties: length, determinism, sensitivity ───────────── */
    {
        uint8_t a[32];
        uint8_t b[32];
        size_t lens[5];
        size_t li;
        lens[0] = 1;
        lens[1] = 16;
        lens[2] = 32;
        lens[3] = 64;
        lens[4] = 100;
        for (li = 0; li < 5; li++) {
            uint8_t buf[100];
            ISO_CHECK_EQ_INT((int)scrypt((const uint8_t *)"pw", 2,
                                         (const uint8_t *)"salt", 4, 16, 1, 1,
                                         lens[li], buf),
                             (int)SCRYPT_OK);
        }
        /* deterministic. */
        scrypt((const uint8_t *)"password", 8, (const uint8_t *)"salt", 4, 16, 1,
               1, 32, a);
        scrypt((const uint8_t *)"password", 8, (const uint8_t *)"salt", 4, 16, 1,
               1, 32, b);
        ISO_CHECK_MEM_EQ(a, b, 32);
        /* password sensitivity. */
        scrypt((const uint8_t *)"password1", 9, (const uint8_t *)"salt", 4, 16, 1,
               1, 32, a);
        scrypt((const uint8_t *)"password2", 9, (const uint8_t *)"salt", 4, 16, 1,
               1, 32, b);
        ISO_CHECK(memcmp(a, b, 32) != 0);
        /* salt sensitivity. */
        scrypt((const uint8_t *)"password", 8, (const uint8_t *)"salt1", 5, 16, 1,
               1, 32, a);
        scrypt((const uint8_t *)"password", 8, (const uint8_t *)"salt2", 5, 16, 1,
               1, 32, b);
        ISO_CHECK(memcmp(a, b, 32) != 0);
        /* N sensitivity. */
        scrypt((const uint8_t *)"password", 8, (const uint8_t *)"salt", 4, 16, 1,
               1, 32, a);
        scrypt((const uint8_t *)"password", 8, (const uint8_t *)"salt", 4, 32, 1,
               1, 32, b);
        ISO_CHECK(memcmp(a, b, 32) != 0);
    }

    /* ── parameter validation (mirrors the Rust unit tests) ─────────────── */
    {
        uint8_t dk[32];
        ISO_CHECK_EQ_INT((int)scrypt((const uint8_t *)"p", 1,
                                     (const uint8_t *)"s", 1, 1, 1, 1, 32, dk),
                         (int)SCRYPT_INVALID_N); /* N < 2 */
        ISO_CHECK_EQ_INT((int)scrypt((const uint8_t *)"p", 1,
                                     (const uint8_t *)"s", 1, 0, 1, 1, 32, dk),
                         (int)SCRYPT_INVALID_N); /* N == 0 */
        ISO_CHECK_EQ_INT((int)scrypt((const uint8_t *)"p", 1,
                                     (const uint8_t *)"s", 1, 3, 1, 1, 32, dk),
                         (int)SCRYPT_INVALID_N); /* not a power of two */
        ISO_CHECK_EQ_INT(
            (int)scrypt((const uint8_t *)"p", 1, (const uint8_t *)"s", 1,
                        SCRYPT_MAX_N + 1, 1, 1, 32, dk),
            (int)SCRYPT_N_TOO_LARGE);
        ISO_CHECK_EQ_INT((int)scrypt((const uint8_t *)"p", 1,
                                     (const uint8_t *)"s", 1, 2, 0, 1, 32, dk),
                         (int)SCRYPT_INVALID_R);
        ISO_CHECK_EQ_INT((int)scrypt((const uint8_t *)"p", 1,
                                     (const uint8_t *)"s", 1, 2, 1, 0, 32, dk),
                         (int)SCRYPT_INVALID_P);
        ISO_CHECK_EQ_INT((int)scrypt((const uint8_t *)"p", 1,
                                     (const uint8_t *)"s", 1, 2, 1, 1, 0, dk),
                         (int)SCRYPT_INVALID_KEY_LENGTH);
        ISO_CHECK_EQ_INT(
            (int)scrypt((const uint8_t *)"p", 1, (const uint8_t *)"s", 1, 2, 1, 1,
                        SCRYPT_MAX_DK_LEN + 1, dk),
            (int)SCRYPT_KEY_LENGTH_TOO_LARGE);
        /* p*r > 2^30. */
        ISO_CHECK_EQ_INT(
            (int)scrypt((const uint8_t *)"p", 1, (const uint8_t *)"s", 1, 2,
                        (size_t)1 << 15, (size_t)1 << 16, 32, dk),
            (int)SCRYPT_PR_TOO_LARGE);
        /* p*r <= 2^30 but p*128*r > 2^30. */
        ISO_CHECK_EQ_INT(
            (int)scrypt((const uint8_t *)"p", 1, (const uint8_t *)"s", 1, 2,
                        (size_t)1 << 24, 1, 32, dk),
            (int)SCRYPT_PR_TOO_LARGE);
    }

    return ISO_TEST_RESULT();
}
