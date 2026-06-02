//! # iir-to-riscv — IIR → RV32I machine code backend.
//!
//! Lowers an [`interpreter_ir::IIRModule`] to a `Vec<u32>` of encoded
//! 32-bit RISC-V instructions, suitable to drop into the in-tree
//! [`riscv-simulator`] or to write out as a flat `.bin` for
//! `qemu-riscv32`.
//!
//! ## Scope of v0.2.0 (A1+)
//!
//! v0.1.0 (A1) emitted a single `ret` regardless of input.  v0.2.0
//! (A1+) is the first slice of real lowering:
//!
//! | IIR op | RV32I lowering |
//! |--------|----------------|
//! | `const dest, Int(n)` (12-bit imm) | `addi rd, x0, n` |
//! | `add dest, a, b` | `add rd, rs1, rs2` |
//! | `sub dest, a, b` | `sub rd, rs1, rs2` |
//! | `mov dest, src`  | `addi rd, rs1, 0` (canonical move) |
//! | `ret <var>` (int) | `addi a0, var_reg, 0` (mv to a0) + `jalr x0, x1, 0` |
//! | `ret_void` | `jalr x0, x1, 0` |
//!
//! ### Register allocation (linear, no spilling)
//!
//! * Function parameters land in `a0..a7` (`x10..x17`) per the RISC-V
//!   calling convention.
//! * Locals (anything bound to a new `dest`) get the next free
//!   temporary register from `[t0, t1, t2, t3, t4, t5, t6]` =
//!   `[x5, x6, x7, x28, x29, x30, x31]`.  When the pool is exhausted
//!   we return [`IIRRiscvError::UnsupportedOp`] — a real register
//!   allocator with spilling lands in A1++.
//!
//! ### Comparisons, branches, calls
//!
//! Not yet in v0.2.0.  Lands in v0.3.0 (A1++) alongside `ecall` for
//! `print_i64`.  This keeps the v0.2.0 PR focused on the data-flow
//! core (arith + ret) so the register-allocator behaviour is
//! reviewable in isolation.
//!
//! ## Why `Vec<u32>` (not textual asm)?
//!
//! - Round-trips with `riscv-simulator`'s decoder.
//! - Deterministic test surface (`assert!(words[0] == 0x...)`).
//! - No GNU-vs-LLVM assembler syntax coupling.
//!
//! ## Quick start
//!
//! ```
//! use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
//! use iir_to_riscv::{lower_iir_to_riscv, IIRRiscvConfig};
//!
//! // fn answer() -> i32 { const v = 7; ret v }
//! let f = IIRFunction::new(
//!     "answer",
//!     vec![],
//!     "i32",
//!     vec![
//!         IIRInstr::new("const", Some("v".into()), vec![Operand::Int(7)], "i32"),
//!         IIRInstr::new("ret",   None,             vec![Operand::Var("v".into())], "i32"),
//!     ],
//! );
//! let module = IIRModule {
//!     name: "demo".into(),
//!     functions: vec![f],
//!     entry_point: Some("answer".into()),
//!     language: "test".into(),
//!     exports: vec![],
//!     imports: vec![],
//! };
//! let words = lower_iir_to_riscv(&module, &IIRRiscvConfig::default())
//!     .expect("lowering should succeed");
//! // Three words: addi t0, x0, 7;  addi a0, t0, 0;  jalr x0, x1, 0
//! assert_eq!(words.len(), 3);
//! ```

use std::collections::HashMap;
use std::fmt;

use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
use riscv_simulator::encoding::{
    encode_add, encode_addi, encode_beq, encode_bne, encode_ecall, encode_jal,
    encode_jalr, encode_lui, encode_slt, encode_sltiu, encode_sltu, encode_sub,
    encode_xor, encode_xori,
};

// ===========================================================================
// Register layout
// ===========================================================================

// Hardwired zero (x0) and return address (x1) used by the ret encoding.
const X0_ZERO: u32 = 0;
const X1_RA:   u32 = 1;

/// Argument / return-value registers `a0..a7` per the RISC-V calling
/// convention.  The first 8 function parameters land here, in order; the
/// return value (for non-void functions) is moved into `a0` before the
/// epilogue's `ret`.
const ARG_REGISTERS: [u32; 8] = [10, 11, 12, 13, 14, 15, 16, 17];

