// armv7_encoder.hpp — a pure ARMv7-A (A32) instruction encoder, in pure ISO
// C++17, header-only, in namespace ca::armv7. A faithful port of the Rust
// `armv7-encoder` crate.
// ===========================================================================
//
// ARMv7-A is the 32-bit ARM instruction set of billions of Cortex-A7/A8/A9-era
// phone-class SoCs. This encoder knows nothing about IR: canonical
// instruction-word constants plus typed `encode_*` helpers that return the
// 32-bit machine word. Every value is an exact ARM A32 encoding (e.g.
// `MOV r0, #42` is `0xE3A0002A`); the constants and helpers are `constexpr`.
//
// ABI (AAPCS32): first argument / return value in `r0`; `lr`/`r14` holds the
// return address.
//
// PORTABILITY. Pure ISO C++17 — standard library only. Compiles clean under GCC,
// Clang, and MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
#ifndef CA_ARMV7_ENCODER_HPP
#define CA_ARMV7_ENCODER_HPP

#include <cstddef>
#include <cstdint>

namespace ca {
namespace armv7 {

// ── Canonical word constants ─────────────────────────────────────────────────

// `BX LR` — the AAPCS32 return-from-function instruction.
inline constexpr std::uint32_t BX_LR = 0xE12FFF1Eu;
// `BKPT #0` — breakpoint trap (a HLT-equivalent in emit-only contexts).
inline constexpr std::uint32_t BKPT = 0xE12FFF7Fu;

// Base encoding for `MOV Rd, #imm8` with `Rd = r0`; OR in `(rd << 12) | imm8`.
inline constexpr std::uint32_t MOV_IMM_R0_BASE = 0xE3A00000u;
// Base encoding for `MOV Rd, Rm`; OR in `(rd << 12) | rm`.
inline constexpr std::uint32_t MOV_REG_BASE = 0xE1A00000u;

// GP registers in the ABI scratch + saved set (`r0..r11`); r12–r15 reserved.
inline constexpr std::size_t GP_REGISTER_COUNT = 12;
// The widest immediate a MOV can carry directly (= 255).
inline constexpr std::uint32_t MOV_IMM_MAX = 255u;

// ── Encoders ─────────────────────────────────────────────────────────────────

// Encode `MOV Rd, #imm8`. `rd` is masked to 4 bits; `imm8` is 8 bits.
// Out-of-range values are the caller's responsibility.
constexpr std::uint32_t encode_mov_imm(std::uint8_t rd, std::uint8_t imm8) {
    return MOV_IMM_R0_BASE |
           (static_cast<std::uint32_t>(rd & 0x0F) << 12) |
           static_cast<std::uint32_t>(imm8);
}

// Encode `MOV Rd, Rm` — register-to-register copy. Both indices masked to 4 bits.
constexpr std::uint32_t encode_mov_reg(std::uint8_t rd, std::uint8_t rm) {
    return MOV_REG_BASE |
           (static_cast<std::uint32_t>(rd & 0x0F) << 12) |
           static_cast<std::uint32_t>(rm & 0x0F);
}

}  // namespace armv7
}  // namespace ca

#endif  // CA_ARMV7_ENCODER_HPP
