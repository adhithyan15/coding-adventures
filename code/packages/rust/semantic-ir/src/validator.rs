//! Module validation.
//!
//! `validate(module)` performs structural and semantic checks that
//! must hold for every well-formed SIR module.  The validator is
//! intentionally strict: it catches programming errors in
//! *frontends* (a frontend that emits an invalid module is buggy),
//! not user-source errors.  User-facing errors should be reported
//! during lowering with proper source spans.
//!
//! Validator output is split into errors (which abort compilation)
//! and warnings (which are informational).  Use
//! [`ValidationResult::is_ok`] to gate further processing.

use crate::limits::MAX_IR_DEPTH;
use crate::manifest::{Feature, FeatureManifest};
use crate::nodes::*;
use crate::span::Span;
use std::collections::HashSet;
use std::fmt;

/// A validator finding (error or warning) with a source position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorIssue {
    pub message: String,
    pub span: Span,
    pub severity: Severity,
}

/// Severity tag for a validator issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl fmt::Display for ValidatorIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sev = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(f, "{} {}: {}", sev, self.span, self.message)
    }
}

/// Collected validator output.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidationResult {
    pub issues: Vec<ValidatorIssue>,
}

impl ValidationResult {
    /// `true` iff there are no `Error`-severity issues.
    pub fn is_ok(&self) -> bool {
        !self.issues.iter().any(|i| i.severity == Severity::Error)
    }

    /// Filter the collected issues by severity.
    pub fn errors(&self) -> impl Iterator<Item = &ValidatorIssue> {
        self.issues.iter().filter(|i| i.severity == Severity::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &ValidatorIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
    }
}

/// Run the v0 validator over a module.  Returns collected issues;
/// callers gate compilation on `result.is_ok()`.
pub fn validate(module: &Module) -> ValidationResult {
    let mut v = ValidatorState::new(module);
    v.run();
    v.result
}

// ---------------------------------------------------------------------------
// Internal validator state
// ---------------------------------------------------------------------------

struct ValidatorState<'m> {
    module: &'m Module,
    result: ValidationResult,
    /// Features actually observed in the module body.  Compared
    /// against the declared manifest at the end.
    observed: FeatureManifest,
    /// All function names declared in this module.  Used to validate
    /// `DirectCall` targets and to detect duplicates.
    function_names: HashSet<String>,
    /// All global names declared in this module.
    global_names: HashSet<String>,
    /// `true` once a depth-overflow error has been recorded for
    /// this module.  Suppresses duplicate spam.
    depth_overflow_reported: bool,
}

impl<'m> ValidatorState<'m> {
    fn new(module: &'m Module) -> Self {
        Self {
            module,
            result: ValidationResult::default(),
            observed: FeatureManifest::new(),
            function_names: HashSet::new(),
            global_names: HashSet::new(),
            depth_overflow_reported: false,
        }
    }

    /// Report a depth-overflow error once.  Returns `true` if the
    /// caller should stop recursing.
    fn check_depth(&mut self, depth: usize, span: &Span) -> bool {
        if depth >= MAX_IR_DEPTH {
            if !self.depth_overflow_reported {
                self.depth_overflow_reported = true;
                self.error(
                    format!(
                        "expression nesting exceeds MAX_IR_DEPTH ({})",
                        MAX_IR_DEPTH
                    ),
                    span,
                );
            }
            return true;
        }
        false
    }

    fn run(&mut self) {
        self.check_module_metadata();
        self.collect_top_level_names();
        self.walk_bodies();
        self.compare_manifests();
    }

    fn error(&mut self, message: impl Into<String>, span: &Span) {
        self.result.issues.push(ValidatorIssue {
            message: message.into(),
            span: span.clone(),
            severity: Severity::Error,
        });
    }

    fn warning(&mut self, message: impl Into<String>, span: &Span) {
        self.result.issues.push(ValidatorIssue {
            message: message.into(),
            span: span.clone(),
            severity: Severity::Warning,
        });
    }