/// Caller-saved temporary registers `t0..t6` per the RV32I ABI:
/// `t0..t2` = `x5..x7`, `t3..t6` = `x28..x31`.  This pool is what the
/// naive linear allocator hands out for local variables (anything other
/// than function parameters or the canonical return slot `a0`).
///
/// 7 slots is generous for the v0.2.0 IIR subset (no calls yet, so no
/// caller-save concerns).  A1++ replaces this with a real allocator.
const TEMP_REGISTERS: [u32; 7] = [5, 6, 7, 28, 29, 30, 31];

// ===========================================================================
// IIRRiscvConfig
// ===========================================================================

/// Configuration for the IIR → RV32I lowering pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IIRRiscvConfig {
    /// Module name — reserved for future ELF / linker artefact emission.
    pub module_name: String,
}

impl IIRRiscvConfig {
    pub fn new(module_name: impl Into<String>) -> Self {
        Self { module_name: module_name.into() }
    }
}

impl Default for IIRRiscvConfig {
    fn default() -> Self {
        Self { module_name: "iir_module".into() }
    }
}

// ===========================================================================
// IIRRiscvError
// ===========================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IIRRiscvError {
    ValidationFailed(Vec<String>),
    UnsupportedOp { function: String, op: String },
    UnsupportedType { function: String, type_hint: String },
    InvalidOperand { function: String, detail: String },
    /// A variable name was used before it was bound by `const`, `mov`,
    /// or a function param.
    UndefinedVariable { function: String, name: String },
    /// Too many function params (>8) — RV32I caller convention only
    /// passes the first 8 in `a0..a7`.  Real lowering would spill onto
    /// the stack; v0.2.0 rejects this case.
    TooManyParams { function: String, count: usize },
    /// Too many locals — the v0.2.0 register pool (7 temps) is full.
    /// A1++ replaces this with a stack-spilling allocator.
    OutOfRegisters { function: String, name: String },
    /// An `Operand::Int(n)` literal exceeds the RV32I 12-bit signed
    /// immediate range (-2048..2047).  v0.2.0 doesn't synthesise the
    /// `lui` + `addi` pair for wider constants — A1++ adds it.
    ImmediateOutOfRange { function: String, value: i64 },
    /// A `jmp` / `jmp_if_*` referenced a label that was never defined
    /// by a `label "<name>"` instruction in the same function.
    UndefinedLabel { function: String, label: String },
    /// A branch's target is too far away to encode.  B-type (beq/bne)
    /// gives ±4096 bytes; J-type (jal) gives ±1 MiB.
    BranchOutOfRange { function: String, label: String, offset: i64, max: i64 },
}

impl fmt::Display for IIRRiscvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ValidationFailed(errs) => write!(f, "validation failed:\n  {}", errs.join("\n  ")),
            Self::UnsupportedOp { function, op } =>
                write!(f, "unsupported op in function {function:?}: {op}"),
            Self::UnsupportedType { function, type_hint } =>
                write!(f, "unsupported type in function {function:?}: {type_hint}"),
            Self::InvalidOperand { function, detail } =>
                write!(f, "invalid operand in function {function:?}: {detail}"),
            Self::UndefinedVariable { function, name } =>
                write!(f, "undefined variable {name:?} in function {function:?}"),
            Self::TooManyParams { function, count } =>
                write!(f, "function {function:?} has {count} params; RV32I caller convention supports up to 8 in a0..a7 (stack spilling lands in A1++)"),
            Self::OutOfRegisters { function, name } =>
                write!(f, "out of temporary registers (t0..t6) while binding {name:?} in function {function:?}; stack spilling lands in A1++"),
            Self::ImmediateOutOfRange { function, value } =>
                write!(f, "literal {value} exceeds RV32I 12-bit signed immediate range (-2048..2047) in function {function:?}; lui+addi lowering lands in A1++"),
            Self::UndefinedLabel { function, label } =>
                write!(f, "branch to undefined label {label:?} in function {function:?}"),
            Self::BranchOutOfRange { function, label, offset, max } =>
                write!(f, "branch to label {label:?} in function {function:?} would require offset {offset} bytes, exceeding the ±{max}-byte range of this branch encoding"),
        }
    }
}

impl std::error::Error for IIRRiscvError {}

// ===========================================================================
// Supported types
// ===========================================================================

