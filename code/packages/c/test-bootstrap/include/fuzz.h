/*
 * fuzz.h -- the C coverage-guided fuzz engine (VER06j).
 * ===========================================================================
 *
 * A fuzzer feeds mutated inputs to a function ("the target") and watches for
 * three kinds of trouble: a CRASH (the target declares a failure), a TIMEOUT (it
 * ran out of steps) and an OOM (it asked for more memory than its budget).
 * "Coverage-guided" means the fuzzer keeps the inputs that reached NEW program
 * points ("edges") and mutates those further, so it climbs through nested
 * checks that blind mutation would never get past:
 *
 *     if (d[0] == 'F') if (d[1] == 'U') if (d[2] == 'Z') crash();
 *
 *   blind:   needs three exact bytes at once          (1 in 16 million)
 *   guided:  keeps "F.." (new edge), then "FU." ...   (a few hundred runs)
 *
 * WHERE THE EDGES COME FROM
 * -------------------------
 * ISO C has no way to observe which branches ran, so the default coverage
 * source is the TARGET ITSELF: it calls fz_edge(h, id) at the program points it
 * wants guided.  A campaign that saw no edge says so ("black-box-uninstrumented").
 * An optional, non-ISO unit (src/fuzz_sancov.c) feeds real compiler edges; see
 * VER06j section 4.
 *
 * WHAT A CRASH IS
 * ---------------
 * C has no exceptions and a real SIGSEGV cannot be survived portably, so a
 * target RETURNS nonzero and describes the failure in an fz_crash (a short kind
 * and a stack of frame labels).  Real faults need a separate process (VER06j
 * section 5.1): run one through fz_run_outcomes with your own executor.
 *
 * CONVENTIONS
 * -----------
 *   - Functions return 0 on success and an FZ_E_* status otherwise.
 *   - Owned buffers are fz_bytes; free them with fz_bytes_free.
 *   - Single threaded.  The only global state is the handle ring (section "Handles"
 *     below) and the test seams of fuzz_hooks.h.  A TARGET MUST NOT start threads that
 *     outlive its return (nor call fz_edge / fz_tick / fz_reserve from another thread):
 *     the coverage map and the handle are unsynchronised, and a late write would race
 *     the engine.  Join every thread before returning.  fz_bitmap_record refuses to
 *     grow past the map as defence in depth, but that is not thread safety.
 *   - fz_hooks is a mutable TEST-SEAM global: production code must never write it.
 *   - The fz_crash * a target receives is valid only during the call; keeping it past
 *     the return is misuse (the engine owns and reuses it).
 *   - Nothing here ever prints an input byte or a seed.
 */
#ifndef FUZZ_H
#define FUZZ_H

#include <stddef.h>
#include <stdint.h>

#include "pbt.h"

/* ---- status codes (the numbers match the PBT_E_* of the same meaning) ------- */
#define FZ_OK 0
#define FZ_E_NOMEM PBT_E_NOMEM /* an allocation failed */
#define FZ_E_ARG PBT_E_ARG     /* invalid argument */
#define FZ_E_LIMIT PBT_E_LIMIT /* a size or count limit was hit */
#define FZ_E_WORK 7            /* a work budget was exhausted */

/* ---- limits (CWE-400 / CWE-190): a caller or fixture can ask for more only by being refused */
#define FZ_COVERAGE_MAP_SIZE 65536
#define FZ_COVERAGE_BYTES (FZ_COVERAGE_MAP_SIZE / 8)
#define FZ_MAX_ITERATIONS 1000000
#define FZ_MAX_INPUT_BYTES ((size_t)1 << 20)
#define FZ_MAX_MUTATE_BYTES ((size_t)1 << 30)
#define FZ_MAX_CRASHES 1000
#define FZ_MAX_INITIAL_CORPUS 10000
#define FZ_MAX_STEP_BUDGET ((uint64_t)100000000)
#define FZ_MAX_MEM_BUDGET ((uint64_t)1 << 30)
#define FZ_MAX_MINIMIZE_BUDGET 100000
#define FZ_DEFAULT_MINIMIZE_BUDGET 2000
#define FZ_DEFAULT_STEP_BUDGET 200000
#define FZ_DEFAULT_MEM_BUDGET ((uint64_t)64 << 20)
#define FZ_MAX_RETAINED_BYTES ((uint64_t)256 << 20)
#define FZ_CORPUS_MAX_SIZE 10000
#define FZ_CORPUS_MAX_SEQUENCE_WORK ((uint64_t)1 << 24)
#define FZ_V1_CASE_COUNT 25
#define FZ_MAX_TARGET_LEN 128
#define FZ_HANDLE_RING 4096

#define FZ_KIND_MAX 64
#define FZ_FRAME_MAX 64
#define FZ_MAX_FRAMES 64

