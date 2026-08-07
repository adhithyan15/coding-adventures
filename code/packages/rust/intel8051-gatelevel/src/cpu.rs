//! Intel 8051 gate-level CPU simulator.
//!
//! # Memory model (Harvard architecture)
//!
//! Three independent address spaces, all indexed without overlap:
//!
//! | Space   | Size  | Access instructions |
//! |---------|-------|---------------------|
//! | `code`  | 64 KB | MOVC, implicit fetch |
//! | `iram`  | 256 B | MOV (direct/Rn), PUSH/POP |
//! | `xdata` | 64 KB | MOVX |
//!
//! # SFR constants
//!
//! ```text
//! SFR_P0  = 0x80   SFR_SP  = 0x81   SFR_DPL = 0x82   SFR_DPH = 0x83
//! SFR_P1  = 0x90   SFR_P2  = 0xA0   SFR_P3  = 0xB0
//! SFR_PSW = 0xD0   SFR_ACC = 0xE0   SFR_B   = 0xF0
//! ```
//!
//! # Halt sentinel
//!
//! Opcode `0xA5` is reserved/undefined on the real 8051.  This simulator
//! uses it as a soft HALT: `step()` sets `halted=true` and returns.

use crate::alu::{
    add8, anl8, da8, dec8, div8, inc8, mul8, orl8, rl8, rlc8, rr8, rrc8, subb8, swap8, xrl8,
};
use crate::bits::{add_16bit_full, int_to_bits8};
use crate::registers::RegisterFile8051;
use logic_gates::gates::{and_gate, not_gate, or_gate};

// ─── SFR addresses ────────────────────────────────────────────────────────────

const SFR_P0: u8 = 0x80;
const SFR_SP: u8 = 0x81;
const SFR_DPL: u8 = 0x82;
const SFR_DPH: u8 = 0x83;
const SFR_P1: u8 = 0x90;
const SFR_P2: u8 = 0xA0;
const SFR_P3: u8 = 0xB0;
const SFR_PSW: u8 = 0xD0;
const SFR_ACC: u8 = 0xE0;
const SFR_B: u8 = 0xF0;

// ─── PSW flag masks ───────────────────────────────────────────────────────────

const PSW_CY: u8 = 0x80; // bit 7
const PSW_AC: u8 = 0x40; // bit 6
const PSW_OV: u8 = 0x04; // bit 2
const PSW_P: u8 = 0x01; // bit 0

/// Soft-halt opcode: 0xA5 is undefined/reserved on real 8051.
const HALT_OPCODE: u8 = 0xA5;

// ─── CPU struct ───────────────────────────────────────────────────────────────

/// Intel 8051 gate-level simulator.
///
/// All arithmetic and logical operations in the data path route through
/// gate primitives from the `logic-gates` and `arithmetic` crates.
///
/// # Harvard memory model
///
/// The 8051 has three separate address spaces.  Code memory is loaded via
/// `execute()` or `load()`.  External data memory (`xdata`) is accessible
/// only through MOVX instructions.
///
/// # Example
///
/// ```
/// use coding_adventures_intel8051_gatelevel::cpu::Cpu8051;
/// let mut cpu = Cpu8051::new();
/// // MOV A, #0x0A; MOV R0, #0x05; ADD A, R0; HALT
/// cpu.execute(&[0x74, 0x0A, 0x78, 0x05, 0x28, 0xA5], 0, 100);
/// assert_eq!(cpu.rf.read_iram8(0xE0), 0x0F); // ACC = 0x0A + 0x05
/// assert!(cpu.halted);
/// ```
pub struct Cpu8051 {
    /// Register file: IRAM (including SFRs) + PC.
    pub rf: RegisterFile8051,
    /// Code memory: 64 KB, read-only during execution.
    pub code: Vec<u8>,
    /// External data memory: 64 KB, accessed via MOVX.
    pub xdata: Vec<u8>,
    /// True after a HALT opcode (0xA5) is executed.
    pub halted: bool,
}

impl Cpu8051 {
    /// Create a new CPU with zeroed memory.
    pub fn new() -> Self {
        Self {
            rf: RegisterFile8051::new(),
            code: vec![0u8; 65536],
            xdata: vec![0u8; 65536],
            halted: false,
        }
    }

    /// Reset to power-on state.
    ///
    /// Clears IRAM (except SFR initial values), sets PC=0, SP=0x07,
    /// P0-P3=0xFF (all port latches high).
    pub fn reset(&mut self) {
        self.rf = RegisterFile8051::new();
        self.halted = false;
        // Initial SFR values
        self.rf.write_iram8(SFR_SP, 0x07);
        self.rf.write_iram8(SFR_P0, 0xFF);
        self.rf.write_iram8(SFR_P1, 0xFF);
        self.rf.write_iram8(SFR_P2, 0xFF);
        self.rf.write_iram8(SFR_P3, 0xFF);
    }

    /// Load a program into code memory at `origin` and reset CPU state.
    ///
    /// Bytes that fall past address 0xFFFF are silently dropped.  In debug
    /// builds an assertion fires if the program does not fit entirely within
    /// the code space, making oversized loads visible during development.
    pub fn load(&mut self, program: &[u8], origin: u16) {
        self.reset();
        let start = origin as usize;
        let available = 65536usize.saturating_sub(start);
        debug_assert!(
            program.len() <= available,
            "load: program length {} exceeds available code space {} at origin {:#06x}",
            program.len(), available, origin
        );
        let end = (start + program.len()).min(65536);
        self.code[start..end].copy_from_slice(&program[..end - start]);
        self.rf.write_pc(origin);
    }

    /// Run a program until HALT or `max_steps` exceeded.  Returns step count.
    ///
    /// Equivalent to `load(program, origin)` then stepping until halted.
    pub fn execute(&mut self, program: &[u8], origin: u16, max_steps: u32) -> u32 {
        self.load(program, origin);
        let mut steps = 0u32;
        while !self.halted && steps < max_steps {
            self.step();
            steps += 1;
        }
        steps
    }

    /// Execute one instruction.
    pub fn step(&mut self) {
        if self.halted {
            return;
        }
        let opcode = self.fetch8();
        self.execute_one(opcode);
    }

    // ── Fetch helpers ─────────────────────────────────────────────────────────

    /// Fetch one byte from code memory at PC, then increment PC.
    fn fetch8(&mut self) -> u8 {
        let pc = self.rf.read_pc();
        let byte = self.code[pc as usize];
        self.rf.increment_pc(1);
        byte
    }

    /// Fetch two bytes big-endian from code memory, advance PC by 2.
    fn fetch16(&mut self) -> u16 {
        let hi = self.fetch8();
        let lo = self.fetch8();
        ((hi as u16) << 8) | (lo as u16)
    }

    // ── Register bank helpers ──────────────────────────────────────────────────

    /// Return IRAM address of Rn in the current register bank.
    ///
    /// Bank is selected by PSW.RS1:RS0 (bits 4:3).
    fn rn_addr(&self, n: u8) -> u8 {
        let psw = self.rf.read_iram8(SFR_PSW);
        let bank = (psw >> 3) & 0x03;
        bank.wrapping_mul(8).wrapping_add(n & 0x07)
    }

    fn rn(&self, n: u8) -> u8 {
        self.rf.read_iram8(self.rn_addr(n))
    }

