/*
 * pbt_gen.c -- the generator algebra (VER05 section 3, VER05i section 4).
 *
 * A generator is a small heap object:
 *
 *     pbt_gen { refs, sample(), shrink(), self, destroy() }
 *
 * `sample` draws a value from the random stream; `shrink` (optional) knows how
 * to make a failing value of this generator smaller while keeping it valid.
 * Combinators BORROW their children (they take a reference), and return NULL on
 * invalid arguments or out-of-memory, and a NULL child makes the combinator
 * return NULL, so a chain can be built first and checked once.
 *
 * DRAW ORDER is the compatibility contract between language lanes: the pbt-v1
 * vectors pin exact outputs per seed, so reordering a draw breaks conformance.
 */
#include <string.h>

#include "pbt.h"
#include "pbt_hooks.h"
#include "pbt_internal.h"
#include "pbt_shrink_internal.h"

struct pbt_gen {
    size_t refs;
    pbt_sample_fn sample;
    pbt_shrink_fn shrink;
    void *self;
    void (*destroy)(void *self);
};

pbt_gen *pbt_gen_custom(pbt_sample_fn sample, pbt_shrink_fn shrink, void *self, void (*destroy)(void *self))
{
    pbt_gen *g;
    if (sample == NULL) {
        return NULL;
    }
    g = (pbt_gen *)pbt_alloc(1, sizeof *g);
    if (g == NULL) {
        return NULL;
    }
    g->refs = 1;
    g->sample = sample;
    g->shrink = shrink;
    g->self = self;
    g->destroy = destroy;
    return g;
}

/* Like pbt_gen_custom, but on failure the context is destroyed so the caller never leaks it. */
static pbt_gen *make_gen(pbt_sample_fn sample, pbt_shrink_fn shrink, void *self, void (*destroy)(void *self))
{
    pbt_gen *g = pbt_gen_custom(sample, shrink, self, destroy);
    if (g == NULL && destroy != NULL && self != NULL) {
        destroy(self);
    }
    return g;
}

void pbt_gen_retain(pbt_gen *g)
{
    if (g != NULL) {
        g->refs++;
    }
}

void pbt_gen_release(pbt_gen *g)
{
    if (g == NULL) {
        return;
    }
    if (--g->refs == 0) {
        if (g->destroy != NULL) {
            g->destroy(g->self);
        }
        pbt_free(g);
    }
}

int pbt_gen_sample(pbt_run *run, const pbt_gen *g, pbt_val *out)
{
    int rc;
    pbt_val_nil(out);
    if (g == NULL) {
        return PBT_E_ARG;
    }
    if (run->depth >= PBT_MAX_SAMPLE_DEPTH) {
        return PBT_E_LIMIT;
    }
    run->depth++;
    rc = g->sample(g->self, run, out);
    run->depth--;
    if (rc != 0) {
        pbt_val_free(out); /* whatever the sampler left behind */
    }
    return rc;
}

/* A candidate evaluation that charges one attempt first and returns "falsifies" only if afforded. */
typedef struct charged_pred {
    pbt_pred inner;
    pbt_budget *scope;
} charged_pred;

static int charged_fn(void *self, pbt_budget *b, const pbt_val *v)
{
    charged_pred *c = (charged_pred *)self;
    (void)b;
    if (!pbt_budget_charge(c->scope, 1)) {
        return 0;
    }
    return c->inner.fn(c->inner.self, c->scope, v);
}

int pbt_gen_shrink_nested(pbt_budget *b, const pbt_gen *g, pbt_val *value, pbt_pred falsifies, int max_steps,
                          int *steps)
{
    *steps = 0;
    if (g == NULL || b == NULL || max_steps < 0 || max_steps > PBT_MAX_SHRINK_STEPS) {
        return PBT_E_ARG;
    }
    if (g->shrink == NULL) {
        return 0;
    }
    return g->shrink(g->self, b, value, falsifies, max_steps, steps);
}

int pbt_gen_shrink(pbt_budget *parent, const pbt_gen *g, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps)
{
    pbt_budget storage;
    pbt_budget *b;
    charged_pred cp;
    pbt_pred pred;
    int rc;
    *steps = 0;
    if (g == NULL || max_steps < 0 || max_steps > PBT_MAX_SHRINK_STEPS) {
        return PBT_E_ARG;
    }
    b = pbt_budget_open_child(&storage, parent, max_steps);
    cp.inner = falsifies;
    cp.scope = b;
    pred.fn = charged_fn;
    pred.self = &cp;
    rc = pbt_gen_shrink_nested(b, g, value, pred, max_steps, steps);
    pbt_budget_close(&storage);
    return rc;
}

