/*
 * symbolic_ir.c — implementation of the symbolic-expression IR.
 * ===========================================================================
 * A `SirNode` is a tagged union; Apply owns its head and an array of argument
 * pointers. All tree walks (free, equals, hash, to_string) recurse over that
 * structure. See symbolic_ir.h for the API and ownership rules.
 */
#include "symbolic_ir.h"

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct SirNode {
    SirKind kind;
    union {
        char *symbol; /* SIR_SYMBOL (owned) */
        int64_t integer;
        struct {
            int64_t numer, denom;
        } rational;
        double flt;
        char *str; /* SIR_STR (owned) */
        struct {
            SirNode *head;
            SirNode **args; /* owned array of owned nodes */
            size_t n_args;
        } apply;
    } as;
};

/* ---------------------------------------------------------------------------
 *  Small helpers
 * ------------------------------------------------------------------------- */

static char *dup_cstr(const char *s) {
    size_t n = strlen(s);
    char *out = malloc(n + 1);
    if (out) {
        memcpy(out, s, n + 1);
    }
    return out;
}

/* Two's-complement magnitude of an int64 as u64 — correct even for INT64_MIN,
 * with no signed-overflow UB. */
static uint64_t uabs64(int64_t v) {
    return v < 0 ? ~(uint64_t)v + 1u : (uint64_t)v;
}

/* Rebuild a signed value from a magnitude and a sign, without UB (the negation
 * is done in unsigned arithmetic; the final cast is implementation-defined for
 * the single unrepresentable 2^63 magnitude, never undefined). */
static int64_t i64_from_mag(uint64_t mag, int neg) {
    return neg ? (int64_t)(~mag + 1u) : (int64_t)mag;
}

/* Euclidean GCD; gcd(0, 0) == 0 by convention. */
static uint64_t gcd_u64(uint64_t a, uint64_t b) {
    while (b != 0) {
        uint64_t t = b;
        b = a % b;
        a = t;
    }
    return a;
}

static SirNode *alloc_node(SirKind kind) {
    SirNode *n = calloc(1, sizeof *n);
    if (n) {
        n->kind = kind;
    }
    return n;
}

/* ---------------------------------------------------------------------------
 *  Constructors
 * ------------------------------------------------------------------------- */

SirNode *sir_sym(const char *name) {
    SirNode *n = alloc_node(SIR_SYMBOL);
    if (!n) {
        return NULL;
    }
    n->as.symbol = dup_cstr(name);
    if (!n->as.symbol) {
        free(n);
        return NULL;
    }
    return n;
}

SirNode *sir_int(int64_t v) {
    SirNode *n = alloc_node(SIR_INTEGER);
    if (n) {
        n->as.integer = v;
    }
    return n;
}

SirNode *sir_flt(double v) {
    SirNode *n = alloc_node(SIR_FLOAT);
    if (n) {
        n->as.flt = v;
    }
    return n;
}

SirNode *sir_str(const char *s) {
    SirNode *n = alloc_node(SIR_STR);
    if (!n) {
        return NULL;
    }
    n->as.str = dup_cstr(s);
    if (!n->as.str) {
        free(n);
        return NULL;
    }
    return n;
}

SirStatus sir_rational(int64_t numer, int64_t denom, SirNode **out) {
    if (denom == 0) {
        return SIR_ERR_ZERO_DENOM;
    }
    int neg = (numer < 0) != (denom < 0);
    uint64_t un = uabs64(numer);
    uint64_t ud = uabs64(denom);
    uint64_t g = gcd_u64(un, ud);
    if (g == 0) {
        g = 1; /* only if un == ud == 0, impossible since denom != 0 */
    }
    un /= g;
    ud /= g;

    SirNode *node;
    if (ud == 1) {
        node = sir_int(i64_from_mag(un, neg));
    } else {
        node = alloc_node(SIR_RATIONAL);
        if (node) {
            node->as.rational.numer = i64_from_mag(un, neg);
            node->as.rational.denom = (int64_t)ud;
        }
    }
    if (!node) {
        return SIR_ERR_NOMEM;
    }
    *out = node;
    return SIR_OK;
}

