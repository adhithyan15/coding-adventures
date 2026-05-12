//! Pre-flight validation for IIR → BEAM lowering.
//!
//! # Why validate separately?
//!
//! The BEAM virtual machine is a typed, garbage-collected runtime.  Not every
//! IIR program can be lowered: BEAM has no direct support for raw memory
//! operations, I/O syscalls, float immediates (without boxing), or dynamically-
//! typed ("any") instructions.  Catching these problems *before* lowering
//! produces clear, actionable error messages rather than a panic deep inside
//! the code-generation pass.
//!
//! This module implements a single public function, [`validate_for_beam`].
//! The lowering pass ([`crate::lower::lower_iir_to_beam`]) calls it
//! automatically on entry and returns `Err(ValidationFailed(…))` if there are
//! problems, so callers that just want a Result can skip the explicit
//! validate call.  Callers that want to display errors to the user should
//! call it directly.
//!
//! # Checks performed
//!
//! | Error kind | Condition |
//! |------------|-----------|
//! | `EmptyModule` | Module has zero functions |
//! | `EmptyFunction` | A function has zero instructions |
//! | `UntypedInstruction` | `type_hint` is `"any"` or `"polymorphic"` |
//! | `UnsupportedType` | `type_hint` is `"str"` or starts with `"ref<"` — see exceptions |
//! | `UnsupportedType` (float const) | `op == "const"` and src is `Operand::Float` |
//! | `UnsupportedOp` | op is a runtime/memory/IO/GC opcode (list below) |
//!
//! Unsupported ops: `call_builtin`, `io_in`, `io_out`, `cast`, `load_mem`,
//! `store_mem`, `box`, `unbox`, `safepoint`.
//!
//! **Phase 2 heap ops accepted by this backend:**
//!
//! After `iir-builtin-lowering` Phase 2 runs, the following IIR heap opcodes
//! are emitted and must be accepted (not rejected) by this validator:
//!
//! | Op | Accepted when |
//! |----|--------------|
//! | `alloc` | `type_hint == "ref<LispyPair>"` |
//! | `field_load` | any (type_hint is `"ref<any>"` or similar) |
//! | `field_store` | any (type_hint is `"void"`) |
//! | `is_null` | `type_hint == "bool"` |
//!
//! These ops are lowered to BEAM instructions by `lower.rs`:
//! - `alloc` + adjacent `field_store`s → `put_list`
//! - `field_load` → `get_list`
//! - `is_null` → synthesized `is_nil` + boolean synthesis
//!
//! The `ref<LispyPair>` type on `alloc` is also accepted so that `const`
//! instructions representing nil (`const 0 : ref<LispyPair>`) can pass
//! through to the lowering pass which converts them to a BEAM `[]` atom move.

use interpreter_ir::{IIRModule, Operand};

// ---------------------------------------------------------------------------
// Opcodes not supported by this BEAM backend
// ---------------------------------------------------------------------------
//
// These opcodes all have runtime / OS / memory semantics that cannot be
// expressed as pure BEAM integer arithmetic or list operations:
//
// - `call_builtin`  — host built-in; should be fully lowered before reaching
//                     the backend (by iir-builtin-lowering Phases 1–3).
// - `io_in`         — raw byte-level I/O input; BEAM does this via Erlang I/O
//                     modules, not opcodes.
// - `cast`          — type reinterpretation; BEAM is dynamically typed at runtime.
// - `load_mem/store_mem` — raw pointer access; BEAM has no unsafe memory.
// - `box/unbox`     — explicit boxing; BEAM does not expose box/unbox as instructions.
// - `safepoint`     — GC coordination; handled by the BEAM runtime itself.
//
// Note: `alloc`, `field_load`, `field_store`, and `is_null` are intentionally
// NOT in this list.  They are produced by iir-builtin-lowering Phase 2 and
// are lowered to BEAM put_list / get_list / is_nil instructions by lower.rs.
//
// LANG32 — supported in BEAM backend (Phase 3):
// - `io_out`        — lowered to `erlang:display/1` via gc_bif1.
// - `global_store`  — lowered to `erlang:put/2` (process dictionary) via gc_bif2.
// - `global_load`   — lowered to `erlang:get/1` (process dictionary) via gc_bif1.

const UNSUPPORTED_OPS: &[&str] = &[
    "call_builtin",
    "io_in",
    // "io_out"      — LANG32: now supported (erlang:display/1).
    // "global_store"— LANG32: now supported (erlang:put/2).
    // "global_load" — LANG32: now supported (erlang:get/1).
    "cast",
    "load_mem",
    "store_mem",
    // "alloc"       — accepted: lowered to put_list (via alloc+field_store pattern)
    "box",
    "unbox",
    // "field_load"  — accepted: lowered to get_list
    // "field_store" — accepted: lowered as part of the put_list pattern
    // "is_null"     — accepted: lowered to is_nil synthesis
    "safepoint",
];

