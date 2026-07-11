/*
 * lzw.c — implementation of variable-width LZW. Ported from the Rust `lzw`
 * crate; code widths, CLEAR/STOP handling, LSB-first bit packing, dictionary
 * reset, and the KwKwK "tricky token" case all match.
 *
 * The encoder keys its dictionary on (prefix_code, byte) rather than on whole
 * byte sequences — this assigns codes in exactly the same order as the crate's
 * Vec<u8> map, so the output is bit-identical, but without O(sequence) hashing.
 */
#include "lzw.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, calloc, realloc, free */
#include <string.h> /* memcpy, memset */

#define CLEAR_CODE 256u
#define STOP_CODE 257u
#define INITIAL_NEXT_CODE 258u
#define INITIAL_CODE_SIZE 9
#define MAX_CODE_SIZE 16
#define MAX_ENTRIES (1u << MAX_CODE_SIZE) /* 65536 */

/* ── growable byte buffer ─────────────────────────────────────────────────── */
typedef struct {
    uint8_t *data;
    size_t len;
    size_t cap;
    int failed;
} bytebuf;

static void bb_push(bytebuf *b, uint8_t byte) {
    if (b->failed) {
        return;
    }
    if (b->len == b->cap) {
        size_t new_cap = b->cap == 0 ? 64 : b->cap * 2;
        uint8_t *grown;
        if (b->cap > SIZE_MAX / 2) {
            b->failed = 1;
            return;
        }
        grown = (uint8_t *)realloc(b->data, new_cap);
        if (grown == NULL) {
            b->failed = 1;
            return;
        }
        b->data = grown;
        b->cap = new_cap;
    }
    b->data[b->len++] = byte;
}

/* ── LSB-first bit writer ─────────────────────────────────────────────────── */
typedef struct {
    bytebuf out;
    uint64_t buffer;
    unsigned bit_pos;
} bitwriter;

static void bw_write(bitwriter *w, unsigned code, unsigned size) {
    w->buffer |= (uint64_t)code << w->bit_pos;
    w->bit_pos += size;
    while (w->bit_pos >= 8) {
        bb_push(&w->out, (uint8_t)(w->buffer & 0xff));
        w->buffer >>= 8;
        w->bit_pos -= 8;
    }
}

static void bw_flush(bitwriter *w) {
    if (w->bit_pos > 0) {
        bb_push(&w->out, (uint8_t)(w->buffer & 0xff));
        w->buffer = 0;
        w->bit_pos = 0;
    }
}

/* ── LSB-first bit reader ─────────────────────────────────────────────────── */
typedef struct {
    const uint8_t *data;
    size_t len;
    size_t pos;
    uint64_t buffer;
    unsigned bit_pos;
} bitreader;

/* Returns 1 and sets *code, or 0 at end of stream. */
static int br_read(bitreader *r, unsigned size, unsigned *code) {
    uint64_t mask;
    while (r->bit_pos < size) {
        if (r->pos >= r->len) {
            if (r->bit_pos == 0) {
                return 0;
            }
            break;
        }
        r->buffer |= (uint64_t)r->data[r->pos] << r->bit_pos;
        r->pos++;
        r->bit_pos += 8;
    }
    if (r->bit_pos < size) {
        return 0;
    }
    mask = ((uint64_t)1 << size) - 1;
    *code = (unsigned)(r->buffer & mask);
    r->buffer >>= size;
    r->bit_pos -= size;
    return 1;
}

/* ── encoder dictionary: (prefix, byte) → code, open-addressed ────────────── */
#define ENC_SLOTS (1u << 17) /* 131072; load factor stays < 0.5 */

typedef struct {
    uint32_t prefix; /* prefix code (0..65535) */
    uint16_t byte;   /* the following byte (0..255) */
    uint32_t code;   /* assigned code */
    uint8_t used;
} enc_slot;

static uint32_t enc_hash(uint32_t prefix, uint16_t byte) {
    uint32_t h = prefix * 2654435761u + byte * 40503u;
    return h & (ENC_SLOTS - 1);
}

