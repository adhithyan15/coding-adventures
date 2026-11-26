/*
 * fuzz_engine.c -- handles, the executor, minimisation and the campaign (VER06 sections 5-7, VER06j 5-6).
 *
 * THE CAMPAIGN IN ONE PICTURE
 * ---------------------------
 *
 *   seeds --triage--> corpus ----pick (random)----> mutate_sequence (1-4 steps) --> maybe splice a donor
 *                       ^                                                              |
 *                       |  new edges?                                                  v
 *                       +------- admit  <------ pass ------ run the target ---> crash / timeout / oom
 *                                                                                   |
 *                                                          dedup by signature, shrink the first of each
 *
 * Everything is a pure function of (corpus, options, target): the same seed gives the same report.
 *
 * ONE HANDLE PER EXECUTION
 * ------------------------
 * The target receives an fz_handle * to report edges and to count work and memory.  A handle is issued
 * fresh for each execution and RETIRED the moment the target returns.  A target that kept the pointer and
 * calls it later finds a retired handle: fz_edge does nothing, fz_tick / fz_reserve say "exhausted" and count
 * nothing.  Handles live in static storage (a ring of 4,096 slots), never on a stack, so a stale pointer is
 * memory-safe even after the campaign has returned.  Each handle has an epoch number that is never reused.
 *
 * STICKY EXHAUSTION
 * -----------------
 * The first budget to break (steps or memory) is remembered.  From then on BOTH fz_tick and fz_reserve answer
 * "exhausted" at once, and when the target returns the execution is a TIMEOUT or OOM (by the budget that broke
 * first) whatever the target returned: a target that ignored the answer cannot turn the run into a pass or a
 * crash.
 */
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fuzz.h"
#include "fuzz_hooks.h"
#include "fuzz_internal.h"
#include "pbt_internal.h"

/* ---- the handle ring ------------------------------------------------------------------------------------ */

static fz_handle g_ring[FZ_HANDLE_RING];
static size_t g_ring_next;
static uint64_t g_epoch;
static fz_handle *g_active;

fz_handle *fz_handle_issue(fz_bitmap *cov, uint64_t step_budget, uint64_t mem_budget, void *user)
{
    size_t i;
    for (i = 0; i < FZ_HANDLE_RING; i++) {
        size_t slot = (g_ring_next + i) % FZ_HANDLE_RING;
        fz_handle *h = &g_ring[slot];
        if (!h->live) {
            g_ring_next = (slot + 1) % FZ_HANDLE_RING;
            memset(h, 0, sizeof *h);
            h->live = 1;
            h->epoch = ++g_epoch;
            h->step_budget = step_budget;
            h->mem_budget = mem_budget;
            h->cov = cov;
            h->user = user;
            return h;
        }
    }
    return NULL;
}

int fz_handle_retire(fz_handle *h, int *sancov)
{
    int state = h->exhausted;
    if (sancov != NULL) {
        *sancov = h->sancov;
    }
    h->live = 0;
    h->cov = NULL;
    h->user = NULL;
    return state;
}

fz_handle *fz_handle_set_active(fz_handle *h)
{
    fz_handle *prev = g_active;
    g_active = h;
    return prev;
}

size_t fz_handles_live(void)
{
    size_t n = 0;
    size_t i;
    for (i = 0; i < FZ_HANDLE_RING; i++) {
        n += g_ring[i].live ? 1u : 0u;
    }
    return n;
}

fz_handle *fz_active_handle(void)
{
    return g_active;
}

void fz_edge(fz_handle *h, uint64_t id)
{
    if (h != NULL && h->live && h->cov != NULL) {
        fz_bitmap_record(h->cov, id);
    }
}

void fz_edge_sancov(fz_handle *h, uint32_t guard_id)
{
    if (h != NULL && h->live && h->cov != NULL) {
        fz_bitmap_record(h->cov, ((uint64_t)guard_id * 0x9E3779B97F4A7C15u) >> 48);
        h->sancov = 1;
    }
}