    fn check_module_metadata(&mut self) {
        // SIR version must be present and match.
        if let Some(v) = &self.module.metadata.sir_version {
            if v != crate::metadata::CURRENT_SIR_VERSION {
                self.error(
                    format!(
                        "module declares sir_version `{}` but validator implements `{}`",
                        v,
                        crate::metadata::CURRENT_SIR_VERSION
                    ),
                    &self.module.span,
                );
            }
        }
        // No anonymous modules.
        if self.module.name.is_empty() {
            self.error("module name must be non-empty", &self.module.span);
        }
    }

    fn collect_top_level_names(&mut self) {
        for f in &self.module.functions {
            if !self.function_names.insert(f.name.clone()) {
                self.error(
                    format!("duplicate function name `{}`", f.name),
                    &f.span,
                );
            }
        }
        for g in &self.module.globals {
            if !self.global_names.insert(g.name.clone()) {
                self.error(
                    format!("duplicate global name `{}`", g.name),
                    &g.span,
                );
            }
        }
        if !self.module.globals.is_empty() {
            self.observed.add(Feature::Globals);
        }
    }

    fn walk_bodies(&mut self) {
        // Stage: per-function checks (params, captures, scope correctness).
        // We split this out so we can build the scoped names lazily
        // for each function rather than using the generic Visitor
        // (which doesn't track scopes).
        for f in &self.module.functions.clone() {
            self.check_function(f);
        }
    }

    fn check_function(&mut self, f: &Function) {
        // Collect parameter and capture names; they must be unique
        // within their respective lists.
        let mut param_names: HashSet<String> = HashSet::new();
        for p in &f.params {
            if !param_names.insert(p.name.clone()) {
                self.error(
                    format!("duplicate parameter `{}`", p.name),
                    &p.span,
                );
            }
            if p.sir_type.is_some() {
                self.observed.add(Feature::OptionalTypeAnnotations);
            } else {
                self.observed.add(Feature::DynamicTyping);
            }
        }

        let mut capture_names: HashSet<String> = HashSet::new();
        for c in &f.captures {
            if !capture_names.insert(c.name.clone()) {
                self.error(format!("duplicate capture `{}`", c.name), &f.span);
            }
        }
        // Closures = function has non-empty captures, or the body
        // contains MakeClosure / IndirectCall (observed below).
        if !f.captures.is_empty() {
            self.observed.add(Feature::Closures);
        }

        // Now walk the body checking name resolution.  A small
        // scope stack tracks `let` and `let*` bindings.
        let mut env = LocalEnv::new(&param_names, &capture_names);
        self.check_block(&f.body, &mut env, 0);
    }

    fn check_block(&mut self, b: &Block, env: &mut LocalEnv, depth: usize) {
        if self.check_depth(depth, &b.span) {
            return;
        }
        let mark = env.mark();
        // Walk statements in *groups*: a run of consecutive LetBinding
        // statements forms one parallel-let group whose RHS expressions
        // all evaluate in the scope BEFORE the group.  All names from
        // the group are added at once after every RHS has been checked.
        // LetStarBinding and ExprStmt break the run; LetStarBinding
        // adds its name immediately (sequential semantics).
        let mut i = 0;
        while i < b.stmts.len() {
            match &b.stmts[i] {
                Stmt::LetBinding { .. } => {
                    // Find the maximal run of LetBindings starting at i.
                    let mut j = i;
                    while j < b.stmts.len() && matches!(b.stmts[j], Stmt::LetBinding { .. }) {
                        j += 1;
                    }
                    // Check every RHS in the *outer* env (no new
                    // names added yet).
                    for k in i..j {
                        if let Stmt::LetBinding { value, sir_type, .. } = &b.stmts[k] {
                            self.check_expr(value, env, depth + 1);
                            if sir_type.is_some() {
                                self.observed.add(Feature::OptionalTypeAnnotations);
                            }
                        }
                    }
                    // Add every bound name to the env, all at once.
                    for k in i..j {
                        if let Stmt::LetBinding { name, .. } = &b.stmts[k] {
                            env.add_local(name.clone());
                        }
                    }
                    i = j;
                }
                Stmt::LetStarBinding { name, sir_type, value, .. } => {
                    self.check_expr(value, env, depth + 1);
                    env.add_local(name.clone());
                    if sir_type.is_some() {
                        self.observed.add(Feature::OptionalTypeAnnotations);
                    }
                    i += 1;
                }
                Stmt::ExprStmt { expr, .. } => {
                    self.check_expr(expr, env, depth + 1);
                    i += 1;
                }
            }
        }
        self.check_expr(&b.value, env, depth + 1);
        env.rewind(mark);
    }

