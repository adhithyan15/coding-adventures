/* Tests for the C Huffman compression, using the iso_test.h harness.
 * Compress/decompress round-trips over varied frequency distributions, plus
 * header and edge-case checks. */
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

#include "huffman_compression.h"

static void round_trip(const uint8_t *input, size_t len) {
    uint8_t *packed = NULL;
    uint8_t *restored = NULL;
    size_t packed_len = 0, restored_len = 0;
    ISO_CHECK(huffman_compress(input, len, &packed, &packed_len));
    ISO_CHECK(huffman_decompress(packed, packed_len, &restored, &restored_len));
    ISO_CHECK_EQ_UINT(restored_len, len);
    if (restored_len == len && len > 0) {
        ISO_CHECK_MEM_EQ(restored, input, len);
    }
    free(packed);
    free(restored);
}

int main(void) {
    /* Empty input → 8-byte header, decodes to empty. */
    {
        uint8_t *packed = NULL, *restored = NULL;
        size_t pl = 0, rl = 0;
        ISO_CHECK(huffman_compress((const uint8_t *)"", 0, &packed, &pl));
        ISO_CHECK_EQ_UINT(pl, 8);
        ISO_CHECK(huffman_decompress(packed, pl, &restored, &rl));
        ISO_CHECK_EQ_UINT(rl, 0);
        free(packed);
        free(restored);
    }

    /* Single repeated byte (one distinct symbol → length-1 code). */
    {
        uint8_t buf[50];
        memset(buf, 'X', sizeof buf);
        round_trip(buf, sizeof buf);
    }

    /* Skewed distribution — the classic Huffman case. */
    round_trip((const uint8_t *)"aaaaaaaaaaaabbbbbbcccdde", 24);

    /* Natural-language text (varied frequencies). */
    round_trip((const uint8_t *)"the quick brown fox jumps over the lazy dog", 43);

    /* All 256 byte values, uneven counts. */
    {
        uint8_t buf[2000];
        size_t i;
        for (i = 0; i < sizeof buf; i++) {
            buf[i] = (uint8_t)((i * 7 + (i / 3)) & 0xff);
        }
        round_trip(buf, sizeof buf);
    }

    /* Two symbols. */
    round_trip((const uint8_t *)"ababababab", 10);

    /* Header carries the big-endian original length. */
    {
        uint8_t *packed = NULL;
        size_t pl = 0;
        ISO_CHECK(huffman_compress((const uint8_t *)"hello world", 11, &packed,
                                   &pl));
        ISO_CHECK_EQ_UINT(((size_t)packed[0] << 24) | ((size_t)packed[1] << 16) |
                              ((size_t)packed[2] << 8) | packed[3],
                          11);
        free(packed);
    }

    /* Too-short input is rejected. */
    {
        uint8_t *o = NULL;
        size_t ol = 0;
        ISO_CHECK(!huffman_decompress((const uint8_t *)"abc", 3, &o, &ol));
    }

    return ISO_TEST_RESULT();
}