/* ============================================================================
 * Integer generators
 * ========================================================================== */

typedef struct int_gen {
    pbt_big lo;
    pbt_big hi;
    pbt_big target; /* the in-range point nearest zero */
    int full_word;  /* sample as a raw 64-bit word (u64) */
    int is_i128;
    int is_u8;
} int_gen;

/* The integer shrink target: the in-range point nearest zero. */
static void clamp_zero(pbt_big *out, const pbt_big *lo, const pbt_big *hi)
{
    if (pbt_big_sign(lo) > 0) {
        *out = *lo;
    } else if (pbt_big_sign(hi) < 0) {
        *out = *hi;
    } else {
        pbt_big_zero(out);
    }
}

typedef struct gen_probe {
    pbt_pred pred;
    pbt_budget *b;
} gen_probe;

static int gen_probe_fn(void *self, const pbt_big *c)
{
    gen_probe *p = (gen_probe *)self;
    pbt_val v;
    pbt_val_set_int(&v, c);
    return p->pred.fn(p->pred.self, p->b, &v);
}

static int int_shrink(void *self, pbt_budget *b, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps)
{
    int_gen *g = (int_gen *)self;
    gen_probe probe;
    pbt_big cur;
    *steps = 0;
    if (value->kind != PBT_V_INT) {
        return PBT_E_ARG;
    }
    probe.pred = falsifies;
    probe.b = b;
    cur = value->u.i;
    pbt_int_shrink_core(&cur, &g->target, &g->lo, &g->hi, gen_probe_fn, &probe, max_steps, steps);
    pbt_val_set_int(value, &cur);
    return 0;
}

static void sample_i128(pbt_rng *r, pbt_big *out)
{
    /* Two words, high word first, read as two's complement. */
    uint64_t w1 = pbt_rng_next(r);
    uint64_t w2 = pbt_rng_next(r);
    pbt_big v;
    pbt_big_zero(&v);
    v.d[0] = (uint32_t)(w2 & 0xffffffffu);
    v.d[1] = (uint32_t)(w2 >> 32);
    v.d[2] = (uint32_t)(w1 & 0xffffffffu);
    v.d[3] = (uint32_t)(w1 >> 32);
    v.n = 4;
    while (v.n > 0 && v.d[v.n - 1] == 0) {
        v.n--;
    }
    if ((w1 >> 63) != 0) { /* >= 2^127: subtract 2^128 */
        pbt_big p;
        (void)pbt_big_pow2(&p, 128);
        (void)pbt_big_sub(&v, &v, &p);
    }
    *out = v;
}

static int int_sample(void *self, pbt_run *run, pbt_val *out)
{
    int_gen *g = (int_gen *)self;
    pbt_big v;
    if (g->is_i128) {
        sample_i128(&run->rng, &v);
    } else if (g->full_word) {
        pbt_big_set_u64(&v, pbt_rng_next(&run->rng));
    } else if (g->is_u8) {
        pbt_big_set_u64(&v, pbt_rng_bounded(&run->rng, 256));
    } else {
        pbt_big span;
        pbt_big off;
        pbt_big one;
        pbt_big_set_u64(&one, 1);
        (void)pbt_big_sub(&span, &g->hi, &g->lo);
        (void)pbt_big_add(&span, &span, &one);
        if (pbt_rng_bounded_big(&run->rng, &span, &off) != 0 || pbt_big_add(&v, &g->lo, &off) != 0) {
            return PBT_E_LIMIT;
        }
    }
    pbt_val_set_int(out, &v);
    return 0;
}

static void int_destroy(void *self)
{
    pbt_free(self);
}

