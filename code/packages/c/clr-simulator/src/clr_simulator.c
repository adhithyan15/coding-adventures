/*
 * clr_simulator.c — the CLR bytecode simulator, pure ISO C17.
 * =========================================================
 *
 * A faithful port of the Rust `clr-simulator` crate. The interesting design
 * decision in this port is BOUNDS SAFETY: the Rust original indexes slices
 * (`bytecode[pc + 1]`), which panics on out-of-range access — safe, because
 * Rust checks every index. C does not, so we treat the bytecode as UNTRUSTED
 * input and check every operand read and every heap/array index explicitly,
 * returning a `ClrStatus` where the Rust code would have panicked.
 *
 * Arithmetic wraps modulo 2^32 exactly like Rust's `wrapping_*`, done through
 * `uint32_t` so we never trip signed-overflow UB.
 */
#include "clr_simulator.h"

#include <limits.h> /* INT32_MIN via stdint */
#include <stdint.h>
#include <stdlib.h> /* malloc, calloc, realloc, free */
#include <string.h> /* memcpy, memset */

/* ── Growable slot vector (stack, locals, args) ────────────────────────────*/

typedef struct {
    ClrSlot *data;
    size_t len;
    size_t cap;
} SlotVec;

/* An `object[]` on the heap. */
typedef struct {
    ClrValue *data;
    size_t len;
} HeapArray;

typedef struct {
    HeapArray *data;
    size_t len;
    size_t cap;
} HeapVec;

/* A saved caller frame (operand stack + heap are shared; only per-method
 * registers are saved). */
typedef struct {
    size_t return_pc;
    size_t return_method;
    uint8_t *return_bytecode; /* owned copy of the caller's body */
    size_t return_bytecode_len;
    SlotVec return_locals;
    SlotVec return_args;
} Frame;

typedef struct {
    Frame *data;
    size_t len;
    size_t cap;
} FrameVec;

/* A loaded method. `body` is an owned copy. */
typedef struct {
    uint8_t *body;
    size_t body_len;
    size_t num_locals;
    size_t num_args;
} Method;

typedef struct {
    Method *data;
    size_t len;
    size_t cap;
} MethodVec;

struct ClrSimulator {
    SlotVec stack;
    SlotVec locals;
    SlotVec args;
    HeapVec heap;
    uint8_t *bytecode; /* owned copy of the CURRENT method's body */
    size_t bytecode_len;
    size_t pc;
    int halted;
    MethodVec methods;
    size_t cur_method;
    FrameVec frames;
    int oom; /* sticky: an allocation failed */
};

/* ── Growable-vector helpers (all guard size_t overflow) ───────────────────*/

/* Grow SlotVec `v` to hold at least `need` items. Returns 0 on OOM. */
static int slotvec_reserve(SlotVec *v, size_t need) {
    size_t nc;
    ClrSlot *nd;
    if (need <= v->cap) {
        return 1;
    }
    nc = v->cap ? v->cap : 4;
    while (nc < need) {
        if (nc > (size_t)-1 / 2) { /* doubling would overflow */
            nc = need;
            break;
        }
        nc *= 2;
    }
    if (nc > (size_t)-1 / sizeof(ClrSlot)) {
        return 0;
    }
    nd = (ClrSlot *)realloc(v->data, nc * sizeof(ClrSlot));
    if (!nd) {
        return 0;
    }
    v->data = nd;
    v->cap = nc;
    return 1;
}

static int slotvec_push(SlotVec *v, ClrSlot s) {
    if (!slotvec_reserve(v, v->len + 1)) {
        return 0;
    }
    v->data[v->len++] = s;
    return 1;
}

static void slotvec_free(SlotVec *v) {
    free(v->data);
    v->data = NULL;
    v->len = v->cap = 0;
}

/* A None (unset) slot with a zero-initialized value (keeps UBSan quiet). */
static ClrSlot slot_none(void) {
    ClrSlot none;
    none.present = 0;
    memset(&none.value, 0, sizeof none.value);
    return none;
}

/* Set slot `idx` (0-based) to `s`, growing with None-filled slots as needed. */
static int slotvec_set(SlotVec *v, size_t idx, ClrSlot s) {
    if (idx >= v->len) {
        if (!slotvec_reserve(v, idx + 1)) {
            return 0;
        }
        while (v->len <= idx) {
            v->data[v->len++] = slot_none();
        }
    }
    v->data[idx] = s;
    return 1;
}

static int heapvec_reserve(HeapVec *v, size_t need) {
    size_t nc;
    HeapArray *nd;
    if (need <= v->cap) {
        return 1;
    }
    nc = v->cap ? v->cap : 4;
    while (nc < need) {
        if (nc > (size_t)-1 / 2) {
            nc = need;
            break;
        }
        nc *= 2;
    }
    if (nc > (size_t)-1 / sizeof(HeapArray)) {
        return 0;
    }
    nd = (HeapArray *)realloc(v->data, nc * sizeof(HeapArray));
    if (!nd) {
        return 0;
    }
    v->data = nd;
    v->cap = nc;
    return 1;
}

