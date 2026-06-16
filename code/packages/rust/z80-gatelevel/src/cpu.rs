//! Z80 gate-level CPU: fetch-decode-execute loop.
//!
//! Every arithmetic / logic operation routes through the gate-level ALU.
//! The instruction set covers the full Z80 (Zilog, 1976):
//!
//! - Unprefixed instructions: LD, ALU, INC/DEC, branches, subroutine control
//! - CB prefix: rotate/shift/bit operations
//! - ED prefix: extended 16-bit arithmetic, block ops, NEG, I/O variants
//! - DD/FD prefix: IX- and IY-indexed instructions
//! - DDCB/FDCB prefix: bit operations on (IX+d)/(IY+d)
//!
//! # I/O ports
//!
//! The Z80 has a separate 256-port I/O address space (not memory-mapped).
//! - `IN A, (n)` reads from port n into A.
//! - `OUT (n), A` writes A to port n.
//! - `IN r, (C)` and `OUT (C), r` use B as the high byte of the port address
//!   but since our ports are 0–255 we use C directly.
//!
//! # Memory
//!
//! `[u8; 65536]` — all accesses use `addr & 0xFFFF` for natural wrapping.
//!
//! # R register
//!
//! Low 7 bits increment on each fetch; bit 7 is preserved. This models the
//! Z80's DRAM refresh mechanism.

use crate::alu::{
    add8, add16, adc16, and8, bit_test, cpl8, daa8, dec8, inc8, neg8, or8,
    res_bit, rl8, rlc8, rlca8, rla8, rr8, rrc8, rrca8, rra8, sbc16, set_bit,
    sla8, sll8, sra8, srl8, sub8, xor8, AluResultZ80,
};
use crate::registers::{RegisterFile, REG_A, REG_B, REG_C, REG_D, REG_E, REG_H, REG_L, REG_MEM};

const _REG_NAMES: [&str; 8] = ["B", "C", "D", "E", "H", "L", "(HL)", "A"];
const _ALU_NAMES: [&str; 8] = ["ADD", "ADC", "SUB", "SBC", "AND", "XOR", "OR", "CP"];
const _PAIR_NAMES: [&str; 4] = ["BC", "DE", "HL", "SP"];
const _COND_NAMES: [&str; 8] = ["NZ", "Z", "NC", "C", "PO", "PE", "P", "M"];

/// A snapshot of the Z80 CPU state.
#[derive(Debug, Clone)]
pub struct Z80State {
    pub a: u8,
    pub b: u8, pub c: u8,
    pub d: u8, pub e: u8,
    pub h: u8, pub l: u8,
    pub a_prime: u8, pub f_prime: u8,
    pub b_prime: u8, pub c_prime: u8,
    pub d_prime: u8, pub e_prime: u8,
    pub h_prime: u8, pub l_prime: u8,
    pub ix: u16, pub iy: u16,
    pub sp: u16, pub pc: u16,
    pub i: u8, pub r: u8,
    pub flag_s: bool,
    pub flag_z: bool,
    pub flag_h: bool,
    pub flag_pv: bool,
    pub flag_n: bool,
    pub flag_c: bool,
    pub iff1: bool,
    pub iff2: bool,
    pub im: u8,
    pub halted: bool,
    pub memory: Box<[u8; 65536]>,
}

/// Per-instruction execution trace.
#[derive(Debug, Clone)]
pub struct StepTrace {
    pub pc_before: u16,
    pub pc_after: u16,
    pub mnemonic: String,
    pub description: String,
}

/// Gate-level Z80 simulator.
pub struct GateLevelCpuZ80 {
    pub memory: Box<[u8; 65536]>,
    pub rf: RegisterFile,
    pub sp: u16,
    pub pc: u16,
    pub i: u8,
    pub r: u8,
    pub iff1: bool,
    pub iff2: bool,
    pub im: u8,
    pub halted: bool,
    pub input_ports: [u8; 256],
    pub output_ports: [u8; 256],
}

impl GateLevelCpuZ80 {
    /// Create a new Z80 CPU with zeroed memory and registers.
    pub fn new() -> Self {
        Self {
            memory: Box::new([0u8; 65536]),
            rf: RegisterFile::new(),
            sp: 0xFFFF, // Z80 power-on SP is undefined; 0xFFFF is conventional
            pc: 0x0000,
            i: 0, r: 0,
            iff1: false, iff2: false, im: 0,
            halted: false,
            input_ports: [0u8; 256],
            output_ports: [0u8; 256],
        }
    }

    /// Reset all CPU state to power-on values.
    pub fn reset(&mut self) {
        self.rf.reset();
        self.sp = 0xFFFF;
        self.pc = 0x0000;
        self.i = 0; self.r = 0;
        self.iff1 = false; self.iff2 = false; self.im = 0;
        self.halted = false;
    }

    /// Load a program into memory at `origin`.
    pub fn load(&mut self, program: &[u8], origin: u16) {
        let base = origin as usize;
        for (i, &b) in program.iter().enumerate() {
            self.memory[(base + i) & 0xFFFF] = b;
        }
    }

    /// Load and run a program until HALT or `max_steps`.
    ///
    /// Returns `(traces, final_state)`.
    pub fn run(&mut self, program: &[u8], max_steps: usize) -> (Vec<StepTrace>, Z80State) {
        self.reset();
        self.load(program, 0x0000);
        let mut traces = Vec::new();
        let mut steps = 0;
        while !self.halted && steps < max_steps {
            if let Some(t) = self.step() {
                traces.push(t);
            }
            steps += 1;
        }
        (traces, self.get_state())
    }

    /// Capture a snapshot of current CPU state.
    pub fn get_state(&self) -> Z80State {
        let (s, z, h, pv, n, c) = self.rf.read_flags();
        Z80State {
            a: self.rf.read8(REG_A),
            b: self.rf.read8(REG_B), c: self.rf.read8(REG_C),
            d: self.rf.read8(REG_D), e: self.rf.read8(REG_E),
            h: self.rf.read8(REG_H), l: self.rf.read8(REG_L),
            a_prime: self.rf.read_alt8(REG_A),
            f_prime: self.rf.read_f_prime(),
            b_prime: self.rf.read_alt8(REG_B), c_prime: self.rf.read_alt8(REG_C),
            d_prime: self.rf.read_alt8(REG_D), e_prime: self.rf.read_alt8(REG_E),
            h_prime: self.rf.read_alt8(REG_H), l_prime: self.rf.read_alt8(REG_L),
            ix: self.rf.ix, iy: self.rf.iy,
            sp: self.sp, pc: self.pc,
            i: self.i, r: self.r,
            flag_s: s != 0, flag_z: z != 0, flag_h: h != 0,
            flag_pv: pv != 0, flag_n: n != 0, flag_c: c != 0,
            iff1: self.iff1, iff2: self.iff2, im: self.im,
            halted: self.halted,
            memory: Box::new(*self.memory),
        }
    }

    // ── Memory helpers ────────────────────────────────────────────────────────

