/*
 * lzss.c — implementation of LZSS (see lzss.h). A faithful port of the Rust
 * `lzss` crate: the same greedy longest-match encoder, overlap-safe decoder,
 * and CMP02 wire format.
 */
#include "lzss.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* memcpy */

/* ---- growable buffers -------------------------------------------------- */

typedef struct {
    unsigned char *data;
    size_t len, cap;
    int ok;
} ByteBuf;

static void bb_init(ByteBuf *b) {
    b->data = NULL;
    b->len = 0;
    b->cap = 0;
    b->ok = 1;
}
static int bb_reserve(ByteBuf *b, size_t extra) {
    size_t need, nc;
    if (!b->ok) {
        return 0;
    }
    if (extra > SIZE_MAX - b->len) {
        b->ok = 0;
        return 0;
    }
    need = b->len + extra;
    if (need <= b->cap) {
        return 1;
    }
    nc = b->cap ? b->cap : 32;
    while (nc < need) {
        if (nc > SIZE_MAX / 2) {
            nc = need;
            break;
        }
        nc *= 2;
    }
    {
        unsigned char *nd = realloc(b->data, nc);
        if (!nd) {
            b->ok = 0;
            return 0;
        }
        b->data = nd;
        b->cap = nc;
    }
    return 1;
}
static void bb_push(ByteBuf *b, unsigned char c) {
    if (bb_reserve(b, 1)) {
        b->data[b->len++] = c;
    }
}

typedef struct {
    LzssToken *data;
    size_t count, cap;
    int ok;
} TokBuf;

static void tb_init(TokBuf *t) {
    t->data = NULL;
    t->count = 0;
    t->cap = 0;
    t->ok = 1;
}
static void tb_push(TokBuf *t, LzssToken tok) {
    if (!t->ok) {
        return;
    }
    if (t->count == t->cap) {
        size_t nc = t->cap ? t->cap * 2 : 16;
        LzssToken *nd;
        if (t->cap > (SIZE_MAX / sizeof(LzssToken)) / 2) {
            t->ok = 0;
            return;
        }
        nd = realloc(t->data, nc * sizeof *nd);
        if (!nd) {
            t->ok = 0;
            return;
        }
        t->data = nd;
        t->cap = nc;
    }
    t->data[t->count++] = tok;
}

/* ---- encode ----------------------------------------------------------- */

/* Longest match for data[cursor..] in data[win_start..cursor]. Matches may
 * overlap the cursor (run-length as a degenerate case). */
static void find_longest_match(const unsigned char *data, size_t len,
                               size_t cursor, size_t win_start, size_t max_match,
                               unsigned short *best_off, unsigned char *best_len) {
    size_t blen = 0, boff = 0, pos;
    size_t lookahead_end = cursor + max_match;
    if (lookahead_end > len) {
        lookahead_end = len;
    }
    for (pos = win_start; pos < cursor; pos++) {
        size_t l = 0;
        while (cursor + l < lookahead_end && data[pos + l] == data[cursor + l]) {
            l++;
        }
        if (l > blen) {
            blen = l;
            boff = cursor - pos;
        }
    }
    *best_off = (unsigned short)boff; /* truncates like the crate for huge windows */
    *best_len = (unsigned char)blen;  /* max_match <= 255, so this fits */
}

LzssStatus lzss_encode(const unsigned char *data, size_t len,
                       size_t window_size, size_t max_match, size_t min_match,
                       LzssToken **out_tokens, size_t *out_count) {
    TokBuf tokens;
    size_t cursor = 0;

    *out_tokens = NULL;
    *out_count = 0;
    tb_init(&tokens);

    while (cursor < len) {
        size_t win_start = cursor > window_size ? cursor - window_size : 0;
        unsigned short offset;
        unsigned char length;
        LzssToken tok;
        find_longest_match(data, len, cursor, win_start, max_match, &offset,
                           &length);
        if ((size_t)length >= min_match) {
            tok.is_match = 1;
            tok.literal = 0;
            tok.offset = offset;
            tok.length = length;
            tb_push(&tokens, tok);
            cursor += (size_t)length;
        } else {
            tok.is_match = 0;
            tok.literal = data[cursor];
            tok.offset = 0;
            tok.length = 0;
            tb_push(&tokens, tok);
            cursor += 1;
        }
        if (!tokens.ok) {
            free(tokens.data);
            return LZSS_ERR_ALLOC;
        }
    }

    *out_tokens = tokens.data;
    *out_count = tokens.count;
    return LZSS_OK;
}

/* ---- decode ----------------------------------------------------------- */

LzssStatus lzss_decode(const LzssToken *tokens, size_t count,
                       int has_original_length, size_t original_length,
                       unsigned char **out, size_t *out_len) {
    ByteBuf output;
    size_t t;

    *out = NULL;
    *out_len = 0;
    bb_init(&output);

    for (t = 0; t < count; t++) {
        if (!tokens[t].is_match) {
            bb_push(&output, tokens[t].literal);
        } else {
            size_t off = tokens[t].offset;
            /* Skip malformed matches (offset 0 or beyond the output). */
            if (off != 0 && off <= output.len) {
                size_t start = output.len - off;
                size_t i;
                for (i = 0; i < tokens[t].length; i++) {
                    bb_push(&output, output.data[start + i]);
                    if (!output.ok) {
                        break;
                    }
                }
            }
        }
        if (!output.ok) {
            free(output.data);
            return LZSS_ERR_ALLOC;
        }
        /* Once we hold `original_length` bytes, the rest would be truncated
         * away — stop (identical result, bounded memory). */
        if (has_original_length && output.len >= original_length) {
            break;
        }
    }

    if (has_original_length && output.len > original_length) {
        output.len = original_length;
    }
    *out = output.data;
    *out_len = output.len;
    return LZSS_OK;
}