static int framevec_reserve(FrameVec *v, size_t need) {
    size_t nc;
    Frame *nd;
    if (need <= v->cap) {
        return 1;
    }
    nc = v->cap ? v->cap : 4;
    while (nc < need) {
        if (nc > (size_t)-1 / 2) {
            nc = need;
            break;
        }
        nc *= 2;
    }
    if (nc > (size_t)-1 / sizeof(Frame)) {
        return 0;
    }
    nd = (Frame *)realloc(v->data, nc * sizeof(Frame));
    if (!nd) {
        return 0;
    }
    v->data = nd;
    v->cap = nc;
    return 1;
}

static int methodvec_reserve(MethodVec *v, size_t need) {
    size_t nc;
    Method *nd;
    if (need <= v->cap) {
        return 1;
    }
    nc = v->cap ? v->cap : 4;
    while (nc < need) {
        if (nc > (size_t)-1 / 2) {
            nc = need;
            break;
        }
        nc *= 2;
    }
    if (nc > (size_t)-1 / sizeof(Method)) {
        return 0;
    }
    nd = (Method *)realloc(v->data, nc * sizeof(Method));
    if (!nd) {
        return 0;
    }
    v->data = nd;
    v->cap = nc;
    return 1;
}

/* ── Value constructors ────────────────────────────────────────────────────*/

static ClrValue value_int(int32_t i) {
    ClrValue v;
    v.kind = CLR_INT;
    v.i = i;
    v.ref_some = 0;
    v.ref_idx = 0;
    return v;
}

static ClrValue value_null(void) {
    ClrValue v;
    v.kind = CLR_REF;
    v.i = 0;
    v.ref_some = 0;
    v.ref_idx = 0;
    return v;
}

static ClrValue value_ref(size_t idx) {
    ClrValue v;
    v.kind = CLR_REF;
    v.i = 0;
    v.ref_some = 1;
    v.ref_idx = idx;
    return v;
}

static ClrSlot slot_some(ClrValue v) {
    ClrSlot s;
    s.present = 1;
    s.value = v;
    return s;
}

/* Value.as_int(): the integer, or an error if it is a reference. */
static ClrStatus value_as_int(ClrValue v, int32_t *out) {
    if (v.kind != CLR_INT) {
        return CLR_ERR_EXPECTED_INT;
    }
    *out = v.i;
    return CLR_OK;
}

/* Value.as_cmp_int(): int as-is; ref None -> 0, Some -> 1. Used by ceq/cgt/clt
 * where either a number or a reference is acceptable. */
static int32_t value_as_cmp_int(ClrValue v) {
    if (v.kind == CLR_INT) {
        return v.i;
    }
    return v.ref_some ? 1 : 0;
}

/* Value.is_truthy(): int != 0, or ref is Some. */
static int value_is_truthy(ClrValue v) {
    if (v.kind == CLR_INT) {
        return v.i != 0;
    }
    return v.ref_some;
}

/* ── Stack ops (checked) ───────────────────────────────────────────────────*/

/* Pop the top slot; error on empty stack or an unset (None) slot. */
static ClrStatus pop_value(ClrSimulator *sim, ClrValue *out) {
    ClrSlot s;
    if (sim->stack.len == 0) {
        return CLR_ERR_STACK_UNDERFLOW;
    }
    s = sim->stack.data[--sim->stack.len];
    if (!s.present) {
        return CLR_ERR_NULL_OPERAND;
    }
    *out = s.value;
    return CLR_OK;
}

static ClrStatus push_value(ClrSimulator *sim, ClrValue v) {
    if (!slotvec_push(&sim->stack, slot_some(v))) {
        sim->oom = 1;
        return CLR_ERR_OUT_OF_MEMORY;
    }
    return CLR_OK;
}

/* ── Bytecode operand reads (checked) ──────────────────────────────────────*/

/* Read the single byte at `pc + off`, or CLR_ERR_BYTECODE_OVERRUN. */
static ClrStatus read_u8(const ClrSimulator *sim, size_t off, uint8_t *out) {
    if (sim->pc >= sim->bytecode_len || off > sim->bytecode_len - 1 - sim->pc) {
        return CLR_ERR_BYTECODE_OVERRUN;
    }
    *out = sim->bytecode[sim->pc + off];
    return CLR_OK;
}

/* Ensure `n` bytes (opcode + operand) are available starting at pc. */
static ClrStatus need_bytes(const ClrSimulator *sim, size_t n) {
    if (sim->pc >= sim->bytecode_len || sim->bytecode_len - sim->pc < n) {
        return CLR_ERR_BYTECODE_OVERRUN;
    }
    return CLR_OK;
}

