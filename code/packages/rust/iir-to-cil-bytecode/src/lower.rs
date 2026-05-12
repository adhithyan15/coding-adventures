//! IIR → CLR CIL lowering pass.
//!
//! This module converts an [`IIRModule`] into a [`CILProgramArtifact`] by
//! emitting CIL method bodies using [`CILBytecodeBuilder`].
//!
//! # CIL execution model (mini-primer)
//!
//! CIL (Common Intermediate Language) is a **stack machine**.  Every
//! instruction operates on an implicit evaluation stack:
//!
//! ```text
//! ldc.i4.3     →  stack: [3]
//! ldc.i4.4     →  stack: [3, 4]
//! add          →  stack: [7]   (pops 3 and 4, pushes 7)
//! stloc.0      →  stack: []   (pops 7, stores in local variable 0)
//! ```
//!
//! Unlike BEAM (which is a register machine), CIL has no explicit named
//! temporaries — all intermediate values live on the stack.  Locals
//! (`ldloc`/`stloc`) and method parameters (`ldarg`/`starg`) are how
//! long-lived values survive across instructions.
//!
//! # Register allocation strategy
//!
//! IIR uses named SSA variables (`"a"`, `"result"`, …).  CIL has:
//!
//! - **Method arguments** (0-based): loaded with `ldarg`, stored with `starg`.
//! - **Local variables** (0-based): loaded with `ldloc`, stored with `stloc`.
//!
//! Two-pass allocation:
//!
//! 1. **Pass 1** — scan the function for all distinct variable names.
//!    Parameters are assigned **argument** slots 0..N-1 in order.
//!    All other variables are assigned **local** slots 0..M-1 in order.
//!
//! 2. **Pass 2** — translate each IIR instruction using the slot map,
//!    pushing/popping CIL stack values with `ldloc`/`stloc`/`ldarg`/`starg`
//!    as appropriate.
//!
//! # CIL synthesis for derived operations
//!
//! Some IIR operations have no single CIL opcode:
//!
//! | IIR | CIL synthesis |
//! |-----|---------------|
//! | `mod` | `rem` (opcode 0x5D — not in the `CILOpcode` enum, emitted raw) |
//! | `neg` | `neg` (opcode 0x65 — raw) |
//! | `not` | `not` (opcode 0x66 — bitwise complement, raw) |
//! | `cmp_ne r1 r2` | `ceq; ldc.i4.0; ceq` (NOT of equality) |
//! | `cmp_le r1 r2` | `cgt; ldc.i4.0; ceq` (NOT of greater-than) |
//! | `cmp_ge r1 r2` | `clt; ldc.i4.0; ceq` (NOT of less-than) |
//!
//! All three derived comparisons use the **double-invert** pattern: compute
//! the negation of the complementary comparison, then compare against 0 to
//! produce the boolean result.  This is the standard CIL idiom used by the
//! C# compiler.

use std::collections::HashMap;

use interpreter_ir::{IIRModule, Operand};
use ir_to_cil_bytecode::backend::{CILMethodArtifact, CILProgramArtifact};
use ir_to_cil_bytecode::builder::{CILBranchKind, CILBytecodeBuilder};
use ir_to_cil_bytecode::OBJECT_ARRAY_TYPE_TOKEN;

/// Sentinel token for `System.Console.WriteLine(int64)`.
///
/// In a real CLR PE file this is a MemberRef metadata token (table 0x0A,
/// row 2).  For simulation, backends that parse the token emit the appropriate
/// `call` instruction.  Row 2 is the second entry in the MemberRef table,
/// which by convention we reserve for `Console.WriteLine(int64)`.
const CONSOLE_WRITELINE_I64_TOKEN: u32 = 0x0A00_0002;

use crate::validate::validate_iir_for_clr;

// ===========================================================================
// IIRClrConfig
// ===========================================================================

/// Configuration for the IIR → CLR CIL lowering pass.
///
/// Currently only the assembly name is configurable.  CLR assembly names are
/// typically CamelCase identifiers, e.g. `"MyApp"`, `"Calculator"`.
#[derive(Debug, Clone)]
pub struct IIRClrConfig {
    /// The CLR assembly name.  Used as a label prefix in the artifact.
    pub assembly_name: String,
}

impl Default for IIRClrConfig {
    fn default() -> Self {
        Self { assembly_name: "IIRAssembly".to_string() }
    }
}

impl IIRClrConfig {
    /// Create a config with the given assembly name.
    ///
    /// # Example
    ///
    /// ```
    /// use iir_to_cil_bytecode::lower::IIRClrConfig;
    /// let cfg = IIRClrConfig::new("MyLib");
    /// assert_eq!(cfg.assembly_name, "MyLib");
    /// ```
    pub fn new(name: impl Into<String>) -> Self {
        Self { assembly_name: name.into() }
    }
}

// ===========================================================================
// IIRClrError
// ===========================================================================

/// Errors that can occur during IIR → CLR CIL lowering.
///
/// Every variant carries at minimum the function name where the error
/// occurred, which makes error messages actionable without requiring the
/// caller to track execution context.
#[derive(Debug, Clone, PartialEq)]
pub enum IIRClrError {
    /// The module failed pre-flight validation (see [`validate_iir_for_clr`]).
    ValidationFailed(Vec<String>),
    /// An IIR opcode that is not supported by this CLR backend.
    UnsupportedOp { function: String, op: String },
    /// An instruction has a type that cannot be lowered to CIL.
    UnsupportedType { function: String, type_hint: String },
    /// A branch targets a label name that has no definition in the function.
    UndefinedLabel { function: String, label: String },
    /// A `Var` operand references a variable name that was never defined.
    UndefinedVariable { function: String, name: String },
    /// An operand has an unexpected shape (e.g. Float where Int was expected).
    InvalidOperand { function: String, detail: String },
    /// The `CILBytecodeBuilder` could not assemble the method body.
    AssemblyError { function: String, detail: String },
}

impl std::fmt::Display for IIRClrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ValidationFailed(errs) => {
                write!(f, "validation failed:\n  {}", errs.join("\n  "))
            }
            Self::UnsupportedOp { function, op } => {
                write!(f, "function {function:?}: unsupported op {op:?}")
            }
            Self::UnsupportedType { function, type_hint } => {
                write!(f, "function {function:?}: unsupported type {type_hint:?}")
            }
            Self::UndefinedLabel { function, label } => {
                write!(f, "function {function:?}: undefined label {label:?}")
            }
            Self::UndefinedVariable { function, name } => {
                write!(f, "function {function:?}: undefined variable {name:?}")
            }
            Self::InvalidOperand { function, detail } => {
                write!(f, "function {function:?}: invalid operand: {detail}")
            }
            Self::AssemblyError { function, detail } => {
                write!(f, "function {function:?}: assembly error: {detail}")
            }
        }
    }
}

