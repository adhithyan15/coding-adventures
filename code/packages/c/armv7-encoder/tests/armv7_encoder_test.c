/*
 * Tests for the C armv7-encoder, using the header-only iso_test.h harness (pure
 * ISO). Every expected value is an exact ARMv7-A (A32) machine word — the doc
 * examples from the Rust crate plus additional canonical encodings derived from
 * the ARM ARM (Architecture Reference Manual) field layout.
 */
#include "iso_test.h"

#include <stdint.h>

#include "armv7_encoder.h"

int main(void) {
    /* ── canonical word constants ───────────────────────────────────────── */
    ISO_CHECK_EQ_UINT(ARMV7_BX_LR, 0xE12FFF1Eu); /* BX LR */
    ISO_CHECK_EQ_UINT(ARMV7_BKPT, 0xE12FFF7Fu);  /* BKPT #0 */
    ISO_CHECK_EQ_UINT(ARMV7_MOV_IMM_R0_BASE, 0xE3A00000u);
    ISO_CHECK_EQ_UINT(ARMV7_MOV_REG_BASE, 0xE1A00000u);
    ISO_CHECK_EQ_UINT((unsigned)ARMV7_GP_REGISTER_COUNT, 12u);
    ISO_CHECK_EQ_UINT(ARMV7_MOV_IMM_MAX, 255u);

    /* ── MOV Rd, #imm8 ──────────────────────────────────────────────────── */
    ISO_CHECK_EQ_UINT(armv7_encode_mov_imm(0, 42), 0xE3A0002Au); /* MOV r0,#42 */
    ISO_CHECK_EQ_UINT(armv7_encode_mov_imm(0, 0), 0xE3A00000u);
    ISO_CHECK_EQ_UINT(armv7_encode_mov_imm(1, 0), 0xE3A01000u);  /* MOV r1,#0 */
    ISO_CHECK_EQ_UINT(armv7_encode_mov_imm(3, 255), 0xE3A030FFu); /* MOV r3,#255 */
    ISO_CHECK_EQ_UINT(armv7_encode_mov_imm(11, 1), 0xE3A0B001u); /* MOV r11,#1 */
    /* rd is masked to 4 bits (out-of-range is the caller's problem). */
    ISO_CHECK_EQ_UINT(armv7_encode_mov_imm(0x10, 0), 0xE3A00000u);
    ISO_CHECK_EQ_UINT(armv7_encode_mov_imm(0x1F, 0), 0xE3A0F000u);

    /* ── MOV Rd, Rm ─────────────────────────────────────────────────────── */
    ISO_CHECK_EQ_UINT(armv7_encode_mov_reg(0, 1), 0xE1A00001u); /* MOV r0,r1 */
    ISO_CHECK_EQ_UINT(armv7_encode_mov_reg(2, 3), 0xE1A02003u); /* MOV r2,r3 */
    ISO_CHECK_EQ_UINT(armv7_encode_mov_reg(0, 0), 0xE1A00000u); /* MOV r0,r0 (nop) */
    ISO_CHECK_EQ_UINT(armv7_encode_mov_reg(11, 11), 0xE1A0B00Bu);
    /* both indices are masked to 4 bits. */
    ISO_CHECK_EQ_UINT(armv7_encode_mov_reg(0x1F, 0x1F), 0xE1A0F00Fu);

    return ISO_TEST_RESULT();
}
