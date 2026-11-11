/* Tests for the C LZ77, using the iso_test.h harness. Covers token structure
 * (literals, a backreference) and compress/decompress round-trips. */
#include "iso_test.h"

#include <stdlib.h>
#include <string.h>

#include "lz77.h"

/* Compress then decompress `input`, asserting the result equals the original. */
static void round_trip(const char *input, size_t len) {
    uint8_t *packed = NULL;
    uint8_t *restored = NULL;
    size_t packed_len = 0, restored_len = 0;
    ISO_CHECK(lz77_compress((const uint8_t *)input, len, LZ77_DEFAULT_WINDOW,
                            LZ77_DEFAULT_MAX_MATCH, LZ77_DEFAULT_MIN_MATCH,
                            &packed, &packed_len));
    ISO_CHECK(lz77_decompress(packed, packed_len, &restored, &restored_len));
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
    int ok = lz77_compress(d, n, LZ77_DEFAULT_WINDOW, LZ77_DEFAULT_MAX_MATCH,
                           LZ77_DEFAULT_MIN_MATCH, &packed, &packed_len) &&
             lz77_decompress(packed, packed_len, &restored, &restored_len);
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

    /* Max count, no tokens: must return success with empty output. */
    {
        static const uint8_t d[] = {0xFF, 0xFF, 0xFF, 0xFF};
        ISO_CHECK(lz77_decompress(d, sizeof d, &out, &out_len));
        ISO_CHECK_EQ_UINT(out_len, 0);
        free(out);
        out = NULL;
    }
    /* Count far larger than the data: only the present tokens decode. */
    {
        static const uint8_t d[] = {0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 'h',
                                    0,    0,    0,    'i',  0, 0, 0};
        ISO_CHECK(lz77_decompress(d, sizeof d, &out, &out_len));
        ISO_CHECK_EQ_UINT(out_len, 2);
        ISO_CHECK(out_len == 2 && memcmp(out, "hi", 2) == 0);
        free(out);
        out = NULL;
    }
    /* Invalid back-references are rejected (previously: wrapped index, OOB read
     * / NULL dereference). */
    {
        static const uint8_t far_off[] = {0, 0, 0, 1, 0, 5, 3, 'x'};
        static const uint8_t zero_off[] = {0, 0, 0, 1, 0, 0, 3, 'x'};
        ISO_CHECK(!lz77_decompress(far_off, sizeof far_off, &out, &out_len));
        ISO_CHECK(!lz77_decompress(zero_off, sizeof zero_off, &out, &out_len));
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
    lz77_token *tokens = NULL;
    size_t count = 0;

    /* Empty input → no tokens. */
    ISO_CHECK(lz77_encode((const uint8_t *)"", 0, LZ77_DEFAULT_WINDOW,
                          LZ77_DEFAULT_MAX_MATCH, LZ77_DEFAULT_MIN_MATCH, &tokens,
                          &count));
    ISO_CHECK_EQ_UINT(count, 0);
    free(tokens);
    tokens = NULL;

    /* "ABCDE" has no repeats → five literal tokens. */
    ISO_CHECK(lz77_encode((const uint8_t *)"ABCDE", 5, LZ77_DEFAULT_WINDOW,
                          LZ77_DEFAULT_MAX_MATCH, LZ77_DEFAULT_MIN_MATCH, &tokens,
                          &count));
    ISO_CHECK_EQ_UINT(count, 5);
    {
        size_t i;
        for (i = 0; i < count; i++) {
            ISO_CHECK_EQ_UINT(tokens[i].offset, 0);
            ISO_CHECK_EQ_UINT(tokens[i].length, 0);
        }
    }
    ISO_CHECK_EQ_INT(tokens[0].next_char, 'A');
    ISO_CHECK_EQ_INT(tokens[4].next_char, 'E');
    free(tokens);
    tokens = NULL;

    /* "AAAAAAA" (7 bytes) → a literal A, then a backreference (offset 1, length
     * 5, next_char A). The backreference covers bytes 1..6 (length 5 + the
     * next_char), so the cursor reaches the end — just two tokens. */
    ISO_CHECK(lz77_encode((const uint8_t *)"AAAAAAA", 7, LZ77_DEFAULT_WINDOW,
                          LZ77_DEFAULT_MAX_MATCH, LZ77_DEFAULT_MIN_MATCH, &tokens,
                          &count));
    ISO_CHECK_EQ_UINT(count, 2);
    ISO_CHECK_EQ_UINT(tokens[0].length, 0); /* literal */
    ISO_CHECK_EQ_UINT(tokens[1].offset, 1);
    ISO_CHECK_EQ_UINT(tokens[1].length, 5);
    ISO_CHECK_EQ_INT(tokens[1].next_char, 'A');
    free(tokens);
    tokens = NULL;

    /* Round-trips of assorted inputs. */
    round_trip("", 0);
    round_trip("A", 1);
    round_trip("ABCDE", 5);
    round_trip("AAAAAAA", 7);
    round_trip("abcabcabcabcabc", 15);
    round_trip("the quick brown fox jumps over the lazy dog. the quick brown fox.",
               64);

    hostile_tests();

    return ISO_TEST_RESULT();
}
