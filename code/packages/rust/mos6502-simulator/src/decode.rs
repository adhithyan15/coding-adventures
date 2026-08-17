//! Fetch + decode for the MOS 6502's 13 addressing modes.
//!
//! # Why fetch and decode are combined here (unlike `mips-r2000-simulator`)
//!
//! MIPS R2000 instructions are a fixed 32 bits — `decode()` there is a pure
//! function of the already-fetched word.  The 6502 is variable-length: the
//! addressing mode selected by the opcode byte determines how many further
//! operand bytes (0, 1, or 2) must be read from memory, and several modes
//! (`Zpx`/`Zpy`/`Abx`/`Aby`/`Inx`/`Iny`) also need the **current X/Y
//! register values** to compute the effective address.  Decoding a 6502
//! instruction is therefore inseparable from reading memory and advancing
//! the program counter — exactly what the Python original's
//! `_resolve_address` method does (it both reads bytes and mutates
//! `self._pc`).  This module's [`fetch_decode`] is the direct Rust
//! transcription of `_OPTABLE` lookup + `_resolve_address`.
//!
//! # The indirect JMP page-wrap bug
//!
//! `JMP ($10FF)` reads its low byte from `$10FF` but its high byte from
//! `$1000`, not `$1100` — a documented hardware bug in every NMOS 6502.
//! [`fetch_decode`] replicates it exactly (see the `Ind` arm below), just
//! as the Python original does.

use cpu_simulator::Memory;

use crate::opcodes::{lookup, AddrMode};

/// A fully decoded instruction: mnemonic, addressing mode, and (for modes
/// that produce one) the effective memory address.
///
/// For [`AddrMode::Imm`], `addr` is the address *of the immediate operand
/// byte itself* (not its value) — the caller reads memory at `addr` to get
/// the operand, mirroring the Python original's `_resolve_address` return
/// convention for `_IMM`.  For [`AddrMode::Imp`] and [`AddrMode::Acc`],
/// `addr` is `None`.  For [`AddrMode::Rel`], `addr` is the *branch target*
/// PC (not an operand address) — the offset has already been applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decoded {
    pub mnemonic: &'static str,
    pub mode: AddrMode,
    pub addr: Option<u16>,
    pub opcode: u8,
}

fn read_pc(mem: &Memory, pc: &mut u16) -> u8 {
    let b = mem.read_byte(*pc as usize);
    *pc = pc.wrapping_add(1);
    b
}

fn read_pc16(mem: &Memory, pc: &mut u16) -> u16 {
    let lo = read_pc(mem, pc) as u16;
    let hi = read_pc(mem, pc) as u16;
    (hi << 8) | lo
}

/// Resolve the effective address for `mode`, consuming whatever operand
/// bytes that mode requires from `*pc` (advancing it in place).  `x`/`y`
/// are the *current* index-register values (needed by the indexed modes).
fn resolve_address(mem: &Memory, pc: &mut u16, mode: AddrMode, x: u8, y: u8) -> Option<u16> {
    match mode {
        AddrMode::Imp | AddrMode::Acc => None,
        AddrMode::Imm => {
            let addr = *pc;
            *pc = pc.wrapping_add(1);
            Some(addr)
        }
        AddrMode::Zp => Some(read_pc(mem, pc) as u16),
        AddrMode::Zpx => Some(read_pc(mem, pc).wrapping_add(x) as u16),
        AddrMode::Zpy => Some(read_pc(mem, pc).wrapping_add(y) as u16),
        AddrMode::Abs => Some(read_pc16(mem, pc)),
        AddrMode::Abx => Some(read_pc16(mem, pc).wrapping_add(x as u16)),
        AddrMode::Aby => Some(read_pc16(mem, pc).wrapping_add(y as u16)),
        AddrMode::Inx => {
            let zp = read_pc(mem, pc).wrapping_add(x);
            let lo = mem.read_byte(zp as usize) as u16;
            let hi = mem.read_byte(zp.wrapping_add(1) as usize) as u16;
            Some((hi << 8) | lo)
        }
        AddrMode::Iny => {
            let zp = read_pc(mem, pc);
            let lo = mem.read_byte(zp as usize) as u16;
            let hi = mem.read_byte(zp.wrapping_add(1) as usize) as u16;
            Some(((hi << 8) | lo).wrapping_add(y as u16))
        }
        AddrMode::Ind => {
            // Absolute Indirect — JMP only.  The 6502 bug: if the low byte
            // of the pointer is 0xFF, the high byte wraps within the same
            // page instead of crossing into the next page.
            let ptr = read_pc16(mem, pc);
            let lo = mem.read_byte(ptr as usize) as u16;
            let hi_addr = (ptr & 0xFF00) | (ptr.wrapping_add(1) & 0x00FF);
            let hi = mem.read_byte(hi_addr as usize) as u16;
            Some((hi << 8) | lo)
        }
        AddrMode::Rel => {
            // Branch: read a signed 8-bit offset, return the *target* PC
            // (relative to the PC *after* the offset byte has been consumed).
            let raw = read_pc(mem, pc);
            let offset = raw as i8;
            Some((*pc as i32 + offset as i32) as u16)
        }
    }
}

