/*
 * pbt_shrink.c -- integrated shrinking (VER05 section 4, VER05i section 5).
 *
 * A property that fails on [913, -4, 77, 0, 2] has told you nothing; one that
 * fails on [8, 5, 8] has told you what is wrong.  Shrinking walks from the
 * first failing candidate toward the smallest one that still fails.
 *
 * Every shrinker works on a FALSIFIES predicate (1 = the value still fails) and
 * counts ACCEPTED moves only.
 *
 * THE INTEGER WALK
 * ----------------
 * Each turn tries, in order,
 *   1. the bisection candidate  current - floor(diff/2),  diff = current - target,
 *      when |diff| > 1;
 *   2. the unit candidate       current -/+ 1 toward the target.
 * A candidate is accepted only if it is inside [lo, hi], differs from current and
 * still falsifies.  No acceptable candidate ends the walk.
 *
 * Worked example (corpus pbt/shrink_int/54321_le_100, property x <= 100):
 *
 *     54321 -> 27161 -> 13581 -> ... -> 101          15 accepted moves
 *
 * 101 is the smallest value that still fails: 100 satisfies the property, so
 * neither candidate from 101 is accepted.
 *
 * THE SEQUENCE SHRINKER (bytes and lists), in place
 * -------------------------------------------------
 *   phase 0  probe: does the length-min_len prefix fail?  (min_len 0: the empty
 *            list; then it is the answer, one step.)
 *   phase 1  bisection: keep the left half, else the right half, while one of
 *            them still fails.
 *   phase 2  single deletion, right to left.
 *   phase 3  element-wise, left to right.
 *
 * Worked example (corpus list_sum_le_20, [10 5 8 4]): the empty list satisfies
 * the property; both halves [10 5] and [8 4] satisfy it; deleting index 3 leaves
 * [10 5 8] (sum 23, still failing); the element walk takes 10 to 8, giving the
 * 1-minimal [8 5 8].
 *
 * NO COPIES.  A prefix or half candidate is a (pointer, length) VIEW of the
 * working array.  A deletion candidate is made by shifting the tail left over
 * the element, evaluating the shorter view, and then either keeping the shift
 * (accepted) or shifting back (rejected).  An element candidate temporarily
 * overwrites the element.  So the shrinker allocates nothing, and a property
 * that obeys the `const` contract cannot tell the difference.  The budget
 * (VER05i section 6) is still charged BEFORE every candidate at its logical
 * size, exactly as in the garbage-collected lanes.
 */
#include <string.h>

#include "pbt.h"
#include "pbt_hooks.h"
#include "pbt_shrink_internal.h"

/* ---- integer walk ---------------------------------------------------------- */

static int in_bounds(const pbt_big *v, const pbt_big *lo, const pbt_big *hi)
{
    return (lo == NULL || pbt_big_cmp(lo, v) <= 0) && (hi == NULL || pbt_big_cmp(v, hi) <= 0);
}

/* current - floor(diff / 2) */
static void default_bisect(pbt_big *out, const pbt_big *current, const pbt_big *diff)
{
    pbt_big half;
    pbt_big_floor_half(&half, diff);
    (void)pbt_big_sub(out, current, &half);
}

/* current - sign(diff) */
static void default_unit(pbt_big *out, const pbt_big *current, const pbt_big *diff)
{
    pbt_big s;
    pbt_big_set_i64(&s, pbt_big_sign(diff));
    (void)pbt_big_sub(out, current, &s);
}

void pbt_int_walk(pbt_big *current, const pbt_big *target, const pbt_big *lo, const pbt_big *hi,
                  pbt_big_falsifies_fn falsifies, void *self, int budget, int *steps)
{
    int n = 0;
    while (n < budget && pbt_big_cmp(current, target) != 0) {
        pbt_big diff;
        pbt_big cand;
        pbt_big one;
        int moved = 0;
        pbt_big_set_u64(&one, 1);
        (void)pbt_big_sub(&diff, current, target);
        if (pbt_big_cmp_abs(&diff, &one) > 0) {
            if (pbt_hooks.bisect != NULL) {
                pbt_hooks.bisect(&cand, current, &diff);
            } else {
                default_bisect(&cand, current, &diff);
            }
            if (pbt_big_cmp(&cand, current) != 0 && in_bounds(&cand, lo, hi) && falsifies(self, &cand)) {
                *current = cand;
                n++;
                moved = 1;
            }
        }
        if (!moved) {
            if (pbt_hooks.unit != NULL) {
                pbt_hooks.unit(&cand, current, &diff);
            } else {
                default_unit(&cand, current, &diff);
            }
            if (pbt_big_cmp(&cand, current) != 0 && in_bounds(&cand, lo, hi) && falsifies(self, &cand)) {
                *current = cand;
                n++;
                moved = 1;
            }
        }
        if (!moved) {
            break;
        }
    }
    *steps = n;
}

