//! Combinational instruction decoder for the MOS 6502.
//!
//! # How the real 6502 decoder works
//!
//! The 6502 uses a PLA (Programmable Logic Array) with ~130 AND product terms
//! and 21 output lines. The AND rows activate on specific opcode patterns;
//! their OR-reduced outputs drive the control ROM for microcode timing.
//!
//! # Opcode structure: "aaa bbb cc" encoding
//!
//! ```text
//! Bits 7–5  (aaa): operation within the class
//! Bits 4–2  (bbb): addressing mode selector
//! Bits 1–0  (cc) : instruction class
//!
//! Class 01 → ALU group  (ORA, AND, EOR, ADC, STA, LDA, CMP, SBC)
//! Class 10 → shift/load (ASL, ROL, LSR, ROR, STX, LDX, DEC, INC)
//! Class 00 → misc       (BIT, JMP, STY, LDY, CPY, CPX, branches, flags)
//! ```
//!
//! # Gate-level group decode
//!
//! ```text
//! cc_bit0 = opcode[0]; cc_bit1 = opcode[1]
//! class01 = AND(cc_bit0, NOT(cc_bit1))
//! class10 = AND(NOT(cc_bit0), cc_bit1)
//! class00 = AND(NOT(cc_bit0), NOT(cc_bit1))
//! class11 = AND(cc_bit0, cc_bit1)  — mostly illegal
//! ```
//! 2-to-4 decoder: 2 NOT + 4 AND = 6 gates.
//!
//! # Branch instruction pattern
//!
//! All 8 branches follow `xxy10000` (bit4=1, bits 3–0 = 0000):
//! ```text
//! xx = flag selector (00=N, 01=V, 10=C, 11=Z)
//!  y = expected flag value
//! ```

use logic_gates::gates::{and_gate, not_gate};

// ─── Addressing mode constants ────────────────────────────────────────────────

pub const IMM: u8 = 0;  // Immediate:         #$nn
pub const ZP: u8 = 1;   // Zero Page:         $nn
pub const ZPX: u8 = 2;  // Zero Page,X:       $nn,X
pub const ZPY: u8 = 3;  // Zero Page,Y:       $nn,Y
pub const ABS: u8 = 4;  // Absolute:          $nnnn
pub const ABX: u8 = 5;  // Absolute,X:        $nnnn,X
pub const ABY: u8 = 6;  // Absolute,Y:        $nnnn,Y
pub const INX: u8 = 7;  // (Indirect,X):      ($nn,X)
pub const INY: u8 = 8;  // (Indirect),Y:      ($nn),Y
pub const IMP: u8 = 9;  // Implied
pub const ACC: u8 = 10; // Accumulator
pub const REL: u8 = 11; // Relative (branches)
pub const IND: u8 = 12; // Absolute Indirect  (JMP only)

/// Decoded instruction — result of the PLA lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedInstruction {
    pub opcode: u8,
    pub mnemonic: &'static str,
    pub mode: u8,
}

