/*
 * Tests for the C canonical-cbor library, using the header-only iso_test.h
 * harness (pure ISO). The byte vectors and error expectations mirror the Rust
 * crate's own unit tests one-for-one.
 */
#include "iso_test.h"

#include <stdlib.h> /* free */
#include <string.h> /* memcmp */

#include "canonical_cbor.h"

/* Encode `v` (consuming it) and check the bytes equal exp[0..n). */
static int enc_eq(CborValue *v, const uint8_t *exp, size_t n) {
    uint8_t *out = NULL;
    size_t len = 0;
    int ok = 0;
    if (v != NULL && cbor_encode(v, &out, &len) == CBOR_OK)
        ok = (len == n) && (n == 0 || memcmp(out, exp, n) == 0);
    free(out);
    cbor_free(v);
    return ok;
}

/* Decode and return the status; frees any produced value. */
static CborStatus dec_err(const uint8_t *b, size_t n) {
    CborValue *out = NULL;
    CborStatus st = cbor_decode(b, n, &out);
    cbor_free(out);
    return st;
}

/* Convenience map builder for two text->unsigned entries. */
static CborValue *map2(const char *k1, uint64_t v1, const char *k2,
                       uint64_t v2) {
    CborValue *m = cbor_map();
    cbor_map_push(m, cbor_text(k1, strlen(k1)), cbor_unsigned(v1));
    cbor_map_push(m, cbor_text(k2, strlen(k2)), cbor_unsigned(v2));
    return m;
}