void pbt_int_shrink_core(pbt_big *current, const pbt_big *target, const pbt_big *lo, const pbt_big *hi,
                         pbt_big_falsifies_fn falsifies, void *self, int max_steps, int *steps)
{
    *steps = 0;
    if (max_steps == 0) {
        return; /* no budget, no shrinking */
    }
    if (in_bounds(target, lo, hi) && pbt_big_cmp(target, current) != 0 && falsifies(self, target)) {
        *current = *target; /* the target itself fails: cannot do better */
        *steps = 1;
        return;
    }
    pbt_int_walk(current, target, lo, hi, falsifies, self, max_steps, steps);
}

/* ---- sequence shrinker ----------------------------------------------------- */

static size_t seq_len(const pbt_val *v)
{
    return v->kind == PBT_V_BYTES ? v->u.bytes.n : v->u.list.n;
}

/* A view of src[off .. off+n): same kind, pointer into the same storage, owns nothing. */
static void seq_view(pbt_val *out, const pbt_val *src, size_t off, size_t n)
{
    pbt_val_nil(out);
    out->kind = src->kind;
    if (src->kind == PBT_V_BYTES) {
        out->u.bytes.p = src->u.bytes.p != NULL ? src->u.bytes.p + off : NULL;
        out->u.bytes.n = n;
    } else {
        out->u.list.items = src->u.list.items != NULL ? src->u.list.items + off : NULL;
        out->u.list.n = n;
    }
}

/* Keep the first n elements; free the rest. */
static void seq_truncate(pbt_val *v, size_t n)
{
    size_t i;
    if (v->kind == PBT_V_BYTES) {
        v->u.bytes.n = n;
        return;
    }
    for (i = n; i < v->u.list.n; i++) {
        pbt_val_free(&v->u.list.items[i]);
    }
    v->u.list.n = n;
}

/* Drop the first k elements (freeing them) and move the rest to the front. */
static void seq_drop_front(pbt_val *v, size_t k)
{
    size_t i;
    if (v->kind == PBT_V_BYTES) {
        memmove(v->u.bytes.p, v->u.bytes.p + k, v->u.bytes.n - k);
        v->u.bytes.n -= k;
        return;
    }
    for (i = 0; i < k; i++) {
        pbt_val_free(&v->u.list.items[i]);
    }
    memmove(v->u.list.items, v->u.list.items + k, (v->u.list.n - k) * sizeof(pbt_val));
    v->u.list.n -= k;
}

/* The element removed by a trial deletion, kept so a rejection can put it back. */
typedef struct detached {
    uint8_t byte;
    pbt_val val;
} detached;

static void seq_detach(pbt_val *v, size_t i, detached *d)
{
    if (v->kind == PBT_V_BYTES) {
        d->byte = v->u.bytes.p[i];
        memmove(v->u.bytes.p + i, v->u.bytes.p + i + 1, v->u.bytes.n - i - 1);
        v->u.bytes.n--;
    } else {
        d->val = v->u.list.items[i];
        memmove(v->u.list.items + i, v->u.list.items + i + 1, (v->u.list.n - i - 1) * sizeof(pbt_val));
        v->u.list.n--;
    }
}

static void seq_reattach(pbt_val *v, size_t i, const detached *d)
{
    if (v->kind == PBT_V_BYTES) {
        memmove(v->u.bytes.p + i + 1, v->u.bytes.p + i, v->u.bytes.n - i);
        v->u.bytes.p[i] = d->byte;
        v->u.bytes.n++;
    } else {
        memmove(v->u.list.items + i + 1, v->u.list.items + i, (v->u.list.n - i) * sizeof(pbt_val));
        v->u.list.items[i] = d->val;
        v->u.list.n++;
    }
}

static void seq_commit_detach(pbt_val *v, detached *d)
{
    if (v->kind == PBT_V_LIST) {
        pbt_val_free(&d->val);
    }
}

/* Context of an element probe: put `candidate` at index `at`, evaluate the whole sequence, restore. */
typedef struct elem_probe {
    pbt_budget *b;
    pbt_val *seq;
    pbt_pred falsifies;
    size_t at;
} elem_probe;

