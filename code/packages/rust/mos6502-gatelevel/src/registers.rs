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

/// 8-bit register modeled as 8 D flip-flops.
///
/// Power-on value: 0x00. Stack pointer (`S`) is initialized to 0xFD.
#[derive(Debug, Clone)]
pub struct Register8 {
    value: u8,
}

impl Register8 {
    pub fn new(initial: u8) -> Self {
        Self { value: initial }
    }

    /// Clock a new value into the register (rising clock edge).
    pub fn write(&mut self, value: u8) {
        self.value = value;
    }

    /// Read the current Q output.
    pub fn read(&self) -> u8 {
        self.value
    }
}

/// 16-bit register modeled as 16 D flip-flops.
///
/// Used for the program counter (PC).
#[derive(Debug, Clone)]
pub struct Register16 {
    value: u16,
}

impl Register16 {
    pub fn new(initial: u16) -> Self {
        Self { value: initial }
    }

    /// Clock a new 16-bit value into the register.
    pub fn write(&mut self, value: u16) {
        self.value = value;
    }

    /// Read the current value.
    pub fn read(&self) -> u16 {
        self.value
    }

    /// Increment by `amount` via the 16-bit ripple-carry adder.
    ///
    /// Used for PC advancement during instruction fetch.
    pub fn inc(&mut self, amount: u16) {
        let (new_val, _carry) = add_16bit(self.value, amount, 0);
        self.value = new_val;
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
}

impl FlagRegister {
    /// Power-on state: I=1, all others 0.
    pub fn new() -> Self {
        Self { n: 0, v: 0, b: 0, d: 0, i: 1, z: 0, c: 0 }
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
        (self.n << 7) | (self.v << 6) | 0x20 | (b_bit << 4)
            | (self.d << 3) | (self.i << 2) | (self.z << 1) | self.c
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

    pub fn reset(&mut self) {
        self.n = 0; self.v = 0; self.b = 0; self.d = 0;
        self.i = 1; // I=1 at power-on
        self.z = 0; self.c = 0;
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

    /// Push a byte to the stack (0x0100 page, pre-decrement S).
    ///
    /// S is decremented via the 8-bit adder (S + 0xFF = S - 1 mod 256).
    pub fn stack_push(&mut self, memory: &mut [u8; 65536], value: u8) {
        let s = self.s.read();
        memory[0x0100 | s as usize] = value;
        let (new_s, _carry) = add_8bit(s, 0xFF, 0); // S - 1 via adder
        self.s.write(new_s);
    }

    /// Pull a byte from the stack (post-increment S).
    pub fn stack_pull(&mut self, memory: &[u8; 65536]) -> u8 {
        let s = self.s.read();
        let (new_s, _carry) = add_8bit(s, 1, 0); // S + 1
        self.s.write(new_s);
        memory[0x0100 | new_s as usize]
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
        assert_eq!(f.pack(Some(0)) & 0x10, 0);  // B forced to 0
    }

    #[test]
    fn flag_register_pack_unpack_roundtrip() {
        let mut f = FlagRegister::new();
        f.n = 1; f.v = 0; f.b = 1; f.d = 0; f.i = 1; f.z = 1; f.c = 0;
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
        let mut mem = [0u8; 65536];
        rf.stack_push(&mut mem, 0xAB);
        let v = rf.stack_pull(&mem);
        assert_eq!(v, 0xAB);
    }

    #[test]
    fn stack_sp_decrements_on_push() {
        let mut rf = RegisterFile6502::new();
        let mut mem = [0u8; 65536];
        let s_before = rf.s.read();
        rf.stack_push(&mut mem, 0x42);
        assert_eq!(rf.s.read(), s_before.wrapping_sub(1));
    }

    #[test]
    fn stack_sp_increments_on_pull() {
        let mut rf = RegisterFile6502::new();
        let mut mem = [0u8; 65536];
        rf.stack_push(&mut mem, 0x42);
        let s_after_push = rf.s.read();
        rf.stack_pull(&mem);
        assert_eq!(rf.s.read(), s_after_push.wrapping_add(1));
    }
}
