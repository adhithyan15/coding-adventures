/*
 * x86_64_encoder.h — x86-64 (AMD64) instruction encoder, pure ISO C17.
 * ===================================================================
 *
 * A faithful port of the Rust `x86_64-encoder` crate: a stream-style assembler
 * that produces little-endian x86-64 machine-code byte streams in 64-bit (long)
 * mode — the bottom of a CIR → native-code lowering.
 *
 * Each `x64_*` call emits one logical instruction (1–15 bytes). Branches
 * reference an `X64Label` bound to a later byte; the rel32 displacement is
 * patched at `x64_finish` time. Cross-function / runtime references are recorded
 * as external relocations (queryable via x64_external_reloc_*).
 *
 * V1 "always-long-form" policy: branches use rel32, memory operands use disp32.
 * Encodings follow the Intel SDM Vol. 2 / AMD64 APM Vol. 3.
 *
 * ERROR MODEL. The assembler carries a sticky error (like a builder): x64_bind
 * on a re-bound label latches it, and x64_finish surfaces an unbound-label /
 * out-of-range branch. Query with x64_error.
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef X86_64_ENCODER_H
#define X86_64_ENCODER_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint32_t, uint64_t, int32_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Registers (value == 4-bit register code) ───────────────────────────────*/
typedef enum {
    X64_RAX = 0, X64_RCX = 1, X64_RDX = 2, X64_RBX = 3,
    X64_RSP = 4, X64_RBP = 5, X64_RSI = 6, X64_RDI = 7,
    X64_R8 = 8, X64_R9 = 9, X64_R10 = 10, X64_R11 = 11,
    X64_R12 = 12, X64_R13 = 13, X64_R14 = 14, X64_R15 = 15
} X64Reg;

/* ── Condition codes (4-bit tttn) ───────────────────────────────────────────*/
typedef enum {
    X64_O = 0x0, X64_NO = 0x1, X64_B = 0x2, X64_AE = 0x3,
    X64_E = 0x4, X64_NE = 0x5, X64_BE = 0x6, X64_A = 0x7,
    X64_S = 0x8, X64_NS = 0x9, X64_P = 0xA, X64_NP = 0xB,
    X64_L = 0xC, X64_GE = 0xD, X64_LE = 0xE, X64_G = 0xF
} X64Cond;

typedef enum {
    X64_RELOC_PLT_REL32,
    X64_RELOC_PC_REL32,
    X64_RELOC_GOT_PC_REL32
} X64RelocKind;

typedef enum {
    X64_OK = 0,
    X64_ERR_UNBOUND_LABEL,
    X64_ERR_LABEL_ALREADY_BOUND,
    X64_ERR_BRANCH_OUT_OF_RANGE,
    X64_ERR_OUT_OF_MEMORY
} X64Status;

typedef uint32_t X64Label;

typedef struct X64Assembler X64Assembler;

X64Assembler *x64_new(void); /* NULL on OOM */
void x64_free(X64Assembler *a);
X64Status x64_error(const X64Assembler *a);
size_t x64_len(const X64Assembler *a);

/* ── Labels ─────────────────────────────────────────────────────────────────*/
X64Label x64_create_label(X64Assembler *a);
void x64_bind(X64Assembler *a, X64Label label);

/* ── MOV family ─────────────────────────────────────────────────────────────*/
void x64_mov_r64_r64(X64Assembler *a, X64Reg dst, X64Reg src);
void x64_mov_r64_imm32(X64Assembler *a, X64Reg dst, int32_t imm);
void x64_mov_r64_imm64(X64Assembler *a, X64Reg dst, uint64_t imm);
void x64_mov_r64_mem(X64Assembler *a, X64Reg dst, X64Reg base, int32_t disp);
void x64_mov_mem_r64(X64Assembler *a, X64Reg base, int32_t disp, X64Reg src);
void x64_lea_rip_rel(X64Assembler *a, X64Reg dst, const char *symbol,
                     X64RelocKind kind);

/* ── Arithmetic ─────────────────────────────────────────────────────────────*/
void x64_add(X64Assembler *a, X64Reg dst, X64Reg src);
void x64_sub(X64Assembler *a, X64Reg dst, X64Reg src);
void x64_imul(X64Assembler *a, X64Reg dst, X64Reg src);
void x64_idiv(X64Assembler *a, X64Reg divisor);
void x64_div(X64Assembler *a, X64Reg divisor);
void x64_cqo(X64Assembler *a);
void x64_add_imm32(X64Assembler *a, X64Reg dst, int32_t imm);
void x64_sub_imm32(X64Assembler *a, X64Reg dst, int32_t imm);
void x64_neg(X64Assembler *a, X64Reg dst);

