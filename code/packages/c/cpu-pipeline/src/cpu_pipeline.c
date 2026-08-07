/*
 * cpu_pipeline.c — Configurable N-stage CPU instruction pipeline, pure ISO C17.
 * =====================================================================
 *
 * See cpu_pipeline.h. A faithful port of the Rust `cpu-pipeline` crate. The
 * pipeline holds one slot per stage; each `step()` computes the next state from
 * the current state (so transitions are simultaneous), then runs stage
 * callbacks from last stage to first, retires the last stage, tallies
 * statistics, and records a snapshot.
 */
#include "cpu_pipeline.h"

#include <stdio.h>  /* snprintf */
#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* strcmp, strncpy, memset, memcpy */

struct CpPipeline {
    CpPipelineConfig config;
    CpStageSlot stages[CP_MAX_STAGES];
    int64_t pc;
    int64_t cycle;
    int halted;
    CpPipelineStats stats;

    CpSnapshot *history;
    size_t hist_len, hist_cap;

    CpFetchFn fetch;
    void *fetch_ctx;
    CpDecodeFn decode;
    void *decode_ctx;
    CpExecuteFn execute;
    void *execute_ctx;
    CpMemoryFn memory;
    void *memory_ctx;
    CpWritebackFn writeback;
    void *writeback_ctx;

    CpHazardFn hazard;
    void *hazard_ctx;
    int has_hazard;
    CpPredictFn predict;
    void *predict_ctx;
    int has_predict;
};

/* ── Stage category ─────────────────────────────────────────────────────────*/
const char *cp_stage_category_str(CpStageCategory c) {
    switch (c) {
    case CP_FETCH:
        return "fetch";
    case CP_DECODE:
        return "decode";
    case CP_EXECUTE:
        return "execute";
    case CP_MEMORY:
        return "memory";
    case CP_WRITEBACK:
        return "writeback";
    }
    return "unknown";
}

/* ── Token ──────────────────────────────────────────────────────────────────*/
void cp_token_init(CpToken *t) {
    memset(t, 0, sizeof *t);
    t->rs1 = -1;
    t->rs2 = -1;
    t->rd = -1;
    /* opcode/forwarded_from already zeroed (empty); is_bubble = 0. */
}

void cp_token_init_bubble(CpToken *t) {
    cp_token_init(t);
    t->is_bubble = 1;
}

void cp_token_to_string(const CpToken *t, char *buf, size_t n) {
    if (t->is_bubble) {
        snprintf(buf, n, "---");
    } else if (t->opcode[0] != '\0') {
        snprintf(buf, n, "%.15s@%lld", t->opcode, (long long)t->pc);
    } else {
        snprintf(buf, n, "instr@%lld", (long long)t->pc);
    }
}

int cp_token_stage_get(const CpToken *t, const char *name, int64_t *out) {
    int i;
    for (i = 0; i < t->stage_entered_count; i++) {
        if (strcmp(t->stage_entered[i].name, name) == 0) {
            if (out) {
                *out = t->stage_entered[i].cycle;
            }
            return 1;
        }
    }
    return 0;
}

int cp_token_stage_contains(const CpToken *t, const char *name) {
    return cp_token_stage_get(t, name, NULL);
}

/* Insert only if absent (HashMap::entry().or_insert()). */
static void stage_or_insert(CpToken *t, const char *name, int64_t cycle) {
    int i;
    for (i = 0; i < t->stage_entered_count; i++) {
        if (strcmp(t->stage_entered[i].name, name) == 0) {
            return;
        }
    }
    if (t->stage_entered_count < CP_MAX_STAGES) {
        strncpy(t->stage_entered[t->stage_entered_count].name, name,
                CP_NAME_LEN - 1);
        t->stage_entered[t->stage_entered_count].name[CP_NAME_LEN - 1] = '\0';
        t->stage_entered[t->stage_entered_count].cycle = cycle;
        t->stage_entered_count++;
    }
}