int fz_tick(fz_handle *h)
{
    if (h == NULL || !h->live || h->exhausted != 0) {
        return 1;
    }
    if (h->steps >= h->step_budget) {
        h->exhausted = 1;
        return 1;
    }
    h->steps++;
    return 0;
}

int fz_reserve(fz_handle *h, size_t n)
{
    if (h == NULL || !h->live || h->exhausted != 0) {
        return 1;
    }
    /* overflow-safe: never compute reserved + n */
    if ((uint64_t)n > h->mem_budget - h->reserved) {
        h->exhausted = 2;
        return 1;
    }
    h->reserved += n;
    return 0;
}

void *fz_handle_user(const fz_handle *h)
{
    return (h != NULL && h->live) ? h->user : NULL;
}

uint64_t fz_handle_epoch(const fz_handle *h)
{
    return h != NULL ? h->epoch : 0;
}

/* ---- options -------------------------------------------------------------------------------------------- */

fz_options fz_options_default(void)
{
    fz_options o;
    o.seed = 42;
    o.max_iterations = 1000;
    o.max_input_len = 4096;
    o.base_energy = 10;
    o.minimize_budget = FZ_DEFAULT_MINIMIZE_BUDGET;
    o.step_budget = FZ_DEFAULT_STEP_BUDGET;
    o.mem_budget = FZ_DEFAULT_MEM_BUDGET;
    o.user = NULL;
    return o;
}

int fz_options_validate(const fz_options *o)
{
    if (o == NULL) {
        return FZ_E_ARG;
    }
    if (o->max_iterations > FZ_MAX_ITERATIONS || o->max_input_len > FZ_MAX_INPUT_BYTES ||
        o->base_energy > FZ_MAX_ITERATIONS || o->minimize_budget > FZ_MAX_MINIMIZE_BUDGET ||
        o->step_budget > FZ_MAX_STEP_BUDGET || o->mem_budget > FZ_MAX_MEM_BUDGET) {
        return FZ_E_ARG;
    }
    return FZ_OK;
}

/* ---- the report ----------------------------------------------------------------------------------------- */

void fz_report_init(fz_report *r)
{
    memset(r, 0, sizeof *r);
    r->tier = FZ_TIER_BLACKBOX;
}

void fz_report_free(fz_report *r)
{
    size_t i;
    for (i = 0; i < r->ncrashes; i++) {
        pbt_free(r->crashes[i].minimal);
    }
    for (i = 0; i < r->ncorpus; i++) {
        pbt_free(r->corpus[i].p);
    }
    pbt_free(r->crashes);
    pbt_free(r->corpus);
    fz_report_init(r);
}

int fz_report_is_clean(const fz_report *r)
{
    return r->iterations > 0 && r->ncrashes == 0 && r->timeouts == 0 && r->ooms == 0 && r->crashes_dropped == 0;
}

size_t fz_report_unique_crashes(const fz_report *r)
{
    return r->ncrashes + r->crashes_dropped;
}

size_t fz_report_summary(const fz_report *r, char *buf, size_t cap)
{
    int n = snprintf(buf, cap,
                     "fuzz.Report{iterations=%lu admitted=%lu edges=%lu crashes=%lu timeouts=%lu ooms=%lu "
                     "dropped=%lu admissions_dropped=%lu tier=%s clean=%d}",
                     (unsigned long)r->iterations, (unsigned long)r->admitted_seeds, (unsigned long)r->total_edges,
                     (unsigned long)r->ncrashes, (unsigned long)r->timeouts, (unsigned long)r->ooms,
                     (unsigned long)r->crashes_dropped, (unsigned long)r->admissions_dropped, r->tier,
                     fz_report_is_clean(r));
    return n < 0 ? 0 : (size_t)n;
}

size_t fz_tap_failure_count(const fz_report *r)
{
    return r->ncrashes + (r->timeouts > 0 ? 1u : 0u) + (r->ooms > 0 ? 1u : 0u);
}