    fn set_rn(&mut self, n: u8, val: u8) {
        let addr = self.rn_addr(n);
        self.rf.write_iram8(addr, val);
    }

    // ── Accumulator helpers ───────────────────────────────────────────────────

    fn acc(&self) -> u8 {
        self.rf.read_iram8(SFR_ACC)
    }

    /// Write accumulator and recompute PSW.P (parity).
    fn set_acc(&mut self, val: u8) {
        self.rf.write_iram8(SFR_ACC, val);
        self.update_parity();
    }

    /// Recompute PSW.P from current ACC value via the gate-level XOR tree.
    fn update_parity(&mut self) {
        let acc = self.rf.read_iram8(SFR_ACC);
        let p = crate::bits::compute_parity(&int_to_bits8(acc));
        let psw = self.rf.read_iram8(SFR_PSW);
        if p != 0 {
            self.rf.write_iram8(SFR_PSW, psw | PSW_P);
        } else {
            self.rf.write_iram8(SFR_PSW, psw & !PSW_P);
        }
    }

    // ── PSW flag helpers ──────────────────────────────────────────────────────

    fn cy(&self) -> u8 {
        (self.rf.read_iram8(SFR_PSW) >> 7) & 1
    }

    fn set_cy(&mut self, cy: u8) {
        let psw = self.rf.read_iram8(SFR_PSW);
        if cy & 1 != 0 {
            self.rf.write_iram8(SFR_PSW, psw | PSW_CY);
        } else {
            self.rf.write_iram8(SFR_PSW, psw & !PSW_CY);
        }
    }

    /// Apply a full ALU result to ACC and PSW (CY, AC, OV, P).
    fn apply_alu_result(&mut self, res: &crate::alu::AluResult8051) {
        self.rf.write_iram8(SFR_ACC, res.result);
        let mut psw = self.rf.read_iram8(SFR_PSW);
        psw &= !(PSW_CY | PSW_AC | PSW_OV | PSW_P);
        if res.cy != 0 { psw |= PSW_CY; }
        if res.ac != 0 { psw |= PSW_AC; }
        if res.ov != 0 { psw |= PSW_OV; }
        if res.parity != 0 { psw |= PSW_P; }
        self.rf.write_iram8(SFR_PSW, psw);
    }

    /// Update CY, AC, OV in PSW without touching ACC or parity.
    fn set_flags_cy_ac_ov(&mut self, cy: u8, ac: u8, ov: u8) {
        let mut psw = self.rf.read_iram8(SFR_PSW);
        psw &= !(PSW_CY | PSW_AC | PSW_OV);
        if cy != 0 { psw |= PSW_CY; }
        if ac != 0 { psw |= PSW_AC; }
        if ov != 0 { psw |= PSW_OV; }
        self.rf.write_iram8(SFR_PSW, psw);
    }

    // ── DPTR helpers ──────────────────────────────────────────────────────────

    fn dptr(&self) -> u16 {
        let dph = self.rf.read_iram8(SFR_DPH) as u16;
        let dpl = self.rf.read_iram8(SFR_DPL) as u16;
        (dph << 8) | dpl
    }

    fn set_dptr(&mut self, val: u16) {
        self.rf.write_iram8(SFR_DPH, (val >> 8) as u8);
        self.rf.write_iram8(SFR_DPL, val as u8);
    }

    // ── Direct / indirect IRAM access ────────────────────────────────────────

    fn direct_read(&self, addr: u8) -> u8 {
        self.rf.read_iram8(addr)
    }

    fn direct_write(&mut self, addr: u8, val: u8) {
        self.rf.write_iram8(addr, val);
        // If ACC was written directly, recompute parity
        if addr == SFR_ACC {
            self.update_parity();
        }
    }

    /// Read via register-indirect addressing (@R0 or @R1).
    ///
    /// On the base 8051, indirect addressing only reaches IRAM[0x00-0x7F].
    fn indirect_read(&self, ri: u8) -> u8 {
        let addr = self.rf.read_iram8(self.rn_addr(ri & 1));
        self.rf.read_iram8(addr)
    }

    fn indirect_write(&mut self, ri: u8, val: u8) {
        let addr = self.rf.read_iram8(self.rn_addr(ri & 1));
        self.rf.write_iram8(addr, val);
    }

    // ── Bit addressing wrappers ───────────────────────────────────────────────

    fn read_bit_addr(&self, bit_addr: u8) -> u8 {
        self.rf.read_bit(bit_addr)
    }

    fn write_bit_addr(&mut self, bit_addr: u8, val: u8) {
        self.rf.write_bit(bit_addr, val & 1);
        // Recompute parity if ACC bit was changed
        let (byte_addr, _) = self.rf.resolve_bit_addr(bit_addr);
        if byte_addr == SFR_ACC {
            self.update_parity();
        }
    }

    // ── Stack helpers ─────────────────────────────────────────────────────────

    /// Push one byte: SP++; IRAM[SP] = val.
    ///
    /// On the real 8051 the SP wraps from 0xFF back to 0x00 silently,
    /// corrupting the register banks (0x00-0x1F).  In debug builds this
    /// assertion makes stack overflow visible instead of silently corrupting
    /// state.
    fn push8(&mut self, val: u8) {
        let sp = self.rf.read_iram8(SFR_SP);
        debug_assert!(sp != 0xFF, "push8: stack pointer overflow (SP wrapped 0xFF → 0x00)");
        let new_sp = inc8(sp).result; // gate-level increment
        self.rf.write_iram8(SFR_SP, new_sp);
        self.rf.write_iram8(new_sp, val);
    }

    /// Pop one byte: val = IRAM[SP]; SP--.
    fn pop8(&mut self) -> u8 {
        let sp = self.rf.read_iram8(SFR_SP);
        let val = self.rf.read_iram8(sp);
        let new_sp = dec8(sp).result; // gate-level decrement
        self.rf.write_iram8(SFR_SP, new_sp);
        val
    }

    /// Push 16-bit PC: low byte first, then high byte (8051 convention).
    fn push_pc(&mut self) {
        let pc = self.rf.read_pc();
        self.push8(pc as u8);
        self.push8((pc >> 8) as u8);
    }

    /// Pop 16-bit PC: high byte first, then low byte.
    fn pop_pc(&mut self) {
        let hi = self.pop8();
        let lo = self.pop8();
        self.rf.write_pc(((hi as u16) << 8) | (lo as u16));
    }

    // ── Branch helper ─────────────────────────────────────────────────────────

    /// Sign-extend an 8-bit relative offset to i16.
    fn sign_extend_rel8(rel: u8) -> i16 {
        if rel >= 0x80 { (rel as i16) - 0x100 } else { rel as i16 }
    }

    /// Apply a signed 8-bit relative offset to PC using the gate-level adder.
    fn branch_by(&mut self, rel: i16) {
        let pc = self.rf.read_pc();
        // Two's complement cast: negative i16 → wrapping u16
        let (new_pc, _) = add_16bit_full(pc, rel as u16, 0);
        self.rf.write_pc(new_pc);
    }

    // ── Instruction execution ─────────────────────────────────────────────────