/* Read a 32-bit little-endian operand at pc+1..pc+5. */
static ClrStatus read_i32_operand(const ClrSimulator *sim, int32_t *out) {
    uint32_t u;
    ClrStatus st = need_bytes(sim, 5);
    if (st != CLR_OK) {
        return st;
    }
    u = (uint32_t)sim->bytecode[sim->pc + 1] |
        ((uint32_t)sim->bytecode[sim->pc + 2] << 8) |
        ((uint32_t)sim->bytecode[sim->pc + 3] << 16) |
        ((uint32_t)sim->bytecode[sim->pc + 4] << 24);
    *out = (int32_t)u;
    return CLR_OK;
}

/* ── Arithmetic (wrapping via uint32) ──────────────────────────────────────*/

static ClrStatus execute_binop(ClrSimulator *sim, uint8_t op) {
    ClrValue bv, av;
    int32_t a, b;
    uint32_t r;
    ClrStatus st;
    /* Rust pops b then a. */
    st = pop_value(sim, &bv);
    if (st != CLR_OK) {
        return st;
    }
    st = pop_value(sim, &av);
    if (st != CLR_OK) {
        return st;
    }
    st = value_as_int(bv, &b);
    if (st != CLR_OK) {
        return st;
    }
    st = value_as_int(av, &a);
    if (st != CLR_OK) {
        return st;
    }
    switch (op) {
        case CLR_OP_ADD:
            r = (uint32_t)a + (uint32_t)b;
            break;
        case CLR_OP_SUB:
            r = (uint32_t)a - (uint32_t)b;
            break;
        case CLR_OP_MUL:
            r = (uint32_t)a * (uint32_t)b;
            break;
        case CLR_OP_XOR:
            r = (uint32_t)a ^ (uint32_t)b;
            break;
        default:
            return CLR_ERR_UNKNOWN_OPCODE;
    }
    return push_value(sim, value_int((int32_t)r));
}

/* div: wrapping_div — INT_MIN / -1 wraps to INT_MIN instead of trapping. */
static ClrStatus execute_div(ClrSimulator *sim) {
    ClrValue bv, av;
    int32_t a, b, q;
    ClrStatus st;
    st = pop_value(sim, &bv);
    if (st != CLR_OK) {
        return st;
    }
    st = pop_value(sim, &av);
    if (st != CLR_OK) {
        return st;
    }
    st = value_as_int(bv, &b);
    if (st != CLR_OK) {
        return st;
    }
    st = value_as_int(av, &a);
    if (st != CLR_OK) {
        return st;
    }
    if (b == 0) {
        return CLR_ERR_DIVIDE_BY_ZERO;
    }
    if (a == INT32_MIN && b == -1) {
        q = INT32_MIN;
    } else {
        q = a / b;
    }
    return push_value(sim, value_int(q));
}

/* ── Comparison (FE-prefixed ceq/cgt/clt) ──────────────────────────────────*/

static ClrStatus execute_compare(ClrSimulator *sim, uint8_t sub) {
    ClrValue bv, av;
    int32_t a, b, r;
    ClrStatus st;
    st = pop_value(sim, &bv);
    if (st != CLR_OK) {
        return st;
    }
    st = pop_value(sim, &av);
    if (st != CLR_OK) {
        return st;
    }
    a = value_as_cmp_int(av);
    b = value_as_cmp_int(bv);
    switch (sub) {
        case CLR_CEQ_BYTE:
            r = (a == b) ? 1 : 0;
            break;
        case CLR_CGT_BYTE:
            r = (a > b) ? 1 : 0;
            break;
        case CLR_CLT_BYTE:
            r = (a < b) ? 1 : 0;
            break;
        default:
            return CLR_ERR_UNKNOWN_OPCODE;
    }
    return push_value(sim, value_int(r));
}

/* ── Constructor / destructor ──────────────────────────────────────────────*/

ClrSimulator *clr_new(void) {
    return (ClrSimulator *)calloc(1, sizeof(ClrSimulator));
}

static void method_free(Method *m) {
    free(m->body);
    m->body = NULL;
}

static void frame_free(Frame *f) {
    free(f->return_bytecode);
    f->return_bytecode = NULL;
    slotvec_free(&f->return_locals);
    slotvec_free(&f->return_args);
}

static void heap_free(HeapVec *h) {
    size_t i;
    for (i = 0; i < h->len; i++) {
        free(h->data[i].data);
    }
    free(h->data);
    h->data = NULL;
    h->len = h->cap = 0;
}

void clr_free(ClrSimulator *sim) {
    size_t i;
    if (!sim) {
        return;
    }
    slotvec_free(&sim->stack);
    slotvec_free(&sim->locals);
    slotvec_free(&sim->args);
    heap_free(&sim->heap);
    free(sim->bytecode);
    for (i = 0; i < sim->methods.len; i++) {
        method_free(&sim->methods.data[i]);
    }
    free(sim->methods.data);
    for (i = 0; i < sim->frames.len; i++) {
        frame_free(&sim->frames.data[i]);
    }
    free(sim->frames.data);
    free(sim);
}

