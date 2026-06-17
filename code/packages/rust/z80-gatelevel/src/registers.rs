//! Register file for the Zilog Z80 gate-level simulator.
//!
//! # Z80 Register Architecture
//!
//! The Z80 has a significantly richer register set than the Intel 8080:
//!
//! **Main bank** (active at any time):
//! - `A` — 8-bit accumulator
//! - `F` — 8-bit flags register (S Z _ H _ PV N C)
//! - `B`, `C` — 8-bit general purpose; BC = 16-bit pair
//! - `D`, `E` — 8-bit general purpose; DE = 16-bit pair
//! - `H`, `L` — 8-bit general purpose; HL = 16-bit pointer
//!
//! **Alternate bank** (shadow — swapped via EX AF,AF' and EXX):
//! - `A'`, `F'`, `B'`, `C'`, `D'`, `E'`, `H'`, `L'`
//! - Only one bank is active at any time.
//! - The real Z80 has two complete 8-register banks = 128 flip-flops.
//!
//! **Index registers** (16-bit):
//! - `IX`, `IY` — with signed 8-bit displacement for indirect addressing
//!
//! **Special registers**:
//! - `SP` — 16-bit stack pointer (pre-decrement on PUSH, post-increment on POP)
//! - `PC` — 16-bit program counter
//! - `I` — 8-bit interrupt vector base (used in IM 2)
//! - `R` — 8-bit memory refresh counter (low 7 bits auto-increment each fetch)
//!
//! **Interrupt state**:
//! - `IFF1`, `IFF2` — interrupt enable flip-flops
//! - `IM` — interrupt mode (0, 1, or 2)
//!
//! # F register layout
//!
//! ```text
//! Bit 7  S   Sign
//! Bit 6  Z   Zero
//! Bit 5  Y   (undocumented — we set to 0)
//! Bit 4  H   Half-carry
//! Bit 3  X   (undocumented — we set to 0)
//! Bit 2  PV  Parity / Overflow
//! Bit 1  N   Subtract
//! Bit 0  C   Carry
//! ```
//!
//! # 3-bit register codes
//!
//! Z80 instructions use 3-bit fields to select 8-bit registers:
//! ```text
//! 000 = B    001 = C    010 = D    011 = E
//! 100 = H    101 = L    110 = (HL) pseudo    111 = A
//! ```
//! Code 6 is the `(HL)` pseudo-register (memory access, not a register).

/// 8-bit register codes (3-bit Z80 field).
pub const REG_B: usize = 0;
pub const REG_C: usize = 1;
pub const REG_D: usize = 2;
pub const REG_E: usize = 3;
pub const REG_H: usize = 4;
pub const REG_L: usize = 5;
pub const REG_MEM: usize = 6;  // (HL) pseudo-register
pub const REG_A: usize = 7;

/// Pack Z80 flag bits into the F register byte.
///
/// ```text
/// F: S Z 0 H 0 PV N C
/// ```
/// Bits 5 (Y) and 3 (X) are undocumented; we keep them 0.
#[inline]
pub fn pack_f(s: u8, z: u8, h: u8, pv: u8, n: u8, c: u8) -> u8 {
    ((s & 1) << 7)
        | ((z & 1) << 6)
        | ((h & 1) << 4)
        | ((pv & 1) << 2)
        | ((n & 1) << 1)
        | (c & 1)
}

/// Unpack an F register byte into individual flag bits.
/// Returns (s, z, h, pv, n, c).
#[inline]
pub fn unpack_f(byte: u8) -> (u8, u8, u8, u8, u8, u8) {
    let s  = (byte >> 7) & 1;
    let z  = (byte >> 6) & 1;
    let h  = (byte >> 4) & 1;
    let pv = (byte >> 2) & 1;
    let n  = (byte >> 1) & 1;
    let c  = byte & 1;
    (s, z, h, pv, n, c)
}

