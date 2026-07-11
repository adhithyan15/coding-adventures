/*
 * range_coder.h — the VP8 boolean range coder (RFC 6386 §7), in pure ISO C17. A
 * faithful port of the Rust `range-coder` crate.
 * ===========================================================================
 *
 * A boolean range coder (a binary arithmetic coder) compresses a sequence of
 * bits, each with an 8-bit probability that the bit is 0 (`prob`, where 128 is
 * 50/50). It is the entropy stage of VP8 / WebP.
 *
 * Encoder: maintain a coding interval; `write_bit(bit, prob)` narrows it and
 * emits high-order bytes as they become determined; `finish` flushes.
 * Decoder: seed a 16-bit window from the first two bytes, then `read_bit(prob)`
 * splits the interval and renormalizes, pulling fresh bits from the stream.
 *
 *   split    = 1 + (((range - 1) * prob) >> 8)      // the +1 keeps both halves
 *   bit==0 -> lower sub-interval, bit==1 -> upper    // non-empty
 *
 * Bits are MSB-first; an exhausted stream reads as zeros. Encoding a sequence
 * then decoding it with the same probabilities recovers the original bits.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef RANGE_CODER_H
#define RANGE_CODER_H

#include <stddef.h> /* size_t */

typedef enum { RC_OK = 0, RC_ERR_ALLOC } RcStatus;

/* ---- encoder ---------------------------------------------------------- */

typedef struct {
    unsigned long long bottom; /* low end of the coding interval (working reg) */
    unsigned int range;        /* interval width, kept in [128, 255] */
    int bit_count;             /* renormalization counter (starts at -24) */
    unsigned char *output;
    size_t out_len, out_cap;
    int ok; /* cleared on allocation failure */
} RcBoolEncoder;

/* rc_encoder_init — put an encoder in its initial state. */
void rc_encoder_init(RcBoolEncoder *e);

/* rc_encoder_write_bit — encode one bit (`bit` != 0 for a 1-bit); `prob` is the
 * probability the bit is 0, scaled to 0..255. */
void rc_encoder_write_bit(RcBoolEncoder *e, int bit, unsigned char prob);

/* rc_encoder_write_bits — encode the low `n` bits of `value`, MSB first, with a
 * uniform probability (prob = 128). `n` must be <= 32. */
void rc_encoder_write_bits(RcBoolEncoder *e, unsigned int value,
                           unsigned char n);

/* rc_encoder_finish — flush and hand off the encoded bytes. On RC_OK *out is the
 * malloc'd output (caller frees) and *out_len its length; the encoder is left
 * empty. Returns RC_ERR_ALLOC if any write ran out of memory. */
RcStatus rc_encoder_finish(RcBoolEncoder *e, unsigned char **out,
                           size_t *out_len);

/* rc_encoder_free — free an encoder's buffer without finishing (for the abandon
 * path). Safe after rc_encoder_finish (which already transferred the buffer). */
void rc_encoder_free(RcBoolEncoder *e);

/* ---- decoder ---------------------------------------------------------- */

/* Borrows `data` — it must outlive the decoder (like the crate's &[u8]). */
typedef struct {
    const unsigned char *data;
    size_t len;
    size_t pos;
    unsigned char bit_pos;
    unsigned int range;
    unsigned int value;
} RcBoolDecoder;

/* rc_decoder_init — seed a decoder from the first two bytes of `data`. */
void rc_decoder_init(RcBoolDecoder *d, const unsigned char *data, size_t len);

/* rc_decoder_read_bit — decode one bit; returns 1 or 0. `prob` matches the value
 * passed to the encoder. */
int rc_decoder_read_bit(RcBoolDecoder *d, unsigned char prob);

/* rc_decoder_read_bits — decode `n` bits (prob = 128) MSB-first into a value;
 * n == 0 returns 0. */
unsigned int rc_decoder_read_bits(RcBoolDecoder *d, unsigned char n);

/* rc_decoder_is_exhausted — 1 if the byte cursor has reached the end of data
 * (bits may still be read; missing bytes read as 0). */
int rc_decoder_is_exhausted(const RcBoolDecoder *d);

#endif /* RANGE_CODER_H */
