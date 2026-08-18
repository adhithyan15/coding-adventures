//! Instruction executor for the Intel 8080 ISA.
//!
//! Mutates CPU registers/flags/memory/ports according to a [`DecodeResult`].
//! `pc` is the address **immediately following** the fully-fetched
//! instruction (the default fallthrough target for non-control-flow ops);
//! control-flow ops (`jmp`/`jcond`/`call`/`ccond`/`ret`/`rcond`/`rst`/`pchl`)
//! override `next_pc` explicitly, mirroring how the Python original's
//! `self._pc` has already been advanced past every operand byte by the time
//! a given `_exec_*` branch runs (`_fetch_word`/`_fetch_byte` advance PC as
//! they read).
//!
//! # Flag arithmetic — masked-first instead of Python's mask-at-the-end
//!
//! The Python original computes `S`/`Z`/`P` from the **unmasked** sum/
//! difference (e.g. `result = a + b + carry`, possibly > 255) and only
//! masks into `self._a` afterward.  This is safe because `X & 0x80`,
//! `(X & 0xFF) == 0`, and parity-of-`(X & 0xFF)` only ever depend on the
//! low 8 bits of `X` — carries into bit 8+ cannot change bit 0-7's value.
//! This port therefore computes the masked `u8` result first (via
//! `u16`-widened arithmetic for carry-out detection) and derives S/Z/P
//! from that single masked byte — equivalent, and more idiomatic Rust
//! (mirrors `mips_r2000_simulator::execute`'s use of `overflowing_add`).

use cpu_simulator::Memory;

use crate::decode::DecodeResult;
use crate::opcodes::*;

/// The Intel 8080's seven 8-bit working registers plus the 16-bit stack
/// pointer.  PC lives on [`crate::simulator::Intel8080Simulator`] directly
/// (mirrors how `mips_r2000_simulator::MipsR2000Simulator` keeps `pc`
/// top-level rather than folding it into `RegisterFile`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Registers {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
}

impl Registers {
    pub fn bc(&self) -> u16 {
        ((self.b as u16) << 8) | self.c as u16
    }
    pub fn de(&self) -> u16 {
        ((self.d as u16) << 8) | self.e as u16
    }
    pub fn hl(&self) -> u16 {
        ((self.h as u16) << 8) | self.l as u16
    }
    fn set_hl(&mut self, v: u16) {
        self.h = (v >> 8) as u8;
        self.l = v as u8;
    }

    /// Read a register (or memory[HL] for the M pseudo-register, code 6)
    /// by its 3-bit code.
    fn read(&self, code: u8, mem: &Memory) -> u8 {
        match code {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => mem.read_byte(self.hl() as usize),
            _ => self.a,
        }
    }

    /// Write a register (or memory[HL] for M) by its 3-bit code.
    fn write(&mut self, code: u8, value: u8, mem: &mut Memory) {
        match code {
            0 => self.b = value,
            1 => self.c = value,
            2 => self.d = value,
            3 => self.e = value,
            4 => self.h = value,
            5 => self.l = value,
            6 => mem.write_byte(self.hl() as usize, value),
            _ => self.a = value,
        }
    }

    /// Read a register pair by its 2-bit code (0=BC, 1=DE, 2=HL, 3=SP).
    fn read_pair(&self, code: u8) -> u16 {
        match code {
            0 => self.bc(),
            1 => self.de(),
            2 => self.hl(),
            _ => self.sp,
        }
    }

    /// Write a register pair by its 2-bit code.
    fn write_pair(&mut self, code: u8, value: u16) {
        match code {
            0 => {
                self.b = (value >> 8) as u8;
                self.c = value as u8;
            }
            1 => {
                self.d = (value >> 8) as u8;
                self.e = value as u8;
            }
            2 => self.set_hl(value),
            _ => self.sp = value,
        }
    }
}

/// The five Intel 8080 condition flags.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    pub s: bool,
    pub z: bool,
    pub ac: bool,
    pub p: bool,
    pub cy: bool,
}

impl Flags {
    /// Pack into the 8080's flags byte (`S Z 0 AC 0 P 1 CY`) — used by
    /// `PUSH PSW`.  Bit 1 is a documented fixed `1`.
    pub fn to_byte(self) -> u8 {
        ((self.s as u8) << 7)
            | ((self.z as u8) << 6)
            | ((self.ac as u8) << 4)
            | ((self.p as u8) << 2)
            | (1 << 1)
            | (self.cy as u8)
    }

    /// Unpack from a flags byte — used by `POP PSW`.
    pub fn from_byte(b: u8) -> Self {
        Flags {
            s: b & 0x80 != 0,
            z: b & 0x40 != 0,
            ac: b & 0x10 != 0,
            p: b & 0x04 != 0,
            cy: b & 0x01 != 0,
        }
    }
}