/* Insert or overwrite (HashMap::insert()). */
static void stage_insert(CpToken *t, const char *name, int64_t cycle) {
    int i;
    for (i = 0; i < t->stage_entered_count; i++) {
        if (strcmp(t->stage_entered[i].name, name) == 0) {
            t->stage_entered[i].cycle = cycle;
            return;
        }
    }
    if (t->stage_entered_count < CP_MAX_STAGES) {
        strncpy(t->stage_entered[t->stage_entered_count].name, name,
                CP_NAME_LEN - 1);
        t->stage_entered[t->stage_entered_count].name[CP_NAME_LEN - 1] = '\0';
        t->stage_entered[t->stage_entered_count].cycle = cycle;
        t->stage_entered_count++;
    }
}

/* ── Config ─────────────────────────────────────────────────────────────────*/
int cp_config_num_stages(const CpPipelineConfig *c) { return c->num_stages; }

int cp_config_validate(const CpPipelineConfig *c, char *err, size_t errn) {
    int i, j, has_fetch = 0, has_writeback = 0;
    if (c->num_stages < 2) {
        if (err) {
            snprintf(err, errn, "pipeline must have at least 2 stages, got %d",
                     c->num_stages);
        }
        return 0;
    }
    if (c->num_stages > CP_MAX_STAGES) {
        if (err) {
            snprintf(err, errn, "pipeline exceeds CP_MAX_STAGES (%d), got %d",
                     CP_MAX_STAGES, c->num_stages);
        }
        return 0;
    }
    if (c->execution_width < 1) {
        if (err) {
            snprintf(err, errn, "execution width must be at least 1, got %lld",
                     (long long)c->execution_width);
        }
        return 0;
    }
    for (i = 0; i < c->num_stages; i++) {
        for (j = i + 1; j < c->num_stages; j++) {
            if (strcmp(c->stages[i].name, c->stages[j].name) == 0) {
                if (err) {
                    snprintf(err, errn, "duplicate stage name: %s",
                             c->stages[i].name);
                }
                return 0;
            }
        }
        if (c->stages[i].category == CP_FETCH) {
            has_fetch = 1;
        }
        if (c->stages[i].category == CP_WRITEBACK) {
            has_writeback = 1;
        }
    }
    if (!has_fetch) {
        if (err) {
            snprintf(err, errn, "pipeline must have at least one fetch stage");
        }
        return 0;
    }
    if (!has_writeback) {
        if (err) {
            snprintf(err, errn,
                     "pipeline must have at least one writeback stage");
        }
        return 0;
    }
    return 1;
}

static void set_stage(CpPipelineStage *s, const char *name, const char *desc,
                      CpStageCategory cat) {
    strncpy(s->name, name, CP_NAME_LEN - 1);
    s->name[CP_NAME_LEN - 1] = '\0';
    strncpy(s->description, desc, CP_DESC_LEN - 1);
    s->description[CP_DESC_LEN - 1] = '\0';
    s->category = cat;
}

void cp_config_classic_5_stage(CpPipelineConfig *out) {
    memset(out, 0, sizeof *out);
    set_stage(&out->stages[0], "IF", "Instruction Fetch", CP_FETCH);
    set_stage(&out->stages[1], "ID", "Instruction Decode", CP_DECODE);
    set_stage(&out->stages[2], "EX", "Execute", CP_EXECUTE);
    set_stage(&out->stages[3], "MEM", "Memory Access", CP_MEMORY);
    set_stage(&out->stages[4], "WB", "Write Back", CP_WRITEBACK);
    out->num_stages = 5;
    out->execution_width = 1;
}

