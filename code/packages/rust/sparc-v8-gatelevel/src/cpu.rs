//! SPARC V8 CPU — instruction fetch, decode, and execution.
//!
//! # Halting convention
//!
//! `ta 0` encodes as `0x91D0_2000`.  When the CPU decodes this trap with
//! `cond=8` (always) and `trap_number=0`, it sets `halted = true` and stops.
//!
//! # Memory
//!
//! 64 KiB flat memory, addresses masked to 16 bits.  Instructions are
//! big-endian 32-bit words.
//!
//! # No branch delay slots
//!
//! This simulator omits the SPARC delay slot for simplicity; branches take
//! effect immediately (the instruction after the branch is *not* executed).

use crate::alu::{
    add32, addcc32, addx32, addxcc32, and32, andcc32, andn32, andncc32, mulscc, or32, orcc32,
    orn32, orncc32, sdiv64, sethi as alu_sethi, sll32, smul32, sra32, srl32, sub32, subcc32,
    subx32, subxcc32, udiv64, umul32, xnor32, xnorcc32, xor32, xorcc32, Cc,
};
use crate::bits::{sext22, sext30, u32_to_bits};
use crate::decoder::{decode, Instruction, Src2};
use crate::register_file::{MEM_SIZE, RegisterFile};

const HALT_WORD: u32 = 0x91D0_2000;
const MEM_MASK: u32 = (MEM_SIZE - 1) as u32;

/// Errors the CPU can raise.
#[derive(Debug, Clone, PartialEq)]
pub enum SparcError {
    InvalidLoad(String),
    WindowOverflow,
    IllegalInstruction(u32),
}

impl std::fmt::Display for SparcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SparcError::InvalidLoad(s) => write!(f, "invalid load: {}", s),
            SparcError::WindowOverflow => write!(f, "register window overflow"),
            SparcError::IllegalInstruction(w) => write!(f, "illegal instruction: {:#010x}", w),
        }
    }
}

impl std::error::Error for SparcError {}

/// The SPARC V8 CPU.
pub struct SparcCpu {
    pub rf: RegisterFile,
    pub mem: [u8; MEM_SIZE],
    pub halted: bool,
    pub steps: u64,
}

impl SparcCpu {
    pub fn new() -> Self {
        Self {
            rf: RegisterFile::new(),
            mem: [0u8; MEM_SIZE],
            halted: false,
            steps: 0,
        }
    }

    /// Load a program into memory starting at `origin` and reset state.
    ///
    /// Validates that both `origin` and `origin + program.len()` are within
    /// the 64 KiB address space before touching any state.
    pub fn load(&mut self, program: &[u8], origin: u32) -> Result<(), SparcError> {
        let start = origin as usize;
        if start >= MEM_SIZE {
            return Err(SparcError::InvalidLoad(format!(
                "origin {:#010x} is outside the 64 KiB memory range",
                origin
            )));
        }
        let available = MEM_SIZE - start;
        if program.len() > available {
            return Err(SparcError::InvalidLoad(format!(
                "program length {} exceeds available {} bytes at origin {:#06x}",
                program.len(),
                available,
                origin
            )));
        }
        // Only mutate state after successful validation.
        self.rf.reset();
        self.halted = false;
        self.steps = 0;
        let end = start + program.len();
        self.mem[start..end].copy_from_slice(program);
        self.rf.pc = origin;
        Ok(())
    }

    /// Run the program until halted or `max_steps` are reached.
    pub fn execute(&mut self, program: &[u8], origin: u32, max_steps: u64) -> Result<(), SparcError> {
        self.load(program, origin)?;
        while !self.halted && self.steps < max_steps {
            self.step()?;
        }
        Ok(())
    }

    /// Execute a single instruction.
    pub fn step(&mut self) -> Result<(), SparcError> {
        let word = self.fetch();
        if word == HALT_WORD {
            self.halted = true;
            return Ok(());
        }
        let instr = decode(word);
        self.dispatch(instr, word)?;
        self.steps += 1;
        Ok(())
    }

    // ── Memory access ─────────────────────────────────────────────────────────

