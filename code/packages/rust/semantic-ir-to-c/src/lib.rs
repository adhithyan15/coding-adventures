//! # semantic-ir-to-c
//!
//! Sixth backend for the narrow-waist Semantic IR — emits **self-contained**
//! ISO C99 source from a [`semantic_ir::Module`].
//!
//! Output is a single `.c` file with the runtime inlined; no external library
//! beyond the C standard library.  It compiles with any C99 compiler
//! (`cc <file>.c -o <file>`) on MSVC (`/std:c11`), GCC, and Clang, and runs.
//!
//! Because every SIR frontend lowers to the same waist, this one backend gives
//! **Ruby → C** (the driving goal) and Python/JS/Twig → C for free.
//!
//! Implements [SIR24](../../../specs/SIR24-semantic-ir-to-c.md).  This is the
//! **v0 core**; later feature batches (floats, loops, sequences, maps,
//! params, collection methods, exceptions, OOP) land incrementally via the
//! same cascade the Go backend followed.

mod emit;
mod runtime;

use semantic_ir::{
    Artifact, ArtifactMetadata, Backend, BackendError, BackendErrorKind, Feature, Module,
};

pub use emit::sanitize_ident;

/// Compile a module to a C artifact (convenience wrapper over [`CBackend`]).
pub fn compile(module: &Module) -> Result<Artifact, BackendError> {
    CBackend::new().compile(module)
}

/// The C backend.
pub struct CBackend;

