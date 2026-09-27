//! # `intel8086-backend` — Intel 8086 backend for jit-core / aot-core.
//!
//! Lowers a `Vec<CIRInstr>` into Intel 8086 machine code via
//! [`intel8086_encoder`]. Output is `Vec<u8>` — the 8086 is byte-oriented
//! at the encoding level (multi-byte immediates are little-endian, but
//! there's no fixed instruction-word width to flatten, unlike
//! `arm1-backend`/`mips-r2000-backend`), so the encoder's `Vec<u8>` bytes
//! are already the wire format.
//!
//! Mirror of [`mos6502_backend`]/[`arm1_backend`]/[`armv7_backend`] in
//! shape. Ninth and **final** lane of the 9-architecture expansion
//! following the pattern documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! ## Scope (WORD02)
//!
//! The fixed-width result ABI now includes a bounded two-live-value allocator
//! and wrapping arithmetic:
//!
//! | CIR op | Lowering |
//! |--------|----------|
//! | `const_u8`, `const_bool` | `MOV AX, #imm16` with `AH = 0` |
//! | `const_u16` | `MOV AX, #imm16` |
//! | `add`/`sub`/`and`/`or`/`xor`/`not` on `u8` and `u16` | register ALU lowering through `AX`/`BX` (`CX` scratch) |
//! | matching typed returns, `ret_void` | `HLT` (a genuine hardware halt — see below) |
//! | Anything else | returns `None` |
//!
//! Up to two same-width values are live in `AX` and `BX`. Binary operations
//! consume both and leave their result in `AX`; `CX` is scratch for reversed
//! subtraction and byte-width masks. Comparisons and control flow remain
//! outside this increment.
//!
//! Per the migration spec, this is acceptable: the architectural
//! correctness win (IIR → CIR via `Backend` trait) is delivered as soon
//! as the AOT path is wired, regardless of op-set parity.
//!
//! ## Why does `ret_*` lower to real `HLT`, not a pseudo-halt?
//!
//! Unlike ARM1 (1985, no real halt instruction — `arm1-backend` had to
//! invent a pseudo-halt via `SWI #0x123456`) or MOS 6502 (whose `BRK` is
//! technically a software-interrupt opcode that this repo's simulator
//! stack *treats* as HALT by convention), the Intel 8086 has a genuine,
//! single-byte, no-operand hardware instruction whose sole purpose is to
//! stop the fetch-decode-execute loop: `HLT` (opcode `0xF4`). This isn't
//! a simulator-level convention this lane invented or inherited — it's
//! real silicon behaviour, faithfully ported from `code/packages/python/
//! intel-8086-simulator`'s `simulator.py` (`if op == 0xF4: self._halted =
//! True; return "HLT"`). `ret_*`/`ret_void` lowering to `HLT` is
//! therefore the most direct, least-invented choice of any halt-related
//! decision in this entire 9-architecture campaign.
//!
//! ## The `terminated: bool` pattern — and the bug class it avoids
//!
//! A real bug was found and fixed in **four** prior lanes of this
//! campaign (Intel 8051, Intel 8080, MOS 6502, Zilog Z80): the backend's
//! defensive "is the program already terminated?" check was written as a
//! **trailing-byte-value comparison** (`bytes.last() == Some(&HALT_BYTE)`)
//! or, worse, an `is_empty()` check. Both are unsound for this exact
//! reason: `MOV AX, imm16` encodes as `[0xB8, imm_lo, imm_hi]`, and
//! **the immediate's own bytes can numerically collide with the halt
//! opcode**. `HALT_BYTE` is `0xF4`; an immediate like `0xF400` encodes as
//! `[0xB8, 0x00, 0xF4]` — trailing byte `0xF4`, identical to `HLT`,
//! despite this program never having executed a real halt instruction at
//! all. A trailing-byte check would wrongly conclude "already
//! terminated" and skip appending the real `HLT`, silently shipping a
//! program with **no genuine halt instruction** — the CPU would fetch
//! whatever garbage byte follows in memory as the next opcode. `is_empty
//! ()` is unsound for a different reason: any `const_*` at all makes
//! `bytes` non-empty long before a real terminator is ever emitted, so
//! it can never correctly detect "no terminator yet" once the loop is
//! underway.
//!
//! This backend avoids the whole bug class by tracking an explicit
//! `terminated: bool` local, not a byte-value proxy:
//!
//! - Starts `false`.
//! - Set to `true` **only** when a real `ret_*`/`ret_void` arm pushes a
//!   genuine `HLT`.
//! - Reset to `false` whenever any further `const_*` (or other
//!   non-terminating instruction) is emitted afterward.
//! - At the end of the loop, if `terminated` is still `false`, a real
//!   `HLT` is appended — regardless of what byte value happens to sit
//!   last in the buffer.
//!
//! See `const_whose_encoded_high_byte_collides_with_halt_opcode_still_gets_real_terminator`
//! in `tests/test_backend.rs` for a regression test that would fail
//! against a naive trailing-byte-comparison implementation.
//!
//! ## Why is `Backend::run` not implemented?
//!
//! Emit-only target per the migration spec. Bytes go to
//! `intel8086-simulator`.

