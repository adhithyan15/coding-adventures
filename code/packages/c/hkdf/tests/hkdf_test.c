/* Tests for the C HKDF, using the iso_test.h harness. HKDF-SHA256 is checked
 * against the published RFC 5869 vectors (the sibling `hmac` + `sha256` packages
 * supply the primitives, compiled in via tools/run.sh). */
#include "iso_test.h"

#include <string.h>

#include "hkdf.h"
#include "sha256.h"

static void to_hex(const uint8_t *d, size_t n, char *out) {
    static const char hex[] = "0123456789abcdef";
    size_t i;
    for (i = 0; i < n; i++) {
        out[i * 2] = hex[d[i] >> 4];
        out[i * 2 + 1] = hex[d[i] & 0x0f];
    }
    out[n * 2] = '\0';
}

int main(void) {
    /* HKDF-SHA256: digest 32, block 64. */
    uint8_t ikm[22];
    uint8_t okm[42];
    uint8_t prk[32];
    char hex[128];

    memset(ikm, 0x0b, sizeof ikm);

    /* RFC 5869 Test Case 1. */
    {
        const uint8_t salt[13] = {0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
                                  0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c};
        const uint8_t info[10] = {0xf0, 0xf1, 0xf2, 0xf3, 0xf4,
                                  0xf5, 0xf6, 0xf7, 0xf8, 0xf9};
        /* extract → PRK */
        ISO_CHECK_EQ_INT(hkdf_extract(sha256, 32, 64, salt, sizeof salt, ikm,
                                      sizeof ikm, prk),
                         HKDF_OK);
        to_hex(prk, 32, hex);
        ISO_CHECK_STR_EQ(
            hex,
            "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5");
        /* full hkdf → OKM (42 bytes) */
        ISO_CHECK_EQ_INT(hkdf(sha256, 32, 64, salt, sizeof salt, ikm, sizeof ikm,
                              info, sizeof info, okm, 42),
                         HKDF_OK);
        to_hex(okm, 42, hex);
        ISO_CHECK_STR_EQ(hex,
                         "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d"
                         "56ecc4c5bf34007208d5b887185865");
    }

    /* RFC 5869 Test Case 3: empty salt and empty info. */
    {
        ISO_CHECK_EQ_INT(
            hkdf(sha256, 32, 64, NULL, 0, ikm, sizeof ikm, NULL, 0, okm, 42),
            HKDF_OK);
        to_hex(okm, 42, hex);
        ISO_CHECK_STR_EQ(hex,
                         "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e"
                         "5f3c738d2d9d201395faa4b61a96c8");
    }

    /* Error paths. */
    ISO_CHECK_EQ_INT(hkdf(sha256, 32, 64, NULL, 0, ikm, sizeof ikm, NULL, 0, okm,
                          0),
                     HKDF_OUTPUT_TOO_SHORT);
    ISO_CHECK_EQ_INT(hkdf_expand(sha256, 32, 64, prk, 32, NULL, 0, okm,
                                 (size_t)255 * 32 + 1),
                     HKDF_OUTPUT_TOO_LONG);

    return ISO_TEST_RESULT();
}
