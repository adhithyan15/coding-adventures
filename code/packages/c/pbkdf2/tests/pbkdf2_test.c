/* Tests for the C pbkdf2, using the header-only iso_test.h harness (pure ISO).
 * Vectors are the published RFC 6070 (PBKDF2-HMAC-SHA1) and RFC 7914
 * (PBKDF2-HMAC-SHA256) test vectors, matching the Rust crate's own tests. */
#include "iso_test.h"

#include <string.h> /* memcmp */

#include "pbkdf2.h"

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
    /* ── RFC 6070 PBKDF2-HMAC-SHA1 ──────────────────────────────────────── */
    {
        uint8_t dk[25];
        uint8_t want[25];

        /* c=1, dkLen=20. */
        ISO_CHECK_EQ_INT(
            (int)pbkdf2_hmac_sha1((const uint8_t *)"password", 8,
                                  (const uint8_t *)"salt", 4, 1, dk, 20, 0),
            (int)PBKDF2_OK);
        from_hex("0c60c80f961f0e71f3a9b524af6012062fe037a6", want);
        ISO_CHECK_MEM_EQ(dk, want, 20);

        /* c=4096, dkLen=20. */
        ISO_CHECK_EQ_INT(
            (int)pbkdf2_hmac_sha1((const uint8_t *)"password", 8,
                                  (const uint8_t *)"salt", 4, 4096, dk, 20, 0),
            (int)PBKDF2_OK);
        from_hex("4b007901b765489abead49d926f721d065a429c1", want);
        ISO_CHECK_MEM_EQ(dk, want, 20);

        /* Long password & salt, c=4096, dkLen=25 (multi-block). */
        ISO_CHECK_EQ_INT(
            (int)pbkdf2_hmac_sha1(
                (const uint8_t *)"passwordPASSWORDpassword", 24,
                (const uint8_t *)"saltSALTsaltSALTsaltSALTsaltSALTsalt", 36,
                4096, dk, 25, 0),
            (int)PBKDF2_OK);
        from_hex("3d2eec4fe41c849b80c8d83662c0e44a8b291a964cf2f07038", want);
        ISO_CHECK_MEM_EQ(dk, want, 25);

        /* Embedded NUL bytes, c=4096, dkLen=16. */
        ISO_CHECK_EQ_INT(
            (int)pbkdf2_hmac_sha1((const uint8_t *)"pass\x00word", 9,
                                  (const uint8_t *)"sa\x00lt", 5, 4096, dk, 16,
                                  0),
            (int)PBKDF2_OK);
        from_hex("56fa6aa75548099dcc37d7f03425e0c3", want);
        ISO_CHECK_MEM_EQ(dk, want, 16);
    }

    /* ── RFC 7914 PBKDF2-HMAC-SHA256 ────────────────────────────────────── */
    {
        uint8_t dk[64];
        uint8_t want[64];

        /* c=1, dkLen=64 (two 32-byte blocks). */
        ISO_CHECK_EQ_INT(
            (int)pbkdf2_hmac_sha256((const uint8_t *)"passwd", 6,
                                    (const uint8_t *)"salt", 4, 1, dk, 64, 0),
            (int)PBKDF2_OK);
        from_hex(
            "55ac046e56e3089fec1691c22544b605"
            "f94185216dde0465e68b9d57c20dacbc"
            "49ca9cccf179b645991664b39d77ef31"
            "7c71b845b1e30bd509112041d3a19783",
            want);
        ISO_CHECK_MEM_EQ(dk, want, 64);

        /* Truncation consistency: 16-byte DK is the prefix of the 32-byte DK. */
        {
            uint8_t s16[16];
            uint8_t f32[32];
            pbkdf2_hmac_sha256((const uint8_t *)"key", 3,
                               (const uint8_t *)"salt", 4, 1, s16, 16, 0);
            pbkdf2_hmac_sha256((const uint8_t *)"key", 3,
                               (const uint8_t *)"salt", 4, 1, f32, 32, 0);
            ISO_CHECK_MEM_EQ(s16, f32, 16);
        }
    }

    /* ── SHA-512 sanity: deterministic, correct length ──────────────────── */
    {
        uint8_t a[64];
        uint8_t b[64];
        ISO_CHECK_EQ_INT(
            (int)pbkdf2_hmac_sha512((const uint8_t *)"secret", 6,
                                    (const uint8_t *)"nacl", 4, 1, a, 64, 0),
            (int)PBKDF2_OK);
        ISO_CHECK_EQ_INT(
            (int)pbkdf2_hmac_sha512((const uint8_t *)"secret", 6,
                                    (const uint8_t *)"nacl", 4, 1, b, 64, 0),
            (int)PBKDF2_OK);
        ISO_CHECK_MEM_EQ(a, b, 64); /* deterministic */
    }

    /* ── validation / error paths ───────────────────────────────────────── */
    {
        uint8_t dk[32];
        /* empty password rejected unless allowed. */
        ISO_CHECK_EQ_INT((int)pbkdf2_hmac_sha256((const uint8_t *)"", 0,
                                                 (const uint8_t *)"salt", 4, 1,
                                                 dk, 32, 0),
                         (int)PBKDF2_EMPTY_PASSWORD);
        /* empty password allowed with the flag. */
        ISO_CHECK_EQ_INT((int)pbkdf2_hmac_sha256((const uint8_t *)"", 0,
                                                 (const uint8_t *)"salt", 4, 1,
                                                 dk, 32, 1),
                         (int)PBKDF2_OK);
        /* zero iterations / zero length / too large. */
        ISO_CHECK_EQ_INT((int)pbkdf2_hmac_sha256((const uint8_t *)"pw", 2,
                                                 (const uint8_t *)"salt", 4, 0,
                                                 dk, 32, 0),
                         (int)PBKDF2_INVALID_ITERATIONS);
        ISO_CHECK_EQ_INT((int)pbkdf2_hmac_sha256((const uint8_t *)"pw", 2,
                                                 (const uint8_t *)"salt", 4, 1,
                                                 dk, 0, 0),
                         (int)PBKDF2_INVALID_KEY_LENGTH);
        ISO_CHECK_EQ_INT(
            (int)pbkdf2_hmac_sha256((const uint8_t *)"pw", 2,
                                    (const uint8_t *)"salt", 4, 1, dk,
                                    PBKDF2_MAX_KEY_LENGTH + 1, 0),
            (int)PBKDF2_KEY_LENGTH_TOO_LARGE);
        /* NULL output buffer. */
        ISO_CHECK_EQ_INT((int)pbkdf2_hmac_sha256((const uint8_t *)"pw", 2,
                                                 (const uint8_t *)"salt", 4, 1,
                                                 NULL, 32, 0),
                         (int)PBKDF2_BAD_ARGS);
        /* empty salt is allowed. */
        ISO_CHECK_EQ_INT((int)pbkdf2_hmac_sha256((const uint8_t *)"password", 8,
                                                 NULL, 0, 1, dk, 32, 0),
                         (int)PBKDF2_OK);
    }

    /* ── different inputs give different keys ────────────────────────────── */
    {
        uint8_t a[32];
        uint8_t b[32];
        pbkdf2_hmac_sha256((const uint8_t *)"password", 8,
                           (const uint8_t *)"salt1", 5, 1, a, 32, 0);
        pbkdf2_hmac_sha256((const uint8_t *)"password", 8,
                           (const uint8_t *)"salt2", 5, 1, b, 32, 0);
        ISO_CHECK(memcmp(a, b, 32) != 0); /* different salts */
        pbkdf2_hmac_sha256((const uint8_t *)"password", 8,
                           (const uint8_t *)"salt", 4, 1, a, 32, 0);
        pbkdf2_hmac_sha256((const uint8_t *)"password", 8,
                           (const uint8_t *)"salt", 4, 2, b, 32, 0);
        ISO_CHECK(memcmp(a, b, 32) != 0); /* different iteration counts */
    }

    return ISO_TEST_RESULT();
}
