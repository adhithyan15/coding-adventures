// Tests for intel4004-encoder, using the header-only iso_test.h harness (pure
// ISO). Vectors mirror the Rust crate's doctests.
#include "iso_test.h"

#include <array>
#include <cstdint>

#include "intel4004_encoder.hpp"

namespace i4 = ca::intel4004_encoder;
using Word = std::array<std::uint8_t, 2>;

int main() {
    // ── constants ────────────────────────────────────────────────────────────
    ISO_CHECK_EQ_UINT(i4::kLdmOpcode, 0xD0u);
    ISO_CHECK_EQ_UINT(i4::kLdOpcode, 0xA0u);
    ISO_CHECK_EQ_UINT(i4::kXchOpcode, 0xB0u);
    ISO_CHECK_EQ_UINT(i4::kJunOpcode, 0x40u);
    ISO_CHECK_EQ_INT(i4::kGpRegisterCount, 16);
    ISO_CHECK_EQ_INT(i4::kLdmMax, 15);
    ISO_CHECK_EQ_INT(i4::kLdmMinSigned, -8);

    // ── HALT_LOOP ────────────────────────────────────────────────────────────
    ISO_CHECK((i4::kHaltLoop == Word{0x40, 0x00}));

    // ── single-byte ops (nibble masked) ──────────────────────────────────────
    ISO_CHECK_EQ_UINT(i4::encode_ldm(5), 0xD5u);
    ISO_CHECK_EQ_UINT(i4::encode_ldm(0x15), 0xD5u);  // masked to 5
    ISO_CHECK_EQ_UINT(i4::encode_ld(3), 0xA3u);
    ISO_CHECK_EQ_UINT(i4::encode_xch(3), 0xB3u);
    ISO_CHECK_EQ_UINT(i4::encode_xch(0xF3), 0xB3u);  // masked to 3
    ISO_CHECK_EQ_UINT(i4::encode_ldm(15), 0xDFu);

    // ── JUN 2-byte (12-bit address) ───────────────────────────────────────────
    ISO_CHECK((i4::encode_jun(0xABC) == Word{0x4A, 0xBC}));
    ISO_CHECK((i4::encode_jun(0x1ABC) == Word{0x4A, 0xBC}));  // masked
    ISO_CHECK((i4::encode_jun(0) == Word{0x40, 0x00}));

    return ISO_TEST_RESULT();
}
