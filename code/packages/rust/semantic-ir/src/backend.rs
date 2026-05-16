//! Backend interface.
//!
//! A SIR **backend** consumes a [`Module`] and produces an
//! [`Artifact`] — typically the source code of a target language —
//! or rejects the module with a [`BackendError`] when its
//! capability declaration doesn't support what the module needs.
//!
//! Backends never silently emit wrong code.  The defaults set up by
//! [`Backend::check_module`] enforce the manifest contract: every
//! feature declared by the module must be in the backend's
//! `accepts_features()` list; every intrinsic name must be in
//! `accepts_intrinsics()`.

use crate::manifest::Feature;
use crate::nodes::{Expr, Module};
use crate::span::Span;
use std::collections::BTreeMap;
use std::fmt;

/// A piece of generated output ready to be written to disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    /// Suggested filename for the output (the backend chooses).
    pub filename: String,
    /// The generated source code.
    pub source: String,
    /// Diagnostic / structural information about the artifact.
    pub metadata: ArtifactMetadata,
}

/// Side-band info about an artifact.  Free for backends to populate.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ArtifactMetadata {
    pub bytes: usize,
    pub line_count: usize,
    pub notes: BTreeMap<String, String>,
}

/// Backend error categories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendErrorKind {
    /// The module declared an SIR version this backend doesn't support.
    UnsupportedSirVersion,
    /// The module declared a feature the backend doesn't accept.
    UnsupportedFeature,
    /// The module used an intrinsic whose name is not in the
    /// backend's whitelist.
    UnsupportedIntrinsic,
    /// The validator reported errors that blocked lowering.
    InvalidModule,
    /// A node-specific lowering rule is missing or failed.
    LoweringError,
}

/// A backend rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendError {
    pub kind: BackendErrorKind,
    pub message: String,
    pub span: Span,
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "backend error [{:?}] at {}: {}", self.kind, self.span, self.message)
    }
}

impl std::error::Error for BackendError {}

/// The backend trait.
///
/// A backend pairs three things:
///
/// 1. A target identifier (`target_tag`).
/// 2. A capability declaration (`accepts_features`, `accepts_intrinsics`).
/// 3. A `compile` method that consumes a `Module` and returns an
///    `Artifact` or a `BackendError`.
///
/// The default-provided [`Backend::check_module`] runs the manifest
/// and intrinsic-whitelist checks; concrete implementations should
/// call it first inside `compile`.
pub trait Backend {
    /// Target language identifier (e.g. `"typescript"`).  Must be
    /// stable across versions — frontends tag intrinsics with this
    /// string.
    fn target_tag(&self) -> &'static str;

    /// Features this backend accepts.  The slice is consulted by
    /// [`check_module`].
    fn accepts_features(&self) -> &'static [Feature];

    /// Intrinsics this backend accepts by name.  Default is empty
    /// (the safe choice — no intrinsics accepted unless explicitly
    /// listed).
    fn accepts_intrinsics(&self) -> &'static [&'static str] {
        &[]
    }

    /// Run the capability checks on `module`.  Returns a list of
    /// errors (empty = OK).  Implementations should call this at the
    /// start of `compile` and fail fast on any error.
    fn check_module(&self, module: &Module) -> Vec<BackendError> {
        let mut errs = Vec::new();

        // 1. Manifest features must all be accepted.
        for feat in module.manifest.iter() {
            if !self.accepts_features().contains(&feat) {
                errs.push(BackendError {
                    kind: BackendErrorKind::UnsupportedFeature,
                    message: format!(
                        "backend `{}` does not accept feature `{}`",
                        self.target_tag(),
                        feat
                    ),
                    span: module.span.clone(),
                });
            }
        }

        // 2. Intrinsics must be on the whitelist *and* target-tagged
        //    for this backend.
        let whitelist = self.accepts_intrinsics();
        let target = self.target_tag();
        walk_intrinsics(module, &mut |name, targets, span| {
            if !whitelist.contains(&name) {
                errs.push(BackendError {
                    kind: BackendErrorKind::UnsupportedIntrinsic,
                    message: format!(
                        "backend `{}` does not accept intrinsic `{}`",
                        target, name
                    ),
                    span: span.clone(),
                });
                return;
            }
            if !targets.iter().any(|t| t == target) {
                errs.push(BackendError {
                    kind: BackendErrorKind::UnsupportedIntrinsic,
                    message: format!(
                        "intrinsic `{}` not tagged for target `{}`",
                        name, target
                    ),
                    span: span.clone(),
                });
            }
        });

        errs
    }

    /// Compile a SIR module to an artifact.
    fn compile(&self, module: &Module) -> Result<Artifact, BackendError>;
}

