//! CIL (Common Intermediate Language) bytecode builder.
//!
//! The CLR stores executable method bodies as CIL byte streams. Most opcodes
//! are one byte; a few use the `0xFE` prefix byte for extended opcodes like
//! `ceq`, `cgt`, and `clt`. Branch operands are signed offsets **relative to
//! the instruction immediately after the branch**, making them tricky to
//! compute when labels are forward references.
//!
//! This module implements a two-pass assembler:
//!
//! 1. **Building phase** — caller emits opcodes and deferred branches via the
//!    builder methods. Branches are stored as unresolved `BranchRef` items.
//! 2. **Assembly phase** (`assemble()`) — measures the assembled size with
//!    initial short-branch estimates, detects out-of-range short branches,
//!    promotes them to long form, and repeats until stable. Finally, encodes
//!    every item to produce the finished `Vec<u8>`.
//!
//! # Example
//!
//! ```
//! use ir_to_cil_bytecode::builder::{CILBytecodeBuilder, CILBranchKind};
//!
//! let mut b = CILBytecodeBuilder::new();
//! b.emit_ldc_i4(42);
//! b.emit_ret();
//! let bytes = b.assemble().unwrap();
//! assert_eq!(bytes, vec![0x1F, 42, 0x2A]); // ldc.i4.s 42; ret
//! ```

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Minimum value of a signed 8-bit integer (used for `ldc.i4.s` range check).
const INT8_MIN: i32 = -128;
/// Maximum value of a signed 8-bit integer.
const INT8_MAX: i32 = 127;

// ---------------------------------------------------------------------------
// CILBuilderError
// ---------------------------------------------------------------------------

/// Error raised when a CIL method body cannot be assembled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CILBuilderError(pub String);

impl std::fmt::Display for CILBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CILBuilderError: {}", self.0)
    }
}

// ---------------------------------------------------------------------------
// CILOpcode
// ---------------------------------------------------------------------------

/// CIL opcodes used by the compiler-backend MVP.
///
/// The value is the **first opcode byte**. Two-byte opcodes (those prefixed by
/// `0xFE` — `ceq`, `cgt`, `clt`) are emitted by dedicated helper methods
/// because their interesting byte is the second one.
///
/// # Two-byte opcode layout
///
/// ```text
/// 0xFE 0x01 → ceq   (compare equal)
/// 0xFE 0x02 → cgt   (compare greater-than signed)
/// 0xFE 0x04 → clt   (compare less-than signed)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CILOpcode {
    Nop       = 0x00,
    LdArg0    = 0x02,
    LdArg1    = 0x03,
    LdArg2    = 0x04,
    LdArg3    = 0x05,
    LdLoc0    = 0x06,
    LdLoc1    = 0x07,
    LdLoc2    = 0x08,
    LdLoc3    = 0x09,
    StLoc0    = 0x0A,
    StLoc1    = 0x0B,
    StLoc2    = 0x0C,
    StLoc3    = 0x0D,
    LdArgS    = 0x0E,
    StArgS    = 0x10,
    LdLocS    = 0x11,
    StLocS    = 0x13,
    LdcI4M1   = 0x15,
    LdcI40    = 0x16,
    LdcI41    = 0x17,
    LdcI42    = 0x18,
    LdcI43    = 0x19,
    LdcI44    = 0x1A,
    LdcI45    = 0x1B,
    LdcI46    = 0x1C,
    LdcI47    = 0x1D,
    LdcI48    = 0x1E,
    LdcI4S    = 0x1F,
    LdcI4     = 0x20,
    Dup       = 0x25,
    Pop       = 0x26,
    Call      = 0x28,
    Ret       = 0x2A,
    BrS       = 0x2B,
    BrFalseS  = 0x2C,
    BrTrueS   = 0x2D,
    BeqS      = 0x2E,
    BgeS      = 0x2F,
    BgtS      = 0x30,
    BleS      = 0x31,
    BltS      = 0x32,
    BneUnS    = 0x33,
    Br        = 0x38,
    BrFalse   = 0x39,
    BrTrue    = 0x3A,
    Beq       = 0x3B,
    Bge       = 0x3C,
    Bgt       = 0x3D,
    Ble       = 0x3E,
    Blt       = 0x3F,
    BneUn     = 0x40,
    Add       = 0x58,
    Sub       = 0x59,
    Mul       = 0x5A,
    Div       = 0x5B,
    And       = 0x5F,
    Or        = 0x60,
    Xor       = 0x61,
    Shl       = 0x62,
    Shr       = 0x63,
    CallVirt  = 0x6F,
    LdSFld    = 0x7E,
    StSFld    = 0x80,
    NewArr    = 0x8D,
    LdElemU1  = 0x91,
    LdElemI4  = 0x94,
    StElemI1  = 0x9C,
    StElemI4  = 0x9E,
    PrefixFe  = 0xFE,
}

