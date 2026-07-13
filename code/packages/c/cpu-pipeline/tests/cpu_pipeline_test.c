/*
 * Tests for cpu-pipeline, mirroring the Rust crate's unit tests across token,
 * snapshot, and pipeline modules, using the header-only iso_test.h harness.
 */
#include "iso_test.h"

#include "cpu_pipeline.h"

#include <string.h>

#define EPS 1e-3

/* ── Test ISA: raw = (opcode<<24) | (rd<<16) | (rs1<<8) | rs2 ─────────────── */
enum { OP_NOP = 0x00, OP_ADD = 0x01, OP_LDR = 0x02, OP_BEQ = 0x04,
       OP_HALT = 0xFF };
static int64_t make_instruction(int64_t op, int64_t rd, int64_t rs1,
                                int64_t rs2) {
    return (op << 24) | (rd << 16) | (rs1 << 8) | rs2;
}

/* ── Stage callbacks ─────────────────────────────────────────────────────── */
typedef struct {
    const int64_t *instrs;
    size_t n;
} FetchCtx;
static int64_t simple_fetch(void *ctx, int64_t pc) {
    FetchCtx *c = (FetchCtx *)ctx;
    size_t idx = (size_t)(pc / 4);
    return idx < c->n ? c->instrs[idx] : 0;
}
static void simple_decode(void *ctx, int64_t raw, CpToken *t) {
    int64_t op = (raw >> 24) & 0xFF, rd = (raw >> 16) & 0xFF,
            rs1 = (raw >> 8) & 0xFF, rs2 = raw & 0xFF;
    (void)ctx;
    switch (op) {
    case OP_ADD:
        strcpy(t->opcode, "ADD");
        t->rd = rd;
        t->rs1 = rs1;
        t->rs2 = rs2;
        t->reg_write = 1;
        break;
    case OP_LDR:
        strcpy(t->opcode, "LDR");
        t->rd = rd;
        t->rs1 = rs1;
        t->mem_read = 1;
        t->reg_write = 1;
        break;
    case 0x03:
        strcpy(t->opcode, "STR");
        t->rs1 = rs1;
        t->rs2 = rs2;
        t->mem_write = 1;
        break;
    case OP_BEQ:
        strcpy(t->opcode, "BEQ");
        t->rs1 = rs1;
        t->rs2 = rs2;
        t->is_branch = 1;
        break;
    case OP_HALT:
        strcpy(t->opcode, "HALT");
        t->is_halt = 1;
        break;
    default:
        strcpy(t->opcode, "NOP");
        break;
    }
}
static void simple_execute(void *ctx, CpToken *t) {
    (void)ctx;
    if (strcmp(t->opcode, "ADD") == 0) {
        t->alu_result = t->rs1 + t->rs2;
    } else if (strcmp(t->opcode, "LDR") == 0 || strcmp(t->opcode, "STR") == 0) {
        t->alu_result = t->rs1 + t->immediate;
    } else if (strcmp(t->opcode, "BEQ") == 0) {
        t->branch_target = t->pc + t->immediate;
    }
}
static void simple_memory(void *ctx, CpToken *t) {
    (void)ctx;
    if (t->mem_read) {
        t->mem_data = 42;
        t->write_data = t->mem_data;
    } else {
        t->write_data = t->alu_result;
    }
}
typedef struct {
    int64_t pcs[256];
    size_t n;
} CompletedCtx;
static void completed_writeback(void *ctx, const CpToken *t) {
    CompletedCtx *c = (CompletedCtx *)ctx;
    if (c && c->n < 256) {
        c->pcs[c->n++] = t->pc;
    }
}
static void noop_writeback(void *ctx, const CpToken *t) {
    (void)ctx;
    (void)t;
}

/* ── Hazard callbacks ────────────────────────────────────────────────────── */
typedef struct {
    int count;
} CountCtx;
typedef struct {
    int done;
} FlagCtx;

