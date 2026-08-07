/*
 * logic_core.c — implementation of terms, substitutions, and unification.
 * =======================================================================
 *
 * Terms are an owned tagged-union tree. A substitution is a small array of
 * (var-id → owned term) bindings, copied on every `extend` so nothing is ever
 * mutated in place. `lc_unify` is the classic recursive first-order unification
 * algorithm with an occurs-check.
 */
#include "logic_core.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* strcmp, strlen, memcpy */

/* ── Variables ─────────────────────────────────────────────────────────────*/

/* Process-wide monotonic id source. Rust uses AtomicU64 for thread-safety; this
 * single-threaded pure-ISO port only needs distinct values, so a plain static
 * counter is faithful to the observable semantics. */
static uint64_t g_next_var_id = 0;

LcVar lc_var_fresh(const char *display_name) {
    LcVar v;
    v.id = g_next_var_id++;
    v.display_name[0] = '\0';
    if (display_name != NULL) {
        size_t i = 0;
        for (; display_name[i] != '\0' && i + 1 < LC_VAR_NAME_CAP; i++)
            v.display_name[i] = display_name[i];
        v.display_name[i] = '\0';
    }
    return v;
}

/* ── Numbers ───────────────────────────────────────────────────────────────*/

static int number_equal(LcNumber a, LcNumber b) {
    if (a.kind != b.kind) return 0;
    return a.kind == LC_INT ? (a.i == b.i) : (a.f == b.f);
}

/* ── Terms ─────────────────────────────────────────────────────────────────*/

typedef enum { T_ATOM, T_NUM, T_STR, T_VAR, T_COMPOUND } TermKind;

struct LcTerm {
    TermKind kind;
    union {
        char *atom;
        LcNumber num;
        char *str;
        LcVar var;
        struct {
            char *functor;
            LcTerm **args;
            size_t n_args;
        } compound;
    } as;
};

static char *str_dup(const char *s) {
    size_t n = strlen(s) + 1;
    char *p = (char *)malloc(n);
    if (p != NULL) memcpy(p, s, n);
    return p;
}

static LcTerm *alloc_term(TermKind kind) {
    LcTerm *t = (LcTerm *)calloc(1, sizeof(LcTerm));
    if (t != NULL) t->kind = kind;
    return t;
}

void lc_term_free(LcTerm *t) {
    if (t == NULL) return;
    switch (t->kind) {
        case T_ATOM:
            free(t->as.atom);
            break;
        case T_STR:
            free(t->as.str);
            break;
        case T_COMPOUND:
            for (size_t i = 0; i < t->as.compound.n_args; i++)
                lc_term_free(t->as.compound.args[i]);
            free(t->as.compound.args);
            free(t->as.compound.functor);
            break;
        default:
            break; /* NUM, VAR own no heap */
    }
    free(t);
}

LcTerm *lc_atom(const char *name) {
    LcTerm *t = alloc_term(T_ATOM);
    if (t == NULL) return NULL;
    t->as.atom = str_dup(name);
    if (t->as.atom == NULL) {
        free(t);
        return NULL;
    }
    return t;
}

LcTerm *lc_int(int64_t value) {
    LcTerm *t = alloc_term(T_NUM);
    if (t != NULL) {
        t->as.num.kind = LC_INT;
        t->as.num.i = value;
    }
    return t;
}

LcTerm *lc_float(double value) {
    LcTerm *t = alloc_term(T_NUM);
    if (t != NULL) {
        t->as.num.kind = LC_FLOAT;
        t->as.num.f = value;
    }
    return t;
}

LcTerm *lc_string(const char *value) {
    LcTerm *t = alloc_term(T_STR);
    if (t == NULL) return NULL;
    t->as.str = str_dup(value);
    if (t->as.str == NULL) {
        free(t);
        return NULL;
    }
    return t;
}

LcTerm *lc_term_var(LcVar v) {
    LcTerm *t = alloc_term(T_VAR);
    if (t != NULL) t->as.var = v;
    return t;
}