/// Type hints the v0.2.0 backend understands.  Everything is treated as
/// a 32-bit value at this scope (RV32I native width); u8/u16/i8/i16
/// flow through unchanged since we don't sign-extend or mask yet.
///
/// `i64`/`u64`/`f32`/`f64` are deferred: real 64-bit on RV32 needs
/// register pairs, and floats need the F-extension.
fn is_supported_type(t: &str) -> bool {
    matches!(t, "void" | "i8" | "u8" | "i16" | "u16" | "i32" | "u32")
}

// ===========================================================================
// Supported opcodes
// ===========================================================================

const SUPPORTED_OPS: &[&str] = &[
    // A1+
    "const", "mov", "ret", "ret_void", "add", "sub",
    // A1++ — comparisons (both naked and cmp_-prefixed per G1)
    "eq", "ne", "lt", "le", "gt", "ge",
    "cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge",
    // A1++ — host call
    "call_builtin",
    // A1++.5 — control flow within a function
    "label", "jmp", "jmp_if_true", "jmp_if_false",
];

/// Syscall number used by the RV32I `ecall` for `call_builtin print_i64`.
///
/// The convention: the host (a future riscv-simulator launcher) decodes
/// `a7 == 1` as "print signed 64-bit integer in a0/a1 to stdout".  We
/// pick this value because:
///
/// * `1` is the canonical Linux RV32 `__NR_write` index, easy to
///   remember.  We're not implementing actual `write(fd, buf, len)`;
///   we're piggybacking on the same convention slot so a future
///   real-syscall pass can fold this into a true `write`.
/// * Other backends pick their own sentinel: wasm uses
///   `env.__print_i64` (a host import), JVM uses
///   `env/BasicRuntime.println(J)V`, CLR uses
///   `env.BasicRuntime::PrintI64(int64)`, LLVM uses
///   `@__print_i64` extern.  The RV32I sentinel completes the parity.
///
/// Today we only emit `a0` (the low 32 bits).  64-bit pair handling
/// is deferred to A1++.5 alongside i64 arithmetic.
const ECALL_PRINT_I64_NUM: i32 = 1;

/// Builtins the RV32I backend knows how to lower via `ecall`.
const SUPPORTED_BUILTINS: &[&str] = &["print_i64"];

// ===========================================================================
// validate_for_riscv
// ===========================================================================

/// Pre-flight validation for IIR → RV32I lowering.
///
/// Returns a `Vec<String>` of human-readable error messages.  An empty
/// vector means the module is safe to pass to [`lower_iir_to_riscv`].
///
/// # Checks
///
/// 1. Every instruction's `op` is in [`SUPPORTED_OPS`].
/// 2. Every instruction's `type_hint` is in [`is_supported_type`].
/// 3. Every function's return type is supported.
/// 4. Every function's param types are supported, and there are at
///    most 8 of them.
pub fn validate_for_riscv(module: &IIRModule) -> Vec<String> {
    let mut errors = Vec::new();
    for f in &module.functions {
        if !is_supported_type(&f.return_type) {
            errors.push(format!(
                "UnsupportedType: function {:?}, return type {:?} not supported",
                f.name, f.return_type
            ));
        }
        if f.params.len() > 8 {
            errors.push(format!(
                "TooManyParams: function {:?} has {} params; max 8 in v0.2.0",
                f.name, f.params.len()
            ));
        }
        for (pname, pty) in &f.params {
            if !is_supported_type(pty) {
                errors.push(format!(
                    "UnsupportedType: function {:?}, param {:?} type {:?} not supported",
                    f.name, pname, pty
                ));
            }
        }
        for instr in &f.instructions {
            if !SUPPORTED_OPS.contains(&instr.op.as_str()) {
                errors.push(format!(
                    "UnsupportedOp: function {:?}, op {:?} not in v0.2.0 whitelist (supported: {:?})",
                    f.name, instr.op, SUPPORTED_OPS
                ));
            }
            if !is_supported_type(&instr.type_hint) {
                errors.push(format!(
                    "UnsupportedType: function {:?}, instr {:?} type_hint {:?} not supported",
                    f.name, instr.op, instr.type_hint
                ));
            }
        }
    }
    errors
}

// ===========================================================================
// lower_iir_to_riscv
// ===========================================================================