    fn check_expr(&mut self, e: &Expr, env: &mut LocalEnv, depth: usize) {
        if self.check_depth(depth, e.span()) {
            return;
        }
        match e {
            Expr::IntLit { .. } | Expr::BoolLit { .. } | Expr::NilLit { .. } => {}
            Expr::SymLit { .. } => {
                self.observed.add(Feature::Symbols);
            }
            Expr::StrLit { .. } => {
                self.observed.add(Feature::Strings);
            }
            Expr::VarRef { name, scope, span } => {
                self.check_varref(name, *scope, span, env);
            }
            Expr::If { cond, then_branch, else_branch, .. } => {
                self.check_expr(cond, env, depth + 1);
                self.check_block(then_branch, env, depth + 1);
                self.check_block(else_branch, env, depth + 1);
            }
            Expr::Block(b) => self.check_block(b, env, depth + 1),
            Expr::DirectCall { fn_name, args, .. } => {
                if !self.function_names.contains(fn_name) {
                    self.error(
                        format!("direct call to unknown function `{}`", fn_name),
                        e.span(),
                    );
                }
                for a in args {
                    self.check_expr(a, env, depth + 1);
                }
            }
            Expr::IndirectCall { target, args, .. } => {
                self.observed.add(Feature::Closures);
                self.check_expr(target, env, depth + 1);
                for a in args {
                    self.check_expr(a, env, depth + 1);
                }
            }
            Expr::BuiltinCall { name, args, .. } => {
                match name.as_str() {
                    "cons" | "car" | "cdr" | "pair?" => self.observed.add(Feature::Pairs),
                    _ => {}
                }
                for a in args {
                    self.check_expr(a, env, depth + 1);
                }
            }
            Expr::MakeClosure { fn_name, captures, span } => {
                self.observed.add(Feature::Closures);
                if !self.function_names.contains(fn_name) {
                    self.error(
                        format!("make-closure references unknown function `{}`", fn_name),
                        span,
                    );
                }
                for c in captures {
                    self.check_expr(&c.value, env, depth + 1);
                }
            }
            Expr::Intrinsic { targets, args, span, .. } => {
                self.observed.add(Feature::Intrinsics);
                if targets.is_empty() {
                    self.error("intrinsic must declare at least one target tag", span);
                }
                for a in args {
                    self.check_expr(a, env, depth + 1);
                }
            }
        }
    }

    /// Validate a single `VarRef` against the current local
    /// environment and the module's top-level table.
    ///
    /// The check is *structural* — we verify that the scope tag is
    /// consistent with what is in scope at this point.  We do *not*
    /// require the name to exist in the env for `Global` or
    /// `Builtin` scopes (those are resolved outside the module).
    fn check_varref(&mut self, name: &str, scope: Scope, span: &Span, env: &LocalEnv) {
        match scope {
            Scope::Local => {
                if !env.has_local(name) {
                    self.error(
                        format!("var-ref scope=local references unknown name `{}`", name),
                        span,
                    );
                }
            }
            Scope::Param => {
                if !env.has_param(name) {
                    self.error(
                        format!("var-ref scope=param references unknown parameter `{}`", name),
                        span,
                    );
                }
            }
            Scope::Capture => {
                if !env.has_capture(name) {
                    self.error(
                        format!("var-ref scope=capture references unknown capture `{}`", name),
                        span,
                    );
                }
            }
            Scope::Global => {
                if !self.global_names.contains(name) && !self.function_names.contains(name) {
                    // Globals include both `(define x ...)` and `(define (f ...) ...)`
                    // forms; the latter declare a function name reachable as a value.
                    self.error(
                        format!("var-ref scope=global references unknown name `{}`", name),
                        span,
                    );
                }
            }
            Scope::Builtin => {
                // Builtin names are not enumerated by the SIR;
                // resolution is the backend's responsibility.
            }
        }
    }