static pbt_gen *int_gen_new(const pbt_big *lo, const pbt_big *hi, int full_word, int is_i128, int is_u8)
{
    int_gen *g;
    if (pbt_big_cmp(lo, hi) > 0 || pbt_big_bitlen(lo) > PBT_BIG_MAX_BITS || pbt_big_bitlen(hi) > PBT_BIG_MAX_BITS) {
        return NULL;
    }
    g = (int_gen *)pbt_alloc(1, sizeof *g);
    if (g == NULL) {
        return NULL;
    }
    g->lo = *lo;
    g->hi = *hi;
    clamp_zero(&g->target, lo, hi);
    g->full_word = full_word;
    g->is_i128 = is_i128;
    g->is_u8 = is_u8;
    return make_gen(int_sample, int_shrink, g, int_destroy);
}

pbt_gen *pbt_gen_u8(void)
{
    pbt_big lo;
    pbt_big hi;
    pbt_big_zero(&lo);
    pbt_big_set_u64(&hi, 255);
    return int_gen_new(&lo, &hi, 0, 0, 1);
}

pbt_gen *pbt_gen_u64(void)
{
    pbt_big lo;
    pbt_big hi;
    pbt_big_zero(&lo);
    pbt_big_set_u64(&hi, UINT64_MAX);
    return int_gen_new(&lo, &hi, 1, 0, 0);
}

pbt_gen *pbt_gen_i128(void)
{
    pbt_big lo;
    pbt_big hi;
    pbt_big one;
    pbt_big_set_u64(&one, 1);
    (void)pbt_big_pow2(&hi, 127);
    pbt_big_neg(&lo, &hi);
    (void)pbt_big_sub(&hi, &hi, &one);
    return int_gen_new(&lo, &hi, 0, 1, 0);
}

pbt_gen *pbt_gen_int(const pbt_big *min, const pbt_big *max)
{
    if (min == NULL || max == NULL) {
        return NULL;
    }
    return int_gen_new(min, max, 0, 0, 0);
}

pbt_gen *pbt_gen_int_i64(int64_t min, int64_t max)
{
    pbt_big lo;
    pbt_big hi;
    pbt_big_set_i64(&lo, min);
    pbt_big_set_i64(&hi, max);
    return int_gen_new(&lo, &hi, 0, 0, 0);
}

/* ---- bool ------------------------------------------------------------------ */

static int bool_sample(void *self, pbt_run *run, pbt_val *out)
{
    (void)self;
    pbt_val_set_bool(out, pbt_rng_bounded(&run->rng, 2) == 1);
    return 0;
}

static int bool_shrink(void *self, pbt_budget *b, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps)
{
    pbt_val f;
    (void)self;
    *steps = 0;
    if (value->kind == PBT_V_BOOL && value->u.b && max_steps > 0) {
        pbt_val_set_bool(&f, 0);
        if (falsifies.fn(falsifies.self, b, &f)) {
            pbt_val_set_bool(value, 0);
            *steps = 1;
        }
    }
    return 0;
}

pbt_gen *pbt_gen_bool(void)
{
    return make_gen(bool_sample, bool_shrink, NULL, NULL);
}

/* ---- bytes ----------------------------------------------------------------- */

typedef struct bytes_gen {
    size_t min_len;
    size_t max_len;
} bytes_gen;

static int bytes_sample(void *self, pbt_run *run, pbt_val *out)
{
    bytes_gen *g = (bytes_gen *)self;
    uint64_t span = (uint64_t)(g->max_len - g->min_len) + 1;
    size_t n = g->min_len + (size_t)pbt_rng_bounded(&run->rng, span);
    size_t i;
    int rc = pbt_val_set_bytes(out, NULL, n);
    if (rc != 0) {
        return rc;
    }
    for (i = 0; i < n; i++) {
        out->u.bytes.p[i] = (uint8_t)pbt_rng_bounded(&run->rng, 256);
    }
    return 0;
}

static int bytes_shrink(void *self, pbt_budget *b, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps)
{
    bytes_gen *g = (bytes_gen *)self;
    *steps = 0;
    if (value->kind != PBT_V_BYTES) {
        return PBT_E_ARG;
    }
    return pbt_shrink_sequence(b, value, falsifies, max_steps, g->min_len, NULL, NULL, steps);
}

static void plain_destroy(void *self)
{
    pbt_free(self);
}

pbt_gen *pbt_gen_bytes(size_t min_len, size_t max_len)
{
    bytes_gen *g;
    if (max_len < min_len || max_len > PBT_MAX_INPUT_BYTES) {
        return NULL;
    }
    g = (bytes_gen *)pbt_alloc(1, sizeof *g);
    if (g == NULL) {
        return NULL;
    }
    g->min_len = min_len;
    g->max_len = max_len;
    return make_gen(bytes_sample, bytes_shrink, g, plain_destroy);
}