    fn fetch(&mut self) -> u32 {
        let addr = (self.rf.pc & MEM_MASK) as usize;
        // Each byte is masked individually so that wrapping at 0xFFFF is safe.
        let b0 = self.mem[addr];
        let b1 = self.mem[(addr + 1) & (MEM_SIZE - 1)];
        let b2 = self.mem[(addr + 2) & (MEM_SIZE - 1)];
        let b3 = self.mem[(addr + 3) & (MEM_SIZE - 1)];
        self.rf.pc = self.rf.pc.wrapping_add(4);
        ((b0 as u32) << 24) | ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32)
    }

    fn load_word(&self, addr: u32) -> u32 {
        let a = (addr & MEM_MASK) as usize;
        let b0 = self.mem[a];
        let b1 = self.mem[(a + 1) & (MEM_SIZE - 1)];
        let b2 = self.mem[(a + 2) & (MEM_SIZE - 1)];
        let b3 = self.mem[(a + 3) & (MEM_SIZE - 1)];
        ((b0 as u32) << 24) | ((b1 as u32) << 16) | ((b2 as u32) << 8) | (b3 as u32)
    }

    fn load_half_unsigned(&self, addr: u32) -> u32 {
        let a = (addr & MEM_MASK) as usize;
        let b0 = self.mem[a];
        let b1 = self.mem[(a + 1) & (MEM_SIZE - 1)];
        ((b0 as u32) << 8) | (b1 as u32)
    }

    fn load_half_signed(&self, addr: u32) -> u32 {
        let h = self.load_half_unsigned(addr) as u16;
        (h as i16) as u32
    }

    fn load_byte_unsigned(&self, addr: u32) -> u32 {
        let a = (addr & MEM_MASK) as usize;
        self.mem[a] as u32
    }

    fn load_byte_signed(&self, addr: u32) -> u32 {
        let b = self.mem[(addr & MEM_MASK) as usize] as i8;
        b as u32
    }

    fn store_word(&mut self, addr: u32, val: u32) {
        let a = (addr & MEM_MASK) as usize;
        self.mem[a] = (val >> 24) as u8;
        self.mem[(a + 1) & (MEM_SIZE - 1)] = (val >> 16) as u8;
        self.mem[(a + 2) & (MEM_SIZE - 1)] = (val >> 8) as u8;
        self.mem[(a + 3) & (MEM_SIZE - 1)] = val as u8;
    }

    fn store_half(&mut self, addr: u32, val: u32) {
        let a = (addr & MEM_MASK) as usize;
        self.mem[a] = (val >> 8) as u8;
        self.mem[(a + 1) & (MEM_SIZE - 1)] = val as u8;
    }

    fn store_byte(&mut self, addr: u32, val: u32) {
        let a = (addr & MEM_MASK) as usize;
        self.mem[a] = val as u8;
    }

    // ── Instruction dispatch ──────────────────────────────────────────────────

    fn resolve_src2(&self, s: &Src2) -> u32 {
        match s {
            Src2::Reg(r) => self.rf.read(*r),
            Src2::Imm(imm) => *imm,
        }
    }

    fn apply_cc(&mut self, cc: Cc) {
        self.rf.psr.n = cc.n;
        self.rf.psr.z = cc.z;
        self.rf.psr.v = cc.v;
        self.rf.psr.c = cc.c;
    }

    fn dispatch(&mut self, instr: Instruction, raw: u32) -> Result<(), SparcError> {
        match instr {
            Instruction::Nop => {}

            Instruction::Call { disp30 } => {
                // Save PC of this instruction into %o7 (logical 15).
                let pc_of_call = self.rf.pc.wrapping_sub(4);
                self.rf.write(15, pc_of_call);
                self.rf.pc = pc_of_call.wrapping_add(sext30(disp30).wrapping_mul(4));
            }

            Instruction::Sethi { rd, imm22 } => {
                self.rf.write(rd, alu_sethi(imm22));
            }

            Instruction::Bicc { cond, disp22, .. } => {
                if self.branch_taken(cond) {
                    let disp = sext22(disp22).wrapping_mul(4);
                    self.rf.pc = self.rf.pc.wrapping_sub(4).wrapping_add(disp);
                }
            }

            Instruction::Ticc { cond, rs1, src2 } => {
                if self.branch_taken(cond) {
                    let trap_num = add32(self.rf.read(rs1), self.resolve_src2(&src2)) & 0x7F;
                    if trap_num == 0 {
                        self.halted = true;
                    }
                }
            }

            Instruction::Alu { op3, rd, rs1, src2 } => {
                let a = self.rf.read(rs1);
                let b = self.resolve_src2(&src2);
                self.exec_alu(op3, rd, rs1, a, b, &src2)?;
            }

            Instruction::Load { op3, rd, rs1, src2 } => {
                let base = self.rf.read(rs1);
                let offset = self.resolve_src2(&src2);
                let addr = add32(base, offset);
                let val = match op3 {
                    0x00 => self.load_word(addr),                 // LD
                    0x01 => self.load_byte_unsigned(addr),        // LDUB
                    0x02 => self.load_half_unsigned(addr),        // LDUH
                    0x09 => self.load_byte_signed(addr),          // LDSB
                    0x0A => self.load_half_signed(addr),          // LDSH
                    _ => return Err(SparcError::IllegalInstruction(raw)),
                };
                self.rf.write(rd, val);
            }

            Instruction::Store { op3, rd, rs1, src2 } => {
                let base = self.rf.read(rs1);
                let offset = self.resolve_src2(&src2);
                let addr = add32(base, offset);
                let val = self.rf.read(rd);
                match op3 {
                    0x04 => self.store_word(addr, val),           // ST
                    0x05 => self.store_byte(addr, val),           // STB
                    0x06 => self.store_half(addr, val),           // STH
                    _ => return Err(SparcError::IllegalInstruction(raw)),
                }
            }

            Instruction::Illegal(w) => return Err(SparcError::IllegalInstruction(w)),
        }
        Ok(())
    }

    fn exec_alu(&mut self, op3: u8, rd: u32, rs1: u32, a: u32, b: u32, _src2: &Src2) -> Result<(), SparcError> {
        match op3 {
            // ── Arithmetic ────────────────────────────────────────────────────
            0x00 => { self.rf.write(rd, add32(a, b)); }                   // ADD
            0x10 => { let (r, cc) = addcc32(a, b); self.rf.write(rd, r); self.apply_cc(cc); } // ADDcc
            0x08 => { self.rf.write(rd, addx32(a, b, self.rf.psr.c)); }   // ADDX
            0x18 => {                                                       // ADDXcc
                let (r, cc) = addxcc32(a, b, self.rf.psr.c);
                self.rf.write(rd, r);
                self.apply_cc(cc);
            }
            0x04 => { self.rf.write(rd, sub32(a, b)); }                   // SUB
            0x14 => { let (r, cc) = subcc32(a, b); self.rf.write(rd, r); self.apply_cc(cc); } // SUBcc
            0x0C => { self.rf.write(rd, subx32(a, b, self.rf.psr.c)); }   // SUBX
            0x1C => {                                                       // SUBXcc
                let (r, cc) = subxcc32(a, b, self.rf.psr.c);
                self.rf.write(rd, r);
                self.apply_cc(cc);
            }

            // ── Logical ───────────────────────────────────────────────────────
            0x01 => { self.rf.write(rd, and32(a, b)); }
            0x11 => { let (r, cc) = andcc32(a, b); self.rf.write(rd, r); self.apply_cc(cc); }
            0x05 => { self.rf.write(rd, andn32(a, b)); }
            0x15 => { let (r, cc) = andncc32(a, b); self.rf.write(rd, r); self.apply_cc(cc); }
            0x02 => { self.rf.write(rd, or32(a, b)); }
            0x12 => { let (r, cc) = orcc32(a, b); self.rf.write(rd, r); self.apply_cc(cc); }
            0x06 => { self.rf.write(rd, orn32(a, b)); }
            0x16 => { let (r, cc) = orncc32(a, b); self.rf.write(rd, r); self.apply_cc(cc); }
            0x03 => { self.rf.write(rd, xor32(a, b)); }
            0x13 => { let (r, cc) = xorcc32(a, b); self.rf.write(rd, r); self.apply_cc(cc); }
            0x07 => { self.rf.write(rd, xnor32(a, b)); }
            0x17 => { let (r, cc) = xnorcc32(a, b); self.rf.write(rd, r); self.apply_cc(cc); }

            // ── Shifts ────────────────────────────────────────────────────────
            0x25 => { self.rf.write(rd, sll32(a, b & 0x1F)); }            // SLL
            0x26 => { self.rf.write(rd, srl32(a, b & 0x1F)); }            // SRL
            0x27 => { self.rf.write(rd, sra32(a, b & 0x1F)); }            // SRA

            // ── Multiply ──────────────────────────────────────────────────────
            0x0A => {                                                        // UMUL
                let (y, lo) = umul32(a, b);
                self.rf.write(rd, lo);
                self.rf.y = y;
            }
            0x0B => {                                                        // SMUL
                let (y, lo) = smul32(a, b);
                self.rf.write(rd, lo);
                self.rf.y = y;
            }
            0x1A => {                                                        // UMULcc
                let (y, lo) = umul32(a, b);
                self.rf.write(rd, lo);
                self.rf.y = y;
                let bits = u32_to_bits(lo);
                use crate::bits::compute_zero;
                self.rf.psr.n = bits[31];
                self.rf.psr.z = compute_zero(&bits);
                self.rf.psr.v = 0;
                self.rf.psr.c = 0;
            }
            0x1B => {                                                        // SMULcc
                let (y, lo) = smul32(a, b);
                self.rf.write(rd, lo);
                self.rf.y = y;
                let bits = u32_to_bits(lo);
                use crate::bits::compute_zero;
                self.rf.psr.n = bits[31];
                self.rf.psr.z = compute_zero(&bits);
                self.rf.psr.v = 0;
                self.rf.psr.c = 0;
            }

            // ── Divide ────────────────────────────────────────────────────────
            0x0E => { self.rf.write(rd, udiv64(self.rf.y, a, b)); }       // UDIV
            0x0F => { self.rf.write(rd, sdiv64(self.rf.y, a, b)); }       // SDIV

            // ── MULScc ───────────────────────────────────────────────────────
            0x24 => {                                                        // MULScc
                let (new_rd, new_y, cc) = mulscc(
                    self.rf.read(rd), self.rf.y, a,
                    self.rf.psr.n, self.rf.psr.v,
                );
                self.rf.write(rd, new_rd);
                self.rf.y = new_y;
                self.apply_cc(cc);
            }

            // ── Special ───────────────────────────────────────────────────────
            // JMPL: rd = PC of this instruction; PC = rs1 + src2
            0x38 => {
                let pc_of_jmpl = self.rf.pc.wrapping_sub(4);
                self.rf.write(rd, pc_of_jmpl);
                self.rf.pc = add32(a, b);
            }

            // SAVE
            0x3C => {
                let result = {
                    let rs1_val = a;
                    let src2_val = b;
                    let rf = &mut self.rf;
                    if rf.save_depth >= crate::register_file::NWINDOWS - 1 {
                        return Err(SparcError::WindowOverflow);
                    }
                    let r = add32(rs1_val, src2_val);
                    rf.cwp = (rf.cwp + crate::register_file::NWINDOWS - 1) % crate::register_file::NWINDOWS;
                    rf.save_depth += 1;
                    r
                };
                self.rf.write(rd, result);
            }

            // RESTORE
            0x3D => {
                let result = add32(a, b);
                self.rf.cwp = (self.rf.cwp + 1) % crate::register_file::NWINDOWS;
                if self.rf.save_depth > 0 {
                    self.rf.save_depth -= 1;
                }
                self.rf.write(rd, result);
            }

            // RD %y (read Y register)
            0x28 if rs1 == 0 => { self.rf.write(rd, self.rf.y); }

            // WR %y (write Y register): rd=0 means Y, src=rs1 XOR src2
            0x30 => { self.rf.y = xor32(a, b); }

            _ => return Err(SparcError::IllegalInstruction(op3 as u32)),
        }
        Ok(())
    }

    fn branch_taken(&self, cond: u8) -> bool {
        let n = self.rf.psr.n;
        let z = self.rf.psr.z;
        let v = self.rf.psr.v;
        let c = self.rf.psr.c;
        match cond {
            0  => false,             // BN  — never
            1  => c == 1 || z == 1, // BLE ≡ Z OR (N XOR V) — unsigned: lower/equal
            2  => z == 1 || (n ^ v) == 1, // BLE signed
            3  => c == 1,            // BCS / BLU
            4  => z == 1,            // BE
            5  => (n ^ v) == 1,      // BL
            6  => z == 1 || (n ^ v) == 1, // BLE signed (duplicate encoding in spec)
            7  => n == 1,            // BNEG
            8  => true,              // BA  — always
            9  => c == 0 && z == 0,  // BGU
            10 => z == 0 && (n ^ v) == 0, // BGE signed part
            11 => c == 0,            // BCC / BGEU
            12 => z == 0,            // BNE
            13 => z == 0 && (n ^ v) == 0, // BG signed
            14 => n == 0,            // BPOS
            15 => v == 0,            // BVC
            _  => false,
        }
    }
}