/// Lower an [`IIRModule`] to a `Vec<u32>` of RV32I instruction words.
///
/// Lowers every function in order, concatenating their word sequences.
/// Each function is independent — no cross-function calls in v0.2.0 (that
/// requires real PC-relative `jal` + symbol resolution, which lands in
/// A1++).
pub fn lower_iir_to_riscv(
    module: &IIRModule,
    _cfg: &IIRRiscvConfig,
) -> Result<Vec<u32>, IIRRiscvError> {
    let errors = validate_for_riscv(module);
    if !errors.is_empty() {
        return Err(IIRRiscvError::ValidationFailed(errors));
    }

    let mut words = Vec::new();
    for f in &module.functions {
        let fn_words = lower_function(f)?;
        words.extend(fn_words);
    }
    // An empty module emits no words at all.  This is the trivial-input
    // contract: callers that want a minimum 1-instruction `.text` should
    // wrap the result accordingly.
    Ok(words)
}

/// Kind of branch placeholder waiting to be patched once the target
/// label's byte offset is known.
///
/// `B_TYPE_*` cover the conditional branches (`beq`/`bne`) emitted by
/// `jmp_if_true` / `jmp_if_false`; the placeholder records which
/// registers were already encoded into the opcode word but with a zero
/// offset, plus the comparison kind so the patcher can re-emit with
/// the right offset.  `J_TYPE_JAL` covers the unconditional `jmp` →
/// `jal x0, offset`.
#[derive(Debug, Clone, Copy)]
enum BranchKind {
    /// `beq rs1, x0, +offset` — fire branch when cond is zero (false).
    BeqZero { rs1: u32 },
    /// `bne rs1, x0, +offset` — fire branch when cond is non-zero (true).
    BneZero { rs1: u32 },
    /// `jal x0, +offset` — unconditional jump, discard return address.
    JalDiscard,
}

/// Per-function state shared by `lower_function` and `lower_instr`.
struct FnState<'a> {
    fn_name: &'a str,
    /// IIR var name → assigned RV32I register index (`x*`).
    env: HashMap<String, u32>,
    /// Next free index into [`TEMP_REGISTERS`].  When this exceeds the
    /// pool length we return `OutOfRegisters` — A1++.5 replaces this with
    /// a stack-spilling allocator.
    next_temp: usize,
    /// Label name → byte offset within the function body.
    /// Filled lazily as `label "<name>"` instructions are emitted.
    labels: HashMap<String, usize>,
    /// Pending branch patches.  Each entry records:
    ///   - the word index into the function body that needs patching,
    ///   - the target label name,
    ///   - the encoding kind (so we know which encoder to call once we
    ///     know the resolved byte offset).
    branches: Vec<(usize, String, BranchKind)>,
}

impl FnState<'_> {
    /// Reserve a fresh temporary register for `name`.
    fn alloc_temp(&mut self, name: &str) -> Result<u32, IIRRiscvError> {
        if self.next_temp >= TEMP_REGISTERS.len() {
            return Err(IIRRiscvError::OutOfRegisters {
                function: self.fn_name.into(),
                name: name.into(),
            });
        }
        let reg = TEMP_REGISTERS[self.next_temp];
        self.next_temp += 1;
        self.env.insert(name.into(), reg);
        Ok(reg)
    }

    /// Resolve a variable name to its register.
    fn lookup(&self, name: &str) -> Result<u32, IIRRiscvError> {
        self.env.get(name).copied().ok_or_else(|| IIRRiscvError::UndefinedVariable {
            function: self.fn_name.into(),
            name: name.into(),
        })
    }
}