    /// Final pass: compare declared manifest to observed manifest.
    fn compare_manifests(&mut self) {
        // Snapshot the feature lists before mutating self via error/warning,
        // and clone the module span we'll reuse for every diagnostic.
        let observed: Vec<Feature> = self.observed.iter().collect();
        let declared: Vec<Feature> = self.module.manifest.iter().collect();
        let span = self.module.span.clone();

        // Used but not declared → error.
        for feat in &observed {
            if !declared.contains(feat) {
                self.error(
                    format!(
                        "manifest does not declare feature `{}` but module uses it",
                        feat
                    ),
                    &span,
                );
            }
        }
        // Declared but not used → warning.
        for feat in &declared {
            if !observed.contains(feat) {
                self.warning(
                    format!(
                        "manifest declares feature `{}` but module does not use it",
                        feat
                    ),
                    &span,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// LocalEnv — a small scoped name table used during validation
// ---------------------------------------------------------------------------

struct LocalEnv<'a> {
    params: &'a HashSet<String>,
    captures: &'a HashSet<String>,
    locals: Vec<String>,
}

impl<'a> LocalEnv<'a> {
    fn new(params: &'a HashSet<String>, captures: &'a HashSet<String>) -> Self {
        Self {
            params,
            captures,
            locals: Vec::new(),
        }
    }

    fn mark(&self) -> usize {
        self.locals.len()
    }

    fn rewind(&mut self, mark: usize) {
        self.locals.truncate(mark);
    }

    fn add_local(&mut self, name: String) {
        self.locals.push(name);
    }

    fn has_local(&self, name: &str) -> bool {
        self.locals.iter().any(|n| n == name)
    }

    fn has_param(&self, name: &str) -> bool {
        self.params.contains(name)
    }

    fn has_capture(&self, name: &str) -> bool {
        self.captures.contains(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::EffectSet;
    use crate::manifest::{Feature, FeatureManifest};
    use crate::metadata::{Metadata, CURRENT_SIR_VERSION};
    use crate::span::Span;

    fn s() -> Span {
        Span::synthetic()
    }

    fn empty_module(manifest: FeatureManifest) -> Module {
        Module {
            name: "demo".into(),
            manifest,
            imports: vec![],
            exports: vec![],
            functions: vec![],
            globals: vec![],
            metadata: Metadata::new().with_sir_version(CURRENT_SIR_VERSION),
            span: s(),
        }
    }

    #[test]
    fn empty_module_is_valid() {
        let r = validate(&empty_module(FeatureManifest::new()));
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn missing_module_name_is_error() {
        let mut m = empty_module(FeatureManifest::new());
        m.name = "".into();
        let r = validate(&m);
        assert!(!r.is_ok());
    }

    #[test]
    fn duplicate_function_name_is_error() {
        let mut m = empty_module(FeatureManifest::new());
        let body = Block {
            stmts: vec![],
            value: Expr::NilLit { span: s() },
            span: s(),
        };
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: body.clone(),
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(!r.is_ok());
    }

    #[test]
    fn missing_manifest_feature_is_error() {
        // Function uses a symbol literal but the manifest doesn't
        // declare `Symbols`.
        let mut m = empty_module(FeatureManifest::new());
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::SymLit { name: "x".into(), span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r
            .errors()
            .any(|i| i.message.contains("symbols")));
    }

    #[test]
    fn over_declared_manifest_is_warning_only() {
        let mut m = empty_module(FeatureManifest::from_features(&[Feature::Symbols]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(r.is_ok());
        assert!(r.warnings().count() >= 1);
    }

    #[test]
    fn direct_call_to_unknown_is_error() {
        let mut m = empty_module(FeatureManifest::new());
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::DirectCall {
                    fn_name: "ghost".into(),
                    args: vec![],
                    effects: EffectSet::PURE,
                    span: s(),
                },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(!r.is_ok());
    }

    #[test]
    fn varref_local_must_exist() {
        let mut m = empty_module(FeatureManifest::new());
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::VarRef {
                    name: "ghost".into(),
                    scope: Scope::Local,
                    span: s(),
                },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(!r.is_ok());
    }

    #[test]
    fn intrinsic_without_targets_is_error() {
        let mut m = empty_module(FeatureManifest::from_features(&[Feature::Intrinsics]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::Intrinsic {
                    targets: vec![],
                    name: "asm".into(),
                    args: vec![],
                    return_type: crate::types::SirType::Any,
                    effects: EffectSet::PURE,
                    span: s(),
                },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("target tag")));
    }

    #[test]
    fn parallel_let_does_not_leak_binding_into_sibling_rhs() {
        // (let ((x 1) (y x)) y) — `y`'s RHS references `x`, which is
        // NOT yet in scope (parallel let).  Should error.
        let mut m = empty_module(FeatureManifest::new());
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![
                    Stmt::LetBinding {
                        name: "x".into(),
                        sir_type: None,
                        value: Expr::IntLit { value: 1, span: s() },
                        span: s(),
                    },
                    Stmt::LetBinding {
                        name: "y".into(),
                        sir_type: None,
                        value: Expr::VarRef {
                            name: "x".into(),
                            scope: Scope::Local,
                            span: s(),
                        },
                        span: s(),
                    },
                ],
                value: Expr::VarRef {
                    name: "y".into(),
                    scope: Scope::Local,
                    span: s(),
                },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(
            !r.is_ok(),
            "expected error from parallel let referencing sibling, got {:?}",
            r.issues
        );
    }

    #[test]
    fn depth_overflow_is_reported_not_panicked() {
        // Validating a pathologically deep tree must produce a
        // depth-overflow Error, not blow the host stack.  We run on
        // a dedicated thread with a big stack so the *test
        // infrastructure* (which has its own recursive Drop and
        // construction code paths) doesn't overflow before the
        // validator runs.  The validator itself — the system under
        // test — is what we're proving safe.
        let handle = std::thread::Builder::new()
            .stack_size(32 * 1024 * 1024)
            .name("depth-overflow-test".into())
            .spawn(|| {
                let mut m = empty_module(FeatureManifest::new());
                let deep = build_deep_if_chain(crate::MAX_IR_DEPTH + 100);
                m.functions.push(Function {
                    name: "f".into(),
                    params: vec![],
                    return_type: None,
                    captures: vec![],
                    body: Block {
                        stmts: vec![],
                        value: deep,
                        span: s(),
                    },
                    effects: EffectSet::PURE,
                    metadata: Metadata::new(),
                    span: s(),
                });
                let r = validate(&m);
                let ok = r.is_ok();
                let saw_overflow = r
                    .errors()
                    .any(|i| i.message.contains("exceeds MAX_IR_DEPTH"));
                // Leak the deep module to skip the recursive Drop,
                // which itself would overflow even this larger
                // stack on tear-down.
                Box::leak(Box::new(m));
                (ok, saw_overflow)
            })
            .expect("spawn test thread");
        let (ok, saw_overflow) = handle.join().expect("thread join");
        assert!(!ok, "expected validation to fail with depth overflow");
        assert!(saw_overflow, "expected depth-overflow error to be reported");
    }

    /// Build a chain of `(if true then <inner> else <nil>)` of the
    /// given depth, with `(nil)` at the bottom.  Constructed
    /// iteratively to avoid host-stack issues during build.
    fn build_deep_if_chain(depth: usize) -> Expr {
        let mut e = Expr::NilLit { span: s() };
        for _ in 0..depth {
            e = Expr::If {
                cond: Box::new(Expr::BoolLit { value: true, span: s() }),
                then_branch: Box::new(Block {
                    stmts: vec![],
                    value: e,
                    span: s(),
                }),
                else_branch: Box::new(Block {
                    stmts: vec![],
                    value: Expr::NilLit { span: s() },
                    span: s(),
                }),
                span: s(),
            };
        }
        e
    }

    #[test]
    fn sequential_let_star_sees_prior_binding() {
        // (let* ((x 1) (y x)) y) — fine in let*.
        let mut m = empty_module(FeatureManifest::new());
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![
                    Stmt::LetStarBinding {
                        name: "x".into(),
                        sir_type: None,
                        value: Expr::IntLit { value: 1, span: s() },
                        span: s(),
                    },
                    Stmt::LetStarBinding {
                        name: "y".into(),
                        sir_type: None,
                        value: Expr::VarRef {
                            name: "x".into(),
                            scope: Scope::Local,
                            span: s(),
                        },
                        span: s(),
                    },
                ],
                value: Expr::VarRef {
                    name: "y".into(),
                    scope: Scope::Local,
                    span: s(),
                },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }
}
