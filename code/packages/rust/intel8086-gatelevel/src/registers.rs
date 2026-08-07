//! Register file for the Intel 8086 gate-level simulator.
//!
//! # Register architecture
//!
//! ```text
//! General-purpose (16-bit, with byte-accessible halves):
//!   AX (AH:AL)  — Accumulator; implicit in MUL/DIV/string/BCD ops
//!   BX (BH:BL)  — Base; memory addressing base register
//!   CX (CH:CL)  — Counter; LOOP, REP prefix, shift counts
//!   DX (DH:DL)  — Data; high word of 32-bit MUL/DIV; I/O port address
//!
//! Index / pointer (16-bit only):
//!   SI  — Source Index; default DS segment
//!   DI  — Destination Index; default ES segment
//!   SP  — Stack Pointer; SS segment
//!   BP  — Base Pointer; SS segment
//!
//! Segment registers (16-bit):
//!   CS  — Code Segment; physical instruction fetch = CS×16 + IP
//!   DS  — Data Segment; default for most memory references
//!   SS  — Stack Segment; PUSH/POP and BP-relative accesses
//!   ES  — Extra Segment; destination for string operations
//!
//! Instruction pointer:
//!   IP  — 16-bit offset within CS
//!
//! FLAGS (16-bit word, individual flip-flops):
//!   bit 0: CF    bit 1: 1(always)  bit 2: PF   bit 4: AF
//!   bit 6: ZF    bit 7: SF         bit 8: TF   bit 9: IF
//!   bit 10: DF   bit 11: OF
//! ```
//!
//! # Segment addressing
//!
//! Physical address = (seg_reg << 4) + offset, modulo 1 MB.
//!
//! The "× 16" shift is **wire routing** — bits 0-15 of the segment feed into
//! positions 4-19 of the 20-bit address bus, with positions 0-3 tied to zero.
//! The 16-bit offset is then added using a 20-bit ripple-carry chain.
//!
//! # ModRM register encodings
//!
//! ```text
//! 16-bit:    0=AX 1=CX 2=DX 3=BX 4=SP 5=BP 6=SI 7=DI
//!  8-bit:    0=AL 1=CL 2=DL 3=BL 4=AH 5=CH 6=DH 7=BH
//! Segment:   0=ES 1=CS 2=SS 3=DS
//! ```

use arithmetic::adders::full_adder;
use logic_gates::gates::{and_gate, or_gate};


// ─── Register file struct ─────────────────────────────────────────────────────

/// Complete Intel 8086 register file.
///
/// All registers are stored as `u16` integers; flag bits as `u8` (0 or 1).
/// Byte-access methods (AL/AH etc.) read/write the appropriate 8-bit half.
///
/// # Example
/// ```
/// use coding_adventures_intel8086_gatelevel::registers::RegisterFile8086;
/// let mut rf = RegisterFile8086::new();
/// rf.write16(0, 0x1234); // AX = 0x1234
/// assert_eq!(rf.read8_low(0), 0x34); // AL = 0x34
/// assert_eq!(rf.read8_high(0), 0x12); // AH = 0x12
/// ```
#[derive(Debug, Clone)]
pub struct RegisterFile8086 {
    // General-purpose registers
    pub ax: u16,
    pub bx: u16,
    pub cx: u16,
    pub dx: u16,
    // Index and pointer registers
    pub si: u16,
    pub di: u16,
    pub sp: u16,
    pub bp: u16,
    // Segment registers
    pub cs: u16,
    pub ds: u16,
    pub ss: u16,
    pub es: u16,
    // Instruction pointer
    pub ip: u16,
    // FLAGS — individual D flip-flops
    pub flag_cf: u8, // carry
    pub flag_pf: u8, // parity
    pub flag_af: u8, // auxiliary carry
    pub flag_zf: u8, // zero
    pub flag_sf: u8, // sign
    pub flag_tf: u8, // trap
    pub flag_if: u8, // interrupt enable
    pub flag_df: u8, // direction
    pub flag_of: u8, // overflow
}

impl RegisterFile8086 {
    /// Create a new register file with all registers initialised to 0.
    pub fn new() -> Self {
        RegisterFile8086 {
            ax: 0, bx: 0, cx: 0, dx: 0,
            si: 0, di: 0, sp: 0, bp: 0,
            cs: 0, ds: 0, ss: 0, es: 0,
            ip: 0,
            flag_cf: 0, flag_pf: 0, flag_af: 0, flag_zf: 0,
            flag_sf: 0, flag_tf: 0, flag_if: 0, flag_df: 0,
            flag_of: 0,
        }
    }

    // ── ModRM 16-bit register read/write ──────────────────────────────────────
    //
    // ModRM field r/m / reg encoding for 16-bit operands:
    //   0=AX, 1=CX, 2=DX, 3=BX, 4=SP, 5=BP, 6=SI, 7=DI

