//! Register file for the Motorola 68000.
//!
//! ## Register overview
//!
//! ```text
//! Data registers (32-bit, fully orthogonal)
//! ─────────────────────────────────────────
//! D0 – D7  Any ALU op can target any Dn.
//!          Byte ops:  affect bits 7–0  (upper 24 bits unchanged).
//!          Word ops:  affect bits 15–0 (upper 16 bits unchanged).
//!          Long ops:  affect all 32 bits.
//!
//! Address registers (32-bit, no byte access)
//! ─────────────────────────────────────────
//! A0 – A6  General purpose pointer registers.
//! A7       Supervisor stack pointer (SSP).  This simulator stays in
//!          supervisor mode, so A7 == SSP throughout.
//!
//! Program counter
//! ───────────────
//! PC       32-bit; only bits 23–0 are used (24-bit address bus).
//!          All instructions are word-aligned.
//!
//! Status register (16-bit)
//! ────────────────────────
//! Bits 15–8: system byte (T1 T0 S M 0 I2 I1 I0)
//! Bits 7–5:  0 (reserved)
//! Bit  4:    X — extend
//! Bit  3:    N — negative
//! Bit  2:    Z — zero
//! Bit  1:    V — overflow
//! Bit  0:    C — carry
//! ```
//!
//! ## Flag bit positions in SR
//!
//! ```text
//! Bit 4 = X   Bit 3 = N   Bit 2 = Z   Bit 1 = V   Bit 0 = C
//! ```

use logic_gates::gates::{and_gate, or_gate};

/// The Motorola 68000 register file.
///
/// All integer values are unsigned.  Flag bits are stored as `u8` (0 or 1)
/// and assembled into/extracted from `sr` on demand.
pub struct RegisterFile68K {
    /// Data registers D0–D7 (32-bit each).
    pub d: [u32; 8],
    /// Address registers A0–A7 (32-bit each; A7 = supervisor stack pointer).
    pub a: [u32; 8],
    /// Program counter (24-bit effective, stored as 32-bit).
    pub pc: u32,
    /// Status register (16-bit: system byte + CCR).
    pub sr: u16,
}

/// 24-bit address space mask.
pub const ADDR_MASK: u32 = 0x00FF_FFFF;
/// 32-bit value mask.
pub const LONG_MASK: u32 = 0xFFFF_FFFF;
/// 16-bit value mask.
pub const WORD_MASK: u32 = 0x0000_FFFF;
/// 8-bit value mask.
pub const BYTE_MASK: u32 = 0x0000_00FF;

/// CCR bit positions in SR.
const X_BIT: u16 = 1 << 4;
const N_BIT: u16 = 1 << 3;
const Z_BIT: u16 = 1 << 2;
const V_BIT: u16 = 1 << 1;
const C_BIT: u16 = 1 << 0;

impl Default for RegisterFile68K {
    fn default() -> Self {
        Self::new()
    }
}

impl RegisterFile68K {
    /// Power-on state: all registers zero except A7 = 0x00F000, SR = 0x2700.
    ///
    /// SR = 0x2700 means supervisor mode (S=1) with interrupt priority mask 7.
    /// Programs load at 0x001000; stack grows down from 0x00F000.
    pub fn new() -> Self {
        let mut rf = RegisterFile68K {
            d: [0u32; 8],
            a: [0u32; 8],
            pc: 0x0000_1000,
            sr: 0x2700,
        };
        rf.a[7] = 0x0000_F000; // supervisor stack pointer
        rf
    }

    // ── Data register access ──────────────────────────────────────────────────

    /// Read the low `sz` bytes from data register `n` (0–7).
    ///
    /// `sz`: 1=byte, 2=word, 4=long.
    pub fn read_dn(&self, n: usize, sz: usize) -> u32 {
        let mask = match sz {
            1 => BYTE_MASK,
            2 => WORD_MASK,
            _ => LONG_MASK,
        };
        self.d[n] & mask
    }