static CpHazardResponse haz_stall_on_3(void *ctx, const CpStageSlot *s, int n) {
    CountCtx *c = (CountCtx *)ctx;
    CpHazardResponse r = cp_hazard_response_default();
    (void)s;
    (void)n;
    if (++c->count == 3) {
        r.action = CP_HAZARD_STALL;
        r.stall_stages = 2;
    }
    return r;
}
static CpHazardResponse haz_stall_default(void *ctx, const CpStageSlot *s,
                                          int n) {
    CountCtx *c = (CountCtx *)ctx;
    CpHazardResponse r = cp_hazard_response_default();
    (void)s;
    (void)n;
    if (++c->count == 3) {
        r.action = CP_HAZARD_STALL;
        r.stall_stages = 0; /* default point */
    }
    return r;
}
static CpHazardResponse haz_stall_big(void *ctx, const CpStageSlot *s, int n) {
    CountCtx *c = (CountCtx *)ctx;
    CpHazardResponse r = cp_hazard_response_default();
    (void)s;
    (void)n;
    if (++c->count == 3) {
        r.action = CP_HAZARD_STALL;
        r.stall_stages = 100; /* clamped */
    }
    return r;
}
static CpHazardResponse haz_stall_mod5(void *ctx, const CpStageSlot *s, int n) {
    CountCtx *c = (CountCtx *)ctx;
    CpHazardResponse r = cp_hazard_response_default();
    (void)s;
    (void)n;
    if (++c->count % 5 == 0) {
        r.action = CP_HAZARD_STALL;
        r.stall_stages = 2;
    }
    return r;
}
static CpHazardResponse haz_stall_ldr_add(void *ctx, const CpStageSlot *s,
                                          int n) {
    FlagCtx *c = (FlagCtx *)ctx;
    CpHazardResponse r = cp_hazard_response_default();
    if (!c->done && n >= 3 && s[2].occupied && s[1].occupied &&
        !s[2].tok.is_bubble && strcmp(s[2].tok.opcode, "LDR") == 0 &&
        !s[1].tok.is_bubble && strcmp(s[1].tok.opcode, "ADD") == 0) {
        c->done = 1;
        r.action = CP_HAZARD_STALL;
        r.stall_stages = 2;
    }
    return r;
}
static CpHazardResponse haz_forward_ex_on_4(void *ctx, const CpStageSlot *s,
                                            int n) {
    CountCtx *c = (CountCtx *)ctx;
    CpHazardResponse r = cp_hazard_response_default();
    (void)s;
    (void)n;
    if (++c->count == 4) {
        r.action = CP_HAZARD_FORWARD_FROM_EX;
        r.forward_value = 99;
        strcpy(r.forward_source, "EX");
    }
    return r;
}
static CpHazardResponse haz_forward_mem_on_4(void *ctx, const CpStageSlot *s,
                                             int n) {
    CountCtx *c = (CountCtx *)ctx;
    CpHazardResponse r = cp_hazard_response_default();
    (void)s;
    (void)n;
    if (++c->count == 4) {
        r.action = CP_HAZARD_FORWARD_FROM_MEM;
        r.forward_value = 77;
        strcpy(r.forward_source, "MEM");
    }
    return r;
}
static CpHazardResponse haz_flush_branch(void *ctx, const CpStageSlot *s,
                                         int n) {
    FlagCtx *c = (FlagCtx *)ctx;
    CpHazardResponse r = cp_hazard_response_default();
    if (!c->done && n >= 3 && s[2].occupied && !s[2].tok.is_bubble &&
        s[2].tok.is_branch) {
        c->done = 1;
        r.action = CP_HAZARD_FLUSH;
        r.flush_count = 2;
        r.redirect_pc = 20;
    }
    return r;
}
static CpHazardResponse haz_flush_default(void *ctx, const CpStageSlot *s,
                                          int n) {
    FlagCtx *c = (FlagCtx *)ctx;
    CpHazardResponse r = cp_hazard_response_default();
    if (!c->done && n >= 3 && s[2].occupied && !s[2].tok.is_bubble) {
        c->done = 1;
        r.action = CP_HAZARD_FLUSH;
        r.flush_count = 0; /* default */
        r.redirect_pc = 100;
    }
    return r;
}
static CpHazardResponse haz_flush_big(void *ctx, const CpStageSlot *s, int n) {
    FlagCtx *c = (FlagCtx *)ctx;
    CpHazardResponse r = cp_hazard_response_default();
    if (!c->done && n >= 3 && s[2].occupied && !s[2].tok.is_bubble) {
        c->done = 1;
        r.action = CP_HAZARD_FLUSH;
        r.flush_count = 100; /* clamped */
        r.redirect_pc = 0;
    }
    return r;
}
static CpHazardResponse haz_flush_huge(void *ctx, const CpStageSlot *s, int n) {
    FlagCtx *c = (FlagCtx *)ctx;
    CpHazardResponse r = cp_hazard_response_default();
    if (!c->done && n >= 3 && s[2].occupied && !s[2].tok.is_bubble) {
        c->done = 1;
        r.action = CP_HAZARD_FLUSH;
        r.flush_count = (size_t)-1; /* SIZE_MAX: must clamp, not go negative */
        r.redirect_pc = 0;
    }
    return r;
}
static CpHazardResponse haz_stall_huge(void *ctx, const CpStageSlot *s, int n) {
    CountCtx *c = (CountCtx *)ctx;
    CpHazardResponse r = cp_hazard_response_default();
    (void)s;
    (void)n;
    if (++c->count == 3) {
        r.action = CP_HAZARD_STALL;
        r.stall_stages = (size_t)-1; /* SIZE_MAX: must clamp */
    }
    return r;
}
static CpHazardResponse haz_multi(void *ctx, const CpStageSlot *s, int n) {
    CountCtx *c = (CountCtx *)ctx;
    CpHazardResponse r = cp_hazard_response_default();
    int v;
    (void)s;
    (void)n;
    v = ++c->count;
    if (v == 5 || v == 10) {
        r.action = CP_HAZARD_STALL;
        r.stall_stages = 2;
    } else if (v == 15) {
        r.action = CP_HAZARD_FLUSH;
        r.flush_count = 2;
        r.redirect_pc = 0;
    }
    return r;
}

static int64_t predict_plus8(void *ctx, int64_t pc) {
    (void)ctx;
    return pc + 8;
}

