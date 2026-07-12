/*
 * jvm_simulator.c — implementation of the typed stack-based JVM VM.
 * ===========================================================================
 *
 * The dispatch mirrors the Rust crate: the compact iconst_N / iload_N / istore_N
 * opcode ranges are handled first, then the remaining opcodes by value. Locals
 * are `JvmLocal` slots that may be uninitialized; arithmetic wraps modulo 2^32;
 * branches read a 16-bit signed offset. Each executed instruction yields a trace
 * snapshotting the stack before/after, the locals, and a description.
 */
#include "jvm_simulator.h"

#include <inttypes.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ===========================================================================
 *  Small helpers
 * =========================================================================== */

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

static JvmLocal *dup_locals(const JvmLocal *src, size_t n, int *ok) {
    *ok = 1;
    if (n == 0) return NULL;
    if (n > ((size_t)-1) / sizeof(JvmLocal)) {
        *ok = 0;
        return NULL;
    }
    JvmLocal *out = malloc(n * sizeof(JvmLocal));
    if (!out) {
        *ok = 0;
        return NULL;
    }
    memcpy(out, src, n * sizeof(JvmLocal));
    return out;
}

static char *dup_str(const char *s) {
    size_t n = strlen(s);
    char *out = malloc(n + 1);
    if (out) memcpy(out, s, n + 1);
    return out;
}

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
 *  Trace
 * =========================================================================== */

void jvm_trace_free(JvmTrace *t) {
    if (!t) return;
    free(t->opcode);
    free(t->stack_before);
    free(t->stack_after);
    free(t->locals_snapshot);
    free(t->description);
    t->opcode = NULL;
    t->stack_before = NULL;
    t->stack_after = NULL;
    t->locals_snapshot = NULL;
    t->description = NULL;
}

/* ===========================================================================
 *  Simulator
 * =========================================================================== */

struct JvmSimulator {
    int32_t *stack;
    size_t stack_len, stack_cap;
    JvmLocal *locals;
    size_t num_locals;
    int32_t *constants;
    size_t num_constants;
    size_t pc;
    int halted;
    int has_return;
    int32_t return_value;
    uint8_t *bytecode;
    size_t bytecode_len;
};

JvmSimulator *jvm_sim_new(void) {
    JvmSimulator *s = calloc(1, sizeof *s);
    if (!s) return NULL;
    s->locals = calloc(16, sizeof(JvmLocal)); /* default 16 locals, all zeroed */
    if (!s->locals) {
        free(s);
        return NULL;
    }
    s->num_locals = 16;
    return s;
}

void jvm_sim_free(JvmSimulator *s) {
    if (!s) return;
    free(s->stack);
    free(s->locals);
    free(s->constants);
    free(s->bytecode);
    free(s);
}

int jvm_sim_load(JvmSimulator *s, const uint8_t *bytecode, size_t blen,
                 const int32_t *constants, size_t nconstants,
                 size_t num_locals) {
    uint8_t *bc = NULL;
    if (blen > 0) {
        bc = malloc(blen);
        if (!bc) return -1;
        memcpy(bc, bytecode, blen);
    }
    int32_t *cs = NULL;
    if (nconstants > 0) {
        if (nconstants > ((size_t)-1) / sizeof(int32_t)) {
            free(bc);
            return -1;
        }
        cs = malloc(nconstants * sizeof(int32_t));
        if (!cs) {
            free(bc);
            return -1;
        }
        memcpy(cs, constants, nconstants * sizeof(int32_t));
    }
    JvmLocal *locals = NULL;
    if (num_locals > 0) {
        locals = calloc(num_locals, sizeof(JvmLocal)); /* uninitialized slots */
        if (!locals) {
            free(bc);
            free(cs);
            return -1;
        }
    }

    free(s->bytecode);
    free(s->constants);
    free(s->locals);
    s->bytecode = bc;
    s->bytecode_len = blen;
    s->constants = cs;
    s->num_constants = nconstants;
    s->locals = locals;
    s->num_locals = num_locals;
    s->stack_len = 0;
    s->pc = 0;
    s->halted = 0;
    s->has_return = 0;
    s->return_value = 0;
    return 0;
}

static int stack_push(JvmSimulator *s, int32_t v) {
    if (!ensure_cap((void **)&s->stack, &s->stack_cap, s->stack_len + 1,
                    sizeof(int32_t)))
        return 0;
    s->stack[s->stack_len++] = v;
    return 1;
}

/* ---- shared instruction bodies (mutate state, fill desc, return status) --- */

