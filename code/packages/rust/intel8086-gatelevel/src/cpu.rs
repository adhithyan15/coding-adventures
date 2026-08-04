//! Intel 8086 gate-level CPU simulator.
//!
//! # Design
//!
//! Every arithmetic/logical operation on data routes through:
//! - `logic_gates` crate: AND, OR, XOR, NOT
//! - `arithmetic` crate: full_adder chains
//! - `bits` module: `add_8bit`, `add_16bit`, `add_20bit` wrappers
//!
//! The only exceptions are MUL/DIV (host arithmetic — gate-level ×16 is out
//! of scope) and segment left-shift (`seg × 16 = seg << 4` is wiring, not
//! arithmetic).
//!
//! # Memory model
//!
//! 1 MB flat byte array, heap-allocated. Physical address = (seg×16 + off) & 0xFFFFF.
//!
//! # Interrupts
//!
//! `INT n` pushes FLAGS, CS, IP onto the stack, then jumps through the
//! interrupt vector table at address `n × 4`.
//!
//! # Halt
//!
//! `HLT` (0xF4) sets `halted = true`. Calling `step()` when halted panics.

#![allow(clippy::too_many_lines)]

use crate::alu::{
    add8, add16, and8, and16, aaa, aad, aam, aas, daa, das,
    dec8, dec16, div8, div16, idiv8, idiv16, imul8, imul16, inc8, inc16,
    mul8, mul16, neg8, neg16, not8, not16, or8, or16,
    rcl, rcr, rol, ror, sar, shl, shr, sub8, sub16, xor8, xor16,
    AluResult8086,
};
use crate::bits::{
    add_16bit, compute_parity, compute_zero, int_to_bits8, invert_16bit,
};
use crate::registers::{add_20bit, RegisterFile8086};

const MEM_SIZE: usize = 1_048_576;
const PORT_SIZE: usize = 256;
const BYTE_MASK: u16 = 0xFF;
const WORD_MASK: u16 = 0xFFFF;
const PHYS_MASK: u32 = 0xFFFFF;

// ─── Public CPU state snapshot ────────────────────────────────────────────────

/// Snapshot of the full Intel 8086 state, returned by `get_state()`.
#[derive(Debug, Clone)]
pub struct CpuState {
    pub ax: u16, pub bx: u16, pub cx: u16, pub dx: u16,
    pub si: u16, pub di: u16, pub sp: u16, pub bp: u16,
    pub cs: u16, pub ds: u16, pub ss: u16, pub es: u16,
    pub ip: u16,
    pub cf: bool, pub pf: bool, pub af: bool, pub zf: bool,
    pub sf: bool, pub tf: bool, pub if_: bool, pub df: bool, pub of: bool,
    pub halted: bool,
}

// ─── CPU struct ───────────────────────────────────────────────────────────────

/// Gate-level Intel 8086 simulator.
///
/// # Example
/// ```
/// use coding_adventures_intel8086_gatelevel::cpu::Cpu8086;
/// let mut cpu = Cpu8086::new();
/// // MOV AX, 42; HLT
/// cpu.execute(&[0xB8, 42, 0, 0xF4], 1000);
/// assert_eq!(cpu.rf.ax, 42);
/// assert!(cpu.halted);
/// ```
pub struct Cpu8086 {
    pub rf: RegisterFile8086,
    pub mem: Box<[u8; MEM_SIZE]>,
    pub halted: bool,
    pub input_ports: [u8; PORT_SIZE],
    pub output_ports: [u8; PORT_SIZE],
}

impl Cpu8086 {
    /// Create a new CPU with zeroed registers and memory.
    pub fn new() -> Self {
        Cpu8086 {
            rf: RegisterFile8086::new(),
            mem: Box::new([0u8; MEM_SIZE]),
            halted: false,
            input_ports: [0u8; PORT_SIZE],
            output_ports: [0u8; PORT_SIZE],
        }
    }

    /// Reset to power-on state: all registers, flags, and memory zeroed.
    pub fn reset(&mut self) {
        self.rf = RegisterFile8086::new();
        self.mem.fill(0);
        self.halted = false;
        self.input_ports = [0u8; PORT_SIZE];
        self.output_ports = [0u8; PORT_SIZE];
    }

    /// Write program bytes into memory at a physical address.
    ///
    /// Bytes that would exceed the 1 MB address space are silently dropped.
    /// If `origin >= 1_048_576`, the load is a no-op.
    pub fn load(&mut self, program: &[u8], origin: usize) {
        if origin >= MEM_SIZE { return; }
        let end = origin.saturating_add(program.len()).min(MEM_SIZE);
        self.mem[origin..end].copy_from_slice(&program[..end - origin]);
    }

    /// Return a snapshot of the full CPU state.
    pub fn get_state(&self) -> CpuState {
        let rf = &self.rf;
        CpuState {
            ax: rf.ax, bx: rf.bx, cx: rf.cx, dx: rf.dx,
            si: rf.si, di: rf.di, sp: rf.sp, bp: rf.bp,
            cs: rf.cs, ds: rf.ds, ss: rf.ss, es: rf.es,
            ip: rf.ip,
            cf: rf.flag_cf != 0, pf: rf.flag_pf != 0, af: rf.flag_af != 0,
            zf: rf.flag_zf != 0, sf: rf.flag_sf != 0, tf: rf.flag_tf != 0,
            if_: rf.flag_if != 0, df: rf.flag_df != 0, of: rf.flag_of != 0,
            halted: self.halted,
        }
    }

    /// Reset, load program at address 0, run until HLT or `max_steps`.
    ///
    /// Returns number of steps executed.
    pub fn execute(&mut self, program: &[u8], max_steps: usize) -> usize {
        self.reset();
        self.load(program, 0);
        let mut steps = 0;
        while !self.halted && steps < max_steps {
            self.step();
            steps += 1;
        }
        steps
    }

    /// Execute one fetch-decode-execute cycle. Panics if halted.
    pub fn step(&mut self) {
        assert!(!self.halted, "CPU is halted; call reset() to restart");
        self.fetch_decode_execute();
    }

    // ── Memory helpers ────────────────────────────────────────────────────────

    fn phys(&self, seg: u16, offset: u16) -> usize {
        (add_20bit((seg as u32) << 4, offset as u32) & PHYS_MASK) as usize
    }

    fn read_byte(&self, seg: u16, offset: u16) -> u8 {
        self.mem[self.phys(seg, offset)]
    }

    fn write_byte(&mut self, seg: u16, offset: u16, value: u8) {
        let addr = self.phys(seg, offset);
        self.mem[addr] = value;
    }

    fn read_word(&self, seg: u16, offset: u16) -> u16 {
        let lo = self.mem[self.phys(seg, offset)];
        let hi = self.mem[self.phys(seg, offset.wrapping_add(1))];
        (lo as u16) | ((hi as u16) << 8)
    }

    fn write_word(&mut self, seg: u16, offset: u16, value: u16) {
        let addr_lo = self.phys(seg, offset);
        let addr_hi = self.phys(seg, offset.wrapping_add(1));
        self.mem[addr_lo] = (value & 0xFF) as u8;
        self.mem[addr_hi] = ((value >> 8) & 0xFF) as u8;
    }

    // Fetch one byte from CS:IP, advance IP through the 16-bit adder.
    fn fetch8(&mut self) -> u8 {
        let ip = self.rf.ip;
        let cs = self.rf.cs;
        let v = self.mem[self.phys(cs, ip)];
        let (new_ip, _, _) = add_16bit(ip, 1, 0);
        self.rf.ip = new_ip;
        v
    }

    fn fetch16(&mut self) -> u16 {
        let lo = self.fetch8() as u16;
        let hi = self.fetch8() as u16;
        lo | (hi << 8)
    }

    // Fetch a sign-extended 8-bit displacement.
    fn fetch_s8(&mut self) -> i16 {
        self.fetch8() as i8 as i16
    }

    // Fetch a 16-bit signed value.
    fn fetch_s16(&mut self) -> i16 {
        self.fetch16() as i16
    }