/* ---- constant -------------------------------------------------------------- */

static int constant_sample(void *self, pbt_run *run, pbt_val *out)
{
    (void)run;
    return pbt_val_clone(out, (const pbt_val *)self);
}

static void constant_destroy(void *self)
{
    pbt_val_free((pbt_val *)self);
    pbt_free(self);
}

pbt_gen *pbt_gen_constant(const pbt_val *v)
{
    pbt_val *copy;
    if (v == NULL) {
        return NULL;
    }
    copy = (pbt_val *)pbt_alloc(1, sizeof *copy);
    if (copy == NULL) {
        return NULL;
    }
    if (pbt_val_clone(copy, v) != 0) {
        pbt_free(copy);
        return NULL;
    }
    return make_gen(constant_sample, NULL, copy, constant_destroy);
}

/* ============================================================================
 * Combinators
 * ========================================================================== */

/* ---- map: drops shrinking (no inverse) ------------------------------------- */

typedef struct map_gen {
    pbt_gen *child;
    pbt_map_fn f;
    void *self;
} map_gen;

static int map_sample(void *self, pbt_run *run, pbt_val *out)
{
    map_gen *m = (map_gen *)self;
    pbt_val in;
    int rc = pbt_gen_sample(run, m->child, &in);
    if (rc != 0) {
        return rc;
    }
    rc = m->f(m->self, &in, out);
    pbt_val_free(&in);
    return rc == 0 ? 0 : (rc > 0 ? rc : PBT_E_GEN);
}

static void map_destroy(void *self)
{
    map_gen *m = (map_gen *)self;
    pbt_gen_release(m->child);
    pbt_free(m);
}

pbt_gen *pbt_gen_map(pbt_gen *g, pbt_map_fn f, void *self)
{
    map_gen *m;
    if (g == NULL || f == NULL) {
        return NULL;
    }
    m = (map_gen *)pbt_alloc(1, sizeof *m);
    if (m == NULL) {
        return NULL;
    }
    pbt_gen_retain(g);
    m->child = g;
    m->f = f;
    m->self = self;
    return make_gen(map_sample, NULL, m, map_destroy);
}

/* ---- filter ----------------------------------------------------------------- */

typedef struct filter_gen {
    pbt_gen *child;
    pbt_filter_fn pred;
    void *self;
    int max_retries;
} filter_gen;

/*
 * Every retry is charged whenever a budget exists, so stacked filters cannot
 * multiply uncharged retries.  A nested check pays one attempt up the chain; a
 * top-level check root pays one element of WORK (its attempt cap would wrongly
 * exhaust a legitimate rare-but-satisfiable filter).  Refused: the sample is
 * exhausted, and the root's `refused` flag turns a pass into exhausted.
 */
static int filter_sample(void *self, pbt_run *run, pbt_val *out)
{
    filter_gen *f = (filter_gen *)self;
    pbt_budget *budget = run->budget;
    int nested = budget != NULL && pbt_budget_is_nested(budget);
    int i;
    for (i = 0; i < f->max_retries; i++) {
        pbt_val v;
        int rc;
        int keep;
        if (budget != NULL && !(nested ? pbt_budget_charge(budget, 1) : pbt_budget_charge_copy(budget, 1))) {
            return PBT_E_EXHAUSTED;
        }
        rc = pbt_gen_sample(run, f->child, &v);
        if (rc != 0) {
            return rc;
        }
        keep = f->pred(f->self, budget, &v);
        if (keep > 0) {
            *out = v;
            return 0;
        }
        pbt_val_free(&v);
        if (keep < 0) {
            return PBT_E_GEN;
        }
    }
    return PBT_E_EXHAUSTED;
}

typedef struct filter_shrink_ctx {
    filter_gen *f;
    pbt_pred outer;
} filter_shrink_ctx;

static int filter_shrink_pred(void *self, pbt_budget *b, const pbt_val *v)
{
    filter_shrink_ctx *c = (filter_shrink_ctx *)self;
    /* Charge before the user predicate runs: it is property code and may be costly. */
    if (b != NULL && !pbt_budget_charge(b, 1)) {
        return 0;
    }
    if (c->f->pred(c->f->self, b, v) <= 0) {
        return 0; /* a candidate that fails the precondition (or errors) is not a valid input */
    }
    return c->outer.fn(c->outer.self, b, v);
}

