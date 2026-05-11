//! WASM binary encoding helpers.
//!
//! This module centralises all raw-byte emission for WASM instructions.
//! It exists to keep the lowering logic in `lower.rs` readable: instead of
//! sprinkling magic hex values throughout the codegen, every opcode is a
//! named constant, and every encoding operation is a small helper function.
//!
//! ## WASM binary format reminder
//!
//! A WASM function body is a flat sequence of bytes.  Each instruction starts
//! with a one-byte opcode, followed by zero or more immediates.  Immediates
//! are encoded as:
//!
//! - **Unsigned LEB128** — for depths, local indices, function indices, and
//!   vector counts.
//! - **Signed LEB128** — for integer constants (`i32.const`, `i64.const`).
//! - **Little-endian IEEE 754** — for `f32.const` (4 bytes) and `f64.const`
//!   (8 bytes).
//!
//! Opcodes themselves are **raw bytes** — NOT LEB128.  Even the byte `0x81`
//! (`i64.rem_s`) is just pushed as a single `u8`.
//!
//! ## WASM operand stack model
//!
//! WASM is a **stack machine**.  Instructions consume operands from the top of
//! the stack and push results.  For example, `i32.add` pops two `i32` values
//! and pushes one `i32`.
//!
//! IIR, on the other hand, is a **register machine** — each instruction names
//! its source and destination variables.  The bridge: we use `local.get` to
//! push IIR variables onto the WASM stack, then `local.set` to pop the result
//! back into a WASM local.
//!
//! ```text
//! IIR:  v2 = add(v0, v1) : i32
//!
//! WASM:
//!   local.get 0    ;; push v0 (local index 0)
//!   local.get 1    ;; push v1 (local index 1)
//!   i32.add        ;; pop both, push sum
//!   local.set 2    ;; pop sum into v2 (local index 2)
//! ```

use wasm_leb128::{encode_signed, encode_unsigned};

// ---------------------------------------------------------------------------
// WASM opcode constants
// ---------------------------------------------------------------------------
//
// These are the raw byte values from the WASM 1.0 binary encoding spec.
// Source: https://webassembly.github.io/spec/core/binary/instructions.html

// ── Control flow ──────────────────────────────────────────────────────────

/// `unreachable` (0x00) — trap unconditionally.
pub const UNREACHABLE: u8 = 0x00;

/// `nop` (0x01) — no operation.
pub const NOP: u8 = 0x01;

/// `block` (0x02) — begin a block with a label.
pub const BLOCK: u8 = 0x02;

/// `loop` (0x03) — begin a loop (branch target at top).
pub const LOOP: u8 = 0x03;

/// `if` (0x04) — begin an if block (condition popped from stack).
pub const IF: u8 = 0x04;

/// `else` (0x05) — else clause of an if block.
pub const ELSE: u8 = 0x05;

/// `end` (0x0B) — end of a block, loop, if, or function.
pub const END: u8 = 0x0B;

/// `br` (0x0C) — unconditional branch to a label depth.
pub const BR: u8 = 0x0C;

/// `br_if` (0x0D) — conditional branch if top-of-stack is non-zero.
pub const BR_IF: u8 = 0x0D;

/// `br_table` (0x0E) — jump table: pop index, branch to targets[index]
/// or default if out-of-range.
pub const BR_TABLE: u8 = 0x0E;

/// `return` (0x0F) — return from the current function.
pub const RETURN: u8 = 0x0F;

/// `call` (0x10) — call a function by index.
pub const CALL: u8 = 0x10;

// ── Block type tag ────────────────────────────────────────────────────────

/// Empty block type (0x40) — the block produces no value.
///
/// In the WASM binary, `block 0x40` means "this block pushes nothing on exit".
/// This is the most common case for control-flow blocks that we use for the
/// dispatch-loop pattern.
pub const BLOCK_EMPTY: u8 = 0x40;

// ── Variable access ───────────────────────────────────────────────────────

/// `local.get` (0x20) — push a local variable onto the stack.
pub const LOCAL_GET: u8 = 0x20;

/// `local.set` (0x21) — pop stack top into a local variable.
pub const LOCAL_SET: u8 = 0x21;

// ── i32 constants and arithmetic ─────────────────────────────────────────

/// `i32.const` (0x41) — push a 32-bit integer immediate.
pub const I32_CONST: u8 = 0x41;

