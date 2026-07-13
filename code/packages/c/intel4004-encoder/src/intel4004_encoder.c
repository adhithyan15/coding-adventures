/*
 * intel4004_encoder.c — implementation of the pure-ISO C Intel 4004 encoder.
 * =========================================================================
 *
 * See intel4004_encoder.h. Each helper is a small, total bit operation
 * mirroring the Rust crate exactly.
 */
#include "intel4004_encoder.h"

const uint8_t INTEL4004_HALT_LOOP[2] = {INTEL4004_JUN_OPCODE, 0x00};

uint8_t intel4004_encode_ldm(uint8_t n) {
    return (uint8_t)(INTEL4004_LDM_OPCODE | (n & 0x0Fu));
}
uint8_t intel4004_encode_ld(uint8_t r) {
    return (uint8_t)(INTEL4004_LD_OPCODE | (r & 0x0Fu));
}
uint8_t intel4004_encode_xch(uint8_t r) {
    return (uint8_t)(INTEL4004_XCH_OPCODE | (r & 0x0Fu));
}
void intel4004_encode_jun(uint16_t addr, uint8_t out[2]) {
    uint16_t masked = (uint16_t)(addr & 0x0FFFu);
    out[0] = (uint8_t)(INTEL4004_JUN_OPCODE | ((masked >> 8) & 0x0Fu));
    out[1] = (uint8_t)(masked & 0xFFu);
}
