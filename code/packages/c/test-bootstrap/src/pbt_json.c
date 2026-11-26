/*
 * pbt_json.c -- strict JSON reader (VER05i section 8).
 *
 * A recursive-descent parser whose recursion is bounded by PBT_JSON_MAX_DEPTH,
 * so no input can overflow the C stack.  Every other resource is capped too:
 * the document size is checked before parsing starts, the value count is
 * checked as values are created, and the duplicate-key check sorts (O(n log n))
 * instead of comparing every pair (an object with 100,000 keys must not cost
 * 10^10 comparisons).
 *
 * Memory safety: every node is valid (type NULL, no storage) BEFORE its
 * contents are parsed, so on any error the partially built tree can be freed
 * by pbt_json_free without special cases.
 */
#include "pbt_json.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "pbt_internal.h"

typedef struct parser {
    const unsigned char *t;
    size_t len;
    size_t pos;
    size_t nodes;
    char *err;
    size_t errcap;
    int failed;
} parser;

static int perr(parser *p, const char *msg)
{
    if (!p->failed && p->err != NULL && p->errcap > 0) {
        (void)snprintf(p->err, p->errcap, "%s at byte %lu", msg, (unsigned long)p->pos);
    }
    p->failed = 1;
    return 1;
}

static void node_init(pbt_json *n)
{
    memset(n, 0, sizeof *n);
    n->type = PBT_J_NULL;
}

static void free_node(pbt_json *n)
{
    size_t i;
    pbt_free(n->s);
    for (i = 0; i < n->n; i++) {
        free_node(&n->items[i]);
        if (n->keys != NULL) {
            pbt_free(n->keys[i]);
        }
    }
    pbt_free(n->items);
    pbt_free(n->keys);
    pbt_free(n->keylens);
    node_init(n);
}

void pbt_json_free(pbt_json *root)
{
    if (root != NULL) {
        free_node(root);
        pbt_free(root);
    }
}

static void skip_ws(parser *p)
{
    while (p->pos < p->len) {
        unsigned char c = p->t[p->pos];
        if (c == ' ' || c == '\t' || c == '\n' || c == '\r') {
            p->pos++;
        } else {
            break;
        }
    }
}

/* ---- strings ------------------------------------------------------------------ */

typedef struct sbuf {
    char *p;
    size_t n;
    size_t cap;
} sbuf;

static int sb_put(sbuf *b, const char *data, size_t len)
{
    int ok = 1;
    size_t need = pbt_add_size(b->n, len, &ok);
    need = pbt_add_size(need, 1, &ok);
    if (!ok) {
        return 1;
    }
    if (need > b->cap) {
        size_t cap = b->cap == 0 ? 32 : b->cap;
        char *np;
        while (cap < need) {
            cap = pbt_mul_size(cap, 2, &ok);
            if (!ok) {
                return 1;
            }
        }
        np = (char *)pbt_realloc(b->p, cap, 1);
        if (np == NULL) {
            return 1;
        }
        b->p = np;
        b->cap = cap;
    }
    memcpy(b->p + b->n, data, len);
    b->n += len;
    b->p[b->n] = '\0';
    return 0;
}

static int hexval(unsigned char c)
{
    if (c >= '0' && c <= '9') {
        return c - '0';
    }
    if (c >= 'a' && c <= 'f') {
        return c - 'a' + 10;
    }
    if (c >= 'A' && c <= 'F') {
        return c - 'A' + 10;
    }
    return -1;
}

static int read_hex4(parser *p, unsigned *out)
{
    unsigned v = 0;
    int i;
    if (p->len - p->pos < 4) {
        return perr(p, "truncated \\u escape");
    }
    for (i = 0; i < 4; i++) {
        int h = hexval(p->t[p->pos + (size_t)i]);
        if (h < 0) {
            return perr(p, "bad \\u escape");
        }
        v = v * 16u + (unsigned)h;
    }
    p->pos += 4;
    *out = v;
    return 0;
}

