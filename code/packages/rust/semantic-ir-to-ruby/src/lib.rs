//! # semantic-ir-to-ruby
//!
//! Seventh backend for the narrow-waist Semantic IR — emits **self-contained**
//! Ruby source from a [`semantic_ir::Module`].
//!
//! Output is a single `.rb` file with a small inlined runtime; it runs with
//! `ruby <file>.rb`, no gems.  Ruby was previously only a *frontend*
//! ([`ruby-to-semantic-ir`]); this backend lets SIR *emit* Ruby — enabling
//! Ruby↔SIR round-trips, Twig/Python/JavaScript→Ruby, and the motivating
//! **C→SIR→Ruby** path.
//!
//! Implements [SIR25](../../../specs/SIR25-semantic-ir-to-ruby.md).  This is the
//! **v0 core**; later feature batches (SIR16, params, the `Convert` node,
//! collection methods, exceptions, OOP) land incrementally.

mod emit;
mod runtime;

use semantic_ir::{
    Artifact, ArtifactMetadata, Backend, BackendError, BackendErrorKind, Feature, Module,
};

pub use emit::sanitize_ident;

/// Compile a module to a Ruby artifact (convenience wrapper over [`RubyBackend`]).
pub fn compile(module: &Module) -> Result<Artifact, BackendError> {
    RubyBackend::new().compile(module)
}

/// The Ruby backend.
pub struct RubyBackend;

impl RubyBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RubyBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// The SIR-v0 feature set.  Later batches extend this in lockstep with the
/// emitter and runtime.
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
    // `Expr::Convert` renders as an inlined mask helper (`sir_u8`/`sir_i32`/…);
    // a Convert's target type also makes the validator observe these SIR21
    // type-implied features, so the Ruby backend must accept them to compile a
    // conversion-bearing module.  Ruby's arbitrary-precision Integer makes the
    // masking exact for every width (its bitwise ops use two's complement).
    Feature::Conversions,
    Feature::SizedIntegers,
    Feature::Unsigned,
    Feature::WrappingArithmetic,
    // ── SIR16 floats ─────────────────────────────────────────────────
    // Ruby has a native `Float`, so a `Expr::FloatLit` renders directly as a
    // Ruby float literal (via `float_to_ruby_literal`, which guarantees the
    // literal round-trips as a Float — `7.0`, not the Integer `7`). `Floats`
    // gates ONLY `FloatLit`; float arithmetic reuses the same `+`/`-`/`*`/`/`
    // builtins (native Ruby operators, so `1.5 + 2.5 == 4.0` and `7.0 / 2 ==
    // 3.5` are exact), and the runtime's `sir_fmt_float` already renders every
    // float (integral floats keep their `.0`; `NaN`/`Infinity` are named). So
    // accepting the feature plus the one emit arm keeps the emitter total.
    Feature::Floats,
    // ── SIR16 short-circuit ──────────────────────────────────────────
    // `Expr::LogicalAnd` / `Expr::LogicalOr` (`&&` / `||`) render as Ruby's
    // native short-circuit operators — which ARE the SIR semantics exactly:
    // they yield the DECIDING OPERAND (not a bool) and skip the rhs when the
    // lhs decides, and Ruby truthiness is the SIR convention (only `nil`/`false`
    // falsy), so no `sir_truthy` wrapper is needed. Two nodes, both handled →
    // the emitter stays total. (Distinct from the eager `and`/`or` builtins,
    // which the emitter also renders with `&&`/`||`.)
    Feature::ShortCircuit,
    // ── SIR16 control flow / mutation ────────────────────────────────
    // `Stmt::While` (loops) and `Stmt::Assign` (re-binding) render as Ruby's
    // native `while … end` and `name = value` — Ruby is fully mutable and
    // expression-oriented, so these are direct.  The C frontend's milestone-2
    // `if`/`while`/`for` lower to these plus `Expr::If` (which needs no feature).
    Feature::Loops,
    Feature::MutableBindings,
    // ── SIR16 sequences ──────────────────────────────────────────────
    // Ruby has native arrays, so the SIR16 sequence nodes render directly (no
    // runtime value-boxing like the Go/Rust backends' `_sir_seq_*`). The
    // emitter handles EVERY construct the `sequences` feature can surface:
    //   `Expr::SeqLit`   → `[1, 2, 3]`      (structural `Array#==`)
    //   `Expr::SeqIndex` → `(a)[i]`         (nil on OOB, negative-from-end)
    //   `Expr::SeqLen`   → `(a).length`
    //   `Stmt::SeqSet`   → `sir_seq_set(a, i, v)` (raises on OOB, per the ref)
    //   `Stmt::ForEach`  → `(a).each { |x| … }` (also reachable once `Loops`
    //                       is accepted — a block, so `x` is block-scoped,
    //                       matching the validator's rewind and Go's `:=` var)
    // handling all five keeps the emitter TOTAL for this feature (no
    // `unreachable!` reachable from a conforming producer). Array *indexing
    // via `Expr::IndexGet`* and slicing are a DIFFERENT feature (`NDArrays`,
    // not accepted); array-*pattern* destructuring needs `ShortCircuit` (not
    // accepted) — so those stay rejected at the feature gate.
    Feature::Sequences,
    // ── SIR16 maps ───────────────────────────────────────────────────
    // Ruby has a native Hash, so the three `maps` nodes render directly (no
    // runtime value-boxing like the Go/Rust `_sir_map_*`):
    //   `Expr::MapLit` → `{k => v, …}` (a Hash literal; keys compared by
    //                     `eql?`/`hash`, which is structural for composite keys)
    //   `Expr::MapGet` → `(h)[k]`      (missing key → nil, no raise)
    //   `Stmt::MapSet` → `(h)[k] = v`  (insert/update, mutates the shared Hash)
    // `ForEach` over a Hash is already covered — `(h).each { |kv| … }` works on
    // a Hash as well as an Array — so accepting Maps adds no new `unreachable!`.
    Feature::Maps,
    // ── SIR19 default parameters ─────────────────────────────────────
    // A positional parameter carrying a default expression renders as Ruby's
    // native `def f(a, b = <default>)`.  Ruby evaluates the default at call time
    // when the argument is omitted — exactly the SIR semantics — so no runtime
    // support is needed.  Only the positional case is `DefaultParams`; a keyword
    // default is the separate (still-unaccepted) `KeywordParams` feature. The
    // unsupported-builtin pre-check now scans each parameter default too, so the
    // emitter stays total.
    Feature::DefaultParams,
    // ── SIR19 keyword parameters ─────────────────────────────────────
    // A keyword parameter (`def f(x:)` / `def f(x: 1)`) and a keyword argument
    // (`f(x: 5)`) render as Ruby's NATIVE keyword forms — Ruby matches a keyword
    // argument to its parameter by name, so no positional resolution is needed
    // (unlike the Go/C backends' KW6 lowering).  A keyword default rides on this
    // feature (an optional keyword), not `DefaultParams`.
    Feature::KeywordParams,
    // ── SIR17 exceptions ─────────────────────────────────────────────
    // `Stmt::TryCatch` (`begin … rescue … ensure … end`) and the `raise` /
    // `retry` builtins render as Ruby's NATIVE exception handling — no runtime
    // support (Ruby raises/rescues by exception class natively).  A `rescue`
    // clause matches by exception-class NAME (advisory strings the frontend
    // takes from source; validated as constant paths before emit) and may bind
    // the caught exception to a local.  `raise SomeClass` (a specific class) is
    // a `Const` reference → it observes `Feature::Constants` (unaccepted) and is
    // rejected; `raise "message"`, a bare re-raise, and `rescue` by a standard
    // class or catch-all are the accepted forms.
    Feature::Exceptions,
];