LcTerm *lc_compound(const char *functor, LcTerm **args, size_t n) {
    LcTerm *t = alloc_term(T_COMPOUND);
    if (t == NULL) {
        for (size_t i = 0; i < n; i++) lc_term_free(args[i]);
        return NULL;
    }
    t->as.compound.functor = str_dup(functor);
    if (t->as.compound.functor == NULL) {
        for (size_t i = 0; i < n; i++) lc_term_free(args[i]);
        free(t);
        return NULL;
    }
    if (n > 0) {
        t->as.compound.args = (LcTerm **)malloc(n * sizeof(LcTerm *));
        if (t->as.compound.args == NULL) {
            for (size_t i = 0; i < n; i++) lc_term_free(args[i]);
            free(t->as.compound.functor);
            free(t);
            return NULL;
        }
        memcpy(t->as.compound.args, args, n * sizeof(LcTerm *));
    }
    t->as.compound.n_args = n;
    return t;
}

LcTerm *lc_logic_list(LcTerm **items, size_t n) {
    LcTerm *result = lc_atom("[]");
    if (result == NULL) {
        for (size_t i = 0; i < n; i++) lc_term_free(items[i]);
        return NULL;
    }
    /* Fold from the right: .(i0, .(i1, … [])). */
    for (size_t k = n; k > 0; k--) {
        LcTerm *pair[2];
        pair[0] = items[k - 1];
        pair[1] = result;
        LcTerm *cell = lc_compound(".", pair, 2);
        if (cell == NULL) {
            /* lc_compound freed pair[0] (items[k-1]) and pair[1] (result);
             * free the remaining unconsumed items[0..k-1). */
            for (size_t j = 0; j < k - 1; j++) lc_term_free(items[j]);
            return NULL;
        }
        result = cell;
    }
    return result;
}

LcTerm *lc_term_clone(const LcTerm *t) {
    if (t == NULL) return NULL;
    switch (t->kind) {
        case T_ATOM:
            return lc_atom(t->as.atom);
        case T_NUM: {
            LcTerm *c = alloc_term(T_NUM);
            if (c != NULL) c->as.num = t->as.num;
            return c;
        }
        case T_STR:
            return lc_string(t->as.str);
        case T_VAR:
            return lc_term_var(t->as.var);
        case T_COMPOUND: {
            LcTerm *c = alloc_term(T_COMPOUND);
            if (c == NULL) return NULL;
            c->as.compound.functor = str_dup(t->as.compound.functor);
            if (c->as.compound.functor == NULL) {
                free(c);
                return NULL;
            }
            size_t n = t->as.compound.n_args;
            if (n > 0) {
                c->as.compound.args = (LcTerm **)calloc(n, sizeof(LcTerm *));
                if (c->as.compound.args == NULL) {
                    free(c->as.compound.functor);
                    free(c);
                    return NULL;
                }
            }
            c->as.compound.n_args = n;
            for (size_t i = 0; i < n; i++) {
                c->as.compound.args[i] = lc_term_clone(t->as.compound.args[i]);
                if (c->as.compound.args[i] == NULL) {
                    lc_term_free(c);
                    return NULL;
                }
            }
            return c;
        }
    }
    return NULL;
}

int lc_term_equal(const LcTerm *a, const LcTerm *b) {
    if (a == NULL || b == NULL) return a == b;
    if (a->kind != b->kind) return 0;
    switch (a->kind) {
        case T_ATOM:
            return strcmp(a->as.atom, b->as.atom) == 0;
        case T_NUM:
            return number_equal(a->as.num, b->as.num);
        case T_STR:
            return strcmp(a->as.str, b->as.str) == 0;
        case T_VAR:
            return a->as.var.id == b->as.var.id;
        case T_COMPOUND:
            if (strcmp(a->as.compound.functor, b->as.compound.functor) != 0)
                return 0;
            if (a->as.compound.n_args != b->as.compound.n_args) return 0;
            for (size_t i = 0; i < a->as.compound.n_args; i++)
                if (!lc_term_equal(a->as.compound.args[i],
                                   b->as.compound.args[i]))
                    return 0;
            return 1;
    }
    return 0;
}

/* ── String builder (for lc_term_to_string) ────────────────────────────────*/

typedef struct {
    char *buf;
    size_t len, cap;
    int oom;
} Sb;

static void sb_putn(Sb *sb, const char *s, size_t n) {
    if (sb->oom) return;
    if (sb->len + n + 1 > sb->cap) {
        size_t nc = sb->cap ? sb->cap : 32;
        while (nc < sb->len + n + 1) {
            if (nc > ((size_t)-1) / 2) {
                sb->oom = 1;
                return;
            }
            nc *= 2;
        }
        char *nb = (char *)realloc(sb->buf, nc);
        if (nb == NULL) {
            sb->oom = 1;
            return;
        }
        sb->buf = nb;
        sb->cap = nc;
    }
    memcpy(sb->buf + sb->len, s, n);
    sb->len += n;
    sb->buf[sb->len] = '\0';
}