/* ---- wire format ------------------------------------------------------ */

static void put_be32(ByteBuf *b, unsigned long v) {
    bb_push(b, (unsigned char)((v >> 24) & 0xFF));
    bb_push(b, (unsigned char)((v >> 16) & 0xFF));
    bb_push(b, (unsigned char)((v >> 8) & 0xFF));
    bb_push(b, (unsigned char)(v & 0xFF));
}

LzssStatus lzss_serialise(const LzssToken *tokens, size_t count,
                          size_t original_length, unsigned char **out,
                          size_t *out_len) {
    ByteBuf buf;
    size_t block_count = (count + 7) / 8; /* ceil(count / 8) */
    size_t blk;

    *out = NULL;
    *out_len = 0;
    bb_init(&buf);
    put_be32(&buf, (unsigned long)(original_length & 0xFFFFFFFFul));
    put_be32(&buf, (unsigned long)(block_count & 0xFFFFFFFFul));

    for (blk = 0; blk < block_count; blk++) {
        size_t base = blk * 8;
        size_t chunk = count - base < 8 ? count - base : 8;
        unsigned char flag = 0;
        size_t bit;
        for (bit = 0; bit < chunk; bit++) {
            if (tokens[base + bit].is_match) {
                flag |= (unsigned char)(1u << bit);
            }
        }
        bb_push(&buf, flag);
        for (bit = 0; bit < chunk; bit++) {
            const LzssToken *tk = &tokens[base + bit];
            if (tk->is_match) {
                bb_push(&buf, (unsigned char)((tk->offset >> 8) & 0xFF));
                bb_push(&buf, (unsigned char)(tk->offset & 0xFF));
                bb_push(&buf, tk->length);
            } else {
                bb_push(&buf, tk->literal);
            }
        }
    }

    if (!buf.ok) {
        free(buf.data);
        return LZSS_ERR_ALLOC;
    }
    *out = buf.data;
    *out_len = buf.len;
    return LZSS_OK;
}

LzssStatus lzss_deserialise(const unsigned char *data, size_t len,
                            LzssToken **out_tokens, size_t *out_count,
                            size_t *out_original_length) {
    TokBuf tokens;
    size_t orig_len, block_count, max_possible, pos = 8, blk;

    *out_tokens = NULL;
    *out_count = 0;
    *out_original_length = 0;
    if (len < 8) {
        return LZSS_OK; /* empty */
    }
    orig_len = ((size_t)data[0] << 24) | ((size_t)data[1] << 16) |
               ((size_t)data[2] << 8) | (size_t)data[3];
    block_count = ((size_t)data[4] << 24) | ((size_t)data[5] << 16) |
                  ((size_t)data[6] << 8) | (size_t)data[7];
    *out_original_length = orig_len;

    /* Cap block_count to the payload size (1 byte minimum per block). */
    max_possible = len - 8;
    if (block_count > max_possible) {
        block_count = max_possible;
    }

    tb_init(&tokens);
    for (blk = 0; blk < block_count; blk++) {
        unsigned char flag;
        int bit;
        if (pos >= len) {
            break;
        }
        flag = data[pos];
        pos++;
        for (bit = 0; bit < 8; bit++) {
            LzssToken tok;
            if (pos >= len) {
                break;
            }
            if (flag & (unsigned char)(1u << bit)) {
                if (pos + 3 > len) {
                    break;
                }
                tok.is_match = 1;
                tok.literal = 0;
                tok.offset =
                    (unsigned short)(((unsigned)data[pos] << 8) | data[pos + 1]);
                tok.length = data[pos + 2];
                pos += 3;
            } else {
                tok.is_match = 0;
                tok.literal = data[pos];
                tok.offset = 0;
                tok.length = 0;
                pos += 1;
            }
            tb_push(&tokens, tok);
            if (!tokens.ok) {
                free(tokens.data);
                return LZSS_ERR_ALLOC;
            }
        }
    }

    *out_tokens = tokens.data;
    *out_count = tokens.count;
    return LZSS_OK;
}

/* ---- one-shot --------------------------------------------------------- */

LzssStatus lzss_compress(const unsigned char *data, size_t len,
                         unsigned char **out, size_t *out_len) {
    LzssToken *tokens = NULL;
    size_t count = 0;
    LzssStatus st;

    *out = NULL;
    *out_len = 0;
    st = lzss_encode(data, len, LZSS_DEFAULT_WINDOW_SIZE, LZSS_DEFAULT_MAX_MATCH,
                     LZSS_DEFAULT_MIN_MATCH, &tokens, &count);
    if (st != LZSS_OK) {
        return st;
    }
    st = lzss_serialise(tokens, count, len, out, out_len);
    free(tokens);
    return st;
}

LzssStatus lzss_decompress(const unsigned char *data, size_t len,
                           unsigned char **out, size_t *out_len) {
    LzssToken *tokens = NULL;
    size_t count = 0, orig_len = 0;
    LzssStatus st;

    *out = NULL;
    *out_len = 0;
    st = lzss_deserialise(data, len, &tokens, &count, &orig_len);
    if (st != LZSS_OK) {
        return st;
    }
    st = lzss_decode(tokens, count, 1, orig_len, out, out_len);
    free(tokens);
    return st;
}
