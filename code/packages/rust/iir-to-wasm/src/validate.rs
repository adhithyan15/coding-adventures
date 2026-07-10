//! Pre-flight validation for IIR → WASM lowering.
//!
//! # Why validate separately?
//!
//! WebAssembly (both 1.0 and WasmGC) is a **statically typed, structured**
//! instruction set.  Not every IIR program can be lowered:
//!
//! - WASM has no "any" type — every local and stack slot must have a concrete
//!   numeric type (`i32`, `i64`, `f32`, `f64`) or a known GC reference type.
//! - WASM has only the E4 literal string foothold in this lowering:
//!   `str_const` + `str_len` + `str_index` + `str_eq` + `str_cmp` + `str_concat` +
//!   `print_str`; richer dynamic string ops remain rejected.
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
//! | `UnsupportedType` | `type_hint` is `"str"` outside `str_const` or is an unsupported `"ref<X>"` |
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

const SUPPORTED_REF_TYPES: &[&str] = &["ref<LispyPair>", "ref<any>"];

/// Return `true` if `type_hint` is a reference type that this backend can
/// lower to a WasmGC struct reference.
///
/// `ref<LispyPair>` lowers to `(ref $LispyPair)` — a typed cons-cell
/// reference.  `ref<any>` lowers to `anyref` — used as the result type
/// of `field_load` since cons-cell fields are declared as
/// `(mut (ref null any))`.  This matches BEAM's convention (loaded
/// field has type `ref<any>`).
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
    // `call_builtin` is *conditionally* unsupported — handled below.  See
    // `CALL_BUILTIN_SUPPORTED_NAMES` and the call_builtin branch in the
    // per-instruction loop.  Whitelisting specific builtins (`putchar`,
    // `getchar`) lets Brainfuck flow through this backend while still
    // rejecting unknown / unsafe builtin names.
    "io_in",
    // "io_out"       — LANG32: now supported (host import $__print_i64).
    // "global_store" — LANG32: now supported (WASM global.set).
    // "global_load"  — LANG32: now supported (WASM global.get).
    "cast",
    // "load_mem"     — Brainfuck: now supported (i32.load8_u over linear memory).
    // "store_mem"    — Brainfuck: now supported (i32.store8 over linear memory).
    // "box" / "unbox" — LANG77 L3b-3a: now supported. `box` lowers to `ref.i31`
    //   (I31New — box an i32 into an `i31ref`, a WasmGC tagged 31-bit integer
    //   reference) and `unbox` to `i31.get_s` (I31GetS — read it back as a
    //   sign-extended i32). These are the boxing primitives the uniform-anyref
    //   lisp value model needs: a lisp integer atom becomes an `i31ref` so it
    //   can live in a cons cell's `anyref` field alongside heap pairs.
    "safepoint",
];