    #[allow(clippy::cognitive_complexity)]
    fn execute_one(&mut self, opcode: u8) {
        // ── Soft HALT ────────────────────────────────────────────────────────
        if opcode == HALT_OPCODE {
            self.halted = true;
            return;
        }

        // ── NOP ──────────────────────────────────────────────────────────────
        if opcode == 0x00 {
            return;
        }

        // ═══════════════════════════════════════════════════════════════════
        // MOV — data transfer
        // ═══════════════════════════════════════════════════════════════════

        // MOV A, Rn  (0xE8-0xEF)
        if (0xE8..=0xEF).contains(&opcode) {
            self.set_acc(self.rn(opcode & 7));
            return;
        }
        // MOV A, dir  (0xE5)
        if opcode == 0xE5 {
            let d = self.fetch8();
            self.set_acc(self.direct_read(d));
            return;
        }
        // MOV A, @Ri  (0xE6-0xE7)
        if opcode == 0xE6 || opcode == 0xE7 {
            self.set_acc(self.indirect_read(opcode & 1));
            return;
        }
        // MOV A, #imm  (0x74)
        if opcode == 0x74 {
            let imm = self.fetch8();
            self.set_acc(imm);
            return;
        }

        // MOV Rn, A  (0xF8-0xFF)
        if (0xF8..=0xFF).contains(&opcode) {
            self.set_rn(opcode & 7, self.acc());
            return;
        }
        // MOV Rn, dir  (0xA8-0xAF)
        if (0xA8..=0xAF).contains(&opcode) {
            let d = self.fetch8();
            let v = self.direct_read(d);
            self.set_rn(opcode & 7, v);
            return;
        }
        // MOV Rn, #imm  (0x78-0x7F)
        if (0x78..=0x7F).contains(&opcode) {
            let imm = self.fetch8();
            self.set_rn(opcode & 7, imm);
            return;
        }

        // MOV dir, A  (0xF5)
        if opcode == 0xF5 {
            let d = self.fetch8();
            self.direct_write(d, self.acc());
            return;
        }
        // MOV dir, Rn  (0x88-0x8F)
        if (0x88..=0x8F).contains(&opcode) {
            let d = self.fetch8();
            let v = self.rn(opcode & 7);
            self.direct_write(d, v);
            return;
        }
        // MOV dir, dir2  (0x85)  — NOTE: src byte comes first in encoding
        if opcode == 0x85 {
            let src = self.fetch8();
            let dst = self.fetch8();
            let v = self.direct_read(src);
            self.direct_write(dst, v);
            return;
        }
        // MOV dir, @Ri  (0x86-0x87)
        if opcode == 0x86 || opcode == 0x87 {
            let d = self.fetch8();
            let v = self.indirect_read(opcode & 1);
            self.direct_write(d, v);
            return;
        }
        // MOV dir, #imm  (0x75)
        if opcode == 0x75 {
            let d = self.fetch8();
            let imm = self.fetch8();
            self.direct_write(d, imm);
            return;
        }

        // MOV @Ri, A  (0xF6-0xF7)
        if opcode == 0xF6 || opcode == 0xF7 {
            self.indirect_write(opcode & 1, self.acc());
            return;
        }
        // MOV @Ri, dir  (0xA6-0xA7)
        if opcode == 0xA6 || opcode == 0xA7 {
            let d = self.fetch8();
            let v = self.direct_read(d);
            self.indirect_write(opcode & 1, v);
            return;
        }
        // MOV @Ri, #imm  (0x76-0x77)
        if opcode == 0x76 || opcode == 0x77 {
            let imm = self.fetch8();
            self.indirect_write(opcode & 1, imm);
            return;
        }

        // MOV DPTR, #imm16  (0x90)
        if opcode == 0x90 {
            let v = self.fetch16();
            self.set_dptr(v);
            return;
        }

        // MOVC A, @A+DPTR  (0x93) — code table lookup
        if opcode == 0x93 {
            let acc = self.acc() as u16;
            let dptr = self.dptr();
            let (ea, _) = add_16bit_full(acc, dptr, 0);
            self.set_acc(self.code[ea as usize]);
            return;
        }
        // MOVC A, @A+PC  (0x83)
        if opcode == 0x83 {
            let acc = self.acc() as u16;
            let pc = self.rf.read_pc();
            let (ea, _) = add_16bit_full(acc, pc, 0);
            self.set_acc(self.code[ea as usize]);
            return;
        }

        // MOVX A, @Ri  (0xE2-0xE3)
        if opcode == 0xE2 || opcode == 0xE3 {
            let addr = self.rn(opcode & 1) as usize;
            self.set_acc(self.xdata[addr]);
            return;
        }
        // MOVX A, @DPTR  (0xE0)
        if opcode == 0xE0 {
            let addr = self.dptr() as usize;
            self.set_acc(self.xdata[addr]);
            return;
        }
        // MOVX @Ri, A  (0xF2-0xF3)
        if opcode == 0xF2 || opcode == 0xF3 {
            let addr = self.rn(opcode & 1) as usize;
            let v = self.acc();
            self.xdata[addr] = v;
            return;
        }
        // MOVX @DPTR, A  (0xF0)
        if opcode == 0xF0 {
            let addr = self.dptr() as usize;
            let v = self.acc();
            self.xdata[addr] = v;
            return;
        }

        // ═══════════════════════════════════════════════════════════════════
        // Stack
        // ═══════════════════════════════════════════════════════════════════

        // PUSH dir  (0xC0)
        if opcode == 0xC0 {
            let d = self.fetch8();
            let v = self.direct_read(d);
            self.push8(v);
            return;
        }
        // POP dir  (0xD0)  — note: 0xD0 also happens to be SFR_PSW, but
        // as an opcode 0xD0 is POP.  The SFR address 0xD0 is only used as
        // a memory location argument, not as an opcode.
        if opcode == 0xD0 {
            let d = self.fetch8();
            let v = self.pop8();
            self.direct_write(d, v);
            return;
        }

        // ═══════════════════════════════════════════════════════════════════
        // Exchange
        // ═══════════════════════════════════════════════════════════════════

        // XCH A, Rn  (0xC8-0xCF)
        if (0xC8..=0xCF).contains(&opcode) {
            let n = opcode & 7;
            let a = self.acc();
            let rn = self.rn(n);
            self.set_acc(rn);
            self.set_rn(n, a);
            return;
        }
        // XCH A, dir  (0xC5)
        if opcode == 0xC5 {
            let d = self.fetch8();
            let a = self.acc();
            let m = self.direct_read(d);
            self.set_acc(m);
            self.direct_write(d, a);
            return;
        }
        // XCH A, @Ri  (0xC6-0xC7)
        if opcode == 0xC6 || opcode == 0xC7 {
            let i = opcode & 1;
            let a = self.acc();
            let m = self.indirect_read(i);
            self.set_acc(m);
            self.indirect_write(i, a);
            return;
        }
        // XCHD A, @Ri  (0xD6-0xD7) — swap lower nibbles only
        if opcode == 0xD6 || opcode == 0xD7 {
            let i = opcode & 1;
            let a = self.acc();
            let m = self.indirect_read(i);
            // Gate-level nibble isolation: ANL and ORL gates
            let high_a = anl8(a, 0xF0).result;
            let low_m = anl8(m, 0x0F).result;
            let new_a = orl8(high_a, low_m).result;
            let high_m = anl8(m, 0xF0).result;
            let low_a = anl8(a, 0x0F).result;
            let new_m = orl8(high_m, low_a).result;
            self.set_acc(new_a);
            self.indirect_write(i, new_m);
            return;
        }

        // ═══════════════════════════════════════════════════════════════════
        // Arithmetic — ADD / ADDC
        // ═══════════════════════════════════════════════════════════════════

        // ADD A, Rn  (0x28-0x2F)
        if (0x28..=0x2F).contains(&opcode) {
            let res = add8(self.acc(), self.rn(opcode & 7), 0);
            self.apply_alu_result(&res);
            return;
        }
        // ADD A, dir  (0x25)
        if opcode == 0x25 {
            let d = self.fetch8();
            let res = add8(self.acc(), self.direct_read(d), 0);
            self.apply_alu_result(&res);
            return;
        }
        // ADD A, @Ri  (0x26-0x27)
        if opcode == 0x26 || opcode == 0x27 {
            let res = add8(self.acc(), self.indirect_read(opcode & 1), 0);
            self.apply_alu_result(&res);
            return;
        }
        // ADD A, #imm  (0x24)
        if opcode == 0x24 {
            let imm = self.fetch8();
            let res = add8(self.acc(), imm, 0);
            self.apply_alu_result(&res);
            return;
        }

        // ADDC A, Rn  (0x38-0x3F)
        if (0x38..=0x3F).contains(&opcode) {
            let res = add8(self.acc(), self.rn(opcode & 7), self.cy());
            self.apply_alu_result(&res);
            return;
        }
        // ADDC A, dir  (0x35)
        if opcode == 0x35 {
            let d = self.fetch8();
            let cy = self.cy();
            let res = add8(self.acc(), self.direct_read(d), cy);
            self.apply_alu_result(&res);
            return;
        }
        // ADDC A, @Ri  (0x36-0x37)
        if opcode == 0x36 || opcode == 0x37 {
            let cy = self.cy();
            let res = add8(self.acc(), self.indirect_read(opcode & 1), cy);
            self.apply_alu_result(&res);
            return;
        }
        // ADDC A, #imm  (0x34)
        if opcode == 0x34 {
            let imm = self.fetch8();
            let cy = self.cy();
            let res = add8(self.acc(), imm, cy);
            self.apply_alu_result(&res);
            return;
        }

        // ═══════════════════════════════════════════════════════════════════
        // Arithmetic — SUBB
        // ═══════════════════════════════════════════════════════════════════

        // SUBB A, Rn  (0x98-0x9F)
        if (0x98..=0x9F).contains(&opcode) {
            let cy = self.cy();
            let res = subb8(self.acc(), self.rn(opcode & 7), cy);
            self.apply_alu_result(&res);
            return;
        }
        // SUBB A, dir  (0x95)
        if opcode == 0x95 {
            let d = self.fetch8();
            let cy = self.cy();
            let res = subb8(self.acc(), self.direct_read(d), cy);
            self.apply_alu_result(&res);
            return;
        }
        // SUBB A, @Ri  (0x96-0x97)
        if opcode == 0x96 || opcode == 0x97 {
            let cy = self.cy();
            let res = subb8(self.acc(), self.indirect_read(opcode & 1), cy);
            self.apply_alu_result(&res);
            return;
        }
        // SUBB A, #imm  (0x94)
        if opcode == 0x94 {
            let imm = self.fetch8();
            let cy = self.cy();
            let res = subb8(self.acc(), imm, cy);
            self.apply_alu_result(&res);
            return;
        }

        // ═══════════════════════════════════════════════════════════════════
        // INC / DEC
        // ═══════════════════════════════════════════════════════════════════

        // INC A  (0x04) — does NOT update CY/AC/OV
        if opcode == 0x04 {
            let res = inc8(self.acc());
            self.rf.write_iram8(SFR_ACC, res.result);
            self.update_parity();
            return;
        }
        // INC Rn  (0x08-0x0F)
        if (0x08..=0x0F).contains(&opcode) {
            let n = opcode & 7;
            let v = inc8(self.rn(n)).result;
            self.set_rn(n, v);
            return;
        }
        // INC dir  (0x05)
        if opcode == 0x05 {
            let d = self.fetch8();
            let v = inc8(self.direct_read(d)).result;
            self.direct_write(d, v);
            return;
        }
        // INC @Ri  (0x06-0x07)
        if opcode == 0x06 || opcode == 0x07 {
            let i = opcode & 1;
            let v = inc8(self.indirect_read(i)).result;
            self.indirect_write(i, v);
            return;
        }
        // INC DPTR  (0xA3)
        if opcode == 0xA3 {
            let (new_dptr, _) = add_16bit_full(self.dptr(), 1, 0);
            self.set_dptr(new_dptr);
            return;
        }

        // DEC A  (0x14) — does NOT update CY/AC/OV
        if opcode == 0x14 {
            let res = dec8(self.acc());
            self.rf.write_iram8(SFR_ACC, res.result);
            self.update_parity();
            return;
        }
        // DEC Rn  (0x18-0x1F)
        if (0x18..=0x1F).contains(&opcode) {
            let n = opcode & 7;
            let v = dec8(self.rn(n)).result;
            self.set_rn(n, v);
            return;
        }
        // DEC dir  (0x15)
        if opcode == 0x15 {
            let d = self.fetch8();
            let v = dec8(self.direct_read(d)).result;
            self.direct_write(d, v);
            return;
        }
        // DEC @Ri  (0x16-0x17)
        if opcode == 0x16 || opcode == 0x17 {
            let i = opcode & 1;
            let v = dec8(self.indirect_read(i)).result;
            self.indirect_write(i, v);
            return;
        }

        // MUL AB  (0xA4)
        if opcode == 0xA4 {
            let a = self.acc();
            let b = self.rf.read_iram8(SFR_B);
            let (hi, lo, ov) = mul8(a, b);
            self.rf.write_iram8(SFR_ACC, lo);
            self.rf.write_iram8(SFR_B, hi);
            self.set_flags_cy_ac_ov(0, 0, ov);
            self.update_parity();
            return;
        }

        // DIV AB  (0x84)
        if opcode == 0x84 {
            let a = self.acc();
            let b = self.rf.read_iram8(SFR_B);
            let (q, r, ov) = div8(a, b);
            self.rf.write_iram8(SFR_ACC, q);
            self.rf.write_iram8(SFR_B, r);
            self.set_flags_cy_ac_ov(0, 0, ov);
            self.update_parity();
            return;
        }

        // DA A  (0xD4)
        if opcode == 0xD4 {
            let psw = self.rf.read_iram8(SFR_PSW);
            let cy_in = (psw >> 7) & 1;
            let ac_in = (psw >> 6) & 1;
            let res = da8(self.acc(), cy_in, ac_in);
            self.rf.write_iram8(SFR_ACC, res.result);
            self.set_cy(res.cy);
            self.update_parity();
            return;
        }

        // ═══════════════════════════════════════════════════════════════════
        // Logical — ANL / ORL / XRL / CLR / CPL
        // ═══════════════════════════════════════════════════════════════════

        // ANL A, Rn  (0x58-0x5F)
        if (0x58..=0x5F).contains(&opcode) {
            let res = anl8(self.acc(), self.rn(opcode & 7));
            self.set_acc(res.result);
            return;
        }
        // ANL A, dir  (0x55)
        if opcode == 0x55 {
            let d = self.fetch8();
            let res = anl8(self.acc(), self.direct_read(d));
            self.set_acc(res.result);
            return;
        }
        // ANL A, @Ri  (0x56-0x57)
        if opcode == 0x56 || opcode == 0x57 {
            let res = anl8(self.acc(), self.indirect_read(opcode & 1));
            self.set_acc(res.result);
            return;
        }
        // ANL A, #imm  (0x54)
        if opcode == 0x54 {
            let imm = self.fetch8();
            let res = anl8(self.acc(), imm);
            self.set_acc(res.result);
            return;
        }
        // ANL dir, A  (0x52)
        if opcode == 0x52 {
            let d = self.fetch8();
            let res = anl8(self.direct_read(d), self.acc());
            self.direct_write(d, res.result);
            return;
        }
        // ANL dir, #imm  (0x53)
        if opcode == 0x53 {
            let d = self.fetch8();
            let imm = self.fetch8();
            let res = anl8(self.direct_read(d), imm);
            self.direct_write(d, res.result);
            return;
        }

        // ORL A, Rn  (0x48-0x4F)
        if (0x48..=0x4F).contains(&opcode) {
            let res = orl8(self.acc(), self.rn(opcode & 7));
            self.set_acc(res.result);
            return;
        }
        // ORL A, dir  (0x45)
        if opcode == 0x45 {
            let d = self.fetch8();
            let res = orl8(self.acc(), self.direct_read(d));
            self.set_acc(res.result);
            return;
        }
        // ORL A, @Ri  (0x46-0x47)
        if opcode == 0x46 || opcode == 0x47 {
            let res = orl8(self.acc(), self.indirect_read(opcode & 1));
            self.set_acc(res.result);
            return;
        }
        // ORL A, #imm  (0x44)
        if opcode == 0x44 {
            let imm = self.fetch8();
            let res = orl8(self.acc(), imm);
            self.set_acc(res.result);
            return;
        }
        // ORL dir, A  (0x42)
        if opcode == 0x42 {
            let d = self.fetch8();
            let res = orl8(self.direct_read(d), self.acc());
            self.direct_write(d, res.result);
            return;
        }
        // ORL dir, #imm  (0x43)
        if opcode == 0x43 {
            let d = self.fetch8();
            let imm = self.fetch8();
            let res = orl8(self.direct_read(d), imm);
            self.direct_write(d, res.result);
            return;
        }

        // XRL A, Rn  (0x68-0x6F)
        if (0x68..=0x6F).contains(&opcode) {
            let res = xrl8(self.acc(), self.rn(opcode & 7));
            self.set_acc(res.result);
            return;
        }
        // XRL A, dir  (0x65)
        if opcode == 0x65 {
            let d = self.fetch8();
            let res = xrl8(self.acc(), self.direct_read(d));
            self.set_acc(res.result);
            return;
        }
        // XRL A, @Ri  (0x66-0x67)
        if opcode == 0x66 || opcode == 0x67 {
            let res = xrl8(self.acc(), self.indirect_read(opcode & 1));
            self.set_acc(res.result);
            return;
        }
        // XRL A, #imm  (0x64)
        if opcode == 0x64 {
            let imm = self.fetch8();
            let res = xrl8(self.acc(), imm);
            self.set_acc(res.result);
            return;
        }
        // XRL dir, A  (0x62)
        if opcode == 0x62 {
            let d = self.fetch8();
            let res = xrl8(self.direct_read(d), self.acc());
            self.direct_write(d, res.result);
            return;
        }
        // XRL dir, #imm  (0x63)
        if opcode == 0x63 {
            let d = self.fetch8();
            let imm = self.fetch8();
            let res = xrl8(self.direct_read(d), imm);
            self.direct_write(d, res.result);
            return;
        }

        // CLR A  (0xE4)
        if opcode == 0xE4 {
            self.set_acc(0);
            return;
        }
        // CPL A  (0xF4) — bitwise complement via XRL with 0xFF (8 XOR gates)
        if opcode == 0xF4 {
            let res = xrl8(self.acc(), 0xFF);
            self.set_acc(res.result);
            return;
        }

        // ── Rotate ───────────────────────────────────────────────────────────

        // RL A  (0x23)
        if opcode == 0x23 {
            let res = rl8(self.acc());
            self.rf.write_iram8(SFR_ACC, res.result);
            self.set_cy(res.cy);
            self.update_parity();
            return;
        }
        // RLC A  (0x33)
        if opcode == 0x33 {
            let cy = self.cy();
            let res = rlc8(self.acc(), cy);
            self.rf.write_iram8(SFR_ACC, res.result);
            self.set_cy(res.cy);
            self.update_parity();
            return;
        }
        // RR A  (0x03)
        if opcode == 0x03 {
            let res = rr8(self.acc());
            self.rf.write_iram8(SFR_ACC, res.result);
            self.set_cy(res.cy);
            self.update_parity();
            return;
        }
        // RRC A  (0x13)
        if opcode == 0x13 {
            let cy = self.cy();
            let res = rrc8(self.acc(), cy);
            self.rf.write_iram8(SFR_ACC, res.result);
            self.set_cy(res.cy);
            self.update_parity();
            return;
        }
        // SWAP A  (0xC4)
        if opcode == 0xC4 {
            let res = swap8(self.acc());
            // SWAP does NOT update parity
            self.rf.write_iram8(SFR_ACC, res.result);
            return;
        }

        // ═══════════════════════════════════════════════════════════════════
        // Bit operations
        // ═══════════════════════════════════════════════════════════════════

        // CLR C  (0xC3)
        if opcode == 0xC3 {
            self.set_cy(0);
            return;
        }
        // CLR bit  (0xC2)
        if opcode == 0xC2 {
            let b = self.fetch8();
            self.write_bit_addr(b, 0);
            return;
        }
        // SETB C  (0xD3)
        if opcode == 0xD3 {
            self.set_cy(1);
            return;
        }
        // SETB bit  (0xD2)
        if opcode == 0xD2 {
            let b = self.fetch8();
            self.write_bit_addr(b, 1);
            return;
        }
        // CPL C  (0xB3) — complement carry via NOT gate
        if opcode == 0xB3 {
            let cy = self.cy();
            self.set_cy(not_gate(cy));
            return;
        }
        // CPL bit  (0xB2) — complement a bit-addressable bit
        if opcode == 0xB2 {
            let b = self.fetch8();
            let val = not_gate(self.read_bit_addr(b));
            self.write_bit_addr(b, val);
            return;
        }
        // ANL C, bit  (0x82)
        if opcode == 0x82 {
            let b = self.fetch8();
            let val = and_gate(self.cy(), self.read_bit_addr(b));
            self.set_cy(val);
            return;
        }
        // ANL C, /bit  (0xB0)
        if opcode == 0xB0 {
            let b = self.fetch8();
            let val = and_gate(self.cy(), not_gate(self.read_bit_addr(b)));
            self.set_cy(val);
            return;
        }
        // ORL C, bit  (0x72)
        if opcode == 0x72 {
            let b = self.fetch8();
            let val = or_gate(self.cy(), self.read_bit_addr(b));
            self.set_cy(val);
            return;
        }
        // ORL C, /bit  (0xA0)
        if opcode == 0xA0 {
            let b = self.fetch8();
            let val = or_gate(self.cy(), not_gate(self.read_bit_addr(b)));
            self.set_cy(val);
            return;
        }
        // MOV C, bit  (0xA2)
        if opcode == 0xA2 {
            let b = self.fetch8();
            let v = self.read_bit_addr(b);
            self.set_cy(v);
            return;
        }
        // MOV bit, C  (0x92)
        if opcode == 0x92 {
            let b = self.fetch8();
            let cy = self.cy();
            self.write_bit_addr(b, cy);
            return;
        }

        // ═══════════════════════════════════════════════════════════════════
        // Branch / Jump
        // ═══════════════════════════════════════════════════════════════════

        // LJMP addr16  (0x02)
        if opcode == 0x02 {
            let addr = self.fetch16();
            self.rf.write_pc(addr);
            return;
        }
        // SJMP rel  (0x80)
        if opcode == 0x80 {
            let rel = Self::sign_extend_rel8(self.fetch8());
            self.branch_by(rel);
            return;
        }
        // JMP @A+DPTR  (0x73)
        if opcode == 0x73 {
            let acc = self.acc() as u16;
            let (ea, _) = add_16bit_full(acc, self.dptr(), 0);
            self.rf.write_pc(ea);
            return;
        }
        // AJMP  — 11-bit absolute jump; opcode bits[7:5]=page_hi, byte2=addr_lo
        if (opcode & 0x1F) == 0x01 {
            let addr11_hi = (opcode >> 5) as u16 & 0x07;
            let addr11_lo = self.fetch8() as u16;
            let pc = self.rf.read_pc();
            let new_pc = (pc & 0xF800) | (addr11_hi << 8) | addr11_lo;
            self.rf.write_pc(new_pc);
            return;
        }

        // JZ rel  (0x60) — jump if ACC == 0
        if opcode == 0x60 {
            let rel = Self::sign_extend_rel8(self.fetch8());
            let acc_bits = int_to_bits8(self.acc());
            if crate::bits::compute_zero(&acc_bits) {
                self.branch_by(rel);
            }
            return;
        }
        // JNZ rel  (0x70)
        if opcode == 0x70 {
            let rel = Self::sign_extend_rel8(self.fetch8());
            let acc_bits = int_to_bits8(self.acc());
            if !crate::bits::compute_zero(&acc_bits) {
                self.branch_by(rel);
            }
            return;
        }
        // JC rel  (0x40)
        if opcode == 0x40 {
            let rel = Self::sign_extend_rel8(self.fetch8());
            if self.cy() != 0 {
                self.branch_by(rel);
            }
            return;
        }
        // JNC rel  (0x50)
        if opcode == 0x50 {
            let rel = Self::sign_extend_rel8(self.fetch8());
            if not_gate(self.cy()) != 0 {
                self.branch_by(rel);
            }
            return;
        }
        // JB bit, rel  (0x20)
        if opcode == 0x20 {
            let b = self.fetch8();
            let rel = Self::sign_extend_rel8(self.fetch8());
            if self.read_bit_addr(b) != 0 {
                self.branch_by(rel);
            }
            return;
        }
        // JNB bit, rel  (0x30)
        if opcode == 0x30 {
            let b = self.fetch8();
            let rel = Self::sign_extend_rel8(self.fetch8());
            if not_gate(self.read_bit_addr(b)) != 0 {
                self.branch_by(rel);
            }
            return;
        }
        // JBC bit, rel  (0x10) — jump if bit set, then clear bit
        if opcode == 0x10 {
            let b = self.fetch8();
            let rel = Self::sign_extend_rel8(self.fetch8());
            if self.read_bit_addr(b) != 0 {
                self.write_bit_addr(b, 0);
                self.branch_by(rel);
            }
            return;
        }

        // CJNE A, dir, rel  (0xB5)
        if opcode == 0xB5 {
            let d = self.fetch8();
            let rel = Self::sign_extend_rel8(self.fetch8());
            let val = self.direct_read(d);
            let a = self.acc();
            let cmp_res = subb8(a, val, 0);
            self.set_cy(cmp_res.cy);
            if a != val {
                self.branch_by(rel);
            }
            return;
        }
        // CJNE A, #imm, rel  (0xB4)
        if opcode == 0xB4 {
            let imm = self.fetch8();
            let rel = Self::sign_extend_rel8(self.fetch8());
            let a = self.acc();
            let cmp_res = subb8(a, imm, 0);
            self.set_cy(cmp_res.cy);
            if a != imm {
                self.branch_by(rel);
            }
            return;
        }
        // CJNE Rn, #imm, rel  (0xB8-0xBF)
        if (0xB8..=0xBF).contains(&opcode) {
            let n = opcode & 7;
            let imm = self.fetch8();
            let rel = Self::sign_extend_rel8(self.fetch8());
            let rn = self.rn(n);
            let cmp_res = subb8(rn, imm, 0);
            self.set_cy(cmp_res.cy);
            if rn != imm {
                self.branch_by(rel);
            }
            return;
        }
        // CJNE @Ri, #imm, rel  (0xB6-0xB7)
        if opcode == 0xB6 || opcode == 0xB7 {
            let i = opcode & 1;
            let imm = self.fetch8();
            let rel = Self::sign_extend_rel8(self.fetch8());
            let mem = self.indirect_read(i);
            let cmp_res = subb8(mem, imm, 0);
            self.set_cy(cmp_res.cy);
            if mem != imm {
                self.branch_by(rel);
            }
            return;
        }

        // DJNZ Rn, rel  (0xD8-0xDF)
        if (0xD8..=0xDF).contains(&opcode) {
            let n = opcode & 7;
            let rel = Self::sign_extend_rel8(self.fetch8());
            let v = dec8(self.rn(n)).result;
            self.set_rn(n, v);
            if v != 0 {
                self.branch_by(rel);
            }
            return;
        }
        // DJNZ dir, rel  (0xD5)
        if opcode == 0xD5 {
            let d = self.fetch8();
            let rel = Self::sign_extend_rel8(self.fetch8());
            let v = dec8(self.direct_read(d)).result;
            self.direct_write(d, v);
            if v != 0 {
                self.branch_by(rel);
            }
            return;
        }

        // ═══════════════════════════════════════════════════════════════════
        // Subroutine calls and returns
        // ═══════════════════════════════════════════════════════════════════

        // LCALL addr16  (0x12)
        if opcode == 0x12 {
            let addr = self.fetch16();
            self.push_pc();
            self.rf.write_pc(addr);
            return;
        }
        // ACALL  — 11-bit page call; opcode bits[7:5]=page_hi, byte2=addr_lo
        if (opcode & 0x1F) == 0x11 {
            let addr11_hi = (opcode >> 5) as u16 & 0x07;
            let addr11_lo = self.fetch8() as u16;
            self.push_pc();
            let pc = self.rf.read_pc();
            let new_pc = (pc & 0xF800) | (addr11_hi << 8) | addr11_lo;
            self.rf.write_pc(new_pc);
            return;
        }
        // RET  (0x22)
        if opcode == 0x22 {
            self.pop_pc();
            return;
        }
        // RETI  (0x32) — return from interrupt (same as RET for behavioural sim)
        if opcode == 0x32 {
            self.pop_pc();
        }

        // Unknown opcode — silently skip (undefined on real 8051)
    }
}

