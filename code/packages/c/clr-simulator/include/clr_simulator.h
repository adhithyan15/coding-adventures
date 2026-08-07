/*
 * clr_simulator.h — a CLR (.NET) bytecode simulator, pure ISO C17.
 * ===============================================================
 *
 * A faithful port of the Rust `clr-simulator` crate: a type-inferring,
 * stack-based virtual machine for a subset of Microsoft's CIL (the CLR's
 * bytecode). Unlike the JVM, the CLR infers operand types from the stack — one
 * `add` opcode works for any numeric type.
 *
 * The value model is `ClrValue`: a 32-bit integer, or an object reference
 * (`null`, or an index into the object heap of `object[]` arrays). Stack and
 * local slots are optional (an unset local is distinct from a `null` value).
 *
 * The Rust crate PANICS on malformed input (stack underflow, out-of-range
 * operand, bad opcode) — safe in Rust because slice indexing is bounds-checked.
 * This port returns a `ClrStatus` for every such case instead, so executing
 * UNTRUSTED bytecode never reads out of bounds.
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef CLR_SIMULATOR_H
#define CLR_SIMULATOR_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* int32_t, uint8_t, uint32_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Opcodes ───────────────────────────────────────────────────────────────*/
#define CLR_OP_NOP 0x00u
#define CLR_OP_LDNULL 0x14u
#define CLR_OP_LDLOC_0 0x06u
#define CLR_OP_LDLOC_3 0x09u
#define CLR_OP_STLOC_0 0x0Au
#define CLR_OP_STLOC_3 0x0Du
#define CLR_OP_LDLOC_S 0x11u
#define CLR_OP_STLOC_S 0x13u
#define CLR_OP_LDC_I4_0 0x16u
#define CLR_OP_LDC_I4_8 0x1Eu
#define CLR_OP_LDC_I4_S 0x1Fu
#define CLR_OP_LDC_I4 0x20u
#define CLR_OP_DUP 0x25u
#define CLR_OP_LDARG_0 0x02u
#define CLR_OP_LDARG_3 0x05u
#define CLR_OP_LDARG_S 0x0Eu
#define CLR_OP_CALL 0x28u
#define CLR_OP_RET 0x2Au
#define CLR_OP_BR_S 0x2Bu
#define CLR_OP_BRFALSE_S 0x2Cu
#define CLR_OP_BRTRUE_S 0x2Du
#define CLR_OP_ADD 0x58u
#define CLR_OP_SUB 0x59u
#define CLR_OP_MUL 0x5Au
#define CLR_OP_DIV 0x5Bu
#define CLR_OP_XOR 0x61u
#define CLR_OP_BOX 0x8Cu
#define CLR_OP_NEWARR 0x8Du
#define CLR_OP_LDELEM_REF 0xA2u
#define CLR_OP_STELEM_REF 0xA4u
#define CLR_OP_UNBOX_ANY 0xA5u
#define CLR_OP_ISINST 0x75u
#define CLR_OP_PREFIX_FE 0xFEu
#define CLR_CEQ_BYTE 0x01u
#define CLR_CGT_BYTE 0x02u
#define CLR_CLT_BYTE 0x04u

/* DoS guards (matching the Rust crate). */
#define CLR_MAX_ARRAY_LEN ((size_t)(1u << 20))
#define CLR_MAX_CALL_DEPTH ((size_t)10000)