impl std::error::Error for IIRClrError {}

// ===========================================================================
// RegInfo — per-variable slot descriptor
// ===========================================================================

/// Describes how to access a named IIR variable in CIL.
///
/// CIL has two separate register files:
/// - **Method arguments** (0-based): `ldarg`/`starg`.
/// - **Local variables** (0-based): `ldloc`/`stloc`.
///
/// Whether a variable is a parameter or a local affects which instruction we
/// emit for loading and storing it.
#[derive(Debug, Clone)]
struct RegInfo {
    /// Slot index (argument index for params, local index for others).
    idx: u32,
    /// `true` if this slot is a method argument; `false` for a local.
    is_param: bool,
}

// ===========================================================================
// Emit helpers
// ===========================================================================

/// Load a variable onto the CIL evaluation stack.
///
/// Emits `ldarg` for method parameters and `ldloc` for local variables.
/// Both have short forms (0..3 → single byte), a byte form (4..255 → 2 bytes),
/// and a wide form (256+ → 4 bytes); the builder handles this automatically.
///
/// Returns an error if the slot index exceeds the CIL encoding limit
/// (`ldarg` uses u8 indices — max 255 params; `ldloc` uses u16 — max 65535 locals).
fn emit_load(builder: &mut CILBytecodeBuilder, info: &RegInfo, fn_name: &str) -> Result<(), IIRClrError> {
    if info.is_param {
        let idx = u8::try_from(info.idx).map_err(|_| IIRClrError::InvalidOperand {
            function: fn_name.to_string(),
            detail: format!(
                "ldarg index {} exceeds u8 max (255); CIL allows at most 256 parameters",
                info.idx
            ),
        })?;
        builder.emit_ldarg(idx);
    } else {
        let idx = u16::try_from(info.idx).map_err(|_| IIRClrError::InvalidOperand {
            function: fn_name.to_string(),
            detail: format!(
                "ldloc index {} exceeds u16 max (65535); CIL allows at most 65536 locals",
                info.idx
            ),
        })?;
        builder.emit_ldloc(idx);
    }
    Ok(())
}

/// Pop the top of the CIL stack and store it into a variable's slot.
///
/// Emits `starg` for method parameters and `stloc` for local variables.
/// Returns an error if the slot index exceeds the CIL encoding limit.
fn emit_store(builder: &mut CILBytecodeBuilder, info: &RegInfo, fn_name: &str) -> Result<(), IIRClrError> {
    if info.is_param {
        let idx = u8::try_from(info.idx).map_err(|_| IIRClrError::InvalidOperand {
            function: fn_name.to_string(),
            detail: format!(
                "starg index {} exceeds u8 max (255); CIL allows at most 256 parameters",
                info.idx
            ),
        })?;
        builder.emit_starg(idx);
    } else {
        let idx = u16::try_from(info.idx).map_err(|_| IIRClrError::InvalidOperand {
            function: fn_name.to_string(),
            detail: format!(
                "stloc index {} exceeds u16 max (65535); CIL allows at most 65536 locals",
                info.idx
            ),
        })?;
        builder.emit_stloc(idx);
    }
    Ok(())
}

// ===========================================================================
// lower_iir_to_cil — main entry point
// ===========================================================================

