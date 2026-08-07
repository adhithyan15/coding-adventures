/*
 * range_coder.c — implementation of the VP8 boolean range coder (see
 * range_coder.h). A faithful port of the Rust `range-coder` crate's encoder.rs
 * and decoder.rs.
 */
#include "range_coder.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, realloc, free */

/* ===================================================================== *
 *  Encoder
 * ===================================================================== */

void rc_encoder_init(RcBoolEncoder *e) {
    e->bottom = 0;
    e->range = 255;
    e->bit_count = -24;
    e->output = NULL;
    e->out_len = 0;
    e->out_cap = 0;
    e->ok = 1;
}

static void enc_push(RcBoolEncoder *e, unsigned char b) {
    if (!e->ok) {
        return;
    }
    if (e->out_len == e->out_cap) {
        size_t ncap = e->out_cap ? e->out_cap * 2 : 32;
        unsigned char *nd;
        if (e->out_cap > SIZE_MAX / 2) {
            e->ok = 0;
            return;
        }
        nd = realloc(e->output, ncap);
        if (!nd) {
            e->ok = 0;
            return;
        }
        e->output = nd;
        e->out_cap = ncap;
    }
    e->output[e->out_len++] = b;
}

void rc_encoder_write_bit(RcBoolEncoder *e, int bit, unsigned char prob) {
    unsigned int split = 1u + (((e->range - 1u) * (unsigned int)prob) >> 8);

    if (bit) {
        e->bottom += split; /* upper sub-interval */
        e->range -= split;
    } else {
        e->range = split; /* lower sub-interval */
    }

    while (e->range < 128u) {
        e->range <<= 1;
        e->bottom <<= 1;
        e->bit_count += 1;
        if (e->bit_count == 0) {
            enc_push(e, (unsigned char)((e->bottom >> 24) & 0xFFu));
            e->bottom &= 0x00FFFFFFull; /* keep the 24-bit working register */
            e->bit_count = -8;
        }
    }
}

void rc_encoder_write_bits(RcBoolEncoder *e, unsigned int value,
                           unsigned char n) {
    int i;
    for (i = (int)n - 1; i >= 0; i--) {
        /* Guard the shift: bits at i >= 32 are 0 (value is 32-bit), which also
         * avoids shift-width UB if a caller ignores the n <= 32 contract. */
        int bit = (i < 32) ? (int)((value >> i) & 1u) : 0;
        rc_encoder_write_bit(e, bit, 128);
    }
}

RcStatus rc_encoder_finish(RcBoolEncoder *e, unsigned char **out,
                           size_t *out_len) {
    int i;
    *out = NULL;
    *out_len = 0;
    for (i = 0; i < 32; i++) { /* flush 32 zero bits */
        rc_encoder_write_bit(e, 0, 128);
    }
    if (!e->ok) {
        free(e->output);
        e->output = NULL;
        e->out_len = e->out_cap = 0;
        return RC_ERR_ALLOC;
    }
    *out = e->output;
    *out_len = e->out_len;
    e->output = NULL; /* ownership transferred */
    e->out_len = e->out_cap = 0;
    return RC_OK;
}

void rc_encoder_free(RcBoolEncoder *e) {
    if (!e) {
        return;
    }
    free(e->output);
    e->output = NULL;
    e->out_len = e->out_cap = 0;
}

/* ===================================================================== *
 *  Decoder
 * ===================================================================== */

void rc_decoder_init(RcBoolDecoder *d, const unsigned char *data, size_t len) {
    d->data = data;
    d->len = len;
    d->pos = 2;
    d->bit_pos = 0;
    d->range = 255;
    if (len >= 2) {
        d->value = ((unsigned int)data[0] << 8) | (unsigned int)data[1];
    } else if (len == 1) {
        d->value = (unsigned int)data[0] << 8;
    } else {
        d->value = 0;
    }
}

static unsigned int dec_next_msb_bit(RcBoolDecoder *d) {
    unsigned char byte;
    unsigned int bit;
    if (d->pos >= d->len) {
        return 0; /* pad an exhausted stream with zeros */
    }
    byte = d->data[d->pos];
    bit = ((unsigned int)byte >> (7 - d->bit_pos)) & 1u;
    d->bit_pos++;
    if (d->bit_pos == 8) {
        d->bit_pos = 0;
        d->pos++;
    }
    return bit;
}

int rc_decoder_read_bit(RcBoolDecoder *d, unsigned char prob) {
    unsigned int split = 1u + (((d->range - 1u) * (unsigned int)prob) >> 8);
    unsigned int bigsplit = split << 8;
    int bit;

    if (d->value >= bigsplit) {
        d->range -= split;
        d->value -= bigsplit;
        bit = 1;
    } else {
        d->range = split;
        bit = 0;
    }

    while (d->range < 128u) {
        d->range <<= 1;
        d->value = (d->value << 1) | dec_next_msb_bit(d);
    }
    return bit;
}

unsigned int rc_decoder_read_bits(RcBoolDecoder *d, unsigned char n) {
    unsigned int result = 0;
    unsigned char i;
    for (i = 0; i < n; i++) {
        result = (result << 1) | (unsigned int)rc_decoder_read_bit(d, 128);
    }
    return result;
}

int rc_decoder_is_exhausted(const RcBoolDecoder *d) {
    return d->pos >= d->len;
}
