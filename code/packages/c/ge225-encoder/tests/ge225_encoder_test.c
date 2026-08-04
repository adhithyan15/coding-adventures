/* Tests for ge225-encoder, using the header-only iso_test.h harness (pure ISO).
 * Vectors mirror the Rust crate's doctests. */
#include "iso_test.h"

#include <stdint.h>

#include "ge225_encoder.h"

int main(void) {
    uint8_t w[3];

    /* ── canonical words ───────────────────────────────────────────────────*/
    {
        static const uint8_t hlt[3] = {0x00, 0x00, 0x00};
        static const uint8_t rts[3] = {0x0A, 0x00, 0x00};
        ISO_CHECK_MEM_EQ(GE225_HALT_WORD, hlt, 3);
        ISO_CHECK_MEM_EQ(GE225_RTS_WORD, rts, 3);
    }

    /* ── capacity / opcode constants ───────────────────────────────────────*/
    ISO_CHECK_EQ_INT(GE225_GP_REGISTER_COUNT, 16);
    ISO_CHECK_EQ_INT(GE225_LDA_MAX_SIGNED, 32767);
    ISO_CHECK_EQ_INT(GE225_LDA_MIN_SIGNED, -32768);
    ISO_CHECK_EQ_INT(GE225_LDA_MAX_UNSIGNED, 65535);
    ISO_CHECK_EQ_UINT(GE225_LDA_OPCODE_NIBBLE, 0x1u);
    ISO_CHECK_EQ_UINT(GE225_BMI_OPCODE_NIBBLE, 0xBu);

    /* ── LDA immediate ─────────────────────────────────────────────────────*/
    {
        static const uint8_t lda5[3] = {0x01, 0x00, 0x05};
        static const uint8_t lda1234[3] = {0x01, 0x12, 0x34};
        ge225_encode_lda(5, w);
        ISO_CHECK_MEM_EQ(w, lda5, 3);
        ge225_encode_lda(0x1234, w);
        ISO_CHECK_MEM_EQ(w, lda1234, 3);
    }

    /* ── register ops (r masked to 4 bits) ─────────────────────────────────*/
    {
        static const uint8_t sta3[3] = {0x02, 0x00, 0x03};
        static const uint8_t ld3[3] = {0x03, 0x00, 0x03};
        static const uint8_t add3[3] = {0x04, 0x00, 0x03};
        static const uint8_t sub3[3] = {0x05, 0x00, 0x03};
        ge225_encode_sta(3, w);
        ISO_CHECK_MEM_EQ(w, sta3, 3);
        ge225_encode_sta(0x13, w); /* masked to 0x3 */
        ISO_CHECK_MEM_EQ(w, sta3, 3);
        ge225_encode_ld(3, w);
        ISO_CHECK_MEM_EQ(w, ld3, 3);
        ge225_encode_add(3, w);
        ISO_CHECK_MEM_EQ(w, add3, 3);
        ge225_encode_sub(0xF3, w); /* masked to 0x3 */
        ISO_CHECK_MEM_EQ(w, sub3, 3);
    }

    /* ── branches (16-bit big-endian address) ──────────────────────────────*/
    {
        static const uint8_t br[3] = {0x06, 0xAB, 0xCD};
        static const uint8_t bnz[3] = {0x07, 0xAB, 0xCD};
        static const uint8_t bz[3] = {0x08, 0xAB, 0xCD};
        static const uint8_t bmi[3] = {0x0B, 0xAB, 0xCD};
        static const uint8_t jsr[3] = {0x09, 0xAB, 0xCD};
        ge225_encode_br(0xABCD, w);
        ISO_CHECK_MEM_EQ(w, br, 3);
        ge225_encode_bnz(0xABCD, w);
        ISO_CHECK_MEM_EQ(w, bnz, 3);
        ge225_encode_bz(0xABCD, w);
        ISO_CHECK_MEM_EQ(w, bz, 3);
        ge225_encode_bmi(0xABCD, w);
        ISO_CHECK_MEM_EQ(w, bmi, 3);
        ge225_encode_jsr(0xABCD, w);
        ISO_CHECK_MEM_EQ(w, jsr, 3);
    }

    /* ── decode is the inverse (strips the top nibble of byte 0) ───────────*/
    {
        uint8_t op;
        uint16_t payload;
        ge225_encode_lda(0x1234, w);
        ge225_decode_word(w, &op, &payload);
        ISO_CHECK(op == GE225_LDA_OPCODE_NIBBLE && payload == 0x1234);
        ge225_encode_bmi(0xFFFF, w);
        ge225_decode_word(w, &op, &payload);
        ISO_CHECK(op == GE225_BMI_OPCODE_NIBBLE && payload == 0xFFFF);
        /* high nibble of byte 0 is ignored on decode */
        {
            static const uint8_t dirty[3] = {0xF6, 0x00, 0x2A};
            ge225_decode_word(dirty, &op, &payload);
            ISO_CHECK(op == GE225_BR_OPCODE_NIBBLE && payload == 0x002A);
        }
    }

    return ISO_TEST_RESULT();
}
