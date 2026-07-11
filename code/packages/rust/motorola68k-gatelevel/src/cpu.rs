//! Motorola 68000 fetch-decode-execute loop.
//!
//! ## Memory layout
//!
//! ```text
//! 0x000000 – 0x0003FF   Exception vector table
//! 0x001000              Program load address  (LOAD_ADDR)
//! 0x00F000              Initial supervisor stack pointer  (INIT_SP)
//! 0xFFFFFF              Top of 24-bit address space
//! ```
//!
//! ## Big-endian memory
//!
//! The 68000 stores multi-byte values MSB-first (big-endian):
//!
//! ```text
//! 16-bit word at address A:   mem[A]   = high byte,  mem[A+1] = low byte
//! 32-bit long at address A:   mem[A]   = bits 31–24, mem[A+1] = bits 23–16,
//!                             mem[A+2] = bits 15–8,  mem[A+3] = bits 7–0
//! ```
//!
//! Word/long reads/writes must be to even (word-aligned) addresses.
//!
//! ## Effective address (EA) encoding
//!
//! ```text
//! mode  reg   Notation          Description
//! 000   Dn    Dn               Data register direct
//! 001   An    An               Address register direct
//! 010   An    (An)             Address register indirect
//! 011   An    (An)+            Post-increment
//! 100   An    -(An)            Pre-decrement
//! 101   An    d16(An)          16-bit displacement
//! 110   An    d8(An,Xn.sz)     Index + 8-bit displacement
//! 111   000   (abs).W          Absolute short (sign-extended)
//! 111   001   (abs).L          Absolute long
//! 111   010   d16(PC)          PC-relative + 16-bit displacement
//! 111   011   d8(PC,Xn.sz)     PC-relative + index
//! 111   100   #imm             Immediate data
//! ```
//!
//! ## MUL/DIV exception
//!
//! Gate-level implementation of a 16×16-bit multiplier (~1000 gates) is out of
//! scope.  `MULU`, `MULS`, `DIVU`, `DIVS` use host Rust arithmetic.

use crate::alu::{
    add16, add32, add8, and16, and32, and8, cmp16, cmp32, cmp8, neg16, neg32, neg8, negx16,
    negx32, negx8, not16_flags, not32_flags, not8_flags, or16, or32, or8, shift_op, sub16,
    sub32, sub8, xor16, xor32, xor8, AluResult68K,
};
use crate::registers::{RegisterFile68K, ADDR_MASK, BYTE_MASK, LONG_MASK, WORD_MASK};

/// 16 MB address space.
pub const MEM_SIZE: usize = 0x100_0000;
/// Programs load here.
const LOAD_ADDR: usize = 0x001000;
#[allow(dead_code)] // used only in test assertions; dead in lib compilation
const INIT_SP: u32 = 0x00F000;

/// Motorola 68000 CPU — gate-level simulation.
pub struct Cpu68K {
    /// Register file (D0–D7, A0–A7, PC, SR).
    pub rf: RegisterFile68K,
    /// Flat 16 MB memory (big-endian byte order).
    /// Heap-allocated via `vec!` to avoid stack overflow (16 MB > default stack).
    pub mem: Vec<u8>,
    /// True after STOP / TRAP #15.
    pub halted: bool,
}

impl Cpu68K {
    /// Create a new 68000 in power-on state (memory zeroed, registers default).
    pub fn new() -> Self {
        Cpu68K {
            rf: RegisterFile68K::new(),
            mem: vec![0u8; MEM_SIZE],
            halted: false,
        }
    }

    /// Reset CPU state and memory to power-on.
    pub fn reset(&mut self) {
        self.rf = RegisterFile68K::new();
        self.mem.iter_mut().for_each(|b| *b = 0);
        self.halted = false;
    }

    /// Load a program at LOAD_ADDR, clamping to available memory.
    ///
    /// Security note: origin + program.len() is computed with saturating_add to
    /// prevent integer overflow; the result is clamped to MEM_SIZE so no
    /// out-of-bounds write can occur.
    pub fn load(&mut self, program: &[u8]) {
        let end = LOAD_ADDR.saturating_add(program.len()).min(MEM_SIZE);
        let len = end - LOAD_ADDR;
        self.mem[LOAD_ADDR..end].copy_from_slice(&program[..len]);
    }

    /// Reset, load, and run up to `max_steps` instructions.  Returns steps taken.
    ///
    /// ```rust
    /// use coding_adventures_motorola68k_gatelevel::cpu::Cpu68K;
    ///
    /// let mut cpu = Cpu68K::new();
    /// // MOVEQ #5, D0; MOVEQ #3, D1; ADD.L D1, D0; STOP #0x2700
    /// let steps = cpu.execute(&[
    ///     0x70, 0x05,              // MOVEQ #5, D0
    ///     0x72, 0x03,              // MOVEQ #3, D1
    ///     0xD0, 0x81,              // ADD.L D1, D0
    ///     0x4E, 0x72, 0x27, 0x00, // STOP #0x2700
    /// ], 1000);
    /// assert_eq!(cpu.rf.d[0], 8);
    /// assert!(cpu.halted);
    /// ```
    pub fn execute(&mut self, program: &[u8], max_steps: usize) -> usize {
        self.reset();
        self.load(program);
        let mut steps = 0;
        while !self.halted && steps < max_steps {
            self.step();
            steps += 1;
        }
        steps
    }

    /// Execute one instruction.
    pub fn step(&mut self) {
        if self.halted { return; }
        let op = self.fetch_word();
        let hi = (op >> 12) & 0xF;
        match hi {
            0x0 => self.exec_line0(op),
            0x1..=0x3 => self.exec_move(op),
            0x4 => self.exec_line4(op),
            0x5 => self.exec_line5(op),
            0x6 => self.exec_line6(op),
            0x7 => self.exec_moveq(op),
            0x8 => self.exec_line8(op),
            0x9 => self.exec_line9(op),
            0xB => self.exec_line_b(op),
            0xC => self.exec_line_c(op),
            0xD => self.exec_line_d(op),
            0xE => self.exec_line_e(op),
            _   => { self.halted = true; } // unimplemented — halt
        }
    }

    // ── Memory helpers ────────────────────────────────────────────────────────

    fn mem_read_byte(&self, addr: u32) -> u8 {
        self.mem[(addr & ADDR_MASK) as usize]
    }

    fn mem_read_word(&self, addr: u32) -> u16 {
        // Mask each byte address individually so a word fetch at 0xFFFFFF wraps
        // to 0x000000 rather than indexing one past the end of the 16 MB Vec.
        let a0 = (addr.wrapping_add(0) & ADDR_MASK) as usize;
        let a1 = (addr.wrapping_add(1) & ADDR_MASK) as usize;
        ((self.mem[a0] as u16) << 8) | (self.mem[a1] as u16)
    }

    fn mem_read_long(&self, addr: u32) -> u32 {
        let a0 = (addr.wrapping_add(0) & ADDR_MASK) as usize;
        let a1 = (addr.wrapping_add(1) & ADDR_MASK) as usize;
        let a2 = (addr.wrapping_add(2) & ADDR_MASK) as usize;
        let a3 = (addr.wrapping_add(3) & ADDR_MASK) as usize;
        ((self.mem[a0] as u32) << 24)
            | ((self.mem[a1] as u32) << 16)
            | ((self.mem[a2] as u32) << 8)
            | (self.mem[a3] as u32)
    }

    fn mem_read(&self, addr: u32, sz: usize) -> u32 {
        match sz {
            1 => self.mem_read_byte(addr) as u32,
            2 => self.mem_read_word(addr) as u32,
            _ => self.mem_read_long(addr),
        }
    }

    fn mem_write_byte(&mut self, addr: u32, val: u32) {
        self.mem[(addr & ADDR_MASK) as usize] = (val & BYTE_MASK) as u8;
    }

    fn mem_write_word(&mut self, addr: u32, val: u32) {
        let a0 = (addr.wrapping_add(0) & ADDR_MASK) as usize;
        let a1 = (addr.wrapping_add(1) & ADDR_MASK) as usize;
        self.mem[a0] = ((val >> 8) & 0xFF) as u8;
        self.mem[a1] = (val & 0xFF) as u8;
    }

    fn mem_write_long(&mut self, addr: u32, val: u32) {
        let a0 = (addr.wrapping_add(0) & ADDR_MASK) as usize;
        let a1 = (addr.wrapping_add(1) & ADDR_MASK) as usize;
        let a2 = (addr.wrapping_add(2) & ADDR_MASK) as usize;
        let a3 = (addr.wrapping_add(3) & ADDR_MASK) as usize;
        self.mem[a0] = ((val >> 24) & 0xFF) as u8;
        self.mem[a1] = ((val >> 16) & 0xFF) as u8;
        self.mem[a2] = ((val >>  8) & 0xFF) as u8;
        self.mem[a3] = (val & 0xFF) as u8;
    }

    fn mem_write(&mut self, addr: u32, sz: usize, val: u32) {
        match sz {
            1 => self.mem_write_byte(addr, val),
            2 => self.mem_write_word(addr, val),
            _ => self.mem_write_long(addr, val),
        }
    }

