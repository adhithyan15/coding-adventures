/*
 * hazard_detection.c — implementation of the pure-ISO C pipeline hazard unit.
 * ==========================================================================
 *
 * Three stateless detectors (data / control / structural) each return a value-
 * type `HdHazardResult`; the combined `HdHazardUnit` runs all three and keeps
 * the highest-priority one (FLUSH > STALL > FORWARD > NONE), recording every
 * cycle for the statistics accessors.
 */
#include "hazard_detection.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* strcpy */

uint8_t hd_priority(HdHazardAction action) {
    switch (action) {
        case HD_NONE: return 0;
        case HD_FORWARD_FROM_MEM: return 1;
        case HD_FORWARD_FROM_EX: return 2;
        case HD_STALL: return 3;
        case HD_FLUSH: return 4;
    }
    return 0;
}

/* A zeroed result with the given action and reason (mirrors HazardResult::new
 * layered on Default). */
static HdHazardResult hr_new(HdHazardAction action, const char *reason) {
    HdHazardResult r;
    r.action = action;
    r.has_forwarded_value = 0;
    r.forwarded_value = 0;
    r.forwarded_from[0] = '\0';
    r.stall_cycles = 0;
    r.flush_count = 0;
    /* reason buffers are 192 bytes; every literal/format below fits. */
    snprintf(r.reason, sizeof(r.reason), "%s", reason ? reason : "");
    return r;
}

HdHazardResult hd_pick_higher_priority(HdHazardResult a, HdHazardResult b) {
    return hd_priority(b.action) > hd_priority(a.action) ? b : a;
}

/* ── Data hazard ──────────────────────────────────────────────────────────*/

/* Check one source register against the EX and MEM destinations. */
static HdHazardResult data_check_single(uint32_t src_reg,
                                        const HdPipelineSlot *ex,
                                        const HdPipelineSlot *mem) {
    if (ex->valid && ex->has_dest_reg && ex->dest_reg == src_reg) {
        if (ex->mem_read) {
            HdHazardResult r = hr_new(HD_STALL, "");
            r.stall_cycles = 1;
            snprintf(r.reason, sizeof(r.reason),
                     "load-use hazard: R%u is being loaded by instruction at "
                     "PC=0x%04X -- must stall 1 cycle",
                     src_reg, (unsigned)ex->pc);
            return r;
        }
        HdHazardResult r = hr_new(HD_FORWARD_FROM_EX, "");
        r.has_forwarded_value = ex->has_dest_value;
        r.forwarded_value = ex->dest_value;
        strcpy(r.forwarded_from, "EX");
        snprintf(r.reason, sizeof(r.reason),
                 "RAW hazard on R%u: forwarding from EX stage (instruction at "
                 "PC=0x%04X)",
                 src_reg, (unsigned)ex->pc);
        return r;
    }

    if (mem->valid && mem->has_dest_reg && mem->dest_reg == src_reg) {
        HdHazardResult r = hr_new(HD_FORWARD_FROM_MEM, "");
        r.has_forwarded_value = mem->has_dest_value;
        r.forwarded_value = mem->dest_value;
        strcpy(r.forwarded_from, "MEM");
        snprintf(r.reason, sizeof(r.reason),
                 "RAW hazard on R%u: forwarding from MEM stage (instruction at "
                 "PC=0x%04X)",
                 src_reg, (unsigned)mem->pc);
        return r;
    }

    HdHazardResult r = hr_new(HD_NONE, "");
    snprintf(r.reason, sizeof(r.reason),
             "R%u has no pending writes in EX or MEM", src_reg);
    return r;
}

HdHazardResult hd_data_detect(const HdPipelineSlot *id, const HdPipelineSlot *ex,
                              const HdPipelineSlot *mem) {
    if (!id->valid) return hr_new(HD_NONE, "ID stage is empty (bubble)");
    if (id->num_source_regs == 0)
        return hr_new(HD_NONE, "instruction has no source registers");

    HdHazardResult worst = hr_new(HD_NONE, "no data dependencies detected");
    for (size_t i = 0; i < id->num_source_regs; i++) {
        HdHazardResult r = data_check_single(id->source_regs[i], ex, mem);
        worst = hd_pick_higher_priority(worst, r);
    }
    return worst;
}

/* ── Control hazard ───────────────────────────────────────────────────────*/

HdHazardResult hd_control_detect(const HdPipelineSlot *ex) {
    if (!ex->valid) return hr_new(HD_NONE, "EX stage is empty (bubble)");
    if (!ex->is_branch)
        return hr_new(HD_NONE, "EX stage instruction is not a branch");

    if ((ex->branch_predicted_taken != 0) == (ex->branch_taken != 0)) {
        HdHazardResult r = hr_new(HD_NONE, "");
        snprintf(r.reason, sizeof(r.reason),
                 "branch at PC=0x%04X correctly predicted %s", (unsigned)ex->pc,
                 ex->branch_taken ? "taken" : "not taken");
        return r;
    }

    const char *direction = ex->branch_taken
                                ? "predicted not-taken, actually taken"
                                : "predicted taken, actually not-taken";
    HdHazardResult r = hr_new(HD_FLUSH, "");
    r.flush_count = 2;
    snprintf(r.reason, sizeof(r.reason),
             "branch misprediction at PC=0x%04X: %s -- flushing IF and ID "
             "stages",
             (unsigned)ex->pc, direction);
    return r;
}

/* ── Structural hazard ────────────────────────────────────────────────────*/