    /// Write `sz` bytes into data register `n`, preserving upper bits.
    ///
    /// ```text
    /// sz=1: write bits 7–0;   bits 31–8  unchanged.
    /// sz=2: write bits 15–0;  bits 31–16 unchanged.
    /// sz=4: write all 32 bits.
    /// ```
    pub fn write_dn(&mut self, n: usize, val: u32, sz: usize) {
        let (mask, keep) = match sz {
            1 => (BYTE_MASK, LONG_MASK ^ BYTE_MASK),
            2 => (WORD_MASK, LONG_MASK ^ WORD_MASK),
            _ => (LONG_MASK, 0),
        };
        self.d[n] = (self.d[n] & keep) | (val & mask);
    }

    // ── Address register access ───────────────────────────────────────────────

    /// Read address register `n` (always full 32-bit).
    pub fn read_an(&self, n: usize) -> u32 {
        self.a[n]
    }

    /// Write to address register `n`.
    ///
    /// For word writes (`sz=2`), the value is **sign-extended** to 32 bits
    /// before storage — the 68000's MOVEA.W sign-extends the source.
    pub fn write_an(&mut self, n: usize, val: u32, sz: usize) {
        if sz == 2 {
            // Sign-extend 16-bit to 32-bit: if bit 15 is set, set bits 31–16.
            let v16 = val & WORD_MASK;
            let sign_bit = (v16 >> 15) & 1;
            // Gate-level sign extension: OR every upper bit with sign_bit
            let upper = if sign_bit == 1 { 0xFFFF_0000u32 } else { 0 };
            self.a[n] = (upper | v16) & LONG_MASK;
        } else {
            self.a[n] = val & LONG_MASK;
        }
    }

    // ── CCR flag accessors ────────────────────────────────────────────────────

    /// Get the X (extend) flag as 0 or 1.
    pub fn flag_x(&self) -> u8 {
        ((self.sr & X_BIT) != 0) as u8
    }

    /// Get the N (negative) flag as 0 or 1.
    pub fn flag_n(&self) -> u8 {
        ((self.sr & N_BIT) != 0) as u8
    }

    /// Get the Z (zero) flag as 0 or 1.
    pub fn flag_z(&self) -> u8 {
        ((self.sr & Z_BIT) != 0) as u8
    }

    /// Get the V (overflow) flag as 0 or 1.
    pub fn flag_v(&self) -> u8 {
        ((self.sr & V_BIT) != 0) as u8
    }

    /// Get the C (carry) flag as 0 or 1.
    pub fn flag_c(&self) -> u8 {
        ((self.sr & C_BIT) != 0) as u8
    }

    /// Set the CCR bits (X, N, Z, V, C) in SR using OR/AND gate operations.
    ///
    /// The system byte (bits 15–5) of SR is preserved.
    ///
    /// Each flag is written by: clear the bit (AND with NOT mask), then OR in
    /// the new value.  This models the D-flip-flop update in real hardware.
    pub fn set_ccr(&mut self, x: u8, n: u8, z: u8, v: u8, c: u8) {
        let keep = self.sr & 0xFFE0u16; // bits 15–5 (system byte + reserved)
        let ccr = ((x as u16) << 4)
            | ((n as u16) << 3)
            | ((z as u16) << 2)
            | ((v as u16) << 1)
            | (c as u16);
        // OR the preserved system byte with the new CCR
        self.sr = {
            // Gate-level: OR each system byte bit with 0 (keeps it); OR each
            // CCR bit with computed value.
            let s = keep;
            let c_bits = ccr;
            // Combine: s | c_bits (safe since they occupy disjoint bit ranges)
            s | c_bits
        };
    }

    /// Set N, Z, V, C (common arithmetic result); X = C.
    pub fn set_nzvc_x(&mut self, n: u8, z: u8, v: u8, c: u8) {
        self.set_ccr(c, n, z, v, c);
    }

