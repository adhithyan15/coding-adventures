/*
 * pbt.h -- the C property-based testing engine (VER05i).
 * ===========================================================================
 *
 * A property-based test checks a CLAIM about every input a generator can
 * produce, not one example:
 *
 *     for every list xs of small ints:   sum(xs) <= 20
 *
 * The engine has three jobs:
 *
 *   1. GENERATE candidates from a deterministic random stream (SplitMix64), so
 *      the same seed gives the same candidates on every machine and in every
 *      language lane.  The shared pbt-v1 corpus pins the draws.
 *   2. SHRINK a failing candidate to a witness a human can read.  [913,-4,77]
 *      is noise; [8,5,8] is a diagnosis.  Shrinking is "integrated": the
 *      generator that made a value knows its bounds, so a shrunk int(5,10)
 *      never becomes 0.
 *   3. JUDGE honestly.  A verdict is one of a closed set of failure ids, and
 *      the engine refuses to report a pass unless every case ran and held.
 *
 * CONVENTIONS
 * -----------
 *   - Functions return 0 on success and a PBT_E_* status otherwise.
 *   - Constructors return NULL on invalid arguments or out-of-memory.
 *   - Properties return 1 (holds), 0 (fails) or a negative number (the
 *     property itself errored: "property-panic").
 *   - Properties and predicates receive `const pbt_val *` and must not mutate
 *     or free it.
 *   - Everything is single-threaded.  There is no global state besides the
 *     test seams in pbt_hooks.h; the shrink budget is passed explicitly as a
 *     `pbt_budget *` (NULL at the top level).
 */
#ifndef PBT_H
#define PBT_H

#include <stddef.h>
#include <stdint.h>

#include "pbt_big.h"
#include "pbt_json.h"

/* ---- status codes ---------------------------------------------------------- */
#define PBT_OK 0
#define PBT_E_EXHAUSTED 1     /* a generator could not produce a value (filter) */
#define PBT_E_NOMEM 2         /* an allocation failed */
#define PBT_E_ARG 3           /* invalid argument */
#define PBT_E_GEN 4           /* a user generator/filter/map reported an error */
#define PBT_E_LIMIT 5         /* a size or depth limit was hit */
#define PBT_E_NOT_FALSIFYING 6 /* shrinking was asked about a value that does not fail */

/* ---- limits (CWE-400): a caller or fixture can ask for more only by being refused -- */
#define PBT_MAX_CASES 100000
#define PBT_MAX_SHRINK_STEPS 10000
#define PBT_MAX_INPUT_BYTES ((size_t)1 << 20)
#define PBT_MAX_LIST_LEN 100000
#define PBT_MAX_GEN_COUNT 1000000
#define PBT_MAX_GEN_BYTES_TOTAL ((size_t)1 << 20)
#define PBT_MAX_FREQUENCY_TOTAL ((uint64_t)1 << 32)
#define PBT_MAX_FILTER_RETRIES 10000
#define PBT_DEFAULT_MAX_RETRIES 100
#define PBT_MAX_TUPLE 8
#define PBT_MAX_VAL_DEPTH 64
#define PBT_MAX_SAMPLE_DEPTH 64
#define PBT_ATTEMPT_FACTOR 16
#define PBT_ATTEMPT_SLACK 64
#define PBT_MAX_SHRINK_WORK ((uint64_t)64 * 1024 * 1024)
#define PBT_BUDGET_MAX_DEPTH 64
#define PBT_CANON_MAX_DEPTH 16
#define PBT_CANON_MAX_NODES 10000
#define PBT_CANON_MAX_BYTES ((size_t)1 << 20)
#define PBT_V1_CASE_COUNT 25

/* ============================================================================
 * PRNG (VER05 section 2)
 * ========================================================================== */
typedef struct pbt_rng {
    uint64_t state;
} pbt_rng;

