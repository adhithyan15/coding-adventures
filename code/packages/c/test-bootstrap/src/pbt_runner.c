/*
 * pbt_runner.c -- the honest runner (VER05 section 7, VER05i section 7).
 *
 * "Verdict honesty" means the engine never says PASS unless every case really
 * ran and held:
 *
 *   - zero cases evaluated is property-exhausted, never a pass;
 *   - a generator error (including out-of-memory) is property-exhausted;
 *   - a property that reports an error (negative return) is property-panic;
 *   - a property that returns 0 is shrunk, then property-counterexample;
 *   - the reported minimal witness is RE-VERIFIED: if the shrinker fails or
 *     lies (returns a value that no longer fails, a nonsense step count), the
 *     ORIGINAL candidate is reported with 0 steps;
 *   - a nested check whose enclosing budget is spent is exhausted, never a pass;
 *   - redacted (CI) mode never emits the witness or the seed.
 *
 * The whole body of a check runs in ONE budget scope (case loop included): a
 * root when there is no parent, a child of the parent's scope otherwise.
 */
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#include "pbt.h"
#include "pbt_hooks.h"
#include "pbt_internal.h"

/* ---- failure vocabulary ----------------------------------------------------- */

int pbt_is_failure_id(const char *candidate)
{
    return candidate != NULL &&
           (strcmp(candidate, PBT_FAIL_COUNTEREXAMPLE) == 0 || strcmp(candidate, PBT_FAIL_EXHAUSTED) == 0 ||
            strcmp(candidate, PBT_FAIL_TIMEOUT) == 0 || strcmp(candidate, PBT_FAIL_PANIC) == 0);
}

int pbt_ci_from_value(const char *v)
{
    size_t start = 0;
    size_t end;
    size_t len;
    char buf[8];
    size_t i;
    if (v == NULL) {
        return 0;
    }
    end = strlen(v);
    while (start < end && (v[start] == ' ' || v[start] == '\t' || v[start] == '\n' || v[start] == '\r')) {
        start++;
    }
    while (end > start && (v[end - 1] == ' ' || v[end - 1] == '\t' || v[end - 1] == '\n' || v[end - 1] == '\r')) {
        end--;
    }
    len = end - start;
    if (len == 0) {
        return 0;
    }
    if (len >= sizeof buf) {
        return 1; /* longer than "false": certainly something else */
    }
    for (i = 0; i < len; i++) {
        char c = v[start + i];
        buf[i] = (char)(c >= 'A' && c <= 'Z' ? c - 'A' + 'a' : c);
    }
    buf[len] = '\0';
    return !(strcmp(buf, "0") == 0 || strcmp(buf, "false") == 0);
}

pbt_options pbt_options_default(void)
{
    pbt_options o;
    o.num_tests = 100;
    o.max_shrink_steps = 1000;
    o.ci = PBT_CI_UNSET;
    return o;
}

void pbt_verdict_init(pbt_verdict *v)
{
    memset(v, 0, sizeof *v);
    pbt_val_nil(&v->counterexample);
    pbt_val_nil(&v->minimal);
}

void pbt_verdict_free(pbt_verdict *v)
{
    pbt_free(v->message);
    pbt_val_free(&v->counterexample);
    pbt_val_free(&v->minimal);
    pbt_verdict_init(v);
}

/* ---- message building -------------------------------------------------------- */

/* printf into a fresh heap string; NULL on out-of-memory.  Never truncates. */
static char *msgf(const char *fmt, ...)
{
    va_list ap;
    va_list ap2;
    int len;
    char *out;
    va_start(ap, fmt);
    va_copy(ap2, ap);
    len = vsnprintf(NULL, 0, fmt, ap);
    va_end(ap);
    if (len < 0) {
        va_end(ap2);
        return NULL;
    }
    out = (char *)pbt_alloc((size_t)len + 1, 1);
    if (out != NULL) {
        (void)vsnprintf(out, (size_t)len + 1, fmt, ap2);
    }
    va_end(ap2);
    return out;
}

/* Fill a failing verdict; a NULL message (out-of-memory) is tolerated by callers. */
static void fail(pbt_verdict *out, const char *id, char *message, int cases_run, int iteration)
{
    out->passed = 0;
    out->failure_id = id;
    pbt_free(out->message);
    out->message = message;
    out->cases_run = cases_run;
    out->iteration = iteration;
}

/* Canonical text of v, or a constant when rendering fails.  The caller frees with pbt_str_free. */
static const char *render(const pbt_val *v, pbt_str *s)
{
    pbt_str_init(s);
    if (pbt_canon(v, s) != 0 || s->s == NULL) {
        pbt_str_free(s);
        return "<unrenderable>";
    }
    return s->s;
}

/* ---- property adapters ------------------------------------------------------- */

typedef struct prop_ctx {
    pbt_prop prop;
} prop_ctx;