static size_t encode_utf8(unsigned cp, char out[4])
{
    if (cp < 0x80u) {
        out[0] = (char)cp;
        return 1;
    }
    if (cp < 0x800u) {
        out[0] = (char)(0xC0u | (cp >> 6));
        out[1] = (char)(0x80u | (cp & 0x3Fu));
        return 2;
    }
    if (cp < 0x10000u) {
        out[0] = (char)(0xE0u | (cp >> 12));
        out[1] = (char)(0x80u | ((cp >> 6) & 0x3Fu));
        out[2] = (char)(0x80u | (cp & 0x3Fu));
        return 3;
    }
    out[0] = (char)(0xF0u | (cp >> 18));
    out[1] = (char)(0x80u | ((cp >> 12) & 0x3Fu));
    out[2] = (char)(0x80u | ((cp >> 6) & 0x3Fu));
    out[3] = (char)(0x80u | (cp & 0x3Fu));
    return 4;
}

/* Length of the valid UTF-8 sequence at s[0..avail), or 0 if invalid (strict: no overlongs, no surrogates, <= U+10FFFF). */
size_t pbt_utf8_len(const unsigned char *s, size_t avail)
{
    unsigned char c = s[0];
    if (c < 0x80) {
        return 1;
    }
    if (c >= 0xC2 && c <= 0xDF) {
        return avail >= 2 && (s[1] & 0xC0) == 0x80 ? 2 : 0;
    }
    if (c >= 0xE0 && c <= 0xEF) {
        unsigned char lo = c == 0xE0 ? 0xA0 : 0x80;
        unsigned char hi = c == 0xED ? 0x9F : 0xBF;
        return avail >= 3 && s[1] >= lo && s[1] <= hi && (s[2] & 0xC0) == 0x80 ? 3 : 0;
    }
    if (c >= 0xF0 && c <= 0xF4) {
        unsigned char lo = c == 0xF0 ? 0x90 : 0x80;
        unsigned char hi = c == 0xF4 ? 0x8F : 0xBF;
        return avail >= 4 && s[1] >= lo && s[1] <= hi && (s[2] & 0xC0) == 0x80 && (s[3] & 0xC0) == 0x80 ? 4 : 0;
    }
    return 0;
}

/* Parse a string starting at the opening quote; the decoded text goes to *out (heap, NUL terminated). */
static int parse_string(parser *p, char **out, size_t *outlen)
{
    sbuf b;
    b.p = NULL;
    b.n = 0;
    b.cap = 0;
    p->pos++; /* opening quote */
    for (;;) {
        unsigned char c;
        if (p->pos >= p->len) {
            pbt_free(b.p);
            return perr(p, "unterminated string");
        }
        c = p->t[p->pos];
        if (c == '"') {
            p->pos++;
            break;
        }
        if (c < 0x20) {
            pbt_free(b.p);
            return perr(p, "control character in string");
        }
        if (c == '\\') {
            unsigned cp;
            char enc[4];
            size_t el;
            p->pos++;
            if (p->pos >= p->len) {
                pbt_free(b.p);
                return perr(p, "truncated escape");
            }
            c = p->t[p->pos++];
            switch (c) {
            case '"': cp = '"'; break;
            case '\\': cp = '\\'; break;
            case '/': cp = '/'; break;
            case 'b': cp = 8; break;
            case 'f': cp = 12; break;
            case 'n': cp = 10; break;
            case 'r': cp = 13; break;
            case 't': cp = 9; break;
            case 'u':
                if (read_hex4(p, &cp)) {
                    pbt_free(b.p);
                    return 1;
                }
                if (cp >= 0xDC00u && cp <= 0xDFFFu) {
                    pbt_free(b.p);
                    return perr(p, "lone low surrogate");
                }
                if (cp >= 0xD800u && cp <= 0xDBFFu) {
                    unsigned lo;
                    if (p->len - p->pos < 2 || p->t[p->pos] != '\\' || p->t[p->pos + 1] != 'u') {
                        pbt_free(b.p);
                        return perr(p, "lone high surrogate");
                    }
                    p->pos += 2;
                    if (read_hex4(p, &lo)) {
                        pbt_free(b.p);
                        return 1;
                    }
                    if (lo < 0xDC00u || lo > 0xDFFFu) {
                        pbt_free(b.p);
                        return perr(p, "high surrogate not followed by a low surrogate");
                    }
                    cp = 0x10000u + ((cp - 0xD800u) << 10) + (lo - 0xDC00u);
                }
                break;
            default:
                pbt_free(b.p);
                return perr(p, "bad escape");
            }
            el = encode_utf8(cp, enc);
            if (sb_put(&b, enc, el)) {
                pbt_free(b.p);
                return perr(p, "out of memory");
            }
            continue;
        }
        {
            size_t l = pbt_utf8_len(p->t + p->pos, p->len - p->pos);
            if (l == 0) {
                pbt_free(b.p);
                return perr(p, "invalid UTF-8");
            }
            if (sb_put(&b, (const char *)p->t + p->pos, l)) {
                pbt_free(b.p);
                return perr(p, "out of memory");
            }
            p->pos += l;
        }
    }
    if (b.p == NULL && sb_put(&b, "", 0)) { /* empty string still needs a NUL */
        return perr(p, "out of memory");
    }
    *out = b.p;
    *outlen = b.n;
    return 0;
}