/* Reset a SlotVec to hold `n` None slots. */
static int slotvec_fill_none(SlotVec *v, size_t n) {
    size_t i;
    slotvec_free(v);
    if (n == 0) {
        return 1;
    }
    if (!slotvec_reserve(v, n)) {
        return 0;
    }
    for (i = 0; i < n; i++) {
        v->data[i] = slot_none();
    }
    v->len = n;
    return 1;
}

/* Drop any live call frames and heap objects so a reload starts from a clean
 * machine (mirrors the C++ port; guards against host API misuse). */
static void reset_frames_and_heap(ClrSimulator *sim) {
    size_t i;
    for (i = 0; i < sim->frames.len; i++) {
        frame_free(&sim->frames.data[i]);
    }
    sim->frames.len = 0;
    heap_free(&sim->heap);
}

ClrStatus clr_load(ClrSimulator *sim, const uint8_t *bytecode, size_t len,
                   size_t num_locals) {
    uint8_t *copy;
    if (!sim) {
        return CLR_ERR_NO_METHOD;
    }
    reset_frames_and_heap(sim);
    copy = NULL;
    if (len > 0) {
        copy = (uint8_t *)malloc(len);
        if (!copy) {
            sim->oom = 1;
            return CLR_ERR_OUT_OF_MEMORY;
        }
        memcpy(copy, bytecode, len);
    }
    free(sim->bytecode);
    sim->bytecode = copy;
    sim->bytecode_len = len;
    sim->pc = 0;
    sim->halted = 0;
    sim->cur_method = 0;
    if (!slotvec_fill_none(&sim->locals, num_locals ? num_locals : 16)) {
        sim->oom = 1;
        return CLR_ERR_OUT_OF_MEMORY;
    }
    sim->stack.len = 0;
    sim->args.len = 0;
    return CLR_OK;
}

ClrStatus clr_load_program(ClrSimulator *sim, const ClrMethod *methods,
                           size_t n_methods, size_t entry) {
    size_t i;
    Method *em;
    uint8_t *copy;
    if (!sim) {
        return CLR_ERR_NO_METHOD;
    }
    if (n_methods == 0 || entry >= n_methods) {
        return CLR_ERR_NO_METHOD;
    }
    reset_frames_and_heap(sim);
    for (i = 0; i < sim->methods.len; i++) {
        method_free(&sim->methods.data[i]);
    }
    sim->methods.len = 0;
    if (!methodvec_reserve(&sim->methods, n_methods)) {
        sim->oom = 1;
        return CLR_ERR_OUT_OF_MEMORY;
    }
    for (i = 0; i < n_methods; i++) {
        Method m;
        m.body = NULL;
        m.body_len = methods[i].body_len;
        m.num_locals = methods[i].num_locals;
        m.num_args = methods[i].num_args;
        if (m.body_len > 0) {
            m.body = (uint8_t *)malloc(m.body_len);
            if (!m.body) {
                sim->oom = 1;
                return CLR_ERR_OUT_OF_MEMORY;
            }
            memcpy(m.body, methods[i].body, m.body_len);
        }
        sim->methods.data[sim->methods.len++] = m;
    }
    /* Enter the entry method: copy its body as the running bytecode. */
    em = &sim->methods.data[entry];
    copy = NULL;
    if (em->body_len > 0) {
        copy = (uint8_t *)malloc(em->body_len);
        if (!copy) {
            sim->oom = 1;
            return CLR_ERR_OUT_OF_MEMORY;
        }
        memcpy(copy, em->body, em->body_len);
    }
    free(sim->bytecode);
    sim->bytecode = copy;
    sim->bytecode_len = em->body_len;
    sim->pc = 0;
    sim->halted = 0;
    sim->cur_method = entry;
    if (!slotvec_fill_none(&sim->locals, em->num_locals)) {
        sim->oom = 1;
        return CLR_ERR_OUT_OF_MEMORY;
    }
    if (!slotvec_fill_none(&sim->args, em->num_args)) {
        sim->oom = 1;
        return CLR_ERR_OUT_OF_MEMORY;
    }
    sim->stack.len = 0;
    return CLR_OK;
}

/* ── ldarg / ldloc / stloc helpers (checked) ───────────────────────────────*/

static ClrStatus do_ldarg(ClrSimulator *sim, size_t idx) {
    ClrSlot s;
    if (idx >= sim->args.len) {
        return CLR_ERR_UNINITIALIZED_ARG;
    }
    s = sim->args.data[idx];
    if (!s.present) {
        return CLR_ERR_UNINITIALIZED_ARG;
    }
    return push_value(sim, s.value);
}