/* A byte candidate (integer walk over [0, 255]). */
static int probe_byte(void *self, const pbt_big *c)
{
    elem_probe *p = (elem_probe *)self;
    uint8_t old;
    int r;
    if (!pbt_budget_charge(p->b, p->seq->u.bytes.n)) {
        return 0;
    }
    old = p->seq->u.bytes.p[p->at];
    p->seq->u.bytes.p[p->at] = (uint8_t)pbt_big_low_u64(c);
    r = p->falsifies.fn(p->falsifies.self, p->b, p->seq);
    p->seq->u.bytes.p[p->at] = old;
    return r;
}

/* An integer candidate for a LIST of INT (unbounded integer walk). */
static int probe_list_int(void *self, const pbt_big *c)
{
    elem_probe *p = (elem_probe *)self;
    pbt_val saved;
    int r;
    if (!pbt_budget_charge(p->b, p->seq->u.list.n)) {
        return 0;
    }
    saved = p->seq->u.list.items[p->at];
    pbt_val_set_int(&p->seq->u.list.items[p->at], c);
    r = p->falsifies.fn(p->falsifies.self, p->b, p->seq);
    p->seq->u.list.items[p->at] = saved;
    return r;
}

/* A candidate for a generator-shrunk element (any value). */
static int probe_list_val(void *self, pbt_budget *b, const pbt_val *c)
{
    elem_probe *p = (elem_probe *)self;
    pbt_val saved;
    int r;
    (void)b;
    if (!pbt_budget_charge(p->b, p->seq->u.list.n)) {
        return 0;
    }
    saved = p->seq->u.list.items[p->at];
    p->seq->u.list.items[p->at] = *c; /* shallow alias; restored below, never freed */
    r = p->falsifies.fn(p->falsifies.self, p->b, p->seq);
    p->seq->u.list.items[p->at] = saved;
    return r;
}

int pbt_shrink_sequence(pbt_budget *b, pbt_val *seq, pbt_pred falsifies, int max_steps, size_t min_len,
                        const pbt_gen *elem, const pbt_big *int_target, int *steps)
{
    int n = 0;
    size_t floor_len = min_len > 1 ? min_len : 1;
    pbt_val view;
    *steps = 0;
    if (max_steps == 0) {
        return 0;
    }
    /* The initial copy is charged too; once the budget is spent the input is returned as is. */
    if (!pbt_budget_charge_copy(b, seq_len(seq))) {
        return 0;
    }

    /* Phase 0: the length-min_len prefix. */
    if (seq_len(seq) > min_len && pbt_budget_charge(b, min_len)) {
        seq_view(&view, seq, 0, min_len);
        if (falsifies.fn(falsifies.self, b, &view)) {
            seq_truncate(seq, min_len);
            *steps = 1;
            if (min_len == 0) {
                return 0;
            }
            n = 1;
        }
    }

    /* Phase 1: bisection, left half first, then right half. */
    while (seq_len(seq) > floor_len && n < max_steps) {
        size_t len = seq_len(seq);
        size_t mid = len / 2;
        size_t right = len - mid;
        if (mid >= min_len && pbt_budget_charge(b, mid)) {
            seq_view(&view, seq, 0, mid);
            if (falsifies.fn(falsifies.self, b, &view)) {
                seq_truncate(seq, mid);
                n++;
                continue;
            }
        }
        if (right >= min_len && pbt_budget_charge(b, right)) {
            seq_view(&view, seq, mid, right);
            if (falsifies.fn(falsifies.self, b, &view)) {
                seq_drop_front(seq, mid);
                n++;
                continue;
            }
        }
        break;
    }

    /* Phase 2: single-element deletion, right to left. */
    if (!pbt_hooks.skip_deletion) {
        size_t i = seq_len(seq); /* one past the candidate index: size_t cannot go below 0 */
        while (i > 0 && seq_len(seq) > floor_len && n < max_steps && pbt_budget_has_room(b)) {
            detached d;
            size_t at = i - 1;
            d.byte = 0;
            pbt_val_nil(&d.val);
            if (!pbt_budget_charge(b, seq_len(seq) - 1)) {
                break; /* charge BEFORE shifting: a spent budget must not cost another pass */
            }
            seq_detach(seq, at, &d);
            if (falsifies.fn(falsifies.self, b, seq)) {
                size_t new_len;
                seq_commit_detach(seq, &d);
                n++;
                /* The next candidate is min(at, new_len - 1) (0-based); `i` is that plus one. */
                new_len = seq_len(seq);
                i = at + 1 < new_len ? at + 1 : new_len;
            } else {
                seq_reattach(seq, at, &d);
                i--;
            }
        }
    }

    /* Phase 3: element-wise, left to right. */
    if (elem != NULL || seq->kind == PBT_V_BYTES || int_target != NULL) {
        size_t size = seq_len(seq);
        size_t index;
        for (index = 0; index < size; index++) {
            int remaining = max_steps - n;
            int used = 0;
            elem_probe probe;
            if (remaining <= 0 || !pbt_budget_has_room(b)) {
                break;
            }
            probe.b = b;
            probe.seq = seq;
            probe.falsifies = falsifies;
            probe.at = index;
            if (seq->kind == PBT_V_BYTES) {
                pbt_big cur;
                pbt_big zero;
                pbt_big top;
                pbt_big_set_u64(&cur, seq->u.bytes.p[index]);
                pbt_big_zero(&zero);
                pbt_big_set_u64(&top, 255);
                pbt_int_walk(&cur, &zero, &zero, &top, probe_byte, &probe, remaining, &used);
                if (used > 0) {
                    seq->u.bytes.p[index] = (uint8_t)pbt_big_low_u64(&cur);
                }
            } else if (elem != NULL) {
                pbt_pred ep;
                ep.fn = probe_list_val;
                ep.self = &probe;
                (void)pbt_gen_shrink_nested(b, elem, &seq->u.list.items[index], ep, remaining, &used);
            } else if (seq->u.list.items[index].kind == PBT_V_INT) {
                pbt_big cur = seq->u.list.items[index].u.i;
                pbt_int_walk(&cur, int_target, NULL, NULL, probe_list_int, &probe, remaining, &used);
                if (used > 0) {
                    pbt_val_set_int(&seq->u.list.items[index], &cur);
                }
            }
            if (used > 0) {
                n += used;
            }
        }
    }
    *steps = n;
    return 0;
}