    /// Read a 16-bit general or index register by ModRM code (0–7).
    pub fn read16(&self, code: u8) -> u16 {
        match code & 7 {
            0 => self.ax, 1 => self.cx, 2 => self.dx, 3 => self.bx,
            4 => self.sp, 5 => self.bp, 6 => self.si, _ => self.di,
        }
    }

    /// Write a 16-bit general or index register by ModRM code (0–7).
    pub fn write16(&mut self, code: u8, value: u16) {
        match code & 7 {
            0 => self.ax = value, 1 => self.cx = value,
            2 => self.dx = value, 3 => self.bx = value,
            4 => self.sp = value, 5 => self.bp = value,
            6 => self.si = value, _ => self.di = value,
        }
    }

    // ── 8-bit register access (ModRM encoding) ────────────────────────────────
    //
    // Byte register encoding:
    //   0=AL, 1=CL, 2=DL, 3=BL, 4=AH, 5=CH, 6=DH, 7=BH

    /// Read an 8-bit register by ModRM byte-register code (0–7).
    pub fn read8(&self, code: u8) -> u8 {
        match code & 7 {
            0 => self.read8_low(0),  // AL
            1 => self.read8_low(1),  // CL
            2 => self.read8_low(2),  // DL
            3 => self.read8_low(3),  // BL
            4 => self.read8_high(0), // AH
            5 => self.read8_high(1), // CH
            6 => self.read8_high(2), // DH
            _ => self.read8_high(3), // BH
        }
    }

    /// Write an 8-bit register by ModRM byte-register code (0–7).
    pub fn write8(&mut self, code: u8, value: u8) {
        match code & 7 {
            0 => self.write8_low(0, value),  // AL
            1 => self.write8_low(1, value),  // CL
            2 => self.write8_low(2, value),  // DL
            3 => self.write8_low(3, value),  // BL
            4 => self.write8_high(0, value), // AH
            5 => self.write8_high(1, value), // CH
            6 => self.write8_high(2, value), // DH
            _ => self.write8_high(3, value), // BH
        }
    }

    // ── Byte-half helpers ─────────────────────────────────────────────────────
    //
    // The four general-purpose registers provide access to their low byte (xL)
    // and high byte (xH). These are physical sub-word ports of a single 16-bit
    // register cell.
    //
    // `code` here is the 16-bit ModRM code (0=AX,1=CX,2=DX,3=BX).

    /// Read the low byte of a general-purpose register (AL/CL/DL/BL).
    pub fn read8_low(&self, code: u8) -> u8 {
        (self.read16(code) & 0xFF) as u8
    }

    /// Write the low byte of a general-purpose register (AL/CL/DL/BL).
    pub fn write8_low(&mut self, code: u8, value: u8) {
        let old = self.read16(code);
        self.write16(code, (old & 0xFF00) | (value as u16));
    }

    /// Read the high byte of a general-purpose register (AH/CH/DH/BH).
    pub fn read8_high(&self, code: u8) -> u8 {
        ((self.read16(code) >> 8) & 0xFF) as u8
    }

    /// Write the high byte of a general-purpose register (AH/CH/DH/BH).
    pub fn write8_high(&mut self, code: u8, value: u8) {
        let old = self.read16(code);
        self.write16(code, (old & 0x00FF) | ((value as u16) << 8));
    }

    // ── Segment register access ───────────────────────────────────────────────
    //
    // ModRM segment field encoding: 0=ES, 1=CS, 2=SS, 3=DS

    /// Read a segment register by ModRM segment field (0–3).
    pub fn read_seg(&self, code: u8) -> u16 {
        match code & 3 {
            0 => self.es, 1 => self.cs, 2 => self.ss, _ => self.ds,
        }
    }

    /// Write a segment register by ModRM segment field (0–3).
    pub fn write_seg(&mut self, code: u8, value: u16) {
        match code & 3 {
            0 => self.es = value, 1 => self.cs = value,
            2 => self.ss = value, _ => self.ds = value,
        }
    }

    // ── FLAGS pack / unpack ───────────────────────────────────────────────────
    //
    // The FLAGS word is laid out as a 16-bit register:
    //
    //   bit 0: CF   bit 1: 1   bit 2: PF   bit 4: AF
    //   bit 6: ZF   bit 7: SF  bit 8: TF   bit 9: IF
    //   bit 10: DF  bit 11: OF
    //
    // Gate path: each flag passes through an OR gate with 0 (no-op identity)
    // to model driving the bus through a real gate layer.

