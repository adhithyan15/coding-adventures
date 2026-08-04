/*
 * wasm_simulator.h — a stack-based WebAssembly virtual machine, in pure ISO
 * C17. A faithful port of the Rust `wasm-simulator` crate.
 * ===========================================================================
 *
 * Unlike a register machine (RISC-V, ARM) whose instructions name explicit
 * register operands, WASM is a STACK machine: operands live on an implicit
 * operand stack. `i32.const 10 / i32.const 20 / i32.add` pushes 10 and 20, then
 * `add` pops both and pushes 30. Bytecode is variable-length: one opcode byte,
 * optionally followed by operand bytes.
 *
 * Supported opcodes (an i32 subset):
 *   0x0B end · 0x20 local.get · 0x21 local.set · 0x41 i32.const · 0x6A i32.add
 *   · 0x6B i32.sub. Arithmetic wraps modulo 2^32.
 *
 * The simulator decodes and executes bytecode, producing a `WasmStepTrace` per
 * instruction (the stack before/after, a locals snapshot, and a description) —
 * the WASM equivalent of a CPU pipeline trace.
 *
 * OWNERSHIP. Malloc-owned handles and traces; release each with the matching
 * `*_free`. The `WasmProgram` builder assembles bytecode.
 *
 * DIVERGENCE FROM RUST. Where the Rust panics (unknown opcode, stack underflow,
 * stepping a halted VM, out-of-range local/read), this port returns a
 * `WasmStatus` code — a library must not abort its host.
 *
 * PORTABILITY. Pure ISO C17 — no extensions. Builds clean under GCC, Clang, and
 * MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
 */
#ifndef CA_WASM_SIMULATOR_H
#define CA_WASM_SIMULATOR_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* WASM instruction opcodes (the supported i32 subset). */
#define WASM_OP_END 0x0Bu
#define WASM_OP_LOCAL_GET 0x20u
#define WASM_OP_LOCAL_SET 0x21u
#define WASM_OP_I32_CONST 0x41u
#define WASM_OP_I32_ADD 0x6Au
#define WASM_OP_I32_SUB 0x6Bu

/* Status of a fallible operation (the Rust panics, as return codes). */
typedef enum {
    WASM_OK = 0,
    WASM_ERR_NOMEM,
    WASM_ERR_UNKNOWN_OPCODE,   /* decode hit an unsupported opcode */
    WASM_ERR_TRUNCATED,        /* an instruction ran past the end of the code */
    WASM_ERR_STACK_UNDERFLOW,  /* a pop on an empty stack */
    WASM_ERR_LOCAL_OUT_OF_RANGE, /* local.get/set index past the locals */
    WASM_ERR_HALTED            /* step() after the VM halted */
} WasmStatus;

/* A decoded instruction: opcode, mnemonic, an optional operand, and byte size. */
typedef struct {
    uint8_t opcode;
    const char *mnemonic; /* static string (e.g. "i32.const") */
    int has_operand;
    int32_t operand;
    size_t size; /* opcode + operand bytes (how far to advance the PC) */
} WasmInstruction;

/* Decode one instruction at `pc`. Returns WASM_OK (fills *out), or
 * WASM_ERR_UNKNOWN_OPCODE / WASM_ERR_TRUNCATED. */
WasmStatus wasm_decode(const uint8_t *bytecode, size_t len, size_t pc,
                       WasmInstruction *out);

/* A complete record of one instruction's execution. All arrays and the
 * description are malloc'd; release with `wasm_step_trace_free`. */
typedef struct {
    size_t pc;
    WasmInstruction instruction;
    int32_t *stack_before;
    size_t n_stack_before;
    int32_t *stack_after;
    size_t n_stack_after;
    int32_t *locals_snapshot;
    size_t n_locals;
    char *description;
    int halted;
} WasmStepTrace;

void wasm_step_trace_free(WasmStepTrace *t);

/* The full simulation environment. */
typedef struct WasmSimulator WasmSimulator;

WasmSimulator *wasm_sim_new(size_t num_locals); /* NULL on OOM */
void wasm_sim_free(WasmSimulator *s);
/* Load bytecode (copied), resetting pc / stack / locals / halted / cycle.
 * Returns 0 or -1 on OOM. */
int wasm_sim_load(WasmSimulator *s, const uint8_t *bytecode, size_t len);

/* Execute one instruction, filling *trace (on WASM_OK; release with
 * wasm_step_trace_free). Returns WASM_ERR_HALTED / decode / execution errors
 * otherwise. */
WasmStatus wasm_sim_step(WasmSimulator *s, WasmStepTrace *trace);

/* Run `program` to completion (an `end`) or `max_steps`. On WASM_OK, writes a
 * malloc'd array of `*count_out` traces to *traces_out (free with
 * wasm_traces_free); on an execution error, returns that status and produces no
 * traces. */
WasmStatus wasm_sim_run(WasmSimulator *s, const uint8_t *program, size_t len,
                        size_t max_steps, WasmStepTrace **traces_out,
                        size_t *count_out);
void wasm_traces_free(WasmStepTrace *traces, size_t count);

/* State accessors (borrowed; valid until the next mutation / free). */
const int32_t *wasm_sim_stack(const WasmSimulator *s, size_t *count_out);
const int32_t *wasm_sim_locals(const WasmSimulator *s, size_t *count_out);
size_t wasm_sim_pc(const WasmSimulator *s);
int wasm_sim_halted(const WasmSimulator *s);
size_t wasm_sim_cycle(const WasmSimulator *s);

/* ── Bytecode assembler ─────────────────────────────────────────────────── */

/* A growable bytecode buffer. */
typedef struct {
    uint8_t *data;
    size_t len, cap;
} WasmProgram;

void wasm_program_init(WasmProgram *p);
void wasm_program_free(WasmProgram *p);
const uint8_t *wasm_program_bytes(const WasmProgram *p, size_t *len_out);

/* Emit helpers (append encoded instructions). Each returns 0 or -1 on OOM. */
int wasm_emit_i32_const(WasmProgram *p, int32_t val); /* opcode + LE i32 (5 B) */
int wasm_emit_i32_add(WasmProgram *p);
int wasm_emit_i32_sub(WasmProgram *p);
int wasm_emit_local_get(WasmProgram *p, uint8_t idx);
int wasm_emit_local_set(WasmProgram *p, uint8_t idx);
int wasm_emit_end(WasmProgram *p);

#ifdef __cplusplus
}
#endif

#endif /* CA_WASM_SIMULATOR_H */
