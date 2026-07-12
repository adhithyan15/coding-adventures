/*
 * lz78.c — implementation of LZ78 (see lz78.h). A faithful port of the Rust
 * `lz78` crate: the same trie-cursor encoder, parallel-dictionary decoder, and
 * CMP01 wire format.
 */
#include "lz78.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* memcpy */

/* ===================================================================== *
 *  TrieCursor
 * ===================================================================== */

typedef struct {
    unsigned char byte;
    size_t child;
} Edge;

typedef struct {
    unsigned short dict_id;
    Edge *edges;
    size_t edge_count, edge_cap;
} CNode;

struct Lz78TrieCursor {
    CNode *arena;
    size_t arena_count, arena_cap;
    size_t current;
};

static int grow_arena(Lz78TrieCursor *c) {
    if (c->arena_count == c->arena_cap) {
        size_t ncap = c->arena_cap ? c->arena_cap * 2 : 8;
        CNode *na;
        if (c->arena_cap > (SIZE_MAX / sizeof(CNode)) / 2) {
            return 0;
        }
        na = realloc(c->arena, ncap * sizeof *na);
        if (!na) {
            return 0;
        }
        c->arena = na;
        c->arena_cap = ncap;
    }
    return 1;
}

Lz78TrieCursor *lz78_cursor_new(void) {
    Lz78TrieCursor *c = malloc(sizeof *c);
    if (!c) {
        return NULL;
    }
    c->arena = NULL;
    c->arena_count = 0;
    c->arena_cap = 0;
    c->current = 0;
    if (!grow_arena(c)) {
        free(c);
        return NULL;
    }
    c->arena[0].dict_id = 0; /* root */
    c->arena[0].edges = NULL;
    c->arena[0].edge_count = 0;
    c->arena[0].edge_cap = 0;
    c->arena_count = 1;
    return c;
}

void lz78_cursor_free(Lz78TrieCursor *c) {
    size_t i;
    if (!c) {
        return;
    }
    for (i = 0; i < c->arena_count; i++) {
        free(c->arena[i].edges);
    }
    free(c->arena);
    free(c);
}

int lz78_cursor_step(Lz78TrieCursor *c, unsigned char byte) {
    CNode *node = &c->arena[c->current];
    size_t i;
    for (i = 0; i < node->edge_count; i++) {
        if (node->edges[i].byte == byte) {
            c->current = node->edges[i].child;
            return 1;
        }
    }
    return 0;
}

int lz78_cursor_insert(Lz78TrieCursor *c, unsigned char byte,
                       unsigned short dict_id) {
    size_t new_idx;
    CNode *cur;
    if (!grow_arena(c)) { /* may realloc the arena */
        return 0;
    }
    new_idx = c->arena_count;
    c->arena[new_idx].dict_id = dict_id;
    c->arena[new_idx].edges = NULL;
    c->arena[new_idx].edge_count = 0;
    c->arena[new_idx].edge_cap = 0;
    c->arena_count++;

    cur = &c->arena[c->current]; /* arena is stable now */
    if (cur->edge_count == cur->edge_cap) {
        size_t ncap = cur->edge_cap ? cur->edge_cap * 2 : 4;
        Edge *ne;
        if (cur->edge_cap > (SIZE_MAX / sizeof(Edge)) / 2) {
            c->arena_count--; /* undo the node we just added */
            return 0;
        }
        ne = realloc(cur->edges, ncap * sizeof *ne);
        if (!ne) {
            c->arena_count--;
            return 0;
        }
        cur->edges = ne;
        cur->edge_cap = ncap;
    }
    cur->edges[cur->edge_count].byte = byte;
    cur->edges[cur->edge_count].child = new_idx;
    cur->edge_count++;
    return 1;
}

void lz78_cursor_reset(Lz78TrieCursor *c) { c->current = 0; }

unsigned short lz78_cursor_dict_id(const Lz78TrieCursor *c) {
    return c->arena[c->current].dict_id;
}

int lz78_cursor_at_root(const Lz78TrieCursor *c) { return c->current == 0; }

/* ===================================================================== *
 *  Growable buffers
 * ===================================================================== */

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
    Lz78Token *data;
    size_t count, cap;
    int ok;
} TokBuf;

