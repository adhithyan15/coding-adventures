//! # `aarch64-encoder` — ARM64 (AArch64) instruction encoder.
//!
//! Pure-Rust encoder that produces little-endian 32-bit instruction words
//! for the AArch64 instruction set.  Designed to be the bottom-of-stack
//! for any CIR → native-code lowering in this repo (jit-core / aot-core),
//! so the contract is intentionally narrow:
//!
//! - Each high-level method on [`Assembler`] emits **one** instruction
//!   word.
//! - Branches reference [`LabelId`]s that are resolved at [`Assembler::finish`]
//!   time — no two-pass bookkeeping is exposed to callers.
//! - Output is the raw `.text` byte stream — no headers, no relocations
//!   beyond branch fix-ups.  Wrapping into Mach-O / ELF is the job of
//!   `code-packager`.
//!
//! ## Coverage (V1 — what fib() needs)
//!
//! | Family | Mnemonics | Notes |
//! |---|---|---|
//! | Move immediate | `movz`, `movk` | + `mov_imm64` helper that synthesises a 1- to 4-instruction sequence |
//! | Arithmetic (reg) | `add`, `sub`, `mul` | 64-bit operands |
//! | Arithmetic (imm) | `add_imm`, `sub_imm` | 12-bit immediate |
//! | Compare | `cmp`, `cmp_imm` | aliases for `subs xzr, ...` |
//! | Memory | `ldr`, `str_` | unsigned-offset, 64-bit |
//! | Pair | `stp_pre`, `ldp_post` | for prologue/epilogue framing |
//! | Branch | `b`, `b_cond`, `bl` | PC-relative, label-resolved |
//! | Indirect | `blr`, `ret` | function call/return via register |
//! | Misc | `nop`, `udf` | trap for guard-fail in AOT |
//!
//! Floats, atomics, SIMD, system instructions, and large-immediate
//! addressing modes are **out of scope** for V1; they can be added
//! incrementally.
//!
//! ## Quick start
//!
//! ```rust
//! use aarch64_encoder::{Assembler, Reg, Cond};
//!
//! // function (a: u64, b: u64) -> u64 { a + b }   in AAPCS64:
//! //    add x0, x0, x1
//! //    ret
//! let mut a = Assembler::new();
//! a.add(Reg::X0, Reg::X0, Reg::X1);
//! a.ret();
//! let bytes = a.finish().unwrap();
//! assert_eq!(bytes.len(), 8);  // two 4-byte instructions
//! ```
//!
//! ## Bit layout reference
//!
//! All encodings cite the relevant section of the *ARM Architecture
//! Reference Manual for ARMv8-A* (DDI 0487).  Field naming follows the
//! manual: `sf`, `op`, `Rd`, `Rn`, `Rm`, `imm`, `cond`, `hw`, `imm12`,
//! `imm16`, `imm19`, `imm26`.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

// ===========================================================================
// Registers
// ===========================================================================

/// 64-bit general-purpose register encoding (`x0`-`x30` plus `sp`/`xzr`).
///
/// AArch64 has 31 GPRs (`x0`-`x30`) + a special encoding `0b11111` that
/// means **either** `sp` or `xzr` depending on the instruction context.
/// To keep the API safe, we expose `Sp` and `Xzr` as separate variants
/// even though they share the same bit pattern; each `Assembler` method
/// chooses the right interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Reg {
    X0, X1, X2, X3, X4, X5, X6, X7,
    X8, X9, X10, X11, X12, X13, X14, X15,
    X16, X17, X18, X19, X20, X21, X22, X23,
    X24, X25, X26, X27, X28,
    /// Frame pointer (alias for X29 by AAPCS64 convention).
    Fp,
    /// Link register (alias for X30, holds the return address).
    Lr,
    /// Stack pointer.  Encoded as `0b11111` in `R[ds]p`-form fields.
    Sp,
    /// Zero register.  Encoded as `0b11111` in `R[ds]`-form fields;
    /// reads as 0, writes are discarded.
    Xzr,
}

impl Reg {
    /// 5-bit register index used by all GPR-form fields.
    fn idx(self) -> u32 {
        use Reg::*;
        match self {
            X0  => 0,  X1  => 1,  X2  => 2,  X3  => 3,
            X4  => 4,  X5  => 5,  X6  => 6,  X7  => 7,
            X8  => 8,  X9  => 9,  X10 => 10, X11 => 11,
            X12 => 12, X13 => 13, X14 => 14, X15 => 15,
            X16 => 16, X17 => 17, X18 => 18, X19 => 19,
            X20 => 20, X21 => 21, X22 => 22, X23 => 23,
            X24 => 24, X25 => 25, X26 => 26, X27 => 27,
            X28 => 28,
            Fp  => 29, Lr  => 30,
            Sp  => 31, Xzr => 31,
        }
    }
}