use intel8086_encoder::{
    encode_add_reg_reg16, encode_and_reg_reg16, encode_hlt, encode_mov_reg_imm16,
    encode_mov_reg_reg16, encode_or_reg_reg16, encode_sub_reg_reg16, encode_xor_reg_reg16, REG_AX,
    REG_BX, REG_CX,
};
use jit_core::backend::{Backend, FunctionContext};
use jit_core::cir::{CIRInstr, CIROperand};
use std::fmt;
use vm_core::value::Value;

#[derive(Debug, Default, Clone, Copy)]
pub struct Intel8086Backend;

impl Intel8086Backend {
    pub fn new() -> Self {
        Intel8086Backend
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    UnsupportedOp(String),
    InvalidOperand(String),
    UndefinedVariable(String),
    ImmediateOutOfRange(i64),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedOp(op) => write!(f, "intel8086-backend: unsupported op {op:?}"),
            Self::InvalidOperand(d) => write!(f, "intel8086-backend: invalid operand: {d}"),
            Self::UndefinedVariable(n) => {
                write!(f, "intel8086-backend: undefined variable {n:?}")
            }
            Self::ImmediateOutOfRange(n) => write!(
                f,
                "intel8086-backend: const {n} is outside the selected unsigned result width"
            ),
        }
    }
}

impl std::error::Error for BackendError {}

/// Compile a single function's CIR into Intel 8086 bytes.
pub fn compile(_ctx: &FunctionContext<'_>, cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    compile_to_bytes(cir)
}

fn compile_to_bytes(cir: &[CIRInstr]) -> Result<Vec<u8>, BackendError> {
    if cir.is_empty() {
        // Empty CIR -> just HLT so the program halts immediately.
        return Ok(encode_hlt());
    }

    let mut bytes = Vec::new();
    // WORD02 keeps at most two same-width values in AX/BX. Operations consume
    // those inputs and place their result back in AX.
    let mut live_values: Vec<LiveValue> = Vec::new();

    // Tracks "has a genuine halt-convention instruction (HLT) already
    // been pushed?" -- an explicit boolean, NOT a trailing-byte-value
    // comparison. See this module's doc for the exact byte-collision bug
    // class this avoids (fixed in four prior lanes of this campaign:
    // Intel 8051, Intel 8080, MOS 6502, Zilog Z80).
    let mut terminated = false;

    for instr in cir {
        let op = instr.op.as_str();

        if op == "ret_void" {
            bytes.extend_from_slice(&encode_hlt());
            terminated = true;
            continue;
        }

        if op.strip_prefix("ret_").is_some() {
            let expected_width = result_width_for_ret(op)
                .ok_or_else(|| BackendError::UnsupportedOp(op.to_string()))?;
            let src_name = parse_var_src(instr, 0, op)?;
            // A secondary result is copied from BX into the AX result ABI.
            let Some(value) = live_values.iter().find(|value| value.name == src_name) else {
                return Err(BackendError::UndefinedVariable(src_name));
            };
            if value.width != expected_width {
                return Err(BackendError::UnsupportedOp(format!(
                    "{op} cannot return the current {}-bit value",
                    value.width.bits()
                )));
            }
            materialize_primary(&mut bytes, value.location);
            bytes.extend_from_slice(&encode_hlt());
            terminated = true;
            continue;
        }

        if op.strip_prefix("const_").is_some() {
            let dest = require_dest(instr, op)?;
            let width = result_width_for_const(op)
                .ok_or_else(|| BackendError::UnsupportedOp(op.to_string()))?;
            let location = allocate_location(&live_values, width)?;
            let imm = encode_typed_immediate(op, instr.srcs.first())?;
            let register = match location {
                ValueLocation::Primary => REG_AX,
                ValueLocation::Secondary => REG_BX,
            };
            bytes.extend_from_slice(&encode_mov_reg_imm16(register, imm));
            live_values.push(LiveValue {
                name: dest.to_string(),
                width,
                location,
            });
            // A non-terminating instruction was just emitted -- even if
            // the buffer's trailing byte now happens to numerically
            // equal HALT_BYTE (imm's high byte can be 0xF4), the program
            // has NOT halted. This reset is the crux of the
            // terminated:bool pattern: a byte-value check has no
            // equivalent "reset" step, which is exactly how the bug
            // class this avoids slips in.
            terminated = false;
            continue;
        }

        if let Some((width, binary_op)) = binary_op(op) {
            let dest = require_dest(instr, op)?;
            let lhs_name = parse_var_src(instr, 0, op)?;
            let rhs_name = parse_var_src(instr, 1, op)?;
            let lhs = find_live_value(&live_values, &lhs_name, width, op)?;
            let rhs = find_live_value(&live_values, &rhs_name, width, op)?;
            emit_binary(&mut bytes, width, binary_op, lhs.location, rhs.location);
            live_values = vec![LiveValue {
                name: dest.to_string(),
                width,
                location: ValueLocation::Primary,
            }];
            terminated = false;
            continue;
        }

        if let Some(width) = unary_not_width(op) {
            let dest = require_dest(instr, op)?;
            let src_name = parse_var_src(instr, 0, op)?;
            let src = find_live_value(&live_values, &src_name, width, op)?;
            materialize_primary(&mut bytes, src.location);
            let mask = match width {
                ResultWidth::Byte => 0x00FF,
                ResultWidth::Word => 0xFFFF,
            };
            bytes.extend_from_slice(&encode_mov_reg_imm16(REG_CX, mask));
            bytes.extend_from_slice(&encode_xor_reg_reg16(REG_AX, REG_CX));
            live_values = vec![LiveValue {
                name: dest.to_string(),
                width,
                location: ValueLocation::Primary,
            }];
            terminated = false;
            continue;
        }

        return Err(BackendError::UnsupportedOp(op.to_string()));
    }

    // Defensive -- if no terminator was emitted, append HLT so the
    // program halts instead of running off the end (which would read
    // whatever byte follows in memory as the next opcode). Driven by
    // the `terminated` flag, NOT by inspecting `bytes`' trailing byte --
    // see this module's doc and the regression test in
    // tests/test_backend.rs.
    if !terminated {
        bytes.extend_from_slice(&encode_hlt());
    }
    Ok(bytes)
}

