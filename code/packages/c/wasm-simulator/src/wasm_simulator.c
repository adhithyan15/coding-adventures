/*
 * wasm_simulator.c — implementation of the stack-based WASM virtual machine.
 * ===========================================================================
 *
 * The decoder inspects each opcode byte to know how many operand bytes follow;
 * the executor pops/pushes the operand stack and reads/writes locals, wrapping
 * arithmetic modulo 2^32 (WASM i32 semantics). Each executed instruction yields
 * a trace snapshotting the stack before/after, the locals, and a description.
 */
#include "wasm_simulator.h"

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ===========================================================================
 *  Small helpers
 * =========================================================================== */

/* Duplicate an int32 array (NULL for n==0). *ok reports OOM. */
static int32_t *dup_i32(const int32_t *src, size_t n, int *ok) {
    *ok = 1;
    if (n == 0) return NULL;
    if (n > ((size_t)-1) / sizeof(int32_t)) {
        *ok = 0;
        return NULL;
    }
    int32_t *out = malloc(n * sizeof(int32_t));
    if (!out) {
        *ok = 0;
        return NULL;
    }
    memcpy(out, src, n * sizeof(int32_t));
    return out;
}

/* Duplicate a NUL-terminated string. */
static char *dup_str(const char *s) {
    size_t n = strlen(s);
    char *out = malloc(n + 1);
    if (out) memcpy(out, s, n + 1);
    return out;
}

/* Grow a dynamic array to `needed` elements (elem bytes each). 1 ok, 0 fail. */
static int ensure_cap(void **data, size_t *cap, size_t needed, size_t elem) {
    if (needed <= *cap) return 1;
    size_t nc = *cap ? *cap : 8;
    while (nc < needed) {
        if (nc > ((size_t)-1) / 2 / elem) return 0;
        nc *= 2;
    }
    void *nd = realloc(*data, nc * elem);
    if (!nd) return 0;
    *data = nd;
    *cap = nc;
    return 1;
}

/* ===========================================================================
 *  Decoder
 * =========================================================================== */

WasmStatus wasm_decode(const uint8_t *bytecode, size_t len, size_t pc,
                       WasmInstruction *out) {
    if (pc >= len) return WASM_ERR_TRUNCATED;
    uint8_t opcode = bytecode[pc];
    out->opcode = opcode;
    switch (opcode) {
        case WASM_OP_I32_CONST: {
            if (pc + 4 >= len) return WASM_ERR_TRUNCATED; /* need pc+1..pc+4 */
            uint32_t v = (uint32_t)bytecode[pc + 1] |
                         ((uint32_t)bytecode[pc + 2] << 8) |
                         ((uint32_t)bytecode[pc + 3] << 16) |
                         ((uint32_t)bytecode[pc + 4] << 24);
            out->mnemonic = "i32.const";
            out->has_operand = 1;
            out->operand = (int32_t)v;
            out->size = 5;
            return WASM_OK;
        }
        case WASM_OP_I32_ADD:
            out->mnemonic = "i32.add";
            out->has_operand = 0;
            out->operand = 0;
            out->size = 1;
            return WASM_OK;
        case WASM_OP_I32_SUB:
            out->mnemonic = "i32.sub";
            out->has_operand = 0;
            out->operand = 0;
            out->size = 1;
            return WASM_OK;
        case WASM_OP_LOCAL_GET:
            if (pc + 1 >= len) return WASM_ERR_TRUNCATED;
            out->mnemonic = "local.get";
            out->has_operand = 1;
            out->operand = (int32_t)bytecode[pc + 1];
            out->size = 2;
            return WASM_OK;
        case WASM_OP_LOCAL_SET:
            if (pc + 1 >= len) return WASM_ERR_TRUNCATED;
            out->mnemonic = "local.set";
            out->has_operand = 1;
            out->operand = (int32_t)bytecode[pc + 1];
            out->size = 2;
            return WASM_OK;
        case WASM_OP_END:
            out->mnemonic = "end";
            out->has_operand = 0;
            out->operand = 0;
            out->size = 1;
            return WASM_OK;
        default:
            return WASM_ERR_UNKNOWN_OPCODE;
    }
}

/* ===========================================================================
 *  Trace
 * =========================================================================== */

void wasm_step_trace_free(WasmStepTrace *t) {
    if (!t) return;
    free(t->stack_before);
    free(t->stack_after);
    free(t->locals_snapshot);
    free(t->description);
    t->stack_before = NULL;
    t->stack_after = NULL;
    t->locals_snapshot = NULL;
    t->description = NULL;
}

/* ===========================================================================
 *  Simulator
 * =========================================================================== */

struct WasmSimulator {
    int32_t *stack;
    size_t stack_len, stack_cap;
    int32_t *locals;
    size_t num_locals;
    size_t pc;
    uint8_t *bytecode;
    size_t bytecode_len;
    int halted;
    size_t cycle;
};

