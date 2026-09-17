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
//! | `UntypedInstruction` | `type_hint` is `"any"` or `"polymorphic"` — except for `call_closure` |
//! | `UnsupportedType` | `type_hint` is `"str"` or starts with `"ref<"` — see exceptions |
//! | `UnsupportedType` (float const) | `op == "const"`, src is `Operand::Float`, and `type_hint != "f64"` |
//! | `UnsupportedOp` | op is a runtime/memory/IO/GC opcode (list below) |
//!
//! Unsupported ops: `call_builtin`, `io_in`, `io_out`, `cast`, `load_mem`,
//! `store_mem`, `box`, `unbox`, `safepoint`.
//!
//! **LANG35 closure opcodes accepted by this backend:**
//!
//! | Op | type_hint | Lowering |
//! |----|-----------|---------|
//! | `alloc_closure` | `"closure"` | `put_list` cons-cell chain + fn-name atom |
//! | `call_closure`  | `"any"` | `get_list` + `call_ext erlang:'++'` + `call_ext erlang:apply/3` |
//!
//! `call_closure` is the only opcode permitted with a `"any"` type hint: its
//! return type is necessarily unknown at compile time (Twig is dynamically
//! typed), and the BEAM runtime enforces type safety at execution time.
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
//! | `mov` | any `ref<…>` type_hint (VM-041 — a plain register copy of a heap pointer, same as the `"str"` exception) |
//!
//! These ops are lowered to BEAM instructions by `lower.rs`:
//! - `alloc` + adjacent `field_store`s → `put_list`
//! - `field_load` → `get_list`
//! - `is_null` → synthesized `is_nil` + boolean synthesis
//!
//! The `ref<LispyPair>` type on `alloc` is also accepted so that `const`
//! instructions representing nil (`const 0 : ref<LispyPair>`) can pass
//! through to the lowering pass which converts them to a BEAM `[]` atom move.

// The float literals in this module (e.g. 3.14...) are hand-written test/demo
// values, not attempts to approximate `std::f64::consts::PI`. This is a `mod`
// file, so the inner attribute applies to this module only.
#![allow(clippy::approx_constant)]

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
//
// LANG35 — supported in BEAM backend:
// - `alloc_closure` — lowered to put_list cons-cell chain (fn atom + captures).
// - `call_closure`  — lowered to get_list + call_ext erlang:'++'/2 +
//                     call_ext erlang:apply/3.

/// McCarthy W10: the `call_builtin` names the BEAM backend lowers (the predicate
/// set → `is_nonempty_list`/`is_eq_exact` 0/1 synthesis). `call_builtin` with any
/// OTHER name is rejected by the validator (Check 6b) so that validation stays in
/// sync with the lowering arm — `IIRBeamCodeGenerator::generate` assumes a
/// validated module never reaches an unsupported op.
const BEAM_PREDICATE_BUILTINS: &[&str] = &["pair?", "equal?", "not"];