impl CBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// The SIR-v0 feature set.  Later batches extend this in lockstep with the
/// emitter and runtime; every accepted feature has a real (non-panicking)
/// emit path, and every not-yet-implemented node is unreachable because its
/// feature stays unaccepted.
const ACCEPTED_FEATURES: &[Feature] = &[
    Feature::Closures,
    Feature::Pairs,
    Feature::Symbols,
    Feature::Strings,
    Feature::DynamicTyping,
    Feature::OptionalTypeAnnotations,
    Feature::MutualRecursion,
    Feature::Globals,
    // ── SIR26 integer conversions ────────────────────────────────────
    // `Expr::Convert` renders as the portable `_sir_convert(v, bits, signed)`
    // runtime helper (two's-complement reduction over int64/uint64).  A
    // Convert's target type also makes the validator observe these SIR21
    // type-implied features, so the C backend must accept them.
    Feature::Conversions,
    Feature::SizedIntegers,
    Feature::Unsigned,
    Feature::WrappingArithmetic,
    // ── SIR16 floats ─────────────────────────────────────────────────
    // `SirValue` has carried a `SIR_FLOAT` tag since v0 (`_sir_float`, the
    // arithmetic helpers' int→float promotion, `_sir_divide_v`'s IEEE float
    // path, `_sir_fmt_float`), so this batch is purely an EMITTER addition:
    // `Expr::FloatLit` → `_sir_float(<literal>)` (via `emit_float_literal`,
    // which spells a finite value as a C `double` literal and a non-finite one
    // with the `<math.h>` `INFINITY`/`NAN` macros). `Floats` gates ONLY
    // `FloatLit`; float `+`/`-`/`*`/`/` reuse the existing variadic helpers, so
    // accepting the feature plus the one emit arm keeps the emitter total.
    Feature::Floats,
    // ── SIR16 short-circuit ──────────────────────────────────────────
    // `Expr::LogicalAnd` / `Expr::LogicalOr` (`&&` / `||`) — the same lowering
    // the emitter already uses for the eager `and`/`or` builtins: assign the
    // left operand into `dst`, then conditionally OVERWRITE with the right, so
    // the right is not evaluated when the left decides (true short-circuit) and
    // `dst` holds the DECIDING OPERAND (not a bool). Because the nodes are not
    // `is_simple`, they route through `emit_assign` (and, in tail position,
    // through the compute-into-a-temp fallback), so no other arm is needed.
    Feature::ShortCircuit,
    // ── SIR16 control flow / mutation ────────────────────────────────
    // `Stmt::While` renders as a portable `for (;;) { … if (!truthy) break; }`
    // (the condition is re-evaluated each iteration, so it may be compound);
    // `Stmt::Assign` re-binds an already-declared `SirValue`.  Both are needed by
    // the C frontend's milestone-2 `if`/`while`/`for` lowering.
    Feature::Loops,
    Feature::MutableBindings,
    // ── SIR16 sequences ──────────────────────────────────────────────
    // `SirValue` gains a `SIR_SEQ` heap-boxed dynamic array. The emitter
    // handles every construct the feature can surface: `SeqLit` (`[1, 2, 3]`),
    // `SeqIndex` (`a[i]`, nil-on-OOB), `SeqLen` (`a.length`), `SeqSet`
    // (`a[i] = v`, traps out-of-range), and `ForEach` (`for x in a`, over a
    // `_sir_seq_iter` snapshot — so it is no longer rejected by `first_foreach`
    // as it was in 0.5.0). Structural `_sir_value_eq` makes `[1, 2] == [1, 2]`
    // true, matching every backend that carries sequences.
    Feature::Sequences,
    // ── SIR16 maps ───────────────────────────────────────────────────
    // `SirValue` gains a `SIR_MAP` heap-boxed, insertion-ordered ASSOC-ARRAY
    // (linear scan, structural keys — like the Go/Rust `[]MapEntry` /
    // `Vec<(Value, Value)>` reference, not a hash table). The emitter handles
    // every construct the feature can surface: `MapLit` (`{k => v}`,
    // `_sir_map_lit`), `MapGet` (`h[k]`, nil-on-miss, `_sir_map_get`), and
    // `MapSet` (`h[k] = v`, insert/update the shared box, `_sir_map_set`). No
    // `MapLen` node exists. `value_eq`/`fmt` gain `SIR_MAP` arms (positional
    // structural equality and `{k: v}` display, matching Go/Rust), reusing the
    // `SeqSet`-era depth caps to bound a cyclic map (`m[k] = m`, now
    // constructible via the mutable `MapSet`). `ForEach` over a map is NOT
    // special-cased — iterating a map is reference-undefined (Go's
    // `_sir_seq_iter` panics on it); C's lenient `_sir_seq_iter` else-branch
    // (empty iteration) already covers it without an emitter `unreachable!`.
    Feature::Maps,
    // ── SIR19 default parameters ─────────────────────────────────────
    // A positional parameter carrying a default value.  C has no native
    // defaults, so — like the Go backend — this uses a `_sir_missing` sentinel:
    // a `DirectCall` that omits trailing defaulted arguments pads the call with
    // `_sir_missing()` (call-site padding, keyed off a thread-local arity map),
    // and each function opens with a prologue `if (_sir_is_missing(p)) { p =
    // <default>; }` in declaration order (so a later default may reference an
    // earlier parameter).  The unsupported-builtin pre-check also scans each
    // default, so the emitter stays total.
    Feature::DefaultParams,
    // ── SIR19 keyword parameters ─────────────────────────────────────
    // A keyword parameter (`def f(x:)`) and keyword argument (`f(x: 5)`).  C has
    // no native keywords, so — like the Go backend's KW6 — a `KeywordArg` is
    // resolved to its callee's parameter SLOT BY NAME at emit time (using the
    // thread-local signature map's parameter names), producing a plain
    // positional C call; an omitted optional keyword is filled with
    // `_sir_missing()` and substituted by the same default prologue as
    // `DefaultParams`.  A keyword parameter needs NO special signature — it is a
    // positional `SirValue` C parameter like any other; only the call site
    // resolves by name.
    Feature::KeywordParams,
    // ── SIR17 exceptions ─────────────────────────────────────────────
    // `Stmt::TryCatch` (`begin/rescue/ensure`) + the `raise` builtin lower to a
    // `setjmp`/`longjmp` handler stack (a new `SIR_ERROR` runtime value, a
    // baked-in exception-class ancestry table for `rescue`-by-class matching,
    // and a two-handler structure so `ensure` runs even when a rescue body
    // raises).  Rescue-type names are emitted as QUOTED string literals, so no
    // rescue type can inject source.  `raise SomeClass` (a specific class) is a
    // `Const` reference → observes `Feature::Constants` (unaccepted) → rejected;
    // `retry` is not yet lowered (rejected by the builtin gate) — both deferred.
    Feature::Exceptions,
    // ── OOP mirror, slice 1: instance runtime + empty class + constants ──
    // Mirrors the Ruby backend's OOP slice 1.  `Feature::Classes` +
    // `Feature::Constants` are ENTANGLED: the frontend records `Constants` for
    // any `Foo.new` (the receiver is a constant), so an instantiable class needs
    // both.  This slice accepts:
    //   `class Foo; end`  → `Stmt::ClassDef` → a comment (a class is just a NAME
    //                        in the C runtime — an instance carries its class
    //                        string; there is no class object).
    //   `Foo.new`         → `BuiltinCall("__new__", [StrLit("Foo")])` →
    //                        `_sir_new_instance("Foo")` (a `SIR_INSTANCE` box).
    //   `PI = 3` / `PI`   → a `Scope::Const` `Assign`/`VarRef` → a tiny runtime
    //                        constant table (`_sir_const_set`/`_sir_const_get`).
    // Names are emitted as QUOTED C string literals (no injection, as with rescue
    // types).  Deferred to later slices (rejected cleanly): `__new__` with
    // constructor arguments (needs `initialize`), a `class << self` singleton, the
    // OOP method builtins (`__def_method__`/`__method__`/…), and — via their own
    // unaccepted features — `@ivars`/`@@cvars`/inheritance-dispatch/modules.
    Feature::Classes,
    Feature::Constants,
];