/* Start a stream.  (Applies the test seam pbt_hooks.seed_map if one is set.) */
void pbt_rng_init(pbt_rng *r, uint64_t seed);
uint64_t pbt_rng_next(pbt_rng *r);
/* Uniform on [0, n), n >= 1, by rejection sampling.  bounded(1) consumes one word.  n == 0 returns 0, consuming nothing. */
uint64_t pbt_rng_bounded(pbt_rng *r, uint64_t n);
/* Uniform on [0, span) for 1 <= span (<= 1000 bits).  Returns nonzero for an invalid span. */
int pbt_rng_bounded_big(pbt_rng *r, const pbt_big *span, pbt_big *out);
uint64_t pbt_fnv1a64(const void *data, size_t len);
uint64_t pbt_sub_seed(uint64_t run_seed, const char *property_id);

/* ============================================================================
 * Tagged values
 * ========================================================================== */
typedef enum pbt_kind {
    PBT_V_NIL,
    PBT_V_BOOL,
    PBT_V_INT,
    PBT_V_BYTES,
    PBT_V_STR,
    PBT_V_LIST,
    PBT_V_TUPLE,
    PBT_V_PTR
} pbt_kind;

typedef struct pbt_val pbt_val;
struct pbt_val {
    pbt_kind kind;
    union {
        int b;
        pbt_big i;
        struct {
            uint8_t *p; /* NULL when n == 0 */
            size_t n;
        } bytes; /* PBT_V_BYTES and PBT_V_STR */
        struct {
            pbt_val *items; /* NULL when n == 0 */
            size_t n;
        } list; /* PBT_V_LIST and PBT_V_TUPLE */
        void *ptr;
    } u;
};

/* Initialise to nil (owns nothing).  Always valid to pass to pbt_val_free. */
void pbt_val_nil(pbt_val *v);
void pbt_val_set_bool(pbt_val *v, int b);
void pbt_val_set_int(pbt_val *v, const pbt_big *i);
void pbt_val_set_u64(pbt_val *v, uint64_t x);
void pbt_val_set_i64(pbt_val *v, int64_t x);
void pbt_val_set_ptr(pbt_val *v, void *p);
/* Copy `n` bytes (BYTES) / characters (STR).  Nonzero on out-of-memory (v is then nil). */
int pbt_val_set_bytes(pbt_val *v, const uint8_t *data, size_t n);
int pbt_val_set_str(pbt_val *v, const char *s, size_t n);
/* A LIST or TUPLE of n nil items.  Nonzero on out-of-memory or n above PBT_MAX_LIST_LEN. */
int pbt_val_set_list(pbt_val *v, pbt_kind kind, size_t n);
/* Free what v owns (recursively, bounded) and leave it nil. */
void pbt_val_free(pbt_val *v);
/* Deep copy; dst must not own anything.  Nonzero on out-of-memory / depth; dst is then nil. */
int pbt_val_clone(pbt_val *dst, const pbt_val *src);
/* Kind-sensitive deep equality (depth bounded: deeper values compare unequal). */
int pbt_val_equal(const pbt_val *a, const pbt_val *b);
/* Approximate size in elements (>= 1 for a collection): the budget's "work" of a value. */
uint64_t pbt_val_weight(const pbt_val *v);

/* ============================================================================
 * Canonical rendering (VER05 section 7.2)
 * ========================================================================== */
typedef struct pbt_str {
    char *s; /* NUL terminated, NULL when empty and never grown */
    size_t n;
    size_t cap;
    int truncated;
} pbt_str;

void pbt_str_init(pbt_str *s);
void pbt_str_free(pbt_str *s);
/* Render v: total, depth <= 16, <= 10,000 nodes, <= 1 MiB.  Nonzero only on out-of-memory. */
int pbt_canon(const pbt_val *v, pbt_str *out);

/* ============================================================================
 * The shrink budget tree (VER05e section 4.1, VER05i section 6)
 * ========================================================================== */