void cp_config_deep_13_stage(CpPipelineConfig *out) {
    memset(out, 0, sizeof *out);
    set_stage(&out->stages[0], "IF1", "Fetch 1 - TLB lookup", CP_FETCH);
    set_stage(&out->stages[1], "IF2", "Fetch 2 - cache read", CP_FETCH);
    set_stage(&out->stages[2], "IF3", "Fetch 3 - align/buffer", CP_FETCH);
    set_stage(&out->stages[3], "ID1", "Decode 1 - pre-decode", CP_DECODE);
    set_stage(&out->stages[4], "ID2", "Decode 2 - full decode", CP_DECODE);
    set_stage(&out->stages[5], "ID3", "Decode 3 - register read", CP_DECODE);
    set_stage(&out->stages[6], "EX1", "Execute 1 - ALU", CP_EXECUTE);
    set_stage(&out->stages[7], "EX2", "Execute 2 - shift/multiply", CP_EXECUTE);
    set_stage(&out->stages[8], "EX3", "Execute 3 - result select", CP_EXECUTE);
    set_stage(&out->stages[9], "MEM1", "Memory 1 - address calc", CP_MEMORY);
    set_stage(&out->stages[10], "MEM2", "Memory 2 - cache access", CP_MEMORY);
    set_stage(&out->stages[11], "MEM3", "Memory 3 - data align", CP_MEMORY);
    set_stage(&out->stages[12], "WB", "Write Back", CP_WRITEBACK);
    out->num_stages = 13;
    out->execution_width = 1;
}

/* ── Stats ──────────────────────────────────────────────────────────────────*/
double cp_stats_ipc(const CpPipelineStats *s) {
    if (s->total_cycles == 0) {
        return 0.0;
    }
    return (double)s->instructions_completed / (double)s->total_cycles;
}
double cp_stats_cpi(const CpPipelineStats *s) {
    if (s->instructions_completed == 0) {
        return 0.0;
    }
    return (double)s->total_cycles / (double)s->instructions_completed;
}

/* ── Hazard ─────────────────────────────────────────────────────────────────*/
const char *cp_hazard_action_str(CpHazardAction a) {
    switch (a) {
    case CP_HAZARD_NONE:
        return "NONE";
    case CP_HAZARD_FORWARD_FROM_EX:
        return "FORWARD_FROM_EX";
    case CP_HAZARD_FORWARD_FROM_MEM:
        return "FORWARD_FROM_MEM";
    case CP_HAZARD_STALL:
        return "STALL";
    case CP_HAZARD_FLUSH:
        return "FLUSH";
    }
    return "NONE";
}

CpHazardResponse cp_hazard_response_default(void) {
    CpHazardResponse r;
    memset(&r, 0, sizeof r);
    r.action = CP_HAZARD_NONE;
    return r;
}

/* ── Snapshot ───────────────────────────────────────────────────────────────*/
const CpToken *cp_snapshot_stage(const CpSnapshot *s, const char *name) {
    int i;
    for (i = 0; i < s->num_stages; i++) {
        if (strcmp(s->stage_names[i], name) == 0) {
            return s->stages[i].occupied ? &s->stages[i].tok : NULL;
        }
    }
    return NULL;
}

/* ── Pipeline construction ──────────────────────────────────────────────────*/
CpPipeline *cp_pipeline_new(const CpPipelineConfig *config, CpFetchFn fetch,
                            void *fetch_ctx, CpDecodeFn decode, void *decode_ctx,
                            CpExecuteFn execute, void *execute_ctx,
                            CpMemoryFn memory, void *memory_ctx,
                            CpWritebackFn writeback, void *writeback_ctx,
                            char *err, size_t errn) {
    CpPipeline *p;
    if (!cp_config_validate(config, err, errn)) {
        return NULL;
    }
    p = (CpPipeline *)calloc(1, sizeof(CpPipeline));
    if (!p) {
        if (err) {
            snprintf(err, errn, "out of memory");
        }
        return NULL;
    }
    p->config = *config;
    /* stages already zeroed (unoccupied). */
    p->fetch = fetch;
    p->fetch_ctx = fetch_ctx;
    p->decode = decode;
    p->decode_ctx = decode_ctx;
    p->execute = execute;
    p->execute_ctx = execute_ctx;
    p->memory = memory;
    p->memory_ctx = memory_ctx;
    p->writeback = writeback;
    p->writeback_ctx = writeback_ctx;
    return p;
}