/* ---- numbers -------------------------------------------------------------------- */

static int is_digit(unsigned char c)
{
    return c >= '0' && c <= '9';
}

static int parse_number(parser *p, pbt_json *node)
{
    size_t start = p->pos;
    size_t i = p->pos;
    size_t n = p->len;
    const unsigned char *t = p->t;
    if (i < n && t[i] == '-') {
        i++;
    }
    if (i >= n || !is_digit(t[i])) {
        return perr(p, "bad number");
    }
    if (t[i] == '0') {
        i++;
        if (i < n && is_digit(t[i])) {
            p->pos = i;
            return perr(p, "leading zero in number");
        }
    } else {
        while (i < n && is_digit(t[i])) {
            i++;
        }
    }
    if (i < n && t[i] == '.') {
        i++;
        if (i >= n || !is_digit(t[i])) {
            p->pos = i;
            return perr(p, "bad fraction");
        }
        while (i < n && is_digit(t[i])) {
            i++;
        }
    }
    if (i < n && (t[i] == 'e' || t[i] == 'E')) {
        i++;
        if (i < n && (t[i] == '+' || t[i] == '-')) {
            i++;
        }
        if (i >= n || !is_digit(t[i])) {
            p->pos = i;
            return perr(p, "bad exponent");
        }
        while (i < n && is_digit(t[i])) {
            i++;
        }
    }
    node->s = (char *)pbt_alloc(i - start + 1, 1);
    if (node->s == NULL) {
        return perr(p, "out of memory");
    }
    memcpy(node->s, t + start, i - start);
    node->s[i - start] = '\0';
    node->slen = i - start;
    node->type = PBT_J_NUM;
    p->pos = i;
    return 0;
}

/* ---- containers ----------------------------------------------------------------- */

static int parse_value(parser *p, pbt_json *node, int depth);

/* Grow the item array (and the key arrays of an object) by one slot; the new slot is a null node. */
static int grow(pbt_json *n, size_t *cap, int is_obj)
{
    if (n->n == *cap) {
        size_t ncap = *cap == 0 ? 4 : *cap;
        int ok = 1;
        pbt_json *items;
        if (*cap != 0) {
            ncap = pbt_mul_size(*cap, 2, &ok);
            if (!ok) {
                return 1;
            }
        }
        items = (pbt_json *)pbt_realloc(n->items, ncap, sizeof *items);
        if (items == NULL) {
            return 1;
        }
        n->items = items;
        if (is_obj) {
            char **keys = (char **)pbt_realloc(n->keys, ncap, sizeof *keys);
            size_t *kl;
            if (keys == NULL) {
                return 1;
            }
            n->keys = keys;
            kl = (size_t *)pbt_realloc(n->keylens, ncap, sizeof *kl);
            if (kl == NULL) {
                return 1;
            }
            n->keylens = kl;
        }
        *cap = ncap;
    }
    node_init(&n->items[n->n]);
    if (is_obj) {
        n->keys[n->n] = NULL;
        n->keylens[n->n] = 0;
    }
    n->n++;
    return 0;
}

typedef struct keyref {
    const char *p;
    size_t n;
} keyref;

static int cmp_keys(const void *a, const void *b)
{
    const keyref *x = (const keyref *)a;
    const keyref *y = (const keyref *)b;
    size_t m = x->n < y->n ? x->n : y->n;
    int c = m == 0 ? 0 : memcmp(x->p, y->p, m);
    if (c != 0) {
        return c;
    }
    return x->n < y->n ? -1 : (x->n > y->n ? 1 : 0);
}