static int filter_shrink(void *self, pbt_budget *b, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps)
{
    filter_gen *f = (filter_gen *)self;
    filter_shrink_ctx c;
    pbt_pred pred;
    c.f = f;
    c.outer = falsifies;
    pred.fn = filter_shrink_pred;
    pred.self = &c;
    return pbt_gen_shrink_nested(b, f->child, value, pred, max_steps, steps);
}

static void filter_destroy(void *self)
{
    filter_gen *f = (filter_gen *)self;
    pbt_gen_release(f->child);
    pbt_free(f);
}

pbt_gen *pbt_gen_filter(pbt_gen *g, pbt_filter_fn pred, void *self, int max_retries)
{
    filter_gen *f;
    if (g == NULL || pred == NULL || max_retries < 1 || max_retries > PBT_MAX_FILTER_RETRIES) {
        return NULL;
    }
    f = (filter_gen *)pbt_alloc(1, sizeof *f);
    if (f == NULL) {
        return NULL;
    }
    pbt_gen_retain(g);
    f->child = g;
    f->pred = pred;
    f->self = self;
    f->max_retries = max_retries;
    return make_gen(filter_sample, filter_shrink, f, filter_destroy);
}

/* ---- flat_map: dependent generation, no shrinking --------------------------- */

typedef struct flat_gen {
    pbt_gen *child;
    pbt_flat_map_fn f;
    void *self;
} flat_gen;

static int flat_sample(void *self, pbt_run *run, pbt_val *out)
{
    flat_gen *m = (flat_gen *)self;
    pbt_val in;
    pbt_gen *next = NULL;
    int rc = pbt_gen_sample(run, m->child, &in);
    if (rc != 0) {
        return rc;
    }
    rc = m->f(m->self, &in, &next);
    pbt_val_free(&in);
    if (rc != 0 || next == NULL) {
        pbt_gen_release(next);
        return rc > 0 ? rc : PBT_E_GEN;
    }
    rc = pbt_gen_sample(run, next, out);
    pbt_gen_release(next);
    return rc;
}

static void flat_destroy(void *self)
{
    flat_gen *m = (flat_gen *)self;
    pbt_gen_release(m->child);
    pbt_free(m);
}

pbt_gen *pbt_gen_flat_map(pbt_gen *g, pbt_flat_map_fn f, void *self)
{
    flat_gen *m;
    if (g == NULL || f == NULL) {
        return NULL;
    }
    m = (flat_gen *)pbt_alloc(1, sizeof *m);
    if (m == NULL) {
        return NULL;
    }
    pbt_gen_retain(g);
    m->child = g;
    m->f = f;
    m->self = self;
    return make_gen(flat_sample, NULL, m, flat_destroy);
}

/* ---- one_of / frequency ------------------------------------------------------ */

typedef struct choice_gen {
    size_t n;
    pbt_arm *arms;     /* weight is ignored by one_of */
    uint64_t total;    /* 0 for one_of */
} choice_gen;

static void choice_destroy(void *self)
{
    choice_gen *c = (choice_gen *)self;
    size_t i;
    for (i = 0; i < c->n; i++) {
        pbt_gen_release(c->arms[i].gen);
    }
    pbt_free(c->arms);
    pbt_free(c);
}

static int one_of_sample(void *self, pbt_run *run, pbt_val *out)
{
    choice_gen *c = (choice_gen *)self;
    size_t k = (size_t)pbt_rng_bounded(&run->rng, (uint64_t)c->n);
    return pbt_gen_sample(run, c->arms[k].gen, out);
}

static int frequency_sample(void *self, pbt_run *run, pbt_val *out)
{
    choice_gen *c = (choice_gen *)self;
    uint64_t pick = pbt_rng_bounded(&run->rng, c->total);
    size_t i;
    for (i = 0; i < c->n; i++) {
        if (pick < c->arms[i].weight) {
            return pbt_gen_sample(run, c->arms[i].gen, out);
        }
        pick -= c->arms[i].weight;
    }
    return PBT_E_GEN; /* unreachable: pick < total */
}