/* "Still fails": the property returned exactly 0.  An error (negative) is a different failure, never "smaller". */
static int prop_falsifies(void *self, pbt_budget *b, const pbt_val *v)
{
    prop_ctx *c = (prop_ctx *)self;
    return c->prop.fn(c->prop.user, b, v) == 0;
}

typedef struct charged {
    pbt_pred inner;
    pbt_budget *scope;
} charged;

static int charged_falsifies(void *self, pbt_budget *b, const pbt_val *v)
{
    charged *c = (charged *)self;
    (void)b;
    if (!pbt_budget_charge(c->scope, 1)) {
        return 0;
    }
    return c->inner.fn(c->inner.self, c->scope, v);
}

/* ---- the falsified path -------------------------------------------------------- */

static int falsified(pbt_verdict *out, uint64_t seed, int iteration, pbt_val *candidate, const pbt_gen *gen,
                     prop_ctx *pc, const pbt_shrinker *override, const pbt_options *o, int redact,
                     pbt_budget *check_scope)
{
    pbt_pred falsifies;
    pbt_val work;
    const pbt_val *minimal = candidate;
    int steps = 0;
    int adopted = 0;
    pbt_str s_min;
    pbt_str s_init;
    const char *t_min;
    const char *t_init;

    falsifies.fn = prop_falsifies;
    falsifies.self = pc;

    /* The shrinker works on a copy: a failing or lying shrinker cannot damage the original. */
    if (pbt_val_clone(&work, candidate) == 0) {
        pbt_budget storage;
        pbt_budget *scope = pbt_budget_open_child(&storage, check_scope, o->max_shrink_steps);
        charged ch;
        pbt_pred budgeted;
        int used = 0;
        int rc;
        ch.inner = falsifies;
        ch.scope = scope;
        budgeted.fn = charged_falsifies;
        budgeted.self = &ch;
        if (override != NULL) {
            rc = override->fn(override->self, scope, &work, budgeted, o->max_shrink_steps, &used);
        } else {
            rc = pbt_gen_shrink_nested(scope, gen, &work, budgeted, o->max_shrink_steps, &used);
        }
        pbt_budget_close(&storage);
        /*
         * Re-verify: the witness we report must really fail the property.  One
         * attempt is charged to the CHECK scope (not the possibly spent shrink
         * scope); if the chain refuses it the shrunk value is not adopted.
         */
        if (rc == 0 && used >= 0 && used <= o->max_shrink_steps && pbt_budget_charge(check_scope, 1) &&
            falsifies.fn(falsifies.self, check_scope, &work)) {
            minimal = &work;
            steps = used;
            adopted = 1;
        }
    } else {
        pbt_val_nil(&work);
    }

    t_min = render(minimal, &s_min);
    if (redact) {
        char *pre = msgf("iter:%d;steps:%d;shrunk:%s", iteration, steps, t_min);
        uint64_t h = pre != NULL ? pbt_fnv1a64(pre, strlen(pre)) : 0;
        pbt_free(pre);
        (void)snprintf(out->digest, sizeof out->digest, "%016llx", (unsigned long long)h);
        fail(out, PBT_FAIL_COUNTEREXAMPLE,
             msgf("property falsified after %d iterations (shrunk in %d steps, repro-fnv: %s)", iteration, steps,
                  out->digest),
             iteration, iteration);
        out->shrink_steps = steps;
    } else {
        t_init = render(candidate, &s_init);
        fail(out, PBT_FAIL_COUNTEREXAMPLE,
             msgf("property falsified at iteration %d (seed=%llu): initial=%s, minimal=%s after %d shrink steps",
                  iteration, (unsigned long long)seed, t_init, t_min, steps),
             iteration, iteration);
        out->shrink_steps = steps;
        out->has_seed = 1;
        out->seed = seed;
        pbt_str_free(&s_init);
        /* Hand the witnesses to the verdict (it owns them). */
        out->counterexample = *candidate;
        pbt_val_nil(candidate);
        if (adopted) {
            out->minimal = work;
            pbt_val_nil(&work);
        } else if (pbt_val_clone(&out->minimal, &out->counterexample) != 0) {
            pbt_val_nil(&out->minimal);
        }
    }
    pbt_str_free(&s_min);
    pbt_val_free(&work);
    return 0;
}

/* ---- the case loop -------------------------------------------------------------- */

