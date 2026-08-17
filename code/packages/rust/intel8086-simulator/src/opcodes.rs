//! Opcode table and register-index constants for the curated Intel 8086
//! instruction subset this simulator ports.
//!
//! # Why "curated subset", not the full CISC opcode map
//!
//! The Intel 8086 is a **big** ISA: ~250 opcode bytes, a two-operand ModRM
//! byte that can address either a register or one of eight memory
//! effective-address forms ( `[BX+SI]`, `[BX+SI+disp8]`, `[BP+DI+disp16]`,
//! …), segment-override prefixes, `REP`/`REPNE` string-op prefixes, BCD
//! adjust instructions (`AAA`/`AAS`/`DAA`/`DAS`/`AAM`/`AAD`), and a full
//! shift/rotate group. `code/packages/python/intel_8086_simulator/
//! simulator.py` (the reference this crate ports) implements essentially
//! all of it in ~1670 lines.
//!
//! This crate ports a **curated core**: register-immediate data transfer,
//! register-to-register data transfer and ALU ops (ModRM **mod=11 only** —
//! no memory effective-address computation), increment/decrement, and the
//! real `HLT` halt instruction. This is deliberately more than the
//! `const_*`/`ret_*`-only trivial-ROM scope every other lane in the
//! 9-architecture expansion needed (see
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md)),
//! because the task for this lane explicitly asks for "core data-transfer/
//! arithmetic ops" — but it is still far short of the full ISA. See the
//! crate-level doc's "Deferred" section for the complete list of what's
//! out of scope.
//!
//! # Register index encoding
//!
//! Mirrors the real 8086 ModRM `reg`/`rm` field encoding (and the Python
//! reference's `_get_reg16`/`_get_reg8`) exactly, so a future increment
//! that ports full ModRM effective-address decoding doesn't need to
//! renumber anything here.

// ===========================================================================
// 16-bit general-purpose / pointer register indices
// ===========================================================================

/// `AX` — accumulator.  Also the register this crate's `intel8086-backend`
/// always targets for `const_*` (see that crate's module doc).
pub const REG_AX: u8 = 0;
pub const REG_CX: u8 = 1;
pub const REG_DX: u8 = 2;
pub const REG_BX: u8 = 3;
pub const REG_SP: u8 = 4;
pub const REG_BP: u8 = 5;
pub const REG_SI: u8 = 6;
pub const REG_DI: u8 = 7;

/// 16-bit register mnemonics, indexed by the constants above.
pub const REG16_NAMES: [&str; 8] = ["AX", "CX", "DX", "BX", "SP", "BP", "SI", "DI"];

// ===========================================================================
// 8-bit register indices (byte halves of AX/CX/DX/BX)
// ===========================================================================

pub const REG_AL: u8 = 0;
pub const REG_CL: u8 = 1;
pub const REG_DL: u8 = 2;
pub const REG_BL: u8 = 3;
pub const REG_AH: u8 = 4;
pub const REG_CH: u8 = 5;
pub const REG_DH: u8 = 6;
pub const REG_BH: u8 = 7;

/// 8-bit register mnemonics, indexed by the constants above.
pub const REG8_NAMES: [&str; 8] = ["AL", "CL", "DL", "BL", "AH", "CH", "DH", "BH"];

// ===========================================================================
// HALT — the genuine hardware instruction, not a pseudo-halt
// ===========================================================================

/// `HLT` (`0xF4`) — a single-byte, no-operand instruction that stops the
/// CPU's fetch-decode-execute loop until the next interrupt (or, on real
/// silicon, `RESET`). Unlike ARM1's invented pseudo-halt (`SWI
/// #0x123456`, since ARMv1 has no real halt instruction) or MOS 6502's
/// repurposed `BRK` (a software-interrupt opcode the *simulator stack*
/// treats as HALT by convention), `HLT` genuinely means "halt" on real
/// 8086/8088 silicon — see `code/packages/python/intel-8086-simulator`'s
/// `simulator.py`, which sets `self._halted = True` on this exact opcode.
pub const HLT_OPCODE: u8 = 0xF4;

/// `NOP` (`0x90`) — a documented alias for `XCHG AX, AX` (see
/// `simulator.py`'s `0x90 <= op <= 0x97` XCHG-AX-with-reg group, where
/// `reg == 0` is special-cased to return `"NOP"`). No operand bytes.
pub const NOP_OPCODE: u8 = 0x90;

