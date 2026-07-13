/*
 * aarch64_encoder.h — AArch64 (ARM64) instruction encoder, pure ISO C17.
 * =====================================================================
 *
 * A faithful port of the Rust `aarch64-encoder` crate: a stream-style assembler
 * that produces little-endian 32-bit instruction words for the AArch64
 * instruction set (the bottom of a CIR → native-code lowering).
 *
 * Each `a64_*` method emits one 4-byte instruction word. Branches reference an
 * `A64Label` bound to a later instruction; the displacement is patched at
 * `a64_finish` time. `a64_finish` produces the raw `.text` byte stream.
 *
 * ERROR MODEL. The assembler carries a sticky error (like a builder). A method
 * that validates an immediate — or `a64_bind` on a re-bound label — latches the
 * first error, which any later call and `a64_finish` propagate; query it with
 * `a64_error`. This mirrors the Rust crate's `Result` at every fallible step
 * while keeping the emit methods `void`.
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef AARCH64_ENCODER_H
#define AARCH64_ENCODER_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint16_t, uint32_t, uint64_t, int32_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Registers (value == 5-bit register index; Sp and Xzr share code 31) ────*/
typedef enum {
    A64_X0 = 0, A64_X1, A64_X2, A64_X3, A64_X4, A64_X5, A64_X6, A64_X7,
    A64_X8, A64_X9, A64_X10, A64_X11, A64_X12, A64_X13, A64_X14, A64_X15,
    A64_X16, A64_X17, A64_X18, A64_X19, A64_X20, A64_X21, A64_X22, A64_X23,
    A64_X24, A64_X25, A64_X26, A64_X27, A64_X28,
    A64_FP = 29,
    A64_LR = 30,
    A64_SP = 31,
    A64_XZR = 31
} A64Reg;

/* ── Condition codes (4-bit) ────────────────────────────────────────────────*/
typedef enum {
    A64_EQ = 0, A64_NE, A64_HS, A64_LO, A64_MI, A64_PL, A64_VS, A64_VC,
    A64_HI, A64_LS, A64_GE, A64_LT, A64_GT, A64_LE, A64_AL
} A64Cond;

/* ── Status / errors ────────────────────────────────────────────────────────*/
typedef enum {
    A64_OK = 0,
    A64_ERR_UNBOUND_LABEL,
    A64_ERR_LABEL_ALREADY_BOUND,
    A64_ERR_IMMEDIATE_OUT_OF_RANGE,
    A64_ERR_BRANCH_OUT_OF_RANGE,
    A64_ERR_OUT_OF_MEMORY
} A64Status;

/* An opaque label handle. */
typedef uint32_t A64Label;

typedef struct A64Assembler A64Assembler;

A64Assembler *a64_new(void);          /* NULL on OOM */
void a64_free(A64Assembler *a);
/* The first latched error (A64_OK if none). */
A64Status a64_error(const A64Assembler *a);
/* Words emitted so far (each 4 bytes). */
size_t a64_len_words(const A64Assembler *a);

/* ── Labels ─────────────────────────────────────────────────────────────────*/
A64Label a64_create_label(A64Assembler *a);
void a64_bind(A64Assembler *a, A64Label label);

/* ── Move-immediate ─────────────────────────────────────────────────────────*/
void a64_movz(A64Assembler *a, A64Reg rd, uint16_t imm16, uint8_t hw);
void a64_movk(A64Assembler *a, A64Reg rd, uint16_t imm16, uint8_t hw);
void a64_mov_imm64(A64Assembler *a, A64Reg rd, uint64_t imm);

/* ── Arithmetic / division / logical / shifts / unary (register) ────────────*/
void a64_add(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm);
void a64_sub(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm);
void a64_mul(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm);
void a64_sdiv(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm);
void a64_udiv(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm);
void a64_msub(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm, A64Reg ra);
void a64_and(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm);
void a64_orr(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm);
void a64_eor(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm);
void a64_mvn(A64Assembler *a, A64Reg rd, A64Reg rm);
void a64_lsl_reg(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm);
void a64_lsr_reg(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm);
void a64_asr_reg(A64Assembler *a, A64Reg rd, A64Reg rn, A64Reg rm);
void a64_neg(A64Assembler *a, A64Reg rd, A64Reg rm);

