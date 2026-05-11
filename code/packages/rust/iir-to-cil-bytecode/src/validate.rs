//! Pre-flight validation for IIR → CLR CIL lowering.
//!
//! # Why validate separately?
//!
//! The CLR is a typed, managed runtime.  Not every IIR program can be lowered
//! to CIL without constraints:
//!
//! - Float immediates (f32, f64) require different CIL instructions (`ldc.r4`,
//!   `ldc.r8`) that this v1 backend does not emit — we reject them early.
//! - The `"any"` / `"polymorphic"` type hints mean the frontend did not resolve
//!   types; CLR CIL relies on knowing stack element widths.
//! - Some IIR opcodes have no CIL equivalent in this lowering.
//!
//! Catching these problems *before* lowering gives clear, actionable errors
//! instead of a panic deep inside the code-generation pass.
//!
//! # Checks performed
//!
//! | Error kind              | Condition |
//! |-------------------------|-----------|
//! | `EmptyModule`           | Module has zero functions |
//! | `EmptyFunction`         | A function has zero instructions |
//! | `UntypedInstruction`    | `type_hint` is `"any"` or `"polymorphic"` |
//! | `UnsupportedType`       | `type_hint` is `"str"` or starts with `"ref<"` |
//! | `UnsupportedType` (float const) | `op == "const"` and src is `Operand::Float` |
//! | `UnsupportedOp`         | op is a runtime/memory/IO/GC opcode (list below) |
//!
//! Unsupported ops: `call_builtin`, `io_in`, `io_out`, `cast`, `load_mem`,
//! `store_mem`, `alloc`, `box`, `unbox`, `field_load`, `field_store`,
//! `is_null`, `safepoint`.

use interpreter_ir::{IIRModule, Operand};

// ---------------------------------------------------------------------------
// Opcodes not supported by this CLR backend
// ---------------------------------------------------------------------------
//
// These opcodes all have runtime / OS / memory semantics that cannot be
// expressed as pure CIL integer arithmetic:
//
// - `call_builtin`  — host built-in; this lowering has no host bridge.
// - `io_in/io_out`  — raw I/O; CIL does this via System.Console, not opcodes.
// - `cast`          — type reinterpretation; not needed for typed IIR.
// - `load_mem/store_mem` — raw pointer access; CIL has unsafe but we don't
//                    lower it here.
// - `alloc/box/unbox/field_load/field_store/is_null` — GC heap ops; the CLR
//   manages its own heap; these are lowered separately.
// - `safepoint`     — GC coordination; handled by the CLR runtime.

const UNSUPPORTED_OPS: &[&str] = &[
    "call_builtin",
    "io_in",
    "io_out",
    "cast",
    "load_mem",
    "store_mem",
    "alloc",
    "box",
    "unbox",
    "field_load",
    "field_store",
    "is_null",
    "safepoint",
];

// ---------------------------------------------------------------------------
// validate_iir_for_clr
// ---------------------------------------------------------------------------