void cp_pipeline_free(CpPipeline *p) {
    if (!p) {
        return;
    }
    free(p->history);
    free(p);
}

void cp_pipeline_set_hazard_fn(CpPipeline *p, CpHazardFn fn, void *ctx) {
    p->hazard = fn;
    p->hazard_ctx = ctx;
    p->has_hazard = 1;
}
void cp_pipeline_set_predict_fn(CpPipeline *p, CpPredictFn fn, void *ctx) {
    p->predict = fn;
    p->predict_ctx = ctx;
    p->has_predict = 1;
}
void cp_pipeline_set_pc(CpPipeline *p, int64_t pc) { p->pc = pc; }
int64_t cp_pipeline_pc(const CpPipeline *p) { return p->pc; }
int64_t cp_pipeline_cycle(const CpPipeline *p) { return p->cycle; }
int cp_pipeline_is_halted(const CpPipeline *p) { return p->halted; }
CpPipelineStats cp_pipeline_stats(const CpPipeline *p) { return p->stats; }
const CpPipelineConfig *cp_pipeline_config(const CpPipeline *p) {
    return &p->config;
}

const CpToken *cp_pipeline_stage_contents(const CpPipeline *p,
                                          const char *stage_name) {
    int i;
    for (i = 0; i < p->config.num_stages; i++) {
        if (strcmp(p->config.stages[i].name, stage_name) == 0) {
            return p->stages[i].occupied ? &p->stages[i].tok : NULL;
        }
    }
    return NULL;
}

/* Build a snapshot from the current pipeline state. */
static void take_snapshot(const CpPipeline *p, CpSnapshot *out, int stalled,
                          int flushing) {
    int i;
    out->cycle = p->cycle;
    out->pc = p->pc;
    out->stalled = stalled;
    out->flushing = flushing;
    out->num_stages = p->config.num_stages;
    for (i = 0; i < p->config.num_stages; i++) {
        out->stages[i] = p->stages[i];
        strncpy(out->stage_names[i], p->config.stages[i].name, CP_NAME_LEN - 1);
        out->stage_names[i][CP_NAME_LEN - 1] = '\0';
    }
    for (; i < CP_MAX_STAGES; i++) {
        out->stages[i].occupied = 0;
        out->stage_names[i][0] = '\0';
    }
}

/* Fetch a new instruction into a fresh token and advance the PC. */
static CpToken fetch_new_instruction(CpPipeline *p) {
    CpToken tok;
    cp_token_init(&tok);
    tok.pc = p->pc;
    tok.raw_instruction = p->fetch(p->fetch_ctx, p->pc);
    stage_insert(&tok, p->config.stages[0].name, p->cycle);
    if (p->has_predict) {
        p->pc = p->predict(p->predict_ctx, p->pc);
    } else {
        p->pc += 4;
    }
    return tok;
}

static int grow_history(CpPipeline *p) {
    size_t nc;
    CpSnapshot *nd;
    if (p->hist_len < p->hist_cap) {
        return 1;
    }
    nc = p->hist_cap ? p->hist_cap : 8;
    if (nc > (size_t)-1 / 2) {
        return 0;
    }
    nc *= 2;
    if (nc > (size_t)-1 / sizeof(CpSnapshot)) {
        return 0;
    }
    nd = (CpSnapshot *)realloc(p->history, nc * sizeof(CpSnapshot));
    if (!nd) {
        return 0;
    }
    p->history = nd;
    p->hist_cap = nc;
    return 1;
}

