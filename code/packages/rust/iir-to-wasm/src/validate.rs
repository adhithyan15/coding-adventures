//! Pre-flight validation for IIR → WASM lowering.
//!
//! # Why validate separately?
//!
//! WebAssembly 1.0 is a **statically typed, structured** instruction set.  Not
//! every IIR program can be lowered:
//!
//! - WASM has no "any" type — every local and stack slot must have a concrete
//!   numeric type (`i32`, `i64`, `f32`, `f64`).
//! - WASM has no strings or heap-pointer indirection in this lowering.
//! - Runtime / I/O / GC opcodes have no WASM-opcode equivalent without a host
//!   import, which this direct lowering does not provide.
//!
//! Catching these problems *before* lowering gives clear, actionable error
//! messages rather than a panic or a silently malformed binary.
//!
//! # Key difference from the BEAM backend
//!
//! **Float constants ARE allowed here.**  WASM has native `f64.const` and
//! `f32.const` instructions, so `Operand::Float` and type hints `"f32"`/`"f64"`
//! are fully supported.
//!
//! # Checks performed
//!
//! | Error label | Condition |
//! |-------------|-----------|
//! | `EmptyModule` | Module has zero functions |
//! | `EmptyFunction` | A function has zero instructions |
//! | `UntypedInstruction` | `type_hint` is `"any"` or `"polymorphic"` |
//! | `UnsupportedType` | `type_hint` is `"str"` or starts with `"ref<"` |
//! | `UnsupportedOp` | op is any runtime / I/O / GC / NIF opcode |

use interpreter_ir::IIRModule;

// ---------------------------------------------------------------------------
// Unsupported opcode table
// ---------------------------------------------------------------------------
//
// These opcodes all require runtime support that WASM 1.0 cannot express as
// plain numeric instructions:
//
// - `call_builtin`  — host built-in bridge; not available without an import.
// - `io_in/io_out`  — raw I/O; WASM does I/O only through host imports (WASI).
// - `cast`          — type reinterpretation; WASM is strictly typed — you
//                     cannot round-trip an i32 to a float by punning bits
//                     without explicit conversion instructions (reinterpret
//                     exists but we don't generate it in v1).
// - `load_mem/store_mem` — raw linear-memory access; this lowering produces
//                     no linear memory section (no `(memory ...)` declaration).
// - `alloc/box/unbox/field_load/field_store/is_null` — GC heap ops; WASM 1.0
//                     has no garbage collector.
// - `safepoint`     — GC coordination; handled by the runtime, not by us.

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
// validate_for_wasm
// ---------------------------------------------------------------------------