const UNSUPPORTED_OPS: &[&str] = &[
    // "call_builtin"  — McCarthy W10: conditionally supported (see Check 6b /
    //   BEAM_PREDICATE_BUILTINS). Handled below, not via this blanket list.
    "io_in",
    // "io_out"       — LANG32: now supported (erlang:display/1).
    // "global_store" — LANG32: now supported (erlang:put/2).
    // "global_load"  — LANG32: now supported (erlang:get/1).
    "cast",
    "load_mem",
    "store_mem",
    // "alloc"         — accepted: lowered to put_list (via alloc+field_store pattern)
    "box",
    "unbox",
    // "field_load"    — accepted: lowered to get_list
    // "field_store"   — accepted: lowered as part of the put_list pattern
    // "is_null"       — accepted: lowered to is_nil synthesis
    "safepoint",
    // "alloc_closure" — LANG35: accepted; lowered to put_list cons-cell
    // "call_closure"  — LANG35: accepted; lowered to get_list + call_ext apply
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
///    arithmetic in this backend, except the `str_const`/`str_concat`/
///    `str_slice`/`call`/`ret`/`mov`/`array_set`/`array_get` ops this
///    backend does lower — the last two are BEAM06's `str`-typed array
///    element read/write, reusing BEAM04's `:ets` substrate unmodified) or
///    start with `"ref<"` (heap pointers have no BEAM equivalent in this
///    lowering).
///
/// 5. **UnsupportedType for float const** — `op == "const"` with an
///    `Operand::Float` source and `type_hint != "f64"` is rejected. BEAM03
///    added `"f64"` float-const support (lowered via the module literal
///    table, see `lower.rs`); any other type_hint on a float const is still
///    rejected as unsupported.
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

    // ── Check 1.5: Atom length (trust-boundary guard) ────────────────────────
    //
    // BEAM atoms are limited to 255 bytes (UTF-8).  The module name, all
    // function names, and all `Operand::Str` string literals become BEAM
    // atoms during lowering.  Catch oversized names here so the encoder
    // never reaches the `assert!` guard (which would panic in debug builds).
    //
    // This is the sole input-validation point for atom length; it runs on
    // all user-supplied IIR before any encoding takes place.
    const BEAM_ATOM_MAX: usize = 255;

    if module.name.len() > BEAM_ATOM_MAX {
        errors.push(format!(
            "AtomTooLong: module name is {} bytes (max {})",
            module.name.len(), BEAM_ATOM_MAX
        ));
    }

    for func in &module.functions {
        if func.name.len() > BEAM_ATOM_MAX {
            errors.push(format!(
                "AtomTooLong: function name {:?} is {} bytes (max {})",
                func.name, func.name.len(), BEAM_ATOM_MAX
            ));
        }
        for instr in &func.instructions {
            if instr.op == "str_const" {
                continue;
            }
            if let Some(Operand::Str(s)) = instr.srcs.first() {
                if s.len() > BEAM_ATOM_MAX {
                    errors.push(format!(
                        "AtomTooLong: Operand::Str in function {:?}, op {:?} is {} bytes (max {})",
                        func.name, instr.op, s.len(), BEAM_ATOM_MAX
                    ));
                }
            }
        }
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
            // ── Check 2.5: ClosureOpcode early-accept (LANG35) ───────────────
            //
            // `alloc_closure` (type_hint "closure") and `call_closure`
            // (type_hint "any") are fully supported by this backend.  Skip
            // all downstream type and op checks for these two opcodes so they
            // don't fall into UntypedInstruction or UnsupportedOp.
            //
            // `alloc_closure` carries the function name as Operand::Str in
            // srcs[0] and captures in srcs[1..].  It produces a cons-cell
            // `[fn_atom | caps_list]` using put_list.
            //
            // `call_closure` always has type_hint "any" because Twig is
            // dynamically typed; the return type is unknown at compile time.
            // BEAM enforces types at runtime via the normal exception path.
            if matches!(instr.op.as_str(), "alloc_closure" | "call_closure") {
                continue;
            }

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
            //
            // EXCEPTION: `call_closure` (handled above by the early-accept
            // continue) is always "any" and is valid.
            if instr.type_hint == "any" || instr.type_hint == "polymorphic" {
                errors.push(format!(
                    "UntypedInstruction: function {:?}, op {:?} has type_hint {:?}; \
                     BEAM lowering requires concrete types",
                    func.name, instr.op, instr.type_hint
                ));
            }

            // ── Check 3.5: LANG35 "closure" type_hint accepted ───────────────
            //
            // `alloc_closure` uses type_hint "closure" (from interpreter_ir's
            // CLOSURE_TYPE constant).  This type is not in CONCRETE_TYPES but
            // is valid here.  The early-accept continue above already skips
            // the UntypedInstruction check; this comment is a note for any
            // future validator that scans for unknown type_hint values.

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
            // BEAM06: `array_set`/`array_get` with `type_hint == "str"` are
            // a `str`-typed array element read/write (Dartmouth BASIC's
            // `array<str>` — `DIM A$(n)` — and its mixed numeric/string
            // `DATA` pool's string pool). These dispatch to the exact same
            // `:ets` substrate BEAM04 built for `array<f64>` (see the
            // module-setup comment in `lower.rs`): a `str` value is already
            // an ordinary Erlang character list (the `str_const` scalar
            // representation), and `:ets` holds arbitrary terms natively,
            // so no new representation is needed — confirmed on real `erl`,
            // see `code/specs/BEAM06-string-array-representation.md`.
            // BEAM07: `call_builtin "input_str"` has type_hint "str" — BASIC
            // string `INPUT A$` reads a whole line as a runtime string, the
            // same "str" type_hint every other string-producing op here
            // carries. Check 6b below further restricts `call_builtin` to
            // its known builtin-name set; this check only decides whether
            // the OP is allowed to carry a "str" type_hint at all.
            if instr.type_hint == "str"
                && !matches!(
                    instr.op.as_str(),
                    "str_const" | "str_concat" | "str_slice" | "call" | "ret" | "mov"
                        | "array_set" | "array_get" | "call_builtin"
                )
            {
                errors.push(format!(
                    "UnsupportedType: function {:?}, op {:?} has type_hint \"str\"; \
                     only the ASCII string subset is supported in this BEAM backend",
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

                    // mov ref<…> — a plain register-to-register copy of a heap
                    // pointer. VM-041: Twig's `match`/`if` compile a "phi via
                    // mutable variable" pattern — each arm's `emit_move`
                    // (`twig-ir-compiler::compiler.rs`) writes into a shared
                    // result variable with a typed `mov`, using the SOURCE
                    // value's own inferred type as the mov's type_hint. When a
                    // `union` variant constructor's cons cell (or a value that
                    // started life as `box`/`unbox`, which
                    // `concretize_scalar_any_for_beam` renames to `mov` without
                    // touching its type_hint — see `lang-aot::lib.rs`) is one of
                    // the arms, that type_hint is `ref<LispyPair>`, not a
                    // scalar. `lower.rs`'s `"mov"` arm already lowers this
                    // correctly for ANY type_hint (it is an unconditional
                    // `{operand} -> {x,rd}` BEAM `move`, agnostic to what the
                    // register holds — see the `integer_operand!` macro, which
                    // resolves a `Var` source to its x-register regardless of
                    // type); this validator was simply never told to accept the
                    // ref-typed case, even though `"mov"` was already accepted
                    // for the analogous `"str"` type_hint case above. Confirmed
                    // safe against real `erl`:
                    // `test_99_real_erl_mov_ref_lispy_pair_lowers_correctly`.
                    "mov" => true,

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
            // BEAM03: a `const` with an `Operand::Float` source is accepted
            // when `type_hint == "f64"` — it lowers to a `move` referencing
            // the module's literal table (see `lower.rs`'s `LiteralPool` and
            // `ir_to_beam::literal_operand`). Any OTHER type_hint on a float
            // const (e.g. a stray "f32", which no current frontend emits) is
            // still rejected: this backend only implements the one float
            // width every frontend actually uses.
            if instr.op == "const" {
                if let Some(Operand::Float(_)) = instr.srcs.first() {
                    if instr.type_hint != "f64" {
                        errors.push(format!(
                            "UnsupportedType: function {:?}, const instruction has a Float \
                             operand with type_hint {:?}; only \"f64\" float constants are \
                             supported in this BEAM backend",
                            func.name, instr.type_hint
                        ));
                    }
                }
            }

            // ALGOL runtime strings are Erlang character lists. Restrict the
            // source literal to printable ASCII plus familiar whitespace so
            // lowering never needs an unvalidated binary or Unicode codec.
            match instr.op.as_str() {
                "str_const" => match (instr.dest.as_ref(), instr.srcs.as_slice()) {
                    (Some(_), [Operand::Str(text)])
                        if text.bytes().all(|byte| {
                            matches!(byte, b'\n' | b'\r' | b'\t' | b' '..=b'~')
                        }) => {}
                    _ => errors.push(format!(
                        "InvalidString: function {:?}, str_const requires a destination and printable ASCII text",
                        func.name
                    )),
                },
                "str_concat"
                    if instr.dest.is_none()
                        || !matches!(instr.srcs.as_slice(), [Operand::Var(_), Operand::Var(_)])
                        || instr.type_hint != "str" =>
                {
                    errors.push(format!(
                        "InvalidString: function {:?}, str_concat requires dest and two string variables",
                        func.name
                    ));
                }
                "str_slice"
                    if instr.dest.is_none()
                        || !matches!(
                            instr.srcs.as_slice(),
                            [Operand::Var(_), Operand::Var(_), Operand::Var(_)]
                        )
                        || instr.type_hint != "str" =>
                {
                    errors.push(format!(
                        "InvalidString: function {:?}, str_slice requires dest, a string variable, and two integer bound variables",
                        func.name
                    ));
                }
                "str_len"
                    if instr.dest.is_none()
                        || !matches!(instr.srcs.as_slice(), [Operand::Var(_)])
                        || !matches!(instr.type_hint.as_str(), "i32" | "i64") =>
                {
                    errors.push(format!(
                        "InvalidString: function {:?}, str_len requires dest, one string variable, and an integer result",
                        func.name
                    ));
                }
                "str_index"
                    if instr.dest.is_none()
                        || !matches!(instr.srcs.as_slice(), [Operand::Var(_), Operand::Var(_)])
                        || !matches!(instr.type_hint.as_str(), "i32" | "i64") =>
                {
                    errors.push(format!(
                        "InvalidString: function {:?}, str_index requires dest, a string variable, an integer index variable, and an integer result",
                        func.name
                    ));
                }
                "str_eq"
                    if instr.dest.is_none()
                        || !matches!(instr.srcs.as_slice(), [Operand::Var(_), Operand::Var(_)])
                        || !matches!(instr.type_hint.as_str(), "i32" | "i64") =>
                {
                    errors.push(format!(
                        "InvalidString: function {:?}, str_eq requires dest, two string variables, and an integer result",
                        func.name
                    ));
                }
                "str_cmp"
                    if instr.dest.is_none()
                        || !matches!(instr.srcs.as_slice(), [Operand::Var(_), Operand::Var(_)])
                        || !matches!(instr.type_hint.as_str(), "i32" | "i64") =>
                {
                    errors.push(format!(
                        "InvalidString: function {:?}, str_cmp requires dest, two string variables, and an integer result",
                        func.name
                    ));
                }
                "print_str"
                    if !matches!(instr.srcs.as_slice(), [Operand::Var(_)])
                        || instr.type_hint != "void" =>
                {
                    errors.push(format!(
                        "InvalidString: function {:?}, print_str requires one string variable and void type",
                        func.name
                    ));
                }
                _ => {}
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

            // ── Check 6b: call_builtin — predicates and integer output ───
            //
            // `call_builtin` supports the McCarthy predicates, print_i64,
            // putchar, and (BEAM07) the host-input builtins `input_more`/
            // `input_i64`/`input_str` (BASIC `INPUT` and FlowMatic
            // `READ-ITEM`'s EOF peek — see `lower.rs`'s "BEAM07: host-input
            // atoms and imports" comment); any other builtin name has no
            // BEAM lowering. We reject it here (not just at lowering) so
            // that a validated module is always lowerable — `generate()`
            // panics on a lowering error, assuming validation already
            // screened the module.
            if instr.op == "call_builtin" {
                let supported = matches!(
                    instr.srcs.first(),
                    Some(Operand::Var(n)) if BEAM_PREDICATE_BUILTINS.contains(&n.as_str())
                        || matches!(n.as_str(), "print_i64" | "putchar"
                            | "input_more" | "input_i64" | "input_str")
                );
                if !supported {
                    errors.push(format!(
                        "UnsupportedOp: function {:?}, call_builtin {:?} is not in the \
                         BEAM builtin set (pair?/equal?/not/print_i64/putchar/\
                         input_more/input_i64/input_str)",
                        func.name, instr.srcs.first()
                    ));
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

    /// BEAM03: an `f64` float const is now accepted (previously rejected
    /// unconditionally) — it lowers to a literal-table reference.
    #[test]
    fn float_const_f64_accepted() {
        let errs = validate_for_beam(&single_fn_module(vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Float(3.14)], "f64"),
            IIRInstr::new("ret", None, vec![Operand::Var("v".into())], "f64"),
        ]));
        assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    }

    /// A float const with any type_hint other than "f64" (e.g. a stray
    /// "f32", which no current frontend emits) is still rejected — this
    /// backend only implements the one float width every frontend uses.
    #[test]
    fn float_const_non_f64_type_hint_rejected() {
        let errs = validate_for_beam(&single_fn_module(vec![
            IIRInstr::new("const", Some("v".into()), vec![Operand::Float(3.14)], "f32"),
        ]));
        assert!(errs.iter().any(|e| e.contains("Float") && e.contains("f32")));
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

    #[test]
    fn ascii_string_subset_passes_validation() {
        let errs = validate_for_beam(&single_fn_module(vec![
            IIRInstr::new("str_const", Some("a".into()), vec![Operand::Str("HE".into())], "str"),
            IIRInstr::new("str_const", Some("b".into()), vec![Operand::Str("LLO".into())], "str"),
            IIRInstr::new(
                "str_concat",
                Some("word".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "str",
            ),
            IIRInstr::new(
                "str_eq",
                Some("same".into()),
                vec![Operand::Var("word".into()), Operand::Var("word".into())],
                "i64",
            ),
            IIRInstr::new(
                "str_cmp",
                Some("ordering".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i64",
            ),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("const", Some("two".into()), vec![Operand::Int(2)], "i64"),
            IIRInstr::new(
                "str_slice",
                Some("head".into()),
                vec![Operand::Var("word".into()), Operand::Var("z".into()), Operand::Var("two".into())],
                "str",
            ),
            IIRInstr::new("print_str", None, vec![Operand::Var("word".into())], "void"),
            IIRInstr::new("print_str", None, vec![Operand::Var("head".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    }

    #[test]
    fn str_slice_wrong_arity_is_rejected() {
        // str_slice with only one bound (missing the end index) must be a
        // clear InvalidString error, not an UnsupportedType — VM-040's COBOL
        // signed/algebra probe added str_slice support; this pins its shape
        // check down the same way str_concat/str_eq/str_cmp already are.
        let errs = validate_for_beam(&single_fn_module(vec![
            IIRInstr::new("str_const", Some("a".into()), vec![Operand::Str("HE".into())], "str"),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new(
                "str_slice",
                Some("head".into()),
                vec![Operand::Var("a".into()), Operand::Var("z".into())],
                "str",
            ),
            IIRInstr::new("print_str", None, vec![Operand::Var("head".into())], "void"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ]));
        assert!(
            errs.iter().any(|e| e.contains("InvalidString") && e.contains("str_slice")),
            "expected an InvalidString error naming str_slice; got: {errs:?}"
        );
    }

    #[test]
    fn str_len_and_str_index_are_accepted() {
        // VM-040's COBOL BEAM STRING SIZE/delimiter slice added str_len/
        // str_index support (COBOL's STRING ... DELIMITED BY delim and
        // UNSTRING both scan a sending field character-by-character via
        // `str_len`/`str_index`/`str_slice`). Both must validate cleanly with
        // their documented shapes.
        let errs = validate_for_beam(&single_fn_module(vec![
            IIRInstr::new("str_const", Some("a".into()), vec![Operand::Str("HE".into())], "str"),
            IIRInstr::new("str_len", Some("n".into()), vec![Operand::Var("a".into())], "i64"),
            IIRInstr::new("const", Some("z".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new(
                "str_index",
                Some("c".into()),
                vec![Operand::Var("a".into()), Operand::Var("z".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
        ]));
        assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    }

    #[test]
    fn str_len_wrong_arity_is_rejected() {
        // str_len with an extra source operand must be a clear InvalidString
        // error, matching str_slice_wrong_arity_is_rejected's discipline.
        let errs = validate_for_beam(&single_fn_module(vec![
            IIRInstr::new("str_const", Some("a".into()), vec![Operand::Str("HE".into())], "str"),
            IIRInstr::new("str_const", Some("b".into()), vec![Operand::Str("X".into())], "str"),
            IIRInstr::new(
                "str_len",
                Some("n".into()),
                vec![Operand::Var("a".into()), Operand::Var("b".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("n".into())], "i64"),
        ]));
        assert!(
            errs.iter().any(|e| e.contains("InvalidString") && e.contains("str_len")),
            "expected an InvalidString error naming str_len; got: {errs:?}"
        );
    }

    #[test]
    fn str_index_wrong_arity_is_rejected() {
        // str_index with only the string operand (missing the index) must be
        // a clear InvalidString error, matching str_slice_wrong_arity_is_rejected.
        let errs = validate_for_beam(&single_fn_module(vec![
            IIRInstr::new("str_const", Some("a".into()), vec![Operand::Str("HE".into())], "str"),
            IIRInstr::new(
                "str_index",
                Some("c".into()),
                vec![Operand::Var("a".into())],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("c".into())], "i64"),
        ]));
        assert!(
            errs.iter().any(|e| e.contains("InvalidString") && e.contains("str_index")),
            "expected an InvalidString error naming str_index; got: {errs:?}"
        );
    }
}
