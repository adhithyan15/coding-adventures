// intel4004_encoder.hpp — pure Intel 4004 encoder, header-only ISO C++17.
// =======================================================================
//
// A faithful port of the Rust `intel4004-encoder` crate, in namespace
// `ca::intel4004_encoder`: the encoding tables for the Intel 4004 (1971), the
// world's first commercial microprocessor.
//
//   LDM n : 0xD0 | n   (1 byte)   ACC <- 4-bit immediate
//   LD  r : 0xA0 | r   (1 byte)   ACC <- register r
//   XCH r : 0xB0 | r   (1 byte)   ACC <-> register r (the 4004's store)
//   JUN a : 0100 aaaa aaaaaaaa (2 bytes)   unconditional 12-bit ROM jump
//
// The 4004 has no formal HLT; `JUN 0x000` at ROM address 0 loops forever, which
// every simulator treats as halt (`kHaltLoop = {0x40, 0x00}`).
//
// Pure ISO C++17 — no <cmath>, no compiler extensions.
#ifndef INTEL4004_ENCODER_HPP
#define INTEL4004_ENCODER_HPP

#include <array>
#include <cstddef>
#include <cstdint>

namespace ca {
namespace intel4004_encoder {

// ── Opcode high nibbles ─────────────────────────────────────────────────────
inline constexpr std::uint8_t kLdmOpcode = 0xD0;
inline constexpr std::uint8_t kLdOpcode = 0xA0;
inline constexpr std::uint8_t kXchOpcode = 0xB0;
inline constexpr std::uint8_t kJunOpcode = 0x40;

// ── Capacity constants ──────────────────────────────────────────────────────
inline constexpr std::size_t kGpRegisterCount = 16;
inline constexpr std::int32_t kLdmMax = 15;
inline constexpr std::int32_t kLdmMinSigned = -8;

// Canonical 2-byte "halt loop" — JUN 0x000.
inline constexpr std::array<std::uint8_t, 2> kHaltLoop = {kJunOpcode, 0x00};

// ── encode_* helpers ────────────────────────────────────────────────────────

inline std::uint8_t encode_ldm(std::uint8_t n) {
    return static_cast<std::uint8_t>(kLdmOpcode | (n & 0x0F));
}
inline std::uint8_t encode_ld(std::uint8_t r) {
    return static_cast<std::uint8_t>(kLdOpcode | (r & 0x0F));
}
inline std::uint8_t encode_xch(std::uint8_t r) {
    return static_cast<std::uint8_t>(kXchOpcode | (r & 0x0F));
}
inline std::array<std::uint8_t, 2> encode_jun(std::uint16_t addr) {
    std::uint16_t masked = static_cast<std::uint16_t>(addr & 0x0FFF);
    return {static_cast<std::uint8_t>(kJunOpcode | ((masked >> 8) & 0x0F)),
            static_cast<std::uint8_t>(masked & 0xFF)};
}

}  // namespace intel4004_encoder
}  // namespace ca

#endif  // INTEL4004_ENCODER_HPP
