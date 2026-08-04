/*
 * rans.c — implementation of table-based rANS (see rans.h). A faithful port of
 * the Rust `rans` crate: the same largest-remainder table normalisation, the
 * reverse-order encoder, and the O(1)-lookup decoder.
 */
#include "rans.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, calloc, realloc, free */

/* ===================================================================== *
 *  AnsTable
 * ===================================================================== */

/* Total order over symbol indices for remainder distribution: zero-frequency
 * symbols first, then descending fractional part, then ascending index. Returns
 * < 0 if a should come before b, > 0 if after, 0 if equal. */
static int order_cmp(size_t a, size_t b, const unsigned int *freq,
                     const unsigned int *counts, unsigned int m,
                     unsigned long long total) {
    int a_zero = (freq[a] == 0);
    int b_zero = (freq[b] == 0);
    unsigned long long fa, fb;
    if (a_zero != b_zero) {
        return a_zero ? -1 : 1; /* zero-frequency first */
    }
    fa = ((unsigned long long)counts[a] * m) % total;
    fb = ((unsigned long long)counts[b] * m) % total;
    if (fa != fb) {
        return (fa > fb) ? -1 : 1; /* descending fractional part */
    }
    return (a < b) ? -1 : (a > b ? 1 : 0); /* ascending index (stable) */
}

void ans_table_free(AnsTable *t) {
    if (!t) {
        return;
    }
    free(t->freq);
    free(t->cumfreq);
    free(t->decode_sym);
    free(t->decode_freq);
    free(t->decode_cumfreq);
    t->freq = t->cumfreq = NULL;
    t->decode_freq = t->decode_cumfreq = NULL;
    t->decode_sym = NULL;
    t->n = 0;
    t->m = 0;
    t->log2m = 0;
}

RansStatus ans_table_new(const unsigned int *counts, size_t n, AnsTable *out) {
    unsigned long long total = 0, min_m;
    unsigned int m, log2m = 0;
    unsigned int *freq = NULL, *cumfull = NULL;
    size_t *order = NULL;
    size_t i, sym;
    unsigned int remainder;

    /* zero the output so a failed call leaves it safe to free. */
    out->n = 0;
    out->freq = NULL;
    out->cumfreq = NULL;
    out->m = 0;
    out->log2m = 0;
    out->decode_sym = NULL;
    out->decode_freq = NULL;
    out->decode_cumfreq = NULL;

    if (n == 0) {
        return RANS_ERR_EMPTY;
    }
    if (n > 256) {
        return RANS_ERR_ALPHABET_TOO_LARGE;
    }
    for (i = 0; i < n; i++) {
        total += (unsigned long long)counts[i];
    }
    if (total == 0) {
        return RANS_ERR_ALL_ZERO;
    }

    min_m = (unsigned long long)n;
    if (total > min_m) {
        min_m = total;
    }
    if (min_m < 1) {
        min_m = 1;
    }
    if (min_m > (1ull << 16)) {
        return RANS_ERR_M_TOO_LARGE;
    }
    {
        unsigned long long mm = 1;
        while (mm < min_m) {
            log2m += 1;
            mm <<= 1;
        }
        m = (unsigned int)mm;
    }

    freq = calloc(n, sizeof *freq);
    cumfull = calloc(n + 1, sizeof *cumfull);
    order = calloc(n, sizeof *order);
    if (!freq || !cumfull || !order) {
        free(freq);
        free(cumfull);
        free(order);
        return RANS_ERR_ALLOC;
    }

    /* floor(count * m / total). */
    remainder = m;
    for (i = 0; i < n; i++) {
        freq[i] =
            (unsigned int)(((unsigned long long)counts[i] * m) / total);
        remainder -= freq[i];
    }

    /* Distribute the remainder by the total order (insertion sort — n <= 256). */
    for (i = 0; i < n; i++) {
        order[i] = i;
    }
    for (i = 1; i < n; i++) {
        size_t key = order[i];
        size_t j = i;
        while (j > 0 &&
               order_cmp(order[j - 1], key, freq, counts, m, total) > 0) {
            order[j] = order[j - 1];
            j--;
        }
        order[j] = key;
    }
    for (i = 0; i < n && remainder > 0; i++) {
        freq[order[i]] += 1;
        remainder--;
    }

    /* Every symbol must now have freq >= 1. */
    for (i = 0; i < n; i++) {
        if (freq[i] == 0) {
            free(freq);
            free(cumfull);
            free(order);
            return RANS_ERR_ZERO_FREQ;
        }
    }
    free(order);
    order = NULL;

    /* Cumulative frequencies (cumfull[n] == m). */
    cumfull[0] = 0;
    for (i = 0; i < n; i++) {
        cumfull[i + 1] = cumfull[i] + freq[i];
    }

    /* Build the flat decode table (m entries). */
    {
        unsigned char *dsym = calloc(m, sizeof *dsym);
        unsigned int *dfreq = calloc(m, sizeof *dfreq);
        unsigned int *dcum = calloc(m, sizeof *dcum);
        if (!dsym || !dfreq || !dcum) {
            free(freq);
            free(cumfull);
            free(dsym);
            free(dfreq);
            free(dcum);
            return RANS_ERR_ALLOC;
        }
        for (sym = 0; sym < n; sym++) {
            unsigned int lo = cumfull[sym];
            unsigned int hi = cumfull[sym + 1];
            unsigned int slot;
            for (slot = lo; slot < hi; slot++) {
                dsym[slot] = (unsigned char)sym;
                dfreq[slot] = freq[sym];
                dcum[slot] = cumfull[sym];
            }
        }
        out->decode_sym = dsym;
        out->decode_freq = dfreq;
        out->decode_cumfreq = dcum;
    }

    /* cumfreq stored is the length-n prefix (cumfull without the sentinel). */
    out->n = n;
    out->m = m;
    out->log2m = log2m;
    out->freq = freq;
    out->cumfreq = cumfull; /* length n+1 allocation; we only read [0, n) */
    return RANS_OK;
}