    // ── Stack helpers ─────────────────────────────────────────────────────────

    fn push16(&mut self, val: u16) {
        // SP -= 2 via two gate-level decrements
        let sp = self.rf.sp;
        let (sp1, _, _) = add_16bit(sp, invert_16bit(1), 1);
        let (sp2, _, _) = add_16bit(sp1, invert_16bit(1), 1);
        self.rf.sp = sp2;
        let ss = self.rf.ss;
        self.write_word(ss, sp2, val);
    }

    fn pop16(&mut self) -> u16 {
        let sp = self.rf.sp;
        let ss = self.rf.ss;
        let val = self.read_word(ss, sp);
        let (new_sp, _, _) = add_16bit(sp, 2, 0);
        self.rf.sp = new_sp;
        val
    }

    // ── FLAGS helpers ─────────────────────────────────────────────────────────

    fn apply_alu_result(&mut self, r: &AluResult8086) {
        self.rf.flag_cf = r.flag_cf;
        self.rf.flag_of = r.flag_of;
        self.rf.flag_sf = r.flag_sf;
        self.rf.flag_zf = r.flag_zf;
        self.rf.flag_af = r.flag_af;
        self.rf.flag_pf = r.flag_pf;
    }

    fn apply_alu_no_cf(&mut self, r: &AluResult8086) {
        self.rf.flag_of = r.flag_of;
        self.rf.flag_sf = r.flag_sf;
        self.rf.flag_zf = r.flag_zf;
        self.rf.flag_af = r.flag_af;
        self.rf.flag_pf = r.flag_pf;
    }

    fn apply_logic_flags(&mut self, r: &AluResult8086) {
        self.rf.flag_cf = 0;
        self.rf.flag_of = 0;
        self.rf.flag_af = 0;
        self.rf.flag_sf = r.flag_sf;
        self.rf.flag_zf = r.flag_zf;
        self.rf.flag_pf = r.flag_pf;
    }

    fn set_szp_byte(&mut self, val: u8) {
        let bits = int_to_bits8(val);
        self.rf.flag_sf = bits[7];
        self.rf.flag_zf = compute_zero(&bits);
        self.rf.flag_pf = compute_parity(&bits);
    }

    fn flags_low8(&self) -> u8 {
        let rf = &self.rf;
        rf.flag_cf | (1 << 1) | (rf.flag_pf << 2) |
        (rf.flag_af << 4) | (rf.flag_zf << 6) | (rf.flag_sf << 7)
    }

    fn load_flags_low8(&mut self, f: u8) {
        self.rf.flag_cf = f & 1;
        self.rf.flag_pf = (f >> 2) & 1;
        self.rf.flag_af = (f >> 4) & 1;
        self.rf.flag_zf = (f >> 6) & 1;
        self.rf.flag_sf = (f >> 7) & 1;
    }

    // ── ModRM decode ──────────────────────────────────────────────────────────

    /// Returns `(mod, reg, rm, effective_address_offset)`.
    /// For mod=3 (register), ea = rm.
    fn decode_modrm(&mut self, modrm: u8, _word: bool) -> (u8, u8, u8, u16) {
        let mod_ = (modrm >> 6) & 3;
        let reg = (modrm >> 3) & 7;
        let rm = modrm & 7;

        if mod_ == 3 {
            return (mod_, reg, rm, rm as u16);
        }

        let bx = self.rf.bx;
        let si = self.rf.si;
        let di = self.rf.di;
        let bp = self.rf.bp;

        let base: u16 = match rm {
            0 => { let (v, _, _) = add_16bit(bx, si, 0); v }
            1 => { let (v, _, _) = add_16bit(bx, di, 0); v }
            2 => { let (v, _, _) = add_16bit(bp, si, 0); v }
            3 => { let (v, _, _) = add_16bit(bp, di, 0); v }
            4 => si,
            5 => di,
            6 => if mod_ == 0 { self.fetch16() } else { bp },
            _ => bx,
        };

        let ea: u16 = match mod_ {
            1 => {
                let disp = self.fetch_s8() as u16; // sign-extend → u16 via i16
                let (v, _, _) = add_16bit(base, disp, 0);
                v
            }
            2 => {
                let disp = self.fetch_s16() as u16;
                let (v, _, _) = add_16bit(base, disp, 0);
                v
            }
            _ => base, // mod_ == 0
        };

        (mod_, reg, rm, ea)
    }

    /// Return the effective segment register for a memory access.
    ///
    /// Rule: BP-based r/m (rm=2,3 or rm=6 with mod≠0) → SS; otherwise → DS.
    fn effective_seg(&self, rm: u8, mod_: u8, seg_override: Option<u8>) -> u16 {
        if let Some(s) = seg_override {
            return self.rf.read_seg(s);
        }
        let uses_bp = rm == 2 || rm == 3 || (rm == 6 && mod_ != 0);
        if uses_bp { self.rf.ss } else { self.rf.ds }
    }

    fn read_rm(&self, mod_: u8, rm: u8, seg: u16, ea: u16, word: bool) -> u16 {
        if mod_ == 3 {
            if word { self.rf.read16(rm) } else { self.rf.read8(rm) as u16 }
        } else if word {
            self.read_word(seg, ea)
        } else {
            self.read_byte(seg, ea) as u16
        }
    }

    fn write_rm(&mut self, mod_: u8, rm: u8, seg: u16, ea: u16, val: u16, word: bool) {
        if mod_ == 3 {
            if word { self.rf.write16(rm, val); } else { self.rf.write8(rm, val as u8); }
        } else if word {
            self.write_word(seg, ea, val);
        } else {
            self.write_byte(seg, ea, val as u8);
        }
    }

    // ── ALU dispatcher ────────────────────────────────────────────────────────

    /// Execute one of 8 ALU ops. Returns (result, mnemonic_index).
    /// op: 0=ADD 1=OR 2=ADC 3=SBB 4=AND 5=SUB 6=XOR 7=CMP
    fn alu_op(&mut self, op: u8, a: u16, b: u16, word: bool) -> (u16, u8) {
        let cf = self.rf.flag_cf;
        let result = if word {
            match op {
                0 => { let r = add16(a, b, 0);       self.apply_alu_result(&r);   r.result }
                1 => { let r = or16(a, b);            self.apply_logic_flags(&r);  r.result }
                2 => { let r = add16(a, b, cf);      self.apply_alu_result(&r);   r.result }
                3 => { let r = sub16(a, b, cf);      self.apply_alu_result(&r);   r.result }
                4 => { let r = and16(a, b);           self.apply_logic_flags(&r);  r.result }
                5 => { let r = sub16(a, b, 0);        self.apply_alu_result(&r);   r.result }
                6 => { let r = xor16(a, b);           self.apply_logic_flags(&r);  r.result }
                _ => { let r = sub16(a, b, 0);        self.apply_alu_result(&r);   a }        // CMP: discard
            }
        } else {
            match op {
                0 => { let r = add8(a as u8, b as u8, 0);  self.apply_alu_result(&r);  r.result }
                1 => { let r = or8(a as u8, b as u8);       self.apply_logic_flags(&r); r.result }
                2 => { let r = add8(a as u8, b as u8, cf);  self.apply_alu_result(&r);  r.result }
                3 => { let r = sub8(a as u8, b as u8, cf);  self.apply_alu_result(&r);  r.result }
                4 => { let r = and8(a as u8, b as u8);      self.apply_logic_flags(&r); r.result }
                5 => { let r = sub8(a as u8, b as u8, 0);   self.apply_alu_result(&r);  r.result }
                6 => { let r = xor8(a as u8, b as u8);      self.apply_logic_flags(&r); r.result }
                _ => { let r = sub8(a as u8, b as u8, 0);   self.apply_alu_result(&r);  a }    // CMP
            }
        };
        (result, op)
    }

    // ── Condition evaluation ──────────────────────────────────────────────────

