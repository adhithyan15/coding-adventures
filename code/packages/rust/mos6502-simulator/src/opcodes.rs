//! Opcode table and addressing-mode constants for the MOS 6502 (NMOS) ISA.
//!
//! Unlike MIPS R2000 (fixed 32-bit words, three formats) or ARM1 (fixed
//! 32-bit words, one format per instruction class), the 6502 is a
//! **variable-length, byte-oriented** ISA: every instruction starts with a
//! single opcode byte, and the *addressing mode* that opcode byte selects
//! determines how many further operand bytes (0, 1, or 2) follow it.
//!
//! ```text
//! LDA #$2A        ; A9 2A        (2 bytes: opcode + immediate)
//! LDA $10         ; A5 10        (2 bytes: opcode + zero-page addr)
//! LDA $1234       ; AD 34 12     (3 bytes: opcode + little-endian abs addr)
//! BRK             ; 00           (1 byte: opcode only)
//! ```
//!
//! This module is the direct Rust transcription of the Python
//! `_OPTABLE` dict in `code/packages/python/mos6502-simulator/src/
//! mos6502_simulator/simulator.py` — same 151 official opcodes, same
//! (mnemonic, addressing_mode) pairing per opcode byte.  It carries no
//! decode/execute logic itself (see `decode.rs` / `execute.rs`).

// ===========================================================================
// Addressing modes
// ===========================================================================

/// The 13 addressing modes of the 6502.  Each determines how many operand
/// bytes follow the opcode and how the effective address (if any) is
/// computed — see `decode::resolve_address` for the resolution logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddrMode {
    /// `LDA #$nn` — operand is the literal byte immediately after the opcode.
    Imm,
    /// `LDA $nn` — address is `$00nn` (zero page).
    Zp,
    /// `LDA $nn,X` — address is `($nn + X) & 0xFF` (wraps within zero page).
    Zpx,
    /// `LDX $nn,Y` — address is `($nn + Y) & 0xFF`.
    Zpy,
    /// `LDA $nnnn` — full 16-bit little-endian address.
    Abs,
    /// `LDA $nnnn,X` — address is `$nnnn + X` (may cross a page boundary).
    Abx,
    /// `LDA $nnnn,Y` — address is `$nnnn + Y`.
    Aby,
    /// `LDA ($nn,X)` — indexed indirect: zero-page pointer at `($nn+X)&0xFF`.
    Inx,
    /// `LDA ($nn),Y` — indirect indexed: zero-page pointer at `$nn`, then `+Y`.
    Iny,
    /// `CLC` — no operand bytes, no address.
    Imp,
    /// `ASL A` — operand is the accumulator itself, no address.
    Acc,
    /// `BEQ $nn` — signed 8-bit PC-relative offset (branches only).
    Rel,
    /// `JMP ($nnnn)` — absolute indirect (`JMP` only); carries the famous
    /// page-wrap bug (see `decode::resolve_address`).
    Ind,
}

// ===========================================================================
// HALT sentinel
// ===========================================================================

/// `BRK` — the universal 6502 HALT sentinel used throughout this repo's
/// simulator stack (mirrors the Python original's documented convention:
/// "BRK (opcode 0x00) is treated as HALT ... matches the convention used
/// throughout the simulator stack (HLT for 8080, TRAP for IBM 704, etc.)").
/// Real 6502 `BRK` is a software interrupt; this simulator does not model
/// interrupt vectoring beyond the documented stack-push side effects (see
/// `execute::execute`).
pub const BRK_OPCODE: u8 = 0x00;

/// `NOP` — no operation, one byte, no operand.  Included here (rather than
/// only in the opcode table) since `mos6502-encoder` re-exports it as a
/// convenience padding instruction.
pub const NOP_OPCODE: u8 = 0xEA;

/// `LDA #imm` — immediate-mode load accumulator.  The one opcode
/// `mos6502-backend`'s minimal-viable scope actually emits.
pub const LDA_IMM_OPCODE: u8 = 0xA9;

// ===========================================================================
// Opcode table
// ===========================================================================

