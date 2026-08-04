/*
 * armv7_encoder.c — implementation of the ARMv7-A (A32) instruction encoder.
 * ===========================================================================
 *
 * Each encoder ORs the register/immediate fields into the opcode-family base
 * word, exactly as the ARM A32 encoding lays them out:
 *   - data-processing MOV places Rd in bits [15:12];
 *   - the 8-bit immediate MOV places imm8 in bits [7:0];
 *   - register MOV places Rm in bits [3:0].
 */
#include "armv7_encoder.h"

uint32_t armv7_encode_mov_imm(uint8_t rd, uint8_t imm8) {
    return ARMV7_MOV_IMM_R0_BASE | ((uint32_t)(rd & 0x0F) << 12) |
           (uint32_t)imm8;
}

uint32_t armv7_encode_mov_reg(uint8_t rd, uint8_t rm) {
    return ARMV7_MOV_REG_BASE | ((uint32_t)(rd & 0x0F) << 12) |
           (uint32_t)(rm & 0x0F);
}
