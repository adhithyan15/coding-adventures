/* Tests for the C range-coder, using the iso_test.h harness. Round trips and
 * bit-field vectors are taken from the Rust crate's own tests. */
#include "iso_test.h"

#include <stdlib.h> /* free */

#include "range_coder.h"

/* Encode a (bit, prob) sequence, then decode it and assert the bits match. */
static void round_trip(const int *bits, const unsigned char *probs, size_t n) {
    RcBoolEncoder enc;
    unsigned char *bytes = NULL;
    size_t byte_len = 0;
    size_t i;
    RcBoolDecoder dec;

    rc_encoder_init(&enc);
    for (i = 0; i < n; i++) {
        rc_encoder_write_bit(&enc, bits[i], probs[i]);
    }
    ISO_CHECK_EQ_INT((int)rc_encoder_finish(&enc, &bytes, &byte_len), (int)RC_OK);

    rc_decoder_init(&dec, bytes, byte_len);
    for (i = 0; i < n; i++) {
        int got = rc_decoder_read_bit(&dec, probs[i]);
        ISO_CHECK(got == (bits[i] ? 1 : 0));
    }
    free(bytes);
}

int main(void) {
    /* finish() on a fresh encoder produces non-empty output. */
    {
        RcBoolEncoder enc;
        unsigned char *bytes = NULL;
        size_t byte_len = 0;
        rc_encoder_init(&enc);
        rc_encoder_finish(&enc, &bytes, &byte_len);
        ISO_CHECK(byte_len > 0);
        free(bytes);
    }

    /* Single-bit round trips at 50/50. */
    {
        int t = 1, f = 0;
        unsigned char p = 128;
        round_trip(&t, &p, 1);
        round_trip(&f, &p, 1);
    }

    /* Mixed sequence from the crate's docs. */
    {
        int bits[] = {1, 0, 1, 0};
        unsigned char probs[] = {128, 200, 64, 128};
        round_trip(bits, probs, 4);
    }

    /* A longer sequence across a range of probabilities. */
    {
        int bits[32];
        unsigned char probs[32];
        size_t i;
        for (i = 0; i < 32; i++) {
            bits[i] = (int)((i * 7 + 3) % 2);
            probs[i] = (unsigned char)(1 + (i * 8) % 255);
        }
        round_trip(bits, probs, 32);
    }

    /* Skewed probabilities (near-certain bits) still round-trip. */
    {
        int bits[] = {0, 0, 0, 1, 0, 0};
        unsigned char probs[] = {250, 250, 250, 5, 250, 250};
        round_trip(bits, probs, 6);
    }

    /* write_bits / read_bits for u8, u16, u32. */
    {
        RcBoolEncoder enc;
        unsigned char *bytes = NULL;
        size_t byte_len = 0;
        RcBoolDecoder dec;
        rc_encoder_init(&enc);
        rc_encoder_write_bits(&enc, 0xAB, 8);
        rc_encoder_finish(&enc, &bytes, &byte_len);
        rc_decoder_init(&dec, bytes, byte_len);
        ISO_CHECK_EQ_UINT(rc_decoder_read_bits(&dec, 8), 0xABu);
        free(bytes);
    }
    {
        RcBoolEncoder enc;
        unsigned char *bytes = NULL;
        size_t byte_len = 0;
        RcBoolDecoder dec;
        rc_encoder_init(&enc);
        rc_encoder_write_bits(&enc, 0xDEAD, 16);
        rc_encoder_finish(&enc, &bytes, &byte_len);
        rc_decoder_init(&dec, bytes, byte_len);
        ISO_CHECK_EQ_UINT(rc_decoder_read_bits(&dec, 16), 0xDEADu);
        free(bytes);
    }
    {
        RcBoolEncoder enc;
        unsigned char *bytes = NULL;
        size_t byte_len = 0;
        RcBoolDecoder dec;
        rc_encoder_init(&enc);
        rc_encoder_write_bits(&enc, 0xCAFEBABEu, 32);
        rc_encoder_finish(&enc, &bytes, &byte_len);
        rc_decoder_init(&dec, bytes, byte_len);
        ISO_CHECK_EQ_UINT(rc_decoder_read_bits(&dec, 32), 0xCAFEBABEu);
        free(bytes);
    }

    /* write_bits(_, 0) writes nothing; read_bits(0) returns 0. */
    {
        RcBoolEncoder enc;
        unsigned char *bytes = NULL;
        size_t byte_len = 0;
        RcBoolDecoder dec;
        rc_encoder_init(&enc);
        rc_encoder_write_bits(&enc, 0xFF, 0);
        rc_encoder_finish(&enc, &bytes, &byte_len);
        rc_decoder_init(&dec, bytes, byte_len);
        ISO_CHECK_EQ_UINT(rc_decoder_read_bits(&dec, 0), 0u);
        free(bytes);
    }

    /* Deterministic output. */
    {
        RcBoolEncoder e1, e2;
        unsigned char *b1 = NULL, *b2 = NULL;
        size_t l1 = 0, l2 = 0, i;
        int bits[] = {1, 0, 1, 0};
        unsigned char probs[] = {128, 200, 64, 128};
        rc_encoder_init(&e1);
        rc_encoder_init(&e2);
        for (i = 0; i < 4; i++) {
            rc_encoder_write_bit(&e1, bits[i], probs[i]);
            rc_encoder_write_bit(&e2, bits[i], probs[i]);
        }
        rc_encoder_finish(&e1, &b1, &l1);
        rc_encoder_finish(&e2, &b2, &l2);
        ISO_CHECK_EQ_UINT(l1, l2);
        {
            int same = (l1 == l2);
            for (i = 0; same && i < l1; i++) {
                if (b1[i] != b2[i]) {
                    same = 0;
                }
            }
            ISO_CHECK(same);
        }
        free(b1);
        free(b2);
    }

    /* Decoder seeding and exhaustion (from the crate's tests). */
    {
        unsigned char seed[] = {0xAB, 0xCD};
        unsigned char one[] = {0xFF};
        unsigned char three[] = {0x00, 0x00, 0xFF};
        RcBoolDecoder d;
        rc_decoder_init(&d, seed, 2);
        ISO_CHECK(d.value == 0xABCDu && d.range == 255u && d.pos == 2 &&
                  d.bit_pos == 0);
        ISO_CHECK(rc_decoder_is_exhausted(&d));
        rc_decoder_init(&d, one, 1);
        ISO_CHECK(d.value == 0xFF00u);
        rc_decoder_init(&d, NULL, 0);
        ISO_CHECK(d.value == 0u && d.range == 255u);
        rc_decoder_init(&d, three, 3);
        ISO_CHECK(!rc_decoder_is_exhausted(&d));
    }

    return ISO_TEST_RESULT();
}