/// Validate an `IIRModule` for WASM lowering.
///
/// Returns a `Vec<String>` of human-readable error messages.  An empty
/// vector means the module is safe to pass to [`crate::lower::lower_iir_to_wasm`].
///
/// # Checks
///
/// 1. **EmptyModule** — at least one function must exist; otherwise the WASM
///    module has no type or code sections.
///
/// 2. **EmptyFunction** — each function must have at least one instruction.
///    An empty body is almost certainly a front-end bug.
///
/// 3. **UntypedInstruction** — `type_hint` must not be `"any"` or
///    `"polymorphic"`.  WASM arithmetic is typed: the stack type must be
///    known statically, so we require the front-end to have resolved all
///    `"any"` annotations before lowering.
///
/// 4. **UnsupportedType** — `type_hint` must not be `"str"` (no string
///    arithmetic) or start with `"ref<"` (no heap pointers in this lowering).
///    Float types (`"f32"`, `"f64"`) ARE supported (unlike the BEAM backend).
///
/// 5. **UnsupportedOp** — see `UNSUPPORTED_OPS` above.
///
/// # Example
///
/// ```
/// use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
/// use iir_to_wasm::validate_for_wasm;
///
/// let fn_ = IIRFunction::new("main", vec![], "void",
///     vec![IIRInstr::new("ret_void", None, vec![], "void")]);
/// let module = IIRModule {
///     name: "test".into(),
///     functions: vec![fn_],
///     entry_point: Some("main".into()),
///     language: "test".into(),
/// };
/// assert!(validate_for_wasm(&module).is_empty());
/// ```
pub fn validate_for_wasm(module: &IIRModule) -> Vec<String> {
    let mut errors = Vec::new();

    // ── Check 1: EmptyModule ─────────────────────────────────────────────────
    //
    // A WASM module with no functions has empty type, function, export, and
    // code sections — technically valid binary, but produces nothing useful.
    // Reject early so the caller gets a clear diagnostic.
    if module.functions.is_empty() {
        errors.push("EmptyModule: module has no functions".to_string());
        // Return early — per-function checks below would be vacuous.
        return errors;
    }

    for func in &module.functions {
        // ── Check 2: EmptyFunction ───────────────────────────────────────────
        //
        // WASM requires every code-section entry to end with an `end` (0x0B)
        // opcode.  An empty IIR function body would produce a code entry with
        // only the trailing `end`, which is valid WASM but almost certainly
        // indicates a front-end bug.
        if func.instructions.is_empty() {
            errors.push(format!(
                "EmptyFunction: function {:?} has no instructions",
                func.name
            ));
            // Skip instruction-level checks for this function.
            continue;
        }

        for instr in &func.instructions {
            // ── Check 3: UntypedInstruction ──────────────────────────────────
            //
            // WASM is typed: every value pushed onto the operand stack must
            // have a known type at code-generation time.  We cannot emit a
            // WASM `add` without knowing whether to emit `i32.add`, `i64.add`,
            // or `f64.add`.  Require the front-end to have resolved all `"any"`
            // annotations via type inference or profiling.
            //
            // `"polymorphic"` is the profiler's sentinel for "seen multiple
            // types at runtime" — equally useless for static WASM lowering.
            if instr.type_hint == "any" || instr.type_hint == "polymorphic" {
                errors.push(format!(
                    "UntypedInstruction: function {:?}, op {:?} has type_hint {:?}; \
                     WASM lowering requires concrete types (not \"any\"/\"polymorphic\")",
                    func.name, instr.op, instr.type_hint
                ));
            }

            // ── Check 4: UnsupportedType ─────────────────────────────────────
            //
            // `"str"` — we produce no string data section and no string
            // operations; this lowering is purely numeric.
            //
            // `"ref<…>"` — heap pointer types require GC-managed memory.
            // WASM 1.0 has no garbage collector and no reference types (the
            // reference-types proposal came later).
            //
            // NOTE: float types (`"f32"`, `"f64"`) are NOT rejected here.
            // WASM has native float arithmetic, so they are fully supported.
            if instr.type_hint == "str" {
                errors.push(format!(
                    "UnsupportedType: function {:?}, op {:?} has type_hint \"str\"; \
                     string operations are not supported in this WASM backend",
                    func.name, instr.op
                ));
            } else if instr.type_hint.starts_with("ref<") {
                errors.push(format!(
                    "UnsupportedType: function {:?}, op {:?} has reference type {:?}; \
                     heap pointer types are not supported in this WASM backend",
                    func.name, instr.op, instr.type_hint
                ));
            }

            // ── Check 5: UnsupportedOp ───────────────────────────────────────
            //
            // These opcodes require host imports, OS system calls, or GC
            // infrastructure that this direct lowering does not provide.
            if UNSUPPORTED_OPS.contains(&instr.op.as_str()) {
                errors.push(format!(
                    "UnsupportedOp: function {:?}, op {:?} is not supported by \
                     the WASM backend; it requires a host import or runtime support",
                    func.name, instr.op
                ));
            }
        }
    }

    errors
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};

    // Helper: build a single-function module with the given instructions.
    fn module_with(instrs: Vec<IIRInstr>) -> IIRModule {
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
        let errs = validate_for_wasm(&module);
        assert!(!errs.is_empty(), "should reject empty module");
        assert!(errs[0].contains("EmptyModule"));
    }

    #[test]
    fn empty_function_rejected() {
        let errs = validate_for_wasm(&module_with(vec![]));
        assert!(!errs.is_empty());
        assert!(errs[0].contains("EmptyFunction"));
    }

    #[test]
    fn any_type_rejected() {
        let errs = validate_for_wasm(&module_with(vec![IIRInstr::new(
            "add",
            Some("v".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())],
            "any",
        )]));
        assert!(errs.iter().any(|e| e.contains("UntypedInstruction")));
    }

    #[test]
    fn polymorphic_type_rejected() {
        let errs = validate_for_wasm(&module_with(vec![IIRInstr::new(
            "add",
            Some("v".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())],
            "polymorphic",
        )]));
        assert!(errs.iter().any(|e| e.contains("UntypedInstruction")));
    }

    #[test]
    fn str_type_rejected() {
        let errs = validate_for_wasm(&module_with(vec![IIRInstr::new(
            "const",
            Some("v".into()),
            vec![Operand::Int(0)],
            "str",
        )]));
        assert!(errs.iter().any(|e| e.contains("UnsupportedType")));
    }

    #[test]
    fn ref_type_rejected() {
        let errs = validate_for_wasm(&module_with(vec![IIRInstr::new(
            "const",
            Some("v".into()),
            vec![Operand::Int(0)],
            "ref<Foo>",
        )]));
        assert!(errs.iter().any(|e| e.contains("UnsupportedType")));
    }

    #[test]
    fn float_type_accepted() {
        // Float types are valid WASM — unlike the BEAM backend, we do NOT
        // reject them.
        let errs = validate_for_wasm(&module_with(vec![
            IIRInstr::new(
                "const",
                Some("v".into()),
                vec![Operand::Float(3.14)],
                "f64",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(
            errs.is_empty(),
            "float types should be accepted; got: {:?}",
            errs
        );
    }

    #[test]
    fn unsupported_ops_rejected() {
        for op in &[
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
        ] {
            let errs = validate_for_wasm(&module_with(vec![IIRInstr::new(
                *op,
                None,
                vec![],
                "void",
            )]));
            assert!(
                errs.iter().any(|e| e.contains("UnsupportedOp")),
                "expected UnsupportedOp for op {:?}; got {:?}",
                op,
                errs
            );
        }
    }

    #[test]
    fn valid_void_function_no_errors() {
        let errs = validate_for_wasm(&module_with(vec![IIRInstr::new(
            "ret_void",
            None,
            vec![],
            "void",
        )]));
        assert!(errs.is_empty(), "unexpected errors: {:?}", errs);
    }

    #[test]
    fn valid_i32_add_no_errors() {
        let errs = validate_for_wasm(&module_with(vec![
            IIRInstr::new(
                "add",
                Some("v0".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i32",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
        ]));
        assert!(errs.is_empty(), "unexpected errors: {:?}", errs);
    }
}