/// Lower one IIR function.  Emits the body words; no prologue/epilogue
/// stack-frame setup yet (v0.2.0 has no locals on the stack).
fn lower_function(func: &IIRFunction) -> Result<Vec<u32>, IIRRiscvError> {
    let mut state = FnState {
        fn_name: &func.name,
        env: HashMap::new(),
        next_temp: 0,
        labels: HashMap::new(),
        branches: Vec::new(),
    };
    // Bind parameters to a0..a7.  Validator already guarantees count <= 8.
    for (i, (pname, _)) in func.params.iter().enumerate() {
        state.env.insert(pname.clone(), ARG_REGISTERS[i]);
    }
    let mut words = Vec::with_capacity(func.instructions.len() + 1);
    for instr in &func.instructions {
        lower_instr(instr, &mut state, &mut words)?;
    }

    // ── Second pass: resolve branch offsets ─────────────────────────────
    //
    // After all instructions are emitted we know the byte offset of every
    // label (recorded in `state.labels`).  Walk the pending patch list
    // and replace each placeholder word with the real encoded branch.
    //
    // PC-relative offset = target_byte - source_byte.  RV32I branch
    // encoders take a signed byte offset directly; range-check before
    // re-encoding.
    for (word_idx, label_name, kind) in &state.branches {
        let target = state.labels.get(label_name).copied().ok_or_else(|| {
            IIRRiscvError::UndefinedLabel {
                function: state.fn_name.into(),
                label: label_name.clone(),
            }
        })?;
        let src_byte = (*word_idx) * 4;
        let offset = target as i64 - src_byte as i64;
        let new_word = match kind {
            BranchKind::BeqZero { rs1 } => {
                check_branch_offset(state.fn_name, label_name, offset, 4096)?;
                encode_beq(*rs1, X0_ZERO, offset as i32)
            }
            BranchKind::BneZero { rs1 } => {
                check_branch_offset(state.fn_name, label_name, offset, 4096)?;
                encode_bne(*rs1, X0_ZERO, offset as i32)
            }
            BranchKind::JalDiscard => {
                check_branch_offset(state.fn_name, label_name, offset, 1 << 20)?;
                encode_jal(X0_ZERO, offset as i32)
            }
        };
        words[*word_idx] = new_word;
    }

    Ok(words)
}

fn check_branch_offset(
    fn_name: &str,
    label: &str,
    offset: i64,
    max: i64,
) -> Result<(), IIRRiscvError> {
    if offset < -max || offset >= max {
        return Err(IIRRiscvError::BranchOutOfRange {
            function: fn_name.into(),
            label: label.into(),
            offset,
            max,
        });
    }
    Ok(())
}

