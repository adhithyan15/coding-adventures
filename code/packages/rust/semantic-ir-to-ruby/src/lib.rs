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
    // ── SIR constants ────────────────────────────────────────────────────
    // `Feature::Constants` is observed by a `Scope::Const` name (an uppercase
    // identifier or a `Foo::Bar` path).  Two nodes carry it, both handled:
    //   `PI = 3`   → `Stmt::Assign { scope: Const, .. }` → native `PI = 3`
    //   `PI` / `Foo::Bar` → `Expr::VarRef { scope: Const, .. }` → native `PI`
    // A Ruby constant is emitted VERBATIM (not through `sanitize_ident`, which
    // would prefix an uppercase name and destroy its constant-hood); the
    // pre-emit scan validates every such name as a constant path (`Foo` /
    // `Foo::Bar`), so a hand-built module cannot inject source through one.
    //
    // Constants is folded in with `Classes` because the two are ENTANGLED: the
    // frontend records `Constants` in the manifest for any `Foo.new` (the
    // receiver `Foo` is a constant), so an instantiable class cannot compile
    // without it.  Accepting it also lets `raise SomeClass` compile (a specific
    // exception class is a `Const` reference — a form the exceptions slice
    // deferred precisely because Constants was unaccepted).
    Feature::Constants,
    // ── OOP classes, slice 1: empty declaration + construction ───────────
    // `Feature::Classes` is observed by a module holding at least one
    // `Stmt::ClassDef`.  This first slice accepts ONLY the minimal shape a
    // frontend emits for `class Foo; end` — an empty-bodied, base (no
    // superclass) class — plus `Foo.new`, which the frontend lowers to a
    // `__new__` builtin whose first argument is the class name (a `StrLit`).
    //   `class Foo; end`  → `Stmt::ClassDef { name: "Foo", superclass: None, body: [] }`
    //                        → native Ruby `class Foo\nend`
    //   `Foo.new(args…)`  → `BuiltinCall("__new__", [StrLit("Foo"), args…])`
    //                        → native Ruby `Foo.new(args…)`
    // Ruby is a class-based OO language, so both render as NATIVE Ruby (no
    // runtime method-table like the Go/Rust/C value backends need).
    //
    // TOTALITY — accepting this feature obligates handling every node it can
    // now surface.  `Classes` gates only `Stmt::ClassDef`, handled below; the
    // OOP *builtins* (`__def_method__`, `__method__`, `__super__`, `__self__`,
    // `__class_method__`, …) are NOT in `SUPPORTED_BUILTINS`, so a method-bearing
    // class is rejected cleanly by the pre-emit scan, never reaching an
    // `unreachable!`.  Within this slice the scan also rejects the two class
    // shapes deferred to later slices — a **superclass** (inheritance) and a
    // **non-empty class body** (class-level code / constants) — and validates
    // the class name (of both `ClassDef` and `__new__`) as a Ruby constant path
    // so a hand-built module cannot inject source through a crafted name.
    // Class variables / modules remain unaccepted features (their own slices).
    Feature::Classes,
    // ── OOP classes, slice 3: instance variables (`@ivars`) ──────────────
    // `Feature::InstanceVars` is observed by a `Scope::Instance` name (`@v`).
    // Two nodes carry it, both handled natively:
    //   `@v = x` → `Stmt::Assign { scope: Instance, .. }` → native `@v = x`
    //   `@v`     → `Expr::VarRef  { scope: Instance, .. }` → native `@v`
    // The frontend puts the leading `@` in the node's `name`, emitted VERBATIM;
    // the pre-emit scan validates it as `@<identifier>` (co-total with the
    // emitter) so no name can inject.  Instance-method bodies are installed with
    // `define_method` (slice 2), which binds `self` to the receiver, so `@v`
    // inside a method reads/writes the instance's own variable — no runtime
    // support.  The `__self__` builtin (a bare `self`) rides in here too,
    // rendering the native `self`.
    Feature::InstanceVars,
    // ── OOP classes, slice 6: class variables (`@@x`) ────────────────────
    // `Feature::ClassVars` is observed by a `Scope::ClassVar` name (`@@x`).
    //   `@@x = v` → `Stmt::Assign { scope: ClassVar }`
    //   `@@x`     → `Expr::VarRef  { scope: ClassVar }`
    // A method body runs in a HOISTED top-level function (not a lexical class
    // scope), where a bare `@@x` is a Ruby error ("class variable access from
    // toplevel").  So `@@x` read/write in a method routes through
    // `sir_cvar_owner(self).class_variable_get/set(:"@@x")` — the owner is the
    // class in both instance- (`self.class`) and class-method (`self`) contexts,
    // so both share the same `@@x`.  A class-BODY `@@x = <init>` (the only body
    // content accepted, making a non-empty class body legal for the first time)
    // instead writes on the class by NAME (`<Class>.class_variable_set`), since it
    // runs where `self` is `main`, not the class.  Every `@@`-name is validated as
    // `@@<identifier>` in the co-total scan (no injection).
    Feature::ClassVars,
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
            // A constant name/path (a `ClassDef` name, a `__new__` class name, a
            // `Const` reference, or a `Const` assignment target) that is not a
            // valid Ruby constant path — it is emitted verbatim, so a
            // metacharacter could inject source.
            Some(emit::ScanHit::ConstantName(name, span)) => {
                return Err(BackendError {
                    kind: BackendErrorKind::UnsupportedFeature,
                    message: format!(
                        "the Ruby backend cannot emit the constant name `{name}` \
                         (not a valid Ruby constant path)"
                    ),
                    span,
                });
            }
            // A well-formed construct beyond this slice's support (class
            // inheritance, a non-empty class body, or a namespaced class /
            // constant definition) — deferred to a later slice, rejected cleanly
            // rather than mis-emitted.
            Some(emit::ScanHit::Unsupported(reason, span)) => {
                return Err(BackendError {
                    kind: BackendErrorKind::UnsupportedFeature,
                    message: format!(
                        "the Ruby backend does not yet support {reason} \
                         (deferred to a later slice)"
                    ),
                    span,
                });
            }
            // A `Scope::Instance` name emitted verbatim (a `@v` read / write) that
            // is not a valid `@identifier` — a metacharacter could inject source.
            Some(emit::ScanHit::InstanceVarName(name, span)) => {
                return Err(BackendError {
                    kind: BackendErrorKind::UnsupportedFeature,
                    message: format!(
                        "the Ruby backend cannot emit the instance variable `{name}` \
                         (not a valid `@identifier`)"
                    ),
                    span,
                });
            }
            // A `Scope::ClassVar` name that is not a valid `@@identifier`.
            Some(emit::ScanHit::ClassVarName(name, span)) => {
                return Err(BackendError {
                    kind: BackendErrorKind::UnsupportedFeature,
                    message: format!(
                        "the Ruby backend cannot emit the class variable `{name}` \
                         (not a valid `@@identifier`)"
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