/* True if any two keys of the object are equal.  Sorts a key array: O(n log n), no global state. */
static int has_duplicate_key(const pbt_json *obj, int *oom)
{
    keyref *refs;
    size_t i;
    int dup = 0;
    if (obj->n < 2) {
        return 0;
    }
    refs = (keyref *)pbt_alloc(obj->n, sizeof *refs);
    if (refs == NULL) {
        *oom = 1;
        return 0;
    }
    for (i = 0; i < obj->n; i++) {
        refs[i].p = obj->keys[i];
        refs[i].n = obj->keylens[i];
    }
    qsort(refs, obj->n, sizeof *refs, cmp_keys);
    for (i = 1; i < obj->n && !dup; i++) {
        dup = cmp_keys(&refs[i - 1], &refs[i]) == 0;
    }
    pbt_free(refs);
    return dup;
}

static int parse_array(parser *p, pbt_json *node, int depth)
{
    size_t cap = 0;
    node->type = PBT_J_ARR;
    p->pos++; /* [ */
    skip_ws(p);
    if (p->pos < p->len && p->t[p->pos] == ']') {
        p->pos++;
        return 0;
    }
    for (;;) {
        if (grow(node, &cap, 0)) {
            return perr(p, "out of memory");
        }
        if (parse_value(p, &node->items[node->n - 1], depth + 1)) {
            return 1;
        }
        skip_ws(p);
        if (p->pos >= p->len) {
            return perr(p, "unterminated array");
        }
        if (p->t[p->pos] == ',') {
            p->pos++;
            skip_ws(p);
            continue;
        }
        if (p->t[p->pos] == ']') {
            p->pos++;
            return 0;
        }
        return perr(p, "expected , or ]");
    }
}

static int parse_object(parser *p, pbt_json *node, int depth)
{
    size_t cap = 0;
    int oom = 0;
    node->type = PBT_J_OBJ;
    p->pos++; /* { */
    skip_ws(p);
    if (p->pos < p->len && p->t[p->pos] == '}') {
        p->pos++;
        return 0;
    }
    for (;;) {
        size_t at;
        if (p->pos >= p->len || p->t[p->pos] != '"') {
            return perr(p, "expected a string key");
        }
        if (grow(node, &cap, 1)) {
            return perr(p, "out of memory");
        }
        at = node->n - 1;
        if (parse_string(p, &node->keys[at], &node->keylens[at])) {
            return 1;
        }
        skip_ws(p);
        if (p->pos >= p->len || p->t[p->pos] != ':') {
            return perr(p, "expected :");
        }
        p->pos++;
        skip_ws(p);
        if (parse_value(p, &node->items[at], depth + 1)) {
            return 1;
        }
        skip_ws(p);
        if (p->pos >= p->len) {
            return perr(p, "unterminated object");
        }
        if (p->t[p->pos] == ',') {
            p->pos++;
            skip_ws(p);
            continue;
        }
        if (p->t[p->pos] == '}') {
            p->pos++;
            break;
        }
        return perr(p, "expected , or }");
    }
    if (has_duplicate_key(node, &oom)) {
        return perr(p, "duplicate object key");
    }
    if (oom) {
        return perr(p, "out of memory");
    }
    return 0;
}

static int literal(parser *p, const char *word, size_t wl)
{
    if (p->len - p->pos < wl || memcmp(p->t + p->pos, word, wl) != 0) {
        return perr(p, "bad literal");
    }
    p->pos += wl;
    return 0;
}

static int parse_value(parser *p, pbt_json *node, int depth)
{
    unsigned char c;
    node_init(node);
    if (depth > PBT_JSON_MAX_DEPTH) {
        return perr(p, "nesting too deep");
    }
    if (++p->nodes > PBT_JSON_MAX_NODES) {
        return perr(p, "too many values");
    }
    skip_ws(p);
    if (p->pos >= p->len) {
        return perr(p, "unexpected end of input");
    }
    c = p->t[p->pos];
    switch (c) {
    case '{':
        return parse_object(p, node, depth);
    case '[':
        return parse_array(p, node, depth);
    case '"':
        node->type = PBT_J_STR;
        return parse_string(p, &node->s, &node->slen);
    case 't':
        node->type = PBT_J_BOOL;
        node->b = 1;
        return literal(p, "true", 4);
    case 'f':
        node->type = PBT_J_BOOL;
        node->b = 0;
        return literal(p, "false", 5);
    case 'n':
        return literal(p, "null", 4);
    default:
        if (c == '-' || is_digit(c)) {
            return parse_number(p, node);
        }
        return perr(p, "unexpected character");
    }
}