/// Z80 register file: main bank + alternate bank + index registers.
///
/// Main bank: `regs[0..8]` (indices match 3-bit codes; index 6 unused).
/// Alternate bank: `alt[0..8]` (same layout).
/// F and F' stored separately as packed bytes for flag operations.
#[derive(Debug, Clone)]
pub struct RegisterFile {
    /// Main registers: B=0, C=1, D=2, E=3, H=4, L=5, _=6, A=7
    regs: [u8; 8],
    /// Alternate registers (same layout)
    alt: [u8; 8],
    /// Flags register (packed byte)
    f: u8,
    /// Alternate flags register
    f_prime: u8,
    /// Index register X
    pub ix: u16,
    /// Index register Y
    pub iy: u16,
}

impl RegisterFile {
    /// Initialize all registers to zero.
    pub fn new() -> Self {
        Self {
            regs: [0u8; 8],
            alt: [0u8; 8],
            f: 0,
            f_prime: 0,
            ix: 0,
            iy: 0,
        }
    }

    /// Reset all registers to zero (power-on state).
    pub fn reset(&mut self) {
        self.regs = [0u8; 8];
        self.alt = [0u8; 8];
        self.f = 0;
        self.f_prime = 0;
        self.ix = 0;
        self.iy = 0;
    }

    // ── 8-bit register access ────────────────────────────────────────────────

    /// Read an 8-bit register by 3-bit code.
    ///
    /// Panics if code is REG_MEM (6) — that requires a memory access.
    #[inline]
    pub fn read8(&self, reg_id: usize) -> u8 {
        debug_assert_ne!(reg_id, REG_MEM, "REG_MEM is a pseudo-register");
        self.regs[reg_id]
    }

    /// Write an 8-bit value to a register by 3-bit code.
    #[inline]
    pub fn write8(&mut self, reg_id: usize, value: u8) {
        debug_assert_ne!(reg_id, REG_MEM, "REG_MEM is a pseudo-register");
        self.regs[reg_id] = value;
    }

    // ── 16-bit register pair access ──────────────────────────────────────────

    /// Read a 16-bit register pair (0=BC, 1=DE, 2=HL, 3=SP via sp arg).
    pub fn read16_pair(&self, pair_id: u8, sp: u16) -> u16 {
        match pair_id {
            0 => ((self.regs[REG_B] as u16) << 8) | (self.regs[REG_C] as u16),
            1 => ((self.regs[REG_D] as u16) << 8) | (self.regs[REG_E] as u16),
            2 => ((self.regs[REG_H] as u16) << 8) | (self.regs[REG_L] as u16),
            3 => sp,
            _ => panic!("invalid pair_id {}", pair_id),
        }
    }

    /// Write a 16-bit value to a register pair.
    /// For pair_id=3 (SP), the caller must update sp separately.
    pub fn write16_pair(&mut self, pair_id: u8, value: u16) -> Option<u16> {
        let hi = ((value >> 8) & 0xFF) as u8;
        let lo = (value & 0xFF) as u8;
        match pair_id {
            0 => { self.regs[REG_B] = hi; self.regs[REG_C] = lo; None }
            1 => { self.regs[REG_D] = hi; self.regs[REG_E] = lo; None }
            2 => { self.regs[REG_H] = hi; self.regs[REG_L] = lo; None }
            3 => Some(value), // SP: caller sets it
            _ => panic!("invalid pair_id {}", pair_id),
        }
    }

    // ── Flags access ─────────────────────────────────────────────────────────

    /// Read all flags from the F register.
    /// Returns (s, z, h, pv, n, c).
    #[inline]
    pub fn read_flags(&self) -> (u8, u8, u8, u8, u8, u8) {
        unpack_f(self.f)
    }

    /// Write all flags to the F register.
    #[inline]
    pub fn write_flags(&mut self, s: u8, z: u8, h: u8, pv: u8, n: u8, c: u8) {
        self.f = pack_f(s, z, h, pv, n, c);
    }

    /// Read the raw F byte.
    #[inline]
    pub fn read_f(&self) -> u8 { self.f }

    /// Write raw F byte.
    #[inline]
    pub fn write_f(&mut self, byte: u8) { self.f = byte; }

    /// Read raw F' byte.
    #[inline]
    pub fn read_f_prime(&self) -> u8 { self.f_prime }