static ClrStatus do_ldloc(ClrSimulator *sim, size_t idx) {
    ClrSlot s;
    if (idx >= sim->locals.len) {
        return CLR_ERR_UNINITIALIZED_LOCAL;
    }
    s = sim->locals.data[idx];
    if (!s.present) {
        return CLR_ERR_UNINITIALIZED_LOCAL;
    }
    return push_value(sim, s.value);
}

static ClrStatus do_stloc(ClrSimulator *sim, size_t idx) {
    ClrValue v;
    ClrStatus st = pop_value(sim, &v);
    if (st != CLR_OK) {
        return st;
    }
    if (!slotvec_set(&sim->locals, idx, slot_some(v))) {
        sim->oom = 1;
        return CLR_ERR_OUT_OF_MEMORY;
    }
    return CLR_OK;
}

/* ── One step ──────────────────────────────────────────────────────────────*/

ClrStatus clr_step(ClrSimulator *sim) {
    uint8_t op;
    if (!sim) {
        return CLR_ERR_NO_METHOD;
    }
    if (sim->halted) {
        return CLR_ERR_HALTED;
    }
    if (sim->pc >= sim->bytecode_len) {
        return CLR_ERR_PC_OUT_OF_RANGE;
    }
    op = sim->bytecode[sim->pc];

    if (op == CLR_OP_NOP) {
        sim->pc += 1;
        return CLR_OK;
    }
    if (op >= CLR_OP_LDARG_0 && op <= CLR_OP_LDARG_3) {
        ClrStatus st = do_ldarg(sim, (size_t)(op - CLR_OP_LDARG_0));
        if (st != CLR_OK) {
            return st;
        }
        sim->pc += 1;
        return CLR_OK;
    }
    if (op == CLR_OP_LDARG_S) {
        uint8_t idx;
        ClrStatus st = read_u8(sim, 1, &idx);
        if (st != CLR_OK) {
            return st;
        }
        st = do_ldarg(sim, idx);
        if (st != CLR_OK) {
            return st;
        }
        sim->pc += 2;
        return CLR_OK;
    }
    if (op == CLR_OP_LDNULL) {
        ClrStatus st = push_value(sim, value_null());
        if (st != CLR_OK) {
            return st;
        }
        sim->pc += 1;
        return CLR_OK;
    }
    if (op == CLR_OP_DUP) {
        ClrValue v;
        ClrStatus st;
        if (sim->stack.len == 0) {
            return CLR_ERR_STACK_UNDERFLOW;
        }
        if (!sim->stack.data[sim->stack.len - 1].present) {
            return CLR_ERR_NULL_OPERAND;
        }
        v = sim->stack.data[sim->stack.len - 1].value;
        st = push_value(sim, v);
        if (st != CLR_OK) {
            return st;
        }
        sim->pc += 1;
        return CLR_OK;
    }
    if (op >= CLR_OP_LDC_I4_0 && op <= CLR_OP_LDC_I4_8) {
        ClrStatus st =
            push_value(sim, value_int((int32_t)(op - CLR_OP_LDC_I4_0)));
        if (st != CLR_OK) {
            return st;
        }
        sim->pc += 1;
        return CLR_OK;
    }
    if (op == CLR_OP_LDC_I4_S) {
        uint8_t b;
        ClrStatus st = read_u8(sim, 1, &b);
        if (st != CLR_OK) {
            return st;
        }
        st = push_value(sim, value_int((int32_t)(int8_t)b));
        if (st != CLR_OK) {
            return st;
        }
        sim->pc += 2;
        return CLR_OK;
    }
    if (op == CLR_OP_LDC_I4) {
        int32_t n;
        ClrStatus st = read_i32_operand(sim, &n);
        if (st != CLR_OK) {
            return st;
        }
        st = push_value(sim, value_int(n));
        if (st != CLR_OK) {
            return st;
        }
        sim->pc += 5;
        return CLR_OK;
    }
    if (op >= CLR_OP_LDLOC_0 && op <= CLR_OP_LDLOC_3) {
        ClrStatus st = do_ldloc(sim, (size_t)(op - CLR_OP_LDLOC_0));
        if (st != CLR_OK) {
            return st;
        }
        sim->pc += 1;
        return CLR_OK;
    }
    if (op == CLR_OP_LDLOC_S) {
        uint8_t idx;
        ClrStatus st = read_u8(sim, 1, &idx);
        if (st != CLR_OK) {
            return st;
        }
        st = do_ldloc(sim, idx);
        if (st != CLR_OK) {
            return st;
        }
        sim->pc += 2;
        return CLR_OK;
    }
    if (op >= CLR_OP_STLOC_0 && op <= CLR_OP_STLOC_3) {
        ClrStatus st = do_stloc(sim, (size_t)(op - CLR_OP_STLOC_0));
        if (st != CLR_OK) {
            return st;
        }
        sim->pc += 1;
        return CLR_OK;
    }
    if (op == CLR_OP_STLOC_S) {
        uint8_t idx;
        ClrStatus st = read_u8(sim, 1, &idx);
        if (st != CLR_OK) {
            return st;
        }
        st = do_stloc(sim, idx);
        if (st != CLR_OK) {
            return st;
        }
        sim->pc += 2;
        return CLR_OK;
    }
    if (op == CLR_OP_NEWARR) {
        ClrValue lenv;
        int32_t n;
        size_t count, i;
        HeapArray arr;
        ClrStatus st = need_bytes(sim, 5); /* type token operand */
        if (st != CLR_OK) {
            return st;
        }
        st = pop_value(sim, &lenv);
        if (st != CLR_OK) {
            return st;
        }
        st = value_as_int(lenv, &n);
        if (st != CLR_OK) {
            return st;
        }
        count = (n > 0) ? (size_t)n : 0; /* len.max(0) */
        if (count > CLR_MAX_ARRAY_LEN) {
            return CLR_ERR_ARRAY_TOO_LARGE;
        }
        arr.len = count;
        arr.data = NULL;
        if (count > 0) {
            arr.data = (ClrValue *)calloc(count, sizeof(ClrValue));
            if (!arr.data) {
                sim->oom = 1;
                return CLR_ERR_OUT_OF_MEMORY;
            }
            for (i = 0; i < count; i++) {
                arr.data[i] = value_null();
            }
        }
        if (!heapvec_reserve(&sim->heap, sim->heap.len + 1)) {
            free(arr.data);
            sim->oom = 1;
            return CLR_ERR_OUT_OF_MEMORY;
        }
        st = push_value(sim, value_ref(sim->heap.len));
        if (st != CLR_OK) {
            free(arr.data);
            return st;
        }
        sim->heap.data[sim->heap.len++] = arr;
        sim->pc += 5;
        return CLR_OK;
    }
    if (op == CLR_OP_STELEM_REF) {
        ClrValue val, idxv, arrv;
        int32_t idx;
        HeapArray *arr;
        ClrStatus st = pop_value(sim, &val);
        if (st != CLR_OK) {
            return st;
        }
        st = pop_value(sim, &idxv);
        if (st != CLR_OK) {
            return st;
        }
        st = pop_value(sim, &arrv);
        if (st != CLR_OK) {
            return st;
        }
        st = value_as_int(idxv, &idx);
        if (st != CLR_OK) {
            return st;
        }
        if (arrv.kind != CLR_REF) {
            return CLR_ERR_EXPECTED_ARRAY;
        }
        if (!arrv.ref_some) {
            return CLR_ERR_NULL_REFERENCE;
        }
        if (arrv.ref_idx >= sim->heap.len) {
            return CLR_ERR_INDEX_OUT_OF_RANGE;
        }
        arr = &sim->heap.data[arrv.ref_idx];
        if (idx < 0 || (size_t)idx >= arr->len) {
            return CLR_ERR_INDEX_OUT_OF_RANGE;
        }
        arr->data[(size_t)idx] = val;
        sim->pc += 1;
        return CLR_OK;
    }
    if (op == CLR_OP_LDELEM_REF) {
        ClrValue idxv, arrv;
        int32_t idx;
        HeapArray *arr;
        ClrStatus st = pop_value(sim, &idxv);
        if (st != CLR_OK) {
            return st;
        }
        st = pop_value(sim, &arrv);
        if (st != CLR_OK) {
            return st;
        }
        st = value_as_int(idxv, &idx);
        if (st != CLR_OK) {
            return st;
        }
        if (arrv.kind != CLR_REF) {
            return CLR_ERR_EXPECTED_ARRAY;
        }
        if (!arrv.ref_some) {
            return CLR_ERR_NULL_REFERENCE;
        }
        if (arrv.ref_idx >= sim->heap.len) {
            return CLR_ERR_INDEX_OUT_OF_RANGE;
        }
        arr = &sim->heap.data[arrv.ref_idx];
        if (idx < 0 || (size_t)idx >= arr->len) {
            return CLR_ERR_INDEX_OUT_OF_RANGE;
        }
        st = push_value(sim, arr->data[(size_t)idx]);
        if (st != CLR_OK) {
            return st;
        }
        sim->pc += 1;
        return CLR_OK;
    }
    if (op == CLR_OP_BOX || op == CLR_OP_UNBOX_ANY) {
        /* identity in this model; type token operand at pc+1..pc+5 */
        ClrStatus st = need_bytes(sim, 5);
        if (st != CLR_OK) {
            return st;
        }
        if (sim->stack.len == 0) {
            return CLR_ERR_STACK_UNDERFLOW;
        }
        sim->pc += 5;
        return CLR_OK;
    }
    if (op == CLR_OP_ISINST) {
        ClrValue v;
        ClrStatus st = need_bytes(sim, 5);
        if (st != CLR_OK) {
            return st;
        }
        st = pop_value(sim, &v);
        if (st != CLR_OK) {
            return st;
        }
        if (v.kind == CLR_REF && v.ref_some) {
            st = push_value(sim, v);
        } else {
            st = push_value(sim, value_null());
        }
        if (st != CLR_OK) {
            return st;
        }
        sim->pc += 5;
        return CLR_OK;
    }
    if (op == CLR_OP_ADD || op == CLR_OP_SUB || op == CLR_OP_MUL ||
        op == CLR_OP_XOR) {
        ClrStatus st = execute_binop(sim, op);
        if (st != CLR_OK) {
            return st;
        }
        sim->pc += 1;
        return CLR_OK;
    }
    if (op == CLR_OP_DIV) {
        ClrStatus st = execute_div(sim);
        if (st != CLR_OK) {
            return st;
        }
        sim->pc += 1;
        return CLR_OK;
    }
    if (op == CLR_OP_PREFIX_FE) {
        uint8_t sub;
        ClrStatus st = read_u8(sim, 1, &sub);
        if (st != CLR_OK) {
            return st;
        }
        st = execute_compare(sim, sub);
        if (st != CLR_OK) {
            return st;
        }
        sim->pc += 2;
        return CLR_OK;
    }
    if (op == CLR_OP_CALL) {
        int32_t token;
        uint32_t ordinal;
        size_t callee_idx, k;
        Method *callee;
        Frame fr;
        SlotVec new_args, new_locals;
        uint8_t *copy;
        ClrStatus st = read_i32_operand(sim, &token);
        if (st != CLR_OK) {
            return st;
        }
        ordinal = (uint32_t)token & 0x00FFFFFFu;
        if (ordinal == 0) {
            return CLR_ERR_INVALID_TOKEN;
        }
        callee_idx = (size_t)(ordinal - 1);
        if (callee_idx >= sim->methods.len) {
            return CLR_ERR_INVALID_TOKEN;
        }
        if (sim->frames.len >= CLR_MAX_CALL_DEPTH) {
            return CLR_ERR_CALL_DEPTH_EXCEEDED;
        }
        callee = &sim->methods.data[callee_idx];
        if (sim->stack.len < callee->num_args) {
            return CLR_ERR_STACK_UNDERFLOW;
        }
        /* Build the callee's frame in fresh storage BEFORE mutating sim, so an
         * OOM leaves the machine untouched. */
        new_args.data = NULL;
        new_args.len = new_args.cap = 0;
        new_locals.data = NULL;
        new_locals.len = new_locals.cap = 0;
        if (!slotvec_fill_none(&new_args, callee->num_args)) {
            slotvec_free(&new_args);
            sim->oom = 1;
            return CLR_ERR_OUT_OF_MEMORY;
        }
        if (!slotvec_fill_none(&new_locals, callee->num_locals)) {
            slotvec_free(&new_args);
            slotvec_free(&new_locals);
            sim->oom = 1;
            return CLR_ERR_OUT_OF_MEMORY;
        }
        copy = NULL;
        if (callee->body_len > 0) {
            copy = (uint8_t *)malloc(callee->body_len);
            if (!copy) {
                slotvec_free(&new_args);
                slotvec_free(&new_locals);
                sim->oom = 1;
                return CLR_ERR_OUT_OF_MEMORY;
            }
            memcpy(copy, callee->body, callee->body_len);
        }
        if (!framevec_reserve(&sim->frames, sim->frames.len + 1)) {
            free(copy);
            slotvec_free(&new_args);
            slotvec_free(&new_locals);
            sim->oom = 1;
            return CLR_ERR_OUT_OF_MEMORY;
        }
        /* Commit. Pop args in reverse into the fresh args vector. */
        for (k = 0; k < callee->num_args; k++) {
            size_t dst = callee->num_args - 1 - k;
            new_args.data[dst] = sim->stack.data[--sim->stack.len];
        }
        fr.return_pc = sim->pc + 5;
        fr.return_method = sim->cur_method;
        fr.return_bytecode = sim->bytecode; /* transfer ownership */
        fr.return_bytecode_len = sim->bytecode_len;
        fr.return_locals = sim->locals;
        fr.return_args = sim->args;
        sim->frames.data[sim->frames.len++] = fr;
        sim->bytecode = copy;
        sim->bytecode_len = callee->body_len;
        sim->args = new_args;
        sim->locals = new_locals;
        sim->cur_method = callee_idx;
        sim->pc = 0;
        return CLR_OK;
    }
    if (op == CLR_OP_RET) {
        Frame fr;
        if (sim->frames.len == 0) {
            sim->halted = 1;
            return CLR_OK;
        }
        fr = sim->frames.data[--sim->frames.len];
        free(sim->bytecode);
        slotvec_free(&sim->locals);
        slotvec_free(&sim->args);
        sim->bytecode = fr.return_bytecode;
        sim->bytecode_len = fr.return_bytecode_len;
        sim->locals = fr.return_locals;
        sim->args = fr.return_args;
        sim->cur_method = fr.return_method;
        sim->pc = fr.return_pc;
        return CLR_OK;
    }
    if (op == CLR_OP_BR_S) {
        uint8_t b;
        int32_t offset;
        int64_t target;
        ClrStatus st = read_u8(sim, 1, &b);
        if (st != CLR_OK) {
            return st;
        }
        offset = (int32_t)(int8_t)b;
        target = (int64_t)sim->pc + 2 + offset;
        if (target < 0 || (uint64_t)target > (uint64_t)sim->bytecode_len) {
            return CLR_ERR_PC_OUT_OF_RANGE;
        }
        sim->pc = (size_t)target;
        return CLR_OK;
    }
    if (op == CLR_OP_BRFALSE_S || op == CLR_OP_BRTRUE_S) {
        uint8_t b;
        int32_t offset;
        ClrValue v;
        int truthy;
        ClrStatus st = read_u8(sim, 1, &b);
        if (st != CLR_OK) {
            return st;
        }
        offset = (int32_t)(int8_t)b;
        st = pop_value(sim, &v);
        if (st != CLR_OK) {
            return st;
        }
        truthy = value_is_truthy(v);
        if ((op == CLR_OP_BRTRUE_S && truthy) ||
            (op == CLR_OP_BRFALSE_S && !truthy)) {
            int64_t target = (int64_t)sim->pc + 2 + offset;
            if (target < 0 || (uint64_t)target > (uint64_t)sim->bytecode_len) {
                return CLR_ERR_PC_OUT_OF_RANGE;
            }
            sim->pc = (size_t)target;
        } else {
            sim->pc += 2;
        }
        return CLR_OK;
    }
    return CLR_ERR_UNKNOWN_OPCODE;
}