int fz_tap_failure_line(const fz_report *r, size_t index, const char *target, int first_test, char *buf, size_t cap)
{
    char sig[9];
    const char *id = FZ_FID_CRASH;
    int rc;
    if (index >= fz_tap_failure_count(r) || index > (size_t)INT_MAX || first_test < 0 ||
        first_test > INT_MAX - (int)index) {
        return FZ_E_ARG; /* out of range, or first_test + index would overflow an int */
    }
    if (index < r->ncrashes) {
        memcpy(sig, r->crashes[index].sig, 9);
    } else {
        int is_timeout = r->timeouts > 0 && index == r->ncrashes;
        const char *kind = is_timeout ? "timeout" : "oom";
        id = is_timeout ? FZ_FID_TIMEOUT : FZ_FID_OOM;
        fz_crash_signature(sig, kind, NULL, 0);
    }
    rc = fz_format_tap_failure(buf, cap, first_test + (int)index, target, strlen(target), id, strlen(id), sig, 8);
    return rc;
}

/* ---- running one input ------------------------------------------------------------------------------- */

typedef struct run_ctx {
    fz_target_fn target;
    const fz_options *opts;
    int sancov;
} run_ctx;

/*
 * Copy the input, issue a handle, call the target, retire the handle and classify.  The copy means a target
 * that writes through its pointer (or keeps it) cannot corrupt the corpus; an allocation failure of the copy
 * is the ENGINE's out-of-memory, not the target's.
 */
static int exec_target(void *user, const uint8_t *data, size_t len, fz_bitmap *cov, fz_outcome *out)
{
    run_ctx *rc = (run_ctx *)user;
    uint8_t *copy = (uint8_t *)pbt_alloc(len, 1);
    fz_handle *h;
    fz_handle *prev;
    int ret;
    int state;
    int sancov = 0;
    if (copy == NULL) {
        return FZ_E_NOMEM;
    }
    if (len > 0) {
        memcpy(copy, data, len);
    }
    h = fz_handle_issue(cov, rc->opts->step_budget, rc->opts->mem_budget, rc->opts->user);
    if (h == NULL) {
        pbt_free(copy);
        return FZ_E_LIMIT;
    }
    memset(&out->crash, 0, sizeof out->crash);
    prev = fz_handle_set_active(h);
    ret = rc->target(copy, len, h, &out->crash);
    (void)fz_handle_set_active(prev);
    state = fz_handle_retire(h, &sancov);
    pbt_free(copy);
    if (sancov) {
        rc->sancov = 1;
    }
    if (state == 1) {
        out->kind = FZ_TIMEOUT;
    } else if (state == 2) {
        out->kind = FZ_OOM;
    } else if (ret != 0) {
        out->kind = FZ_CRASH;
        fz_crash_normalize(&out->crash);
    } else {
        out->kind = FZ_PASS;
    }
    return FZ_OK;
}

/* ---- minimisation (VER06 section 5.3) ---------------------------------------------------------------- */

/* Does running the candidate crash with exactly this signature?  Counts one execution. */
static int reproduces(fz_test_fn test, void *user, const uint8_t *d, size_t n, const char *sig, size_t *spent,
                      int *same)
{
    fz_outcome o;
    char got[9];
    int rc;
    (*spent)++;
    o.kind = FZ_PASS;
    o.crash.kind[0] = '\0';
    o.crash.nframes = 0;
    rc = test(user, d, n, &o);
    if (rc != FZ_OK) {
        return rc;
    }
    *same = 0;
    if (o.kind == FZ_CRASH) {
        fz_crash_normalize(&o.crash);
        fz_crash_signature_of(got, &o.crash);
        *same = memcmp(got, sig, 8) == 0;
    }
    return FZ_OK;
}

/*
 * Shrink while keeping the EXACT signature.  Pass 1 deletes blocks, halving the block size (starting at
 * max(1, len/2): starting at len/2 would make the empty input unreachable from a 1-byte crasher); pass 2 zeroes
 * single bytes.  Every candidate execution counts against `budget`, accepted or rejected, and the budget is
 * checked BEFORE each.
 */
