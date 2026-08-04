/*
 * armv7_encoder.h — a pure ARMv7-A (A32) instruction encoder, in pure ISO C17.
 * A faithful port of the Rust `armv7-encoder` crate.
 * ===========================================================================
 *
 * ARMv7-A is the 32-bit ARM instruction set deployed in billions of
 * Cortex-A7/A8/A9-era phone-class SoCs. This encoder knows nothing about IR: it
 * is just canonical instruction-word constants plus typed `encode_*` helpers
 * that take ARM register indices / immediates and return the 32-bit machine
 * word.
 *
 * ABI (AAPCS32): the first integer/pointer argument is in `r0`, the return value
 * in `r0`, and the link register `lr`/`r14` holds the return address.
 *
 * Every value here is an exact ARM A32 encoding (e.g. `MOV r0, #42` is
 * `0xE3A0002A`); the constants and helpers are compile-time / branch-free.
 *
 * PORTABILITY. Pure ISO C17 — no extensions. Builds clean under GCC, Clang, and
 * MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
 */
#ifndef CA_ARMV7_ENCODER_H
#define CA_ARMV7_ENCODER_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Canonical word constants ───────────────────────────────────────────── */

/* `BX LR` — branch and exchange to the link register (the AAPCS32
 * return-from-function instruction). */
#define ARMV7_BX_LR 0xE12FFF1Eu
/* `BKPT #0` — breakpoint trap (a HLT-equivalent in emit-only contexts). */
#define ARMV7_BKPT 0xE12FFF7Fu

/* ── MOV-immediate (data-processing immediate) ──────────────────────────── */

/* Base encoding for `MOV Rd, #imm8` with `Rd = r0`; OR in `(rd << 12) | imm8`. */
#define ARMV7_MOV_IMM_R0_BASE 0xE3A00000u

/* ── MOV-register (data-processing register) ────────────────────────────── */

/* Base encoding for `MOV Rd, Rm`; OR in `(rd << 12) | rm`. */
#define ARMV7_MOV_REG_BASE 0xE1A00000u

/* ── Capacity constants ─────────────────────────────────────────────────── */

/* GP registers in the ABI scratch + saved set (`r0..r11`); r12–r15 are
 * ABI-reserved. */
#define ARMV7_GP_REGISTER_COUNT ((size_t)12)
/* The widest immediate a MOV can carry directly (= 255); wider values need
 * `movw`/`movt` pairs or rotated immediates. */
#define ARMV7_MOV_IMM_MAX 255u

/* ── Encoders ───────────────────────────────────────────────────────────── */

/* Encode `MOV Rd, #imm8` — an 8-bit immediate move. `rd` is masked to 4 bits;
 * `imm8` is 8 bits. Out-of-range register/immediate values are the caller's
 * responsibility (the backend range-checks at lowering time). */
uint32_t armv7_encode_mov_imm(uint8_t rd, uint8_t imm8);

/* Encode `MOV Rd, Rm` — a register-to-register copy. Both indices are masked to
 * 4 bits. */
uint32_t armv7_encode_mov_reg(uint8_t rd, uint8_t rm);

#ifdef __cplusplus
}
#endif

#endif /* CA_ARMV7_ENCODER_H */