/// Emit one IIR instruction's worth of RV32I words.
fn lower_instr(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut Vec<u32>,
) -> Result<(), IIRRiscvError> {
    match instr.op.as_str() {
        // ── const dest, Int(n) ──────────────────────────────────────────
        //
        // Three lowering paths by literal width:
        //
        //   * `n` fits in i12 ([-2048, 2047]) — single `addi rd, x0, n`.
        //   * `n` fits in i32 — `lui rd, upper20 + carry; addi rd, rd, lower12`.
        //     The lower-12-bit field is sign-extended by `addi`, so when the
        //     low bit is set we increment `upper20` to compensate.  This is
        //     the standard RISC-V wide-immediate idiom.
        //   * `n` outside i32 — `ImmediateOutOfRange`.  64-bit literals
        //     need a register-pair shuffle and arrive in A1++.5.
        "const" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRRiscvError::InvalidOperand {
                function: state.fn_name.into(),
                detail: "const requires a dest".into(),
            })?;
            let n = match instr.srcs.first() {
                Some(Operand::Int(n)) => *n,
                Some(Operand::Bool(b)) => if *b { 1 } else { 0 },
                _ => return Err(IIRRiscvError::InvalidOperand {
                    function: state.fn_name.into(),
                    detail: "const srcs[0] must be Int or Bool".into(),
                }),
            };
            if !((i32::MIN as i64)..=(i32::MAX as i64)).contains(&n) {
                return Err(IIRRiscvError::ImmediateOutOfRange {
                    function: state.fn_name.into(),
                    value: n,
                });
            }
            let rd = state.alloc_temp(dest)?;
            emit_const_i32(rd, n as i32, out);
            Ok(())
        }

        // ── mov dest, src — `addi rd, rs1, 0` (canonical move) ──────────
        "mov" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRRiscvError::InvalidOperand {
                function: state.fn_name.into(),
                detail: "mov requires a dest".into(),
            })?;
            let src = match instr.srcs.first() {
                Some(Operand::Var(s)) => s.clone(),
                _ => return Err(IIRRiscvError::InvalidOperand {
                    function: state.fn_name.into(),
                    detail: "mov srcs[0] must be Var".into(),
                }),
            };
            let rs1 = state.lookup(&src)?;
            let rd = state.alloc_temp(dest)?;
            out.push(encode_addi(rd, rs1, 0));
            Ok(())
        }

        // ── add / sub ───────────────────────────────────────────────────
        //
        // `add dest, a, b` → R-type `add rd, rs1, rs2`.
        // `sub dest, a, b` → R-type `sub rd, rs1, rs2`.
        // Signedness is irrelevant for both (two's-complement on RV32I).
        "add" | "sub" => {
            let dest = instr.dest.as_deref().ok_or_else(|| IIRRiscvError::InvalidOperand {
                function: state.fn_name.into(),
                detail: format!("{} requires a dest", instr.op),
            })?;
            let a = match instr.srcs.first() {
                Some(Operand::Var(s)) => s.clone(),
                _ => return Err(IIRRiscvError::InvalidOperand {
                    function: state.fn_name.into(),
                    detail: format!("{} srcs[0] must be Var", instr.op),
                }),
            };
            let b = match instr.srcs.get(1) {
                Some(Operand::Var(s)) => s.clone(),
                _ => return Err(IIRRiscvError::InvalidOperand {
                    function: state.fn_name.into(),
                    detail: format!("{} srcs[1] must be Var", instr.op),
                }),
            };
            let rs1 = state.lookup(&a)?;
            let rs2 = state.lookup(&b)?;
            let rd  = state.alloc_temp(dest)?;
            let word = if instr.op == "add" {
                encode_add(rd, rs1, rs2)
            } else {
                encode_sub(rd, rs1, rs2)
            };
            out.push(word);
            Ok(())
        }

        // ── ret <var> — move var to a0 (mv pseudo), then ret ────────────
        "ret" => {
            let src = match instr.srcs.first() {
                Some(Operand::Var(s)) => s.clone(),
                _ => return Err(IIRRiscvError::InvalidOperand {
                    function: state.fn_name.into(),
                    detail: "ret srcs[0] must be Var".into(),
                }),
            };
            let rs1 = state.lookup(&src)?;
            // mv a0, rs1   →   addi a0, rs1, 0
            // Skip the move if the value already lives in a0 — common
            // when the return value comes from arg0 of a 1-param fn.
            if rs1 != ARG_REGISTERS[0] {
                out.push(encode_addi(ARG_REGISTERS[0], rs1, 0));
            }
            out.push(encode_jalr(X0_ZERO, X1_RA, 0));
            Ok(())
        }

        // ── ret_void — just `ret` ────────────────────────────────────────
        "ret_void" => {
            out.push(encode_jalr(X0_ZERO, X1_RA, 0));
            Ok(())
        }

        // ── eq/ne/lt/le/gt/ge (and cmp_-prefixed variants per G1) ───────
        //
        // Each produces an i32 0/1 result in `dest` — same convention as
        // the wasm and LLVM backends.  Sign comes from the IIR type_hint
        // (u*-prefixed types use `sltu`; i*-prefixed types use `slt`).
        //
        // Output uses dest both as src and dst for the xor/xori synth
        // patterns so no extra temp is needed — matters because A1++
        // hasn't shipped stack spilling yet, so the temp pool is still
        // 7 deep.
        "eq" | "ne" | "lt" | "le" | "gt" | "ge"
        | "cmp_eq" | "cmp_ne" | "cmp_lt" | "cmp_le" | "cmp_gt" | "cmp_ge" => {
            let bare = instr.op.strip_prefix("cmp_").unwrap_or(instr.op.as_str());
            lower_cmp(bare, instr, state, out)
        }

        // ── call_builtin "<name>" — host call via ecall ─────────────────
        //
        // Layout: srcs = [Var("<builtin>"), Var(val)], dest = None.
        // For `print_i64`: load the syscall number ECALL_PRINT_I64_NUM
        // into a7, ensure the value is in a0, then `ecall`.
        "call_builtin" => lower_call_builtin(instr, state, out),

        // ── label "<name>": record byte offset for backpatching ─────────
        //
        // `label` emits zero machine words — it's purely a marker.  The
        // current byte offset is `out.len() * 4` (each word is 4 bytes).
        // A duplicate label name silently overwrites; that's a frontend
        // bug we could catch with a validator pass, but for v0.3.1 we
        // accept any name and let the latest definition win.
        "label" => {
            let name = match instr.srcs.first() {
                Some(Operand::Var(s)) => s.clone(),
                _ => return Err(IIRRiscvError::InvalidOperand {
                    function: state.fn_name.into(),
                    detail: "label requires srcs[0] = Operand::Var(name)".into(),
                }),
            };
            state.labels.insert(name, out.len() * 4);
            Ok(())
        }

        // ── jmp "<name>": unconditional `jal x0, +offset` ───────────────
        //
        // We emit a placeholder zero word and record a patch site so the
        // second pass can compute the PC-relative offset once the label
        // is known.
        "jmp" => {
            let target = match instr.srcs.first() {
                Some(Operand::Var(s)) => s.clone(),
                _ => return Err(IIRRiscvError::InvalidOperand {
                    function: state.fn_name.into(),
                    detail: "jmp requires srcs[0] = Operand::Var(target_label)".into(),
                }),
            };
            state.branches.push((out.len(), target, BranchKind::JalDiscard));
            out.push(0); // placeholder
            Ok(())
        }

        // ── jmp_if_true / jmp_if_false ──────────────────────────────────
        //
        // The cond var is interpreted as a boolean (any non-zero = true).
        // We compare against `x0`:
        //
        //   jmp_if_true  cond, L  →  bne cond, x0, L
        //   jmp_if_false cond, L  →  beq cond, x0, L
        //
        // Operand layout: srcs = [Var(cond), Var(target_label)].
        "jmp_if_true" | "jmp_if_false" => {
            let cond_name = match instr.srcs.first() {
                Some(Operand::Var(s)) => s.clone(),
                _ => return Err(IIRRiscvError::InvalidOperand {
                    function: state.fn_name.into(),
                    detail: format!("{} requires srcs[0] = Operand::Var(cond)", instr.op),
                }),
            };
            let target = match instr.srcs.get(1) {
                Some(Operand::Var(s)) => s.clone(),
                _ => return Err(IIRRiscvError::InvalidOperand {
                    function: state.fn_name.into(),
                    detail: format!("{} requires srcs[1] = Operand::Var(target_label)", instr.op),
                }),
            };
            let rs1 = state.lookup(&cond_name)?;
            let kind = if instr.op == "jmp_if_true" {
                BranchKind::BneZero { rs1 }
            } else {
                BranchKind::BeqZero { rs1 }
            };
            state.branches.push((out.len(), target, kind));
            out.push(0); // placeholder
            Ok(())
        }

        other => Err(IIRRiscvError::UnsupportedOp {
            function: state.fn_name.into(),
            op: other.into(),
        }),
    }
}