ClrStatus clr_run(ClrSimulator *sim, size_t max_steps, size_t *out_steps) {
    size_t n = 0;
    ClrStatus st = CLR_OK;
    if (!sim) {
        if (out_steps) {
            *out_steps = 0;
        }
        return CLR_ERR_NO_METHOD;
    }
    while (n < max_steps && !sim->halted) {
        st = clr_step(sim);
        if (st != CLR_OK) {
            break;
        }
        n++;
    }
    if (out_steps) {
        *out_steps = n;
    }
    return st;
}

/* ── Inspection ────────────────────────────────────────────────────────────*/

int clr_halted(const ClrSimulator *sim) { return sim ? sim->halted : 1; }
size_t clr_pc(const ClrSimulator *sim) { return sim ? sim->pc : 0; }
size_t clr_stack_len(const ClrSimulator *sim) {
    return sim ? sim->stack.len : 0;
}

int clr_stack_at(const ClrSimulator *sim, size_t i, ClrSlot *out) {
    if (!sim || !out || i >= sim->stack.len) {
        return 0;
    }
    *out = sim->stack.data[i];
    return 1;
}

int clr_stack_top(const ClrSimulator *sim, ClrSlot *out) {
    if (!sim || !out || sim->stack.len == 0) {
        return 0;
    }
    *out = sim->stack.data[sim->stack.len - 1];
    return 1;
}