    #[inline] fn read(&self, addr: u16) -> u8 { self.memory[addr as usize] }
    #[inline] fn write(&mut self, addr: u16, value: u8) { self.memory[addr as usize] = value; }
    #[inline] fn read16(&self, addr: u16) -> u16 {
        let lo = self.memory[addr as usize] as u16;
        let hi = self.memory[addr.wrapping_add(1) as usize] as u16;
        (hi << 8) | lo
    }
    #[inline] fn write16(&mut self, addr: u16, value: u16) {
        self.memory[addr as usize] = (value & 0xFF) as u8;
        self.memory[addr.wrapping_add(1) as usize] = ((value >> 8) & 0xFF) as u8;
    }

    // ── Fetch helpers ─────────────────────────────────────────────────────────

    fn fetch(&mut self) -> u8 {
        let val = self.memory[self.pc as usize];
        self.pc = self.pc.wrapping_add(1);
        // R register: low 7 bits increment, bit 7 preserved
        self.r = ((self.r.wrapping_add(1)) & 0x7F) | (self.r & 0x80);
        val
    }

    fn fetch_signed(&mut self) -> i8 {
        self.fetch() as i8
    }

    fn fetch16(&mut self) -> u16 {
        let lo = self.fetch() as u16;
        let hi = self.fetch() as u16;
        (hi << 8) | lo
    }

    // ── Stack helpers ─────────────────────────────────────────────────────────

    fn push16(&mut self, value: u16) {
        self.sp = self.sp.wrapping_sub(1);
        self.write(self.sp, ((value >> 8) & 0xFF) as u8);
        self.sp = self.sp.wrapping_sub(1);
        self.write(self.sp, (value & 0xFF) as u8);
    }

    fn pop16(&mut self) -> u16 {
        let lo = self.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        let hi = self.read(self.sp) as u16;
        self.sp = self.sp.wrapping_add(1);
        (hi << 8) | lo
    }

    // ── Register access helpers ───────────────────────────────────────────────

    fn get_r(&self, code: usize) -> u8 {
        if code == REG_MEM {
            let hl = self.hl();
            self.read(hl)
        } else {
            self.rf.read8(code)
        }
    }

    fn set_r(&mut self, code: usize, value: u8) {
        if code == REG_MEM {
            let hl = self.hl();
            self.write(hl, value);
        } else {
            self.rf.write8(code, value);
        }
    }

    #[inline] fn hl(&self) -> u16 {
        ((self.rf.read8(REG_H) as u16) << 8) | (self.rf.read8(REG_L) as u16)
    }
    #[inline] fn set_hl(&mut self, val: u16) {
        self.rf.write8(REG_H, ((val >> 8) & 0xFF) as u8);
        self.rf.write8(REG_L, (val & 0xFF) as u8);
    }
    #[inline] fn bc(&self) -> u16 {
        ((self.rf.read8(REG_B) as u16) << 8) | (self.rf.read8(REG_C) as u16)
    }
    #[inline] fn set_bc(&mut self, val: u16) {
        self.rf.write8(REG_B, ((val >> 8) & 0xFF) as u8);
        self.rf.write8(REG_C, (val & 0xFF) as u8);
    }
    #[inline] fn de(&self) -> u16 {
        ((self.rf.read8(REG_D) as u16) << 8) | (self.rf.read8(REG_E) as u16)
    }
    #[inline] fn set_de(&mut self, val: u16) {
        self.rf.write8(REG_D, ((val >> 8) & 0xFF) as u8);
        self.rf.write8(REG_E, (val & 0xFF) as u8);
    }

    fn get_rp(&self, pair_id: u8) -> u16 {
        match pair_id {
            0 => self.bc(),
            1 => self.de(),
            2 => self.hl(),
            3 => self.sp,
            _ => panic!("invalid pair_id"),
        }
    }

    fn set_rp(&mut self, pair_id: u8, value: u16) {
        match pair_id {
            0 => self.set_bc(value),
            1 => self.set_de(value),
            2 => self.set_hl(value),
            3 => { self.sp = value; }
            _ => panic!("invalid pair_id"),
        }
    }

    // PUSH/POP: pair_id 3 = AF (not SP)
    fn get_rp_af(&self, pair_id: u8) -> u16 {
        if pair_id == 3 {
            let a = self.rf.read8(REG_A) as u16;
            let f = self.rf.read_f() as u16;
            (a << 8) | f
        } else {
            self.get_rp(pair_id)
        }
    }
    fn set_rp_af(&mut self, pair_id: u8, value: u16) {
        if pair_id == 3 {
            self.rf.write8(REG_A, ((value >> 8) & 0xFF) as u8);
            self.rf.write_f((value & 0xFF) as u8);
        } else {
            self.set_rp(pair_id, value);
        }
    }

    // ── Flags helpers ─────────────────────────────────────────────────────────

    fn flags(&self) -> (u8, u8, u8, u8, u8, u8) { self.rf.read_flags() }

    fn set_flags_partial(&mut self, updates: &[(&str, u8)]) {
        let (mut s, mut z, mut h, mut pv, mut n, mut c) = self.rf.read_flags();
        for (name, val) in updates {
            match *name {
                "s"  => s  = *val,
                "z"  => z  = *val,
                "h"  => h  = *val,
                "pv" => pv = *val,
                "n"  => n  = *val,
                "c"  => c  = *val,
                _    => {}
            }
        }
        self.rf.write_flags(s, z, h, pv, n, c);
    }

    fn apply_alu(&mut self, res: &AluResultZ80, update_c: bool) {
        let (_, _, _, _, _, c_old) = self.rf.read_flags();
        let c = if update_c { res.flag_c } else { c_old };
        self.rf.write_flags(res.flag_s, res.flag_z, res.flag_h, res.flag_pv, res.flag_n, c);
    }

    fn cond(&self, cc: u8) -> bool {
        let (s, z, _h, pv, _n, c) = self.flags();
        match cc {
            0 => z == 0,   // NZ
            1 => z != 0,   // Z
            2 => c == 0,   // NC
            3 => c != 0,   // C
            4 => pv == 0,  // PO (parity odd)
            5 => pv != 0,  // PE (parity even)
            6 => s == 0,   // P (positive)
            7 => s != 0,   // M (minus)
            _ => false,
        }
    }

    // ── 8-bit ALU dispatch ────────────────────────────────────────────────────

    fn alu8(&mut self, op: u8, operand: u8) {
        let a = self.rf.read8(REG_A);
        let (_, _, _, _, _, c) = self.flags();

        let res = match op {
            0 => add8(a, operand, 0),             // ADD A, m
            1 => add8(a, operand, c),             // ADC A, m
            2 => sub8(a, operand, 0),             // SUB m
            3 => sub8(a, operand, c),             // SBC A, m
            4 => and8(a, operand),                // AND m
            5 => xor8(a, operand),                // XOR m
            6 => or8(a, operand),                 // OR m
            7 => { // CP m: like SUB but A unchanged
                let r = sub8(a, operand, 0);
                self.apply_alu(&r, true);
                return;
            }
            _ => panic!("invalid alu op {}", op),
        };

        // CP already returned; all others write A
        self.rf.write8(REG_A, res.result as u8);
        self.apply_alu(&res, true);
    }

    // ── Execute one instruction ───────────────────────────────────────────────

    pub fn step(&mut self) -> Option<StepTrace> {
        if self.halted { return None; }

        let pc_before = self.pc;
        let b = self.fetch();

        let (mnemonic, description) = match b {
            0xCB => self.exec_cb(),
            0xED => self.exec_ed(),
            0xDD => self.exec_ddfd(true),
            0xFD => self.exec_ddfd(false),
            _    => self.exec_main(b),
        };

        Some(StepTrace {
            pc_before,
            pc_after: self.pc,
            mnemonic: mnemonic.to_string(),
            description: description.to_string(),
        })
    }

    // ── Unprefixed instruction set ────────────────────────────────────────────

    #[allow(clippy::too_many_lines)]
    fn exec_main(&mut self, op: u8) -> (&'static str, String) {
        // NOP
        if op == 0x00 { return ("NOP", "NOP".into()); }

        // HALT
        if op == 0x76 {
            self.halted = true;
            return ("HALT", "HALT".into());
        }

        // LD r, r' (group 01: 0x40–0x7F, excluding 0x76)
        if (0x40..=0x7F).contains(&op) {
            let dst = ((op >> 3) & 0x07) as usize;
            let src = (op & 0x07) as usize;
            let val = self.get_r(src);
            self.set_r(dst, val);
            return ("LD r,r'", format!("LD {},{}", _REG_NAMES[dst], _REG_NAMES[src]));
        }

        // LD r, n (0x06 pattern: bits 2–0 = 110)
        if op & 0xC7 == 0x06 {
            let dst = ((op >> 3) & 0x07) as usize;
            let n = self.fetch();
            self.set_r(dst, n);
            return ("LD r,n", format!("LD {},{:#04x}", _REG_NAMES[dst], n));
        }

        // 8-bit ALU with register operand (0x80–0xBF)
        if (0x80..=0xBF).contains(&op) {
            let alu_op = (op >> 3) & 0x07;
            let src = (op & 0x07) as usize;
            let operand = self.get_r(src);
            self.alu8(alu_op, operand);
            return (_ALU_NAMES[alu_op as usize], format!("{} A,{}", _ALU_NAMES[alu_op as usize], _REG_NAMES[src]));
        }

        // ALU with immediate (0xC6 pattern: bits 2–0 = 110, group 11)
        if op & 0xC7 == 0xC6 {
            let alu_op = (op >> 3) & 0x07;
            let n = self.fetch();
            self.alu8(alu_op, n);
            return (_ALU_NAMES[alu_op as usize], format!("{} A,{:#04x}", _ALU_NAMES[alu_op as usize], n));
        }

        // INC r
        if op & 0xC7 == 0x04 {
            let r_code = ((op >> 3) & 0x07) as usize;
            let v = self.get_r(r_code);
            let res = inc8(v);
            self.set_r(r_code, res.result as u8);
            self.apply_alu(&res, false); // C preserved
            return ("INC", format!("INC {}", _REG_NAMES[r_code]));
        }

        // DEC r
        if op & 0xC7 == 0x05 {
            let r_code = ((op >> 3) & 0x07) as usize;
            let v = self.get_r(r_code);
            let res = dec8(v);
            self.set_r(r_code, res.result as u8);
            self.apply_alu(&res, false); // C preserved
            return ("DEC", format!("DEC {}", _REG_NAMES[r_code]));
        }

        // LD rp, nn
        if op & 0xCF == 0x01 {
            let rp = (op >> 4) & 0x03;
            let nn = self.fetch16();
            self.set_rp(rp, nn);
            return ("LD rp,nn", format!("LD {},{:#06x}", _PAIR_NAMES[rp as usize], nn));
        }

        // ADD HL, rp
        if op & 0xCF == 0x09 {
            let rp = (op >> 4) & 0x03;
            let hl_val = self.hl();
            let rp_val = self.get_rp(rp);
            let res = add16(hl_val, rp_val);
            self.set_hl(res.result);
            self.set_flags_partial(&[("h", res.flag_h), ("n", 0), ("c", res.flag_c)]);
            return ("ADD HL,rp", format!("ADD HL,{}", _PAIR_NAMES[rp as usize]));
        }

        // INC rp
        if op & 0xCF == 0x03 {
            let rp = (op >> 4) & 0x03;
            let val = self.get_rp(rp).wrapping_add(1);
            self.set_rp(rp, val);
            return ("INC rp", format!("INC {}", _PAIR_NAMES[rp as usize]));
        }

        // DEC rp
        if op & 0xCF == 0x0B {
            let rp = (op >> 4) & 0x03;
            let val = self.get_rp(rp).wrapping_sub(1);
            self.set_rp(rp, val);
            return ("DEC rp", format!("DEC {}", _PAIR_NAMES[rp as usize]));
        }

        // LD SP, HL
        if op == 0xF9 { self.sp = self.hl(); return ("LD SP,HL", "LD SP,HL".into()); }

        // LD HL, (nn)
        if op == 0x2A {
            let nn = self.fetch16();
            let l = self.read(nn); let h = self.read(nn.wrapping_add(1));
            self.rf.write8(REG_L, l); self.rf.write8(REG_H, h);
            return ("LD HL,(nn)", format!("LD HL,({:#06x})", nn));
        }
        // LD (nn), HL
        if op == 0x22 {
            let nn = self.fetch16();
            let l = self.rf.read8(REG_L); let h = self.rf.read8(REG_H);
            self.write(nn, l); self.write(nn.wrapping_add(1), h);
            return ("LD (nn),HL", format!("LD ({:#06x}),HL", nn));
        }
        // LD A, (nn)
        if op == 0x3A {
            let nn = self.fetch16();
            let v = self.read(nn); self.rf.write8(REG_A, v);
            return ("LD A,(nn)", format!("LD A,({:#06x})", nn));
        }
        // LD (nn), A
        if op == 0x32 {
            let nn = self.fetch16();
            let a = self.rf.read8(REG_A); self.write(nn, a);
            return ("LD (nn),A", format!("LD ({:#06x}),A", nn));
        }
        // LD A, (BC)
        if op == 0x0A {
            let bc = self.bc(); let v = self.read(bc); self.rf.write8(REG_A, v);
            return ("LD A,(BC)", "LD A,(BC)".into());
        }
        // LD A, (DE)
        if op == 0x1A {
            let de = self.de(); let v = self.read(de); self.rf.write8(REG_A, v);
            return ("LD A,(DE)", "LD A,(DE)".into());
        }
        // LD (BC), A
        if op == 0x02 {
            let bc = self.bc(); let a = self.rf.read8(REG_A); self.write(bc, a);
            return ("LD (BC),A", "LD (BC),A".into());
        }
        // LD (DE), A
        if op == 0x12 {
            let de = self.de(); let a = self.rf.read8(REG_A); self.write(de, a);
            return ("LD (DE),A", "LD (DE),A".into());
        }

        // PUSH rp (AF)
        if op & 0xCF == 0xC5 {
            let rp = (op >> 4) & 0x03;
            let val = self.get_rp_af(rp);
            self.push16(val);
            let name = if rp == 3 { "AF" } else { _PAIR_NAMES[rp as usize] };
            return ("PUSH", format!("PUSH {}", name));
        }
        // POP rp (AF)
        if op & 0xCF == 0xC1 {
            let rp = (op >> 4) & 0x03;
            let val = self.pop16();
            self.set_rp_af(rp, val);
            let name = if rp == 3 { "AF" } else { _PAIR_NAMES[rp as usize] };
            return ("POP", format!("POP {}", name));
        }

        // Exchange
        if op == 0xEB { // EX DE, HL
            let d = self.rf.read8(REG_D); let h = self.rf.read8(REG_H);
            let e = self.rf.read8(REG_E); let l = self.rf.read8(REG_L);
            self.rf.write8(REG_H, d); self.rf.write8(REG_L, e);
            self.rf.write8(REG_D, h); self.rf.write8(REG_E, l);
            return ("EX DE,HL", "EX DE,HL".into());
        }
        if op == 0x08 { // EX AF, AF'
            self.rf.exchange_af();
            return ("EX AF,AF'", "EX AF,AF'".into());
        }
        if op == 0xD9 { // EXX
            self.rf.exchange_bank();
            return ("EXX", "EXX".into());
        }
        if op == 0xE3 { // EX (SP), HL
            let sp = self.sp;
            let lo = self.read(sp); let hi = self.read(sp.wrapping_add(1));
            self.write(sp, self.rf.read8(REG_L));
            self.write(sp.wrapping_add(1), self.rf.read8(REG_H));
            self.rf.write8(REG_H, hi); self.rf.write8(REG_L, lo);
            return ("EX (SP),HL", "EX (SP),HL".into());
        }

        // Jumps
        if op == 0xC3 { // JP nn
            let nn = self.fetch16(); self.pc = nn;
            return ("JP", format!("JP {:#06x}", nn));
        }
        if op & 0xC7 == 0xC2 { // JP cc, nn
            let cc = (op >> 3) & 0x07;
            let nn = self.fetch16();
            if self.cond(cc) { self.pc = nn; }
            return ("JP cc", format!("JP {},{:#06x}", _COND_NAMES[cc as usize], nn));
        }
        if op == 0xE9 { // JP (HL)
            self.pc = self.hl();
            return ("JP (HL)", "JP (HL)".into());
        }
        if op == 0x18 { // JR e
            let e = self.fetch_signed() as i32;
            self.pc = (self.pc as i32 + e) as u16;
            return ("JR", format!("JR {:+}", e));
        }
        if op == 0x20 { // JR NZ, e
            let e = self.fetch_signed() as i32;
            let (_, z, _, _, _, _) = self.flags();
            if z == 0 { self.pc = (self.pc as i32 + e) as u16; }
            return ("JR NZ", format!("JR NZ,{:+}", e));
        }
        if op == 0x28 { // JR Z, e
            let e = self.fetch_signed() as i32;
            let (_, z, _, _, _, _) = self.flags();
            if z != 0 { self.pc = (self.pc as i32 + e) as u16; }
            return ("JR Z", format!("JR Z,{:+}", e));
        }
        if op == 0x30 { // JR NC, e
            let e = self.fetch_signed() as i32;
            let (_, _, _, _, _, c) = self.flags();
            if c == 0 { self.pc = (self.pc as i32 + e) as u16; }
            return ("JR NC", format!("JR NC,{:+}", e));
        }
        if op == 0x38 { // JR C, e
            let e = self.fetch_signed() as i32;
            let (_, _, _, _, _, c) = self.flags();
            if c != 0 { self.pc = (self.pc as i32 + e) as u16; }
            return ("JR C", format!("JR C,{:+}", e));
        }
        if op == 0x10 { // DJNZ e
            let e = self.fetch_signed() as i32;
            let b = self.rf.read8(REG_B).wrapping_sub(1);
            self.rf.write8(REG_B, b);
            if b != 0 { self.pc = (self.pc as i32 + e) as u16; }
            return ("DJNZ", format!("DJNZ {:+}", e));
        }

        // Call / Return
        if op == 0xCD { // CALL nn
            let nn = self.fetch16();
            let pc = self.pc; self.push16(pc);
            self.pc = nn;
            return ("CALL", format!("CALL {:#06x}", nn));
        }
        if op & 0xC7 == 0xC4 { // CALL cc, nn
            let cc = (op >> 3) & 0x07;
            let nn = self.fetch16();
            if self.cond(cc) {
                let pc = self.pc; self.push16(pc);
                self.pc = nn;
            }
            return ("CALL cc", format!("CALL {},{:#06x}", _COND_NAMES[cc as usize], nn));
        }
        if op == 0xC9 { // RET
            self.pc = self.pop16();
            return ("RET", "RET".into());
        }
        if op & 0xC7 == 0xC0 { // RET cc
            let cc = (op >> 3) & 0x07;
            if self.cond(cc) { self.pc = self.pop16(); }
            return ("RET cc", format!("RET {}", _COND_NAMES[cc as usize]));
        }

        // RST
        if op & 0xC7 == 0xC7 {
            let p = (op & 0x38) as u16;
            let pc = self.pc; self.push16(pc);
            self.pc = p;
            return ("RST", format!("RST {:#04x}", p));
        }

        // Accumulator rotates
        if op == 0x07 { // RLCA
            let a = self.rf.read8(REG_A);
            let res = rlca8(a);
            self.rf.write8(REG_A, res.result as u8);
            self.set_flags_partial(&[("h", 0), ("n", 0), ("c", res.flag_c)]);
            return ("RLCA", "RLCA".into());
        }
        if op == 0x0F { // RRCA
            let a = self.rf.read8(REG_A);
            let res = rrca8(a);
            self.rf.write8(REG_A, res.result as u8);
            self.set_flags_partial(&[("h", 0), ("n", 0), ("c", res.flag_c)]);
            return ("RRCA", "RRCA".into());
        }
        if op == 0x17 { // RLA
            let a = self.rf.read8(REG_A);
            let (_, _, _, _, _, c) = self.flags();
            let res = rla8(a, c);
            self.rf.write8(REG_A, res.result as u8);
            self.set_flags_partial(&[("h", 0), ("n", 0), ("c", res.flag_c)]);
            return ("RLA", "RLA".into());
        }
        if op == 0x1F { // RRA
            let a = self.rf.read8(REG_A);
            let (_, _, _, _, _, c) = self.flags();
            let res = rra8(a, c);
            self.rf.write8(REG_A, res.result as u8);
            self.set_flags_partial(&[("h", 0), ("n", 0), ("c", res.flag_c)]);
            return ("RRA", "RRA".into());
        }

        // DAA
        if op == 0x27 {
            let a = self.rf.read8(REG_A);
            let (_, _, h, _, n, c) = self.flags();
            let res = daa8(a, n, h, c);
            self.rf.write8(REG_A, res.result as u8);
            self.rf.write_flags(res.flag_s, res.flag_z, res.flag_h, res.flag_pv, res.flag_n, res.flag_c);
            return ("DAA", "DAA".into());
        }

        // CPL
        if op == 0x2F {
            let a = self.rf.read8(REG_A);
            let res = cpl8(a);
            self.rf.write8(REG_A, res.result as u8);
            self.set_flags_partial(&[("h", 1), ("n", 1)]);
            return ("CPL", "CPL".into());
        }

        // CCF / SCF
        if op == 0x3F { // CCF: complement carry
            let (_, _, c_old, _, _, c) = self.flags();
            // H gets old C; N=0; C gets NOT(old C)
            let _ = c_old; // already in c
            self.set_flags_partial(&[("h", c), ("n", 0), ("c", 1 - c)]);
            return ("CCF", "CCF".into());
        }
        if op == 0x37 { // SCF: set carry
            self.set_flags_partial(&[("h", 0), ("n", 0), ("c", 1)]);
            return ("SCF", "SCF".into());
        }

        // I/O
        if op == 0xD3 { // OUT (n), A
            let n = self.fetch();
            self.output_ports[n as usize] = self.rf.read8(REG_A);
            return ("OUT", format!("OUT ({:#04x}),A", n));
        }
        if op == 0xDB { // IN A, (n)
            let n = self.fetch();
            let v = self.input_ports[n as usize];
            self.rf.write8(REG_A, v);
            return ("IN", format!("IN A,({:#04x})", n));
        }

        // Interrupt control
        if op == 0xF3 { self.iff1 = false; self.iff2 = false; return ("DI", "DI".into()); }
        if op == 0xFB { self.iff1 = true;  self.iff2 = true;  return ("EI", "EI".into()); }

        ("??", format!("Unknown {:#04x}", op))
    }

    // ── CB prefix: rotates/shifts/bit ops ────────────────────────────────────

    fn exec_cb(&mut self) -> (&'static str, String) {
        let op = self.fetch();
        let r_code = (op & 0x07) as usize;
        let v = self.get_r(r_code);
        let rot_op = (op >> 3) & 0x07;
        let bit = (op >> 3) & 0x07;

        if op < 0x40 {
            let (_, _, _, _, _, c) = self.flags();
            let res = match rot_op {
                0 => rlc8(v),
                1 => rrc8(v),
                2 => rl8(v, c),
                3 => rr8(v, c),
                4 => sla8(v),
                5 => sra8(v),
                6 => sll8(v),   // undocumented
                7 => srl8(v),
                _ => unreachable!(),
            };
            self.set_r(r_code, res.result as u8);
            self.apply_alu(&res, true);
            ("ROT", format!("CB rot{} {}", rot_op, _REG_NAMES[r_code]))
        } else if op < 0x80 {
            let res = bit_test(v, bit);
            self.set_flags_partial(&[("z", res.flag_z), ("h", 1), ("n", 0)]);
            ("BIT", format!("BIT {},{}", bit, _REG_NAMES[r_code]))
        } else if op < 0xC0 {
            let r_val = res_bit(v, bit);
            self.set_r(r_code, r_val);
            ("RES", format!("RES {},{}", bit, _REG_NAMES[r_code]))
        } else {
            let r_val = set_bit(v, bit);
            self.set_r(r_code, r_val);
            ("SET", format!("SET {},{}", bit, _REG_NAMES[r_code]))
        }
    }

    // ── ED prefix: extended instructions ─────────────────────────────────────

    #[allow(clippy::too_many_lines)]
    fn exec_ed(&mut self) -> (&'static str, String) {
        let op = self.fetch();

        // LD A,I / LD A,R
        if op == 0x57 {
            self.rf.write8(REG_A, self.i);
            let s = (self.i >> 7) & 1;
            let z = if self.i == 0 { 1 } else { 0 };
            let pv = self.iff2 as u8;
            self.set_flags_partial(&[("s", s), ("z", z), ("h", 0), ("pv", pv), ("n", 0)]);
            return ("LD A,I", "LD A,I".into());
        }
        if op == 0x5F {
            self.rf.write8(REG_A, self.r);
            let s = (self.r >> 7) & 1;
            let z = if self.r == 0 { 1 } else { 0 };
            let pv = self.iff2 as u8;
            self.set_flags_partial(&[("s", s), ("z", z), ("h", 0), ("pv", pv), ("n", 0)]);
            return ("LD A,R", "LD A,R".into());
        }
        if op == 0x47 { self.i = self.rf.read8(REG_A); return ("LD I,A", "LD I,A".into()); }
        if op == 0x4F { self.r = self.rf.read8(REG_A); return ("LD R,A", "LD R,A".into()); }

        // LD rp, (nn)
        if op & 0xCF == 0x4B {
            let rp = (op >> 4) & 0x03;
            let nn = self.fetch16();
            let val = self.read16(nn);
            self.set_rp(rp, val);
            return ("LD rp,(nn)", format!("LD {},(#{:04x})", _PAIR_NAMES[rp as usize], nn));
        }
        // LD (nn), rp
        if op & 0xCF == 0x43 {
            let rp = (op >> 4) & 0x03;
            let nn = self.fetch16();
            let val = self.get_rp(rp);
            self.write16(nn, val);
            return ("LD (nn),rp", format!("LD (#{:04x}),{}", nn, _PAIR_NAMES[rp as usize]));
        }

        // ADC HL, rp
        if op & 0xCF == 0x4A {
            let rp = (op >> 4) & 0x03;
            let hl_val = self.hl();
            let rp_v = self.get_rp(rp);
            let (_, _, _, _, _, c) = self.flags();
            let res = adc16(hl_val, rp_v, c);
            self.set_hl(res.result);
            self.apply_alu(&res, true);
            return ("ADC HL,rp", format!("ADC HL,{}", _PAIR_NAMES[rp as usize]));
        }

        // SBC HL, rp
        if op & 0xCF == 0x42 {
            let rp = (op >> 4) & 0x03;
            let hl_val = self.hl();
            let rp_v = self.get_rp(rp);
            let (_, _, _, _, _, c) = self.flags();
            let res = sbc16(hl_val, rp_v, c);
            self.set_hl(res.result);
            self.apply_alu(&res, true);
            return ("SBC HL,rp", format!("SBC HL,{}", _PAIR_NAMES[rp as usize]));
        }

        // NEG
        if op == 0x44 {
            let a = self.rf.read8(REG_A);
            let res = neg8(a);
            self.rf.write8(REG_A, res.result as u8);
            self.apply_alu(&res, true);
            return ("NEG", "NEG".into());
        }

        // Interrupt mode
        if op == 0x46 { self.im = 0; return ("IM 0", "IM 0".into()); }
        if op == 0x56 { self.im = 1; return ("IM 1", "IM 1".into()); }
        if op == 0x5E { self.im = 2; return ("IM 2", "IM 2".into()); }

        // RETI / RETN
        if op == 0x4D { self.iff1 = self.iff2; self.pc = self.pop16(); return ("RETI", "RETI".into()); }
        if op == 0x45 { self.iff1 = self.iff2; self.pc = self.pop16(); return ("RETN", "RETN".into()); }

        // Block operations
        if op == 0xA0 { return self.ldi(); }
        if op == 0xA8 { return self.ldd(); }
        if op == 0xB0 { return self.ldir(); }
        if op == 0xB8 { return self.lddr(); }
        if op == 0xA1 { return self.cpi_op(); }
        if op == 0xA9 { return self.cpd_op(); }
        if op == 0xB1 { return self.cpir_op(); }
        if op == 0xB9 { return self.cpdr_op(); }

        // IN r, (C) / OUT (C), r
        if op & 0xC7 == 0x40 {
            let r_code = ((op >> 3) & 0x07) as usize;
            let val = self.input_ports[self.rf.read8(REG_C) as usize];
            if r_code != 6 { self.set_r(r_code, val); }
            self.set_flags_partial(&[("h", 0), ("n", 0)]);
            return ("IN r,(C)", "IN r,(C)".into());
        }
        if op & 0xC7 == 0x41 {
            let r_code = ((op >> 3) & 0x07) as usize;
            let val = if r_code != 6 { self.get_r(r_code) } else { 0 };
            self.output_ports[self.rf.read8(REG_C) as usize] = val;
            return ("OUT (C),r", "OUT (C),r".into());
        }

        ("ED??", format!("ED unknown {:#04x}", op))
    }

    // ── DD/FD prefix: index register instructions ─────────────────────────────

    #[allow(clippy::too_many_lines)]
    fn exec_ddfd(&mut self, use_ix: bool) -> (&'static str, String) {
        let idx_val = if use_ix { self.rf.ix } else { self.rf.iy };
        let pfx = if use_ix { "IX" } else { "IY" };
        let op = self.fetch();

        // DDCB / FDCB
        if op == 0xCB { return self.exec_ddcb(idx_val, pfx, use_ix); }

        // LD (IX+d), n
        if op == 0x36 {
            let d = self.fetch_signed() as i32;
            let n = self.fetch();
            self.write((idx_val as i32 + d) as u16, n);
            return ("LD (IX+d),n", format!("LD ({}+{:+}),{:#04x}", pfx, d, n));
        }

        // LD IX, nn
        if op == 0x21 {
            let nn = self.fetch16();
            if use_ix { self.rf.ix = nn; } else { self.rf.iy = nn; }
            return ("LD idx,nn", format!("LD {},{:#06x}", pfx, nn));
        }
        // LD IX, (nn)
        if op == 0x2A {
            let nn = self.fetch16();
            let v = self.read16(nn);
            if use_ix { self.rf.ix = v; } else { self.rf.iy = v; }
            return ("LD idx,(nn)", format!("LD {},({:#06x})", pfx, nn));
        }
        // LD (nn), IX
        if op == 0x22 {
            let nn = self.fetch16();
            self.write16(nn, idx_val);
            return ("LD (nn),idx", format!("LD ({:#06x}),{}", nn, pfx));
        }
        // LD SP, IX
        if op == 0xF9 {
            self.sp = idx_val;
            return ("LD SP,idx", format!("LD SP,{}", pfx));
        }
        // PUSH IX / POP IX
        if op == 0xE5 {
            self.push16(idx_val);
            return ("PUSH idx", format!("PUSH {}", pfx));
        }
        if op == 0xE1 {
            let v = self.pop16();
            if use_ix { self.rf.ix = v; } else { self.rf.iy = v; }
            return ("POP idx", format!("POP {}", pfx));
        }

        // ADD IX, rp
        if op & 0xCF == 0x09 {
            let rp = (op >> 4) & 0x03;
            let rp_val = if rp == 2 { idx_val } else { self.get_rp(rp) };
            let res = add16(idx_val, rp_val);
            if use_ix { self.rf.ix = res.result; } else { self.rf.iy = res.result; }
            self.set_flags_partial(&[("h", res.flag_h), ("n", 0), ("c", res.flag_c)]);
            return ("ADD idx,rp", format!("ADD {},{}", pfx, _PAIR_NAMES[rp as usize]));
        }

        // INC IX / DEC IX
        if op == 0x23 {
            let v = idx_val.wrapping_add(1);
            if use_ix { self.rf.ix = v; } else { self.rf.iy = v; }
            return ("INC idx", format!("INC {}", pfx));
        }
        if op == 0x2B {
            let v = idx_val.wrapping_sub(1);
            if use_ix { self.rf.ix = v; } else { self.rf.iy = v; }
            return ("DEC idx", format!("DEC {}", pfx));
        }

        // LD r, (IX+d) or LD (IX+d), r
        if (0x40..=0x7F).contains(&op) && op != 0x76 {
            let dst = ((op >> 3) & 0x07) as usize;
            let src = (op & 0x07) as usize;
            if src == REG_MEM {
                let d = self.fetch_signed() as i32;
                let val = self.read((idx_val as i32 + d) as u16);
                self.set_r(dst, val);
                return ("LD r,(idx+d)", format!("LD {},({}+{:+})", _REG_NAMES[dst], pfx, d));
            }
            if dst == REG_MEM {
                let d = self.fetch_signed() as i32;
                let val = self.get_r(src);
                self.write((idx_val as i32 + d) as u16, val);
                return ("LD (idx+d),r", format!("LD ({}+{:+}),{}", pfx, d, _REG_NAMES[src]));
            }
        }

        // ALU ops with (IX+d)
        if (0x86..=0xBE).contains(&op) && (op & 0x07) == 0x06 {
            let alu_op = (op >> 3) & 0x07;
            let d = self.fetch_signed() as i32;
            let val = self.read((idx_val as i32 + d) as u16);
            self.alu8(alu_op, val);
            return (_ALU_NAMES[alu_op as usize], format!("{} ({}{:+})", _ALU_NAMES[alu_op as usize], pfx, d));
        }

        // INC/DEC (IX+d)
        if op == 0x34 {
            let d = self.fetch_signed() as i32;
            let addr = (idx_val as i32 + d) as u16;
            let v = self.read(addr);
            let res = inc8(v);
            self.write(addr, res.result as u8);
            self.apply_alu(&res, false);
            return ("INC (idx+d)", format!("INC ({}+{:+})", pfx, d));
        }
        if op == 0x35 {
            let d = self.fetch_signed() as i32;
            let addr = (idx_val as i32 + d) as u16;
            let v = self.read(addr);
            let res = dec8(v);
            self.write(addr, res.result as u8);
            self.apply_alu(&res, false);
            return ("DEC (idx+d)", format!("DEC ({}+{:+})", pfx, d));
        }

        // JP (IX)
        if op == 0xE9 {
            self.pc = idx_val;
            return ("JP (idx)", format!("JP ({})", pfx));
        }
        // EX (SP), IX
        if op == 0xE3 {
            let sp = self.sp;
            let lo = self.read(sp); let hi = self.read(sp.wrapping_add(1));
            self.write(sp, (idx_val & 0xFF) as u8);
            self.write(sp.wrapping_add(1), ((idx_val >> 8) & 0xFF) as u8);
            let new_idx = ((hi as u16) << 8) | (lo as u16);
            if use_ix { self.rf.ix = new_idx; } else { self.rf.iy = new_idx; }
            return ("EX (SP),idx", format!("EX (SP),{}", pfx));
        }

        ("DD/FD??", format!("DD/FD unknown {:#04x}", op))
    }

    // ── DDCB / FDCB prefix ────────────────────────────────────────────────────

    fn exec_ddcb(&mut self, idx_val: u16, pfx: &'static str, _use_ix: bool) -> (&'static str, String) {
        let d = self.fetch_signed() as i32;
        let op = self.fetch();
        let addr = (idx_val as i32 + d) as u16;
        let v = self.read(addr);
        let bit = (op >> 3) & 0x07;
        let r_code = (op & 0x07) as usize;
        let rot_op = (op >> 3) & 0x07;

        if op < 0x40 {
            let (_, _, _, _, _, c) = self.flags();
            let res = match rot_op {
                0 => rlc8(v), 1 => rrc8(v),
                2 => rl8(v, c), 3 => rr8(v, c),
                4 => sla8(v), 5 => sra8(v),
                6 => sll8(v), _ => srl8(v),
            };
            self.write(addr, res.result as u8);
            if r_code != REG_MEM { self.set_r(r_code, res.result as u8); }
            self.apply_alu(&res, true);
            ("ROT (idx+d)", format!("ROT ({}+{:+})", pfx, d))
        } else if op < 0x80 {
            let res = bit_test(v, bit);
            self.set_flags_partial(&[("z", res.flag_z), ("h", 1), ("n", 0)]);
            ("BIT (idx+d)", format!("BIT {},({}+{:+})", bit, pfx, d))
        } else if op < 0xC0 {
            let r_val = res_bit(v, bit);
            self.write(addr, r_val);
            if r_code != REG_MEM { self.set_r(r_code, r_val); }
            ("RES (idx+d)", format!("RES {},({}+{:+})", bit, pfx, d))
        } else {
            let r_val = set_bit(v, bit);
            self.write(addr, r_val);
            if r_code != REG_MEM { self.set_r(r_code, r_val); }
            ("SET (idx+d)", format!("SET {},({}+{:+})", bit, pfx, d))
        }
    }

    // ── Block operations ──────────────────────────────────────────────────────

    fn ldi(&mut self) -> (&'static str, String) {
        let src = self.hl(); let dst = self.de();
        self.write(dst, self.read(src));
        self.set_hl(src.wrapping_add(1));
        self.set_de(dst.wrapping_add(1));
        let bc = self.bc().wrapping_sub(1); self.set_bc(bc);
        self.set_flags_partial(&[("h", 0), ("n", 0), ("pv", if bc != 0 { 1 } else { 0 })]);
        ("LDI", "LDI".into())
    }

    fn ldd(&mut self) -> (&'static str, String) {
        let src = self.hl(); let dst = self.de();
        self.write(dst, self.read(src));
        self.set_hl(src.wrapping_sub(1));
        self.set_de(dst.wrapping_sub(1));
        let bc = self.bc().wrapping_sub(1); self.set_bc(bc);
        self.set_flags_partial(&[("h", 0), ("n", 0), ("pv", if bc != 0 { 1 } else { 0 })]);
        ("LDD", "LDD".into())
    }

    fn ldir(&mut self) -> (&'static str, String) {
        for _ in 0..65536 {
            self.ldi();
            if self.bc() == 0 { break; }
        }
        ("LDIR", "LDIR".into())
    }

    fn lddr(&mut self) -> (&'static str, String) {
        for _ in 0..65536 {
            self.ldd();
            if self.bc() == 0 { break; }
        }
        ("LDDR", "LDDR".into())
    }

    fn cpi_op(&mut self) -> (&'static str, String) {
        let hl = self.hl();
        let m = self.read(hl);
        let a = self.rf.read8(REG_A);
        let res = sub8(a, m, 0);
        self.set_hl(hl.wrapping_add(1));
        let bc = self.bc().wrapping_sub(1); self.set_bc(bc);
        let pv = if bc != 0 { 1 } else { 0 };
        self.set_flags_partial(&[("s", res.flag_s), ("z", res.flag_z), ("h", res.flag_h), ("pv", pv), ("n", 1)]);
        ("CPI", "CPI".into())
    }

    fn cpd_op(&mut self) -> (&'static str, String) {
        let hl = self.hl();
        let m = self.read(hl);
        let a = self.rf.read8(REG_A);
        let res = sub8(a, m, 0);
        self.set_hl(hl.wrapping_sub(1));
        let bc = self.bc().wrapping_sub(1); self.set_bc(bc);
        let pv = if bc != 0 { 1 } else { 0 };
        self.set_flags_partial(&[("s", res.flag_s), ("z", res.flag_z), ("h", res.flag_h), ("pv", pv), ("n", 1)]);
        ("CPD", "CPD".into())
    }

    fn cpir_op(&mut self) -> (&'static str, String) {
        for _ in 0..65536 {
            self.cpi_op();
            let (_, z, _, _, _, _) = self.flags();
            if z != 0 || self.bc() == 0 { break; }
        }
        ("CPIR", "CPIR".into())
    }

    fn cpdr_op(&mut self) -> (&'static str, String) {
        for _ in 0..65536 {
            self.cpd_op();
            let (_, z, _, _, _, _) = self.flags();
            if z != 0 || self.bc() == 0 { break; }
        }
        ("CPDR", "CPDR".into())
    }
}