/* ── Logical ────────────────────────────────────────────────────────────────*/
void x64_and(X64Assembler *a, X64Reg dst, X64Reg src);
void x64_or(X64Assembler *a, X64Reg dst, X64Reg src);
void x64_xor(X64Assembler *a, X64Reg dst, X64Reg src);
void x64_test(X64Assembler *a, X64Reg lhs, X64Reg rhs);
void x64_not(X64Assembler *a, X64Reg dst);

/* ── Shifts ─────────────────────────────────────────────────────────────────*/
void x64_shl_cl(X64Assembler *a, X64Reg dst);
void x64_shr_cl(X64Assembler *a, X64Reg dst);
void x64_sar_cl(X64Assembler *a, X64Reg dst);
void x64_shl_imm8(X64Assembler *a, X64Reg dst, uint8_t imm);
void x64_shr_imm8(X64Assembler *a, X64Reg dst, uint8_t imm);
void x64_sar_imm8(X64Assembler *a, X64Reg dst, uint8_t imm);

/* ── Compare + set ──────────────────────────────────────────────────────────*/
void x64_cmp(X64Assembler *a, X64Reg lhs, X64Reg rhs);
void x64_cmp_imm32(X64Assembler *a, X64Reg lhs, int32_t imm);
void x64_setcc(X64Assembler *a, X64Cond cond, X64Reg dst);
void x64_movzx_r64_r8(X64Assembler *a, X64Reg dst, X64Reg src);
/* Precondition: base must not be RSP/R12 (low3==4) or RBP/R13 (low3==5). */
void x64_movzx_r64_byte_at(X64Assembler *a, X64Reg dst, X64Reg base);
void x64_mov_byte_at_r8(X64Assembler *a, X64Reg base, X64Reg src);

/* ── SSE2 scalar double + conversions ───────────────────────────────────────*/
void x64_movsd_load(X64Assembler *a, X64Reg dst_xmm, X64Reg base, int32_t disp);
void x64_movsd_store(X64Assembler *a, X64Reg base, int32_t disp, X64Reg src_xmm);
void x64_addsd(X64Assembler *a, X64Reg dst, X64Reg src);
void x64_subsd(X64Assembler *a, X64Reg dst, X64Reg src);
void x64_mulsd(X64Assembler *a, X64Reg dst, X64Reg src);
void x64_divsd(X64Assembler *a, X64Reg dst, X64Reg src);
void x64_ucomisd(X64Assembler *a, X64Reg lhs, X64Reg rhs);
void x64_cvtsi2sd(X64Assembler *a, X64Reg xmm_dst, X64Reg gpr_src);
void x64_cvttsd2si(X64Assembler *a, X64Reg gpr_dst, X64Reg xmm_src);
void x64_roundsd(X64Assembler *a, X64Reg xmm_dst, X64Reg xmm_src, uint8_t imm8);
void x64_sqrtsd(X64Assembler *a, X64Reg xmm_dst, X64Reg xmm_src);

/* ── Stack ──────────────────────────────────────────────────────────────────*/
void x64_push(X64Assembler *a, X64Reg src);
void x64_pop(X64Assembler *a, X64Reg dst);

/* ── Control flow / misc ────────────────────────────────────────────────────*/
void x64_jmp(X64Assembler *a, X64Label target);
void x64_jcc(X64Assembler *a, X64Cond cond, X64Label target);
void x64_call_rel32(X64Assembler *a, const char *symbol, X64RelocKind kind);
void x64_call_label(X64Assembler *a, X64Label target);
void x64_call_r64(X64Assembler *a, X64Reg target);
void x64_ret(X64Assembler *a);
void x64_nop(X64Assembler *a);
void x64_int3(X64Assembler *a);
void x64_ud2(X64Assembler *a);

/* ── External relocations (recorded by lea_rip_rel / call_rel32) ────────────*/
size_t x64_external_reloc_count(const X64Assembler *a);
/* Fills the out-params for reloc `i`; returns 1 if present. `symbol` points into
 * the assembler and is valid until x64_free. */
int x64_external_reloc(const X64Assembler *a, size_t i, size_t *patch_offset,
                       const char **symbol, X64RelocKind *kind, int32_t *addend);

/* ── Finalisation ───────────────────────────────────────────────────────────*/
/* Resolve branch fix-ups and emit the byte stream into a malloc'd buffer
 * (*out_bytes, free with free) of length *out_len. Returns the sticky/resolve
 * error if any. */
X64Status x64_finish(X64Assembler *a, uint8_t **out_bytes, size_t *out_len);

#ifdef __cplusplus
}
#endif

#endif /* X86_64_ENCODER_H */
