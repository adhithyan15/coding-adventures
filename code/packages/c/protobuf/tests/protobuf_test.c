/* Tests for protobuf, using the header-only iso_test.h harness (pure ISO).
 * Vectors mirror the Rust crate's own unit tests, including the canonical
 * examples from the protobuf encoding docs. */
#include "iso_test.h"

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "protobuf.h"

/* Read the next field, asserting success and presence; returns it by value. */
static PbField next_ok(PbReader *r) {
    PbField f;
    int has = 0;
    PbError err = pb_reader_next_field(r, &f, &has);
    ISO_CHECK_MSG(err == PB_OK, "next_field returned an error");
    ISO_CHECK_MSG(has == 1, "expected a field, got end of message");
    return f;
}

int main(void) {
    /* ── varint round-trip at boundaries ───────────────────────────────────*/
    {
        static const uint64_t vals[] = {0u,
                                        1u,
                                        127u,
                                        128u,
                                        300u,
                                        16383u,
                                        16384u,
                                        (uint64_t)0xFFFFFFFFu,
                                        (uint64_t)0xFFFFFFFFFFFFFFFFull};
        size_t i;
        for (i = 0; i < sizeof vals / sizeof vals[0]; i++) {
            PbWriter w;
            PbReader r;
            uint64_t got = 0;
            pb_writer_init(&w);
            pb_write_varint(&w, vals[i]);
            ISO_CHECK(!w.oom);
            pb_reader_init(&r, pb_writer_bytes(&w), pb_writer_len(&w));
            /* next_field needs a tag; here we test the raw varint path via a
             * field wrapper instead: re-encode as field 1 and read it back. */
            pb_reader_init(&r, pb_writer_bytes(&w), pb_writer_len(&w));
            {
                /* Decode the raw varint through a tiny helper reader. */
                PbReader rr;
                uint64_t v2 = 0;
                PbWriter fw;
                PbField f;
                int has = 0;
                pb_writer_init(&fw);
                pb_varint(&fw, 1, vals[i]);
                pb_reader_init(&rr, pb_writer_bytes(&fw), pb_writer_len(&fw));
                (void)pb_reader_next_field(&rr, &f, &has);
                ISO_CHECK(has == 1);
                ISO_CHECK(pb_value_as_varint(&f.value, &v2));
                ISO_CHECK(v2 == vals[i]);
                ISO_CHECK(pb_reader_is_empty(&rr));
                pb_writer_free(&fw);
            }
            (void)got;
            pb_writer_free(&w);
        }
    }

    /* ── varint 300 matches the spec bytes ─────────────────────────────────*/
    {
        PbWriter w;
        static const uint8_t expect[] = {0xac, 0x02};
        pb_writer_init(&w);
        pb_write_varint(&w, 300);
        ISO_CHECK(pb_writer_len(&w) == 2);
        ISO_CHECK_MEM_EQ(pb_writer_bytes(&w), expect, 2);
        pb_writer_free(&w);
    }

    /* ── field 1, varint 150 → tag 0x08, then 0x96 0x01 ────────────────────*/
    {
        PbWriter w;
        static const uint8_t expect[] = {0x08, 0x96, 0x01};
        pb_writer_init(&w);
        pb_varint(&w, 1, 150);
        ISO_CHECK(pb_writer_len(&w) == 3);
        ISO_CHECK_MEM_EQ(pb_writer_bytes(&w), expect, 3);
        pb_writer_free(&w);
    }

    /* ── all wire types round-trip ─────────────────────────────────────────*/
    {
        PbWriter w;
        PbReader r;
        static const uint8_t payload[] = {0xde, 0xad, 0xbe, 0xef};
        PbField f;
        const uint8_t *b;
        size_t bl;
        pb_writer_init(&w);
        pb_varint(&w, 1, 150);
        pb_string(&w, 2, "testing");
        pb_bytes(&w, 3, payload, 4);
        pb_fixed32(&w, 4, 0x12345678u);
        pb_fixed64(&w, 5, 0x0102030405060708ull);
        ISO_CHECK(!w.oom);
        pb_reader_init(&r, pb_writer_bytes(&w), pb_writer_len(&w));

        f = next_ok(&r);
        ISO_CHECK(f.number == 1 && f.value.kind == PB_WIRE_VARINT &&
                  f.value.varint == 150);
        f = next_ok(&r);
        ISO_CHECK(f.number == 2 && pb_value_as_bytes(&f.value, &b, &bl) &&
                  bl == 7 && memcmp(b, "testing", 7) == 0);
        f = next_ok(&r);
        ISO_CHECK(f.number == 3 && pb_value_as_bytes(&f.value, &b, &bl) &&
                  bl == 4 && memcmp(b, payload, 4) == 0);
        f = next_ok(&r);
        ISO_CHECK(f.number == 4 && f.value.kind == PB_WIRE_FIXED32 &&
                  f.value.fixed32 == 0x12345678u);
        f = next_ok(&r);
        ISO_CHECK(f.number == 5 && f.value.kind == PB_WIRE_FIXED64 &&
                  f.value.fixed64 == 0x0102030405060708ull);
        {
            int has = 1;
            PbError err = pb_reader_next_field(&r, &f, &has);
            ISO_CHECK(err == PB_OK && has == 0); /* clean end */
        }
        pb_writer_free(&w);
    }

    /* ── reader skips unknown fields ───────────────────────────────────────*/
    {
        PbWriter w;
        PbReader r;
        PbField f;
        int has = 0;
        int kept[4];
        int nkept = 0;
        pb_writer_init(&w);
        pb_varint(&w, 1, 11);
        pb_varint(&w, 7, 999);
        pb_string(&w, 2, "keep");
        pb_reader_init(&r, pb_writer_bytes(&w), pb_writer_len(&w));
        while (pb_reader_next_field(&r, &f, &has) == PB_OK && has) {
            if (f.number == 1 || f.number == 2) kept[nkept++] = (int)f.number;
        }
        ISO_CHECK(nkept == 2 && kept[0] == 1 && kept[1] == 2);
        pb_writer_free(&w);
    }

    /* ── nested message round-trip ─────────────────────────────────────────*/
    {
        PbWriter inner, outer;
        PbReader r, inner_r;
        PbField f, g;
        const uint8_t *b = NULL;
        size_t bl = 0;
        pb_writer_init(&inner);
        pb_string(&inner, 1, "inner");
        pb_writer_init(&outer);
        pb_message(&outer, 1, pb_writer_bytes(&inner), pb_writer_len(&inner));
        pb_varint(&outer, 2, 5);

        pb_reader_init(&r, pb_writer_bytes(&outer), pb_writer_len(&outer));
        f = next_ok(&r);
        ISO_CHECK(f.number == 1 && pb_value_as_bytes(&f.value, &b, &bl));
        pb_reader_init(&inner_r, b, bl);
        g = next_ok(&inner_r);
        ISO_CHECK(g.value.kind == PB_WIRE_LENGTH_DELIMITED &&
                  g.value.bytes_len == 5 &&
                  memcmp(g.value.bytes, "inner", 5) == 0);
        pb_writer_free(&inner);
        pb_writer_free(&outer);
    }

    /* ── error cases ───────────────────────────────────────────────────────*/
    {
        static const uint8_t truncated[] = {0x80}; /* continuation, no next */
        PbReader r;
        PbField f;
        int has = 1;
        pb_reader_init(&r, truncated, 1);
        ISO_CHECK(pb_reader_next_field(&r, &f, &has) ==
                      PB_ERR_TRUNCATED_VARINT &&
                  has == 0);
    }
    {
        /* Field 1, length-delimited, claims 100 bytes but supplies none. */
        static const uint8_t overlong[] = {0x0a, 0x64};
        PbReader r;
        PbField f;
        int has = 1;
        pb_reader_init(&r, overlong, 2);
        ISO_CHECK(pb_reader_next_field(&r, &f, &has) == PB_ERR_UNEXPECTED_EOF &&
                  has == 0);
    }
    {
        /* A tag with field number 0 is illegal. */
        static const uint8_t zero_field[] = {0x00}; /* tag: field 0, varint */
        PbReader r;
        PbField f;
        int has = 1;
        pb_reader_init(&r, zero_field, 1);
        ISO_CHECK(pb_reader_next_field(&r, &f, &has) ==
                      PB_ERR_ZERO_FIELD_NUMBER &&
                  has == 0);
    }
    {
        /* Wire type 3 (start-group, deprecated) is rejected. */
        static const uint8_t group[] = {0x0b}; /* field 1, wire type 3 */
        PbReader r;
        PbField f;
        int has = 1;
        pb_reader_init(&r, group, 1);
        ISO_CHECK(pb_reader_next_field(&r, &f, &has) ==
                  PB_ERR_UNKNOWN_WIRE_TYPE);
    }

    /* ── pb_writer_take transfers ownership ────────────────────────────────*/
    {
        PbWriter w;
        uint8_t *taken;
        size_t n = 0;
        pb_writer_init(&w);
        pb_varint(&w, 1, 150);
        taken = pb_writer_take(&w, &n);
        ISO_CHECK(taken != NULL && n == 3);
        ISO_CHECK(pb_writer_len(&w) == 0 && pb_writer_bytes(&w) == NULL);
        free(taken);
        pb_writer_free(&w);
    }

    /* ── error-message strings are non-empty and distinct-ish ──────────────*/
    ISO_CHECK(strlen(pb_error_message(PB_ERR_TRUNCATED_VARINT)) > 0);
    ISO_CHECK(strcmp(pb_error_message(PB_OK), "ok") == 0);

    return ISO_TEST_RESULT();
}