static int minimize_core(fz_bytes *out, const uint8_t *data, size_t len, const char *sig, fz_test_fn test,
                         void *user, size_t budget, size_t *spent_out, int *finished_out)
{
    uint8_t *cur = (uint8_t *)pbt_alloc(len, 1);
    uint8_t *cand = (uint8_t *)pbt_alloc(len, 1);
    size_t curlen = len;
    size_t spent = 0;
    int finished = 1;
    int rc = FZ_OK;
    size_t chunk;
    size_t offset;
    size_t i;
    out->p = NULL;
    out->n = 0;
    if (cur == NULL || cand == NULL) {
        pbt_free(cur);
        pbt_free(cand);
        return FZ_E_NOMEM;
    }
    if (len > 0) {
        memcpy(cur, data, len);
    }
    if (len == 0) {
        goto done;
    }
    for (chunk = len / 2 > 1 ? len / 2 : 1; chunk > 0; chunk /= 2) {
        for (offset = 0; offset < curlen;) {
            size_t end;
            int same = 0;
            if (spent >= budget) {
                finished = 0;
                goto done;
            }
            end = offset + (chunk < curlen - offset ? chunk : curlen - offset);
            memcpy(cand, cur, offset);
            memcpy(cand + offset, cur + end, curlen - end);
            rc = reproduces(test, user, cand, curlen - (end - offset), sig, &spent, &same);
            if (rc != FZ_OK) {
                goto done;
            }
            if (same) {
                uint8_t *t = cur;
                cur = cand;
                cand = t;
                curlen -= end - offset; /* same offset again: the tail slid left */
                continue;
            }
            offset += chunk;
        }
    }
    for (i = 0; i < curlen; i++) {
        uint8_t original;
        int same = 0;
        if (cur[i] == 0) {
            continue;
        }
        if (spent >= budget) {
            finished = 0;
            goto done;
        }
        original = cur[i];
        cur[i] = 0;
        rc = reproduces(test, user, cur, curlen, sig, &spent, &same);
        if (rc != FZ_OK) {
            goto done;
        }
        if (!same) {
            cur[i] = original;
        }
    }
done:
    pbt_free(cand);
    if (rc != FZ_OK) {
        pbt_free(cur);
        return rc;
    }
    out->p = cur;
    out->n = curlen;
    if (spent_out != NULL) {
        *spent_out = spent;
    }
    if (finished_out != NULL) {
        *finished_out = finished;
    }
    return FZ_OK;
}

int fz_minimize_crashing_input(fz_bytes *out, const uint8_t *data, size_t len, const char *sig, fz_test_fn test,
                               void *user, size_t budget)
{
    size_t i;
    out->p = NULL;
    out->n = 0;
    if (budget > FZ_MAX_MINIMIZE_BUDGET || test == NULL || sig == NULL || (data == NULL && len > 0) ||
        len > FZ_MAX_MUTATE_BYTES) {
        return FZ_E_ARG;
    }
    for (i = 0; i < 8; i++) {
        if (!((sig[i] >= '0' && sig[i] <= '9') || (sig[i] >= 'a' && sig[i] <= 'f'))) {
            return FZ_E_ARG;
        }
    }
    return minimize_core(out, data, len, sig, test, user, budget, NULL, NULL);
}

/* ---- the campaign --------------------------------------------------------------------------------------- */

typedef struct campaign {
    fz_exec_fn exec;
    void *exec_user;
    fz_options opts;
    pbt_rng rng;
    fz_bitmap *step; /* edges of the current execution */
    fz_bitmap *cum;
    fz_report rep;
    size_t cap_corpus;
    size_t cap_crashes;
    uint64_t retained;
    size_t minimize_left;
} campaign;

static uint64_t retained_cap(void)
{
    return fz_hooks.max_retained_bytes != 0 ? fz_hooks.max_retained_bytes : FZ_MAX_RETAINED_BYTES;
}

static size_t crash_cap(void)
{
    return fz_hooks.max_crashes != 0 ? fz_hooks.max_crashes : FZ_MAX_CRASHES;
}

