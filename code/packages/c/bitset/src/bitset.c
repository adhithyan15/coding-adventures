/*
 * bitset.c — implementation of the growable 64-bit-word bitset. Ported from the
 * Rust `bitset` crate; the word layout, auto-grow (capacity doubling), and
 * trailing-bit cleanup all match it.
 */
#include "bitset.h"

#include <stdlib.h> /* malloc, calloc, realloc, free */
#include <string.h> /* strlen */

#define BITS_PER_WORD 64u

static size_t words_needed(size_t bit_count) {
    return (bit_count + (BITS_PER_WORD - 1)) / BITS_PER_WORD;
}

static size_t word_index(size_t i) { return i / BITS_PER_WORD; }

static uint64_t bitmask(size_t i) {
    return (uint64_t)1 << (i % BITS_PER_WORD);
}

/* Portable 64-bit population count (no compiler intrinsics). */
static size_t popcount64(uint64_t w) {
    size_t count = 0;
    while (w != 0) {
        w &= w - 1; /* clear the lowest set bit */
        count++;
    }
    return count;
}

/* Mask off any bits above `len` in the last allocated word — the crate's
 * clean_trailing_bits, so higher bitwise-op words never expose phantom bits. */
static void clean_trailing_bits(bitset *b) {
    size_t remaining;
    if (b->len == 0 || b->nwords == 0) {
        return;
    }
    remaining = b->len % BITS_PER_WORD;
    if (remaining != 0) {
        b->words[b->nwords - 1] &= ((uint64_t)1 << remaining) - 1;
    }
}

int bitset_init(bitset *b, size_t size) {
    size_t nwords = words_needed(size);
    b->words = NULL;
    b->nwords = 0;
    b->len = size;
    if (nwords > 0) {
        b->words = (uint64_t *)calloc(nwords, sizeof(uint64_t));
        if (b->words == NULL) {
            b->len = 0;
            return 0;
        }
        b->nwords = nwords;
    }
    return 1;
}

int bitset_from_integer(bitset *b, uint64_t low, uint64_t high) {
    if (low == 0 && high == 0) {
        return bitset_init(b, 0); /* zero → empty bitset, matching the crate */
    }
    if (high != 0) {
        if (!bitset_init(b, 128)) {
            return 0;
        }
        b->words[0] = low;
        b->words[1] = high;
    } else {
        if (!bitset_init(b, 64)) {
            return 0;
        }
        b->words[0] = low;
    }
    return 1;
}

int bitset_from_binary_str(bitset *b, const char *s) {
    size_t len = strlen(s);
    size_t k;
    /* Validate first: every character must be '0' or '1'. */
    for (k = 0; k < len; k++) {
        if (s[k] != '0' && s[k] != '1') {
            return -1;
        }
    }
    if (len == 0) {
        return bitset_init(b, 0);
    }
    if (!bitset_init(b, len)) {
        return 0;
    }
    /* The string is most-significant-bit-first, so the rightmost char is bit 0:
     * reversed index char_idx maps to bit char_idx. */
    for (k = 0; k < len; k++) {
        char ch = s[len - 1 - k];
        if (ch == '1') {
            b->words[word_index(k)] |= bitmask(k);
        }
    }
    clean_trailing_bits(b);
    return 1;
}

void bitset_free(bitset *b) {
    free(b->words);
    b->words = NULL;
    b->nwords = 0;
    b->len = 0;
}

size_t bitset_capacity(const bitset *b) { return b->nwords * BITS_PER_WORD; }

/* Grow so bit `i` is addressable, doubling capacity like the crate; also extend
 * len to i+1 when i is at or past it. Returns 1, or 0 on allocation failure. */
static int ensure_capacity(bitset *b, size_t i) {
    size_t cap = bitset_capacity(b);
    size_t new_cap, new_word_count;
    uint64_t *grown;
    size_t w;

    if (i < cap) {
        if (i >= b->len) {
            b->len = i + 1;
        }
        return 1;
    }
    if (i == SIZE_MAX) {
        return 0; /* len = i + 1 would overflow; no allocation could hold it */
    }
    /* Double capacity, but stop before size_t overflow (which would wrap to 0
     * and spin forever). If doubling can no longer reach i, size the buffer to
     * exactly cover bit i (i / 64 + 1 words — overflow-free). */
    new_cap = cap > BITS_PER_WORD ? cap : BITS_PER_WORD;
    while (new_cap <= i && new_cap <= SIZE_MAX / 2) {
        new_cap *= 2;
    }
    new_word_count =
        (new_cap > i) ? (new_cap / BITS_PER_WORD) : (i / BITS_PER_WORD + 1);
    if (new_word_count > SIZE_MAX / sizeof(uint64_t)) {
        return 0; /* byte size would overflow */
    }
    grown = (uint64_t *)realloc(b->words, new_word_count * sizeof(uint64_t));
    if (grown == NULL) {
        return 0;
    }
    for (w = b->nwords; w < new_word_count; w++) {
        grown[w] = 0; /* zero the freshly-added words */
    }
    b->words = grown;
    b->nwords = new_word_count;
    b->len = i + 1;
    return 1;
}

int bitset_set(bitset *b, size_t i) {
    if (!ensure_capacity(b, i)) {
        return 0;
    }
    b->words[word_index(i)] |= bitmask(i);
    return 1;
}

