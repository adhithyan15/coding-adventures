/*
 * rans.h — table-based rANS (range Asymmetric Numeral Systems) entropy coding,
 * in pure ISO C17. A faithful port of the Rust `rans` crate.
 * ===========================================================================
 *
 * rANS is a modern entropy coder (the "A" in Zstandard/JPEG XL): it codes a
 * stream of alphabet symbols against a fixed frequency table, approaching the
 * Shannon entropy while being fast and integer-only.
 *
 * AnsTable — built from raw symbol counts. The counts are normalised so their
 *   frequencies sum to a power of two M = 2^k (M >= alphabet size, M <= 2^16),
 *   using the largest-remainder method; a flat M-entry decode table gives O(1)
 *   symbol lookup.
 * RansEncoder — `put` symbols in REVERSE order (rANS is LIFO), then `finish`
 *   to get the byte stream (an 8-byte big-endian state header + renorm bytes).
 * RansDecoder — `get` symbols back in forward order.
 *
 * The decode table is constructed so that for every slot, `slot - cumfreq` is in
 * [0, freq); the decoder therefore stays in bounds and never underflows for any
 * input state, so a malformed byte stream cannot cause an out-of-bounds access.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. Arithmetic is 64-bit (no 128-bit ints).
 */
#ifndef RANS_H
#define RANS_H

#include <stddef.h> /* size_t */

typedef enum {
    RANS_OK = 0,
    RANS_ERR_EMPTY,             /* counts is empty */
    RANS_ERR_ALPHABET_TOO_LARGE,/* alphabet size > 256 */
    RANS_ERR_ALL_ZERO,          /* all counts are zero */
    RANS_ERR_M_TOO_LARGE,       /* normalised M would exceed 2^16 */
    RANS_ERR_ZERO_FREQ,         /* a symbol has zero frequency after normalising */
    RANS_ERR_SHORT_DATA,        /* decoder input shorter than 8 bytes */
    RANS_ERR_SYMBOL_RANGE,      /* encoder given a symbol >= alphabet size */
    RANS_ERR_ALLOC
} RansStatus;

/* A precomputed rANS frequency table. Build with ans_table_new, release with
 * ans_table_free. Prefer the accessors over the fields. */
typedef struct {
    size_t n;                    /* alphabet size */
    unsigned int *freq;          /* normalised frequencies (n), summing to m */
    unsigned int *cumfreq;       /* cumfreq[i] = sum of freq[0..i] (n) */
    unsigned int m;              /* table size M = 2^k */
    unsigned int log2m;          /* log2(M) */
    unsigned char *decode_sym;   /* decode table: symbol at each slot (m) */
    unsigned int *decode_freq;   /* freq of the symbol at each slot (m) */
    unsigned int *decode_cumfreq;/* cumfreq of the symbol at each slot (m) */
} AnsTable;

/* ---- table ------------------------------------------------------------ */

RansStatus ans_table_new(const unsigned int *counts, size_t n, AnsTable *out);
void ans_table_free(AnsTable *t);

unsigned int ans_table_m(const AnsTable *t);
unsigned int ans_table_log2m(const AnsTable *t);
size_t ans_table_alphabet_size(const AnsTable *t);
/* freq / cumfreq: 1 and *out set if s < alphabet size, else 0 (like Option). */
int ans_table_freq(const AnsTable *t, size_t s, unsigned int *out);
int ans_table_cumfreq(const AnsTable *t, size_t s, unsigned int *out);

/* ---- encoder ---------------------------------------------------------- */

typedef struct {
    const AnsTable *table; /* borrowed; must outlive the encoder */
    unsigned long long x;
    unsigned char *pending;
    size_t pend_len, pend_cap;
    int ok; /* cleared on allocation failure or an out-of-range symbol */
} RansEncoder;

/* rans_encoder_init — start an encoder over `table`. */
void rans_encoder_init(RansEncoder *e, const AnsTable *table);

/* rans_encoder_put — encode one symbol INDEX (0 .. alphabet-1). Symbols are
 * pushed in reverse order. Returns 1, or 0 on an out-of-range symbol or
 * allocation failure (the error is latched for finish). */
int rans_encoder_put(RansEncoder *e, unsigned char symbol);

/* rans_encoder_finish — flush and hand off the byte stream. On RANS_OK *out is
 * the malloc'd output (caller frees) and *out_len its length; the encoder is
 * left empty. */
RansStatus rans_encoder_finish(RansEncoder *e, unsigned char **out,
                               size_t *out_len);

/* rans_encoder_free — free an encoder abandoned without finishing. */
void rans_encoder_free(RansEncoder *e);

/* ---- decoder ---------------------------------------------------------- */

typedef struct {
    const AnsTable *table; /* borrowed */
    unsigned long long x;
    const unsigned char *data; /* borrowed; must outlive the decoder */
    size_t len, pos;
} RansDecoder;

/* rans_decoder_init — start a decoder from a `finish`ed byte stream. Returns
 * RANS_ERR_SHORT_DATA if len < 8. */
RansStatus rans_decoder_init(RansDecoder *d, const AnsTable *table,
                             const unsigned char *data, size_t len);

/* rans_decoder_get — decode the next symbol index. */
unsigned char rans_decoder_get(RansDecoder *d);

/* rans_decoder_is_exhausted — 1 if all input bytes have been consumed. */
int rans_decoder_is_exhausted(const RansDecoder *d);

#endif /* RANS_H */