/// `i32.eqz` (0x45) — test if i32 == 0; push i32 result.
pub const I32_EQZ: u8 = 0x45;

/// `i32.eq` (0x46) — i32 == i32; push i32 (1 or 0).
pub const I32_EQ: u8 = 0x46;

/// `i32.ne` (0x47) — i32 != i32.
pub const I32_NE: u8 = 0x47;

/// `i32.lt_s` (0x48) — signed <.
pub const I32_LT_S: u8 = 0x48;

/// `i32.lt_u` (0x49) — unsigned <.
pub const I32_LT_U: u8 = 0x49;

/// `i32.gt_s` (0x4A) — signed >.
pub const I32_GT_S: u8 = 0x4A;

/// `i32.gt_u` (0x4B) — unsigned >.
pub const I32_GT_U: u8 = 0x4B;

/// `i32.le_s` (0x4C) — signed <=.
pub const I32_LE_S: u8 = 0x4C;

/// `i32.le_u` (0x4D) — unsigned <=.
pub const I32_LE_U: u8 = 0x4D;

/// `i32.ge_s` (0x4E) — signed >=.
pub const I32_GE_S: u8 = 0x4E;

/// `i32.ge_u` (0x4F) — unsigned >=.
pub const I32_GE_U: u8 = 0x4F;

/// `i32.add` (0x6A) — pop two i32, push sum.
pub const I32_ADD: u8 = 0x6A;

/// `i32.sub` (0x6B) — pop two i32, push difference.
pub const I32_SUB: u8 = 0x6B;

/// `i32.mul` (0x6C) — pop two i32, push product.
pub const I32_MUL: u8 = 0x6C;

/// `i32.div_s` (0x6D) — signed integer division.
pub const I32_DIV_S: u8 = 0x6D;

/// `i32.div_u` (0x6E) — unsigned integer division.
pub const I32_DIV_U: u8 = 0x6E;

/// `i32.rem_s` (0x6F) — signed remainder.
pub const I32_REM_S: u8 = 0x6F;

/// `i32.rem_u` (0x70) — unsigned remainder.
pub const I32_REM_U: u8 = 0x70;

/// `i32.and` (0x71) — bitwise AND.
pub const I32_AND: u8 = 0x71;

/// `i32.or` (0x72) — bitwise OR.
pub const I32_OR: u8 = 0x72;

/// `i32.xor` (0x73) — bitwise XOR.
pub const I32_XOR: u8 = 0x73;

/// `i32.shl` (0x74) — shift left.
pub const I32_SHL: u8 = 0x74;

/// `i32.shr_s` (0x75) — signed shift right (arithmetic).
pub const I32_SHR_S: u8 = 0x75;

/// `i32.shr_u` (0x76) — unsigned shift right (logical).
pub const I32_SHR_U: u8 = 0x76;

// ── i64 constants and arithmetic ─────────────────────────────────────────

/// `i64.const` (0x42) — push a 64-bit integer immediate.
pub const I64_CONST: u8 = 0x42;

/// `i64.eq` (0x51) — i64 == i64.
pub const I64_EQ: u8 = 0x51;

/// `i64.ne` (0x52) — i64 != i64.
pub const I64_NE: u8 = 0x52;

/// `i64.lt_s` (0x53) — signed i64 <.
pub const I64_LT_S: u8 = 0x53;

/// `i64.gt_s` (0x55) — signed i64 >.
pub const I64_GT_S: u8 = 0x55;

/// `i64.le_s` (0x57) — signed i64 <=.
pub const I64_LE_S: u8 = 0x57;

/// `i64.ge_s` (0x59) — signed i64 >=.
pub const I64_GE_S: u8 = 0x59;

/// `i64.add` (0x7C) — i64 addition.
pub const I64_ADD: u8 = 0x7C;

/// `i64.sub` (0x7D) — i64 subtraction.
pub const I64_SUB: u8 = 0x7D;

/// `i64.mul` (0x7E) — i64 multiplication.
pub const I64_MUL: u8 = 0x7E;

/// `i64.div_s` (0x7F) — signed i64 division.
pub const I64_DIV_S: u8 = 0x7F;

/// `i64.div_u` (0x80) — unsigned i64 division.
pub const I64_DIV_U: u8 = 0x80;