typedef struct pbt_budget pbt_budget;
struct pbt_budget {
    pbt_budget *parent; /* nearest owning ancestor, or NULL */
    pbt_budget *root;   /* outermost node of the tree (itself for a root) */
    int depth;
    int owning;         /* 0 for a storage that joined its parent */
    int refused;        /* set on a ROOT when any charge in its tree was refused */
    int nest;           /* pbt_check calls enclosing this scope (explicit nesting counter, capped at 64) */
    uint64_t max_attempts;
    uint64_t max_work;
    uint64_t attempts;
    uint64_t work;
};

/*
 * Open a scope in `storage` (a node on the caller's stack) and return the node
 * to use: `storage` itself, or `parent` when the depth cap makes the scope join
 * its parent.  Always pair with pbt_budget_close(storage).
 *
 *   open_child   public entry points: fresh local cap 16*max_steps+64 attempts
 *                (a fresh ROOT when parent is NULL)
 *   open_check   a whole pbt_check: cap num_tests + 16*max_steps + 64
 */
pbt_budget *pbt_budget_open_child(pbt_budget *storage, pbt_budget *parent, int max_steps);
pbt_budget *pbt_budget_open_child_cap(pbt_budget *storage, pbt_budget *parent, int max_steps, uint64_t max_work);
pbt_budget *pbt_budget_open_check(pbt_budget *storage, pbt_budget *parent, uint64_t num_tests, int max_steps);
void pbt_budget_close(pbt_budget *storage);

/* One attempt over a candidate of `elements` (at least 1) elements; 1 if afforded everywhere on the path, else 0 and nothing changes. */
int pbt_budget_charge(pbt_budget *b, uint64_t elements);
/* Work only (a copy that is not a property evaluation). */
int pbt_budget_charge_copy(pbt_budget *b, uint64_t elements);
int pbt_budget_has_room(const pbt_budget *b);
/* 1 when this node has an owning ancestor (it is not an outermost root). */
int pbt_budget_is_nested(const pbt_budget *b);
int pbt_budget_was_refused(const pbt_budget *b);
uint64_t pbt_budget_attempts(const pbt_budget *b);
uint64_t pbt_budget_work(const pbt_budget *b);
/* ATTEMPT_FACTOR * max_steps + ATTEMPT_SLACK */
uint64_t pbt_attempt_budget(int max_steps);

/* ============================================================================
 * Predicates and properties
 * ========================================================================== */
/* True while the value STILL FAILS the property.  `b` is the budget scope the call runs in. */
typedef int (*pbt_falsifies_fn)(void *self, pbt_budget *b, const pbt_val *v);
typedef struct pbt_pred {
    pbt_falsifies_fn fn;
    void *self;
} pbt_pred;

/* 1 holds, 0 fails, negative: the property errored ("property-panic"). */
typedef int (*pbt_prop_fn)(void *user, pbt_budget *scope, const pbt_val *v);
typedef struct pbt_prop {
    pbt_prop_fn fn;
    void *user;
} pbt_prop;

/* ============================================================================
 * Generators
 * ========================================================================== */
typedef struct pbt_run {
    pbt_rng rng;
    pbt_budget *budget; /* the scope sampling runs in; may be NULL */
    int depth;
} pbt_run;

/* Draw one value into *out (nil on entry, caller frees).  0 or a PBT_E_* status; *out must stay valid either way. */
typedef int (*pbt_sample_fn)(void *self, pbt_run *run, pbt_val *out);
/*
 * Shrink *value in place while `falsifies` says it still fails; add the number
 * of accepted moves to *steps.  *value must be valid and freeable on every
 * return.  `b` is the enclosing scope to join (never NULL).
 */
typedef int (*pbt_shrink_fn)(void *self, pbt_budget *b, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps);

typedef struct pbt_gen pbt_gen;

