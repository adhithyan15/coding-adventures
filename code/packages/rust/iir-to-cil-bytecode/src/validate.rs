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
//! | `UnsupportedType`       | `type_hint` is unsupported (`"str"` except `str_const`, or unsupported `ref<...>`) |
//! | `UnsupportedType` (float const) | `op == "const"` and src is `Operand::Float` |
//! | `UnsupportedOp`         | op is a runtime/memory/IO/GC opcode that hasn't been promoted (list below) |
//!
//! Remaining unsupported ops: `call_builtin`, `io_in`, `io_out`, `cast`,
//! `load_mem`, `store_mem`, `box`, `unbox`, `safepoint`, and the byte-oriented
//! E4 string algebra beyond `str_const` + `str_len` + `str_index` +
//! `str_eq` + `str_cmp` + `str_concat` + `print_str`.
//! Previously unsupported but now accepted: `alloc` (LispyPair only),
//! `field_load`, `field_store`, `is_null`.

use std::collections::HashMap;

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
//
// LANG37 — supported in CLR backend:
// - `alloc_closure` — lowered to `newarr int32[]` + `stelem.i4` sequence.
// - `call_closure`  — lowered to `__callClosure(int32[], int32[])` call.
//   Exception: i64/u64/f32/f64 captures produce a ClosureOpcode error.

const UNSUPPORTED_OPS: &[&str] = &[
    // `call_builtin` is *conditionally* unsupported — handled below.
    // See [`CALL_BUILTIN_SUPPORTED_NAMES`] for the whitelist.  Whitelisting
    // specific builtins (today: `putchar`, `getchar`) lets Brainfuck flow
    // through this backend while still rejecting unknown / unsafe names.
    "io_in",
    // "io_out"       — LANG32: now supported (Console.WriteLine).
    // "global_store" — returns UnsupportedOp from lower.rs, not rejected by validator.
    // "global_load"  — returns UnsupportedOp from lower.rs, not rejected by validator.
    "cast",
    // "load_mem"   — Brainfuck: now supported (ldelem.u1 over env.BFRuntime::__tape).
    // "store_mem"  — Brainfuck: now supported (stelem.i1 over env.BFRuntime::__tape).
    // "alloc"      — promoted in Phase 2 (ref<LispyPair> only)
    // "box" / "unbox" — promoted in McCarthy W6b (box [int32] / unbox.any [int32]).
    // "field_load" — promoted in Phase 2
    // "field_store" — promoted in Phase 2
    // "is_null"    — promoted in Phase 2
    "safepoint",
];