static pbt_gen *choice_new(const pbt_arm *arms, size_t n, uint64_t total, pbt_sample_fn sample)
{
    choice_gen *c;
    size_t i;
    c = (choice_gen *)pbt_alloc(1, sizeof *c);
    if (c == NULL) {
        return NULL;
    }
    c->arms = (pbt_arm *)pbt_alloc(n, sizeof *c->arms);
    if (c->arms == NULL) {
        pbt_free(c);
        return NULL;
    }
    for (i = 0; i < n; i++) {
        c->arms[i] = arms[i];
        pbt_gen_retain(arms[i].gen);
    }
    c->n = n;
    c->total = total;
    return make_gen(sample, NULL, c, choice_destroy);
}

pbt_gen *pbt_gen_one_of(pbt_gen *const *gens, size_t n)
{
    pbt_arm *arms;
    pbt_gen *g;
    size_t i;
    if (gens == NULL || n == 0 || n > PBT_MAX_LIST_LEN) {
        return NULL;
    }
    arms = (pbt_arm *)pbt_alloc(n, sizeof *arms);
    if (arms == NULL) {
        return NULL;
    }
    for (i = 0; i < n; i++) {
        if (gens[i] == NULL) {
            pbt_free(arms);
            return NULL;
        }
        arms[i].weight = 1;
        arms[i].gen = gens[i];
    }
    g = choice_new(arms, n, 0, one_of_sample);
    pbt_free(arms);
    return g;
}

pbt_gen *pbt_gen_frequency(const pbt_arm *arms, size_t n)
{
    uint64_t total = 0;
    size_t i;
    if (arms == NULL || n == 0 || n > PBT_MAX_LIST_LEN) {
        return NULL;
    }
    for (i = 0; i < n; i++) {
        if (arms[i].gen == NULL || arms[i].weight > PBT_MAX_FREQUENCY_TOTAL) {
            return NULL;
        }
        total += arms[i].weight; /* at most n * 2^32: cannot wrap a uint64_t */
        if (total > PBT_MAX_FREQUENCY_TOTAL) {
            return NULL;
        }
    }
    if (total == 0) {
        return NULL;
    }
    return choice_new(arms, n, total, frequency_sample);
}

/* ---- list ------------------------------------------------------------------- */

typedef struct list_gen {
    pbt_gen *elem;
    size_t min_len;
    size_t max_len;
} list_gen;

static int list_sample(void *self, pbt_run *run, pbt_val *out)
{
    list_gen *g = (list_gen *)self;
    uint64_t span = (uint64_t)(g->max_len - g->min_len) + 1;
    size_t n = g->min_len + (size_t)pbt_rng_bounded(&run->rng, span);
    size_t i;
    int rc = pbt_val_set_list(out, PBT_V_LIST, n);
    if (rc != 0) {
        return rc;
    }
    for (i = 0; i < n; i++) {
        rc = pbt_gen_sample(run, g->elem, &out->u.list.items[i]);
        if (rc != 0) {
            return rc; /* the caller frees *out, which owns the partial list */
        }
    }
    return 0;
}

static int list_shrink(void *self, pbt_budget *b, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps)
{
    list_gen *g = (list_gen *)self;
    *steps = 0;
    if (value->kind != PBT_V_LIST) {
        return PBT_E_ARG;
    }
    return pbt_shrink_sequence(b, value, falsifies, max_steps, g->min_len, g->elem, NULL, steps);
}

static void list_destroy(void *self)
{
    list_gen *g = (list_gen *)self;
    pbt_gen_release(g->elem);
    pbt_free(g);
}

pbt_gen *pbt_gen_list(pbt_gen *elem, size_t min_len, size_t max_len)
{
    list_gen *g;
    if (elem == NULL || max_len < min_len || max_len > PBT_MAX_LIST_LEN) {
        return NULL;
    }
    g = (list_gen *)pbt_alloc(1, sizeof *g);
    if (g == NULL) {
        return NULL;
    }
    pbt_gen_retain(elem);
    g->elem = elem;
    g->min_len = min_len;
    g->max_len = max_len;
    return make_gen(list_sample, list_shrink, g, list_destroy);
}

/* ---- tuple ------------------------------------------------------------------ */

typedef struct tuple_gen {
    size_t n;
    pbt_gen *gens[PBT_MAX_TUPLE];
} tuple_gen;

