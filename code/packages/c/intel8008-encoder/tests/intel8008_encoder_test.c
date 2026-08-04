/*
 * Tests for intel8008-encoder, using the header-only iso_test.h harness.
 * Vectors mirror the Rust crate's doctests, plus address-masking cases.
 */
#include "iso_test.h"

#include <stdint.h>

#include "intel8008_encoder.h"

int main(void) {
    uint8_t b2[2];
    uint8_t b3[3];

    /* ── constants ─────────────────────────────────────────────────────────*/
    ISO_CHECK_EQ_UINT(INTEL8008_HLT, 0x76u);
    ISO_CHECK_EQ_UINT(INTEL8008_RET, 0x07u);
    ISO_CHECK_EQ_UINT(INTEL8008_MVI_A, 0x3Eu);
    ISO_CHECK_EQ_UINT(INTEL8008_JMP, 0x7Cu);
    ISO_CHECK_EQ_UINT(INTEL8008_CAL, 0x7Eu);
    ISO_CHECK_EQ_UINT(INTEL8008_GP_REGISTER_COUNT, 7u);
    ISO_CHECK_EQ_UINT(INTEL8008_MVI_MAX, 255u);

    /* ── MVI A, n ──────────────────────────────────────────────────────────*/
    intel8008_encode_mvi_a(42, b2);
    ISO_CHECK(b2[0] == 0x3E && b2[1] == 0x2A);
    intel8008_encode_mvi_a(0, b2);
    ISO_CHECK(b2[0] == 0x3E && b2[1] == 0x00);
    intel8008_encode_mvi_a(255, b2);
    ISO_CHECK(b2[0] == 0x3E && b2[1] == 0xFF);

    /* ── JMP addr (14-bit, low byte first, high 6 bits) ────────────────────*/
    intel8008_encode_jmp(0x000A, b3);
    ISO_CHECK(b3[0] == 0x7C && b3[1] == 0x0A && b3[2] == 0x00);
    intel8008_encode_jmp(0x0100, b3);
    ISO_CHECK(b3[0] == 0x7C && b3[1] == 0x00 && b3[2] == 0x01);
    intel8008_encode_jmp(0x3FFF, b3);
    ISO_CHECK(b3[0] == 0x7C && b3[1] == 0xFF && b3[2] == 0x3F);
    intel8008_encode_jmp(0xFFFF, b3); /* masked to 14 bits */
    ISO_CHECK(b3[0] == 0x7C && b3[1] == 0xFF && b3[2] == 0x3F);

    /* ── CAL addr ──────────────────────────────────────────────────────────*/
    intel8008_encode_cal(0x0100, b3);
    ISO_CHECK(b3[0] == 0x7E && b3[1] == 0x00 && b3[2] == 0x01);
    intel8008_encode_cal(0x1234, b3);
    ISO_CHECK(b3[0] == 0x7E && b3[1] == 0x34 && b3[2] == 0x12);

    return ISO_TEST_RESULT();
}