    // ── PC fetch helpers ──────────────────────────────────────────────────────

    fn fetch_word(&mut self) -> u16 {
        let w = self.mem_read_word(self.rf.pc);
        self.rf.pc = (self.rf.pc + 2) & ADDR_MASK;
        w
    }

    fn fetch_long(&mut self) -> u32 {
        let v = self.mem_read_long(self.rf.pc);
        self.rf.pc = (self.rf.pc + 4) & ADDR_MASK;
        v
    }

    fn fetch_word_signed(&mut self) -> i32 {
        let w = self.fetch_word() as i16;
        w as i32
    }

    /// Fetch immediate value of `sz` bytes; byte immediates use a 16-bit extension.
    fn fetch_imm(&mut self, sz: usize) -> u32 {
        if sz == 4 {
            self.fetch_long()
        } else {
            let w = self.fetch_word() as u32;
            w & sz_mask(sz)
        }
    }

    // ── Stack helpers ─────────────────────────────────────────────────────────

    fn push_long(&mut self, val: u32) {
        self.rf.a[7] = (self.rf.a[7].wrapping_sub(4)) & ADDR_MASK;
        let sp = self.rf.a[7];
        self.mem_write_long(sp, val);
    }

    fn pop_long(&mut self) -> u32 {
        let sp = self.rf.a[7];
        let val = self.mem_read_long(sp);
        self.rf.a[7] = (self.rf.a[7] + 4) & ADDR_MASK;
        val
    }

    #[allow(dead_code)] // defined for completeness; push_long is sufficient for this ISA subset
    fn push_word(&mut self, val: u32) {
        self.rf.a[7] = (self.rf.a[7].wrapping_sub(2)) & ADDR_MASK;
        let sp = self.rf.a[7];
        self.mem_write_word(sp, val);
    }

    fn pop_word(&mut self) -> u32 {
        let sp = self.rf.a[7];
        let val = self.mem_read_word(sp) as u32;
        self.rf.a[7] = (self.rf.a[7] + 2) & ADDR_MASK;
        val
    }

    // ── Effective address resolution ──────────────────────────────────────────

    /// Compute the memory address for an EA field.
    ///
    /// Updates An for pre/post-increment modes.  Invalid for Dn/An/imm.
    fn ea_address(&mut self, mode: u16, reg: u16, sz: usize) -> u32 {
        match mode {
            2 => self.rf.a[reg as usize] & ADDR_MASK,

            3 => {
                // (An)+ — postincrement: return current An, then increment.
                let addr = self.rf.a[reg as usize] & ADDR_MASK;
                let inc = if reg == 7 { sz.max(2) } else { sz } as u32;
                self.rf.a[reg as usize] = (self.rf.a[reg as usize] + inc) & ADDR_MASK;
                addr
            }

            4 => {
                // -(An) — predecrement: decrement An, then return new An.
                let dec = if reg == 7 { sz.max(2) } else { sz } as u32;
                self.rf.a[reg as usize] = (self.rf.a[reg as usize].wrapping_sub(dec)) & ADDR_MASK;
                self.rf.a[reg as usize] & ADDR_MASK
            }

            5 => {
                // d16(An) — indirect with 16-bit displacement.
                let d16 = self.fetch_word_signed();
                (self.rf.a[reg as usize].wrapping_add(d16 as u32)) & ADDR_MASK
            }

            6 => {
                // d8(An,Xn) — indirect + index + 8-bit displacement.
                let ext = self.fetch_word();
                let d8 = sign_extend_8((ext & 0xFF) as u32);
                let xn_n = ((ext >> 12) & 7) as usize;
                let xn_long = (ext >> 11) & 1; // 0=word, 1=long
                let is_an = (ext >> 15) & 1;
                let xn_val = if is_an == 1 { self.rf.a[xn_n] } else { self.rf.d[xn_n] };
                let xn = if xn_long == 0 {
                    sign_extend_16(xn_val & WORD_MASK)
                } else {
                    xn_val & LONG_MASK
                };
                (self.rf.a[reg as usize].wrapping_add(xn).wrapping_add(d8)) & ADDR_MASK
            }

            7 => {
                match reg {
                    0 => {
                        // (abs).W — absolute short (sign-extended)
                        let w = self.fetch_word();
                        sign_extend_16(w as u32) & ADDR_MASK
                    }
                    1 => self.fetch_long() & ADDR_MASK,   // (abs).L
                    2 => {
                        // d16(PC)
                        let pc_base = self.rf.pc;
                        let d16 = self.fetch_word_signed();
                        (pc_base.wrapping_add(d16 as u32)) & ADDR_MASK
                    }
                    3 => {
                        // d8(PC,Xn)
                        let pc_base = self.rf.pc;
                        let ext = self.fetch_word();
                        let d8 = sign_extend_8((ext & 0xFF) as u32);
                        let xn_n = ((ext >> 12) & 7) as usize;
                        let xn_long = (ext >> 11) & 1;
                        let is_an = (ext >> 15) & 1;
                        let xn_val = if is_an == 1 { self.rf.a[xn_n] } else { self.rf.d[xn_n] };
                        let xn = if xn_long == 0 {
                            sign_extend_16(xn_val & WORD_MASK)
                        } else {
                            xn_val & LONG_MASK
                        };
                        (pc_base.wrapping_add(xn).wrapping_add(d8)) & ADDR_MASK
                    }
                    _ => { self.halted = true; 0 }
                }
            }

            _ => { self.halted = true; 0 }
        }
    }

    /// Read `sz` bytes from EA.  Works for all modes including Dn/An/imm.
    fn ea_read(&mut self, mode: u16, reg: u16, sz: usize) -> u32 {
        match mode {
            0 => self.rf.d[reg as usize] & sz_mask(sz),
            1 => self.rf.a[reg as usize] & LONG_MASK, // An always full 32-bit
            _ => {
                if mode == 7 && reg == 4 {
                    return self.fetch_imm(sz);
                }
                let addr = self.ea_address(mode, reg, sz);
                self.mem_read(addr, sz)
            }
        }
    }

    /// Write `sz` bytes to EA.
    fn ea_write(&mut self, mode: u16, reg: u16, sz: usize, val: u32) {
        match mode {
            0 => self.rf.write_dn(reg as usize, val, sz),
            1 => self.rf.write_an(reg as usize, val, sz),
            _ => {
                let addr = self.ea_address(mode, reg, sz);
                self.mem_write(addr, sz, val);
            }
        }
    }

    /// Read from memory EA; return `(value, address)` for RMW operations.
    ///
    /// Pre/postincrement is applied exactly once.
    fn ea_read_addr(&mut self, mode: u16, reg: u16, sz: usize) -> (u32, u32) {
        let addr = self.ea_address(mode, reg, sz);
        (self.mem_read(addr, sz), addr)
    }

    // ── ALU result → CCR ──────────────────────────────────────────────────────

    fn apply_alu(&mut self, r: &AluResult68K) {
        self.rf.set_nzvc_x(r.flag_n, r.flag_z, r.flag_v, r.flag_c);
    }

    fn apply_logic(&mut self, r: &AluResult68K) {
        // Logic ops: N/Z from result; V=0, C=0; X unchanged.
        self.rf.set_nz_clear_vc(r.flag_n, r.flag_z);
    }

    fn apply_cmp(&mut self, r: &AluResult68K) {
        // CMP: N/Z/V/C from result; X unchanged.
        let old_x = self.rf.flag_x();
        self.rf.set_ccr(old_x, r.flag_n, r.flag_z, r.flag_v, r.flag_c);
    }

    // ── Decode helpers ────────────────────────────────────────────────────────

    fn sz_from_code_arith(code: u16) -> Option<usize> {
        match code { 0 => Some(1), 1 => Some(2), 2 => Some(4), _ => None }
    }

    fn sz_from_code_move(code: u16) -> Option<usize> {
        match code { 1 => Some(1), 3 => Some(2), 2 => Some(4), _ => None }
    }

    // ── Bit operations (BTST/BCHG/BCLR/BSET) ─────────────────────────────────

