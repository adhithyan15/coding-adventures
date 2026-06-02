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
use riscv_simulator::encoding::{encode_add, encode_addi, encode_jalr, encode_sub};

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
    "const", "mov", "ret", "ret_void", "add", "sub",
];

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

/// Per-function state shared by `lower_function` and `lower_instr`.
struct FnState<'a> {
    fn_name: &'a str,
    /// IIR var name → assigned RV32I register index (`x*`).
    env: HashMap<String, u32>,
    /// Next free index into [`TEMP_REGISTERS`].  When this exceeds the
    /// pool length we return `OutOfRegisters` — A1++ replaces this with
    /// a stack-spilling allocator.
    next_temp: usize,
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
    };
    // Bind parameters to a0..a7.  Validator already guarantees count <= 8.
    for (i, (pname, _)) in func.params.iter().enumerate() {
        state.env.insert(pname.clone(), ARG_REGISTERS[i]);
    }
    let mut words = Vec::with_capacity(func.instructions.len() + 1);
    for instr in &func.instructions {
        lower_instr(instr, &mut state, &mut words)?;
    }
    Ok(words)
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
        // Lower to `addi rd, x0, imm12`.  This is the canonical
        // small-constant lowering on RISC-V: `x0` is hardwired to zero,
        // so `addi rd, x0, n` computes `n`.  For values outside the
        // 12-bit signed range we'd need `lui + addi`; v0.2.0 returns
        // ImmediateOutOfRange instead and defers that to A1++.
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
            if !(-2048..=2047).contains(&n) {
                return Err(IIRRiscvError::ImmediateOutOfRange {
                    function: state.fn_name.into(),
                    value: n,
                });
            }
            let rd = state.alloc_temp(dest)?;
            out.push(encode_addi(rd, X0_ZERO, n as i32));
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

        other => Err(IIRRiscvError::UnsupportedOp {
            function: state.fn_name.into(),
            op: other.into(),
        }),
    }
}
