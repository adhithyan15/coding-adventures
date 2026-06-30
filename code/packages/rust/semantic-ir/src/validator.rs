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
use std::collections::{HashMap, HashSet};
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

/// Pre-computed call-arity profile of a known top-level function,
/// cached by [`ValidatorState`] so each `DirectCall` arity check is a
/// constant-time map lookup rather than a re-scan of the callee's
/// params.
///
/// `min` is [`Function::required_param_count`] (the leading run of
/// no-default plain positionals); `max` is the total positional param
/// count.  `variadic` records whether the callee has a `*rest`/`**opts`
/// param or the synthetic trailing block param (`__sir_block__`) — when
/// set, the strict bounds are not enforced (deferred; see the
/// `DirectCall` arm of `check_expr`).
#[derive(Debug, Clone, Copy)]
struct FnArity {
    min: usize,
    max: usize,
    variadic: bool,
}

/// `true` iff every argument in a call contributes exactly one positional
/// value — i.e. the call has no splat/forwarding expansion and no implicit
/// block handle appended to the argument list.  Only such "plain" calls
/// have a statically meaningful `args.len()`, so the strict default-param
/// arity bounds (SIR10) are applied to them alone.
///
/// The dynamic-arity argument shapes we exclude, all produced by the Ruby
/// frontend's call-position lowerings, are:
///   - `BuiltinCall("splat", …)`        — `f(*arr)` expands `arr` in place
///   - `BuiltinCall("double_splat", …)` — `f(**hsh)` expands `hsh`
///   - `BuiltinCall("forward_args", …)` — `f(...)` argument forwarding
///   - `BuiltinCall("block_pass", …)`   — `f(&blk)` block-pass
///   - `MakeClosure { … }`              — an implicit block (`f(x) { … }`)
///     appended as a trailing positional argument even when the callee
///     does not declare a block param.
///
/// Encountering any of these means the static argument count cannot be
/// compared against the declared parameter count, so the caller skips the
/// arity check (deferred — see the `DirectCall` arm of `check_expr`).
fn args_are_plain(args: &[Expr]) -> bool {
    !args.iter().any(|a| match a {
        // An implicit block handle appended to the call (`f(x) { … }`).
        Expr::MakeClosure { .. } => true,
        // Splat / forwarding / block-pass markers expand to an unknown
        // number of positional values.
        Expr::BuiltinCall { name, .. } => matches!(
            name.as_str(),
            "splat" | "double_splat" | "forward_args" | "block_pass"
        ),
        _ => false,
    })
}

