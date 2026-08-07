// intel8008_encoder.hpp — pure Intel 8008 encoder, header-only ISO C++17.
// =======================================================================
//
// A faithful port of the Rust `intel8008-encoder` crate, in namespace
// `ca::intel8008_encoder`: the instruction-encoding tables for the Intel 8008
// (1972), the second-generation 8-bit Intel microprocessor. A companion to the
// ported `intel4004-encoder` / `ge225-encoder`.
//
//   HLT       : 0x76           (1 byte)   halt (01_110_110)
//   RET       : 0x07           (1 byte)   return from subroutine
//   MVI A, n  : 0x3E nn        (2 bytes)  A <- 8-bit immediate n
//   JMP addr  : 0x7C lo hi     (3 bytes)  unconditional jump, 14-bit address
//   CAL addr  : 0x7E lo hi     (3 bytes)  call subroutine, 14-bit address
//
// For the 3-byte address instructions the 14-bit address is encoded low byte
// first, then the high 6 bits: `[opcode, addr & 0xFF, (addr >> 8) & 0x3F]`.
//
// Pure ISO C++17 — no <cmath>, no compiler extensions.
#ifndef INTEL8008_ENCODER_HPP
#define INTEL8008_ENCODER_HPP

#include <array>
#include <cstddef>
#include <cstdint>

namespace ca {
namespace intel8008_encoder {

// ── Opcodes ──────────────────────────────────────────────────────────────────
inline constexpr std::uint8_t kHlt = 0x76;
inline constexpr std::uint8_t kRet = 0x07;
inline constexpr std::uint8_t kMviA = 0x3E;
inline constexpr std::uint8_t kJmp = 0x7C;
inline constexpr std::uint8_t kCal = 0x7E;

// ── Capacity constants ───────────────────────────────────────────────────────
// The 8008 exposes 7 GP registers (A, B, C, D, E, H, L).
inline constexpr std::size_t kGpRegisterCount = 7;
// Maximum unsigned 8-bit `MVI A` immediate.
inline constexpr std::uint8_t kMviMax = 255;

// ── encode_* helpers ─────────────────────────────────────────────────────────

// `MVI A, n` → [0x3E, n].
inline constexpr std::array<std::uint8_t, 2> encode_mvi_a(std::uint8_t n) {
    return {kMviA, n};
}

// `JMP addr` → [0x7C, lo8(addr), hi6(addr)]; the 14-bit address is masked.
inline constexpr std::array<std::uint8_t, 3> encode_jmp(std::uint16_t addr) {
    std::uint16_t masked = static_cast<std::uint16_t>(addr & 0x3FFF);
    return {kJmp, static_cast<std::uint8_t>(masked & 0xFF),
            static_cast<std::uint8_t>((masked >> 8) & 0x3F)};
}

// `CAL addr` → [0x7E, lo8(addr), hi6(addr)] (same address shape as JMP).
inline constexpr std::array<std::uint8_t, 3> encode_cal(std::uint16_t addr) {
    std::uint16_t masked = static_cast<std::uint16_t>(addr & 0x3FFF);
    return {kCal, static_cast<std::uint8_t>(masked & 0xFF),
            static_cast<std::uint8_t>((masked >> 8) & 0x3F)};
}

}  // namespace intel8008_encoder
}  // namespace ca

#endif  // INTEL8008_ENCODER_HPP