// ---------------------------------------------------------------------------
// CILBranchKind
// ---------------------------------------------------------------------------

/// Branch families, each with short (8-bit) and long (32-bit) encodings.
///
/// The short form uses a 1-byte signed offset (-128..127), the long form uses
/// a 4-byte signed offset. The assembler automatically picks the short form
/// where possible and promotes to long if a branch would be out of range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CILBranchKind {
    /// Unconditional branch (`br`).
    Always,
    /// Branch if false / zero (`brfalse`).
    False,
    /// Branch if true / nonzero (`brtrue`).
    True,
    /// Branch if equal (`beq`).
    Eq,
    /// Branch if greater-or-equal (`bge`).
    Ge,
    /// Branch if greater-than (`bgt`).
    Gt,
    /// Branch if less-or-equal (`ble`).
    Le,
    /// Branch if less-than (`blt`).
    Lt,
    /// Branch if not equal, unsigned (`bne.un`).
    NeUn,
}

/// Short and long opcode bytes for each branch kind.
/// Returns `(short_opcode, long_opcode)`.
fn branch_opcodes(kind: CILBranchKind) -> (u8, u8) {
    match kind {
        CILBranchKind::Always => (CILOpcode::BrS  as u8, CILOpcode::Br     as u8),
        CILBranchKind::False  => (CILOpcode::BrFalseS as u8, CILOpcode::BrFalse as u8),
        CILBranchKind::True   => (CILOpcode::BrTrueS  as u8, CILOpcode::BrTrue  as u8),
        CILBranchKind::Eq     => (CILOpcode::BeqS  as u8, CILOpcode::Beq  as u8),
        CILBranchKind::Ge     => (CILOpcode::BgeS  as u8, CILOpcode::Bge  as u8),
        CILBranchKind::Gt     => (CILOpcode::BgtS  as u8, CILOpcode::Bgt  as u8),
        CILBranchKind::Le     => (CILOpcode::BleS  as u8, CILOpcode::Ble  as u8),
        CILBranchKind::Lt     => (CILOpcode::BltS  as u8, CILOpcode::Blt  as u8),
        CILBranchKind::NeUn   => (CILOpcode::BneUnS as u8, CILOpcode::BneUn as u8),
    }
}

// ---------------------------------------------------------------------------
// Builder items
// ---------------------------------------------------------------------------

/// An item in the builder's output stream.
///
/// The stream is a flat `Vec<Item>` that the assembler walks twice: once to
/// measure sizes (and resolve label offsets), once to emit bytes.
#[derive(Debug, Clone)]
enum Item {
    /// An already-encoded sequence of bytes (opcodes, operands, or helpers).
    Raw(Vec<u8>),
    /// A named label anchored at this position (no bytes emitted).
    Label(String),
    /// A branch instruction whose target label is not yet resolved.
    Branch {
        kind: CILBranchKind,
        target: String,
        /// If `true`, always use the 5-byte long form (opcode + i32 offset).
        force_long: bool,
    },
}

