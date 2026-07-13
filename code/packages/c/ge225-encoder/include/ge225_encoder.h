/*
 * ge225_encoder.h — pure GE-225 instruction encoder, ISO C17.
 * ==========================================================
 *
 * A faithful port of the Rust `ge225-encoder` crate: the encoding tables for the
 * GE-225 (1959), the General Electric mainframe at Dartmouth College where
 * Dartmouth BASIC was designed in 1964. It owns opcode constants and `encode_*`
 * helpers and nothing else — no IR knowledge — so a simulator, decoder, or
 * fuzzer can reuse it.
 *
 * ── Word packing ───────────────────────────────────────────────────────────
 * Each 20-bit instruction word is emitted as 3 bytes (24 bits), big-endian, the
 * top 4 bits of byte 0 always zero:
 *   byte 0: 0000 OOOO   (4-bit opcode nibble)
 *   byte 1: high 8 bits of immediate / address
 *   byte 2: low 8 bits (for STA/LD/ADD/SUB the low 4 bits hold the register idx)
 *
 * ── Opcodes ────────────────────────────────────────────────────────────────
 *   0x0 HLT   0x1 LDA n   0x2 STA r (exchange)   0x3 LD r    0x4 ADD r
 *   0x5 SUB r 0x6 BR a    0x7 BNZ a  0x8 BZ a    0x9 JSR a   0xA RTS   0xB BMI a
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef GE225_ENCODER_H
#define GE225_ENCODER_H

#include <stdint.h> /* uint8_t, uint16_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Opcode nibbles ────────────────────────────────────────────────────────*/
#define GE225_LDA_OPCODE_NIBBLE 0x1u
#define GE225_STA_OPCODE_NIBBLE 0x2u
#define GE225_LD_OPCODE_NIBBLE 0x3u
#define GE225_ADD_OPCODE_NIBBLE 0x4u
#define GE225_SUB_OPCODE_NIBBLE 0x5u
#define GE225_BR_OPCODE_NIBBLE 0x6u
#define GE225_BNZ_OPCODE_NIBBLE 0x7u
#define GE225_BZ_OPCODE_NIBBLE 0x8u
#define GE225_JSR_OPCODE_NIBBLE 0x9u
#define GE225_RTS_OPCODE_NIBBLE 0xAu
#define GE225_BMI_OPCODE_NIBBLE 0xBu

/* ── Canonical word constants ──────────────────────────────────────────────*/
/* HLT (all zeros) and RTS. */
extern const uint8_t GE225_HALT_WORD[3];
extern const uint8_t GE225_RTS_WORD[3];

/* ── Capacity constants ────────────────────────────────────────────────────*/
#define GE225_GP_REGISTER_COUNT 16
#define GE225_LDA_MAX_SIGNED 32767
#define GE225_LDA_MIN_SIGNED (-32768)
#define GE225_LDA_MAX_UNSIGNED 65535

/* ── encode_* helpers (write 3 big-endian bytes into `out`) ────────────────*/

/* LDA imm16 — load the accumulator with a 16-bit immediate. */
void ge225_encode_lda(uint16_t imm16, uint8_t out[3]);
/* STA r — exchange register r with the accumulator (r masked to 4 bits). */
void ge225_encode_sta(uint8_t r, uint8_t out[3]);
/* LD r — copy register r into the accumulator (r masked to 4 bits). */
void ge225_encode_ld(uint8_t r, uint8_t out[3]);
/* ADD r — ACC += r (r masked to 4 bits). */
void ge225_encode_add(uint8_t r, uint8_t out[3]);
/* SUB r — ACC -= r (r masked to 4 bits). */
void ge225_encode_sub(uint8_t r, uint8_t out[3]);
/* BR / BNZ / BZ / BMI / JSR — branch to a 16-bit big-endian address. */
void ge225_encode_br(uint16_t addr, uint8_t out[3]);
void ge225_encode_bnz(uint16_t addr, uint8_t out[3]);
void ge225_encode_bz(uint16_t addr, uint8_t out[3]);
void ge225_encode_bmi(uint16_t addr, uint8_t out[3]);
void ge225_encode_jsr(uint16_t addr, uint8_t out[3]);

/* ── Decoding (for downstream simulators / decoders) ───────────────────────*/

/* Decode a 3-byte word into its opcode nibble (top 4 bits of byte 0 stripped)
 * and the 16-bit address/immediate payload. */
void ge225_decode_word(const uint8_t word[3], uint8_t *out_opcode,
                       uint16_t *out_payload);

#ifdef __cplusplus
}
#endif

#endif /* GE225_ENCODER_H */
