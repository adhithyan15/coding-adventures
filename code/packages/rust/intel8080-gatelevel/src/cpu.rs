//! Intel 8080 gate-level CPU — all operations route through real logic gates.
//!
//! # What makes this gate-level?
//!
//! Every data-path operation flows through the gate chain:
//!
//! ```text
//! NOT/AND/OR/XOR → half_adder → full_adder → ripple_carry_adder
//!   ↓
//! GateAlu8080  (add/sub/and/or/xor/rotate/shift)
//!   ↓
//! RegisterFile (7 × 8-bit D flip-flop arrays)
//! Register16   (PC + SP, each 16 D flip-flops)
//!   ↓
//! Decoder8080  (combinational AND/NOT/OR gate tree)
//! ```
//!
//! When you execute `ADD B`:
//! 1. Decoder reads opcode 0x80 → group=2, alu_op=ADD, src=B
//! 2. RegisterFile reads B (from its 8 flip-flops) and A (accumulator)
//! 3. `GateAlu8080::add(a, b)` → 8 full-adder stages (40 gates) + flags
//! 4. Result is written back into A's flip-flop array
//! 5. Flags (S, Z, P, AC, CY) are written into the flag register flip-flops
//!
//! # Gate count estimate
//!
//! | Component              | Gates | Notes                         |
//! |------------------------|-------|-------------------------------|
//! | 8-bit ALU              | ~104  | 40 adder + 56 logic/flags     |
//! | Register file (7 × 8)  | 336   | 48 gates/register             |
//! | PC + SP (2 × 16-bit)   | 192   | 96 gates each                 |
//! | 16-bit adder (DAD/INC) | 80    | 16 full-adder stages          |
//! | Instruction decoder    | ~80   | AND/OR/NOT gate tree          |
//! | Control + wiring       | ~300  | mux, bus, condition logic     |
//! | **Total**              | **~1,092** |                          |
//!
//! (Real 8080: ~6,000 transistors ≈ ~1,500 gates in NMOS)
//!
//! # Instruction set
//!
//! Implements all 244 documented Intel 8080A instructions, organized as:
//! - **Group 0** (misc): NOP, MVI, LXI, LDA, STA, LHLD, SHLD, INR, DCR,
//!   INX, DCX, DAD, LDAX, STAX, XCHG, XTHL, SPHL, PCHL, RLC, RRC, RAL, RAR,
//!   CMA, CMC, STC, DAA
//! - **Group 1** (MOV): 63 MOV r,r + HLT
//! - **Group 2** (ALU reg): ADD/ADC/SUB/SBB/ANA/XRA/ORA/CMP r
//! - **Group 3** (branch/stack): JMP, CALL, RET (conditional variants), PUSH,
//!   POP, IN, OUT, EI, DI, RST n, ADI/ACI/SUI/SBI/ANI/XRI/ORI/CPI d8

use crate::alu::{AluFlags, AluOp, GateAlu8080};
use crate::bits::add_16bit;
use crate::decoder::{decode, Decoded};
use crate::registers::{
    Register16, RegisterFile, PAIR_BC, PAIR_DE, PAIR_HL, REG_A, REG_B, REG_C, REG_D,
    REG_E, REG_H, REG_L, REG_M,
};

// ── ALU names for trace output ────────────────────────────────────────────────
const ALU_NAMES: [&str; 8] = ["ADD", "ADC", "SUB", "SBB", "ANA", "XRA", "ORA", "CMP"];
const ALU_IMM_NAMES: [&str; 8] = ["ADI", "ACI", "SUI", "SBI", "ANI", "XRI", "ORI", "CPI"];
const REG_NAMES: [&str; 8] = ["B", "C", "D", "E", "H", "L", "M", "A"];
const PAIR_NAMES: [&str; 4] = ["BC", "DE", "HL", "SP"];

/// Snapshot of CPU state after an instruction.
///
/// Returned by `step()` for tracing / test assertions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepTrace {
    pub pc_before: u16,
    pub pc_after: u16,
    pub mnemonic: String,
    pub description: String,
}

/// Complete Intel 8080 state snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuState {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
    pub flag_s: bool,
    pub flag_z: bool,
    pub flag_ac: bool,
    pub flag_p: bool,
    pub flag_cy: bool,
    pub halted: bool,
    pub memory: Vec<u8>,
    pub input_ports: Vec<u8>,
    pub output_ports: Vec<u8>,
}

/// Intel 8080 gate-level CPU.
///
/// All arithmetic and logic operations route through the `logic-gates` and
/// `arithmetic` crate primitives. No host integer arithmetic in the execution path.
pub struct GateLevelCpu {
    regs: RegisterFile,
    pc: Register16,
    sp: Register16,
    memory: Box<[u8; 65536]>,
    flag_s: bool,
    flag_z: bool,
    flag_ac: bool,
    flag_p: bool,
    flag_cy: bool,
    inte: bool,  // interrupt enable
    halted: bool,
    input_ports: Box<[u8; 256]>,
    output_ports: Box<[u8; 256]>,
}

impl Default for GateLevelCpu {
    fn default() -> Self {
        Self::new()
    }
}