/// Full 151-opcode lookup table for all official NMOS 6502 instructions.
///
/// Represents the programmed PLA product terms. Each entry: (mnemonic, mode).
fn optable(opcode: u8) -> Option<(&'static str, u8)> {
    match opcode {
        // BRK / NOP
        0x00 => Some(("BRK", IMP)), 0xEA => Some(("NOP", IMP)),

        // LDA
        0xA9 => Some(("LDA", IMM)), 0xA5 => Some(("LDA", ZP)),  0xB5 => Some(("LDA", ZPX)),
        0xAD => Some(("LDA", ABS)), 0xBD => Some(("LDA", ABX)), 0xB9 => Some(("LDA", ABY)),
        0xA1 => Some(("LDA", INX)), 0xB1 => Some(("LDA", INY)),

        // LDX
        0xA2 => Some(("LDX", IMM)), 0xA6 => Some(("LDX", ZP)),  0xB6 => Some(("LDX", ZPY)),
        0xAE => Some(("LDX", ABS)), 0xBE => Some(("LDX", ABY)),

        // LDY
        0xA0 => Some(("LDY", IMM)), 0xA4 => Some(("LDY", ZP)),  0xB4 => Some(("LDY", ZPX)),
        0xAC => Some(("LDY", ABS)), 0xBC => Some(("LDY", ABX)),

        // STA
        0x85 => Some(("STA", ZP)),  0x95 => Some(("STA", ZPX)), 0x8D => Some(("STA", ABS)),
        0x9D => Some(("STA", ABX)), 0x99 => Some(("STA", ABY)), 0x81 => Some(("STA", INX)),
        0x91 => Some(("STA", INY)),

        // STX
        0x86 => Some(("STX", ZP)), 0x96 => Some(("STX", ZPY)), 0x8E => Some(("STX", ABS)),

        // STY
        0x84 => Some(("STY", ZP)), 0x94 => Some(("STY", ZPX)), 0x8C => Some(("STY", ABS)),

        // Register transfers
        0xAA => Some(("TAX", IMP)), 0xA8 => Some(("TAY", IMP)),
        0x8A => Some(("TXA", IMP)), 0x98 => Some(("TYA", IMP)),
        0xBA => Some(("TSX", IMP)), 0x9A => Some(("TXS", IMP)),

        // Stack
        0x48 => Some(("PHA", IMP)), 0x68 => Some(("PLA", IMP)),
        0x08 => Some(("PHP", IMP)), 0x28 => Some(("PLP", IMP)),

        // ADC
        0x69 => Some(("ADC", IMM)), 0x65 => Some(("ADC", ZP)),  0x75 => Some(("ADC", ZPX)),
        0x6D => Some(("ADC", ABS)), 0x7D => Some(("ADC", ABX)), 0x79 => Some(("ADC", ABY)),
        0x61 => Some(("ADC", INX)), 0x71 => Some(("ADC", INY)),

        // SBC
        0xE9 => Some(("SBC", IMM)), 0xE5 => Some(("SBC", ZP)),  0xF5 => Some(("SBC", ZPX)),
        0xED => Some(("SBC", ABS)), 0xFD => Some(("SBC", ABX)), 0xF9 => Some(("SBC", ABY)),
        0xE1 => Some(("SBC", INX)), 0xF1 => Some(("SBC", INY)),

        // AND
        0x29 => Some(("AND", IMM)), 0x25 => Some(("AND", ZP)),  0x35 => Some(("AND", ZPX)),
        0x2D => Some(("AND", ABS)), 0x3D => Some(("AND", ABX)), 0x39 => Some(("AND", ABY)),
        0x21 => Some(("AND", INX)), 0x31 => Some(("AND", INY)),

        // ORA
        0x09 => Some(("ORA", IMM)), 0x05 => Some(("ORA", ZP)),  0x15 => Some(("ORA", ZPX)),
        0x0D => Some(("ORA", ABS)), 0x1D => Some(("ORA", ABX)), 0x19 => Some(("ORA", ABY)),
        0x01 => Some(("ORA", INX)), 0x11 => Some(("ORA", INY)),

        // EOR
        0x49 => Some(("EOR", IMM)), 0x45 => Some(("EOR", ZP)),  0x55 => Some(("EOR", ZPX)),
        0x4D => Some(("EOR", ABS)), 0x5D => Some(("EOR", ABX)), 0x59 => Some(("EOR", ABY)),
        0x41 => Some(("EOR", INX)), 0x51 => Some(("EOR", INY)),

        // BIT
        0x24 => Some(("BIT", ZP)), 0x2C => Some(("BIT", ABS)),

        // INC
        0xE6 => Some(("INC", ZP)), 0xF6 => Some(("INC", ZPX)),
        0xEE => Some(("INC", ABS)), 0xFE => Some(("INC", ABX)),

        // INX / INY
        0xE8 => Some(("INX", IMP)), 0xC8 => Some(("INY", IMP)),

        // DEC
        0xC6 => Some(("DEC", ZP)), 0xD6 => Some(("DEC", ZPX)),
        0xCE => Some(("DEC", ABS)), 0xDE => Some(("DEC", ABX)),

        // DEX / DEY
        0xCA => Some(("DEX", IMP)), 0x88 => Some(("DEY", IMP)),

        // ASL
        0x0A => Some(("ASL", ACC)), 0x06 => Some(("ASL", ZP)),  0x16 => Some(("ASL", ZPX)),
        0x0E => Some(("ASL", ABS)), 0x1E => Some(("ASL", ABX)),

        // LSR
        0x4A => Some(("LSR", ACC)), 0x46 => Some(("LSR", ZP)),  0x56 => Some(("LSR", ZPX)),
        0x4E => Some(("LSR", ABS)), 0x5E => Some(("LSR", ABX)),

        // ROL
        0x2A => Some(("ROL", ACC)), 0x26 => Some(("ROL", ZP)),  0x36 => Some(("ROL", ZPX)),
        0x2E => Some(("ROL", ABS)), 0x3E => Some(("ROL", ABX)),

        // ROR
        0x6A => Some(("ROR", ACC)), 0x66 => Some(("ROR", ZP)),  0x76 => Some(("ROR", ZPX)),
        0x6E => Some(("ROR", ABS)), 0x7E => Some(("ROR", ABX)),

        // CMP
        0xC9 => Some(("CMP", IMM)), 0xC5 => Some(("CMP", ZP)),  0xD5 => Some(("CMP", ZPX)),
        0xCD => Some(("CMP", ABS)), 0xDD => Some(("CMP", ABX)), 0xD9 => Some(("CMP", ABY)),
        0xC1 => Some(("CMP", INX)), 0xD1 => Some(("CMP", INY)),

        // CPX
        0xE0 => Some(("CPX", IMM)), 0xE4 => Some(("CPX", ZP)), 0xEC => Some(("CPX", ABS)),

        // CPY
        0xC0 => Some(("CPY", IMM)), 0xC4 => Some(("CPY", ZP)), 0xCC => Some(("CPY", ABS)),

        // Branches
        0x90 => Some(("BCC", REL)), 0xB0 => Some(("BCS", REL)),
        0xF0 => Some(("BEQ", REL)), 0xD0 => Some(("BNE", REL)),
        0x10 => Some(("BPL", REL)), 0x30 => Some(("BMI", REL)),
        0x50 => Some(("BVC", REL)), 0x70 => Some(("BVS", REL)),

        // Jumps and subroutines
        0x4C => Some(("JMP", ABS)), 0x6C => Some(("JMP", IND)),
        0x20 => Some(("JSR", ABS)), 0x60 => Some(("RTS", IMP)), 0x40 => Some(("RTI", IMP)),

        // Flag instructions
        0x18 => Some(("CLC", IMP)), 0x38 => Some(("SEC", IMP)),
        0xD8 => Some(("CLD", IMP)), 0xF8 => Some(("SED", IMP)),
        0x58 => Some(("CLI", IMP)), 0x78 => Some(("SEI", IMP)),
        0xB8 => Some(("CLV", IMP)),

        _ => None,
    }
}