int bitset_toggle(bitset *b, size_t i) {
    if (!ensure_capacity(b, i)) {
        return 0;
    }
    b->words[word_index(i)] ^= bitmask(i);
    clean_trailing_bits(b);
    return 1;
}

void bitset_clear(bitset *b, size_t i) {
    if (i >= b->len) {
        return;
    }
    b->words[word_index(i)] &= ~bitmask(i);
}

int bitset_test(const bitset *b, size_t i) {
    if (i >= b->len) {
        return 0;
    }
    return (b->words[word_index(i)] & bitmask(i)) != 0 ? 1 : 0;
}

/* Shared driver for and/or/xor/and_not: apply `op` word-by-word over the wider
 * of the two operands, into a freshly allocated `out`. `op` selects the logic. */
typedef enum { OP_AND, OP_OR, OP_XOR, OP_AND_NOT } binop;

static int binary_op(const bitset *a, const bitset *b, bitset *out, binop op) {
    size_t result_len = a->len > b->len ? a->len : b->len;
    size_t max_words = a->nwords > b->nwords ? a->nwords : b->nwords;
    size_t i;

    out->words = NULL;
    out->nwords = 0;
    out->len = result_len;
    if (max_words == 0) {
        return 1; /* both empty */
    }
    out->words = (uint64_t *)calloc(max_words, sizeof(uint64_t));
    if (out->words == NULL) {
        out->len = 0;
        return 0;
    }
    out->nwords = max_words;
    for (i = 0; i < max_words; i++) {
        uint64_t av = i < a->nwords ? a->words[i] : 0;
        uint64_t bv = i < b->nwords ? b->words[i] : 0;
        switch (op) {
        case OP_AND:
            out->words[i] = av & bv;
            break;
        case OP_OR:
            out->words[i] = av | bv;
            break;
        case OP_XOR:
            out->words[i] = av ^ bv;
            break;
        case OP_AND_NOT:
            out->words[i] = av & ~bv;
            break;
        }
    }
    clean_trailing_bits(out);
    return 1;
}

int bitset_and(const bitset *a, const bitset *b, bitset *out) {
    return binary_op(a, b, out, OP_AND);
}
int bitset_or(const bitset *a, const bitset *b, bitset *out) {
    return binary_op(a, b, out, OP_OR);
}
int bitset_xor(const bitset *a, const bitset *b, bitset *out) {
    return binary_op(a, b, out, OP_XOR);
}
int bitset_and_not(const bitset *a, const bitset *b, bitset *out) {
    return binary_op(a, b, out, OP_AND_NOT);
}

int bitset_not(const bitset *a, bitset *out) {
    size_t i;
    out->words = NULL;
    out->nwords = 0;
    out->len = a->len;
    if (a->nwords == 0) {
        return 1;
    }
    out->words = (uint64_t *)malloc(a->nwords * sizeof(uint64_t));
    if (out->words == NULL) {
        out->len = 0;
        return 0;
    }
    out->nwords = a->nwords;
    for (i = 0; i < a->nwords; i++) {
        out->words[i] = ~a->words[i];
    }
    clean_trailing_bits(out);
    return 1;
}

size_t bitset_popcount(const bitset *b) {
    size_t total = 0;
    size_t i;
    for (i = 0; i < b->nwords; i++) {
        total += popcount64(b->words[i]);
    }
    return total;
}

size_t bitset_len(const bitset *b) { return b->len; }

int bitset_any(const bitset *b) {
    size_t i;
    for (i = 0; i < b->nwords; i++) {
        if (b->words[i] != 0) {
            return 1;
        }
    }
    return 0;
}

int bitset_all(const bitset *b) {
    size_t i;
    size_t remaining;
    if (b->len == 0) {
        return 1;
    }
    /* Every full word below the last must be all ones. */
    for (i = 0; i + 1 < b->nwords; i++) {
        if (b->words[i] != ~(uint64_t)0) {
            return 0;
        }
    }
    remaining = b->len % BITS_PER_WORD;
    if (remaining == 0) {
        return b->words[b->nwords - 1] == ~(uint64_t)0 ? 1 : 0;
    }
    return b->words[b->nwords - 1] == (((uint64_t)1 << remaining) - 1) ? 1 : 0;
}

int bitset_none(const bitset *b) { return bitset_any(b) ? 0 : 1; }

int bitset_is_empty(const bitset *b) { return b->len == 0 ? 1 : 0; }

int bitset_to_integer(const bitset *b, uint64_t *out) {
    size_t i;
    if (b->len == 0) {
        *out = 0;
        return 1;
    }
    for (i = 1; i < b->nwords; i++) {
        if (b->words[i] != 0) {
            return 0; /* bits set beyond 63 → does not fit in 64 bits */
        }
    }
    *out = b->words[0];
    return 1;
}

long bitset_to_binary_str(const bitset *b, char *buf, size_t buf_size) {
    size_t i;
    if (buf_size <= b->len) {
        return -1; /* need len chars + NUL */
    }
    for (i = 0; i < b->len; i++) {
        /* Most-significant bit first: position 0 in the string is bit len-1. */
        size_t bit = b->len - 1 - i;
        buf[i] = bitset_test(b, bit) ? '1' : '0';
    }
    buf[b->len] = '\0';
    return (long)b->len;
}