static void sb_puts(Sb *sb, const char *s) { sb_putn(sb, s, strlen(s)); }

static void term_render(Sb *sb, const LcTerm *t) {
    char num[64];
    switch (t->kind) {
        case T_ATOM:
            sb_puts(sb, t->as.atom);
            break;
        case T_NUM:
            if (t->as.num.kind == LC_INT)
                snprintf(num, sizeof num, "%lld", (long long)t->as.num.i);
            else
                snprintf(num, sizeof num, "%g", t->as.num.f);
            sb_puts(sb, num);
            break;
        case T_STR: {
            /* Rust `{:?}`: wrap in quotes, escape '"' and '\\'. */
            sb_puts(sb, "\"");
            for (const char *p = t->as.str; *p; p++) {
                if (*p == '"' || *p == '\\') sb_puts(sb, "\\");
                sb_putn(sb, p, 1);
            }
            sb_puts(sb, "\"");
            break;
        }
        case T_VAR:
            if (t->as.var.display_name[0] != '\0') {
                sb_puts(sb, t->as.var.display_name);
            } else {
                snprintf(num, sizeof num, "_G%llu",
                         (unsigned long long)t->as.var.id);
                sb_puts(sb, num);
            }
            break;
        case T_COMPOUND:
            sb_puts(sb, t->as.compound.functor);
            sb_puts(sb, "(");
            for (size_t i = 0; i < t->as.compound.n_args; i++) {
                if (i > 0) sb_puts(sb, ", ");
                term_render(sb, t->as.compound.args[i]);
            }
            sb_puts(sb, ")");
            break;
    }
}

char *lc_term_to_string(const LcTerm *t) {
    Sb sb = {NULL, 0, 0, 0};
    term_render(&sb, t);
    if (sb.oom) {
        free(sb.buf);
        return NULL;
    }
    if (sb.buf == NULL) return str_dup(""); /* never hit; terms render non-empty */
    return sb.buf;
}

/* ── Substitutions ─────────────────────────────────────────────────────────*/

typedef struct {
    uint64_t id;
    LcTerm *term; /* owned */
} Binding;

struct LcSubst {
    Binding *items;
    size_t n, cap;
};

LcSubst *lc_subst_empty(void) {
    return (LcSubst *)calloc(1, sizeof(LcSubst));
}

void lc_subst_free(LcSubst *s) {
    if (s == NULL) return;
    for (size_t i = 0; i < s->n; i++) lc_term_free(s->items[i].term);
    free(s->items);
    free(s);
}

static const LcTerm *subst_lookup(const LcSubst *s, uint64_t id) {
    for (size_t i = 0; i < s->n; i++)
        if (s->items[i].id == id) return s->items[i].term;
    return NULL;
}

/* Deep-copy a substitution. NULL on OOM. */
static LcSubst *subst_clone(const LcSubst *s) {
    LcSubst *c = (LcSubst *)calloc(1, sizeof(LcSubst));
    if (c == NULL) return NULL;
    if (s->n > 0) {
        c->items = (Binding *)malloc(s->n * sizeof(Binding));
        if (c->items == NULL) {
            free(c);
            return NULL;
        }
        c->cap = s->n;
    }
    for (size_t i = 0; i < s->n; i++) {
        c->items[i].id = s->items[i].id;
        c->items[i].term = lc_term_clone(s->items[i].term);
        if (c->items[i].term == NULL) {
            c->n = i; /* free the i we managed to clone */
            lc_subst_free(c);
            return NULL;
        }
    }
    c->n = s->n;
    return c;
}

LcSubst *lc_subst_extend(const LcSubst *s, uint64_t var_id, const LcTerm *term) {
    LcSubst *c = subst_clone(s);
    if (c == NULL) return NULL;
    LcTerm *bound = lc_term_clone(term);
    if (bound == NULL) {
        lc_subst_free(c);
        return NULL;
    }
    /* Map-insert semantics: replace an existing binding for the same id. */
    for (size_t i = 0; i < c->n; i++) {
        if (c->items[i].id == var_id) {
            lc_term_free(c->items[i].term);
            c->items[i].term = bound;
            return c;
        }
    }
    if (c->n == c->cap) {
        size_t nc = c->cap ? c->cap : 4;
        if (nc > ((size_t)-1) / 2 / sizeof(Binding)) {
            lc_term_free(bound);
            lc_subst_free(c);
            return NULL;
        }
        nc *= 2;
        Binding *ni = (Binding *)realloc(c->items, nc * sizeof(Binding));
        if (ni == NULL) {
            lc_term_free(bound);
            lc_subst_free(c);
            return NULL;
        }
        c->items = ni;
        c->cap = nc;
    }
    c->items[c->n].id = var_id;
    c->items[c->n].term = bound;
    c->n++;
    return c;
}