/// Builtin names that the WASM backend can lower via a host import.
///
/// Each entry maps to a `(env, name)` WASM import pair that the host
/// environment is expected to supply.  Today's list covers the
/// Brainfuck I/O builtins:
///
/// | Builtin       | Host import         | Signature |
/// |---------------|---------------------|-----------|
/// | `"putchar"`    | `env.putchar`        | `(i32) -> ()`     |
/// | `"getchar"`    | `env.getchar`        | `() -> i32`       |
/// | `"print_i64"`  | `env.__print_i64`    | `(i64) -> ()`     |
/// | `"input_i64"`  | `env.__input_i64`    | `() -> i64`       |
/// | `"input_str"`  | `env.__input_str`    | `(i32,i32) -> ()`  |
///
/// `print_i64` (G2) reuses the same `env.__print_i64` import the
/// `io_out` opcode already injects.  This lets BASIC's `PRINT`
/// statement (which lowers to `call_builtin "print_i64"`) and
/// Twig's `io_out` opcode share a single host-provided printer
/// function.
///
/// `input_i64` (BA-INPUT) reads a line from stdin and parses it as
/// an i64.  The host provides `env.__input_i64() -> i64`.
///
/// Adding a new builtin requires:
///   1. Listing it here so the validator accepts it.
///   2. Adding a matching `case` to `lower.rs::emit_instr`'s
///      `call_builtin` branch that emits the right WASM call.
///   3. Injecting the import entry in `lower_iir_to_wasm` (see the
///      analogous wiring for `io_out` → `env.__print_i64`).
pub(crate) const CALL_BUILTIN_SUPPORTED_NAMES: &[&str] =
    // `pair?` / `not` / `equal?` are McCarthy lisp predicates (LANG77 L3b-3a-4):
    // `pair?` lowers to `ref.test $LispyPair` (is this lisp value a cons cell?),
    // the lisp `not` to `i32.eqz` (boolean negation), and `equal?` (McCarthy
    // `EQ` on atoms) to unbox-both-and-`i32.eq`. `ATOM x` = `not(pair? x)`.
    &["putchar", "getchar", "print_i64", "input_i64", "input_str", "pair?", "not", "equal?"];

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
            // ── Check 2.5: ClosureOpcode (LANG35) ────────────────────────────
            //
            // `alloc_closure` and `call_closure` are valid IIR opcodes (LANG34)
            // but the WASM backend does not yet implement closure lowering.
            // WasmGC closure support requires a `$Closure` struct type and
            // `call_indirect` dispatch, which are planned for a future LANG spec.
            //
            // We emit a specific `ClosureOpcode` error (not `UntypedInstruction`)
            // so callers understand the precise remediation: either run the
            // `iir-builtin-lowering` Phase 4 pass to downgrade these opcodes to
            // `call_builtin "make_closure"` / `"apply_closure"` before lowering,
            // or wait for the WASM closure backend spec.
            if matches!(instr.op.as_str(), "alloc_closure" | "call_closure") {
                errors.push(format!(
                    "ClosureOpcode: function {:?}, op {:?} — closure opcodes are not \
                     yet supported by the WASM backend; apply iir-builtin-lowering \
                     Phase 4 to downgrade to call_builtin form before lowering to WASM",
                    func.name, instr.op
                ));
                continue;
            }

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
            // `"str"` — accepted for the E4 literal-output/metadata foothold's
            // direct string producers (`str_const`, `str_concat`, `str_slice`). `str_len`,
            // `str_index`, `str_eq`, and `str_cmp` produce integers, not string values.
            // E4-dyn (E4d-3b): a `str` value is an i32 **handle**, so it may also
            // flow through a `call` (a `str` return / call result) and a `ret` (a
            // `str`-returning function) — both carry the handle as an i32.
            // Richer dynamic string ops still fail explicitly below.
            //
            // `"ref<X>"` — reference types require WasmGC.  We accept
            // `"ref<LispyPair>"` (the only struct type we define).  All
            // other `ref<...>` types are rejected with an explanation.
            //
            // NOTE: float types (`"f32"`, `"f64"`) are NOT rejected here.
            // WASM has native float arithmetic, so they are fully supported.
            if instr.type_hint == "str"
                && !matches!(
                    instr.op.as_str(),
                    // `call_builtin`: BASIC string `INPUT A$` — `input_str` returns a
                    //   `str` (the i32 handle of a `[i32 len][bytes]` linear-memory block).
                    // `mov`: copy the input handle into the `$`-variable's slot (a plain
                    //   i32 local copy).
                    // `array_get` / `array_set`: E4d-BA-arr BASIC string arrays — a `str`
                    //   element is a 4-byte i32 handle stored in / loaded from an
                    //   `array<str>` block (see `wasm_array_elem` in lower.rs).
                    "str_const" | "str_concat" | "str_slice" | "call" | "ret"
                        | "call_builtin" | "mov" | "array_get" | "array_set"
                )
            {
                errors.push(format!(
                    "UnsupportedType: function {:?}, op {:?} has type_hint \"str\"; \
                     only str_const + str_concat + str_slice + str_len + str_index + str_eq + str_cmp + print_str literal output is supported in this WASM backend",
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
            //
            // `call_builtin` is conditionally accepted: the builtin name
            // (carried in `srcs[0]` as `Operand::Var`) must be in
            // [`CALL_BUILTIN_SUPPORTED_NAMES`].  This lets Brainfuck's
            // `putchar` / `getchar` flow through while still rejecting
            // unknown / unsafe builtins.
            if instr.op == "str_const" {
                match (instr.dest.as_ref(), instr.srcs.first()) {
                    (Some(_), Some(interpreter_ir::Operand::Str(s)))
                        if s.is_ascii()
                            && s.bytes()
                                .all(|b| b >= 0x20 || matches!(b, b'\n' | b'\r' | b'\t')) =>
                    {
                        // Accepted — lower.rs puts the literal in a data segment.
                    }
                    (Some(_), Some(interpreter_ir::Operand::Str(_))) => {
                        errors.push(format!(
                            "UnsupportedOp: function {:?}, op \"str_const\" only supports \
                             printable ASCII string literals in the WASM literal-output slice",
                            func.name
                        ));
                    }
                    _ => {
                        errors.push(format!(
                            "UnsupportedOp: function {:?}, op \"str_const\" requires \
                             a dest and srcs[0] = Operand::Str(literal)",
                            func.name
                        ));
                    }
                }
            } else if instr.op == "str_concat" {
                match (instr.dest.as_ref(), instr.srcs.as_slice(), instr.type_hint.as_str()) {
                    (
                        Some(_),
                        [
                            interpreter_ir::Operand::Var(_),
                            interpreter_ir::Operand::Var(_),
                        ],
                        "str",
                    ) => {
                        // Accepted — lower.rs materialises literal concatenation metadata.
                    }
                    _ => {
                        errors.push(format!(
                            "UnsupportedOp: function {:?}, op \"str_concat\" requires \
                             dest, two Operand::Var sources, and str result type",
                            func.name
                        ));
                    }
                }
            } else if instr.op == "str_slice" {
                match (instr.dest.as_ref(), instr.srcs.as_slice(), instr.type_hint.as_str()) {
                    (
                        Some(_),
                        [
                            interpreter_ir::Operand::Var(_),
                            interpreter_ir::Operand::Var(_),
                            interpreter_ir::Operand::Var(_),
                        ],
                        "str",
                    ) => {
                        // Accepted — lower.rs materialises literal slice metadata.
                    }
                    _ => {
                        errors.push(format!(
                            "UnsupportedOp: function {:?}, op \"str_slice\" requires \
                             dest, string/start/end Operand::Var sources, and str result type",
                            func.name
                        ));
                    }
                }
            } else if instr.op == "str_index" {
                match (instr.dest.as_ref(), instr.srcs.as_slice(), instr.type_hint.as_str()) {
                    (
                        Some(_),
                        [
                            interpreter_ir::Operand::Var(_),
                            interpreter_ir::Operand::Var(_),
                        ],
                        "i64" | "i32",
                    ) => {
                        // Accepted — lower.rs bounds-checks and loads a literal byte.
                    }
                    _ => {
                        errors.push(format!(
                            "UnsupportedOp: function {:?}, op \"str_index\" requires \
                             dest, string Operand::Var, index Operand::Var, and i64/i32 result type",
                            func.name
                        ));
                    }
                }
            } else if instr.op == "str_len" {
                match (instr.dest.as_ref(), instr.srcs.as_slice(), instr.type_hint.as_str()) {
                    (Some(_), [interpreter_ir::Operand::Var(_)], "i64" | "i32") => {
                        // Accepted — lower.rs materialises the direct literal's byte length.
                    }
                    _ => {
                        errors.push(format!(
                            "UnsupportedOp: function {:?}, op \"str_len\" requires \
                             dest, one Operand::Var source, and i64/i32 result type",
                            func.name
                        ));
                    }
                }
            } else if instr.op == "str_eq" {
                match (instr.dest.as_ref(), instr.srcs.as_slice(), instr.type_hint.as_str()) {
                    (
                        Some(_),
                        [
                            interpreter_ir::Operand::Var(_),
                            interpreter_ir::Operand::Var(_),
                        ],
                        "i64" | "i32",
                    ) => {
                        // Accepted — lower.rs materialises literal equality.
                    }
                    _ => {
                        errors.push(format!(
                            "UnsupportedOp: function {:?}, op \"str_eq\" requires \
                             dest, two Operand::Var sources, and i64/i32 result type",
                            func.name
                        ));
                    }
                }
            } else if instr.op == "str_cmp" {
                match (instr.dest.as_ref(), instr.srcs.as_slice(), instr.type_hint.as_str()) {
                    (
                        Some(_),
                        [
                            interpreter_ir::Operand::Var(_),
                            interpreter_ir::Operand::Var(_),
                        ],
                        "i64" | "i32",
                    ) => {
                        // Accepted — lower.rs materialises literal ordering.
                    }
                    _ => {
                        errors.push(format!(
                            "UnsupportedOp: function {:?}, op \"str_cmp\" requires \
                             dest, two Operand::Var sources, and i64/i32 result type",
                            func.name
                        ));
                    }
                }
            } else if instr.op == "print_str" {
                match (instr.type_hint.as_str(), instr.srcs.first()) {
                    ("void", Some(interpreter_ir::Operand::Var(_))) => {
                        // Accepted — lower.rs calls env.__print_str(ptr, len).
                    }
                    _ => {
                        errors.push(format!(
                            "UnsupportedOp: function {:?}, op \"print_str\" requires \
                             type_hint \"void\" and srcs[0] = Operand::Var(str)",
                            func.name
                        ));
                    }
                }
            } else if UNSUPPORTED_OPS.contains(&instr.op.as_str()) {
                errors.push(format!(
                    "UnsupportedOp: function {:?}, op {:?} is not supported by \
                     the WASM backend; it requires a host import or runtime support",
                    func.name, instr.op
                ));
            } else if instr.op == "call_builtin" {
                // Inspect srcs[0] for the builtin name.  IIR carries it as
                // `Operand::Var(name)` (Rust's IIR has no separate string
                // operand kind — names and var refs share `Var`).
                let name: Option<&str> = match instr.srcs.first() {
                    Some(interpreter_ir::Operand::Var(s)) => Some(s.as_str()),
                    _ => None,
                };
                match name {
                    Some(n) if CALL_BUILTIN_SUPPORTED_NAMES.contains(&n) => {
                        // Accepted — emit_instr will lower it to a call into
                        // the corresponding host import.
                    }
                    _ => {
                        errors.push(format!(
                            "UnsupportedOp: function {:?}, op \"call_builtin\" with \
                             builtin name {:?} is not in the WASM backend's host-import \
                             whitelist (supported: {:?}); add the builtin to \
                             CALL_BUILTIN_SUPPORTED_NAMES and the lowering rule in \
                             lower.rs to extend coverage",
                            func.name, name, CALL_BUILTIN_SUPPORTED_NAMES
                        ));
                    }
                }
            } else if instr.op == "alloc" {
                // `alloc` requires the instruction's type_hint to be a
                // CONCRETE struct ref — currently only `ref<LispyPair>`.
                // We can't allocate `ref<any>` because we don't know which
                // struct shape to create.
                if instr.type_hint != "ref<LispyPair>" {
                    errors.push(format!(
                        "UnsupportedOp: function {:?}, op {:?} (GC op) requires \
                         type_hint \"ref<LispyPair>\" but got {:?}",
                        func.name, instr.op, instr.type_hint
                    ));
                }
            } else if instr.op == "field_load" {
                // `field_load` follows iir-builtin-lowering's Phase 2
                // convention: the loaded value's type is `"ref<any>"`
                // because cons-cell fields can hold any Lisp value.  We
                // additionally accept `"ref<LispyPair>"` for forward
                // compatibility with frontends that propagate the typed
                // tail of a cons chain back into the field_load.
                if instr.type_hint != "ref<any>" && !is_supported_ref_type(&instr.type_hint) {
                    errors.push(format!(
                        "UnsupportedOp: function {:?}, op {:?} (GC op) requires \
                         type_hint \"ref<any>\" or \"ref<LispyPair>\" but got {:?}",
                        func.name, instr.op, instr.type_hint
                    ));
                }
            } else if instr.op == "field_store" {
                // `field_store` matches iir-builtin-lowering's Phase 2
                // convention: `type_hint == "void"` (the write returns
                // nothing).  The pair type is determined from the cons-cell
                // operand's typing context, not from the instruction's
                // hint.  We additionally accept `"ref<LispyPair>"` for
                // forward compatibility with frontends that propagate the
                // object type onto the store.
                if instr.type_hint != "void" && !is_supported_ref_type(&instr.type_hint) {
                    errors.push(format!(
                        "UnsupportedOp: function {:?}, op {:?} (GC op) requires \
                         type_hint \"void\" or \"ref<LispyPair>\" but got {:?}",
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
    fn e4_literal_print_accepted() {
        let errs = validate_for_wasm(&module_with(vec![
            IIRInstr::new(
                "str_const",
                Some("s".into()),
                vec![Operand::Str("HELLO".into())],
                "str",
            ),
            IIRInstr::new("print_str", None, vec![Operand::Var("s".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(
            errs.is_empty(),
            "str_const + print_str should be accepted; got: {:?}",
            errs
        );
    }

    #[test]
    fn e4_literal_str_len_accepted() {
        let errs = validate_for_wasm(&module_with(vec![
            IIRInstr::new(
                "str_const",
                Some("s".into()),
                vec![Operand::Str("ABC".into())],
                "str",
            ),
            IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("s".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
        ]));
        assert!(
            errs.is_empty(),
            "str_len over a direct literal should be accepted; got: {:?}",
            errs
        );
    }

    #[test]
    fn e4_literal_str_eq_accepted() {
        let errs = validate_for_wasm(&module_with(vec![
            IIRInstr::new(
                "str_const",
                Some("a".into()),
                vec![Operand::Str("A".into())],
                "str",
            ),
            IIRInstr::new(
                "str_const",
                Some("b".into()),
                vec![Operand::Str("A".into())],
                "str",
            ),
            IIRInstr::new("str_eq", Some("ok".into()), vec![
                Operand::Var("a".into()),
                Operand::Var("b".into()),
            ], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("ok".into())], "i64"),
        ]));
        assert!(
            errs.is_empty(),
            "str_eq over direct literals should be accepted; got: {:?}",
            errs
        );
    }

    #[test]
    fn e4_literal_str_cmp_accepted() {
        let errs = validate_for_wasm(&module_with(vec![
            IIRInstr::new(
                "str_const",
                Some("a".into()),
                vec![Operand::Str("ALPHA".into())],
                "str",
            ),
            IIRInstr::new(
                "str_const",
                Some("b".into()),
                vec![Operand::Str("BETA".into())],
                "str",
            ),
            IIRInstr::new("str_cmp", Some("ord".into()), vec![
                Operand::Var("a".into()),
                Operand::Var("b".into()),
            ], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("ord".into())], "i64"),
        ]));
        assert!(
            errs.is_empty(),
            "str_cmp over direct literals should be accepted; got: {:?}",
            errs
        );
    }

    #[test]
    fn e4_literal_str_concat_len_accepted() {
        let errs = validate_for_wasm(&module_with(vec![
            IIRInstr::new(
                "str_const",
                Some("a".into()),
                vec![Operand::Str("AB".into())],
                "str",
            ),
            IIRInstr::new(
                "str_const",
                Some("b".into()),
                vec![Operand::Str("CDE".into())],
                "str",
            ),
            IIRInstr::new("str_concat", Some("s".into()), vec![
                Operand::Var("a".into()),
                Operand::Var("b".into()),
            ], "str"),
            IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("s".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
        ]));
        assert!(
            errs.is_empty(),
            "str_concat + str_len over direct literals should be accepted; got: {:?}",
            errs
        );
    }

    #[test]
    fn e4_literal_str_slice_index_accepted() {
        let errs = validate_for_wasm(&module_with(vec![
            IIRInstr::new(
                "str_const",
                Some("s".into()),
                vec![Operand::Str("ABCDE".into())],
                "str",
            ),
            IIRInstr::new("const", Some("start".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("const", Some("end".into()), vec![Operand::Int(4)], "i64"),
            IIRInstr::new(
                "str_slice",
                Some("sub".into()),
                vec![
                    Operand::Var("s".into()),
                    Operand::Var("start".into()),
                    Operand::Var("end".into()),
                ],
                "str",
            ),
            IIRInstr::new("const", Some("i".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new(
                "str_index",
                Some("b".into()),
                vec![Operand::Var("sub".into()), Operand::Var("i".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i64"),
        ]));
        assert!(
            errs.is_empty(),
            "str_slice + str_index over direct literals should be accepted; got: {:?}",
            errs
        );
    }

    #[test]
    fn e4_literal_str_index_accepted() {
        let errs = validate_for_wasm(&module_with(vec![
            IIRInstr::new(
                "str_const",
                Some("s".into()),
                vec![Operand::Str("ABC".into())],
                "str",
            ),
            IIRInstr::new("const", Some("i".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("str_index", Some("b".into()), vec![
                Operand::Var("s".into()),
                Operand::Var("i".into()),
            ], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i64"),
        ]));
        assert!(
            errs.is_empty(),
            "str_index over a direct literal should be accepted; got: {:?}",
            errs
        );
    }

    #[test]
    fn richer_string_ops_still_rejected() {
        let errs = validate_for_wasm(&module_with(vec![
            IIRInstr::new(
                "str_const",
                Some("s".into()),
                vec![Operand::Str("A".into())],
                "str",
            ),
            IIRInstr::new("str_index", Some("b".into()), vec![Operand::Var("s".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("b".into())], "i64"),
        ]));
        assert!(
            errs.iter().any(|e| e.contains("UnsupportedOp")),
            "malformed string ops must remain rejected; got: {:?}",
            errs
        );
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
        // `load_mem`, `store_mem` are NOT in this list — Brainfuck linear-
        // memory lowering promoted them to supported in the BF→WASM PR.
        // `call_builtin` is NOT in this list — it's conditionally accepted
        // for builtin names in `CALL_BUILTIN_SUPPORTED_NAMES`; see the
        // call_builtin_*_tests below.
        for op in &[
            "io_in",
            "cast",
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

    // ─── BF lowering: load_mem / store_mem now pass ─────────────────────────

    #[test]
    fn load_mem_accepted_for_bf() {
        let errs = validate_for_wasm(&module_with(vec![
            // load_mem v ptr [u8]  — read tape cell into v.
            IIRInstr::new(
                "load_mem",
                Some("v".into()),
                vec![Operand::Var("ptr".into())],
                "u8",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(
            errs.iter().all(|e| !e.contains("UnsupportedOp")),
            "load_mem should be accepted by WASM validator for Brainfuck \
             linear-memory lowering; got: {:?}",
            errs
        );
    }

    #[test]
    fn store_mem_accepted_for_bf() {
        let errs = validate_for_wasm(&module_with(vec![
            // store_mem ptr v [u8]  — write v into tape[ptr].
            IIRInstr::new(
                "store_mem",
                None,
                vec![Operand::Var("ptr".into()), Operand::Var("v".into())],
                "u8",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(
            errs.iter().all(|e| !e.contains("UnsupportedOp")),
            "store_mem should be accepted by WASM validator; got: {:?}",
            errs
        );
    }

    // ─── BF lowering: call_builtin whitelist (putchar / getchar) ─────────────

    #[test]
    fn call_builtin_putchar_accepted() {
        let errs = validate_for_wasm(&module_with(vec![
            // call_builtin putchar v [void]  — write v as a byte to stdout.
            IIRInstr::new(
                "call_builtin",
                None,
                vec![
                    Operand::Var("putchar".into()),
                    Operand::Var("v".into()),
                ],
                "void",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(
            errs.iter().all(|e| !e.contains("UnsupportedOp")),
            "call_builtin \"putchar\" should be accepted; got: {:?}",
            errs
        );
    }

    #[test]
    fn call_builtin_getchar_accepted() {
        let errs = validate_for_wasm(&module_with(vec![
            // call_builtin v getchar [u8]  — read a byte from stdin into v.
            IIRInstr::new(
                "call_builtin",
                Some("v".into()),
                vec![Operand::Var("getchar".into())],
                "u8",
            ),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(
            errs.iter().all(|e| !e.contains("UnsupportedOp")),
            "call_builtin \"getchar\" should be accepted; got: {:?}",
            errs
        );
    }

    #[test]
    fn call_builtin_unknown_name_rejected() {
        // An arbitrary builtin name not in the whitelist must still fail
        // validation so unknown / unsafe builtins can't slip through.
        let errs = validate_for_wasm(&module_with(vec![IIRInstr::new(
            "call_builtin",
            None,
            vec![Operand::Var("system_exec".into())],
            "void",
        )]));
        assert!(
            errs.iter().any(|e| e.contains("UnsupportedOp")
                && e.contains("system_exec")),
            "unknown call_builtin name should be rejected; got: {:?}",
            errs
        );
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