static JvmStatus do_iload(JvmSimulator *s, size_t slot, size_t size, char *desc,
                          size_t desc_sz) {
    if (slot >= s->num_locals) return JVM_ERR_LOCAL_OUT_OF_RANGE;
    if (!s->locals[slot].initialized) return JVM_ERR_LOCAL_UNINITIALIZED;
    int32_t val = s->locals[slot].value;
    if (!stack_push(s, val)) return JVM_ERR_NOMEM;
    s->pc += size;
    snprintf(desc, desc_sz, "push locals[%zu] = %" PRId32, slot, val);
    return JVM_OK;
}

static JvmStatus do_istore(JvmSimulator *s, size_t slot, size_t size,
                           char *desc, size_t desc_sz) {
    if (slot >= s->num_locals) return JVM_ERR_LOCAL_OUT_OF_RANGE;
    if (s->stack_len < 1) return JVM_ERR_STACK_UNDERFLOW;
    int32_t val = s->stack[--s->stack_len];
    s->locals[slot].value = val;
    s->locals[slot].initialized = 1;
    s->pc += size;
    snprintf(desc, desc_sz, "pop %" PRId32 ", store in locals[%zu]", val, slot);
    return JVM_OK;
}

static JvmStatus do_binary(JvmSimulator *s, uint8_t op, char *desc,
                           size_t desc_sz) {
    if (s->stack_len < 2) return JVM_ERR_STACK_UNDERFLOW;
    int32_t b = s->stack[s->stack_len - 1]; /* divisor for idiv */
    int32_t a = s->stack[s->stack_len - 2];
    if (op == JVM_OP_IDIV && b == 0) return JVM_ERR_DIV_BY_ZERO;
    s->stack_len -= 2;
    int32_t result;
    switch (op) {
        case JVM_OP_IADD: result = (int32_t)((uint32_t)a + (uint32_t)b); break;
        case JVM_OP_ISUB: result = (int32_t)((uint32_t)a - (uint32_t)b); break;
        case JVM_OP_IMUL: result = (int32_t)((uint32_t)a * (uint32_t)b); break;
        case JVM_OP_IDIV:
            /* wrapping_div: the one overflow case is INT32_MIN / -1. */
            result = (a == INT32_MIN && b == -1) ? INT32_MIN : (a / b);
            break;
        default: result = 0; break;
    }
    (void)stack_push(s, result); /* room guaranteed (popped 2, push 1) */
    s->pc += 1;
    snprintf(desc, desc_sz, "pop %" PRId32 " and %" PRId32 ", push %" PRId32, b,
             a, result);
    return JVM_OK;
}

static JvmStatus do_if_icmp(JvmSimulator *s, size_t pc, int is_eq, char *desc,
                            size_t desc_sz) {
    if (pc + 2 >= s->bytecode_len) return JVM_ERR_TRUNCATED;
    if (s->stack_len < 2) return JVM_ERR_STACK_UNDERFLOW;
    uint16_t raw = ((uint16_t)s->bytecode[pc + 1] << 8) | s->bytecode[pc + 2];
    int32_t offset = (raw >= 0x8000) ? (int32_t)raw - 0x10000 : (int32_t)raw;
    int32_t b = s->stack[--s->stack_len];
    int32_t a = s->stack[--s->stack_len];
    int taken = is_eq ? (a == b) : (a > b);
    if (taken) {
        size_t target = (size_t)((int64_t)pc + offset);
        s->pc = target;
        snprintf(desc, desc_sz,
                 "pop %" PRId32 " and %" PRId32 ", true, jump to PC=%zu", b, a,
                 target);
    } else {
        s->pc = pc + 3;
        snprintf(desc, desc_sz,
                 "pop %" PRId32 " and %" PRId32 ", false, fall through", b, a);
    }
    return JVM_OK;
}