/* ============================================================================
 * Standalone shrinkers (the corpus entry points)
 *
 * Each is a PUBLIC ENTRY that runs user code, so each opens a child scope of the
 * scope it is given and charges every property evaluation, the initial one
 * included (VER05i section 6).
 * ========================================================================== */

static int check_steps(int max_steps)
{
    return max_steps < 0 || max_steps > PBT_MAX_SHRINK_STEPS ? PBT_E_ARG : 0;
}

/* A candidate integer: charge one attempt, then ask the user's property (1 holds, so 0 means it falsifies). */
typedef struct int_prop_ctx {
    pbt_budget *b;
    pbt_prop prop;
} int_prop_ctx;

static int int_prop_falsifies(void *self, const pbt_big *c)
{
    int_prop_ctx *x = (int_prop_ctx *)self;
    pbt_val v;
    if (!pbt_budget_charge(x->b, 1)) {
        return 0;
    }
    pbt_val_set_int(&v, c);
    return x->prop.fn(x->prop.user, x->b, &v) == 0;
}

int pbt_shrink_int(pbt_budget *parent, const pbt_big *initial, const pbt_big *target, const pbt_big *lo,
                   const pbt_big *hi, pbt_prop prop, int max_steps, pbt_big *minimal, int *steps)
{
    pbt_budget storage;
    pbt_budget *b;
    int_prop_ctx ctx;
    pbt_big cur;
    pbt_val v;
    int rc;
    if (initial == NULL || target == NULL || lo == NULL || hi == NULL || minimal == NULL || steps == NULL) {
        return PBT_E_ARG;
    }
    *steps = 0;
    *minimal = *initial;
    if (prop.fn == NULL || check_steps(max_steps) != 0 || pbt_big_cmp(lo, hi) > 0 || !in_bounds(initial, lo, hi)) {
        return PBT_E_ARG;
    }
    b = pbt_budget_open_child(&storage, parent, max_steps);
    ctx.b = b;
    ctx.prop = prop;
    if (!pbt_budget_charge(b, 1)) {
        pbt_budget_close(&storage);
        return 0; /* starved by an enclosing shrink: the start is returned unchanged */
    }
    pbt_val_set_int(&v, initial);
    rc = prop.fn(prop.user, b, &v);
    if (rc != 0) {
        pbt_budget_close(&storage);
        return rc > 0 ? PBT_E_NOT_FALSIFYING : PBT_E_GEN;
    }
    cur = *initial;
    pbt_int_shrink_core(&cur, target, lo, hi, int_prop_falsifies, &ctx, max_steps, steps);
    *minimal = cur;
    pbt_budget_close(&storage);
    return 0;
}