// ---------------------------------------------------------------------------
// validate_for_beam
// ---------------------------------------------------------------------------

/// Validate an `IIRModule` for BEAM lowering.
///
/// Returns a `Vec<String>` of human-readable error messages.
/// An empty vector means the module is safe to pass to
/// [`crate::lower::lower_iir_to_beam`].
///
/// # Checks
///
/// 1. **EmptyModule** — At least one function must exist; BEAM modules with no
///    code produce no-op `.beam` files that cannot be loaded meaningfully.
///
/// 2. **EmptyFunction** — Each function must have at least one instruction.
///    An empty body is almost certainly a front-end bug.
///
/// 3. **UntypedInstruction** — `type_hint` must not be `"any"` or
///    `"polymorphic"`.  BEAM integer arithmetic (via `gc_bif`) is typed:
///    passing a non-integer to `erlang:+/2` raises a `badarith` exception.
///    We require the frontend to have resolved types before lowering.
///
/// 4. **UnsupportedType** — `type_hint` must not be `"str"` (no string
///    arithmetic in this backend) or start with `"ref<"` (heap pointers have
///    no BEAM equivalent in this lowering).
///
/// 5. **UnsupportedType for float const** — `op == "const"` with an
///    `Operand::Float` source is rejected.  BEAM does support floats, but
///    loading them requires a different instruction path (`fmove`), which this
///    backend does not implement in v1.
///
/// 6. **UnsupportedOp** — see [`UNSUPPORTED_OPS`].
///
/// # Example
///
/// ```
/// use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
/// use iir_to_beam::validate_for_beam;
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
/// assert!(validate_for_beam(&module).is_empty());
/// ```
pub fn validate_for_beam(module: &IIRModule) -> Vec<String> {
    let mut errors = Vec::new();

    // ── Check 1: EmptyModule ─────────────────────────────────────────────────
    //
    // A BEAM module with no functions has no code section entries.  The BEAM
    // loader requires at least one entry in the `ExpT` (export) chunk, which
    // in turn requires at least one function.
    if module.functions.is_empty() {
        errors.push("EmptyModule: module has no functions".to_string());
        // Return early — the per-function checks below would be vacuous.
        return errors;
    }

    for func in &module.functions {
        // ── Check 2: EmptyFunction ───────────────────────────────────────────
        //
        // An empty function body would produce a `func_info` preamble with no
        // code — valid BEAM syntax, but almost certainly a front-end bug.  We
        // reject it to surface the issue early.
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
            // BEAM arithmetic is performed via Erlang BIFs (erlang:+/2, etc.).
            // These BIFs expect integers; if a non-integer is passed, the BEAM
            // VM raises a `badarith` exception at runtime.  Rather than silently
            // produce broken code, we require the frontend to have resolved
            // `"any"` types via type inference or profiling before lowering.
            //
            // `"polymorphic"` is the profiler's sentinel for "seen multiple
            // types at runtime" — it means the JIT should NOT specialise.  It
            // is equally useless for static BEAM lowering.
            if instr.type_hint == "any" || instr.type_hint == "polymorphic" {
                errors.push(format!(
                    "UntypedInstruction: function {:?}, op {:?} has type_hint {:?}; \
                     BEAM lowering requires concrete types",
                    func.name, instr.op, instr.type_hint
                ));
            }

            // ── Check 4: UnsupportedType ─────────────────────────────────────
            //
            // `"str"` — BEAM has strings (binaries/lists), but there is no
            // integer BIF equivalent for string arithmetic; we do not emit
            // string handling code in v1.
            //
            // `"ref<…>"` — heap pointer types require GC-managed terms.
            // EXCEPTION: the Phase 2 heap lowering pass produces instructions
            // with specific reference types that this backend knows how to lower:
            //
            //   - `alloc` with `"ref<LispyPair>"` → put_list (with adjacent field_stores)
            //   - `const` with `"ref<LispyPair>"` → move {a,"[]"} (nil sentinel)
            //   - `field_load` with `"ref<any>"` → get_list
            //   - `field_store` with `"void"` → part of put_list pattern
            //   - `is_null` with `"bool"` → is_nil synthesis
            //
            // Any other ref<…> type on any other op is rejected as before.
            if instr.type_hint == "str" {
                errors.push(format!(
                    "UnsupportedType: function {:?}, op {:?} has type_hint \"str\"; \
                     string operations are not supported in this BEAM backend",
                    func.name, instr.op
                ));
            } else if instr.type_hint.starts_with("ref<") {
                // Allow the specific (op, type_hint) combinations produced by
                // the Phase 2 heap lowering pass.  Everything else is rejected.
                let accepted = match instr.op.as_str() {
                    // alloc ref<LispyPair> — heap-allocate a cons cell.
                    // Lowered to put_list in combination with the two field_stores.
                    "alloc" if instr.type_hint == "ref<LispyPair>" => true,

                    // const ref<LispyPair> with Int(0) — nil sentinel.
                    // Lowered to `move {a,"[]"} {x,rd}`.
                    "const" if instr.type_hint == "ref<LispyPair>" => true,

                    // field_load ref<any> — read car or cdr from a cons cell.
                    // Lowered to get_list.
                    "field_load" => true,

                    // field_store void — write head or tail into a fresh cons cell.
                    // Consumed by the put_list pattern along with its preceding alloc.
                    "field_store" if instr.type_hint == "void" => true,

                    _ => false,
                };

                // Also allow ret and field_load with any ref type — `ret`
                // returns whatever the function produces, and functions may
                // return cons cells (ref<LispyPair>) or any-typed values
                // (ref<any>).  field_load already handled above.
                let accepted = accepted || matches!(instr.op.as_str(), "ret" | "field_load");

                if !accepted {
                    errors.push(format!(
                        "UnsupportedType: function {:?}, op {:?} has reference type {:?}; \
                         heap pointer types are not supported in this BEAM backend \
                         (only ref<LispyPair> on alloc/const, ref<any> on field_load, \
                         and ref<*> on ret are accepted)",
                        func.name, instr.op, instr.type_hint
                    ));
                }
            }

            // ── Check 5: float const ─────────────────────────────────────────
            //
            // BEAM does support floating-point, but the instruction form is
            // `fmove` into a float register, which this lowering does not emit.
            // Rejecting float constants here gives a clear error rather than
            // silently truncating the value to an integer.
            if instr.op == "const" {
                if let Some(Operand::Float(_)) = instr.srcs.first() {
                    errors.push(format!(
                        "UnsupportedType: function {:?}, const instruction has a Float \
                         operand; float constants are not supported (use integer arithmetic \
                         or a separate fp lowering pass)",
                        func.name
                    ));
                }
            }

            // ── Check 6: UnsupportedOp ───────────────────────────────────────
            //
            // The BEAM backend in this crate implements a focused subset of IIR.
            // Runtime, I/O, heap, and NIF-bridge operations have no direct
            // BEAM-opcode equivalent here.
            if UNSUPPORTED_OPS.contains(&instr.op.as_str()) {
                errors.push(format!(
                    "UnsupportedOp: function {:?}, op {:?} is not supported by \
                     the BEAM backend; it requires a NIF or Erlang standard-library call",
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
        let errs = validate_for_beam(&module);
        assert!(!errs.is_empty(), "should reject empty module");
        assert!(errs[0].contains("EmptyModule"));
    }

    #[test]
    fn empty_function_rejected() {
        let errs = validate_for_beam(&single_fn_module(vec![]));
        assert!(!errs.is_empty());
        assert!(errs[0].contains("EmptyFunction"));
    }

    #[test]
    fn any_type_rejected() {
        let errs = validate_for_beam(&single_fn_module(vec![
            IIRInstr::new("add", Some("v".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())], "any"),
        ]));
        assert!(errs.iter().any(|e| e.contains("UntypedInstruction")));
    }

    #[test]
    fn float_const_rejected() {
        let errs = validate_for_beam(&single_fn_module(vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Float(3.14)], "f64"),
        ]));
        assert!(errs.iter().any(|e| e.contains("Float")));
    }

    #[test]
    fn valid_module_no_errors() {
        let errs = validate_for_beam(&single_fn_module(vec![
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(errs.is_empty(), "unexpected errors: {:?}", errs);
    }

    // LANG32: io_out, global_store, global_load are now supported by the
    // BEAM backend and must NOT be rejected by the validator.
    #[test]
    fn io_out_passes_validation() {
        let errs = validate_for_beam(&single_fn_module(vec![IIRInstr::new(
            "io_out",
            None,
            vec![Operand::Var("v".into())],
            "void",
        )]));
        assert!(
            errs.iter().all(|e| !e.contains("UnsupportedOp")),
            "io_out should pass BEAM validation (LANG32); got: {:?}",
            errs
        );
    }

    #[test]
    fn global_store_passes_validation() {
        let errs = validate_for_beam(&single_fn_module(vec![IIRInstr::new(
            "global_store",
            None,
            vec![Operand::Str("x".into()), Operand::Var("v".into())],
            "void",
        )]));
        assert!(
            errs.iter().all(|e| !e.contains("UnsupportedOp")),
            "global_store should pass BEAM validation (LANG32); got: {:?}",
            errs
        );
    }

    #[test]
    fn global_load_passes_validation() {
        let errs = validate_for_beam(&single_fn_module(vec![IIRInstr::new(
            "global_load",
            Some("r".into()),
            vec![Operand::Str("x".into())],
            "i64",
        )]));
        assert!(
            errs.iter().all(|e| !e.contains("UnsupportedOp")),
            "global_load should pass BEAM validation (LANG32); got: {:?}",
            errs
        );
    }
}
