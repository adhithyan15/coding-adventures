/* Tests for the C lzss, using the iso_test.h harness. Vectors and round trips
 * are taken from the Rust crate's own tests. */
#include "iso_test.h"

#include <stdlib.h> /* malloc, free */
#include <string.h> /* memcmp, memset, strlen */

#include "lzss.h"

/* Encode with default params. */
static LzssToken *enc(const char *text, size_t *count) {
    LzssToken *toks = NULL;
    lzss_encode((const unsigned char *)text, strlen(text),
                LZSS_DEFAULT_WINDOW_SIZE, LZSS_DEFAULT_MAX_MATCH,
                LZSS_DEFAULT_MIN_MATCH, &toks, count);
    return toks;
}

static void check_round_trip(const unsigned char *data, size_t len) {
    unsigned char *comp = NULL, *back = NULL;
    size_t comp_len = 0, back_len = 0;
    ISO_CHECK_EQ_INT((int)lzss_compress(data, len, &comp, &comp_len),
                     (int)LZSS_OK);
    ISO_CHECK_EQ_INT((int)lzss_decompress(comp, comp_len, &back, &back_len),
                     (int)LZSS_OK);
    ISO_CHECK_EQ_UINT(back_len, len);
    if (back_len == len) {
        ISO_CHECK(len == 0 || memcmp(back, data, len) == 0);
    }
    free(comp);
    free(back);
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

static void hostile_round_trip(const unsigned char *d, size_t n) {
    unsigned char *packed = NULL;
    unsigned char *restored = NULL;
    size_t packed_len = 0, restored_len = 0;
    int ok = lzss_compress(d, n, &packed, &packed_len) == LZSS_OK &&
             lzss_decompress(packed, packed_len, &restored, &restored_len) == LZSS_OK;
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
    unsigned char *out = NULL;
    size_t out_len = 0;
    unsigned a, b, i;

    {
        unsigned char d[8];
        memset(d, 0xFF, sizeof d);
        ISO_CHECK(lzss_decompress(d, sizeof d, &out, &out_len) == LZSS_OK);
        ISO_CHECK_EQ_UINT(out_len, 0);
        free(out);
        out = NULL;
    }
    /* Declared length 2^32-1, one literal present. */
    {
        static const unsigned char d[] = {0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 1,
                                          0x00, 'x'};
        ISO_CHECK(lzss_decompress(d, sizeof d, &out, &out_len) == LZSS_OK);
        ISO_CHECK_EQ_UINT(out_len, 1);
        ISO_CHECK(out_len == 1 && out[0] == 'x');
        free(out);
        out = NULL;
    }
    /* Block count far larger than the data. */
    {
        static const unsigned char d[] = {0, 0, 0, 2, 0xFF, 0xFF, 0xFF, 0xFF,
                                          0x00, 'h', 'i'};
        ISO_CHECK(lzss_decompress(d, sizeof d, &out, &out_len) == LZSS_OK);
        ISO_CHECK_EQ_UINT(out_len, 2);
        ISO_CHECK(out_len == 2 && memcmp(out, "hi", 2) == 0);
        free(out);
        out = NULL;
    }
    /* Invalid match offsets are skipped. */
    {
        static const unsigned char d1[] = {0, 0, 0, 100, 0, 0, 0, 1, 0x01, 0, 0, 5};
        static const unsigned char d2[] = {0, 0, 0, 100, 0, 0, 0, 1,
                                           0x01, 0xFF, 0xFF, 255};
        ISO_CHECK(lzss_decompress(d1, sizeof d1, &out, &out_len) == LZSS_OK);
        ISO_CHECK_EQ_UINT(out_len, 0);
        free(out);
        out = NULL;
        ISO_CHECK(lzss_decompress(d2, sizeof d2, &out, &out_len) == LZSS_OK);
        ISO_CHECK_EQ_UINT(out_len, 0);
        free(out);
        out = NULL;
    }

    hostile_round_trip((const unsigned char *)"", 0);
    for (a = 0; a < 256; a++) {
        unsigned char one[1];
        one[0] = (unsigned char)a;
        hostile_round_trip(one, 1);
        for (b = 0; b < 256; b++) {
            unsigned char two[2];
            two[0] = (unsigned char)a;
            two[1] = (unsigned char)b;
            hostile_round_trip(two, 2);
        }
    }
    for (i = 0; i < 300; i++) {
        unsigned char buf[600];
        unsigned n = 3 + hostile_rng() % 597;
        unsigned alpha = (i % 4 == 0) ? 2 : (i % 4 == 1) ? 4 : (i % 4 == 2) ? 16 : 256;
        unsigned j;
        for (j = 0; j < n; j++) {
            buf[j] = (unsigned char)(hostile_rng() % alpha);
        }
        hostile_round_trip(buf, n);
    }
}

int main(void) {
    /* Empty encode. */
    {
        size_t count = 99;
        LzssToken *toks = enc("", &count);
        ISO_CHECK_EQ_UINT(count, 0u);
        free(toks);
    }

    /* Single byte -> one literal. */
    {
        size_t count = 0;
        LzssToken *toks = enc("A", &count);
        ISO_CHECK_EQ_UINT(count, 1u);
        ISO_CHECK(!toks[0].is_match && toks[0].literal == 'A');
        free(toks);
    }

    /* No repetition -> all literals. */
    {
        size_t count = 0, i;
        LzssToken *toks = enc("ABCDE", &count);
        ISO_CHECK_EQ_UINT(count, 5u);
        for (i = 0; i < count; i++) {
            ISO_CHECK(!toks[i].is_match);
        }
        free(toks);
    }

    /* "AABCBBABC": 7 tokens, last is Match{offset:5, length:3}. */
    {
        size_t count = 0;
        LzssToken *toks = enc("AABCBBABC", &count);
        ISO_CHECK_EQ_UINT(count, 7u);
        if (count == 7) {
            ISO_CHECK(toks[6].is_match && toks[6].offset == 5 &&
                      toks[6].length == 3);
        }
        free(toks);
    }

    /* "ABABAB" -> [Lit A, Lit B, Match{2,4}]. */
    {
        size_t count = 0;
        LzssToken *toks = enc("ABABAB", &count);
        ISO_CHECK_EQ_UINT(count, 3u);
        if (count == 3) {
            ISO_CHECK(!toks[0].is_match && toks[0].literal == 'A');
            ISO_CHECK(!toks[1].is_match && toks[1].literal == 'B');
            ISO_CHECK(toks[2].is_match && toks[2].offset == 2 &&
                      toks[2].length == 4);
        }
        free(toks);
    }

    /* "AAAAAAA" -> [Lit A, Match{1,6}]. */
    {
        size_t count = 0;
        LzssToken *toks = enc("AAAAAAA", &count);
        ISO_CHECK_EQ_UINT(count, 2u);
        if (count == 2) {
            ISO_CHECK(!toks[0].is_match && toks[0].literal == 'A');
            ISO_CHECK(toks[1].is_match && toks[1].offset == 1 &&
                      toks[1].length == 6);
        }
        free(toks);
    }

    /* min_match large forces all literals. */
    {
        LzssToken *toks = NULL;
        size_t count = 0, i;
        lzss_encode((const unsigned char *)"ABABAB", 6, LZSS_DEFAULT_WINDOW_SIZE,
                    LZSS_DEFAULT_MAX_MATCH, 100, &toks, &count);
        for (i = 0; i < count; i++) {
            ISO_CHECK(!toks[i].is_match);
        }
        free(toks);
    }

    /* Matches stay within a small window. */
    {
        LzssToken *toks = NULL;
        size_t count = 0, i;
        lzss_encode((const unsigned char *)"ABCABCABCABC", 12, 8,
                    LZSS_DEFAULT_MAX_MATCH, LZSS_DEFAULT_MIN_MATCH, &toks,
                    &count);
        for (i = 0; i < count; i++) {
            if (toks[i].is_match) {
                ISO_CHECK(toks[i].offset <= 8);
            }
        }
        free(toks);
    }

    /* Match length bounded by max_match. */
    {
        unsigned char aaa[100];
        LzssToken *toks = NULL;
        size_t count = 0, i;
        memset(aaa, 'A', sizeof aaa);
        lzss_encode(aaa, sizeof aaa, LZSS_DEFAULT_WINDOW_SIZE, 5,
                    LZSS_DEFAULT_MIN_MATCH, &toks, &count);
        for (i = 0; i < count; i++) {
            if (toks[i].is_match) {
                ISO_CHECK(toks[i].length <= 5);
            }
        }
        free(toks);
    }

    /* decode vectors. */
    {
        unsigned char *out = NULL;
        size_t out_len = 0;
        LzssToken empty_lit = {0, 'A', 0, 0};
        LzssToken tokens[2];
        /* empty */
        lzss_decode(NULL, 0, 1, 0, &out, &out_len);
        ISO_CHECK_EQ_UINT(out_len, 0u);
        free(out);
        /* single literal */
        lzss_decode(&empty_lit, 1, 1, 1, &out, &out_len);
        ISO_CHECK(out_len == 1 && out[0] == 'A');
        free(out);
        /* overlapping match: Lit A + Match{1,6} -> AAAAAAA */
        tokens[0].is_match = 0;
        tokens[0].literal = 'A';
        tokens[0].offset = 0;
        tokens[0].length = 0;
        tokens[1].is_match = 1;
        tokens[1].literal = 0;
        tokens[1].offset = 1;
        tokens[1].length = 6;
        lzss_decode(tokens, 2, 1, 7, &out, &out_len);
        ISO_CHECK_EQ_UINT(out_len, 7u);
        ISO_CHECK(out_len == 7 && memcmp(out, "AAAAAAA", 7) == 0);
        free(out);
    }

    /* Round trips (text + binary). */
    {
        const char *texts[] = {"",         "A",        "ABCDE",
                               "AAAAAAA",  "ABABABAB", "AABCBBABC",
                               "hello world hello world", "the quick brown fox"};
        size_t i;
        for (i = 0; i < 8; i++) {
            check_round_trip((const unsigned char *)texts[i], strlen(texts[i]));
        }
    }
    {
        unsigned char all256[256];
        unsigned char reps[3000];
        size_t i;
        for (i = 0; i < 256; i++) {
            all256[i] = (unsigned char)i;
        }
        check_round_trip(all256, 256);
        for (i = 0; i < 3000; i++) {
            reps[i] = (unsigned char)"ABC"[i % 3];
        }
        check_round_trip(reps, 3000);
    }

    /* Repetitive data compresses below its original size. */
    {
        unsigned char *data = malloc(10000);
        unsigned char *comp = NULL;
        size_t comp_len = 0;
        memset(data, 'A', 10000);
        lzss_compress(data, 10000, &comp, &comp_len);
        ISO_CHECK(comp_len < 10000);
        free(data);
        free(comp);
    }

    /* Malformed decompress must not crash (bad offsets, capped blocks). */
    {
        unsigned char bad[] = {0,   0,   0,   8,   /* orig_len 8 */
                               0,   0,   0,   99,  /* block_count 99 (capped) */
                               0x01, 0xFF, 0xFF, 5, /* flag: token0 match; off 65535 (>output) */
                               0x00, 66};           /* flag: token0 literal 'B' */
        unsigned char *out = NULL;
        size_t out_len = 0;
        ISO_CHECK_EQ_INT((int)lzss_decompress(bad, sizeof bad, &out, &out_len),
                         (int)LZSS_OK);
        free(out);
    }

    hostile_tests();

    return ISO_TEST_RESULT();
}
