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

/* Build a CMP04 stream from explicit (symbol, length) pairs for hostile-input
 * tests. Returns the total length written into buf. */
static size_t hostile(uint8_t *buf, const uint8_t *pairs, size_t n_entries,
                      unsigned long original, size_t payload_len,
                      uint8_t payload_byte) {
    size_t i, pos = 0;
    buf[pos++] = (uint8_t)(original >> 24);
    buf[pos++] = (uint8_t)(original >> 16);
    buf[pos++] = (uint8_t)(original >> 8);
    buf[pos++] = (uint8_t)original;
    buf[pos++] = (uint8_t)(n_entries >> 24);
    buf[pos++] = (uint8_t)(n_entries >> 16);
    buf[pos++] = (uint8_t)(n_entries >> 8);
    buf[pos++] = (uint8_t)n_entries;
    for (i = 0; i < n_entries * 2; i++) {
        buf[pos++] = pairs[i];
    }
    for (i = 0; i < payload_len; i++) {
        buf[pos++] = payload_byte;
    }
    return pos;
}

/* Decompress hostile input; the decoder must return 0 and never crash. */
static void expect_rejected(const uint8_t *data, size_t len) {
    uint8_t *o = NULL;
    size_t ol = 0;
    ISO_CHECK(!huffman_decompress(data, len, &o, &ol));
    free(o);
}

static void hostile_tests(void) {
    static uint8_t buf[100000 + 1024];
    size_t n;

    /* Huge length gap 1 -> 255. */
    {
        const uint8_t t[] = {0, 1, 1, 255};
        n = hostile(buf, t, 2, 4, 8, 0xff);
        expect_rejected(buf, n);
    }
    /* Zero length. */
    {
        const uint8_t t[] = {0, 0};
        n = hostile(buf, t, 1, 1, 1, 0);
        expect_rejected(buf, n);
    }
    /* Over-subscribed: three 1-bit codes. */
    {
        const uint8_t t[] = {0, 1, 1, 1, 2, 1};
        n = hostile(buf, t, 3, 1, 1, 0);
        expect_rejected(buf, n);
    }
    /* Unsorted. */
    {
        const uint8_t t[] = {0, 2, 1, 1};
        n = hostile(buf, t, 2, 1, 1, 0);
        expect_rejected(buf, n);
    }
    /* More than 256 entries. */
    {
        static uint8_t t[257 * 2];
        size_t i;
        for (i = 0; i < 257; i++) {
            t[2 * i] = (uint8_t)(i & 0xff);
            t[2 * i + 1] = 9;
        }
        n = hostile(buf, t, 257, 1, 1, 0);
        expect_rejected(buf, n);
    }
    /* Duplicate symbol. */
    {
        const uint8_t t[] = {0, 1, 0, 1};
        n = hostile(buf, t, 2, 1, 1, 0);
        expect_rejected(buf, n);
    }
    /* Declared length u32 max with a tiny payload. */
    {
        const uint8_t t[] = {0, 1, 1, 1};
        n = hostile(buf, t, 2, 0xFFFFFFFFul, 4, 0);
        expect_rejected(buf, n);
    }
    /* Truncated table. */
    {
        const uint8_t t[] = {0, 1, 0, 1, 0, 1, 0, 1, 0, 1};
        n = hostile(buf, t, 5, 1, 0, 0);
        n = 10; /* claims 5 entries, carries only one */
        expect_rejected(buf, n);
    }
    /* 100KB of 0xff against a one-code table: fails at the first bit. */
    {
        const uint8_t t[] = {0, 1};
        n = hostile(buf, t, 1, 8, 100000, 0xff);
        expect_rejected(buf, n);
    }
}

/* Largest code length recorded in a compressed stream's lengths table
 * (entries are [symbol, length] pairs after the 8-byte header). */
static unsigned max_table_length(const uint8_t *packed, size_t packed_len) {
    size_t n = ((size_t)packed[4] << 24) | ((size_t)packed[5] << 16) |
               ((size_t)packed[6] << 8) | packed[7];
    size_t i;
    unsigned max_len = 0;
    ISO_CHECK(packed_len >= 8 + 2 * n);
    for (i = 0; i < n; i++) {
        unsigned l = packed[8 + 2 * i + 1];
        if (l > max_len) {
            max_len = l;
        }
    }
    return max_len;
}