impl Backend for RubyBackend {
    fn target_tag(&self) -> &'static str {
        "ruby"
    }

    fn accepts_features(&self) -> &'static [Feature] {
        ACCEPTED_FEATURES
    }

    fn accepts_intrinsics(&self) -> &'static [&'static str] {
        &[]
    }

    fn compile(&self, module: &Module) -> Result<Artifact, BackendError> {
        // 1. Validate.
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
        if let Some(first) = self.check_module(module).into_iter().next() {
            return Err(first);
        }

        // 3. Structural gate: the `__method__` collection-dispatch protocol (and
        //    other reserved builtins) are not gated by an unaccepted feature, so
        //    reject a module that uses a builtin v0 cannot lower rather than
        //    emit a call with no lowering.
        //    A single traversal reports BOTH an unlowerable builtin AND an
        //    injectable `rescue` type (a `rescue` clause name is emitted verbatim
        //    as a Ruby constant reference, so it must be a valid constant path —
        //    otherwise a hand-built module could inject source).  Sharing one
        //    walk keeps the check co-total with the emitter.
        match emit::first_scan_issue(module) {
            Some(emit::ScanHit::Builtin(name, span)) => {
                return Err(BackendError {
                    kind: BackendErrorKind::UnsupportedFeature,
                    message: format!(
                        "the v0 Ruby backend does not yet lower the `{name}` builtin \
                         (deferred to a later feature batch)"
                    ),
                    span,
                });
            }
            Some(emit::ScanHit::RescueType(name, span)) => {
                return Err(BackendError {
                    kind: BackendErrorKind::UnsupportedFeature,
                    message: format!(
                        "the Ruby backend cannot emit the rescue exception type `{name}` \
                         (not a valid Ruby constant path)"
                    ),
                    span,
                });
            }
            None => {}
        }

        // 4. Emit.
        let source = emit::emit_module(module);
        let line_count = source.lines().count();
        Ok(Artifact {
            filename: format!("{}.rb", module.name),
            source: source.clone(),
            metadata: ArtifactMetadata {
                bytes: source.len(),
                line_count,
                notes: Default::default(),
            },
        })
    }
}