WasmSimulator *wasm_sim_new(size_t num_locals) {
    WasmSimulator *s = calloc(1, sizeof *s);
    if (!s) return NULL;
    if (num_locals > 0) {
        s->locals = calloc(num_locals, sizeof(int32_t)); /* checked multiply */
        if (!s->locals) {
            free(s);
            return NULL;
        }
    }
    s->num_locals = num_locals;
    return s;
}

void wasm_sim_free(WasmSimulator *s) {
    if (!s) return;
    free(s->stack);
    free(s->locals);
    free(s->bytecode);
    free(s);
}

int wasm_sim_load(WasmSimulator *s, const uint8_t *bytecode, size_t len) {
    uint8_t *copy = NULL;
    if (len > 0) {
        copy = malloc(len);
        if (!copy) return -1;
        memcpy(copy, bytecode, len);
    }
    free(s->bytecode);
    s->bytecode = copy;
    s->bytecode_len = len;
    s->pc = 0;
    s->halted = 0;
    s->cycle = 0;
    s->stack_len = 0;
    for (size_t i = 0; i < s->num_locals; i++) s->locals[i] = 0;
    return 0;
}

static int stack_push(WasmSimulator *s, int32_t v) {
    if (!ensure_cap((void **)&s->stack, &s->stack_cap, s->stack_len + 1,
                    sizeof(int32_t)))
        return 0;
    s->stack[s->stack_len++] = v;
    return 1;
}

WasmStatus wasm_sim_step(WasmSimulator *s, WasmStepTrace *trace) {
    if (s->halted) return WASM_ERR_HALTED;

    WasmInstruction inst;
    WasmStatus st = wasm_decode(s->bytecode, s->bytecode_len, s->pc, &inst);
    if (st != WASM_OK) return st;

    /* Snapshot the stack before executing. */
    int ok;
    int32_t *stack_before = dup_i32(s->stack, s->stack_len, &ok);
    if (!ok) return WASM_ERR_NOMEM;
    size_t n_before = s->stack_len;

    char desc[96];
    int halted = 0;

    switch (inst.opcode) {
        case WASM_OP_I32_CONST:
            if (!stack_push(s, inst.operand)) goto oom_before;
            snprintf(desc, sizeof desc, "push %" PRId32, inst.operand);
            break;
        case WASM_OP_I32_ADD:
        case WASM_OP_I32_SUB: {
            if (s->stack_len < 2) {
                free(stack_before);
                return WASM_ERR_STACK_UNDERFLOW;
            }
            int32_t b = s->stack[--s->stack_len];
            int32_t a = s->stack[--s->stack_len];
            /* i32 arithmetic wraps modulo 2^32 (unsigned wrap, then reinterpret). */
            uint32_t res_u = (inst.opcode == WASM_OP_I32_ADD)
                                 ? (uint32_t)a + (uint32_t)b
                                 : (uint32_t)a - (uint32_t)b;
            int32_t res = (int32_t)res_u;
            if (!stack_push(s, res)) goto oom_before;
            snprintf(desc, sizeof desc,
                     "pop %" PRId32 " and %" PRId32 ", push %" PRId32, b, a,
                     res);
            break;
        }
        case WASM_OP_LOCAL_GET: {
            size_t idx = (size_t)(uint32_t)inst.operand;
            if (idx >= s->num_locals) {
                free(stack_before);
                return WASM_ERR_LOCAL_OUT_OF_RANGE;
            }
            int32_t val = s->locals[idx];
            if (!stack_push(s, val)) goto oom_before;
            snprintf(desc, sizeof desc, "push locals[%zu] = %" PRId32, idx, val);
            break;
        }
        case WASM_OP_LOCAL_SET: {
            size_t idx = (size_t)(uint32_t)inst.operand;
            if (idx >= s->num_locals) {
                free(stack_before);
                return WASM_ERR_LOCAL_OUT_OF_RANGE;
            }
            if (s->stack_len < 1) {
                free(stack_before);
                return WASM_ERR_STACK_UNDERFLOW;
            }
            int32_t val = s->stack[--s->stack_len];
            s->locals[idx] = val;
            snprintf(desc, sizeof desc, "pop %" PRId32 ", store in locals[%zu]",
                     val, idx);
            break;
        }
        case WASM_OP_END:
            halted = 1;
            snprintf(desc, sizeof desc, "halt");
            break;
        default:
            free(stack_before);
            return WASM_ERR_UNKNOWN_OPCODE; /* unreachable (decode guards) */
    }

    /* Snapshot the stack after, and the locals. */
    int32_t *stack_after = dup_i32(s->stack, s->stack_len, &ok);
    if (!ok) goto oom_before;
    int32_t *locals_snapshot = dup_i32(s->locals, s->num_locals, &ok);
    if (!ok) {
        free(stack_before);
        free(stack_after);
        return WASM_ERR_NOMEM;
    }
    char *description = dup_str(desc);
    if (!description) {
        free(stack_before);
        free(stack_after);
        free(locals_snapshot);
        return WASM_ERR_NOMEM;
    }

    trace->pc = s->pc;
    trace->instruction = inst;
    trace->stack_before = stack_before;
    trace->n_stack_before = n_before;
    trace->stack_after = stack_after;
    trace->n_stack_after = s->stack_len;
    trace->locals_snapshot = locals_snapshot;
    trace->n_locals = s->num_locals;
    trace->description = description;
    trace->halted = halted;

    s->pc += inst.size;
    s->halted = halted;
    s->cycle++;
    return WASM_OK;

oom_before:
    free(stack_before);
    return WASM_ERR_NOMEM;
}