/* A generator from a sampler, an optional shrinker, a context and an optional destructor of the context. */
pbt_gen *pbt_gen_custom(pbt_sample_fn sample, pbt_shrink_fn shrink, void *self, void (*destroy)(void *self));
void pbt_gen_retain(pbt_gen *g);
void pbt_gen_release(pbt_gen *g);
/* Sample with the depth guard (a recursive generator cannot overflow the C stack). */
int pbt_gen_sample(pbt_run *run, const pbt_gen *g, pbt_val *out);
/* Public shrink entry: opens a child scope of `parent` (a fresh root if NULL), charges every evaluation. */
int pbt_gen_shrink(pbt_budget *parent, const pbt_gen *g, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps);
/* Shrink joining the scope `b` (for combinators and custom shrinkers). */
int pbt_gen_shrink_nested(pbt_budget *b, const pbt_gen *g, pbt_val *value, pbt_pred falsifies, int max_steps, int *steps);

pbt_gen *pbt_gen_bool(void);
pbt_gen *pbt_gen_u8(void);
pbt_gen *pbt_gen_u64(void);
pbt_gen *pbt_gen_i128(void);
/* Uniform over [min, max] (both within PBT_BIG_MAX_BITS bits). */
pbt_gen *pbt_gen_int(const pbt_big *min, const pbt_big *max);
pbt_gen *pbt_gen_int_i64(int64_t min, int64_t max);
pbt_gen *pbt_gen_bytes(size_t min_len, size_t max_len);
pbt_gen *pbt_gen_constant(const pbt_val *v);

typedef int (*pbt_map_fn)(void *self, const pbt_val *in, pbt_val *out);
/* 1 keep, 0 reject, negative error.  `b` is the scope sampling/shrinking runs in (for nested checks). */
typedef int (*pbt_filter_fn)(void *self, pbt_budget *b, const pbt_val *v);
/* Return a generator holding one reference that the engine releases; 0 or a PBT_E_* status. */
typedef int (*pbt_flat_map_fn)(void *self, const pbt_val *in, pbt_gen **out);

pbt_gen *pbt_gen_map(pbt_gen *g, pbt_map_fn f, void *self);
pbt_gen *pbt_gen_filter(pbt_gen *g, pbt_filter_fn pred, void *self, int max_retries);
pbt_gen *pbt_gen_flat_map(pbt_gen *g, pbt_flat_map_fn f, void *self);
pbt_gen *pbt_gen_one_of(pbt_gen *const *gens, size_t n);

typedef struct pbt_arm {
    uint64_t weight;
    pbt_gen *gen;
} pbt_arm;
pbt_gen *pbt_gen_frequency(const pbt_arm *arms, size_t n);
pbt_gen *pbt_gen_list(pbt_gen *elem, size_t min_len, size_t max_len);
pbt_gen *pbt_gen_tuple(pbt_gen *const *gens, size_t n);

/* ============================================================================
 * Standalone shrinkers (the corpus entry points)
 * ========================================================================== */
/*
 * Shrink a falsifying integer toward `target` inside [lo, hi].  prop is the
 * corpus convention (1 holds).  *minimal receives the result.  Returns
 * PBT_E_NOT_FALSIFYING if `initial` satisfies prop, PBT_E_ARG for lo > hi, an
 * initial outside the bounds or a budget outside [0, PBT_MAX_SHRINK_STEPS].
 */
int pbt_shrink_int(pbt_budget *parent, const pbt_big *initial, const pbt_big *target, const pbt_big *lo,
                   const pbt_big *hi, pbt_prop prop, int max_steps, pbt_big *minimal, int *steps);
/* Shrink a LIST of INT: empty probe, halves, deletion, then each element toward elem_target (unbounded). In place. */
int pbt_shrink_list_ints(pbt_budget *parent, pbt_val *list, const pbt_big *elem_target, pbt_prop prop, int max_steps,
                         int *steps);
