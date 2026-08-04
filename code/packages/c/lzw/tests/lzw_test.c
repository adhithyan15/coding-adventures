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

    return ISO_TEST_RESULT();
}
