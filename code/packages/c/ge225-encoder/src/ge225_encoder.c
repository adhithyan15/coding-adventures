/*
 * ge225_encoder.c — implementation of the pure-ISO C GE-225 encoder.
 * =================================================================
 *
 * See ge225_encoder.h. Each helper is a small, total (never-failing) bit
 * operation mirroring the Rust crate exactly.
 */
#include "ge225_encoder.h"

const uint8_t GE225_HALT_WORD[3] = {0x00, 0x00, 0x00};
const uint8_t GE225_RTS_WORD[3] = {GE225_RTS_OPCODE_NIBBLE, 0x00, 0x00};

/* Pack a nibble + 16-bit payload into 3 big-endian bytes. */
static void encode_word(uint8_t nibble, uint16_t payload, uint8_t out[3]) {
    out[0] = nibble;
    out[1] = (uint8_t)((payload >> 8) & 0xFF);
    out[2] = (uint8_t)(payload & 0xFF);
}
/* Pack a nibble + a 4-bit register index into 3 bytes. */
static void encode_reg(uint8_t nibble, uint8_t r, uint8_t out[3]) {
    out[0] = nibble;
    out[1] = 0x00;
    out[2] = (uint8_t)(r & 0x0F);
}

void ge225_encode_lda(uint16_t imm16, uint8_t out[3]) {
    encode_word(GE225_LDA_OPCODE_NIBBLE, imm16, out);
}
void ge225_encode_sta(uint8_t r, uint8_t out[3]) {
    encode_reg(GE225_STA_OPCODE_NIBBLE, r, out);
}
void ge225_encode_ld(uint8_t r, uint8_t out[3]) {
    encode_reg(GE225_LD_OPCODE_NIBBLE, r, out);
}
void ge225_encode_add(uint8_t r, uint8_t out[3]) {
    encode_reg(GE225_ADD_OPCODE_NIBBLE, r, out);
}
void ge225_encode_sub(uint8_t r, uint8_t out[3]) {
    encode_reg(GE225_SUB_OPCODE_NIBBLE, r, out);
}
void ge225_encode_br(uint16_t addr, uint8_t out[3]) {
    encode_word(GE225_BR_OPCODE_NIBBLE, addr, out);
}
void ge225_encode_bnz(uint16_t addr, uint8_t out[3]) {
    encode_word(GE225_BNZ_OPCODE_NIBBLE, addr, out);
}
void ge225_encode_bz(uint16_t addr, uint8_t out[3]) {
    encode_word(GE225_BZ_OPCODE_NIBBLE, addr, out);
}
void ge225_encode_bmi(uint16_t addr, uint8_t out[3]) {
    encode_word(GE225_BMI_OPCODE_NIBBLE, addr, out);
}
void ge225_encode_jsr(uint16_t addr, uint8_t out[3]) {
    encode_word(GE225_JSR_OPCODE_NIBBLE, addr, out);
}

void ge225_decode_word(const uint8_t word[3], uint8_t *out_opcode,
                       uint16_t *out_payload) {
    *out_opcode = (uint8_t)(word[0] & 0x0F);
    *out_payload = (uint16_t)(((uint16_t)word[1] << 8) | (uint16_t)word[2]);
}
