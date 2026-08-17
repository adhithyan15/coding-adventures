//! Encoding helpers for constructing Intel 8086 machine code (used by
//! this crate's own tests and re-exported by `intel8086-encoder`).
//!
//! Like `mos6502_simulator::encoding` (and unlike `mips_r2000_simulator::
//! encoding`, which assembles fixed 32-bit *words*), the 8086 is a
//! variable-length, byte-oriented ISA — every `encode_*` helper here
//! returns a `Vec<u8>` of the instruction's raw bytes directly (already
//! little-endian for multi-byte immediates, the 8086's native byte
//! order), and [`assemble`] is a trivial concatenation.
//!
//! Only the subset of mnemonics this crate's curated opcode table
//! (`opcodes.rs`) covers gets an `encode_*` helper — mirrors how
//! `mos6502_encoder`/`arm1_encoder` only re-export the mnemonics their
//! consuming backend needs, not the full opcode table.

use crate::opcodes;

/// `MOV reg16, imm16` — `[0xB8+reg, imm_lo, imm_hi]` (little-endian
/// immediate). This is the one instruction `intel8086-backend`'s
/// `const_*` lowering emits (always with `reg = opcodes::REG_AX`).
pub fn encode_mov_reg_imm16(reg: u8, imm: u16) -> Vec<u8> {
    vec![
        opcodes::MOV_REG_IMM16_BASE + reg,
        (imm & 0xFF) as u8,
        (imm >> 8) as u8,
    ]
}

/// `MOV reg8, imm8` — `[0xB0+reg, imm]`.
pub fn encode_mov_reg_imm8(reg: u8, imm: u8) -> Vec<u8> {
    vec![opcodes::MOV_REG_IMM8_BASE + reg, imm]
}

/// `ADD AX, imm16` — `[0x05, imm_lo, imm_hi]`.
pub fn encode_add_ax_imm16(imm: u16) -> Vec<u8> {
    vec![opcodes::ADD_AX_IMM16, (imm & 0xFF) as u8, (imm >> 8) as u8]
}

/// `SUB AX, imm16` — `[0x2D, imm_lo, imm_hi]`.
pub fn encode_sub_ax_imm16(imm: u16) -> Vec<u8> {
    vec![opcodes::SUB_AX_IMM16, (imm & 0xFF) as u8, (imm >> 8) as u8]
}

/// `MOV reg16, r/m16` register-to-register form — `[0x8B, modrm]` with
/// `mod=11`, `reg=dest`, `rm=src`.
pub fn encode_mov_reg_reg16(dest: u8, src: u8) -> Vec<u8> {
    vec![opcodes::MOV_REG_RM16, 0b1100_0000 | (dest << 3) | src]
}

/// `INC reg16` — `[0x40+reg]`.
pub fn encode_inc_reg16(reg: u8) -> Vec<u8> {
    vec![opcodes::INC_REG16_BASE + reg]
}

/// `DEC reg16` — `[0x48+reg]`.
pub fn encode_dec_reg16(reg: u8) -> Vec<u8> {
    vec![opcodes::DEC_REG16_BASE + reg]
}

/// `NOP` — `[0x90]`.
pub fn encode_nop() -> Vec<u8> {
    vec![opcodes::NOP_OPCODE]
}

/// `HLT` — the genuine halt instruction (see
/// [`crate::opcodes::HLT_OPCODE`]). `[0xF4]`.
pub fn encode_hlt() -> Vec<u8> {
    vec![opcodes::HLT_OPCODE]
}

/// Concatenate a sequence of per-instruction byte vectors into one flat
/// byte stream. Trivial here (no fixed instruction width, no word-
/// endianness conversion at the assembly level — each `encode_*` helper
/// already produced its bytes in wire order).
pub fn assemble(instructions: &[Vec<u8>]) -> Vec<u8> {
    instructions.iter().flat_map(|i| i.iter().copied()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mov_ax_imm16_encoding() {
        assert_eq!(
            encode_mov_reg_imm16(opcodes::REG_AX, 42),
            vec![0xB8, 42, 0x00]
        );
    }

    #[test]
    fn mov_reg_imm16_encodes_little_endian() {
        assert_eq!(
            encode_mov_reg_imm16(opcodes::REG_CX, 0x1234),
            vec![0xB9, 0x34, 0x12]
        );
    }

    #[test]
    fn hlt_encoding() {
        assert_eq!(encode_hlt(), vec![0xF4]);
    }

    #[test]
    fn mov_reg_reg16_encoding() {
        // MOV CX, AX -> mod=11 reg=CX(1) rm=AX(0) -> 0xC8
        assert_eq!(
            encode_mov_reg_reg16(opcodes::REG_CX, opcodes::REG_AX),
            vec![0x8B, 0xC8]
        );
    }

    #[test]
    fn assemble_concatenates() {
        let bytes = assemble(&[encode_mov_reg_imm16(opcodes::REG_AX, 42), encode_hlt()]);
        assert_eq!(bytes, vec![0xB8, 42, 0x00, 0xF4]);
    }

    #[test]
    fn round_trip_through_simulator() {
        use crate::simulator::Intel8086Simulator;
        let bytes = assemble(&[
            encode_mov_reg_imm16(opcodes::REG_AX, 7),
            encode_add_ax_imm16(5),
            encode_hlt(),
        ]);
        let mut sim = Intel8086Simulator::new(65536);
        sim.run(&bytes);
        assert_eq!(sim.ax, 12);
        assert!(sim.halted);
    }
}