/* ── Status codes (the Rust API panics; this one returns) ──────────────────*/
typedef enum {
    CLR_OK = 0,
    CLR_ERR_STACK_UNDERFLOW,
    CLR_ERR_EXPECTED_INT,       /* arithmetic on a reference */
    CLR_ERR_NULL_OPERAND,       /* popped an unset (None) slot */
    CLR_ERR_DIVIDE_BY_ZERO,
    CLR_ERR_NULL_REFERENCE,
    CLR_ERR_EXPECTED_ARRAY,
    CLR_ERR_INDEX_OUT_OF_RANGE,
    CLR_ERR_UNINITIALIZED_LOCAL,
    CLR_ERR_UNINITIALIZED_ARG,
    CLR_ERR_LOCAL_OUT_OF_RANGE,
    CLR_ERR_PC_OUT_OF_RANGE,
    CLR_ERR_BYTECODE_OVERRUN,   /* an operand ran past the bytecode end */
    CLR_ERR_HALTED,
    CLR_ERR_UNKNOWN_OPCODE,
    CLR_ERR_ARRAY_TOO_LARGE,
    CLR_ERR_CALL_DEPTH_EXCEEDED,
    CLR_ERR_INVALID_TOKEN,
    CLR_ERR_NO_METHOD,
    CLR_ERR_OUT_OF_MEMORY
} ClrStatus;

/* ── Value model ───────────────────────────────────────────────────────────*/

typedef enum { CLR_INT, CLR_REF } ClrValueKind;

/* A stack value: an int, or an object reference (`ref_some == 0` is null;
 * otherwise `ref_idx` indexes the object heap). */
typedef struct {
    ClrValueKind kind;
    int32_t i;      /* CLR_INT */
    int ref_some;   /* CLR_REF: 1 = Some(heap idx), 0 = null */
    size_t ref_idx; /* CLR_REF && ref_some */
} ClrValue;

/* A stack / local / arg slot: `present == 0` is an unset (None) slot. */
typedef struct {
    int present;
    ClrValue value;
} ClrSlot;

/* One method's bytecode + frame shape. `body` is borrowed for the duration of
 * the load call (copied internally). */
typedef struct {
    const uint8_t *body;
    size_t body_len;
    size_t num_locals;
    size_t num_args;
} ClrMethod;

/* ── The simulator ─────────────────────────────────────────────────────────*/

typedef struct ClrSimulator ClrSimulator;

ClrSimulator *clr_new(void);
void clr_free(ClrSimulator *sim);

/* Load a single method (no calls). Copies `bytecode`. */
ClrStatus clr_load(ClrSimulator *sim, const uint8_t *bytecode, size_t len,
                   size_t num_locals);
/* Load a whole method table and start executing at `entry`. Copies bodies. */
ClrStatus clr_load_program(ClrSimulator *sim, const ClrMethod *methods,
                           size_t n_methods, size_t entry);

/* Execute one instruction. On a clean `ret` from the entry method the machine
 * halts (returns CLR_OK with clr_halted() true). */
ClrStatus clr_step(ClrSimulator *sim);
/* Step up to `max_steps` times, stopping when halted or on error. `out_steps`
 * (may be NULL) receives the number of steps executed. */
ClrStatus clr_run(ClrSimulator *sim, size_t max_steps, size_t *out_steps);

int clr_halted(const ClrSimulator *sim);
size_t clr_pc(const ClrSimulator *sim);

/* Stack / local inspection. `*out` receives the slot; returns 0 if the index is
 * out of range. */
size_t clr_stack_len(const ClrSimulator *sim);
int clr_stack_at(const ClrSimulator *sim, size_t i, ClrSlot *out);
int clr_stack_top(const ClrSimulator *sim, ClrSlot *out);
int clr_local_at(const ClrSimulator *sim, size_t slot, ClrSlot *out);

/* ── Encoding helpers ──────────────────────────────────────────────────────*/

/* Encode `ldc.i4 n` in its most compact form into `out` (needs room for 5
 * bytes); returns the number of bytes written. */
size_t clr_encode_ldc_i4(int32_t n, uint8_t *out);
/* Encode `stloc slot` (compact for 0..3) into `out` (needs 2 bytes). */
size_t clr_encode_stloc(uint8_t slot, uint8_t *out);
/* Encode `ldloc slot` (compact for 0..3) into `out` (needs 2 bytes). */
size_t clr_encode_ldloc(uint8_t slot, uint8_t *out);

#ifdef __cplusplus
}
#endif

#endif /* CLR_SIMULATOR_H */