/* SEC-003c: Fibonacci frequencies (1,1,2,3,5,8,...) are the worst case for
 * Huffman depth: a tree of depth d needs total weight >= Fibonacci(d + 2). With
 * 34 symbols the total is ~14.9 MB and the deepest code is 33 bits, which
 * overflowed the old 32-bit code values. It must round-trip, and every emitted
 * length must stay within the decoder's 1..56 limit. */
static void fibonacci_depth_test(void) {
    enum { SYMS = 34 };
    size_t count[SYMS];
    size_t total = 0, pos = 0, s;
    uint8_t *input, *packed = NULL, *restored = NULL;
    size_t packed_len = 0, restored_len = 0;
    unsigned max_len;

    count[0] = 1;
    count[1] = 1;
    for (s = 2; s < SYMS; s++) {
        count[s] = count[s - 1] + count[s - 2];
    }
    for (s = 0; s < SYMS; s++) {
        total += count[s];
    }
    input = (uint8_t *)malloc(total);
    ISO_CHECK(input != NULL);
    if (input == NULL) {
        return;
    }
    /* Interleave symbols so the input is not trivially run-length ordered. */
    while (pos < total) {
        for (s = 0; s < SYMS; s++) {
            if (count[s] > 0) {
                input[pos++] = (uint8_t)s;
                count[s]--;
            }
        }
    }
    ISO_CHECK(huffman_compress(input, total, &packed, &packed_len));
    if (packed != NULL) {
        max_len = max_table_length(packed, packed_len);
        ISO_CHECK(max_len > 32); /* really exercises the >32-bit path */
        ISO_CHECK(max_len <= 56);
        ISO_CHECK(huffman_decompress(packed, packed_len, &restored, &restored_len));
        ISO_CHECK_EQ_UINT(restored_len, total);
        if (restored_len == total) {
            ISO_CHECK_MEM_EQ(restored, input, total);
        }
    }
    free(packed);
    free(restored);
    free(input);
}

/* All 256 symbols with equal counts: a flat tree, every code exactly 8 bits. */
static void all_symbols_test(void) {
    uint8_t buf[256 * 3];
    uint8_t *packed = NULL;
    size_t packed_len = 0, i;
    for (i = 0; i < sizeof buf; i++) {
        buf[i] = (uint8_t)(i & 0xff);
    }
    round_trip(buf, sizeof buf);
    ISO_CHECK(huffman_compress(buf, sizeof buf, &packed, &packed_len));
    if (packed != NULL) {
        ISO_CHECK_EQ_UINT(max_table_length(packed, packed_len), 8);
    }
    free(packed);
}

/* Hand-built stream whose table uses the longest legal code (56 bits): the
 * decoder must accept it and decode one symbol, and reject 57. */
static void long_code_decode_test(void) {
    static uint8_t buf[8 + 2 * 57 + 16];
    uint8_t pairs[2 * 57];
    uint8_t *o = NULL;
    size_t ol = 0, n, i;
    /* A degenerate "comb" tree: lengths 1,2,...,56,56 (57 symbols). */
    for (i = 0; i < 57; i++) {
        pairs[2 * i] = (uint8_t)i;
        pairs[2 * i + 1] = (uint8_t)(i < 56 ? i + 1 : 56);
    }
    n = hostile(buf, pairs, 57, 1, 8, 0x00);
    /* All-zero payload decodes the symbol with the code 0 (length 1). */
    ISO_CHECK(huffman_decompress(buf, n, &o, &ol));
    ISO_CHECK_EQ_UINT(ol, 1);
    free(o);
    pairs[2 * 56 + 1] = 57;
    pairs[2 * 55 + 1] = 56;
    n = hostile(buf, pairs, 57, 1, 8, 0x00);
    expect_rejected(buf, n);
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

    hostile_tests();
    all_symbols_test();
    long_code_decode_test();
    fibonacci_depth_test();

    /* Large input round-trips in linear time. */
    {
        size_t big_len = 400000, k;
        uint8_t *big = (uint8_t *)malloc(big_len);
        ISO_CHECK(big != NULL);
        if (big != NULL) {
            for (k = 0; k < big_len; k++) {
                big[k] = (uint8_t)(k % 7);
            }
            round_trip(big, big_len);
            free(big);
        }
    }

    return ISO_TEST_RESULT();
}