    fn eval_cond(&self, cond: u8) -> bool {
        let rf = &self.rf;
        let cf = rf.flag_cf != 0;
        let of = rf.flag_of != 0;
        let sf = rf.flag_sf != 0;
        let zf = rf.flag_zf != 0;
        let pf = rf.flag_pf != 0;
        match cond & 0xF {
            0  => of,
            1  => !of,
            2  => cf,
            3  => !cf,
            4  => zf,
            5  => !zf,
            6  => cf || zf,
            7  => !cf && !zf,
            8  => sf,
            9  => !sf,
            10 => pf,
            11 => !pf,
            12 => sf != of,
            13 => sf == of,
            14 => zf || (sf != of),
            _  => !zf && (sf == of),
        }
    }

    // ── Interrupt trigger ─────────────────────────────────────────────────────

    fn trigger_interrupt(&mut self, vector: u8) {
        let flags = self.rf.pack_flags();
        self.push16(flags);
        let cs = self.rf.cs;
        self.push16(cs);
        let ip = self.rf.ip;
        self.push16(ip);
        let ivt_addr = (vector as usize) * 4;
        let new_ip = (self.mem[ivt_addr] as u16) | ((self.mem[ivt_addr + 1] as u16) << 8);
        let new_cs = (self.mem[ivt_addr + 2] as u16) | ((self.mem[ivt_addr + 3] as u16) << 8);
        self.rf.ip = new_ip;
        self.rf.cs = new_cs;
        self.rf.flag_if = 0;
        self.rf.flag_tf = 0;
    }

    // ── String operation helpers ──────────────────────────────────────────────

    fn str_step(&self, word: bool) -> i16 {
        let inc: i16 = if word { 2 } else { 1 };
        if self.rf.flag_df != 0 { -inc } else { inc }
    }

    fn exec_lods(&mut self, word: bool, step: i16, seg_src: u16, rep: Option<u8>) {
        let count = if rep.is_some() { self.rf.cx } else { 1 };
        let mut remaining = count;
        loop {
            let si = self.rf.si;
            if word {
                let v = self.read_word(seg_src, si);
                self.rf.ax = v;
            } else {
                let v = self.read_byte(seg_src, si);
                self.rf.ax = (self.rf.ax & 0xFF00) | (v as u16);
            }
            let (new_si, _, _) = add_16bit(si, step as u16, 0);
            self.rf.si = new_si;
            if rep.is_some() {
                let cx = self.rf.cx;
                let (new_cx, _, _) = add_16bit(cx, invert_16bit(1), 1);
                self.rf.cx = new_cx;
                if new_cx == 0 { break; }
            }
            remaining = remaining.saturating_sub(1);
            if remaining == 0 { break; }
        }
    }

    fn exec_stos(&mut self, word: bool, step: i16, rep: Option<u8>) {
        let es = self.rf.es;
        let count = if rep.is_some() { self.rf.cx } else { 1 };
        let mut remaining = count;
        loop {
            let di = self.rf.di;
            if word {
                let v = self.rf.ax;
                self.write_word(es, di, v);
            } else {
                let v = (self.rf.ax & 0xFF) as u8;
                self.write_byte(es, di, v);
            }
            let (new_di, _, _) = add_16bit(di, step as u16, 0);
            self.rf.di = new_di;
            if rep.is_some() {
                let cx = self.rf.cx;
                let (new_cx, _, _) = add_16bit(cx, invert_16bit(1), 1);
                self.rf.cx = new_cx;
                if new_cx == 0 { break; }
            }
            remaining = remaining.saturating_sub(1);
            if remaining == 0 { break; }
        }
    }

    fn exec_movs(&mut self, word: bool, step: i16, seg_src: u16, rep: Option<u8>) {
        let es = self.rf.es;
        let step_u = step as u16;
        loop {
            let si = self.rf.si;
            let di = self.rf.di;
            if word {
                let v = self.read_word(seg_src, si);
                self.write_word(es, di, v);
            } else {
                let v = self.read_byte(seg_src, si);
                self.write_byte(es, di, v);
            }
            let (new_si, _, _) = add_16bit(si, step_u, 0);
            let (new_di, _, _) = add_16bit(di, step_u, 0);
            self.rf.si = new_si;
            self.rf.di = new_di;
            if rep.is_some() {
                let cx = self.rf.cx;
                let (new_cx, _, _) = add_16bit(cx, invert_16bit(1), 1);
                self.rf.cx = new_cx;
                if new_cx == 0 { break; }
            } else {
                break;
            }
        }
    }

    fn exec_cmps(&mut self, word: bool, step: i16, seg_src: u16, rep: Option<u8>) {
        let es = self.rf.es;
        let step_u = step as u16;
        loop {
            let si = self.rf.si;
            let di = self.rf.di;
            let a = if word { self.read_word(seg_src, si) } else { self.read_byte(seg_src, si) as u16 };
            let b = if word { self.read_word(es, di) } else { self.read_byte(es, di) as u16 };
            let r = if word { sub16(a, b, 0) } else { sub8(a as u8, b as u8, 0) };
            self.apply_alu_result(&r);
            let (new_si, _, _) = add_16bit(si, step_u, 0);
            let (new_di, _, _) = add_16bit(di, step_u, 0);
            self.rf.si = new_si;
            self.rf.di = new_di;
            if let Some(prefix) = rep {
                let cx = self.rf.cx;
                let (new_cx, _, _) = add_16bit(cx, invert_16bit(1), 1);
                self.rf.cx = new_cx;
                if new_cx == 0 { break; }
                let zf = self.rf.flag_zf != 0;
                if prefix == 0xF3 && !zf { break; }
                if prefix == 0xF2 && zf { break; }
            } else {
                break;
            }
        }
    }

    fn exec_scas(&mut self, word: bool, step: i16, rep: Option<u8>) {
        let es = self.rf.es;
        let step_u = step as u16;
        loop {
            let di = self.rf.di;
            let b = if word { self.read_word(es, di) } else { self.read_byte(es, di) as u16 };
            let a = if word { self.rf.ax } else { self.rf.ax & 0xFF };
            let r = if word { sub16(a, b, 0) } else { sub8(a as u8, b as u8, 0) };
            self.apply_alu_result(&r);
            let (new_di, _, _) = add_16bit(di, step_u, 0);
            self.rf.di = new_di;
            if let Some(prefix) = rep {
                let cx = self.rf.cx;
                let (new_cx, _, _) = add_16bit(cx, invert_16bit(1), 1);
                self.rf.cx = new_cx;
                if new_cx == 0 { break; }
                let zf = self.rf.flag_zf != 0;
                if prefix == 0xF3 && !zf { break; }
                if prefix == 0xF2 && zf { break; }
            } else {
                break;
            }
        }
    }

    // ── Main fetch-decode-execute ─────────────────────────────────────────────

    fn fetch_decode_execute(&mut self) {
        let mut seg_override: Option<u8> = None;
        let mut rep_prefix: Option<u8> = None;

        // Prefix loop — consume all prefix bytes before dispatching.
        // Capped at 16 iterations: an infinite stream of prefix bytes would spin
        // forever within a single step() call, bypassing the max_steps guard in
        // execute(). The real 8086 has undefined behaviour for excessive prefixes;
        // we treat it as #UD (halt).
        let op = 'prefix: {
            for _ in 0..16 {
                let b = self.fetch8();
                match b {
                    0x26 => seg_override = Some(0), // ES:
                    0x2E => seg_override = Some(1), // CS:
                    0x36 => seg_override = Some(2), // SS:
                    0x3E => seg_override = Some(3), // DS:
                    0xF2 | 0xF3 => rep_prefix = Some(b),
                    0xF0 => {} // LOCK — ignored in this educational simulator
                    _ => break 'prefix b,
                }
            }
            self.halted = true; // too many prefix bytes — undefined behaviour
            return;
        };

