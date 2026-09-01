//! Instruction executor for the Zilog Z80 ISA.
//!
//! Mutates CPU registers/flags/memory/ports according to a
//! [`DecodeResult`].  `pc` is the address **immediately following** the
//! fully-fetched instruction (the default fallthrough target for
//! non-control-flow ops); control-flow ops (`jp`/`jp_cond`/`call`/
//! `call_cond`/`ret`/`ret_cond`/`rst`/`jp_hl`/`jr`/`jr_cond`/`djnz`)
//! override `next_pc` explicitly — same convention
//! `intel8080_simulator::execute` uses.
//!
//! # Flags differ from the 8080
//!
//! The Z80 F register carries an extra **N** (add/subtract) flag the
//! 8080 has no equivalent for, and its **P/V** bit is genuinely
//! dual-purpose: logical ops (`AND`/`XOR`/`OR`) set it to parity, while
//! arithmetic ops (`ADD`/`ADC`/`SUB`/`SBC`/`CP`/`INC`/`DEC`) set it to
//! signed overflow.  This is a direct transliteration of
//! `z80_simulator.simulator.Z80Simulator._alu8` /
//! `z80_simulator.flags.compute_overflow_add` /
//! `compute_overflow_sub` — see the module docs on `crate::opcodes` for
//! the byte-identity claims this module's *encoding* half makes; this
//! module's *flag semantics* are Z80-specific from the ALU group onward
//! even where the *bytes* are shared with 8080.
//!
//! # Undefined opcodes fail closed
//!
//! The checked simulator lifecycle rejects genuinely undefined opcodes
//! atomically with a typed error before this executor is invoked.

use cpu_simulator::Memory;

use crate::decode::DecodeResult;
use crate::opcodes::*;

// ===========================================================================
// Registers
// ===========================================================================

/// The Z80's main + alternate register banks, index registers, stack
/// pointer, and the I/R special registers.  PC lives on
/// [`crate::simulator::Z80Simulator`] directly, mirroring
/// `intel8080_simulator::execute::Registers`.
///
/// The alternate bank (`a2`/`f2`/`b2`/…) is stored as **raw byte values**
/// (not unpacked flags) since it is entirely opaque to every instruction
/// except `EX AF,AF'`/`EXX`, which swap it wholesale with the live bank —
/// exactly how `Z80State`'s Python original models it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Registers {
    // ── Main bank ──
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    // ── Alternate bank ──
    pub a2: u8,
    pub f2: u8,
    pub b2: u8,
    pub c2: u8,
    pub d2: u8,
    pub e2: u8,
    pub h2: u8,
    pub l2: u8,
    // ── Index / special registers ──
    pub ix: u16,
    pub iy: u16,
    pub sp: u16,
    pub i: u8,
    pub r: u8,
}

impl Default for Registers {
    fn default() -> Self {
        Self {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            a2: 0,
            f2: 0xFF,
            b2: 0,
            c2: 0,
            d2: 0,
            e2: 0,
            h2: 0,
            l2: 0,
            ix: 0,
            iy: 0,
            sp: 0,
            i: 0,
            r: 0,
        }
    }
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

    /// Read a register (or memory[HL] for the `(HL)` pseudo-register,
    /// code 6) by its 3-bit code.
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

    /// Write a register (or memory[HL] for `(HL)`) by its 3-bit code.
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

    /// Read a register pair for `PUSH` (0=BC, 1=DE, 2=HL, 3=AF — unlike
    /// [`Self::read_pair`], code 3 always means AF here, never SP).
    fn read_pair_af(&self, code: u8, flags: &Flags) -> u16 {
        if code == PAIR_AF {
            ((self.a as u16) << 8) | flags.to_byte() as u16
        } else {
            self.read_pair(code)
        }
    }

    /// Write a register pair for `POP` (0=BC, 1=DE, 2=HL, 3=AF).
    fn write_pair_af(&mut self, code: u8, value: u16, flags: &mut Flags) {
        if code == PAIR_AF {
            self.a = (value >> 8) as u8;
            *flags = Flags::from_byte(value as u8);
        } else {
            self.write_pair(code, value);
        }
    }
}

// ===========================================================================
// Flags
// ===========================================================================

/// The six named Z80 condition flags (S, Z, H, P/V, N, C).  The two
/// undocumented bits (Y = bit 5, X = bit 3) are always packed/unpacked as
/// 0 — same simplification `z80_simulator.flags.pack_f`/`unpack_f` makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flags {
    pub s: bool,
    pub z: bool,
    pub h: bool,
    pub pv: bool,
    pub n: bool,
    pub c: bool,
}

impl Default for Flags {
    fn default() -> Self {
        Self {
            s: true,
            z: true,
            h: true,
            pv: true,
            n: true,
            c: true,
        }
    }
}

impl Flags {
    /// Pack into the Z80's F register byte (`S Z 0 H 0 PV N C`).
    pub fn to_byte(self) -> u8 {
        ((self.s as u8) << 7)
            | ((self.z as u8) << 6)
            | ((self.h as u8) << 4)
            | ((self.pv as u8) << 2)
            | ((self.n as u8) << 1)
            | (self.c as u8)
    }

