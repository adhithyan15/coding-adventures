/* Tests for the C HMAC, using the iso_test.h harness. HMAC-SHA256 is checked
 * against the published RFC 4231 test vectors (the sibling `sha256` package
 * supplies the hash). Also exercises key-longer-than-block and constant-time
 * verify. */
#include "iso_test.h"

#include <string.h>

#include "hmac.h"
#include "sha256.h" /* sibling package; compiled in via tools/run.sh */

/* HMAC-SHA256 convenience: block size 64, digest size 32. */
static int hmac_sha256(const uint8_t *key, size_t keylen, const uint8_t *msg,
                       size_t msglen, uint8_t out[32]) {
    return hmac_compute(sha256, 32, 64, key, keylen, msg, msglen, out);
}

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
    uint8_t mac[32];
    char hex[65];

    /* RFC 4231 Test Case 1: key = 20 * 0x0b, data = "Hi There". */
    {
        uint8_t key[20];
        memset(key, 0x0b, sizeof key);
        ISO_CHECK(hmac_sha256(key, sizeof key, (const uint8_t *)"Hi There", 8,
                              mac));
        to_hex(mac, 32, hex);
        ISO_CHECK_STR_EQ(
            hex,
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7");
    }

    /* RFC 4231 Test Case 2: key = "Jefe", data = "what do ya want for nothing?".
     * Also verified as raw bytes via ISO_CHECK_MEM_EQ. */
    {
        const uint8_t expected[32] = {
            0x5b, 0xdc, 0xc1, 0x46, 0xbf, 0x60, 0x75, 0x4e, 0x6a, 0x04, 0x24,
            0x26, 0x08, 0x95, 0x75, 0xc7, 0x5a, 0x00, 0x3f, 0x08, 0x9d, 0x27,
            0x39, 0x83, 0x9d, 0xec, 0x58, 0xb9, 0x64, 0xec, 0x38, 0x43};
        ISO_CHECK(hmac_sha256((const uint8_t *)"Jefe", 4,
                              (const uint8_t *)"what do ya want for nothing?", 28,
                              mac));
        ISO_CHECK_MEM_EQ(mac, expected, 32);
    }

    /* RFC 4231 Test Case 6: key = 131 * 0xaa (longer than the 64-byte block, so
     * it is hashed first), data = the long "Test Using..." string. */
    {
        uint8_t key[131];
        const char *data =
            "Test Using Larger Than Block-Size Key - Hash Key First";
        memset(key, 0xaa, sizeof key);
        ISO_CHECK(hmac_sha256(key, sizeof key, (const uint8_t *)data,
                              strlen(data), mac));
        to_hex(mac, 32, hex);
        ISO_CHECK_STR_EQ(
            hex,
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54");
    }

    /* Constant-time verify: equal vs. one-byte-different. */
    {
        uint8_t a[4] = {1, 2, 3, 4};
        uint8_t b[4] = {1, 2, 3, 4};
        uint8_t c[4] = {1, 2, 3, 5};
        ISO_CHECK(hmac_verify(a, b, 4));
        ISO_CHECK(!hmac_verify(a, c, 4));
    }

    return ISO_TEST_RESULT();
}
