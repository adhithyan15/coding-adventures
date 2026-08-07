/*
 * cpu_pipeline.h — Configurable N-stage CPU instruction pipeline, pure ISO C17.
 * =====================================================================
 *
 * A faithful port of the Rust `cpu-pipeline` crate. It manages the FLOW of
 * instructions through pipeline stages (IF → ID → EX → MEM → WB and deeper
 * variants). It does NOT interpret instructions — the ISA work is injected via
 * callbacks. The pipeline moves "tokens" through stages, handling normal
 * advancement, stalls (freeze + bubble), flushes (discard speculative work),
 * forwarding, and statistics (IPC/CPI, stall/flush/bubble cycles).
 *
 * ## Port shape
 *
 * The Rust `PipelineToken` carries a `HashMap<String,i64>` of stage-entry
 * cycles and `String` fields. To keep the C port heap-light and memory-safe,
 * `CpToken` is a fixed-size **plain value type**: opcode/forwarded_from are
 * bounded char arrays and stage-entry timestamps live in a fixed array indexed
 * up to `CP_MAX_STAGES`. Tokens copy by assignment — no per-token allocation.
 * Consequently a pipeline may have at most `CP_MAX_STAGES` stages (the deepest
 * preset, 13 stages, fits comfortably); `cp_config_validate` rejects more.
 *
 * Callbacks take a `void *ctx` user-data pointer (the C equivalent of Rust's
 * captured closures). The Rust callbacks take a token by value and return it;
 * here they mutate a `CpToken*` in place, which is equivalent.
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef CPU_PIPELINE_H
#define CPU_PIPELINE_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* int64_t */

