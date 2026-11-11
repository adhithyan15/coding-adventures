/* Tests for the C LZW, using the iso_test.h harness. Compress/decompress
 * round-trips over inputs that exercise dictionary growth, the KwKwK "tricky
 * token" case (long runs), and larger data. */
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

#include "lzw.h"

/* Compress then decompress, asserting the result equals the original. */
static void round_trip(const uint8_t *input, size_t len) {
    uint8_t *packed = NULL;
    uint8_t *restored = NULL;
    size_t packed_len = 0, restored_len = 0;
    ISO_CHECK(lzw_compress(input, len, &packed, &packed_len));
    ISO_CHECK(packed_len >= 4); /* at least the length header */
    ISO_CHECK(lzw_decompress(packed, packed_len, &restored, &restored_len));
    ISO_CHECK_EQ_UINT(restored_len, len);
    if (restored_len == len && len > 0) {
        ISO_CHECK_MEM_EQ(restored, input, len);
    }
    free(packed);
    free(restored);
}


/* ---- Security regression tests: hostile headers ------------------------- *
 * Each case is a few bytes of input whose header claims an enormous count or
 * length.  An implementation that sized an allocation or loop from the claim
 * would allocate gigabytes or read out of bounds; these checks (best run under
 * ASan/UBSan, see the CHANGELOG) assert the call returns promptly with the
 * data actually present.  Also: round trips over every input of length <= 2
 * and a seeded random sample. */
static unsigned long hostile_rng_state = 0x5EEDu;
static unsigned hostile_rng(void) {
    /* xorshift32 */
    unsigned long x = hostile_rng_state & 0xFFFFFFFFul;
    x ^= (x << 13) & 0xFFFFFFFFul;
    x ^= (x >> 17);
    x ^= (x << 5) & 0xFFFFFFFFul;
    hostile_rng_state = x;
    return (unsigned)x;
}

static void hostile_round_trip(const uint8_t *d, size_t n) {
    uint8_t *packed = NULL;
    uint8_t *restored = NULL;
    size_t packed_len = 0, restored_len = 0;
    int ok = lzw_compress(d, n, &packed, &packed_len) &&
             lzw_decompress(packed, packed_len, &restored, &restored_len);
    ISO_CHECK(ok);
    if (ok) {
        ISO_CHECK_EQ_UINT(restored_len, n);
        if (restored_len == n && n > 0) {
            ISO_CHECK(memcmp(restored, d, n) == 0);
        }
    }
    free(packed);
    free(restored);
}

static void hostile_tests(void) {
    uint8_t *out = NULL;
    size_t out_len = 0;
    unsigned a, b, i;

    /* Header only / garbage: must fail or return promptly, never over-allocate. */
    {
        uint8_t d[64];
        memset(d, 0xFF, sizeof d);
        if (lzw_decompress(d, 4, &out, &out_len)) {
            free(out);
        }
        out = NULL;
        if (lzw_decompress(d, sizeof d, &out, &out_len)) {
            free(out);
        }
        out = NULL;
    }
    /* Declared length 2^32-1 on a small valid stream: output is what the codes
     * produce. */
    {
        const char *text = "hello hello hello";
        uint8_t *packed = NULL;
        size_t packed_len = 0;
        ISO_CHECK(lzw_compress((const uint8_t *)text, 17, &packed, &packed_len));
        if (packed != NULL && packed_len >= 4) {
            packed[0] = 0xFF;
            packed[1] = 0xFF;
            packed[2] = 0xFF;
            packed[3] = 0xFF;
            ISO_CHECK(lzw_decompress(packed, packed_len, &out, &out_len));
            ISO_CHECK_EQ_UINT(out_len, 17);
            ISO_CHECK(out_len == 17 && memcmp(out, text, 17) == 0);
            free(out);
            out = NULL;
            /* Declared length 5 truncates (and stops expanding early). */
            packed[0] = 0;
            packed[1] = 0;
            packed[2] = 0;
            packed[3] = 5;
            ISO_CHECK(lzw_decompress(packed, packed_len, &out, &out_len));
            ISO_CHECK_EQ_UINT(out_len, 5);
            ISO_CHECK(out_len == 5 && memcmp(out, "hello", 5) == 0);
            free(out);
            out = NULL;
        }
        free(packed);
    }

    hostile_round_trip((const uint8_t *)"", 0);
    for (a = 0; a < 256; a++) {
        uint8_t one[1];
        one[0] = (uint8_t)a;
        hostile_round_trip(one, 1);
        for (b = 0; b < 256; b++) {
            uint8_t two[2];
            two[0] = (uint8_t)a;
            two[1] = (uint8_t)b;
            hostile_round_trip(two, 2);
        }
    }
    for (i = 0; i < 300; i++) {
        uint8_t buf[600];
        unsigned n = 3 + hostile_rng() % 597;
        unsigned alpha = (i % 4 == 0) ? 2 : (i % 4 == 1) ? 4 : (i % 4 == 2) ? 16 : 256;
        unsigned j;
        for (j = 0; j < n; j++) {
            buf[j] = (uint8_t)(hostile_rng() % alpha);
        }
        hostile_round_trip(buf, n);
    }
}

int main(void) {
    /* Small canonical cases. */
    round_trip((const uint8_t *)"", 0);
    round_trip((const uint8_t *)"A", 1);
    round_trip((const uint8_t *)"AB", 2);
    round_trip((const uint8_t *)"TOBEORNOTTOBEORTOBEORNOT", 24);

    /* Long single-byte run — heavily exercises the KwKwK case. */
    {
        uint8_t buf[1000];
        memset(buf, 'A', sizeof buf);
        round_trip(buf, sizeof buf);
    }

    /* Repeating multi-byte pattern — exercises dictionary growth. */
    {
        uint8_t buf[900];
        size_t i;
        for (i = 0; i < sizeof buf; i++) {
            buf[i] = (uint8_t)("abc"[i % 3]);
        }
        round_trip(buf, sizeof buf);
    }

    /* All 256 byte values, a few times — exercises the full byte alphabet. */
    {
        uint8_t buf[1024];
        size_t i;
        for (i = 0; i < sizeof buf; i++) {
            buf[i] = (uint8_t)(i & 0xff);
        }
        round_trip(buf, sizeof buf);
    }

    /* Header sanity: the first 4 bytes are the big-endian original length. */
    {
        uint8_t *packed = NULL;
        size_t packed_len = 0;
        ISO_CHECK(lzw_compress((const uint8_t *)"hello", 5, &packed, &packed_len));
        ISO_CHECK_EQ_UINT(((size_t)packed[0] << 24) | ((size_t)packed[1] << 16) |
                              ((size_t)packed[2] << 8) | packed[3],
                          5);
        free(packed);
    }

    /* Malformed input (too short) is rejected. */
    {
        uint8_t *o = NULL;
        size_t ol = 0;
        ISO_CHECK(!lzw_decompress((const uint8_t *)"ab", 2, &o, &ol));
    }

    hostile_tests();

    return ISO_TEST_RESULT();
}