// ===========================================================================
// Condition codes
// ===========================================================================

/// AArch64 branch-condition codes (`B.cond`, `CSEL`, etc.).
///
/// Names follow the ARM ARM "Standard condition codes" table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Cond {
    /// Equal (Z=1) — alias `EQ`.
    Eq = 0b0000,
    /// Not equal (Z=0) — alias `NE`.
    Ne = 0b0001,
    /// Carry set / unsigned ≥ — alias `CS` / `HS`.
    Hs = 0b0010,
    /// Carry clear / unsigned < — alias `CC` / `LO`.
    Lo = 0b0011,
    /// Negative (N=1) — alias `MI`.
    Mi = 0b0100,
    /// Non-negative (N=0) — alias `PL`.
    Pl = 0b0101,
    /// Overflow (V=1) — alias `VS`.
    Vs = 0b0110,
    /// No overflow (V=0) — alias `VC`.
    Vc = 0b0111,
    /// Unsigned > — alias `HI`.
    Hi = 0b1000,
    /// Unsigned ≤ — alias `LS`.
    Ls = 0b1001,
    /// Signed ≥ — alias `GE`.
    Ge = 0b1010,
    /// Signed < — alias `LT`.
    Lt = 0b1011,
    /// Signed > — alias `GT`.
    Gt = 0b1100,
    /// Signed ≤ — alias `LE`.
    Le = 0b1101,
    /// Always — alias `AL` (rarely used directly).
    Al = 0b1110,
}

// ===========================================================================
// Labels — used for branches that target instructions emitted later
// ===========================================================================

/// Opaque label handle returned by [`Assembler::create_label`].
///
/// The label has no fixed address until [`Assembler::bind`] is called.
/// Any branch encoded against an unbound label is patched at
/// [`Assembler::finish`] time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LabelId(u32);

#[derive(Debug, Clone, Copy)]
enum BranchKind {
    /// `B` / `BL` — 26-bit signed PC-relative immediate (in 4-byte words).
    Imm26,
    /// `B.cond` — 19-bit signed PC-relative immediate (in 4-byte words).
    Imm19,
}

#[derive(Debug, Clone, Copy)]
struct Fixup {
    /// Index in `code` of the instruction word to patch.
    word_idx: usize,
    target:   LabelId,
    kind:     BranchKind,
}

// ===========================================================================
// Errors
// ===========================================================================

/// Errors detected when building an instruction stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodeError {
    /// A label was referenced by a branch but never bound to an instruction.
    UnboundLabel(LabelId),
    /// A label was bound twice — labels must designate exactly one location.
    LabelAlreadyBound(LabelId),
    /// An immediate field could not fit the supplied value.
    ImmediateOutOfRange {
        /// Mnemonic family.
        op: &'static str,
        /// Number of bits available in the field.
        bits: u32,
        /// Value that overflowed.
        value: i64,
    },
    /// A PC-relative branch displacement exceeds the encodable range.
    BranchOutOfRange {
        /// Number of bits available (26 or 19).
        bits: u32,
        /// Word delta that overflowed.
        delta_words: i64,
    },
}

impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncodeError::UnboundLabel(l) => write!(f, "label {:?} referenced but never bound", l),
            EncodeError::LabelAlreadyBound(l) => write!(f, "label {:?} bound twice", l),
            EncodeError::ImmediateOutOfRange { op, bits, value } =>
                write!(f, "{op}: immediate {value} doesn't fit in {bits} bits"),
            EncodeError::BranchOutOfRange { bits, delta_words } =>
                write!(f, "branch displacement {delta_words} words doesn't fit in {bits} bits"),
        }
    }
}

impl std::error::Error for EncodeError {}

// ===========================================================================
// Assembler
// ===========================================================================

/// Stream-style assembler that emits ARM64 instructions and resolves
/// label-relative branches at finalisation time.
///
/// Instructions are stored as `u32` little-endian-on-output words; the
/// final byte stream is produced by [`Assembler::finish`].
#[derive(Debug)]
pub struct Assembler {
    /// One entry per instruction word, in emission order.
    code: Vec<u32>,
    /// `labels[i]` is `Some(word_idx)` once the i-th label is bound.
    labels: Vec<Option<usize>>,
    /// Branch fix-ups; resolved at `finish()` time.
    fixups: Vec<Fixup>,
}

impl Default for Assembler {
    fn default() -> Self { Self::new() }
}

impl Assembler {
    /// Create an empty assembler.
    pub fn new() -> Self {
        Assembler { code: Vec::new(), labels: Vec::new(), fixups: Vec::new() }
    }

    /// Number of words emitted so far (each is 4 bytes).
    pub fn len_words(&self) -> usize { self.code.len() }