/// Fetch the opcode byte at `*pc`, look it up, resolve its addressing
/// mode's effective address (consuming any operand bytes), and advance
/// `*pc` past the whole instruction.
///
/// Returns `Err` for an illegal/undocumented opcode byte (mirrors the
/// Python original's `ValueError("Illegal opcode ...")`).
pub fn fetch_decode(mem: &Memory, pc: &mut u16, x: u8, y: u8) -> Result<Decoded, String> {
    let opcode = read_pc(mem, pc);
    let (mnemonic, mode) = lookup(opcode)
        .ok_or_else(|| format!("Illegal opcode {opcode:#04x}"))?;
    let addr = resolve_address(mem, pc, mode, x, y);
    Ok(Decoded { mnemonic, mode, addr, opcode })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_lda_immediate() {
        let mut mem = Memory::new(65536);
        mem.load_bytes(0, &[0xA9, 0x2A]); // LDA #$2A
        let mut pc = 0u16;
        let d = fetch_decode(&mem, &mut pc, 0, 0).unwrap();
        assert_eq!(d.mnemonic, "LDA");
        assert_eq!(d.mode, AddrMode::Imm);
        assert_eq!(d.addr, Some(1)); // address of the immediate byte
        assert_eq!(pc, 2);
    }

    #[test]
    fn decode_lda_zero_page() {
        let mut mem = Memory::new(65536);
        mem.load_bytes(0, &[0xA5, 0x10]); // LDA $10
        let mut pc = 0u16;
        let d = fetch_decode(&mem, &mut pc, 0, 0).unwrap();
        assert_eq!(d.addr, Some(0x0010));
        assert_eq!(pc, 2);
    }

    #[test]
    fn decode_lda_zero_page_x_wraps() {
        let mut mem = Memory::new(65536);
        mem.load_bytes(0, &[0xB5, 0xFF]); // LDA $FF,X
        let mut pc = 0u16;
        let d = fetch_decode(&mem, &mut pc, 0x02, 0).unwrap();
        assert_eq!(d.addr, Some(0x0001)); // (0xFF + 0x02) & 0xFF = 0x01
    }

    #[test]
    fn decode_lda_absolute() {
        let mut mem = Memory::new(65536);
        mem.load_bytes(0, &[0xAD, 0x34, 0x12]); // LDA $1234
        let mut pc = 0u16;
        let d = fetch_decode(&mem, &mut pc, 0, 0).unwrap();
        assert_eq!(d.addr, Some(0x1234));
        assert_eq!(pc, 3);
    }

    #[test]
    fn decode_brk_is_implied() {
        let mut mem = Memory::new(65536);
        mem.load_bytes(0, &[0x00]);
        let mut pc = 0u16;
        let d = fetch_decode(&mem, &mut pc, 0, 0).unwrap();
        assert_eq!(d.mnemonic, "BRK");
        assert_eq!(d.mode, AddrMode::Imp);
        assert_eq!(d.addr, None);
        assert_eq!(pc, 1);
    }

    #[test]
    fn decode_illegal_opcode_errs() {
        let mut mem = Memory::new(65536);
        mem.load_bytes(0, &[0x02]); // KIL/JAM — not in the official table
        let mut pc = 0u16;
        assert!(fetch_decode(&mem, &mut pc, 0, 0).is_err());
    }

    #[test]
    fn decode_indirect_jmp_page_wrap_bug() {
        // JMP ($10FF): low byte from $10FF, high byte from $1000
        // (NOT $1100) -- the documented NMOS 6502 hardware bug.
        let mut mem = Memory::new(65536);
        mem.load_bytes(0, &[0x6C, 0xFF, 0x10]);
        mem.write_byte(0x10FF, 0x00); // low byte of target
        mem.write_byte(0x1000, 0x20); // high byte of target (bug: from $1000)
        mem.write_byte(0x1100, 0x99); // decoy -- must NOT be read
        let mut pc = 0u16;
        let d = fetch_decode(&mem, &mut pc, 0, 0).unwrap();
        assert_eq!(d.addr, Some(0x2000));
    }

    #[test]
    fn decode_relative_branch_target() {
        let mut mem = Memory::new(65536);
        // BEQ -2 (branch back to itself's opcode byte, a tight loop)
        mem.load_bytes(0x10, &[0xF0, (-2i8) as u8]);
        let mut pc = 0x10u16;
        let d = fetch_decode(&mem, &mut pc, 0, 0).unwrap();
        // pc after consuming both bytes is 0x12; target = 0x12 - 2 = 0x10
        assert_eq!(d.addr, Some(0x10));
    }

    #[test]
    fn decode_indexed_indirect_x() {
        let mut mem = Memory::new(65536);
        mem.load_bytes(0, &[0xA1, 0x20]); // LDA ($20,X)
        mem.write_byte(0x22, 0x00); // zp = (0x20 + 2) & 0xFF = 0x22
        mem.write_byte(0x23, 0x30);
        let mut pc = 0u16;
        let d = fetch_decode(&mem, &mut pc, 0x02, 0).unwrap();
        assert_eq!(d.addr, Some(0x3000));
    }

    #[test]
    fn decode_indirect_indexed_y() {
        let mut mem = Memory::new(65536);
        mem.load_bytes(0, &[0xB1, 0x20]); // LDA ($20),Y
        mem.write_byte(0x20, 0x00);
        mem.write_byte(0x21, 0x30);
        let mut pc = 0u16;
        let d = fetch_decode(&mem, &mut pc, 0, 0x05).unwrap();
        assert_eq!(d.addr, Some(0x3005));
    }
}