/// Lower an `IIRModule` to a `CILProgramArtifact`.
///
/// Each `IIRFunction` in the module is independently lowered to a
/// `CILMethodArtifact` containing assembled CIL body bytes.  The artifact
/// is structured with one method per function, ready to be wrapped in
/// `.method` headers by a PE/COFF packager or fed directly to a CLR simulator.
///
/// # Algorithm
///
/// 1. **Validate** the module; return `Err(ValidationFailed)` on errors.
/// 2. For each function, run **pass 1**: build a `reg_map` from variable
///    name → `RegInfo` (argument index for params, local index for others).
/// 3. For each function, run **pass 2**: emit CIL instructions into a
///    `CILBytecodeBuilder`.  The builder uses a two-pass branch-promotion
///    algorithm to produce the shortest valid byte stream.
/// 4. Call `builder.assemble()` to get the final `Vec<u8>` body.
/// 5. Construct `CILMethodArtifact { name, body, … }` and push to `methods`.
/// 6. Construct `CILProgramArtifact` from the collected methods.
///
/// # Errors
///
/// Returns [`IIRClrError::ValidationFailed`] if pre-flight validation fails.
/// Other variants are returned for malformed instruction operands or undefined
/// variable/label references (which should not occur for well-formed IIR
/// produced by a correct frontend).
pub fn lower_iir_to_cil(
    module: &IIRModule,
    _config: &IIRClrConfig,
) -> Result<CILProgramArtifact, IIRClrError> {
    // ── Step 1: pre-flight validation ──────────────────────────────────────
    let errs = validate_iir_for_clr(module);
    if !errs.is_empty() {
        return Err(IIRClrError::ValidationFailed(errs));
    }

    let mut methods: Vec<CILMethodArtifact> = Vec::with_capacity(module.functions.len());

    for func in &module.functions {
        let fn_name = &func.name;

        // ── Pass 1: build register map ─────────────────────────────────────
        //
        // The register map associates each named IIR variable with a CIL slot:
        // - Parameters get argument slots 0, 1, 2, … in declaration order.
        //   They are accessed with ldarg/starg.
        // - All other variables (dests of instructions and Var sources that
        //   are not params) get local slots 0, 1, 2, … in first-seen order.
        //   They are accessed with ldloc/stloc.
        //
        // This simple linear allocation is not optimal (it doesn't track
        // liveness), but it is correct.  A liveness-based allocator can be
        // swapped in later without changing the interface.

        let mut reg_map: HashMap<String, RegInfo> = HashMap::new();
        let mut next_local: u32 = 0;

        // Step 1a: assign argument slots for parameters.
        for (idx, (param_name, _param_type)) in func.params.iter().enumerate() {
            reg_map.insert(param_name.clone(), RegInfo {
                idx: idx as u32,
                is_param: true,
            });
        }

        // Step 1b: scan instructions for all other variable names.
        // We process Var sources before dests so that variables referenced
        // before their definition (e.g. forward-used labels) still get a slot.
        for instr in &func.instructions {
            // Source Var operands
            for src in &instr.srcs {
                if let Operand::Var(name) = src {
                    if !reg_map.contains_key(name.as_str()) {
                        reg_map.insert(name.clone(), RegInfo {
                            idx: next_local,
                            is_param: false,
                        });
                        next_local += 1;
                    }
                }
            }
            // Destination variable
            if let Some(dest) = &instr.dest {
                if !reg_map.contains_key(dest.as_str()) {
                    reg_map.insert(dest.clone(), RegInfo {
                        idx: next_local,
                        is_param: false,
                    });
                    next_local += 1;
                }
            }
        }

        let local_count = next_local as usize;
        let param_count = func.params.len();

        // ── Pass 2: emit CIL instructions ─────────────────────────────────
        //
        // We walk each IIR instruction and emit the equivalent CIL sequence.
        // The `CILBytecodeBuilder` accumulates unresolved branches and resolves
        // them in a second pass inside `assemble()`.

        let mut builder = CILBytecodeBuilder::new();

        // Helper: look up a variable's RegInfo or return UndefinedVariable.
        // We accept any expression that implements AsRef<str> by using
        // a two-step binding to avoid the unstable `str_as_str` feature.
        macro_rules! reg_info {
            ($name:expr) => {{
                let key: &str = &$name;
                match reg_map.get(key) {
                    Some(info) => info,
                    None => return Err(IIRClrError::UndefinedVariable {
                        function: fn_name.clone(),
                        name: key.to_string(),
                    }),
                }
            }};
        }

        for instr in &func.instructions {
            match instr.op.as_str() {

                // ── const Int / Bool ─────────────────────────────────────────
                //
                // `const` loads an immediate into a destination register.
                // CIL: push the value via `ldc.i4`; pop into the slot with
                // `stloc`/`starg`.
                //
                // Integer: ldc.i4 selects the shortest encoding automatically
                // (single byte for -1..8, sign-extended byte for -128..127,
                // full 4-byte little-endian otherwise).
                //
                // Bool: true = 1, false = 0, matching the CLR's native bool
                // representation (System.Boolean is stored as a 1-byte integer
                // in the runtime, but on the stack it is widened to int32).
                "const" => {
                    let dest_name = instr.dest.as_deref().ok_or_else(|| {
                        IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "const instruction must have a dest".into(),
                        }
                    })?;
                    let dest = reg_info!(dest_name).clone();

                    // Phase 2: `const` with type_hint `"ref<LispyPair>"` and no
                    // source operand represents the nil value.
                    //
                    // CIL: `ldnull; stloc <dest>`
                    //
                    // `ldnull` (0x14) pushes the null managed reference, which is
                    // the canonical representation of nil in this backend.  An
                    // empty list is just a null `object[]` reference.
                    if instr.type_hint == "ref<LispyPair>" && instr.srcs.is_empty() {
                        builder.emit_ldnull();
                        emit_store(&mut builder, &dest, fn_name)?;
                        continue;  // skip the srcs dispatch below
                    }

                    match instr.srcs.first() {
                        Some(Operand::Int(n)) => {
                            // Push the integer immediate in the most compact form.
                            builder.emit_ldc_i4(*n as i32);
                        }
                        Some(Operand::Bool(b)) => {
                            // Booleans map to 1 (true) and 0 (false) on the stack.
                            builder.emit_ldc_i4(if *b { 1 } else { 0 });
                        }
                        Some(Operand::Float(_)) => {
                            // Float constants are rejected by the validator, but
                            // guard here for defence-in-depth.
                            return Err(IIRClrError::UnsupportedType {
                                function: fn_name.clone(),
                                type_hint: "float const".into(),
                            });
                        }
                        Some(Operand::Var(v)) => {
                            // A Var src in a const is unusual; treat it as a copy.
                            let src = reg_info!(v).clone();
                            emit_load(&mut builder, &src, fn_name)?;
                        }
                        // LANG32: Str is a compile-time string literal (global variable name).
                        // The CLR backend doesn't yet support string-value constants; skip.
                        Some(Operand::Str(_)) => {
                            continue;
                        }
                        None => {
                            return Err(IIRClrError::InvalidOperand {
                                function: fn_name.clone(),
                                detail: "const instruction has no source operand".into(),
                            });
                        }
                    }

                    emit_store(&mut builder, &dest, fn_name)?;
                }

                // ── Binary arithmetic: add, sub, mul, div ────────────────────
                //
                // These map 1-to-1 to CIL opcodes.  Each:
                // 1. Pushes the left-hand source.
                // 2. Pushes the right-hand source.
                // 3. Emits the arithmetic opcode (pops two, pushes result).
                // 4. Stores the result into the destination slot.
                "add" | "sub" | "mul" | "div" => {
                    let dest_name = instr.dest.as_deref().ok_or_else(|| {
                        IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: format!("{} must have a dest", instr.op),
                        }
                    })?;
                    let dest = reg_info!(dest_name).clone();

                    let lhs = get_operand_reg(&instr.srcs, 0, &reg_map, fn_name)?;
                    let rhs = get_operand_reg(&instr.srcs, 1, &reg_map, fn_name)?;

                    emit_load(&mut builder, &lhs, fn_name)?;
                    emit_load(&mut builder, &rhs, fn_name)?;

                    match instr.op.as_str() {
                        "add" => builder.emit_add(),
                        "sub" => builder.emit_sub(),
                        "mul" => builder.emit_mul(),
                        "div" => builder.emit_div(),
                        _ => unreachable!(),
                    }

                    emit_store(&mut builder, &dest, fn_name)?;
                }

                // ── mod r1, r2 → rd ──────────────────────────────────────────
                //
                // CIL opcode `rem` (0x5D) computes the signed remainder.
                // This opcode is not included in the `CILOpcode` enum of the
                // upstream crate (which covers only the MVP subset), so we
                // emit it as a raw byte.
                //
                // Semantics: `rem` truncates toward zero, matching the IIR
                // `mod` definition (same as C# `%` and Rust `%`).
                "mod" => {
                    let dest_name = instr.dest.as_deref().ok_or_else(|| {
                        IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "mod must have a dest".into(),
                        }
                    })?;
                    let dest = reg_info!(dest_name).clone();

                    let lhs = get_operand_reg(&instr.srcs, 0, &reg_map, fn_name)?;
                    let rhs = get_operand_reg(&instr.srcs, 1, &reg_map, fn_name)?;

                    emit_load(&mut builder, &lhs, fn_name)?;
                    emit_load(&mut builder, &rhs, fn_name)?;
                    // `rem` opcode: 0x5D.  Not in CILOpcode enum, emitted raw.
                    builder.emit_raw(vec![0x5D]);
                    emit_store(&mut builder, &dest, fn_name)?;
                }

                // ── neg r → rd ───────────────────────────────────────────────
                //
                // CIL opcode `neg` (0x65) computes two's complement negation.
                // Not in the CILOpcode enum; emitted as a raw byte.
                "neg" => {
                    let dest_name = instr.dest.as_deref().ok_or_else(|| {
                        IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "neg must have a dest".into(),
                        }
                    })?;
                    let dest = reg_info!(dest_name).clone();

                    let src = get_operand_reg(&instr.srcs, 0, &reg_map, fn_name)?;
                    emit_load(&mut builder, &src, fn_name)?;
                    // `neg` opcode: 0x65.
                    builder.emit_raw(vec![0x65]);
                    emit_store(&mut builder, &dest, fn_name)?;
                }

                // ── Binary bitwise: and, or, xor, shl, shr ──────────────────
                //
                // These map 1-to-1 to CIL opcodes.  Same pattern as binary
                // arithmetic: push lhs, push rhs, emit opcode, store result.
                "and" | "or" | "xor" | "shl" | "shr" => {
                    let dest_name = instr.dest.as_deref().ok_or_else(|| {
                        IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: format!("{} must have a dest", instr.op),
                        }
                    })?;
                    let dest = reg_info!(dest_name).clone();

                    let lhs = get_operand_reg(&instr.srcs, 0, &reg_map, fn_name)?;
                    let rhs = get_operand_reg(&instr.srcs, 1, &reg_map, fn_name)?;

                    emit_load(&mut builder, &lhs, fn_name)?;
                    emit_load(&mut builder, &rhs, fn_name)?;

                    match instr.op.as_str() {
                        "and" => builder.emit_and(),
                        "or"  => builder.emit_or(),
                        "xor" => builder.emit_xor(),
                        "shl" => builder.emit_shl(),
                        "shr" => builder.emit_shr(),
                        _ => unreachable!(),
                    }

                    emit_store(&mut builder, &dest, fn_name)?;
                }

                // ── not r → rd ───────────────────────────────────────────────
                //
                // CIL opcode `not` (0x66) computes bitwise (one's complement)
                // negation.  Not in the CILOpcode enum; emitted as a raw byte.
                //
                // Note: the `ir-to-cil-bytecode` existing backend synthesizes
                // bitwise NOT as `ldc.i4.m1; xor`.  We use the native `not`
                // opcode here because it is shorter (1 byte vs 2).  Both are
                // semantically equivalent.
                "not" => {
                    let dest_name = instr.dest.as_deref().ok_or_else(|| {
                        IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "not must have a dest".into(),
                        }
                    })?;
                    let dest = reg_info!(dest_name).clone();

                    let src = get_operand_reg(&instr.srcs, 0, &reg_map, fn_name)?;
                    emit_load(&mut builder, &src, fn_name)?;
                    // `not` opcode: 0x66.
                    builder.emit_raw(vec![0x66]);
                    emit_store(&mut builder, &dest, fn_name)?;
                }

                // ── cmp_eq r1, r2 → rd ───────────────────────────────────────
                //
                // CIL: ldloc r1; ldloc r2; ceq; stloc rd
                //
                // `ceq` (0xFE 0x01) compares the top two stack values for
                // equality and pushes 1 (true) or 0 (false).
                "cmp_eq" => {
                    let dest_name = instr.dest.as_deref().ok_or_else(|| {
                        IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "cmp_eq must have a dest".into(),
                        }
                    })?;
                    let dest = reg_info!(dest_name).clone();
                    let r1 = get_operand_reg(&instr.srcs, 0, &reg_map, fn_name)?;
                    let r2 = get_operand_reg(&instr.srcs, 1, &reg_map, fn_name)?;

                    emit_load(&mut builder, &r1, fn_name)?;
                    emit_load(&mut builder, &r2, fn_name)?;
                    builder.emit_ceq();
                    emit_store(&mut builder, &dest, fn_name)?;
                }

                // ── cmp_lt r1, r2 → rd ───────────────────────────────────────
                //
                // CIL: ldloc r1; ldloc r2; clt; stloc rd
                //
                // `clt` (0xFE 0x04) pushes 1 if stack[-2] < stack[-1] (signed).
                "cmp_lt" => {
                    let dest_name = instr.dest.as_deref().ok_or_else(|| {
                        IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "cmp_lt must have a dest".into(),
                        }
                    })?;
                    let dest = reg_info!(dest_name).clone();
                    let r1 = get_operand_reg(&instr.srcs, 0, &reg_map, fn_name)?;
                    let r2 = get_operand_reg(&instr.srcs, 1, &reg_map, fn_name)?;

                    emit_load(&mut builder, &r1, fn_name)?;
                    emit_load(&mut builder, &r2, fn_name)?;
                    builder.emit_clt();
                    emit_store(&mut builder, &dest, fn_name)?;
                }

                // ── cmp_gt r1, r2 → rd ───────────────────────────────────────
                //
                // CIL: ldloc r1; ldloc r2; cgt; stloc rd
                //
                // `cgt` (0xFE 0x02) pushes 1 if stack[-2] > stack[-1] (signed).
                "cmp_gt" => {
                    let dest_name = instr.dest.as_deref().ok_or_else(|| {
                        IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "cmp_gt must have a dest".into(),
                        }
                    })?;
                    let dest = reg_info!(dest_name).clone();
                    let r1 = get_operand_reg(&instr.srcs, 0, &reg_map, fn_name)?;
                    let r2 = get_operand_reg(&instr.srcs, 1, &reg_map, fn_name)?;

                    emit_load(&mut builder, &r1, fn_name)?;
                    emit_load(&mut builder, &r2, fn_name)?;
                    builder.emit_cgt();
                    emit_store(&mut builder, &dest, fn_name)?;
                }

                // ── cmp_ne r1, r2 → rd ───────────────────────────────────────
                //
                // CIL does not have a native "compare not-equal" opcode.
                // We synthesize it as NOT(ceq):
                //   ldloc r1; ldloc r2; ceq; ldc.i4.0; ceq; stloc rd
                //
                // The first `ceq` pushes 1 if equal.  `ldc.i4.0` pushes 0.
                // The second `ceq` compares 1 vs 0 → pushes 0 (not equal case
                // when r1==r2), or compares 0 vs 0 → pushes 1 (r1!=r2 case).
                // This is the standard "boolean NOT" pattern in CIL.
                "cmp_ne" => {
                    let dest_name = instr.dest.as_deref().ok_or_else(|| {
                        IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "cmp_ne must have a dest".into(),
                        }
                    })?;
                    let dest = reg_info!(dest_name).clone();
                    let r1 = get_operand_reg(&instr.srcs, 0, &reg_map, fn_name)?;
                    let r2 = get_operand_reg(&instr.srcs, 1, &reg_map, fn_name)?;

                    emit_load(&mut builder, &r1, fn_name)?;
                    emit_load(&mut builder, &r2, fn_name)?;
                    builder.emit_ceq();                 // 1 if equal, 0 if not
                    builder.emit_ldc_i4(0);             // push 0
                    builder.emit_ceq();                 // NOT: 0 if equal, 1 if not
                    emit_store(&mut builder, &dest, fn_name)?;
                }

                // ── cmp_le r1, r2 → rd ───────────────────────────────────────
                //
                // Synthesized as NOT(cgt):
                //   ldloc r1; ldloc r2; cgt; ldc.i4.0; ceq; stloc rd
                //
                // `cgt` pushes 1 if r1 > r2.  Inverting gives 1 if r1 <= r2.
                "cmp_le" => {
                    let dest_name = instr.dest.as_deref().ok_or_else(|| {
                        IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "cmp_le must have a dest".into(),
                        }
                    })?;
                    let dest = reg_info!(dest_name).clone();
                    let r1 = get_operand_reg(&instr.srcs, 0, &reg_map, fn_name)?;
                    let r2 = get_operand_reg(&instr.srcs, 1, &reg_map, fn_name)?;

                    emit_load(&mut builder, &r1, fn_name)?;
                    emit_load(&mut builder, &r2, fn_name)?;
                    builder.emit_cgt();                 // 1 if r1 > r2
                    builder.emit_ldc_i4(0);
                    builder.emit_ceq();                 // NOT: 1 if r1 <= r2
                    emit_store(&mut builder, &dest, fn_name)?;
                }

                // ── cmp_ge r1, r2 → rd ───────────────────────────────────────
                //
                // Synthesized as NOT(clt):
                //   ldloc r1; ldloc r2; clt; ldc.i4.0; ceq; stloc rd
                //
                // `clt` pushes 1 if r1 < r2.  Inverting gives 1 if r1 >= r2.
                "cmp_ge" => {
                    let dest_name = instr.dest.as_deref().ok_or_else(|| {
                        IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "cmp_ge must have a dest".into(),
                        }
                    })?;
                    let dest = reg_info!(dest_name).clone();
                    let r1 = get_operand_reg(&instr.srcs, 0, &reg_map, fn_name)?;
                    let r2 = get_operand_reg(&instr.srcs, 1, &reg_map, fn_name)?;

                    emit_load(&mut builder, &r1, fn_name)?;
                    emit_load(&mut builder, &r2, fn_name)?;
                    builder.emit_clt();                 // 1 if r1 < r2
                    builder.emit_ldc_i4(0);
                    builder.emit_ceq();                 // NOT: 1 if r1 >= r2
                    emit_store(&mut builder, &dest, fn_name)?;
                }

                // ── label name ───────────────────────────────────────────────
                //
                // Labels in IIR are instructions that mark a named position.
                // In CIL, labels are not instructions — they are just named
                // positions in the byte stream.  The `CILBytecodeBuilder`
                // handles labels as zero-byte anchors (`builder.mark`).
                "label" => {
                    if let Some(Operand::Var(name)) = instr.srcs.first() {
                        builder.mark(name.as_str());
                    }
                    // If srcs[0] is not a Var (shouldn't happen for well-formed
                    // IIR), we silently skip — the module validator already
                    // checked label definitions.
                }

                // ── jmp name ────────────────────────────────────────────────
                //
                // Unconditional branch.  The `CILBytecodeBuilder` uses a
                // two-pass algorithm to pick the shortest encoding:
                // `br.s` (2 bytes, ±128) or `br` (5 bytes, ±2^31).
                "jmp" => {
                    let label_name = match instr.srcs.first() {
                        Some(Operand::Var(name)) => name.as_str(),
                        _ => return Err(IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "jmp requires a Var source (label name)".into(),
                        }),
                    };
                    builder.emit_branch(CILBranchKind::Always, label_name, false);
                }

                // ── jmp_if_true cond, name ───────────────────────────────────
                //
                // Branch to `name` if `cond` is non-zero (truthy).
                // CIL: ldloc cond; brtrue.s / brtrue label
                //
                // srcs = [Var(cond), Var(label_name)]
                "jmp_if_true" => {
                    let cond_name = match instr.srcs.first() {
                        Some(Operand::Var(name)) => name.as_str(),
                        _ => return Err(IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "jmp_if_true: first src must be Var(cond)".into(),
                        }),
                    };
                    let label_name = match instr.srcs.get(1) {
                        Some(Operand::Var(name)) => name.as_str(),
                        _ => return Err(IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "jmp_if_true: second src must be Var(label)".into(),
                        }),
                    };
                    let cond = reg_info!(cond_name).clone();
                    emit_load(&mut builder, &cond, fn_name)?;
                    builder.emit_branch(CILBranchKind::True, label_name, false);
                }

                // ── jmp_if_false cond, name ──────────────────────────────────
                //
                // Branch to `name` if `cond` is zero (falsy).
                // CIL: ldloc cond; brfalse.s / brfalse label
                //
                // srcs = [Var(cond), Var(label_name)]
                "jmp_if_false" => {
                    let cond_name = match instr.srcs.first() {
                        Some(Operand::Var(name)) => name.as_str(),
                        _ => return Err(IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "jmp_if_false: first src must be Var(cond)".into(),
                        }),
                    };
                    let label_name = match instr.srcs.get(1) {
                        Some(Operand::Var(name)) => name.as_str(),
                        _ => return Err(IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "jmp_if_false: second src must be Var(label)".into(),
                        }),
                    };
                    let cond = reg_info!(cond_name).clone();
                    emit_load(&mut builder, &cond, fn_name)?;
                    builder.emit_branch(CILBranchKind::False, label_name, false);
                }

                // ── ret value ────────────────────────────────────────────────
                //
                // Return a value from the current method.
                // CIL: ldloc value; ret
                "ret" => {
                    let src_name = match instr.srcs.first() {
                        Some(Operand::Var(name)) => name.as_str(),
                        _ => return Err(IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "ret: source must be Var(value)".into(),
                        }),
                    };
                    let src = reg_info!(src_name).clone();
                    emit_load(&mut builder, &src, fn_name)?;
                    builder.emit_ret();
                }

                // ── ret_void ─────────────────────────────────────────────────
                //
                // Return from a void method.
                // CIL: ret (no value on the stack)
                "ret_void" => {
                    builder.emit_ret();
                }

                // ── call fn_name, args… → dest ───────────────────────────────
                //
                // Call another function in the same module.
                //
                // IIR layout: op="call", dest=Some(result), srcs=[Var(fn_name), arg0, arg1, …]
                //
                // CIL calling convention for same-module calls:
                //   1. Push each argument onto the stack.
                //   2. `call <method_token>` — calls the method, pushes return value.
                //   3. Store the return value into `dest`.
                //
                // We use a simplified token: methods at ordinal `idx` in the
                // function list get token `0x06000001 + idx`.  This matches the
                // `SequentialCILTokenProvider` convention used by the upstream
                // `ir-to-cil-bytecode` crate.
                "call" => {
                    let callee_name = match instr.srcs.first() {
                        Some(Operand::Var(name)) => name.as_str(),
                        _ => return Err(IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "call: first src must be Var(function_name)".into(),
                        }),
                    };

                    // Find the 0-based ordinal of the callee in the module.
                    let callee_idx = module.functions.iter()
                        .position(|f| f.name == callee_name)
                        .ok_or_else(|| IIRClrError::UndefinedLabel {
                            function: fn_name.clone(),
                            label: callee_name.to_string(),
                        })?;

                    // Compute the CIL method token.
                    // Token table 0x06 = MethodDef; ordinal is 1-based.
                    // Use checked_add and return a proper error on overflow rather
                    // than silently emitting a wrong token (which would dispatch to
                    // the wrong method at runtime).
                    let method_token = 0x0600_0001u32
                        .checked_add(callee_idx as u32)
                        .ok_or_else(|| IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: format!(
                                "method token overflow for callee {:?} (index {})",
                                callee_name, callee_idx
                            ),
                        })?;

                    // Push arguments in order.
                    for src in instr.srcs.iter().skip(1) {
                        match src {
                            Operand::Var(name) => {
                                let info = reg_info!(name).clone();
                                emit_load(&mut builder, &info, fn_name)?;
                            }
                            Operand::Int(n) => {
                                builder.emit_ldc_i4(*n as i32);
                            }
                            Operand::Bool(b) => {
                                builder.emit_ldc_i4(if *b { 1 } else { 0 });
                            }
                            Operand::Float(_) => {
                                return Err(IIRClrError::UnsupportedType {
                                    function: fn_name.clone(),
                                    type_hint: "float argument".into(),
                                });
                            }
                            // LANG32: Str is a compile-time string literal — not a passable
                            // call argument in V1.
                            Operand::Str(_) => {
                                return Err(IIRClrError::UnsupportedType {
                                    function: fn_name.clone(),
                                    type_hint: "str argument — string args not yet supported".into(),
                                });
                            }
                        }
                    }

                    // Emit the call instruction.
                    builder.emit_call(method_token);

                    // Store the return value if the call has a destination.
                    if let Some(dest_name) = &instr.dest {
                        let dest = reg_info!(dest_name).clone();
                        emit_store(&mut builder, &dest, fn_name)?;
                    } else {
                        // No destination: discard the return value with `pop`.
                        builder.emit_pop();
                    }
                }

                // ── load_reg v → rd ──────────────────────────────────────────
                //
                // Copy variable `v` into `rd`.
                // CIL: ldloc v; stloc rd
                "load_reg" => {
                    let dest_name = instr.dest.as_deref().ok_or_else(|| {
                        IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "load_reg must have a dest".into(),
                        }
                    })?;
                    let dest = reg_info!(dest_name).clone();
                    let src = get_operand_reg(&instr.srcs, 0, &reg_map, fn_name)?;
                    emit_load(&mut builder, &src, fn_name)?;
                    emit_store(&mut builder, &dest, fn_name)?;
                }

                // ── store_reg v, src ─────────────────────────────────────────
                //
                // Write `src` into the slot of variable `v`.
                // srcs = [Var(v), Var(src)]
                // CIL: ldloc src; stloc v
                "store_reg" => {
                    let v = get_operand_reg(&instr.srcs, 0, &reg_map, fn_name)?;
                    let src = get_operand_reg(&instr.srcs, 1, &reg_map, fn_name)?;
                    emit_load(&mut builder, &src, fn_name)?;
                    emit_store(&mut builder, &v, fn_name)?;
                }

                // ── type_assert ──────────────────────────────────────────────
                //
                // A frontend hint that a variable has a given type.  The CLR
                // enforces type safety through the JIT verifier; there is no
                // explicit "assert type" instruction in CIL.  We emit a `nop`
                // to preserve instruction-count parity with the source, which
                // helps debuggers that map CIL offsets back to source lines.
                "type_assert" => {
                    builder.emit_nop();
                }

                // ── alloc ref<LispyPair> → dest ──────────────────────────────
                //
                // Allocate a new cons cell as a 2-element `System.Object[]`.
                //
                // In Lisp, every cons cell has two fields:
                //   index 0 → `car` (the head / first element)
                //   index 1 → `cdr` (the tail / rest of the list)
                //
                // The CLR garbage collector manages the object[]'s lifetime
                // automatically — no reference counting, no explicit free.
                //
                // CIL expansion:
                // ```text
                // ldc.i4.2                    ; array length = 2
                // newarr OBJECT_ARRAY_TYPE_TOKEN  ; allocate System.Object[2]
                // stloc  <dest>               ; store the ref in dest
                // ```
                //
                // The fields are written by subsequent `field_store` instructions.
                // We do NOT pre-initialize the slots; the CLR zero-initialises
                // all array elements to null on allocation.  This means an
                // uninitialized `car`/`cdr` is already null (= nil), which is
                // the correct default for a fresh cons cell.
                "alloc" => {
                    let dest_name = instr.dest.as_deref().ok_or_else(|| {
                        IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "alloc instruction must have a dest".into(),
                        }
                    })?;
                    let dest = reg_info!(dest_name).clone();

                    // ldc.i4.2: the array will have exactly 2 slots.
                    builder.emit_ldc_i4(2);
                    // newarr allocates a 1-D array of the given element type.
                    // We use the sentinel token for System.Object[].
                    builder.emit_newarr(OBJECT_ARRAY_TYPE_TOKEN);
                    // Store the fresh array reference in the destination slot.
                    emit_store(&mut builder, &dest, fn_name)?;
                }

                // ── field_load dest pair idx → dest ──────────────────────────
                //
                // Load field `idx` from a cons-cell pair into `dest`.
                // This implements both `car` (idx=0) and `cdr` (idx=1).
                //
                // IIR layout: op="field_load", dest=Some(dest), srcs=[Var(pair), Int(idx)]
                //
                // CIL expansion:
                // ```text
                // ldloc  <pair>               ; push the pair (object[]) ref
                // ldc.i4 <idx>                ; push field index (0 or 1)
                // ldelem.ref                  ; pop array+idx, push array[idx]
                // stloc  <dest>               ; pop and store result
                // ```
                //
                // `ldelem.ref` (0xA2) is the typed variant for reference arrays.
                // It performs a bounds check at runtime and throws
                // `IndexOutOfRangeException` if idx >= array.Length, which
                // gives a safe fail rather than a memory corruption.
                "field_load" => {
                    let dest_name = instr.dest.as_deref().ok_or_else(|| {
                        IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "field_load instruction must have a dest".into(),
                        }
                    })?;
                    let dest = reg_info!(dest_name).clone();

                    // srcs[0] = the pair (object[] reference)
                    let pair = get_operand_reg(&instr.srcs, 0, &reg_map, fn_name)?;
                    // srcs[1] = field index (must be Int 0 or 1)
                    let field_idx = match instr.srcs.get(1) {
                        Some(Operand::Int(n)) => *n as i32,
                        Some(other) => return Err(IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: format!(
                                "field_load: srcs[1] must be Int field index, got {:?}", other
                            ),
                        }),
                        None => return Err(IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "field_load: missing field index at srcs[1]".into(),
                        }),
                    };

                    emit_load(&mut builder, &pair, fn_name)?;   // push pair ref
                    builder.emit_ldc_i4(field_idx);             // push index
                    builder.emit_ldelem_ref();                   // array[idx]
                    emit_store(&mut builder, &dest, fn_name)?;  // pop to dest
                }

                // ── field_store pair idx value ────────────────────────────────
                //
                // Store `value` into field `idx` of cons-cell `pair`.
                // This implements writing `car` (idx=0) and `cdr` (idx=1).
                //
                // IIR layout: op="field_store", dest=None,
                //             srcs=[Var(pair), Int(idx), Var(value)]
                //
                // CIL expansion:
                // ```text
                // ldloc  <pair>               ; push the pair (object[]) ref
                // ldc.i4 <idx>                ; push field index
                // ldloc  <value>              ; push value to store
                // stelem.ref                  ; array[idx] = value; pops all three
                // ```
                //
                // `stelem.ref` (0xA4) takes three operands from the stack:
                //   1. array ref  (bottom)
                //   2. index      (middle)
                //   3. value ref  (top)
                // and stores value into array[index], performing a runtime
                // assignability check (similar to Java's aastore).
                "field_store" => {
                    // srcs[0] = pair (the target array)
                    let pair = get_operand_reg(&instr.srcs, 0, &reg_map, fn_name)?;
                    // srcs[1] = field index
                    let field_idx = match instr.srcs.get(1) {
                        Some(Operand::Int(n)) => *n as i32,
                        Some(other) => return Err(IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: format!(
                                "field_store: srcs[1] must be Int field index, got {:?}", other
                            ),
                        }),
                        None => return Err(IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "field_store: missing field index at srcs[1]".into(),
                        }),
                    };
                    // srcs[2] = value to write
                    let value = get_operand_reg(&instr.srcs, 2, &reg_map, fn_name)?;

                    emit_load(&mut builder, &pair, fn_name)?;   // push pair ref
                    builder.emit_ldc_i4(field_idx);             // push index
                    emit_load(&mut builder, &value, fn_name)?;  // push value
                    builder.emit_stelem_ref();                   // array[idx] = value
                }

                // ── is_null dest x ────────────────────────────────────────────
                //
                // Test whether `x` is a null reference (the IIR nil value).
                // Produces 1 (true) if `x == null`, 0 (false) otherwise.
                //
                // IIR layout: op="is_null", dest=Some(dest), srcs=[Var(x)]
                //
                // CIL expansion:
                // ```text
                // ldloc  <x>                  ; push the value to test
                // ldnull                      ; push null reference
                // ceq                         ; 1 if equal (both null), 0 otherwise
                // stloc  <dest>               ; store boolean result
                // ```
                //
                // `ceq` (0xFE 0x01) compares the top two stack values for
                // equality.  When both are null references, the CLR considers
                // them equal: `ceq(ldloc x, ldnull)` ≡ `x == null`.
                //
                // This is the standard C# pattern for `obj == null`:
                // ```csharp
                // // C#: obj == null
                // // IL: ldarg.0; ldnull; ceq
                // ```
                "is_null" => {
                    let dest_name = instr.dest.as_deref().ok_or_else(|| {
                        IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "is_null instruction must have a dest".into(),
                        }
                    })?;
                    let dest = reg_info!(dest_name).clone();
                    let x = get_operand_reg(&instr.srcs, 0, &reg_map, fn_name)?;

                    emit_load(&mut builder, &x, fn_name)?;  // push value
                    builder.emit_ldnull();                   // push null
                    builder.emit_ceq();                      // 1 if equal
                    emit_store(&mut builder, &dest, fn_name)?;
                }

                // ── global_load → UnsupportedOp (LANG32b) ───────────────────
                //
                // Full CLR static-field globals require emitting `ldsfld` with a
                // proper FieldDef or FieldRef metadata token, which in turn
                // requires extending `CILProgramArtifact` with a fields table.
                // That work is scoped to LANG32b.  Return a descriptive error
                // so the pipeline surfaces a clear message rather than a silent
                // wrong-code failure.
                "global_load" => {
                    return Err(IIRClrError::UnsupportedOp {
                        function: fn_name.clone(),
                        op: "global_load: CLR static-field globals not yet implemented — LANG32b".to_string(),
                    });
                }

                // ── global_store → UnsupportedOp (LANG32b) ──────────────────
                "global_store" => {
                    return Err(IIRClrError::UnsupportedOp {
                        function: fn_name.clone(),
                        op: "global_store: CLR static-field globals not yet implemented — LANG32b".to_string(),
                    });
                }

                // ── io_out → Console.WriteLine(int64) ───────────────────────
                //
                // `io_out %val` prints an i64 value.  CIL steps:
                //   1. Load the variable onto the stack (`ldloc`/`ldarg`).
                //   2. `call void [mscorlib]System.Console::WriteLine(int64)`
                //
                // The `call` opcode takes a 4-byte metadata token.  We use the
                // CONSOLE_WRITELINE_I64_TOKEN sentinel (MemberRef table row 2),
                // which a CLR simulator or PE packager resolves at runtime.
                "io_out" => {
                    let val_src = match instr.srcs.first() {
                        Some(Operand::Var(v)) => v.clone(),
                        _ => return Err(IIRClrError::InvalidOperand {
                            function: fn_name.clone(),
                            detail: "io_out requires a Var operand".to_string(),
                        }),
                    };
                    let val_info = reg_info!(val_src).clone();
                    // Push the value onto the CIL evaluation stack.
                    emit_load(&mut builder, &val_info, fn_name)?;
                    // call void [mscorlib]System.Console::WriteLine(int64)
                    builder.emit_call(CONSOLE_WRITELINE_I64_TOKEN);
                }

                // ── Unsupported ops ──────────────────────────────────────────
                //
                // Caught by the validator, but we guard here for defence-in-
                // depth: if validation was skipped or a new opcode was added
                // without updating the validator, we return a clear error
                // rather than silently producing wrong code.
                other => {
                    return Err(IIRClrError::UnsupportedOp {
                        function: fn_name.clone(),
                        op: other.to_string(),
                    });
                }
            }
        }

        // ── Assemble the builder into a Vec<u8> ────────────────────────────
        //
        // `assemble()` runs the two-pass branch-promotion algorithm:
        // 1. Start all branches as short (2-byte).
        // 2. Measure offsets; promote any short branch whose target is >127
        //    bytes away to long (5-byte).
        // 3. Repeat until stable.
        // 4. Encode all items.

        let body = builder.assemble().map_err(|e| IIRClrError::AssemblyError {
            function: fn_name.clone(),
            detail: e.0.clone(),
        })?;

        // Build the method artifact.
        // - `max_stack`: 16 is conservative enough for all synthesized
        //   sequences in this backend.  The CLR's verifier checks this at
        //   load time; an overshoot wastes a few bytes in the header but
        //   never causes a failure.
        // - `local_types`: one "int32" entry per local variable slot.
        //   NOTE (Phase 2): in a full CLR PE/COFF packager, `ref<LispyPair>`
        //   locals would be declared as `object` (System.Object).  We leave
        //   them as "int32" here because `CILMethodArtifact` uses this field
        //   for human-readable annotations only — the actual CIL bytecode
        //   carries all the type information the JIT needs.
        // - `return_type`: "int32" for all methods in v1.
        // - `parameter_types`: one "int32" per parameter.

        let local_types: Vec<String> = (0..local_count).map(|_| "int32".to_string()).collect();
        let parameter_types: Vec<String> =
            (0..param_count).map(|_| "int32".to_string()).collect();

        methods.push(CILMethodArtifact {
            name: func.name.clone(),
            body,
            max_stack: 16,
            local_types,
            return_type: "int32",
            parameter_types,
        });
    }

    // ── Construct CILProgramArtifact ───────────────────────────────────────
    //
    // `entry_label` is the first function in the module (or the declared
    // entry point if set).  `data_offsets` and `data_size` are zero because
    // IIR has no static data section.  `helper_specs` is empty because we
    // don't inject runtime helpers in this backend (no syscalls, no memory).
    //
    // The `token_provider` field is required by the type but unused by this
    // backend — we provide a sequential provider built from the method names.

    let entry_label = module.entry_point
        .clone()
        .unwrap_or_else(|| {
            module.functions.first().map(|f| f.name.clone()).unwrap_or_default()
        });

    let callable_labels: Vec<&str> = module.functions.iter()
        .map(|f| f.name.as_str())
        .collect();

    let token_provider = Box::new(
        ir_to_cil_bytecode::SequentialCILTokenProvider::new(&callable_labels)
    );

    Ok(CILProgramArtifact {
        entry_label,
        methods,
        data_offsets: std::collections::HashMap::new(),
        data_size: 0,
        helper_specs: vec![],
        token_provider,
    })
}