unsigned int ans_table_m(const AnsTable *t) { return t->m; }
unsigned int ans_table_log2m(const AnsTable *t) { return t->log2m; }
size_t ans_table_alphabet_size(const AnsTable *t) { return t->n; }

int ans_table_freq(const AnsTable *t, size_t s, unsigned int *out) {
    if (s >= t->n) {
        return 0;
    }
    *out = t->freq[s];
    return 1;
}
int ans_table_cumfreq(const AnsTable *t, size_t s, unsigned int *out) {
    if (s >= t->n) {
        return 0;
    }
    *out = t->cumfreq[s];
    return 1;
}

/* ===================================================================== *
 *  Encoder
 * ===================================================================== */

void rans_encoder_init(RansEncoder *e, const AnsTable *table) {
    e->table = table;
    e->x = table->m;
    e->pending = NULL;
    e->pend_len = 0;
    e->pend_cap = 0;
    e->ok = 1;
}

static void enc_push(RansEncoder *e, unsigned char b) {
    if (!e->ok) {
        return;
    }
    if (e->pend_len == e->pend_cap) {
        size_t ncap = e->pend_cap ? e->pend_cap * 2 : 32;
        unsigned char *nd;
        if (e->pend_cap > SIZE_MAX / 2) {
            e->ok = 0;
            return;
        }
        nd = realloc(e->pending, ncap);
        if (!nd) {
            e->ok = 0;
            return;
        }
        e->pending = nd;
        e->pend_cap = ncap;
    }
    e->pending[e->pend_len++] = b;
}