    // ── Bank exchange operations ──────────────────────────────────────────────

    /// EX AF, AF' — swap main A/F with alternate A'/F'.
    pub fn exchange_af(&mut self) {
        let a_main = self.regs[REG_A];
        let a_alt  = self.alt[REG_A];
        let f_main = self.f;
        let f_alt  = self.f_prime;
        self.regs[REG_A] = a_alt;
        self.alt[REG_A]  = a_main;
        self.f           = f_alt;
        self.f_prime     = f_main;
    }

    /// EXX — swap BC, DE, HL with B'C', D'E', H'L' (AF not affected).
    pub fn exchange_bank(&mut self) {
        for reg_id in [REG_B, REG_C, REG_D, REG_E, REG_H, REG_L] {
            let main = self.regs[reg_id];
            let alt  = self.alt[reg_id];
            self.regs[reg_id] = alt;
            self.alt[reg_id]  = main;
        }
    }

    // ── Alternate register access ─────────────────────────────────────────────

    /// Read an alternate register by code.
    #[inline]
    pub fn read_alt8(&self, reg_id: usize) -> u8 {
        self.alt[reg_id]
    }
}

impl Default for RegisterFile {
    fn default() -> Self { Self::new() }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write_8bit() {
        let mut rf = RegisterFile::new();
        rf.write8(REG_A, 0xAB);
        assert_eq!(rf.read8(REG_A), 0xAB);
        rf.write8(REG_B, 0xFF);
        assert_eq!(rf.read8(REG_B), 0xFF);
    }

    #[test]
    fn read_write_16bit_pair() {
        let mut rf = RegisterFile::new();
        rf.write16_pair(0, 0x1234); // BC
        assert_eq!(rf.read8(REG_B), 0x12);
        assert_eq!(rf.read8(REG_C), 0x34);
        assert_eq!(rf.read16_pair(0, 0), 0x1234);
    }

    #[test]
    fn flags_roundtrip() {
        let mut rf = RegisterFile::new();
        rf.write_flags(1, 0, 1, 0, 1, 0); // S=1, H=1, N=1
        let (s, z, h, pv, n, c) = rf.read_flags();
        assert_eq!(s, 1);
        assert_eq!(z, 0);
        assert_eq!(h, 1);
        assert_eq!(pv, 0);
        assert_eq!(n, 1);
        assert_eq!(c, 0);
    }

    #[test]
    fn exchange_af() {
        let mut rf = RegisterFile::new();
        rf.write8(REG_A, 0x42);
        rf.write_f(0b10000001); // S=1, C=1
        rf.exchange_af();
        assert_eq!(rf.read8(REG_A), 0x00); // now holds A' (was 0)
        assert_eq!(rf.read_f(), 0x00);     // now holds F' (was 0)
        assert_eq!(rf.read_alt8(REG_A), 0x42); // A' holds old A
        assert_eq!(rf.read_f_prime(), 0b10000001); // F' holds old F

        // Exchange again — should restore
        rf.exchange_af();
        assert_eq!(rf.read8(REG_A), 0x42);
        assert_eq!(rf.read_f(), 0b10000001);
    }

    #[test]
    fn exchange_bank() {
        let mut rf = RegisterFile::new();
        rf.write8(REG_B, 0x12);
        rf.write8(REG_C, 0x34);
        rf.exchange_bank();
        assert_eq!(rf.read8(REG_B), 0x00); // now holds B' (was 0)
        assert_eq!(rf.read_alt8(REG_B), 0x12); // B' holds old B

        // Exchange again — restore
        rf.exchange_bank();
        assert_eq!(rf.read8(REG_B), 0x12);
    }

    #[test]
    fn pack_unpack_f() {
        let f = pack_f(1, 0, 1, 0, 1, 1);
        let (s, z, h, pv, n, c) = unpack_f(f);
        assert_eq!(s, 1); assert_eq!(z, 0); assert_eq!(h, 1);
        assert_eq!(pv, 0); assert_eq!(n, 1); assert_eq!(c, 1);
    }
}
