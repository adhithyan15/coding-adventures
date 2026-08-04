/*
 * intel8008_encoder.h — pure Intel 8008 instruction encoder, ISO C17.
 * ===================================================================
 *
 * A faithful port of the Rust `intel8008-encoder` crate: the instruction-
 * encoding tables for the Intel 8008 (1972), the second-generation 8-bit Intel
 * microprocessor. A companion to the ported `intel4004-encoder`.
 *
 *   HLT       : 0x76        (1 byte)   halt (01_110_110)
 *   RET       : 0x07        (1 byte)   return from subroutine
 *   MVI A, n  : 0x3E nn     (2 bytes)  A <- 8-bit immediate n
 *   JMP addr  : 0x7C lo hi  (3 bytes)  unconditional jump, 14-bit address
 *   CAL addr  : 0x7E lo hi  (3 bytes)  call subroutine, 14-bit address
 *
 * The 3-byte address instructions encode the 14-bit address low byte first,
 * then the high 6 bits: [opcode, addr & 0xFF, (addr >> 8) & 0x3F].
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef INTEL8008_ENCODER_H
#define INTEL8008_ENCODER_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint16_t */

#ifdef __cplusplus
extern "C" {
#endif

/* Opcodes. */
#define INTEL8008_HLT ((uint8_t)0x76)
#define INTEL8008_RET ((uint8_t)0x07)
#define INTEL8008_MVI_A ((uint8_t)0x3E)
#define INTEL8008_JMP ((uint8_t)0x7C)
#define INTEL8008_CAL ((uint8_t)0x7E)

/* Capacity constants. */
#define INTEL8008_GP_REGISTER_COUNT ((size_t)7)
#define INTEL8008_MVI_MAX ((uint8_t)255)

/* Encode `MVI A, n` into out[2]: [0x3E, n]. */
void intel8008_encode_mvi_a(uint8_t n, uint8_t out[2]);

/* Encode `JMP addr` into out[3] (14-bit address masked): [0x7C, lo, hi6]. */
void intel8008_encode_jmp(uint16_t addr, uint8_t out[3]);

/* Encode `CAL addr` into out[3] (same address shape as JMP). */
void intel8008_encode_cal(uint16_t addr, uint8_t out[3]);

#ifdef __cplusplus
}
#endif

#endif /* INTEL8008_ENCODER_H */