/// Builtin names that the CLR backend can lower via `call` to a
/// host-provided helper class.
///
/// Each entry maps to a `(class, name, signature)` triple resolved at
/// lowering time via reserved metadata tokens (MemberRef table rows on
/// the simulated `env.BFRuntime` class):
///
/// | Builtin       | CIL call                                              |
/// |---------------|-------------------------------------------------------|
/// | `"putchar"`   | `call void env.BFRuntime::putchar(int32)`             |
/// | `"getchar"`   | `call int32 env.BFRuntime::getchar()`                 |
/// | `"print_i64"` | `call void env.BasicRuntime::PrintI64(int64)`         |
///
/// Adding a new builtin requires:
///   1. Listing the name here so the validator accepts it.
///   2. Reserving a new metadata-token constant in `lower.rs`.
///   3. Adding a matching `case` to lower.rs's `call_builtin` arm
///      that emits the right `call <token>` sequence.
///
/// G4 note: `print_i64` is the CLR counterpart to wasm's `env.__print_i64`
/// (iir-to-wasm v0.8.0) and JVM's `env/BasicRuntime.println(J)V`
/// (iir-to-jvm-class-file v0.7.0).  Together the three lower BASIC's
/// `PRINT` statement to real backend bytecode without the backend itself
/// owning a stdout.  We pick a dedicated `env.BasicRuntime` host class
/// (distinct from `env.BFRuntime`) because BASIC's I/O model is
/// line/value oriented while Brainfuck's is byte-stream oriented; a CLR
/// runtime can stub or provide either independently.
pub(crate) const CALL_BUILTIN_SUPPORTED_NAMES: &[&str] =
    &["putchar", "getchar", "print_i64", "pair?", "not", "equal?"];

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

        // ── Check 2.5 pre-pass: build variable-type map for this function ────
        //
        // Needed to detect i64/float-typed captures in `alloc_closure`.
        // The map is built from function parameters and instruction dest
        // type_hints (first declaration wins).
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
            // ── Check 2.5: Closure early-accept (LANG37) ─────────────────────
            //
            // `alloc_closure` and `call_closure` (LANG34 opcodes) are fully
            // supported by the CLR backend since LANG37, using an `int32[]`
            // dispatch-table approach:
            //
            //   alloc_closure(Str(fn_name), Var(cap0), …) : "closure"
            //     → int32[] {fn_dispatch_idx, cap0_as_i32, …}
            //
            //   call_closure(Var(handle), Var(arg0), …) : "any"
            //     → static __callClosure(int32[] closure, int32[] args)
            //
            // EXCEPTION: i64/u64/f32/f64 captures are not supported in v1.
            // The `int32[]` array can only hold 32-bit integers; wider captures
            // require either boxing (LANG38) or a wider array type.
            if instr.op == "alloc_closure" {
                // Check each capture operand (srcs[1..]).
                // srcs[0] is Operand::Str(fn_name) — not a capture variable.
                //
                // Each capture must be:
                //   (a) an Operand::Var — non-Var captures (Int, Float, Str)
                //       have no place in a variable-captured closure and are
                //       rejected with a clear error (not silently ignored).
                //   (b) of type i32/bool — i64/u64/f32/f64 captures require
                //       wider storage than int32[] provides (deferred to LANG38).
                for (i, src) in instr.srcs.iter().skip(1).enumerate() {
                    match src {
                        Operand::Var(cap_name) => {
                            let cap_type =
                                var_types.get(cap_name.as_str()).copied().unwrap_or("");
                            let is_wide = matches!(
                                cap_type,
                                "i64" | "u64" | "f32" | "f64"
                            );
                            if is_wide {
                                errors.push(format!(
                                    "ClosureOpcode: function {:?}, alloc_closure captures \
                                     variable {:?} (type {:?}); only i32/bool captures are \
                                     supported by the CLR backend in v1 — use integer types \
                                     or upgrade to LANG38",
                                    func.name, cap_name, cap_type
                                ));
                            }
                        }
                        other => {
                            // Non-Var operands at capture positions are always
                            // invalid — reject with a ClosureOpcode error.
                            errors.push(format!(
                                "ClosureOpcode: function {:?}, alloc_closure srcs[{}] \
                                 must be Var(captured variable), got {:?}; only variable \
                                 captures are supported",
                                func.name, i + 1, other
                            ));
                        }
                    }
                }
                continue; // accepted (i32/bool Var captures OK)
            }
            if instr.op == "call_closure" {
                // `call_closure` always has type_hint "any" — accepted here.
                // The lowering pass validates that the closure target exists.
                continue;
            }

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
            // `"str"` — The textual CLR path can now load ASCII literals and
            // concatenate them through `System.String` for the narrow E4
            // foothold. `str_len`, `str_eq`, and `str_cmp` produce integers. Other
            // string-typed producers still need a fuller byte-oriented
            // representation before we can map them to `System.String` safely.
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
            if instr.type_hint == "str"
                && !matches!(instr.op.as_str(), "str_const" | "str_concat" | "str_slice")
            {
                errors.push(format!(
                    "UnsupportedType: function {:?}, op {:?} has type_hint \"str\"; \
                     only str_const, str_concat, and str_slice literals are supported in this CLR backend",
                    func.name, instr.op
                ));
            } else if instr.type_hint.starts_with("ref<") {
                // Phase 2: `ref<LispyPair>` lowers to System.Object[2].
                // `ref<any>` lowers to System.Object (the field-load result
                // type — cons-cell fields are System.Object, matching
                // iir-builtin-lowering's Phase 2 convention and BEAM).
                let is_supported_ref = instr.type_hint == LISTY_PAIR_TYPE
                    || instr.type_hint == "ref<any>";
                let is_heap_op = matches!(
                    instr.op.as_str(),
                    "alloc" | "field_load" | "field_store" | "is_null"
                    | "const" | "ret" | "load_reg" | "store_reg"
                    | "jmp_if_true" | "jmp_if_false" | "mov"
                    // McCarthy W6b: `box` produces a `ref<any>` (boxed int32).
                    | "box" | "unbox"
                    // McCarthy W8b (lambda): a lisp `call` returns `ref<any>`
                    // (the callee's uniform-reference result).
                    | "call"
                );
                if !(is_supported_ref && is_heap_op) {
                    errors.push(format!(
                        "UnsupportedType: function {:?}, op {:?} has reference type {:?}; \
                         heap pointer types require ref<LispyPair> or ref<any> and a \
                         supported heap op (alloc, field_load, field_store, is_null, \
                         const, ret, load_reg, store_reg, jmp_if_true, jmp_if_false, mov)",
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
            // A floating-point constant is loaded with `ldc.r4`/`ldc.r8`. The
            // **textual `.il` emitter** ([`crate::il_text`]) supports `f32`/`f64`
            // (LANG-FULL E3 — ALGOL `real`), so a float const with a float
            // `type_hint` is accepted. A float const with a *non-float* type_hint
            // is still a bug (it would silently truncate), and is rejected.
            //
            // (The structured *bytecode* emitter [`crate::lower`] does not yet
            // emit `ldc.r8` and keeps its own guard; the real-CLR matrix path
            // uses the textual emitter, which this check unblocks.)
            if instr.op == "const" {
                if let Some(Operand::Float(_)) = instr.srcs.first() {
                    if instr.type_hint != "f64" && instr.type_hint != "f32" {
                        errors.push(format!(
                            "UnsupportedType: function {:?}, const has a Float operand but a \
                             non-float type_hint {:?} (would truncate)",
                            func.name, instr.type_hint
                        ));
                    }
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
            //
            // `call_builtin` is conditionally accepted: the builtin name
            // carried in `srcs[0]` as `Operand::Var` must be in
            // [`CALL_BUILTIN_SUPPORTED_NAMES`].  This lets Brainfuck's
            // `putchar` / `getchar` flow through while still rejecting
            // unknown / unsafe builtins.
            if instr.op == "str_len" {
                match (instr.dest.as_ref(), instr.srcs.as_slice(), instr.type_hint.as_str()) {
                    (Some(_), [Operand::Var(_)], "i64" | "i32") => {
                        // Accepted — il_text.rs calls System.String::get_Length().
                    }
                    _ => {
                        errors.push(format!(
                            "UnsupportedOp: function {:?}, op \"str_len\" requires \
                             dest, one Operand::Var source, and i64/i32 result type",
                            func.name
                        ));
                    }
                }
            } else if instr.op == "str_concat" {
                match (instr.dest.as_ref(), instr.srcs.as_slice(), instr.type_hint.as_str()) {
                    (Some(_), [Operand::Var(_), Operand::Var(_)], "str") => {
                        // Accepted — il_text.rs calls System.String::Concat(string,string).
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
                    (Some(_), [Operand::Var(_), Operand::Var(_), Operand::Var(_)], "str") => {
                        // Accepted — il_text.rs calls System.String::Substring(int32,int32).
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
                    (Some(_), [Operand::Var(_), Operand::Var(_)], "i64" | "i32") => {
                        // Accepted — il_text.rs calls System.String::get_Chars(int32).
                    }
                    _ => {
                        errors.push(format!(
                            "UnsupportedOp: function {:?}, op \"str_index\" requires \
                             dest, string Operand::Var, index Operand::Var, and i64/i32 result type",
                            func.name
                        ));
                    }
                }
            } else if instr.op == "str_eq" {
                match (instr.dest.as_ref(), instr.srcs.as_slice(), instr.type_hint.as_str()) {
                    (Some(_), [Operand::Var(_), Operand::Var(_)], "i64" | "i32") => {
                        // Accepted — il_text.rs calls System.String::Equals(string,string).
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
                    (Some(_), [Operand::Var(_), Operand::Var(_)], "i64" | "i32") => {
                        // Accepted — il_text.rs calls String.CompareOrdinal and Math.Sign.
                    }
                    _ => {
                        errors.push(format!(
                            "UnsupportedOp: function {:?}, op \"str_cmp\" requires \
                             dest, two Operand::Var sources, and i64/i32 result type",
                            func.name
                        ));
                    }
                }
            } else if UNSUPPORTED_OPS.contains(&instr.op.as_str()) {
                errors.push(format!(
                    "UnsupportedOp: function {:?}, op {:?} is not supported by \
                     the CLR backend; it requires a P/Invoke or .NET BCL call",
                    func.name, instr.op
                ));
            } else if instr.op == "call_builtin" {
                // Inspect srcs[0] for the builtin name.  IIR carries it as
                // `Operand::Var(name)` (Rust's IIR has no separate string
                // operand kind for builtin lookup — names share the Var variant).
                let name: Option<&str> = match instr.srcs.first() {
                    Some(Operand::Var(s)) => Some(s.as_str()),
                    _ => None,
                };
                match name {
                    Some(n) if CALL_BUILTIN_SUPPORTED_NAMES.contains(&n) => {
                        // Accepted — lower.rs emits the corresponding
                        // `call <token>` to env.BFRuntime::<name>.
                    }
                    _ => {
                        errors.push(format!(
                            "UnsupportedOp: function {:?}, op \"call_builtin\" with \
                             builtin name {:?} is not in the CLR backend's host-class \
                             whitelist (supported: {:?}); add the builtin to \
                             CALL_BUILTIN_SUPPORTED_NAMES and the lowering rule in \
                             lower.rs to extend coverage",
                            func.name, name, CALL_BUILTIN_SUPPORTED_NAMES
                        ));
                    }
                }
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

    /// An `f64` float const is now ACCEPTED (LANG-FULL E3 — the textual `.il`
    /// emitter lowers it to `ldc.r8`). (Was `float_const_rejected`.)
    #[test]
    fn f64_float_const_accepted() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Float(3.14)], "f64"),
        ]));
        assert!(!errs.iter().any(|e| e.contains("Float")),
            "an f64 float const should be accepted; got: {errs:?}");
    }

    /// A Float operand with a *non-float* type_hint is still rejected (it would
    /// silently truncate).
    #[test]
    fn float_const_with_int_hint_rejected() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Float(3.14)], "i32"),
        ]));
        assert!(errs.iter().any(|e| e.contains("Float")),
            "a Float with an int type_hint should be rejected; got: {errs:?}");
    }

    #[test]
    fn str_type_rejected() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("ret_void", None, vec![], "str"),
        ]));
        assert!(errs.iter().any(|e| e.contains("UnsupportedType")));
    }

    #[test]
    fn str_const_literal_accepted() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new(
                "str_const",
                Some("s".into()),
                vec![Operand::Str("HELLO".into())],
                "str",
            ),
            IIRInstr::new("print_str", None, vec![Operand::Var("s".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(errs.is_empty(), "str_const + print_str should pass: {:?}", errs);
    }

    #[test]
    fn str_len_literal_accepted() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new(
                "str_const",
                Some("s".into()),
                vec![Operand::Str("HELLO".into())],
                "str",
            ),
            IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("s".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
        ]));
        assert!(
            errs.is_empty(),
            "str_len over a direct literal should pass: {:?}",
            errs
        );
    }

    #[test]
    fn str_eq_literal_accepted() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new(
                "str_const",
                Some("a".into()),
                vec![Operand::Str("HELLO".into())],
                "str",
            ),
            IIRInstr::new(
                "str_const",
                Some("b".into()),
                vec![Operand::Str("HELLO".into())],
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
            "str_eq over direct literals should pass: {:?}",
            errs
        );
    }

    #[test]
    fn str_cmp_literal_accepted() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
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
            "str_cmp over direct literals should pass: {:?}",
            errs
        );
    }

    #[test]
    fn str_concat_literal_accepted() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
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
            "str_concat over direct literals should pass: {:?}",
            errs
        );
    }

    #[test]
    fn str_slice_literal_accepted() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
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
            IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("sub".into())], "i64"),
            IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
        ]));
        assert!(
            errs.is_empty(),
            "str_slice over direct literals should pass: {:?}",
            errs
        );
    }

    #[test]
    fn str_index_literal_accepted() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
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
            "str_index over a direct literal should pass: {:?}",
            errs
        );
    }

    #[test]
    fn byte_string_algebra_still_rejected() {
        {
            let op = "str_index";
            let errs = validate_iir_for_clr(&single_fn_module(vec![
                IIRInstr::new(
                    op,
                    Some("v".into()),
                    vec![Operand::Var("s".into())],
                    "i32",
                ),
                IIRInstr::new("ret_void", None, vec![], "void"),
            ]));
            assert!(
                errs.iter().any(|e| e.contains("UnsupportedOp")),
                "{op} should require the literal-index shape: {:?}",
                errs
            );
        }
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

    // ─── BF lowering: load_mem / store_mem now pass ─────────────────────────

    #[test]
    fn load_mem_accepted_for_bf() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("load_mem", Some("v".into()),
                vec![Operand::Var("ptr".into())], "u8"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(
            errs.iter().all(|e| !e.contains("UnsupportedOp")),
            "load_mem should be accepted by CLR validator after BF→CLR PR; got: {:?}",
            errs
        );
    }

    #[test]
    fn store_mem_accepted_for_bf() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("store_mem", None::<String>,
                vec![Operand::Var("ptr".into()), Operand::Var("v".into())], "u8"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(
            errs.iter().all(|e| !e.contains("UnsupportedOp")),
            "store_mem should be accepted by CLR validator; got: {:?}",
            errs
        );
    }

    // ─── BF lowering: call_builtin whitelist (putchar / getchar) ─────────────

    #[test]
    fn call_builtin_putchar_accepted() {
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("call_builtin", None::<String>,
                vec![Operand::Var("putchar".into()), Operand::Var("v".into())],
                "void"),
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
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("call_builtin", Some("v".into()),
                vec![Operand::Var("getchar".into())], "u8"),
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
        let errs = validate_iir_for_clr(&single_fn_module(vec![
            IIRInstr::new("call_builtin", None::<String>,
                vec![Operand::Var("system_exec".into())], "void"),
        ]));
        assert!(
            errs.iter().any(|e| e.contains("UnsupportedOp")
                && e.contains("system_exec")),
            "unknown call_builtin name should be rejected with surfaced \
             whitelist; got: {:?}",
            errs
        );
    }
}