LcTerm *lc_subst_walk(const LcSubst *s, const LcTerm *term) {
    LcTerm *current = lc_term_clone(term);
    if (current == NULL) return NULL;
    while (current->kind == T_VAR) {
        const LcTerm *bound = subst_lookup(s, current->as.var.id);
        if (bound == NULL) break;
        LcTerm *next = lc_term_clone(bound);
        if (next == NULL) {
            lc_term_free(current);
            return NULL;
        }
        lc_term_free(current);
        current = next;
    }
    return current;
}

LcTerm *lc_subst_walk_var(const LcSubst *s, LcVar v) {
    LcTerm *tv = lc_term_var(v);
    if (tv == NULL) return NULL;
    LcTerm *r = lc_subst_walk(s, tv);
    lc_term_free(tv);
    return r;
}

size_t lc_subst_len(const LcSubst *s) { return s->n; }

int lc_subst_equal(const LcSubst *a, const LcSubst *b) {
    if (a->n != b->n) return 0;
    for (size_t i = 0; i < a->n; i++) {
        const LcTerm *bt = subst_lookup(b, a->items[i].id);
        if (bt == NULL || !lc_term_equal(a->items[i].term, bt)) return 0;
    }
    return 1;
}

/* Does `var_id` occur inside `term` (after walking)? Occurs-check helper. */
static int subst_occurs(const LcSubst *s, uint64_t var_id, const LcTerm *term) {
    LcTerm *w = lc_subst_walk(s, term);
    if (w == NULL) return 0;
    int res = 0;
    if (w->kind == T_VAR) {
        res = w->as.var.id == var_id;
    } else if (w->kind == T_COMPOUND) {
        for (size_t i = 0; i < w->as.compound.n_args && !res; i++)
            res = subst_occurs(s, var_id, w->as.compound.args[i]);
    }
    lc_term_free(w);
    return res;
}

/* ── Unification ───────────────────────────────────────────────────────────*/

LcSubst *lc_unify(const LcTerm *a, const LcTerm *b, const LcSubst *s) {
    LcTerm *wa = lc_subst_walk(s, a);
    LcTerm *wb = lc_subst_walk(s, b);
    LcSubst *result = NULL;
    if (wa == NULL || wb == NULL) goto done;

    if (wa->kind == T_VAR && wb->kind == T_VAR &&
        wa->as.var.id == wb->as.var.id) {
        result = subst_clone(s);
    } else if (wa->kind == T_VAR) {
        result = subst_occurs(s, wa->as.var.id, wb)
                     ? NULL
                     : lc_subst_extend(s, wa->as.var.id, wb);
    } else if (wb->kind == T_VAR) {
        result = subst_occurs(s, wb->as.var.id, wa)
                     ? NULL
                     : lc_subst_extend(s, wb->as.var.id, wa);
    } else if (wa->kind == T_ATOM && wb->kind == T_ATOM) {
        result = strcmp(wa->as.atom, wb->as.atom) == 0 ? subst_clone(s) : NULL;
    } else if (wa->kind == T_NUM && wb->kind == T_NUM) {
        result = number_equal(wa->as.num, wb->as.num) ? subst_clone(s) : NULL;
    } else if (wa->kind == T_STR && wb->kind == T_STR) {
        result = strcmp(wa->as.str, wb->as.str) == 0 ? subst_clone(s) : NULL;
    } else if (wa->kind == T_COMPOUND && wb->kind == T_COMPOUND &&
               strcmp(wa->as.compound.functor, wb->as.compound.functor) == 0 &&
               wa->as.compound.n_args == wb->as.compound.n_args) {
        LcSubst *cur = subst_clone(s);
        for (size_t i = 0; cur != NULL && i < wa->as.compound.n_args; i++) {
            LcSubst *next =
                lc_unify(wa->as.compound.args[i], wb->as.compound.args[i], cur);
            lc_subst_free(cur);
            cur = next;
        }
        result = cur;
    } else {
        result = NULL;
    }

done:
    lc_term_free(wa);
    lc_term_free(wb);
    return result;
}
