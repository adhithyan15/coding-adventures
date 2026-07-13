/* Tests for intel4004-encoder, using the header-only iso_test.h harness (pure
 * ISO). Vectors mirror the Rust crate's doctests. */
#include "iso_test.h"

#include <stdint.h>

#include "intel4004_encoder.h"

int main(void) {
    /* ── constants ─────────────────────────────────────────────────────────*/
    ISO_CHECK_EQ_UINT(INTEL4004_LDM_OPCODE, 0xD0u);
    ISO_CHECK_EQ_UINT(INTEL4004_LD_OPCODE, 0xA0u);
    ISO_CHECK_EQ_UINT(INTEL4004_XCH_OPCODE, 0xB0u);
    ISO_CHECK_EQ_UINT(INTEL4004_JUN_OPCODE, 0x40u);
    ISO_CHECK_EQ_INT(INTEL4004_GP_REGISTER_COUNT, 16);
    ISO_CHECK_EQ_INT(INTEL4004_LDM_MAX, 15);
    ISO_CHECK_EQ_INT(INTEL4004_LDM_MIN_SIGNED, -8);

    /* ── HALT_LOOP (JUN 0x000) ─────────────────────────────────────────────*/
    {
        static const uint8_t halt[2] = {0x40, 0x00};
        ISO_CHECK_MEM_EQ(INTEL4004_HALT_LOOP, halt, 2);
    }

    /* ── single-byte ops (nibble masked) ───────────────────────────────────*/
    ISO_CHECK_EQ_UINT(intel4004_encode_ldm(5), 0xD5u);
    ISO_CHECK_EQ_UINT(intel4004_encode_ldm(0x15), 0xD5u); /* masked to 5 */
    ISO_CHECK_EQ_UINT(intel4004_encode_ld(3), 0xA3u);
    ISO_CHECK_EQ_UINT(intel4004_encode_xch(3), 0xB3u);
    ISO_CHECK_EQ_UINT(intel4004_encode_xch(0xF3), 0xB3u); /* masked to 3 */
    ISO_CHECK_EQ_UINT(intel4004_encode_ldm(15), 0xDFu);

    /* ── JUN 2-byte (12-bit address) ───────────────────────────────────────*/
    {
        uint8_t w[2];
        static const uint8_t jun_abc[2] = {0x4A, 0xBC};
        static const uint8_t jun_zero[2] = {0x40, 0x00};
        intel4004_encode_jun(0xABC, w);
        ISO_CHECK_MEM_EQ(w, jun_abc, 2);
        intel4004_encode_jun(0x1ABC, w); /* masked to 0xABC */
        ISO_CHECK_MEM_EQ(w, jun_abc, 2);
        intel4004_encode_jun(0, w);
        ISO_CHECK_MEM_EQ(w, jun_zero, 2);
    }

    return ISO_TEST_RESULT();
}