        self.exec_op(op, seg_override, rep_prefix);
    }

    #[allow(clippy::cognitive_complexity)]
    fn exec_op(&mut self, op: u8, seg_override: Option<u8>, rep_prefix: Option<u8>) {
        // ── MOV r/m, reg or reg, r/m (88/89/8A/8B) ───────────────────────────
        if matches!(op, 0x88..=0x8B) {
            let word = (op & 1) != 0;
            let d = (op & 2) != 0;
            let modrm = self.fetch8();
            let (mod_, reg, rm, ea) = self.decode_modrm(modrm, word);
            let seg = self.effective_seg(rm, mod_, seg_override);
            if d {
                let src = self.read_rm(mod_, rm, seg, ea, word);
                if word { self.rf.write16(reg, src); } else { self.rf.write8(reg, src as u8); }
            } else {
                let src = if word { self.rf.read16(reg) } else { self.rf.read8(reg) as u16 };
                self.write_rm(mod_, rm, seg, ea, src, word);
            }
            return;
        }

        // MOV r/m8, imm8 (C6)
        if op == 0xC6 {
            let modrm = self.fetch8();
            let (mod_, _, rm, ea) = self.decode_modrm(modrm, false);
            let seg = self.effective_seg(rm, mod_, seg_override);
            let imm = self.fetch8();
            self.write_rm(mod_, rm, seg, ea, imm as u16, false);
            return;
        }

        // MOV r/m16, imm16 (C7)
        if op == 0xC7 {
            let modrm = self.fetch8();
            let (mod_, _, rm, ea) = self.decode_modrm(modrm, true);
            let seg = self.effective_seg(rm, mod_, seg_override);
            let imm = self.fetch16();
            self.write_rm(mod_, rm, seg, ea, imm, true);
            return;
        }

        // MOV reg8, imm8 (B0–B7)
        if (0xB0..=0xB7).contains(&op) {
            let reg = op - 0xB0;
            let imm = self.fetch8();
            self.rf.write8(reg, imm);
            return;
        }

        // MOV reg16, imm16 (B8–BF)
        if (0xB8..=0xBF).contains(&op) {
            let reg = op - 0xB8;
            let imm = self.fetch16();
            self.rf.write16(reg, imm);
            return;
        }

        // MOV AL/AX, [addr] (A0/A1)
        if op == 0xA0 || op == 0xA1 {
            let word = (op & 1) != 0;
            let addr = self.fetch16();
            let seg = seg_override.map_or(self.rf.ds, |s| self.rf.read_seg(s));
            if word {
                let v = self.read_word(seg, addr);
                self.rf.ax = v;
            } else {
                let v = self.read_byte(seg, addr);
                self.rf.ax = (self.rf.ax & 0xFF00) | (v as u16);
            }
            return;
        }

        // MOV [addr], AL/AX (A2/A3)
        if op == 0xA2 || op == 0xA3 {
            let word = (op & 1) != 0;
            let addr = self.fetch16();
            let seg = seg_override.map_or(self.rf.ds, |s| self.rf.read_seg(s));
            if word { self.write_word(seg, addr, self.rf.ax); }
            else { self.write_byte(seg, addr, (self.rf.ax & 0xFF) as u8); }
            return;
        }

        // MOV r/m, sreg (8C)
        if op == 0x8C {
            let modrm = self.fetch8();
            let (mod_, reg, rm, ea) = self.decode_modrm(modrm, true);
            let seg_r = self.effective_seg(rm, mod_, seg_override);
            let val = self.rf.read_seg(reg & 3);
            self.write_rm(mod_, rm, seg_r, ea, val, true);
            return;
        }

        // MOV sreg, r/m (8E)
        if op == 0x8E {
            let modrm = self.fetch8();
            let (mod_, reg, rm, ea) = self.decode_modrm(modrm, true);
            let seg_r = self.effective_seg(rm, mod_, seg_override);
            let val = self.read_rm(mod_, rm, seg_r, ea, true);
            self.rf.write_seg(reg & 3, val);
            return;
        }

        // ── XCHG ──────────────────────────────────────────────────────────────

        // XCHG AX, reg (90–97; 90 = NOP)
        if (0x90..=0x97).contains(&op) {
            let reg = op - 0x90;
            if reg != 0 {
                let tmp = self.rf.ax;
                self.rf.ax = self.rf.read16(reg);
                self.rf.write16(reg, tmp);
            }
            return;
        }

        // XCHG r/m, reg (86/87)
        if op == 0x86 || op == 0x87 {
            let word = (op & 1) != 0;
            let modrm = self.fetch8();
            let (mod_, reg, rm, ea) = self.decode_modrm(modrm, word);
            let seg = self.effective_seg(rm, mod_, seg_override);
            let a = self.read_rm(mod_, rm, seg, ea, word);
            let b = if word { self.rf.read16(reg) } else { self.rf.read8(reg) as u16 };
            self.write_rm(mod_, rm, seg, ea, b, word);
            if word { self.rf.write16(reg, a); } else { self.rf.write8(reg, a as u8); }
            return;
        }

        // ── PUSH / POP ─────────────────────────────────────────────────────────

        // PUSH reg (50–57)
        if (0x50..=0x57).contains(&op) {
            let v = self.rf.read16(op - 0x50);
            self.push16(v);
            return;
        }

        // POP reg (58–5F)
        if (0x58..=0x5F).contains(&op) {
            let v = self.pop16();
            self.rf.write16(op - 0x58, v);
            return;
        }

        // PUSH sreg
        if matches!(op, 0x06 | 0x0E | 0x16 | 0x1E) {
            let code = match op { 0x06 => 0, 0x0E => 1, 0x16 => 2, _ => 3 };
            let v = self.rf.read_seg(code);
            self.push16(v);
            return;
        }

        // POP sreg
        if matches!(op, 0x07 | 0x17 | 0x1F) {
            let code = match op { 0x07 => 0, 0x17 => 2, _ => 3 };
            let v = self.pop16();
            self.rf.write_seg(code, v);
            return;
        }

        // POP r/m (8F)
        if op == 0x8F {
            let modrm = self.fetch8();
            let (mod_, _, rm, ea) = self.decode_modrm(modrm, true);
            let seg = self.effective_seg(rm, mod_, seg_override);
            let v = self.pop16();
            self.write_rm(mod_, rm, seg, ea, v, true);
            return;
        }

        // PUSHF / POPF
        if op == 0x9C { let f = self.rf.pack_flags(); self.push16(f); return; }
        if op == 0x9D { let v = self.pop16(); self.rf.unpack_flags(v); return; }

        // ── LEA / LDS / LES ───────────────────────────────────────────────────

        if op == 0x8D {
            let modrm = self.fetch8();
            let (_, reg, _, ea) = self.decode_modrm(modrm, true);
            self.rf.write16(reg, ea);
            return;
        }

        if op == 0xC5 { // LDS
            let modrm = self.fetch8();
            let (mod_, reg, rm, ea) = self.decode_modrm(modrm, true);
            let seg_r = self.effective_seg(rm, mod_, seg_override);
            let off = self.read_word(seg_r, ea);
            let new_ds = self.read_word(seg_r, ea.wrapping_add(2));
            self.rf.write16(reg, off);
            self.rf.ds = new_ds;
            return;
        }

        if op == 0xC4 { // LES
            let modrm = self.fetch8();
            let (mod_, reg, rm, ea) = self.decode_modrm(modrm, true);
            let seg_r = self.effective_seg(rm, mod_, seg_override);
            let off = self.read_word(seg_r, ea);
            let new_es = self.read_word(seg_r, ea.wrapping_add(2));
            self.rf.write16(reg, off);
            self.rf.es = new_es;
            return;
        }

        // ── LAHF / SAHF ───────────────────────────────────────────────────────

        if op == 0x9F { // LAHF
            let f = self.flags_low8();
            self.rf.ax = (self.rf.ax & 0x00FF) | ((f as u16) << 8);
            return;
        }
        if op == 0x9E { // SAHF
            let ah = ((self.rf.ax >> 8) & 0xFF) as u8;
            self.load_flags_low8(ah);
            return;
        }

        // ── CBW / CWD ─────────────────────────────────────────────────────────

        if op == 0x98 { // CBW — sign-extend AL → AX
            let al = (self.rf.ax & 0xFF) as u8;
            self.rf.ax = if al & 0x80 != 0 { (al as u16) | 0xFF00 } else { al as u16 };
            return;
        }
        if op == 0x99 { // CWD — sign-extend AX → DX:AX
            self.rf.dx = if self.rf.ax & 0x8000 != 0 { 0xFFFF } else { 0 };
            return;
        }

        // ── XLAT ──────────────────────────────────────────────────────────────

        if op == 0xD7 {
            let al = (self.rf.ax & 0xFF) as u8;
            let seg = seg_override.map_or(self.rf.ds, |s| self.rf.read_seg(s));
            let (xlat_addr, _, _) = add_16bit(self.rf.bx, al as u16, 0);
            let v = self.read_byte(seg, xlat_addr);
            self.rf.ax = (self.rf.ax & 0xFF00) | (v as u16);
            return;
        }

        // ── 80-group: r/m, imm ALU ─────────────────────────────────────────────

        if matches!(op, 0x80..=0x83) {
            let word = op == 0x81 || op == 0x83;
            let modrm = self.fetch8();
            let ext = (modrm >> 3) & 7;
            let rm = modrm & 7;
            let mod_ = (modrm >> 6) & 3;
            let (_, _, _, ea) = self.decode_modrm(modrm, word);
            let seg = self.effective_seg(rm, mod_, seg_override);
            let imm: u16 = match op {
                0x80 | 0x82 => self.fetch8() as u16,
                0x81 => self.fetch16(),
                _ => { // 0x83: sign-extend 8→16
                    let v = self.fetch8() as i8 as i16;
                    v as u16
                }
            };
            let a = self.read_rm(mod_, rm, seg, ea, word);
            let (result, _) = self.alu_op(ext, a, imm, word);
            if ext != 7 { self.write_rm(mod_, rm, seg, ea, result, word); }
            return;
        }

        // TEST r/m, reg (84/85)
        if op == 0x84 || op == 0x85 {
            let word = (op & 1) != 0;
            let modrm = self.fetch8();
            let (mod_, reg, rm, ea) = self.decode_modrm(modrm, word);
            let seg = self.effective_seg(rm, mod_, seg_override);
            let a = self.read_rm(mod_, rm, seg, ea, word);
            let b = if word { self.rf.read16(reg) } else { self.rf.read8(reg) as u16 };
            let r = if word { and16(a, b) } else { and8(a as u8, b as u8) };
            self.apply_logic_flags(&r);
            return;
        }

        // Accumulator-immediate ALU (04, 05, 0C, 0D, … 3C, 3D, A8, A9)
        {
            const ACC_IMM: [(u8, u8, bool); 18] = [
                (0x04, 0, false), (0x05, 0, true),
                (0x0C, 1, false), (0x0D, 1, true),
                (0x14, 2, false), (0x15, 2, true),
                (0x1C, 3, false), (0x1D, 3, true),
                (0x24, 4, false), (0x25, 4, true),
                (0x2C, 5, false), (0x2D, 5, true),
                (0x34, 6, false), (0x35, 6, true),
                (0x3C, 7, false), (0x3D, 7, true),
                (0xA8, 4, false), (0xA9, 4, true),
            ];
            if let Some(&(_, alu_op, word)) = ACC_IMM.iter().find(|&&(o, _, _)| o == op) {
                let imm = if word { self.fetch16() } else { self.fetch8() as u16 };
                let a = if word { self.rf.ax } else { self.rf.ax & 0xFF };
                let (result, _) = self.alu_op(alu_op, a, imm, word);
                // TEST (A8/A9) and CMP (3C/3D) don't write back
                if alu_op != 7 && op != 0xA8 && op != 0xA9 {
                    if word { self.rf.ax = result; }
                    else { self.rf.ax = (self.rf.ax & 0xFF00) | (result & 0xFF); }
                }
                return;
            }
        }

        // Standard ALU r/m ↔ reg (00–3B)
        {
            // (opcode, alu_op, word, d_bit)
            const STD_ALU: [(u8, u8, bool, bool); 32] = [
                (0x00,0,false,false),(0x01,0,true,false),(0x02,0,false,true),(0x03,0,true,true),
                (0x08,1,false,false),(0x09,1,true,false),(0x0A,1,false,true),(0x0B,1,true,true),
                (0x10,2,false,false),(0x11,2,true,false),(0x12,2,false,true),(0x13,2,true,true),
                (0x18,3,false,false),(0x19,3,true,false),(0x1A,3,false,true),(0x1B,3,true,true),
                (0x20,4,false,false),(0x21,4,true,false),(0x22,4,false,true),(0x23,4,true,true),
                (0x28,5,false,false),(0x29,5,true,false),(0x2A,5,false,true),(0x2B,5,true,true),
                (0x30,6,false,false),(0x31,6,true,false),(0x32,6,false,true),(0x33,6,true,true),
                (0x38,7,false,false),(0x39,7,true,false),(0x3A,7,false,true),(0x3B,7,true,true),
            ];
            if let Some(&(_, alu_op, word, d_bit)) = STD_ALU.iter().find(|&&(o, _, _, _)| o == op) {
                let modrm = self.fetch8();
                let (mod_, reg, rm, ea) = self.decode_modrm(modrm, word);
                let seg = self.effective_seg(rm, mod_, seg_override);
                if d_bit {
                    let a = if word { self.rf.read16(reg) } else { self.rf.read8(reg) as u16 };
                    let b = self.read_rm(mod_, rm, seg, ea, word);
                    let (result, _) = self.alu_op(alu_op, a, b, word);
                    if alu_op != 7 {
                        if word { self.rf.write16(reg, result); } else { self.rf.write8(reg, result as u8); }
                    }
                } else {
                    let a = self.read_rm(mod_, rm, seg, ea, word);
                    let b = if word { self.rf.read16(reg) } else { self.rf.read8(reg) as u16 };
                    let (result, _) = self.alu_op(alu_op, a, b, word);
                    if alu_op != 7 { self.write_rm(mod_, rm, seg, ea, result, word); }
                }
                return;
            }
        }

        // ── INC / DEC reg16 (40–4F) ───────────────────────────────────────────

        if (0x40..=0x47).contains(&op) {
            let reg = op - 0x40;
            let old_cf = self.rf.flag_cf;
            let r = inc16(self.rf.read16(reg));
            self.rf.write16(reg, r.result);
            self.apply_alu_no_cf(&r);
            self.rf.flag_cf = old_cf;
            return;
        }
        if (0x48..=0x4F).contains(&op) {
            let reg = op - 0x48;
            let old_cf = self.rf.flag_cf;
            let r = dec16(self.rf.read16(reg));
            self.rf.write16(reg, r.result);
            self.apply_alu_no_cf(&r);
            self.rf.flag_cf = old_cf;
            return;
        }

        // FE group: INC/DEC r/m8
        if op == 0xFE {
            let modrm = self.fetch8();
            let ext = (modrm >> 3) & 7;
            let rm = modrm & 7;
            let mod_ = (modrm >> 6) & 3;
            let (_, _, _, ea) = self.decode_modrm(modrm, false);
            let seg = self.effective_seg(rm, mod_, seg_override);
            let a = self.read_rm(mod_, rm, seg, ea, false) as u8;
            let old_cf = self.rf.flag_cf;
            let r = if ext == 0 { inc8(a) } else { dec8(a) };
            self.write_rm(mod_, rm, seg, ea, r.result, false);
            self.apply_alu_no_cf(&r);
            self.rf.flag_cf = old_cf;
            return;
        }

        // FF group: INC/DEC/CALL/JMP/PUSH r/m16
        if op == 0xFF {
            let modrm = self.fetch8();
            let ext = (modrm >> 3) & 7;
            let rm = modrm & 7;
            let mod_ = (modrm >> 6) & 3;
            let (_, _, _, ea) = self.decode_modrm(modrm, true);
            let seg = self.effective_seg(rm, mod_, seg_override);
            let val = self.read_rm(mod_, rm, seg, ea, true);
            match ext {
                0 => { // INC m16
                    let old_cf = self.rf.flag_cf;
                    let r = inc16(val);
                    self.write_rm(mod_, rm, seg, ea, r.result, true);
                    self.apply_alu_no_cf(&r);
                    self.rf.flag_cf = old_cf;
                }
                1 => { // DEC m16
                    let old_cf = self.rf.flag_cf;
                    let r = dec16(val);
                    self.write_rm(mod_, rm, seg, ea, r.result, true);
                    self.apply_alu_no_cf(&r);
                    self.rf.flag_cf = old_cf;
                }
                2 => { // CALL rm16 (near)
                    let ip = self.rf.ip;
                    self.push16(ip);
                    self.rf.ip = val;
                }
                3 => { // CALL FAR m32
                    let new_off = self.read_word(seg, ea);
                    let new_cs = self.read_word(seg, ea.wrapping_add(2));
                    let cs = self.rf.cs; self.push16(cs);
                    let ip = self.rf.ip; self.push16(ip);
                    self.rf.cs = new_cs;
                    self.rf.ip = new_off;
                }
                4 => { self.rf.ip = val; } // JMP rm16
                5 => { // JMP FAR m32
                    let new_off = self.read_word(seg, ea);
                    let new_cs = self.read_word(seg, ea.wrapping_add(2));
                    self.rf.cs = new_cs;
                    self.rf.ip = new_off;
                }
                6 => { self.push16(val); } // PUSH m16
                _ => {}
            }
            return;
        }

        // ── F6/F7 group ───────────────────────────────────────────────────────

        if op == 0xF6 || op == 0xF7 {
            let word = (op & 1) != 0;
            let modrm = self.fetch8();
            let ext = (modrm >> 3) & 7;
            let rm = modrm & 7;
            let mod_ = (modrm >> 6) & 3;
            let (_, _, _, ea) = self.decode_modrm(modrm, word);
            let seg = self.effective_seg(rm, mod_, seg_override);
            let a = self.read_rm(mod_, rm, seg, ea, word);
            match ext {
                0 => { // TEST
                    let imm = if word { self.fetch16() } else { self.fetch8() as u16 };
                    let r = if word { and16(a, imm) } else { and8(a as u8, imm as u8) };
                    self.apply_logic_flags(&r);
                }
                2 => { // NOT
                    let result = if word { not16(a) } else { not8(a as u8) as u16 };
                    self.write_rm(mod_, rm, seg, ea, result, word);
                }
                3 => { // NEG
                    let r = if word { neg16(a) } else { neg8(a as u8) };
                    self.write_rm(mod_, rm, seg, ea, r.result, word);
                    self.apply_alu_result(&r);
                    self.rf.flag_cf = if a != 0 { 1 } else { 0 };
                }
                4 => { // MUL
                    if word {
                        let (dx, ax, cf_of) = mul16(self.rf.ax, a);
                        self.rf.ax = ax; self.rf.dx = dx;
                        self.rf.flag_cf = cf_of; self.rf.flag_of = cf_of;
                    } else {
                        let (ax, cf_of) = mul8((self.rf.ax & 0xFF) as u8, a as u8);
                        self.rf.ax = ax;
                        self.rf.flag_cf = cf_of; self.rf.flag_of = cf_of;
                    }
                }
                5 => { // IMUL
                    if word {
                        let (dx, ax, cf_of) = imul16(self.rf.ax, a);
                        self.rf.ax = ax; self.rf.dx = dx;
                        self.rf.flag_cf = cf_of; self.rf.flag_of = cf_of;
                    } else {
                        let (ax, cf_of) = imul8((self.rf.ax & 0xFF) as u8, a as u8);
                        self.rf.ax = ax;
                        self.rf.flag_cf = cf_of; self.rf.flag_of = cf_of;
                    }
                }
                6 => { // DIV
                    if word {
                        let dividend = ((self.rf.dx as u32) << 16) | (self.rf.ax as u32);
                        match div16(dividend, a) {
                            None => { self.halted = true; }
                            Some((ax, dx)) => { self.rf.ax = ax; self.rf.dx = dx; }
                        }
                    } else {
                        match div8(self.rf.ax, a as u8) {
                            None => { self.halted = true; }
                            Some((q, r)) => { self.rf.ax = ((r as u16) << 8) | (q as u16); }
                        }
                    }
                }
                7 => { // IDIV
                    if word {
                        let d32 = ((self.rf.dx as u32) << 16) | (self.rf.ax as u32);
                        match idiv16(d32, a) {
                            None => { self.halted = true; }
                            Some((ax, dx)) => { self.rf.ax = ax; self.rf.dx = dx; }
                        }
                    } else {
                        match idiv8(self.rf.ax, a as u8) {
                            None => { self.halted = true; }
                            Some((q, r)) => {
                                self.rf.ax = ((r as u16) << 8) | ((q as u16) & 0xFF);
                            }
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        // ── BCD operations ────────────────────────────────────────────────────

        if op == 0x27 { // DAA
            let al = (self.rf.ax & 0xFF) as u8;
            let (new_al, new_af, new_cf) = daa(al, self.rf.flag_af, self.rf.flag_cf);
            self.rf.ax = (self.rf.ax & 0xFF00) | (new_al as u16);
            self.rf.flag_af = new_af;
            self.rf.flag_cf = new_cf;
            self.set_szp_byte(new_al);
            return;
        }
        if op == 0x2F { // DAS
            let al = (self.rf.ax & 0xFF) as u8;
            let (new_al, new_af, new_cf) = das(al, self.rf.flag_af, self.rf.flag_cf);
            self.rf.ax = (self.rf.ax & 0xFF00) | (new_al as u16);
            self.rf.flag_af = new_af;
            self.rf.flag_cf = new_cf;
            self.set_szp_byte(new_al);
            return;
        }
        if op == 0x37 { // AAA
            let al = (self.rf.ax & 0xFF) as u8;
            let ah = ((self.rf.ax >> 8) & 0xFF) as u8;
            let (new_al, new_ah, af_cf) = aaa(al, ah, self.rf.flag_af);
            self.rf.ax = ((new_ah as u16) << 8) | (new_al as u16);
            self.rf.flag_af = af_cf;
            self.rf.flag_cf = af_cf;
            return;
        }
        if op == 0x3F { // AAS
            let al = (self.rf.ax & 0xFF) as u8;
            let ah = ((self.rf.ax >> 8) & 0xFF) as u8;
            let (new_al, new_ah, af_cf) = aas(al, ah, self.rf.flag_af);
            self.rf.ax = ((new_ah as u16) << 8) | (new_al as u16);
            self.rf.flag_af = af_cf;
            self.rf.flag_cf = af_cf;
            return;
        }
        if op == 0xD4 { // AAM
            let base = self.fetch8();
            let al = (self.rf.ax & 0xFF) as u8;
            let (new_ah, new_al) = aam(al, base);
            self.rf.ax = ((new_ah as u16) << 8) | (new_al as u16);
            self.set_szp_byte(new_al);
            return;
        }
        if op == 0xD5 { // AAD
            let base = self.fetch8();
            let ah = ((self.rf.ax >> 8) & 0xFF) as u8;
            let al = (self.rf.ax & 0xFF) as u8;
            let new_al = aad(ah, al, base);
            self.rf.ax = new_al as u16;
            self.set_szp_byte(new_al);
            return;
        }

        // ── Shifts / rotates (D0/D1/D2/D3) ───────────────────────────────────

        if matches!(op, 0xD0..=0xD3) {
            let word = (op & 1) != 0;
            let count: u8 = if op < 0xD2 { 1 } else { (self.rf.cx & 0xFF) as u8 };
            let modrm = self.fetch8();
            let ext = (modrm >> 3) & 7;
            let rm = modrm & 7;
            let mod_ = (modrm >> 6) & 3;
            let (_, _, _, ea) = self.decode_modrm(modrm, word);
            let seg = self.effective_seg(rm, mod_, seg_override);
            let a = self.read_rm(mod_, rm, seg, ea, word);
            let width: u8 = if word { 16 } else { 8 };
            let mask: u16 = if word { WORD_MASK } else { BYTE_MASK };

            let (result, _) = match ext {
                0 => { // ROL
                    let (r, cf) = rol(a, count, width);
                    let msb = ((r >> (width - 1)) & 1) as u8;
                    self.rf.flag_cf = cf;
                    if count == 1 { self.rf.flag_of = cf ^ msb; }
                    (r, cf)
                }
                1 => { // ROR
                    let (r, cf) = ror(a, count, width);
                    let msb = ((r >> (width - 1)) & 1) as u8;
                    let msb2 = ((r >> (width - 2)) & 1) as u8;
                    self.rf.flag_cf = cf;
                    if count == 1 { self.rf.flag_of = msb ^ msb2; }
                    (r, cf)
                }
                2 => { // RCL
                    let cf_old = self.rf.flag_cf;
                    let (r, cf) = rcl(a, count, width, cf_old);
                    let msb = ((r >> (width - 1)) & 1) as u8;
                    self.rf.flag_cf = cf;
                    if count == 1 { self.rf.flag_of = cf ^ msb; }
                    (r, cf)
                }
                3 => { // RCR
                    let cf_old = self.rf.flag_cf;
                    let (r, cf) = rcr(a, count, width, cf_old);
                    let msb = ((r >> (width - 1)) & 1) as u8;
                    let msb2 = ((r >> (width - 2)) & 1) as u8;
                    self.rf.flag_cf = cf;
                    if count == 1 { self.rf.flag_of = msb ^ msb2; }
                    (r, cf)
                }
                4 | 6 => { // SHL/SAL
                    let (r, cf) = shl(a, count, width);
                    let msb = ((r >> (width - 1)) & 1) as u8;
                    self.rf.flag_cf = cf;
                    if count == 1 { self.rf.flag_of = cf ^ msb; }
                    // SF/ZF/PF set below
                    (r, cf)
                }
                5 => { // SHR
                    let msb_orig = ((a >> (width - 1)) & 1) as u8;
                    let (r, cf) = shr(a, count, width);
                    self.rf.flag_cf = cf;
                    if count == 1 { self.rf.flag_of = msb_orig; }
                    (r, cf)
                }
                _ => { // SAR (7)
                    let (r, cf) = sar(a, count, width);
                    self.rf.flag_cf = cf;
                    if count == 1 { self.rf.flag_of = 0; }
                    (r, cf)
                }
            };

            // SF/ZF/PF for shift operations (4/5/6/7)
            if ext >= 4 {
                let r_masked = result & mask;
                self.rf.flag_sf = ((r_masked >> (width - 1)) & 1) as u8;
                self.rf.flag_zf = if r_masked == 0 { 1 } else { 0 };
                let bits = int_to_bits8((r_masked & 0xFF) as u8);
                self.rf.flag_pf = compute_parity(&bits);
            }

            self.write_rm(mod_, rm, seg, ea, result & mask, word);
            return;
        }

        // ── Control flow ──────────────────────────────────────────────────────

        if op == 0xEB { // JMP short
            let disp = self.fetch_s8() as u16;
            let (new_ip, _, _) = add_16bit(self.rf.ip, disp, 0);
            self.rf.ip = new_ip;
            return;
        }
        if op == 0xE9 { // JMP near
            let disp = self.fetch_s16() as u16;
            let (new_ip, _, _) = add_16bit(self.rf.ip, disp, 0);
            self.rf.ip = new_ip;
            return;
        }
        if op == 0xEA { // JMP far
            let new_ip = self.fetch16();
            let new_cs = self.fetch16();
            self.rf.ip = new_ip;
            self.rf.cs = new_cs;
            return;
        }
        if op == 0xE8 { // CALL near
            let disp = self.fetch_s16() as u16;
            let ip = self.rf.ip;
            self.push16(ip);
            let (new_ip, _, _) = add_16bit(self.rf.ip, disp, 0);
            self.rf.ip = new_ip;
            return;
        }
        if op == 0x9A { // CALL far
            let new_ip = self.fetch16();
            let new_cs = self.fetch16();
            let cs = self.rf.cs; self.push16(cs);
            let ip = self.rf.ip; self.push16(ip);
            self.rf.ip = new_ip;
            self.rf.cs = new_cs;
            return;
        }
        if op == 0xC3 { // RET near
            self.rf.ip = self.pop16();
            return;
        }
        if op == 0xC2 { // RET n
            let n = self.fetch16();
            self.rf.ip = self.pop16();
            let (new_sp, _, _) = add_16bit(self.rf.sp, n, 0);
            self.rf.sp = new_sp;
            return;
        }
        if op == 0xCB { // RETF
            self.rf.ip = self.pop16();
            self.rf.cs = self.pop16();
            return;
        }
        if op == 0xCA { // RETF n
            let n = self.fetch16();
            self.rf.ip = self.pop16();
            self.rf.cs = self.pop16();
            let (new_sp, _, _) = add_16bit(self.rf.sp, n, 0);
            self.rf.sp = new_sp;
            return;
        }

        // Jcc (70–7F)
        if (0x70..=0x7F).contains(&op) {
            let disp = self.fetch_s8() as u16;
            if self.eval_cond(op - 0x70) {
                let (new_ip, _, _) = add_16bit(self.rf.ip, disp, 0);
                self.rf.ip = new_ip;
            }
            return;
        }

        // LOOP / LOOPZ / LOOPNZ / JCXZ (E0–E3)
        if (0xE0..=0xE3).contains(&op) {
            let disp = self.fetch_s8() as u16;
            if op != 0xE3 {
                let (new_cx, _, _) = add_16bit(self.rf.cx, invert_16bit(1), 1);
                self.rf.cx = new_cx;
            }
            let zf = self.rf.flag_zf != 0;
            let cx = self.rf.cx;
            let taken = match op {
                0xE2 => cx != 0,
                0xE1 => cx != 0 && zf,
                0xE0 => cx != 0 && !zf,
                _    => cx == 0, // JCXZ
            };
            if taken {
                let (new_ip, _, _) = add_16bit(self.rf.ip, disp, 0);
                self.rf.ip = new_ip;
            }
            return;
        }

        // ── Interrupts ─────────────────────────────────────────────────────────

        if op == 0xCC { // INT 3
            self.trigger_interrupt(3);
            self.halted = true;
            return;
        }
        if op == 0xCD { // INT n
            let n = self.fetch8();
            self.trigger_interrupt(n);
            self.halted = true;
            return;
        }
        if op == 0xCE { // INTO
            self.trigger_interrupt(4);
            self.halted = true;
            return;
        }
        if op == 0xCF { // IRET
            self.rf.ip = self.pop16();
            self.rf.cs = self.pop16();
            let f = self.pop16();
            self.rf.unpack_flags(f);
            return;
        }

        // ── String operations ──────────────────────────────────────────────────

        if matches!(op, 0xA4 | 0xA5 | 0xA6 | 0xA7 | 0xAE | 0xAF | 0xAC | 0xAD | 0xAA | 0xAB) {
            let word = (op & 1) != 0;
            let step = self.str_step(word);
            let seg_src = seg_override.map_or(self.rf.ds, |s| self.rf.read_seg(s));
            match op {
                0xAC | 0xAD => self.exec_lods(word, step, seg_src, rep_prefix),
                0xAA | 0xAB => self.exec_stos(word, step, rep_prefix),
                0xA4 | 0xA5 => self.exec_movs(word, step, seg_src, rep_prefix),
                0xA6 | 0xA7 => self.exec_cmps(word, step, seg_src, rep_prefix),
                _ => self.exec_scas(word, step, rep_prefix),
            }
            return;
        }

        // ── Miscellaneous ──────────────────────────────────────────────────────

        if op == 0xF4 { self.halted = true; return; } // HLT
        if op == 0xF8 { self.rf.flag_cf = 0; return; } // CLC
        if op == 0xF9 { self.rf.flag_cf = 1; return; } // STC
        if op == 0xF5 { self.rf.flag_cf = 1 - self.rf.flag_cf; return; } // CMC
        if op == 0xFC { self.rf.flag_df = 0; return; } // CLD
        if op == 0xFD { self.rf.flag_df = 1; return; } // STD
        if op == 0xFA { self.rf.flag_if = 0; return; } // CLI
        if op == 0xFB { self.rf.flag_if = 1; return; } // STI

        // ── I/O ports ──────────────────────────────────────────────────────────

        if op == 0xE4 { // IN AL, imm8
            let port = self.fetch8() as usize;
            self.rf.ax = (self.rf.ax & 0xFF00) | (self.input_ports[port] as u16);
            return;
        }
        if op == 0xE5 { // IN AX, imm8
            let port = self.fetch8() as usize;
            let lo = self.input_ports[port] as u16;
            let hi = self.input_ports[(port + 1) & 0xFF] as u16;
            self.rf.ax = lo | (hi << 8);
            return;
        }
        if op == 0xEC { // IN AL, DX
            let port = (self.rf.dx & 0xFF) as usize;
            self.rf.ax = (self.rf.ax & 0xFF00) | (self.input_ports[port] as u16);
            return;
        }
        if op == 0xED { // IN AX, DX
            let port = (self.rf.dx & 0xFF) as usize;
            let lo = self.input_ports[port] as u16;
            let hi = self.input_ports[(port + 1) & 0xFF] as u16;
            self.rf.ax = lo | (hi << 8);
            return;
        }
        if op == 0xE6 { // OUT imm8, AL
            let port = self.fetch8() as usize;
            self.output_ports[port] = (self.rf.ax & 0xFF) as u8;
            return;
        }
        if op == 0xE7 { // OUT imm8, AX
            let port = self.fetch8() as usize;
            self.output_ports[port] = (self.rf.ax & 0xFF) as u8;
            self.output_ports[(port + 1) & 0xFF] = ((self.rf.ax >> 8) & 0xFF) as u8;
            return;
        }
        if op == 0xEE { // OUT DX, AL
            let port = (self.rf.dx & 0xFF) as usize;
            self.output_ports[port] = (self.rf.ax & 0xFF) as u8;
            return;
        }
        if op == 0xEF { // OUT DX, AX
            let port = (self.rf.dx & 0xFF) as usize;
            self.output_ports[port] = (self.rf.ax & 0xFF) as u8;
            self.output_ports[(port + 1) & 0xFF] = ((self.rf.ax >> 8) & 0xFF) as u8;
            return;
        }

        if op == 0x9B { return; } // WAIT — no-op

        // Unknown opcode — halt
        self.halted = true;
    }
}

impl Default for Cpu8086 {
    fn default() -> Self { Self::new() }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn run(program: &[u8]) -> Cpu8086 {
        let mut cpu = Cpu8086::new();
        cpu.execute(program, 10_000);
        cpu
    }

    #[test]
    fn mov_ax_imm16_hlt() {
        // MOV AX, 0x1234; HLT
        let cpu = run(&[0xB8, 0x34, 0x12, 0xF4]);
        assert_eq!(cpu.rf.ax, 0x1234);
        assert!(cpu.halted);
    }

    #[test]
    fn add_ax_imm16() {
        // MOV AX, 5; ADD AX, 3; HLT
        let cpu = run(&[0xB8, 5, 0, 0x05, 3, 0, 0xF4]);
        assert_eq!(cpu.rf.ax, 8);
        assert_eq!(cpu.rf.flag_cf, 0);
        assert_eq!(cpu.rf.flag_zf, 0);
    }

    #[test]
    fn sub_sets_flags() {
        // MOV AX, 1; SUB AX, 1; HLT
        let cpu = run(&[0xB8, 1, 0, 0x2D, 1, 0, 0xF4]);
        assert_eq!(cpu.rf.ax, 0);
        assert_eq!(cpu.rf.flag_zf, 1);
        assert_eq!(cpu.rf.flag_cf, 0);
    }

    #[test]
    fn inc_dec_preserve_cf() {
        // STC (CF=1); MOV AX, 1; INC AX; HLT
        let cpu = run(&[0xF9, 0xB8, 1, 0, 0x40, 0xF4]);
        assert_eq!(cpu.rf.ax, 2);
        assert_eq!(cpu.rf.flag_cf, 1); // CF preserved by INC
    }

    #[test]
    fn push_pop_roundtrip() {
        // MOV AX, 0xABCD; PUSH AX; MOV AX, 0; POP BX; HLT
        let cpu = run(&[
            0xB8, 0xCD, 0xAB, // MOV AX, 0xABCD
            0x50,             // PUSH AX
            0xB8, 0, 0,       // MOV AX, 0
            0x5B,             // POP BX
            0xF4,
        ]);
        assert_eq!(cpu.rf.bx, 0xABCD);
    }

    #[test]
    fn jmp_short() {
        // JMP +1; MOV AX, 0xFF; MOV AX, 0x42; HLT
        let cpu = run(&[
            0xEB, 0x03,       // JMP +3 (skip next 3 bytes)
            0xB8, 0xFF, 0x00, // MOV AX, 0xFF  ← skipped
            0xB8, 0x42, 0x00, // MOV AX, 0x42
            0xF4,
        ]);
        assert_eq!(cpu.rf.ax, 0x42);
    }

    #[test]
    fn jnz_loop() {
        // MOV CX, 3; DEC CX; JNZ -2; HLT  → CX ends at 0
        let cpu = run(&[
            0xB9, 3, 0,  // MOV CX, 3
            0x49,        // DEC CX (no CF change)
            0x75, 0xFD,  // JNZ -3 (back to DEC CX)
            0xF4,
        ]);
        assert_eq!(cpu.rf.cx, 0);
    }

    #[test]
    fn shl_basic() {
        // MOV AL, 1; SHL AL, 1; HLT
        let cpu = run(&[
            0xB0, 0x01, // MOV AL, 1
            0xD0, 0xE0, // SHL AL, 1 (D0 /4)
            0xF4,
        ]);
        assert_eq!(cpu.rf.ax & 0xFF, 2);
        assert_eq!(cpu.rf.flag_cf, 0);
    }

    #[test]
    fn and_clears_cf_of() {
        // MOV AX, 0x8001; ADD AX, 0x7FFF (sets CF/OF); AND AX, 0xFF; HLT
        let cpu = run(&[
            0xB8, 0x01, 0x80, // MOV AX, 0x8001
            0x05, 0xFF, 0x7F, // ADD AX, 0x7FFF
            0x25, 0xFF, 0x00, // AND AX, 0xFF
            0xF4,
        ]);
        assert_eq!(cpu.rf.flag_cf, 0); // AND clears CF
        assert_eq!(cpu.rf.flag_of, 0); // AND clears OF
    }

    #[test]
    fn mul8_basic() {
        // MOV AL, 10; MOV BL, 5; MUL BL; HLT
        let cpu = run(&[
            0xB0, 10,    // MOV AL, 10
            0xB3, 5,     // MOV BL, 5
            0xF6, 0xE3,  // MUL BL (F6 /4)
            0xF4,
        ]);
        assert_eq!(cpu.rf.ax, 50);
    }

    #[test]
    fn string_movs() {
        // Set up state manually (execute() would reset it).
        // DS:SI = 0:0x200 → 0xAA; ES:DI = 0:0x100; MOVSB copies one byte.
        let mut cpu = Cpu8086::new();
        cpu.mem[0x200] = 0xAA; // source byte
        cpu.mem[0] = 0xA4;     // MOVSB
        cpu.rf.si = 0x200;
        cpu.rf.di = 0x100;
        cpu.step(); // execute MOVSB
        assert_eq!(cpu.mem[0x100], 0xAA);
    }

    #[test]
    fn physical_address_wrapping() {
        // Verify segment addressing: CS=0x0100, IP starts at 0
        // Physical 0x0100 × 16 + 0 = 0x1000
        let mut cpu = Cpu8086::new();
        cpu.rf.cs = 0x0100;
        cpu.rf.ip = 0;
        cpu.mem[0x1000] = 0xF4; // HLT at physical 0x1000
        cpu.halted = false;
        cpu.step();
        assert!(cpu.halted);
    }
}
