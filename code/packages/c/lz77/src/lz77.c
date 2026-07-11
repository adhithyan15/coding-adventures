/*
 * lz77.c — implementation of LZ77 encode/decode/serialise. Ported from the Rust
 * `lz77` crate; the match search, token layout, and serialisation all match.
 */
#include "lz77.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* memcpy */

/* --- a growable byte buffer -------------------------------------------------
 * `failed` latches on allocation failure so callers check once at the end. */
typedef struct {
    uint8_t *data;
    size_t len;
    size_t cap;
    int failed;
} bytebuf;

static void bb_grow(bytebuf *b, size_t need_total) {
    size_t new_cap;
    uint8_t *grown;
    if (need_total <= b->cap) {
        return;
    }
    new_cap = b->cap == 0 ? 64 : b->cap;
    while (new_cap < need_total) {
        if (new_cap > SIZE_MAX / 2) {
            b->failed = 1;
            return;
        }
        new_cap *= 2;
    }
    grown = (uint8_t *)realloc(b->data, new_cap);
    if (grown == NULL) {
        b->failed = 1;
        return;
    }
    b->data = grown;
    b->cap = new_cap;
}

static void bb_push(bytebuf *b, uint8_t byte) {
    if (b->failed) {
        return;
    }
    bb_grow(b, b->len + 1);
    if (!b->failed) {
        b->data[b->len++] = byte;
    }
}

/* find_longest_match — scan the window [cursor-window, cursor) for the longest
 * run matching the lookahead at `cursor`, leaving at least one byte for the
 * token's next_char. Writes the match into *best_offset / *best_length. */
static void find_longest_match(const uint8_t *data, size_t len, size_t cursor,
                               size_t window_size, size_t max_match,
                               size_t *best_offset, size_t *best_length) {
    size_t search_start = cursor > window_size ? cursor - window_size : 0;
    size_t lookahead_end = cursor + max_match;
    size_t pos;
    if (lookahead_end > len - 1) {
        lookahead_end = len - 1; /* the last byte is reserved as a next_char */
    }
    *best_offset = 0;
    *best_length = 0;
    for (pos = search_start; pos < cursor; pos++) {
        size_t length = 0;
        while (cursor + length < lookahead_end &&
               data[pos + length] == data[cursor + length]) {
            length++;
        }
        if (length > *best_length) {
            *best_length = length;
            *best_offset = cursor - pos;
        }
    }
}

int lz77_encode(const uint8_t *data, size_t len, size_t window_size,
                size_t max_match, size_t min_match, lz77_token **tokens_out,
                size_t *count_out) {
    lz77_token *tokens = NULL;
    size_t count = 0, cap = 0;
    size_t cursor = 0;

    while (cursor < len) {
        lz77_token tok;
        if (cursor == len - 1) {
            tok.offset = 0;
            tok.length = 0;
            tok.next_char = data[cursor];
            cursor += 1;
        } else {
            size_t offset, length;
            find_longest_match(data, len, cursor, window_size, max_match,
                               &offset, &length);
            if (length >= min_match) {
                tok.offset = (uint16_t)offset;
                tok.length = (uint8_t)length;
                tok.next_char = data[cursor + length];
                cursor += length + 1;
            } else {
                tok.offset = 0;
                tok.length = 0;
                tok.next_char = data[cursor];
                cursor += 1;
            }
        }
        if (count == cap) {
            size_t new_cap = cap == 0 ? 16 : cap * 2;
            lz77_token *grown;
            if (cap > SIZE_MAX / 2 || new_cap > SIZE_MAX / sizeof(lz77_token)) {
                free(tokens);
                return 0;
            }
            grown = (lz77_token *)realloc(tokens, new_cap * sizeof(lz77_token));
            if (grown == NULL) {
                free(tokens);
                return 0;
            }
            tokens = grown;
            cap = new_cap;
        }
        tokens[count++] = tok;
    }
    *tokens_out = tokens;
    *count_out = count;
    return 1;
}