/* Run one input and validate the outcome kind.  The edges of the run are left in c->step. */
static int execute(campaign *c, const uint8_t *d, size_t n, fz_outcome *o)
{
    int rc;
    fz_bitmap_reset(c->step);
    o->kind = FZ_PASS;
    o->crash.kind[0] = '\0';
    o->crash.nframes = 0;
    rc = c->exec(c->exec_user, d, n, c->step, o);
    if (rc != FZ_OK) {
        return rc;
    }
    if (o->kind < FZ_PASS || o->kind > FZ_OOM) {
        return FZ_E_ARG;
    }
    if (o->kind == FZ_CRASH) {
        fz_crash_normalize(&o->crash);
    }
    return FZ_OK;
}

/* The minimiser's re-run: coverage is discarded and nothing is counted. */
static int probe(void *user, const uint8_t *d, size_t n, fz_outcome *o)
{
    return execute((campaign *)user, d, n, o);
}

/* A passing input joins the corpus iff it covered a new edge.  The first passing seed is always admitted, so there
 * is something to mutate in black-box mode. */
static int admit(campaign *c, const uint8_t *d, size_t n, size_t generation)
{
    size_t fresh = 0;
    int has_new = fz_bitmap_admit(c->cum, c->step->edges, c->step->nedges, &fresh);
    fz_corpus_entry *e;
    if (!(has_new || (generation == 0 && c->rep.ncorpus == 0))) {
        return FZ_OK;
    }
    if ((uint64_t)n > retained_cap() - c->retained) {
        c->rep.admissions_dropped++;
        return FZ_OK;
    }
    if (c->rep.ncorpus == c->cap_corpus) {
        size_t cap = c->cap_corpus == 0 ? 16 : c->cap_corpus * 2;
        fz_corpus_entry *grown = (fz_corpus_entry *)pbt_realloc(c->rep.corpus, cap, sizeof *grown);
        if (grown == NULL) {
            return FZ_E_NOMEM;
        }
        c->rep.corpus = grown;
        c->cap_corpus = cap;
    }
    e = &c->rep.corpus[c->rep.ncorpus];
    e->p = (uint8_t *)pbt_alloc(n, 1);
    if (e->p == NULL) {
        return FZ_E_NOMEM;
    }
    if (n > 0) {
        memcpy(e->p, d, n);
    }
    e->n = n;
    e->new_edges = fresh;
    e->generation = generation;
    c->rep.ncorpus++;
    c->retained += n;
    return FZ_OK;
}

static int record_crash(campaign *c, const fz_outcome *o, const uint8_t *d, size_t n)
{
    char sig[9];
    size_t i;
    fz_crash_record *rec;
    fz_bytes minimal;
    size_t allowance;
    size_t spent = 0;
    int finished = 0;
    int rc;
    fz_crash_signature_of(sig, &o->crash);
    for (i = 0; i < c->rep.ncrashes; i++) {
        if (memcmp(c->rep.crashes[i].sig, sig, 9) == 0) {
            rec = &c->rep.crashes[i];
            rec->hits++;
            if (rec->minimal != NULL && n < rec->minimal_len) {
                uint8_t *copy = (uint8_t *)pbt_alloc(n, 1);
                if (copy == NULL) {
                    return FZ_E_NOMEM;
                }
                if (n > 0) {
                    memcpy(copy, d, n);
                }
                c->retained -= rec->minimal_len - n;
                pbt_free(rec->minimal);
                rec->minimal = copy;
                rec->minimal_len = n;
            }
            return FZ_OK;
        }
    }
    if (c->rep.ncrashes >= crash_cap()) {
        c->rep.crashes_dropped++;
        return FZ_OK;
    }
    if (c->rep.ncrashes == c->cap_crashes) {
        size_t cap = c->cap_crashes == 0 ? 8 : c->cap_crashes * 2;
        fz_crash_record *grown = (fz_crash_record *)pbt_realloc(c->rep.crashes, cap, sizeof *grown);
        if (grown == NULL) {
            return FZ_E_NOMEM;
        }
        c->rep.crashes = grown;
        c->cap_crashes = cap;
    }
    rec = &c->rep.crashes[c->rep.ncrashes];
    memset(rec, 0, sizeof *rec);
    memcpy(rec->sig, sig, 9);
    rec->crash = o->crash;
    rec->hits = 1;
    if ((uint64_t)n > retained_cap() - c->retained) {
        /* the retained-bytes cap: keep the signature, keep no input, do not minimise */
        rec->input_dropped = 1;
        c->rep.ncrashes++;
        return FZ_OK;
    }
    allowance = c->opts.minimize_budget < c->minimize_left ? c->opts.minimize_budget : c->minimize_left;
    rc = minimize_core(&minimal, d, n, sig, probe, c, allowance, &spent, &finished);
    if (rc != FZ_OK) {
        return rc;
    }
    c->minimize_left -= spent;
    rec->minimal = minimal.p;
    rec->minimal_len = minimal.n;
    rec->minimized = finished;
    c->retained += minimal.n;
    c->rep.ncrashes++;
    return FZ_OK;
}