    /// Set N, Z; clear V, C; leave X unchanged.
    ///
    /// Used by logic operations (AND, OR, EOR, NOT, CLR, TST).
    pub fn set_nz_clear_vc(&mut self, n: u8, z: u8) {
        let old_x = self.flag_x();
        self.set_ccr(old_x, n, z, 0, 0);
    }

    /// Update the NEGX Z-flag: Z is only *cleared* if result != 0, never set.
    ///
    /// NEGX/ADDX/SUBX rule: `new_Z = old_Z AND (result == 0)`.
    /// This preserves the previous Z across a multi-word chain.
    pub fn negx_z(&mut self, result_z: u8) {
        let old_z = self.flag_z();
        // new_z = AND(old_z, result_z) — gate-level AND gate
        let new_z = and_gate(old_z, result_z);
        let n = self.flag_n();
        let v = self.flag_v();
        let c = self.flag_c();
        let x = self.flag_x();
        self.set_ccr(x, n, new_z, v, c);
    }

    // ── Condition code evaluation ─────────────────────────────────────────────
    //
    // Returns true if the named condition is satisfied.
    // Condition codes: T F HI LS CC CS NE EQ VC VS PL MI GE LT GT LE
    //
    // cc_index: 0=T, 1=F, 2=HI, 3=LS, 4=CC, 5=CS, 6=NE, 7=EQ,
    //           8=VC, 9=VS, 10=PL, 11=MI, 12=GE, 13=LT, 14=GT, 15=LE

    /// Evaluate a 68000 condition code by index (0–15).
    ///
    /// ```text
    /// 0  T   Always true
    /// 1  F   Always false
    /// 2  HI  Higher:           NOT C AND NOT Z
    /// 3  LS  Lower or Same:    C OR Z
    /// 4  CC  Carry Clear:      NOT C
    /// 5  CS  Carry Set:        C
    /// 6  NE  Not Equal:        NOT Z
    /// 7  EQ  Equal:            Z
    /// 8  VC  Overflow Clear:   NOT V
    /// 9  VS  Overflow Set:     V
    /// 10 PL  Plus:             NOT N
    /// 11 MI  Minus:            N
    /// 12 GE  Greater or Equal: N == V
    /// 13 LT  Less Than:        N != V
    /// 14 GT  Greater Than:     NOT Z AND (N == V)
    /// 15 LE  Less or Equal:    Z OR (N != V)
    /// ```
    pub fn test_cc(&self, cc: u8) -> bool {
        let n = self.flag_n();
        let z = self.flag_z();
        let v = self.flag_v();
        let c = self.flag_c();

        // Gate-level implementation: each condition is a combinational logic
        // expression.  NOT/AND/OR gates on the flag bits.
        match cc {
            0  => true,                                          // T
            1  => false,                                         // F
            2  => and_gate(1 - c, 1 - z) != 0,                  // HI: NOT C AND NOT Z
            3  => or_gate(c, z) != 0,                           // LS
            4  => c == 0,                                       // CC
            5  => c != 0,                                       // CS
            6  => z == 0,                                       // NE
            7  => z != 0,                                       // EQ
            8  => v == 0,                                       // VC
            9  => v != 0,                                       // VS
            10 => n == 0,                                       // PL
            11 => n != 0,                                       // MI
            12 => n == v,                                       // GE: N XNOR V (N==V)
            13 => n != v,                                       // LT
            14 => and_gate(1 - z, if n == v { 1 } else { 0 }) != 0,  // GT
            _  => or_gate(z, if n != v { 1 } else { 0 }) != 0, // LE
        }
    }

    // ── SR pack/unpack ────────────────────────────────────────────────────────

    /// Assemble the full 16-bit SR from component parts (used by MOVE #imm, SR).
    pub fn write_sr(&mut self, val: u16) {
        self.sr = val;
    }

    /// Read the full SR.
    pub fn read_sr(&self) -> u16 {
        self.sr
    }