/* The coverage tiers a report can carry. */
#define FZ_TIER_CALLER "caller-reported"
#define FZ_TIER_BLACKBOX "black-box-uninstrumented"
#define FZ_TIER_SANCOV "sancov-trace-pc-guard"

/* The five registered failure ids (fuzz-leak and fuzz-panic are never produced by this engine). */
#define FZ_FID_CRASH "fuzz-crash"
#define FZ_FID_TIMEOUT "fuzz-timeout"
#define FZ_FID_OOM "fuzz-oom"
#define FZ_FID_LEAK "fuzz-leak"
#define FZ_FID_PANIC "fuzz-panic"

/* ============================================================================
 * Owned bytes
 * ========================================================================== */
typedef struct fz_bytes {
    uint8_t *p; /* NULL when nothing is owned; non-NULL after a successful call even when n == 0 */
    size_t n;
} fz_bytes;

void fz_bytes_free(fz_bytes *b);

/* ============================================================================
 * Mutators (VER06 section 3).  Each has an int64 form and a pbt_big form.
 * out is overwritten (it must not own anything).  Outputs never alias inputs.
 * ========================================================================== */
typedef enum fz_endian { FZ_LITTLE = 0, FZ_BIG = 1 } fz_endian;

/* Interest lists: fz_interest_values(width, out) copies the boundary values for width 1/2/4/8 and
 * returns how many (0 for any other width); out needs room for 7. */
size_t fz_interest_values(int width, uint64_t out[7]);
extern const int fz_deltas8[6];
extern const uint8_t fz_interest8[5];

int fz_bit_flip(fz_bytes *out, const uint8_t *d, size_t n, int64_t bit_offset, int64_t count);
int fz_bit_flip_big(fz_bytes *out, const uint8_t *d, size_t n, const pbt_big *bit_offset, const pbt_big *count);
int fz_byte_flip(fz_bytes *out, const uint8_t *d, size_t n, int64_t byte_offset, int64_t count);
int fz_byte_flip_big(fz_bytes *out, const uint8_t *d, size_t n, const pbt_big *byte_offset, const pbt_big *count);
int fz_byte_arithmetic(fz_bytes *out, const uint8_t *d, size_t n, int64_t byte_offset, int64_t delta, int width,
                       fz_endian e);
int fz_byte_arithmetic_big(fz_bytes *out, const uint8_t *d, size_t n, const pbt_big *byte_offset,
                           const pbt_big *delta, int width, fz_endian e);
int fz_boundary_insert(fz_bytes *out, const uint8_t *d, size_t n, int64_t byte_offset, int width, uint64_t val,
                       fz_endian e);
int fz_boundary_insert_big(fz_bytes *out, const uint8_t *d, size_t n, const pbt_big *byte_offset, int width,
                           const pbt_big *val, fz_endian e);
int fz_block_delete(fz_bytes *out, const uint8_t *d, size_t n, int64_t offset, int64_t length);
int fz_block_delete_big(fz_bytes *out, const uint8_t *d, size_t n, const pbt_big *offset, const pbt_big *length);
int fz_block_insert(fz_bytes *out, const uint8_t *d, size_t n, int64_t offset, const uint8_t *ins, size_t ins_n);
int fz_block_insert_big(fz_bytes *out, const uint8_t *d, size_t n, const pbt_big *offset, const uint8_t *ins,
                        size_t ins_n);
int fz_block_duplicate(fz_bytes *out, const uint8_t *d, size_t n, int64_t src, int64_t length, int64_t dst);
int fz_block_duplicate_big(fz_bytes *out, const uint8_t *d, size_t n, const pbt_big *src, const pbt_big *length,
                           const pbt_big *dst);
int fz_splice(fz_bytes *out, const uint8_t *t, size_t tn, const uint8_t *donor, size_t dn, int64_t target_split,
              int64_t donor_split);
int fz_splice_big(fz_bytes *out, const uint8_t *t, size_t tn, const uint8_t *donor, size_t dn,
                  const pbt_big *target_split, const pbt_big *donor_split);

/* steps seeded mutations (draw order is the cross-lane contract).  FZ_E_LIMIT for steps above
 * FZ_MAX_ITERATIONS or n above FZ_MAX_INPUT_BYTES. */
int fz_mutate_sequence(fz_bytes *out, const uint8_t *d, size_t n, uint64_t seed, size_t steps, size_t max_len);
/* The same, charging the bytes it copies against work_cap (VER06j section 3).  *work_done is written on EVERY
 * return, including failures; FZ_E_WORK means the cap would have been exceeded (nothing past it ran). */
int fz_mutate_sequence_budget(fz_bytes *out, const uint8_t *d, size_t n, uint64_t seed, size_t steps, size_t max_len,
                              uint64_t work_cap, uint64_t *work_done);

/* ============================================================================
 * Coverage (VER06 section 4)
 * ========================================================================== */