// ---------------------------------------------------------------------------
// A1++ helpers — wide consts, comparisons, ecall
// ---------------------------------------------------------------------------

fn is_unsigned_type_hint(t: &str) -> bool {
    t.starts_with('u')
}

/// Materialize an i32 constant into `rd` using the smallest RV32I sequence.
///
/// * If `n` fits in i12 (`[-2048, 2047]`) we emit a single
///   `addi rd, x0, n`.
/// * Otherwise we emit the canonical `lui + addi` wide-immediate idiom:
///   ```text
///   lui  rd, upper20_with_carry
///   addi rd, rd, lower12_signed
///   ```
///   The lower-12-bit slot is sign-extended by `addi`, so when it is
///   negative we add 1 to the upper 20 bits to compensate.  This is the
///   exact sequence GNU and LLVM assemblers emit for `li rd, imm32`.
fn emit_const_i32(rd: u32, n: i32, out: &mut Vec<u32>) {
    if (-2048..=2047).contains(&n) {
        out.push(encode_addi(rd, X0_ZERO, n));
        return;
    }
    // Compute lower12 (signed) and upper20 with carry.
    let lower = n & 0xFFF;
    let lower_signed: i32 = if lower & 0x800 != 0 { lower - 0x1000 } else { lower };
    // Adjust upper: if lower was negative (sign-extended) we add 1.
    // Wrapping_add handles the (rare) lui imm wraparound at i32::MAX boundary.
    let upper: u32 = ((n.wrapping_sub(lower_signed)) as u32) >> 12;
    out.push(encode_lui(rd, upper & 0xFFFFF));
    if lower_signed != 0 {
        out.push(encode_addi(rd, rd, lower_signed));
    }
}