/* Returns the word index of the emitted placeholder. */
size_t a64_adrp_placeholder(A64Assembler *a, A64Reg rd);

/* ── Arithmetic (immediate, 12-bit) / compare ───────────────────────────────*/
void a64_add_imm(A64Assembler *a, A64Reg rd, A64Reg rn, uint32_t imm12);
void a64_sub_imm(A64Assembler *a, A64Reg rd, A64Reg rn, uint32_t imm12);
void a64_cmp(A64Assembler *a, A64Reg rn, A64Reg rm);
void a64_cmp_imm(A64Assembler *a, A64Reg rn, uint32_t imm12);

/* ── Memory (scaled unsigned offset) ────────────────────────────────────────*/
void a64_ldr(A64Assembler *a, A64Reg rt, A64Reg rn, uint32_t imm);
void a64_str(A64Assembler *a, A64Reg rt, A64Reg rn, uint32_t imm);
void a64_ldr_d(A64Assembler *a, A64Reg dt, A64Reg rn, uint32_t imm);
void a64_str_d(A64Assembler *a, A64Reg dt, A64Reg rn, uint32_t imm);
void a64_ldrb(A64Assembler *a, A64Reg rt, A64Reg rn, uint32_t imm);
void a64_strb(A64Assembler *a, A64Reg rt, A64Reg rn, uint32_t imm);
void a64_strb_pre_neg1(A64Assembler *a, A64Reg wt, A64Reg rn);

/* ── Scalar double-precision FP + int⇄real conversions ──────────────────────*/
void a64_fadd(A64Assembler *a, A64Reg dd, A64Reg dn, A64Reg dm);
void a64_fsub(A64Assembler *a, A64Reg dd, A64Reg dn, A64Reg dm);
void a64_fmul(A64Assembler *a, A64Reg dd, A64Reg dn, A64Reg dm);
void a64_fdiv(A64Assembler *a, A64Reg dd, A64Reg dn, A64Reg dm);
void a64_fcmp(A64Assembler *a, A64Reg dn, A64Reg dm);
void a64_scvtf(A64Assembler *a, A64Reg dd, A64Reg xn);
void a64_fcvtzs(A64Assembler *a, A64Reg xd, A64Reg dn);
void a64_frintm(A64Assembler *a, A64Reg dd, A64Reg dn);
void a64_fsqrt(A64Assembler *a, A64Reg dd, A64Reg dn);

/* ── STP / LDP (7-bit signed scaled imm) ────────────────────────────────────*/
void a64_stp_pre(A64Assembler *a, A64Reg rt1, A64Reg rt2, A64Reg rn, int32_t imm);
void a64_ldp_post(A64Assembler *a, A64Reg rt1, A64Reg rt2, A64Reg rn, int32_t imm);

/* ── Branches / misc ────────────────────────────────────────────────────────*/
void a64_b(A64Assembler *a, A64Label target);
void a64_bl(A64Assembler *a, A64Label target);
void a64_b_cond(A64Assembler *a, A64Cond cond, A64Label target);
void a64_blr(A64Assembler *a, A64Reg rn);
void a64_ret(A64Assembler *a);
void a64_cset(A64Assembler *a, A64Reg rd, A64Cond cond);
void a64_cbz(A64Assembler *a, A64Reg rt, A64Label target);
void a64_cbnz(A64Assembler *a, A64Reg rt, A64Label target);
void a64_nop(A64Assembler *a);
void a64_udf(A64Assembler *a, uint16_t imm);
void a64_svc(A64Assembler *a, uint16_t imm);

/* ── Finalisation ───────────────────────────────────────────────────────────*/
/* Resolve branch fix-ups and emit the byte stream into a malloc'd buffer
 * (*out_bytes, free with free) of length *out_len. Returns the sticky error if
 * any (e.g. an unbound label or an out-of-range immediate). */
A64Status a64_finish(A64Assembler *a, uint8_t **out_bytes, size_t *out_len);

#ifdef __cplusplus
}
#endif

#endif /* AARCH64_ENCODER_H */