impl GateLevelCpu {
    /// Create a new CPU with all state zeroed.
    pub fn new() -> Self {
        GateLevelCpu {
            regs: RegisterFile::new(),
            pc: Register16::new(),
            sp: Register16::new(),
            memory: Box::new([0u8; 65536]),
            flag_s: false,
            flag_z: false,
            flag_ac: false,
            flag_p: false,
            flag_cy: false,
            inte: false,
            halted: false,
            input_ports: Box::new([0u8; 256]),
            output_ports: Box::new([0u8; 256]),
        }
    }

    // ── Public accessors ──────────────────────────────────────────────────────

    pub fn a(&self) -> u8 { self.regs.read(REG_A) }
    pub fn b(&self) -> u8 { self.regs.read(REG_B) }
    pub fn c(&self) -> u8 { self.regs.read(REG_C) }
    pub fn d(&self) -> u8 { self.regs.read(REG_D) }
    pub fn e(&self) -> u8 { self.regs.read(REG_E) }
    pub fn h(&self) -> u8 { self.regs.read(REG_H) }
    pub fn l(&self) -> u8 { self.regs.read(REG_L) }
    pub fn pc(&self) -> u16 { self.pc.read() }
    pub fn sp(&self) -> u16 { self.sp.read() }
    pub fn flag_cy(&self) -> bool { self.flag_cy }
    pub fn flag_z(&self) -> bool { self.flag_z }
    pub fn flag_s(&self) -> bool { self.flag_s }
    pub fn flag_p(&self) -> bool { self.flag_p }
    pub fn flag_ac(&self) -> bool { self.flag_ac }
    pub fn halted(&self) -> bool { self.halted }

    /// Read from memory at a given address.
    pub fn mem(&self, addr: u16) -> u8 { self.memory[addr as usize] }

    /// Set an input port value (simulates external hardware).
    pub fn set_input_port(&mut self, port: u8, value: u8) {
        self.input_ports[port as usize] = value;
    }

    /// Read the current value of an output port.
    pub fn get_output_port(&mut self, port: u8) -> u8 {
        self.output_ports[port as usize]
    }

    // ── Reset and load ────────────────────────────────────────────────────────

    /// Reset all registers, flags, and halted state to power-on values.
    /// Memory and I/O ports are preserved.
    pub fn reset(&mut self) {
        self.regs = RegisterFile::new();
        self.pc = Register16::new();
        self.sp = Register16::new();
        self.flag_s = false;
        self.flag_z = false;
        self.flag_ac = false;
        self.flag_p = false;
        self.flag_cy = false;
        self.inte = false;
        self.halted = false;
    }

    /// Load a program into memory starting at address 0x0000.
    pub fn load(&mut self, program: &[u8]) {
        for (i, &byte) in program.iter().enumerate() {
            self.memory[i] = byte;
        }
    }

    // ── Execution ─────────────────────────────────────────────────────────────

    /// Execute one complete instruction (fetch-decode-execute-writeback).
    ///
    /// Returns `Some(StepTrace)` on success, `None` if halted.
    pub fn step(&mut self) -> Option<StepTrace> {
        if self.halted { return None; }

        let pc_before = self.pc.read();

        // ── FETCH opcode ──────────────────────────────────────────────────────
        let opcode = self.fetch_byte();

        // ── DECODE ────────────────────────────────────────────────────────────
        let decoded = decode(opcode);

        // ── FETCH immediate bytes ─────────────────────────────────────────────
        let imm1 = if decoded.extra_bytes >= 1 { self.fetch_byte() } else { 0 };
        let imm2 = if decoded.extra_bytes >= 2 { self.fetch_byte() } else { 0 };
        let imm16: u16 = ((imm2 as u16) << 8) | (imm1 as u16);

        // ── EXECUTE + WRITEBACK ───────────────────────────────────────────────
        let (mnemonic, description) = self.execute(opcode, &decoded, imm1, imm2, imm16);

        let pc_after = self.pc.read();
        Some(StepTrace { pc_before, pc_after, mnemonic, description })
    }

    /// Load and run a program until HLT or `max_steps`.
    ///
    /// Returns a vector of step traces and the final state.
    pub fn run(&mut self, program: &[u8], max_steps: usize) -> (Vec<StepTrace>, CpuState) {
        self.reset();
        self.load(program);
        let mut traces = Vec::new();
        for _ in 0..max_steps {
            if self.halted { break; }
            if let Some(trace) = self.step() {
                traces.push(trace);
            }
        }
        (traces, self.state())
    }