/// Walk the module's tree calling `f` on every `Intrinsic` node.
fn walk_intrinsics<F>(module: &Module, f: &mut F)
where
    F: FnMut(&str, &[String], &Span),
{
    for fn_ in &module.functions {
        walk_intrinsics_in_expr(&fn_.body.value, f);
        for s in &fn_.body.stmts {
            walk_intrinsics_in_stmt(s, f);
        }
    }
}

fn walk_intrinsics_in_stmt<F>(s: &crate::nodes::Stmt, f: &mut F)
where
    F: FnMut(&str, &[String], &Span),
{
    use crate::nodes::Stmt;
    match s {
        Stmt::LetBinding { value, .. } | Stmt::LetStarBinding { value, .. } => {
            walk_intrinsics_in_expr(value, f);
        }
        Stmt::ExprStmt { expr, .. } => walk_intrinsics_in_expr(expr, f),
    }
}

fn walk_intrinsics_in_expr<F>(e: &Expr, f: &mut F)
where
    F: FnMut(&str, &[String], &Span),
{
    match e {
        Expr::IntLit { .. }
        | Expr::BoolLit { .. }
        | Expr::NilLit { .. }
        | Expr::SymLit { .. }
        | Expr::StrLit { .. }
        | Expr::VarRef { .. } => {}
        Expr::If { cond, then_branch, else_branch, .. } => {
            walk_intrinsics_in_expr(cond, f);
            for s in &then_branch.stmts {
                walk_intrinsics_in_stmt(s, f);
            }
            walk_intrinsics_in_expr(&then_branch.value, f);
            for s in &else_branch.stmts {
                walk_intrinsics_in_stmt(s, f);
            }
            walk_intrinsics_in_expr(&else_branch.value, f);
        }
        Expr::Block(b) => {
            for s in &b.stmts {
                walk_intrinsics_in_stmt(s, f);
            }
            walk_intrinsics_in_expr(&b.value, f);
        }
        Expr::DirectCall { args, .. } | Expr::BuiltinCall { args, .. } => {
            for a in args {
                walk_intrinsics_in_expr(a, f);
            }
        }
        Expr::IndirectCall { target, args, .. } => {
            walk_intrinsics_in_expr(target, f);
            for a in args {
                walk_intrinsics_in_expr(a, f);
            }
        }
        Expr::MakeClosure { captures, .. } => {
            for c in captures {
                walk_intrinsics_in_expr(&c.value, f);
            }
        }
        Expr::Intrinsic { targets, name, args, span, .. } => {
            f(name.as_str(), targets, span);
            for a in args {
                walk_intrinsics_in_expr(a, f);
            }
        }
    }
}