/// `i64.rem_s` (0x81) — signed i64 remainder.
pub const I64_REM_S: u8 = 0x81;

/// `i64.rem_u` (0x82) — unsigned i64 remainder.
pub const I64_REM_U: u8 = 0x82;

/// `i64.and` (0x83) — bitwise AND.
pub const I64_AND: u8 = 0x83;

/// `i64.or` (0x84) — bitwise OR.
pub const I64_OR: u8 = 0x84;

/// `i64.xor` (0x85) — bitwise XOR.
pub const I64_XOR: u8 = 0x85;

/// `i64.shl` (0x86) — shift left.
pub const I64_SHL: u8 = 0x86;

/// `i64.shr_s` (0x87) — signed shift right (arithmetic).
pub const I64_SHR_S: u8 = 0x87;

/// `i64.shr_u` (0x88) — unsigned shift right (logical).
pub const I64_SHR_U: u8 = 0x88;

// ── f32 constants and arithmetic ─────────────────────────────────────────

/// `f32.const` (0x43) — push a 32-bit float immediate (4 bytes, little-endian).
pub const F32_CONST: u8 = 0x43;

/// `f32.eq` (0x5B) — f32 equality.
pub const F32_EQ: u8 = 0x5B;

/// `f32.ne` (0x5C) — f32 inequality.
pub const F32_NE: u8 = 0x5C;

/// `f32.lt` (0x5D) — f32 less-than.
pub const F32_LT: u8 = 0x5D;

/// `f32.gt` (0x5E) — f32 greater-than.
pub const F32_GT: u8 = 0x5E;

/// `f32.le` (0x5F) — f32 less-or-equal.
pub const F32_LE: u8 = 0x5F;

/// `f32.ge` (0x60) — f32 greater-or-equal.
pub const F32_GE: u8 = 0x60;

/// `f32.add` (0x92) — f32 addition.
pub const F32_ADD: u8 = 0x92;

/// `f32.sub` (0x93) — f32 subtraction.
pub const F32_SUB: u8 = 0x93;

/// `f32.mul` (0x94) — f32 multiplication.
pub const F32_MUL: u8 = 0x94;

/// `f32.div` (0x95) — f32 division.
pub const F32_DIV: u8 = 0x95;

/// `f32.neg` (0x8C) — f32 negation.
pub const F32_NEG: u8 = 0x8C;

// ── f64 constants and arithmetic ─────────────────────────────────────────

/// `f64.const` (0x44) — push a 64-bit float immediate (8 bytes, little-endian).
pub const F64_CONST: u8 = 0x44;

/// `f64.eq` (0x61) — f64 equality.
pub const F64_EQ: u8 = 0x61;

/// `f64.ne` (0x62) — f64 inequality.
pub const F64_NE: u8 = 0x62;

/// `f64.lt` (0x63) — f64 less-than.
pub const F64_LT: u8 = 0x63;

/// `f64.gt` (0x64) — f64 greater-than.
pub const F64_GT: u8 = 0x64;

/// `f64.le` (0x65) — f64 less-or-equal.
pub const F64_LE: u8 = 0x65;

/// `f64.ge` (0x66) — f64 greater-or-equal.
pub const F64_GE: u8 = 0x66;

/// `f64.add` (0xA0) — f64 addition.
pub const F64_ADD: u8 = 0xA0;

/// `f64.sub` (0xA1) — f64 subtraction.
pub const F64_SUB: u8 = 0xA1;

/// `f64.mul` (0xA2) — f64 multiplication.
pub const F64_MUL: u8 = 0xA2;

/// `f64.div` (0xA3) — f64 division.
pub const F64_DIV: u8 = 0xA3;

/// `f64.neg` (0x9A) — f64 negation.
pub const F64_NEG: u8 = 0x9A;

// ── drop (stack cleanup) ──────────────────────────────────────────────────

/// `drop` (0x1A) — discard the top of the stack.
pub const DROP: u8 = 0x1A;

// ---------------------------------------------------------------------------
// Encoding helper functions
// ---------------------------------------------------------------------------
//
// Each helper returns a `Vec<u8>` containing the full binary encoding of
// one or more instructions (opcode + immediates).

