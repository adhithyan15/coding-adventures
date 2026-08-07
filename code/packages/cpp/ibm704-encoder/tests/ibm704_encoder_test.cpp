// Tests for ibm704-encoder, using the header-only iso_test.h harness (pure ISO).
// Vectors mirror the Rust crate's doctests, including McCarthy's canonical "42"
// program (CLA 42 ; HTR 0).
#include "iso_test.h"

#include <array>
#include <cstdint>

#include "ibm704_encoder.hpp"

namespace ib = ca::ibm704_encoder;

int main() {
    // ── Opcode constants ─────────────────────────────────────────────────────
    ISO_CHECK_EQ_UINT(ib::kHtr, 0420u);  // octal 420 == 272
    ISO_CHECK_EQ_UINT(ib::kCla, 0500u);  // octal 500 == 320
    ISO_CHECK_EQ_UINT(ib::kHtr, 272u);
    ISO_CHECK_EQ_UINT(ib::kCla, 320u);

    // ── Word geometry ────────────────────────────────────────────────────────
    ISO_CHECK_EQ_UINT(ib::kWordBits, 36u);
    ISO_CHECK(ib::kWordMask == 0xFFFFFFFFFull);
    ISO_CHECK_EQ_UINT(ib::kBytesPerWord, 5u);
    ISO_CHECK_EQ_UINT(ib::kAddrBits, 15u);
    ISO_CHECK(ib::kAddrMask == 0x7FFFu);
    ISO_CHECK_EQ_UINT(ib::kOpcodeShift, 27u);

    // ── encode_* 36-bit word values ──────────────────────────────────────────
    {
        std::uint64_t cla_42 = ib::encode_cla(42);
        std::uint64_t htr_0 = ib::encode_htr(0);
        ISO_CHECK((cla_42 & ib::kWordMask) == 0xA0000002Aull);
        ISO_CHECK((htr_0 & ib::kWordMask) == 0x880000000ull);
        ISO_CHECK(ib::encode_instruction(ib::kCla, 42) == cla_42);
        ISO_CHECK(ib::encode_instruction(ib::kHtr, 0) == htr_0);
    }

    // ── Address masking ──────────────────────────────────────────────────────
    {
        std::uint64_t w =
            ib::encode_instruction(ib::kCla, static_cast<std::uint16_t>(0x8000));
        ISO_CHECK((w & ib::kAddrMask) == 0u);
        w = ib::encode_instruction(ib::kCla,
                                   static_cast<std::uint16_t>(0xFFFF));
        ISO_CHECK((w & ib::kAddrMask) == 0x7FFFu);
    }

    // ── 5-byte little-endian packing ─────────────────────────────────────────
    {
        std::array<std::uint8_t, 5> cla_42 = {0x2A, 0x00, 0x00, 0x00, 0x0A};
        std::array<std::uint8_t, 5> htr_0 = {0x00, 0x00, 0x00, 0x80, 0x08};
        ISO_CHECK(ib::pack_word(ib::encode_cla(42)) == cla_42);
        ISO_CHECK(ib::pack_word(ib::encode_htr(0)) == htr_0);
        // pack masks off any stray high bits (bits 36+).
        std::array<std::uint8_t, 5> all_low36 = {0xFF, 0xFF, 0xFF, 0xFF, 0x0F};
        ISO_CHECK(ib::pack_word(0xFFFFFFFFFFFFFFFFull) == all_low36);
    }

    // ── Pre-computed halt sentinel ───────────────────────────────────────────
    {
        std::array<std::uint8_t, 5> expect = {0x00, 0x00, 0x00, 0x80, 0x08};
        ISO_CHECK(ib::kHtrHaltBytes == expect);
        ISO_CHECK(ib::kHtrHaltBytes == ib::pack_word(ib::encode_htr(0)));
    }

    return ISO_TEST_RESULT();
}
