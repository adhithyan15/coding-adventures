//! Register file for the MOS 6502 gate-level simulator.
//!
//! # The 6502 Register Architecture
//!
//! The 6502 has a famously tiny register file — a deliberate design choice
//! (vs Motorola 6800, which the team previously designed). Zero-page memory
//! serves as "cheap registers" instead.
//!
//! Active registers:
//! - `A`  — 8-bit accumulator
//! - `X`  — 8-bit index register
//! - `Y`  — 8-bit index register
//! - `S`  — 8-bit stack pointer (effective address = 0x0100 + S)
//! - `PC` — 16-bit program counter
//!
//! Processor status (P register — 7 separate flag flip-flops):
//! ```text
//! Bit 7  N  Negative
//! Bit 6  V  Overflow
//! Bit 5  -  (always 1 — hardwired Vcc, no flip-flop)
//! Bit 4  B  Break (only set in pushed copy for BRK/PHP)
//! Bit 3  D  Decimal
//! Bit 2  I  Interrupt disable
//! Bit 1  Z  Zero
//! Bit 0  C  Carry
//! ```
//!
//! # D flip-flop model
//!
//! Each register is stored as a plain integer (the Q output of the flip-flop
//! array). Write operations model the clock edge latching new data in.

use crate::bits::{add_16bit, add_8bit};
use crate::state::{DffMemory, StateRegister};

/// 8-bit register modeled as 8 D flip-flops.
///
/// Power-on value: 0x00. Stack pointer (`S`) is initialized to 0xFD.
#[derive(Debug, Clone)]
pub struct Register8 {
    state: StateRegister,
}

impl Register8 {
    pub fn new(initial: u8) -> Self {
        let mut state = StateRegister::new(8);
        state.write(u16::from(initial));
        Self { state }
    }

    /// Clock a new value into the register (rising clock edge).
    pub fn write(&mut self, value: u8) {
        self.state.write(u16::from(value));
    }

    /// Read the current Q output.
    pub fn read(&self) -> u8 {
        self.state.read() as u8
    }
}

/// 16-bit register modeled as 16 D flip-flops.
///
/// Used for the program counter (PC).
#[derive(Debug, Clone)]
pub struct Register16 {
    state: StateRegister,
}

impl Register16 {
    pub fn new(initial: u16) -> Self {
        let mut state = StateRegister::new(16);
        state.write(initial);
        Self { state }
    }

    /// Clock a new 16-bit value into the register.
    pub fn write(&mut self, value: u16) {
        self.state.write(value);
    }

    /// Read the current value.
    pub fn read(&self) -> u16 {
        self.state.read()
    }

    /// Increment by `amount` via the 16-bit ripple-carry adder.
    ///
    /// Used for PC advancement during instruction fetch.
    pub fn inc(&mut self, amount: u16) {
        let (new_val, _carry) = add_16bit(self.read(), amount, 0);
        self.write(new_val);
    }
}

/// The 6502 processor status register — 7 individual flag flip-flops.
///
/// Bit 5 (always 1) has no physical flip-flop on the NMOS 6502; it is
/// hardwired to Vcc and always reads as 1 in the packed P byte.
#[derive(Debug, Clone)]
pub struct FlagRegister {
    pub n: u8, // Negative
    pub v: u8, // Overflow
    pub b: u8, // Break (set in pushed copy for BRK/PHP)
    pub d: u8, // Decimal
    pub i: u8, // Interrupt disable
    pub z: u8, // Zero
    pub c: u8, // Carry
    state: StateRegister,
}

impl FlagRegister {
    /// Power-on state: I=1, all others 0.
    pub fn new() -> Self {
        let mut flags = Self {
            n: 0,
            v: 0,
            b: 0,
            d: 0,
            i: 1,
            z: 0,
            c: 0,
            state: StateRegister::new(7),
        };
        flags.clock();
        flags
    }

    /// Pack all flags into the P status byte.
    ///
    /// Bit 5 is hardwired to 1 (no flip-flop on real NMOS 6502).
    ///
    /// `with_b` overrides the B bit in the packed result:
    /// - PHP/BRK push P with B=1
    /// - IRQ/NMI push P with B=0
    /// - Pass `None` to use the stored B value.
    pub fn pack(&self, with_b: Option<u8>) -> u8 {
        let b_bit = with_b.unwrap_or(self.b) & 1;
        (self.n << 7)
            | (self.v << 6)
            | 0x20
            | (b_bit << 4)
            | (self.d << 3)
            | (self.i << 2)
            | (self.z << 1)
            | self.c
    }

    /// Unpack a P byte into individual flag flip-flops.
    ///
    /// Used by PLP and RTI to restore processor status from stack.
    /// Bit 5 is ignored (no flip-flop to set).
    pub fn unpack(&mut self, p: u8) {
        self.n = (p >> 7) & 1;
        self.v = (p >> 6) & 1;
        self.b = (p >> 4) & 1;
        self.d = (p >> 3) & 1;
        self.i = (p >> 2) & 1;
        self.z = (p >> 1) & 1;
        self.c = p & 1;
    }

    pub(crate) fn load_wires(&mut self) {
        let value = self.state.read() as u8;
        self.n = value & 1;
        self.v = (value >> 1) & 1;
        self.b = (value >> 2) & 1;
        self.d = (value >> 3) & 1;
        self.i = (value >> 4) & 1;
        self.z = (value >> 5) & 1;
        self.c = (value >> 6) & 1;
    }