/// Look up the `(mnemonic, addressing_mode)` pair for an opcode byte.
///
/// Returns `None` for the ~100 undocumented/illegal opcode bytes not in
/// the 151-entry official table — mirrors the Python original raising
/// `ValueError("Illegal opcode ...")` for the same bytes.
#[allow(clippy::too_many_lines)]
pub fn lookup(opcode: u8) -> Option<(&'static str, AddrMode)> {
    use AddrMode::*;
    Some(match opcode {
        // BRK / NOP
        0x00 => ("BRK", Imp),
        0xEA => ("NOP", Imp),

        // Load A
        0xA9 => ("LDA", Imm),
        0xA5 => ("LDA", Zp),
        0xB5 => ("LDA", Zpx),
        0xAD => ("LDA", Abs),
        0xBD => ("LDA", Abx),
        0xB9 => ("LDA", Aby),
        0xA1 => ("LDA", Inx),
        0xB1 => ("LDA", Iny),

        // Load X
        0xA2 => ("LDX", Imm),
        0xA6 => ("LDX", Zp),
        0xB6 => ("LDX", Zpy),
        0xAE => ("LDX", Abs),
        0xBE => ("LDX", Aby),

        // Load Y
        0xA0 => ("LDY", Imm),
        0xA4 => ("LDY", Zp),
        0xB4 => ("LDY", Zpx),
        0xAC => ("LDY", Abs),
        0xBC => ("LDY", Abx),

        // Store A
        0x85 => ("STA", Zp),
        0x95 => ("STA", Zpx),
        0x8D => ("STA", Abs),
        0x9D => ("STA", Abx),
        0x99 => ("STA", Aby),
        0x81 => ("STA", Inx),
        0x91 => ("STA", Iny),

        // Store X
        0x86 => ("STX", Zp),
        0x96 => ("STX", Zpy),
        0x8E => ("STX", Abs),

        // Store Y
        0x84 => ("STY", Zp),
        0x94 => ("STY", Zpx),
        0x8C => ("STY", Abs),

        // Register transfers
        0xAA => ("TAX", Imp),
        0xA8 => ("TAY", Imp),
        0x8A => ("TXA", Imp),
        0x98 => ("TYA", Imp),
        0xBA => ("TSX", Imp),
        0x9A => ("TXS", Imp),

        // Stack
        0x48 => ("PHA", Imp),
        0x68 => ("PLA", Imp),
        0x08 => ("PHP", Imp),
        0x28 => ("PLP", Imp),

        // ADC
        0x69 => ("ADC", Imm),
        0x65 => ("ADC", Zp),
        0x75 => ("ADC", Zpx),
        0x6D => ("ADC", Abs),
        0x7D => ("ADC", Abx),
        0x79 => ("ADC", Aby),
        0x61 => ("ADC", Inx),
        0x71 => ("ADC", Iny),

        // SBC
        0xE9 => ("SBC", Imm),
        0xE5 => ("SBC", Zp),
        0xF5 => ("SBC", Zpx),
        0xED => ("SBC", Abs),
        0xFD => ("SBC", Abx),
        0xF9 => ("SBC", Aby),
        0xE1 => ("SBC", Inx),
        0xF1 => ("SBC", Iny),

        // AND
        0x29 => ("AND", Imm),
        0x25 => ("AND", Zp),
        0x35 => ("AND", Zpx),
        0x2D => ("AND", Abs),
        0x3D => ("AND", Abx),
        0x39 => ("AND", Aby),
        0x21 => ("AND", Inx),
        0x31 => ("AND", Iny),

        // ORA
        0x09 => ("ORA", Imm),
        0x05 => ("ORA", Zp),
        0x15 => ("ORA", Zpx),
        0x0D => ("ORA", Abs),
        0x1D => ("ORA", Abx),
        0x19 => ("ORA", Aby),
        0x01 => ("ORA", Inx),
        0x11 => ("ORA", Iny),

        // EOR
        0x49 => ("EOR", Imm),
        0x45 => ("EOR", Zp),
        0x55 => ("EOR", Zpx),
        0x4D => ("EOR", Abs),
        0x5D => ("EOR", Abx),
        0x59 => ("EOR", Aby),
        0x41 => ("EOR", Inx),
        0x51 => ("EOR", Iny),

        // BIT
        0x24 => ("BIT", Zp),
        0x2C => ("BIT", Abs),

        // INC
        0xE6 => ("INC", Zp),
        0xF6 => ("INC", Zpx),
        0xEE => ("INC", Abs),
        0xFE => ("INC", Abx),

        // INX / INY
        0xE8 => ("INX", Imp),
        0xC8 => ("INY", Imp),

        // DEC
        0xC6 => ("DEC", Zp),
        0xD6 => ("DEC", Zpx),
        0xCE => ("DEC", Abs),
        0xDE => ("DEC", Abx),

        // DEX / DEY
        0xCA => ("DEX", Imp),
        0x88 => ("DEY", Imp),

        // ASL
        0x0A => ("ASL", Acc),
        0x06 => ("ASL", Zp),
        0x16 => ("ASL", Zpx),
        0x0E => ("ASL", Abs),
        0x1E => ("ASL", Abx),

        // LSR
        0x4A => ("LSR", Acc),
        0x46 => ("LSR", Zp),
        0x56 => ("LSR", Zpx),
        0x4E => ("LSR", Abs),
        0x5E => ("LSR", Abx),

        // ROL
        0x2A => ("ROL", Acc),
        0x26 => ("ROL", Zp),
        0x36 => ("ROL", Zpx),
        0x2E => ("ROL", Abs),
        0x3E => ("ROL", Abx),

        // ROR
        0x6A => ("ROR", Acc),
        0x66 => ("ROR", Zp),
        0x76 => ("ROR", Zpx),
        0x6E => ("ROR", Abs),
        0x7E => ("ROR", Abx),

        // CMP
        0xC9 => ("CMP", Imm),
        0xC5 => ("CMP", Zp),
        0xD5 => ("CMP", Zpx),
        0xCD => ("CMP", Abs),
        0xDD => ("CMP", Abx),
        0xD9 => ("CMP", Aby),
        0xC1 => ("CMP", Inx),
        0xD1 => ("CMP", Iny),

        // CPX
        0xE0 => ("CPX", Imm),
        0xE4 => ("CPX", Zp),
        0xEC => ("CPX", Abs),

        // CPY
        0xC0 => ("CPY", Imm),
        0xC4 => ("CPY", Zp),
        0xCC => ("CPY", Abs),

        // Branches (all REL mode)
        0x90 => ("BCC", Rel),
        0xB0 => ("BCS", Rel),
        0xF0 => ("BEQ", Rel),
        0xD0 => ("BNE", Rel),
        0x10 => ("BPL", Rel),
        0x30 => ("BMI", Rel),
        0x50 => ("BVC", Rel),
        0x70 => ("BVS", Rel),

        // Jumps
        0x4C => ("JMP", Abs),
        0x6C => ("JMP", Ind),
        0x20 => ("JSR", Abs),
        0x60 => ("RTS", Imp),
        0x40 => ("RTI", Imp),

        // Flag instructions
        0x18 => ("CLC", Imp),
        0x38 => ("SEC", Imp),
        0xD8 => ("CLD", Imp),
        0xF8 => ("SED", Imp),
        0x58 => ("CLI", Imp),
        0x78 => ("SEI", Imp),
        0xB8 => ("CLV", Imp),

        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brk_is_opcode_zero() {
        assert_eq!(BRK_OPCODE, 0x00);
        assert_eq!(lookup(0x00), Some(("BRK", AddrMode::Imp)));
    }

    #[test]
    fn lda_imm_is_0xa9() {
        assert_eq!(LDA_IMM_OPCODE, 0xA9);
        assert_eq!(lookup(0xA9), Some(("LDA", AddrMode::Imm)));
    }

    #[test]
    fn nop_is_0xea() {
        assert_eq!(NOP_OPCODE, 0xEA);
        assert_eq!(lookup(0xEA), Some(("NOP", AddrMode::Imp)));
    }

    #[test]
    fn illegal_opcode_returns_none() {
        // 0x02 is a well-known illegal/undocumented (KIL/JAM) opcode on
        // NMOS 6502 -- not in the official 151-opcode table.
        assert_eq!(lookup(0x02), None);
    }

    #[test]
    fn all_151_official_opcodes_resolve() {
        // Sanity check: count non-None entries across the full byte range
        // and confirm it matches the documented "151 official opcodes".
        let count = (0u16..=255).filter(|&b| lookup(b as u8).is_some()).count();
        assert_eq!(count, 151);
    }

    #[test]
    fn branches_are_all_relative() {
        for op in [0x90, 0xB0, 0xF0, 0xD0, 0x10, 0x30, 0x50, 0x70] {
            let (_, mode) = lookup(op).unwrap();
            assert_eq!(mode, AddrMode::Rel);
        }
    }

    #[test]
    fn jmp_indirect_is_0x6c() {
        assert_eq!(lookup(0x6C), Some(("JMP", AddrMode::Ind)));
    }
}