impl Default for Cpu8051 {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a program bytes slice, prepended with HALT sentinel (0xA5) terminator.
    fn prog(bytes: &[u8]) -> Vec<u8> {
        let mut v = bytes.to_vec();
        v.push(HALT_OPCODE);
        v
    }

    fn run(bytes: &[u8]) -> Cpu8051 {
        let mut cpu = Cpu8051::new();
        cpu.execute(&prog(bytes), 0, 1000);
        cpu
    }

    // ── NOP ──────────────────────────────────────────────────────────────────

    #[test]
    fn nop_does_nothing() {
        let cpu = run(&[0x00]);
        assert!(cpu.halted);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0);
    }

    // ── MOV immediate ─────────────────────────────────────────────────────────

    #[test]
    fn mov_a_imm() {
        let cpu = run(&[0x74, 0x42]); // MOV A, #0x42
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0x42);
    }

    #[test]
    fn mov_rn_imm() {
        let cpu = run(&[0x78, 0x55]); // MOV R0, #0x55
        assert_eq!(cpu.rf.read_iram8(0x00), 0x55);
    }

    #[test]
    fn mov_dir_imm() {
        let cpu = run(&[0x75, 0x30, 0xAB]); // MOV 0x30, #0xAB
        assert_eq!(cpu.rf.read_iram8(0x30), 0xAB);
    }

    // ── MOV register transfer ─────────────────────────────────────────────────

    #[test]
    fn mov_a_rn() {
        // MOV R1, #0x10; MOV A, R1
        let cpu = run(&[0x79, 0x10, 0xE9]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0x10);
    }

    #[test]
    fn mov_rn_a() {
        // MOV A, #0x20; MOV R2, A
        let cpu = run(&[0x74, 0x20, 0xFA]);
        assert_eq!(cpu.rf.read_iram8(0x02), 0x20);
    }

    #[test]
    fn mov_dir_a() {
        // MOV A, #0xFF; MOV 0x40, A
        let cpu = run(&[0x74, 0xFF, 0xF5, 0x40]);
        assert_eq!(cpu.rf.read_iram8(0x40), 0xFF);
    }

    // ── ADD ───────────────────────────────────────────────────────────────────

    #[test]
    fn add_a_imm() {
        // MOV A, #10; ADD A, #20
        let cpu = run(&[0x74, 10, 0x24, 20]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 30);
    }

    #[test]
    fn add_sets_carry() {
        // MOV A, #0xFF; ADD A, #0x01 → ACC=0x00, CY=1
        let cpu = run(&[0x74, 0xFF, 0x24, 0x01]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0x00);
        assert_eq!((cpu.rf.read_iram8(SFR_PSW) >> 7) & 1, 1); // CY=1
    }

    #[test]
    fn addc_uses_carry() {
        // MOV A, #0xFF; ADD A, #1; ADDC A, #0 → ACC=0x00+CY=1=1
        let cpu = run(&[0x74, 0xFF, 0x24, 0x01, 0x34, 0x00]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0x01); // 0 + 0 + CY(1) = 1
    }

    // ── SUBB ──────────────────────────────────────────────────────────────────

    #[test]
    fn subb_no_borrow() {
        // MOV A, #10; SUBB A, #3
        let cpu = run(&[0x74, 10, 0x94, 3]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 7);
        assert_eq!((cpu.rf.read_iram8(SFR_PSW) >> 7) & 1, 0); // CY=0
    }

    #[test]
    fn subb_with_borrow() {
        // MOV A, #1; SUBB A, #5 (1 - 5 → borrow)
        let cpu = run(&[0x74, 1, 0x94, 5]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0xFC); // 1-5 = -4 = 0xFC
        assert_eq!((cpu.rf.read_iram8(SFR_PSW) >> 7) & 1, 1); // CY=1
    }

    // ── INC / DEC ─────────────────────────────────────────────────────────────

    #[test]
    fn inc_a_no_cy() {
        // MOV A, #0xFF; INC A (wraps to 0, CY stays 0)
        let cpu = run(&[0x74, 0xFF, 0x04]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0x00);
        assert_eq!((cpu.rf.read_iram8(SFR_PSW) >> 7) & 1, 0); // CY=0
    }

    #[test]
    fn dec_rn() {
        // MOV R0, #5; DEC R0
        let cpu = run(&[0x78, 5, 0x18]);
        assert_eq!(cpu.rf.read_iram8(0x00), 4);
    }

    // ── ANL / ORL / XRL ──────────────────────────────────────────────────────

    #[test]
    fn anl_a_imm() {
        // MOV A, #0xFF; ANL A, #0x0F
        let cpu = run(&[0x74, 0xFF, 0x54, 0x0F]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0x0F);
    }

    #[test]
    fn orl_a_imm() {
        // MOV A, #0xF0; ORL A, #0x0F
        let cpu = run(&[0x74, 0xF0, 0x44, 0x0F]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0xFF);
    }

    #[test]
    fn xrl_a_imm() {
        // MOV A, #0xFF; XRL A, #0xFF → 0x00
        let cpu = run(&[0x74, 0xFF, 0x64, 0xFF]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0x00);
    }

    #[test]
    fn cpl_a() {
        // MOV A, #0x55; CPL A → 0xAA
        let cpu = run(&[0x74, 0x55, 0xF4]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0xAA);
    }

    // ── Rotate ───────────────────────────────────────────────────────────────

    #[test]
    fn rl_a() {
        // MOV A, #0b10000001; RL A → 0b00000011, CY=1
        let cpu = run(&[0x74, 0x81, 0x23]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0x03);
        assert_eq!((cpu.rf.read_iram8(SFR_PSW) >> 7) & 1, 1);
    }

    #[test]
    fn rr_a() {
        // MOV A, #0b10000001; RR A → 0b11000000, CY=1
        let cpu = run(&[0x74, 0x81, 0x03]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0xC0);
        assert_eq!((cpu.rf.read_iram8(SFR_PSW) >> 7) & 1, 1);
    }

    #[test]
    fn rlc_a() {
        // MOV A, #0x80; RLC A (CY_in=0) → A=0x00, CY=1
        let cpu = run(&[0x74, 0x80, 0x33]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0x00);
        assert_eq!((cpu.rf.read_iram8(SFR_PSW) >> 7) & 1, 1);
    }

    #[test]
    fn rrc_a() {
        // MOV A, #0x01; RRC A (CY_in=0) → A=0x00, CY=1
        let cpu = run(&[0x74, 0x01, 0x13]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0x00);
        assert_eq!((cpu.rf.read_iram8(SFR_PSW) >> 7) & 1, 1);
    }

    #[test]
    fn swap_a() {
        // MOV A, #0xAB; SWAP A → 0xBA
        let cpu = run(&[0x74, 0xAB, 0xC4]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0xBA);
    }

    // ── CLR / SETB ───────────────────────────────────────────────────────────

    #[test]
    fn clr_setb_carry() {
        // SETB C; CLR C
        let cpu = run(&[0xD3, 0xC3]);
        assert_eq!(cpu.cy(), 0);

        let mut cpu2 = Cpu8051::new();
        cpu2.execute(&prog(&[0xD3]), 0, 100);
        assert_eq!(cpu2.cy(), 1);
    }

    #[test]
    fn clr_bit() {
        // MOV dir, #0xFF; CLR bit (addr 0x00 = byte 0x20, bit 0)
        //   → set 0x20 to 0xFF, then CLR bit 0x00 → bit 0 of 0x20 = 0
        let mut cpu = Cpu8051::new();
        cpu.execute(&prog(&[0x75, 0x20, 0xFF, 0xC2, 0x00]), 0, 100);
        assert_eq!(cpu.rf.read_iram8(0x20), 0xFE); // bit 0 cleared
    }

    #[test]
    fn setb_bit() {
        // SETB bit 0x00 → byte 0x20, bit 0 = 1
        let cpu = run(&[0xD2, 0x00]);
        assert_eq!(cpu.rf.read_iram8(0x20) & 1, 1);
    }

    // ── DJNZ ─────────────────────────────────────────────────────────────────

    #[test]
    fn djnz_loop() {
        // MOV R0, #3; loop: INC A; DJNZ R0, loop(-3)
        // Executes 3 iterations: A = 0+1+1+1 = 3
        let cpu = run(&[0x78, 3, 0x04, 0xD8, 0xFD]);
        // Offset: -3 from after DJNZ opcode+offset = -3 means back to INC A
        // After fetch of DJNZ opcode (PC=4), fetch rel=0xFD=-3 (PC=5), branch to PC=5+(-3)=2
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 3);
    }

    // ── CJNE ─────────────────────────────────────────────────────────────────

    #[test]
    fn cjne_a_imm_branches() {
        // MOV A, #5; CJNE A, #3, +2; MOV A, #0; HALT
        // Since 5≠3, branch over MOV A, #0
        let cpu = run(&[0x74, 5, 0xB4, 3, 0x02, 0x74, 0x00]);
        // rel=+2: after PC=5 (past 3-byte CJNE), jump to PC=7 (HALT)
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 5); // not clobbered
    }

    #[test]
    fn cjne_a_imm_no_branch() {
        // MOV A, #3; CJNE A, #3, +2; HALT (at addr 5, so branch would be 7)
        let cpu = run(&[0x74, 3, 0xB4, 3, 0x02]);
        // 3==3 → no branch, continue to next byte (HALT)
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 3);
    }

    // ── LCALL / RET ──────────────────────────────────────────────────────────

    #[test]
    fn lcall_ret() {
        // addr 0: LCALL 0x0005   (0x12, 0x00, 0x05)   [3 bytes]
        // addr 3: HALT           (0xA5)                [return lands here]
        // addr 4: NOP            (0x00)                [padding]
        // addr 5: MOV A, #0xBE  (0x74, 0xBE)          [subroutine]
        // addr 7: RET            (0x22)
        let code = [0x12u8, 0x00, 0x05, HALT_OPCODE, 0x00, 0x74, 0xBE, 0x22];
        let mut cpu = Cpu8051::new();
        cpu.execute(&code, 0, 100);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0xBE);
    }

    // ── PUSH / POP ────────────────────────────────────────────────────────────

    #[test]
    fn push_pop() {
        // MOV R0, #0x42; PUSH 0x00 (R0); POP 0x01 (R1 slot)
        let cpu = run(&[0x78, 0x42, 0xC0, 0x00, 0xD0, 0x01]);
        assert_eq!(cpu.rf.read_iram8(0x01), 0x42);
    }

    // ── MUL / DIV ────────────────────────────────────────────────────────────

    #[test]
    fn mul_ab() {
        // MOV A, #12; MOV B, #13; MUL AB → 156 = 0x9C
        // MOV B = MOV dir #0xF0 (SFR_B), #13
        let cpu = run(&[0x74, 12, 0x75, 0xF0, 13, 0xA4]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 156); // lo=0x9C
        assert_eq!(cpu.rf.read_iram8(SFR_B), 0);    // hi=0, OV=0
    }

    #[test]
    fn div_ab() {
        // MOV A, #17; MOV B, #5; DIV AB → q=3, r=2
        let cpu = run(&[0x74, 17, 0x75, 0xF0, 5, 0x84]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 3);
        assert_eq!(cpu.rf.read_iram8(SFR_B), 2);
    }

    // ── Parity ───────────────────────────────────────────────────────────────

    #[test]
    fn parity_flag_set() {
        // MOV A, #0x01 (1 bit set → P=1)
        let cpu = run(&[0x74, 0x01]);
        assert_eq!(cpu.rf.read_iram8(SFR_PSW) & PSW_P, PSW_P);
    }

    #[test]
    fn parity_flag_clear() {
        // MOV A, #0x03 (2 bits set → P=0)
        let cpu = run(&[0x74, 0x03]);
        assert_eq!(cpu.rf.read_iram8(SFR_PSW) & PSW_P, 0);
    }

    // ── XCH ──────────────────────────────────────────────────────────────────

    #[test]
    fn xch_a_rn() {
        // MOV A, #0x11; MOV R0, #0x22; XCH A, R0
        let cpu = run(&[0x74, 0x11, 0x78, 0x22, 0xC8]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0x22);
        assert_eq!(cpu.rf.read_iram8(0x00), 0x11);
    }

    // ── INC DPTR ─────────────────────────────────────────────────────────────

    #[test]
    fn inc_dptr() {
        // MOV DPTR, #0xFFFE; INC DPTR; INC DPTR → 0x0000
        let cpu = run(&[0x90, 0xFF, 0xFE, 0xA3, 0xA3]);
        assert_eq!(cpu.dptr(), 0x0000);
    }

    // ── Bit boolean ops ───────────────────────────────────────────────────────

    #[test]
    fn anl_c_bit() {
        // SETB C; ANL C, /bit(0x00=0)  → C AND NOT(0) = 1 AND 1 = 1
        let cpu = run(&[0xD3, 0xB0, 0x00]);
        assert_eq!(cpu.cy(), 1);
    }

    #[test]
    fn cpl_carry() {
        // SETB C; CPL C → C=0
        let cpu = run(&[0xD3, 0xB3]);
        assert_eq!(cpu.cy(), 0);
    }

    // ── SJMP ─────────────────────────────────────────────────────────────────

    #[test]
    fn sjmp_forward() {
        // SJMP +2; MOV A, #0xFF (skipped); HALT
        // addr 0: 0x80 0x02  (SJMP +2)
        // addr 2: 0x74 0xFF (MOV A,#0xFF — skipped)
        // addr 4: HALT
        let cpu = run(&[0x80, 0x02, 0x74, 0xFF]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0);
    }

    // ── JZ / JNZ ─────────────────────────────────────────────────────────────

    #[test]
    fn jz_taken() {
        // MOV A, #0; JZ +1; MOV A, #1 (skipped); HALT
        // addr 0: 0x74 0x00
        // addr 2: 0x60 0x01  (JZ +1 → jumps to addr 5)
        // addr 4: 0x74 0x01  (skipped)
        // addr 6: HALT
        let cpu = run(&[0x74, 0x00, 0x60, 0x01, 0x74, 0x01]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 0);
    }

    #[test]
    fn jnz_taken() {
        // MOV A, #5; JNZ +1; MOV A, #0 (skipped); HALT
        let cpu = run(&[0x74, 0x05, 0x70, 0x01, 0x74, 0x00]);
        assert_eq!(cpu.rf.read_iram8(SFR_ACC), 5);
    }
}