/// Runtime registry mapping target tags to registered backends.
/// Tooling uses this to enumerate available backends.
pub struct BackendRegistry {
    entries: Vec<Box<dyn Backend + Send + Sync>>,
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn register<B: Backend + Send + Sync + 'static>(&mut self, b: B) {
        self.entries.push(Box::new(b));
    }

    pub fn get(&self, target_tag: &str) -> Option<&dyn Backend> {
        self.entries
            .iter()
            .map(|b| b.as_ref() as &dyn Backend)
            .find(|b| b.target_tag() == target_tag)
    }

    pub fn target_tags(&self) -> Vec<&'static str> {
        self.entries.iter().map(|b| b.target_tag()).collect()
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::EffectSet;
    use crate::manifest::FeatureManifest;
    use crate::metadata::Metadata;
    use crate::nodes::{Block, Expr, Function, Module, Stmt};
    use crate::types::SirType;

    fn s() -> Span {
        Span::synthetic()
    }

    struct NoFeaturesBackend;
    impl Backend for NoFeaturesBackend {
        fn target_tag(&self) -> &'static str {
            "test-empty"
        }
        fn accepts_features(&self) -> &'static [Feature] {
            &[]
        }
        fn compile(&self, _module: &Module) -> Result<Artifact, BackendError> {
            Ok(Artifact {
                filename: "x".into(),
                source: "".into(),
                metadata: ArtifactMetadata::default(),
            })
        }
    }

    struct AllFeaturesBackend;
    impl Backend for AllFeaturesBackend {
        fn target_tag(&self) -> &'static str {
            "test-all"
        }
        fn accepts_features(&self) -> &'static [Feature] {
            Feature::ALL
        }
        fn accepts_intrinsics(&self) -> &'static [&'static str] {
            &["raw_asm"]
        }
        fn compile(&self, _module: &Module) -> Result<Artifact, BackendError> {
            Ok(Artifact {
                filename: "x".into(),
                source: "".into(),
                metadata: ArtifactMetadata::default(),
            })
        }
    }

    fn module_with_feature(feat: Feature) -> Module {
        Module {
            name: "m".into(),
            manifest: FeatureManifest::from_features(&[feat]),
            imports: vec![],
            exports: vec![],
            functions: vec![],
            globals: vec![],
            metadata: Metadata::new(),
            span: s(),
        }
    }

    #[test]
    fn rejects_unsupported_feature() {
        let b = NoFeaturesBackend;
        let m = module_with_feature(Feature::Closures);
        let errs = b.check_module(&m);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].kind, BackendErrorKind::UnsupportedFeature);
    }

    #[test]
    fn accepts_when_feature_supported() {
        let b = AllFeaturesBackend;
        let m = module_with_feature(Feature::Closures);
        let errs = b.check_module(&m);
        assert!(errs.is_empty());
    }

    #[test]
    fn rejects_unknown_intrinsic() {
        let b = AllFeaturesBackend;
        let mut m = module_with_feature(Feature::Intrinsics);
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::ExprStmt {
                    expr: Expr::Intrinsic {
                        targets: vec!["test-all".into()],
                        name: "not_whitelisted".into(),
                        args: vec![],
                        return_type: SirType::Any,
                        effects: EffectSet::PURE,
                        span: s(),
                    },
                    span: s(),
                }],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let errs = b.check_module(&m);
        assert!(errs
            .iter()
            .any(|e| e.kind == BackendErrorKind::UnsupportedIntrinsic));
    }

    #[test]
    fn intrinsic_must_be_target_tagged() {
        let b = AllFeaturesBackend;
        let mut m = module_with_feature(Feature::Intrinsics);
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::ExprStmt {
                    expr: Expr::Intrinsic {
                        // whitelisted by name but tagged for another target.
                        targets: vec!["different".into()],
                        name: "raw_asm".into(),
                        args: vec![],
                        return_type: SirType::Any,
                        effects: EffectSet::PURE,
                        span: s(),
                    },
                    span: s(),
                }],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let errs = b.check_module(&m);
        assert!(errs.iter().any(|e| e.message.contains("not tagged for target")));
    }

    #[test]
    fn registry_round_trips() {
        let mut reg = BackendRegistry::new();
        reg.register(NoFeaturesBackend);
        reg.register(AllFeaturesBackend);
        assert!(reg.get("test-empty").is_some());
        assert!(reg.get("test-all").is_some());
        assert!(reg.get("nope").is_none());
        let tags = reg.target_tags();
        assert!(tags.contains(&"test-empty"));
        assert!(tags.contains(&"test-all"));
    }
}