fn lower_cmp(
    bare: &str,
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut Vec<u32>,
) -> Result<(), IIRRiscvError> {
    let dest = instr.dest.as_deref().ok_or_else(|| IIRRiscvError::InvalidOperand {
        function: state.fn_name.into(),
        detail: format!("{bare} requires a dest"),
    })?;
    let a = match instr.srcs.first() {
        Some(Operand::Var(s)) => s.clone(),
        _ => return Err(IIRRiscvError::InvalidOperand {
            function: state.fn_name.into(),
            detail: format!("{bare} srcs[0] must be Var"),
        }),
    };
    let b = match instr.srcs.get(1) {
        Some(Operand::Var(s)) => s.clone(),
        _ => return Err(IIRRiscvError::InvalidOperand {
            function: state.fn_name.into(),
            detail: format!("{bare} srcs[1] must be Var"),
        }),
    };
    let rs1 = state.lookup(&a)?;
    let rs2 = state.lookup(&b)?;
    let rd  = state.alloc_temp(dest)?;
    let is_u = is_unsigned_type_hint(&instr.type_hint);

    // Comparison idioms — all produce i32 0/1 in `rd`.  Sign comes from
    // the operand type hint (u* → unsigned variant of slt).  We reuse
    // `rd` as both src and dst for the xor/xori synth patterns so no
    // extra temp is needed (matters until A1++ ships stack spilling).
    match bare {
        "lt" => {
            // a < b
            out.push(if is_u { encode_sltu(rd, rs1, rs2) } else { encode_slt(rd, rs1, rs2) });
        }
        "gt" => {
            // a > b   ⇔   b < a
            out.push(if is_u { encode_sltu(rd, rs2, rs1) } else { encode_slt(rd, rs2, rs1) });
        }
        "le" => {
            // a <= b  ⇔   !(b < a)
            out.push(if is_u { encode_sltu(rd, rs2, rs1) } else { encode_slt(rd, rs2, rs1) });
            out.push(encode_xori(rd, rd, 1));
        }
        "ge" => {
            // a >= b  ⇔   !(a < b)
            out.push(if is_u { encode_sltu(rd, rs1, rs2) } else { encode_slt(rd, rs1, rs2) });
            out.push(encode_xori(rd, rd, 1));
        }
        "eq" => {
            // a == b  ⇔   sltiu(a ^ b, 1)
            out.push(encode_xor(rd, rs1, rs2));
            out.push(encode_sltiu(rd, rd, 1));
        }
        "ne" => {
            // a != b  ⇔   sltu(x0, a ^ b)
            out.push(encode_xor(rd, rs1, rs2));
            out.push(encode_sltu(rd, X0_ZERO, rd));
        }
        _ => unreachable!("lower_cmp called with non-cmp op {bare}"),
    }
    Ok(())
}

fn lower_call_builtin(
    instr: &IIRInstr,
    state: &mut FnState,
    out: &mut Vec<u32>,
) -> Result<(), IIRRiscvError> {
    let name = match instr.srcs.first() {
        Some(Operand::Var(s)) => s.clone(),
        _ => return Err(IIRRiscvError::InvalidOperand {
            function: state.fn_name.into(),
            detail: "call_builtin requires srcs[0] = Operand::Var(builtin_name)".into(),
        }),
    };
    if !SUPPORTED_BUILTINS.contains(&name.as_str()) {
        return Err(IIRRiscvError::UnsupportedOp {
            function: state.fn_name.into(),
            op: format!("call_builtin {name:?}: not in RV32I backend whitelist"),
        });
    }
    match name.as_str() {
        "print_i64" => {
            // Sequence:
            //   addi a0, val_reg, 0     ; ensure value is in a0 (skip if already)
            //   addi a7, x0, ECALL_PRINT_I64_NUM
            //   ecall
            //
            // `a7` is the RV32 syscall-number register by convention; we
            // pick `1` (Linux __NR_write slot) as the print_i64 sentinel.
            // A future real-syscall pass can fold this into write(2).
            let val_name = match instr.srcs.get(1) {
                Some(Operand::Var(s)) => s.clone(),
                _ => return Err(IIRRiscvError::InvalidOperand {
                    function: state.fn_name.into(),
                    detail: "call_builtin \"print_i64\" requires srcs[1] = Operand::Var(val)".into(),
                }),
            };
            let val_reg = state.lookup(&val_name)?;
            if val_reg != ARG_REGISTERS[0] {
                out.push(encode_addi(ARG_REGISTERS[0], val_reg, 0));
            }
            // a7 is x17 = ARG_REGISTERS[7].
            out.push(encode_addi(ARG_REGISTERS[7], X0_ZERO, ECALL_PRINT_I64_NUM));
            out.push(encode_ecall());
            Ok(())
        }
        _ => unreachable!("SUPPORTED_BUILTINS guard above prevents this"),
    }
}