int rans_encoder_put(RansEncoder *e, unsigned char symbol) {
    size_t s = symbol;
    unsigned long long f, m, upper, q, r;
    if (!e->ok) {
        return 0;
    }
    if (s >= e->table->n) {
        e->ok = 0;
        return 0;
    }
    f = e->table->freq[s];
    m = e->table->m;

    upper = f << 8; /* f * 256 */
    while (e->x >= upper) {
        enc_push(e, (unsigned char)(e->x & 0xFF));
        e->x >>= 8;
        if (!e->ok) {
            return 0;
        }
    }
    q = e->x / f;
    r = e->x % f;
    e->x = q * m + e->table->cumfreq[s] + r;
    return 1;
}

RansStatus rans_encoder_finish(RansEncoder *e, unsigned char **out,
                               size_t *out_len) {
    unsigned long long x = e->x;
    size_t a, b;
    *out = NULL;
    *out_len = 0;
    /* Push the 8-byte state LSB-first; after reversing it becomes big-endian. */
    enc_push(e, (unsigned char)(x & 0xFF));
    enc_push(e, (unsigned char)((x >> 8) & 0xFF));
    enc_push(e, (unsigned char)((x >> 16) & 0xFF));
    enc_push(e, (unsigned char)((x >> 24) & 0xFF));
    enc_push(e, (unsigned char)((x >> 32) & 0xFF));
    enc_push(e, (unsigned char)((x >> 40) & 0xFF));
    enc_push(e, (unsigned char)((x >> 48) & 0xFF));
    enc_push(e, (unsigned char)((x >> 56) & 0xFF));
    if (!e->ok) {
        free(e->pending);
        e->pending = NULL;
        e->pend_len = e->pend_cap = 0;
        return RANS_ERR_ALLOC;
    }
    /* Reverse pending into decode-time order. */
    a = 0;
    b = e->pend_len;
    while (a + 1 < b) {
        unsigned char t = e->pending[a];
        e->pending[a] = e->pending[b - 1];
        e->pending[b - 1] = t;
        a++;
        b--;
    }
    *out = e->pending;
    *out_len = e->pend_len;
    e->pending = NULL;
    e->pend_len = e->pend_cap = 0;
    return RANS_OK;
}

void rans_encoder_free(RansEncoder *e) {
    if (!e) {
        return;
    }
    free(e->pending);
    e->pending = NULL;
    e->pend_len = e->pend_cap = 0;
}

/* ===================================================================== *
 *  Decoder
 * ===================================================================== */

RansStatus rans_decoder_init(RansDecoder *d, const AnsTable *table,
                             const unsigned char *data, size_t len) {
    d->table = table;
    d->data = data;
    d->len = len;
    d->pos = 8;
    d->x = 0;
    if (len < 8) {
        return RANS_ERR_SHORT_DATA;
    }
    d->x = ((unsigned long long)data[0] << 56) |
           ((unsigned long long)data[1] << 48) |
           ((unsigned long long)data[2] << 40) |
           ((unsigned long long)data[3] << 32) |
           ((unsigned long long)data[4] << 24) |
           ((unsigned long long)data[5] << 16) |
           ((unsigned long long)data[6] << 8) |
           (unsigned long long)data[7];
    return RANS_OK;
}

unsigned char rans_decoder_get(RansDecoder *d) {
    unsigned long long m = d->table->m;
    size_t slot = (size_t)(d->x % m); /* always < m, so in bounds */
    unsigned char sym = d->table->decode_sym[slot];
    unsigned long long f = d->table->decode_freq[slot];
    unsigned long long cf = d->table->decode_cumfreq[slot];

    /* x = f * (x / M) + (x % M) - cumfreq[sym].  (x % M) >= cf by construction. */
    d->x = f * (d->x / m) + (d->x % m) - cf;

    while (d->x < m) {
        if (d->pos < d->len) {
            d->x = (d->x << 8) | d->data[d->pos];
            d->pos++;
        } else {
            d->x <<= 8;
            if (d->x == 0) {
                /* Stream exhausted and the state is stuck at 0 (malformed
                 * input): stop instead of looping forever. */
                break;
            }
        }
    }
    return sym;
}

int rans_decoder_is_exhausted(const RansDecoder *d) {
    return d->pos >= d->len;
}
