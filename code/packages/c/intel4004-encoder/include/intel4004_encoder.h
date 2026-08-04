/*
 * intel4004_encoder.h — pure Intel 4004 instruction encoder, ISO C17.
 * ==================================================================
 *
 * A faithful port of the Rust `intel4004-encoder` crate: the encoding tables for
 * the Intel 4004 (1971), the world's first commercial microprocessor. Opcode
 * high-nibble constants plus one `encode_*` helper per opcode family; no IR
 * knowledge.
 *
 * ── ISA subset ─────────────────────────────────────────────────────────────
 *   LDM n : 1101 nnnn (0xD0 | n)          1 byte   ACC <- 4-bit immediate n
 *   LD  r : 1010 rrrr (0xA0 | r)          1 byte   ACC <- register r
 *   XCH r : 1011 rrrr (0xB0 | r)          1 byte   ACC <-> register r (store)
 *   JUN a : 0100 aaaa aaaaaaaa            2 bytes  unconditional 12-bit ROM jump
 *
 * The 4004 has no formal HLT; `JUN 0x000` from ROM address 0 loops on itself,
 * which every 4004 simulator treats as halt (`HALT_LOOP = {0x40, 0x00}`).
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef INTEL4004_ENCODER_H
#define INTEL4004_ENCODER_H

#include <stdint.h> /* uint8_t, uint16_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Opcode high nibbles ───────────────────────────────────────────────────*/
#define INTEL4004_LDM_OPCODE 0xD0u
#define INTEL4004_LD_OPCODE 0xA0u
#define INTEL4004_XCH_OPCODE 0xB0u
#define INTEL4004_JUN_OPCODE 0x40u

/* ── Capacity constants ────────────────────────────────────────────────────*/
#define INTEL4004_GP_REGISTER_COUNT 16
#define INTEL4004_LDM_MAX 15
#define INTEL4004_LDM_MIN_SIGNED (-8)

/* Canonical 2-byte "halt loop" — JUN 0x000: {0x40, 0x00}. */
extern const uint8_t INTEL4004_HALT_LOOP[2];

/* ── encode_* helpers ──────────────────────────────────────────────────────*/

/* LDM n — load a 4-bit immediate into the accumulator (n masked to 4 bits). */
uint8_t intel4004_encode_ldm(uint8_t n);
/* LD r — copy register r into the accumulator (r masked to 4 bits). */
uint8_t intel4004_encode_ld(uint8_t r);
/* XCH r — exchange register r with the accumulator (r masked to 4 bits). */
uint8_t intel4004_encode_xch(uint8_t r);
/* JUN addr — unconditional jump; write 2 bytes (addr masked to 12 bits). */
void intel4004_encode_jun(uint16_t addr, uint8_t out[2]);

#ifdef __cplusplus
}
#endif

#endif /* INTEL4004_ENCODER_H */