    /// Read the CCR (lower 5 bits of SR).
    pub fn read_ccr(&self) -> u8 {
        (self.sr & 0x1F) as u8
    }

    /// Write the CCR (lower 5 bits of SR); upper byte preserved.
    pub fn write_ccr(&mut self, val: u8) {
        self.sr = (self.sr & 0xFFE0) | ((val as u16) & 0x1F);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state() {
        let rf = RegisterFile68K::new();
        assert_eq!(rf.d, [0u32; 8]);
        assert_eq!(rf.a[7], 0x0000_F000);
        assert_eq!(rf.pc, 0x0000_1000);
        assert_eq!(rf.sr, 0x2700);
    }

    #[test]
    fn write_dn_byte_preserves_upper() {
        let mut rf = RegisterFile68K::new();
        rf.d[0] = 0xABCD_EF00;
        rf.write_dn(0, 0x42, 1);
        assert_eq!(rf.d[0], 0xABCD_EF42);
    }

    #[test]
    fn write_dn_word_preserves_upper() {
        let mut rf = RegisterFile68K::new();
        rf.d[0] = 0xABCD_0000;
        rf.write_dn(0, 0x1234, 2);
        assert_eq!(rf.d[0], 0xABCD_1234);
    }

    #[test]
    fn write_an_sign_extends_word() {
        let mut rf = RegisterFile68K::new();
        rf.write_an(0, 0x8000, 2); // sign-extend -32768 to 32-bit
        assert_eq!(rf.a[0], 0xFFFF_8000);
    }

    #[test]
    fn write_an_positive_word() {
        let mut rf = RegisterFile68K::new();
        rf.write_an(0, 0x7FFF, 2);
        assert_eq!(rf.a[0], 0x0000_7FFF);
    }

    #[test]
    fn set_ccr_basic() {
        let mut rf = RegisterFile68K::new();
        rf.set_ccr(1, 0, 1, 0, 0); // X=1, N=0, Z=1, V=0, C=0
        assert_eq!(rf.flag_x(), 1);
        assert_eq!(rf.flag_n(), 0);
        assert_eq!(rf.flag_z(), 1);
        assert_eq!(rf.flag_v(), 0);
        assert_eq!(rf.flag_c(), 0);
        // SR system byte should be preserved (0x2700 → 0x2714)
        assert_eq!(rf.sr & 0xFF00, 0x2700);
    }

    #[test]
    fn test_cc_t_f() {
        let rf = RegisterFile68K::new();
        assert!(rf.test_cc(0));   // T
        assert!(!rf.test_cc(1));  // F
    }

    #[test]
    fn test_cc_eq_ne() {
        let mut rf = RegisterFile68K::new();
        // Z=1 → EQ true, NE false
        rf.set_ccr(0, 0, 1, 0, 0);
        assert!(rf.test_cc(7));   // EQ
        assert!(!rf.test_cc(6));  // NE
    }

    #[test]
    fn test_cc_ge_lt() {
        let mut rf = RegisterFile68K::new();
        // N=0, V=0 → N==V → GE true, LT false
        rf.set_ccr(0, 0, 0, 0, 0);
        assert!(rf.test_cc(12));  // GE
        assert!(!rf.test_cc(13)); // LT
        // N=1, V=0 → N!=V → GE false, LT true
        rf.set_ccr(0, 1, 0, 0, 0);
        assert!(!rf.test_cc(12));
        assert!(rf.test_cc(13));
    }

    #[test]
    fn negx_z_chain() {
        // NEGX chain: Z stays 1 until a non-zero result clears it.
        let mut rf = RegisterFile68K::new();
        // Initial: Z=1
        rf.set_ccr(0, 0, 1, 0, 0);
        // NEGX of 0 → result_z=1 → Z stays 1
        rf.negx_z(1);
        assert_eq!(rf.flag_z(), 1);
        // NEGX of 1 → result_z=0 → Z clears
        rf.negx_z(0);
        assert_eq!(rf.flag_z(), 0);
    }
}