JvmStatus jvm_sim_step(JvmSimulator *s, JvmTrace *trace) {
    if (s->halted) return JVM_ERR_HALTED;
    if (s->pc >= s->bytecode_len) return JVM_ERR_PC_OUT_OF_BOUNDS;

    size_t pc = s->pc;
    uint8_t op = s->bytecode[pc];

    int ok;
    int32_t *stack_before = dup_i32(s->stack, s->stack_len, &ok);
    if (!ok) return JVM_ERR_NOMEM;
    size_t n_before = s->stack_len;

    char opstr[24] = {0};
    char desc[96] = {0};
    JvmStatus st = JVM_OK;

    if (op >= JVM_OP_ICONST_0 && op <= JVM_OP_ICONST_5) {
        int32_t val = (int32_t)(op - JVM_OP_ICONST_0);
        if (!stack_push(s, val)) {
            st = JVM_ERR_NOMEM;
        } else {
            s->pc += 1;
            snprintf(opstr, sizeof opstr, "iconst_%" PRId32, val);
            snprintf(desc, sizeof desc, "push %" PRId32, val);
        }
    } else if (op >= JVM_OP_ILOAD_0 && op <= JVM_OP_ILOAD_3) {
        size_t slot = (size_t)(op - JVM_OP_ILOAD_0);
        snprintf(opstr, sizeof opstr, "iload_%zu", slot);
        st = do_iload(s, slot, 1, desc, sizeof desc);
    } else if (op >= JVM_OP_ISTORE_0 && op <= JVM_OP_ISTORE_3) {
        size_t slot = (size_t)(op - JVM_OP_ISTORE_0);
        snprintf(opstr, sizeof opstr, "istore_%zu", slot);
        st = do_istore(s, slot, 1, desc, sizeof desc);
    } else {
        switch (op) {
            case JVM_OP_BIPUSH:
                if (pc + 1 >= s->bytecode_len) {
                    st = JVM_ERR_TRUNCATED;
                } else {
                    int32_t val = (int32_t)(int8_t)s->bytecode[pc + 1];
                    if (!stack_push(s, val)) {
                        st = JVM_ERR_NOMEM;
                    } else {
                        s->pc += 2;
                        snprintf(opstr, sizeof opstr, "bipush");
                        snprintf(desc, sizeof desc, "push %" PRId32, val);
                    }
                }
                break;
            case JVM_OP_LDC:
                if (pc + 1 >= s->bytecode_len) {
                    st = JVM_ERR_TRUNCATED;
                } else {
                    size_t idx = (size_t)s->bytecode[pc + 1];
                    if (idx >= s->num_constants) {
                        st = JVM_ERR_CONST_OUT_OF_RANGE;
                    } else {
                        int32_t val = s->constants[idx];
                        if (!stack_push(s, val)) {
                            st = JVM_ERR_NOMEM;
                        } else {
                            s->pc += 2;
                            snprintf(opstr, sizeof opstr, "ldc");
                            snprintf(desc, sizeof desc,
                                     "push constant[%zu] = %" PRId32, idx, val);
                        }
                    }
                }
                break;
            case JVM_OP_ILOAD:
                if (pc + 1 >= s->bytecode_len) {
                    st = JVM_ERR_TRUNCATED;
                } else {
                    snprintf(opstr, sizeof opstr, "iload");
                    st = do_iload(s, (size_t)s->bytecode[pc + 1], 2, desc,
                                  sizeof desc);
                }
                break;
            case JVM_OP_ISTORE:
                if (pc + 1 >= s->bytecode_len) {
                    st = JVM_ERR_TRUNCATED;
                } else {
                    snprintf(opstr, sizeof opstr, "istore");
                    st = do_istore(s, (size_t)s->bytecode[pc + 1], 2, desc,
                                   sizeof desc);
                }
                break;
            case JVM_OP_IADD:
                snprintf(opstr, sizeof opstr, "iadd");
                st = do_binary(s, op, desc, sizeof desc);
                break;
            case JVM_OP_ISUB:
                snprintf(opstr, sizeof opstr, "isub");
                st = do_binary(s, op, desc, sizeof desc);
                break;
            case JVM_OP_IMUL:
                snprintf(opstr, sizeof opstr, "imul");
                st = do_binary(s, op, desc, sizeof desc);
                break;
            case JVM_OP_IDIV:
                snprintf(opstr, sizeof opstr, "idiv");
                st = do_binary(s, op, desc, sizeof desc);
                break;
            case JVM_OP_GOTO:
                if (pc + 2 >= s->bytecode_len) {
                    st = JVM_ERR_TRUNCATED;
                } else {
                    uint16_t raw = ((uint16_t)s->bytecode[pc + 1] << 8) |
                                   s->bytecode[pc + 2];
                    int32_t offset =
                        (raw >= 0x8000) ? (int32_t)raw - 0x10000 : (int32_t)raw;
                    size_t target = (size_t)((int64_t)pc + offset);
                    s->pc = target;
                    snprintf(opstr, sizeof opstr, "goto");
                    snprintf(desc, sizeof desc, "jump to PC=%zu", target);
                }
                break;
            case JVM_OP_IF_ICMPEQ:
                snprintf(opstr, sizeof opstr, "if_icmpeq");
                st = do_if_icmp(s, pc, 1, desc, sizeof desc);
                break;
            case JVM_OP_IF_ICMPGT:
                snprintf(opstr, sizeof opstr, "if_icmpgt");
                st = do_if_icmp(s, pc, 0, desc, sizeof desc);
                break;
            case JVM_OP_IRETURN:
                if (s->stack_len < 1) {
                    st = JVM_ERR_STACK_UNDERFLOW;
                } else {
                    int32_t val = s->stack[--s->stack_len];
                    s->return_value = val;
                    s->has_return = 1;
                    s->halted = 1;
                    s->pc += 1;
                    snprintf(opstr, sizeof opstr, "ireturn");
                    snprintf(desc, sizeof desc, "return %" PRId32, val);
                }
                break;
            case JVM_OP_RETURN:
                s->halted = 1;
                s->pc += 1;
                snprintf(opstr, sizeof opstr, "return");
                snprintf(desc, sizeof desc, "return void");
                break;
            default:
                st = JVM_ERR_UNKNOWN_OPCODE;
                break;
        }
    }

    if (st != JVM_OK) {
        free(stack_before);
        return st;
    }

    int32_t *stack_after = dup_i32(s->stack, s->stack_len, &ok);
    if (!ok) {
        free(stack_before);
        return JVM_ERR_NOMEM;
    }
    JvmLocal *locals_snapshot = dup_locals(s->locals, s->num_locals, &ok);
    if (!ok) {
        free(stack_before);
        free(stack_after);
        return JVM_ERR_NOMEM;
    }
    char *opcode = dup_str(opstr);
    char *description = dup_str(desc);
    if (!opcode || !description) {
        free(stack_before);
        free(stack_after);
        free(locals_snapshot);
        free(opcode);
        free(description);
        return JVM_ERR_NOMEM;
    }

    trace->pc = pc;
    trace->opcode = opcode;
    trace->stack_before = stack_before;
    trace->n_stack_before = n_before;
    trace->stack_after = stack_after;
    trace->n_stack_after = s->stack_len;
    trace->locals_snapshot = locals_snapshot;
    trace->n_locals = s->num_locals;
    trace->description = description;
    return JVM_OK;
}