static void tb_init(TokBuf *t) {
    t->data = NULL;
    t->count = 0;
    t->cap = 0;
    t->ok = 1;
}
static void tb_push(TokBuf *t, Lz78Token tok) {
    if (!t->ok) {
        return;
    }
    if (t->count == t->cap) {
        size_t nc = t->cap ? t->cap * 2 : 16;
        Lz78Token *nd;
        if (t->cap > (SIZE_MAX / sizeof(Lz78Token)) / 2) {
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

/* ===================================================================== *
 *  Encode / decode
 * ===================================================================== */

Lz78Status lz78_encode(const unsigned char *data, size_t len,
                       size_t max_dict_size, Lz78Token **out_tokens,
                       size_t *out_count) {
    Lz78TrieCursor *cur;
    TokBuf tokens;
    unsigned next_id = 1;
    size_t i;

    *out_tokens = NULL;
    *out_count = 0;
    cur = lz78_cursor_new();
    if (!cur) {
        return LZ78_ERR_ALLOC;
    }
    tb_init(&tokens);

    for (i = 0; i < len; i++) {
        unsigned char byte = data[i];
        if (!lz78_cursor_step(cur, byte)) {
            Lz78Token tok;
            tok.dict_index = lz78_cursor_dict_id(cur);
            tok.next_char = byte;
            tb_push(&tokens, tok);
            if (!tokens.ok) {
                goto oom;
            }
            if ((size_t)next_id < max_dict_size) {
                if (!lz78_cursor_insert(cur, byte, (unsigned short)next_id)) {
                    goto oom;
                }
                next_id++;
            }
            lz78_cursor_reset(cur);
        }
    }
    if (!lz78_cursor_at_root(cur)) {
        Lz78Token tok;
        tok.dict_index = lz78_cursor_dict_id(cur);
        tok.next_char = 0;
        tb_push(&tokens, tok);
        if (!tokens.ok) {
            goto oom;
        }
    }

    lz78_cursor_free(cur);
    *out_tokens = tokens.data;
    *out_count = tokens.count;
    return LZ78_OK;
oom:
    free(tokens.data);
    lz78_cursor_free(cur);
    return LZ78_ERR_ALLOC;
}

typedef struct {
    unsigned short parent;
    unsigned char byte;
} DictEntry;

/* Append the byte sequence for dictionary entry `index` (in order) to `out`.
 * Bounds- and cycle-guarded against malformed input. Returns 0 on OOM. */
static int reconstruct_append(ByteBuf *out, const DictEntry *table,
                              size_t table_size, unsigned short index) {
    size_t start = out->len;
    size_t idx = index;
    size_t iterations = 0;
    unsigned char *p;
    size_t a, b;
    while (idx != 0) {
        if (idx >= table_size) {
            break; /* out-of-range reference (malformed) */
        }
        if (iterations++ > table_size) {
            break; /* cyclic reference (malformed) */
        }
        bb_push(out, table[idx].byte);
        if (!out->ok) {
            return 0;
        }
        idx = table[idx].parent;
    }
    /* Reverse out[start..len] (we appended in leaf-to-root order). */
    p = out->data;
    a = start;
    b = out->len;
    while (a + 1 < b) {
        unsigned char t = p[a];
        p[a] = p[b - 1];
        p[b - 1] = t;
        a++;
        b--;
    }
    return 1;
}

Lz78Status lz78_decode(const Lz78Token *tokens, size_t token_count,
                       int has_original_length, size_t original_length,
                       unsigned char **out_data, size_t *out_len) {
    ByteBuf out;
    DictEntry *table = NULL;
    size_t table_count = 0, table_cap = 0, t;

    *out_data = NULL;
    *out_len = 0;
    bb_init(&out);

    /* table[0] is the root sentinel. */
    table_cap = 16;
    table = malloc(table_cap * sizeof *table);
    if (!table) {
        return LZ78_ERR_ALLOC;
    }
    table[0].parent = 0;
    table[0].byte = 0;
    table_count = 1;

    for (t = 0; t < token_count; t++) {
        if (!reconstruct_append(&out, table, table_count, tokens[t].dict_index)) {
            goto oom;
        }
        if (!has_original_length || out.len < original_length) {
            bb_push(&out, tokens[t].next_char);
            if (!out.ok) {
                goto oom;
            }
        }
        if (table_count == table_cap) {
            size_t ncap;
            DictEntry *nt;
            if (table_cap > (SIZE_MAX / sizeof(DictEntry)) / 2) {
                goto oom;
            }
            ncap = table_cap * 2;
            nt = realloc(table, ncap * sizeof *nt);
            if (!nt) {
                goto oom;
            }
            table = nt;
            table_cap = ncap;
        }
        table[table_count].parent = tokens[t].dict_index;
        table[table_count].byte = tokens[t].next_char;
        table_count++;

        /* Once we already hold `original_length` bytes, any later output would
         * be truncated away — stop (same result, bounded memory). */
        if (has_original_length && out.len >= original_length) {
            break;
        }
    }

    free(table);
    if (has_original_length && out.len > original_length) {
        out.len = original_length; /* truncate */
    }
    *out_data = out.data;
    *out_len = out.len;
    return LZ78_OK;
oom:
    free(table);
    free(out.data);
    return LZ78_ERR_ALLOC;
}

/* ===================================================================== *
 *  Wire format
 * ===================================================================== */

static void put_be32(ByteBuf *b, unsigned long v) {
    bb_push(b, (unsigned char)((v >> 24) & 0xFF));
    bb_push(b, (unsigned char)((v >> 16) & 0xFF));
    bb_push(b, (unsigned char)((v >> 8) & 0xFF));
    bb_push(b, (unsigned char)(v & 0xFF));
}

Lz78Status lz78_compress(const unsigned char *data, size_t len,
                         size_t max_dict_size, unsigned char **out,
                         size_t *out_len) {
    Lz78Token *tokens = NULL;
    size_t count = 0, i;
    Lz78Status st;
    ByteBuf b;

    *out = NULL;
    *out_len = 0;
    st = lz78_encode(data, len, max_dict_size, &tokens, &count);
    if (st != LZ78_OK) {
        return st;
    }
    bb_init(&b);
    put_be32(&b, (unsigned long)(len & 0xFFFFFFFFul));
    put_be32(&b, (unsigned long)(count & 0xFFFFFFFFul));
    for (i = 0; i < count; i++) {
        bb_push(&b, (unsigned char)((tokens[i].dict_index >> 8) & 0xFF));
        bb_push(&b, (unsigned char)(tokens[i].dict_index & 0xFF));
        bb_push(&b, tokens[i].next_char);
        bb_push(&b, 0x00);
    }
    free(tokens);
    if (!b.ok) {
        free(b.data);
        return LZ78_ERR_ALLOC;
    }
    *out = b.data;
    *out_len = b.len;
    return LZ78_OK;
}

Lz78Status lz78_decompress(const unsigned char *data, size_t len,
                           unsigned char **out, size_t *out_len) {
    size_t orig_len, token_count, avail, nread, i;
    Lz78Token *tokens = NULL;
    Lz78Status st;

    *out = NULL;
    *out_len = 0;
    if (len < 8) {
        /* Nothing to decode: original length 0. */
        return lz78_decode(NULL, 0, 1, 0, out, out_len);
    }
    orig_len = ((size_t)data[0] << 24) | ((size_t)data[1] << 16) |
               ((size_t)data[2] << 8) | (size_t)data[3];
    token_count = ((size_t)data[4] << 24) | ((size_t)data[5] << 16) |
                  ((size_t)data[6] << 8) | (size_t)data[7];
    avail = (len - 8) / 4;
    nread = token_count < avail ? token_count : avail;

    if (nread > 0) {
        tokens = malloc(nread * sizeof *tokens); /* nread <= (len-8)/4 */
        if (!tokens) {
            return LZ78_ERR_ALLOC;
        }
        for (i = 0; i < nread; i++) {
            size_t base = 8 + i * 4;
            tokens[i].dict_index =
                (unsigned short)(((unsigned)data[base] << 8) | data[base + 1]);
            tokens[i].next_char = data[base + 2];
        }
    }
    st = lz78_decode(tokens, nread, 1, orig_len, out, out_len);
    free(tokens);
    return st;
}
