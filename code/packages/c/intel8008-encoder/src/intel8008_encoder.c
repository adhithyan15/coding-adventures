/*
 * intel8008_encoder.c — pure Intel 8008 instruction encoder, ISO C17.
 * ===================================================================
 *
 * See intel8008_encoder.h. The encoders are branch-free byte layout: a 2-byte
 * immediate load and two 3-byte address instructions that split a 14-bit
 * address into a low byte and a high 6-bit byte.
 */
#include "intel8008_encoder.h"

void intel8008_encode_mvi_a(uint8_t n, uint8_t out[2]) {
    out[0] = INTEL8008_MVI_A;
    out[1] = n;
}

void intel8008_encode_jmp(uint16_t addr, uint8_t out[3]) {
    uint16_t masked = (uint16_t)(addr & 0x3FFF); /* 14 bits */
    out[0] = INTEL8008_JMP;
    out[1] = (uint8_t)(masked & 0xFF);
    out[2] = (uint8_t)((masked >> 8) & 0x3F);
}

void intel8008_encode_cal(uint16_t addr, uint8_t out[3]) {
    uint16_t masked = (uint16_t)(addr & 0x3FFF);
    out[0] = INTEL8008_CAL;
    out[1] = (uint8_t)(masked & 0xFF);
    out[2] = (uint8_t)((masked >> 8) & 0x3F);
}