fn require_dest<'a>(instr: &'a CIRInstr, op: &str) -> Result<&'a str, BackendError> {
    instr
        .dest
        .as_deref()
        .ok_or_else(|| BackendError::InvalidOperand(format!("{op} requires a dest")))
}

fn parse_var_src(instr: &CIRInstr, idx: usize, op: &str) -> Result<String, BackendError> {
    match instr.srcs.get(idx) {
        Some(CIROperand::Var(s)) => Ok(s.clone()),
        _ => Err(BackendError::InvalidOperand(format!(
            "{op} srcs[{idx}] must be Var"
        ))),
    }
}

/// Width-preserving materialization through `MOV AX,#imm16`. Byte values are
/// range-checked before being zero-extended into AX; word values accept the
/// full unsigned 16-bit range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultWidth {
    Byte,
    Word,
}

impl ResultWidth {
    fn bits(self) -> u8 {
        match self {
            Self::Byte => 8,
            Self::Word => 16,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LiveValue {
    name: String,
    width: ResultWidth,
    location: ValueLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueLocation {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryOp {
    Add,
    Sub,
    And,
    Or,
    Xor,
}

fn allocate_location(
    live_values: &[LiveValue],
    width: ResultWidth,
) -> Result<ValueLocation, BackendError> {
    match live_values {
        [] => Ok(ValueLocation::Primary),
        [value] if value.width == width => Ok(ValueLocation::Secondary),
        [value] => Err(BackendError::UnsupportedOp(format!(
            "cannot keep a {}-bit and {}-bit value live together in WORD02",
            value.width.bits(),
            width.bits()
        ))),
        _ => Err(BackendError::UnsupportedOp(
            "WORD02 supports at most two live values".into(),
        )),
    }
}

fn find_live_value<'a>(
    live_values: &'a [LiveValue],
    name: &str,
    width: ResultWidth,
    op: &str,
) -> Result<&'a LiveValue, BackendError> {
    let value = live_values
        .iter()
        .find(|value| value.name == name)
        .ok_or_else(|| BackendError::UndefinedVariable(name.to_string()))?;
    if value.width == width {
        Ok(value)
    } else {
        Err(BackendError::UnsupportedOp(format!(
            "{op} requires {}-bit operands",
            width.bits()
        )))
    }
}

fn binary_op(op: &str) -> Option<(ResultWidth, BinaryOp)> {
    let (name, width) = op.rsplit_once('_')?;
    let width = match width {
        "u8" => ResultWidth::Byte,
        "u16" => ResultWidth::Word,
        _ => return None,
    };
    let operation = match name {
        "add" => BinaryOp::Add,
        "sub" => BinaryOp::Sub,
        "and" => BinaryOp::And,
        "or" => BinaryOp::Or,
        "xor" => BinaryOp::Xor,
        _ => return None,
    };
    Some((width, operation))
}

fn unary_not_width(op: &str) -> Option<ResultWidth> {
    match op {
        "not_u8" => Some(ResultWidth::Byte),
        "not_u16" => Some(ResultWidth::Word),
        _ => None,
    }
}

fn materialize_primary(bytes: &mut Vec<u8>, location: ValueLocation) {
    if location == ValueLocation::Secondary {
        bytes.extend_from_slice(&encode_mov_reg_reg16(REG_AX, REG_BX));
    }
}

fn emit_binary(
    bytes: &mut Vec<u8>,
    width: ResultWidth,
    op: BinaryOp,
    lhs: ValueLocation,
    rhs: ValueLocation,
) {
    if lhs == rhs {
        materialize_primary(bytes, lhs);
        emit_reg_binary(bytes, op, REG_AX);
    } else if lhs == ValueLocation::Secondary && op == BinaryOp::Sub {
        bytes.extend_from_slice(&encode_mov_reg_reg16(REG_CX, REG_AX));
        bytes.extend_from_slice(&encode_mov_reg_reg16(REG_AX, REG_BX));
        bytes.extend_from_slice(&encode_sub_reg_reg16(REG_AX, REG_CX));
    } else {
        emit_reg_binary(bytes, op, REG_BX);
    }

    if width == ResultWidth::Byte && matches!(op, BinaryOp::Add | BinaryOp::Sub) {
        bytes.extend_from_slice(&encode_mov_reg_imm16(REG_CX, 0x00FF));
        bytes.extend_from_slice(&encode_and_reg_reg16(REG_AX, REG_CX));
    }
}

fn emit_reg_binary(bytes: &mut Vec<u8>, op: BinaryOp, rhs: u8) {
    let encoded = match op {
        BinaryOp::Add => encode_add_reg_reg16(REG_AX, rhs),
        BinaryOp::Sub => encode_sub_reg_reg16(REG_AX, rhs),
        BinaryOp::And => encode_and_reg_reg16(REG_AX, rhs),
        BinaryOp::Or => encode_or_reg_reg16(REG_AX, rhs),
        BinaryOp::Xor => encode_xor_reg_reg16(REG_AX, rhs),
    };
    bytes.extend_from_slice(&encoded);
}

fn result_width_for_const(op: &str) -> Option<ResultWidth> {
    match op {
        "const_u8" | "const_bool" => Some(ResultWidth::Byte),
        "const_i64" | "const_u16" => Some(ResultWidth::Word),
        _ => None,
    }
}

fn result_width_for_ret(op: &str) -> Option<ResultWidth> {
    match op {
        "ret_u8" | "ret_bool" => Some(ResultWidth::Byte),
        "ret_i64" | "ret_u16" => Some(ResultWidth::Word),
        _ => None,
    }
}

fn encode_typed_immediate(op: &str, operand: Option<&CIROperand>) -> Result<u16, BackendError> {
    if op == "const_bool" {
        return match operand {
            Some(CIROperand::Bool(value)) => Ok(u16::from(*value)),
            _ => Err(BackendError::InvalidOperand(
                "const_bool srcs[0] must be Bool".into(),
            )),
        };
    }

    let n = match operand {
        Some(CIROperand::Int(n)) => *n,
        Some(CIROperand::Bool(b)) if op == "const_i64" => i64::from(*b),
        _ => {
            return Err(BackendError::InvalidOperand(format!(
                "{op} srcs[0] must be Int"
            )));
        }
    };
    let max = if op == "const_u8" { 0xFF } else { 0xFFFF };
    if (0..=max).contains(&n) {
        Ok(n as u16)
    } else {
        Err(BackendError::ImmediateOutOfRange(n))
    }
}

impl Backend for Intel8086Backend {
    fn name(&self) -> &str {
        "intel8086"
    }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        compile_to_bytes(ir).ok()
    }

    fn compile_function(&self, _ctx: &FunctionContext<'_>, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        self.compile(ir)
    }

    fn run(&self, _binary: &[u8], _args: &[Value]) -> Value {
        panic!("intel8086 backend is emit-only; load bytes into intel8086-simulator to execute");
    }
}