int lz77_decode(const lz77_token *tokens, size_t count, const uint8_t *initial,
                size_t initial_len, uint8_t **out, size_t *out_len) {
    bytebuf b;
    size_t t;
    b.data = NULL;
    b.len = 0;
    b.cap = 0;
    b.failed = 0;

    if (initial_len > 0) {
        bb_grow(&b, initial_len);
        if (!b.failed) {
            memcpy(b.data, initial, initial_len);
            b.len = initial_len;
        }
    }
    for (t = 0; t < count && !b.failed; t++) {
        if (tokens[t].length > 0) {
            size_t start = b.len - tokens[t].offset;
            size_t i;
            for (i = 0; i < tokens[t].length; i++) {
                /* Read the source byte BEFORE pushing (push may realloc). */
                uint8_t byte = b.data[start + i];
                bb_push(&b, byte);
            }
        }
        bb_push(&b, tokens[t].next_char);
    }
    if (b.failed) {
        free(b.data);
        return 0;
    }
    *out = b.data;
    *out_len = b.len;
    return 1;
}

int lz77_serialise(const lz77_token *tokens, size_t count, uint8_t **out,
                   size_t *out_len) {
    size_t total;
    uint8_t *buf;
    size_t i;
    /* 4-byte header + 4 bytes per token; guard the multiply/addition. */
    if (count > (SIZE_MAX - 4) / 4) {
        return 0;
    }
    total = 4 + count * 4;
    buf = (uint8_t *)malloc(total);
    if (buf == NULL) {
        return 0;
    }
    buf[0] = (uint8_t)(count >> 24);
    buf[1] = (uint8_t)(count >> 16);
    buf[2] = (uint8_t)(count >> 8);
    buf[3] = (uint8_t)(count);
    for (i = 0; i < count; i++) {
        size_t base = 4 + i * 4;
        buf[base] = (uint8_t)(tokens[i].offset >> 8);
        buf[base + 1] = (uint8_t)(tokens[i].offset);
        buf[base + 2] = tokens[i].length;
        buf[base + 3] = tokens[i].next_char;
    }
    *out = buf;
    *out_len = total;
    return 1;
}

int lz77_deserialise(const uint8_t *data, size_t len, lz77_token **tokens_out,
                     size_t *count_out) {
    size_t declared, actual, i;
    lz77_token *tokens;
    if (len < 4) {
        *tokens_out = NULL;
        *count_out = 0;
        return 1;
    }
    declared = ((size_t)data[0] << 24) | ((size_t)data[1] << 16) |
               ((size_t)data[2] << 8) | ((size_t)data[3]);
    /* Only trust as many tokens as the buffer actually holds. */
    actual = (len - 4) / 4;
    if (actual > declared) {
        actual = declared;
    }
    if (actual == 0) {
        *tokens_out = NULL;
        *count_out = 0;
        return 1;
    }
    tokens = (lz77_token *)malloc(actual * sizeof(lz77_token));
    if (tokens == NULL) {
        return 0;
    }
    for (i = 0; i < actual; i++) {
        size_t base = 4 + i * 4;
        tokens[i].offset = (uint16_t)(((uint16_t)data[base] << 8) | data[base + 1]);
        tokens[i].length = data[base + 2];
        tokens[i].next_char = data[base + 3];
    }
    *tokens_out = tokens;
    *count_out = actual;
    return 1;
}

int lz77_compress(const uint8_t *data, size_t len, size_t window_size,
                  size_t max_match, size_t min_match, uint8_t **out,
                  size_t *out_len) {
    lz77_token *tokens = NULL;
    size_t count = 0;
    int ok;
    if (!lz77_encode(data, len, window_size, max_match, min_match, &tokens,
                     &count)) {
        return 0;
    }
    ok = lz77_serialise(tokens, count, out, out_len);
    free(tokens);
    return ok;
}

int lz77_decompress(const uint8_t *data, size_t len, uint8_t **out,
                    size_t *out_len) {
    lz77_token *tokens = NULL;
    size_t count = 0;
    int ok;
    if (!lz77_deserialise(data, len, &tokens, &count)) {
        return 0;
    }
    ok = lz77_decode(tokens, count, NULL, 0, out, out_len);
    free(tokens);
    return ok;
}