#ifdef __cplusplus
extern "C" {
#endif

#define CP_MAX_STAGES 16
#define CP_NAME_LEN 16
#define CP_OPCODE_LEN 16
#define CP_DESC_LEN 64

/* ── Stage category ─────────────────────────────────────────────────────────*/
typedef enum {
    CP_FETCH,
    CP_DECODE,
    CP_EXECUTE,
    CP_MEMORY,
    CP_WRITEBACK
} CpStageCategory;

/* Human-readable name ("fetch", "decode", ...). */
const char *cp_stage_category_str(CpStageCategory c);

/* ── Pipeline stage definition ──────────────────────────────────────────────*/
typedef struct {
    char name[CP_NAME_LEN];        /* "IF", "ID", "EX1", ... */
    char description[CP_DESC_LEN]; /* human-readable */
    CpStageCategory category;
} CpPipelineStage;

/* ── Pipeline token — one instruction flowing through the pipeline ───────────*/
typedef struct {
    char name[CP_NAME_LEN]; /* stage name */
    int64_t cycle;          /* cycle the token entered that stage */
} CpStageEntry;

typedef struct {
    /* Instruction identity. */
    int64_t pc;
    int64_t raw_instruction;
    char opcode[CP_OPCODE_LEN];

    /* Decoded operands (-1 = unused register). */
    int64_t rs1, rs2, rd, immediate;

    /* Control signals (0/1). */
    int reg_write, mem_read, mem_write, is_branch, is_halt;

    /* Computed values. */
    int64_t alu_result, mem_data, write_data, branch_target;
    int branch_taken;

    /* Pipeline metadata. */
    int is_bubble;
    CpStageEntry stage_entered[CP_MAX_STAGES];
    int stage_entered_count;
    char forwarded_from[CP_NAME_LEN];
} CpToken;

/* Initialize a fresh (non-bubble) token: registers -1, all signals clear. */
void cp_token_init(CpToken *t);
/* Initialize a bubble (NOP) token. */
void cp_token_init_bubble(CpToken *t);
/* Human-readable form: "---" for a bubble, "OPCODE@pc", else "instr@pc". */
void cp_token_to_string(const CpToken *t, char *buf, size_t n);
/* Look up the cycle recorded for stage `name`; returns 1 and writes *out if
 * present, else 0. */
int cp_token_stage_get(const CpToken *t, const char *name, int64_t *out);
/* 1 if a cycle is recorded for stage `name`, else 0. */
int cp_token_stage_contains(const CpToken *t, const char *name);

/* ── Pipeline configuration ─────────────────────────────────────────────────*/
typedef struct {
    CpPipelineStage stages[CP_MAX_STAGES];
    int num_stages;
    int64_t execution_width;
} CpPipelineConfig;

int cp_config_num_stages(const CpPipelineConfig *c);
/* Validate the configuration. Returns 1 if well-formed, else 0 and writes a
 * message to `err` (if non-NULL): needs >=2 stages, width>=1, unique names, at
 * least one fetch and one writeback stage, and <= CP_MAX_STAGES stages. */
int cp_config_validate(const CpPipelineConfig *c, char *err, size_t errn);
/* Standard presets. */
void cp_config_classic_5_stage(CpPipelineConfig *out);
void cp_config_deep_13_stage(CpPipelineConfig *out);

/* ── Statistics ─────────────────────────────────────────────────────────────*/
typedef struct {
    int64_t total_cycles;
    int64_t instructions_completed;
    int64_t stall_cycles;
    int64_t flush_cycles;
    int64_t bubble_cycles;
} CpPipelineStats;

double cp_stats_ipc(const CpPipelineStats *s);
double cp_stats_cpi(const CpPipelineStats *s);

/* ── Snapshot — full pipeline state at one cycle ────────────────────────────*/
typedef struct {
    int occupied; /* 0 if the stage is empty */
    CpToken tok;
} CpStageSlot;

typedef struct {
    int64_t cycle;
    int64_t pc;
    int stalled;
    int flushing;
    int num_stages;
    CpStageSlot stages[CP_MAX_STAGES];             /* parallel to the config */
    char stage_names[CP_MAX_STAGES][CP_NAME_LEN];  /* the config's stage names */
} CpSnapshot;

/* Token in stage `name`, or NULL if that stage is empty / unknown. Mirrors the
 * Rust `snapshot.stages.get(name)` (which omits empty stages from the map). */
const CpToken *cp_snapshot_stage(const CpSnapshot *s, const char *name);

/* ── Hazard detection ───────────────────────────────────────────────────────*/
typedef enum {
    CP_HAZARD_NONE,
    CP_HAZARD_FORWARD_FROM_EX,
    CP_HAZARD_FORWARD_FROM_MEM,
    CP_HAZARD_STALL,
    CP_HAZARD_FLUSH
} CpHazardAction;

const char *cp_hazard_action_str(CpHazardAction a);

typedef struct {
    CpHazardAction action;
    int64_t forward_value;
    char forward_source[CP_NAME_LEN];
    size_t stall_stages;
    size_t flush_count;
    int64_t redirect_pc;
} CpHazardResponse;

/* A default (no-hazard) response. */
CpHazardResponse cp_hazard_response_default(void);

/* ── Callback signatures (each carries a user-data pointer) ──────────────────*/
typedef int64_t (*CpFetchFn)(void *ctx, int64_t pc);
typedef void (*CpDecodeFn)(void *ctx, int64_t raw, CpToken *tok);
typedef void (*CpExecuteFn)(void *ctx, CpToken *tok);
typedef void (*CpMemoryFn)(void *ctx, CpToken *tok);
typedef void (*CpWritebackFn)(void *ctx, const CpToken *tok);
typedef CpHazardResponse (*CpHazardFn)(void *ctx, const CpStageSlot *stages,
                                       int num_stages);
typedef int64_t (*CpPredictFn)(void *ctx, int64_t pc);

/* ── The pipeline (opaque) ──────────────────────────────────────────────────*/
typedef struct CpPipeline CpPipeline;

/* Create a pipeline. Validates the config; on failure returns NULL and writes
 * a message to `err` (if non-NULL). The five stage callbacks are required; each
 * `*_ctx` may be NULL. */
CpPipeline *cp_pipeline_new(const CpPipelineConfig *config, CpFetchFn fetch,
                            void *fetch_ctx, CpDecodeFn decode, void *decode_ctx,
                            CpExecuteFn execute, void *execute_ctx,
                            CpMemoryFn memory, void *memory_ctx,
                            CpWritebackFn writeback, void *writeback_ctx,
                            char *err, size_t errn);
void cp_pipeline_free(CpPipeline *p);

void cp_pipeline_set_hazard_fn(CpPipeline *p, CpHazardFn fn, void *ctx);
void cp_pipeline_set_predict_fn(CpPipeline *p, CpPredictFn fn, void *ctx);
void cp_pipeline_set_pc(CpPipeline *p, int64_t pc);
int64_t cp_pipeline_pc(const CpPipeline *p);
int64_t cp_pipeline_cycle(const CpPipeline *p);
int cp_pipeline_is_halted(const CpPipeline *p);
CpPipelineStats cp_pipeline_stats(const CpPipeline *p);
const CpPipelineConfig *cp_pipeline_config(const CpPipeline *p);
/* Token in the named stage, or NULL if empty / unknown. */
const CpToken *cp_pipeline_stage_contents(const CpPipeline *p,
                                          const char *stage_name);

/* Advance one clock cycle; writes the resulting snapshot to *out (if non-NULL). */
void cp_pipeline_step(CpPipeline *p, CpSnapshot *out);
/* Step until halt or `max_cycles` cycles; returns final statistics. */
CpPipelineStats cp_pipeline_run(CpPipeline *p, int64_t max_cycles);
/* Snapshot the current state without advancing. */
void cp_pipeline_snapshot(const CpPipeline *p, CpSnapshot *out);

/* Recorded snapshot history (one per executed cycle). */
size_t cp_pipeline_trace_count(const CpPipeline *p);
int cp_pipeline_trace(const CpPipeline *p, size_t i, CpSnapshot *out);

#ifdef __cplusplus
}
#endif

#endif /* CPU_PIPELINE_H */