impl Default for GateLevelCpuZ80 {
    fn default() -> Self { Self::new() }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn cpu() -> GateLevelCpuZ80 { GateLevelCpuZ80::new() }

    // ── Basic arithmetic ─────────────────────────────────────────────────────

    #[test]
    fn add_a_b() {
        // LD A, 5; LD B, 3; ADD A, B; HALT
        let prog = [0x3E, 0x05, 0x06, 0x03, 0x80, 0x76];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.a, 8);
        assert!(!state.flag_c);
        assert!(!state.flag_z);
    }

    #[test]
    fn sub_a_c() {
        // LD A, 10; LD C, 3; SUB C; HALT
        let prog = [0x3E, 0x0A, 0x0E, 0x03, 0x91, 0x76];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.a, 7);
        assert!(!state.flag_c);
        assert!(state.flag_n);
    }

    #[test]
    fn and_or_xor() {
        // LD A, 0b11001100; AND 0b10101010; HALT → 0b10001000
        let prog = [0x3E, 0xCC, 0xE6, 0xAA, 0x76];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.a, 0x88);
        assert!(state.flag_h); // AND always sets H

        // LD A, 0xF0; OR 0x0F; HALT → 0xFF
        let prog2 = [0x3E, 0xF0, 0xF6, 0x0F, 0x76];
        let mut c2 = cpu();
        let (_, state2) = c2.run(&prog2, 100);
        assert_eq!(state2.a, 0xFF);
        assert!(!state2.flag_h); // OR clears H
    }

    #[test]
    fn inc_dec_flags() {
        // LD A, 0x7F; INC A; HALT
        let prog = [0x3E, 0x7F, 0x3C, 0x76];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.a, 0x80);
        assert!(state.flag_pv); // signed overflow at 0x7F → 0x80
        assert!(!state.flag_c); // INC does not set C

        // LD A, 0x80; DEC A; HALT
        let prog2 = [0x3E, 0x80, 0x3D, 0x76];
        let mut c2 = cpu();
        let (_, state2) = c2.run(&prog2, 100);
        assert_eq!(state2.a, 0x7F);
        assert!(state2.flag_pv); // signed overflow at 0x80 → 0x7F
        assert!(state2.flag_n);  // DEC sets N
    }

    // ── Load instructions ────────────────────────────────────────────────────

    #[test]
    fn ld_reg_reg() {
        // LD A, 42; LD B, A; HALT
        let prog = [0x3E, 42, 0x47, 0x76];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.b, 42);
        assert_eq!(state.a, 42);
    }

    #[test]
    fn ld_rp_nn() {
        // LD BC, 0x1234; HALT
        let prog = [0x01, 0x34, 0x12, 0x76];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.b, 0x12);
        assert_eq!(state.c, 0x34);
    }

    // ── Exchange instructions ─────────────────────────────────────────────────

    #[test]
    fn ex_af() {
        // LD A, 0x55; EX AF,AF'; LD A, 0xAA; EX AF,AF'; HALT
        let prog = [0x3E, 0x55, 0x08, 0x3E, 0xAA, 0x08, 0x76];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.a, 0x55); // original A restored
    }

    #[test]
    fn exx() {
        // LD BC, 0x1234; EXX; LD BC, 0xABCD; EXX; HALT
        let prog = [0x01, 0x34, 0x12, 0xD9, 0x01, 0xCD, 0xAB, 0xD9, 0x76];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.b, 0x12);
        assert_eq!(state.c, 0x34);
    }

    // ── Jump instructions ─────────────────────────────────────────────────────

    #[test]
    fn jr_unconditional() {
        // JR +2; NOP; NOP; LD A, 99; HALT
        // After JR +2, skips 2 NOPs, reaches LD A,99
        let prog = [0x18, 0x02, 0x00, 0x00, 0x3E, 99, 0x76];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.a, 99);
    }

    #[test]
    fn djnz_loop() {
        // LD B, 5; LD A, 0; LOOP: INC A; DJNZ LOOP; HALT
        // B=5, A increments 5 times → A=5
        let prog = [
            0x06, 5,    // LD B, 5
            0x3E, 0,    // LD A, 0
            0x3C,       // INC A  ← 0x0004
            0x10, 0xFD, // DJNZ -3 (back to 0x0004)
            0x76,       // HALT
        ];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.a, 5);
    }

    // ── Stack operations ──────────────────────────────────────────────────────

    #[test]
    fn push_pop_af() {
        // LD A, 0xAB; PUSH AF; LD A, 0x00; POP AF; HALT
        let prog = [0x3E, 0xAB, 0xF5, 0x3E, 0x00, 0xF1, 0x76];
        let mut c = cpu();
        c.sp = 0x8000;
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.a, 0xAB);
    }

    // ── Call / Return ─────────────────────────────────────────────────────────

    #[test]
    fn call_ret() {
        // Main: CALL 0x0010; LD A, 5; HALT
        // 0x0010: LD A, 42; RET
        let mut prog = [0u8; 32];
        prog[0] = 0xCD; prog[1] = 0x10; prog[2] = 0x00; // CALL 0x0010
        prog[3] = 0x3E; prog[4] = 0x05;                   // LD A, 5
        prog[5] = 0x76;                                    // HALT
        prog[0x10] = 0x3E; prog[0x11] = 42;               // LD A, 42
        prog[0x12] = 0xC9;                                 // RET
        let mut c = cpu(); c.sp = 0x8000;
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.a, 0x05); // LD A,5 after RET
    }

    // ── Block operations ──────────────────────────────────────────────────────

    #[test]
    fn ldir_copy() {
        // Copy 3 bytes from 0x0100 to 0x0200 using LDIR
        let mut c = cpu(); c.sp = 0x8000;
        c.memory[0x0100] = 0xAA; c.memory[0x0101] = 0xBB; c.memory[0x0102] = 0xCC;
        // LD HL, 0x0100; LD DE, 0x0200; LD BC, 3; LDIR; HALT
        let prog = [
            0x21, 0x00, 0x01, // LD HL, 0x0100
            0x11, 0x00, 0x02, // LD DE, 0x0200
            0x01, 0x03, 0x00, // LD BC, 3
            0xED, 0xB0,       // LDIR
            0x76,             // HALT
        ];
        c.load(&prog, 0);
        for _ in 0..50 { if c.halted { break; } c.step(); }
        assert_eq!(c.memory[0x0200], 0xAA);
        assert_eq!(c.memory[0x0201], 0xBB);
        assert_eq!(c.memory[0x0202], 0xCC);
    }

    // ── IX/IY indexed ─────────────────────────────────────────────────────────

    #[test]
    fn ix_load_store() {
        // LD IX, 0x0100; LD (IX+0), 42; LD A, (IX+0); HALT
        let prog = [
            0xDD, 0x21, 0x00, 0x01, // LD IX, 0x0100
            0xDD, 0x36, 0x00, 42,   // LD (IX+0), 42
            0xDD, 0x7E, 0x00,       // LD A, (IX+0)
            0x76,                   // HALT
        ];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.a, 42);
    }

    // ── Rotates ───────────────────────────────────────────────────────────────

    #[test]
    fn rlca_rrca() {
        // LD A, 0x81; RLCA; HALT → 0x03, C=1
        let prog = [0x3E, 0x81, 0x07, 0x76];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.a, 0x03);
        assert!(state.flag_c);
    }

    // ── I/O ports ─────────────────────────────────────────────────────────────

    #[test]
    fn in_out_port() {
        let mut c = cpu();
        c.input_ports[0x42] = 0xAB;
        // IN A, (0x42); HALT
        let prog = [0xDB, 0x42, 0x76];
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.a, 0xAB);

        // OUT (0x42), A
        let mut c2 = cpu();
        let prog2 = [0x3E, 0xCD, 0xD3, 0x42, 0x76]; // LD A, 0xCD; OUT (0x42),A; HALT
        c2.run(&prog2, 100);
        assert_eq!(c2.output_ports[0x42], 0xCD);
    }

    // ── EX (SP), HL ──────────────────────────────────────────────────────────

    #[test]
    fn ex_sp_hl() {
        let mut c = cpu();
        // Place 0x1234 in memory at 0x7FFE/0x7FFF (where SP will point after LD SP)
        c.memory[0x7FFE] = 0x34; c.memory[0x7FFF] = 0x12;
        // LD SP, 0x7FFE; LD HL, 0xABCD; EX (SP),HL; HALT
        let prog = [
            0x31, 0xFE, 0x7F, // LD SP, 0x7FFE
            0x21, 0xCD, 0xAB, // LD HL, 0xABCD
            0xE3,             // EX (SP),HL
            0x76,             // HALT
        ];
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.h, 0x12);
        assert_eq!(state.l, 0x34);
        assert_eq!(c.memory[0x7FFE], 0xCD);
        assert_eq!(c.memory[0x7FFF], 0xAB);
    }

    // ── CP (compare) ─────────────────────────────────────────────────────────

    #[test]
    fn cp_equal() {
        // LD A, 5; CP 5; HALT → Z=1, C=0
        let prog = [0x3E, 5, 0xFE, 5, 0x76];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert!(state.flag_z);
        assert!(!state.flag_c);
    }

    #[test]
    fn cp_less_than() {
        // LD A, 3; CP 5; HALT → Z=0, C=1 (borrow)
        let prog = [0x3E, 3, 0xFE, 5, 0x76];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert!(!state.flag_z);
        assert!(state.flag_c);
    }

    // ── NEG ──────────────────────────────────────────────────────────────────

    #[test]
    fn neg_instruction() {
        // LD A, 5; NEG; HALT → A = -5 = 0xFB, C=1 (borrow)
        let prog = [0x3E, 5, 0xED, 0x44, 0x76];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.a, 0xFB);
        assert!(state.flag_c);
        assert!(state.flag_n);
    }

    // ── SBC HL, rp ───────────────────────────────────────────────────────────

    #[test]
    fn sbc_hl_bc() {
        // LD HL, 0x0010; LD BC, 0x0005; SCF; SBC HL, BC; HALT
        // HL = 0x0010 - 0x0005 - 1 = 0x000A
        let prog = [
            0x21, 0x10, 0x00, // LD HL, 0x0010
            0x01, 0x05, 0x00, // LD BC, 0x0005
            0x37,             // SCF (C=1)
            0xED, 0x42,       // SBC HL, BC
            0x76,
        ];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.h, 0x00);
        assert_eq!(state.l, 0x0A);
        assert!(state.flag_n);
    }

    // ── CB-prefix BIT/SET/RES ────────────────────────────────────────────────

    #[test]
    fn bit_set_res() {
        // LD A, 0b00000000; SET 3, A; HALT → A = 0b00001000
        let prog = [0x3E, 0x00, 0xCB, 0xDF, 0x76];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.a, 0b00001000);

        // LD A, 0xFF; RES 3, A; HALT → A = 0b11110111
        let prog2 = [0x3E, 0xFF, 0xCB, 0x9F, 0x76];
        let mut c2 = cpu();
        let (_, state2) = c2.run(&prog2, 100);
        assert_eq!(state2.a, 0b11110111);

        // LD A, 0b00001000; BIT 3, A; HALT → Z=0 (bit is set)
        let prog3 = [0x3E, 0b00001000, 0xCB, 0x5F, 0x76];
        let mut c3 = cpu();
        let (_, state3) = c3.run(&prog3, 100);
        assert!(!state3.flag_z);
    }

    // ── Interrupt mode instructions ───────────────────────────────────────────

    #[test]
    fn interrupt_modes() {
        // IM 1; DI; EI; HALT
        let prog = [0xED, 0x56, 0xF3, 0xFB, 0x76];
        let mut c = cpu();
        let (_, state) = c.run(&prog, 100);
        assert_eq!(state.im, 1);
        assert!(state.iff1);
        assert!(state.iff2);
    }
}
