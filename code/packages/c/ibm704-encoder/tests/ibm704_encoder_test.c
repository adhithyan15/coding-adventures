/* Tests for ibm704-encoder, using the header-only iso_test.h harness (pure ISO).
 * Vectors mirror the Rust crate's doctests, including McCarthy's canonical "42"
 * program (CLA 42 ; HTR 0). */
#include "iso_test.h"

#include <stdint.h>

#include "ibm704_encoder.h"

int main(void) {
    /* ── Opcode constants ──────────────────────────────────────────────────*/
    ISO_CHECK_EQ_UINT(IBM704_HTR, 0420u); /* octal 420 == 272 */
    ISO_CHECK_EQ_UINT(IBM704_CLA, 0500u); /* octal 500 == 320 */
    ISO_CHECK_EQ_UINT(IBM704_HTR, 272u);
    ISO_CHECK_EQ_UINT(IBM704_CLA, 320u);

    /* ── Word geometry ─────────────────────────────────────────────────────*/
    ISO_CHECK_EQ_UINT(IBM704_WORD_BITS, 36u);
    ISO_CHECK(IBM704_WORD_MASK == 0xFFFFFFFFFull);
    ISO_CHECK_EQ_UINT(IBM704_BYTES_PER_WORD, 5u);
    ISO_CHECK_EQ_UINT(IBM704_ADDR_BITS, 15u);
    ISO_CHECK(IBM704_ADDR_MASK == 0x7FFFu);
    ISO_CHECK_EQ_UINT(IBM704_OPCODE_SHIFT, 27u);

    /* ── encode_* 36-bit word values ───────────────────────────────────────*/
    {
        uint64_t cla_42 = ibm704_encode_cla(42);
        uint64_t htr_0 = ibm704_encode_htr(0);
        ISO_CHECK((cla_42 & IBM704_WORD_MASK) == 0xA0000002Aull);
        ISO_CHECK((htr_0 & IBM704_WORD_MASK) == 0x880000000ull);
        /* generic encode_instruction agrees with the named helpers */
        ISO_CHECK(ibm704_encode_instruction(IBM704_CLA, 42) == cla_42);
        ISO_CHECK(ibm704_encode_instruction(IBM704_HTR, 0) == htr_0);
    }

    /* ── Address masking (out-of-range address is masked, never errors) ────*/
    {
        /* 0x8000 exceeds the 15-bit field; only the low 15 bits survive. */
        uint64_t w = ibm704_encode_instruction(IBM704_CLA, (uint16_t)0x8000);
        ISO_CHECK((w & IBM704_ADDR_MASK) == 0u);
        /* 0xFFFF -> low 15 bits = 0x7FFF */
        w = ibm704_encode_instruction(IBM704_CLA, (uint16_t)0xFFFF);
        ISO_CHECK((w & IBM704_ADDR_MASK) == 0x7FFFu);
    }

    /* ── 5-byte little-endian packing ──────────────────────────────────────*/
    {
        uint8_t buf[5];
        static const uint8_t cla_42_bytes[5] = {0x2A, 0x00, 0x00, 0x00, 0x0A};
        static const uint8_t htr_0_bytes[5] = {0x00, 0x00, 0x00, 0x80, 0x08};

        ibm704_pack_word(ibm704_encode_cla(42), buf);
        ISO_CHECK_MEM_EQ(buf, cla_42_bytes, 5);

        ibm704_pack_word(ibm704_encode_htr(0), buf);
        ISO_CHECK_MEM_EQ(buf, htr_0_bytes, 5);

        /* pack masks off any stray high bits (bits 36+). */
        ibm704_pack_word(0xFFFFFFFFFFFFFFFFull, buf);
        {
            static const uint8_t all_low36[5] = {0xFF, 0xFF, 0xFF, 0xFF, 0x0F};
            ISO_CHECK_MEM_EQ(buf, all_low36, 5);
        }
    }

    /* ── Pre-computed halt sentinel ────────────────────────────────────────*/
    {
        static const uint8_t expect[5] = {0x00, 0x00, 0x00, 0x80, 0x08};
        uint8_t buf[5];
        ISO_CHECK_MEM_EQ(IBM704_HTR_HALT_BYTES, expect, 5);
        /* ...and it equals pack_word(encode_htr(0)). */
        ibm704_pack_word(ibm704_encode_htr(0), buf);
        ISO_CHECK_MEM_EQ(IBM704_HTR_HALT_BYTES, buf, 5);
    }

    return ISO_TEST_RESULT();
}