JvmStatus jvm_sim_run(JvmSimulator *s, size_t max_steps, JvmTrace **traces_out,
                      size_t *count_out) {
    *traces_out = NULL;
    *count_out = 0;
    JvmTrace *arr = NULL;
    size_t n = 0, cap = 0;
    for (size_t i = 0; i < max_steps; i++) {
        if (s->halted) break;
        JvmTrace t;
        JvmStatus st = jvm_sim_step(s, &t);
        if (st != JVM_OK) {
            jvm_traces_free(arr, n);
            return st;
        }
        if (!ensure_cap((void **)&arr, &cap, n + 1, sizeof(JvmTrace))) {
            jvm_trace_free(&t);
            jvm_traces_free(arr, n);
            return JVM_ERR_NOMEM;
        }
        arr[n++] = t;
    }
    *traces_out = arr;
    *count_out = n;
    return JVM_OK;
}

void jvm_traces_free(JvmTrace *traces, size_t count) {
    if (!traces) return;
    for (size_t i = 0; i < count; i++) jvm_trace_free(&traces[i]);
    free(traces);
}

const int32_t *jvm_sim_stack(const JvmSimulator *s, size_t *count_out) {
    *count_out = s->stack_len;
    return s->stack;
}
const JvmLocal *jvm_sim_locals(const JvmSimulator *s, size_t *count_out) {
    *count_out = s->num_locals;
    return s->locals;
}
size_t jvm_sim_pc(const JvmSimulator *s) { return s->pc; }
int jvm_sim_halted(const JvmSimulator *s) { return s->halted; }
int jvm_sim_return_value(const JvmSimulator *s, int32_t *out) {
    if (!s->has_return) return 0;
    *out = s->return_value;
    return 1;
}

/* ===========================================================================
 *  Bytecode assembler
 * =========================================================================== */

void jvm_program_init(JvmProgram *p) {
    p->data = NULL;
    p->len = 0;
    p->cap = 0;
}
void jvm_program_free(JvmProgram *p) {
    if (!p) return;
    free(p->data);
    p->data = NULL;
    p->len = 0;
    p->cap = 0;
}
const uint8_t *jvm_program_bytes(const JvmProgram *p, size_t *len_out) {
    *len_out = p->len;
    return p->data;
}

static int prog_push(JvmProgram *p, uint8_t b) {
    if (!ensure_cap((void **)&p->data, &p->cap, p->len + 1, sizeof(uint8_t)))
        return -1;
    p->data[p->len++] = b;
    return 0;
}

int jvm_emit(JvmProgram *p, uint8_t opcode, int32_t operand) {
    if (prog_push(p, opcode) != 0) return -1;
    switch (opcode) {
        case JVM_OP_BIPUSH:
        case JVM_OP_ILOAD:
        case JVM_OP_ISTORE:
        case JVM_OP_LDC:
            return prog_push(p, (uint8_t)operand);
        case JVM_OP_GOTO:
        case JVM_OP_IF_ICMPEQ:
        case JVM_OP_IF_ICMPGT: {
            uint16_t off = (uint16_t)operand;
            if (prog_push(p, (uint8_t)(off >> 8)) != 0) return -1;
            return prog_push(p, (uint8_t)(off & 0xFF));
        }
        default:
            return 0; /* operand-less opcode */
    }
}
