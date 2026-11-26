/*
 * pbt_val.c -- tagged values and canonical rendering (VER05i sections 4.1, 7.2).
 *
 * OWNERSHIP RULE
 * --------------
 * A pbt_val OWNS its heap storage (BYTES/STR buffer, LIST/TUPLE item array,
 * recursively).  `pbt_val_nil` makes a value that owns nothing; `pbt_val_free`
 * releases everything and leaves nil, so freeing twice is harmless.  A function
 * that can fail leaves its destination nil (valid, freeable).
 *
 * Collections use NULL storage when empty, so an empty list/byte string never
 * allocates.
 */
#include <string.h>

#include "pbt.h"
#include "pbt_internal.h"

/* Free is generous with depth (generator nesting is shallow); beyond it we leak rather than overflow the stack. */
#define FREE_DEPTH_LIMIT 256

void pbt_val_nil(pbt_val *v)
{
    memset(v, 0, sizeof *v);
    v->kind = PBT_V_NIL;
}

void pbt_val_set_bool(pbt_val *v, int b)
{
    pbt_val_nil(v);
    v->kind = PBT_V_BOOL;
    v->u.b = b != 0;
}

void pbt_val_set_int(pbt_val *v, const pbt_big *i)
{
    pbt_val_nil(v);
    v->kind = PBT_V_INT;
    v->u.i = *i;
}

void pbt_val_set_u64(pbt_val *v, uint64_t x)
{
    pbt_big b;
    pbt_big_set_u64(&b, x);
    pbt_val_set_int(v, &b);
}

void pbt_val_set_i64(pbt_val *v, int64_t x)
{
    pbt_big b;
    pbt_big_set_i64(&b, x);
    pbt_val_set_int(v, &b);
}

void pbt_val_set_ptr(pbt_val *v, void *p)
{
    pbt_val_nil(v);
    v->kind = PBT_V_PTR;
    v->u.ptr = p;
}

static int set_buffer(pbt_val *v, pbt_kind kind, const uint8_t *data, size_t n)
{
    uint8_t *p = NULL;
    pbt_val_nil(v);
    if (n > PBT_MAX_INPUT_BYTES) {
        return PBT_E_LIMIT;
    }
    if (n > 0) {
        p = (uint8_t *)pbt_alloc(n, 1);
        if (p == NULL) {
            return PBT_E_NOMEM;
        }
        if (data != NULL) {
            memcpy(p, data, n);
        } else {
            memset(p, 0, n);
        }
    }
    v->kind = kind;
    v->u.bytes.p = p;
    v->u.bytes.n = n;
    return 0;
}

int pbt_val_set_bytes(pbt_val *v, const uint8_t *data, size_t n)
{
    return set_buffer(v, PBT_V_BYTES, data, n);
}

int pbt_val_set_str(pbt_val *v, const char *s, size_t n)
{
    return set_buffer(v, PBT_V_STR, (const uint8_t *)s, n);
}

int pbt_val_set_list(pbt_val *v, pbt_kind kind, size_t n)
{
    pbt_val *items = NULL;
    size_t i;
    pbt_val_nil(v);
    if (n > PBT_MAX_LIST_LEN) {
        return PBT_E_LIMIT;
    }
    if (n > 0) {
        items = (pbt_val *)pbt_alloc(n, sizeof *items);
        if (items == NULL) {
            return PBT_E_NOMEM;
        }
        for (i = 0; i < n; i++) {
            pbt_val_nil(&items[i]);
        }
    }
    v->kind = kind;
    v->u.list.items = items;
    v->u.list.n = n;
    return 0;
}

static void free_depth(pbt_val *v, int depth)
{
    size_t i;
    if (v->kind == PBT_V_BYTES || v->kind == PBT_V_STR) {
        pbt_free(v->u.bytes.p);
    } else if (v->kind == PBT_V_LIST || v->kind == PBT_V_TUPLE) {
        if (depth < FREE_DEPTH_LIMIT) {
            for (i = 0; i < v->u.list.n; i++) {
                free_depth(&v->u.list.items[i], depth + 1);
            }
            pbt_free(v->u.list.items);
        }
    }
    pbt_val_nil(v);
}

void pbt_val_free(pbt_val *v)
{
    free_depth(v, 0);
}