/* Shrink BYTES toward 0x00 bytes by sequence shrinking.  In place. */
int pbt_shrink_bytes(pbt_budget *parent, pbt_val *bytes, pbt_prop prop, int max_steps, int *steps);
/* Replay: draw up to 1000 bounded(1000) values; the first failing one is shrunk toward 0 over [0, 1000]. */
int pbt_simulate_replay(pbt_budget *parent, uint64_t seed, pbt_prop prop, pbt_big *initial_fail, pbt_big *minimal);

/* ============================================================================
 * The honest runner (VER05 section 7)
 * ========================================================================== */
#define PBT_FAIL_COUNTEREXAMPLE "property-counterexample"
#define PBT_FAIL_EXHAUSTED "property-exhausted"
#define PBT_FAIL_TIMEOUT "property-timeout"
#define PBT_FAIL_PANIC "property-panic"

/* True only for the four registered ids. */
int pbt_is_failure_id(const char *candidate);

typedef enum pbt_ci_mode {
    PBT_CI_UNSET = 0, /* treated as CI: the engine fails closed */
    PBT_CI_ON,
    PBT_CI_OFF
} pbt_ci_mode;

/* Pure CI detection: pass getenv("CI") (NULL when unset).  Anything but NULL, "", "0", "false" (trimmed, any case) is CI. */
int pbt_ci_from_value(const char *ci_env_value);

typedef struct pbt_options {
    int num_tests;       /* 0 is legal and a failure: a property that never ran proves nothing */
    int max_shrink_steps;
    pbt_ci_mode ci;
} pbt_options;

/* 100 cases, 1000 shrink steps, CI redaction. */
pbt_options pbt_options_default(void);

typedef struct pbt_verdict {
    int passed;
    const char *failure_id; /* NULL for a pass; otherwise one of PBT_FAIL_* (static) */
    char *message;          /* heap */
    int cases_run;
    int iteration;
    int shrink_steps;
    int has_seed;           /* 0 when redacted */
    uint64_t seed;
    pbt_val counterexample; /* nil when redacted or not applicable */
    pbt_val minimal;
    char digest[17];        /* repro-fnv, hex, or "" */
} pbt_verdict;

void pbt_verdict_init(pbt_verdict *v);
void pbt_verdict_free(pbt_verdict *v);

typedef struct pbt_shrinker {
    pbt_shrink_fn fn;
    void *self;
} pbt_shrinker;

/*
 * Run `prop` over opts->num_tests candidates from `seed`.  `parent` is the
 * scope of an enclosing shrink when this check is nested in a property (pass the
 * scope the property was handed), else NULL.  `opts` NULL means defaults;
 * `shrink_override` (may be NULL) replaces the generator's shrinker.
 *
 * Returns 0 when a verdict was produced, PBT_E_ARG / PBT_E_NOMEM otherwise; in
 * every case *out is a FAILING verdict unless every case ran and held.
 */
int pbt_check(pbt_budget *parent, uint64_t seed, const pbt_gen *gen, pbt_prop prop, const pbt_options *opts,
              const pbt_shrinker *shrink_override, pbt_verdict *out);

/* ============================================================================
 * pbt-v1 corpus evaluation (VER05 section 6)
 * ========================================================================== */
typedef struct pbt_corpus_report {
    size_t evaluated;  /* cases the evaluator ran */
    size_t nproblems;  /* total problems found (may exceed what is stored) */
    size_t nstored;
    char **problems;   /* the first PBT_CORPUS_MAX_PROBLEMS, each starting with the case id */
} pbt_corpus_report;

#define PBT_CORPUS_MAX_PROBLEMS 1000
#define PBT_CORPUS_MAX_SIZE 10000

void pbt_corpus_report_init(pbt_corpus_report *r);
void pbt_corpus_report_free(pbt_corpus_report *r);
/* Evaluate a parsed cases.json document.  0 means evaluation ran; conformance is rep->nproblems == 0. */
int pbt_corpus_evaluate(const pbt_json *document, pbt_corpus_report *rep);

#endif /* PBT_H */