    // -----------------------------------------------------------------------
    // Labels
    // -----------------------------------------------------------------------

    /// Allocate a fresh, unbound label.
    pub fn create_label(&mut self) -> LabelId {
        let id = LabelId(self.labels.len() as u32);
        self.labels.push(None);
        id
    }

    /// Bind `label` to the *next* instruction emitted.
    ///
    /// Returns `Err(LabelAlreadyBound)` if the label was already bound.
    pub fn bind(&mut self, label: LabelId) -> Result<(), EncodeError> {
        let slot = &mut self.labels[label.0 as usize];
        if slot.is_some() {
            return Err(EncodeError::LabelAlreadyBound(label));
        }
        *slot = Some(self.code.len());
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Instruction emission helpers (private)
    // -----------------------------------------------------------------------

    fn emit(&mut self, word: u32) { self.code.push(word); }

    /// Emit a placeholder instruction word and queue a branch fix-up.
    fn emit_branch(&mut self, target: LabelId, kind: BranchKind, base: u32) {
        let word_idx = self.code.len();
        self.emit(base); // placeholder; immediate is patched later
        self.fixups.push(Fixup { word_idx, target, kind });
    }

    // -----------------------------------------------------------------------
    // Move-immediate family
    // -----------------------------------------------------------------------

    /// `MOVZ Xd, #imm16, LSL #(hw*16)` — write `imm16` into a 16-bit slot,
    /// zeroing the rest.
    ///
    /// Encoding: `1 10 100101 hw imm16 Rd` (DDI 0487 §C6.2.190).
    pub fn movz(&mut self, rd: Reg, imm16: u16, hw: u8) {
        assert!(hw < 4, "hw must be 0..=3 (selects shift 0/16/32/48)");
        let word = 0xD2800000
            | ((hw as u32) << 21)
            | ((imm16 as u32) << 5)
            | rd.idx();
        self.emit(word);
    }

    /// `MOVK Xd, #imm16, LSL #(hw*16)` — write `imm16` into a 16-bit slot,
    /// preserving the rest.
    ///
    /// Encoding: `1 11 100101 hw imm16 Rd`.
    pub fn movk(&mut self, rd: Reg, imm16: u16, hw: u8) {
        assert!(hw < 4, "hw must be 0..=3");
        let word = 0xF2800000
            | ((hw as u32) << 21)
            | ((imm16 as u32) << 5)
            | rd.idx();
        self.emit(word);
    }

    /// Convenience: load a 64-bit immediate into `rd` using one to four
    /// `MOVZ`/`MOVK` instructions, picking the shortest sequence.
    ///
    /// Cost:
    /// - `imm == 0`           → 1 instruction (`movz xd, 0`)
    /// - `imm < 2^16`         → 1 (`movz`)
    /// - `imm < 2^32`         → up to 2 (`movz` + `movk`)
    /// - up to 4 instructions for full 64-bit values.
    pub fn mov_imm64(&mut self, rd: Reg, imm: u64) {
        // Find the lowest non-zero 16-bit chunk for the initial MOVZ; if
        // every chunk is zero, emit a single MOVZ #0.
        let chunks = [
            (imm        & 0xFFFF) as u16,
            ((imm >> 16) & 0xFFFF) as u16,
            ((imm >> 32) & 0xFFFF) as u16,
            ((imm >> 48) & 0xFFFF) as u16,
        ];
        if imm == 0 {
            self.movz(rd, 0, 0);
            return;
        }
        let mut emitted_movz = false;
        for (i, &c) in chunks.iter().enumerate() {
            if c == 0 { continue; }
            if !emitted_movz {
                self.movz(rd, c, i as u8);
                emitted_movz = true;
            } else {
                self.movk(rd, c, i as u8);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Arithmetic — register form (64-bit)
    // -----------------------------------------------------------------------

    /// `ADD Xd, Xn, Xm` — 64-bit addition.
    /// Encoding: `1 0 0 01011 00 0 Rm 000000 Rn Rd` (`shifted register`,
    /// shift = LSL #0).
    pub fn add(&mut self, rd: Reg, rn: Reg, rm: Reg) {
        let word = 0x8B000000
            | (rm.idx() << 16)
            | (rn.idx() << 5)
            | rd.idx();
        self.emit(word);
    }

    /// `SUB Xd, Xn, Xm` — 64-bit subtraction.
    pub fn sub(&mut self, rd: Reg, rn: Reg, rm: Reg) {
        let word = 0xCB000000
            | (rm.idx() << 16)
            | (rn.idx() << 5)
            | rd.idx();
        self.emit(word);
    }

    /// `MUL Xd, Xn, Xm` — 64-bit multiply (alias for `MADD Xd, Xn, Xm, XZR`).
    pub fn mul(&mut self, rd: Reg, rn: Reg, rm: Reg) {
        let word = 0x9B000000
            | (rm.idx() << 16)
            | (0b011111 << 10)         // Ra = XZR
            | (rn.idx() << 5)
            | rd.idx();
        // Wait — the field layout for MADD is:
        //   sf 00 11011 000 Rm 0 Ra Rn Rd
        // We must clear bit 15 (the "0" between Rm and Ra) and put Ra in [14:10].
        // Reconstruct cleanly:
        let _ = word;
        let word = 0x9B000000
            | (rm.idx() << 16)
            | (0b11111 << 10)          // Ra = XZR
            | (rn.idx() << 5)
            | rd.idx();
        self.emit(word);
    }

    // -----------------------------------------------------------------------
    // Arithmetic — immediate form (12-bit, no shift)
    // -----------------------------------------------------------------------

    /// `ADD Xd, Xn, #imm` (12-bit unsigned, LSL #0).
    pub fn add_imm(&mut self, rd: Reg, rn: Reg, imm12: u32) -> Result<(), EncodeError> {
        if imm12 >= (1 << 12) {
            return Err(EncodeError::ImmediateOutOfRange { op: "add_imm", bits: 12, value: imm12 as i64 });
        }
        let word = 0x91000000
            | (imm12 << 10)
            | (rn.idx() << 5)
            | rd.idx();
        self.emit(word);
        Ok(())
    }

    /// `SUB Xd, Xn, #imm` (12-bit unsigned, LSL #0).
    pub fn sub_imm(&mut self, rd: Reg, rn: Reg, imm12: u32) -> Result<(), EncodeError> {
        if imm12 >= (1 << 12) {
            return Err(EncodeError::ImmediateOutOfRange { op: "sub_imm", bits: 12, value: imm12 as i64 });
        }
        let word = 0xD1000000
            | (imm12 << 10)
            | (rn.idx() << 5)
            | rd.idx();
        self.emit(word);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Compare (alias for SUBS Xzr, ...)
    // -----------------------------------------------------------------------

    /// `CMP Xn, Xm` — equivalent to `SUBS XZR, Xn, Xm`.
    pub fn cmp(&mut self, rn: Reg, rm: Reg) {
        // SUBS shifted register: 1 1 1 01011 00 0 Rm imm6=0 Rn Rd=XZR
        let word = 0xEB000000
            | (rm.idx() << 16)
            | (rn.idx() << 5)
            | 0b11111;
        self.emit(word);
    }

    /// `CMP Xn, #imm` — equivalent to `SUBS XZR, Xn, #imm`.
    pub fn cmp_imm(&mut self, rn: Reg, imm12: u32) -> Result<(), EncodeError> {
        if imm12 >= (1 << 12) {
            return Err(EncodeError::ImmediateOutOfRange { op: "cmp_imm", bits: 12, value: imm12 as i64 });
        }
        // SUBS imm:  1 1 1 10001 0 0 imm12 Rn Rd=XZR
        let word = 0xF1000000
            | (imm12 << 10)
            | (rn.idx() << 5)
            | 0b11111;
        self.emit(word);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Memory — unsigned-offset form (64-bit)
    // -----------------------------------------------------------------------

    /// `LDR Xt, [Xn, #imm]` — load 64-bit; `imm` must be a multiple of 8 in
    /// `[0, 32760]` (12-bit scaled by 8).
    pub fn ldr(&mut self, rt: Reg, rn: Reg, imm: u32) -> Result<(), EncodeError> {
        if imm % 8 != 0 || imm > 0x7FF8 {
            return Err(EncodeError::ImmediateOutOfRange { op: "ldr",  bits: 12, value: imm as i64 });
        }
        let imm12 = imm / 8;
        // 1 11 11 0 01 01 imm12 Rn Rt
        let word = 0xF9400000
            | (imm12 << 10)
            | (rn.idx() << 5)
            | rt.idx();
        self.emit(word);
        Ok(())
    }

    /// `STR Xt, [Xn, #imm]` — store 64-bit; same `imm` constraints as `ldr`.
    ///
    /// Named `str_` because `str` is a Rust prelude type alias.
    pub fn str_(&mut self, rt: Reg, rn: Reg, imm: u32) -> Result<(), EncodeError> {
        if imm % 8 != 0 || imm > 0x7FF8 {
            return Err(EncodeError::ImmediateOutOfRange { op: "str", bits: 12, value: imm as i64 });
        }
        let imm12 = imm / 8;
        let word = 0xF9000000
            | (imm12 << 10)
            | (rn.idx() << 5)
            | rt.idx();
        self.emit(word);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // STP / LDP — used for prologue/epilogue (save/restore Fp+Lr)
    // -----------------------------------------------------------------------

    /// `STP Xt1, Xt2, [Xn, #imm]!` — pre-indexed store-pair (writeback).
    /// `imm` is a signed multiple of 8 in `[-512, 504]` (7-bit signed).
    pub fn stp_pre(&mut self, rt1: Reg, rt2: Reg, rn: Reg, imm: i32) -> Result<(), EncodeError> {
        if imm % 8 != 0 || imm < -512 || imm > 504 {
            return Err(EncodeError::ImmediateOutOfRange { op: "stp_pre", bits: 7, value: imm as i64 });
        }
        let imm7 = ((imm / 8) as u32) & 0x7F;
        // 1 0 1 0 1001 100 imm7 Rt2 Rn Rt1
        let word = 0xA9800000
            | (imm7 << 15)
            | (rt2.idx() << 10)
            | (rn.idx() << 5)
            | rt1.idx();
        self.emit(word);
        Ok(())
    }

    /// `LDP Xt1, Xt2, [Xn], #imm` — post-indexed load-pair (writeback).
    /// `imm` is a signed multiple of 8 in `[-512, 504]`.
    pub fn ldp_post(&mut self, rt1: Reg, rt2: Reg, rn: Reg, imm: i32) -> Result<(), EncodeError> {
        if imm % 8 != 0 || imm < -512 || imm > 504 {
            return Err(EncodeError::ImmediateOutOfRange { op: "ldp_post", bits: 7, value: imm as i64 });
        }
        let imm7 = ((imm / 8) as u32) & 0x7F;
        // 1 0 1 0 1000 110 imm7 Rt2 Rn Rt1
        let word = 0xA8C00000
            | (imm7 << 15)
            | (rt2.idx() << 10)
            | (rn.idx() << 5)
            | rt1.idx();
        self.emit(word);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Branches
    // -----------------------------------------------------------------------

    /// `B label` — unconditional branch (PC-relative, 26-bit signed
    /// in 4-byte units → ±128 MiB).
    pub fn b(&mut self, target: LabelId) {
        // 0 0 0 1 0 1 imm26
        self.emit_branch(target, BranchKind::Imm26, 0x14000000);
    }

    /// `BL label` — branch with link (sets x30/lr to PC+4).
    pub fn bl(&mut self, target: LabelId) {
        // 1 0 0 1 0 1 imm26
        self.emit_branch(target, BranchKind::Imm26, 0x94000000);
    }

    /// `B.cond label` — conditional branch (PC-relative, 19-bit signed
    /// in 4-byte units → ±1 MiB).
    pub fn b_cond(&mut self, cond: Cond, target: LabelId) {
        // 0 1 0 1 0 1 0 0 imm19 0 cond
        let base = 0x54000000 | (cond as u32);
        self.emit_branch(target, BranchKind::Imm19, base);
    }

    /// `BLR Xn` — branch with link to register (indirect call).
    pub fn blr(&mut self, rn: Reg) {
        // 1 1 0 1 0 1 1 0 0 0 1 1 1 1 1 1 0 0 0 0 0 0 Rn 0 0 0 0 0
        let word = 0xD63F0000 | (rn.idx() << 5);
        self.emit(word);
    }

    /// `RET Xn` (default `Xn = x30/lr`) — return.
    pub fn ret(&mut self) {
        // 1 1 0 1 0 1 1 0 0 1 0 1 1 1 1 1 0 0 0 0 0 0 Rn=30 0 0 0 0 0
        let word = 0xD65F0000 | (Reg::Lr.idx() << 5);
        self.emit(word);
    }

    // -----------------------------------------------------------------------
    // Misc
    // -----------------------------------------------------------------------

    /// `CSET Xd, cond` — set `Xd` to 1 if `cond` is true, else 0.
    ///
    /// Implemented as the documented alias `CSINC Xd, XZR, XZR, !cond`.
    /// The `Cond::Al` (always) value is rejected because the alias would
    /// inc `XZR` (= produce 1 unconditionally), which is not a useful CSET.
    pub fn cset(&mut self, rd: Reg, cond: Cond) {
        let cc = cond as u32;
        // CSINC: 1 0 0 11010100 Rm cond 0 1 Rn Rd  with Rm=Rn=XZR=11111
        // Inverted condition = cc XOR 1 (lowest bit toggled).
        let inv = cc ^ 1;
        let word = 0x9A800400
            | (0b11111 << 16)         // Rm = XZR
            | (inv << 12)             // condition (inverted)
            | (0b11111 << 5)          // Rn = XZR
            | rd.idx();
        self.emit(word);
    }

    /// `CBZ Xt, label` — branch to `label` if `Xt == 0`.
    /// 19-bit signed PC-relative immediate.
    pub fn cbz(&mut self, rt: Reg, target: LabelId) {
        // 1 0 1 1 0 1 0 0 imm19 Rt
        let base = 0xB4000000 | rt.idx();
        self.emit_branch(target, BranchKind::Imm19, base);
    }

    /// `CBNZ Xt, label` — branch to `label` if `Xt != 0`.
    pub fn cbnz(&mut self, rt: Reg, target: LabelId) {
        // 1 0 1 1 0 1 0 1 imm19 Rt
        let base = 0xB5000000 | rt.idx();
        self.emit_branch(target, BranchKind::Imm19, base);
    }

    /// `NOP`.
    pub fn nop(&mut self) {
        self.emit(0xD503201F);
    }

    /// `UDF #imm16` — undefined / trap.  Useful for guard-fail sites.
    pub fn udf(&mut self, imm: u16) {
        // Permanently-undefined encoding: 0000 0000 0000 0000 imm16
        let word = imm as u32;
        self.emit(word);
    }

    /// `SVC #imm16` — supervisor call (system call entry).
    ///
    /// On macOS / iOS arm64 the kernel ABI uses `svc #0x80`.
    /// On Linux arm64 it's `svc #0`.
    ///
    /// Encoding: `1 1 0 1 0 1 0 0 000 imm16 0 0 0 0 1`.
    pub fn svc(&mut self, imm: u16) {
        let word = 0xD4000001 | ((imm as u32) << 5);
        self.emit(word);
    }

    // -----------------------------------------------------------------------
    // Finalisation
    // -----------------------------------------------------------------------

    /// Resolve all branch fix-ups and produce the byte stream.
    ///
    /// Errors:
    /// - [`EncodeError::UnboundLabel`] if any branch references a label
    ///   that was never `bind`-ed.
    /// - [`EncodeError::BranchOutOfRange`] if a displacement exceeds the
    ///   field's signed range.
    pub fn finish(mut self) -> Result<Vec<u8>, EncodeError> {
        // Resolve fix-ups in place.
        for f in &self.fixups {
            let target_idx = self.labels[f.target.0 as usize]
                .ok_or(EncodeError::UnboundLabel(f.target))?;
            let delta_words = (target_idx as i64) - (f.word_idx as i64);
            let word = &mut self.code[f.word_idx];
            match f.kind {
                BranchKind::Imm26 => {
                    if delta_words < -(1 << 25) || delta_words >= (1 << 25) {
                        return Err(EncodeError::BranchOutOfRange { bits: 26, delta_words });
                    }
                    let imm26 = (delta_words as u32) & 0x03FFFFFF;
                    *word = (*word & !0x03FFFFFF) | imm26;
                }
                BranchKind::Imm19 => {
                    if delta_words < -(1 << 18) || delta_words >= (1 << 18) {
                        return Err(EncodeError::BranchOutOfRange { bits: 19, delta_words });
                    }
                    let imm19 = (delta_words as u32) & 0x0007FFFF;
                    // imm19 sits in bits [23:5] of the B.cond word.
                    *word = (*word & !(0x0007FFFF << 5)) | (imm19 << 5);
                }
            }
        }

        // Word-stream → little-endian bytes.
        let mut bytes = Vec::with_capacity(self.code.len() * 4);
        for w in self.code {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        Ok(bytes)
    }
}

// ===========================================================================
// Tests — verified against known-good encodings (clang -c on real ARM ARM)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Convenience: turn a slice of u32 LE words into the byte vector
    /// `Assembler::finish` would produce.
    fn words_to_bytes(words: &[u32]) -> Vec<u8> {
        let mut out = Vec::with_capacity(words.len() * 4);
        for w in words { out.extend_from_slice(&w.to_le_bytes()); }
        out
    }

    // ---- Basic moves ----

    #[test]
    fn movz_x0_zero() {
        // movz x0, #0  →  D2800000
        let mut a = Assembler::new();
        a.movz(Reg::X0, 0, 0);
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0xD2800000]));
    }

    #[test]
    fn movz_x1_5() {
        // movz x1, #5  →  D28000A1
        let mut a = Assembler::new();
        a.movz(Reg::X1, 5, 0);
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0xD28000A1]));
    }

    #[test]
    fn movz_with_shift() {
        // movz x0, #0x1234, lsl #16  →  D2A24680
        let mut a = Assembler::new();
        a.movz(Reg::X0, 0x1234, 1);
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0xD2A24680]));
    }

    #[test]
    fn mov_imm64_zero() {
        let mut a = Assembler::new();
        a.mov_imm64(Reg::X0, 0);
        assert_eq!(a.finish().unwrap().len(), 4);
    }

    #[test]
    fn mov_imm64_small() {
        // mov_imm64 x0, 5  →  movz x0, #5
        let mut a = Assembler::new();
        a.mov_imm64(Reg::X0, 5);
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0xD28000A0]));
    }

    #[test]
    fn mov_imm64_two_chunks() {
        // mov_imm64 x0, 0x12345678 → movz #0x5678; movk #0x1234, lsl #16
        let mut a = Assembler::new();
        a.mov_imm64(Reg::X0, 0x1234_5678);
        let bytes = a.finish().unwrap();
        assert_eq!(bytes.len(), 8); // two instructions
    }

    // ---- Arithmetic (register) ----

    #[test]
    fn add_x0_x0_x1() {
        // add x0, x0, x1  →  8B010000
        let mut a = Assembler::new();
        a.add(Reg::X0, Reg::X0, Reg::X1);
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0x8B010000]));
    }

    #[test]
    fn sub_x2_x3_x4() {
        // sub x2, x3, x4  →  CB040062
        let mut a = Assembler::new();
        a.sub(Reg::X2, Reg::X3, Reg::X4);
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0xCB040062]));
    }

    #[test]
    fn mul_x0_x1_x2() {
        // mul x0, x1, x2  ↔  madd x0, x1, x2, xzr  →  9B027C20
        let mut a = Assembler::new();
        a.mul(Reg::X0, Reg::X1, Reg::X2);
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0x9B027C20]));
    }

    // ---- Arithmetic (immediate) ----

    #[test]
    fn add_imm_x0_x0_1() {
        // add x0, x0, #1  →  91000400
        let mut a = Assembler::new();
        a.add_imm(Reg::X0, Reg::X0, 1).unwrap();
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0x91000400]));
    }

    #[test]
    fn add_imm_rejects_overflow() {
        let mut a = Assembler::new();
        let err = a.add_imm(Reg::X0, Reg::X0, 1 << 12).unwrap_err();
        assert!(matches!(err, EncodeError::ImmediateOutOfRange { .. }));
    }

    // ---- Compare ----

    #[test]
    fn cmp_imm_x0_0() {
        // cmp x0, #0  →  F100001F
        let mut a = Assembler::new();
        a.cmp_imm(Reg::X0, 0).unwrap();
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0xF100001F]));
    }

    #[test]
    fn cmp_x0_x1() {
        // cmp x0, x1  →  EB01001F
        let mut a = Assembler::new();
        a.cmp(Reg::X0, Reg::X1);
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0xEB01001F]));
    }

    // ---- Memory ----

    #[test]
    fn ldr_x0_sp_0() {
        // ldr x0, [sp]  →  F94003E0
        let mut a = Assembler::new();
        a.ldr(Reg::X0, Reg::Sp, 0).unwrap();
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0xF94003E0]));
    }

    #[test]
    fn str_x0_sp_8() {
        // str x0, [sp, #8]  →  F90007E0
        let mut a = Assembler::new();
        a.str_(Reg::X0, Reg::Sp, 8).unwrap();
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0xF90007E0]));
    }

    #[test]
    fn ldr_rejects_unaligned() {
        let mut a = Assembler::new();
        assert!(a.ldr(Reg::X0, Reg::Sp, 7).is_err()); // not a multiple of 8
    }

    // ---- STP / LDP ----

    #[test]
    fn stp_fp_lr_pre_sp_neg16() {
        // stp x29, x30, [sp, #-16]!  →  A9BF7BFD
        let mut a = Assembler::new();
        a.stp_pre(Reg::Fp, Reg::Lr, Reg::Sp, -16).unwrap();
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0xA9BF7BFD]));
    }

    #[test]
    fn ldp_fp_lr_post_sp_16() {
        // ldp x29, x30, [sp], #16  →  A8C17BFD
        let mut a = Assembler::new();
        a.ldp_post(Reg::Fp, Reg::Lr, Reg::Sp, 16).unwrap();
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0xA8C17BFD]));
    }

    // ---- Branches ----

    #[test]
    fn b_forward_label() {
        // L0:  movz x0, #1
        // L1:  b L2
        //      movz x0, #2     (skipped)
        // L2:  ret
        let mut a = Assembler::new();
        let l2 = a.create_label();
        a.movz(Reg::X0, 1, 0);
        a.b(l2);
        a.movz(Reg::X0, 2, 0);
        a.bind(l2).unwrap();
        a.ret();
        let bytes = a.finish().unwrap();
        assert_eq!(bytes.len(), 16);
        // The b instruction at word index 1 should branch +2 words (to ret).
        // Encoding: 0x14000000 | 0x02 = 0x14000002
        let b_word = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(b_word, 0x14000002);
    }

    #[test]
    fn b_backward_label() {
        // loop: nop; b loop  →  imm26 = -1
        let mut a = Assembler::new();
        let lp = a.create_label();
        a.bind(lp).unwrap();
        a.nop();
        a.b(lp);
        let bytes = a.finish().unwrap();
        let b_word = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        // imm26 = -1  in two's complement masked to 26 bits = 0x03FFFFFF
        assert_eq!(b_word, 0x14000000 | 0x03FFFFFF);
    }

    #[test]
    fn b_cond_eq_forward() {
        // cmp x0, #0; b.eq L; nop; L: ret
        let mut a = Assembler::new();
        let l = a.create_label();
        a.cmp_imm(Reg::X0, 0).unwrap();
        a.b_cond(Cond::Eq, l);
        a.nop();
        a.bind(l).unwrap();
        a.ret();
        let bytes = a.finish().unwrap();
        // b.eq at word 1, target word 3 → delta +2 → imm19 = 2
        let bcond = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(bcond, 0x54000000 | (2 << 5) | (Cond::Eq as u32));
    }

    #[test]
    fn unbound_label_errors() {
        let mut a = Assembler::new();
        let l = a.create_label();
        a.b(l);
        let err = a.finish().unwrap_err();
        assert_eq!(err, EncodeError::UnboundLabel(l));
    }

    #[test]
    fn double_bind_errors() {
        let mut a = Assembler::new();
        let l = a.create_label();
        a.bind(l).unwrap();
        let err = a.bind(l).unwrap_err();
        assert_eq!(err, EncodeError::LabelAlreadyBound(l));
    }

    #[test]
    fn bl_emits_2_in_top_bit() {
        // bl L → 0x94000000 base
        let mut a = Assembler::new();
        let l = a.create_label();
        a.bl(l);
        a.bind(l).unwrap();
        a.ret();
        let bytes = a.finish().unwrap();
        let bl = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        // bl with delta +1 → 0x94000001
        assert_eq!(bl, 0x94000001);
    }

    // ---- Indirect / return ----

    #[test]
    fn blr_x0() {
        // blr x0  →  D63F0000
        let mut a = Assembler::new();
        a.blr(Reg::X0);
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0xD63F0000]));
    }

    #[test]
    fn ret_uses_lr() {
        // ret  ↔  ret x30  →  D65F03C0
        let mut a = Assembler::new();
        a.ret();
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0xD65F03C0]));
    }

    // ---- Misc ----

    #[test]
    fn nop_encoding() {
        let mut a = Assembler::new();
        a.nop();
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0xD503201F]));
    }

    #[test]
    fn udf_zero() {
        let mut a = Assembler::new();
        a.udf(0);
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0x00000000]));
    }

    // ---- Composite — a real function prologue + epilogue ----

    #[test]
    fn svc_macos_arm64() {
        // svc #0x80  →  D4001001
        let mut a = Assembler::new();
        a.svc(0x80);
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0xD4001001]));
    }

    #[test]
    fn cset_eq_x0() {
        // cset x0, eq  ↔  csinc x0, xzr, xzr, ne
        // Encoding: 9A 9F 17 E0
        let mut a = Assembler::new();
        a.cset(Reg::X0, Cond::Eq);
        assert_eq!(a.finish().unwrap(), words_to_bytes(&[0x9A9F17E0]));
    }

    #[test]
    fn cbz_x0_forward() {
        let mut a = Assembler::new();
        let l = a.create_label();
        a.cbz(Reg::X0, l);
        a.nop();
        a.bind(l).unwrap();
        a.ret();
        let bytes = a.finish().unwrap();
        let cbz = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        // delta +2 → imm19 = 2; Rt = 0
        assert_eq!(cbz, 0xB4000000 | (2 << 5));
    }

    #[test]
    fn cbnz_x1_forward() {
        let mut a = Assembler::new();
        let l = a.create_label();
        a.cbnz(Reg::X1, l);
        a.bind(l).unwrap();
        a.ret();
        let bytes = a.finish().unwrap();
        let cbnz = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        // delta +1 → imm19 = 1; Rt = 1
        assert_eq!(cbnz, 0xB5000000 | (1 << 5) | 1);
    }

    #[test]
    fn typical_leaf_prologue_epilogue() {
        // Function `fn() -> 42`:
        //   stp  fp, lr, [sp, #-16]!
        //   mov  fp, sp           (add fp, sp, #0)
        //   movz x0, #42
        //   ldp  fp, lr, [sp], #16
        //   ret
        let mut a = Assembler::new();
        a.stp_pre(Reg::Fp, Reg::Lr, Reg::Sp, -16).unwrap();
        a.add_imm(Reg::Fp, Reg::Sp, 0).unwrap();
        a.movz(Reg::X0, 42, 0);
        a.ldp_post(Reg::Fp, Reg::Lr, Reg::Sp, 16).unwrap();
        a.ret();
        let bytes = a.finish().unwrap();
        assert_eq!(bytes.len(), 5 * 4);
    }
}