static int clone_depth(pbt_val *dst, const pbt_val *src, int depth)
{
    size_t i;
    int rc;
    pbt_val_nil(dst);
    if (depth > PBT_MAX_VAL_DEPTH) {
        return PBT_E_LIMIT;
    }
    switch (src->kind) {
    case PBT_V_BYTES:
    case PBT_V_STR:
        return set_buffer(dst, src->kind, src->u.bytes.p, src->u.bytes.n);
    case PBT_V_LIST:
    case PBT_V_TUPLE:
        rc = pbt_val_set_list(dst, src->kind, src->u.list.n);
        if (rc != 0) {
            return rc;
        }
        for (i = 0; i < src->u.list.n; i++) {
            rc = clone_depth(&dst->u.list.items[i], &src->u.list.items[i], depth + 1);
            if (rc != 0) {
                pbt_val_free(dst);
                return rc;
            }
        }
        return 0;
    default:
        *dst = *src;
        return 0;
    }
}

int pbt_val_clone(pbt_val *dst, const pbt_val *src)
{
    return clone_depth(dst, src, 0);
}

static int equal_depth(const pbt_val *a, const pbt_val *b, int depth)
{
    size_t i;
    if (depth > PBT_MAX_VAL_DEPTH || a->kind != b->kind) {
        return 0;
    }
    switch (a->kind) {
    case PBT_V_NIL:
        return 1;
    case PBT_V_BOOL:
        return a->u.b == b->u.b;
    case PBT_V_INT:
        return pbt_big_cmp(&a->u.i, &b->u.i) == 0;
    case PBT_V_BYTES:
    case PBT_V_STR:
        return a->u.bytes.n == b->u.bytes.n &&
               (a->u.bytes.n == 0 || memcmp(a->u.bytes.p, b->u.bytes.p, a->u.bytes.n) == 0);
    case PBT_V_LIST:
    case PBT_V_TUPLE:
        if (a->u.list.n != b->u.list.n) {
            return 0;
        }
        for (i = 0; i < a->u.list.n; i++) {
            if (!equal_depth(&a->u.list.items[i], &b->u.list.items[i], depth + 1)) {
                return 0;
            }
        }
        return 1;
    case PBT_V_PTR:
        return a->u.ptr == b->u.ptr;
    }
    return 0;
}

int pbt_val_equal(const pbt_val *a, const pbt_val *b)
{
    return equal_depth(a, b, 0);
}

/* Length of a collection or string, 0 for anything else. */
static uint64_t shallow(const pbt_val *v)
{
    switch (v->kind) {
    case PBT_V_BYTES:
    case PBT_V_STR:
        return v->u.bytes.n;
    case PBT_V_LIST:
    case PBT_V_TUPLE:
        return v->u.list.n;
    default:
        return 0;
    }
}

uint64_t pbt_val_weight(const pbt_val *v)
{
    uint64_t total = shallow(v);
    size_t i;
    size_t visit;
    if (total == 0) {
        return 0;
    }
    if (v->kind == PBT_V_LIST || v->kind == PBT_V_TUPLE) {
        visit = v->u.list.n < 10000 ? v->u.list.n : 10000;
        for (i = 0; i < visit; i++) {
            total += shallow(&v->u.list.items[i]);
        }
    }
    return total;
}

/* ============================================================================
 * Canonical rendering
 *
 *   nil -> nil          true -> true          -12 -> -12
 *   bytes 00 ff -> 0x00ff                 string a<LF> -> "a\n"  (quoted, escaped)
 *   list [1,2] -> [1,2]                   tuple (1,2)
 *
 * Total by construction: depth, node count and output size are all capped.
 * ========================================================================== */

void pbt_str_init(pbt_str *s)
{
    memset(s, 0, sizeof *s);
}

void pbt_str_free(pbt_str *s)
{
    pbt_free(s->s);
    pbt_str_init(s);
}

/* Append, growing geometrically; refuses (and marks truncation) past the output cap. */
static int str_append(pbt_str *s, const char *data, size_t len, int *oom)
{
    size_t need;
    if (s->truncated) {
        return 0;
    }
    if (len > PBT_CANON_MAX_BYTES - 4 || s->n > PBT_CANON_MAX_BYTES - 4 - len) {
        s->truncated = 1;
        return 0;
    }
    need = s->n + len + 1;
    if (need > s->cap) {
        size_t cap = s->cap == 0 ? 64 : s->cap;
        char *p;
        while (cap < need) {
            cap *= 2;
        }
        p = (char *)pbt_realloc(s->s, cap, 1);
        if (p == NULL) {
            *oom = 1;
            return 0;
        }
        s->s = p;
        s->cap = cap;
    }
    memcpy(s->s + s->n, data, len);
    s->n += len;
    s->s[s->n] = '\0';
    return 1;
}