/// Decode a single opcode byte into mnemonic and addressing mode.
///
/// Uses AND/NOT gate logic for the 2-to-4 class decoder, then a PLA lookup.
///
/// # Panics
///
/// Panics on illegal (undocumented) NMOS 6502 opcodes.
pub fn decode(opcode: u8) -> DecodedInstruction {
    // Gate-level group decode on cc bits (opcode[1:0])
    let cc_bit0 = opcode & 1;
    let cc_bit1 = (opcode >> 1) & 1;

    // 2-to-4 decoder: produces one-hot class signals
    let _class01 = and_gate(cc_bit0, not_gate(cc_bit1));       // ALU
    let _class10 = and_gate(not_gate(cc_bit0), cc_bit1);       // shift/load
    let _class00 = and_gate(not_gate(cc_bit0), not_gate(cc_bit1)); // misc
    let _class11 = and_gate(cc_bit0, cc_bit1);                 // illegal

    // PLA lookup (programmed product terms)
    let (mnemonic, mode) = optable(opcode)
        .unwrap_or_else(|| panic!("illegal 6502 opcode {opcode:#04x}"));

    DecodedInstruction { opcode, mnemonic, mode }
}

/// Return true if the opcode is a legal NMOS 6502 instruction.
pub fn is_legal(opcode: u8) -> bool {
    optable(opcode).is_some()
}