SirNode *sir_apply(SirNode *head, SirNode **args, size_t n_args) {
    SirNode *n = alloc_node(SIR_APPLY);
    SirNode **owned = NULL;
    if (n && n_args > 0) {
        owned = calloc(n_args, sizeof(SirNode *)); /* checked multiply */
    }
    if (!n || (n_args > 0 && !owned)) {
        /* Consume inputs on failure, per the documented contract. */
        free(n);
        sir_free(head);
        size_t i;
        for (i = 0; i < n_args; i++) {
            sir_free(args[i]);
        }
        return NULL;
    }
    size_t i;
    for (i = 0; i < n_args; i++) {
        owned[i] = args[i];
    }
    n->as.apply.head = head;
    n->as.apply.args = owned;
    n->as.apply.n_args = n_args;
    return n;
}

void sir_free(SirNode *n) {
    if (!n) {
        return;
    }
    switch (n->kind) {
        case SIR_SYMBOL:
            free(n->as.symbol);
            break;
        case SIR_STR:
            free(n->as.str);
            break;
        case SIR_APPLY: {
            sir_free(n->as.apply.head);
            size_t i;
            for (i = 0; i < n->as.apply.n_args; i++) {
                sir_free(n->as.apply.args[i]);
            }
            free(n->as.apply.args);
            break;
        }
        case SIR_INTEGER:
        case SIR_RATIONAL:
        case SIR_FLOAT:
            break; /* nothing owned */
    }
    free(n);
}

/* ---------------------------------------------------------------------------
 *  Accessors
 * ------------------------------------------------------------------------- */

SirKind sir_kind(const SirNode *n) { return n->kind; }
const char *sir_symbol_name(const SirNode *n) { return n->as.symbol; }
int64_t sir_integer_value(const SirNode *n) { return n->as.integer; }
void sir_rational_parts(const SirNode *n, int64_t *numer, int64_t *denom) {
    *numer = n->as.rational.numer;
    *denom = n->as.rational.denom;
}
double sir_float_value(const SirNode *n) { return n->as.flt; }
const char *sir_str_value(const SirNode *n) { return n->as.str; }
const SirNode *sir_apply_head(const SirNode *n) { return n->as.apply.head; }
size_t sir_apply_arity(const SirNode *n) { return n->as.apply.n_args; }
const SirNode *sir_apply_arg(const SirNode *n, size_t i) {
    return n->as.apply.args[i];
}

/* ---------------------------------------------------------------------------
 *  Equality
 * ------------------------------------------------------------------------- */

int sir_equals(const SirNode *a, const SirNode *b) {
    if (a->kind != b->kind) {
        return 0;
    }
    switch (a->kind) {
        case SIR_SYMBOL:
            return strcmp(a->as.symbol, b->as.symbol) == 0;
        case SIR_INTEGER:
            return a->as.integer == b->as.integer;
        case SIR_RATIONAL:
            return a->as.rational.numer == b->as.rational.numer &&
                   a->as.rational.denom == b->as.rational.denom;
        case SIR_FLOAT: {
            /* Compare by raw bit pattern (NaN with equal bits => equal). */
            uint64_t ab, bb;
            memcpy(&ab, &a->as.flt, sizeof ab);
            memcpy(&bb, &b->as.flt, sizeof bb);
            return ab == bb;
        }
        case SIR_STR:
            return strcmp(a->as.str, b->as.str) == 0;
        case SIR_APPLY: {
            if (a->as.apply.n_args != b->as.apply.n_args) {
                return 0;
            }
            if (!sir_equals(a->as.apply.head, b->as.apply.head)) {
                return 0;
            }
            size_t i;
            for (i = 0; i < a->as.apply.n_args; i++) {
                if (!sir_equals(a->as.apply.args[i], b->as.apply.args[i])) {
                    return 0;
                }
            }
            return 1;
        }
    }
    return 0;
}

/* ---------------------------------------------------------------------------
 *  Hash (FNV-1a over the discriminant + payload; consistent with equality)
 * ------------------------------------------------------------------------- */

static uint64_t fnv_mix(uint64_t h, const void *data, size_t len) {
    const unsigned char *p = data;
    size_t i;
    for (i = 0; i < len; i++) {
        h ^= p[i];
        h *= 1099511628211ull; /* FNV prime */
    }
    return h;
}

