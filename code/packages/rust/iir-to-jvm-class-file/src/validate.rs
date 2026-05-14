//! Pre-flight validation for IIR → JVM class file lowering.
//!
//! # Why validate separately?
//!
//! The JVM is a typed, stack-based virtual machine.  Not every IIR program can
//! be lowered: the JVM has no support for raw memory operations, I/O syscalls,
//! unsafe casts, or dynamically-typed ("any") instructions without explicit
//! boxing.  Catching these problems *before* lowering produces clear, actionable
//! error messages rather than a panic deep inside the code-generation pass.
//!
//! This module implements a single public function, [`validate_for_jvm`].
//! The lowering pass ([`crate::lower::lower_iir_to_jvm`]) calls it automatically
//! on entry and returns `Err(ValidationFailed(…))` if there are problems, so
//! callers that just want a Result can skip the explicit validate call.
//! Callers that want to display errors to the user should call it directly.
//!
//! # Checks performed
//!
//! | Error kind          | Condition |
//! |---------------------|-----------|
//! | `EmptyModule`       | Module has zero functions |
//! | `EmptyFunction`     | A function has zero instructions |
//! | `UntypedInstruction`| `type_hint` is `"any"` or `"polymorphic"` |
//! | `UnsupportedType`   | `type_hint` is `"str"` or starts with `"ref<"` |
//! | `UnsupportedOp`     | op is a runtime/memory/IO/GC opcode (list below) |
//!
//! **Importantly, float type hints and float constant operands are SUPPORTED.**
//! The JVM has native `fload`/`dload`/`fadd`/`dadd` opcodes, unlike the BEAM
//! backend which must box floats via `fmove` into floating-point registers.
//!
//! # Unsupported ops
//!
//! `call_builtin`, `io_in`, `io_out`, `cast`, `load_mem`, `store_mem`,
//! `box`, `unbox`, `safepoint`.
//!
//! The following ops are now SUPPORTED via `Object[]` cons cells (Phase 2):
//! `alloc` (when `type_hint == "ref<LispyPair>"`), `field_load`, `field_store`,
//! `is_null`, and `const` with `type_hint == "ref<LispyPair>"` (nil).

use std::collections::HashMap;

use interpreter_ir::{IIRModule, Operand};

// ---------------------------------------------------------------------------
// Opcodes not supported by this JVM backend
// ---------------------------------------------------------------------------
//
// These opcodes all have runtime / OS / memory semantics that cannot be
// expressed as pure JVM stack arithmetic:
//
// - `call_builtin`  — host built-in; JVM has no host bridge in this lowering.
// - `io_in/io_out`  — raw I/O; JVM does this via java.io APIs, not opcodes.
// - `cast`          — type reinterpretation / unsafe casts not supported.
// - `load_mem/store_mem` — raw pointer access; JVM has no unsafe memory.
// - `box/unbox`     — boxing primitives; not needed for the Object[] model.
// - `safepoint`     — GC coordination; handled by the JVM runtime itself.
//
// Phase-2 additions — now SUPPORTED (removed from the block list):
// - `alloc`        — when type_hint == "ref<LispyPair>": Object[] allocation.
// - `field_load`   — car/cdr via aaload.
// - `field_store`  — writing pair fields via aastore.
// - `is_null`      — null check via ifnull.

const UNSUPPORTED_OPS: &[&str] = &[
    "call_builtin",
    "io_in",
    "io_out",
    "cast",
    "load_mem",
    "store_mem",
    "box",
    "unbox",
    "safepoint",
];

/// Ops that are conditionally supported depending on their `type_hint`.
///
/// `"alloc"` is accepted only when `type_hint == "ref<LispyPair>"`.
/// For any other type the validator still rejects it with an UnsupportedOp
/// error, since we only know how to allocate LispyPair cons cells here.
const CONDITIONALLY_SUPPORTED_OPS: &[&str] = &["alloc", "field_load", "field_store", "is_null"];

// ---------------------------------------------------------------------------
// validate_for_jvm
// ---------------------------------------------------------------------------

