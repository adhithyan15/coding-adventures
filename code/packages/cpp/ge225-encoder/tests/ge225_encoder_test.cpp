// Tests for ge225-encoder, using the header-only iso_test.h harness (pure ISO).
// Vectors mirror the Rust crate's doctests.
#include "iso_test.h"

#include <array>
#include <cstdint>

#include "ge225_encoder.hpp"

namespace ge = ca::ge225_encoder;
using Word = std::array<std::uint8_t, 3>;

int main() {
    // ── canonical words ──────────────────────────────────────────────────────
    ISO_CHECK((ge::kHaltWord == Word{0x00, 0x00, 0x00}));
    ISO_CHECK((ge::kRtsWord == Word{0x0A, 0x00, 0x00}));

    // ── constants ────────────────────────────────────────────────────────────
    ISO_CHECK_EQ_INT(ge::kGpRegisterCount, 16);
    ISO_CHECK_EQ_INT(ge::kLdaMaxSigned, 32767);
    ISO_CHECK_EQ_INT(ge::kLdaMinSigned, -32768);
    ISO_CHECK_EQ_INT(ge::kLdaMaxUnsigned, 65535);
    ISO_CHECK_EQ_UINT(ge::kLdaOpcodeNibble, 0x1u);
    ISO_CHECK_EQ_UINT(ge::kBmiOpcodeNibble, 0xBu);

    // ── LDA immediate ────────────────────────────────────────────────────────
    ISO_CHECK((ge::encode_lda(5) == Word{0x01, 0x00, 0x05}));
    ISO_CHECK((ge::encode_lda(0x1234) == Word{0x01, 0x12, 0x34}));

    // ── register ops (r masked to 4 bits) ────────────────────────────────────
    ISO_CHECK((ge::encode_sta(3) == Word{0x02, 0x00, 0x03}));
    ISO_CHECK((ge::encode_sta(0x13) == Word{0x02, 0x00, 0x03}));  // masked
    ISO_CHECK((ge::encode_ld(3) == Word{0x03, 0x00, 0x03}));
    ISO_CHECK((ge::encode_add(3) == Word{0x04, 0x00, 0x03}));
    ISO_CHECK((ge::encode_sub(0xF3) == Word{0x05, 0x00, 0x03}));  // masked

    // ── branches ─────────────────────────────────────────────────────────────
    ISO_CHECK((ge::encode_br(0xABCD) == Word{0x06, 0xAB, 0xCD}));
    ISO_CHECK((ge::encode_bnz(0xABCD) == Word{0x07, 0xAB, 0xCD}));
    ISO_CHECK((ge::encode_bz(0xABCD) == Word{0x08, 0xAB, 0xCD}));
    ISO_CHECK((ge::encode_bmi(0xABCD) == Word{0x0B, 0xAB, 0xCD}));
    ISO_CHECK((ge::encode_jsr(0xABCD) == Word{0x09, 0xAB, 0xCD}));

    // ── decode is the inverse ────────────────────────────────────────────────
    {
        auto [op, payload] = ge::decode_word(ge::encode_lda(0x1234));
        ISO_CHECK(op == ge::kLdaOpcodeNibble && payload == 0x1234);
    }
    {
        auto [op, payload] = ge::decode_word(ge::encode_bmi(0xFFFF));
        ISO_CHECK(op == ge::kBmiOpcodeNibble && payload == 0xFFFF);
    }
    {
        // high nibble of byte 0 is ignored on decode
        auto [op, payload] = ge::decode_word(Word{0xF6, 0x00, 0x2A});
        ISO_CHECK(op == ge::kBrOpcodeNibble && payload == 0x002A);
    }

    return ISO_TEST_RESULT();
}