/// Validate an `IIRModule` for CLR CIL lowering.
///
/// Returns a `Vec<String>` of human-readable error messages.
/// An empty vector means the module is safe to pass to
/// [`crate::lower::lower_iir_to_cil`].
///
/// # Checks
///
/// 1. **EmptyModule** — At least one function must exist; a module with no
///    code section entries cannot be loaded meaningfully by the CLR.
///
/// 2. **EmptyFunction** — Each function must have at least one instruction.
///    An empty body is almost certainly a front-end bug.
///
/// 3. **UntypedInstruction** — `type_hint` must not be `"any"` or
///    `"polymorphic"`.  CIL's stack-based evaluation requires knowing operand
///    widths at emit time.  We require the frontend to have resolved types
///    before lowering.
///
/// 4. **UnsupportedType** — `type_hint` must not be `"str"` (no string
///    arithmetic in v1) or start with `"ref<"` (heap-pointer types have no
///    CIL equivalent in this lowering).
///
/// 5. **UnsupportedType for float const** — `op == "const"` with an
///    `Operand::Float` source is rejected.  CIL does support floats, but
///    loading them requires `ldc.r4`/`ldc.r8` instructions, which this v1
///    backend does not emit.  Rejecting float constants here gives a clear
///    error rather than silently truncating the value to an integer.
///
/// 6. **UnsupportedOp** — see [`UNSUPPORTED_OPS`].
///
/// # Example
///
/// ```
/// use interpreter_ir::{IIRModule, IIRFunction, IIRInstr};
/// use iir_to_cil_bytecode::validate_iir_for_clr;
///
/// let fn_ = IIRFunction::new("main", vec![], "void",
///     vec![IIRInstr::new("ret_void", None, vec![], "void")]);
/// let module = IIRModule {
///     name: "test".into(),
///     functions: vec![fn_],
///     entry_point: Some("main".into()),
///     language: "test".into(),
/// };
/// assert!(validate_iir_for_clr(&module).is_empty());
/// ```
pub fn validate_iir_for_clr(module: &IIRModule) -> Vec<String> {
    let mut errors = Vec::new();

    // ── Check 1: EmptyModule ─────────────────────────────────────────────────
    //
    // A CLR assembly with no methods has no entry point and cannot be loaded.
    // Catching this early avoids an empty `CILProgramArtifact::methods` vector.
    if module.functions.is_empty() {
        errors.push("EmptyModule: module has no functions".to_string());
        // Return early — per-function checks below would be vacuous.
        return errors;
    }

    for func in &module.functions {
        // ── Check 2: EmptyFunction ───────────────────────────────────────────
        //
        // An empty CIL method body would produce an invalid method: the CLR
        // requires every method to end with a `ret` (or `throw`).
        if func.instructions.is_empty() {
            errors.push(format!(
                "EmptyFunction: function {:?} has no instructions",
                func.name
            ));
            continue; // no point scanning the (empty) instruction list
        }

        for instr in &func.instructions {
            // ── Check 3: UntypedInstruction ──────────────────────────────────
            //
            // CIL is a typed stack machine.  The JIT verifier needs to know the
            // type of every stack slot.  An `"any"` type hint means the frontend
            // hasn't resolved the type — we cannot safely emit CIL without it.
            //
            // `"polymorphic"` is the profiler's sentinel for "seen multiple
            // types at runtime" — meaningless for static CIL lowering.
            if instr.type_hint == "any" || instr.type_hint == "polymorphic" {
                errors.push(format!(
                    "UntypedInstruction: function {:?}, op {:?} has type_hint {:?}; \
                     CLR CIL lowering requires concrete types",
                    func.name, instr.op, instr.type_hint
                ));
            }

            // ── Check 4: UnsupportedType ─────────────────────────────────────
            //
            // `"str"` — String operations require System.String method calls;
            // we do not emit them in v1.
            //
            // `"ref<…>"` — Heap pointer types require GC-managed references;
            // we do not map IIR heap ops to CLR object model here.
            if instr.type_hint == "str" {
                errors.push(format!(
                    "UnsupportedType: function {:?}, op {:?} has type_hint \"str\"; \
                     string operations are not supported in this CLR backend",
                    func.name, instr.op
                ));
            } else if instr.type_hint.starts_with("ref<") {
                errors.push(format!(
                    "UnsupportedType: function {:?}, op {:?} has reference type {:?}; \
                     heap pointer types are not supported in this CLR backend",
                    func.name, instr.op, instr.type_hint
                ));
            }

            // ── Check 5: float const ─────────────────────────────────────────
            //
            // CIL does support floating-point, but loading a float immediate
            // requires `ldc.r4` (4-byte float) or `ldc.r8` (8-byte double),
            // which this v1 lowering does not emit.  Rejecting float constants
            // here gives a clear error rather than silently truncating the
            // value to an integer (which would be a silent semantic bug).
            if instr.op == "const" {
                if let Some(Operand::Float(_)) = instr.srcs.first() {
                    errors.push(format!(
                        "UnsupportedType: function {:?}, const instruction has a Float \
                         operand; float constants are not supported in CLR v1 \
                         (use integer arithmetic or a separate fp lowering pass)",
                        func.name
                    ));
                }
            }

            // ── Check 6: UnsupportedOp ───────────────────────────────────────
            //
            // The CLR backend implements a focused subset of IIR.
            // Runtime, I/O, heap, and FFI operations have no direct
            // CIL-opcode equivalent here.
            if UNSUPPORTED_OPS.contains(&instr.op.as_str()) {
                errors.push(format!(
                    "UnsupportedOp: function {:?}, op {:?} is not supported by \
                     the CLR backend; it requires a P/Invoke or .NET BCL call",
                    func.name, instr.op
                ));
            }
        }
    }

    errors
}

// ---------------------------------------------------------------------------
// Unit tests (in-module)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};

    fn single_fn_module(instrs: Vec<IIRInstr>) -> IIRModule {
        let fn_ = IIRFunction::new("main", vec![], "void", instrs);
        IIRModule {
            name: "test".into(),
            functions: vec![fn_],
            entry_point: Some("main".into()),
            language: "test".into(),
        }
    }

    #[test]
    fn empty_module_rejected() {
        let module = IIRModule {
            name: "empty".into(),
            functions: vec![],
            entry_point: None,
            language: "test".into(),
        };
        let errs = validate_iir_for_clr(&module);
        assert!(!errs.is_empty(), "should reject empty module");
        assert!(errs[0].contains("EmptyModule"));
    }

    #[test]
    fn empty_function_rejected() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![]));
        assert!(!errs.is_empty());
        assert!(errs[0].contains("EmptyFunction"));
    }

    #[test]
    fn any_type_rejected() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("add", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
        ]));
        assert!(errs.iter().any(|e| e.contains("UntypedInstruction")));
    }

    #[test]
    fn polymorphic_type_rejected() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("add", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "polymorphic"),
        ]));
        assert!(errs.iter().any(|e| e.contains("UntypedInstruction")));
    }

    #[test]
    fn float_const_rejected() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Float(3.14)], "f64"),
        ]));
        assert!(errs.iter().any(|e| e.contains("Float")));
    }

    #[test]
    fn str_type_rejected() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("ret_void", None, vec![], "str"),
        ]));
        assert!(errs.iter().any(|e| e.contains("UnsupportedType")));
    }

    #[test]
    fn ref_type_rejected() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("ret_void", None, vec![], "ref<u8>"),
        ]));
        assert!(errs.iter().any(|e| e.contains("UnsupportedType")));
    }

    #[test]
    fn unsupported_op_rejected() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("io_in", Some("v".into()), vec![], "i32"),
        ]));
        assert!(errs.iter().any(|e| e.contains("UnsupportedOp")));
    }

    #[test]
    fn valid_module_no_errors() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(errs.is_empty(), "unexpected errors: {:?}", errs);
    }

    #[test]
    fn valid_typed_arithmetic_no_errors() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("const", Some("v0".into()),
                vec![Operand::Int(42)], "i32"),
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
        ]));
        assert!(errs.is_empty(), "unexpected errors: {:?}", errs);
    }
}