    /// Unpack from an F register byte.
    pub fn from_byte(b: u8) -> Self {
        Flags {
            s: b & 0x80 != 0,
            z: b & 0x40 != 0,
            h: b & 0x10 != 0,
            pv: b & 0x04 != 0,
            n: b & 0x02 != 0,
            c: b & 0x01 != 0,
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
// Flag-computation helpers (direct port of z80_simulator.flags)
// ===========================================================================

fn s_flag(r: u8) -> bool {
    r & 0x80 != 0
}
fn z_flag(r: u8) -> bool {
    r == 0
}
fn parity(r: u8) -> bool {
    r.count_ones().is_multiple_of(2)
}

fn half_carry_add(a: u8, b: u8, carry: u8) -> bool {
    ((a & 0x0F) + (b & 0x0F) + carry) > 0x0F
}
fn half_carry_sub(a: u8, b: u8, borrow: u8) -> bool {
    (a & 0x0F) < (b & 0x0F) + borrow
}

/// V (overflow) for addition: same-sign inputs produced a different-sign
/// output.  `compute_overflow_add` in the Python original.
fn overflow_add(a: u8, b: u8, result: u8) -> bool {
    let a7 = (a >> 7) & 1;
    let b7 = (b >> 7) & 1;
    let r7 = (result >> 7) & 1;
    ((!(a7 ^ b7)) & (a7 ^ r7) & 1) != 0
}

/// V (overflow) for subtraction — SBC/SUB compute `A + (~B) + carry`
/// internally, so overflow reuses the ADC formula with an inverted
/// operand.  `compute_overflow_sub` in the Python original.
fn overflow_sub(a: u8, b: u8, result: u8) -> bool {
    overflow_add(a, !b, result)
}

/// Widened add: returns `(masked_result, carry_out)`.
fn add8(a: u8, b: u8, carry_in: u8) -> (u8, bool) {
    let sum = a as u16 + b as u16 + carry_in as u16;
    (sum as u8, sum > 0xFF)
}

/// Widened subtract: returns `(masked_result, borrow_out)`.
fn sub8(a: u8, b: u8, borrow_in: u8) -> (u8, bool) {
    let diff = a as i16 - b as i16 - borrow_in as i16;
    (diff as u8, diff < 0)
}

/// 8-bit ALU dispatch shared by `alu_reg`/`alu_imm` — a direct port of
/// `Z80Simulator._alu8`.  Mutates `flags` in place; returns the value to
/// write to A (the caller skips the write for `CP`).
fn alu8(op: u8, a: u8, m: u8, flags: &mut Flags) -> u8 {
    match op {
        ALU_ADD => {
            let (r, cy) = add8(a, m, 0);
            flags.h = half_carry_add(a, m, 0);
            flags.pv = overflow_add(a, m, r);
            flags.n = false;
            flags.c = cy;
            flags.s = s_flag(r);
            flags.z = z_flag(r);
            r
        }
        ALU_ADC => {
            let c = flags.c as u8;
            let (r, cy) = add8(a, m, c);
            flags.h = half_carry_add(a, m, c);
            flags.pv = overflow_add(a, m, r);
            flags.n = false;
            flags.c = cy;
            flags.s = s_flag(r);
            flags.z = z_flag(r);
            r
        }
        ALU_SUB => {
            let (r, cy) = sub8(a, m, 0);
            flags.h = half_carry_sub(a, m, 0);
            flags.pv = overflow_sub(a, m, r);
            flags.n = true;
            flags.c = cy;
            flags.s = s_flag(r);
            flags.z = z_flag(r);
            r
        }
        ALU_SBC => {
            let borrow = flags.c as u8;
            let (r, cy) = sub8(a, m, borrow);
            flags.h = half_carry_sub(a, m, borrow);
            flags.pv = overflow_sub(a, m, r);
            flags.n = true;
            flags.c = cy;
            flags.s = s_flag(r);
            flags.z = z_flag(r);
            r
        }
        ALU_AND => {
            let r = a & m;
            flags.h = true;
            flags.n = false;
            flags.c = false;
            flags.s = s_flag(r);
            flags.z = z_flag(r);
            flags.pv = parity(r);
            r
        }
        ALU_XOR => {
            let r = a ^ m;
            flags.h = false;
            flags.n = false;
            flags.c = false;
            flags.s = s_flag(r);
            flags.z = z_flag(r);
            flags.pv = parity(r);
            r
        }
        ALU_OR => {
            let r = a | m;
            flags.h = false;
            flags.n = false;
            flags.c = false;
            flags.s = s_flag(r);
            flags.z = z_flag(r);
            flags.pv = parity(r);
            r
        }
        _ => {
            // ALU_CP — compare: like SUB but the caller does not write A.
            let (r, cy) = sub8(a, m, 0);
            flags.h = half_carry_sub(a, m, 0);
            flags.pv = overflow_sub(a, m, r);
            flags.n = true;
            flags.c = cy;
            flags.s = s_flag(r);
            flags.z = z_flag(r);
            r
        }
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
        COND_NC => !f.c,
        COND_C => f.c,
        COND_PO => !f.pv,
        COND_PE => f.pv,
        COND_P => !f.s,
        _ => f.s, // COND_M
    }
}

fn read16(mem: &Memory, address: u16) -> u16 {
    let lo = mem.read_byte(address as usize) as u16;
    let hi = mem.read_byte(address.wrapping_add(1) as usize) as u16;
    (hi << 8) | lo
}

fn write16(mem: &mut Memory, address: u16, value: u16) {
    mem.write_byte(address as usize, value as u8);
    mem.write_byte(address.wrapping_add(1) as usize, (value >> 8) as u8);
}

fn advance(value: u16, decrement: bool) -> u16 {
    if decrement {
        value.wrapping_sub(1)
    } else {
        value.wrapping_add(1)
    }
}

fn block_ld(regs: &mut Registers, flags: &mut Flags, mem: &mut Memory, decrement: bool) {
    let source = regs.hl();
    let destination = regs.de();
    mem.write_byte(destination as usize, mem.read_byte(source as usize));
    regs.set_hl(advance(source, decrement));
    let de = advance(destination, decrement);
    regs.d = (de >> 8) as u8;
    regs.e = de as u8;
    let bc = regs.bc().wrapping_sub(1);
    regs.b = (bc >> 8) as u8;
    regs.c = bc as u8;
    flags.h = false;
    flags.n = false;
    flags.pv = bc != 0;
}

fn block_cp(regs: &mut Registers, flags: &mut Flags, mem: &Memory, decrement: bool) {
    let address = regs.hl();
    let operand = mem.read_byte(address as usize);
    let result = regs.a.wrapping_sub(operand);
    regs.set_hl(advance(address, decrement));
    let bc = regs.bc().wrapping_sub(1);
    regs.b = (bc >> 8) as u8;
    regs.c = bc as u8;
    flags.h = half_carry_sub(regs.a, operand, 0);
    flags.n = true;
    flags.pv = bc != 0;
    flags.s = s_flag(result);
    flags.z = z_flag(result);
}

fn block_in(
    regs: &mut Registers,
    flags: &mut Flags,
    mem: &mut Memory,
    input_ports: &[u8; 256],
    decrement: bool,
) {
    let address = regs.hl();
    mem.write_byte(address as usize, input_ports[regs.c as usize]);
    regs.set_hl(advance(address, decrement));
    regs.b = regs.b.wrapping_sub(1);
    flags.n = true;
    flags.z = regs.b == 0;
}

fn block_out(
    regs: &mut Registers,
    flags: &mut Flags,
    mem: &Memory,
    output_ports: &mut [u8; 256],
    decrement: bool,
) {
    let address = regs.hl();
    output_ports[regs.c as usize] = mem.read_byte(address as usize);
    regs.set_hl(advance(address, decrement));
    regs.b = regs.b.wrapping_sub(1);
    flags.n = true;
    flags.z = regs.b == 0;
}

fn index_value(regs: &Registers, use_ix: bool) -> u16 {
    if use_ix {
        regs.ix
    } else {
        regs.iy
    }
}

fn set_index_value(regs: &mut Registers, use_ix: bool, value: u16) {
    if use_ix {
        regs.ix = value;
    } else {
        regs.iy = value;
    }
}

fn indexed_address(regs: &Registers, use_ix: bool, displacement: i32) -> u16 {
    index_value(regs, use_ix).wrapping_add(displacement as i16 as u16)
}

fn rotate_shift(value: u8, operation: u8, carry: bool) -> (u8, bool) {
    match operation {
        0 => {
            let outgoing = (value >> 7) & 1;
            ((value << 1) | outgoing, outgoing != 0)
        }
        1 => {
            let outgoing = value & 1;
            ((outgoing << 7) | (value >> 1), outgoing != 0)
        }
        2 => {
            let outgoing = (value >> 7) & 1;
            ((value << 1) | carry as u8, outgoing != 0)
        }
        3 => {
            let outgoing = value & 1;
            (((carry as u8) << 7) | (value >> 1), outgoing != 0)
        }
        4 => (value << 1, value & 0x80 != 0),
        5 => ((value & 0x80) | (value >> 1), value & 1 != 0),
        6 => ((value << 1) | 1, value & 0x80 != 0),
        _ => (value >> 1, value & 1 != 0),
    }
}

// ===========================================================================
// Dispatch
// ===========================================================================

/// Execute one decoded instruction.  `input_ports`/`output_ports` are
/// 256-entry port arrays; `iff1`/`iff2` are the interrupt-enable
/// flip-flops (`EI`/`DI` toggle both, matching the Python original).
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
pub fn execute(
    decoded: &DecodeResult,
    regs: &mut Registers,
    flags: &mut Flags,
    mem: &mut Memory,
    input_ports: &[u8; 256],
    output_ports: &mut [u8; 256],
    iff1: &mut bool,
    iff2: &mut bool,
    im: &mut u8,
    pc: u16,
) -> ExecuteResult {
    let fallthrough = ExecuteResult {
        next_pc: pc,
        halted: false,
    };

    match decoded.mnemonic.as_str() {
        "halt" => ExecuteResult {
            next_pc: pc,
            halted: true,
        },
        "nop" => fallthrough,

        // ── 8080-legacy data movement ──
        "ld_r_r" => {
            let dst = get(decoded, "dst") as u8;
            let src = get(decoded, "src") as u8;
            let v = regs.read(src, mem);
            regs.write(dst, v, mem);
            fallthrough
        }
        "ld_r_n" => {
            let dst = get(decoded, "dst") as u8;
            let imm = get(decoded, "imm") as u8;
            regs.write(dst, imm, mem);
            fallthrough
        }
        "ld_rp_nn" => {
            let pair = get(decoded, "pair") as u8;
            let imm = get(decoded, "imm") as u16;
            regs.write_pair(pair, imm);
            fallthrough
        }
        "inc_rp" => {
            let pair = get(decoded, "pair") as u8;
            let v = regs.read_pair(pair).wrapping_add(1);
            regs.write_pair(pair, v);
            fallthrough
        }
        "dec_rp" => {
            let pair = get(decoded, "pair") as u8;
            let v = regs.read_pair(pair).wrapping_sub(1);
            regs.write_pair(pair, v);
            fallthrough
        }
        "add_hl_rp" => {
            let pair = get(decoded, "pair") as u8;
            let hl = regs.hl() as u32;
            let rp = regs.read_pair(pair) as u32;
            let sum = hl + rp;
            regs.set_hl(sum as u16);
            flags.c = sum > 0xFFFF;
            flags.h = ((hl & 0x0FFF) + (rp & 0x0FFF)) > 0x0FFF;
            flags.n = false;
            fallthrough
        }

        "inc_r" => {
            let r_code = get(decoded, "dst") as u8;
            let v = regs.read(r_code, mem);
            let r = v.wrapping_add(1);
            regs.write(r_code, r, mem);
            flags.h = half_carry_add(v, 1, 0);
            flags.pv = v == 0x7F;
            flags.n = false;
            flags.s = s_flag(r);
            flags.z = z_flag(r);
            fallthrough
        }
        "dec_r" => {
            let r_code = get(decoded, "dst") as u8;
            let v = regs.read(r_code, mem);
            let r = v.wrapping_sub(1);
            regs.write(r_code, r, mem);
            flags.h = half_carry_sub(v, 1, 0);
            flags.pv = v == 0x80;
            flags.n = true;
            flags.s = s_flag(r);
            flags.z = z_flag(r);
            fallthrough
        }

        "ld_rp_a" => {
            let pair = get(decoded, "pair") as u8;
            let addr = if pair == PAIR_DE {
                regs.de()
            } else {
                regs.bc()
            };
            mem.write_byte(addr as usize, regs.a);
            fallthrough
        }
        "ld_a_rp" => {
            let pair = get(decoded, "pair") as u8;
            let addr = if pair == PAIR_DE {
                regs.de()
            } else {
                regs.bc()
            };
            regs.a = mem.read_byte(addr as usize);
            fallthrough
        }
        "ld_nn_hl" => {
            let addr = get(decoded, "addr") as u16;
            mem.write_byte(addr as usize, regs.l);
            mem.write_byte(addr.wrapping_add(1) as usize, regs.h);
            fallthrough
        }
        "ld_hl_nn" => {
            let addr = get(decoded, "addr") as u16;
            regs.l = mem.read_byte(addr as usize);
            regs.h = mem.read_byte(addr.wrapping_add(1) as usize);
            fallthrough
        }
        "ld_nn_a" => {
            let addr = get(decoded, "addr") as u16;
            mem.write_byte(addr as usize, regs.a);
            fallthrough
        }
        "ld_a_nn" => {
            let addr = get(decoded, "addr") as u16;
            regs.a = mem.read_byte(addr as usize);
            fallthrough
        }
        "ex_de_hl" => {
            std::mem::swap(&mut regs.h, &mut regs.d);
            std::mem::swap(&mut regs.l, &mut regs.e);
            fallthrough
        }

        "rlca" => {
            let bit7 = (regs.a >> 7) & 1;
            regs.a = (regs.a << 1) | bit7;
            flags.c = bit7 == 1;
            flags.h = false;
            flags.n = false;
            fallthrough
        }
        "rrca" => {
            let bit0 = regs.a & 1;
            regs.a = (bit0 << 7) | (regs.a >> 1);
            flags.c = bit0 == 1;
            flags.h = false;
            flags.n = false;
            fallthrough
        }
        "rla" => {
            let cy_in = flags.c as u8;
            let new_cy = (regs.a >> 7) & 1 == 1;
            regs.a = (regs.a << 1) | cy_in;
            flags.c = new_cy;
            flags.h = false;
            flags.n = false;
            fallthrough
        }
        "rra" => {
            let cy_in = flags.c as u8;
            let new_cy = regs.a & 1 == 1;
            regs.a = (cy_in << 7) | (regs.a >> 1);
            flags.c = new_cy;
            flags.h = false;
            flags.n = false;
            fallthrough
        }
        "cpl" => {
            regs.a = !regs.a;
            flags.h = true;
            flags.n = true;
            fallthrough
        }
        "scf" => {
            flags.c = true;
            flags.h = false;
            flags.n = false;
            fallthrough
        }
        "ccf" => {
            flags.h = flags.c;
            flags.c = !flags.c;
            flags.n = false;
            fallthrough
        }
        "daa" => {
            let (new_a, new_h, new_pv, new_c) = daa(regs.a, flags.n, flags.h, flags.c);
            regs.a = new_a;
            flags.h = new_h;
            flags.pv = new_pv;
            flags.c = new_c;
            flags.s = s_flag(new_a);
            flags.z = z_flag(new_a);
            fallthrough
        }

        "alu_reg" => {
            let op = get(decoded, "op") as u8;
            let src = get(decoded, "src") as u8;
            let operand = regs.read(src, mem);
            let r = alu8(op, regs.a, operand, flags);
            if op != ALU_CP {
                regs.a = r;
            }
            fallthrough
        }
        "alu_imm" => {
            let op = get(decoded, "op") as u8;
            let imm = get(decoded, "imm") as u8;
            let r = alu8(op, regs.a, imm, flags);
            if op != ALU_CP {
                regs.a = r;
            }
            fallthrough
        }

        "jp" => ExecuteResult {
            next_pc: get(decoded, "addr") as u16,
            halted: false,
        },
        "jp_cond" => {
            let cond = get(decoded, "cond") as u8;
            let addr = get(decoded, "addr") as u16;
            let next_pc = if condition_met(cond, flags) { addr } else { pc };
            ExecuteResult {
                next_pc,
                halted: false,
            }
        }
        "jp_hl" => ExecuteResult {
            next_pc: regs.hl(),
            halted: false,
        },
        "call" => {
            let addr = get(decoded, "addr") as u16;
            push16(mem, &mut regs.sp, pc);
            ExecuteResult {
                next_pc: addr,
                halted: false,
            }
        }
        "call_cond" => {
            let cond = get(decoded, "cond") as u8;
            let addr = get(decoded, "addr") as u16;
            if condition_met(cond, flags) {
                push16(mem, &mut regs.sp, pc);
                ExecuteResult {
                    next_pc: addr,
                    halted: false,
                }
            } else {
                fallthrough
            }
        }
        "ret" => ExecuteResult {
            next_pc: pop16(mem, &mut regs.sp),
            halted: false,
        },
        "ret_cond" => {
            let cond = get(decoded, "cond") as u8;
            if condition_met(cond, flags) {
                ExecuteResult {
                    next_pc: pop16(mem, &mut regs.sp),
                    halted: false,
                }
            } else {
                fallthrough
            }
        }
        "rst" => {
            let n = get(decoded, "n") as u16;
            push16(mem, &mut regs.sp, pc);
            ExecuteResult {
                next_pc: n * 8,
                halted: false,
            }
        }

        "push" => {
            let pair = get(decoded, "pair") as u8;
            let value = regs.read_pair_af(pair, flags);
            push16(mem, &mut regs.sp, value);
            fallthrough
        }
        "pop" => {
            let pair = get(decoded, "pair") as u8;
            let value = pop16(mem, &mut regs.sp);
            regs.write_pair_af(pair, value, flags);
            fallthrough
        }
        "ex_sp_hl" => {
            let lo = mem.read_byte(regs.sp as usize);
            let hi = mem.read_byte(regs.sp.wrapping_add(1) as usize);
            mem.write_byte(regs.sp as usize, regs.l);
            mem.write_byte(regs.sp.wrapping_add(1) as usize, regs.h);
            regs.l = lo;
            regs.h = hi;
            fallthrough
        }
        "ld_sp_hl" => {
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
            *iff1 = true;
            *iff2 = true;
            fallthrough
        }
        "di" => {
            *iff1 = false;
            *iff2 = false;
            fallthrough
        }

        // ── Z80-only: alternate register bank ──
        "ex_af_af" => {
            std::mem::swap(&mut regs.a, &mut regs.a2);
            let f_cur = flags.to_byte();
            let f2 = Flags::from_byte(regs.f2);
            regs.f2 = f_cur;
            *flags = f2;
            fallthrough
        }
        "exx" => {
            std::mem::swap(&mut regs.b, &mut regs.b2);
            std::mem::swap(&mut regs.c, &mut regs.c2);
            std::mem::swap(&mut regs.d, &mut regs.d2);
            std::mem::swap(&mut regs.e, &mut regs.e2);
            std::mem::swap(&mut regs.h, &mut regs.h2);
            std::mem::swap(&mut regs.l, &mut regs.l2);
            fallthrough
        }

        // ── Z80-only: relative jumps ──
        "jr" => {
            let e = get(decoded, "e");
            ExecuteResult {
                next_pc: pc.wrapping_add(e as i16 as u16),
                halted: false,
            }
        }
        "jr_cond" => {
            let cond = get(decoded, "cond") as u8;
            let e = get(decoded, "e");
            let next_pc = if condition_met(cond, flags) {
                pc.wrapping_add(e as i16 as u16)
            } else {
                pc
            };
            ExecuteResult {
                next_pc,
                halted: false,
            }
        }
        "djnz" => {
            let e = get(decoded, "e");
            regs.b = regs.b.wrapping_sub(1);
            let next_pc = if regs.b != 0 {
                pc.wrapping_add(e as i16 as u16)
            } else {
                pc
            };
            ExecuteResult {
                next_pc,
                halted: false,
            }
        }

        // ── ED-prefixed extended instructions ──
        "ed_ld_a_i" | "ed_ld_a_r" => {
            regs.a = if decoded.mnemonic == "ed_ld_a_i" {
                regs.i
            } else {
                regs.r
            };
            flags.s = s_flag(regs.a);
            flags.z = z_flag(regs.a);
            flags.h = false;
            flags.n = false;
            flags.pv = *iff2;
            fallthrough
        }
        "ed_ld_i_a" => {
            regs.i = regs.a;
            fallthrough
        }
        "ed_ld_r_a" => {
            regs.r = regs.a;
            fallthrough
        }
        "ed_ld_rp_mem" => {
            let pair = get(decoded, "pair") as u8;
            let address = get(decoded, "addr") as u16;
            regs.write_pair(pair, read16(mem, address));
            fallthrough
        }
        "ed_ld_mem_rp" => {
            let pair = get(decoded, "pair") as u8;
            let address = get(decoded, "addr") as u16;
            write16(mem, address, regs.read_pair(pair));
            fallthrough
        }
        "ed_adc_hl_rp" => {
            let hl = regs.hl();
            let operand = regs.read_pair(get(decoded, "pair") as u8);
            let carry = flags.c as u32;
            let total = u32::from(hl) + u32::from(operand) + carry;
            let result = total as u16;
            flags.c = total > 0xFFFF;
            flags.h = (u32::from(hl & 0x0FFF) + u32::from(operand & 0x0FFF) + carry) > 0x0FFF;
            flags.pv = overflow_add((hl >> 8) as u8, (operand >> 8) as u8, (result >> 8) as u8);
            flags.n = false;
            flags.s = result & 0x8000 != 0;
            flags.z = result == 0;
            regs.set_hl(result);
            fallthrough
        }
        "ed_sbc_hl_rp" => {
            let hl = regs.hl();
            let operand = regs.read_pair(get(decoded, "pair") as u8);
            let borrow = flags.c as i32;
            let total = i32::from(hl) - i32::from(operand) - borrow;
            let result = total as u16;
            flags.c = total < 0;
            flags.h = i32::from(hl & 0x0FFF) < i32::from(operand & 0x0FFF) + borrow;
            flags.pv = overflow_sub((hl >> 8) as u8, (operand >> 8) as u8, (result >> 8) as u8);
            flags.n = true;
            flags.s = result & 0x8000 != 0;
            flags.z = result == 0;
            regs.set_hl(result);
            fallthrough
        }
        "ed_neg" => {
            let operand = regs.a;
            regs.a = 0u8.wrapping_sub(operand);
            flags.c = operand != 0;
            flags.h = operand & 0x0F != 0;
            flags.pv = operand == 0x80;
            flags.n = true;
            flags.s = s_flag(regs.a);
            flags.z = z_flag(regs.a);
            fallthrough
        }
        "ed_im0" | "ed_im1" | "ed_im2" => {
            *im = match decoded.mnemonic.as_str() {
                "ed_im0" => 0,
                "ed_im1" => 1,
                _ => 2,
            };
            fallthrough
        }
        "ed_reti" | "ed_retn" => {
            *iff1 = *iff2;
            ExecuteResult {
                next_pc: pop16(mem, &mut regs.sp),
                halted: false,
            }
        }
        "ed_rld" => {
            let address = regs.hl();
            let operand = mem.read_byte(address as usize);
            mem.write_byte(address as usize, (operand << 4) | (regs.a & 0x0F));
            regs.a = (regs.a & 0xF0) | (operand >> 4);
            flags.h = false;
            flags.n = false;
            flags.s = s_flag(regs.a);
            flags.z = z_flag(regs.a);
            flags.pv = parity(regs.a);
            fallthrough
        }
        "ed_rrd" => {
            let address = regs.hl();
            let operand = mem.read_byte(address as usize);
            mem.write_byte(address as usize, ((regs.a & 0x0F) << 4) | (operand >> 4));
            regs.a = (regs.a & 0xF0) | (operand & 0x0F);
            flags.h = false;
            flags.n = false;
            flags.s = s_flag(regs.a);
            flags.z = z_flag(regs.a);
            flags.pv = parity(regs.a);
            fallthrough
        }
        "ed_in_r_c" => {
            let register = get(decoded, "reg") as u8;
            let value = input_ports[regs.c as usize];
            if register != REG_M {
                regs.write(register, value, mem);
            }
            flags.h = false;
            flags.n = false;
            flags.s = s_flag(value);
            flags.z = z_flag(value);
            flags.pv = parity(value);
            fallthrough
        }
        "ed_out_c_r" => {
            let register = get(decoded, "reg") as u8;
            output_ports[regs.c as usize] = if register == REG_M {
                0
            } else {
                regs.read(register, mem)
            };
            fallthrough
        }
        "ed_ldi" | "ed_ldd" | "ed_ldir" | "ed_lddr" => {
            let decrement = matches!(decoded.mnemonic.as_str(), "ed_ldd" | "ed_lddr");
            loop {
                block_ld(regs, flags, mem, decrement);
                if !matches!(decoded.mnemonic.as_str(), "ed_ldir" | "ed_lddr") || regs.bc() == 0 {
                    break;
                }
            }
            fallthrough
        }
        "ed_cpi" | "ed_cpd" | "ed_cpir" | "ed_cpdr" => {
            let decrement = matches!(decoded.mnemonic.as_str(), "ed_cpd" | "ed_cpdr");
            loop {
                block_cp(regs, flags, mem, decrement);
                if !matches!(decoded.mnemonic.as_str(), "ed_cpir" | "ed_cpdr")
                    || flags.z
                    || regs.bc() == 0
                {
                    break;
                }
            }
            fallthrough
        }
        "ed_ini" | "ed_ind" | "ed_inir" | "ed_indr" => {
            let decrement = matches!(decoded.mnemonic.as_str(), "ed_ind" | "ed_indr");
            loop {
                block_in(regs, flags, mem, input_ports, decrement);
                if !matches!(decoded.mnemonic.as_str(), "ed_inir" | "ed_indr") || regs.b == 0 {
                    break;
                }
            }
            fallthrough
        }
        "ed_outi" | "ed_outd" | "ed_otir" | "ed_otdr" => {
            let decrement = matches!(decoded.mnemonic.as_str(), "ed_outd" | "ed_otdr");
            loop {
                block_out(regs, flags, mem, output_ports, decrement);
                if !matches!(decoded.mnemonic.as_str(), "ed_otir" | "ed_otdr") || regs.b == 0 {
                    break;
                }
            }
            fallthrough
        }

        // ── Z80-only: CB-prefixed bit manipulation / extended rotate-shift ──
        "cb_rot" => {
            let rot_op = get(decoded, "op") as u8;
            let r_code = get(decoded, "reg") as u8;
            let v = regs.read(r_code, mem);
            // `v` is already `u8`, so a trailing `& 0xFF` on any of these
            // arms would be a true no-op (unlike, say, `-1i32 & 0xFFFF`
            // truncating a wider signed type) -- clippy::identity_op
            // correctly flags it, so the masking is simply omitted here;
            // `u8 << 1`/`u8 >> 1` already wrap within 8 bits by
            // construction.
            let (r, cy) = match rot_op {
                0 => {
                    // RLC
                    let c = (v >> 7) & 1;
                    ((v << 1) | c, c == 1)
                }
                1 => {
                    // RRC
                    let c = v & 1;
                    ((c << 7) | (v >> 1), c == 1)
                }
                2 => {
                    // RL
                    let c = (v >> 7) & 1;
                    ((v << 1) | (flags.c as u8), c == 1)
                }
                3 => {
                    // RR
                    let c = v & 1;
                    (((flags.c as u8) << 7) | (v >> 1), c == 1)
                }
                4 => {
                    // SLA
                    let c = (v >> 7) & 1;
                    (v << 1, c == 1)
                }
                5 => {
                    // SRA (arithmetic: bit 7 preserved)
                    let c = v & 1;
                    ((v & 0x80) | (v >> 1), c == 1)
                }
                6 => {
                    // SLL (undocumented: shifts in a 1)
                    let c = (v >> 7) & 1;
                    ((v << 1) | 1, c == 1)
                }
                _ => {
                    // SRL (logical: 0 shifted in)
                    let c = v & 1;
                    (v >> 1, c == 1)
                }
            };
            regs.write(r_code, r, mem);
            flags.c = cy;
            flags.h = false;
            flags.n = false;
            flags.s = s_flag(r);
            flags.z = z_flag(r);
            flags.pv = parity(r);
            fallthrough
        }
        "bit" => {
            let bit = get(decoded, "bit") as u8;
            let r_code = get(decoded, "reg") as u8;
            let v = regs.read(r_code, mem);
            flags.z = v & (1 << bit) == 0;
            flags.h = true;
            flags.n = false;
            fallthrough
        }
        "res" => {
            let bit = get(decoded, "bit") as u8;
            let r_code = get(decoded, "reg") as u8;
            let v = regs.read(r_code, mem);
            regs.write(r_code, v & !(1 << bit), mem);
            fallthrough
        }
        "set" => {
            let bit = get(decoded, "bit") as u8;
            let r_code = get(decoded, "reg") as u8;
            let v = regs.read(r_code, mem);
            regs.write(r_code, v | (1 << bit), mem);
            fallthrough
        }

        // ── DD/FD-prefixed IX/IY operations ──
        "index_ld_nn" => {
            set_index_value(regs, get(decoded, "index") != 0, get(decoded, "imm") as u16);
            fallthrough
        }
        "index_ld_from_mem" => {
            let value = read16(mem, get(decoded, "addr") as u16);
            set_index_value(regs, get(decoded, "index") != 0, value);
            fallthrough
        }
        "index_ld_to_mem" => {
            let use_ix = get(decoded, "index") != 0;
            write16(mem, get(decoded, "addr") as u16, index_value(regs, use_ix));
            fallthrough
        }
        "index_ld_mem_n" => {
            let use_ix = get(decoded, "index") != 0;
            let address = indexed_address(regs, use_ix, get(decoded, "d"));
            mem.write_byte(address as usize, get(decoded, "imm") as u8);
            fallthrough
        }
        "index_ld_r_mem" => {
            let use_ix = get(decoded, "index") != 0;
            let address = indexed_address(regs, use_ix, get(decoded, "d"));
            let value = mem.read_byte(address as usize);
            regs.write(get(decoded, "dst") as u8, value, mem);
            fallthrough
        }
        "index_ld_mem_r" => {
            let use_ix = get(decoded, "index") != 0;
            let address = indexed_address(regs, use_ix, get(decoded, "d"));
            let value = regs.read(get(decoded, "src") as u8, mem);
            mem.write_byte(address as usize, value);
            fallthrough
        }
        "index_ld_sp" => {
            regs.sp = index_value(regs, get(decoded, "index") != 0);
            fallthrough
        }
        "index_push" => {
            let value = index_value(regs, get(decoded, "index") != 0);
            push16(mem, &mut regs.sp, value);
            fallthrough
        }
        "index_pop" => {
            let value = pop16(mem, &mut regs.sp);
            set_index_value(regs, get(decoded, "index") != 0, value);
            fallthrough
        }
        "index_add_rp" => {
            let use_ix = get(decoded, "index") != 0;
            let current = index_value(regs, use_ix);
            let pair = get(decoded, "pair") as u8;
            let operand = if pair == PAIR_HL {
                current
            } else {
                regs.read_pair(pair)
            };
            let total = u32::from(current) + u32::from(operand);
            flags.c = total > 0xFFFF;
            flags.h = u32::from(current & 0x0FFF) + u32::from(operand & 0x0FFF) > 0x0FFF;
            flags.n = false;
            set_index_value(regs, use_ix, total as u16);
            fallthrough
        }
        "index_inc" | "index_dec" => {
            let use_ix = get(decoded, "index") != 0;
            let current = index_value(regs, use_ix);
            let value = if decoded.mnemonic == "index_inc" {
                current.wrapping_add(1)
            } else {
                current.wrapping_sub(1)
            };
            set_index_value(regs, use_ix, value);
            fallthrough
        }
        "index_inc_mem" | "index_dec_mem" => {
            let use_ix = get(decoded, "index") != 0;
            let address = indexed_address(regs, use_ix, get(decoded, "d"));
            let value = mem.read_byte(address as usize);
            let result = if decoded.mnemonic == "index_inc_mem" {
                flags.h = half_carry_add(value, 1, 0);
                flags.pv = value == 0x7F;
                flags.n = false;
                value.wrapping_add(1)
            } else {
                flags.h = half_carry_sub(value, 1, 0);
                flags.pv = value == 0x80;
                flags.n = true;
                value.wrapping_sub(1)
            };
            flags.s = s_flag(result);
            flags.z = z_flag(result);
            mem.write_byte(address as usize, result);
            fallthrough
        }
        "index_alu_mem" => {
            let use_ix = get(decoded, "index") != 0;
            let address = indexed_address(regs, use_ix, get(decoded, "d"));
            let operand = mem.read_byte(address as usize);
            let operation = get(decoded, "op") as u8;
            let result = alu8(operation, regs.a, operand, flags);
            if operation != ALU_CP {
                regs.a = result;
            }
            fallthrough
        }
        "index_jp" => ExecuteResult {
            next_pc: index_value(regs, get(decoded, "index") != 0),
            halted: false,
        },
        "index_ex_sp" => {
            let use_ix = get(decoded, "index") != 0;
            let current = index_value(regs, use_ix);
            let stacked = read16(mem, regs.sp);
            write16(mem, regs.sp, current);
            set_index_value(regs, use_ix, stacked);
            fallthrough
        }
        "index_cb_rot" | "index_cb_bit" | "index_cb_res" | "index_cb_set" => {
            let use_ix = get(decoded, "index") != 0;
            let address = indexed_address(regs, use_ix, get(decoded, "d"));
            let value = mem.read_byte(address as usize);
            let register = get(decoded, "reg") as u8;
            let bit = get(decoded, "bit") as u8;
            match decoded.mnemonic.as_str() {
                "index_cb_rot" => {
                    let (result, carry) = rotate_shift(value, get(decoded, "op") as u8, flags.c);
                    mem.write_byte(address as usize, result);
                    if register != REG_M {
                        regs.write(register, result, mem);
                    }
                    flags.c = carry;
                    flags.h = false;
                    flags.n = false;
                    flags.s = s_flag(result);
                    flags.z = z_flag(result);
                    flags.pv = parity(result);
                }
                "index_cb_bit" => {
                    flags.z = value & (1 << bit) == 0;
                    flags.h = true;
                    flags.n = false;
                }
                "index_cb_res" => {
                    let result = value & !(1 << bit);
                    mem.write_byte(address as usize, result);
                    if register != REG_M {
                        regs.write(register, result, mem);
                    }
                }
                _ => {
                    let result = value | (1 << bit);
                    mem.write_byte(address as usize, result);
                    if register != REG_M {
                        regs.write(register, result, mem);
                    }
                }
            }
            fallthrough
        }

        // Defense in depth: the checked simulator rejects undefined
        // decodes before execution, but keep the executor fail-closed.
        _ => ExecuteResult {
            next_pc: pc,
            halted: true,
        },
    }
}

/// `DAA` — decimal-adjust accumulator after `ADD`/`SUB` on BCD values.
/// Direct port of `z80_simulator.flags.daa`.  Returns
/// `(new_a, new_h, new_pv, new_c)`; S/Z are derived by the caller.
fn daa(a: u8, flag_n: bool, flag_h: bool, flag_c: bool) -> (u8, bool, bool, bool) {
    let mut correction: u8 = 0;
    let mut c_out = flag_c;

    if !flag_n {
        if flag_h || (a & 0x0F) > 9 {
            correction |= 0x06;
        }
        if flag_c || a > 0x99 {
            correction |= 0x60;
            c_out = true;
        }
        let new_a = a.wrapping_add(correction);
        let new_h = (a & 0x0F) + (correction & 0x0F) > 0x0F;
        (new_a, new_h, parity(new_a), c_out)
    } else {
        if flag_h || (a & 0x0F) > 9 {
            correction |= 0x06;
        }
        if flag_c || a > 0x99 {
            correction |= 0x60;
            c_out = true;
        }
        let new_a = a.wrapping_sub(correction);
        let new_h = flag_h && (a & 0x0F) < 6;
        (new_a, new_h, parity(new_a), c_out)
    }
}
