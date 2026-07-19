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
    // ── SIR16 control flow / mutation ────────────────────────────────
    // `Stmt::While` (loops) and `Stmt::Assign` (re-binding) render as Ruby's
    // native `while … end` and `name = value` — Ruby is fully mutable and
    // expression-oriented, so these are direct.  The C frontend's milestone-2
    // `if`/`while`/`for` lower to these plus `Expr::If` (which needs no feature).
    Feature::Loops,
    Feature::MutableBindings,
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
        if let Some((name, span)) = emit::first_unsupported_builtin(module) {
            return Err(BackendError {
                kind: BackendErrorKind::UnsupportedFeature,
                message: format!(
                    "the v0 Ruby backend does not yet lower the `{name}` builtin \
                     (deferred to a later feature batch)"
                ),
                span,
            });
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
