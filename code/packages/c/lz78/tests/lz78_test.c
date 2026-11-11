/* Tests for the C lz78, using the iso_test.h harness. Token vectors and round
 * trips are taken from the Rust crate's own tests. */
#include "iso_test.h"

#include <stdlib.h> /* malloc, free */
#include <string.h> /* memcmp, strlen */

#include "lz78.h"

#define MAX_DICT 65536u

/* Round-trip `data` through compress/decompress and assert equality. */
static void check_round_trip(const unsigned char *data, size_t len) {
    unsigned char *comp = NULL, *back = NULL;
    size_t comp_len = 0, back_len = 0;
    ISO_CHECK_EQ_INT((int)lz78_compress(data, len, MAX_DICT, &comp, &comp_len),
                     (int)LZ78_OK);
    ISO_CHECK_EQ_INT((int)lz78_decompress(comp, comp_len, &back, &back_len),
                     (int)LZ78_OK);
    ISO_CHECK_EQ_UINT(back_len, len);
    if (back_len == len) {
        ISO_CHECK(len == 0 || memcmp(back, data, len) == 0);
    }
    free(comp);
    free(back);
}

/* Assert the token stream for `text` matches `expected` (count `n`). */
static void check_tokens(const char *text, const Lz78Token *expected, size_t n) {
    Lz78Token *toks = NULL;
    size_t count = 0, i;
    ISO_CHECK_EQ_INT(
        (int)lz78_encode((const unsigned char *)text, strlen(text), MAX_DICT,
                         &toks, &count),
        (int)LZ78_OK);
    ISO_CHECK_EQ_UINT(count, n);
    if (count == n) {
        for (i = 0; i < n; i++) {
            ISO_CHECK(toks[i].dict_index == expected[i].dict_index &&
                      toks[i].next_char == expected[i].next_char);
        }
    }
    free(toks);
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
    int ok = lz78_compress(d, n, 65536, &packed, &packed_len) == LZ78_OK &&
             lz78_decompress(packed, packed_len, &restored, &restored_len) == LZ78_OK;
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
        ISO_CHECK(lz78_decompress(d, sizeof d, &out, &out_len) == LZ78_OK);
        ISO_CHECK_EQ_UINT(out_len, 0);
        free(out);
        out = NULL;
    }
    /* Declared length 2^32-1 but only one token present. */
    {
        static const unsigned char d[] = {0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 1,
                                          0,    0,    'h',  0};
        ISO_CHECK(lz78_decompress(d, sizeof d, &out, &out_len) == LZ78_OK);
        ISO_CHECK_EQ_UINT(out_len, 1);
        ISO_CHECK(out_len == 1 && out[0] == 'h');
        free(out);
        out = NULL;
    }
    /* Token count far larger than the data. */
    {
        static const unsigned char d[] = {0, 0, 0, 2,   0xFF, 0xFF, 0xFF, 0xFF,
                                          0, 0, 'h', 0, 0,    0,    'i',  0, 0, 0};
        ISO_CHECK(lz78_decompress(d, sizeof d, &out, &out_len) == LZ78_OK);
        ISO_CHECK_EQ_UINT(out_len, 2);
        ISO_CHECK(out_len == 2 && memcmp(out, "hi", 2) == 0);
        free(out);
        out = NULL;
    }
    /* Dangling dictionary index must not read out of bounds. */
    {
        static const unsigned char d[] = {0, 0, 0, 10, 0, 0, 0, 2, 0,
                                          0, 'a', 0,   0x27, 0x0F, 'b', 0};
        ISO_CHECK(lz78_decompress(d, sizeof d, &out, &out_len) == LZ78_OK);
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
    /* Empty input. */
    {
        Lz78Token *toks = NULL;
        size_t count = 99;
        unsigned char *out = NULL;
        size_t out_len = 99;
        lz78_encode((const unsigned char *)"", 0, MAX_DICT, &toks, &count);
        ISO_CHECK_EQ_UINT(count, 0u);
        free(toks);
        lz78_decode(NULL, 0, 1, 0, &out, &out_len);
        ISO_CHECK_EQ_UINT(out_len, 0u);
        free(out);
    }

    /* Single byte 'A' -> {0, 65}. */
    {
        Lz78Token want[] = {{0, 65}};
        check_tokens("A", want, 1);
    }

    /* No repetition: "ABCDE" -> 5 literals. */
    {
        Lz78Token *toks = NULL;
        size_t count = 0, i;
        lz78_encode((const unsigned char *)"ABCDE", 5, MAX_DICT, &toks, &count);
        ISO_CHECK_EQ_UINT(count, 5u);
        for (i = 0; i < count; i++) {
            ISO_CHECK(toks[i].dict_index == 0);
        }
        free(toks);
    }

    /* "AABCBBABC" token vector. */
    {
        Lz78Token want[] = {{0, 65}, {1, 66}, {0, 67}, {0, 66}, {4, 65}, {4, 67}};
        check_tokens("AABCBBABC", want, 6);
    }

    /* "ABABAB" token vector (ends with a flush sentinel next_char 0). */
    {
        Lz78Token want[] = {{0, 65}, {0, 66}, {1, 66}, {3, 0}};
        check_tokens("ABABAB", want, 4);
    }

    /* "AAAAAAA" -> 4 tokens. */
    {
        Lz78Token *toks = NULL;
        size_t count = 0;
        lz78_encode((const unsigned char *)"AAAAAAA", 7, MAX_DICT, &toks,
                    &count);
        ISO_CHECK_EQ_UINT(count, 4u);
        free(toks);
    }

    /* Round trips: text and binary. */
    {
        const char *texts[] = {"",         "A",       "ABCDE",
                               "AAAAAAA",  "ABABABAB", "AABCBBABC",
                               "hello world", "ababababab"};
        size_t i;
        for (i = 0; i < 8; i++) {
            check_round_trip((const unsigned char *)texts[i], strlen(texts[i]));
        }
    }
    {
        unsigned char zeros[] = {0, 0, 0};
        unsigned char maxb[] = {255, 255, 255};
        unsigned char mixed[] = {0, 1, 2, 0, 1, 2};
        unsigned char mixed2[] = {0, 0, 0, 255, 255};
        unsigned char all256[256];
        size_t i;
        check_round_trip(zeros, 3);
        check_round_trip(maxb, 3);
        check_round_trip(mixed, 6);
        check_round_trip(mixed2, 5);
        for (i = 0; i < 256; i++) {
            all256[i] = (unsigned char)i;
        }
        check_round_trip(all256, 256);
    }

    /* max_dict_size is respected. */
    {
        Lz78Token *toks = NULL;
        size_t count = 0, i;
        lz78_encode((const unsigned char *)"ABCABCABCABCABC", 15, 10, &toks,
                    &count);
        for (i = 0; i < count; i++) {
            ISO_CHECK(toks[i].dict_index < 10);
        }
        free(toks);
    }
    {
        Lz78Token *toks = NULL;
        size_t count = 0, i;
        lz78_encode((const unsigned char *)"AAAA", 4, 1, &toks, &count);
        for (i = 0; i < count; i++) {
            ISO_CHECK(toks[i].dict_index == 0);
        }
        free(toks);
    }

    /* compress wire size == 8 + tokens*4. */
    {
        Lz78Token *toks = NULL;
        size_t count = 0;
        unsigned char *comp = NULL;
        size_t comp_len = 0;
        lz78_encode((const unsigned char *)"AB", 2, MAX_DICT, &toks, &count);
        lz78_compress((const unsigned char *)"AB", 2, MAX_DICT, &comp,
                      &comp_len);
        ISO_CHECK_EQ_UINT(comp_len, 8u + count * 4u);
        free(toks);
        free(comp);
    }

    /* Deterministic. */
    {
        const char *data = "hello world test repeated";
        unsigned char *a = NULL, *b = NULL;
        size_t al = 0, bl = 0;
        lz78_compress((const unsigned char *)data, strlen(data), MAX_DICT, &a,
                      &al);
        lz78_compress((const unsigned char *)data, strlen(data), MAX_DICT, &b,
                      &bl);
        ISO_CHECK_EQ_UINT(al, bl);
        ISO_CHECK(al == 0 || memcmp(a, b, al) == 0);
        free(a);
        free(b);
    }

    /* Repetitive data compresses below its original size. */
    {
        unsigned char *data = malloc(3000);
        unsigned char *comp = NULL;
        size_t comp_len = 0, i;
        for (i = 0; i < 3000; i++) {
            data[i] = (unsigned char)"ABC"[i % 3];
        }
        lz78_compress(data, 3000, MAX_DICT, &comp, &comp_len);
        ISO_CHECK(comp_len < 3000);
        free(data);
        free(comp);
    }
    {
        unsigned char *data = malloc(10000);
        unsigned char *comp = NULL, *back = NULL;
        size_t comp_len = 0, back_len = 0;
        memset(data, 65, 10000);
        lz78_compress(data, 10000, MAX_DICT, &comp, &comp_len);
        ISO_CHECK(comp_len < 10000);
        lz78_decompress(comp, comp_len, &back, &back_len);
        ISO_CHECK_EQ_UINT(back_len, 10000u);
        ISO_CHECK(memcmp(back, data, 10000) == 0);
        free(data);
        free(comp);
        free(back);
    }

    /* TrieCursor doctest behaviour. */
    {
        Lz78TrieCursor *c = lz78_cursor_new();
        ISO_CHECK(c != NULL);
        ISO_CHECK(!lz78_cursor_step(c, 'A')); /* no child yet */
        lz78_cursor_insert(c, 'A', 1);
        lz78_cursor_reset(c);
        ISO_CHECK(lz78_cursor_step(c, 'A'));
        ISO_CHECK(lz78_cursor_dict_id(c) == 1);
        ISO_CHECK(!lz78_cursor_at_root(c));
        lz78_cursor_free(c);
    }

    /* Malformed decompress input must not crash (bounds/cycle guards). */
    {
        unsigned char bad[] = {0,   0,   0,   4,   /* orig_len 4 */
                               0,   0,   0,   2,   /* token_count 2 */
                               0xFF, 0xFF, 65, 0,  /* dict_index 65535 (OOB) */
                               0,   1,   66, 0};   /* dict_index 1 */
        unsigned char *out = NULL;
        size_t out_len = 0;
        ISO_CHECK_EQ_INT((int)lz78_decompress(bad, sizeof bad, &out, &out_len),
                         (int)LZ78_OK);
        free(out); /* whatever it produced, it must not have crashed */
    }

    hostile_tests();

    return ISO_TEST_RESULT();
}