// ===========================================================================
// Register-immediate opcode bases
// ===========================================================================

/// `MOV reg16, imm16` — opcodes `0xB8..=0xBF`; the destination register is
/// `opcode - MOV_REG_IMM16_BASE`.  `MOV_REG_IMM16_BASE + REG_AX == 0xB8` is
/// the specific opcode `intel8086-backend`'s `const_*` lowering always
/// emits (it only ever targets `AX`).
pub const MOV_REG_IMM16_BASE: u8 = 0xB8;

/// `MOV reg8, imm8` — opcodes `0xB0..=0xB7`.
pub const MOV_REG_IMM8_BASE: u8 = 0xB0;

/// `INC reg16` — opcodes `0x40..=0x47`.
pub const INC_REG16_BASE: u8 = 0x40;

/// `DEC reg16` — opcodes `0x48..=0x4F`.
pub const DEC_REG16_BASE: u8 = 0x48;

// ===========================================================================
// Accumulator-immediate ALU opcodes (single byte each, no ModRM)
// ===========================================================================

pub const ADD_AX_IMM16: u8 = 0x05;
pub const OR_AX_IMM16: u8 = 0x0D;
pub const AND_AX_IMM16: u8 = 0x25;
pub const SUB_AX_IMM16: u8 = 0x2D;
pub const XOR_AX_IMM16: u8 = 0x35;
pub const CMP_AX_IMM16: u8 = 0x3D;

// ===========================================================================
// Register-to-register ALU/MOV opcodes (ModRM, mod=11 only supported)
// ===========================================================================

/// `MOV reg16, r/m16` (`d`=1, `w`=1) — this crate only supports `mod=11`
/// (register source); a memory operand (`mod != 11`) is a decode error
/// (see `decode::fetch_decode`).
pub const MOV_REG_RM16: u8 = 0x8B;
pub const ADD_REG_RM16: u8 = 0x03;
pub const OR_REG_RM16: u8 = 0x0B;
pub const AND_REG_RM16: u8 = 0x23;
pub const SUB_REG_RM16: u8 = 0x2B;
pub const XOR_REG_RM16: u8 = 0x33;
pub const CMP_REG_RM16: u8 = 0x3B;

// ===========================================================================
// Instruction "shape" — how the opcode's further bytes are interpreted
// ===========================================================================

/// The decode shape for an opcode in this crate's curated subset.
///
/// A deliberately small vocabulary next to `mos6502_simulator::opcodes::
/// AddrMode`'s 13 variants — the 8086's full ModRM/memory-addressing
/// machinery (effective-address computation for `[BX+SI]` and friends,
/// displacement bytes, segment-override prefixes) is out of scope for
/// this lane. See the crate-level doc's "Deferred" section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// No operand bytes at all (`HLT`, `NOP`).
    Implied,
    /// Destination register is encoded in the low 3 bits of the opcode
    /// byte itself; one little-endian `imm16` word follows
    /// (`MOV reg16,imm16`, `0xB8..=0xBF`).
    RegImm16,
    /// Same shape but one `imm8` byte follows (`MOV reg8,imm8`,
    /// `0xB0..=0xB7`).
    RegImm8,
    /// Register encoded in the low 3 bits of the opcode; no operand bytes
    /// (`INC`/`DEC reg16`).
    RegOnly,
    /// Operand is always `AX`; one little-endian `imm16` word follows
    /// (`ADD/OR/AND/SUB/XOR/CMP AX,imm16`).
    AccImm16,
    /// A ModRM byte follows. This crate only supports `mod=11`
    /// (register-to-register) — `mod != 11` (a memory operand) is a
    /// decode error.
    ModRegOnly16,
}