// ===========================================================================
// Helper: extract a Var operand as a RegInfo
// ===========================================================================

/// Extract the operand at position `idx` from `srcs` as a `RegInfo`.
///
/// Returns `UndefinedVariable` if the name is not in the map,
/// `InvalidOperand` if the operand is not a `Var`.
fn get_operand_reg(
    srcs: &[Operand],
    idx: usize,
    reg_map: &HashMap<String, RegInfo>,
    fn_name: &str,
) -> Result<RegInfo, IIRClrError> {
    match srcs.get(idx) {
        Some(Operand::Var(name)) => {
            reg_map.get(name.as_str())
                .cloned()
                .ok_or_else(|| IIRClrError::UndefinedVariable {
                    function: fn_name.to_string(),
                    name: name.clone(),
                })
        }
        Some(other) => Err(IIRClrError::InvalidOperand {
            function: fn_name.to_string(),
            detail: format!("expected Var operand at index {idx}, got {:?}", other),
        }),
        None => Err(IIRClrError::InvalidOperand {
            function: fn_name.to_string(),
            detail: format!("missing operand at index {idx}"),
        }),
    }
}

// ===========================================================================
// Unit tests (in-module)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};

    fn single_fn(instrs: Vec<IIRInstr>) -> IIRModule {
        let fn_ = IIRFunction::new("main", vec![], "void", instrs);
        IIRModule {
            name: "test".into(),
            functions: vec![fn_],
            entry_point: Some("main".into()),
            language: "test".into(),
        }
    }

    fn default_cfg() -> IIRClrConfig {
        IIRClrConfig::default()
    }

    #[test]
    fn validation_failure_propagated() {
        let module = IIRModule {
            name: "empty".into(),
            functions: vec![],
            entry_point: None,
            language: "test".into(),
        };
        let result = lower_iir_to_cil(&module, &default_cfg());
        assert!(matches!(result, Err(IIRClrError::ValidationFailed(_))));
    }

    #[test]
    fn ret_void_produces_non_empty_body() {
        let module = single_fn(vec![
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]);
        let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
        assert!(!artifact.methods[0].body.is_empty());
        // ret = 0x2A
        assert!(artifact.methods[0].body.contains(&0x2A));
    }

    #[test]
    fn const_int_emits_ldc_and_stloc() {
        let module = single_fn(vec![
            IIRInstr::new("const", Some("v0".into()),
                vec![Operand::Int(42)], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
        ]);
        let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
        let body = &artifact.methods[0].body;
        // ldc.i4.s 42 = [0x1F, 42]
        assert!(body.windows(2).any(|w| w == [0x1F, 42]),
            "expected ldc.i4.s 42 in body: {body:?}");
    }

    #[test]
    fn add_emits_add_opcode() {
        let module = single_fn(vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(3)], "i32"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(4)], "i32"),
            IIRInstr::new("add", Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i32"),
        ]);
        let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
        assert!(artifact.methods[0].body.contains(&0x58), "add = 0x58");
    }

    #[test]
    fn cmp_ne_has_two_ceq_sequences() {
        // cmp_ne synthesizes: ceq; ldc.i4.0; ceq
        let module = single_fn(vec![
            IIRInstr::new("const", Some("a".into()), vec![Operand::Int(1)], "i32"),
            IIRInstr::new("const", Some("b".into()), vec![Operand::Int(2)], "i32"),
            IIRInstr::new("cmp_ne", Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "bool"),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "bool"),
        ]);
        let artifact = lower_iir_to_cil(&module, &default_cfg()).unwrap();
        let body = &artifact.methods[0].body;
        let ceq_count = body.windows(2).filter(|w| *w == [0xFE, 0x01]).count();
        assert_eq!(ceq_count, 2, "cmp_ne must produce exactly two ceq sequences");
    }
}