int clr_local_at(const ClrSimulator *sim, size_t slot, ClrSlot *out) {
    if (!sim || !out || slot >= sim->locals.len) {
        return 0;
    }
    *out = sim->locals.data[slot];
    return 1;
}

/* ── Encoding helpers ──────────────────────────────────────────────────────*/

size_t clr_encode_ldc_i4(int32_t n, uint8_t *out) {
    if (n >= 0 && n <= 8) {
        out[0] = (uint8_t)(CLR_OP_LDC_I4_0 + n);
        return 1;
    }
    if (n >= -128 && n <= 127) {
        out[0] = CLR_OP_LDC_I4_S;
        out[1] = (uint8_t)(int8_t)n;
        return 2;
    }
    {
        uint32_t u = (uint32_t)n;
        out[0] = CLR_OP_LDC_I4;
        out[1] = (uint8_t)(u & 0xFFu);
        out[2] = (uint8_t)((u >> 8) & 0xFFu);
        out[3] = (uint8_t)((u >> 16) & 0xFFu);
        out[4] = (uint8_t)((u >> 24) & 0xFFu);
        return 5;
    }
}

size_t clr_encode_stloc(uint8_t slot, uint8_t *out) {
    if (slot <= 3) {
        out[0] = (uint8_t)(CLR_OP_STLOC_0 + slot);
        return 1;
    }
    out[0] = CLR_OP_STLOC_S;
    out[1] = slot;
    return 2;
}

size_t clr_encode_ldloc(uint8_t slot, uint8_t *out) {
    if (slot <= 3) {
        out[0] = (uint8_t)(CLR_OP_LDLOC_0 + slot);
        return 1;
    }
    out[0] = CLR_OP_LDLOC_S;
    out[1] = slot;
    return 2;
}