/// Result of executing one instruction.
pub struct ExecuteResult {
    pub next_pc: u16,
    pub halted: bool,
}

fn get(d: &DecodeResult, key: &str) -> i32 {
    d.fields.get(key).copied().unwrap_or(0)
}

// ===========================================================================
// Flag-computation helpers (direct port of intel8080_simulator.flags)
// ===========================================================================

fn s_flag(r: u8) -> bool {
    r & 0x80 != 0
}
fn z_flag(r: u8) -> bool {
    r == 0
}
fn p_flag(r: u8) -> bool {
    r.count_ones().is_multiple_of(2)
}

/// `ADD`/`ADC` semantics: widen to `u16` so the carry-out (bit 8) and the
/// masked 8-bit result can both be read off directly.
fn add8(a: u8, b: u8, carry_in: u8) -> (u8, bool, bool) {
    let sum = a as u16 + b as u16 + carry_in as u16;
    let ac = (a & 0x0F) + (b & 0x0F) + carry_in > 0x0F;
    (sum as u8, sum > 0xFF, ac)
}

/// `SUB`/`SBB` semantics: CY=1 means borrow occurred (`a < b + borrow`).
fn sub8(a: u8, b: u8, borrow_in: u8) -> (u8, bool, bool) {
    let cy = (a as i16) < (b as i16 + borrow_in as i16);
    let ac = (a & 0x0F) < (b & 0x0F) + borrow_in;
    let result = a.wrapping_sub(b).wrapping_sub(borrow_in);
    (result, cy, ac)
}

/// ALU dispatch shared by `alu_reg`/`alu_imm`.  Returns
/// `(result, new_cy, new_ac)`; `CMP` computes `result` for flags only
/// (the caller must not write it back to A).
fn alu_op(op: u8, a: u8, b: u8, cy_in: bool) -> (u8, bool, bool) {
    match op {
        ALU_ADD => add8(a, b, 0),
        ALU_ADC => add8(a, b, cy_in as u8),
        ALU_SUB => sub8(a, b, 0),
        ALU_SBB => sub8(a, b, cy_in as u8),
        ALU_ANA => {
            // Per the Intel 8080 System Reference Manual, ANA/ANI set AC to
            // the OR of bit 3 of the two operands (a documented quirk, not
            // "carry out of bit 3" like ADD).  `(a & b) | b == b` always
            // (absorption law), but we spell it out exactly as the Python
            // original computes it (`compute_ac_ana(result | b, 0)` where
            // `result = a & b`) rather than relying on that simplification.
            let result = a & b;
            let ac = (result | b) & 0x08 != 0;
            (result, false, ac)
        }
        ALU_XRA => (a ^ b, false, false),
        ALU_ORA => (a | b, false, false),
        _ => sub8(a, b, 0), // ALU_CMP
    }
}

// ===========================================================================
// Stack helpers
// ===========================================================================

fn push16(mem: &mut Memory, sp: &mut u16, value: u16) {
    *sp = sp.wrapping_sub(2);
    mem.write_byte(*sp as usize, (value & 0xFF) as u8);
    mem.write_byte(sp.wrapping_add(1) as usize, (value >> 8) as u8);
}

fn pop16(mem: &Memory, sp: &mut u16) -> u16 {
    let lo = mem.read_byte(*sp as usize);
    let hi = mem.read_byte(sp.wrapping_add(1) as usize);
    *sp = sp.wrapping_add(2);
    ((hi as u16) << 8) | lo as u16
}

fn condition_met(cond: u8, f: &Flags) -> bool {
    match cond {
        COND_NZ => !f.z,
        COND_Z => f.z,
        COND_NC => !f.cy,
        COND_C => f.cy,
        COND_PO => !f.p,
        COND_PE => f.p,
        COND_P => !f.s,
        _ => f.s, // COND_M
    }
}

// ===========================================================================
// Dispatch
// ===========================================================================