    fn exec_bit_op(&mut self, bit_n_in: u32, kind: u16, mode: u16, reg: u16) {
        if mode == 0 {
            let bit_n = (bit_n_in & 31) as u8;
            let val = self.rf.d[reg as usize];
            let z_val = (val & (1 << bit_n)) == 0;
            match kind {
                1 => self.rf.d[reg as usize] = val ^ (1 << bit_n),
                2 => self.rf.d[reg as usize] = val & !(1 << bit_n),
                3 => self.rf.d[reg as usize] = val | (1 << bit_n),
                _ => {}
            }
            let old_x = self.rf.flag_x();
            let old_n = self.rf.flag_n();
            let old_v = self.rf.flag_v();
            let old_c = self.rf.flag_c();
            let z = if z_val { 1 } else { 0 };
            self.rf.set_ccr(old_x, old_n, z, old_v, old_c);
        } else {
            let bit_n = (bit_n_in & 7) as u8;
            let addr = self.ea_address(mode, reg, 1);
            let val = self.mem_read_byte(addr);
            let z_val = (val & (1 << bit_n)) == 0;
            match kind {
                1 => self.mem_write_byte(addr, (val ^ (1 << bit_n)) as u32),
                2 => self.mem_write_byte(addr, (val & !(1 << bit_n)) as u32),
                3 => self.mem_write_byte(addr, (val | (1 << bit_n)) as u32),
                _ => {}
            }
            let old_x = self.rf.flag_x();
            let old_n = self.rf.flag_n();
            let old_v = self.rf.flag_v();
            let old_c = self.rf.flag_c();
            let z = if z_val { 1 } else { 0 };
            self.rf.set_ccr(old_x, old_n, z, old_v, old_c);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Line 0 — immediate group (ORI, ANDI, SUBI, ADDI, EORI, CMPI, bit ops)
    // ─────────────────────────────────────────────────────────────────────────

    fn exec_line0(&mut self, op: u16) {
        let sz_code = (op >> 6) & 3;
        let mode    = (op >> 3) & 7;
        let reg     = op & 7;

        // BTST/BCHG/BCLR/BSET immediate bit number: 0000 1000 tt eeee
        if (op & 0xFF00) == 0x0800 {
            let kind  = (op >> 6) & 3;
            let bit_n = self.fetch_word() as u32 & 0x1F;
            self.exec_bit_op(bit_n, kind, mode, reg);
            return;
        }

        // BTST/BCHG/BCLR/BSET register bit number: 0000 rrr1 00 ea
        if (op & 0x0138) == 0x0100 && sz_code <= 3 {
            let dn   = (op >> 9) & 7;
            let kind = (op >> 6) & 3;
            let bit_n = self.rf.d[dn as usize];
            self.exec_bit_op(bit_n, kind, mode, reg);
            return;
        }

        let op8 = (op >> 8) & 0xFF;
        match op8 {
            0x00 => {
                // ORI
                let sz = match Self::sz_from_code_arith(sz_code) { Some(s) => s, None => { self.halted = true; return; }};
                let imm = self.fetch_imm(sz);
                if mode == 7 && reg == 4 {
                    // ORI #imm, CCR
                    let ccr = self.rf.read_ccr() | (imm as u8 & 0x1F);
                    self.rf.write_ccr(ccr);
                    return;
                }
                if mode == 7 && reg == 5 {
                    let sr = self.rf.read_sr() | (imm as u16);
                    self.rf.write_sr(sr);
                    return;
                }
                let val = self.ea_read(mode, reg, sz);
                let result = self.do_or(val, imm, sz);
                self.ea_write(mode, reg, sz, result);
            }
            0x02 => {
                // ANDI
                let sz = match Self::sz_from_code_arith(sz_code) { Some(s) => s, None => { self.halted = true; return; }};
                let imm = self.fetch_imm(sz);
                if mode == 7 && reg == 4 {
                    let ccr = self.rf.read_ccr() & (imm as u8 & 0x1F);
                    self.rf.write_ccr(ccr);
                    return;
                }
                if mode == 7 && reg == 5 {
                    let sr = self.rf.read_sr() & (imm as u16 | 0xFF00);
                    self.rf.write_sr(sr);
                    return;
                }
                let val = self.ea_read(mode, reg, sz);
                let result = self.do_and(val, imm, sz);
                self.ea_write(mode, reg, sz, result);
            }
            0x04 => {
                // SUBI
                let sz = match Self::sz_from_code_arith(sz_code) { Some(s) => s, None => { self.halted = true; return; }};
                let imm = self.fetch_imm(sz);
                let a = self.ea_read(mode, reg, sz);
                let result = self.do_sub(a, imm, sz);
                self.ea_write(mode, reg, sz, result);
            }
            0x06 => {
                // ADDI
                let sz = match Self::sz_from_code_arith(sz_code) { Some(s) => s, None => { self.halted = true; return; }};
                let imm = self.fetch_imm(sz);
                let a = self.ea_read(mode, reg, sz);
                let result = self.do_add(a, imm, sz);
                self.ea_write(mode, reg, sz, result);
            }
            0x0A => {
                // EORI
                let sz = match Self::sz_from_code_arith(sz_code) { Some(s) => s, None => { self.halted = true; return; }};
                let imm = self.fetch_imm(sz);
                if mode == 7 && reg == 4 {
                    let ccr = self.rf.read_ccr() ^ (imm as u8 & 0x1F);
                    self.rf.write_ccr(ccr);
                    return;
                }
                let val = self.ea_read(mode, reg, sz);
                let result = self.do_xor(val, imm, sz);
                self.ea_write(mode, reg, sz, result);
            }
            0x0C => {
                // CMPI
                let sz = match Self::sz_from_code_arith(sz_code) { Some(s) => s, None => { self.halted = true; return; }};
                let imm = self.fetch_imm(sz);
                let a = self.ea_read(mode, reg, sz);
                self.do_cmp(a, imm, sz);
            }
            _ => { self.halted = true; }
        }
    }

    // ── Size-dispatched ALU operations ────────────────────────────────────────

    fn do_add(&mut self, a: u32, b: u32, sz: usize) -> u32 {
        let r = match sz {
            1 => add8(a as u8, b as u8, 0),
            2 => add16(a as u16, b as u16, 0),
            _ => add32(a, b, 0),
        };
        self.apply_alu(&r);
        r.result & sz_mask(sz)
    }

    fn do_sub(&mut self, a: u32, b: u32, sz: usize) -> u32 {
        let r = match sz {
            1 => sub8(a as u8, b as u8, 0),
            2 => sub16(a as u16, b as u16, 0),
            _ => sub32(a, b, 0),
        };
        self.apply_alu(&r);
        r.result & sz_mask(sz)
    }

    fn do_and(&mut self, a: u32, b: u32, sz: usize) -> u32 {
        let r = match sz {
            1 => and8(a as u8, b as u8),
            2 => and16(a as u16, b as u16),
            _ => and32(a, b),
        };
        self.apply_logic(&r);
        r.result & sz_mask(sz)
    }

    fn do_or(&mut self, a: u32, b: u32, sz: usize) -> u32 {
        let r = match sz {
            1 => or8(a as u8, b as u8),
            2 => or16(a as u16, b as u16),
            _ => or32(a, b),
        };
        self.apply_logic(&r);
        r.result & sz_mask(sz)
    }

    fn do_xor(&mut self, a: u32, b: u32, sz: usize) -> u32 {
        let r = match sz {
            1 => xor8(a as u8, b as u8),
            2 => xor16(a as u16, b as u16),
            _ => xor32(a, b),
        };
        self.apply_logic(&r);
        r.result & sz_mask(sz)
    }

    fn do_cmp(&mut self, a: u32, b: u32, sz: usize) {
        let r = match sz {
            1 => cmp8(a as u8, b as u8),
            2 => cmp16(a as u16, b as u16),
            _ => cmp32(a, b),
        };
        self.apply_cmp(&r);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Lines 1/2/3 — MOVE / MOVEA
    // ─────────────────────────────────────────────────────────────────────────

    fn exec_move(&mut self, op: u16) {
        let sz_code  = (op >> 12) & 3;
        let sz       = match Self::sz_from_code_move(sz_code) { Some(s) => s, None => { self.halted = true; return; }};
        let dst_reg  = (op >> 9) & 7;
        let dst_mode = (op >> 6) & 7;
        let src_mode = (op >> 3) & 7;
        let src_reg  = op & 7;

        let val = self.ea_read(src_mode, src_reg, sz);

        if dst_mode == 1 {
            // MOVEA — write to address register; no flags affected.
            self.rf.write_an(dst_reg as usize, val, sz);
        } else {
            // Normal MOVE — set N/Z, clear V/C; X unchanged.
            self.ea_write(dst_mode, dst_reg, sz, val);
            let masked = val & sz_mask(sz);
            let n = ((masked & msb_for_sz(sz)) != 0) as u8;
            let z = (masked == 0) as u8;
            self.rf.set_nz_clear_vc(n, z);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Line 4 — miscellaneous
    // ─────────────────────────────────────────────────────────────────────────

    fn exec_line4(&mut self, op: u16) {
        let sz_code = (op >> 6) & 3;
        let mode    = (op >> 3) & 7;
        let reg     = op & 7;

        // NOP
        if op == 0x4E71 { return; }

        // RESET
        if op == 0x4E70 { return; }

        // RTS
        if op == 0x4E75 {
            self.rf.pc = self.pop_long() & ADDR_MASK;
            return;
        }

        // RTR — pop CCR then PC
        if op == 0x4E77 {
            let ccr = self.pop_word() as u8;
            self.rf.write_ccr(ccr);
            self.rf.pc = self.pop_long() & ADDR_MASK;
            return;
        }

        // STOP #imm
        if op == 0x4E72 {
            let imm = self.fetch_word();
            self.rf.write_sr(imm);
            self.halted = true;
            return;
        }

        // TRAP #n: 0x4E40–0x4E4F
        if (0x4E40..=0x4E4F).contains(&op) {
            let n = op & 0xF;
            if n == 15 {
                self.halted = true;
            } else {
                self.rf.d[7] = n as u32;
            }
            return;
        }

        // LINK An, #d16: 0x4E50–0x4E57
        if (0x4E50..=0x4E57).contains(&op) {
            let n = (op & 7) as usize;
            let disp = self.fetch_word_signed();
            self.push_long(self.rf.a[n]);
            self.rf.a[n] = self.rf.a[7];
            self.rf.a[7] = (self.rf.a[7].wrapping_add(disp as u32)) & ADDR_MASK;
            return;
        }

        // UNLK An: 0x4E58–0x4E5F
        if (0x4E58..=0x4E5F).contains(&op) {
            let n = (op & 7) as usize;
            self.rf.a[7] = self.rf.a[n];
            self.rf.a[n] = self.pop_long();
            return;
        }

        // SWAP Dn: 0x4840–0x4847
        if (0x4840..=0x4847).contains(&op) {
            let n = (op & 7) as usize;
            let val = self.rf.d[n];
            let swapped = ((val >> 16) | ((val & WORD_MASK) << 16)) & LONG_MASK;
            self.rf.d[n] = swapped;
            let r = or32(swapped, 0); // compute N/Z only
            self.apply_logic(&r);
            return;
        }

        // EXT.W Dn: 0x4880–0x4887
        if (0x4880..=0x4887).contains(&op) {
            let n = (op & 7) as usize;
            let b = self.rf.d[n] as u8;
            let w = (b as i8) as i16 as u16 as u32;
            self.rf.write_dn(n, w, 2);
            let r = and16(w as u16, 0xFFFF); // compute N/Z (AND with self = identity)
            let rr = AluResult68K { result: w, ..r };
            self.apply_logic(&rr);
            return;
        }

        // EXT.L Dn: 0x48C0–0x48C7
        if (0x48C0..=0x48C7).contains(&op) {
            let n = (op & 7) as usize;
            let w = self.rf.d[n] as u16;
            let lw = (w as i16) as i32 as u32;
            self.rf.d[n] = lw;
            let r2 = AluResult68K {
                result: lw,
                flag_n: ((lw >> 31) & 1) as u8,
                flag_z: (lw == 0) as u8,
                flag_c: 0, flag_v: 0, flag_x: 0,
            };
            self.apply_logic(&r2);
            return;
        }

        // MOVE SR, Dn: 0x40C0–0x40C7
        if (0x40C0..=0x40C7).contains(&op) {
            let n = (op & 7) as usize;
            self.rf.write_dn(n, self.rf.read_sr() as u32, 2);
            return;
        }

        // MOVE CCR, Dn: 0x42C0–0x42C7
        if (0x42C0..=0x42C7).contains(&op) {
            let n = (op & 7) as usize;
            self.rf.write_dn(n, self.rf.read_ccr() as u32, 2);
            return;
        }

        // MOVE #imm, CCR: 0x44FC
        if op == 0x44FC {
            let imm = self.fetch_word() & 0x1F;
            self.rf.write_ccr(imm as u8);
            return;
        }

        // MOVE #imm, SR: 0x46FC
        if op == 0x46FC {
            let imm = self.fetch_word();
            self.rf.write_sr(imm);
            return;
        }

        // NEGX.sz <ea>: 0100 0000 ss ea
        if (op & 0xFF00) == 0x4000 && sz_code <= 2 {
            let sz = arith_sz(sz_code).unwrap_or_else(|| { self.halted = true; 1 });
            let x = self.rf.flag_x();
            let a = self.ea_read(mode, reg, sz);
            let (result, r) = match sz {
                1 => { let r = negx8(a as u8, x); (r.result, r) }
                2 => { let r = negx16(a as u16, x); (r.result, r) }
                _ => { let r = negx32(a, x); (r.result, r) }
            };
            self.ea_write(mode, reg, sz, result & sz_mask(sz));
            // NEGX: C and X from result; V and N from result; Z only cleared.
            let old_z = self.rf.flag_z();
            let new_z = old_z & r.flag_z; // Z never SET by NEGX
            self.rf.set_ccr(r.flag_c, r.flag_n, new_z, r.flag_v, r.flag_c);
            return;
        }

        // CLR.sz <ea>: 0100 0010 ss ea
        if (op & 0xFF00) == 0x4200 && sz_code <= 2 {
            let sz = arith_sz(sz_code).unwrap_or_else(|| { self.halted = true; 1 });
            self.ea_write(mode, reg, sz, 0);
            let old_x = self.rf.flag_x();
            self.rf.set_ccr(old_x, 0, 1, 0, 0); // N=0, Z=1, V=0, C=0
            return;
        }

        // NEG.sz <ea>: 0100 0100 ss ea
        if (op & 0xFF00) == 0x4400 && sz_code <= 2 {
            let sz = arith_sz(sz_code).unwrap_or_else(|| { self.halted = true; 1 });
            let src = self.ea_read(mode, reg, sz);
            let r = match sz {
                1 => neg8(src as u8),
                2 => neg16(src as u16),
                _ => neg32(src),
            };
            self.ea_write(mode, reg, sz, r.result & sz_mask(sz));
            self.apply_alu(&r);
            return;
        }

        // NOT.sz <ea>: 0100 0110 ss ea
        if (op & 0xFF00) == 0x4600 && sz_code <= 2 {
            let sz = arith_sz(sz_code).unwrap_or_else(|| { self.halted = true; 1 });
            let val = self.ea_read(mode, reg, sz);
            let r = match sz {
                1 => not8_flags(val as u8),
                2 => not16_flags(val as u16),
                _ => not32_flags(val),
            };
            self.ea_write(mode, reg, sz, r.result & sz_mask(sz));
            self.apply_logic(&r);
            return;
        }

        // TST.sz <ea>: 0100 1010 ss ea
        if (op & 0xFF00) == 0x4A00 && sz_code <= 2 {
            let sz = arith_sz(sz_code).unwrap_or_else(|| { self.halted = true; 1 });
            let val = self.ea_read(mode, reg, sz) & sz_mask(sz);
            let n = ((val & msb_for_sz(sz)) != 0) as u8;
            let z = (val == 0) as u8;
            self.rf.set_nz_clear_vc(n, z);
            return;
        }

        // PEA <ea>: 0100 1000 01 mm rrr (mode >= 2)
        if (op & 0xFFC0) == 0x4840 && mode >= 2 {
            let addr = self.ea_address(mode, reg, 4);
            self.push_long(addr);
            return;
        }

        // LEA <ea>, An: 0100 aaa1 11 mm rrr
        if (op & 0x01C0) == 0x01C0 && (op & 0xF000) == 0x4000 && mode >= 2 && !(mode == 7 && reg == 4) {
            let an = ((op >> 9) & 7) as usize;
            let addr = self.ea_address(mode, reg, 4);
            self.rf.a[an] = addr & LONG_MASK;
            return;
        }

        // JSR <ea>: 0100 1110 10 mm rrr
        if (op & 0xFFC0) == 0x4E80 {
            let target = self.ea_address(mode, reg, 4);
            self.push_long(self.rf.pc);
            self.rf.pc = target & ADDR_MASK;
            return;
        }

        // JMP <ea>: 0100 1110 11 mm rrr
        if (op & 0xFFC0) == 0x4EC0 {
            let target = self.ea_address(mode, reg, 4);
            self.rf.pc = target & ADDR_MASK;
            return;
        }

        self.halted = true;
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Line 5 — ADDQ, SUBQ, DBcc, Scc
    // ─────────────────────────────────────────────────────────────────────────

    fn exec_line5(&mut self, op: u16) {
        let sz_code = (op >> 6) & 3;
        let mode    = (op >> 3) & 7;
        let reg     = op & 7;
        let data    = (op >> 9) & 7;
        let imm     = if data == 0 { 8u32 } else { data as u32 };

        // DBcc Dn, #disp: sz_code=3, mode=001
        if sz_code == 3 && mode == 1 {
            let cc = ((op >> 8) & 0xF) as u8;
            let pc_before_ext = self.rf.pc;
            let disp = self.fetch_word_signed();
            let target = (pc_before_ext.wrapping_add(disp as u32)) & ADDR_MASK;
            if !self.rf.test_cc(cc) {
                let n = reg as usize;
                let count = (self.rf.d[n] as u16).wrapping_sub(1);
                self.rf.write_dn(n, count as u32, 2);
                if count != 0xFFFF {
                    self.rf.pc = target;
                }
            }
            return;
        }

        // Scc <ea>: sz_code=3
        if sz_code == 3 {
            let cc = ((op >> 8) & 0xF) as u8;
            let val = if self.rf.test_cc(cc) { 0xFF } else { 0x00 };
            self.ea_write(mode, reg, 1, val);
            return;
        }

        let sz = match arith_sz(sz_code) { Some(s) => s, None => { self.halted = true; return; }};
        let sub = (op >> 8) & 1;

        if sub == 0 {
            // ADDQ
            if mode == 1 {
                self.rf.a[reg as usize] = (self.rf.a[reg as usize] + imm) & LONG_MASK;
                return;
            }
            let a = self.ea_read(mode, reg, sz);
            let result = self.do_add(a, imm, sz);
            self.ea_write(mode, reg, sz, result);
        } else {
            // SUBQ
            if mode == 1 {
                self.rf.a[reg as usize] = (self.rf.a[reg as usize].wrapping_sub(imm)) & LONG_MASK;
                return;
            }
            let a = self.ea_read(mode, reg, sz);
            let result = self.do_sub(a, imm, sz);
            self.ea_write(mode, reg, sz, result);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Line 6 — BRA, BSR, Bcc
    // ─────────────────────────────────────────────────────────────────────────

    fn exec_line6(&mut self, op: u16) {
        let cc    = ((op >> 8) & 0xF) as u8;
        let disp8 = op & 0xFF;
        let pc_base = self.rf.pc;

        let disp: i32 = if disp8 == 0 {
            self.fetch_word_signed()
        } else {
            sign_extend_8(disp8 as u32) as i32
        };

        let target = (pc_base.wrapping_add(disp as u32)) & ADDR_MASK;

        if cc == 0 {
            // BRA
            self.rf.pc = target;
        } else if cc == 1 {
            // BSR
            self.push_long(self.rf.pc);
            self.rf.pc = target;
        } else {
            // Bcc
            if self.rf.test_cc(cc) {
                self.rf.pc = target;
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Line 7 — MOVEQ
    // ─────────────────────────────────────────────────────────────────────────

    fn exec_moveq(&mut self, op: u16) {
        if op & 0x0100 != 0 { self.halted = true; return; }
        let dn  = ((op >> 9) & 7) as usize;
        let imm = sign_extend_8(op as u32 & 0xFF);
        self.rf.d[dn] = imm & LONG_MASK;
        let n = ((imm & 0x8000_0000) != 0) as u8;
        let z = (imm == 0) as u8;
        self.rf.set_nz_clear_vc(n, z);
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Line 8 — OR, DIVU, DIVS
    // ─────────────────────────────────────────────────────────────────────────

    fn exec_line8(&mut self, op: u16) {
        let dn      = (op >> 9) & 7;
        let dir_bit = (op >> 8) & 1;
        let sz_code = (op >> 6) & 3;
        let mode    = (op >> 3) & 7;
        let reg     = op & 7;

        // DIVU.W <ea>, Dn: sz_code=3, dir_bit=0
        if sz_code == 3 && dir_bit == 0 {
            let divisor = self.ea_read(mode, reg, 2) & WORD_MASK;
            if divisor == 0 { self.halted = true; return; }
            let dividend = self.rf.d[dn as usize];
            let quotient  = dividend / divisor;
            let remainder = dividend % divisor;
            if quotient > WORD_MASK {
                // Overflow: V=1, others unchanged
                let old_x = self.rf.flag_x();
                let old_n = self.rf.flag_n();
                let old_z = self.rf.flag_z();
                let old_c = self.rf.flag_c();
                self.rf.set_ccr(old_x, old_n, old_z, 1, old_c);
                return;
            }
            self.rf.d[dn as usize] = ((remainder & WORD_MASK) << 16) | (quotient & WORD_MASK);
            let n = ((quotient & 0x8000) != 0) as u8;
            let z = (quotient == 0) as u8;
            self.rf.set_nz_clear_vc(n, z);
            return;
        }

        // DIVS.W <ea>, Dn: sz_code=3, dir_bit=1
        if sz_code == 3 && dir_bit == 1 {
            let divisor_u = self.ea_read(mode, reg, 2) & WORD_MASK;
            let divisor = divisor_u as i16 as i32;
            if divisor == 0 { self.halted = true; return; }
            let dividend = self.rf.d[dn as usize] as i32;
            let quotient  = dividend / divisor; // truncate toward zero
            let remainder = dividend - quotient * divisor;
            if !(-32768..=32767).contains(&quotient) {
                let old_x = self.rf.flag_x();
                let old_n = self.rf.flag_n();
                let old_z = self.rf.flag_z();
                let old_c = self.rf.flag_c();
                self.rf.set_ccr(old_x, old_n, old_z, 1, old_c);
                return;
            }
            let q = quotient as u32 & WORD_MASK;
            let r = remainder as u32 & WORD_MASK;
            self.rf.d[dn as usize] = (r << 16) | q;
            let n = ((q & 0x8000) != 0) as u8;
            let z = (q == 0) as u8;
            self.rf.set_nz_clear_vc(n, z);
            return;
        }

        // OR
        let sz = match arith_sz(sz_code) { Some(s) => s, None => { self.halted = true; return; }};
        if dir_bit == 0 {
            let b = self.ea_read(mode, reg, sz);
            let a = self.rf.d[dn as usize] & sz_mask(sz);
            let result = self.do_or(a, b, sz);
            self.rf.write_dn(dn as usize, result, sz);
        } else {
            let a = self.rf.d[dn as usize] & sz_mask(sz);
            let (val, addr) = self.ea_read_addr(mode, reg, sz);
            let result = self.do_or(val, a, sz);
            self.mem_write(addr, sz, result);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Line 9 — SUB, SUBA, SUBX
    // ─────────────────────────────────────────────────────────────────────────

    fn exec_line9(&mut self, op: u16) {
        let dn      = (op >> 9) & 7;
        let dir_bit = (op >> 8) & 1;
        let sz_code = (op >> 6) & 3;
        let mode    = (op >> 3) & 7;
        let reg     = op & 7;

        // SUBA.W: sz_code=3, dir_bit=0
        if sz_code == 3 && dir_bit == 0 {
            let src = self.ea_read(mode, reg, 2);
            let src = sign_extend_16(src & WORD_MASK);
            self.rf.a[dn as usize] = (self.rf.a[dn as usize].wrapping_sub(src)) & LONG_MASK;
            return;
        }
        // SUBA.L: sz_code=3, dir_bit=1
        if sz_code == 3 && dir_bit == 1 {
            let src = self.ea_read(mode, reg, 4);
            self.rf.a[dn as usize] = (self.rf.a[dn as usize].wrapping_sub(src)) & LONG_MASK;
            return;
        }

        let sz = match arith_sz(sz_code) { Some(s) => s, None => { self.halted = true; return; }};

        // SUBX: dir_bit=1, mode=0 (register form)
        if dir_bit == 1 && mode == 0 {
            let x = self.rf.flag_x();
            let a = self.rf.d[dn as usize] & sz_mask(sz);
            let b = self.rf.d[reg as usize] & sz_mask(sz);
            let r = match sz {
                1 => sub8(a as u8, b as u8, x),
                2 => sub16(a as u16, b as u16, x),
                _ => sub32(a, b, x),
            };
            self.rf.write_dn(dn as usize, r.result & sz_mask(sz), sz);
            // SUBX Z: only clear, never set
            let old_z = self.rf.flag_z();
            let new_z = old_z & r.flag_z;
            self.rf.set_ccr(r.flag_c, r.flag_n, new_z, r.flag_v, r.flag_c);
            return;
        }

        if dir_bit == 0 {
            // SUB <ea>, Dn
            let b = self.ea_read(mode, reg, sz);
            let a = self.rf.d[dn as usize] & sz_mask(sz);
            let result = self.do_sub(a, b, sz);
            self.rf.write_dn(dn as usize, result, sz);
        } else {
            // SUB Dn, <ea>
            let a = self.rf.d[dn as usize] & sz_mask(sz);
            let (val, addr) = self.ea_read_addr(mode, reg, sz);
            let result = self.do_sub(val, a, sz);
            self.mem_write(addr, sz, result);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Line B — CMP, CMPA, EOR
    // ─────────────────────────────────────────────────────────────────────────

    fn exec_line_b(&mut self, op: u16) {
        let dn      = (op >> 9) & 7;
        let dir_bit = (op >> 8) & 1;
        let sz_code = (op >> 6) & 3;
        let mode    = (op >> 3) & 7;
        let reg     = op & 7;

        // CMPA.W: sz_code=3, dir_bit=0
        if sz_code == 3 && dir_bit == 0 {
            let src = self.ea_read(mode, reg, 2);
            let src = sign_extend_16(src & WORD_MASK);
            let a = self.rf.a[dn as usize] & LONG_MASK;
            let r = cmp32(a, src);
            self.apply_cmp(&r);
            return;
        }
        // CMPA.L: sz_code=3, dir_bit=1
        if sz_code == 3 && dir_bit == 1 {
            let src = self.ea_read(mode, reg, 4);
            let a = self.rf.a[dn as usize] & LONG_MASK;
            let r = cmp32(a, src);
            self.apply_cmp(&r);
            return;
        }

        let sz = match arith_sz(sz_code) { Some(s) => s, None => { self.halted = true; return; }};

        if dir_bit == 0 {
            // CMP <ea>, Dn
            let b = self.ea_read(mode, reg, sz);
            let a = self.rf.d[dn as usize] & sz_mask(sz);
            self.do_cmp(a, b, sz);
        } else {
            // EOR Dn, <ea>
            let a = self.rf.d[dn as usize] & sz_mask(sz);
            if mode == 0 {
                let val = self.rf.d[reg as usize] & sz_mask(sz);
                let result = self.do_xor(val, a, sz);
                self.rf.write_dn(reg as usize, result, sz);
            } else {
                let (val, addr) = self.ea_read_addr(mode, reg, sz);
                let result = self.do_xor(val, a, sz);
                self.mem_write(addr, sz, result);
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Line C — AND, MULU, MULS, EXG
    // ─────────────────────────────────────────────────────────────────────────

    fn exec_line_c(&mut self, op: u16) {
        let dn      = (op >> 9) & 7;
        let dir_bit = (op >> 8) & 1;
        let sz_code = (op >> 6) & 3;
        let mode    = (op >> 3) & 7;
        let reg     = op & 7;

        // EXG
        if (op & 0xF1F8) == 0xC140 {
            let (a, b) = (self.rf.d[dn as usize], self.rf.d[reg as usize]);
            self.rf.d[dn as usize] = b; self.rf.d[reg as usize] = a;
            return;
        }
        if (op & 0xF1F8) == 0xC148 {
            let (a, b) = (self.rf.a[dn as usize], self.rf.a[reg as usize]);
            self.rf.a[dn as usize] = b; self.rf.a[reg as usize] = a;
            return;
        }
        if (op & 0xF1F8) == 0xC188 {
            let (a, b) = (self.rf.d[dn as usize], self.rf.a[reg as usize]);
            self.rf.d[dn as usize] = b; self.rf.a[reg as usize] = a;
            return;
        }

        // MULU.W <ea>, Dn: sz_code=3, dir_bit=0
        if sz_code == 3 && dir_bit == 0 {
            let b = self.ea_read(mode, reg, 2) & WORD_MASK;
            let a = self.rf.d[dn as usize] & WORD_MASK;
            let result = (a * b) & LONG_MASK;
            self.rf.d[dn as usize] = result;
            let n = ((result >> 31) & 1) as u8;
            let z = (result == 0) as u8;
            self.rf.set_nz_clear_vc(n, z);
            return;
        }

        // MULS.W <ea>, Dn: sz_code=3, dir_bit=1
        if sz_code == 3 && dir_bit == 1 {
            let b = (self.ea_read(mode, reg, 2) as u16) as i16 as i32;
            let a = (self.rf.d[dn as usize] as u16) as i16 as i32;
            let result = (a * b) as u32 & LONG_MASK;
            self.rf.d[dn as usize] = result;
            let n = ((result >> 31) & 1) as u8;
            let z = (result == 0) as u8;
            self.rf.set_nz_clear_vc(n, z);
            return;
        }

        // AND
        let sz = match arith_sz(sz_code) { Some(s) => s, None => { self.halted = true; return; }};
        if dir_bit == 0 {
            let b = self.ea_read(mode, reg, sz);
            let a = self.rf.d[dn as usize] & sz_mask(sz);
            let result = self.do_and(a, b, sz);
            self.rf.write_dn(dn as usize, result, sz);
        } else {
            let a = self.rf.d[dn as usize] & sz_mask(sz);
            let (val, addr) = self.ea_read_addr(mode, reg, sz);
            let result = self.do_and(val, a, sz);
            self.mem_write(addr, sz, result);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Line D — ADD, ADDA, ADDX
    // ─────────────────────────────────────────────────────────────────────────

    fn exec_line_d(&mut self, op: u16) {
        let dn      = (op >> 9) & 7;
        let dir_bit = (op >> 8) & 1;
        let sz_code = (op >> 6) & 3;
        let mode    = (op >> 3) & 7;
        let reg     = op & 7;

        // ADDA.W: sz_code=3, dir_bit=0
        if sz_code == 3 && dir_bit == 0 {
            let src = self.ea_read(mode, reg, 2);
            let src = sign_extend_16(src & WORD_MASK);
            self.rf.a[dn as usize] = (self.rf.a[dn as usize] + src) & LONG_MASK;
            return;
        }
        // ADDA.L: sz_code=3, dir_bit=1
        if sz_code == 3 && dir_bit == 1 {
            let src = self.ea_read(mode, reg, 4);
            self.rf.a[dn as usize] = (self.rf.a[dn as usize].wrapping_add(src)) & LONG_MASK;
            return;
        }

        let sz = match arith_sz(sz_code) { Some(s) => s, None => { self.halted = true; return; }};

        // ADDX: dir_bit=1, mode=0 (register form)
        if dir_bit == 1 && mode == 0 {
            let x = self.rf.flag_x();
            let a = self.rf.d[dn as usize] & sz_mask(sz);
            let b = self.rf.d[reg as usize] & sz_mask(sz);
            let r = match sz {
                1 => add8(a as u8, b as u8, x),
                2 => add16(a as u16, b as u16, x),
                _ => add32(a, b, x),
            };
            self.rf.write_dn(dn as usize, r.result & sz_mask(sz), sz);
            let old_z = self.rf.flag_z();
            let new_z = old_z & r.flag_z;
            self.rf.set_ccr(r.flag_c, r.flag_n, new_z, r.flag_v, r.flag_c);
            return;
        }

        if dir_bit == 0 {
            // ADD <ea>, Dn
            let b = self.ea_read(mode, reg, sz);
            let a = self.rf.d[dn as usize] & sz_mask(sz);
            let result = self.do_add(a, b, sz);
            self.rf.write_dn(dn as usize, result, sz);
        } else {
            // ADD Dn, <ea>
            let a = self.rf.d[dn as usize] & sz_mask(sz);
            let (val, addr) = self.ea_read_addr(mode, reg, sz);
            let result = self.do_add(val, a, sz);
            self.mem_write(addr, sz, result);
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Line E — shifts and rotates
    // ─────────────────────────────────────────────────────────────────────────

    fn exec_line_e(&mut self, op: u16) {
        let sz_code = (op >> 6) & 3;

        if sz_code == 3 {
            // Memory shift (always word): 1110 d tt1 11 mm rrr
            let direction  = (op >> 11) & 1;
            let shift_type = ((op >> 9) & 3) as u8;
            let mode       = (op >> 3) & 7;
            let reg        = op & 7;
            let addr = self.ea_address(mode, reg, 2);
            let val  = self.mem_read_word(addr) as u32;
            let x_in = self.rf.flag_x();
            let sr   = shift_op(val, 1, direction == 1, shift_type, 16, x_in);
            self.mem_write_word(addr, sr.result);
            self.rf.set_ccr(sr.flag_x, sr.flag_n, sr.flag_z, sr.flag_v, sr.flag_c);
            return;
        }

        // Register shift/rotate
        let sz         = arith_sz(sz_code).unwrap_or(4);
        let direction  = ((op >> 8) & 1) == 1;
        let reg_count  = ((op >> 5) & 1) == 1;
        let shift_type = ((op >> 3) & 3) as u8;
        let dn         = (op & 7) as usize;
        let cnt_field  = ((op >> 9) & 7) as usize;

        let count = if reg_count {
            self.rf.d[cnt_field] % 64
        } else {
            if cnt_field == 0 { 8 } else { cnt_field as u32 }
        };

        let val = self.rf.d[dn] & sz_mask(sz);
        let x_in = self.rf.flag_x();
        let bits = (sz * 8) as u32;
        let sr  = shift_op(val, count, direction, shift_type, bits, x_in);
        self.rf.write_dn(dn, sr.result, sz);
        self.rf.set_ccr(sr.flag_x, sr.flag_n, sr.flag_z, sr.flag_v, sr.flag_c);
    }
}

// ── Module-level helpers ──────────────────────────────────────────────────────

/// Arithmetic size code → byte count.  Returns `None` for invalid code 3.
fn arith_sz(code: u16) -> Option<usize> {
    match code { 0 => Some(1), 1 => Some(2), 2 => Some(4), _ => None }
}

/// Size mask for `sz` bytes.
fn sz_mask(sz: usize) -> u32 {
    match sz { 1 => BYTE_MASK, 2 => WORD_MASK, _ => LONG_MASK }
}

/// MSB position mask for `sz` bytes.
fn msb_for_sz(sz: usize) -> u32 {
    match sz { 1 => 0x80, 2 => 0x8000, _ => 0x8000_0000 }
}

/// Sign-extend an 8-bit value (in the low byte of a u32) to 32 bits.
fn sign_extend_8(val: u32) -> u32 {
    let b = (val & 0xFF) as i8;
    b as i32 as u32
}

/// Sign-extend a 16-bit value to 32 bits.
fn sign_extend_16(val: u32) -> u32 {
    let w = (val & 0xFFFF) as i16;
    w as i32 as u32
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moveq_add_stop() {
        let mut cpu = Cpu68K::new();
        // MOVEQ #5, D0; MOVEQ #3, D1; ADD.L D1, D0; STOP #0x2700
        let steps = cpu.execute(&[
            0x70, 0x05,              // MOVEQ #5, D0
            0x72, 0x03,              // MOVEQ #3, D1
            0xD0, 0x81,              // ADD.L D1, D0
            0x4E, 0x72, 0x27, 0x00, // STOP #0x2700
        ], 1000);
        assert_eq!(cpu.rf.d[0], 8);
        assert!(cpu.halted);
        assert_eq!(steps, 4);
    }

    #[test]
    fn moveq_negative() {
        let mut cpu = Cpu68K::new();
        // MOVEQ #-1, D0 → 0xFFFFFFFF; TRAP #15 (halts without touching SR)
        cpu.execute(&[0x70, 0xFF, 0x4E, 0x4F], 100);
        assert_eq!(cpu.rf.d[0], 0xFFFF_FFFF);
        assert_eq!(cpu.rf.flag_n(), 1);
    }

    #[test]
    fn addi_byte() {
        let mut cpu = Cpu68K::new();
        // MOVEQ #10, D0; ADDI.B #5, D0; STOP
        cpu.execute(&[
            0x70, 0x0A,              // MOVEQ #10, D0
            0x06, 0x00, 0x00, 0x05, // ADDI.B #5, D0
            0x4E, 0x72, 0x27, 0x00, // STOP
        ], 100);
        assert_eq!(cpu.rf.d[0] & 0xFF, 15);
    }

    #[test]
    fn subi_word() {
        let mut cpu = Cpu68K::new();
        // MOVEQ #20, D1; SUBI.W #7, D1; STOP
        cpu.execute(&[
            0x72, 0x14,              // MOVEQ #20, D1
            0x04, 0x41, 0x00, 0x07, // SUBI.W #7, D1
            0x4E, 0x72, 0x27, 0x00,
        ], 100);
        assert_eq!(cpu.rf.d[1] & 0xFFFF, 13);
        assert_eq!(cpu.rf.flag_c(), 0);
    }

    #[test]
    fn move_byte_mem() {
        let mut cpu = Cpu68K::new();
        cpu.reset();
        // Manually: write 0x42 to memory at 0x2000, then MOVE.B (A0), D0
        cpu.mem[0x2000] = 0x42;
        cpu.rf.a[0] = 0x2000;
        // MOVE.B (A0), D0: opcode 1000 0000 0001 0000 = 0x1010
        cpu.mem[LOAD_ADDR]     = 0x10; // MOVE.B (A0), D0
        cpu.mem[LOAD_ADDR + 1] = 0x10;
        cpu.mem[LOAD_ADDR + 2] = 0x4E; // STOP #0x2700
        cpu.mem[LOAD_ADDR + 3] = 0x72;
        cpu.mem[LOAD_ADDR + 4] = 0x27;
        cpu.mem[LOAD_ADDR + 5] = 0x00;
        cpu.rf.pc = LOAD_ADDR as u32;
        cpu.step();
        assert_eq!(cpu.rf.d[0] & 0xFF, 0x42);
    }

    #[test]
    fn sub_long_borrow() {
        let mut cpu = Cpu68K::new();
        // MOVEQ #3, D0; SUBI.L #10, D0; TRAP #15 (halts without touching SR)
        cpu.execute(&[
            0x70, 0x03,                    // MOVEQ #3, D0
            0x04, 0x80, 0x00, 0x00, 0x00, 0x0A, // SUBI.L #10, D0
            0x4E, 0x4F,                    // TRAP #15
        ], 100);
        assert_eq!(cpu.rf.d[0], 3u32.wrapping_sub(10));
        assert_eq!(cpu.rf.flag_c(), 1); // borrow
    }

    #[test]
    fn and_andi_word() {
        let mut cpu = Cpu68K::new();
        // MOVEQ #0x7F, D0; ANDI.W #0x0F, D0; STOP
        cpu.execute(&[
            0x70, 0x7F,              // MOVEQ #0x7F, D0
            0x02, 0x40, 0x00, 0x0F, // ANDI.W #0x0F, D0
            0x4E, 0x72, 0x27, 0x00,
        ], 100);
        assert_eq!(cpu.rf.d[0] & 0xFFFF, 0x0F);
    }

    #[test]
    fn bne_branch_taken() {
        let mut cpu = Cpu68K::new();
        // MOVEQ #1, D0; CMP.W #1, D0 sets Z=0... wait need nonzero difference
        // MOVEQ #5, D0; SUBI.W #3, D0 → D0=2, Z=0 → BNE should branch
        // BNE +2 (skip HLT); STOP
        cpu.execute(&[
            0x70, 0x05,              // MOVEQ #5, D0
            0x04, 0x40, 0x00, 0x03, // SUBI.W #3, D0  (D0=2, Z=0)
            0x66, 0x02,             // BNE +2 (skip next 2 bytes)
            0x4E, 0x40,             // TRAP #0 (should be skipped)
            0x4E, 0x72, 0x27, 0x00, // STOP
        ], 100);
        assert_eq!(cpu.rf.d[0] & 0xFFFF, 2);
        assert!(cpu.halted);
        // D7 should not have been set to 0 (TRAP#0 was skipped)
        assert_eq!(cpu.rf.d[7], 0);
    }

    #[test]
    fn bra_always() {
        let mut cpu = Cpu68K::new();
        // BRA +4 (jump over TRAP#0 + NOP); TRAP#0; STOP
        cpu.execute(&[
            0x60, 0x04,  // BRA +4
            0x4E, 0x40,  // TRAP #0 (should be skipped)
            0x4E, 0x71,  // NOP (also skipped)
            0x4E, 0x72, 0x27, 0x00, // STOP
        ], 100);
        assert_eq!(cpu.rf.d[7], 0); // TRAP#0 not executed
        assert!(cpu.halted);
    }

    #[test]
    fn push_pop_stack() {
        let mut cpu = Cpu68K::new();
        cpu.push_long(0x1234_5678);
        let val = cpu.pop_long();
        assert_eq!(val, 0x1234_5678);
        // SP should be restored
        assert_eq!(cpu.rf.a[7], INIT_SP);
    }

    #[test]
    fn link_unlk() {
        let mut cpu = Cpu68K::new();
        // LINK A6, #-8 should:
        // 1. Push A6 onto stack
        // 2. A6 = SP
        // 3. SP += -8
        // UNLK A6 should reverse it
        cpu.execute(&[
            0x4E, 0x56, 0xFF, 0xF8, // LINK A6, #-8
            0x4E, 0x5E,             // UNLK A6
            0x4E, 0x72, 0x27, 0x00, // STOP
        ], 100);
        // After LINK/UNLK, SP and A6 should be restored to initial values
        assert_eq!(cpu.rf.a[7], INIT_SP);
        assert_eq!(cpu.rf.a[6], 0);
    }

    #[test]
    fn swap_dn() {
        let mut cpu = Cpu68K::new();
        // Load D2=0x00010002, SWAP D2 → 0x00020001
        cpu.reset();
        cpu.rf.d[2] = 0x0001_0002;
        cpu.mem[LOAD_ADDR]     = 0x48; // SWAP D2
        cpu.mem[LOAD_ADDR + 1] = 0x42;
        cpu.mem[LOAD_ADDR + 2] = 0x4E; // STOP
        cpu.mem[LOAD_ADDR + 3] = 0x72;
        cpu.mem[LOAD_ADDR + 4] = 0x27;
        cpu.mem[LOAD_ADDR + 5] = 0x00;
        cpu.rf.pc = LOAD_ADDR as u32;
        cpu.step();
        assert_eq!(cpu.rf.d[2], 0x0002_0001);
    }

    #[test]
    fn ext_w_dn() {
        let mut cpu = Cpu68K::new();
        // EXT.W D0 with D0=0x000000FF → sign-extend byte to word
        cpu.reset();
        cpu.rf.d[0] = 0x0000_00FF; // low byte = -1 as signed
        cpu.mem[LOAD_ADDR]     = 0x48; // EXT.W D0
        cpu.mem[LOAD_ADDR + 1] = 0x80;
        cpu.mem[LOAD_ADDR + 2] = 0x4E; // STOP
        cpu.mem[LOAD_ADDR + 3] = 0x72;
        cpu.mem[LOAD_ADDR + 4] = 0x27;
        cpu.mem[LOAD_ADDR + 5] = 0x00;
        cpu.rf.pc = LOAD_ADDR as u32;
        cpu.step();
        // 0xFF as i8 = -1, sign-extended to word = 0xFFFF
        assert_eq!(cpu.rf.d[0] & 0xFFFF, 0xFFFF);
    }

    #[test]
    fn neg_long() {
        let mut cpu = Cpu68K::new();
        // MOVEQ #5, D0; NEG.L D0 → -5 = 0xFFFFFFFB; TRAP #15 (halts without touching SR)
        cpu.execute(&[
            0x70, 0x05,  // MOVEQ #5, D0
            0x44, 0x80,  // NEG.L D0
            0x4E, 0x4F,  // TRAP #15
        ], 100);
        assert_eq!(cpu.rf.d[0], 0xFFFF_FFFB);
        assert_eq!(cpu.rf.flag_c(), 1); // result != 0 → C=1
        assert_eq!(cpu.rf.flag_n(), 1);
    }

    #[test]
    fn lsr_byte() {
        let mut cpu = Cpu68K::new();
        // MOVEQ #0x10, D0; LSR.B #1, D0 → D0.byte = 0x08
        cpu.reset();
        cpu.rf.d[0] = 0x10;
        // LSR.B #1, D0: encoding 1110 0010 0000 1000 = 0xE208
        // bit 8 = 0 (right), bit 7-6 = 00 (byte), bit 5 = 0 (imm), bit 4-3 = 01 (LS), bit 2-0 = 000 (D0)
        cpu.mem[LOAD_ADDR]     = 0xE2;
        cpu.mem[LOAD_ADDR + 1] = 0x08;
        cpu.mem[LOAD_ADDR + 2] = 0x4E;
        cpu.mem[LOAD_ADDR + 3] = 0x72;
        cpu.mem[LOAD_ADDR + 4] = 0x27;
        cpu.mem[LOAD_ADDR + 5] = 0x00;
        cpu.rf.pc = LOAD_ADDR as u32;
        cpu.step();
        assert_eq!(cpu.rf.d[0] & 0xFF, 0x08);
        assert_eq!(cpu.rf.flag_c(), 0);
    }

    #[test]
    fn asl_word_overflow() {
        let mut cpu = Cpu68K::new();
        // ASL.W #1, D0 with D0.word = 0x4000 → 0x8000, V=1 (sign bit changed)
        cpu.reset();
        cpu.rf.d[0] = 0x4000;
        // ASL.W #1, D0: encoding 1110 0011 0100 0000 = 0xE340
        cpu.mem[LOAD_ADDR]     = 0xE3;
        cpu.mem[LOAD_ADDR + 1] = 0x40;
        cpu.mem[LOAD_ADDR + 2] = 0x4E;
        cpu.mem[LOAD_ADDR + 3] = 0x72;
        cpu.mem[LOAD_ADDR + 4] = 0x27;
        cpu.mem[LOAD_ADDR + 5] = 0x00;
        cpu.rf.pc = LOAD_ADDR as u32;
        cpu.step();
        assert_eq!(cpu.rf.d[0] & 0xFFFF, 0x8000);
        assert_eq!(cpu.rf.flag_v(), 1); // sign bit changed
    }

    #[test]
    fn dbne_loop() {
        let mut cpu = Cpu68K::new();
        // MOVEQ #3, D0 (loop counter)
        // ADDQ.W #1, D1 (body: increment D1)
        // DBNE D0, #-4 (loop back)
        // STOP
        // DBNE: condition NE: loop while NE is false (Z=0) AND counter != -1.
        // Since we never set Z=1, condition is always "not taken" so loop always iterates.
        // After ADDQ #1, Z=0, NE=true → cc satisfied → DBcc does NOT branch
        // Wait, DBcc: if cc NOT satisfied, decrement and branch. If cc satisfied, fall through.
        // DBNE: if NE (Z=0) is NOT satisfied (Z=1), decrement and branch. Otherwise fall through.
        // With ADDQ producing non-zero, Z=0, NE=true → cc IS satisfied → fall through immediately.
        // So DBNE with a non-zero result falls through without looping. Let me rethink.
        //
        // Use DBEQ: if EQ (Z=1) is not satisfied, decrement and branch.
        // Since Z=0 after ADDQ, EQ is not satisfied → decrement D0 and branch.
        // This loops D0+1 times (from 3 down to -1: 4 iterations).
        // Encoding for DBEQ D0, #disp: 0101 0111 1100 1000 = 0x57C8
        cpu.execute(&[
            0x70, 0x03,        // MOVEQ #3, D0   (counter)
            0x52, 0x41,        // ADDQ.W #1, D1  (body)
            0x57, 0xC8, 0xFF, 0xFC, // DBEQ D0, #-4 (pc_before_ext=0x1006, target=0x1002=ADDQ)
            0x4E, 0x72, 0x27, 0x00, // STOP
        ], 1000);
        // 4 iterations (D0: 3,2,1,0,-1), D1 = 4
        assert_eq!(cpu.rf.d[1] & 0xFFFF, 4);
    }

    #[test]
    fn or_xor_memory() {
        let mut cpu = Cpu68K::new();
        cpu.reset();
        cpu.mem[0x2000] = 0x0F;
        cpu.rf.a[0] = 0x2000;
        cpu.rf.d[0] = 0xF0;
        // OR.B D0, (A0): 0x8110
        cpu.mem[LOAD_ADDR]     = 0x81;
        cpu.mem[LOAD_ADDR + 1] = 0x10;
        cpu.mem[LOAD_ADDR + 2] = 0x4E;
        cpu.mem[LOAD_ADDR + 3] = 0x72;
        cpu.mem[LOAD_ADDR + 4] = 0x27;
        cpu.mem[LOAD_ADDR + 5] = 0x00;
        cpu.rf.pc = LOAD_ADDR as u32;
        cpu.step();
        assert_eq!(cpu.mem[0x2000], 0xFF);
    }

    #[test]
    fn clr_byte() {
        let mut cpu = Cpu68K::new();
        // TRAP #15 halts without writing SR, so CLR's Z=1 is preserved
        cpu.execute(&[
            0x70, 0x7F,  // MOVEQ #0x7F, D0
            0x42, 0x00,  // CLR.B D0
            0x4E, 0x4F,  // TRAP #15
        ], 100);
        assert_eq!(cpu.rf.d[0] & 0xFF, 0);
        assert_eq!(cpu.rf.flag_z(), 1);
    }

    #[test]
    fn tst_negative() {
        let mut cpu = Cpu68K::new();
        // TRAP #15 halts without writing SR, so TST's N=1 is preserved
        cpu.execute(&[
            0x70, 0x80u8 as i8 as u8,  // MOVEQ #-128, D0
            0x4A, 0x00,                  // TST.B D0
            0x4E, 0x4F,                  // TRAP #15
        ], 100);
        assert_eq!(cpu.rf.flag_n(), 1);
        assert_eq!(cpu.rf.flag_z(), 0);
    }

    #[test]
    fn big_endian_word_write() {
        let mut cpu = Cpu68K::new();
        cpu.mem_write_word(0x3000, 0xABCD);
        assert_eq!(cpu.mem[0x3000], 0xAB);
        assert_eq!(cpu.mem[0x3001], 0xCD);
    }

    #[test]
    fn big_endian_long_read() {
        let mut cpu = Cpu68K::new();
        cpu.mem[0x4000] = 0x12;
        cpu.mem[0x4001] = 0x34;
        cpu.mem[0x4002] = 0x56;
        cpu.mem[0x4003] = 0x78;
        assert_eq!(cpu.mem_read_long(0x4000), 0x1234_5678);
    }

    #[test]
    fn movea_sign_extends() {
        let mut cpu = Cpu68K::new();
        // MOVEA.W #0x8000, A0 → A0 should be 0xFFFF8000
        cpu.reset();
        // MOVEA.W immediate: opcode 0011 0001 1111 1100 = 0x307C, then #0x8000
        cpu.mem[LOAD_ADDR]     = 0x30;
        cpu.mem[LOAD_ADDR + 1] = 0x7C;
        cpu.mem[LOAD_ADDR + 2] = 0x80;
        cpu.mem[LOAD_ADDR + 3] = 0x00;
        cpu.mem[LOAD_ADDR + 4] = 0x4E;
        cpu.mem[LOAD_ADDR + 5] = 0x72;
        cpu.mem[LOAD_ADDR + 6] = 0x27;
        cpu.mem[LOAD_ADDR + 7] = 0x00;
        cpu.rf.pc = LOAD_ADDR as u32;
        cpu.step();
        assert_eq!(cpu.rf.a[0], 0xFFFF_8000);
    }

    #[test]
    fn addq_an_no_flags() {
        let mut cpu = Cpu68K::new();
        // ADDQ #4, A1 should NOT set flags
        cpu.reset();
        cpu.rf.a[1] = 0x1000;
        cpu.rf.set_ccr(0, 0, 1, 0, 0); // set Z=1 before
        // ADDQ.L #4, A1: 0101 1000 1000 1001 = 0x5889
        // count=4(100), sub=0(ADDQ), sz=10(long), mode=001(An), reg=001(A1)
        cpu.mem[LOAD_ADDR]     = 0x58;
        cpu.mem[LOAD_ADDR + 1] = 0x89;
        cpu.mem[LOAD_ADDR + 2] = 0x4E;
        cpu.mem[LOAD_ADDR + 3] = 0x72;
        cpu.mem[LOAD_ADDR + 4] = 0x27;
        cpu.mem[LOAD_ADDR + 5] = 0x00;
        cpu.rf.pc = LOAD_ADDR as u32;
        cpu.step();
        assert_eq!(cpu.rf.a[1], 0x1004);
        assert_eq!(cpu.rf.flag_z(), 1); // Z unchanged
    }
}
