/*
 * ibm704_encoder.c — implementation of the pure-ISO C IBM 704 encoder.
 * ===================================================================
 *
 * See ibm704_encoder.h for the word format. Every routine is a small,
 * total (never-failing) bit operation mirroring the Rust crate exactly.
 */
#include "ibm704_encoder.h"

uint64_t ibm704_encode_instruction(uint16_t opcode, uint16_t address) {
    uint64_t op = (uint64_t)opcode << IBM704_OPCODE_SHIFT;
    uint64_t addr = (uint64_t)address & IBM704_ADDR_MASK;
    return (op | addr) & IBM704_WORD_MASK;
}

uint64_t ibm704_encode_htr(uint16_t address) {
    return ibm704_encode_instruction(IBM704_HTR, address);
}

uint64_t ibm704_encode_cla(uint16_t address) {
    return ibm704_encode_instruction(IBM704_CLA, address);
}

void ibm704_pack_word(uint64_t word, uint8_t out[5]) {
    uint64_t w = word & IBM704_WORD_MASK;
    out[0] = (uint8_t)(w & 0xFF);
    out[1] = (uint8_t)((w >> 8) & 0xFF);
    out[2] = (uint8_t)((w >> 16) & 0xFF);
    out[3] = (uint8_t)((w >> 24) & 0xFF);
    /* Top 4 bits — word bits 32..35; bits 36+ were masked off above. */
    out[4] = (uint8_t)((w >> 32) & 0x0F);
}

const uint8_t IBM704_HTR_HALT_BYTES[5] = {0x00, 0x00, 0x00, 0x80, 0x08};