/// Emit `i32.const <value>` — push a 32-bit signed integer constant.
///
/// The immediate is encoded as **signed LEB128** (not fixed-width), so small
/// values like `0` or `1` take only 2 bytes total (opcode + 1 LEB128 byte).
///
/// ```text
/// Binary:  0x41  <value as signed LEB128>
/// Example: i32.const 42  →  [0x41, 0x2A]
/// Example: i32.const -1  →  [0x41, 0x7F]
/// ```
pub fn encode_i32_const(value: i32) -> Vec<u8> {
    let mut b = vec![I32_CONST];
    b.extend(encode_signed(value as i64));
    b
}

/// Emit `i64.const <value>` — push a 64-bit signed integer constant.
///
/// The immediate is encoded as signed LEB128.
///
/// ```text
/// Binary:  0x42  <value as signed LEB128>
/// ```
pub fn encode_i64_const(value: i64) -> Vec<u8> {
    let mut b = vec![I64_CONST];
    b.extend(encode_signed(value));
    b
}

/// Emit `f32.const <value>` — push a 32-bit float constant.
///
/// The immediate is the IEEE 754 bit pattern in **little-endian** order
/// (4 bytes).  This is the only instruction in WASM with a raw fixed-width
/// float immediate — not LEB128.
///
/// ```text
/// Binary:  0x43  <4 bytes little-endian f32 bits>
/// ```
pub fn encode_f32_const(value: f32) -> Vec<u8> {
    let mut b = vec![F32_CONST];
    b.extend_from_slice(&value.to_bits().to_le_bytes());
    b
}

/// Emit `f64.const <value>` — push a 64-bit double-precision float constant.
///
/// The immediate is the IEEE 754 bit pattern in **little-endian** order
/// (8 bytes).
///
/// ```text
/// Binary:  0x44  <8 bytes little-endian f64 bits>
/// ```
pub fn encode_f64_const(value: f64) -> Vec<u8> {
    let mut b = vec![F64_CONST];
    b.extend_from_slice(&value.to_bits().to_le_bytes());
    b
}

/// Emit `local.get <idx>` — push the value of local variable `idx` onto the
/// operand stack.
///
/// The index is encoded as unsigned LEB128.
///
/// ```text
/// Binary:  0x20  <idx as unsigned LEB128>
/// ```
pub fn encode_local_get(idx: u32) -> Vec<u8> {
    let mut b = vec![LOCAL_GET];
    b.extend(encode_unsigned(idx as u64));
    b
}

/// Emit `local.set <idx>` — pop the top of the operand stack into local `idx`.
///
/// ```text
/// Binary:  0x21  <idx as unsigned LEB128>
/// ```
pub fn encode_local_set(idx: u32) -> Vec<u8> {
    let mut b = vec![LOCAL_SET];
    b.extend(encode_unsigned(idx as u64));
    b
}

/// Emit `br <depth>` — unconditional branch to the label at nesting depth
/// `depth`.
///
/// Depth 0 = innermost enclosing `block` or `loop`.
/// Depth 1 = one level up, etc.
///
/// ```text
/// Binary:  0x0C  <depth as unsigned LEB128>
/// ```
pub fn encode_br(depth: u32) -> Vec<u8> {
    let mut b = vec![BR];
    b.extend(encode_unsigned(depth as u64));
    b
}

/// Emit `br_if <depth>` — conditional branch if top-of-stack is non-zero.
/// The condition value is consumed (popped) whether or not the branch is
/// taken.
///
/// ```text
/// Binary:  0x0D  <depth as unsigned LEB128>
/// ```
pub fn encode_br_if(depth: u32) -> Vec<u8> {
    let mut b = vec![BR_IF];
    b.extend(encode_unsigned(depth as u64));
    b
}

/// Emit `call <fn_idx>` — call the function at index `fn_idx`.
///
/// ```text
/// Binary:  0x10  <fn_idx as unsigned LEB128>
/// ```
pub fn encode_call(fn_idx: u32) -> Vec<u8> {
    let mut b = vec![CALL];
    b.extend(encode_unsigned(fn_idx as u64));
    b
}