struct ValidatorState<'m> {
    module: &'m Module,
    result: ValidationResult,
    /// Features actually observed in the module body.  Compared
    /// against the declared manifest at the end.
    observed: FeatureManifest,
    /// All function names declared in this module.  Used to validate
    /// `DirectCall` targets and to detect duplicates.
    function_names: HashSet<String>,
    /// Map from function name to its `(required_param_count, total_param_count,
    /// has_variadic_or_synthetic)` arity profile, used to check
    /// `DirectCall` arity (SIR10 default-param call-arity rule).  Built
    /// once in `collect_top_level_names` so the per-call check is O(1).
    /// `has_variadic_or_synthetic` is `true` when the callee has a
    /// `*rest`/`**opts` param or the synthetic trailing block param — in
    /// that case the strict upper-bound / required check is skipped
    /// (deferred to a later phase; see the DirectCall arm).
    fn_arity: HashMap<String, FnArity>,
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
            fn_arity: HashMap::new(),
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
            // Cache the call-arity profile (SIR10 default-param call-arity
            // rule).  `variadic` is set when the callee carries a
            // `*rest`/`**opts` param or the synthetic trailing block param
            // (`__sir_block__`); in that case the strict bounds are not
            // enforced at the call site (deferred — see the `DirectCall`
            // arm).  A duplicate name keeps the first profile, matching how
            // `function_names` reports-but-keeps the first binding.
            let variadic = f.params.iter().any(|p| {
                p.kind != ParamKind::Required || p.name == "__sir_block__"
            });
            self.fn_arity.entry(f.name.clone()).or_insert(FnArity {
                min: f.required_param_count(),
                max: f.params.len(),
                variadic,
            });
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
        // `scope_so_far` accumulates the parameter names *preceding* the
        // current one.  A default-value expression (`def f(a, b = a)`)
        // may refer to earlier parameters but not to itself or to
        // parameters declared later, so we validate each default against
        // the set built up to that point.  Captures are unavailable in a
        // default position, so we pass an empty capture set.
        let mut scope_so_far: HashSet<String> = HashSet::new();
        let no_captures: HashSet<String> = HashSet::new();
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
            // Default-value expression (SIR19): observe the feature and
            // validate the expression as if it appeared in the function's
            // parameter scope with the params declared so far in view.
            if let Some(default) = &p.default {
                self.observed.add(Feature::DefaultParams);
                let mut env = LocalEnv::new(&scope_so_far, &no_captures);
                self.check_expr(default, &mut env, 0);
            }
            scope_so_far.insert(p.name.clone());
        }

        // Variadic-parameter well-formedness (M3). A Ruby-faithful, v0-light
        // rule set over `kind`:
        //   - at most one `Rest` (`*rest`) parameter;
        //   - at most one `KwRest` (`**opts`) parameter;
        //   - ordering: required positionals come first, then the lone Rest,
        //     then the lone KwRest. A Required after a Rest, or anything after
        //     a KwRest, is a structural error (not a panic).
        // Truth table for the offending transitions we reject:
        //   prev\cur | Required | Rest     | KwRest
        //   Rest     | ERROR    | (dup)    | ok
        //   KwRest   | ERROR    | ERROR    | (dup)
        let mut rest_seen = false;
        let mut kwrest_seen = false;
        for p in &f.params {
            match p.kind {
                ParamKind::Rest => {
                    if rest_seen {
                        self.error(
                            format!("more than one rest parameter (`*{}`)", p.name),
                            &p.span,
                        );
                    }
                    if kwrest_seen {
                        self.error(
                            format!("rest parameter `*{}` must precede the keyword-rest parameter", p.name),
                            &p.span,
                        );
                    }
                    rest_seen = true;
                }
                ParamKind::KwRest => {
                    if kwrest_seen {
                        self.error(
                            format!("more than one keyword-rest parameter (`**{}`)", p.name),
                            &p.span,
                        );
                    }
                    kwrest_seen = true;
                }
                ParamKind::Required => {
                    // The reserved trailing block parameter (Q9e) is always
                    // Required and always appended last — after any variadic
                    // params — so it is exempt from the ordering rule.
                    if p.name == "__sir_block__" {
                        continue;
                    }
                    if kwrest_seen {
                        self.error(
                            format!("required parameter `{}` must precede the keyword-rest parameter", p.name),
                            &p.span,
                        );
                    } else if rest_seen {
                        self.error(
                            format!("required parameter `{}` must precede the rest parameter", p.name),
                            &p.span,
                        );
                    }
                }
            }
        }

        // Defaults must be **trailing** (SIR10 default-param call-arity rule).
        // A "hole" — a no-default `Required` param that follows a defaulted
        // `Required` param, e.g. `def f(a = 1, b)` — is rejected.  Why:
        //
        //   - The call-arity rule lets a caller omit trailing defaulted args,
        //     so `required_param_count()` counts only the *leading* no-default
        //     run.  For a hole that count stops at the first default (here 0),
        //     so the validator would accept `f()` / `f(0)`.
        //   - But `missing_defaults(n)` then returns params that include the
        //     trailing no-default `b`, breaking its documented guarantee that
        //     "every returned param carries a default" — a backend that
        //     unwraps `b.default` to fill it would panic.
        //
        // Enforcing "trailing defaults only" makes that guarantee true by
        // construction.  It matches Python and JavaScript exactly and the
        // common Ruby case; Ruby's required-after-optional form
        // (`def f(a = 1, b)`) is a DEFERRED v0 limitation.  The synthetic
        // trailing block param (`__sir_block__`) is always a no-default
        // `Required` appended last, so it is exempt.
        let mut defaulted_seen = false;
        for p in &f.params {
            if p.kind != ParamKind::Required || p.name == "__sir_block__" {
                continue;
            }
            if p.default.is_some() {
                defaulted_seen = true;
            } else if defaulted_seen {
                self.error(
                    format!(
                        "required parameter `{}` may not follow a defaulted parameter (defaults must be trailing)",
                        p.name
                    ),
                    &p.span,
                );
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
        self.check_stmt_seq(&b.stmts, env, depth);
        self.check_expr(&b.value, env, depth + 1);
        env.rewind(mark);
    }

    /// Validate a flat statement sequence (`&[Stmt]`) against `env`.
    ///
    /// Factored out of [`check_block`] (Phase 14b) so a class body
    /// (`Stmt::ClassDef.body`, a bare `Vec<Stmt>` with no trailing
    /// value slot) can reuse the exact same statement-accounting
    /// rules — parallel-`let` grouping, sequential `let*`, mutable
    /// `Assign`, loop/scope handling — without wrapping the body in a
    /// synthetic `Block`.  Callers are responsible for their own
    /// `env.mark()`/`env.rewind()` scoping around the call.
    fn check_stmt_seq(&mut self, stmts: &[Stmt], env: &mut LocalEnv, depth: usize) {
        // Walk statements in *groups*: a run of consecutive LetBinding
        // statements forms one parallel-let group whose RHS expressions
        // all evaluate in the scope BEFORE the group.  All names from
        // the group are added at once after every RHS has been checked.
        // LetStarBinding and ExprStmt break the run; LetStarBinding
        // adds its name immediately (sequential semantics).
        let mut i = 0;
        while i < stmts.len() {
            match &stmts[i] {
                Stmt::LetBinding { .. } => {
                    // Find the maximal run of LetBindings starting at i.
                    let mut j = i;
                    while j < stmts.len() && matches!(stmts[j], Stmt::LetBinding { .. }) {
                        j += 1;
                    }
                    // Check every RHS in the *outer* env (no new
                    // names added yet).
                    for k in i..j {
                        if let Stmt::LetBinding { value, sir_type, .. } = &stmts[k] {
                            self.check_expr(value, env, depth + 1);
                            if sir_type.is_some() {
                                self.observed.add(Feature::OptionalTypeAnnotations);
                            }
                        }
                    }
                    // Add every bound name to the env, all at once.
                    for k in i..j {
                        if let Stmt::LetBinding { name, .. } = &stmts[k] {
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
                Stmt::Assign { name, scope, value, span } => {
                    self.observed.add(Feature::MutableBindings);
                    self.check_expr(value, env, depth + 1);
                    self.check_varref(name, *scope, span, env);
                    i += 1;
                }
                Stmt::While { cond, body, .. } => {
                    self.observed.add(Feature::Loops);
                    self.check_expr(cond, env, depth + 1);
                    self.check_block(body, env, depth + 1);
                    i += 1;
                }
                Stmt::ForRange { var, start, stop, step, body, .. } => {
                    self.observed.add(Feature::Loops);
                    self.check_expr(start, env, depth + 1);
                    self.check_expr(stop, env, depth + 1);
                    self.check_expr(step, env, depth + 1);
                    // Loop variable is in scope inside the body only.
                    let inner_mark = env.mark();
                    env.add_local(var.clone());
                    self.check_block(body, env, depth + 1);
                    env.rewind(inner_mark);
                    i += 1;
                }
                Stmt::ForEach { var, iter, body, .. } => {
                    self.observed.add(Feature::Loops);
                    self.check_expr(iter, env, depth + 1);
                    let inner_mark = env.mark();
                    env.add_local(var.clone());
                    self.check_block(body, env, depth + 1);
                    env.rewind(inner_mark);
                    i += 1;
                }
                Stmt::SeqSet { seq, index, value, .. } => {
                    self.observed.add(Feature::Sequences);
                    self.check_expr(seq, env, depth + 1);
                    self.check_expr(index, env, depth + 1);
                    self.check_expr(value, env, depth + 1);
                    i += 1;
                }
                Stmt::MapSet { map, key, value, .. } => {
                    self.observed.add(Feature::Maps);
                    self.check_expr(map, env, depth + 1);
                    self.check_expr(key, env, depth + 1);
                    self.check_expr(value, env, depth + 1);
                    i += 1;
                }
                Stmt::ClassDef { body, span, .. } => {
                    // A class declaration adds a name to the module
                    // surface (the class itself) and contributes any
                    // statements in its body.  Phase 14b populates the
                    // body with the class's executable statements
                    // (method defs are hoisted to top-level Functions
                    // by the lowerer, so they don't appear here).
                    //
                    // We mark Feature::Classes and recurse into the
                    // body using a fresh local-env mark: class body
                    // names shouldn't leak into the surrounding
                    // statement stream.  We do NOT add the class's
                    // own name to the local env — classes are
                    // top-level/module-level names, not locals.
                    //
                    // The explicit depth guard bounds recursion for a
                    // pathological nest of `class A; class B; …` bodies
                    // (each level re-enters check_stmt_seq); it mirrors
                    // the MAX_IR_DEPTH guard check_block applies.
                    self.observed.add(Feature::Classes);
                    if self.check_depth(depth, span) {
                        i += 1;
                        continue;
                    }
                    let class_mark = env.mark();
                    self.check_stmt_seq(body, env, depth + 1);
                    env.rewind(class_mark);
                    i += 1;
                }
                Stmt::ModuleDef { body, span, .. } => {
                    // A module declaration (Ruby Phase 14d) is validated
                    // exactly like a class body: mark Feature::Modules,
                    // depth-guard the recursion, and walk the body in a
                    // fresh local-env scope so module-body names don't
                    // leak into the surrounding statement stream.
                    self.observed.add(Feature::Modules);
                    if self.check_depth(depth, span) {
                        i += 1;
                        continue;
                    }
                    let module_mark = env.mark();
                    self.check_stmt_seq(body, env, depth + 1);
                    env.rewind(module_mark);
                    i += 1;
                }
                Stmt::SingletonClassDef { body, span, .. } => {
                    // A singleton-class declaration (Ruby Phase 14e) is
                    // a class-opening construct → mark Feature::Classes,
                    // then depth-guard and walk the body in a fresh
                    // local-env scope, same as ClassDef.
                    self.observed.add(Feature::Classes);
                    if self.check_depth(depth, span) {
                        i += 1;
                        continue;
                    }
                    let singleton_mark = env.mark();
                    self.check_stmt_seq(body, env, depth + 1);
                    env.rewind(singleton_mark);
                    i += 1;
                }
                Stmt::TryCatch { body, rescues, ensure_body, span } => {
                    // Structured exception handling (Ruby Phase 16a).
                    // Mark Feature::Exceptions, then depth-guard and walk
                    // each block in a fresh local-env scope so block-local
                    // names don't leak into the surrounding stream.  A
                    // rescue's exception binding (`=> e`) is in scope for
                    // that clause's body only.  Exception class names are
                    // advisory (no symbol table), so they are not resolved.
                    self.observed.add(Feature::Exceptions);
                    if self.check_depth(depth, span) {
                        i += 1;
                        continue;
                    }
                    let body_mark = env.mark();
                    self.check_stmt_seq(body, env, depth + 1);
                    env.rewind(body_mark);
                    for r in rescues {
                        let rescue_mark = env.mark();
                        if let Some(bind) = &r.binding {
                            env.add_local(bind.clone());
                        }
                        self.check_stmt_seq(&r.body, env, depth + 1);
                        env.rewind(rescue_mark);
                    }
                    if let Some(ens) = ensure_body {
                        let ensure_mark = env.mark();
                        self.check_stmt_seq(ens, env, depth + 1);
                        env.rewind(ensure_mark);
                    }
                    i += 1;
                }
            }
        }
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
                if let Some(arity) = self.fn_arity.get(fn_name).copied() {
                    // SIR10 default-param call-arity rule.  Let R be the
                    // callee's required (leading no-default) param count and
                    // M its total param count.  A DirectCall is arity-valid
                    // iff R <= args.len() <= M; the omitted trailing params
                    // (positions args.len()..M) are then exactly the ones
                    // that carry defaults, so the backend can fill them.
                    //
                    // The strict R/M bounds only make sense when every
                    // argument contributes exactly one positional value.  We
                    // therefore skip the check entirely in two situations:
                    //
                    //   (a) the callee is variadic (`*rest`/`**opts`) or
                    //       carries the synthetic trailing block param — the
                    //       upper bound is then open / the block param is
                    //       supplied by the lowerer, not positionally; and
                    //   (b) the call carries an argument whose positional
                    //       count is not statically 1 — a splat / double-splat
                    //       / argument-forwarding marker (which expands to an
                    //       unknown number of values) or an implicit Ruby
                    //       block handle appended to the arg list (a trailing
                    //       `MakeClosure`, or a `block_pass`/`block_given`
                    //       marker).  Counting `args.len()` against the
                    //       declared params would be meaningless there.
                    //
                    // This keeps v0 scope tight (plain positional callees with
                    // trailing defaults) and is deliberately behaviour-neutral
                    // for every existing frontend lowering, which relied on the
                    // validator never checking DirectCall arity at all.
                    if !arity.variadic && args_are_plain(args) {
                        let n = args.len();
                        if n < arity.min {
                            self.error(
                                format!(
                                    "direct call to `{}` passes {} argument(s) but {} required",
                                    fn_name, n, arity.min
                                ),
                                e.span(),
                            );
                        } else if n > arity.max {
                            self.error(
                                format!(
                                    "direct call to `{}` passes {} argument(s) but the function takes at most {}",
                                    fn_name, n, arity.max
                                ),
                                e.span(),
                            );
                        }
                    }
                } else {
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

            // ── SIR16 additions ────────────────────────────────────
            Expr::FloatLit { .. } => {
                self.observed.add(Feature::Floats);
            }
            Expr::SeqLit { items, .. } => {
                self.observed.add(Feature::Sequences);
                for i in items {
                    self.check_expr(i, env, depth + 1);
                }
            }
            Expr::SeqIndex { seq, index, .. } => {
                self.observed.add(Feature::Sequences);
                self.check_expr(seq, env, depth + 1);
                self.check_expr(index, env, depth + 1);
            }
            Expr::SeqLen { seq, .. } => {
                self.observed.add(Feature::Sequences);
                self.check_expr(seq, env, depth + 1);
            }
            Expr::MapLit { entries, .. } => {
                self.observed.add(Feature::Maps);
                for entry in entries {
                    self.check_expr(&entry.key, env, depth + 1);
                    self.check_expr(&entry.value, env, depth + 1);
                }
            }
            Expr::MapGet { map, key, .. } => {
                self.observed.add(Feature::Maps);
                self.check_expr(map, env, depth + 1);
                self.check_expr(key, env, depth + 1);
            }
            Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
                self.observed.add(Feature::ShortCircuit);
                self.check_expr(lhs, env, depth + 1);
                self.check_expr(rhs, env, depth + 1);
            }
            // ── SIR18: string interpolation ────────────────────────
            Expr::StrConcat { parts, span } => {
                self.observed.add(Feature::StringInterpolation);
                // A concat is only meaningful with at least two parts;
                // a degenerate one-part concat signals a frontend bug
                // (it should have emitted the bare part instead).
                if parts.len() < 2 {
                    self.error(
                        format!(
                            "str-concat needs at least 2 parts, got {}",
                            parts.len()
                        ),
                        span,
                    );
                }
                for p in parts {
                    self.check_expr(p, env, depth + 1);
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
            Scope::Instance => {
                // Instance variables (Ruby `@x`) need no prior
                // declaration — reading an unset `@x` yields nil — so
                // there is nothing to resolve against the local env.
                // We only record that the feature is in use so the
                // manifest comparison stays honest.
                self.observed.add(Feature::InstanceVars);
            }
            Scope::ClassVar => {
                // Class variables (Ruby `@@x`) likewise need no prior
                // declaration; record the feature only.
                self.observed.add(Feature::ClassVars);
            }
            Scope::Const => {
                // Constants (Ruby `FOO` / `MyClass`) resolve against the
                // constant scope, not a `let` binding — no local
                // resolution; record the feature only.
                self.observed.add(Feature::Constants);
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

    /// Build a single-function module whose function has `params` and a
    /// trivial nil body — for exercising the M3 variadic ordering rules.
    fn module_with_params(params: Vec<Param>) -> Module {
        let mut m = empty_module(FeatureManifest::from_features(&[Feature::DynamicTyping]));
        m.functions.push(Function {
            name: "f".into(),
            params,
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
        m
    }

    fn p(name: &str, kind: ParamKind) -> Param {
        Param { name: name.into(), sir_type: None, kind, default: None, span: s() }
    }

    #[test]
    fn variadic_params_in_canonical_order_are_valid() {
        // def f(a, *rest, **opts) — required, then one Rest, then one KwRest.
        let m = module_with_params(vec![
            p("a", ParamKind::Required),
            p("rest", ParamKind::Rest),
            p("opts", ParamKind::KwRest),
        ]);
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn two_rest_params_is_error() {
        let m = module_with_params(vec![
            p("a", ParamKind::Rest),
            p("b", ParamKind::Rest),
        ]);
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("more than one rest")));
    }

    #[test]
    fn two_kwrest_params_is_error() {
        let m = module_with_params(vec![
            p("a", ParamKind::KwRest),
            p("b", ParamKind::KwRest),
        ]);
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("more than one keyword-rest")));
    }

    #[test]
    fn kwrest_before_rest_is_error() {
        // def f(**opts, *rest) — Rest must precede KwRest.
        let m = module_with_params(vec![
            p("opts", ParamKind::KwRest),
            p("rest", ParamKind::Rest),
        ]);
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("must precede the keyword-rest")));
    }

    /// SIR19: a parameter carrying a default-value expression validates
    /// OK and causes the validator to observe `Feature::DefaultParams`.
    #[test]
    fn param_with_default_validates_and_observes_feature() {
        // def f(a = 1) — one required param with a default literal `1`.
        let mut m =
            empty_module(FeatureManifest::from_features(&[
                Feature::DynamicTyping,
                Feature::DefaultParams,
            ]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![Param {
                name: "a".into(),
                sir_type: None,
                kind: ParamKind::Required,
                default: Some(Box::new(Expr::IntLit { value: 1, span: s() })),
                span: s(),
            }],
            return_type: None,
            captures: vec![],
            body: Block { stmts: vec![], value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    /// A default that uses an unsupported/undeclared feature (here a
    /// `StrLit`, which declares `Feature::Strings`) is observed through
    /// the default expression — proving the validator recurses into the
    /// default like any other expression.
    #[test]
    fn default_expr_features_are_observed() {
        // def f(a = "x") — manifest must declare Strings (from the default)
        // as well as DefaultParams, or validation fails.
        let mut m =
            empty_module(FeatureManifest::from_features(&[
                Feature::DynamicTyping,
                Feature::DefaultParams,
                Feature::Strings,
            ]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![Param {
                name: "a".into(),
                sir_type: None,
                kind: ParamKind::Required,
                default: Some(Box::new(Expr::StrLit { value: "x".into(), span: s() })),
                span: s(),
            }],
            return_type: None,
            captures: vec![],
            body: Block { stmts: vec![], value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    /// A default expression may reference an earlier parameter
    /// (`def f(a, b = a)`); the validator resolves the `VarRef` against
    /// the params declared so far.
    #[test]
    fn default_expr_may_reference_earlier_param() {
        let mut m =
            empty_module(FeatureManifest::from_features(&[
                Feature::DynamicTyping,
                Feature::DefaultParams,
            ]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![
                Param { name: "a".into(), sir_type: None, kind: ParamKind::Required, default: None, span: s() },
                Param {
                    name: "b".into(),
                    sir_type: None,
                    kind: ParamKind::Required,
                    default: Some(Box::new(Expr::VarRef {
                        name: "a".into(),
                        scope: Scope::Param,
                        span: s(),
                    })),
                    span: s(),
                },
            ],
            return_type: None,
            captures: vec![],
            body: Block { stmts: vec![], value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    /// A default expression that references a *later* parameter is a
    /// scope error — only params declared so far are in view.
    #[test]
    fn default_expr_cannot_reference_later_param() {
        let mut m =
            empty_module(FeatureManifest::from_features(&[
                Feature::DynamicTyping,
                Feature::DefaultParams,
            ]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![
                Param {
                    name: "a".into(),
                    sir_type: None,
                    kind: ParamKind::Required,
                    // references `b`, which is declared *after* `a`.
                    default: Some(Box::new(Expr::VarRef {
                        name: "b".into(),
                        scope: Scope::Param,
                        span: s(),
                    })),
                    span: s(),
                },
                Param { name: "b".into(), sir_type: None, kind: ParamKind::Required, default: None, span: s() },
            ],
            return_type: None,
            captures: vec![],
            body: Block { stmts: vec![], value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(!r.is_ok(), "expected scope error for forward reference");
    }

    // ── SIR10 default-param call-arity (P2a) ───────────────────────────
    //
    // Helpers and tests for the rule: a DirectCall to a known function is
    // arity-valid iff R <= args.len() <= M, where R is the callee's
    // required (leading no-default) param count and M is its total param
    // count.  Omitting a trailing defaulted arg is OK; omitting a required
    // arg or over-supplying is an error.

    /// A param with an integer-literal default (`name = 1`).
    fn p_default(name: &str) -> Param {
        Param {
            name: name.into(),
            sir_type: None,
            kind: ParamKind::Required,
            default: Some(Box::new(Expr::IntLit { value: 1, span: s() })),
            span: s(),
        }
    }

    /// Build a two-function module: a callee `f` with `callee_params`, and
    /// a caller `g` whose body is `f(<n_args> int literals>)` via a
    /// `DirectCall`.  The manifest declares `DefaultParams` so a defaulted
    /// callee passes the manifest check.
    fn module_calling_f(callee_params: Vec<Param>, n_args: usize) -> Module {
        let mut m = empty_module(FeatureManifest::from_features(&[
            Feature::DynamicTyping,
            Feature::DefaultParams,
        ]));
        m.functions.push(Function {
            name: "f".into(),
            params: callee_params,
            return_type: None,
            captures: vec![],
            body: Block { stmts: vec![], value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let args: Vec<Expr> = (0..n_args)
            .map(|i| Expr::IntLit { value: i as i64, span: s() })
            .collect();
        m.functions.push(Function {
            name: "g".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::DirectCall {
                    fn_name: "f".into(),
                    args,
                    effects: EffectSet::PURE,
                    span: s(),
                },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        m
    }

    #[test]
    fn direct_call_exact_arity_is_valid() {
        // def f(a, b = 1); f(0, 1) — R=1, M=2, args=2 → R<=2<=M.
        let m = module_calling_f(vec![p("a", ParamKind::Required), p_default("b")], 2);
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn direct_call_omitting_trailing_default_is_valid() {
        // def f(a, b = 1); f(0) — omit the trailing defaulted `b`. R=1,
        // M=2, args=1 → R<=1<=M, and the omitted param (`b`) has a default.
        let m = module_calling_f(vec![p("a", ParamKind::Required), p_default("b")], 1);
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn direct_call_omitting_all_defaults_is_valid() {
        // def f(a = 1, b = 1); f() — both params defaulted, R=0, M=2,
        // args=0 → 0<=0<=2.
        let m = module_calling_f(vec![p_default("a"), p_default("b")], 0);
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn direct_call_omitting_required_arg_is_error() {
        // def f(a, b = 1); f() — omits the *required* `a`. R=1, args=0 →
        // 0 < R → error.
        let m = module_calling_f(vec![p("a", ParamKind::Required), p_default("b")], 0);
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(
            r.errors().any(|i| i.message.contains("required")),
            "expected a 'required' arity error, got {:?}",
            r.issues
        );
    }

    #[test]
    fn direct_call_too_many_args_is_error() {
        // def f(a, b = 1); f(0, 1, 2) — three args but M=2. args > M → error.
        let m = module_calling_f(vec![p("a", ParamKind::Required), p_default("b")], 3);
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(
            r.errors().any(|i| i.message.contains("at most")),
            "expected an 'at most' arity error, got {:?}",
            r.issues
        );
    }

    #[test]
    fn direct_call_to_no_default_function_still_requires_exact_arity() {
        // Behaviour-neutral check: a default-less callee `def f(a, b)` keeps
        // exact-arity semantics — f(0) is now an error (R=M=2), f(0,1) is ok.
        let short = module_calling_f(
            vec![p("a", ParamKind::Required), p("b", ParamKind::Required)],
            1,
        );
        assert!(!validate(&short).is_ok(), "f(0) for def f(a,b) must error");
        let exact = module_calling_f(
            vec![p("a", ParamKind::Required), p("b", ParamKind::Required)],
            2,
        );
        assert!(
            validate(&exact).is_ok(),
            "f(0,1) for def f(a,b) must validate"
        );
    }

    #[test]
    fn direct_call_to_variadic_callee_skips_strict_bounds() {
        // def f(a, *rest); f(0,1,2,3) — a `*rest` removes the upper bound,
        // so over-supply relative to the positional count is accepted
        // (strict bounds are deferred for variadic callees).
        let m = module_calling_f(
            vec![p("a", ParamKind::Required), p("rest", ParamKind::Rest)],
            4,
        );
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok for variadic callee, got {:?}", r.issues);
    }

    #[test]
    fn direct_call_with_trailing_block_handle_skips_arity_check() {
        // Ruby block-passing convention: `helper(2) { … }` lowers to a
        // DirectCall whose args are [2, MakeClosure(__block_0)] — i.e. one
        // extra trailing block handle is appended even though `helper`'s
        // declared params do NOT include a block param.  The static arg
        // count (2) exceeds M (1), but because the trailing arg is an
        // implicit block handle the strict bounds are skipped — this must
        // still validate (behaviour-neutral for the Ruby frontend).
        let mut m = empty_module(FeatureManifest::from_features(&[
            Feature::DynamicTyping,
            Feature::Closures,
        ]));
        m.functions.push(Function {
            name: "helper".into(),
            params: vec![p("x", ParamKind::Required)],
            return_type: None,
            captures: vec![],
            body: Block { stmts: vec![], value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        // The hoisted block fn (so MakeClosure references a known function).
        m.functions.push(Function {
            name: "__block_0".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block { stmts: vec![], value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        m.functions.push(Function {
            name: "outer".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::DirectCall {
                    fn_name: "helper".into(),
                    args: vec![
                        Expr::IntLit { value: 2, span: s() },
                        Expr::MakeClosure {
                            fn_name: "__block_0".into(),
                            captures: vec![],
                            span: s(),
                        },
                    ],
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
        assert!(r.is_ok(), "block-passing call must validate, got {:?}", r.issues);
    }

    #[test]
    fn direct_call_with_splat_arg_skips_arity_check() {
        // `helper(*arr)` → DirectCall(helper, [BuiltinCall("splat", [arr])]).
        // A splat expands to an unknown count, so even though args.len()==1
        // and helper takes 2 required params, the arity check is skipped.
        let mut m = empty_module(FeatureManifest::from_features(&[Feature::DynamicTyping]));
        m.functions.push(Function {
            name: "helper".into(),
            params: vec![p("a", ParamKind::Required), p("b", ParamKind::Required)],
            return_type: None,
            captures: vec![],
            body: Block { stmts: vec![], value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        m.functions.push(Function {
            name: "g".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::DirectCall {
                    fn_name: "helper".into(),
                    args: vec![Expr::BuiltinCall {
                        name: "splat".into(),
                        args: vec![Expr::NilLit { span: s() }],
                        effects: EffectSet::PURE,
                        span: s(),
                    }],
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
        assert!(r.is_ok(), "splat call must validate, got {:?}", r.issues);
    }

    #[test]
    fn required_param_count_helper() {
        // def f(a, b, c = 1, d = 1) → required_param_count() == 2.
        let f = Function {
            name: "f".into(),
            params: vec![
                p("a", ParamKind::Required),
                p("b", ParamKind::Required),
                p_default("c"),
                p_default("d"),
            ],
            return_type: None,
            captures: vec![],
            body: Block { stmts: vec![], value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        };
        assert_eq!(f.required_param_count(), 2);
        // missing_defaults: the trailing params a caller omits.
        assert_eq!(f.missing_defaults(4).len(), 0);
        let omitted_one = f.missing_defaults(3);
        assert_eq!(omitted_one.len(), 1);
        assert_eq!(omitted_one[0].name, "d");
        let omitted_two = f.missing_defaults(2);
        assert_eq!(omitted_two.len(), 2);
        assert_eq!(omitted_two[0].name, "c");
        assert_eq!(omitted_two[1].name, "d");
        // Over-supply clamps rather than panicking.
        assert_eq!(f.missing_defaults(99).len(), 0);
    }

    /// Build a single-function module `def f(<params>)` with a nil body and
    /// a manifest declaring DynamicTyping + DefaultParams — for exercising
    /// the trailing-defaults-only rule.
    fn module_with_default_params(params: Vec<Param>) -> Module {
        let mut m = empty_module(FeatureManifest::from_features(&[
            Feature::DynamicTyping,
            Feature::DefaultParams,
        ]));
        m.functions.push(Function {
            name: "f".into(),
            params,
            return_type: None,
            captures: vec![],
            body: Block { stmts: vec![], value: Expr::NilLit { span: s() }, span: s() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        m
    }

    #[test]
    fn required_after_defaulted_param_is_a_hole_error() {
        // def f(a = 1, b) — `b` is a required param following a defaulted
        // one.  This "hole" is rejected so `missing_defaults` only ever
        // returns params that carry a default.
        let m = module_with_default_params(vec![p_default("a"), p("b", ParamKind::Required)]);
        let r = validate(&m);
        assert!(!r.is_ok(), "a hole must fail validation");
        assert!(
            r.errors().any(|i| i
                .message
                .contains("may not follow a defaulted parameter")),
            "expected the trailing-defaults error, got {:?}",
            r.issues
        );
    }

    #[test]
    fn trailing_defaults_validate() {
        // def f(a, b = 1, c = 2) — all defaults are trailing; valid.
        let m = module_with_default_params(vec![
            p("a", ParamKind::Required),
            p_default("b"),
            p_default("c"),
        ]);
        let r = validate(&m);
        assert!(r.is_ok(), "trailing defaults must validate, got {:?}", r.issues);
    }

    #[test]
    fn block_param_after_defaulted_param_is_exempt() {
        // def f(a = 1) { yield } → params [a=1, __sir_block__].  The
        // synthetic block param is a no-default Required appended last, so
        // it must NOT trip the trailing-defaults rule.
        let m = module_with_default_params(vec![
            p_default("a"),
            p("__sir_block__", ParamKind::Required),
        ]);
        let r = validate(&m);
        assert!(
            r.is_ok(),
            "block param after a default must be exempt, got {:?}",
            r.issues
        );
    }

    #[test]
    fn required_after_rest_is_error() {
        // def f(*rest, a) — a required positional after the rest param.
        let m = module_with_params(vec![
            p("rest", ParamKind::Rest),
            p("a", ParamKind::Required),
        ]);
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("must precede the rest")));
    }

    #[test]
    fn reserved_block_param_after_rest_is_exempt() {
        // The Q9e trailing block param is always Required and always last,
        // appearing after any variadic params — and must NOT trigger the
        // ordering rule. def f(*rest) { yield } → params [*rest, __sir_block__].
        let m = module_with_params(vec![
            p("rest", ParamKind::Rest),
            p("__sir_block__", ParamKind::Required),
        ]);
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
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
    fn float_lit_observes_floats_feature() {
        // Module uses a float literal but doesn't declare Floats →
        // error.
        let mut m = empty_module(FeatureManifest::new());
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::FloatLit { value: 3.14, span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("floats")));
    }

    #[test]
    fn while_loop_observes_loops_feature() {
        let mut m = empty_module(FeatureManifest::from_features(&[Feature::Loops]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::While {
                    cond: Expr::BoolLit { value: false, span: s() },
                    body: Block {
                        stmts: vec![],
                        value: Expr::NilLit { span: s() },
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
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn for_range_introduces_loop_var_in_body_scope() {
        // `for i in range(0, 10, 1): print(i)` — `i` must be in scope
        // inside the body.
        let mut m = empty_module(FeatureManifest::from_features(&[Feature::Loops]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::ForRange {
                    var: "i".into(),
                    start: Expr::IntLit { value: 0, span: s() },
                    stop: Expr::IntLit { value: 10, span: s() },
                    step: Expr::IntLit { value: 1, span: s() },
                    body: Block {
                        stmts: vec![],
                        value: Expr::VarRef {
                            name: "i".into(),
                            scope: Scope::Local,
                            span: s(),
                        },
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
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn for_range_loop_var_is_not_in_scope_after_loop() {
        // After the for-range, `i` is gone.
        let mut m = empty_module(FeatureManifest::from_features(&[Feature::Loops]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::ForRange {
                    var: "i".into(),
                    start: Expr::IntLit { value: 0, span: s() },
                    stop: Expr::IntLit { value: 10, span: s() },
                    step: Expr::IntLit { value: 1, span: s() },
                    body: Block {
                        stmts: vec![],
                        value: Expr::NilLit { span: s() },
                        span: s(),
                    },
                    span: s(),
                }],
                value: Expr::VarRef {
                    name: "i".into(),
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
        assert!(!r.is_ok(), "expected error for `i` out of scope, got ok");
    }

    #[test]
    fn logical_and_observes_short_circuit_feature() {
        let mut m = empty_module(FeatureManifest::from_features(&[Feature::ShortCircuit]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::LogicalAnd {
                    lhs: Box::new(Expr::BoolLit { value: true, span: s() }),
                    rhs: Box::new(Expr::BoolLit { value: false, span: s() }),
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

    #[test]
    fn str_concat_observes_string_interpolation_feature() {
        // Phase 20b — a well-formed two-part `StrConcat` validates when
        // the manifest declares `StringInterpolation`.
        let mut m =
            empty_module(FeatureManifest::from_features(&[Feature::StringInterpolation, Feature::Strings]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::StrConcat {
                    parts: vec![
                        Expr::StrLit { value: "a".into(), span: s() },
                        Expr::StrLit { value: "b".into(), span: s() },
                    ],
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

    #[test]
    fn str_concat_with_fewer_than_two_parts_is_rejected() {
        // Phase 20b — a one-part concat is degenerate; the frontend
        // should have emitted the bare part instead.  The validator
        // flags it so a buggy lowerer is caught early.
        let mut m =
            empty_module(FeatureManifest::from_features(&[Feature::StringInterpolation, Feature::Strings]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::StrConcat {
                    parts: vec![Expr::StrLit { value: "lonely".into(), span: s() }],
                    span: s(),
                },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(!r.is_ok(), "expected error for 1-part str-concat, got ok");
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

    // -----------------------------------------------------------------
    // Phase 14b (FC) — `Stmt::ClassDef.body` is now a populated
    // `Vec<Stmt>`; the validator walks it via `check_stmt_seq`.
    // -----------------------------------------------------------------

    /// Wrap a single `ClassDef` statement in a one-function module
    /// declaring `Feature::Classes`.
    fn module_with_class_body(name: &str, body: Vec<Stmt>) -> Module {
        let mut m = empty_module(FeatureManifest::from_features(&[Feature::Classes]));
        m.functions.push(Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::ClassDef {
                    name: name.into(),
                    superclass: None,
                    body,
                    span: s(),
                }],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        m
    }

    #[test]
    fn class_def_body_with_let_binding_validates() {
        // A class body holding a single LetBinding is now walked by the
        // validator (Phase 14a no-op'd the body loop) and accepted.
        let m = module_with_class_body(
            "Foo",
            vec![Stmt::LetBinding {
                name: "MAX".into(),
                sir_type: None,
                value: Expr::IntLit { value: 10, span: s() },
                span: s(),
            }],
        );
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn class_def_body_undefined_varref_is_error() {
        // Proves the body is *actually validated* now: a VarRef to an
        // undefined local inside the class body must be reported as an
        // error.  Under the Phase 14a no-op loop this would have been
        // silently accepted.
        let m = module_with_class_body(
            "Foo",
            vec![Stmt::ExprStmt {
                expr: Expr::VarRef {
                    name: "ghost".into(),
                    scope: Scope::Local,
                    span: s(),
                },
                span: s(),
            }],
        );
        let r = validate(&m);
        assert!(
            !r.is_ok(),
            "expected undefined-varref error from class body, got {:?}",
            r.issues
        );
    }

    #[test]
    fn class_def_body_local_does_not_leak_to_sibling() {
        // A binding introduced inside the class body must not be
        // visible to statements *after* the class (the body is scoped
        // by its own env mark/rewind).  Here `INNER` is bound inside
        // the class, then referenced as a sibling statement after it —
        // which must error.
        let mut m = empty_module(FeatureManifest::from_features(&[Feature::Classes]));
        m.functions.push(Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![
                    Stmt::ClassDef {
                        name: "Foo".into(),
                        superclass: None,
                        body: vec![Stmt::LetBinding {
                            name: "INNER".into(),
                            sir_type: None,
                            value: Expr::IntLit { value: 1, span: s() },
                            span: s(),
                        }],
                        span: s(),
                    },
                    Stmt::ExprStmt {
                        expr: Expr::VarRef {
                            name: "INNER".into(),
                            scope: Scope::Local,
                            span: s(),
                        },
                        span: s(),
                    },
                ],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(
            !r.is_ok(),
            "class-body local INNER must not leak to sibling stmt, got {:?}",
            r.issues
        );
    }

    // -----------------------------------------------------------------
    // SIR17 Phase 14d — `Stmt::ModuleDef` validation (mirrors ClassDef).
    // -----------------------------------------------------------------

    #[test]
    fn module_def_body_with_let_binding_validates() {
        // A module body holding a LetBinding is walked by the validator
        // and accepted; the module declares Feature::Modules.
        let mut m = empty_module(FeatureManifest::from_features(&[Feature::Modules]));
        m.functions.push(Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::ModuleDef {
                    name: "Config".into(),
                    body: vec![Stmt::LetBinding {
                        name: "V".into(),
                        sir_type: None,
                        value: Expr::IntLit { value: 3, span: s() },
                        span: s(),
                    }],
                    span: s(),
                }],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn module_def_body_undefined_varref_is_error() {
        // Proves the module body is actually validated: a VarRef to an
        // undefined local inside the module body must be reported.
        let mut m = empty_module(FeatureManifest::from_features(&[Feature::Modules]));
        m.functions.push(Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::ModuleDef {
                    name: "M".into(),
                    body: vec![Stmt::ExprStmt {
                        expr: Expr::VarRef {
                            name: "ghost".into(),
                            scope: Scope::Local,
                            span: s(),
                        },
                        span: s(),
                    }],
                    span: s(),
                }],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(
            !r.is_ok(),
            "expected undefined-varref error from module body, got {:?}",
            r.issues
        );
    }

    #[test]
    fn module_def_without_manifest_feature_is_error() {
        // A ModuleDef present but `Feature::Modules` not declared →
        // the used-but-undeclared check fires.
        let mut m = empty_module(FeatureManifest::new());
        m.functions.push(Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::ModuleDef {
                    name: "M".into(),
                    body: vec![],
                    span: s(),
                }],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(!r.is_ok(), "expected missing-Modules-feature error");
        assert!(r.errors().any(|i| i.message.contains("modules")));
    }

    // -----------------------------------------------------------------
    // SIR17 Phase 14e — `Stmt::SingletonClassDef` validation.
    // -----------------------------------------------------------------

    #[test]
    fn singleton_class_def_body_with_let_binding_validates() {
        // A singleton-class body holding a LetBinding is walked and
        // accepted; the module declares Feature::Classes.
        let mut m = empty_module(FeatureManifest::from_features(&[Feature::Classes]));
        m.functions.push(Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::SingletonClassDef {
                    target: "self".into(),
                    body: vec![Stmt::LetBinding {
                        name: "X".into(),
                        sir_type: None,
                        value: Expr::IntLit { value: 1, span: s() },
                        span: s(),
                    }],
                    span: s(),
                }],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn singleton_class_def_body_undefined_varref_is_error() {
        // Proves the singleton body is actually validated.
        let mut m = empty_module(FeatureManifest::from_features(&[Feature::Classes]));
        m.functions.push(Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::SingletonClassDef {
                    target: "self".into(),
                    body: vec![Stmt::ExprStmt {
                        expr: Expr::VarRef {
                            name: "ghost".into(),
                            scope: Scope::Local,
                            span: s(),
                        },
                        span: s(),
                    }],
                    span: s(),
                }],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(
            !r.is_ok(),
            "expected undefined-varref error from singleton body, got {:?}",
            r.issues
        );
    }

    // -----------------------------------------------------------------
    // SIR17 Phase 15a — `Scope::Instance` (instance variables).
    // -----------------------------------------------------------------

    /// Build a one-function module whose body value is a single
    /// instance-var ref, declaring the given features.
    fn module_with_instance_ref(features: &[Feature]) -> Module {
        let mut m = empty_module(FeatureManifest::from_features(features));
        m.functions.push(Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::VarRef {
                    name: "@x".into(),
                    scope: Scope::Instance,
                    span: s(),
                },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        m
    }

    #[test]
    fn instance_var_ref_needs_no_declaration() {
        // An instance-var ref is accepted with no prior `let`/param —
        // reading an unset `@x` is nil in Ruby.  Module declares
        // `InstanceVars`.
        let m = module_with_instance_ref(&[Feature::InstanceVars]);
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn instance_var_ref_without_manifest_feature_is_error() {
        // The validator observes `InstanceVars` from the Instance-scoped
        // ref; if the manifest doesn't declare it, the
        // used-but-undeclared check fires.
        let m = module_with_instance_ref(&[]);
        let r = validate(&m);
        assert!(!r.is_ok(), "expected missing-InstanceVars-feature error");
        assert!(r.errors().any(|i| i.message.contains("instance-vars")));
    }

    // -----------------------------------------------------------------
    // SIR17 Phase 15b — `Scope::ClassVar` (class variables).
    // -----------------------------------------------------------------

    /// Build a one-function module whose body value is a single
    /// class-var ref, declaring the given features.
    fn module_with_class_var_ref(features: &[Feature]) -> Module {
        let mut m = empty_module(FeatureManifest::from_features(features));
        m.functions.push(Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::VarRef {
                    name: "@@count".into(),
                    scope: Scope::ClassVar,
                    span: s(),
                },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        m
    }

    #[test]
    fn class_var_ref_needs_no_declaration() {
        // A class-var ref is accepted with no prior `let`/param —
        // reading an unset `@@x` is nil in Ruby.  Module declares
        // `ClassVars`.
        let m = module_with_class_var_ref(&[Feature::ClassVars]);
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn class_var_ref_without_manifest_feature_is_error() {
        // The validator observes `ClassVars` from the ClassVar-scoped
        // ref; if the manifest doesn't declare it, the
        // used-but-undeclared check fires.
        let m = module_with_class_var_ref(&[]);
        let r = validate(&m);
        assert!(!r.is_ok(), "expected missing-ClassVars-feature error");
        assert!(r.errors().any(|i| i.message.contains("class-vars")));
    }

    // -----------------------------------------------------------------
    // SIR17 Phase 15c — `Scope::Const` (constants).
    // -----------------------------------------------------------------

    /// Build a one-function module whose body value is a single
    /// constant ref, declaring the given features.
    fn module_with_const_ref(features: &[Feature]) -> Module {
        let mut m = empty_module(FeatureManifest::from_features(features));
        m.functions.push(Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::VarRef {
                    name: "MAX".into(),
                    scope: Scope::Const,
                    span: s(),
                },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        m
    }

    #[test]
    fn const_ref_needs_no_declaration() {
        // A constant ref is accepted with no prior `let`/param — a
        // constant resolves against the constant scope, not a local
        // binding.  Module declares `Constants`.
        let m = module_with_const_ref(&[Feature::Constants]);
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn const_ref_without_manifest_feature_is_error() {
        // The validator observes `Constants` from the Const-scoped ref;
        // if the manifest doesn't declare it, the used-but-undeclared
        // check fires.
        let m = module_with_const_ref(&[]);
        let r = validate(&m);
        assert!(!r.is_ok(), "expected missing-Constants-feature error");
        assert!(r.errors().any(|i| i.message.contains("constants")));
    }

    // -----------------------------------------------------------------
    // SIR17 Phase 16a — `Stmt::TryCatch` (exception handling).
    // -----------------------------------------------------------------

    /// Build a one-function module whose body is a single `TryCatch`
    /// (`begin; let _t=1; rescue Foo => e; let _r=e; ensure; let _e=1;
    /// end`), declaring the given features.  The rescue body references
    /// the bound exception `e`, exercising the binding's scope.
    fn module_with_try_catch(features: &[Feature]) -> Module {
        let mut m = empty_module(FeatureManifest::from_features(features));
        let lb = |name: &str, value: Expr| Stmt::LetBinding {
            name: name.into(),
            sir_type: None,
            value,
            span: s(),
        };
        m.functions.push(Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::TryCatch {
                    body: vec![lb("_t", Expr::IntLit { value: 1, span: s() })],
                    rescues: vec![RescueClause {
                        exception_types: vec!["Foo".into()],
                        binding: Some("e".into()),
                        // The rescue body reads the bound `e` — must resolve.
                        body: vec![lb(
                            "_r",
                            Expr::VarRef { name: "e".into(), scope: Scope::Local, span: s() },
                        )],
                        span: s(),
                    }],
                    ensure_body: Some(vec![lb("_e", Expr::IntLit { value: 1, span: s() })]),
                    span: s(),
                }],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        m
    }

    #[test]
    fn try_catch_validates_and_binding_is_in_scope() {
        // A try/catch validates when the manifest declares `Exceptions`,
        // and the rescue binding `e` resolves inside the rescue body.
        let m = module_with_try_catch(&[Feature::Exceptions]);
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn try_catch_without_manifest_feature_is_error() {
        // The validator observes `Exceptions` from the TryCatch; if the
        // manifest doesn't declare it, the used-but-undeclared check fires.
        let m = module_with_try_catch(&[]);
        let r = validate(&m);
        assert!(!r.is_ok(), "expected missing-Exceptions-feature error");
        assert!(r.errors().any(|i| i.message.contains("exceptions")));
    }

    #[test]
    fn try_catch_binding_does_not_leak_past_rescue() {
        // The rescue binding `e` is scoped to its clause body only — a
        // reference to `e` in the ensure body is undefined.
        let mut m = empty_module(FeatureManifest::from_features(&[Feature::Exceptions]));
        m.functions.push(Function {
            name: "main".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::TryCatch {
                    body: vec![],
                    rescues: vec![RescueClause {
                        exception_types: vec![],
                        binding: Some("e".into()),
                        body: vec![],
                        span: s(),
                    }],
                    // ensure references `e` — out of scope → error.
                    ensure_body: Some(vec![Stmt::LetBinding {
                        name: "_x".into(),
                        sir_type: None,
                        value: Expr::VarRef { name: "e".into(), scope: Scope::Local, span: s() },
                        span: s(),
                    }]),
                    span: s(),
                }],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(!r.is_ok(), "expected `e` out-of-scope error in ensure body");
    }
}