/* Put one non-passing execution in its bucket. */
static int file_outcome(campaign *c, const fz_outcome *o, const uint8_t *d, size_t n)
{
    switch (o->kind) {
    case FZ_TIMEOUT:
        c->rep.timeouts++;
        return FZ_OK;
    case FZ_OOM:
        c->rep.ooms++;
        return FZ_OK;
    case FZ_CRASH:
        return record_crash(c, o, d, n);
    default:
        return FZ_OK;
    }
}

/*
 * Per iteration the stream draws: bounded(len(corpus)) for the seed; per energy slot 1+bounded(4) steps, one
 * next() sub-seed, then, if the corpus holds more than one seed, bounded(4) and on 0 a donor index and two split
 * draws (as Rust, Python, Go).
 */
static int run_loop(campaign *c, const fz_input *seeds, size_t nseeds)
{
    static const uint8_t zero = 0;
    fz_input fallback;
    fz_outcome *o = (fz_outcome *)pbt_alloc(1, sizeof *o);
    size_t s;
    size_t iterations = 0;
    int rc = FZ_OK;
    if (o == NULL) {
        return FZ_E_NOMEM;
    }
    if (nseeds == 0) {
        fallback.p = &zero;
        fallback.n = 1;
        seeds = &fallback;
        nseeds = 1;
    }
    /* 1. Triage: run every seed once. */
    for (s = 0; s < nseeds && rc == FZ_OK; s++) {
        rc = execute(c, seeds[s].p, seeds[s].n, o);
        if (rc != FZ_OK) {
            break;
        }
        rc = o->kind == FZ_PASS ? admit(c, seeds[s].p, seeds[s].n, 0) : file_outcome(c, o, seeds[s].p, seeds[s].n);
    }
    /* 2. The mutation loop. */
    while (rc == FZ_OK && iterations < c->opts.max_iterations && c->rep.ncorpus > 0) {
        size_t pick = (size_t)pbt_rng_bounded(&c->rng, c->rep.ncorpus);
        const uint8_t *sp = c->rep.corpus[pick].p;
        size_t sn = c->rep.corpus[pick].n;
        size_t sgen = c->rep.corpus[pick].generation;
        size_t energy = fz_calculate_energy(c->opts.base_energy, sn, c->rep.corpus[pick].new_edges,
                                            fz_bitmap_count(c->cum));
        size_t slot;
        for (slot = 0; slot < energy && iterations < c->opts.max_iterations && rc == FZ_OK; slot++) {
            fz_bytes cand;
            size_t steps;
            uint64_t sub;
            iterations++;
            steps = 1 + (size_t)pbt_rng_bounded(&c->rng, 4);
            sub = pbt_rng_next(&c->rng);
            rc = fz_mutate_sequence(&cand, sp, sn, sub, steps, FZ_MAX_INPUT_BYTES);
            if (rc != FZ_OK) {
                break;
            }
            if (c->rep.ncorpus > 1 && pbt_rng_bounded(&c->rng, 4) == 0) {
                size_t di = (size_t)pbt_rng_bounded(&c->rng, c->rep.ncorpus);
                const uint8_t *dp = c->rep.corpus[di].p;
                size_t dn = c->rep.corpus[di].n;
                int64_t ts = (int64_t)pbt_rng_bounded(&c->rng, (uint64_t)cand.n + 1);
                int64_t ds = (int64_t)pbt_rng_bounded(&c->rng, (uint64_t)dn + 1);
                fz_bytes sp2;
                rc = fz_splice(&sp2, cand.p, cand.n, dp, dn, ts, ds);
                fz_bytes_free(&cand);
                if (rc != FZ_OK) {
                    break;
                }
                cand = sp2;
            }
            if (cand.n > c->opts.max_input_len) {
                cand.n = c->opts.max_input_len;
            }
            rc = execute(c, cand.p, cand.n, o);
            if (rc == FZ_OK) {
                rc = o->kind == FZ_PASS ? admit(c, cand.p, cand.n, sgen + 1) : file_outcome(c, o, cand.p, cand.n);
            }
            fz_bytes_free(&cand);
        }
    }
    c->rep.iterations = iterations;
    pbt_free(o);
    return rc;
}

