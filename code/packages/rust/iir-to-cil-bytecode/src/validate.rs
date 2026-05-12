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
//! # Phase 2: heap ops are now supported
//!
//! The following ops were previously unsupported and have been promoted in
//! Phase 2 by lowering them to `object[]` cons cells managed entirely by the
//! CLR garbage collector:
//!
//! | IIR op           | Condition for acceptance |
//! |------------------|--------------------------|
//! | `alloc`          | `type_hint == "ref<LispyPair>"` only |
//! | `field_load`     | Always (field index 0 = car, 1 = cdr) |
//! | `field_store`    | Always |
//! | `is_null`        | Always |
//! | `const`          | Also `type_hint == "ref<LispyPair>"` (nil literal) |
//!
//! Allocating a `ref<LispyPair>` allocates a 2-element `System.Object[]`:
//! - Index 0 → head (car)
//! - Index 1 → tail (cdr)
//!
//! # Checks performed
//!
//! | Error kind              | Condition |
//! |-------------------------|-----------|
//! | `EmptyModule`           | Module has zero functions |
//! | `EmptyFunction`         | A function has zero instructions |
//! | `UntypedInstruction`    | `type_hint` is `"any"` or `"polymorphic"` |
//! | `UnsupportedType`       | `type_hint` is `"str"` or starts with `"ref<"` but is not `"ref<LispyPair>"` |
//! | `UnsupportedType` (float const) | `op == "const"` and src is `Operand::Float` |
//! | `UnsupportedOp`         | op is a runtime/memory/IO/GC opcode that hasn't been promoted (list below) |
//!
//! Remaining unsupported ops: `call_builtin`, `io_in`, `io_out`, `cast`,
//! `load_mem`, `store_mem`, `box`, `unbox`, `safepoint`.
//! Previously unsupported but now accepted: `alloc` (LispyPair only),
//! `field_load`, `field_store`, `is_null`.

use interpreter_ir::{IIRModule, Operand};

// ---------------------------------------------------------------------------
// Opcodes not supported by this CLR backend
// ---------------------------------------------------------------------------
//
// These opcodes all have runtime / OS / memory semantics that cannot be
// expressed as pure CIL integer arithmetic:
//
// - `call_builtin`  — host built-in; this lowering has no host bridge.
// - `io_in`         — raw byte-level I/O input; CLR does this via System.Console.
// - `cast`          — type reinterpretation; not needed for typed IIR.
// - `load_mem/store_mem` — raw pointer access; CIL has unsafe but we don't
//                    lower it here.
// - `box/unbox`     — value-type boxing; not used for LispyPair cons cells.
// - `safepoint`     — GC coordination; handled by the CLR runtime.
//
// PROMOTED to supported in Phase 2:
// - `alloc`         — accepted when `type_hint == "ref<LispyPair>"`.
//                    Lowered to `newarr System.Object[]`.
// - `field_load`    — accepted for all ref types (car/cdr on index 0/1).
// - `field_store`   — accepted for all ref types (building cons cells).
// - `is_null`       — accepted (ldnull; ceq).
//
// LANG32 — supported in CLR backend (Phase 3):
// - `io_out`        — lowered to `call System.Console.WriteLine(int64)`.
// - `global_store`  — UnsupportedOp in V1 (LANG32b will add static fields).
// - `global_load`   — UnsupportedOp in V1 (LANG32b will add static fields).

const UNSUPPORTED_OPS: &[&str] = &[
    "call_builtin",
    "io_in",
    // "io_out"       — LANG32: now supported (Console.WriteLine).
    // "global_store" — returns UnsupportedOp from lower.rs, not rejected by validator.
    // "global_load"  — returns UnsupportedOp from lower.rs, not rejected by validator.
    "cast",
    "load_mem",
    "store_mem",
    // "alloc"      — promoted in Phase 2 (ref<LispyPair> only)
    "box",
    "unbox",
    // "field_load" — promoted in Phase 2
    // "field_store" — promoted in Phase 2
    // "is_null"    — promoted in Phase 2
    "safepoint",
];

// ---------------------------------------------------------------------------
// Heap ops that need special validation (type-restricted)
// ---------------------------------------------------------------------------
//
// `alloc` is only accepted with `type_hint == "ref<LispyPair>"`.
// Any other allocated type would require a different object layout strategy
// and must be rejected with a clear error message rather than generating
// silently wrong code.
//
// `field_load` and `field_store` are unrestricted — they operate on any
// reference-typed local variable, and the field index selects array slot 0
// (head) or 1 (tail).
//
// `is_null` is unrestricted — it compiles to `ldnull; ceq` for any variable.