typedef struct fz_bitmap {
    uint8_t m[FZ_COVERAGE_BYTES];
    uint16_t edges[FZ_COVERAGE_MAP_SIZE]; /* masked ids in the order first seen */
    size_t nedges;
} fz_bitmap;

void fz_bitmap_init(fz_bitmap *b);
void fz_bitmap_record(fz_bitmap *b, uint64_t edge);
int fz_bitmap_has(const fz_bitmap *b, uint64_t edge);
/* OR the trace into b; returns whether any edge was new and (if count_new != NULL) how many. */
int fz_bitmap_admit(fz_bitmap *b, const uint16_t *trace, size_t n, size_t *count_new);
/* Distinct covered edges (population count). */
size_t fz_bitmap_count(const fz_bitmap *b);
/* Every covered edge ascending into out (room for fz_bitmap_count entries); returns how many. */
size_t fz_bitmap_edges(const fz_bitmap *b, uint16_t *out);
void fz_bitmap_merge(fz_bitmap *b, const fz_bitmap *o);
void fz_bitmap_reset(fz_bitmap *b);

/* Pure novelty check; new_edges counts UNIQUE novel edges.  FZ_E_NOMEM on allocation failure. */
int fz_corpus_admit(const uint64_t *existing, size_t ne, const uint64_t *trace, size_t nt, int *admitted,
                    size_t *new_edges);
/* max(1, floor(base * (1 + new/total) / sqrt(max(1, len)) + 0.5)); bonus 1.0 when total == 0. */
size_t fz_calculate_energy(size_t base_energy, size_t seed_len, size_t new_edges, size_t total_edges);
/* A stable edge id from a name (FNV-1a), so a campaign is deterministic across runs. */
uint64_t fz_edge_id(const char *label);

/* ============================================================================
 * Crashes, signatures, failure ids, TAP (VER06 sections 5.2, 6)
 * ========================================================================== */
typedef struct fz_crash {
    char kind[FZ_KIND_MAX + 1];
    size_t nframes; /* innermost first */
    char frames[FZ_MAX_FRAMES][FZ_FRAME_MAX + 1];
} fz_crash;

/* Describe a crash from inside a target.  Both copy and sanitise; fz_crash_push collapses a repeat of the
 * previous frame and ignores frames beyond FZ_MAX_FRAMES. */
void fz_crash_set(fz_crash *c, const char *kind);
void fz_crash_push(fz_crash *c, const char *frame);
/* Normalise whatever a target wrote (terminate, sanitise, clamp, collapse, default the kind). */
void fz_crash_normalize(fz_crash *c);

/* A string with an explicit length (corpus strings may hold NUL). */
typedef struct fz_str {
    const char *p;
    size_t n;
} fz_str;

/* lower32(fnv1a64(kind ":" frames joined by ":")) as 8 lowercase hex plus a NUL. */
void fz_crash_signature_n(char out[9], fz_str kind, const fz_str *frames, size_t nframes);
void fz_crash_signature(char out[9], const char *kind, const char *const *frames, size_t nframes);
void fz_crash_signature_of(char out[9], const fz_crash *c);
/* Two crashes with the same signature. */
int fz_is_duplicate_crash(fz_str kind_a, const fz_str *frames_a, size_t na, fz_str kind_b, const fz_str *frames_b,
                          size_t nb);

/* True only for the five registered ids. */
int fz_is_failure_id(const char *candidate, size_t len);
/* not ok N - target [failure-id:hex8] into buf (cap bytes).  FZ_E_ARG for an unregistered id or a
 * signature that is not exactly 8 lowercase hex digits; FZ_E_LIMIT if buf is too small. */
int fz_format_tap_failure(char *buf, size_t cap, int test_num, const char *target, size_t target_len,
                          const char *failure_id, size_t failure_id_len, const char *sig_hash, size_t sig_len);

/* ============================================================================
 * Handles: what a target sees (VER06j section 5.3)
 * ========================================================================== */
typedef struct fz_handle fz_handle;

/* The target reports that it executed the program point id (reduced mod 65,536).  A retired handle ignores it.
 * Handles live in a static ring of FZ_HANDLE_RING (4,096) slots: a pointer kept for more than 4,095 later
 * executions may alias a later execution's handle (memory-safe, but attributed to that execution). */
void fz_edge(fz_handle *h, uint64_t id);
/* One unit of work.  Returns nonzero when the budget is exhausted (and on every later call, STICKY). */
int fz_tick(fz_handle *h);
/* An allocation of n bytes is about to be made on the input's behalf.  Nonzero when the running total would
 * exceed the memory budget (STICKY, shared with fz_tick). */
