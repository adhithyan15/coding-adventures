/*
 * hazard_detection.h — pipeline hazard detection for a 5-stage CPU, pure ISO C17.
 * =============================================================================
 *
 * A faithful port of the Rust `hazard-detection` crate. Detects data, control,
 * and structural hazards in a classic in-order 5-stage pipeline and decides the
 * action: forward, stall, or flush.
 *
 * ## Model
 *
 * A `HdPipelineSlot` is an ISA-independent snapshot of the instruction in a
 * pipeline stage — which registers it reads/writes and which resources it uses.
 * Each detector inspects the relevant slots and returns a `HdHazardResult`
 * carrying the action, any forwarded value, stall/flush counts, and a
 * human-readable reason.
 *
 * Priority (most severe wins): FLUSH > STALL > FORWARD_EX > FORWARD_MEM > NONE.
 *
 * ## Divergences from Rust (documented)
 *
 *   - Rust `Vec<u32>` source registers -> a borrowed `(const uint32_t *, count)`
 *     the caller supplies (no allocation; the slot never owns it).
 *   - Rust `Option<u32>` / `Option<i64>` -> a `has_*` flag plus the value.
 *   - Rust `String` reason / forwarded_from -> fixed inline char buffers, so a
 *     `HdHazardResult` is a plain value type (no heap).
 *   - The `HdHazardUnit` history is the one heap-owning piece; pair init/free.
 *
 * Pure ISO C17: compiles under GCC, Clang and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors; no <math.h>, no compiler extensions.
 */
#ifndef CA_HAZARD_DETECTION_H
#define CA_HAZARD_DETECTION_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint32_t, int64_t */

#ifdef __cplusplus
extern "C" {
#endif

/* The action the hazard unit tells the pipeline to take (ordered by severity
 * only via hd_priority; the enum values are the Rust discriminants). */
typedef enum {
    HD_NONE = 0,
    HD_FORWARD_FROM_MEM,
    HD_FORWARD_FROM_EX,
    HD_STALL,
    HD_FLUSH
} HdHazardAction;

/* Numeric priority (higher = more severe): NONE 0, FWD_MEM 1, FWD_EX 2,
 * STALL 3, FLUSH 4. */
uint8_t hd_priority(HdHazardAction action);

/* An ISA-independent snapshot of one pipeline stage. Zero-initialise ({0}) for
 * the default (an empty/invalid bubble); `source_regs` is borrowed. */
typedef struct {
    int valid;
    uint32_t pc;
    const uint32_t *source_regs; /* borrowed array of length num_source_regs */
    size_t num_source_regs;
    int has_dest_reg;
    uint32_t dest_reg;
    int has_dest_value;
    int64_t dest_value;
    int is_branch;
    int branch_taken;
    int branch_predicted_taken;
    int mem_read;
    int mem_write;
    int uses_alu;
    int uses_fp;
} HdPipelineSlot;

/* The outcome of hazard detection. A plain value type. */
typedef struct {
    HdHazardAction action;
    int has_forwarded_value;
    int64_t forwarded_value;
    char forwarded_from[8]; /* "EX", "MEM", or "" */
    uint32_t stall_cycles;
    uint32_t flush_count;
    char reason[192];
} HdHazardResult;

/* Whichever result is more severe (ties keep `a`). */
HdHazardResult hd_pick_higher_priority(HdHazardResult a, HdHazardResult b);

/* ── Individual detectors (stateless) ─────────────────────────────────────*/

/* RAW data hazard between ID and EX/MEM: forward from EX/MEM, or stall on a
 * load-use hazard, or none. */
HdHazardResult hd_data_detect(const HdPipelineSlot *id, const HdPipelineSlot *ex,
                              const HdPipelineSlot *mem);

/* Control hazard: FLUSH on a branch misprediction in EX, else none. */
HdHazardResult hd_control_detect(const HdPipelineSlot *ex);

/* Structural hazard: an execution-unit or (shared-cache) memory-port conflict.
 * `if_stage` and `mem_stage` may be NULL (the Rust `Option`). */
HdHazardResult hd_structural_detect(uint32_t num_alus, uint32_t num_fp_units,
                                    int split_caches, const HdPipelineSlot *id,
                                    const HdPipelineSlot *ex,
                                    const HdPipelineSlot *if_stage,
                                    const HdPipelineSlot *mem_stage);

/* ── Combined unit (tracks history) ───────────────────────────────────────*/

typedef struct {
    uint32_t num_alus;
    uint32_t num_fp_units;
    int split_caches;
    HdHazardResult *history;
    size_t history_len;
    size_t history_cap;
} HdHazardUnit;

void hd_unit_init(HdHazardUnit *unit, uint32_t num_alus, uint32_t num_fp_units,
                  int split_caches);
void hd_unit_free(HdHazardUnit *unit);

/* Run all three detectors, return the highest-priority result, and append it to
 * the history (a failed append leaves the returned result correct but the cycle
 * uncounted in the stats). */
HdHazardResult hd_unit_check(HdHazardUnit *unit, const HdPipelineSlot *if_stage,
                             const HdPipelineSlot *id, const HdPipelineSlot *ex,
                             const HdPipelineSlot *mem);

/* Recorded results (length via *out_len). */
const HdHazardResult *hd_unit_history(const HdHazardUnit *unit, size_t *out_len);

uint32_t hd_unit_stall_count(const HdHazardUnit *unit);   /* sum of stall_cycles */
uint32_t hd_unit_flush_count(const HdHazardUnit *unit);   /* # FLUSH results */
uint32_t hd_unit_forward_count(const HdHazardUnit *unit); /* # forwards */

#ifdef __cplusplus
}
#endif

#endif /* CA_HAZARD_DETECTION_H */