void cp_pipeline_step(CpPipeline *p, CpSnapshot *out) {
    int num_stages = p->config.num_stages;
    CpHazardResponse hz;
    CpStageSlot next[CP_MAX_STAGES];
    int stalled = 0, flushing = 0;
    int i;
    CpSnapshot snap;

    if (p->halted) {
        take_snapshot(p, &snap, 0, 0);
        if (out) {
            *out = snap;
        }
        return;
    }

    p->cycle += 1;
    p->stats.total_cycles += 1;

    /* Phase 1: hazard check on the current state. */
    if (p->has_hazard) {
        CpStageSlot cur[CP_MAX_STAGES];
        for (i = 0; i < num_stages; i++) {
            cur[i] = p->stages[i];
        }
        hz = p->hazard(p->hazard_ctx, cur, num_stages);
    } else {
        hz = cp_hazard_response_default();
    }

    /* Phase 2: compute next state. */
    for (i = 0; i < CP_MAX_STAGES; i++) {
        next[i].occupied = 0;
    }

    if (hz.action == CP_HAZARD_FLUSH) {
        int flush_count;
        size_t fc = hz.flush_count; /* keep unsigned; clamp BEFORE narrowing */
        flushing = 1;
        p->stats.flush_cycles += 1;

        if (fc == 0) {
            int k;
            for (k = 0; k < num_stages; k++) {
                if (p->config.stages[k].category == CP_EXECUTE) {
                    fc = (size_t)k;
                    break;
                }
            }
            if (fc == 0) {
                fc = 1;
            }
        }
        if (fc > (size_t)num_stages) {
            fc = (size_t)num_stages;
        }
        flush_count = (int)fc; /* now in [1, num_stages], safe to narrow */

        for (i = num_stages - 1; i >= flush_count; i--) {
            if (i > flush_count) {
                next[i] = p->stages[i - 1];
            } else { /* i == flush_count (>= 1) */
                CpToken bubble;
                cp_token_init_bubble(&bubble);
                stage_insert(&bubble, p->config.stages[i].name, p->cycle);
                next[i].occupied = 1;
                next[i].tok = bubble;
            }
        }
        for (i = 0; i < flush_count; i++) {
            CpToken bubble;
            cp_token_init_bubble(&bubble);
            stage_insert(&bubble, p->config.stages[i].name, p->cycle);
            next[i].occupied = 1;
            next[i].tok = bubble;
        }

        p->pc = hz.redirect_pc;
        {
            CpToken tok = fetch_new_instruction(p);
            next[0].occupied = 1;
            next[0].tok = tok;
        }
    } else if (hz.action == CP_HAZARD_STALL) {
        int stall_point;
        size_t sp = hz.stall_stages; /* keep unsigned; clamp BEFORE narrowing */
        stalled = 1;
        p->stats.stall_cycles += 1;

        if (sp == 0) {
            int k;
            for (k = 0; k < num_stages; k++) {
                if (p->config.stages[k].category == CP_EXECUTE) {
                    sp = (size_t)k;
                    break;
                }
            }
            if (sp == 0) {
                sp = 1;
            }
        }
        if (sp >= (size_t)num_stages) {
            sp = (size_t)(num_stages - 1);
        }
        stall_point = (int)sp; /* now in [1, num_stages-1], safe to narrow */

        for (i = num_stages - 1; i >= stall_point + 1; i--) {
            next[i] = p->stages[i - 1];
        }
        {
            CpToken bubble;
            cp_token_init_bubble(&bubble);
            stage_insert(&bubble, p->config.stages[stall_point].name, p->cycle);
            next[stall_point].occupied = 1;
            next[stall_point].tok = bubble;
        }
        for (i = 0; i < stall_point; i++) {
            next[i] = p->stages[i];
        }
        /* PC does not advance during a stall. */
    } else {
        /* NONE or FORWARD: normal advancement. */
        if (hz.action == CP_HAZARD_FORWARD_FROM_EX ||
            hz.action == CP_HAZARD_FORWARD_FROM_MEM) {
            for (i = 0; i < num_stages; i++) {
                if (p->config.stages[i].category == CP_DECODE) {
                    if (p->stages[i].occupied && !p->stages[i].tok.is_bubble) {
                        p->stages[i].tok.alu_result = hz.forward_value;
                        strncpy(p->stages[i].tok.forwarded_from,
                                hz.forward_source, CP_NAME_LEN - 1);
                        p->stages[i].tok.forwarded_from[CP_NAME_LEN - 1] = '\0';
                        break;
                    }
                }
            }
        }

        for (i = num_stages - 1; i >= 1; i--) {
            next[i] = p->stages[i - 1];
        }
        {
            CpToken tok = fetch_new_instruction(p);
            next[0].occupied = 1;
            next[0].tok = tok;
        }
    }

    /* Phase 3: commit. */
    for (i = 0; i < num_stages; i++) {
        p->stages[i] = next[i];
    }
    for (; i < CP_MAX_STAGES; i++) {
        p->stages[i].occupied = 0;
    }

    /* Phase 4: stage callbacks, last to first. */
    for (i = num_stages - 1; i >= 0; i--) {
        CpStageCategory cat;
        const char *name;
        if (!p->stages[i].occupied || p->stages[i].tok.is_bubble) {
            continue;
        }
        cat = p->config.stages[i].category;
        name = p->config.stages[i].name;

        stage_or_insert(&p->stages[i].tok, name, p->cycle);

        if (cat == CP_DECODE) {
            if (p->stages[i].tok.opcode[0] == '\0') {
                p->decode(p->decode_ctx, p->stages[i].tok.raw_instruction,
                          &p->stages[i].tok);
            }
        } else if (cat == CP_EXECUTE) {
            int64_t c;
            if (cp_token_stage_get(&p->stages[i].tok, name, &c) &&
                c == p->cycle) {
                p->execute(p->execute_ctx, &p->stages[i].tok);
            }
        } else if (cat == CP_MEMORY) {
            int64_t c;
            if (cp_token_stage_get(&p->stages[i].tok, name, &c) &&
                c == p->cycle) {
                p->memory(p->memory_ctx, &p->stages[i].tok);
            }
        }
        /* CP_FETCH / CP_WRITEBACK: nothing here. */
    }

    /* Phase 5: retire the last stage. */
    if (p->stages[num_stages - 1].occupied &&
        !p->stages[num_stages - 1].tok.is_bubble) {
        const CpToken *tok = &p->stages[num_stages - 1].tok;
        p->writeback(p->writeback_ctx, tok);
        p->stats.instructions_completed += 1;
        if (tok->is_halt) {
            p->halted = 1;
        }
    }

    /* Phase 6: count bubbles. */
    for (i = 0; i < num_stages; i++) {
        if (p->stages[i].occupied && p->stages[i].tok.is_bubble) {
            p->stats.bubble_cycles += 1;
        }
    }

    /* Phase 7: snapshot + history. */
    take_snapshot(p, &snap, stalled, flushing);
    if (grow_history(p)) {
        p->history[p->hist_len++] = snap;
    }
    if (out) {
        *out = snap;
    }
}

CpPipelineStats cp_pipeline_run(CpPipeline *p, int64_t max_cycles) {
    while (p->cycle < max_cycles && !p->halted) {
        cp_pipeline_step(p, NULL);
    }
    return p->stats;
}

void cp_pipeline_snapshot(const CpPipeline *p, CpSnapshot *out) {
    take_snapshot(p, out, 0, 0);
}

size_t cp_pipeline_trace_count(const CpPipeline *p) { return p->hist_len; }
int cp_pipeline_trace(const CpPipeline *p, size_t i, CpSnapshot *out) {
    if (i >= p->hist_len) {
        return 0;
    }
    if (out) {
        *out = p->history[i];
    }
    return 1;
}