int main(void) {
    /* ── smallest-form unsigned ───────────────────────────────────────── */
    for (unsigned n = 0; n <= 23; n++) {
        uint8_t exp[1] = {(uint8_t)n};
        ISO_CHECK(enc_eq(cbor_unsigned(n), exp, 1));
    }
    {
        uint8_t e24[2] = {0x18, 24};
        ISO_CHECK(enc_eq(cbor_unsigned(24), e24, 2));
        uint8_t e255[2] = {0x18, 255};
        ISO_CHECK(enc_eq(cbor_unsigned(255), e255, 2));
        uint8_t e256[3] = {0x19, 0x01, 0x00};
        ISO_CHECK(enc_eq(cbor_unsigned(256), e256, 3));
        uint8_t e65536[5] = {0x1A, 0x00, 0x01, 0x00, 0x00};
        ISO_CHECK(enc_eq(cbor_unsigned(65536), e65536, 5));
        uint8_t emax[9] = {0x1B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF};
        ISO_CHECK(enc_eq(cbor_unsigned(0xFFFFFFFFFFFFFFFFull), emax, 9));
    }

    /* ── decoder rejects non-minimal integer forms ────────────────────── */
    {
        uint8_t b1[2] = {0x18, 0x05};
        ISO_CHECK(dec_err(b1, 2) == CBOR_ERR_NON_MINIMAL_INTEGER);
        uint8_t b2[3] = {0x19, 0x00, 0xFF};
        ISO_CHECK(dec_err(b2, 3) == CBOR_ERR_NON_MINIMAL_INTEGER);
        uint8_t b3[5] = {0x1A, 0x00, 0x00, 0xFF, 0xFF};
        ISO_CHECK(dec_err(b3, 5) == CBOR_ERR_NON_MINIMAL_INTEGER);
        uint8_t b4[9] = {0x1B, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF};
        ISO_CHECK(dec_err(b4, 9) == CBOR_ERR_NON_MINIMAL_INTEGER);
    }

    /* ── negatives ────────────────────────────────────────────────────── */
    {
        uint8_t e0[1] = {0x20};
        ISO_CHECK(enc_eq(cbor_negative(0), e0, 1)); /* -1 */
        uint8_t e23[1] = {0x37};
        ISO_CHECK(enc_eq(cbor_negative(23), e23, 1)); /* -24 */
        uint8_t e24[2] = {0x38, 24};
        ISO_CHECK(enc_eq(cbor_negative(24), e24, 2)); /* -25 */
    }

    /* ── bytes / text ─────────────────────────────────────────────────── */
    {
        uint8_t empty[1] = {0x40};
        ISO_CHECK(enc_eq(cbor_bytes(NULL, 0), empty, 1));
        uint8_t four[4] = {1, 2, 3, 4};
        uint8_t eb[5] = {0x44, 1, 2, 3, 4};
        ISO_CHECK(enc_eq(cbor_bytes(four, 4), eb, 5));
        uint8_t et[4] = {0x63, 'a', 'b', 'c'};
        ISO_CHECK(enc_eq(cbor_text("abc", 3), et, 4));
        uint8_t bad_utf8[2] = {0x61, 0xFF};
        ISO_CHECK(dec_err(bad_utf8, 2) == CBOR_ERR_INVALID_UTF8);
    }

    /* ── arrays ───────────────────────────────────────────────────────── */
    {
        uint8_t empty[1] = {0x80};
        ISO_CHECK(enc_eq(cbor_array(), empty, 1));

        CborValue *a = cbor_array();
        cbor_array_push(a, cbor_unsigned(1));
        cbor_array_push(a, cbor_unsigned(2));
        cbor_array_push(a, cbor_unsigned(3));
        uint8_t e[4] = {0x83, 0x01, 0x02, 0x03};
        ISO_CHECK(enc_eq(a, e, 4));

        CborValue *r = cbor_array();
        cbor_array_push(r, cbor_unsigned(3));
        cbor_array_push(r, cbor_unsigned(2));
        cbor_array_push(r, cbor_unsigned(1));
        uint8_t e2[4] = {0x83, 0x03, 0x02, 0x01};
        ISO_CHECK(enc_eq(r, e2, 4));
    }

    /* ── maps: canonical length-first ordering ────────────────────────── */
    {
        /* "a" (2 bytes) before "bb" (3 bytes) regardless of input order */
        uint8_t *o1 = NULL, *o2 = NULL;
        size_t l1 = 0, l2 = 0;
        CborValue *m1 = map2("a", 1, "bb", 2);
        CborValue *m2 = map2("bb", 2, "a", 1);
        ISO_CHECK(cbor_encode(m1, &o1, &l1) == CBOR_OK);
        ISO_CHECK(cbor_encode(m2, &o2, &l2) == CBOR_OK);
        ISO_CHECK(l1 == l2 && memcmp(o1, o2, l1) == 0);
        ISO_CHECK(o1[0] == 0xA2 && o1[1] == 0x61 && o1[2] == 'a');
        free(o1);
        free(o2);
        cbor_free(m1);
        cbor_free(m2);

        /* tie broken lex: "a" before "b" */
        CborValue *m = map2("b", 2, "a", 1);
        uint8_t e[7] = {0xA2, 0x61, 'a', 0x01, 0x61, 'b', 0x02};
        ISO_CHECK(enc_eq(m, e, 7));

        /* decoder accepts canonical, rejects non-canonical / duplicate */
        uint8_t canon[7] = {0xA2, 0x61, 'a', 0x01, 0x61, 'b', 0x02};
        CborValue *dv = NULL;
        ISO_CHECK(cbor_decode(canon, 7, &dv) == CBOR_OK);
        ISO_CHECK(dv->type == CBOR_MAP && dv->as.map.len == 2);
        {
            CborValue *ka = cbor_text("a", 1);
            CborValue *kb = cbor_text("b", 1);
            ISO_CHECK(cbor_equal(dv->as.map.entries[0].key, ka));
            ISO_CHECK(cbor_equal(dv->as.map.entries[1].key, kb));
            cbor_free(ka);
            cbor_free(kb);
        }
        cbor_free(dv);

        uint8_t noncanon[7] = {0xA2, 0x61, 'b', 0x02, 0x61, 'a', 0x01};
        ISO_CHECK(dec_err(noncanon, 7) == CBOR_ERR_NON_CANONICAL_MAP_ORDER);
        uint8_t dup[7] = {0xA2, 0x61, 'a', 0x01, 0x61, 'a', 0x02};
        ISO_CHECK(dec_err(dup, 7) == CBOR_ERR_NON_CANONICAL_MAP_ORDER);
    }

    /* ── round-trip: encode -> decode -> re-encode is byte-identical ──── */
    {
        CborValue *meta = cbor_map();
        cbor_map_push(meta, cbor_text("v", 1), cbor_unsigned(1));
        cbor_map_push(meta, cbor_text("draft", 5), cbor_bool(1));
        CborValue *tags = cbor_array();
        cbor_array_push(tags, cbor_text("urgent", 6));
        cbor_array_push(tags, cbor_text("draft", 5));
        uint8_t blob[4] = {0xDE, 0xAD, 0xBE, 0xEF};

        CborValue *v = cbor_map();
        cbor_map_push(v, cbor_text("title", 5), cbor_text("hello world", 11));
        cbor_map_push(v, cbor_text("count", 5), cbor_unsigned(42));
        cbor_map_push(v, cbor_text("tags", 4), tags);
        cbor_map_push(v, cbor_text("meta", 4), meta);
        cbor_map_push(v, cbor_text("note", 4), cbor_null());
        cbor_map_push(v, cbor_text("blob", 4), cbor_bytes(blob, 4));

        uint8_t *bytes = NULL;
        size_t blen = 0;
        ISO_CHECK(cbor_encode(v, &bytes, &blen) == CBOR_OK);
        CborValue *back = NULL;
        ISO_CHECK(cbor_decode(bytes, blen, &back) == CBOR_OK);
        /* decode returns keys in canonical order (not input order), so the tree
         * differs from `v`; faithfulness is checked by re-encoding to the same
         * bytes (exactly as the Rust test does). */
        uint8_t *re = NULL;
        size_t rlen = 0;
        ISO_CHECK(cbor_encode(back, &re, &rlen) == CBOR_OK);
        ISO_CHECK(rlen == blen && memcmp(re, bytes, blen) == 0);
        free(bytes);
        free(re);
        cbor_free(v);
        cbor_free(back);
    }

    /* ── tags ─────────────────────────────────────────────────────────── */
    {
        CborValue *v = cbor_tag(0, cbor_text("2026-05-04", 10));
        uint8_t *bytes = NULL;
        size_t blen = 0;
        ISO_CHECK(cbor_encode(v, &bytes, &blen) == CBOR_OK);
        ISO_CHECK(bytes[0] == 0xC0);
        CborValue *back = NULL;
        ISO_CHECK(cbor_decode(bytes, blen, &back) == CBOR_OK);
        ISO_CHECK(cbor_equal(back, v));
        free(bytes);
        cbor_free(v);
        cbor_free(back);

        CborValue *big = cbor_tag(1234567, cbor_unsigned(0));
        ISO_CHECK(cbor_encode(big, &bytes, &blen) == CBOR_OK);
        back = NULL;
        ISO_CHECK(cbor_decode(bytes, blen, &back) == CBOR_OK);
        ISO_CHECK(cbor_equal(back, big));
        free(bytes);
        cbor_free(big);
        cbor_free(back);
    }

    /* ── rejects indefinite / reserved / undefined / floats ───────────── */
    {
        uint8_t ia[3] = {0x9F, 0x01, 0xFF};
        ISO_CHECK(dec_err(ia, 3) == CBOR_ERR_INDEFINITE);
        uint8_t im[5] = {0xBF, 0x61, 'a', 0x01, 0xFF};
        ISO_CHECK(dec_err(im, 5) == CBOR_ERR_INDEFINITE);
        uint8_t rsv[1] = {0x1C};
        ISO_CHECK(dec_err(rsv, 1) == CBOR_ERR_RESERVED);
        uint8_t undef[1] = {0xF7};
        ISO_CHECK(dec_err(undef, 1) == CBOR_ERR_UNSUPPORTED_SIMPLE);
        uint8_t f16[3] = {0xF9, 0x00, 0x00};
        ISO_CHECK(dec_err(f16, 3) == CBOR_ERR_FLOAT_NOT_SUPPORTED);
        uint8_t f32[5] = {0xFA, 0, 0, 0, 0};
        ISO_CHECK(dec_err(f32, 5) == CBOR_ERR_FLOAT_NOT_SUPPORTED);
        uint8_t f64[9] = {0xFB, 0, 0, 0, 0, 0, 0, 0, 0};
        ISO_CHECK(dec_err(f64, 9) == CBOR_ERR_FLOAT_NOT_SUPPORTED);
    }

    /* ── trailing / EOF / truncation ──────────────────────────────────── */
    {
        uint8_t trail[2] = {0x01, 0x00};
        ISO_CHECK(dec_err(trail, 2) == CBOR_ERR_TRAILING_BYTES);
        ISO_CHECK(dec_err(NULL, 0) == CBOR_ERR_UNEXPECTED_EOF);
        uint8_t eofarg[1] = {0x18};
        ISO_CHECK(dec_err(eofarg, 1) == CBOR_ERR_UNEXPECTED_EOF);
        uint8_t trunc[3] = {0x44, 0xAA, 0xBB}; /* claims 4, has 2 */
        ISO_CHECK(dec_err(trunc, 3) == CBOR_ERR_LENGTH_TOO_LARGE);
    }

    /* ── stress: large array / map round-trips ────────────────────────── */
    {
        CborValue *arr = cbor_array();
        for (uint64_t i = 0; i < 1000; i++)
            cbor_array_push(arr, cbor_unsigned(i));
        uint8_t *bytes = NULL;
        size_t blen = 0;
        ISO_CHECK(cbor_encode(arr, &bytes, &blen) == CBOR_OK);
        CborValue *back = NULL;
        ISO_CHECK(cbor_decode(bytes, blen, &back) == CBOR_OK);
        ISO_CHECK(cbor_equal(back, arr));
        free(bytes);
        cbor_free(arr);
        cbor_free(back);

        /* map of 100 keys, encoded deterministically regardless of order */
        CborValue *m = cbor_map();
        CborValue *rev = cbor_map();
        for (uint64_t i = 0; i < 100; i++)
            cbor_map_push(m, cbor_unsigned(i), cbor_unsigned(i * 7));
        for (uint64_t i = 100; i > 0; i--)
            cbor_map_push(rev, cbor_unsigned(i - 1), cbor_unsigned((i - 1) * 7));
        uint8_t *ba = NULL, *bb = NULL;
        size_t la = 0, lb = 0;
        ISO_CHECK(cbor_encode(m, &ba, &la) == CBOR_OK);
        ISO_CHECK(cbor_encode(rev, &bb, &lb) == CBOR_OK);
        ISO_CHECK(la == lb && memcmp(ba, bb, la) == 0);
        CborValue *dm = NULL;
        ISO_CHECK(cbor_decode(ba, la, &dm) == CBOR_OK);
        ISO_CHECK(dm->type == CBOR_MAP && dm->as.map.len == 100);
        for (uint64_t i = 0; i < 100; i++) /* canonical order is 0,1,...,99 */
            ISO_CHECK(dm->as.map.entries[i].key->type == CBOR_UNSIGNED &&
                      dm->as.map.entries[i].key->as.u == i);
        free(ba);
        free(bb);
        cbor_free(m);
        cbor_free(rev);
        cbor_free(dm);
    }

    /* ── simple values round-trip ─────────────────────────────────────── */
    {
        CborValue *vals[3];
        vals[0] = cbor_bool(0);
        vals[1] = cbor_bool(1);
        vals[2] = cbor_null();
        for (int i = 0; i < 3; i++) {
            uint8_t *bytes = NULL;
            size_t blen = 0;
            ISO_CHECK(cbor_encode(vals[i], &bytes, &blen) == CBOR_OK);
            CborValue *back = NULL;
            ISO_CHECK(cbor_decode(bytes, blen, &back) == CBOR_OK);
            ISO_CHECK(cbor_equal(back, vals[i]));
            free(bytes);
            cbor_free(back);
            cbor_free(vals[i]);
        }
    }

    /* ── DoS defences: depth cap and oversized lengths ────────────────── */
    {
        /* MAX_DECODE_DEPTH+10 nested singleton arrays -> TooDeep */
        {
            enum { N = CBOR_MAX_DECODE_DEPTH + 10 };
            uint8_t buf[N + 1];
            for (int i = 0; i < N; i++) buf[i] = 0x81;
            buf[N] = 0x00;
            ISO_CHECK(dec_err(buf, N + 1) == CBOR_ERR_TOO_DEEP);
        }
        /* nested tags -> TooDeep */
        {
            enum { N = CBOR_MAX_DECODE_DEPTH + 10 };
            uint8_t buf[N + 1];
            for (int i = 0; i < N; i++) buf[i] = 0xC6;
            buf[N] = 0x00;
            ISO_CHECK(dec_err(buf, N + 1) == CBOR_ERR_TOO_DEEP);
        }
        /* exactly MAX_DECODE_DEPTH nesting -> accepted */
        {
            enum { N = CBOR_MAX_DECODE_DEPTH };
            uint8_t buf[N + 1];
            for (int i = 0; i < N; i++) buf[i] = 0x81;
            buf[N] = 0x00;
            CborValue *v = NULL;
            ISO_CHECK(cbor_decode(buf, N + 1, &v) == CBOR_OK);
            cbor_free(v);
        }
        /* oversized declared lengths (2^40) -> LengthTooLarge, never allocates */
        uint8_t oa[10] = {0x9B, 0, 0, 0x01, 0, 0, 0, 0, 0, 0};
        ISO_CHECK(dec_err(oa, 10) == CBOR_ERR_LENGTH_TOO_LARGE);
        uint8_t om[11] = {0xBB, 0, 0, 0x01, 0, 0, 0, 0, 0, 0, 0};
        ISO_CHECK(dec_err(om, 11) == CBOR_ERR_LENGTH_TOO_LARGE);
        uint8_t ob[9] = {0x5B, 0, 0, 0x01, 0, 0, 0, 0, 0};
        ISO_CHECK(dec_err(ob, 9) == CBOR_ERR_LENGTH_TOO_LARGE);
        /* array claiming u64::MAX elements */
        uint8_t mx[9] = {0x9B, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF};
        ISO_CHECK(dec_err(mx, 9) == CBOR_ERR_LENGTH_TOO_LARGE);
    }

    return ISO_TEST_RESULT();
}