/* A sequence is falsified when the user's property (1 = holds) returns 0 for it. */
typedef struct seq_prop_ctx {
    pbt_prop prop;
} seq_prop_ctx;

static int seq_prop_falsifies(void *self, pbt_budget *b, const pbt_val *v)
{
    seq_prop_ctx *x = (seq_prop_ctx *)self;
    return x->prop.fn(x->prop.user, b, v) == 0;
}

int pbt_shrink_list_ints(pbt_budget *parent, pbt_val *list, const pbt_big *elem_target, pbt_prop prop, int max_steps,
                         int *steps)
{
    pbt_budget storage;
    pbt_budget *b;
    seq_prop_ctx ctx;
    pbt_pred pred;
    size_t i;
    int rc;
    *steps = 0;
    if (prop.fn == NULL || list->kind != PBT_V_LIST || check_steps(max_steps) != 0) {
        return PBT_E_ARG;
    }
    for (i = 0; i < list->u.list.n; i++) {
        if (list->u.list.items[i].kind != PBT_V_INT) {
            return PBT_E_ARG;
        }
    }
    b = pbt_budget_open_child(&storage, parent, max_steps);
    if (!pbt_budget_charge_copy(b, 2u * (uint64_t)list->u.list.n) || !pbt_budget_charge(b, 1)) {
        pbt_budget_close(&storage);
        return 0;
    }
    rc = prop.fn(prop.user, b, list);
    if (rc != 0) {
        pbt_budget_close(&storage);
        return rc > 0 ? PBT_E_NOT_FALSIFYING : PBT_E_GEN;
    }
    ctx.prop = prop;
    pred.fn = seq_prop_falsifies;
    pred.self = &ctx;
    rc = pbt_shrink_sequence(b, list, pred, max_steps, 0, NULL, elem_target, steps);
    pbt_budget_close(&storage);
    return rc;
}

int pbt_shrink_bytes(pbt_budget *parent, pbt_val *bytes, pbt_prop prop, int max_steps, int *steps)
{
    pbt_budget storage;
    pbt_budget *b;
    seq_prop_ctx ctx;
    pbt_pred pred;
    int rc;
    *steps = 0;
    if (prop.fn == NULL || bytes->kind != PBT_V_BYTES || check_steps(max_steps) != 0) {
        return PBT_E_ARG;
    }
    b = pbt_budget_open_child(&storage, parent, max_steps);
    if (!pbt_budget_charge_copy(b, bytes->u.bytes.n) || !pbt_budget_charge(b, 1)) {
        pbt_budget_close(&storage);
        return 0;
    }
    rc = prop.fn(prop.user, b, bytes);
    if (rc != 0) {
        pbt_budget_close(&storage);
        return rc > 0 ? PBT_E_NOT_FALSIFYING : PBT_E_GEN;
    }
    ctx.prop = prop;
    pred.fn = seq_prop_falsifies;
    pred.self = &ctx;
    rc = pbt_shrink_sequence(b, bytes, pred, max_steps, 0, NULL, NULL, steps);
    pbt_budget_close(&storage);
    return rc;
}

int pbt_simulate_replay(pbt_budget *parent, uint64_t seed, pbt_prop prop, pbt_big *initial_fail, pbt_big *minimal)
{
    pbt_budget storage;
    pbt_budget *b;
    pbt_rng rng;
    pbt_big lo;
    pbt_big hi;
    pbt_big zero;
    int found = 0;
    int i;
    int steps = 0;
    int rc;
    pbt_big_zero(initial_fail);
    pbt_big_zero(minimal);
    if (prop.fn == NULL) {
        return PBT_E_ARG;
    }
    b = pbt_budget_open_child(&storage, parent, 1000);
    pbt_rng_init(&rng, seed);
    for (i = 0; i < 1000 && pbt_budget_charge(b, 1); i++) {
        pbt_val v;
        pbt_val_set_u64(&v, pbt_rng_bounded(&rng, 1000));
        if (prop.fn(prop.user, b, &v) == 0) {
            *initial_fail = v.u.i;
            found = 1;
            break;
        }
    }
    if (!found) {
        pbt_budget_close(&storage);
        return PBT_E_ARG; /* the property was not falsified within 1000 draws */
    }
    pbt_big_zero(&zero);
    pbt_big_zero(&lo);
    pbt_big_set_u64(&hi, 1000);
    rc = pbt_shrink_int(b, initial_fail, &zero, &lo, &hi, prop, 1000, minimal, &steps);
    pbt_budget_close(&storage);
    return rc;
}