int fz_reserve(fz_handle *h, size_t n);
/* options.user of the running campaign (NULL for a retired handle). */
void *fz_handle_user(const fz_handle *h);
uint64_t fz_handle_epoch(const fz_handle *h);
/* The handle of the execution in progress, or NULL outside a target run. */
fz_handle *fz_active_handle(void);
/* Record an edge found by compiler instrumentation (marks the tier). */
void fz_edge_sancov(fz_handle *h, uint32_t guard_id);

/* ============================================================================
 * Campaigns
 * ========================================================================== */
typedef int (*fz_target_fn)(const uint8_t *data, size_t len, fz_handle *h, fz_crash *out);

typedef enum fz_outcome_kind { FZ_PASS = 0, FZ_CRASH, FZ_TIMEOUT, FZ_OOM } fz_outcome_kind;

typedef struct fz_outcome {
    fz_outcome_kind kind;
    fz_crash crash; /* meaningful only for FZ_CRASH */
} fz_outcome;

/* An outcome-level executor: run the code under test on data, record its edges into cov (reset before each
 * call), fill *out.  Returns 0, or FZ_E_NOMEM / FZ_E_ARG / FZ_E_LIMIT to abort the campaign. */
typedef int (*fz_exec_fn)(void *user, const uint8_t *data, size_t len, fz_bitmap *cov, fz_outcome *out);

typedef struct fz_input {
    const uint8_t *p;
    size_t n;
} fz_input;

typedef struct fz_options {
    uint64_t seed;
    size_t max_iterations;
    size_t max_input_len;
    size_t base_energy;
    size_t minimize_budget;
    uint64_t step_budget;
    uint64_t mem_budget;
    void *user;
} fz_options;

fz_options fz_options_default(void);
/* 0 or FZ_E_ARG. */
int fz_options_validate(const fz_options *o);

typedef struct fz_corpus_entry {
    uint8_t *p; /* PRIVATE: never log */
    size_t n;
    size_t new_edges;
    size_t generation;
} fz_corpus_entry;

typedef struct fz_crash_record {
    char sig[9];
    fz_crash crash;
    size_t hits;
    int minimized;     /* 0 when minimisation was skipped or cut short by a budget */
    int input_dropped; /* 1 when the retained-bytes cap kept no input */
    uint8_t *minimal;  /* PRIVATE: never log.  NULL when input_dropped */
    size_t minimal_len;
} fz_crash_record;

/* Every execution lands in exactly one bucket.  iterations counts MUTATION executions (triage is not counted);
 * crashes/timeouts/ooms count triage and mutation executions. */
typedef struct fz_report {
    size_t iterations;
    size_t admitted_seeds;
    size_t total_edges;
    fz_crash_record *crashes;
    size_t ncrashes;
    size_t timeouts;
    size_t ooms;
    size_t crashes_dropped;
    size_t admissions_dropped;
    const char *tier; /* one of FZ_TIER_* (static) */
    fz_corpus_entry *corpus;
    size_t ncorpus;
} fz_report;

void fz_report_init(fz_report *r);
void fz_report_free(fz_report *r);
/* Found nothing: at least one iteration ran and no execution crashed, timed out or ran out of memory. */
int fz_report_is_clean(const fz_report *r);
size_t fz_report_unique_crashes(const fz_report *r);
/* Counts, tier and cleanliness only; never input bytes.  Returns the length needed (snprintf style). */
size_t fz_report_summary(const fz_report *r, char *buf, size_t cap);
/* TAP: one line per crash signature, then one for timeouts, one for OOMs. */
size_t fz_tap_failure_count(const fz_report *r);
int fz_tap_failure_line(const fz_report *r, size_t index, const char *target, int first_test, char *buf, size_t cap);

/* A bounded coverage-guided campaign.  *out is a fresh report on success and zeroed on failure. */
int fz_run(const fz_input *corpus, size_t ncorpus, const fz_options *opts, fz_target_fn target, fz_report *out);
int fz_run_outcomes(const fz_input *corpus, size_t ncorpus, const fz_options *opts, fz_exec_fn exec, void *exec_user,
                    fz_report *out);

/* Minimisation: test runs one candidate (same contract as fz_exec_fn without coverage). */
typedef int (*fz_test_fn)(void *user, const uint8_t *data, size_t len, fz_outcome *out);
/* Shrinks while keeping the EXACT signature sig (8 hex).  budget caps the executions (0..FZ_MAX_MINIMIZE_BUDGET). */
int fz_minimize_crashing_input(fz_bytes *out, const uint8_t *data, size_t len, const char *sig, fz_test_fn test,
                               void *user, size_t budget);

/* ============================================================================
 * fuzz-v1 corpus evaluation (VER06 section 8)
 * ========================================================================== */
/* Evaluate a parsed cases.json.  0 means evaluation ran; conformance is rep->nproblems == 0. */
int fz_corpus_evaluate(const pbt_json *document, pbt_corpus_report *rep);

#endif /* FUZZ_H */