/// Execute one decoded instruction.  `input_ports`/`output_ports` are
/// 256-entry port arrays; `inte` is the INTE (interrupt-enable) flip-flop.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
pub fn execute(
    decoded: &DecodeResult,
    regs: &mut Registers,
    flags: &mut Flags,
    mem: &mut Memory,
    input_ports: &[u8; 256],
    output_ports: &mut [u8; 256],
    inte: &mut bool,
    pc: u16,
) -> ExecuteResult {
    let fallthrough = ExecuteResult { next_pc: pc, halted: false };

    match decoded.mnemonic.as_str() {
        "hlt" => ExecuteResult { next_pc: pc, halted: true },
        "nop" => fallthrough,

        "mov" => {
            let dst = get(decoded, "dst") as u8;
            let src = get(decoded, "src") as u8;
            let v = regs.read(src, mem);
            regs.write(dst, v, mem);
            fallthrough
        }

        "mvi" => {
            let dst = get(decoded, "dst") as u8;
            let imm = get(decoded, "imm") as u8;
            regs.write(dst, imm, mem);
            fallthrough
        }

        "lxi" => {
            let pair = get(decoded, "pair") as u8;
            let imm = get(decoded, "imm") as u16;
            regs.write_pair(pair, imm);
            fallthrough
        }

        "inx" => {
            let pair = get(decoded, "pair") as u8;
            let v = regs.read_pair(pair).wrapping_add(1);
            regs.write_pair(pair, v);
            fallthrough
        }
        "dcx" => {
            let pair = get(decoded, "pair") as u8;
            let v = regs.read_pair(pair).wrapping_sub(1);
            regs.write_pair(pair, v);
            fallthrough
        }
        "dad" => {
            let pair = get(decoded, "pair") as u8;
            let hl = regs.hl() as u32;
            let rp = regs.read_pair(pair) as u32;
            let sum = hl + rp;
            regs.set_hl(sum as u16);
            flags.cy = sum > 0xFFFF;
            fallthrough
        }

        "inr" => {
            let dst = get(decoded, "dst") as u8;
            let old = regs.read(dst, mem);
            let (result, _, ac) = add8(old, 1, 0);
            regs.write(dst, result, mem);
            flags.s = s_flag(result);
            flags.z = z_flag(result);
            flags.p = p_flag(result);
            flags.ac = ac;
            fallthrough
        }
        "dcr" => {
            let dst = get(decoded, "dst") as u8;
            let old = regs.read(dst, mem);
            let (result, _, ac) = sub8(old, 1, 0);
            regs.write(dst, result, mem);
            flags.s = s_flag(result);
            flags.z = z_flag(result);
            flags.p = p_flag(result);
            flags.ac = ac;
            fallthrough
        }

        "stax" => {
            let pair = get(decoded, "pair") as u8;
            let addr = if pair == PAIR_D { regs.de() } else { regs.bc() };
            mem.write_byte(addr as usize, regs.a);
            fallthrough
        }
        "ldax" => {
            let pair = get(decoded, "pair") as u8;
            let addr = if pair == PAIR_D { regs.de() } else { regs.bc() };
            regs.a = mem.read_byte(addr as usize);
            fallthrough
        }
        "shld" => {
            let addr = get(decoded, "addr") as u16;
            mem.write_byte(addr as usize, regs.l);
            mem.write_byte(addr.wrapping_add(1) as usize, regs.h);
            fallthrough
        }
        "lhld" => {
            let addr = get(decoded, "addr") as u16;
            regs.l = mem.read_byte(addr as usize);
            regs.h = mem.read_byte(addr.wrapping_add(1) as usize);
            fallthrough
        }
        "sta" => {
            let addr = get(decoded, "addr") as u16;
            mem.write_byte(addr as usize, regs.a);
            fallthrough
        }
        "lda" => {
            let addr = get(decoded, "addr") as u16;
            regs.a = mem.read_byte(addr as usize);
            fallthrough
        }
        "xchg" => {
            std::mem::swap(&mut regs.h, &mut regs.d);
            std::mem::swap(&mut regs.l, &mut regs.e);
            fallthrough
        }

        "rlc" => {
            let bit7 = (regs.a >> 7) & 1;
            regs.a = (regs.a << 1) | bit7;
            flags.cy = bit7 == 1;
            fallthrough
        }
        "rrc" => {
            let bit0 = regs.a & 1;
            regs.a = (bit0 << 7) | (regs.a >> 1);
            flags.cy = bit0 == 1;
            fallthrough
        }
        "ral" => {
            let cy_in = flags.cy as u8;
            let new_cy = (regs.a >> 7) & 1 == 1;
            regs.a = (regs.a << 1) | cy_in;
            flags.cy = new_cy;
            fallthrough
        }
        "rar" => {
            let cy_in = flags.cy as u8;
            let new_cy = regs.a & 1 == 1;
            regs.a = (cy_in << 7) | (regs.a >> 1);
            flags.cy = new_cy;
            fallthrough
        }
        "cma" => {
            regs.a = !regs.a;
            fallthrough
        }
        "stc" => {
            flags.cy = true;
            fallthrough
        }
        "cmc" => {
            flags.cy = !flags.cy;
            fallthrough
        }
        "daa" => {
            let a = regs.a;
            let mut correction: u8 = 0;
            let mut new_cy = flags.cy;
            if (a & 0x0F) > 9 || flags.ac {
                correction |= 0x06;
            }
            if ((a as u16 + correction as u16) >> 4) > 9 || flags.cy {
                correction |= 0x60;
                new_cy = true;
            }
            let (result, _, ac) = add8(a, correction, 0);
            regs.a = result;
            flags.ac = ac;
            flags.cy = new_cy;
            flags.s = s_flag(result);
            flags.z = z_flag(result);
            flags.p = p_flag(result);
            fallthrough
        }

        "alu_reg" => {
            let op = get(decoded, "op") as u8;
            let src = get(decoded, "src") as u8;
            let operand = regs.read(src, mem);
            let (result, cy, ac) = alu_op(op, regs.a, operand, flags.cy);
            if op != ALU_CMP {
                regs.a = result;
            }
            flags.s = s_flag(result);
            flags.z = z_flag(result);
            flags.p = p_flag(result);
            flags.cy = cy;
            flags.ac = ac;
            fallthrough
        }
        "alu_imm" => {
            let op = get(decoded, "op") as u8;
            let imm = get(decoded, "imm") as u8;
            let (result, cy, ac) = alu_op(op, regs.a, imm, flags.cy);
            if op != ALU_CMP {
                regs.a = result;
            }
            flags.s = s_flag(result);
            flags.z = z_flag(result);
            flags.p = p_flag(result);
            flags.cy = cy;
            flags.ac = ac;
            fallthrough
        }

        "jmp" => ExecuteResult { next_pc: get(decoded, "addr") as u16, halted: false },
        "jcond" => {
            let cond = get(decoded, "cond") as u8;
            let addr = get(decoded, "addr") as u16;
            let next_pc = if condition_met(cond, flags) { addr } else { pc };
            ExecuteResult { next_pc, halted: false }
        }
        "call" => {
            let addr = get(decoded, "addr") as u16;
            push16(mem, &mut regs.sp, pc);
            ExecuteResult { next_pc: addr, halted: false }
        }
        "ccond" => {
            let cond = get(decoded, "cond") as u8;
            let addr = get(decoded, "addr") as u16;
            if condition_met(cond, flags) {
                push16(mem, &mut regs.sp, pc);
                ExecuteResult { next_pc: addr, halted: false }
            } else {
                fallthrough
            }
        }
        "ret" => ExecuteResult { next_pc: pop16(mem, &mut regs.sp), halted: false },
        "rcond" => {
            let cond = get(decoded, "cond") as u8;
            if condition_met(cond, flags) {
                ExecuteResult { next_pc: pop16(mem, &mut regs.sp), halted: false }
            } else {
                fallthrough
            }
        }
        "rst" => {
            let n = get(decoded, "n") as u16;
            push16(mem, &mut regs.sp, pc);
            ExecuteResult { next_pc: n * 8, halted: false }
        }
        "pchl" => ExecuteResult { next_pc: regs.hl(), halted: false },

        "push" => {
            let pair = get(decoded, "pair") as u8;
            let value = if pair == PAIR_SP {
                ((regs.a as u16) << 8) | flags.to_byte() as u16
            } else {
                regs.read_pair(pair)
            };
            push16(mem, &mut regs.sp, value);
            fallthrough
        }
        "pop" => {
            let pair = get(decoded, "pair") as u8;
            let value = pop16(mem, &mut regs.sp);
            if pair == PAIR_SP {
                regs.a = (value >> 8) as u8;
                *flags = Flags::from_byte(value as u8);
            } else {
                regs.write_pair(pair, value);
            }
            fallthrough
        }
        "xthl" => {
            let lo = mem.read_byte(regs.sp as usize);
            let hi = mem.read_byte(regs.sp.wrapping_add(1) as usize);
            mem.write_byte(regs.sp as usize, regs.l);
            mem.write_byte(regs.sp.wrapping_add(1) as usize, regs.h);
            regs.l = lo;
            regs.h = hi;
            fallthrough
        }
        "sphl" => {
            regs.sp = regs.hl();
            fallthrough
        }

        "in" => {
            let port = get(decoded, "port") as usize;
            regs.a = input_ports[port];
            fallthrough
        }
        "out" => {
            let port = get(decoded, "port") as usize;
            output_ports[port] = regs.a;
            fallthrough
        }
        "ei" => {
            *inte = true;
            fallthrough
        }
        "di" => {
            *inte = false;
            fallthrough
        }

        // Undefined opcode — fail closed: halt rather than silently
        // executing garbage or panicking (no exception channel through
        // `step() -> String`, matching the fail-closed convention
        // `mips_r2000_simulator::execute` uses for signed-overflow /
        // divide-by-zero).
        _ => ExecuteResult { next_pc: pc, halted: true },
    }
}