/// The one `alloc` type hint we accept in Phase 2.
const LISTY_PAIR_TYPE: &str = "ref<LispyPair>";

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
///     exports: vec![],
///     imports: vec![],
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
            // `"ref<…>"` — Heap pointer types require GC-managed references.
            // In Phase 2 we lower `ref<LispyPair>` to `object[]` cons cells.
            // Any other `ref<…>` type is still unsupported and rejected here.
            //
            // The allowed ops for `ref<LispyPair>` are:
            //   - `alloc`       → newarr System.Object[2]
            //   - `field_load`  → ldelem.ref (car/cdr)
            //   - `field_store` → stelem.ref
            //   - `is_null`     → ldnull; ceq
            //   - `const`       → ldnull (nil literal)
            //   - `ret`         → ret (returning a pair reference)
            //   - `load_reg`    → copy (ldloc/stloc)
            //   - `store_reg`   → copy (ldloc/stloc)
            //   - `jmp_if_true` / `jmp_if_false` — used for pattern-match dispatch
            //
            // All other ops remain rejected for `ref<LispyPair>`.
            if instr.type_hint == "str" {
                errors.push(format!(
                    "UnsupportedType: function {:?}, op {:?} has type_hint \"str\"; \
                     string operations are not supported in this CLR backend",
                    func.name, instr.op
                ));
            } else if instr.type_hint.starts_with("ref<") {
                // Phase 2: `ref<LispyPair>` is supported for specific ops.
                let is_pair = instr.type_hint == LISTY_PAIR_TYPE;
                let is_heap_op = matches!(
                    instr.op.as_str(),
                    "alloc" | "field_load" | "field_store" | "is_null"
                    | "const" | "ret" | "load_reg" | "store_reg"
                    | "jmp_if_true" | "jmp_if_false"
                );
                if !(is_pair && is_heap_op) {
                    errors.push(format!(
                        "UnsupportedType: function {:?}, op {:?} has reference type {:?}; \
                         heap pointer types require ref<LispyPair> and a supported heap op \
                         (alloc, field_load, field_store, is_null, const, ret, load_reg, \
                         store_reg, jmp_if_true, jmp_if_false)",
                        func.name, instr.op, instr.type_hint
                    ));
                }
            }

            // ── Check 4b: alloc with unsupported type hint ────────────────────
            //
            // Even though `alloc` is in the "promoted" list, we only accept it
            // for `ref<LispyPair>`.  Any other `alloc` type still triggers an
            // UnsupportedOp (handled below because it stays in UNSUPPORTED_OPS
            // for non-LispyPair allocs — BUT alloc is removed from UNSUPPORTED_OPS,
            // so we add an explicit check here for unsupported alloc types).
            if instr.op == "alloc" && instr.type_hint != LISTY_PAIR_TYPE {
                errors.push(format!(
                    "UnsupportedType: function {:?}, alloc with type_hint {:?} is not \
                     supported; only ref<LispyPair> cons cells are supported in Phase 2",
                    func.name, instr.type_hint
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
            // Runtime, I/O, and FFI operations have no direct CIL-opcode
            // equivalent in this backend.
            //
            // `field_load`, `field_store`, `is_null` are NOT in UNSUPPORTED_OPS
            // (they were removed in Phase 2); they are handled by the lowerer.
            // `alloc` is also removed — it is accepted for ref<LispyPair> and
            // the type-check above handles unsupported alloc types.
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
            exports: vec![],
            imports: vec![],
        }
    }

    #[test]
    fn empty_module_rejected() {
        let module = IIRModule {
            name: "empty".into(),
            functions: vec![],
            entry_point: None,
            language: "test".into(),
            exports: vec![],
            imports: vec![],
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

    // ── Phase 2 heap-op validation tests ─────────────────────────────────

    #[test]
    fn alloc_listy_pair_is_valid() {
        // Phase 2: alloc ref<LispyPair> is accepted.
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("alloc", Some("p".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(errs.is_empty(), "ref<LispyPair> alloc should pass: {:?}", errs);
    }

    #[test]
    fn alloc_other_ref_type_rejected() {
        // alloc with any type other than ref<LispyPair> must be rejected.
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("alloc", Some("p".into()), vec![], "ref<i32>"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(errs.iter().any(|e| e.contains("UnsupportedType") || e.contains("UnsupportedOp")),
            "alloc ref<i32> must be rejected: {:?}", errs);
    }

    #[test]
    fn field_load_is_valid() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("field_load", Some("h".into()),
                vec![Operand::Var("p".into()), Operand::Int(0)], "ref<LispyPair>"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(errs.is_empty(), "field_load should pass: {:?}", errs);
    }

    #[test]
    fn field_store_is_valid() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("field_store", None,
                vec![Operand::Var("p".into()), Operand::Int(0), Operand::Var("v".into())],
                "ref<LispyPair>"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(errs.is_empty(), "field_store should pass: {:?}", errs);
    }

    #[test]
    fn is_null_is_valid() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("is_null", Some("b".into()),
                vec![Operand::Var("p".into())], "ref<LispyPair>"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(errs.is_empty(), "is_null should pass: {:?}", errs);
    }

    #[test]
    fn const_nil_listy_pair_is_valid() {
        // const with type ref<LispyPair> represents nil.
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("const", Some("nil".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(errs.is_empty(), "const nil ref<LispyPair> should pass: {:?}", errs);
    }
}