static int tuple_sample(void *self, pbt_run *run, pbt_val *out)
{
    tuple_gen *t = (tuple_gen *)self;
    size_t i;
    int rc = pbt_val_set_list(out, PBT_V_TUPLE, t->n);
    if (rc != 0) {
        return rc;
    }
    for (i = 0; i < t->n; i++) {
        rc = pbt_gen_sample(run, t->gens[i], &out->u.list.items[i]);
        if (rc != 0) {
            return rc;
        }
    }
    return 0;
}

/*
 * One shrink-wide attempt counter for a tuple: the repeat-until-no-progress loop
 * calls each component shrinker once per pass, and each of those has its own
 * local budget, so without a shared one the work is quadratic in max_steps.
 * Every evaluation of the property, in any component or pass, is counted here;
 * when it is spent nothing further is accepted.
 */
typedef struct tuple_budget {
    pbt_pred inner;
    uint64_t spent;
    uint64_t limit;
} tuple_budget;

static int tuple_budget_fn(void *self, pbt_budget *b, const pbt_val *v)
{
    tuple_budget *t = (tuple_budget *)self;
    if (t->spent++ >= t->limit) {
        return 0;
    }
    return t->inner.fn(t->inner.self, b, v);
}

/* A coordinate candidate: put it into the tuple, evaluate the whole tuple, restore. */
typedef struct coord_probe {
    pbt_val *tuple;
    size_t at;
    pbt_pred outer;
} coord_probe;

static int coord_probe_fn(void *self, pbt_budget *b, const pbt_val *c)
{
    coord_probe *p = (coord_probe *)self;
    pbt_val saved = p->tuple->u.list.items[p->at];
    int r;
    p->tuple->u.list.items[p->at] = *c; /* shallow alias, restored below */
    r = p->outer.fn(p->outer.self, b, p->tuple);
    p->tuple->u.list.items[p->at] = saved;
    return r;
}

static int tuple_shrink(void *self, pbt_budget *b, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps)
{
    tuple_gen *t = (tuple_gen *)self;
    tuple_budget tb;
    pbt_pred budgeted;
    int total = 0;
    int progress = 1;
    *steps = 0;
    if (value->kind != PBT_V_TUPLE || value->u.list.n != t->n) {
        return PBT_E_ARG;
    }
    tb.inner = falsifies;
    tb.spent = 0;
    tb.limit = pbt_attempt_budget(max_steps);
    budgeted.fn = tuple_budget_fn;
    budgeted.self = &tb;
    /* Repeat passes until a whole pass makes no progress: shrinking one coordinate
     * can unlock more shrinking of another.  Every pass that continues accepted at
     * least one move, so the budget bounds it. */
    while (progress && total < max_steps) {
        size_t i;
        progress = 0;
        for (i = 0; i < t->n; i++) {
            if (max_steps - total > 0) {
                coord_probe cp;
                pbt_pred pred;
                int used = 0;
                cp.tuple = value;
                cp.at = i;
                cp.outer = budgeted;
                pred.fn = coord_probe_fn;
                pred.self = &cp;
                (void)pbt_gen_shrink_nested(b, t->gens[i], &value->u.list.items[i], pred, max_steps - total, &used);
                if (used > 0) {
                    total += used;
                    progress = 1;
                }
            }
        }
    }
    *steps = total;
    return 0;
}

static void tuple_destroy(void *self)
{
    tuple_gen *t = (tuple_gen *)self;
    size_t i;
    for (i = 0; i < t->n; i++) {
        pbt_gen_release(t->gens[i]);
    }
    pbt_free(t);
}

pbt_gen *pbt_gen_tuple(pbt_gen *const *gens, size_t n)
{
    tuple_gen *t;
    size_t i;
    if (gens == NULL || n == 0 || n > PBT_MAX_TUPLE) {
        return NULL;
    }
    for (i = 0; i < n; i++) {
        if (gens[i] == NULL) {
            return NULL;
        }
    }
    t = (tuple_gen *)pbt_alloc(1, sizeof *t);
    if (t == NULL) {
        return NULL;
    }
    t->n = n;
    for (i = 0; i < n; i++) {
        pbt_gen_retain(gens[i]);
        t->gens[i] = gens[i];
    }
    return make_gen(tuple_sample, tuple_shrink, t, tuple_destroy);
}