WasmStatus wasm_sim_run(WasmSimulator *s, const uint8_t *program, size_t len,
                        size_t max_steps, WasmStepTrace **traces_out,
                        size_t *count_out) {
    *traces_out = NULL;
    *count_out = 0;
    if (wasm_sim_load(s, program, len) != 0) return WASM_ERR_NOMEM;

    WasmStepTrace *arr = NULL;
    size_t n = 0, cap = 0;
    for (size_t i = 0; i < max_steps; i++) {
        if (s->halted) break;
        WasmStepTrace t;
        WasmStatus st = wasm_sim_step(s, &t);
        if (st != WASM_OK) {
            wasm_traces_free(arr, n);
            return st;
        }
        if (!ensure_cap((void **)&arr, &cap, n + 1, sizeof(WasmStepTrace))) {
            wasm_step_trace_free(&t);
            wasm_traces_free(arr, n);
            return WASM_ERR_NOMEM;
        }
        arr[n++] = t;
    }
    *traces_out = arr;
    *count_out = n;
    return WASM_OK;
}

void wasm_traces_free(WasmStepTrace *traces, size_t count) {
    if (!traces) return;
    for (size_t i = 0; i < count; i++) wasm_step_trace_free(&traces[i]);
    free(traces);
}

const int32_t *wasm_sim_stack(const WasmSimulator *s, size_t *count_out) {
    *count_out = s->stack_len;
    return s->stack;
}
const int32_t *wasm_sim_locals(const WasmSimulator *s, size_t *count_out) {
    *count_out = s->num_locals;
    return s->locals;
}
size_t wasm_sim_pc(const WasmSimulator *s) { return s->pc; }
int wasm_sim_halted(const WasmSimulator *s) { return s->halted; }
size_t wasm_sim_cycle(const WasmSimulator *s) { return s->cycle; }

/* ===========================================================================
 *  Bytecode assembler
 * =========================================================================== */

void wasm_program_init(WasmProgram *p) {
    p->data = NULL;
    p->len = 0;
    p->cap = 0;
}

void wasm_program_free(WasmProgram *p) {
    if (!p) return;
    free(p->data);
    p->data = NULL;
    p->len = 0;
    p->cap = 0;
}

const uint8_t *wasm_program_bytes(const WasmProgram *p, size_t *len_out) {
    *len_out = p->len;
    return p->data;
}

static int prog_push(WasmProgram *p, uint8_t b) {
    if (!ensure_cap((void **)&p->data, &p->cap, p->len + 1, sizeof(uint8_t)))
        return -1;
    p->data[p->len++] = b;
    return 0;
}

int wasm_emit_i32_const(WasmProgram *p, int32_t val) {
    uint32_t v = (uint32_t)val;
    if (prog_push(p, WASM_OP_I32_CONST) != 0) return -1;
    if (prog_push(p, (uint8_t)(v)) != 0) return -1;
    if (prog_push(p, (uint8_t)(v >> 8)) != 0) return -1;
    if (prog_push(p, (uint8_t)(v >> 16)) != 0) return -1;
    if (prog_push(p, (uint8_t)(v >> 24)) != 0) return -1;
    return 0;
}
int wasm_emit_i32_add(WasmProgram *p) { return prog_push(p, WASM_OP_I32_ADD); }
int wasm_emit_i32_sub(WasmProgram *p) { return prog_push(p, WASM_OP_I32_SUB); }
int wasm_emit_local_get(WasmProgram *p, uint8_t idx) {
    if (prog_push(p, WASM_OP_LOCAL_GET) != 0) return -1;
    return prog_push(p, idx);
}
int wasm_emit_local_set(WasmProgram *p, uint8_t idx) {
    if (prog_push(p, WASM_OP_LOCAL_SET) != 0) return -1;
    return prog_push(p, idx);
}
int wasm_emit_end(WasmProgram *p) { return prog_push(p, WASM_OP_END); }
