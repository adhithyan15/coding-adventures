// Tests for intel8008-encoder, using the header-only iso_test.h harness.
// Vectors mirror the Rust crate's doctests, plus address-masking cases.
#include "iso_test.h"

#include <array>
#include <cstdint>

#include "intel8008_encoder.hpp"

namespace i8 = ca::intel8008_encoder;
using B2 = std::array<std::uint8_t, 2>;
using B3 = std::array<std::uint8_t, 3>;

int main() {
    // ── constants ──────────────────────────────────────────────────────────
    ISO_CHECK_EQ_UINT(i8::kHlt, 0x76u);
    ISO_CHECK_EQ_UINT(i8::kRet, 0x07u);
    ISO_CHECK_EQ_UINT(i8::kMviA, 0x3Eu);
    ISO_CHECK_EQ_UINT(i8::kJmp, 0x7Cu);
    ISO_CHECK_EQ_UINT(i8::kCal, 0x7Eu);
    ISO_CHECK_EQ_INT(i8::kGpRegisterCount, 7);
    ISO_CHECK_EQ_UINT(i8::kMviMax, 255u);

    // ── MVI A, n ───────────────────────────────────────────────────────────
    ISO_CHECK((i8::encode_mvi_a(42) == B2{0x3E, 0x2A}));
    ISO_CHECK((i8::encode_mvi_a(0) == B2{0x3E, 0x00}));
    ISO_CHECK((i8::encode_mvi_a(255) == B2{0x3E, 0xFF}));

    // ── JMP addr (14-bit, low byte first, high 6 bits) ─────────────────────
    ISO_CHECK((i8::encode_jmp(0x000A) == B3{0x7C, 0x0A, 0x00}));
    ISO_CHECK((i8::encode_jmp(0x0100) == B3{0x7C, 0x00, 0x01}));
    ISO_CHECK((i8::encode_jmp(0x3FFF) == B3{0x7C, 0xFF, 0x3F}));
    ISO_CHECK((i8::encode_jmp(0xFFFF) == B3{0x7C, 0xFF, 0x3F}));  // masked

    // ── CAL addr ───────────────────────────────────────────────────────────
    ISO_CHECK((i8::encode_cal(0x0100) == B3{0x7E, 0x00, 0x01}));
    ISO_CHECK((i8::encode_cal(0x1234) == B3{0x7E, 0x34, 0x12}));

    // ── constexpr usable in a constant expression ──────────────────────────
    constexpr auto j = i8::encode_jmp(0x000A);
    static_assert(j[0] == 0x7C && j[1] == 0x0A && j[2] == 0x00, "constexpr");

    return ISO_TEST_RESULT();
}