static uint64_t hash_rec(uint64_t h, const SirNode *n) {
    unsigned char tag = (unsigned char)n->kind;
    h = fnv_mix(h, &tag, 1);
    switch (n->kind) {
        case SIR_SYMBOL:
            return fnv_mix(h, n->as.symbol, strlen(n->as.symbol));
        case SIR_INTEGER:
            return fnv_mix(h, &n->as.integer, sizeof n->as.integer);
        case SIR_RATIONAL:
            h = fnv_mix(h, &n->as.rational.numer, sizeof(int64_t));
            return fnv_mix(h, &n->as.rational.denom, sizeof(int64_t));
        case SIR_FLOAT: {
            uint64_t bits;
            memcpy(&bits, &n->as.flt, sizeof bits);
            return fnv_mix(h, &bits, sizeof bits);
        }
        case SIR_STR:
            return fnv_mix(h, n->as.str, strlen(n->as.str));
        case SIR_APPLY: {
            h = hash_rec(h, n->as.apply.head);
            size_t i;
            for (i = 0; i < n->as.apply.n_args; i++) {
                h = hash_rec(h, n->as.apply.args[i]);
            }
            return h;
        }
    }
    return h;
}

uint64_t sir_hash(const SirNode *n) {
    return hash_rec(14695981039346656037ull /* FNV offset basis */, n);
}

/* ---------------------------------------------------------------------------
 *  Display — build into a growable buffer
 * ------------------------------------------------------------------------- */

typedef struct {
    char *data;
    size_t len, cap;
    int ok;
} Buf;

static void buf_reserve(Buf *b, size_t extra) {
    if (!b->ok) {
        return;
    }
    if (b->len + extra + 1 <= b->cap) {
        return;
    }
    size_t nc = b->cap ? b->cap : 32;
    while (nc < b->len + extra + 1) {
        if (nc > ((size_t)-1) / 2) {
            b->ok = 0;
            return;
        }
        nc *= 2;
    }
    char *nd = realloc(b->data, nc);
    if (!nd) {
        b->ok = 0;
        return;
    }
    b->data = nd;
    b->cap = nc;
}

static void buf_puts(Buf *b, const char *s) {
    size_t n = strlen(s);
    buf_reserve(b, n);
    if (!b->ok) {
        return;
    }
    memcpy(b->data + b->len, s, n);
    b->len += n;
    b->data[b->len] = '\0';
}

/* Format a double as the shortest round-tripping decimal, always with a decimal
 * point or exponent (matching Rust's `{:?}` for the common cases). */
static void buf_put_float(Buf *b, double v) {
    char tmp[64];
    int prec;
    for (prec = 1; prec <= 17; prec++) {
        snprintf(tmp, sizeof tmp, "%.*g", prec, v);
        if (strtod(tmp, NULL) == v) {
            break;
        }
    }
    /* Ensure it reads as a float: append ".0" when there is no '.', exponent,
     * or nan/inf letter. */
    if (!strpbrk(tmp, ".eEni")) {
        size_t l = strlen(tmp);
        if (l + 2 < sizeof tmp) {
            tmp[l] = '.';
            tmp[l + 1] = '0';
            tmp[l + 2] = '\0';
        }
    }
    buf_puts(b, tmp);
}

static void display_rec(Buf *b, const SirNode *n) {
    char tmp[64];
    switch (n->kind) {
        case SIR_SYMBOL:
            buf_puts(b, n->as.symbol);
            break;
        case SIR_INTEGER:
            snprintf(tmp, sizeof tmp, "%" PRId64, n->as.integer);
            buf_puts(b, tmp);
            break;
        case SIR_RATIONAL:
            snprintf(tmp, sizeof tmp, "%" PRId64 "/%" PRId64,
                     n->as.rational.numer, n->as.rational.denom);
            buf_puts(b, tmp);
            break;
        case SIR_FLOAT:
            buf_put_float(b, n->as.flt);
            break;
        case SIR_STR:
            buf_puts(b, "\"");
            buf_puts(b, n->as.str);
            buf_puts(b, "\"");
            break;
        case SIR_APPLY: {
            display_rec(b, n->as.apply.head);
            buf_puts(b, "(");
            size_t i;
            for (i = 0; i < n->as.apply.n_args; i++) {
                if (i > 0) {
                    buf_puts(b, ", ");
                }
                display_rec(b, n->as.apply.args[i]);
            }
            buf_puts(b, ")");
            break;
        }
    }
}

char *sir_to_string(const SirNode *n) {
    Buf b;
    b.data = NULL;
    b.len = 0;
    b.cap = 0;
    b.ok = 1;
    buf_reserve(&b, 0); /* ensure a non-NULL empty string on success */
    if (b.ok) {
        b.data[0] = '\0';
    }
    display_rec(&b, n);
    if (!b.ok) {
        free(b.data);
        return NULL;
    }
    return b.data;
}