    /// Pack all flag flip-flops into a 16-bit FLAGS word.
    ///
    /// Bit 1 is always 1 (8086 hardware invariant).
    pub fn pack_flags(&self) -> u16 {
        let cf = or_gate(self.flag_cf, 0);
        let pf = or_gate(self.flag_pf, 0);
        let af = or_gate(self.flag_af, 0);
        let zf = or_gate(self.flag_zf, 0);
        let sf = or_gate(self.flag_sf, 0);
        let tf = or_gate(self.flag_tf, 0);
        let if_ = or_gate(self.flag_if, 0);
        let df = or_gate(self.flag_df, 0);
        let of = or_gate(self.flag_of, 0);
        (cf as u16)
            | (1u16 << 1)
            | ((pf as u16) << 2)
            | ((af as u16) << 4)
            | ((zf as u16) << 6)
            | ((sf as u16) << 7)
            | ((tf as u16) << 8)
            | ((if_ as u16) << 9)
            | ((df as u16) << 10)
            | ((of as u16) << 11)
    }

    /// Unpack a 16-bit FLAGS word into individual flag flip-flops.
    ///
    /// Used by POPF and IRET. Each bit is latched through an AND gate with 1
    /// to model the register input path.
    pub fn unpack_flags(&mut self, flags: u16) {
        self.flag_cf = and_gate((flags & 1) as u8, 1);
        self.flag_pf = and_gate(((flags >> 2) & 1) as u8, 1);
        self.flag_af = and_gate(((flags >> 4) & 1) as u8, 1);
        self.flag_zf = and_gate(((flags >> 6) & 1) as u8, 1);
        self.flag_sf = and_gate(((flags >> 7) & 1) as u8, 1);
        self.flag_tf = and_gate(((flags >> 8) & 1) as u8, 1);
        self.flag_if = and_gate(((flags >> 9) & 1) as u8, 1);
        self.flag_df = and_gate(((flags >> 10) & 1) as u8, 1);
        self.flag_of = and_gate(((flags >> 11) & 1) as u8, 1);
    }

    // ── Physical address computation ──────────────────────────────────────────

    /// Compute the 20-bit physical address for a segment:offset pair.
    ///
    /// Physical address = (seg_reg × 16 + offset) & 0xFFFFF.
    ///
    /// The "× 16" step is **bit rewiring** — the 16-bit segment value is
    /// placed on bus lines 4–19 with bus lines 0–3 fixed to 0 (no gates).
    /// The 16-bit offset is then added via a 20-stage ripple-carry adder.
    ///
    /// # Example
    /// ```
    /// use coding_adventures_intel8086_gatelevel::registers::RegisterFile8086;
    /// let mut rf = RegisterFile8086::new();
    /// rf.cs = 0x1000;
    /// assert_eq!(rf.physical_address(rf.cs, 0x0100), 0x10100);
    /// ```
    pub fn physical_address(&self, seg: u16, offset: u16) -> u32 {
        // seg × 16: wire bits 0-15 of seg to positions 4-19 of the 20-bit bus.
        let seg20 = (seg as u32) << 4;
        add_20bit(seg20, offset as u32) & 0xFFFFF
    }

    // ── Named-register helpers (convenience, not ModRM) ───────────────────────

    /// Read AL (AX low byte).
    pub fn al(&self) -> u8 { self.read8_low(0) }
    /// Write AL.
    pub fn set_al(&mut self, v: u8) { self.write8_low(0, v); }
    /// Read AH (AX high byte).
    pub fn ah(&self) -> u8 { self.read8_high(0) }
    /// Write AH.
    pub fn set_ah(&mut self, v: u8) { self.write8_high(0, v); }
    /// Read BL.
    pub fn bl(&self) -> u8 { self.read8_low(3) }
    /// Write BL.
    pub fn set_bl(&mut self, v: u8) { self.write8_low(3, v); }
    /// Read BH.
    pub fn bh(&self) -> u8 { self.read8_high(3) }
    /// Write BH.
    pub fn set_bh(&mut self, v: u8) { self.write8_high(3, v); }
    /// Read CL.
    pub fn cl(&self) -> u8 { self.read8_low(1) }
    /// Write CL.
    pub fn set_cl(&mut self, v: u8) { self.write8_low(1, v); }
    /// Read CH.
    pub fn ch(&self) -> u8 { self.read8_high(1) }
    /// Write CH.
    pub fn set_ch(&mut self, v: u8) { self.write8_high(1, v); }
    /// Read DL.
    pub fn dl(&self) -> u8 { self.read8_low(2) }
    /// Write DL.
    pub fn set_dl(&mut self, v: u8) { self.write8_low(2, v); }
    /// Read DH.
    pub fn dh(&self) -> u8 { self.read8_high(2) }
    /// Write DH.
    pub fn set_dh(&mut self, v: u8) { self.write8_high(2, v); }
}

