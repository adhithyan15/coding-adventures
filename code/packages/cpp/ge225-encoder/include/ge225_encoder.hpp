// ge225_encoder.hpp — pure GE-225 instruction encoder, header-only ISO C++17.
// ===========================================================================
//
// A faithful port of the Rust `ge225-encoder` crate, in namespace
// `ca::ge225_encoder`: the encoding tables for the GE-225 (1959), the General
// Electric mainframe at Dartmouth where Dartmouth BASIC was designed in 1964.
//
// Each 20-bit word is emitted as 3 big-endian bytes (top 4 bits of byte 0 zero):
//   byte 0: 0000 OOOO   (4-bit opcode nibble)
//   byte 1/2: 16-bit immediate / address (for STA/LD/ADD/SUB the low nibble of
//             byte 2 holds the register index)
//
// Opcodes: 0x0 HLT, 0x1 LDA, 0x2 STA (exchange), 0x3 LD, 0x4 ADD, 0x5 SUB,
//          0x6 BR, 0x7 BNZ, 0x8 BZ, 0x9 JSR, 0xA RTS, 0xB BMI.
//
// Pure ISO C++17 — no <cmath>, no compiler extensions.
#ifndef GE225_ENCODER_HPP
#define GE225_ENCODER_HPP

#include <array>
#include <cstdint>
#include <utility>

namespace ca {
namespace ge225_encoder {

// ── Opcode nibbles ────────────────────────────────────────────────────────────
inline constexpr std::uint8_t kLdaOpcodeNibble = 0x1;
inline constexpr std::uint8_t kStaOpcodeNibble = 0x2;
inline constexpr std::uint8_t kLdOpcodeNibble = 0x3;
inline constexpr std::uint8_t kAddOpcodeNibble = 0x4;
inline constexpr std::uint8_t kSubOpcodeNibble = 0x5;
inline constexpr std::uint8_t kBrOpcodeNibble = 0x6;
inline constexpr std::uint8_t kBnzOpcodeNibble = 0x7;
inline constexpr std::uint8_t kBzOpcodeNibble = 0x8;
inline constexpr std::uint8_t kJsrOpcodeNibble = 0x9;
inline constexpr std::uint8_t kRtsOpcodeNibble = 0xA;
inline constexpr std::uint8_t kBmiOpcodeNibble = 0xB;

// ── Canonical word constants ──────────────────────────────────────────────────
inline constexpr std::array<std::uint8_t, 3> kHaltWord = {0x00, 0x00, 0x00};
inline constexpr std::array<std::uint8_t, 3> kRtsWord = {kRtsOpcodeNibble, 0x00,
                                                         0x00};

// ── Capacity constants ────────────────────────────────────────────────────────
inline constexpr std::size_t kGpRegisterCount = 16;
inline constexpr std::int32_t kLdaMaxSigned = 32767;
inline constexpr std::int32_t kLdaMinSigned = -32768;
inline constexpr std::int32_t kLdaMaxUnsigned = 65535;

namespace detail {
inline std::array<std::uint8_t, 3> encode_word(std::uint8_t nibble,
                                               std::uint16_t payload) {
    return {nibble, static_cast<std::uint8_t>((payload >> 8) & 0xFF),
            static_cast<std::uint8_t>(payload & 0xFF)};
}
inline std::array<std::uint8_t, 3> encode_reg(std::uint8_t nibble,
                                              std::uint8_t r) {
    return {nibble, 0x00, static_cast<std::uint8_t>(r & 0x0F)};
}
}  // namespace detail

// ── encode_* helpers ──────────────────────────────────────────────────────────

inline std::array<std::uint8_t, 3> encode_lda(std::uint16_t imm16) {
    return detail::encode_word(kLdaOpcodeNibble, imm16);
}
inline std::array<std::uint8_t, 3> encode_sta(std::uint8_t r) {
    return detail::encode_reg(kStaOpcodeNibble, r);
}
inline std::array<std::uint8_t, 3> encode_ld(std::uint8_t r) {
    return detail::encode_reg(kLdOpcodeNibble, r);
}
inline std::array<std::uint8_t, 3> encode_add(std::uint8_t r) {
    return detail::encode_reg(kAddOpcodeNibble, r);
}
inline std::array<std::uint8_t, 3> encode_sub(std::uint8_t r) {
    return detail::encode_reg(kSubOpcodeNibble, r);
}
inline std::array<std::uint8_t, 3> encode_br(std::uint16_t addr) {
    return detail::encode_word(kBrOpcodeNibble, addr);
}
inline std::array<std::uint8_t, 3> encode_bnz(std::uint16_t addr) {
    return detail::encode_word(kBnzOpcodeNibble, addr);
}
inline std::array<std::uint8_t, 3> encode_bz(std::uint16_t addr) {
    return detail::encode_word(kBzOpcodeNibble, addr);
}
inline std::array<std::uint8_t, 3> encode_bmi(std::uint16_t addr) {
    return detail::encode_word(kBmiOpcodeNibble, addr);
}
inline std::array<std::uint8_t, 3> encode_jsr(std::uint16_t addr) {
    return detail::encode_word(kJsrOpcodeNibble, addr);
}

// ── Decoding ──────────────────────────────────────────────────────────────────

// Decode a 3-byte word into (opcode nibble, 16-bit payload); the top 4 bits of
// byte 0 are stripped.
inline std::pair<std::uint8_t, std::uint16_t> decode_word(
    const std::array<std::uint8_t, 3> &word) {
    std::uint8_t opcode = static_cast<std::uint8_t>(word[0] & 0x0F);
    std::uint16_t payload = static_cast<std::uint16_t>(
        (static_cast<std::uint16_t>(word[1]) << 8) | word[2]);
    return {opcode, payload};
}

}  // namespace ge225_encoder
}  // namespace ca

#endif  // GE225_ENCODER_HPP
