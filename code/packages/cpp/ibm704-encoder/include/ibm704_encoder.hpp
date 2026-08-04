// ibm704_encoder.hpp — pure IBM 704 instruction encoder, header-only ISO C++17.
// =============================================================================
//
// A faithful port of the Rust `ibm704-encoder` crate: an encoder for the
// IBM 704 (1954), the vacuum-tube mainframe on which John McCarthy and his MIT
// students first ran Lisp in 1959.
//
// ── The Lisp connection ──────────────────────────────────────────────────────
// `CAR`/`CDR`, Lisp's universal accessors, were IBM 704 instruction-word field
// names — Contents of the Address / Decrement part of Register. The 704's
// 36-bit word split into prefix/decrement/tag/address fields; a cons cell fit
// one per word, and `(CAR x)` took the address half, `(CDR x)` the decrement.
//
// ── Word format (idealised, v0.1.0) ──────────────────────────────────────────
//   bits 35..27 (9)  opcode   (HTR=0o420, CLA=0o500)
//   bits 26..15 (12) zero     (unused)
//   bits 14..0  (15) address  (<= 32 K words)
// Wire format: 5 bytes per word, low byte first, top byte's high nibble zero.
//
// Pure ISO C++17 — no <cmath>, no compiler extensions.
#ifndef IBM704_ENCODER_HPP
#define IBM704_ENCODER_HPP

#include <array>
#include <cstddef>
#include <cstdint>

namespace ca {
namespace ibm704_encoder {

// ── Opcodes ──────────────────────────────────────────────────────────────────

// HTR Y — Halt and TransfeR (opcode 0o420); `HTR 0` is the halt sentinel.
inline constexpr std::uint16_t kHtr = 0420;
// CLA Y — CLear accumulator and Add memory at Y (opcode 0o500).
inline constexpr std::uint16_t kCla = 0500;

// ── Word geometry ────────────────────────────────────────────────────────────

inline constexpr std::uint32_t kWordBits = 36;
inline constexpr std::uint64_t kWordMask = (std::uint64_t{1} << kWordBits) - 1;
inline constexpr std::size_t kBytesPerWord = 5;
inline constexpr std::uint32_t kAddrBits = 15;
inline constexpr std::uint64_t kAddrMask = (std::uint64_t{1} << kAddrBits) - 1;
inline constexpr std::uint32_t kOpcodeShift = 27;

// ── encode_* helpers ─────────────────────────────────────────────────────────

// Encode `<opcode> <address>` into a 36-bit instruction word. Opcode occupies
// bits 35..27, address bits 14..0 (out-of-range address bits are masked off).
inline std::uint64_t encode_instruction(std::uint16_t opcode,
                                        std::uint16_t address) {
    std::uint64_t op = static_cast<std::uint64_t>(opcode) << kOpcodeShift;
    std::uint64_t addr = static_cast<std::uint64_t>(address) & kAddrMask;
    return (op | addr) & kWordMask;
}
// Encode `HTR Y` — halt and transfer to Y.
inline std::uint64_t encode_htr(std::uint16_t address) {
    return encode_instruction(kHtr, address);
}
// Encode `CLA Y` — clear-and-add: AC <- memory[Y].
inline std::uint64_t encode_cla(std::uint16_t address) {
    return encode_instruction(kCla, address);
}

// ── Wire format ──────────────────────────────────────────────────────────────

// Pack a 36-bit word into 5 bytes, low byte first; bits 32..35 land in the low
// nibble of the last byte (its high nibble is always zero).
inline std::array<std::uint8_t, kBytesPerWord> pack_word(std::uint64_t word) {
    std::uint64_t w = word & kWordMask;
    return {
        static_cast<std::uint8_t>(w & 0xFF),
        static_cast<std::uint8_t>((w >> 8) & 0xFF),
        static_cast<std::uint8_t>((w >> 16) & 0xFF),
        static_cast<std::uint8_t>((w >> 24) & 0xFF),
        static_cast<std::uint8_t>((w >> 32) & 0x0F),
    };
}

// The pre-computed packing of the canonical `HTR 0` halt sentinel.
inline constexpr std::array<std::uint8_t, kBytesPerWord> kHtrHaltBytes = {
    0x00, 0x00, 0x00, 0x80, 0x08};

}  // namespace ibm704_encoder
}  // namespace ca

#endif  // IBM704_ENCODER_HPP