/* Look up (prefix, byte). Returns 1 and sets *code if present, else 0. */
static int enc_lookup(const enc_slot *tbl, uint32_t prefix, uint16_t byte,
                      uint32_t *code) {
    uint32_t i = enc_hash(prefix, byte);
    while (tbl[i].used) {
        if (tbl[i].prefix == prefix && tbl[i].byte == byte) {
            *code = tbl[i].code;
            return 1;
        }
        i = (i + 1) & (ENC_SLOTS - 1);
    }
    return 0;
}

static void enc_insert(enc_slot *tbl, uint32_t prefix, uint16_t byte,
                       uint32_t code) {
    uint32_t i = enc_hash(prefix, byte);
    while (tbl[i].used) {
        i = (i + 1) & (ENC_SLOTS - 1);
    }
    tbl[i].prefix = prefix;
    tbl[i].byte = byte;
    tbl[i].code = code;
    tbl[i].used = 1;
}

int lzw_compress(const uint8_t *data, size_t len, uint8_t **out,
                 size_t *out_len) {
    enc_slot *tbl;
    bitwriter w;
    long w_code = -1; /* code of the current prefix; -1 = empty */
    uint32_t next_code = INITIAL_NEXT_CODE;
    unsigned code_size = INITIAL_CODE_SIZE;
    size_t i;
    uint32_t original_length = (uint32_t)len;

    tbl = (enc_slot *)calloc(ENC_SLOTS, sizeof(enc_slot));
    if (tbl == NULL) {
        return 0;
    }
    w.out.data = NULL;
    w.out.len = 0;
    w.out.cap = 0;
    w.out.failed = 0;
    w.buffer = 0;
    w.bit_pos = 0;

    bw_write(&w, CLEAR_CODE, code_size);

    for (i = 0; i < len; i++) {
        uint16_t b = data[i];
        uint32_t found;
        if (w_code < 0) {
            w_code = b; /* single-byte prefix */
            continue;
        }
        if (enc_lookup(tbl, (uint32_t)w_code, b, &found)) {
            w_code = (long)found; /* extend the prefix */
            continue;
        }
        /* w + b is new: emit the code for w, then record w + b. */
        bw_write(&w, (unsigned)w_code, code_size);
        if (next_code < MAX_ENTRIES) {
            enc_insert(tbl, (uint32_t)w_code, b, next_code);
            next_code++;
            if (next_code > (1u << code_size) && code_size < MAX_CODE_SIZE) {
                code_size++;
            }
        } else {
            /* Dictionary full — clear and start over (decoder mirrors this). */
            bw_write(&w, CLEAR_CODE, code_size);
            memset(tbl, 0, ENC_SLOTS * sizeof(enc_slot));
            next_code = INITIAL_NEXT_CODE;
            code_size = INITIAL_CODE_SIZE;
        }
        w_code = b;
    }
    if (w_code >= 0) {
        bw_write(&w, (unsigned)w_code, code_size);
    }
    bw_write(&w, STOP_CODE, code_size);
    bw_flush(&w);
    free(tbl);

    if (w.out.failed) {
        free(w.out.data);
        return 0;
    }
    /* Prepend the 4-byte big-endian original length. */
    {
        size_t total;
        uint8_t *buf;
        if (w.out.len > SIZE_MAX - 4) {
            free(w.out.data);
            return 0;
        }
        total = 4 + w.out.len;
        buf = (uint8_t *)malloc(total);
        if (buf == NULL) {
            free(w.out.data);
            return 0;
        }
        buf[0] = (uint8_t)(original_length >> 24);
        buf[1] = (uint8_t)(original_length >> 16);
        buf[2] = (uint8_t)(original_length >> 8);
        buf[3] = (uint8_t)(original_length);
        memcpy(buf + 4, w.out.data, w.out.len);
        free(w.out.data);
        *out = buf;
        *out_len = total;
    }
    return 1;
}