/// Emit a `br_table` instruction.
///
/// `br_table` is the WASM equivalent of a switch / jump table.  It pops an
/// `i32` index from the stack and branches to `targets[index]` if
/// `index < targets.len()`, otherwise to `default`.  All targets and the
/// default are **label depths** (relative to the current nesting level).
///
/// ```text
/// Binary:  0x0E
///          <count: unsigned LEB128>   -- len(targets)
///          (<target_i: unsigned LEB128>)*
///          <default: unsigned LEB128>
/// ```
pub fn encode_br_table(targets: &[u32], default: u32) -> Vec<u8> {
    let mut b = vec![BR_TABLE];
    // Encode the count of explicit targets (not including default).
    b.extend(encode_unsigned(targets.len() as u64));
    for &t in targets {
        b.extend(encode_unsigned(t as u64));
    }
    b.extend(encode_unsigned(default as u64));
    b
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i32_const_zero() {
        // i32.const 0 → [0x41, 0x00]
        assert_eq!(encode_i32_const(0), vec![0x41, 0x00]);
    }

    #[test]
    fn i32_const_pos() {
        // i32.const 42 → [0x41, 0x2A]
        assert_eq!(encode_i32_const(42), vec![0x41, 0x2A]);
    }

    #[test]
    fn i32_const_neg_one() {
        // i32.const -1 → [0x41, 0x7F]  (signed LEB128 of -1 is one byte: 0x7F)
        assert_eq!(encode_i32_const(-1), vec![0x41, 0x7F]);
    }

    #[test]
    fn i64_const_zero() {
        assert_eq!(encode_i64_const(0), vec![0x42, 0x00]);
    }

    #[test]
    fn f64_const_zero() {
        // f64 0.0 has IEEE 754 bit pattern 0x0000000000000000.
        // Little-endian → [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        let enc = encode_f64_const(0.0);
        assert_eq!(enc[0], F64_CONST);
        assert_eq!(enc.len(), 9); // opcode + 8 bytes
    }

    #[test]
    fn f32_const_size() {
        let enc = encode_f32_const(1.0f32);
        assert_eq!(enc[0], F32_CONST);
        assert_eq!(enc.len(), 5); // opcode + 4 bytes
    }

    #[test]
    fn local_get() {
        // local.get 0 → [0x20, 0x00]
        assert_eq!(encode_local_get(0), vec![0x20, 0x00]);
        // local.get 1 → [0x20, 0x01]
        assert_eq!(encode_local_get(1), vec![0x20, 0x01]);
    }

    #[test]
    fn local_set() {
        // local.set 2 → [0x21, 0x02]
        assert_eq!(encode_local_set(2), vec![0x21, 0x02]);
    }

    #[test]
    fn br_depth_zero() {
        // br 0 → [0x0C, 0x00]
        assert_eq!(encode_br(0), vec![0x0C, 0x00]);
    }

    #[test]
    fn br_if_depth() {
        // br_if 1 → [0x0D, 0x01]
        assert_eq!(encode_br_if(1), vec![0x0D, 0x01]);
    }

    #[test]
    fn call_idx() {
        // call 3 → [0x10, 0x03]
        assert_eq!(encode_call(3), vec![0x10, 0x03]);
    }

    #[test]
    fn br_table_encoding() {
        // br_table [0, 1, 2] default=3
        let enc = encode_br_table(&[0, 1, 2], 3);
        assert_eq!(enc[0], BR_TABLE);
        // count = 3
        assert_eq!(enc[1], 0x03);
        // targets: 0, 1, 2
        assert_eq!(enc[2], 0x00);
        assert_eq!(enc[3], 0x01);
        assert_eq!(enc[4], 0x02);
        // default = 3
        assert_eq!(enc[5], 0x03);
    }

    #[test]
    fn opcode_constants_match_spec() {
        // Spot-check a selection of opcodes against the WASM 1.0 spec.
        assert_eq!(NOP, 0x01);
        assert_eq!(BLOCK, 0x02);
        assert_eq!(LOOP, 0x03);
        assert_eq!(END, 0x0B);
        assert_eq!(BR, 0x0C);
        assert_eq!(RETURN, 0x0F);
        assert_eq!(CALL, 0x10);
        assert_eq!(LOCAL_GET, 0x20);
        assert_eq!(LOCAL_SET, 0x21);
        assert_eq!(I32_CONST, 0x41);
        assert_eq!(I64_CONST, 0x42);
        assert_eq!(F32_CONST, 0x43);
        assert_eq!(F64_CONST, 0x44);
        assert_eq!(I32_ADD, 0x6A);
        assert_eq!(I64_ADD, 0x7C);
        assert_eq!(F64_ADD, 0xA0);
    }
}