static HdHazardResult structural_exec_conflict(uint32_t num_alus,
                                               uint32_t num_fp_units,
                                               const HdPipelineSlot *id,
                                               const HdPipelineSlot *ex) {
    if (!id->valid || !ex->valid)
        return hr_new(HD_NONE, "one or both stages are empty (bubble)");

    if (id->uses_alu && ex->uses_alu && num_alus < 2) {
        HdHazardResult r = hr_new(HD_STALL, "");
        r.stall_cycles = 1;
        snprintf(r.reason, sizeof(r.reason),
                 "structural hazard: both ID (PC=0x%04X) and EX (PC=0x%04X) "
                 "need the ALU, but only %u ALU available",
                 (unsigned)id->pc, (unsigned)ex->pc, num_alus);
        return r;
    }

    if (id->uses_fp && ex->uses_fp && num_fp_units < 2) {
        HdHazardResult r = hr_new(HD_STALL, "");
        r.stall_cycles = 1;
        snprintf(r.reason, sizeof(r.reason),
                 "structural hazard: both ID (PC=0x%04X) and EX (PC=0x%04X) "
                 "need the FP unit, but only %u FP unit available",
                 (unsigned)id->pc, (unsigned)ex->pc, num_fp_units);
        return r;
    }

    return hr_new(HD_NONE, "no execution unit conflict");
}

static HdHazardResult structural_mem_conflict(int split_caches,
                                             const HdPipelineSlot *if_stage,
                                             const HdPipelineSlot *mem_stage) {
    if (split_caches)
        return hr_new(HD_NONE, "split caches -- no memory port conflict");

    if (if_stage->valid && mem_stage->valid &&
        (mem_stage->mem_read || mem_stage->mem_write)) {
        HdHazardResult r = hr_new(HD_STALL, "");
        r.stall_cycles = 1;
        snprintf(r.reason, sizeof(r.reason),
                 "structural hazard: IF (fetch at PC=0x%04X) and MEM (%s at "
                 "PC=0x%04X) both need the shared memory bus",
                 (unsigned)if_stage->pc, mem_stage->mem_read ? "load" : "store",
                 (unsigned)mem_stage->pc);
        return r;
    }

    return hr_new(HD_NONE, "no memory port conflict");
}

HdHazardResult hd_structural_detect(uint32_t num_alus, uint32_t num_fp_units,
                                    int split_caches, const HdPipelineSlot *id,
                                    const HdPipelineSlot *ex,
                                    const HdPipelineSlot *if_stage,
                                    const HdPipelineSlot *mem_stage) {
    HdHazardResult exec = structural_exec_conflict(num_alus, num_fp_units, id, ex);
    if (exec.action != HD_NONE) return exec;

    if (if_stage != NULL && mem_stage != NULL) {
        HdHazardResult mem =
            structural_mem_conflict(split_caches, if_stage, mem_stage);
        if (mem.action != HD_NONE) return mem;
    }

    return hr_new(HD_NONE, "no structural hazards -- all resources available");
}

/* ── Combined unit ────────────────────────────────────────────────────────*/

void hd_unit_init(HdHazardUnit *unit, uint32_t num_alus, uint32_t num_fp_units,
                  int split_caches) {
    unit->num_alus = num_alus;
    unit->num_fp_units = num_fp_units;
    unit->split_caches = split_caches;
    unit->history = NULL;
    unit->history_len = 0;
    unit->history_cap = 0;
}

void hd_unit_free(HdHazardUnit *unit) {
    if (unit == NULL) return;
    free(unit->history);
    unit->history = NULL;
    unit->history_len = 0;
    unit->history_cap = 0;
}

/* Append `r` to the history; returns 0 on allocation failure (overflow-guarded
 * doubling). */
static int history_push(HdHazardUnit *unit, HdHazardResult r) {
    if (unit->history_len == unit->history_cap) {
        size_t nc = unit->history_cap ? unit->history_cap : 8;
        if (nc > ((size_t)-1) / 2 / sizeof(HdHazardResult)) return 0;
        nc *= 2;
        HdHazardResult *p = (HdHazardResult *)realloc(
            unit->history, nc * sizeof(HdHazardResult));
        if (p == NULL) return 0;
        unit->history = p;
        unit->history_cap = nc;
    }
    unit->history[unit->history_len++] = r;
    return 1;
}

HdHazardResult hd_unit_check(HdHazardUnit *unit, const HdPipelineSlot *if_stage,
                             const HdPipelineSlot *id, const HdPipelineSlot *ex,
                             const HdPipelineSlot *mem) {
    HdHazardResult control = hd_control_detect(ex);
    HdHazardResult data = hd_data_detect(id, ex, mem);
    HdHazardResult structural = hd_structural_detect(
        unit->num_alus, unit->num_fp_units, unit->split_caches, id, ex, if_stage,
        mem);

    HdHazardResult best = hd_pick_higher_priority(control, data);
    best = hd_pick_higher_priority(best, structural);
    (void)history_push(unit, best); /* best-effort; result correct regardless */
    return best;
}

const HdHazardResult *hd_unit_history(const HdHazardUnit *unit,
                                     size_t *out_len) {
    *out_len = unit->history_len;
    return unit->history;
}

uint32_t hd_unit_stall_count(const HdHazardUnit *unit) {
    uint32_t total = 0;
    for (size_t i = 0; i < unit->history_len; i++)
        total += unit->history[i].stall_cycles;
    return total;
}

uint32_t hd_unit_flush_count(const HdHazardUnit *unit) {
    uint32_t count = 0;
    for (size_t i = 0; i < unit->history_len; i++)
        if (unit->history[i].action == HD_FLUSH) count++;
    return count;
}

uint32_t hd_unit_forward_count(const HdHazardUnit *unit) {
    uint32_t count = 0;
    for (size_t i = 0; i < unit->history_len; i++) {
        HdHazardAction a = unit->history[i].action;
        if (a == HD_FORWARD_FROM_EX || a == HD_FORWARD_FROM_MEM) count++;
    }
    return count;
}
