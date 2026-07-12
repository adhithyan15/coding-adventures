// Tests for the C++ armv7-encoder, using the header-only iso_test.h harness
// (pure ISO). Every expected value is an exact ARMv7-A (A32) machine word.
#include "iso_test.h"

#include <cstdint>

#include "armv7_encoder.hpp"

namespace a7 = ca::armv7;

int main() {
    // ── canonical word constants (also usable in constant expressions) ───
    static_assert(a7::BX_LR == 0xE12FFF1Eu, "BX LR");
    static_assert(a7::BKPT == 0xE12FFF7Fu, "BKPT #0");
    static_assert(a7::GP_REGISTER_COUNT == 12, "GP register count");
    static_assert(a7::MOV_IMM_MAX == 255u, "MOV imm max");
    ISO_CHECK_EQ_UINT(a7::MOV_IMM_R0_BASE, 0xE3A00000u);
    ISO_CHECK_EQ_UINT(a7::MOV_REG_BASE, 0xE1A00000u);

    // ── MOV Rd, #imm8 (the doc vector is a compile-time check) ───────────
    static_assert(a7::encode_mov_imm(0, 42) == 0xE3A0002Au, "MOV r0,#42");
    ISO_CHECK_EQ_UINT(a7::encode_mov_imm(0, 0), 0xE3A00000u);
    ISO_CHECK_EQ_UINT(a7::encode_mov_imm(1, 0), 0xE3A01000u);
    ISO_CHECK_EQ_UINT(a7::encode_mov_imm(3, 255), 0xE3A030FFu);
    ISO_CHECK_EQ_UINT(a7::encode_mov_imm(11, 1), 0xE3A0B001u);
    ISO_CHECK_EQ_UINT(a7::encode_mov_imm(0x10, 0), 0xE3A00000u); // rd masked
    ISO_CHECK_EQ_UINT(a7::encode_mov_imm(0x1F, 0), 0xE3A0F000u);

    // ── MOV Rd, Rm ───────────────────────────────────────────────────────
    static_assert(a7::encode_mov_reg(0, 1) == 0xE1A00001u, "MOV r0,r1");
    ISO_CHECK_EQ_UINT(a7::encode_mov_reg(2, 3), 0xE1A02003u);
    ISO_CHECK_EQ_UINT(a7::encode_mov_reg(0, 0), 0xE1A00000u);
    ISO_CHECK_EQ_UINT(a7::encode_mov_reg(11, 11), 0xE1A0B00Bu);
    ISO_CHECK_EQ_UINT(a7::encode_mov_reg(0x1F, 0x1F), 0xE1A0F00Fu); // masked

    return ISO_TEST_RESULT();
}
