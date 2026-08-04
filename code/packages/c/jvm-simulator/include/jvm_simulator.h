/*
 * jvm_simulator.h — a typed stack-based JVM virtual machine, in pure ISO C17. A
 * faithful port of the Rust `jvm-simulator` crate.
 * ===========================================================================
 *
 * Like WASM, the JVM is a stack machine — but a TYPED one: instead of a generic
 * `add`, the type lives in the opcode (`iadd` / `ladd` / `fadd` / `dadd`), so a
 * verifier can check type safety at class-load time. Locals are numbered slots
 * (this VM models the int-typed subset); compact opcodes exist for slots 0-3.
 *
 * Supported opcodes (an int subset):
 *   iconst_0..5, bipush, ldc (constant pool), iload / iload_0..3, istore /
 *   istore_0..3, iadd / isub / imul / idiv, if_icmpeq / if_icmpgt / goto (16-bit
 *   signed branch offsets), ireturn / return. Arithmetic wraps modulo 2^32.
 *
 * The simulator decodes and executes bytecode, producing a `JvmTrace` per
 * instruction (the stack before/after, a locals snapshot — each slot may be
 * UNINITIALIZED — and a description).
 *
 * DIVERGENCE FROM RUST. Where the Rust panics (halted step, PC past the end,
 * unknown opcode, truncated operand, stack underflow, constant-pool index out of
 * range, division by zero, an uninitialized/out-of-range local), this port
 * returns a `JvmStatus`.
 *
 * PORTABILITY. Pure ISO C17 — no extensions. Builds clean under GCC, Clang, and
 * MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
 */
#ifndef CA_JVM_SIMULATOR_H
#define CA_JVM_SIMULATOR_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opcodes (the supported int subset). The compact forms span a range. */
#define JVM_OP_ICONST_0 0x03u
#define JVM_OP_ICONST_5 0x08u
#define JVM_OP_BIPUSH 0x10u
#define JVM_OP_LDC 0x12u
#define JVM_OP_ILOAD 0x15u
#define JVM_OP_ILOAD_0 0x1Au
#define JVM_OP_ILOAD_3 0x1Du
#define JVM_OP_ISTORE 0x36u
#define JVM_OP_ISTORE_0 0x3Bu
#define JVM_OP_ISTORE_3 0x3Eu
#define JVM_OP_IADD 0x60u
#define JVM_OP_ISUB 0x64u
#define JVM_OP_IMUL 0x68u
#define JVM_OP_IDIV 0x6Cu
#define JVM_OP_IF_ICMPEQ 0x9Fu
#define JVM_OP_IF_ICMPGT 0xA3u
#define JVM_OP_GOTO 0xA7u
#define JVM_OP_IRETURN 0xACu
#define JVM_OP_RETURN 0xB1u

/* Status of a fallible operation (the Rust panics, as codes). */
typedef enum {
    JVM_OK = 0,
    JVM_ERR_NOMEM,
    JVM_ERR_HALTED,
    JVM_ERR_PC_OUT_OF_BOUNDS,
    JVM_ERR_UNKNOWN_OPCODE,
    JVM_ERR_TRUNCATED,
    JVM_ERR_STACK_UNDERFLOW,
    JVM_ERR_CONST_OUT_OF_RANGE,
    JVM_ERR_DIV_BY_ZERO,
    JVM_ERR_LOCAL_UNINITIALIZED,
    JVM_ERR_LOCAL_OUT_OF_RANGE
} JvmStatus;

/* A local variable slot: a value, or uninitialized (`initialized == 0`). */
typedef struct {
    int32_t value;
    int initialized;
} JvmLocal;

/* A record of one instruction's execution (all pointers malloc'd; release with
 * `jvm_trace_free`). */
typedef struct {
    size_t pc;
    char *opcode; /* mnemonic, e.g. "iconst_1", "if_icmpeq" */
    int32_t *stack_before;
    size_t n_stack_before;
    int32_t *stack_after;
    size_t n_stack_after;
    JvmLocal *locals_snapshot;
    size_t n_locals;
    char *description;
} JvmTrace;

void jvm_trace_free(JvmTrace *t);

/* The full simulation environment. */
typedef struct JvmSimulator JvmSimulator;

JvmSimulator *jvm_sim_new(void); /* NULL on OOM (16 locals by default) */
void jvm_sim_free(JvmSimulator *s);
/* Load bytecode, a constant pool, and a locals count; resets all state. Returns
 * 0 or -1 on OOM. */
int jvm_sim_load(JvmSimulator *s, const uint8_t *bytecode, size_t blen,
                 const int32_t *constants, size_t nconstants, size_t num_locals);

/* Execute one instruction, filling *trace on JVM_OK (release with
 * jvm_trace_free). */
JvmStatus jvm_sim_step(JvmSimulator *s, JvmTrace *trace);
/* Run until halt or `max_steps`. On JVM_OK, writes a malloc'd array of
 * `*count_out` traces (free with jvm_traces_free); an execution error yields
 * that status and no traces. */
JvmStatus jvm_sim_run(JvmSimulator *s, size_t max_steps, JvmTrace **traces_out,
                      size_t *count_out);
void jvm_traces_free(JvmTrace *traces, size_t count);

/* State accessors (borrowed). */
const int32_t *jvm_sim_stack(const JvmSimulator *s, size_t *count_out);
const JvmLocal *jvm_sim_locals(const JvmSimulator *s, size_t *count_out);
size_t jvm_sim_pc(const JvmSimulator *s);
int jvm_sim_halted(const JvmSimulator *s);
/* The return value if the method returned one (ireturn). Returns 1 (sets *out)
 * or 0 if there is none. */
int jvm_sim_return_value(const JvmSimulator *s, int32_t *out);

/* ── Bytecode assembler ─────────────────────────────────────────────────── */

typedef struct {
    uint8_t *data;
    size_t len, cap;
} JvmProgram;

void jvm_program_init(JvmProgram *p);
void jvm_program_free(JvmProgram *p);
const uint8_t *jvm_program_bytes(const JvmProgram *p, size_t *len_out);

/* Append one instruction: `opcode` plus its operands (a 1-byte operand for
 * bipush/iload/istore/ldc, or a 2-byte big-endian offset for goto/if_icmp*).
 * `operand` is ignored for operand-less opcodes. Returns 0 or -1 on OOM. */
int jvm_emit(JvmProgram *p, uint8_t opcode, int32_t operand);

#ifdef __cplusplus
}
#endif

#endif /* CA_JVM_SIMULATOR_H */
