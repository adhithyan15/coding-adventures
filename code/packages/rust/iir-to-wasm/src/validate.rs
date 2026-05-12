//! Pre-flight validation for IIR → WASM lowering.
//!
//! # Why validate separately?
//!
//! WebAssembly (both 1.0 and WasmGC) is a **statically typed, structured**
//! instruction set.  Not every IIR program can be lowered:
//!
//! - WASM has no "any" type — every local and stack slot must have a concrete
//!   numeric type (`i32`, `i64`, `f32`, `f64`) or a known GC reference type.
//! - WASM has no strings or raw heap-pointer indirection in this lowering.
//! - Runtime / I/O opcodes have no WASM equivalent without a host import,
//!   which this direct lowering does not provide.
//!
//! Catching these problems *before* lowering gives clear, actionable error
//! messages rather than a panic or a silently malformed binary.
//!
//! # Key differences from WASM 1.0 backend
//!
//! **Float constants ARE allowed here.**  WASM has native `f64.const` and
//! `f32.const` instructions.
//!
//! **WasmGC heap ops ARE allowed here** (Phase 2).  The WasmGC proposal
//! (standardised 2023) ships in V8/Chrome ≥ 119, Firefox ≥ 120, and
//! wasmtime ≥ 14.0.  The following IIR ops now lower to WasmGC bytecode
//! when the `type_hint` is `"ref<LispyPair>"`:
//!
//! | IIR op | Notes |
//! |--------|-------|
//! | `alloc` | Allocates a new `$LispyPair` struct on the GC heap |
//! | `field_load` | `car` (field 0) or `cdr` (field 1) |
//! | `field_store` | Mutate a field of a `$LispyPair` |
//! | `is_null` | Test for null reference |
//! | `const ref<LispyPair>` | Push a typed null (nil) |
//!
//! `"ref<Other>"` types (anything other than `"ref<LispyPair>"`) are still
//! rejected, since we only define the `$LispyPair` struct type.
//!
//! # Checks performed
//!
//! | Error label | Condition |
//! |-------------|-----------|
//! | `EmptyModule` | Module has zero functions |
//! | `EmptyFunction` | A function has zero instructions |
//! | `UntypedInstruction` | `type_hint` is `"any"` or `"polymorphic"` |
//! | `UnsupportedType` | `type_hint` is `"str"` or is an unsupported `"ref<X>"` |
//! | `UnsupportedOp` | op is any runtime / I/O / unsupported GC opcode |

use interpreter_ir::IIRModule;

// ---------------------------------------------------------------------------
// WasmGC-supported type hints
// ---------------------------------------------------------------------------
//
// Reference type hints that this backend understands.  Any `ref<X>` not in
// this set is still rejected (we don't have a struct definition for it).
//
// Currently we support only `ref<LispyPair>` — the 2-field GC cons cell
// used by the Lispy runtime.  Future work can add more struct types here.

const SUPPORTED_REF_TYPES: &[&str] = &["ref<LispyPair>"];

/// Return `true` if `type_hint` is a reference type that this backend can
/// lower to a WasmGC struct reference.
pub fn is_supported_ref_type(type_hint: &str) -> bool {
    SUPPORTED_REF_TYPES.contains(&type_hint)
}

// ---------------------------------------------------------------------------
// WasmGC-supported opcode table
// ---------------------------------------------------------------------------
//
// These opcodes are accepted when paired with an appropriate type hint.
// They lower to WasmGC instructions (`struct.new`, `struct.get`, etc.).

const GC_OPS: &[&str] = &["alloc", "field_load", "field_store", "is_null"];

// ---------------------------------------------------------------------------
// Unsupported opcode table
// ---------------------------------------------------------------------------
//
// These opcodes require runtime support that the WASM backend cannot express
// as plain numeric or WasmGC instructions:
//
// - `call_builtin`  — host built-in bridge; not available without an import.
// - `io_in`         — raw byte-level I/O input; WASM does I/O through host imports.
// - `cast`          — type reinterpretation without a `reinterpret` path.
// - `load_mem/store_mem` — raw linear-memory access; no linear memory section.
// - `box/unbox`     — boxing ops on non-LispyPair types.
// - `safepoint`     — GC coordination; handled by the runtime.
//
// Note: `alloc`, `field_load`, `field_store`, `is_null` are NOT here —
// they are accepted for `ref<LispyPair>` and handled by the GC lowering.
//
// LANG32 — supported in WASM backend (Phase 3):
// - `io_out`        — lowered to `call $__print_i64` (host import).
// - `global_store`  — lowered to `global.set <idx>` (WASM global section).
// - `global_load`   — lowered to `global.get <idx>` (WASM global section).