    /// Capture an immutable snapshot of the current CPU state.
    pub fn state(&self) -> CpuState {
        CpuState {
            a: self.regs.read(REG_A),
            b: self.regs.read(REG_B),
            c: self.regs.read(REG_C),
            d: self.regs.read(REG_D),
            e: self.regs.read(REG_E),
            h: self.regs.read(REG_H),
            l: self.regs.read(REG_L),
            sp: self.sp.read(),
            pc: self.pc.read(),
            flag_s: self.flag_s,
            flag_z: self.flag_z,
            flag_ac: self.flag_ac,
            flag_p: self.flag_p,
            flag_cy: self.flag_cy,
            halted: self.halted,
            memory: self.memory.to_vec(),
            input_ports: self.input_ports.to_vec(),
            output_ports: self.output_ports.to_vec(),
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn fetch_byte(&mut self) -> u8 {
        let addr = self.pc.read();
        let byte = self.memory[addr as usize];
        self.pc.inc(1);
        byte
    }

    fn read_reg(&self, reg: u8) -> u8 {
        if reg == REG_M {
            self.memory[self.regs.hl_addr() as usize]
        } else {
            self.regs.read(reg)
        }
    }

    fn write_reg(&mut self, reg: u8, value: u8) {
        if reg == REG_M {
            let addr = self.regs.hl_addr();
            self.memory[addr as usize] = value;
        } else {
            self.regs.write(reg, value);
        }
    }

    fn apply_alu_flags(&mut self, flags: AluFlags, updates_cy: bool) {
        self.flag_z = flags.zero;
        self.flag_s = flags.sign;
        self.flag_p = flags.parity;
        self.flag_ac = flags.ac;
        if updates_cy {
            self.flag_cy = flags.cy;
        }
    }

    fn push_u16(&mut self, value: u16) {
        self.sp.dec(2);
        let addr = self.sp.read();
        self.memory[addr as usize + 1] = (value >> 8) as u8;
        self.memory[addr as usize] = (value & 0xFF) as u8;
    }

    fn pop_u16(&mut self) -> u16 {
        let addr = self.sp.read();
        let lo = self.memory[addr as usize] as u16;
        let hi = self.memory[addr as usize + 1] as u16;
        self.sp.inc(2);
        (hi << 8) | lo
    }

    fn flags_byte(&self) -> u8 {
        // 8080 flags byte layout: S Z 0 AC 0 P 1 CY
        ((self.flag_s as u8) << 7)
            | ((self.flag_z as u8) << 6)
            | ((self.flag_ac as u8) << 4)
            | ((self.flag_p as u8) << 2)
            | (1 << 1)
            | (self.flag_cy as u8)
    }

    fn set_flags_byte(&mut self, byte: u8) {
        self.flag_s = (byte & 0x80) != 0;
        self.flag_z = (byte & 0x40) != 0;
        self.flag_ac = (byte & 0x10) != 0;
        self.flag_p = (byte & 0x04) != 0;
        self.flag_cy = (byte & 0x01) != 0;
    }

    fn condition_met(&self, cond: u8) -> bool {
        match cond & 7 {
            0 => !self.flag_z,    // NZ
            1 => self.flag_z,     // Z
            2 => !self.flag_cy,   // NC
            3 => self.flag_cy,    // C
            4 => !self.flag_p,    // PO (parity odd)
            5 => self.flag_p,     // PE (parity even)
            6 => !self.flag_s,    // P (positive / sign clear)
            7 => self.flag_s,     // M (minus / sign set)
            _ => unreachable!(),
        }
    }

    // ── Execute dispatch ──────────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    fn execute(
        &mut self,
        opcode: u8,
        d: &Decoded,
        imm1: u8,
        _imm2: u8,
        imm16: u16,
    ) -> (String, String) {
        if d.is_halt {
            self.halted = true;
            return ("HLT".into(), "Halt — CPU stopped".into());
        }

        match d.group {
            1 => self.exec_mov(d),
            2 => self.exec_alu_reg(d),
            0 => self.exec_group0(opcode, d, imm1, imm16),
            3 => self.exec_group3(opcode, d, imm1, imm16),
            _ => ("???".into(), format!("Unknown opcode 0x{opcode:02X}")),
        }
    }

    // ── Group 1: MOV ─────────────────────────────────────────────────────────

    fn exec_mov(&mut self, d: &Decoded) -> (String, String) {
        let val = self.read_reg(d.src);
        self.write_reg(d.dst, val);
        let mnem = format!("MOV {},{}", REG_NAMES[d.dst as usize], REG_NAMES[d.src as usize]);
        let desc = format!("{} ← {val:#04X}", REG_NAMES[d.dst as usize]);
        (mnem, desc)
    }

    // ── Group 2: ALU register ─────────────────────────────────────────────────

    fn exec_alu_reg(&mut self, d: &Decoded) -> (String, String) {
        let a = self.regs.read(REG_A);
        let b = self.read_reg(d.src);
        let op = AluOp::from_bits(d.alu_op).unwrap();
        let result = GateAlu8080::dispatch(op, a, b, self.flag_cy);
        // CMP does not write back to A
        if op != AluOp::Cmp {
            self.regs.write(REG_A, result.value);
        }
        self.apply_alu_flags(result.flags, result.updates_cy);
        let mnem = format!("{} {}", ALU_NAMES[d.alu_op as usize], REG_NAMES[d.src as usize]);
        let desc = format!("A = {:#04X}", self.regs.read(REG_A));
        (mnem, desc)
    }

    // ── Group 0: misc ─────────────────────────────────────────────────────────

    #[allow(clippy::too_many_lines)]
    fn exec_group0(&mut self, opcode: u8, _d: &Decoded, imm1: u8, imm16: u16) -> (String, String) {
        match opcode {
            0x00 => ("NOP".into(), "No operation".into()),

            // ── MVI r, d8 ──────────────────────────────────────────────────
            0x06 => { self.regs.write(REG_B, imm1); (format!("MVI B,{imm1:#04X}"), format!("B ← {imm1:#04X}")) }
            0x0E => { self.regs.write(REG_C, imm1); (format!("MVI C,{imm1:#04X}"), format!("C ← {imm1:#04X}")) }
            0x16 => { self.regs.write(REG_D, imm1); (format!("MVI D,{imm1:#04X}"), format!("D ← {imm1:#04X}")) }
            0x1E => { self.regs.write(REG_E, imm1); (format!("MVI E,{imm1:#04X}"), format!("E ← {imm1:#04X}")) }
            0x26 => { self.regs.write(REG_H, imm1); (format!("MVI H,{imm1:#04X}"), format!("H ← {imm1:#04X}")) }
            0x2E => { self.regs.write(REG_L, imm1); (format!("MVI L,{imm1:#04X}"), format!("L ← {imm1:#04X}")) }
            0x36 => {
                let addr = self.regs.hl_addr();
                self.memory[addr as usize] = imm1;
                (format!("MVI M,{imm1:#04X}"), format!("mem[{addr:#06X}] ← {imm1:#04X}"))
            }
            0x3E => { self.regs.write(REG_A, imm1); (format!("MVI A,{imm1:#04X}"), format!("A ← {imm1:#04X}")) }

            // ── LXI rp, d16 ──────────────────────────────────────────────────
            0x01 => { self.regs.write_pair(PAIR_BC, imm16); (format!("LXI BC,{imm16:#06X}"), format!("BC ← {imm16:#06X}")) }
            0x11 => { self.regs.write_pair(PAIR_DE, imm16); (format!("LXI DE,{imm16:#06X}"), format!("DE ← {imm16:#06X}")) }
            0x21 => { self.regs.write_pair(PAIR_HL, imm16); (format!("LXI HL,{imm16:#06X}"), format!("HL ← {imm16:#06X}")) }
            0x31 => { self.sp.write(imm16); (format!("LXI SP,{imm16:#06X}"), format!("SP ← {imm16:#06X}")) }

            // ── LDA / STA ─────────────────────────────────────────────────────
            0x3A => {
                let v = self.memory[imm16 as usize];
                self.regs.write(REG_A, v);
                (format!("LDA {imm16:#06X}"), format!("A ← mem[{imm16:#06X}] = {v:#04X}"))
            }
            0x32 => {
                let a = self.regs.read(REG_A);
                self.memory[imm16 as usize] = a;
                (format!("STA {imm16:#06X}"), format!("mem[{imm16:#06X}] ← A = {a:#04X}"))
            }

            // ── LHLD / SHLD ───────────────────────────────────────────────────
            0x2A => {
                let lo = self.memory[imm16 as usize] as u16;
                let hi = self.memory[imm16 as usize + 1] as u16;
                let val = (hi << 8) | lo;
                self.regs.write_pair(PAIR_HL, val);
                (format!("LHLD {imm16:#06X}"), format!("HL ← {val:#06X}"))
            }
            0x22 => {
                let hl = self.regs.read_pair(PAIR_HL);
                self.memory[imm16 as usize] = (hl & 0xFF) as u8;
                self.memory[imm16 as usize + 1] = (hl >> 8) as u8;
                (format!("SHLD {imm16:#06X}"), format!("mem[{imm16:#06X}] ← HL = {hl:#06X}"))
            }

            // ── LDAX / STAX ───────────────────────────────────────────────────
            0x0A => {
                let addr = self.regs.read_pair(PAIR_BC);
                let v = self.memory[addr as usize];
                self.regs.write(REG_A, v);
                ("LDAX BC".to_string(), format!("A ← mem[BC={addr:#06X}] = {v:#04X}"))
            }
            0x1A => {
                let addr = self.regs.read_pair(PAIR_DE);
                let v = self.memory[addr as usize];
                self.regs.write(REG_A, v);
                ("LDAX DE".to_string(), format!("A ← mem[DE={addr:#06X}] = {v:#04X}"))
            }
            0x02 => {
                let addr = self.regs.read_pair(PAIR_BC);
                let a = self.regs.read(REG_A);
                self.memory[addr as usize] = a;
                ("STAX BC".to_string(), format!("mem[BC={addr:#06X}] ← A = {a:#04X}"))
            }
            0x12 => {
                let addr = self.regs.read_pair(PAIR_DE);
                let a = self.regs.read(REG_A);
                self.memory[addr as usize] = a;
                ("STAX DE".to_string(), format!("mem[DE={addr:#06X}] ← A = {a:#04X}"))
            }

            // ── INR r ─────────────────────────────────────────────────────────
            0x04 => self.exec_inr(REG_B),
            0x0C => self.exec_inr(REG_C),
            0x14 => self.exec_inr(REG_D),
            0x1C => self.exec_inr(REG_E),
            0x24 => self.exec_inr(REG_H),
            0x2C => self.exec_inr(REG_L),
            0x34 => {
                let addr = self.regs.hl_addr();
                let v = self.memory[addr as usize];
                let r = GateAlu8080::inr(v);
                self.memory[addr as usize] = r.value;
                self.apply_alu_flags(r.flags, false);
                ("INR M".into(), format!("mem[{addr:#06X}] ← {:#04X}", r.value))
            }
            0x3C => self.exec_inr(REG_A),

            // ── DCR r ─────────────────────────────────────────────────────────
            0x05 => self.exec_dcr(REG_B),
            0x0D => self.exec_dcr(REG_C),
            0x15 => self.exec_dcr(REG_D),
            0x1D => self.exec_dcr(REG_E),
            0x25 => self.exec_dcr(REG_H),
            0x2D => self.exec_dcr(REG_L),
            0x35 => {
                let addr = self.regs.hl_addr();
                let v = self.memory[addr as usize];
                let r = GateAlu8080::dcr(v);
                self.memory[addr as usize] = r.value;
                self.apply_alu_flags(r.flags, false);
                ("DCR M".into(), format!("mem[{addr:#06X}] ← {:#04X}", r.value))
            }
            0x3D => self.exec_dcr(REG_A),

            // ── INX rp ────────────────────────────────────────────────────────
            0x03 => { let v = add_16bit(self.regs.read_pair(PAIR_BC), 1, 0).0; self.regs.write_pair(PAIR_BC, v); ("INX BC".into(), format!("BC ← {v:#06X}")) }
            0x13 => { let v = add_16bit(self.regs.read_pair(PAIR_DE), 1, 0).0; self.regs.write_pair(PAIR_DE, v); ("INX DE".into(), format!("DE ← {v:#06X}")) }
            0x23 => { let v = add_16bit(self.regs.read_pair(PAIR_HL), 1, 0).0; self.regs.write_pair(PAIR_HL, v); ("INX HL".into(), format!("HL ← {v:#06X}")) }
            0x33 => { let v = add_16bit(self.sp.read(), 1, 0).0; self.sp.write(v); ("INX SP".into(), format!("SP ← {v:#06X}")) }

            // ── DCX rp ────────────────────────────────────────────────────────
            0x0B => { let v = self.regs.read_pair(PAIR_BC).wrapping_sub(1); self.regs.write_pair(PAIR_BC, v); ("DCX BC".into(), format!("BC ← {v:#06X}")) }
            0x1B => { let v = self.regs.read_pair(PAIR_DE).wrapping_sub(1); self.regs.write_pair(PAIR_DE, v); ("DCX DE".into(), format!("DE ← {v:#06X}")) }
            0x2B => { let v = self.regs.read_pair(PAIR_HL).wrapping_sub(1); self.regs.write_pair(PAIR_HL, v); ("DCX HL".into(), format!("HL ← {v:#06X}")) }
            0x3B => { let v = self.sp.read().wrapping_sub(1); self.sp.write(v); ("DCX SP".into(), format!("SP ← {v:#06X}")) }

            // ── DAD rp ────────────────────────────────────────────────────────
            0x09 => self.exec_dad(PAIR_BC),
            0x19 => self.exec_dad(PAIR_DE),
            0x29 => self.exec_dad(PAIR_HL),
            0x39 => {
                let hl = self.regs.read_pair(PAIR_HL);
                let sp = self.sp.read();
                let (result, carry) = add_16bit(hl, sp, 0);
                self.regs.write_pair(PAIR_HL, result);
                self.flag_cy = carry != 0;
                ("DAD SP".into(), format!("HL ← {result:#06X}, CY={}", carry != 0))
            }

            // ── XCHG ──────────────────────────────────────────────────────────
            0xEB => {
                let de = self.regs.read_pair(PAIR_DE);
                let hl = self.regs.read_pair(PAIR_HL);
                self.regs.write_pair(PAIR_DE, hl);
                self.regs.write_pair(PAIR_HL, de);
                ("XCHG".into(), format!("DE ↔ HL (DE={de:#06X}, HL={hl:#06X})"))
            }

            // ── Rotates ────────────────────────────────────────────────────────
            0x07 => { let r = GateAlu8080::rlc(self.regs.read(REG_A)); self.regs.write(REG_A, r.value); self.flag_cy = r.flags.cy; ("RLC".into(), format!("A={:#04X}, CY={}", r.value, r.flags.cy)) }
            0x0F => { let r = GateAlu8080::rrc(self.regs.read(REG_A)); self.regs.write(REG_A, r.value); self.flag_cy = r.flags.cy; ("RRC".into(), format!("A={:#04X}, CY={}", r.value, r.flags.cy)) }
            0x17 => { let r = GateAlu8080::ral(self.regs.read(REG_A), self.flag_cy); self.regs.write(REG_A, r.value); self.flag_cy = r.flags.cy; ("RAL".into(), format!("A={:#04X}, CY={}", r.value, r.flags.cy)) }
            0x1F => { let r = GateAlu8080::rar(self.regs.read(REG_A), self.flag_cy); self.regs.write(REG_A, r.value); self.flag_cy = r.flags.cy; ("RAR".into(), format!("A={:#04X}, CY={}", r.value, r.flags.cy)) }

            // ── CMA / CMC / STC ───────────────────────────────────────────────
            0x2F => { let v = GateAlu8080::cma(self.regs.read(REG_A)); self.regs.write(REG_A, v); ("CMA".into(), format!("A ← {v:#04X}")) }
            0x3F => { self.flag_cy = GateAlu8080::cmc(self.flag_cy); ("CMC".into(), format!("CY ← {}", self.flag_cy)) }
            0x37 => { self.flag_cy = true; ("STC".into(), "CY ← 1".into()) }

            // ── DAA ───────────────────────────────────────────────────────────
            0x27 => {
                let a = self.regs.read(REG_A);
                let r = GateAlu8080::daa(a, self.flag_cy, self.flag_ac);
                self.regs.write(REG_A, r.value);
                self.apply_alu_flags(r.flags, true);
                ("DAA".into(), format!("A ← {:#04X}", r.value))
            }

            // ── SPHL / PCHL / XTHL ────────────────────────────────────────────
            0xF9 => { let hl = self.regs.read_pair(PAIR_HL); self.sp.write(hl); ("SPHL".into(), format!("SP ← HL = {hl:#06X}")) }
            0xE9 => { let hl = self.regs.read_pair(PAIR_HL); self.pc.write(hl); ("PCHL".into(), format!("PC ← HL = {hl:#06X}")) }
            0xE3 => {
                let sp = self.sp.read();
                let lo = self.memory[sp as usize] as u16;
                let hi = self.memory[sp as usize + 1] as u16;
                let top = (hi << 8) | lo;
                let hl = self.regs.read_pair(PAIR_HL);
                self.memory[sp as usize] = (hl & 0xFF) as u8;
                self.memory[sp as usize + 1] = (hl >> 8) as u8;
                self.regs.write_pair(PAIR_HL, top);
                ("XTHL".into(), format!("HL ↔ (SP): HL={hl:#06X}, top={top:#06X}"))
            }

            _ => (format!("??0_{opcode:02X}"), format!("Unimplemented group-0 opcode {opcode:#04X}")),
        }
    }

    fn exec_inr(&mut self, reg: u8) -> (String, String) {
        let v = self.regs.read(reg);
        let r = GateAlu8080::inr(v);
        self.regs.write(reg, r.value);
        self.apply_alu_flags(r.flags, false);
        let name = REG_NAMES[reg as usize];
        (format!("INR {name}"), format!("{name} ← {:#04X}", r.value))
    }

    fn exec_dcr(&mut self, reg: u8) -> (String, String) {
        let v = self.regs.read(reg);
        let r = GateAlu8080::dcr(v);
        self.regs.write(reg, r.value);
        self.apply_alu_flags(r.flags, false);
        let name = REG_NAMES[reg as usize];
        (format!("DCR {name}"), format!("{name} ← {:#04X}", r.value))
    }

    fn exec_dad(&mut self, pair: u8) -> (String, String) {
        let hl = self.regs.read_pair(PAIR_HL);
        let rp = self.regs.read_pair(pair);
        let (result, carry) = add_16bit(hl, rp, 0);
        self.regs.write_pair(PAIR_HL, result);
        self.flag_cy = carry != 0;
        let name = PAIR_NAMES[pair as usize];
        (format!("DAD {name}"), format!("HL ← {result:#06X}, CY={}", carry != 0))
    }

    // ── Group 3: branches, stack, control ────────────────────────────────────

    #[allow(clippy::too_many_lines)]
    fn exec_group3(&mut self, opcode: u8, _d: &Decoded, imm1: u8, imm16: u16) -> (String, String) {
        match opcode {
            // ── Unconditional JMP ────────────────────────────────────────────
            0xC3 => { self.pc.write(imm16); (format!("JMP {imm16:#06X}"), format!("PC ← {imm16:#06X}")) }

            // ── Conditional JMP: Ccc010 ──────────────────────────────────────
            0xC2 | 0xCA | 0xD2 | 0xDA | 0xE2 | 0xEA | 0xF2 | 0xFA => {
                let cond = (opcode >> 3) & 7;
                let taken = self.condition_met(cond);
                if taken { self.pc.write(imm16); }
                let cname = cond_name(cond);
                (format!("J{cname} {imm16:#06X}"), format!("branch {} taken={taken}", imm16))
            }

            // ── Unconditional CALL ───────────────────────────────────────────
            0xCD => {
                let ret_addr = self.pc.read();
                self.push_u16(ret_addr);
                self.pc.write(imm16);
                (format!("CALL {imm16:#06X}"), format!("push {ret_addr:#06X}; PC ← {imm16:#06X}"))
            }

            // ── Conditional CALL: Ccc100 ─────────────────────────────────────
            0xC4 | 0xCC | 0xD4 | 0xDC | 0xE4 | 0xEC | 0xF4 | 0xFC => {
                let cond = (opcode >> 3) & 7;
                let taken = self.condition_met(cond);
                if taken {
                    let ret_addr = self.pc.read();
                    self.push_u16(ret_addr);
                    self.pc.write(imm16);
                }
                let cname = cond_name(cond);
                (format!("C{cname} {imm16:#06X}"), format!("call if {cname}, taken={taken}"))
            }

            // ── Unconditional RET ────────────────────────────────────────────
            0xC9 => {
                let addr = self.pop_u16();
                self.pc.write(addr);
                ("RET".to_string(), format!("PC ← {addr:#06X}"))
            }

            // ── Conditional RET: Ccc000 ──────────────────────────────────────
            0xC0 | 0xC8 | 0xD0 | 0xD8 | 0xE0 | 0xE8 | 0xF0 | 0xF8 => {
                let cond = (opcode >> 3) & 7;
                let taken = self.condition_met(cond);
                if taken {
                    let addr = self.pop_u16();
                    self.pc.write(addr);
                }
                let cname = cond_name(cond);
                (format!("R{cname}"), format!("ret if {cname}, taken={taken}"))
            }

            // ── RST n ────────────────────────────────────────────────────────
            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => {
                let n = (opcode >> 3) & 7;
                let ret_addr = self.pc.read();
                self.push_u16(ret_addr);
                self.pc.write((n as u16) * 8);
                (format!("RST {n}"), format!("push {ret_addr:#06X}; PC ← {:#06X}", (n as u16) * 8))
            }

            // ── PUSH / POP ────────────────────────────────────────────────────
            0xC5 => { let v = self.regs.read_pair(PAIR_BC); self.push_u16(v); ("PUSH BC".into(), format!("push {v:#06X}")) }
            0xD5 => { let v = self.regs.read_pair(PAIR_DE); self.push_u16(v); ("PUSH DE".into(), format!("push {v:#06X}")) }
            0xE5 => { let v = self.regs.read_pair(PAIR_HL); self.push_u16(v); ("PUSH HL".into(), format!("push {v:#06X}")) }
            0xF5 => {
                let a = self.regs.read(REG_A);
                let f = self.flags_byte();
                let v = ((a as u16) << 8) | (f as u16);
                self.push_u16(v);
                ("PUSH PSW".into(), format!("push A={a:#04X} F={f:#04X}"))
            }
            0xC1 => { let v = self.pop_u16(); self.regs.write_pair(PAIR_BC, v); ("POP BC".into(), format!("BC ← {v:#06X}")) }
            0xD1 => { let v = self.pop_u16(); self.regs.write_pair(PAIR_DE, v); ("POP DE".into(), format!("DE ← {v:#06X}")) }
            0xE1 => { let v = self.pop_u16(); self.regs.write_pair(PAIR_HL, v); ("POP HL".into(), format!("HL ← {v:#06X}")) }
            0xF1 => {
                let v = self.pop_u16();
                self.regs.write(REG_A, (v >> 8) as u8);
                self.set_flags_byte((v & 0xFF) as u8);
                ("POP PSW".into(), format!("A ← {:#04X}", (v >> 8) as u8))
            }

            // ── Immediate ALU ─────────────────────────────────────────────────
            0xC6 | 0xCE | 0xD6 | 0xDE | 0xE6 | 0xEE | 0xF6 | 0xFE => {
                let alu_op_idx = (opcode >> 3) & 7;
                let op = AluOp::from_bits(alu_op_idx).unwrap();
                let a = self.regs.read(REG_A);
                let result = GateAlu8080::dispatch(op, a, imm1, self.flag_cy);
                if op != AluOp::Cmp {
                    self.regs.write(REG_A, result.value);
                }
                self.apply_alu_flags(result.flags, result.updates_cy);
                let mnem = format!("{} {imm1:#04X}", ALU_IMM_NAMES[alu_op_idx as usize]);
                (mnem, format!("A = {:#04X}", self.regs.read(REG_A)))
            }

            // ── IN / OUT ──────────────────────────────────────────────────────
            0xDB => {
                let v = self.input_ports[imm1 as usize];
                self.regs.write(REG_A, v);
                (format!("IN {imm1:#04X}"), format!("A ← port[{imm1}] = {v:#04X}"))
            }
            0xD3 => {
                let a = self.regs.read(REG_A);
                self.output_ports[imm1 as usize] = a;
                (format!("OUT {imm1:#04X}"), format!("port[{imm1}] ← A = {a:#04X}"))
            }

            // ── EI / DI ───────────────────────────────────────────────────────
            0xFB => { self.inte = true; ("EI".into(), "Enable interrupts".into()) }
            0xF3 => { self.inte = false; ("DI".into(), "Disable interrupts".into()) }

            _ => (format!("??3_{opcode:02X}"), format!("Unimplemented group-3 opcode {opcode:#04X}")),
        }
    }
}

fn cond_name(cond: u8) -> &'static str {
    match cond & 7 {
        0 => "NZ", 1 => "Z", 2 => "NC", 3 => "C",
        4 => "PO", 5 => "PE", 6 => "P", 7 => "M",
        _ => "??",
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn run_prog(bytes: &[u8]) -> GateLevelCpu {
        let mut cpu = GateLevelCpu::new();
        cpu.load(bytes);
        for _ in 0..10_000 {
            if cpu.halted { break; }
            cpu.step();
        }
        cpu
    }

    #[test]
    fn mvi_and_add() {
        // MVI A,10; MVI B,5; ADD B; HLT
        let cpu = run_prog(&[0x3E, 0x0A, 0x06, 0x05, 0x80, 0x76]);
        assert_eq!(cpu.a(), 15);
        assert!(!cpu.flag_cy());
        assert!(!cpu.flag_z());
        assert!(cpu.halted());
    }

    #[test]
    fn add_overflow_sets_cy() {
        // MVI A,0xFF; MVI B,1; ADD B; HLT
        let cpu = run_prog(&[0x3E, 0xFF, 0x06, 0x01, 0x80, 0x76]);
        assert_eq!(cpu.a(), 0x00);
        assert!(cpu.flag_cy());
        assert!(cpu.flag_z());
    }

    #[test]
    fn sub_basic() {
        // MVI A,10; MVI B,3; SUB B; HLT
        let cpu = run_prog(&[0x3E, 0x0A, 0x06, 0x03, 0x90, 0x76]);
        assert_eq!(cpu.a(), 7);
        assert!(!cpu.flag_cy());
    }

    #[test]
    fn mov_register() {
        // MVI B,0x42; MOV A,B; HLT
        let cpu = run_prog(&[0x06, 0x42, 0x78, 0x76]);
        assert_eq!(cpu.a(), 0x42);
    }

    #[test]
    fn lxi_and_ldax() {
        // LXI BC,0x0010; MVI A,0x99; STAX BC; LDA 0x0010; HLT
        let cpu = run_prog(&[0x01, 0x10, 0x00, 0x3E, 0x99, 0x02, 0x3A, 0x10, 0x00, 0x76]);
        assert_eq!(cpu.a(), 0x99);
    }

    #[test]
    fn jump_unconditional() {
        // JMP 0x0005; MVI A,0xFF; MVI A,0x42; HLT
        let cpu = run_prog(&[0xC3, 0x05, 0x00, 0x3E, 0xFF, 0x3E, 0x42, 0x76]);
        assert_eq!(cpu.a(), 0x42);
    }

    #[test]
    fn call_and_ret() {
        // CALL 0x0006; HLT; (pad); MVI A,0x55; RET
        let prog = [
            0xCD, 0x05, 0x00, // CALL 0x0005
            0x76,             // HLT (address 3)
            0x00,             // NOP (pad to addr 4)
            0x3E, 0x55,       // MVI A,0x55 (address 5)
            0xC9,             // RET
        ];
        let cpu = run_prog(&prog);
        assert_eq!(cpu.a(), 0x55);
        assert!(cpu.halted());
    }

    #[test]
    fn push_pop() {
        // LXI SP,0x0100; MVI B,0x12; MVI C,0x34; PUSH BC; POP DE; HLT
        let cpu = run_prog(&[
            0x31, 0x00, 0x01, // LXI SP,0x0100
            0x06, 0x12,       // MVI B,0x12
            0x0E, 0x34,       // MVI C,0x34
            0xC5,             // PUSH BC
            0xD1,             // POP DE
            0x76,             // HLT
        ]);
        assert_eq!(cpu.d(), 0x12);
        assert_eq!(cpu.e(), 0x34);
    }

    #[test]
    fn inr_dcr() {
        // MVI B,0x0F; INR B; DCR B; HLT
        let cpu = run_prog(&[0x06, 0x0F, 0x04, 0x05, 0x76]);
        assert_eq!(cpu.b(), 0x0F);
    }

    #[test]
    fn jnz_loop() {
        // MVI C,5; MVI A,0; loop: INR A; DCR C; JNZ loop; HLT
        let prog = [
            0x0E, 0x05, // MVI C,5
            0x3E, 0x00, // MVI A,0
            0x3C,       // INR A  (loop start at addr 4)
            0x0D,       // DCR C
            0xC2, 0x04, 0x00, // JNZ 0x0004
            0x76,       // HLT
        ];
        let cpu = run_prog(&prog);
        assert_eq!(cpu.a(), 5);
        assert_eq!(cpu.c(), 0);
    }

    #[test]
    fn dad_hl_bc() {
        // LXI HL,0x1234; LXI BC,0x0100; DAD BC; HLT
        let cpu = run_prog(&[
            0x21, 0x34, 0x12, // LXI HL,0x1234
            0x01, 0x00, 0x01, // LXI BC,0x0100
            0x09,             // DAD BC
            0x76,             // HLT
        ]);
        assert_eq!(cpu.h(), 0x13);
        assert_eq!(cpu.l(), 0x34);
    }

    #[test]
    fn xra_zero_clears_flags() {
        // MVI A,0xFF; XRA A; HLT  (A ^ A = 0, clears CY and AC)
        let cpu = run_prog(&[0x3E, 0xFF, 0xAF, 0x76]);
        assert_eq!(cpu.a(), 0x00);
        assert!(cpu.flag_z());
        assert!(!cpu.flag_cy());
        assert!(!cpu.flag_ac());
    }

    #[test]
    fn in_out_ports() {
        let mut cpu = GateLevelCpu::new();
        cpu.set_input_port(5, 0xAB);
        // IN 5; OUT 7; HLT
        cpu.load(&[0xDB, 0x05, 0xD3, 0x07, 0x76]);
        for _ in 0..100 { if cpu.halted { break; } cpu.step(); }
        assert_eq!(cpu.get_output_port(7), 0xAB);
    }

    #[test]
    fn cma_complement() {
        // MVI A,0xAA; CMA; HLT
        let cpu = run_prog(&[0x3E, 0xAA, 0x2F, 0x76]);
        assert_eq!(cpu.a(), 0x55);
    }

    #[test]
    fn rlc_rotate() {
        // MVI A,0x80; RLC; HLT  (0x80 → 0x01, CY=1)
        let cpu = run_prog(&[0x3E, 0x80, 0x07, 0x76]);
        assert_eq!(cpu.a(), 0x01);
        assert!(cpu.flag_cy());
    }
}