int pbt_json_parse(const char *text, size_t len, pbt_json **out, char *err, size_t errcap)
{
    parser p;
    pbt_json *root;
    *out = NULL;
    if (err != NULL && errcap > 0) {
        err[0] = '\0';
    }
    if (len > PBT_JSON_MAX_BYTES) {
        if (err != NULL && errcap > 0) {
            (void)snprintf(err, errcap, "document of %lu bytes exceeds the limit", (unsigned long)len);
        }
        return 1;
    }
    p.t = (const unsigned char *)text;
    p.len = len;
    p.pos = 0;
    p.nodes = 0;
    p.err = err;
    p.errcap = errcap;
    p.failed = 0;
    root = (pbt_json *)pbt_alloc(1, sizeof *root);
    if (root == NULL) {
        if (err != NULL && errcap > 0) {
            (void)snprintf(err, errcap, "out of memory");
        }
        return 1;
    }
    node_init(root);
    if (parse_value(&p, root, 1)) {
        pbt_json_free(root);
        return 1;
    }
    skip_ws(&p);
    if (p.pos != p.len) {
        (void)perr(&p, "trailing data");
        pbt_json_free(root);
        return 1;
    }
    *out = root;
    return 0;
}

/* ---- helpers ---------------------------------------------------------------------- */

static int clone_node(pbt_json *dst, const pbt_json *src, int depth)
{
    size_t i;
    node_init(dst);
    if (depth > PBT_JSON_MAX_DEPTH + 1) {
        return 1;
    }
    dst->type = src->type;
    dst->b = src->b;
    if (src->s != NULL) {
        dst->s = (char *)pbt_alloc(src->slen + 1, 1);
        if (dst->s == NULL) {
            return 1;
        }
        memcpy(dst->s, src->s, src->slen);
        dst->s[src->slen] = '\0';
        dst->slen = src->slen;
    }
    if (src->n > 0) {
        dst->items = (pbt_json *)pbt_alloc(src->n, sizeof *dst->items);
        if (dst->items == NULL) {
            return 1;
        }
        if (src->keys != NULL) {
            dst->keys = (char **)pbt_alloc(src->n, sizeof *dst->keys);
            dst->keylens = (size_t *)pbt_alloc(src->n, sizeof *dst->keylens);
            if (dst->keys == NULL || dst->keylens == NULL) {
                return 1;
            }
            for (i = 0; i < src->n; i++) {
                dst->keys[i] = NULL;
            }
        }
        for (i = 0; i < src->n; i++) {
            node_init(&dst->items[i]);
        }
        /* n is set only as slots become initialised, so a failure frees exactly what exists. */
        dst->n = src->n;
        for (i = 0; i < src->n; i++) {
            if (src->keys != NULL) {
                dst->keys[i] = (char *)pbt_alloc(src->keylens[i] + 1, 1);
                if (dst->keys[i] == NULL) {
                    return 1;
                }
                memcpy(dst->keys[i], src->keys[i], src->keylens[i]);
                dst->keys[i][src->keylens[i]] = '\0';
                dst->keylens[i] = src->keylens[i];
            }
            if (clone_node(&dst->items[i], &src->items[i], depth + 1)) {
                return 1;
            }
        }
    }
    return 0;
}

pbt_json *pbt_json_clone(const pbt_json *root)
{
    pbt_json *copy = (pbt_json *)pbt_alloc(1, sizeof *copy);
    if (copy == NULL) {
        return NULL;
    }
    if (clone_node(copy, root, 1)) {
        pbt_json_free(copy);
        return NULL;
    }
    return copy;
}

pbt_json *pbt_json_get(const pbt_json *obj, const char *key)
{
    size_t i;
    size_t kl = strlen(key);
    if (obj == NULL || obj->type != PBT_J_OBJ) {
        return NULL;
    }
    for (i = 0; i < obj->n; i++) {
        if (obj->keylens[i] == kl && memcmp(obj->keys[i], key, kl) == 0) {
            return &obj->items[i];
        }
    }
    return NULL;
}