    pub(crate) fn clock(&mut self) {
        let value = self.n
            | (self.v << 1)
            | (self.b << 2)
            | (self.d << 3)
            | (self.i << 4)
            | (self.z << 5)
            | (self.c << 6);
        self.state.write(u16::from(value));
    }

    pub fn reset(&mut self) {
        self.n = 0;
        self.v = 0;
        self.b = 0;
        self.d = 0;
        self.i = 1; // I=1 at power-on
        self.z = 0;
        self.c = 0;
        self.clock();
    }
}

impl Default for FlagRegister {
    fn default() -> Self {
        Self::new()
    }
}

/// Complete 6502 register file: A, X, Y, S, PC, and flags.
#[derive(Debug, Clone)]
pub struct RegisterFile6502 {
    pub a: Register8,   // Accumulator
    pub x: Register8,   // Index X
    pub y: Register8,   // Index Y
    pub s: Register8,   // Stack pointer (0x0100 page)
    pub pc: Register16, // Program counter
    pub flags: FlagRegister,
}

impl RegisterFile6502 {
    /// Initialize all registers to power-on state.
    ///
    /// A=X=Y=0, S=0xFD (standard 6502 power-on), PC=0, flags: I=1 rest 0.
    pub fn new() -> Self {
        Self {
            a: Register8::new(0),
            x: Register8::new(0),
            y: Register8::new(0),
            s: Register8::new(0xFD), // 6502 power-on stack pointer
            pc: Register16::new(0),
            flags: FlagRegister::new(),
        }
    }

    /// Reset all registers to power-on state.
    pub fn reset(&mut self) {
        self.a.write(0);
        self.x.write(0);
        self.y.write(0);
        self.s.write(0xFD);
        self.pc.write(0);
        self.flags.reset();
    }

    pub(crate) fn stack_push(&mut self, memory: &mut DffMemory, value: u8) {
        let s = self.s.read();
        memory.write(0x0100 | s as usize, value);
        let (new_s, _carry) = add_8bit(s, 0xFF, 0);
        self.s.write(new_s);
    }

    pub(crate) fn stack_pull(&mut self, memory: &DffMemory) -> u8 {
        let s = self.s.read();
        let (new_s, _carry) = add_8bit(s, 1, 0);
        self.s.write(new_s);
        memory.read(0x0100 | new_s as usize)
    }
}

impl Default for RegisterFile6502 {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register8_read_write() {
        let mut r = Register8::new(0);
        r.write(0xAB);
        assert_eq!(r.read(), 0xAB);
        r.write(0xFF);
        assert_eq!(r.read(), 0xFF);
    }

    #[test]
    fn register16_read_write() {
        let mut r = Register16::new(0);
        r.write(0x1234);
        assert_eq!(r.read(), 0x1234);
    }

    #[test]
    fn register16_inc() {
        let mut r = Register16::new(0xFFFE);
        r.inc(1);
        assert_eq!(r.read(), 0xFFFF);
        r.inc(1);
        assert_eq!(r.read(), 0x0000); // wraps
    }

    #[test]
    fn flag_register_power_on() {
        let f = FlagRegister::new();
        assert_eq!(f.i, 1);
        assert_eq!(f.n, 0);
        assert_eq!(f.z, 0);
        assert_eq!(f.c, 0);
    }

    #[test]
    fn flag_register_pack_bit5_always_set() {
        let f = FlagRegister::new();
        let p = f.pack(None);
        assert_ne!(p & 0x20, 0); // bit 5 always 1
    }

    #[test]
    fn flag_register_pack_with_b_override() {
        let mut f = FlagRegister::new();
        f.b = 0;
        assert_ne!(f.pack(Some(1)) & 0x10, 0); // B forced to 1
        assert_eq!(f.pack(Some(0)) & 0x10, 0); // B forced to 0
    }

    #[test]
    fn flag_register_pack_unpack_roundtrip() {
        let mut f = FlagRegister::new();
        f.n = 1;
        f.v = 0;
        f.b = 1;
        f.d = 0;
        f.i = 1;
        f.z = 1;
        f.c = 0;
        let p = f.pack(None);
        let mut f2 = FlagRegister::new();
        f2.unpack(p);
        assert_eq!(f2.n, 1);
        assert_eq!(f2.v, 0);
        assert_eq!(f2.i, 1);
        assert_eq!(f2.z, 1);
        assert_eq!(f2.c, 0);
    }

    #[test]
    fn stack_push_pull() {
        let mut rf = RegisterFile6502::new();
        let mut mem = DffMemory::new();
        rf.stack_push(&mut mem, 0xAB);
        let v = rf.stack_pull(&mem);
        assert_eq!(v, 0xAB);
    }

    #[test]
    fn stack_sp_decrements_on_push() {
        let mut rf = RegisterFile6502::new();
        let mut mem = DffMemory::new();
        let s_before = rf.s.read();
        rf.stack_push(&mut mem, 0x42);
        assert_eq!(rf.s.read(), s_before.wrapping_sub(1));
    }

    #[test]
    fn stack_sp_increments_on_pull() {
        let mut rf = RegisterFile6502::new();
        let mut mem = DffMemory::new();
        rf.stack_push(&mut mem, 0x42);
        let s_after_push = rf.s.read();
        rf.stack_pull(&mem);
        assert_eq!(rf.s.read(), s_after_push.wrapping_add(1));
    }
}