impl Default for SparcCpu {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode HALT (ta 0 = `0x91D0_2000`).
    fn halt() -> [u8; 4] { HALT_WORD.to_be_bytes() }

    /// Encode `sethi imm22, %rd` → op=00, rd, op2=100, imm22.
    fn enc_sethi(rd: u32, imm22: u32) -> u32 {
        ((rd << 25)) | (0b100 << 22) | (imm22 & 0x003F_FFFF)
    }

    /// Encode `add rs1, rs2, rd` (op=10, op3=0, i=0).
    fn enc_add(rd: u32, rs1: u32, rs2: u32) -> u32 {
        ((0b10u32 << 30) | (rd << 25)) | (rs1 << 14) | rs2
    }

    /// Encode `add rs1, simm13, rd` (op=10, op3=0, i=1).
    fn enc_addi(rd: u32, rs1: u32, imm: i32) -> u32 {
        ((0b10u32 << 30) | (rd << 25)) | (rs1 << 14) | (1 << 13) | ((imm as u32) & 0x1FFF)
    }

    /// Encode `sub rs1, rs2, rd` (op=10, op3=4).
    fn enc_sub(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (4u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `subcc rs1, rs2, rd` (op3=0x14).
    fn enc_subcc(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x14u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `addcc rs1, rs2, rd` (op3=0x10).
    fn enc_addcc(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x10u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `or rs1, rs2, rd` (op3=2).
    fn enc_or(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (2u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `and rs1, rs2, rd` (op3=1).
    fn enc_and(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (1u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `xor rs1, rs2, rd` (op3=3).
    fn enc_xor(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (3u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `sll rs1, rs2, rd` (op3=0x25).
    fn enc_sll(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x25u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `srl rs1, rs2, rd` (op3=0x26).
    fn enc_srl(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x26u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `sra rs1, rs2, rd` (op3=0x27).
    fn enc_sra(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x27u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `st rd, [rs1 + rs2]` (op=11, op3=4, i=0).
    fn enc_st(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b11u32 << 30) | (rd << 25) | (4u32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `ld [rs1 + rs2], rd` (op=11, op3=0, i=0).
    fn enc_ld(rd: u32, rs1: u32, rs2: u32) -> u32 {
        ((0b11u32 << 30) | (rd << 25)) | (rs1 << 14) | rs2
    }

    /// Encode `ba disp22` (cond=8, op2=010).
    fn enc_ba(disp22: i32) -> u32 {
        ((8u32 << 25)) | (0b010u32 << 22) | ((disp22 as u32) & 0x003F_FFFF)
    }

    /// Encode `be disp22` (cond=4).
    fn enc_be(disp22: i32) -> u32 {
        ((4u32 << 25)) | (0b010u32 << 22) | ((disp22 as u32) & 0x003F_FFFF)
    }

    /// Encode `bne disp22` (cond=12).
    fn enc_bne(disp22: i32) -> u32 {
        ((12u32 << 25)) | (0b010u32 << 22) | ((disp22 as u32) & 0x003F_FFFF)
    }

    /// Encode `jmpl rs1 + imm13, rd` (op3=0x38, i=1).
    fn enc_jmpl(rd: u32, rs1: u32, imm: i32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x38u32 << 19) | (rs1 << 14) | (1 << 13) | ((imm as u32) & 0x1FFF)
    }

    /// Encode `umul rs1, rs2, rd` (op3=0x0A).
    fn enc_umul(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x0Au32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `smul rs1, rs2, rd` (op3=0x0B).
    fn enc_smul(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x0Bu32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `udiv rs1, rs2, rd` (op3=0x0E).
    fn enc_udiv(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x0Eu32 << 19) | (rs1 << 14) | rs2
    }

    /// Encode `save rs1, imm, rd` (op3=0x3C, i=1).
    fn enc_save(rd: u32, rs1: u32, imm: i32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x3Cu32 << 19) | (rs1 << 14) | (1 << 13) | ((imm as u32) & 0x1FFF)
    }

    /// Encode `restore rs1, rs2, rd` (op3=0x3D).
    fn enc_restore(rd: u32, rs1: u32, rs2: u32) -> u32 {
        (0b10u32 << 30) | (rd << 25) | (0x3Du32 << 19) | (rs1 << 14) | rs2
    }

    fn prog(words: &[u32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(words.len() * 4);
        for &w in words {
            bytes.extend_from_slice(&w.to_be_bytes());
        }
        bytes
    }

    fn run(words: &[u32]) -> SparcCpu {
        let mut cpu = SparcCpu::new();
        let p = prog(words);
        cpu.execute(&p, 0, 10_000).expect("execution failed");
        cpu
    }

    // ── Halt ─────────────────────────────────────────────────────────────────

    #[test]
    fn halts_on_ta0() {
        let cpu = run(&[HALT_WORD]);
        assert!(cpu.halted);
    }

    // ── SETHI + ADD ───────────────────────────────────────────────────────────

    #[test]
    fn sethi_places_value_in_upper_22_bits() {
        // sethi 1, %o0 → %o0 = 1 << 10 = 0x400
        let cpu = run(&[enc_sethi(8, 1), HALT_WORD]);
        assert_eq!(cpu.rf.read(8), 0x400);
    }

    #[test]
    fn add_two_regs() {
        // sethi 0, %o0 = 0; add %o0, %o0, %o1 gives 0+0=0
        // Use immediate: add %g0, 5, %o0; add %g0, 3, %o1; add %o0, %o1, %o2
        let p = prog(&[
            enc_addi(8, 0, 5),  // %o0 = 5
            enc_addi(9, 0, 3),  // %o1 = 3
            enc_add(10, 8, 9),  // %o2 = 8
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 8);
    }

    #[test]
    fn add_immediate() {
        let p = prog(&[enc_addi(8, 0, 42), HALT_WORD]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(8), 42);
    }

    // ── SUB ───────────────────────────────────────────────────────────────────

    #[test]
    fn sub_gives_difference() {
        let p = prog(&[
            enc_addi(8, 0, 10),
            enc_addi(9, 0, 3),
            enc_sub(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 7);
    }

    #[test]
    fn subcc_sets_zero_flag() {
        let p = prog(&[
            enc_addi(8, 0, 5),
            enc_addi(9, 0, 5),
            enc_subcc(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.psr.z, 1);
        assert_eq!(cpu.rf.read(10), 0);
    }

    #[test]
    fn subcc_sets_negative_flag() {
        let p = prog(&[
            enc_addi(8, 0, 3),
            enc_addi(9, 0, 5),
            enc_subcc(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.psr.n, 1);
        assert_eq!(cpu.rf.psr.c, 1); // borrow: 3 < 5
    }

    // ── ADDcc ─────────────────────────────────────────────────────────────────

    #[test]
    fn addcc_carry_on_overflow() {
        // 0xFFFF_FFFF + 1 → carry=1, result=0
        // Load 0xFFFF_FFFF via: add %g0, -1, %o0  (g0=0, simm13=-1 → 0xFFFF_FFFF)
        let p = prog(&[
            enc_addi(8, 0, -1),        // %o0 = 0xFFFF_FFFF
            enc_addi(9, 0, 1),         // %o1 = 1
            enc_addcc(10, 8, 9),       // %o2 = 0; C=1
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 0);
        assert_eq!(cpu.rf.psr.c, 1);
        assert_eq!(cpu.rf.psr.z, 1);
    }

    // ── Logical operations ────────────────────────────────────────────────────

    #[test]
    fn or_combines_bits() {
        let p = prog(&[
            enc_addi(8, 0, 0b1010),
            enc_addi(9, 0, 0b0101),
            enc_or(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 0b1111);
    }

    #[test]
    fn and_masks_bits() {
        let p = prog(&[
            enc_addi(8, 0, 0b1111),
            enc_addi(9, 0, 0b1010),
            enc_and(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 0b1010);
    }

    #[test]
    fn xor_toggles_bits() {
        let p = prog(&[
            enc_addi(8, 0, 0b1010),
            enc_addi(9, 0, 0b1010),
            enc_xor(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 0);
    }

    // ── Shifts ────────────────────────────────────────────────────────────────

    #[test]
    fn sll_shifts_left() {
        let p = prog(&[
            enc_addi(8, 0, 1),
            enc_addi(9, 0, 4),
            enc_sll(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 16);
    }

    #[test]
    fn srl_shifts_right_logical() {
        let p = prog(&[
            enc_addi(8, 0, 0x10),
            enc_addi(9, 0, 4),
            enc_srl(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 1);
    }

    #[test]
    fn sra_preserves_sign() {
        // -8 >> 1 = -4  (arithmetic right shift preserves sign bit)
        let p = prog(&[
            enc_addi(8, 0, -8i32 as u32 as i32),  // %o0 = 0xFFFF_FFF8
            enc_addi(9, 0, 1),
            enc_sra(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10) as i32, -4);
    }

    // ── Load/Store ────────────────────────────────────────────────────────────

    #[test]
    fn store_and_load_word() {
        // Store 0xDEAD_BEEF at address 0x100, then load it back.
        // We need to set %o0 = 0xDEAD_BEEF using sethi + or-immediate.
        // sethi  0xDEAD_B, %o0   → %o0 = 0xDEAD_B000  (upper 22 bits)
        // Hmm, 0xDEAD_BEEF upper 22 = 0xDEAD_B = 0x37_AB5 = 0b11_0111_1010_1011_0101
        // Let's use simpler value: store 42 at address 0x100.
        let p = prog(&[
            enc_addi(8, 0, 42),         // %o0 = 42
            enc_addi(9, 0, 0x100),      // %o1 = 0x100
            enc_st(8, 9, 0),            // ST %o0, [%o1+%g0]
            enc_addi(10, 0, 0),         // %o2 = 0
            enc_ld(10, 9, 0),           // LD [%o1+%g0], %o2
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 42);
    }

    // ── Branch ────────────────────────────────────────────────────────────────

    #[test]
    fn ba_unconditional_branch() {
        // BA skips one instruction (add that would write 99 to %o0).
        // Layout: [0] ba +2 (skip instr at [1]), [1] addi %o0,0,99, [2] addi %o0,0,7, [3] halt
        // disp22 = 2 (skip 2 words forward counting from branch's PC): jump to [2].
        // But SPARC branch displacement is from *current* PC, not next.
        // Our PC after fetch = addr+4, so disp=3 → PC = addr + 3*4 = [0]+12 = [3].
        // Actually we want to skip [1] and run [2]. From [0]: target = [2] = offset +2.
        // We set disp22=2 → target = pc_of_branch + 2*4 = 0 + 8 = [2]. Correct.
        let p = prog(&[
            enc_ba(2),              // [0] BA → jump to [2]
            enc_addi(8, 0, 99),     // [1] skipped
            enc_addi(8, 0, 7),      // [2] %o0 = 7
            HALT_WORD,              // [3]
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(8), 7);
    }

    #[test]
    fn be_taken_when_zero() {
        // subcc %o0, %o0 → Z=1 → be should branch.
        let p = prog(&[
            enc_addi(8, 0, 5),
            enc_subcc(0, 8, 8),  // sets Z=1 (discard to %g0)
            enc_be(2),           // branch +2 = skip [3] → go to [4]
            enc_addi(9, 0, 99),  // [3] skipped
            enc_addi(9, 0, 42),  // [4]
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(9), 42);
    }

    #[test]
    fn bne_not_taken_when_zero() {
        // subcc %o0, %o0 → Z=1 → bne NOT taken.
        let p = prog(&[
            enc_addi(8, 0, 5),
            enc_subcc(0, 8, 8),  // sets Z=1
            enc_bne(2),          // NOT taken
            enc_addi(9, 0, 77),  // [3] executed
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(9), 77);
    }

    // ── JMPL ─────────────────────────────────────────────────────────────────

    #[test]
    fn jmpl_saves_return_address() {
        // jmpl %o1 + 0, %o7 → PC = %o1; %o7 = PC of jmpl.
        // We set %o1 to point at the halt word.
        // [0] addi %o1, 0, 8   (%o1 = 8 = addr of [2])
        // [1] jmpl %o1+0, %o7  (PC=8, %o7=4)
        // [2] halt
        let p = prog(&[
            enc_addi(9, 0, 8),         // [0] %o1 = 8
            enc_jmpl(15, 9, 0),        // [1] JMPL
            HALT_WORD,                 // [2]
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(15), 4); // %o7 = PC of jmpl = 4
        assert!(cpu.halted);
    }

    // ── Multiply ─────────────────────────────────────────────────────────────

    #[test]
    fn umul_basic() {
        // 6 * 7 = 42; Y should be 0 (fits in 32 bits).
        let p = prog(&[
            enc_addi(8, 0, 6),
            enc_addi(9, 0, 7),
            enc_umul(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 42);
        assert_eq!(cpu.rf.y, 0);
    }

    #[test]
    fn umul_large() {
        // 0xFFFF_FFFF * 2 = 0x1_FFFF_FFFE → Y=1, rd=0xFFFF_FFFE.
        // Load 0xFFFF_FFFF via: add %g0, -1, %o0
        let p = prog(&[
            enc_addi(8, 0, -1),        // %o0 = 0xFFFF_FFFF
            enc_addi(9, 0, 2),
            enc_umul(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 0xFFFF_FFFEu32);
        assert_eq!(cpu.rf.y, 1);
    }

    #[test]
    fn smul_negative() {
        // (-1) * 1 = -1 → Y=0xFFFF_FFFF, rd=0xFFFF_FFFF.
        let p = prog(&[
            enc_addi(8, 0, -1i32),     // %o0 = -1 (0xFFFF_FFFF)
            enc_addi(9, 0, 1),
            enc_smul(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 0xFFFF_FFFFu32);
        assert_eq!(cpu.rf.y, 0xFFFF_FFFFu32);
    }

    // ── Divide ────────────────────────────────────────────────────────────────

    #[test]
    fn udiv_basic() {
        // Y=0, %o0=12, %o1=3 → 12/3=4.
        let p = prog(&[
            enc_addi(8, 0, 12),
            enc_addi(9, 0, 3),
            enc_udiv(10, 8, 9),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(10), 4);
    }

    // ── g0 writes are discarded ───────────────────────────────────────────────

    #[test]
    fn g0_write_discarded() {
        // Writing to %g0 should have no effect.
        let p = prog(&[
            enc_addi(0, 0, 99),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(0), 0);
    }

    // ── Load validation ───────────────────────────────────────────────────────

    #[test]
    fn load_rejects_out_of_range_origin() {
        let mut cpu = SparcCpu::new();
        let r = cpu.load(&[0u8; 4], 0x1_0000);
        assert!(r.is_err());
    }

    #[test]
    fn load_rejects_program_that_overflows() {
        let mut cpu = SparcCpu::new();
        let big = vec![0u8; 0x8001];
        let r = cpu.load(&big, 0x8000);
        assert!(r.is_err());
    }

    // ── SAVE / RESTORE ────────────────────────────────────────────────────────

    #[test]
    fn save_restore_round_trip() {
        // SAVE and RESTORE should leave CWP unchanged.
        let p = prog(&[
            enc_save(14, 0, -64),   // SAVE %g0, -64, %sp
            enc_restore(0, 0, 0),   // RESTORE %g0, %g0, %g0
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.cwp, 0);
        assert_eq!(cpu.rf.save_depth, 0);
    }

    #[test]
    fn save_changes_cwp() {
        let p = prog(&[
            enc_save(14, 0, -64),
            HALT_WORD,
        ]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        // CWP decremented: (0 + 3 - 1) % 3 = 2
        assert_eq!(cpu.rf.cwp, 2);
        assert_eq!(cpu.rf.save_depth, 1);
    }

    // ── Fetch wrapping ────────────────────────────────────────────────────────

    #[test]
    fn fetch_wraps_at_boundary() {
        // Place a HALT at 0xFFFC; load origin at 0xFFFC.
        // The fetch should read all 4 bytes and wrap correctly.
        let mut cpu = SparcCpu::new();
        let origin = 0xFFFCu32;
        let h = halt();
        let mut prog_bytes = [0u8; 4];
        prog_bytes.copy_from_slice(&h);
        cpu.load(&prog_bytes, origin).unwrap();
        cpu.step().unwrap();
        assert!(cpu.halted);
    }

    // ── NOP ──────────────────────────────────────────────────────────────────

    #[test]
    fn nop_does_nothing() {
        // Standard NOP = 0x0100_0000.
        let p = prog(&[0x0100_0000, HALT_WORD]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert!(cpu.halted);
    }

    // ── SETHI large value ─────────────────────────────────────────────────────

    #[test]
    fn sethi_large() {
        // sethi 0x3FFFFF, %o0 → %o0 = 0xFFFF_FC00
        let p = prog(&[enc_sethi(8, 0x3F_FFFF), HALT_WORD]);
        let mut cpu = SparcCpu::new();
        cpu.execute(&p, 0, 10_000).unwrap();
        assert_eq!(cpu.rf.read(8), 0xFFFF_FC00);
    }
}