int fz_run_outcomes(const fz_input *corpus, size_t ncorpus, const fz_options *opts, fz_exec_fn exec, void *exec_user,
                    fz_report *out)
{
    campaign *c;
    size_t i;
    int rc;
    if (out == NULL) {
        return FZ_E_ARG;
    }
    fz_report_init(out);
    if (opts == NULL || exec == NULL || fz_options_validate(opts) != FZ_OK || ncorpus > FZ_MAX_INITIAL_CORPUS ||
        (corpus == NULL && ncorpus > 0)) {
        return FZ_E_ARG;
    }
    for (i = 0; i < ncorpus; i++) {
        if (corpus[i].n > FZ_MAX_INPUT_BYTES || (corpus[i].p == NULL && corpus[i].n > 0)) {
            return FZ_E_ARG;
        }
    }
    c = (campaign *)pbt_alloc(1, sizeof *c);
    if (c == NULL) {
        return FZ_E_NOMEM;
    }
    memset(c, 0, sizeof *c);
    c->step = (fz_bitmap *)pbt_alloc(1, sizeof *c->step);
    c->cum = (fz_bitmap *)pbt_alloc(1, sizeof *c->cum);
    if (c->step == NULL || c->cum == NULL) {
        pbt_free(c->step);
        pbt_free(c->cum);
        pbt_free(c);
        return FZ_E_NOMEM;
    }
    fz_bitmap_init(c->step);
    fz_bitmap_init(c->cum);
    fz_report_init(&c->rep);
    c->exec = exec;
    c->exec_user = exec_user;
    c->opts = *opts;
    c->minimize_left = opts->max_iterations;
    pbt_rng_init(&c->rng, opts->seed);
    rc = run_loop(c, corpus, ncorpus);
    if (rc == FZ_OK) {
        c->rep.total_edges = fz_bitmap_count(c->cum);
        c->rep.admitted_seeds = c->rep.ncorpus;
        c->rep.tier = c->rep.total_edges > 0 ? FZ_TIER_CALLER : FZ_TIER_BLACKBOX;
        *out = c->rep;
    } else {
        fz_report_free(&c->rep);
    }
    pbt_free(c->step);
    pbt_free(c->cum);
    pbt_free(c);
    return rc;
}

int fz_run(const fz_input *corpus, size_t ncorpus, const fz_options *opts, fz_target_fn target, fz_report *out)
{
    run_ctx ctx;
    int rc;
    if (target == NULL || opts == NULL) {
        if (out != NULL) {
            fz_report_init(out);
        }
        return FZ_E_ARG;
    }
    ctx.target = target;
    ctx.opts = opts;
    ctx.sancov = 0;
    rc = fz_run_outcomes(corpus, ncorpus, opts, exec_target, &ctx, out);
    if (rc == FZ_OK && ctx.sancov && out->total_edges > 0) {
        out->tier = FZ_TIER_SANCOV;
    }
    return rc;
}