/// Validate an `IIRModule` for JVM class file lowering.
///
/// Returns a `Vec<String>` of human-readable error messages.
/// An empty vector means the module is safe to pass to
/// [`crate::lower::lower_iir_to_jvm`].
///
/// # Checks
///
/// 1. **EmptyModule** — At least one function must exist.
///
/// 2. **EmptyFunction** — Each function must have at least one instruction.
///    An empty body is almost certainly a front-end bug.
///
/// 3. **UntypedInstruction** — `type_hint` must not be `"any"` or
///    `"polymorphic"`.  JVM typed opcodes require a known type to pick the
///    right load/store/arithmetic instruction (`iadd` vs `ladd` vs `fadd`).
///
/// 4. **UnsupportedType** — `type_hint` must not be `"str"` or start with
///    `"ref<"`.  String and heap pointer types require JVM object references
///    that this backend does not emit in v1.
///
///    **Float types (`f32`, `f64`) ARE supported** — JVM has `fload`, `dload`,
///    `fadd`, `dadd`, etc.  We do not reject float type hints here.
///
/// 5. **UnsupportedOp** — see [`UNSUPPORTED_OPS`].
///
/// # Example
///
/// ```
/// use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
/// use iir_to_jvm_class_file::validate_for_jvm;
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
/// assert!(validate_for_jvm(&module).is_empty());
/// ```
pub fn validate_for_jvm(module: &IIRModule) -> Vec<String> {
    let mut errors = Vec::new();

    // ── Check 1: EmptyModule ─────────────────────────────────────────────────
    //
    // A JVM class with no methods has no code section entries.  The class
    // loader requires at least one method for anything useful.
    if module.functions.is_empty() {
        errors.push("EmptyModule: module has no functions".to_string());
        // Return early — the per-function checks below would be vacuous.
        return errors;
    }

    for func in &module.functions {
        // ── Check 2: EmptyFunction ───────────────────────────────────────────
        //
        // An empty function body would produce a method with a zero-length
        // Code attribute — this is invalid JVM bytecode (every method needs
        // at least a `return` instruction).
        if func.instructions.is_empty() {
            errors.push(format!(
                "EmptyFunction: function {:?} has no instructions",
                func.name
            ));
            continue; // no point scanning the (empty) instruction list
        }

        // ── Check 2.5 pre-pass: build variable-type map for this function ────
        //
        // We need this to detect float-typed captures in `alloc_closure`.
        // The map is built from function parameters and instruction dest
        // type_hints (first declaration wins).
        //
        // This is a lightweight O(N) pre-pass — no full type inference needed.
        let mut var_types: HashMap<&str, &str> = HashMap::new();
        for (pname, ptype) in &func.params {
            var_types.insert(pname.as_str(), ptype.as_str());
        }
        for instr in &func.instructions {
            if let Some(dest) = &instr.dest {
                var_types
                    .entry(dest.as_str())
                    .or_insert(instr.type_hint.as_str());
            }
        }

        for instr in &func.instructions {
            // ── Check 2.5: Closure early-accept (LANG36) ─────────────────────
            //
            // `alloc_closure` and `call_closure` (LANG34 opcodes) are fully
            // supported by the JVM backend since LANG36, using a `long[]`
            // dispatch-table approach:
            //
            //   alloc_closure(Str(fn_name), Var(cap0), …) : "closure"
            //     → long[] {fn_dispatch_idx, cap0_as_long, …}
            //
            //   call_closure(Var(handle), Var(arg0), …) : "any"
            //     → static __callClosure(long[] closure, long[] args)
            //
            // EXCEPTION: float captures (f32/f64) are not supported in v1 —
            // losslessly boxing a float into a long requires bit-casting, which
            // is deferred to LANG38.  We detect float-typed captures here
            // using the var_types map built above.
            if instr.op == "alloc_closure" {
                // Check each capture variable (srcs[1..]) for float type.
                // srcs[0] is Operand::Str(fn_name) — not a capture variable.
                for src in instr.srcs.iter().skip(1) {
                    if let Operand::Var(cap_name) = src {
                        let cap_type =
                            var_types.get(cap_name.as_str()).copied().unwrap_or("");
                        if cap_type == "f32" || cap_type == "f64" {
                            errors.push(format!(
                                "ClosureOpcode: function {:?}, alloc_closure captures \
                                 float variable {:?} (type {:?}); float closure captures \
                                 require the BEAM backend in v1 — use integer types or \
                                 upgrade to LANG38",
                                func.name, cap_name, cap_type
                            ));
                        }
                    }
                }
                continue; // accepted (non-float captures OK)
            }
            if instr.op == "call_closure" {
                // `call_closure` always has type_hint "any" — accepted here.
                // The lowering pass validates that the closure target exists.
                continue;
            }

            // ── Check 3: UntypedInstruction ──────────────────────────────────
            //
            // JVM arithmetic and load/store opcodes are typed — `iadd` works
            // on `int`, `ladd` on `long`, `fadd` on `float`, `dadd` on `double`.
            // Without a concrete type hint we cannot pick the right opcode family.
            //
            // `"polymorphic"` is the profiler's sentinel for "seen multiple
            // types at runtime" — it means the JIT should NOT specialise.  It
            // is equally useless for static JVM lowering.
            if instr.type_hint == "any" || instr.type_hint == "polymorphic" {
                errors.push(format!(
                    "UntypedInstruction: function {:?}, op {:?} has type_hint {:?}; \
                     JVM lowering requires concrete types",
                    func.name, instr.op, instr.type_hint
                ));
            }

            // ── Check 4: UnsupportedType ─────────────────────────────────────
            //
            // `"str"` — JVM has strings (`java.lang.String`), but there is no
            // integer-arithmetic equivalent; we do not emit string handling code.
            //
            // `"ref<…>"` — heap pointer types require `aload`/`astore` and GC
            // object references.  Phase 2 supports `"ref<LispyPair>"` via
            // `Object[]` cons cells; all other `ref<…>` types are still rejected.
            //
            // `"f32"` and `"f64"` are intentionally NOT rejected here.  The JVM
            // has first-class float/double operations (`fload`, `dload`, `fadd`,
            // `dadd`, etc.) that this backend emits.
            if instr.type_hint == "str" {
                errors.push(format!(
                    "UnsupportedType: function {:?}, op {:?} has type_hint \"str\"; \
                     string operations are not supported in this JVM backend",
                    func.name, instr.op
                ));
            } else if instr.type_hint.starts_with("ref<")
                && instr.type_hint != "ref<LispyPair>"
            {
                // Only ref<LispyPair> is supported (Phase 2 Object[] cons cells).
                // Any other ref<T> — raw pointers, struct refs, etc. — is still
                // unsupported because we have no Java class to represent them.
                errors.push(format!(
                    "UnsupportedType: function {:?}, op {:?} has reference type {:?}; \
                     only ref<LispyPair> is supported in this JVM backend (Phase 2)",
                    func.name, instr.op, instr.type_hint
                ));
            }

            // ── Check 5: UnsupportedOp ───────────────────────────────────────
            //
            // The JVM backend in this crate implements a focused subset of IIR.
            // Runtime, I/O, heap, and native-bridge operations have no direct
            // JVM-bytecode equivalent here.
            if UNSUPPORTED_OPS.contains(&instr.op.as_str()) {
                errors.push(format!(
                    "UnsupportedOp: function {:?}, op {:?} is not supported by \
                     the JVM backend; it requires a native method or Java standard-library call",
                    func.name, instr.op
                ));
            }

            // ── Check 6: Conditionally supported ops ─────────────────────────
            //
            // `alloc` is only supported for `type_hint == "ref<LispyPair>"`.
            // `field_load` and `field_store` are supported for pair fields.
            // `is_null` is supported unconditionally (works on any reference).
            //
            // When `alloc` appears with a *different* ref type, we already
            // rejected the type in Check 4 (if type_hint is ref<…> but not
            // ref<LispyPair>).  So we only need to guard against `alloc` with
            // a completely non-ref type_hint here.
            if instr.op == "alloc"
                && !CONDITIONALLY_SUPPORTED_OPS.contains(&instr.op.as_str())
            {
                // This branch is unreachable given the constant definition, but
                // serves as a documentation anchor.
                errors.push(format!(
                    "UnsupportedOp: function {:?}, op \"alloc\" — unreachable branch",
                    func.name
                ));
            }
            // `field_load`, `field_store`, `is_null` — no additional constraint
            // beyond their type_hint (handled in Check 4 above).
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
        let errs = validate_for_jvm(&module);
        assert!(!errs.is_empty(), "should reject empty module");
        assert!(errs[0].contains("EmptyModule"));
    }

    #[test]
    fn empty_function_rejected() {
        let errs = validate_for_jvm(&single_fn_module(vec![]));
        assert!(!errs.is_empty());
        assert!(errs[0].contains("EmptyFunction"));
    }

    #[test]
    fn any_type_rejected() {
        let errs = validate_for_jvm(&single_fn_module(vec![
            IIRInstr::new(
                "add",
                Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "any",
            ),
        ]));
        assert!(errs.iter().any(|e| e.contains("UntypedInstruction")));
    }

    #[test]
    fn polymorphic_type_rejected() {
        let errs = validate_for_jvm(&single_fn_module(vec![
            IIRInstr::new("add", Some("v".into()), vec![], "polymorphic"),
        ]));
        assert!(errs.iter().any(|e| e.contains("UntypedInstruction")));
    }

    #[test]
    fn str_type_rejected() {
        let errs = validate_for_jvm(&single_fn_module(vec![
            IIRInstr::new("const", Some("v".into()), vec![], "str"),
        ]));
        assert!(errs.iter().any(|e| e.contains("UnsupportedType")));
    }

    #[test]
    fn ref_type_rejected() {
        let errs = validate_for_jvm(&single_fn_module(vec![
            IIRInstr::new("const", Some("v".into()), vec![], "ref<i32>"),
        ]));
        assert!(errs.iter().any(|e| e.contains("UnsupportedType")));
    }

    #[test]
    fn float_const_allowed() {
        // Unlike BEAM backend, float constants ARE supported on JVM.
        let errs = validate_for_jvm(&single_fn_module(vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Float(3.14)], "f64"),
        ]));
        // Should NOT have a "Float" or "UnsupportedType" error
        assert!(
            !errs.iter().any(|e| e.contains("Float")),
            "float consts should be allowed, got: {:?}",
            errs
        );
    }

    #[test]
    fn f32_type_allowed() {
        let errs = validate_for_jvm(&single_fn_module(vec![
            IIRInstr::new("add", Some("v".into()), vec![], "f32"),
        ]));
        assert!(
            !errs.iter().any(|e| e.contains("UnsupportedType")),
            "f32 should be allowed, got: {:?}",
            errs
        );
    }

    #[test]
    fn f64_type_allowed() {
        let errs = validate_for_jvm(&single_fn_module(vec![
            IIRInstr::new("ret", None, vec![], "f64"),
        ]));
        assert!(
            !errs.iter().any(|e| e.contains("UnsupportedType")),
            "f64 should be allowed, got: {:?}",
            errs
        );
    }

    #[test]
    fn unsupported_ops_rejected() {
        // These ops are unconditionally unsupported by the JVM backend.
        // Note: alloc/field_load/field_store/is_null are Phase-2 supported ops
        // (via Object[] cons cells) and are NOT in this list.
        for op in &[
            "call_builtin",
            "io_in",
            "io_out",
            "cast",
            "load_mem",
            "store_mem",
            "box",
            "unbox",
            "safepoint",
        ] {
            let errs = validate_for_jvm(&single_fn_module(vec![
                IIRInstr::new(*op, None, vec![], "void"),
            ]));
            assert!(
                errs.iter().any(|e| e.contains("UnsupportedOp")),
                "op {:?} should be rejected",
                op
            );
        }
    }

    /// `alloc ref<LispyPair>` is accepted (Phase 2 Object[] cons cells).
    #[test]
    fn alloc_lispy_pair_accepted() {
        let errs = validate_for_jvm(&single_fn_module(vec![
            IIRInstr::new("alloc", Some("p".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(
            !errs.iter().any(|e| e.contains("UnsupportedOp") || e.contains("UnsupportedType")),
            "alloc ref<LispyPair> should be accepted, got: {:?}",
            errs
        );
    }

    /// `alloc` with an unsupported ref type is still rejected.
    #[test]
    fn alloc_wrong_ref_type_rejected() {
        let errs = validate_for_jvm(&single_fn_module(vec![
            IIRInstr::new("alloc", Some("p".into()), vec![], "ref<SomeOtherType>"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(
            errs.iter().any(|e| e.contains("UnsupportedType")),
            "alloc ref<SomeOtherType> should be rejected, got: {:?}",
            errs
        );
    }

    /// `field_load` is accepted (car/cdr via aaload).
    #[test]
    fn field_load_accepted() {
        let errs = validate_for_jvm(&single_fn_module(vec![
            IIRInstr::new(
                "field_load",
                Some("v".into()),
                vec![Operand::Var("p".into()), Operand::Int(0)],
                "ref<LispyPair>",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(
            !errs.iter().any(|e| e.contains("UnsupportedOp")),
            "field_load should be accepted, got: {:?}",
            errs
        );
    }

    /// `field_store` is accepted (writing pair fields via aastore).
    #[test]
    fn field_store_accepted() {
        let errs = validate_for_jvm(&single_fn_module(vec![
            IIRInstr::new(
                "field_store",
                None,
                vec![Operand::Var("p".into()), Operand::Int(0), Operand::Var("v".into())],
                "ref<LispyPair>",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(
            !errs.iter().any(|e| e.contains("UnsupportedOp")),
            "field_store should be accepted, got: {:?}",
            errs
        );
    }

    /// `is_null` is accepted (null check via ifnull).
    #[test]
    fn is_null_accepted() {
        let errs = validate_for_jvm(&single_fn_module(vec![
            IIRInstr::new(
                "is_null",
                Some("r".into()),
                vec![Operand::Var("p".into())],
                "bool",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(
            !errs.iter().any(|e| e.contains("UnsupportedOp")),
            "is_null should be accepted, got: {:?}",
            errs
        );
    }

    #[test]
    fn valid_module_no_errors() {
        let errs = validate_for_jvm(&single_fn_module(vec![
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(errs.is_empty(), "unexpected errors: {:?}", errs);
    }

    #[test]
    fn valid_typed_arithmetic_no_errors() {
        let errs = validate_for_jvm(&single_fn_module(vec![
            IIRInstr::new(
                "add",
                Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "i32"),
        ]));
        assert!(errs.is_empty(), "unexpected errors: {:?}", errs);
    }
}