impl Backend for CBackend {
    fn target_tag(&self) -> &'static str {
        "c"
    }

    fn accepts_features(&self) -> &'static [Feature] {
        ACCEPTED_FEATURES
    }

    fn accepts_intrinsics(&self) -> &'static [&'static str] {
        &[]
    }

    fn compile(&self, module: &Module) -> Result<Artifact, BackendError> {
        // 1. Validate the module.
        let result = semantic_ir::validate(module);
        if !result.is_ok() {
            let first = result
                .issues
                .iter()
                .find(|i| i.severity == semantic_ir::Severity::Error);
            return Err(BackendError {
                kind: BackendErrorKind::InvalidModule,
                message: first
                    .map(|i| i.message.clone())
                    .unwrap_or_else(|| "module failed validation".to_string()),
                span: module.span.clone(),
            });
        }

        // 2. Capability check (manifest features + intrinsics).
        let errs = self.check_module(module);
        if let Some(first) = errs.into_iter().next() {
            return Err(first);
        }

        // 3. Structural gate: some builtins (notably the `__method__`
        //    collection-dispatch protocol) are not gated by an unaccepted
        //    feature, so a module can pass the capability check yet still use a
        //    builtin this v0 has no lowering for.  Reject it cleanly rather than
        //    emit a call that fails at runtime.
        if let Some((name, span)) = emit::first_unsupported_builtin(module) {
            return Err(BackendError {
                kind: BackendErrorKind::UnsupportedFeature,
                message: format!(
                    "the v0 C backend does not yet lower the `{name}` builtin \
                     (deferred to a later feature batch)"
                ),
                span,
            });
        }


        // 4. Emit.
        let source = emit::emit_module(module);
        let line_count = source.lines().count();
        Ok(Artifact {
            filename: format!("{}.c", module.name),
            source: source.clone(),
            metadata: ArtifactMetadata {
                bytes: source.len(),
                line_count,
                notes: Default::default(),
            },
        })
    }
}