typedef struct canon_state {
    pbt_str *out;
    int nodes;
    int oom;
} canon_state;

static void put(canon_state *c, const char *text)
{
    (void)str_append(c->out, text, strlen(text), &c->oom);
}

static void canon_val(canon_state *c, const pbt_val *v, int depth)
{
    static const char hex[] = "0123456789abcdef";
    size_t i;
    c->nodes++;
    if (c->nodes > PBT_CANON_MAX_NODES) {
        put(c, "...");
        return;
    }
    switch (v->kind) {
    case PBT_V_NIL:
        put(c, "nil");
        break;
    case PBT_V_BOOL:
        put(c, v->u.b ? "true" : "false");
        break;
    case PBT_V_INT: {
        char buf[PBT_BIG_DEC_BUF];
        if (pbt_big_to_dec(&v->u.i, buf, sizeof buf) == 0) {
            put(c, "<unrenderable>");
        } else {
            put(c, buf);
        }
        break;
    }
    case PBT_V_BYTES:
        put(c, "0x");
        for (i = 0; i < v->u.bytes.n && !c->out->truncated && !c->oom; i++) {
            char two[2];
            two[0] = hex[v->u.bytes.p[i] >> 4];
            two[1] = hex[v->u.bytes.p[i] & 15];
            (void)str_append(c->out, two, 2, &c->oom);
        }
        break;
    case PBT_V_STR:
        put(c, "\"");
        for (i = 0; i < v->u.bytes.n && !c->out->truncated && !c->oom; i++) {
            unsigned ch = v->u.bytes.p[i];
            char esc[4];
            if (ch == '"') {
                put(c, "\\\"");
            } else if (ch == '\\') {
                put(c, "\\\\");
            } else if (ch == '\n') {
                put(c, "\\n");
            } else if (ch == '\r') {
                put(c, "\\r");
            } else if (ch == '\t') {
                put(c, "\\t");
            } else if (ch < 0x20 || ch >= 0x7f) {
                esc[0] = '\\';
                esc[1] = 'x';
                esc[2] = hex[ch >> 4];
                esc[3] = hex[ch & 15];
                (void)str_append(c->out, esc, 4, &c->oom);
            } else {
                esc[0] = (char)ch;
                (void)str_append(c->out, esc, 1, &c->oom);
            }
        }
        put(c, "\"");
        break;
    case PBT_V_LIST:
    case PBT_V_TUPLE:
        if (depth >= PBT_CANON_MAX_DEPTH) {
            put(c, "...");
            break;
        }
        put(c, v->kind == PBT_V_LIST ? "[" : "(");
        for (i = 0; i < v->u.list.n && !c->out->truncated && !c->oom; i++) {
            if (i > 0) {
                put(c, ",");
            }
            if (c->nodes >= PBT_CANON_MAX_NODES) { /* out of budget: stop, do not walk the rest */
                put(c, "...");
                break;
            }
            canon_val(c, &v->u.list.items[i], depth + 1);
        }
        put(c, v->kind == PBT_V_LIST ? "]" : ")");
        break;
    case PBT_V_PTR:
        put(c, "<ptr>");
        break;
    default:
        put(c, "<unrenderable>");
        break;
    }
}

int pbt_canon(const pbt_val *v, pbt_str *out)
{
    canon_state c;
    c.out = out;
    c.nodes = 0;
    c.oom = 0;
    canon_val(&c, v, 0);
    if (c.oom) {
        return PBT_E_NOMEM;
    }
    if (out->truncated) {
        /* The marker is allowed to overshoot the cap by the 4 bytes held back. */
        char *p = (char *)pbt_realloc(out->s, out->n + 4, 1);
        if (p == NULL) {
            return PBT_E_NOMEM;
        }
        out->s = p;
        out->cap = out->n + 4;
        memcpy(out->s + out->n, "...", 4);
        out->n += 3;
    }
    if (out->s == NULL) {
        /* Nothing was written: still return a valid empty string. */
        char *p = (char *)pbt_alloc(1, 1);
        if (p == NULL) {
            return PBT_E_NOMEM;
        }
        p[0] = '\0';
        out->s = p;
        out->cap = 1;
    }
    return 0;
}
