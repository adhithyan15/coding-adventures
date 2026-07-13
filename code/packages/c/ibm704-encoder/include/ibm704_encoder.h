/*
 * ibm704_encoder.h — pure IBM 704 instruction encoder, ISO C17.
 * ============================================================
 *
 * A faithful port of the Rust `ibm704-encoder` crate: an encoder for the
 * IBM 704 (1954) — the vacuum-tube mainframe on which John McCarthy and his MIT
 * students first ran Lisp in 1959.
 *
 * ── The Lisp connection ────────────────────────────────────────────────────
 * `CAR` and `CDR`, Lisp's two universal accessors, were literally IBM 704
 * instruction-word field names:
 *   CAR = Contents of the Address part of Register
 *   CDR = Contents of the Decrement part of Register
 * The 704's 36-bit word split into prefix / decrement / tag / address fields,
 * and a cons cell fit one per word; `(CAR x)` took the address half, `(CDR x)`
 * the decrement half. The names stuck.
 *
 * ── Word format (idealised, v0.1.0) ────────────────────────────────────────
 * A simplified layout sufficient for a minimal McCarthy-Lisp compile target:
 *   bits 35..27 (9)  opcode   (e.g. HTR=0o420, CLA=0o500)
 *   bits 26..15 (12) zero     (tag + decrement + unused; not used yet)
 *   bits 14..0  (15) address  (15-bit address, <= 32 K words)
 *
 * ── Wire format — 5 bytes per word ─────────────────────────────────────────
 * 36 bits don't divide evenly into 8, so each word packs into 5 bytes (40 bits,
 * 4 wasted), low byte first, with the top byte's high 4 bits always zero.
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef IBM704_ENCODER_H
#define IBM704_ENCODER_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint16_t, uint64_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── Opcodes ───────────────────────────────────────────────────────────────*/

/* HTR Y — Halt and TransfeR. Opcode 0o420. Stops the CPU, parking the program
 * counter at address Y. `HTR 0` is the canonical jump-to-self halt sentinel. */
#define IBM704_HTR ((uint16_t)0420)
/* CLA Y — CLear accumulator and Add memory at Y (AC <- memory[Y]). Opcode
 * 0o500. Used to materialise a 15-bit immediate into the accumulator. */
#define IBM704_CLA ((uint16_t)0500)

/* ── Word geometry ─────────────────────────────────────────────────────────*/

#define IBM704_WORD_BITS ((uint32_t)36)
#define IBM704_WORD_MASK (((uint64_t)1 << IBM704_WORD_BITS) - 1)
#define IBM704_BYTES_PER_WORD ((size_t)5)
#define IBM704_ADDR_BITS ((uint32_t)15)
#define IBM704_ADDR_MASK (((uint64_t)1 << IBM704_ADDR_BITS) - 1)
/* Bit position the 9-bit opcode starts at within a 36-bit word. */
#define IBM704_OPCODE_SHIFT ((uint32_t)27)

/* ── encode_* helpers ──────────────────────────────────────────────────────*/

/* Encode `<opcode> <address>` into a 36-bit instruction word. The opcode
 * occupies bits 35..27; the address occupies bits 14..0 (out-of-range address
 * bits are masked off, never an error). */
uint64_t ibm704_encode_instruction(uint16_t opcode, uint16_t address);
/* Encode `HTR Y` — halt and transfer to Y. */
uint64_t ibm704_encode_htr(uint16_t address);
/* Encode `CLA Y` — clear-and-add: AC <- memory[Y]. */
uint64_t ibm704_encode_cla(uint16_t address);

/* ── Wire format ───────────────────────────────────────────────────────────*/

/* Pack a 36-bit word into 5 bytes, low byte first, into `out`. Word bits 0..7
 * land in out[0], 8..15 in out[1], 16..23 in out[2], 24..31 in out[3], and
 * 32..35 in the LOW nibble of out[4] (out[4]'s high 4 bits are always zero). */
void ibm704_pack_word(uint64_t word, uint8_t out[5]);

/* The pre-computed 5-byte packing of the canonical `HTR 0` halt sentinel:
 * {0x00, 0x00, 0x00, 0x80, 0x08}. Every emitted program ends with these. */
extern const uint8_t IBM704_HTR_HALT_BYTES[5];

#ifdef __cplusplus
}
#endif

#endif /* IBM704_ENCODER_H */