/* ── decoder ──────────────────────────────────────────────────────────────── */
int lzw_decompress(const uint8_t *data, size_t len, uint8_t **out,
                   size_t *out_len) {
    /* dict entry = (prefix code, last byte, first byte of the whole sequence) */
    long *prefix;
    uint8_t *last;
    uint8_t *first;
    uint8_t *stack;
    bytebuf output;
    bitreader r;
    size_t dict_len;
    uint32_t next_code = INITIAL_NEXT_CODE;
    unsigned code_size = INITIAL_CODE_SIZE;
    long prev_code = -1;
    unsigned code;
    size_t original_length;
    int ok = 0;

    if (len < 4) {
        return 0; /* missing header */
    }
    original_length = ((size_t)data[0] << 24) | ((size_t)data[1] << 16) |
                      ((size_t)data[2] << 8) | ((size_t)data[3]);

    prefix = (long *)malloc(MAX_ENTRIES * sizeof(long));
    last = (uint8_t *)malloc(MAX_ENTRIES);
    first = (uint8_t *)malloc(MAX_ENTRIES);
    stack = (uint8_t *)malloc(MAX_ENTRIES);
    if (prefix == NULL || last == NULL || first == NULL || stack == NULL) {
        free(prefix);
        free(last);
        free(first);
        free(stack);
        return 0;
    }
    output.data = NULL;
    output.len = 0;
    output.cap = 0;
    output.failed = 0;

    /* Seed the single-byte codes 0..255; 256/257 are control placeholders. */
    {
        unsigned b;
        for (b = 0; b < 256; b++) {
            prefix[b] = -1;
            last[b] = (uint8_t)b;
            first[b] = (uint8_t)b;
        }
    }
    dict_len = INITIAL_NEXT_CODE; /* next slot to fill */

    r.data = data + 4;
    r.len = len - 4;
    r.pos = 0;
    r.buffer = 0;
    r.bit_pos = 0;

    /* The stream must open with CLEAR. */
    if (!br_read(&r, code_size, &code) || code != CLEAR_CODE) {
        goto done;
    }

    for (;;) {
        size_t entry_len;
        uint8_t entry_first;
        unsigned c;

        if (!br_read(&r, code_size, &code)) {
            break;
        }
        if (code == CLEAR_CODE) {
            dict_len = INITIAL_NEXT_CODE;
            next_code = INITIAL_NEXT_CODE;
            code_size = INITIAL_CODE_SIZE;
            prev_code = -1;
            continue;
        }
        if (code == STOP_CODE) {
            break;
        }

        /* Resolve `code` into a byte sequence, pushed onto `stack` in reverse. */
        entry_len = 0;
        if ((size_t)code < dict_len) {
            c = code;
        } else if ((size_t)code == dict_len && prev_code >= 0) {
            /* KwKwK: the sequence is dict[prev] followed by its own first byte;
             * emit dict[prev] then that first byte. */
            stack[entry_len++] = first[prev_code];
            c = (unsigned)prev_code;
        } else {
            goto done; /* malformed */
        }
        /* Walk the prefix chain, collecting bytes from last back to first. */
        while (1) {
            stack[entry_len++] = last[c];
            if (prefix[c] < 0) {
                break;
            }
            c = (unsigned)prefix[c];
        }
        entry_first = stack[entry_len - 1]; /* the first byte of the sequence */
        /* Emit in forward order. */
        {
            size_t k = entry_len;
            while (k > 0) {
                k--;
                bb_push(&output, stack[k]);
            }
        }
        if (output.failed) {
            goto done;
        }

        /* code_size tracking mirrors the encoder: bump for every data code. */
        if (next_code < MAX_ENTRIES) {
            next_code++;
            if (next_code > (1u << code_size) && code_size < MAX_CODE_SIZE) {
                code_size++;
            }
        }
        /* New dict entry = dict[prev] + entry_first (only when prev exists). */
        if (prev_code >= 0 && dict_len < MAX_ENTRIES) {
            prefix[dict_len] = prev_code;
            last[dict_len] = entry_first;
            first[dict_len] = first[prev_code];
            dict_len++;
        }
        prev_code = (long)code;
    }

    if (output.failed) {
        goto done;
    }
    /* Trim padding artefacts to the recorded original length. */
    if (output.len > original_length) {
        output.len = original_length;
    }
    *out = output.data;
    *out_len = output.len;
    output.data = NULL; /* transferred to caller */
    ok = 1;

done:
    free(prefix);
    free(last);
    free(first);
    free(stack);
    free(output.data);
    return ok;
}