const UNSUPPORTED_OPS: &[&str] = &[
    "call_builtin",
    "io_in",
    // "io_out"       — LANG32: now supported (host import $__print_i64).
    // "global_store" — LANG32: now supported (WASM global.set).
    // "global_load"  — LANG32: now supported (WASM global.get).
    "cast",
    "load_mem",
    "store_mem",
    "box",
    "unbox",
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
///     exports: vec![],
///     imports: vec![],
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
            // `"str"` — we produce no string data section and no string ops.
            //
            // `"ref<X>"` — reference types require WasmGC.  We accept
            // `"ref<LispyPair>"` (the only struct type we define).  All
            // other `ref<...>` types are rejected with an explanation.
            //
            // NOTE: float types (`"f32"`, `"f64"`) are NOT rejected here.
            // WASM has native float arithmetic, so they are fully supported.
            if instr.type_hint == "str" {
                errors.push(format!(
                    "UnsupportedType: function {:?}, op {:?} has type_hint \"str\"; \
                     string operations are not supported in this WASM backend",
                    func.name, instr.op
                ));
            } else if instr.type_hint.starts_with("ref<")
                && !is_supported_ref_type(&instr.type_hint)
            {
                errors.push(format!(
                    "UnsupportedType: function {:?}, op {:?} has reference type {:?}; \
                     only ref<LispyPair> is supported in this WasmGC backend",
                    func.name, instr.op, instr.type_hint
                ));
            }

            // ── Check 5: UnsupportedOp ───────────────────────────────────────
            //
            // Hard-rejected ops (require host imports or unimplemented GC).
            // GC ops (`alloc`, `field_load`, `field_store`, `is_null`) are
            // NOT in UNSUPPORTED_OPS — they are accepted when paired with
            // `ref<LispyPair>`.  Reject them here only when the type hint
            // is NOT a supported reference type.
            if UNSUPPORTED_OPS.contains(&instr.op.as_str()) {
                errors.push(format!(
                    "UnsupportedOp: function {:?}, op {:?} is not supported by \
                     the WASM backend; it requires a host import or runtime support",
                    func.name, instr.op
                ));
            } else if instr.op == "alloc" || instr.op == "field_load" || instr.op == "field_store" {
                // These GC ops require the instruction's type_hint to be a
                // supported reference type.  They allocate or access fields
                // of a specific struct type; without the correct type hint
                // we cannot determine which struct to use.
                if !is_supported_ref_type(&instr.type_hint) {
                    errors.push(format!(
                        "UnsupportedOp: function {:?}, op {:?} (GC op) requires \
                         type_hint \"ref<LispyPair>\" but got {:?}",
                        func.name, instr.op, instr.type_hint
                    ));
                }
            }
            // Note: `is_null` is intentionally NOT checked here because it
            // is a generic null test that works on any nullable reference.
            // Its result type_hint may be "bool" or "i32" (the i32 result
            // of the ref.is_null instruction), not a ref type.
        }

        // ── Check 6: TooManyLabels (DoS guard) ──────────────────────────────
        //
        // The dispatch-loop pattern allocates O(N) memory for N label
        // instructions per function (one basic block + one br_table entry each).
        // Without a cap, a malformed module with millions of labels causes the
        // compiler to allocate gigabytes of memory.  We apply the same limit
        // that a realistic WASM function would approach before hitting the WASM
        // spec's own code-section size limit.
        const MAX_LABELS_PER_FUNCTION: usize = 65_536;
        let label_count = func.instructions.iter().filter(|i| i.op == "label").count();
        if label_count > MAX_LABELS_PER_FUNCTION {
            errors.push(format!(
                "TooManyLabels: function {:?} has {} label instructions; \
                 the WASM dispatch-loop backend supports at most {} per function",
                func.name, label_count, MAX_LABELS_PER_FUNCTION
            ));
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
        // These ops are unconditionally rejected.
        // Note: `io_out`, `global_store`, `global_load` are NOT in this list —
        // they were promoted to supported in LANG32.
        for op in &[
            "call_builtin",
            "io_in",
            "cast",
            "load_mem",
            "store_mem",
            "box",
            "unbox",
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
    fn io_out_passes_validation() {
        // LANG32: io_out is now supported in the WASM backend.
        let errs = validate_for_wasm(&module_with(vec![IIRInstr::new(
            "io_out",
            None,
            vec![Operand::Var("v".into())],
            "void",
        )]));
        assert!(
            errs.iter().all(|e| !e.contains("UnsupportedOp")),
            "io_out should be accepted by WASM validator (LANG32); got: {:?}",
            errs
        );
    }

    // GC ops that require a ref type hint are rejected when given i32.
    #[test]
    fn gc_ops_with_non_ref_type_rejected() {
        // alloc, field_load, field_store REQUIRE ref<LispyPair> type hint.
        for op in &["alloc", "field_load", "field_store"] {
            let errs = validate_for_wasm(&module_with(vec![IIRInstr::new(
                *op,
                None,
                vec![],
                "i32", // wrong type: should be ref<LispyPair>
            )]));
            assert!(
                errs.iter().any(|e| e.contains("UnsupportedOp")),
                "expected UnsupportedOp for GC op {:?} with i32 type; got {:?}",
                op,
                errs
            );
        }
        // is_null works with any type hint (including bool/i32) — it's a
        // generic null test, so we do NOT reject it for non-ref type hints.
    }

    // ref<LispyPair> type hint is accepted (WasmGC Phase 2).
    #[test]
    fn ref_lispy_pair_type_accepted() {
        let errs = validate_for_wasm(&module_with(vec![
            IIRInstr::new(
                "alloc",
                Some("p".into()),
                vec![],
                "ref<LispyPair>",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        // Should have no UnsupportedType error for ref<LispyPair>.
        assert!(
            !errs.iter().any(|e| e.contains("UnsupportedType")),
            "ref<LispyPair> should be accepted; got: {:?}",
            errs
        );
    }

    // ref<Other> is still rejected.
    #[test]
    fn ref_other_type_rejected() {
        let errs = validate_for_wasm(&module_with(vec![IIRInstr::new(
            "alloc",
            Some("p".into()),
            vec![],
            "ref<Other>",
        )]));
        assert!(
            errs.iter().any(|e| e.contains("UnsupportedType")),
            "ref<Other> should be rejected; got: {:?}",
            errs
        );
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