/* Fill a config stage slot in place (for custom-config tests). */
static void set_stage_helper(CpPipelineConfig *cfg, int i, const char *name,
                             CpStageCategory cat) {
    strncpy(cfg->stages[i].name, name, CP_NAME_LEN - 1);
    cfg->stages[i].name[CP_NAME_LEN - 1] = '\0';
    cfg->stages[i].description[0] = '\0';
    cfg->stages[i].category = cat;
}

static CpPipeline *new_test_pipeline(FetchCtx *fc, CompletedCtx *cc) {
    CpPipelineConfig cfg;
    char err[128];
    cp_config_classic_5_stage(&cfg);
    return cp_pipeline_new(&cfg, simple_fetch, fc, simple_decode, NULL,
                           simple_execute, NULL, simple_memory, NULL,
                           cc ? completed_writeback : noop_writeback, cc, err,
                           sizeof err);
}

/* Fill an instruction buffer with `count` ADD instructions. */
static void fill_adds(int64_t *buf, size_t count) {
    size_t i;
    for (i = 0; i < count; i++) {
        buf[i] = make_instruction(OP_ADD, 1, 2, 3);
    }
}

int main(void) {
    /* ══ Token ═════════════════════════════════════════════════════════════ */
    {
        CpToken t;
        cp_token_init(&t);
        ISO_CHECK_EQ_INT(t.rs1, -1);
        ISO_CHECK_EQ_INT(t.rs2, -1);
        ISO_CHECK_EQ_INT(t.rd, -1);
        ISO_CHECK(!t.is_bubble);
        ISO_CHECK_EQ_INT(t.stage_entered_count, 0);
    }
    {
        CpToken b;
        char buf[32];
        cp_token_init_bubble(&b);
        ISO_CHECK(b.is_bubble);
        cp_token_to_string(&b, buf, sizeof buf);
        ISO_CHECK_STR_EQ(buf, "---");
    }
    {
        CpToken t;
        char buf[32];
        cp_token_init(&t);
        strcpy(t.opcode, "ADD");
        t.pc = 100;
        cp_token_to_string(&t, buf, sizeof buf);
        ISO_CHECK_STR_EQ(buf, "ADD@100");
        cp_token_init(&t);
        t.pc = 200;
        cp_token_to_string(&t, buf, sizeof buf);
        ISO_CHECK_STR_EQ(buf, "instr@200");
    }

    /* ══ StageCategory / stage string ═════════════════════════════════════ */
    ISO_CHECK_STR_EQ(cp_stage_category_str(CP_FETCH), "fetch");
    ISO_CHECK_STR_EQ(cp_stage_category_str(CP_DECODE), "decode");
    ISO_CHECK_STR_EQ(cp_stage_category_str(CP_EXECUTE), "execute");
    ISO_CHECK_STR_EQ(cp_stage_category_str(CP_MEMORY), "memory");
    ISO_CHECK_STR_EQ(cp_stage_category_str(CP_WRITEBACK), "writeback");

    /* ══ Config presets + validation ══════════════════════════════════════ */
    {
        CpPipelineConfig c;
        cp_config_classic_5_stage(&c);
        ISO_CHECK_EQ_INT(cp_config_num_stages(&c), 5);
        ISO_CHECK(cp_config_validate(&c, NULL, 0));
        ISO_CHECK_STR_EQ(c.stages[0].name, "IF");
        ISO_CHECK_STR_EQ(c.stages[4].name, "WB");
    }
    {
        CpPipelineConfig c;
        cp_config_deep_13_stage(&c);
        ISO_CHECK_EQ_INT(cp_config_num_stages(&c), 13);
        ISO_CHECK(cp_config_validate(&c, NULL, 0));
    }
    {
        CpPipelineConfig c;
        char e[128];
        memset(&c, 0, sizeof c);
        c.num_stages = 1;
        c.execution_width = 1;
        strcpy(c.stages[0].name, "IF");
        c.stages[0].category = CP_FETCH;
        ISO_CHECK(!cp_config_validate(&c, e, sizeof e));
        cp_config_classic_5_stage(&c);
        c.execution_width = 0;
        ISO_CHECK(!cp_config_validate(&c, e, sizeof e));
        cp_config_classic_5_stage(&c);
        strcpy(c.stages[1].name, "IF");
        ISO_CHECK(!cp_config_validate(&c, e, sizeof e));
        memset(&c, 0, sizeof c);
        c.num_stages = 2;
        c.execution_width = 1;
        strcpy(c.stages[0].name, "EX");
        c.stages[0].category = CP_EXECUTE;
        strcpy(c.stages[1].name, "WB");
        c.stages[1].category = CP_WRITEBACK;
        ISO_CHECK(!cp_config_validate(&c, e, sizeof e));
        memset(&c, 0, sizeof c);
        c.num_stages = 2;
        c.execution_width = 1;
        strcpy(c.stages[0].name, "IF");
        c.stages[0].category = CP_FETCH;
        strcpy(c.stages[1].name, "EX");
        c.stages[1].category = CP_EXECUTE;
        ISO_CHECK(!cp_config_validate(&c, e, sizeof e));
        memset(&c, 0, sizeof c);
        c.num_stages = 2;
        c.execution_width = 1;
        strcpy(c.stages[0].name, "IF");
        c.stages[0].category = CP_FETCH;
        strcpy(c.stages[1].name, "WB");
        c.stages[1].category = CP_WRITEBACK;
        ISO_CHECK(cp_config_validate(&c, e, sizeof e));
    }

    /* ══ Stats: IPC / CPI ═════════════════════════════════════════════════ */
    {
        CpPipelineStats s;
        memset(&s, 0, sizeof s);
        s.total_cycles = 100;
        s.instructions_completed = 80;
        ISO_CHECK_EQ_DBL(cp_stats_ipc(&s), 0.8, EPS);
        memset(&s, 0, sizeof s);
        s.total_cycles = 120;
        s.instructions_completed = 100;
        ISO_CHECK_EQ_DBL(cp_stats_cpi(&s), 1.2, EPS);
        memset(&s, 0, sizeof s);
        ISO_CHECK_EQ_DBL(cp_stats_ipc(&s), 0.0, EPS);
        s.total_cycles = 10;
        ISO_CHECK_EQ_DBL(cp_stats_cpi(&s), 0.0, EPS);
    }

    /* ══ HazardAction strings ═════════════════════════════════════════════ */
    ISO_CHECK_STR_EQ(cp_hazard_action_str(CP_HAZARD_NONE), "NONE");
    ISO_CHECK_STR_EQ(cp_hazard_action_str(CP_HAZARD_FORWARD_FROM_EX),
                     "FORWARD_FROM_EX");
    ISO_CHECK_STR_EQ(cp_hazard_action_str(CP_HAZARD_FORWARD_FROM_MEM),
                     "FORWARD_FROM_MEM");
    ISO_CHECK_STR_EQ(cp_hazard_action_str(CP_HAZARD_STALL), "STALL");
    ISO_CHECK_STR_EQ(cp_hazard_action_str(CP_HAZARD_FLUSH), "FLUSH");

    /* ══ Basic pipeline ═══════════════════════════════════════════════════ */
    {
        int64_t instrs[1];
        FetchCtx fc = {instrs, 1};
        CpPipeline *p;
        instrs[0] = make_instruction(OP_ADD, 1, 2, 3);
        p = new_test_pipeline(&fc, NULL);
        ISO_CHECK(!cp_pipeline_is_halted(p));
        ISO_CHECK_EQ_INT(cp_pipeline_cycle(p), 0);
        ISO_CHECK_EQ_INT(cp_pipeline_pc(p), 0);
        ISO_CHECK_EQ_INT(cp_pipeline_config(p)->num_stages, 5);
        cp_pipeline_free(p);
    }
    {
        CpPipelineConfig c;
        char e[128];
        CpPipeline *p;
        memset(&c, 0, sizeof c);
        c.num_stages = 1;
        c.execution_width = 1;
        strcpy(c.stages[0].name, "IF");
        c.stages[0].category = CP_FETCH;
        p = cp_pipeline_new(&c, simple_fetch, NULL, simple_decode, NULL,
                            simple_execute, NULL, simple_memory, NULL,
                            noop_writeback, NULL, e, sizeof e);
        ISO_CHECK(p == NULL);
    }
    {
        int64_t instrs[5];
        FetchCtx fc;
        CompletedCtx cc = {{0}, 0};
        CpPipeline *p;
        int i;
        instrs[0] = make_instruction(OP_ADD, 1, 2, 3);
        for (i = 1; i < 5; i++) {
            instrs[i] = make_instruction(OP_NOP, 0, 0, 0);
        }
        fc.instrs = instrs;
        fc.n = 5;
        p = new_test_pipeline(&fc, &cc);
        for (i = 0; i < 5; i++) {
            cp_pipeline_step(p, NULL);
        }
        ISO_CHECK(cc.n > 0);
        ISO_CHECK_EQ_INT(cc.pcs[0], 0);
        cp_pipeline_free(p);
    }
    {
        int64_t instrs[20];
        FetchCtx fc = {instrs, 20};
        CompletedCtx cc = {{0}, 0};
        CpPipeline *p;
        int i;
        fill_adds(instrs, 20);
        p = new_test_pipeline(&fc, &cc);
        for (i = 0; i < 4; i++) {
            cp_pipeline_step(p, NULL);
        }
        ISO_CHECK_EQ_UINT(cc.n, 0);
        cp_pipeline_step(p, NULL);
        ISO_CHECK_EQ_UINT(cc.n, 1);
        cp_pipeline_step(p, NULL);
        ISO_CHECK_EQ_UINT(cc.n, 2);
        cp_pipeline_step(p, NULL);
        ISO_CHECK_EQ_UINT(cc.n, 3);
        cp_pipeline_free(p);
    }
    {
        int64_t instrs[100];
        FetchCtx fc = {instrs, 100};
        CpPipeline *p;
        CpPipelineStats st;
        double ipc;
        int i;
        fill_adds(instrs, 100);
        p = new_test_pipeline(&fc, NULL);
        for (i = 0; i < 50; i++) {
            cp_pipeline_step(p, NULL);
        }
        st = cp_pipeline_stats(p);
        ISO_CHECK_EQ_INT(st.instructions_completed, 46);
        ipc = cp_stats_ipc(&st);
        ISO_CHECK(ipc > 0.85 && ipc < 1.01);
        cp_pipeline_free(p);
    }
    {
        int64_t instrs[5];
        FetchCtx fc = {instrs, 5};
        CompletedCtx cc = {{0}, 0};
        CpPipeline *p;
        CpPipelineStats st;
        instrs[0] = make_instruction(OP_ADD, 1, 2, 3);
        instrs[1] = make_instruction(OP_ADD, 4, 5, 6);
        instrs[2] = make_instruction(OP_HALT, 0, 0, 0);
        instrs[3] = make_instruction(OP_NOP, 0, 0, 0);
        instrs[4] = make_instruction(OP_NOP, 0, 0, 0);
        p = new_test_pipeline(&fc, &cc);
        st = cp_pipeline_run(p, 100);
        ISO_CHECK(cp_pipeline_is_halted(p));
        ISO_CHECK_EQ_INT(cp_pipeline_cycle(p), 7);
        ISO_CHECK_EQ_INT(st.instructions_completed, 3);
        /* stats vs callback count agree */
        ISO_CHECK_EQ_UINT(cc.n, 3);
        cp_pipeline_free(p);
    }
    {
        FetchCtx fc = {NULL, 0};
        CpPipeline *p = new_test_pipeline(&fc, NULL);
        CpSnapshot snap;
        cp_pipeline_step(p, &snap);
        ISO_CHECK_EQ_INT(snap.cycle, 1);
        cp_pipeline_free(p);
    }

    /* ══ Stall ════════════════════════════════════════════════════════════ */
    {
        /* freeze earlier stages: LDR in EX + ADD in ID -> stall */
        int64_t instrs[8];
        FetchCtx fc = {instrs, 8};
        FlagCtx hc = {0};
        CpPipeline *p;
        const CpToken *ex, *id;
        CpSnapshot snap;
        int i;
        instrs[0] = make_instruction(OP_LDR, 1, 2, 0);
        instrs[1] = make_instruction(OP_ADD, 3, 1, 4);
        instrs[2] = make_instruction(OP_ADD, 5, 6, 7);
        for (i = 3; i < 8; i++) {
            instrs[i] = make_instruction(OP_NOP, 0, 0, 0);
        }
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_set_hazard_fn(p, haz_stall_ldr_add, &hc);
        cp_pipeline_step(p, NULL);
        cp_pipeline_step(p, NULL);
        cp_pipeline_step(p, NULL);
        cp_pipeline_step(p, &snap); /* cycle 4: stall */
        ISO_CHECK(snap.stalled);
        ex = cp_pipeline_stage_contents(p, "EX");
        ISO_CHECK(ex != NULL && ex->is_bubble);
        id = cp_pipeline_stage_contents(p, "ID");
        ISO_CHECK(id != NULL && strcmp(id->opcode, "ADD") == 0);
        ISO_CHECK_EQ_INT(cp_pipeline_stats(p).stall_cycles, 1);
        cp_pipeline_free(p);
    }
    {
        /* stall on 3rd call inserts bubble at EX */
        int64_t instrs[10];
        FetchCtx fc = {instrs, 10};
        CountCtx hc = {0};
        CpPipeline *p;
        const CpToken *ex;
        int i;
        fill_adds(instrs, 10);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_set_hazard_fn(p, haz_stall_on_3, &hc);
        for (i = 0; i < 3; i++) {
            cp_pipeline_step(p, NULL);
        }
        ex = cp_pipeline_stage_contents(p, "EX");
        ISO_CHECK(ex != NULL && ex->is_bubble);
        cp_pipeline_free(p);
    }
    {
        /* default stall point (stall_stages=0) still stalls once */
        int64_t instrs[20];
        FetchCtx fc = {instrs, 20};
        CountCtx hc = {0};
        CpPipeline *p;
        int i;
        fill_adds(instrs, 20);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_set_hazard_fn(p, haz_stall_default, &hc);
        for (i = 0; i < 5; i++) {
            cp_pipeline_step(p, NULL);
        }
        ISO_CHECK_EQ_INT(cp_pipeline_stats(p).stall_cycles, 1);
        cp_pipeline_free(p);
    }
    {
        /* stall_stages larger than pipeline: clamped, no crash */
        int64_t instrs[20];
        FetchCtx fc = {instrs, 20};
        CountCtx hc = {0};
        CpPipeline *p;
        int i;
        fill_adds(instrs, 20);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_set_hazard_fn(p, haz_stall_big, &hc);
        for (i = 0; i < 10; i++) {
            cp_pipeline_step(p, NULL);
        }
        ISO_CHECK(cp_pipeline_stats(p).stall_cycles >= 1);
        cp_pipeline_free(p);
    }
    {
        /* stalls reduce IPC below 1.0 */
        int64_t instrs[50];
        FetchCtx fc = {instrs, 50};
        CountCtx hc = {0};
        CpPipeline *p;
        CpPipelineStats st;
        int i;
        fill_adds(instrs, 50);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_set_hazard_fn(p, haz_stall_mod5, &hc);
        for (i = 0; i < 30; i++) {
            cp_pipeline_step(p, NULL);
        }
        st = cp_pipeline_stats(p);
        ISO_CHECK(cp_stats_ipc(&st) < 1.0);
        ISO_CHECK(st.stall_cycles > 0);
        cp_pipeline_free(p);
    }

    /* ══ Flush ════════════════════════════════════════════════════════════ */
    {
        int64_t instrs[8];
        FetchCtx fc = {instrs, 8};
        FlagCtx hc = {0};
        CpPipeline *p;
        CpSnapshot snap;
        instrs[0] = make_instruction(OP_BEQ, 0, 1, 2);
        instrs[1] = make_instruction(OP_ADD, 1, 2, 3);
        instrs[2] = make_instruction(OP_ADD, 4, 5, 6);
        instrs[3] = make_instruction(OP_NOP, 0, 0, 0);
        instrs[4] = make_instruction(OP_NOP, 0, 0, 0);
        instrs[5] = make_instruction(OP_ADD, 7, 8, 9);
        instrs[6] = make_instruction(OP_NOP, 0, 0, 0);
        instrs[7] = make_instruction(OP_NOP, 0, 0, 0);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_set_hazard_fn(p, haz_flush_branch, &hc);
        cp_pipeline_step(p, NULL);
        cp_pipeline_step(p, NULL);
        cp_pipeline_step(p, NULL);
        cp_pipeline_step(p, &snap); /* cycle 4: flush */
        ISO_CHECK(snap.flushing);
        ISO_CHECK_EQ_INT(cp_pipeline_pc(p), 24); /* 20 + 4 */
        ISO_CHECK_EQ_INT(cp_pipeline_stats(p).flush_cycles, 1);
        cp_pipeline_free(p);
    }
    {
        /* default flush count */
        int64_t instrs[20];
        FetchCtx fc = {instrs, 20};
        FlagCtx hc = {0};
        CpPipeline *p;
        int i;
        fill_adds(instrs, 20);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_set_hazard_fn(p, haz_flush_default, &hc);
        for (i = 0; i < 5; i++) {
            cp_pipeline_step(p, NULL);
        }
        ISO_CHECK_EQ_INT(cp_pipeline_stats(p).flush_cycles, 1);
        cp_pipeline_free(p);
    }
    {
        /* flush count larger than pipeline: clamped, no crash */
        int64_t instrs[20];
        FetchCtx fc = {instrs, 20};
        FlagCtx hc = {0};
        CpPipeline *p;
        int i;
        fill_adds(instrs, 20);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_set_hazard_fn(p, haz_flush_big, &hc);
        for (i = 0; i < 10; i++) {
            cp_pipeline_step(p, NULL);
        }
        ISO_CHECK(cp_pipeline_stats(p).flush_cycles == 1);
        cp_pipeline_free(p);
    }
    {
        /* multiple stalls and flushes */
        int64_t instrs[50];
        FetchCtx fc = {instrs, 50};
        CountCtx hc = {0};
        CpPipeline *p;
        CpPipelineStats st;
        int i;
        fill_adds(instrs, 50);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_set_hazard_fn(p, haz_multi, &hc);
        for (i = 0; i < 20; i++) {
            cp_pipeline_step(p, NULL);
        }
        st = cp_pipeline_stats(p);
        ISO_CHECK_EQ_INT(st.stall_cycles, 2);
        ISO_CHECK_EQ_INT(st.flush_cycles, 1);
        cp_pipeline_free(p);
    }

    {
        /* SIZE_MAX flush_count must clamp (regression: no int-truncation OOB) */
        int64_t instrs[20];
        FetchCtx fc = {instrs, 20};
        FlagCtx hc = {0};
        CpPipeline *p;
        int i;
        fill_adds(instrs, 20);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_set_hazard_fn(p, haz_flush_huge, &hc);
        for (i = 0; i < 10; i++) {
            cp_pipeline_step(p, NULL);
        }
        ISO_CHECK_EQ_INT(cp_pipeline_stats(p).flush_cycles, 1);
        cp_pipeline_free(p);
    }
    {
        /* SIZE_MAX stall_stages must clamp */
        int64_t instrs[20];
        FetchCtx fc = {instrs, 20};
        CountCtx hc = {0};
        CpPipeline *p;
        int i;
        fill_adds(instrs, 20);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_set_hazard_fn(p, haz_stall_huge, &hc);
        for (i = 0; i < 10; i++) {
            cp_pipeline_step(p, NULL);
        }
        ISO_CHECK(cp_pipeline_stats(p).stall_cycles >= 1);
        cp_pipeline_free(p);
    }

    /* ══ Forwarding ═══════════════════════════════════════════════════════ */
    {
        int64_t instrs[10];
        FetchCtx fc = {instrs, 10};
        CountCtx hc = {0};
        CpPipeline *p;
        const CpToken *ex;
        int i;
        fill_adds(instrs, 10);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_set_hazard_fn(p, haz_forward_ex_on_4, &hc);
        for (i = 0; i < 4; i++) {
            cp_pipeline_step(p, NULL);
        }
        ex = cp_pipeline_stage_contents(p, "EX");
        ISO_CHECK(ex != NULL);
        ISO_CHECK_STR_EQ(ex->forwarded_from, "EX");
        cp_pipeline_free(p);
    }
    {
        int64_t instrs[10];
        FetchCtx fc = {instrs, 10};
        CountCtx hc = {0};
        CpPipeline *p;
        const CpToken *ex;
        int i;
        fill_adds(instrs, 10);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_set_hazard_fn(p, haz_forward_mem_on_4, &hc);
        for (i = 0; i < 4; i++) {
            cp_pipeline_step(p, NULL);
        }
        ex = cp_pipeline_stage_contents(p, "EX");
        ISO_CHECK(ex != NULL);
        ISO_CHECK_STR_EQ(ex->forwarded_from, "MEM");
        cp_pipeline_free(p);
    }

    /* ══ Snapshot / trace ═════════════════════════════════════════════════ */
    {
        int64_t instrs[3];
        FetchCtx fc = {instrs, 3};
        CpPipeline *p;
        CpSnapshot snap1, snap2;
        const CpToken *tok;
        instrs[0] = make_instruction(OP_ADD, 1, 2, 3);
        instrs[1] = make_instruction(OP_ADD, 4, 5, 6);
        instrs[2] = make_instruction(OP_NOP, 0, 0, 0);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_step(p, &snap1);
        ISO_CHECK_EQ_INT(snap1.cycle, 1);
        tok = cp_snapshot_stage(&snap1, "IF");
        ISO_CHECK(tok != NULL && tok->pc == 0);
        cp_pipeline_step(p, &snap2);
        ISO_CHECK_EQ_INT(snap2.cycle, 2);
        tok = cp_snapshot_stage(&snap2, "ID");
        ISO_CHECK(tok != NULL && tok->pc == 0);
        cp_pipeline_free(p);
    }
    {
        int64_t instrs[10];
        FetchCtx fc = {instrs, 10};
        CpPipeline *p;
        CpSnapshot snap;
        size_t i;
        fill_adds(instrs, 10);
        p = new_test_pipeline(&fc, NULL);
        for (i = 0; i < 7; i++) {
            cp_pipeline_step(p, NULL);
        }
        ISO_CHECK_EQ_UINT(cp_pipeline_trace_count(p), 7);
        for (i = 0; i < 7; i++) {
            ISO_CHECK(cp_pipeline_trace(p, i, &snap));
            ISO_CHECK_EQ_INT(snap.cycle, (int64_t)(i + 1));
        }
        cp_pipeline_free(p);
    }
    {
        /* snapshot does not advance */
        int64_t instrs[1];
        FetchCtx fc = {instrs, 1};
        CpPipeline *p;
        CpSnapshot s1, s2;
        instrs[0] = make_instruction(OP_ADD, 1, 2, 3);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_step(p, NULL);
        cp_pipeline_snapshot(p, &s1);
        cp_pipeline_snapshot(p, &s2);
        ISO_CHECK_EQ_INT(s1.cycle, s2.cycle);
        cp_pipeline_free(p);
    }

    /* ══ Deep / custom / two-stage configs ════════════════════════════════ */
    {
        int64_t instrs[30];
        FetchCtx fc = {instrs, 30};
        CpPipelineConfig cfg;
        char err[128];
        CpPipeline *p;
        int i;
        fill_adds(instrs, 30);
        cp_config_deep_13_stage(&cfg);
        p = cp_pipeline_new(&cfg, simple_fetch, &fc, simple_decode, NULL,
                            simple_execute, NULL, simple_memory, NULL,
                            noop_writeback, NULL, err, sizeof err);
        ISO_CHECK(p != NULL);
        for (i = 0; i < 12; i++) {
            cp_pipeline_step(p, NULL);
        }
        ISO_CHECK_EQ_INT(cp_pipeline_stats(p).instructions_completed, 0);
        cp_pipeline_step(p, NULL);
        ISO_CHECK_EQ_INT(cp_pipeline_stats(p).instructions_completed, 1);
        cp_pipeline_free(p);
    }
    {
        /* custom 3-stage: first completion at cycle 3 */
        int64_t instrs[10];
        FetchCtx fc = {instrs, 10};
        CompletedCtx cc = {{0}, 0};
        CpPipelineConfig cfg;
        char err[128];
        CpPipeline *p;
        int i;
        fill_adds(instrs, 10);
        memset(&cfg, 0, sizeof cfg);
        set_stage_helper(&cfg, 0, "IF", CP_FETCH);
        set_stage_helper(&cfg, 1, "EX", CP_EXECUTE);
        set_stage_helper(&cfg, 2, "WB", CP_WRITEBACK);
        cfg.num_stages = 3;
        cfg.execution_width = 1;
        p = cp_pipeline_new(&cfg, simple_fetch, &fc, simple_decode, NULL,
                            simple_execute, NULL, simple_memory, NULL,
                            completed_writeback, &cc, err, sizeof err);
        ISO_CHECK(p != NULL);
        for (i = 0; i < 2; i++) {
            cp_pipeline_step(p, NULL);
        }
        ISO_CHECK_EQ_UINT(cc.n, 0);
        cp_pipeline_step(p, NULL);
        ISO_CHECK_EQ_UINT(cc.n, 1);
        cp_pipeline_free(p);
    }
    {
        /* two-stage: first completion at cycle 2 */
        int64_t instrs[10];
        FetchCtx fc = {instrs, 10};
        CompletedCtx cc = {{0}, 0};
        CpPipelineConfig cfg;
        char err[128];
        CpPipeline *p;
        fill_adds(instrs, 10);
        memset(&cfg, 0, sizeof cfg);
        set_stage_helper(&cfg, 0, "IF", CP_FETCH);
        set_stage_helper(&cfg, 1, "WB", CP_WRITEBACK);
        cfg.num_stages = 2;
        cfg.execution_width = 1;
        p = cp_pipeline_new(&cfg, simple_fetch, &fc, simple_decode, NULL,
                            simple_execute, NULL, simple_memory, NULL,
                            completed_writeback, &cc, err, sizeof err);
        cp_pipeline_step(p, NULL);
        ISO_CHECK_EQ_UINT(cc.n, 0);
        cp_pipeline_step(p, NULL);
        ISO_CHECK_EQ_UINT(cc.n, 1);
        cp_pipeline_free(p);
    }

    /* ══ Predict / set_pc / decode / halted / stage_contents ══════════════ */
    {
        int64_t instrs[100];
        FetchCtx fc = {instrs, 100};
        CpPipeline *p;
        fill_adds(instrs, 100);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_set_predict_fn(p, predict_plus8, NULL);
        cp_pipeline_step(p, NULL);
        ISO_CHECK_EQ_INT(cp_pipeline_pc(p), 8);
        cp_pipeline_step(p, NULL);
        ISO_CHECK_EQ_INT(cp_pipeline_pc(p), 16);
        cp_pipeline_free(p);
    }
    {
        int64_t instrs[10];
        FetchCtx fc = {instrs, 10};
        CpPipeline *p;
        fill_adds(instrs, 10);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_set_pc(p, 100);
        ISO_CHECK_EQ_INT(cp_pipeline_pc(p), 100);
        cp_pipeline_free(p);
    }
    {
        /* decode fills fields when LDR reaches ID */
        int64_t instrs[2];
        FetchCtx fc = {instrs, 2};
        CpPipeline *p;
        const CpToken *id;
        instrs[0] = make_instruction(OP_LDR, 5, 3, 0);
        instrs[1] = make_instruction(OP_NOP, 0, 0, 0);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_step(p, NULL);
        cp_pipeline_step(p, NULL);
        id = cp_pipeline_stage_contents(p, "ID");
        ISO_CHECK(id != NULL);
        ISO_CHECK_STR_EQ(id->opcode, "LDR");
        ISO_CHECK_EQ_INT(id->rd, 5);
        ISO_CHECK(id->mem_read);
        ISO_CHECK(id->reg_write);
        cp_pipeline_free(p);
    }
    {
        /* halted pipeline does not advance */
        int64_t instrs[5];
        FetchCtx fc = {instrs, 5};
        CpPipeline *p;
        int64_t cyc;
        int i;
        instrs[0] = make_instruction(OP_HALT, 0, 0, 0);
        for (i = 1; i < 5; i++) {
            instrs[i] = make_instruction(OP_NOP, 0, 0, 0);
        }
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_run(p, 100);
        cyc = cp_pipeline_cycle(p);
        cp_pipeline_step(p, NULL);
        cp_pipeline_step(p, NULL);
        ISO_CHECK_EQ_INT(cp_pipeline_cycle(p), cyc);
        cp_pipeline_free(p);
    }
    {
        /* run stops at max_cycles */
        int64_t instrs[100];
        FetchCtx fc = {instrs, 100};
        CpPipeline *p;
        CpPipelineStats st;
        fill_adds(instrs, 100);
        p = new_test_pipeline(&fc, NULL);
        st = cp_pipeline_run(p, 10);
        ISO_CHECK_EQ_INT(st.total_cycles, 10);
        ISO_CHECK(!cp_pipeline_is_halted(p));
        cp_pipeline_free(p);
    }
    {
        /* unknown stage name -> NULL */
        int64_t instrs[1];
        FetchCtx fc = {instrs, 1};
        CpPipeline *p;
        instrs[0] = make_instruction(OP_NOP, 0, 0, 0);
        p = new_test_pipeline(&fc, NULL);
        cp_pipeline_step(p, NULL);
        ISO_CHECK(cp_pipeline_stage_contents(p, "NONEXISTENT") == NULL);
        cp_pipeline_free(p);
    }
    {
        /* no hazard func: 0 stalls, 0 flushes */
        int64_t instrs[20];
        FetchCtx fc = {instrs, 20};
        CpPipeline *p;
        CpPipelineStats st;
        int i;
        fill_adds(instrs, 20);
        p = new_test_pipeline(&fc, NULL);
        for (i = 0; i < 10; i++) {
            cp_pipeline_step(p, NULL);
        }
        st = cp_pipeline_stats(p);
        ISO_CHECK_EQ_INT(st.stall_cycles, 0);
        ISO_CHECK_EQ_INT(st.flush_cycles, 0);
        cp_pipeline_free(p);
    }

    return ISO_TEST_RESULT();
}