static int run_cases(pbt_verdict *out, pbt_budget *scope, int nested, uint64_t seed, const pbt_gen *gen, pbt_prop prop,
                     const pbt_options *o, const pbt_shrinker *override, int redact)
{
    pbt_run run;
    prop_ctx pc;
    int iteration;
    pc.prop = prop;
    pbt_rng_init(&run.rng, seed);
    run.budget = scope;
    run.depth = 0;

    for (iteration = 1; iteration <= o->num_tests; iteration++) {
        pbt_val candidate;
        int rc;
        int holds;
        if (nested && iteration > 1 && !pbt_budget_charge(scope, 1)) {
            fail(out, PBT_FAIL_EXHAUSTED, msgf("the enclosing shrink budget was spent at iteration %d", iteration),
                 iteration - 1, iteration);
            return 0;
        }
        rc = pbt_gen_sample(&run, gen, &candidate);
        if (rc != 0) {
            if (rc == PBT_E_EXHAUSTED) {
                fail(out, PBT_FAIL_EXHAUSTED,
                     msgf("filter precondition rejected all candidates at iteration %d", iteration), iteration - 1,
                     iteration);
            } else {
                fail(out, PBT_FAIL_EXHAUSTED, msgf("generator error at iteration %d (status %d)", iteration, rc),
                     iteration - 1, iteration);
            }
            return 0;
        }
        if (nested && !pbt_budget_charge_copy(scope, pbt_val_weight(&candidate))) {
            pbt_val_free(&candidate);
            fail(out, PBT_FAIL_EXHAUSTED, msgf("the enclosing shrink budget was spent at iteration %d", iteration),
                 iteration - 1, iteration);
            return 0;
        }
        holds = prop.fn(prop.user, scope, &candidate);
        if (holds < 0) {
            pbt_val_free(&candidate);
            fail(out, PBT_FAIL_PANIC, msgf("property evaluation reported an error at iteration %d", iteration),
                 iteration, iteration);
            if (!redact) {
                out->has_seed = 1;
                out->seed = seed;
            }
            return 0;
        }
        if (holds == 0) {
            (void)falsified(out, seed, iteration, &candidate, gen, &pc, override, o, redact, scope);
            pbt_val_free(&candidate);
            return 0;
        }
        pbt_val_free(&candidate);
    }
    if (!nested && pbt_budget_was_refused(scope)) {
        /* A nested check or generator inside this run was starved, so the property was not fully evaluated. */
        fail(out, PBT_FAIL_EXHAUSTED,
             msgf("a nested check was starved by the shrink budget; the property was not fully evaluated"),
             o->num_tests, o->num_tests);
        return 0;
    }
    out->passed = 1;
    out->failure_id = NULL;
    out->cases_run = o->num_tests;
    return 0;
}

int pbt_check(pbt_budget *parent, uint64_t seed, const pbt_gen *gen, pbt_prop prop, const pbt_options *opts,
              const pbt_shrinker *shrink_override, pbt_verdict *out)
{
    pbt_options o = opts != NULL ? *opts : pbt_options_default();
    pbt_budget storage;
    pbt_budget *scope;
    int nested = parent != NULL;
    int redact = o.ci != PBT_CI_OFF;
    pbt_verdict_init(out);
    out->failure_id = PBT_FAIL_EXHAUSTED; /* until proven otherwise: a refused call is never a pass */

    if (prop.fn == NULL || gen == NULL) {
        out->message = msgf("invalid argument: null property or generator");
        return PBT_E_ARG;
    }
    if (o.num_tests < 0 || o.num_tests > PBT_MAX_CASES || o.max_shrink_steps < 0 ||
        o.max_shrink_steps > PBT_MAX_SHRINK_STEPS) {
        out->message = msgf("invalid argument: num_tests %d or max_shrink_steps %d out of range", o.num_tests,
                            o.max_shrink_steps);
        return PBT_E_ARG;
    }
    if (o.num_tests == 0) {
        out->message = msgf("no cases were evaluated (num_tests is 0); a property that never ran is not a pass");
        return out->message == NULL ? PBT_E_NOMEM : 0;
    }

    /*
     * A check nested in a property pays one attempt at entry, one per case after
     * the first and the size of every sampled value: neither nesting depth,
     * branching nor num_tests multiplies the outermost budget.  A starved nested
     * check reports property-exhausted without evaluating anything.
     */
    /*
     * An explicit nesting counter: with 64 enclosing checks, or a scope chain so deep that a new
     * scope would merely join its parent (and so never count), nothing is evaluated (never a pass).
     */
    if (nested && (parent->nest >= PBT_BUDGET_MAX_DEPTH || parent->depth + 1 >= PBT_BUDGET_MAX_DEPTH)) {
        parent->root->refused = 1; /* the enclosing run was not fully evaluated: it can never pass */
        fail(out, PBT_FAIL_EXHAUSTED, msgf("checks are nested more than %d deep; nothing was evaluated", PBT_BUDGET_MAX_DEPTH), 0, 0);
        return 0;
    }
    if (nested && !pbt_budget_charge(parent, 1)) {
        fail(out, PBT_FAIL_EXHAUSTED, msgf("the enclosing shrink budget was spent at iteration %d", 0), 0, 0);
        return 0;
    }
    scope = pbt_budget_open_check(&storage, parent, (uint64_t)o.num_tests, o.max_shrink_steps);
    (void)run_cases(out, scope, nested, seed, gen, prop, &o, shrink_override, redact);
    pbt_budget_close(&storage);
    if (out->message == NULL && !out->passed) {
        return PBT_E_NOMEM; /* the verdict is still a failure; only its text was lost */
    }
    return 0;
}