/// Look up the `(mnemonic, format)` pair for an opcode byte.
///
/// Returns `None` for any opcode outside this crate's curated subset —
/// mirrors `mos6502_simulator::opcodes::lookup`'s `None`-for-unlisted
/// convention, except here "unlisted" also covers the hundreds of real
/// 8086 opcodes this lane intentionally hasn't ported yet (not just
/// illegal/undocumented bytes).
pub fn lookup(opcode: u8) -> Option<(&'static str, Format)> {
    use Format::*;
    Some(match opcode {
        HLT_OPCODE => ("HLT", Implied),
        NOP_OPCODE => ("NOP", Implied),

        MOV_REG_IMM16_BASE..=0xBF => ("MOV", RegImm16),
        MOV_REG_IMM8_BASE..=0xB7 => ("MOV", RegImm8),

        INC_REG16_BASE..=0x47 => ("INC", RegOnly),
        DEC_REG16_BASE..=0x4F => ("DEC", RegOnly),

        ADD_AX_IMM16 => ("ADD", AccImm16),
        OR_AX_IMM16 => ("OR", AccImm16),
        AND_AX_IMM16 => ("AND", AccImm16),
        SUB_AX_IMM16 => ("SUB", AccImm16),
        XOR_AX_IMM16 => ("XOR", AccImm16),
        CMP_AX_IMM16 => ("CMP", AccImm16),

        MOV_REG_RM16 => ("MOV", ModRegOnly16),
        ADD_REG_RM16 => ("ADD", ModRegOnly16),
        OR_REG_RM16 => ("OR", ModRegOnly16),
        AND_REG_RM16 => ("AND", ModRegOnly16),
        SUB_REG_RM16 => ("SUB", ModRegOnly16),
        XOR_REG_RM16 => ("XOR", ModRegOnly16),
        CMP_REG_RM16 => ("CMP", ModRegOnly16),

        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hlt_is_0xf4() {
        assert_eq!(HLT_OPCODE, 0xF4);
        assert_eq!(lookup(0xF4), Some(("HLT", Format::Implied)));
    }

    #[test]
    fn nop_is_0x90() {
        assert_eq!(NOP_OPCODE, 0x90);
        assert_eq!(lookup(0x90), Some(("NOP", Format::Implied)));
    }

    #[test]
    fn mov_ax_imm16_is_0xb8() {
        assert_eq!(MOV_REG_IMM16_BASE + REG_AX, 0xB8);
        assert_eq!(lookup(0xB8), Some(("MOV", Format::RegImm16)));
    }

    #[test]
    fn mov_reg_imm16_covers_all_eight_registers() {
        for reg in 0u8..8 {
            assert_eq!(
                lookup(MOV_REG_IMM16_BASE + reg),
                Some(("MOV", Format::RegImm16))
            );
        }
    }

    #[test]
    fn mov_reg_imm8_covers_all_eight_registers() {
        for reg in 0u8..8 {
            assert_eq!(
                lookup(MOV_REG_IMM8_BASE + reg),
                Some(("MOV", Format::RegImm8))
            );
        }
    }

    #[test]
    fn inc_dec_reg16_cover_all_eight_registers() {
        for reg in 0u8..8 {
            assert_eq!(lookup(INC_REG16_BASE + reg), Some(("INC", Format::RegOnly)));
            assert_eq!(lookup(DEC_REG16_BASE + reg), Some(("DEC", Format::RegOnly)));
        }
    }

    #[test]
    fn acc_imm16_alu_ops() {
        assert_eq!(lookup(ADD_AX_IMM16), Some(("ADD", Format::AccImm16)));
        assert_eq!(lookup(SUB_AX_IMM16), Some(("SUB", Format::AccImm16)));
        assert_eq!(lookup(CMP_AX_IMM16), Some(("CMP", Format::AccImm16)));
    }

    #[test]
    fn reg_rm16_alu_ops() {
        assert_eq!(lookup(MOV_REG_RM16), Some(("MOV", Format::ModRegOnly16)));
        assert_eq!(lookup(ADD_REG_RM16), Some(("ADD", Format::ModRegOnly16)));
    }

    #[test]
    fn unsupported_opcode_returns_none() {
        // 0xE8 (CALL near) is a real 8086 opcode but out of this lane's
        // curated subset.
        assert_eq!(lookup(0xE8), None);
    }

    #[test]
    fn register_name_tables_are_eight_long_and_match_python_order() {
        assert_eq!(REG16_NAMES, ["AX", "CX", "DX", "BX", "SP", "BP", "SI", "DI"]);
        assert_eq!(REG8_NAMES, ["AL", "CL", "DL", "BL", "AH", "CH", "DH", "BH"]);
    }
}