impl Item {
    /// Number of bytes this item emits, given the current `is_long` set.
    fn byte_len(&self, long_branches: &std::collections::HashSet<usize>, idx: usize) -> usize {
        match self {
            Item::Raw(b) => b.len(),
            Item::Label(_) => 0,
            Item::Branch { force_long, .. } => {
                if *force_long || long_branches.contains(&idx) {
                    5 // opcode + i32
                } else {
                    2 // opcode + i8
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Encoding helpers (public)
// ---------------------------------------------------------------------------

/// Encode the most compact `ldc.i4` instruction for an `i32` value.
///
/// The CLR has short forms for -1 through 8 (`ldc.i4.m1` through `ldc.i4.8`),
/// a sign-extended byte form for -128..127 (`ldc.i4.s`), and the full 5-byte
/// form for everything else (`ldc.i4`).
///
/// # Examples
///
/// ```
/// use ir_to_cil_bytecode::builder::encode_ldc_i4;
/// assert_eq!(encode_ldc_i4(0),   vec![0x16]);       // ldc.i4.0
/// assert_eq!(encode_ldc_i4(8),   vec![0x1E]);       // ldc.i4.8
/// assert_eq!(encode_ldc_i4(100), vec![0x1F, 100]);  // ldc.i4.s 100
/// assert_eq!(encode_ldc_i4(256), vec![0x20, 0, 1, 0, 0]); // ldc.i4 256
/// ```
pub fn encode_ldc_i4(value: i32) -> Vec<u8> {
    match value {
        -1       => vec![CILOpcode::LdcI4M1 as u8],
        0        => vec![CILOpcode::LdcI40  as u8],
        1        => vec![CILOpcode::LdcI41  as u8],
        2        => vec![CILOpcode::LdcI42  as u8],
        3        => vec![CILOpcode::LdcI43  as u8],
        4        => vec![CILOpcode::LdcI44  as u8],
        5        => vec![CILOpcode::LdcI45  as u8],
        6        => vec![CILOpcode::LdcI46  as u8],
        7        => vec![CILOpcode::LdcI47  as u8],
        8        => vec![CILOpcode::LdcI48  as u8],
        v if v >= INT8_MIN && v <= INT8_MAX => {
            vec![CILOpcode::LdcI4S as u8, v as u8]
        }
        v => {
            let mut out = vec![CILOpcode::LdcI4 as u8];
            out.extend_from_slice(&v.to_le_bytes());
            out
        }
    }
}

/// Encode the shortest `ldloc` instruction for a local variable index.
///
/// Indices 0–3 use the short 1-byte form (`ldloc.0`–`ldloc.3`), indices
/// 4–65535 use the 2-byte `ldloc.s` form.
///
/// # Panics
///
/// Panics if `index > 65535` (the CLR limit for local variable indices).
pub fn encode_ldloc(index: u16) -> Vec<u8> {
    match index {
        0 => vec![CILOpcode::LdLoc0 as u8],
        1 => vec![CILOpcode::LdLoc1 as u8],
        2 => vec![CILOpcode::LdLoc2 as u8],
        3 => vec![CILOpcode::LdLoc3 as u8],
        i => vec![CILOpcode::LdLocS as u8, i as u8],
    }
}

/// Encode the shortest `stloc` instruction for a local variable index.
///
/// Mirrors [`encode_ldloc`].
pub fn encode_stloc(index: u16) -> Vec<u8> {
    match index {
        0 => vec![CILOpcode::StLoc0 as u8],
        1 => vec![CILOpcode::StLoc1 as u8],
        2 => vec![CILOpcode::StLoc2 as u8],
        3 => vec![CILOpcode::StLoc3 as u8],
        i => vec![CILOpcode::StLocS as u8, i as u8],
    }
}

/// Encode the shortest `ldarg` instruction for a method parameter index.
///
/// Indices 0–3 use the 1-byte form; 4–255 use the 2-byte `ldarg.s` form.
pub fn encode_ldarg(index: u8) -> Vec<u8> {
    match index {
        0 => vec![CILOpcode::LdArg0 as u8],
        1 => vec![CILOpcode::LdArg1 as u8],
        2 => vec![CILOpcode::LdArg2 as u8],
        3 => vec![CILOpcode::LdArg3 as u8],
        i => vec![CILOpcode::LdArgS as u8, i],
    }
}

/// Encode `starg.s` for a parameter index (always 2 bytes; no short form).
pub fn encode_starg(index: u8) -> Vec<u8> {
    vec![CILOpcode::StArgS as u8, index]
}

/// Encode a 4-byte little-endian metadata token (used by `call`, `ldsfld`, …).
pub fn encode_metadata_token(token: u32) -> [u8; 4] {
    token.to_le_bytes()
}

/// Encode a 4-byte little-endian signed int32.
pub fn encode_i4(value: i32) -> [u8; 4] {
    value.to_le_bytes()
}

// ---------------------------------------------------------------------------
// CILBytecodeBuilder
// ---------------------------------------------------------------------------

/// Two-pass CIL method-body assembler.
///
/// Caller emits opcodes and deferred branches via the `emit_*` methods.
/// [`assemble`][CILBytecodeBuilder::assemble] performs the two-pass branch-
/// promotion algorithm and returns a fully encoded `Vec<u8>`.
///
/// # Two-pass algorithm
///
/// Short branches (`br.s`, `brfalse.s`, etc.) are 2 bytes (opcode + i8).
/// Long branches (`br`, `brfalse`, etc.) are 5 bytes (opcode + i32).
///
/// 1. Start with all branches short (or long if `force_long` is set).
/// 2. Measure every item to compute label offsets.
/// 3. For each short branch, check if the target offset fits in i8.
///    If not, promote it to long.
/// 4. Repeat from 2 until no promotions happen in a full pass.
///    (Termination is guaranteed because widths only increase.)
/// 5. Final pass: emit all bytes.
#[derive(Debug, Default)]
pub struct CILBytecodeBuilder {
    items: Vec<Item>,
}

impl CILBytecodeBuilder {
    /// Create a new, empty builder.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    // ── Primitive emitters ────────────────────────────────────────────────

    /// Mark a label at the current position (emits no bytes).
    pub fn mark(&mut self, label: impl Into<String>) {
        self.items.push(Item::Label(label.into()));
    }

    /// Append pre-encoded bytes verbatim.
    pub fn emit_raw(&mut self, data: Vec<u8>) {
        if !data.is_empty() {
            self.items.push(Item::Raw(data));
        }
    }

    /// Emit a single-byte opcode with no operand.
    pub fn emit_opcode(&mut self, opcode: CILOpcode) {
        self.emit_raw(vec![opcode as u8]);
    }

    // ── Constant loads ────────────────────────────────────────────────────

    /// Emit the shortest `ldc.i4` sequence for an `i32` immediate.
    pub fn emit_ldc_i4(&mut self, value: i32) {
        self.emit_raw(encode_ldc_i4(value));
    }

    // ── Local variable access ─────────────────────────────────────────────

    /// Emit the shortest `ldloc` for the given local index.
    pub fn emit_ldloc(&mut self, index: u16) {
        self.emit_raw(encode_ldloc(index));
    }

    /// Emit the shortest `stloc` for the given local index.
    pub fn emit_stloc(&mut self, index: u16) {
        self.emit_raw(encode_stloc(index));
    }

    // ── Argument access ───────────────────────────────────────────────────

    /// Emit the shortest `ldarg` for the given parameter index.
    pub fn emit_ldarg(&mut self, index: u8) {
        self.emit_raw(encode_ldarg(index));
    }

    /// Emit `starg.s idx`.
    pub fn emit_starg(&mut self, index: u8) {
        self.emit_raw(encode_starg(index));
    }

    // ── Token instructions ────────────────────────────────────────────────

    /// Emit a 5-byte instruction of the form `opcode + token (LE u32)`.
    ///
    /// Used for `call`, `callvirt`, `ldsfld`, `stsfld`, `newarr`.
    pub fn emit_token_instruction(&mut self, opcode: CILOpcode, token: u32) {
        let mut v = vec![opcode as u8];
        v.extend_from_slice(&encode_metadata_token(token));
        self.emit_raw(v);
    }

    /// Emit `call <method_token>`.
    pub fn emit_call(&mut self, token: u32) {
        self.emit_token_instruction(CILOpcode::Call, token);
    }

    /// Emit `callvirt <method_token>`.
    pub fn emit_callvirt(&mut self, token: u32) {
        self.emit_token_instruction(CILOpcode::CallVirt, token);
    }

    /// Emit `ldsfld <field_token>`.
    pub fn emit_ldsfld(&mut self, token: u32) {
        self.emit_token_instruction(CILOpcode::LdSFld, token);
    }

    /// Emit `stsfld <field_token>`.
    pub fn emit_stsfld(&mut self, token: u32) {
        self.emit_token_instruction(CILOpcode::StSFld, token);
    }

    /// Emit `newarr <type_token>`.
    pub fn emit_newarr(&mut self, token: u32) {
        self.emit_token_instruction(CILOpcode::NewArr, token);
    }

    // ── Comparison opcodes (two-byte, 0xFE prefix) ────────────────────────

    /// Emit `ceq` (0xFE 0x01): push 1 if top two values are equal, else 0.
    pub fn emit_ceq(&mut self) {
        self.emit_raw(vec![CILOpcode::PrefixFe as u8, 0x01]);
    }

    /// Emit `cgt` (0xFE 0x02): push 1 if stack[-2] > stack[-1] (signed).
    pub fn emit_cgt(&mut self) {
        self.emit_raw(vec![CILOpcode::PrefixFe as u8, 0x02]);
    }

    /// Emit `clt` (0xFE 0x04): push 1 if stack[-2] < stack[-1] (signed).
    pub fn emit_clt(&mut self) {
        self.emit_raw(vec![CILOpcode::PrefixFe as u8, 0x04]);
    }

    // ── Arithmetic / bitwise ──────────────────────────────────────────────

    /// Emit `add`.
    pub fn emit_add(&mut self) { self.emit_opcode(CILOpcode::Add); }
    /// Emit `sub`.
    pub fn emit_sub(&mut self) { self.emit_opcode(CILOpcode::Sub); }
    /// Emit `mul`.
    pub fn emit_mul(&mut self) { self.emit_opcode(CILOpcode::Mul); }
    /// Emit `div`.
    pub fn emit_div(&mut self) { self.emit_opcode(CILOpcode::Div); }
    /// Emit `and`.
    pub fn emit_and(&mut self) { self.emit_opcode(CILOpcode::And); }
    /// Emit `or`.
    pub fn emit_or(&mut self)  { self.emit_opcode(CILOpcode::Or);  }
    /// Emit `xor`.
    pub fn emit_xor(&mut self) { self.emit_opcode(CILOpcode::Xor); }
    /// Emit `shl`.
    pub fn emit_shl(&mut self) { self.emit_opcode(CILOpcode::Shl); }
    /// Emit `shr`.
    pub fn emit_shr(&mut self) { self.emit_opcode(CILOpcode::Shr); }
    /// Emit `dup`.
    pub fn emit_dup(&mut self) { self.emit_opcode(CILOpcode::Dup); }
    /// Emit `pop`.
    pub fn emit_pop(&mut self) { self.emit_opcode(CILOpcode::Pop); }
    /// Emit `nop`.
    pub fn emit_nop(&mut self) { self.emit_opcode(CILOpcode::Nop); }
    /// Emit `ret`.
    pub fn emit_ret(&mut self) { self.emit_opcode(CILOpcode::Ret); }

    // ── Branches ──────────────────────────────────────────────────────────

    /// Emit a branch to `target` (short or long, resolved at assembly time).
    ///
    /// If `force_long` is `true`, always use the 5-byte long form regardless
    /// of distance.
    pub fn emit_branch(&mut self, kind: CILBranchKind, target: impl Into<String>, force_long: bool) {
        self.items.push(Item::Branch {
            kind,
            target: target.into(),
            force_long,
        });
    }

    // ── Assembly ──────────────────────────────────────────────────────────

    /// Assemble all emitted items into a finished `Vec<u8>`.
    ///
    /// Performs the two-pass branch-promotion algorithm described in the
    /// module-level docs.
    ///
    /// # Errors
    ///
    /// Returns [`CILBuilderError`] if a branch references an undefined label,
    /// or if a branch offset overflows an `i32`.
    pub fn assemble(&self) -> Result<Vec<u8>, CILBuilderError> {
        // ── Pass 1: start with all branches in short form ─────────────────
        let mut long_set: std::collections::HashSet<usize> =
            self.items
                .iter()
                .enumerate()
                .filter_map(|(i, item)| {
                    if let Item::Branch { force_long: true, .. } = item {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect();

        // ── Passes 2+: promote short branches that are out of range ───────
        loop {
            let offsets = self.compute_label_offsets(&long_set);
            let mut promoted = false;
            for (i, item) in self.items.iter().enumerate() {
                let Item::Branch { target, force_long, .. } = item else { continue };
                if *force_long || long_set.contains(&i) {
                    continue; // already long
                }
                let &target_off = offsets.get(target.as_str())
                    .ok_or_else(|| CILBuilderError(format!("undefined label: {target}")))?;
                let instr_end = self.offset_after_item(i, &long_set);
                let delta = target_off as i64 - instr_end as i64;
                if delta < INT8_MIN as i64 || delta > INT8_MAX as i64 {
                    long_set.insert(i);
                    promoted = true;
                }
            }
            if !promoted { break; }
        }

        // ── Final pass: encode ────────────────────────────────────────────
        let offsets = self.compute_label_offsets(&long_set);
        let mut out = Vec::new();
        for (i, item) in self.items.iter().enumerate() {
            match item {
                Item::Raw(bytes) => out.extend_from_slice(bytes),
                Item::Label(_) => {} // no bytes
                Item::Branch { kind, target, force_long } => {
                    let &target_off = offsets.get(target.as_str())
                        .ok_or_else(|| CILBuilderError(format!("undefined label: {target}")))?;
                    let is_long = *force_long || long_set.contains(&i);
                    let (short_op, long_op) = branch_opcodes(*kind);
                    let instr_end = self.offset_after_item(i, &long_set);
                    let delta = target_off as i64 - instr_end as i64;
                    if is_long {
                        let d32 = i32::try_from(delta).map_err(|_| {
                            CILBuilderError(format!("branch offset overflow to label {target}"))
                        })?;
                        out.push(long_op);
                        out.extend_from_slice(&d32.to_le_bytes());
                    } else {
                        let d8 = i8::try_from(delta).map_err(|_| {
                            CILBuilderError(format!("short branch overflow to label {target}"))
                        })?;
                        out.push(short_op);
                        out.push(d8 as u8);
                    }
                }
            }
        }
        Ok(out)
    }

    // ── Internal helpers ──────────────────────────────────────────────────

    /// Compute a map from label name → byte offset in the output, given the
    /// current set of long-form branch indices.
    fn compute_label_offsets(
        &self,
        long_set: &std::collections::HashSet<usize>,
    ) -> HashMap<&str, usize> {
        let mut offset = 0usize;
        let mut map = HashMap::new();
        for (i, item) in self.items.iter().enumerate() {
            if let Item::Label(name) = item {
                map.insert(name.as_str(), offset);
            } else {
                offset += item.byte_len(long_set, i);
            }
        }
        map
    }

    /// Byte offset of the instruction immediately after item `idx` (i.e. the
    /// branch's reference point for relative offsets).
    fn offset_after_item(&self, idx: usize, long_set: &std::collections::HashSet<usize>) -> usize {
        let mut offset = 0usize;
        for (i, item) in self.items.iter().enumerate() {
            if let Item::Label(_) = item {
                continue;
            }
            offset += item.byte_len(long_set, i);
            if i == idx {
                return offset;
            }
        }
        offset
    }
}

// ===========================================================================
// Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_ldc_i4_short_forms() {
        assert_eq!(encode_ldc_i4(-1), vec![0x15]);
        assert_eq!(encode_ldc_i4(0),  vec![0x16]);
        assert_eq!(encode_ldc_i4(1),  vec![0x17]);
        assert_eq!(encode_ldc_i4(8),  vec![0x1E]);
    }

    #[test]
    fn test_encode_ldc_i4_byte_form() {
        assert_eq!(encode_ldc_i4(100), vec![0x1F, 100]);
        assert_eq!(encode_ldc_i4(-128), vec![0x1F, 0x80_u8]);
        assert_eq!(encode_ldc_i4(127),  vec![0x1F, 0x7F]);
    }

    #[test]
    fn test_encode_ldc_i4_full_form() {
        let enc = encode_ldc_i4(256);
        assert_eq!(enc[0], 0x20); // ldc.i4
        let val = i32::from_le_bytes(enc[1..5].try_into().unwrap());
        assert_eq!(val, 256);
    }

    #[test]
    fn test_encode_ldloc() {
        assert_eq!(encode_ldloc(0), vec![0x06]);
        assert_eq!(encode_ldloc(1), vec![0x07]);
        assert_eq!(encode_ldloc(3), vec![0x09]);
        assert_eq!(encode_ldloc(4), vec![0x11, 4]);
    }

    #[test]
    fn test_encode_stloc() {
        assert_eq!(encode_stloc(0), vec![0x0A]);
        assert_eq!(encode_stloc(3), vec![0x0D]);
        assert_eq!(encode_stloc(4), vec![0x13, 4]);
    }

    #[test]
    fn test_encode_ldarg() {
        assert_eq!(encode_ldarg(0), vec![0x02]);
        assert_eq!(encode_ldarg(3), vec![0x05]);
        assert_eq!(encode_ldarg(4), vec![0x0E, 4]);
    }

    #[test]
    fn test_assemble_simple_ret() {
        let mut b = CILBytecodeBuilder::new();
        b.emit_ldc_i4(42);
        b.emit_ret();
        let bytes = b.assemble().unwrap();
        // ldc.i4.s 42 = [0x1F, 42]; ret = [0x2A]
        assert_eq!(bytes, vec![0x1F, 42, 0x2A]);
    }

    #[test]
    fn test_assemble_forward_branch() {
        let mut b = CILBytecodeBuilder::new();
        b.emit_branch(CILBranchKind::Always, "end", false);
        b.emit_ldc_i4(0); // unreachable
        b.mark("end");
        b.emit_ret();
        let bytes = b.assemble().unwrap();
        // br.s +1; ldc.i4.0; ret
        assert_eq!(bytes[0], 0x2B); // br.s
        let offset = bytes[1] as i8;
        assert_eq!(offset, 1); // skip the ldc.i4.0
    }

    #[test]
    fn test_assemble_backward_branch() {
        let mut b = CILBytecodeBuilder::new();
        b.mark("start");
        b.emit_nop();
        b.emit_branch(CILBranchKind::Always, "start", false);
        let bytes = b.assemble().unwrap();
        assert_eq!(bytes[0], 0x00); // nop
        assert_eq!(bytes[1], 0x2B); // br.s
        let offset = bytes[2] as i8;
        assert_eq!(offset, -3); // branch back past [nop, br.s, offset] = 3 bytes
    }

    #[test]
    fn test_assemble_undefined_label_error() {
        let mut b = CILBytecodeBuilder::new();
        b.emit_branch(CILBranchKind::Always, "missing", false);
        b.emit_ret();
        let result = b.assemble();
        assert!(result.is_err());
        assert!(result.unwrap_err().0.contains("missing"));
    }

    #[test]
    fn test_assemble_long_branch_forced() {
        let mut b = CILBytecodeBuilder::new();
        b.emit_branch(CILBranchKind::Always, "end", true); // force long
        b.mark("end");
        b.emit_ret();
        let bytes = b.assemble().unwrap();
        assert_eq!(bytes[0], 0x38); // br (long form)
        let delta = i32::from_le_bytes(bytes[1..5].try_into().unwrap());
        assert_eq!(delta, 0); // jump to end which is 0 bytes away
        assert_eq!(bytes[5], 0x2A); // ret
    }

    #[test]
    fn test_emit_comparison_opcodes() {
        let mut b = CILBytecodeBuilder::new();
        b.emit_ceq();
        b.emit_cgt();
        b.emit_clt();
        let bytes = b.assemble().unwrap();
        assert_eq!(bytes, vec![0xFE, 0x01, 0xFE, 0x02, 0xFE, 0x04]);
    }

    #[test]
    fn test_emit_call_token() {
        let mut b = CILBytecodeBuilder::new();
        b.emit_call(0x06000001);
        let bytes = b.assemble().unwrap();
        assert_eq!(bytes[0], 0x28); // call
        let token = u32::from_le_bytes(bytes[1..5].try_into().unwrap());
        assert_eq!(token, 0x06000001);
    }

    #[test]
    fn test_emit_conditional_branch() {
        let mut b = CILBytecodeBuilder::new();
        b.emit_branch(CILBranchKind::True, "taken", false);
        b.emit_ldc_i4(0);
        b.mark("taken");
        b.emit_ret();
        let bytes = b.assemble().unwrap();
        assert_eq!(bytes[0], 0x2D); // brtrue.s
    }
}