/// Return true if the opcode is a conditional branch instruction.
///
/// All branches follow the pattern `xxy10000` (bits 3–0 = 0000, bit 4 = 1).
pub fn is_branch(opcode: u8) -> bool {
    let low_nibble_zero = and_gate(
        and_gate(not_gate(opcode & 1), not_gate((opcode >> 1) & 1)),
        and_gate(not_gate((opcode >> 2) & 1), not_gate((opcode >> 3) & 1)),
    );
    let bit4_set = and_gate((opcode >> 4) & 1, 1);
    and_gate(low_nibble_zero, bit4_set) != 0
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_lda_imm() {
        let d = decode(0xA9);
        assert_eq!(d.mnemonic, "LDA");
        assert_eq!(d.mode, IMM);
    }

    #[test]
    fn decode_lda_abs_x() {
        let d = decode(0xBD);
        assert_eq!(d.mnemonic, "LDA");
        assert_eq!(d.mode, ABX);
    }

    #[test]
    fn decode_adc_indirect_x() {
        let d = decode(0x61);
        assert_eq!(d.mnemonic, "ADC");
        assert_eq!(d.mode, INX);
    }

    #[test]
    fn decode_jmp_ind() {
        let d = decode(0x6C);
        assert_eq!(d.mnemonic, "JMP");
        assert_eq!(d.mode, IND);
    }

    #[test]
    fn decode_brk() {
        let d = decode(0x00);
        assert_eq!(d.mnemonic, "BRK");
        assert_eq!(d.mode, IMP);
    }

    #[test]
    fn decode_all_legal_opcodes_no_panic() {
        let legal_opcodes = [
            0x00, 0x01, 0x05, 0x06, 0x08, 0x09, 0x0A, 0x0D, 0x0E,
            0x10, 0x11, 0x15, 0x16, 0x18, 0x19, 0x1D, 0x1E,
            0x20, 0x21, 0x24, 0x25, 0x26, 0x28, 0x29, 0x2A, 0x2C, 0x2D, 0x2E,
            0x30, 0x31, 0x35, 0x36, 0x38, 0x39, 0x3D, 0x3E,
            0x40, 0x41, 0x45, 0x46, 0x48, 0x49, 0x4A, 0x4C, 0x4D, 0x4E,
            0x50, 0x51, 0x55, 0x56, 0x58, 0x59, 0x5D, 0x5E,
            0x60, 0x61, 0x65, 0x66, 0x68, 0x69, 0x6A, 0x6C, 0x6D, 0x6E,
            0x70, 0x71, 0x75, 0x76, 0x78, 0x79, 0x7D, 0x7E,
            0x81, 0x84, 0x85, 0x86, 0x88, 0x8A, 0x8C, 0x8D, 0x8E,
            0x90, 0x91, 0x94, 0x95, 0x96, 0x98, 0x99, 0x9A, 0x9D,
            0xA0, 0xA1, 0xA2, 0xA4, 0xA5, 0xA6, 0xA8, 0xA9, 0xAA, 0xAC, 0xAD, 0xAE,
            0xB0, 0xB1, 0xB4, 0xB5, 0xB6, 0xB8, 0xB9, 0xBA, 0xBC, 0xBD, 0xBE,
            0xC0, 0xC1, 0xC4, 0xC5, 0xC6, 0xC8, 0xC9, 0xCA, 0xCC, 0xCD, 0xCE,
            0xD0, 0xD1, 0xD5, 0xD6, 0xD8, 0xD9, 0xDD, 0xDE,
            0xE0, 0xE1, 0xE4, 0xE5, 0xE6, 0xE8, 0xE9, 0xEA, 0xEC, 0xED, 0xEE,
            0xF0, 0xF1, 0xF5, 0xF6, 0xF8, 0xF9, 0xFD, 0xFE,
        ];
        for &op in &legal_opcodes {
            let _ = decode(op); // must not panic
        }
    }

    #[test]
    fn is_legal_checks() {
        assert!(is_legal(0xA9)); // LDA IMM
        assert!(is_legal(0x00)); // BRK
        assert!(!is_legal(0x02)); // illegal
        assert!(!is_legal(0xFF)); // illegal
    }

    #[test]
    fn is_branch_checks() {
        assert!(is_branch(0x10)); // BPL
        assert!(is_branch(0x30)); // BMI
        assert!(is_branch(0x90)); // BCC
        assert!(is_branch(0xF0)); // BEQ
        assert!(!is_branch(0xA9)); // LDA — not a branch
        assert!(!is_branch(0x4C)); // JMP — not a branch
    }
}