impl Default for RegisterFile8086 {
    fn default() -> Self { Self::new() }
}

// ─── 20-bit ripple-carry adder ────────────────────────────────────────────────

/// Add two 20-bit values through a 20-stage ripple-carry chain.
///
/// Used for segment:offset physical address computation. The result is masked
/// to 20 bits (0–0xFFFFF) to model the 8086's 20-bit address bus.
pub fn add_20bit(a: u32, b: u32) -> u32 {
    // Expand to 20-element LSB-first bit vectors.
    let a_bits: Vec<u8> = (0..20).map(|i| ((a >> i) & 1) as u8).collect();
    let b_bits: Vec<u8> = (0..20).map(|i| ((b >> i) & 1) as u8).collect();
    let mut carry = 0u8;
    let mut sums = Vec::with_capacity(20);
    for i in 0..20 {
        let (s, c) = full_adder(a_bits[i], b_bits[i], carry);
        sums.push(s);
        carry = c;
    }
    sums.iter()
        .take(20)
        .enumerate()
        .fold(0u32, |acc, (i, &b)| acc | ((b as u32) << i))
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write_16bit() {
        let mut rf = RegisterFile8086::new();
        rf.write16(0, 0x1234); // AX
        assert_eq!(rf.read16(0), 0x1234);
        rf.write16(1, 0xABCD); // CX
        assert_eq!(rf.read16(1), 0xABCD);
    }

    #[test]
    fn byte_halves_are_independent() {
        let mut rf = RegisterFile8086::new();
        rf.write16(0, 0x1234); // AX = 0x1234
        assert_eq!(rf.al(), 0x34); // low
        assert_eq!(rf.ah(), 0x12); // high
        rf.set_al(0xFF);
        assert_eq!(rf.read16(0), 0x12FF);
        rf.set_ah(0xAB);
        assert_eq!(rf.read16(0), 0xABFF);
    }

    #[test]
    fn read8_modrm_encoding() {
        let mut rf = RegisterFile8086::new();
        rf.ax = 0x1234;
        rf.cx = 0x5678;
        // ModRM 8-bit encoding: 0=AL, 1=CL, 4=AH, 5=CH
        assert_eq!(rf.read8(0), 0x34); // AL
        assert_eq!(rf.read8(1), 0x78); // CL
        assert_eq!(rf.read8(4), 0x12); // AH
        assert_eq!(rf.read8(5), 0x56); // CH
    }

    #[test]
    fn segment_register_encoding() {
        let mut rf = RegisterFile8086::new();
        rf.write_seg(1, 0x2000); // CS
        assert_eq!(rf.read_seg(1), 0x2000);
        rf.write_seg(3, 0x3000); // DS
        assert_eq!(rf.read_seg(3), 0x3000);
        assert_eq!(rf.ds, 0x3000);
    }

    #[test]
    fn pack_unpack_flags_roundtrip() {
        let mut rf = RegisterFile8086::new();
        rf.flag_cf = 1;
        rf.flag_zf = 1;
        rf.flag_pf = 1;
        let packed = rf.pack_flags();
        assert_eq!(packed & 1, 1); // CF
        assert_eq!((packed >> 1) & 1, 1); // always 1
        assert_eq!((packed >> 6) & 1, 1); // ZF
        let mut rf2 = RegisterFile8086::new();
        rf2.unpack_flags(packed);
        assert_eq!(rf2.flag_cf, 1);
        assert_eq!(rf2.flag_zf, 1);
        assert_eq!(rf2.flag_pf, 1);
        assert_eq!(rf2.flag_sf, 0);
    }

    #[test]
    fn pack_flags_bit1_always_set() {
        let rf = RegisterFile8086::new();
        assert_eq!((rf.pack_flags() >> 1) & 1, 1);
    }

    #[test]
    fn physical_address_basic() {
        let rf = RegisterFile8086::new();
        // CS=0x1000, offset=0x0100 → 0x1000 * 16 + 0x100 = 0x10100
        assert_eq!(rf.physical_address(0x1000, 0x0100), 0x10100);
        // Wrap around: 0xFFFF * 16 + 0xFFFF = 0xFFFF0 + 0xFFFF = 0x1FFEF → masked to 0xFFEF
        // Actually: 0xFFFF0 + 0xFFFF = 0x1FFFEF, masked to 0xFFEF
        assert_eq!(rf.physical_address(0xFFFF, 0xFFFF) & 0xFFFFF, (0xFFFF0u32 + 0xFFFFu32) & 0xFFFFF);
    }

    #[test]
    fn add_20bit_basic() {
        assert_eq!(add_20bit(0x10000, 0x00100), 0x10100);
        assert_eq!(add_20bit(0xFFFFF, 1) & 0xFFFFF, 0); // 20-bit wrap
    }
}
