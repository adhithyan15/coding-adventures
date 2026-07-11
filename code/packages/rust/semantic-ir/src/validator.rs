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
    /// `true` while checking the *immediate* arguments of a call node
    /// (DirectCall / IndirectCall / MakeClosure).  A `KeywordArg` is only
    /// well-placed as such an immediate argument; the flag lets the
    /// `check_expr` `KeywordArg` arm distinguish a legitimate call-position
    /// keyword from a misplaced one (e.g. `(+ (kw a 1))` or a keyword arg
    /// nested inside another expression).  It is set true just around the
    /// per-arg walk of a call and reset to false before recursing into any
    /// argument's sub-expressions.
    in_call_args: bool,
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
            in_call_args: false,
        }
    }

    /// Report a depth-overflow error once.  Returns `true` if the
    /// caller should stop recursing.
    fn check_depth(&mut self, depth: usize, span: &Span) -> bool {
        if depth >= MAX_IR_DEPTH {
            if !self.depth_overflow_reported {
                self.depth_overflow_reported = true;
                self.error(
                    format!("expression nesting exceeds MAX_IR_DEPTH ({})", MAX_IR_DEPTH),
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
                self.error(format!("duplicate function name `{}`", f.name), &f.span);
            }
            // Cache the call-arity profile (SIR10 default-param call-arity
            // rule).  `variadic` is set when the callee carries a
            // `*rest`/`**opts` param or the synthetic trailing block param
            // (`__sir_block__`); in that case the strict bounds are not
            // enforced at the call site (deferred — see the `DirectCall`
            // arm).  A duplicate name keeps the first profile, matching how
            // `function_names` reports-but-keeps the first binding.
            let variadic = f
                .params
                .iter()
                .any(|p| p.kind != ParamKind::Required || p.name == "__sir_block__");
            self.fn_arity.entry(f.name.clone()).or_insert(FnArity {
                min: f.required_param_count(),
                max: f.params.len(),
                variadic,
            });
        }
        for g in &self.module.globals {
            if !self.global_names.insert(g.name.clone()) {
                self.error(format!("duplicate global name `{}`", g.name), &g.span);
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
                self.error(format!("duplicate parameter `{}`", p.name), &p.span);
            }
            if p.sir_type.is_some() {
                self.observed.add(Feature::OptionalTypeAnnotations);
            } else {
                self.observed.add(Feature::DynamicTyping);
            }
            // Keyword parameter (KW1): observe the feature.  A `Keyword`
            // param is matched by name, not position; whether it is
            // REQUIRED (`default == None`) or OPTIONAL (`default == Some`)
            // rides on the same `default` field checked just below, so we
            // do not special-case the default handling here — a keyword
            // default is validated exactly like a positional default.
            if p.kind == ParamKind::Keyword {
                self.observed.add(Feature::KeywordParams);
            }
            // Default-value expression (SIR19): observe the feature and
            // validate the expression as if it appeared in the function's
            // parameter scope with the params declared so far in view.
            //
            // A default on a `Keyword` param means "optional keyword"; it
            // triggers `KeywordParams` (above), NOT `DefaultParams` —
            // `DefaultParams` is specifically the *positional* trailing
            // default feature.  Only observe `DefaultParams` for a
            // non-keyword default.
            if let Some(default) = &p.default {
                if p.kind != ParamKind::Keyword {
                    self.observed.add(Feature::DefaultParams);
                }
                let mut env = LocalEnv::new(&scope_so_far, &no_captures);
                self.check_expr(default, &mut env, 0);
            }
            scope_so_far.insert(p.name.clone());
        }

        // Variadic/keyword-parameter well-formedness (M3 + KW1). A
        // Ruby-faithful, v0-light rule set over `kind`:
        //   - at most one `Rest` (`*rest`) parameter;
        //   - at most one `KwRest` (`**opts`) parameter;
        //   - ordering: positional `Required` first, then the lone `Rest`,
        //     then any number of `Keyword` params, then the lone `KwRest`.
        //     Anything out of that order is a structural error (not a panic).
        //
        // The canonical param list is therefore:
        //     Required*  Rest?  Keyword*  KwRest?
        //
        // Truth table for the offending transitions we reject (prev seen ⇒
        // current kind is illegal):
        //   prev seen \ cur | Required | Rest  | Keyword | KwRest
        //   Rest            | ERROR    | (dup) | ok      | ok
        //   Keyword         | ERROR    | ERROR | ok      | ok
        //   KwRest          | ERROR    | ERROR | ERROR   | (dup)
        //
        // Rationale for `Keyword` sitting *after* `Rest` but *before*
        // `KwRest`: a `*rest` slurps trailing *positional* args, so it must
        // close the positional run before any name-matched keyword params;
        // a `**opts` slurps *unmatched* keywords, so it must come after the
        // explicitly-named keyword params it would otherwise shadow.
        let mut rest_seen = false;
        let mut keyword_seen = false;
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
                    if keyword_seen {
                        self.error(
                            format!(
                                "rest parameter `*{}` must precede keyword parameters",
                                p.name
                            ),
                            &p.span,
                        );
                    }
                    if kwrest_seen {
                        self.error(
                            format!(
                                "rest parameter `*{}` must precede the keyword-rest parameter",
                                p.name
                            ),
                            &p.span,
                        );
                    }
                    rest_seen = true;
                }
                ParamKind::Keyword => {
                    // A keyword param must precede the lone `**opts`; it may
                    // follow positionals, the `*rest`, and other keywords.
                    if kwrest_seen {
                        self.error(
                            format!(
                                "keyword parameter `{}` must precede the keyword-rest parameter",
                                p.name
                            ),
                            &p.span,
                        );
                    }
                    keyword_seen = true;
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
                    // or keyword params — so it is exempt from the ordering
                    // rule.
                    if p.name == "__sir_block__" {
                        continue;
                    }
                    if kwrest_seen {
                        self.error(
                            format!(
                                "required parameter `{}` must precede the keyword-rest parameter",
                                p.name
                            ),
                            &p.span,
                        );
                    } else if keyword_seen {
                        self.error(
                            format!(
                                "required parameter `{}` must precede keyword parameters",
                                p.name
                            ),
                            &p.span,
                        );
                    } else if rest_seen {
                        self.error(
                            format!(
                                "required parameter `{}` must precede the rest parameter",
                                p.name
                            ),
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
                    for stmt in &stmts[i..j] {
                        if let Stmt::LetBinding {
                            value, sir_type, ..
                        } = stmt
                        {
                            self.check_expr(value, env, depth + 1);
                            if sir_type.is_some() {
                                self.observed.add(Feature::OptionalTypeAnnotations);
                            }
                        }
                    }
                    // Add every bound name to the env, all at once.
                    for stmt in &stmts[i..j] {
                        if let Stmt::LetBinding { name, .. } = stmt {
                            env.add_local(name.clone());
                        }
                    }
                    i = j;
                }
                Stmt::LetStarBinding {
                    name,
                    sir_type,
                    value,
                    ..
                } => {
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
                Stmt::Assign {
                    name,
                    scope,
                    value,
                    span,
                } => {
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
                Stmt::ForRange {
                    var,
                    start,
                    stop,
                    step,
                    body,
                    ..
                } => {
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
                Stmt::ForEach {
                    var, iter, body, ..
                } => {
                    self.observed.add(Feature::Loops);
                    self.check_expr(iter, env, depth + 1);
                    let inner_mark = env.mark();
                    env.add_local(var.clone());
                    self.check_block(body, env, depth + 1);
                    env.rewind(inner_mark);
                    i += 1;
                }
                Stmt::SeqSet {
                    seq, index, value, ..
                } => {
                    self.observed.add(Feature::Sequences);
                    self.check_expr(seq, env, depth + 1);
                    self.check_expr(index, env, depth + 1);
                    self.check_expr(value, env, depth + 1);
                    i += 1;
                }
                Stmt::MapSet {
                    map, key, value, ..
                } => {
                    self.observed.add(Feature::Maps);
                    self.check_expr(map, env, depth + 1);
                    self.check_expr(key, env, depth + 1);
                    self.check_expr(value, env, depth + 1);
                    i += 1;
                }
                Stmt::IndexSet {
                    target,
                    indices,
                    value,
                    ..
                } => {
                    // SIR22: `target[indices...] = value` — the mutation-
                    // shaped counterpart of SeqSet/MapSet above (and, per
                    // the spec, the one exception to "every new SIR22 node
                    // is Pure").  `target` is an arbitrary expression (not
                    // a bound name), so — like SeqSet/MapSet, and unlike
                    // Assign — there's no `check_varref` here.
                    self.observed.add(Feature::NDArrays);
                    self.check_expr(target, env, depth + 1);
                    self.check_index_args(indices, env, depth + 1);
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
                Stmt::TryCatch {
                    body,
                    rescues,
                    ensure_body,
                    span,
                } => {
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
            Expr::If {
                cond,
                then_branch,
                else_branch,
                ..
            } => {
                self.check_expr(cond, env, depth + 1);
                self.check_block(then_branch, env, depth + 1);
                self.check_block(else_branch, env, depth + 1);
            }
            Expr::Block(b) => self.check_block(b, env, depth + 1),
            Expr::DirectCall { fn_name, args, .. } => {
                // Call-side keyword-argument checks (ordering, duplicates,
                // name resolution against the known callee) run first, then
                // arity, then the recursive arg walk.
                self.check_call_kwargs(fn_name, args, e.span());
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
                        // Only *positional* args count against the R/M
                        // bounds; any `KeywordArg` is matched by name, not
                        // position, so it is excluded here (its validity is
                        // handled by `check_call_kwargs`).  A plain
                        // positional callee that receives a stray keyword
                        // still gets the precise "unknown keyword" diagnostic
                        // from name resolution rather than a misleading
                        // arity-count error.
                        let n = args
                            .iter()
                            .filter(|a| !matches!(a, Expr::KeywordArg { .. }))
                            .count();
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
                self.check_args(args, env, depth);
            }
            Expr::IndirectCall { target, args, .. } => {
                self.observed.add(Feature::Closures);
                // Ordering + duplicate keyword checks apply, but the callee
                // signature is not statically known, so no name resolution.
                self.check_kwargs_common(None, args, e.span());
                self.check_expr(target, env, depth + 1);
                self.check_args(args, env, depth);
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
            Expr::MakeClosure {
                fn_name,
                captures,
                span,
            } => {
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
            Expr::Intrinsic {
                targets,
                args,
                span,
                ..
            } => {
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
                        format!("str-concat needs at least 2 parts, got {}", parts.len()),
                        span,
                    );
                }
                for p in parts {
                    self.check_expr(p, env, depth + 1);
                }
            }
            // ── KW1: keyword argument ──────────────────────────────
            Expr::KeywordArg { value, span, .. } => {
                // Using a keyword argument at all requires the feature.
                self.observed.add(Feature::KeywordParams);
                // A `KeywordArg` is only well-formed as an *immediate*
                // argument of a call.  `in_call_args` is true exactly when
                // we are walking such immediate arguments (see
                // `check_args`); anywhere else — nested inside another
                // expression, as a `BuiltinCall`/`Intrinsic` argument, as a
                // block value, a let RHS, a default expr, etc. — it is
                // misplaced and rejected.
                if !self.in_call_args {
                    self.error(
                        "keyword argument may only appear directly in a call's argument list"
                            .to_string(),
                        span,
                    );
                }
                // Recurse into the value with the call-args flag cleared:
                // the value itself is an ordinary expression position (a
                // nested `KeywordArg` inside it would be misplaced).
                let prev = self.in_call_args;
                self.in_call_args = false;
                self.check_expr(value, env, depth + 1);
                self.in_call_args = prev;
            }

            // ── SIR22: array/matrix nodes ───────────────────────────
            Expr::ArrayLit { rows, .. } => {
                self.observed.add(Feature::NDArrays);
                self.observed.add(Feature::ArrayColumnMajor);
                for row in rows {
                    for item in row {
                        self.check_expr(item, env, depth + 1);
                    }
                }
            }
            Expr::Range {
                start, step, stop, ..
            } => {
                self.observed.add(Feature::NDArrays);
                self.check_expr(start, env, depth + 1);
                if let Some(step) = step {
                    self.check_expr(step, env, depth + 1);
                }
                self.check_expr(stop, env, depth + 1);
            }
            Expr::MatMul { lhs, rhs, .. } => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                self.check_expr(lhs, env, depth + 1);
                self.check_expr(rhs, env, depth + 1);
            }
            Expr::ElementwiseOp { lhs, rhs, .. } => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                self.check_expr(lhs, env, depth + 1);
                self.check_expr(rhs, env, depth + 1);
            }
            Expr::Transpose { target, .. } => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                self.check_expr(target, env, depth + 1);
            }
            Expr::IndexGet {
                target, indices, ..
            } => {
                self.observed.add(Feature::NDArrays);
                self.check_expr(target, env, depth + 1);
                self.check_index_args(indices, env, depth + 1);
            }

            // ── SIR26: integer conversion ──────────────────────────────
            Expr::Convert { value, to, .. } => {
                // Observe the conversion feature plus the SIR21 type-implied
                // features of the target type, so the manifest and capability
                // check see exactly what a backend must support.
                self.observed.add(Feature::Conversions);
                if !to.is_arbitrary() {
                    self.observed.add(Feature::SizedIntegers);
                }
                if !to.signed {
                    self.observed.add(Feature::Unsigned);
                }
                if to.overflow != crate::types::Overflow::Arbitrary {
                    self.observed.add(Feature::WrappingArithmetic);
                }
                self.check_expr(value, env, depth + 1);
            }

            // ── SIR23: symbolic expression + pattern/rewrite nodes ──
            Expr::SymSymbol { .. } => {
                self.observed.add(Feature::SymbolicExpr);
            }
            Expr::SymRational { .. } => {
                // Shares the SIR22 `Rationals` feature rather than a new
                // one — see the SIR23 spec's "New `Feature` flags".
                self.observed.add(Feature::Rationals);
            }
            Expr::SymApply { head, args, .. } => {
                self.observed.add(Feature::SymbolicExpr);
                self.check_expr(head, env, depth + 1);
                for a in args {
                    self.check_expr(a, env, depth + 1);
                }
            }
            Expr::SymPatternBlank { head, .. } => {
                self.observed.add(Feature::PatternMatching);
                if let Some(h) = head {
                    self.check_expr(h, env, depth + 1);
                }
            }
            Expr::SymPatternNamed { pattern, .. } => {
                self.observed.add(Feature::PatternMatching);
                self.check_expr(pattern, env, depth + 1);
            }
            Expr::SymRule { lhs, rhs, .. } => {
                self.observed.add(Feature::PatternMatching);
                self.check_expr(lhs, env, depth + 1);
                self.check_expr(rhs, env, depth + 1);
            }
            Expr::SymReplaceAll { expr, rules, .. } => {
                self.observed.add(Feature::PatternMatching);
                self.check_expr(expr, env, depth + 1);
                for r in rules {
                    self.check_expr(r, env, depth + 1);
                }
            }
        }
    }

    /// Validate the `Expr` nested inside every `IndexArg` of an
    /// `IndexGet`/`IndexSet` (SIR22).  Factored out because both node
    /// kinds share the exact same index-arg shape (mirroring how
    /// `walker.rs`'s `walk_index_args` is shared between them).
    fn check_index_args(&mut self, indices: &[IndexArg], env: &mut LocalEnv, depth: usize) {
        for arg in indices {
            match arg {
                IndexArg::Scalar(e) => self.check_expr(e, env, depth + 1),
                IndexArg::Whole => {}
                IndexArg::Range(e) => self.check_expr(e, env, depth + 1),
            }
        }
    }

    /// Walk the arguments of a call node (DirectCall / IndirectCall /
    /// BuiltinCall / MakeClosure), permitting a top-level `KeywordArg`.
    ///
    /// A `KeywordArg` is only well-placed as a *direct* argument of a call,
    /// so we set `in_call_args = true` around this loop; the `KeywordArg`
    /// arm of [`Self::check_expr`] reads the flag to allow the keyword here
    /// and to *reject* it anywhere else (it clears the flag before
    /// recursing into the keyword's value).  We save and restore the prior
    /// flag value so nested calls compose correctly.
    fn check_args(&mut self, args: &[Expr], env: &mut LocalEnv, depth: usize) {
        let prev = self.in_call_args;
        self.in_call_args = true;
        for a in args {
            self.check_expr(a, env, depth + 1);
        }
        self.in_call_args = prev;
    }

    /// Call-side keyword-argument validation for a call whose args vec is
    /// `args` (KW1).  Enforces, independent of the callee's identity:
    ///
    ///   1. **Ordering** — every `KeywordArg` must follow all positional
    ///      (non-`KeywordArg`) arguments.  `f(1, a: 2)` is fine; a
    ///      positional after a keyword (`f(a: 2, 1)`) is rejected.
    ///   2. **No duplicate keyword names** within one call's args.
    ///
    /// When `known_callee` is `Some(name)` and that name resolves to a
    /// function in this module, it additionally performs
    ///   3. **Name resolution** — each `KeywordArg.name` must match a
    ///      `Keyword` param of the callee OR the callee declares a
    ///      `KwRest`; and every REQUIRED keyword param (Keyword, default
    ///      None) must be supplied.
    ///
    /// IndirectCall / closure calls pass `None`: the signature is not
    /// statically known, so only ordering + duplicate checks apply.
    fn check_call_kwargs(&mut self, callee: &str, args: &[Expr], call_span: &Span) {
        self.check_kwargs_common(Some(callee), args, call_span);
    }

    /// Ordering + duplicate checks (and optional name resolution) shared by
    /// every call kind.  `callee` is `Some` only for a `DirectCall`, whose
    /// target may be a known module function.
    fn check_kwargs_common(&mut self, callee: Option<&str>, args: &[Expr], call_span: &Span) {
        let mut seen_keyword = false;
        let mut names: HashSet<&str> = HashSet::new();
        let mut supplied: Vec<&str> = Vec::new();
        for a in args {
            match a {
                Expr::KeywordArg { name, span, .. } => {
                    seen_keyword = true;
                    if !names.insert(name.as_str()) {
                        self.error(
                            format!("duplicate keyword argument `{}` in call", name),
                            span,
                        );
                    }
                    supplied.push(name.as_str());
                }
                _ => {
                    // A positional argument.  Once a keyword has appeared,
                    // positionals are illegal — keyword args must trail all
                    // positionals so a backend can split `args` at the first
                    // keyword unambiguously.
                    if seen_keyword {
                        self.error(
                            "positional argument may not follow a keyword argument".to_string(),
                            a.span(),
                        );
                    }
                }
            }
        }

        // ── Indirect/closure keyword rejection (v0 soundness gate) ────────
        //
        // `callee == None` means the call target is *not* a statically-known
        // direct function — it is an `IndirectCall` through a closure/function
        // value.  Per `code/specs/sir-keyword-params.md` ("Out of scope"),
        // keyword arguments on such calls are OUT OF SCOPE for v0: **no**
        // backend can emit them.  Every emitter's `emit_args` for an
        // `IndirectCall` routes each argument through `emit_expr`, whose
        // `KeywordArg` arm is a hard `panic!` (keyword resolution needs the
        // callee's parameter names/order, which an indirect call does not have
        // statically).  If the validator accepted such a module, lowering it
        // would panic — a denial-of-service on validator-accepted input.
        //
        // The ordering/duplicate loop above still runs (a keyword mis-ordered
        // even on an indirect call is malformed), but we additionally reject
        // the mere *presence* of any keyword argument here.  This is purely
        // subtractive: it forbids more programs, changes no accepted DirectCall
        // behaviour (that path passes `Some(callee)` and never reaches this
        // branch), and adds no enum variant — so downstream crates that only
        // *construct* IR are unaffected; only ill-formed IR is now caught.
        let Some(callee) = callee else {
            for a in args {
                if let Expr::KeywordArg { name, span, .. } = a {
                    self.error(
                        format!(
                            "keyword argument `{}` is not allowed on an indirect/closure call (only direct calls support keyword arguments in v0)",
                            name
                        ),
                        span,
                    );
                }
            }
            return;
        };
        // Clone out the callee's keyword-param facts we need, to avoid
        // holding a borrow of `self.module` across the `self.error` calls.
        let Some(f) = self.module.functions.iter().find(|f| f.name == callee) else {
            return;
        };
        let kw_names: HashSet<String> = f.keyword_params().iter().map(|p| p.name.clone()).collect();
        let has_kwrest = f.params.iter().any(|p| p.kind == ParamKind::KwRest);
        let required_kw: Vec<String> = f
            .params
            .iter()
            .filter(|p| p.kind == ParamKind::Keyword && p.default.is_none())
            .map(|p| p.name.clone())
            .collect();

        // Every supplied keyword must name a declared keyword param, unless
        // the callee slurps extras via `**kwrest`.
        if !has_kwrest {
            for a in args {
                if let Expr::KeywordArg { name, span, .. } = a {
                    if !kw_names.contains(name) {
                        self.error(
                            format!(
                                "call to `{}` passes unknown keyword `{}` (callee declares no such keyword parameter and no `**` keyword-rest)",
                                callee, name
                            ),
                            span,
                        );
                    }
                }
            }
        }

        // Every required keyword param must be supplied.
        for req in &required_kw {
            if !supplied.contains(&req.as_str()) {
                self.error(
                    format!("call to `{}` is missing required keyword `{}`", callee, req),
                    call_span,
                );
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
                        format!(
                            "var-ref scope=param references unknown parameter `{}`",
                            name
                        ),
                        span,
                    );
                }
            }
            Scope::Capture => {
                if !env.has_capture(name) {
                    self.error(
                        format!(
                            "var-ref scope=capture references unknown capture `{}`",
                            name
                        ),
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
        Param {
            name: name.into(),
            sir_type: None,
            kind,
            default: None,
            span: s(),
        }
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
        let m = module_with_params(vec![p("a", ParamKind::Rest), p("b", ParamKind::Rest)]);
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("more than one rest")));
    }

    #[test]
    fn two_kwrest_params_is_error() {
        let m = module_with_params(vec![p("a", ParamKind::KwRest), p("b", ParamKind::KwRest)]);
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r
            .errors()
            .any(|i| i.message.contains("more than one keyword-rest")));
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
        assert!(r
            .errors()
            .any(|i| i.message.contains("must precede the keyword-rest")));
    }

    /// SIR19: a parameter carrying a default-value expression validates
    /// OK and causes the validator to observe `Feature::DefaultParams`.
    #[test]
    fn param_with_default_validates_and_observes_feature() {
        // def f(a = 1) — one required param with a default literal `1`.
        let mut m = empty_module(FeatureManifest::from_features(&[
            Feature::DynamicTyping,
            Feature::DefaultParams,
        ]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![Param {
                name: "a".into(),
                sir_type: None,
                kind: ParamKind::Required,
                default: Some(Box::new(Expr::IntLit {
                    value: 1,
                    span: s(),
                })),
                span: s(),
            }],
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
        let mut m = empty_module(FeatureManifest::from_features(&[
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
                default: Some(Box::new(Expr::StrLit {
                    value: "x".into(),
                    span: s(),
                })),
                span: s(),
            }],
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
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    /// A default expression may reference an earlier parameter
    /// (`def f(a, b = a)`); the validator resolves the `VarRef` against
    /// the params declared so far.
    #[test]
    fn default_expr_may_reference_earlier_param() {
        let mut m = empty_module(FeatureManifest::from_features(&[
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
                    default: None,
                    span: s(),
                },
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
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    /// A default expression that references a *later* parameter is a
    /// scope error — only params declared so far are in view.
    #[test]
    fn default_expr_cannot_reference_later_param() {
        let mut m = empty_module(FeatureManifest::from_features(&[
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
                Param {
                    name: "b".into(),
                    sir_type: None,
                    kind: ParamKind::Required,
                    default: None,
                    span: s(),
                },
            ],
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
            default: Some(Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            })),
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
            body: Block {
                stmts: vec![],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let args: Vec<Expr> = (0..n_args)
            .map(|i| Expr::IntLit {
                value: i as i64,
                span: s(),
            })
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
        assert!(
            r.is_ok(),
            "expected ok for variadic callee, got {:?}",
            r.issues
        );
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
            body: Block {
                stmts: vec![],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
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
            body: Block {
                stmts: vec![],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
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
                        Expr::IntLit {
                            value: 2,
                            span: s(),
                        },
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
        assert!(
            r.is_ok(),
            "block-passing call must validate, got {:?}",
            r.issues
        );
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
            body: Block {
                stmts: vec![],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
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
            body: Block {
                stmts: vec![],
                value: Expr::NilLit { span: s() },
                span: s(),
            },
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

    #[test]
    fn required_after_defaulted_param_is_a_hole_error() {
        // def f(a = 1, b) — `b` is a required param following a defaulted
        // one.  This "hole" is rejected so `missing_defaults` only ever
        // returns params that carry a default.
        let m = module_with_default_params(vec![p_default("a"), p("b", ParamKind::Required)]);
        let r = validate(&m);
        assert!(!r.is_ok(), "a hole must fail validation");
        assert!(
            r.errors()
                .any(|i| i.message.contains("may not follow a defaulted parameter")),
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
        assert!(
            r.is_ok(),
            "trailing defaults must validate, got {:?}",
            r.issues
        );
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
        assert!(r
            .errors()
            .any(|i| i.message.contains("must precede the rest")));
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
                value: Expr::SymLit {
                    name: "x".into(),
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
        assert!(r.errors().any(|i| i.message.contains("symbols")));
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
                    return_type: crate::types::SirType::Dynamic,
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
                        value: Expr::IntLit {
                            value: 1,
                            span: s(),
                        },
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
                cond: Box::new(Expr::BoolLit {
                    value: true,
                    span: s(),
                }),
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

    // `3.14` is an arbitrary float literal test value, not an approximation of PI.
    #[allow(clippy::approx_constant)]
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
                value: Expr::FloatLit {
                    value: 3.14,
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
                    cond: Expr::BoolLit {
                        value: false,
                        span: s(),
                    },
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
                    start: Expr::IntLit {
                        value: 0,
                        span: s(),
                    },
                    stop: Expr::IntLit {
                        value: 10,
                        span: s(),
                    },
                    step: Expr::IntLit {
                        value: 1,
                        span: s(),
                    },
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
                    start: Expr::IntLit {
                        value: 0,
                        span: s(),
                    },
                    stop: Expr::IntLit {
                        value: 10,
                        span: s(),
                    },
                    step: Expr::IntLit {
                        value: 1,
                        span: s(),
                    },
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
                    lhs: Box::new(Expr::BoolLit {
                        value: true,
                        span: s(),
                    }),
                    rhs: Box::new(Expr::BoolLit {
                        value: false,
                        span: s(),
                    }),
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
        let mut m = empty_module(FeatureManifest::from_features(&[
            Feature::StringInterpolation,
            Feature::Strings,
        ]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::StrConcat {
                    parts: vec![
                        Expr::StrLit {
                            value: "a".into(),
                            span: s(),
                        },
                        Expr::StrLit {
                            value: "b".into(),
                            span: s(),
                        },
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
        let mut m = empty_module(FeatureManifest::from_features(&[
            Feature::StringInterpolation,
            Feature::Strings,
        ]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::StrConcat {
                    parts: vec![Expr::StrLit {
                        value: "lonely".into(),
                        span: s(),
                    }],
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
                        value: Expr::IntLit {
                            value: 1,
                            span: s(),
                        },
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
                value: Expr::IntLit {
                    value: 10,
                    span: s(),
                },
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
                            value: Expr::IntLit {
                                value: 1,
                                span: s(),
                            },
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
                        value: Expr::IntLit {
                            value: 3,
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
                        value: Expr::IntLit {
                            value: 1,
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
                    body: vec![lb(
                        "_t",
                        Expr::IntLit {
                            value: 1,
                            span: s(),
                        },
                    )],
                    rescues: vec![RescueClause {
                        exception_types: vec!["Foo".into()],
                        binding: Some("e".into()),
                        // The rescue body reads the bound `e` — must resolve.
                        body: vec![lb(
                            "_r",
                            Expr::VarRef {
                                name: "e".into(),
                                scope: Scope::Local,
                                span: s(),
                            },
                        )],
                        span: s(),
                    }],
                    ensure_body: Some(vec![lb(
                        "_e",
                        Expr::IntLit {
                            value: 1,
                            span: s(),
                        },
                    )]),
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
                        value: Expr::VarRef {
                            name: "e".into(),
                            scope: Scope::Local,
                            span: s(),
                        },
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

    // ── KW1: keyword parameters & arguments ────────────────────────────

    /// A keyword param `name:` (required) or `name: 1` (optional).
    fn kw(name: &str, default: Option<i64>) -> Param {
        Param {
            name: name.into(),
            sir_type: None,
            kind: ParamKind::Keyword,
            default: default.map(|v| {
                Box::new(Expr::IntLit {
                    value: v,
                    span: s(),
                })
            }),
            span: s(),
        }
    }

    /// A keyword argument `name: <int>` for a call's args vec.
    fn kwarg(name: &str, v: i64) -> Expr {
        Expr::KeywordArg {
            name: name.into(),
            value: Box::new(Expr::IntLit {
                value: v,
                span: s(),
            }),
            span: s(),
        }
    }

    /// Build a two-function module: callee `f` with `callee_params` and a
    /// caller `g` whose body is a `DirectCall` to `f` with `call_args`.
    /// The manifest declares KeywordParams so keyword-using modules pass
    /// the manifest check; extra features can be appended by the caller
    /// via `extra`.
    fn module_kw_call(
        callee_params: Vec<Param>,
        call_args: Vec<Expr>,
        extra: &[Feature],
    ) -> Module {
        let mut feats = vec![Feature::DynamicTyping, Feature::KeywordParams];
        feats.extend_from_slice(extra);
        let mut m = empty_module(FeatureManifest::from_features(&feats));
        m.functions.push(Function {
            name: "f".into(),
            params: callee_params,
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
        m.functions.push(Function {
            name: "g".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::DirectCall {
                    fn_name: "f".into(),
                    args: call_args,
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
    fn keyword_param_observes_feature_and_gating() {
        // def f(x:) with the feature declared → ok; without it → error.
        let ok = module_with_params(vec![kw("x", Some(1))]);
        // module_with_params only declares DynamicTyping, so the keyword
        // param is undeclared → error.
        assert!(
            !validate(&ok).is_ok(),
            "keyword param without KeywordParams declared must error"
        );
        // Now declare it.
        let mut m = empty_module(FeatureManifest::from_features(&[
            Feature::DynamicTyping,
            Feature::KeywordParams,
        ]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![kw("x", Some(1))],
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
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn keyword_arg_requires_feature() {
        // f(a: 1) where the manifest omits KeywordParams → error, even
        // though the callee accepts `a` (feature gating is independent of
        // name resolution).
        let mut m = empty_module(FeatureManifest::from_features(&[Feature::DynamicTyping]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![kw("a", Some(0))],
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
        m.functions.push(Function {
            name: "g".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::DirectCall {
                    fn_name: "f".into(),
                    args: vec![kwarg("a", 1)],
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
        assert!(!r.is_ok(), "keyword arg without the feature must error");
        assert!(r.errors().any(|i| i.message.contains("keyword-params")));
    }

    // ── def-side ordering ──────────────────────────────────────────────

    #[test]
    fn keyword_params_canonical_order_is_valid() {
        // def f(a, *rest, x:, y: 1, **opts) — required, rest, keywords, kwrest.
        let m = module_with_params_kw(vec![
            p("a", ParamKind::Required),
            p("rest", ParamKind::Rest),
            kw("x", None),
            kw("y", Some(1)),
            p("opts", ParamKind::KwRest),
        ]);
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    /// module_with_params but declaring KeywordParams too.
    fn module_with_params_kw(params: Vec<Param>) -> Module {
        let mut m = empty_module(FeatureManifest::from_features(&[
            Feature::DynamicTyping,
            Feature::KeywordParams,
        ]));
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

    #[test]
    fn keyword_param_before_positional_is_error() {
        // def f(x:, a) — a required positional after a keyword param.
        let m = module_with_params_kw(vec![kw("x", None), p("a", ParamKind::Required)]);
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(
            r.errors()
                .any(|i| i.message.contains("must precede keyword parameters")),
            "got {:?}",
            r.issues
        );
    }

    #[test]
    fn keyword_param_after_kwrest_is_error() {
        // def f(**opts, x:) — a keyword param after the keyword-rest.
        let m = module_with_params_kw(vec![p("opts", ParamKind::KwRest), kw("x", None)]);
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(
            r.errors()
                .any(|i| i.message.contains("must precede the keyword-rest")),
            "got {:?}",
            r.issues
        );
    }

    #[test]
    fn rest_after_keyword_is_error() {
        // def f(x:, *rest) — the rest param must precede keyword params.
        let m = module_with_params_kw(vec![kw("x", None), p("rest", ParamKind::Rest)]);
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(
            r.errors()
                .any(|i| i.message.contains("must precede keyword parameters")),
            "got {:?}",
            r.issues
        );
    }

    // ── call-side ordering / duplicates ────────────────────────────────

    #[test]
    fn keyword_arg_after_positional_is_valid() {
        // def f(a, x:); f(1, x: 2) — one positional then one keyword.
        let m = module_kw_call(
            vec![p("a", ParamKind::Required), kw("x", None)],
            vec![
                Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                kwarg("x", 2),
            ],
            &[],
        );
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn positional_after_keyword_arg_is_error() {
        // f(x: 2, 1) — a positional after a keyword argument.
        let m = module_kw_call(
            vec![kw("x", Some(0)), p("a", ParamKind::Required)],
            vec![
                kwarg("x", 2),
                Expr::IntLit {
                    value: 1,
                    span: s(),
                },
            ],
            &[],
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(
            r.errors().any(|i| i
                .message
                .contains("positional argument may not follow a keyword")),
            "got {:?}",
            r.issues
        );
    }

    #[test]
    fn duplicate_keyword_arg_is_error() {
        // f(x: 1, x: 2) — the same keyword twice in one call.
        let m = module_kw_call(
            vec![kw("x", Some(0))],
            vec![kwarg("x", 1), kwarg("x", 2)],
            &[],
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(
            r.errors()
                .any(|i| i.message.contains("duplicate keyword argument")),
            "got {:?}",
            r.issues
        );
    }

    // ── name resolution against a known callee ─────────────────────────

    #[test]
    fn unknown_keyword_without_kwrest_is_error() {
        // def f(x:); f(y: 1) — `y` is not a declared keyword and there is
        // no **kwrest to absorb it.
        let m = module_kw_call(vec![kw("x", None)], vec![kwarg("y", 1)], &[]);
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(
            r.errors()
                .any(|i| i.message.contains("unknown keyword `y`")),
            "got {:?}",
            r.issues
        );
    }

    #[test]
    fn unknown_keyword_with_kwrest_is_accepted() {
        // def f(**opts); f(y: 1) — the **opts absorbs the unmatched keyword,
        // so an otherwise-unknown keyword name is accepted.
        let m = module_kw_call(vec![p("opts", ParamKind::KwRest)], vec![kwarg("y", 1)], &[]);
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok with **kwrest, got {:?}", r.issues);
    }

    #[test]
    fn missing_required_keyword_is_error() {
        // def f(x:); f() — the required keyword `x` is not supplied.
        let m = module_kw_call(vec![kw("x", None)], vec![], &[]);
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(
            r.errors()
                .any(|i| i.message.contains("missing required keyword `x`")),
            "got {:?}",
            r.issues
        );
    }

    #[test]
    fn optional_keyword_may_be_omitted() {
        // def f(x: 1); f() — the keyword `x` is optional (has a default),
        // so omitting it is fine.
        let m = module_kw_call(vec![kw("x", Some(1))], vec![], &[]);
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn supplying_required_keyword_is_valid() {
        // def f(x:); f(x: 5) — the required keyword is supplied.
        let m = module_kw_call(vec![kw("x", None)], vec![kwarg("x", 5)], &[]);
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    /// Build a single-function module whose body is an `IndirectCall`
    /// through parameter `cb` with the given `call_args`.  The manifest
    /// declares KeywordParams + Closures so the constructs pass the manifest
    /// gate and the interesting failure (if any) is the validator rule.
    fn module_indirect_call(call_args: Vec<Expr>) -> Module {
        let mut m = empty_module(FeatureManifest::from_features(&[
            Feature::DynamicTyping,
            Feature::KeywordParams,
            Feature::Closures,
        ]));
        m.functions.push(Function {
            name: "g".into(),
            params: vec![p("cb", ParamKind::Required)],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::IndirectCall {
                    target: Box::new(Expr::VarRef {
                        name: "cb".into(),
                        scope: Scope::Param,
                        span: s(),
                    }),
                    args: call_args,
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
    fn indirect_call_with_keyword_arg_is_rejected() {
        // DoD (a): a keyword argument on an indirect/closure call is REJECTED.
        // No backend can emit an indirect keyword call (their `emit_expr`
        // `KeywordArg` arm panics), so accepting one would be a DoS on
        // validator-accepted input.  `main(g) { g(x: 1) }` is exactly the
        // program described in the soundness gap.
        let m = module_indirect_call(vec![kwarg("x", 1)]);
        let r = validate(&m);
        assert!(!r.is_ok(), "indirect keyword call must be rejected");
        assert!(
            r.errors().any(|i| i
                .message
                .contains("keyword argument `x` is not allowed on an indirect/closure call")),
            "expected the indirect-keyword rejection message, got {:?}",
            r.issues
        );
    }

    #[test]
    fn direct_call_with_matching_keyword_still_validates() {
        // DoD (b): the SAME keyword arg to a matching-signature DIRECT callee
        // still validates — the fix is subtractive and does not touch the
        // DirectCall path.  def f(x:); g() = f(x: 1).
        let m = module_kw_call(vec![kw("x", None)], vec![kwarg("x", 1)], &[]);
        let r = validate(&m);
        assert!(
            r.is_ok(),
            "direct keyword call must still validate, got {:?}",
            r.issues
        );
    }

    #[test]
    fn indirect_call_with_only_positionals_still_validates() {
        // DoD (c): an indirect call with only positional args is unaffected.
        let m = module_indirect_call(vec![Expr::IntLit {
            value: 1,
            span: s(),
        }]);
        let r = validate(&m);
        assert!(
            r.is_ok(),
            "positional indirect call must validate, got {:?}",
            r.issues
        );
    }

    #[test]
    fn indirect_call_still_enforces_keyword_ordering() {
        // Even without name resolution, a positional after a keyword in an
        // indirect call is rejected.
        let mut m = empty_module(FeatureManifest::from_features(&[
            Feature::DynamicTyping,
            Feature::KeywordParams,
            Feature::Closures,
        ]));
        m.functions.push(Function {
            name: "g".into(),
            params: vec![p("cb", ParamKind::Required)],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::IndirectCall {
                    target: Box::new(Expr::VarRef {
                        name: "cb".into(),
                        scope: Scope::Param,
                        span: s(),
                    }),
                    args: vec![
                        kwarg("x", 1),
                        Expr::IntLit {
                            value: 2,
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
        assert!(!r.is_ok());
        assert!(
            r.errors().any(|i| i
                .message
                .contains("positional argument may not follow a keyword")),
            "got {:?}",
            r.issues
        );
    }

    // ── KeywordArg only in call position ───────────────────────────────

    #[test]
    fn keyword_arg_outside_call_is_error() {
        // A KeywordArg used as a block value (not a call argument) is
        // misplaced.  `def g() = (a: 1)` — invalid.
        let mut m = empty_module(FeatureManifest::from_features(&[
            Feature::DynamicTyping,
            Feature::KeywordParams,
        ]));
        m.functions.push(Function {
            name: "g".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: kwarg("a", 1),
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(
            r.errors()
                .any(|i| i.message.contains("may only appear directly in a call")),
            "got {:?}",
            r.issues
        );
    }

    #[test]
    fn keyword_arg_nested_in_builtin_call_is_error() {
        // A KeywordArg buried in a BuiltinCall's args (not a keyword-taking
        // call position) is misplaced.  `(+ (a: 1))` — invalid.
        let mut m = empty_module(FeatureManifest::from_features(&[
            Feature::DynamicTyping,
            Feature::KeywordParams,
        ]));
        m.functions.push(Function {
            name: "g".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value: Expr::BuiltinCall {
                    name: "+".into(),
                    args: vec![kwarg("a", 1)],
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
        assert!(
            r.errors()
                .any(|i| i.message.contains("may only appear directly in a call")),
            "got {:?}",
            r.issues
        );
    }

    #[test]
    fn keyword_arg_value_may_not_nest_a_keyword_arg() {
        // The *value* of a keyword arg is an ordinary expression position,
        // so a keyword arg nested inside it is misplaced.
        // f(a: (b: 1)) — the inner `(b: 1)` is invalid.
        let inner = Expr::KeywordArg {
            name: "b".into(),
            value: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            span: s(),
        };
        let outer = Expr::KeywordArg {
            name: "a".into(),
            value: Box::new(inner),
            span: s(),
        };
        let m = module_kw_call(vec![p("opts", ParamKind::KwRest)], vec![outer], &[]);
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(
            r.errors()
                .any(|i| i.message.contains("may only appear directly in a call")),
            "got {:?}",
            r.issues
        );
    }

    #[test]
    fn keyword_arg_value_expression_is_validated() {
        // The keyword arg's value is recursed into: a bad var-ref inside it
        // is caught.  f(**opts); f(x: <unknown local>) — scope error.
        let m = module_kw_call(
            vec![p("opts", ParamKind::KwRest)],
            vec![Expr::KeywordArg {
                name: "x".into(),
                value: Box::new(Expr::VarRef {
                    name: "ghost".into(),
                    scope: Scope::Local,
                    span: s(),
                }),
                span: s(),
            }],
            &[],
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(
            r.errors()
                .any(|i| i.message.contains("unknown name `ghost`")),
            "got {:?}",
            r.issues
        );
    }

    // ── SIR22: array/matrix validator tests ──────────────────────────

    fn module_with_fn_body_value(manifest: FeatureManifest, value: Expr) -> Module {
        let mut m = empty_module(manifest);
        m.functions.push(Function {
            name: "f".into(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![],
                value,
                span: s(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: s(),
        });
        m
    }

    #[test]
    fn array_lit_observes_nd_arrays_and_column_major_features() {
        // [1 2; 3 4] used without declaring NDArrays/ArrayColumnMajor → error.
        let m = module_with_fn_body_value(
            FeatureManifest::new(),
            Expr::ArrayLit {
                rows: vec![
                    vec![
                        Expr::IntLit {
                            value: 1,
                            span: s(),
                        },
                        Expr::IntLit {
                            value: 2,
                            span: s(),
                        },
                    ],
                    vec![
                        Expr::IntLit {
                            value: 3,
                            span: s(),
                        },
                        Expr::IntLit {
                            value: 4,
                            span: s(),
                        },
                    ],
                ],
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("nd-arrays")));
        assert!(r.errors().any(|i| i.message.contains("array-column-major")));
    }

    #[test]
    fn array_lit_with_declared_features_is_valid() {
        let m = module_with_fn_body_value(
            FeatureManifest::from_features(&[Feature::NDArrays, Feature::ArrayColumnMajor]),
            Expr::ArrayLit {
                rows: vec![vec![Expr::IntLit {
                    value: 1,
                    span: s(),
                }]],
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn range_observes_nd_arrays_feature() {
        // 1:5 — a bare Range doesn't need MatrixOps or ArrayColumnMajor,
        // only NDArrays.
        let m = module_with_fn_body_value(
            FeatureManifest::new(),
            Expr::Range {
                start: Box::new(Expr::IntLit {
                    value: 1,
                    span: s(),
                }),
                step: None,
                stop: Box::new(Expr::IntLit {
                    value: 5,
                    span: s(),
                }),
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("nd-arrays")));
        assert!(!r.errors().any(|i| i.message.contains("matrix-ops")));
    }

    #[test]
    fn range_with_declared_nd_arrays_is_valid() {
        let m = module_with_fn_body_value(
            FeatureManifest::from_features(&[Feature::NDArrays]),
            Expr::Range {
                start: Box::new(Expr::IntLit {
                    value: 0,
                    span: s(),
                }),
                step: Some(Box::new(Expr::IntLit {
                    value: 2,
                    span: s(),
                })),
                stop: Box::new(Expr::IntLit {
                    value: 10,
                    span: s(),
                }),
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn matmul_observes_matrix_ops_and_column_major_features() {
        let m = module_with_fn_body_value(
            FeatureManifest::new(),
            Expr::MatMul {
                lhs: Box::new(Expr::IntLit {
                    value: 1,
                    span: s(),
                }),
                rhs: Box::new(Expr::IntLit {
                    value: 2,
                    span: s(),
                }),
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("matrix-ops")));
        assert!(r.errors().any(|i| i.message.contains("array-column-major")));
    }

    #[test]
    fn matmul_with_declared_features_is_valid() {
        let m = module_with_fn_body_value(
            FeatureManifest::from_features(&[Feature::MatrixOps, Feature::ArrayColumnMajor]),
            Expr::MatMul {
                lhs: Box::new(Expr::IntLit {
                    value: 1,
                    span: s(),
                }),
                rhs: Box::new(Expr::IntLit {
                    value: 2,
                    span: s(),
                }),
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn elementwise_op_observes_matrix_ops_feature() {
        let m = module_with_fn_body_value(
            FeatureManifest::new(),
            Expr::ElementwiseOp {
                op: ElementwiseOpKind::Mul,
                lhs: Box::new(Expr::IntLit {
                    value: 1,
                    span: s(),
                }),
                rhs: Box::new(Expr::IntLit {
                    value: 2,
                    span: s(),
                }),
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("matrix-ops")));
    }

    #[test]
    fn transpose_observes_matrix_ops_feature() {
        let m = module_with_fn_body_value(
            FeatureManifest::new(),
            Expr::Transpose {
                target: Box::new(Expr::IntLit {
                    value: 1,
                    span: s(),
                }),
                conjugate: true,
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("matrix-ops")));
    }

    #[test]
    fn index_get_observes_nd_arrays_feature_and_validates_index_args() {
        // a(i, :, 1:3) — the Scalar index `ghost` is an unresolvable
        // local, so both the missing-feature error and the scope error
        // are expected.
        let m = module_with_fn_body_value(
            FeatureManifest::new(),
            Expr::IndexGet {
                target: Box::new(Expr::VarRef {
                    name: "a".into(),
                    scope: Scope::Global,
                    span: s(),
                }),
                indices: vec![
                    IndexArg::Scalar(Box::new(Expr::VarRef {
                        name: "ghost".into(),
                        scope: Scope::Local,
                        span: s(),
                    })),
                    IndexArg::Whole,
                ],
                span: s(),
            },
        );
        // `a` is referenced with Scope::Global but never declared as a
        // module global — expect a scope error on `a` too, plus one on
        // `ghost`, plus the missing-feature error.  We only assert the
        // two things this test targets: the feature observation and
        // that IndexArg::Scalar's nested expr is actually validated.
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("nd-arrays")));
        assert!(r
            .errors()
            .any(|i| i.message.contains("unknown name `ghost`")));
    }

    #[test]
    fn index_get_with_declared_feature_and_valid_args_is_valid() {
        // `a` is scope-checked via a function param (Scope::Param) rather
        // than a local/global, keeping this test focused on the
        // feature-observation + index-arg validation path.  The param's
        // `sir_type: None` is itself what triggers `Feature::DynamicTyping`,
        // hence declaring it in the manifest below.
        let m = module_with_fn_body_value(
            FeatureManifest::from_features(&[Feature::NDArrays, Feature::DynamicTyping]),
            Expr::IndexGet {
                target: Box::new(Expr::VarRef {
                    name: "a".into(),
                    scope: Scope::Param,
                    span: s(),
                }),
                indices: vec![IndexArg::Whole],
                span: s(),
            },
        );
        let mut m = m;
        m.functions[0].params.push(Param {
            name: "a".into(),
            sir_type: None,
            kind: ParamKind::Required,
            default: None,
            span: s(),
        });
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn index_set_observes_nd_arrays_feature() {
        // a(1) = 9 — IndexSet without NDArrays declared → error.
        let mut m = empty_module(FeatureManifest::new());
        m.functions.push(Function {
            name: "f".into(),
            params: vec![Param {
                name: "a".into(),
                sir_type: None,
                kind: ParamKind::Required,
                default: None,
                span: s(),
            }],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::IndexSet {
                    target: Box::new(Expr::VarRef {
                        name: "a".into(),
                        scope: Scope::Param,
                        span: s(),
                    }),
                    indices: vec![IndexArg::Scalar(Box::new(Expr::IntLit {
                        value: 0,
                        span: s(),
                    }))],
                    value: Box::new(Expr::IntLit {
                        value: 9,
                        span: s(),
                    }),
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
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("nd-arrays")));
    }

    #[test]
    fn index_set_with_declared_feature_is_valid() {
        let mut m = empty_module(FeatureManifest::from_features(&[
            Feature::NDArrays,
            Feature::DynamicTyping,
        ]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![Param {
                name: "a".into(),
                sir_type: None,
                kind: ParamKind::Required,
                default: None,
                span: s(),
            }],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::IndexSet {
                    target: Box::new(Expr::VarRef {
                        name: "a".into(),
                        scope: Scope::Param,
                        span: s(),
                    }),
                    indices: vec![IndexArg::Whole],
                    value: Box::new(Expr::IntLit {
                        value: 9,
                        span: s(),
                    }),
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
    fn index_set_value_expression_is_validated() {
        // a(0) = ghost — the RHS value expr must still be scope-checked.
        let mut m = empty_module(FeatureManifest::from_features(&[Feature::NDArrays]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![Param {
                name: "a".into(),
                sir_type: None,
                kind: ParamKind::Required,
                default: None,
                span: s(),
            }],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::IndexSet {
                    target: Box::new(Expr::VarRef {
                        name: "a".into(),
                        scope: Scope::Param,
                        span: s(),
                    }),
                    indices: vec![IndexArg::Scalar(Box::new(Expr::IntLit {
                        value: 0,
                        span: s(),
                    }))],
                    value: Box::new(Expr::VarRef {
                        name: "ghost".into(),
                        scope: Scope::Local,
                        span: s(),
                    }),
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
        assert!(!r.is_ok());
        assert!(r
            .errors()
            .any(|i| i.message.contains("unknown name `ghost`")));
    }

    /// Pin the SIR22 "IndexSet is a Stmt, not an Expr" design rule at the
    /// validator layer: an `IndexSet` can only ever be reached via
    /// `check_stmt_seq` (it's not a variant of `Expr` at all, so
    /// `check_expr` cannot dispatch to it — this is enforced by the type
    /// system, not a runtime check). This test documents that a
    /// well-formed `IndexSet` statement validates cleanly through the
    /// statement path, confirming the mutation semantics SIR16's `Assign`
    /// established are followed here too (target checked, value checked,
    /// no `Expr`-position use possible).
    #[test]
    fn index_set_validates_via_stmt_path_only() {
        let mut m = empty_module(FeatureManifest::from_features(&[
            Feature::NDArrays,
            Feature::DynamicTyping,
        ]));
        m.functions.push(Function {
            name: "f".into(),
            params: vec![Param {
                name: "a".into(),
                sir_type: None,
                kind: ParamKind::Required,
                default: None,
                span: s(),
            }],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: vec![Stmt::IndexSet {
                    target: Box::new(Expr::VarRef {
                        name: "a".into(),
                        scope: Scope::Param,
                        span: s(),
                    }),
                    indices: vec![IndexArg::Whole],
                    value: Box::new(Expr::IntLit {
                        value: 1,
                        span: s(),
                    }),
                    span: s(),
                }],
                // The block's trailing *value* position is a plain Expr —
                // IndexSet could not be placed here even if we wanted to,
                // since Expr has no IndexSet variant to construct.
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

    // ── SIR23: symbolic expression + pattern/rewrite validator tests ──
    //
    // Same shape as the SIR22 block above: for every new node kind, one
    // test proves the module is REJECTED when the matching `Feature` is
    // not declared (the "manifest does not declare feature `X` but
    // module uses it" error from `compare_manifests`), and a companion
    // test proves it is ACCEPTED once declared. This is the validator-
    // level half of the SIR23 spec's most important correctness
    // property; `backend.rs` has the end-to-end `Backend::check_module`
    // half.

    #[test]
    fn sym_symbol_observes_symbolic_expr_feature() {
        let m = module_with_fn_body_value(
            FeatureManifest::new(),
            Expr::SymSymbol {
                name: "x".into(),
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("symbolic-expr")));
    }

    #[test]
    fn sym_symbol_with_declared_feature_is_valid() {
        let m = module_with_fn_body_value(
            FeatureManifest::from_features(&[Feature::SymbolicExpr]),
            Expr::SymSymbol {
                name: "x".into(),
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn sym_rational_observes_rationals_feature() {
        // SymRational reuses the SIR22 `Rationals` feature, not a new one.
        let m = module_with_fn_body_value(
            FeatureManifest::new(),
            Expr::SymRational {
                numer: 1,
                denom: 3,
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("rationals")));
    }

    #[test]
    fn sym_rational_with_declared_feature_is_valid() {
        let m = module_with_fn_body_value(
            FeatureManifest::from_features(&[Feature::Rationals]),
            Expr::SymRational {
                numer: 1,
                denom: 3,
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn sym_apply_observes_symbolic_expr_feature_and_validates_head_and_args() {
        // f(ghost) — the unresolvable local `ghost` inside args must
        // still be scope-checked, alongside the missing-feature error.
        let m = module_with_fn_body_value(
            FeatureManifest::new(),
            Expr::SymApply {
                head: Box::new(Expr::SymSymbol {
                    name: "f".into(),
                    span: s(),
                }),
                args: vec![Expr::VarRef {
                    name: "ghost".into(),
                    scope: Scope::Local,
                    span: s(),
                }],
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("symbolic-expr")));
        assert!(r
            .errors()
            .any(|i| i.message.contains("unknown name `ghost`")));
    }

    #[test]
    fn sym_apply_with_declared_feature_is_valid() {
        let m = module_with_fn_body_value(
            FeatureManifest::from_features(&[Feature::SymbolicExpr]),
            Expr::SymApply {
                head: Box::new(Expr::SymSymbol {
                    name: "f".into(),
                    span: s(),
                }),
                args: vec![Expr::IntLit {
                    value: 1,
                    span: s(),
                }],
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn sym_apply_computed_head_is_validated() {
        // f[x][y] — the outer SymApply's own head is a SymApply, which
        // must itself observe SymbolicExpr and be recursively validated.
        let m = module_with_fn_body_value(
            FeatureManifest::from_features(&[Feature::SymbolicExpr]),
            Expr::SymApply {
                head: Box::new(Expr::SymApply {
                    head: Box::new(Expr::SymSymbol {
                        name: "f".into(),
                        span: s(),
                    }),
                    args: vec![Expr::VarRef {
                        name: "ghost".into(),
                        scope: Scope::Local,
                        span: s(),
                    }],
                    span: s(),
                }),
                args: vec![],
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r
            .errors()
            .any(|i| i.message.contains("unknown name `ghost`")));
    }

    #[test]
    fn sym_pattern_blank_observes_pattern_matching_feature() {
        // Bare `_` (head: None).
        let m = module_with_fn_body_value(
            FeatureManifest::new(),
            Expr::SymPatternBlank {
                head: None,
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("pattern-matching")));
    }

    #[test]
    fn sym_pattern_blank_head_constrained_validates_head() {
        // `_h` where `h` is an unresolvable local — the head must be
        // scope-checked too.
        let m = module_with_fn_body_value(
            FeatureManifest::from_features(&[Feature::PatternMatching]),
            Expr::SymPatternBlank {
                head: Some(Box::new(Expr::VarRef {
                    name: "ghost".into(),
                    scope: Scope::Local,
                    span: s(),
                })),
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r
            .errors()
            .any(|i| i.message.contains("unknown name `ghost`")));
    }

    #[test]
    fn sym_pattern_blank_with_declared_feature_is_valid() {
        let m = module_with_fn_body_value(
            FeatureManifest::from_features(&[Feature::PatternMatching]),
            Expr::SymPatternBlank {
                head: None,
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn sym_pattern_named_observes_pattern_matching_feature() {
        let m = module_with_fn_body_value(
            FeatureManifest::new(),
            Expr::SymPatternNamed {
                name: "x".into(),
                pattern: Box::new(Expr::SymPatternBlank {
                    head: None,
                    span: s(),
                }),
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("pattern-matching")));
    }

    #[test]
    fn sym_pattern_named_with_declared_feature_is_valid() {
        let m = module_with_fn_body_value(
            FeatureManifest::from_features(&[Feature::PatternMatching]),
            Expr::SymPatternNamed {
                name: "x".into(),
                pattern: Box::new(Expr::SymPatternBlank {
                    head: None,
                    span: s(),
                }),
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }

    #[test]
    fn sym_rule_observes_pattern_matching_feature_and_validates_lhs_rhs() {
        let m = module_with_fn_body_value(
            FeatureManifest::new(),
            Expr::SymRule {
                lhs: Box::new(Expr::SymSymbol {
                    name: "x".into(),
                    span: s(),
                }),
                rhs: Box::new(Expr::VarRef {
                    name: "ghost".into(),
                    scope: Scope::Local,
                    span: s(),
                }),
                delayed: false,
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("pattern-matching")));
        assert!(r
            .errors()
            .any(|i| i.message.contains("unknown name `ghost`")));
    }

    #[test]
    fn sym_rule_with_declared_feature_is_valid() {
        // Both `->` (delayed: false) and `:>` (delayed: true) observe the
        // same feature.  `lhs`/`rhs` are plain `IntLit`s here (rather than
        // a `SymSymbol`) so this test stays focused on `SymRule`'s own
        // `PatternMatching` observation, without also pulling in
        // `SymbolicExpr` (a `SymSymbol`'s own feature).
        for delayed in [false, true] {
            let m = module_with_fn_body_value(
                FeatureManifest::from_features(&[Feature::PatternMatching]),
                Expr::SymRule {
                    lhs: Box::new(Expr::IntLit {
                        value: 1,
                        span: s(),
                    }),
                    rhs: Box::new(Expr::IntLit {
                        value: 0,
                        span: s(),
                    }),
                    delayed,
                    span: s(),
                },
            );
            let r = validate(&m);
            assert!(r.is_ok(), "expected ok for delayed={}, got {:?}", delayed, r.issues);
        }
    }

    #[test]
    fn sym_replace_all_observes_pattern_matching_feature_and_validates_rules() {
        let m = module_with_fn_body_value(
            FeatureManifest::new(),
            Expr::SymReplaceAll {
                expr: Box::new(Expr::SymSymbol {
                    name: "x".into(),
                    span: s(),
                }),
                rules: vec![Expr::SymRule {
                    lhs: Box::new(Expr::SymSymbol {
                        name: "x".into(),
                        span: s(),
                    }),
                    rhs: Box::new(Expr::VarRef {
                        name: "ghost".into(),
                        scope: Scope::Local,
                        span: s(),
                    }),
                    delayed: false,
                    span: s(),
                }],
                repeated: false,
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(r.errors().any(|i| i.message.contains("pattern-matching")));
        assert!(r
            .errors()
            .any(|i| i.message.contains("unknown name `ghost`")));
    }

    #[test]
    fn sym_replace_all_with_declared_feature_is_valid() {
        // Both `/.` (repeated: false) and `//.` (repeated: true) observe
        // the same feature.  `expr`/`lhs`/`rhs` are plain `IntLit`s here
        // (rather than `SymSymbol`s) for the same reason as
        // `sym_rule_with_declared_feature_is_valid` above: stay focused on
        // `SymReplaceAll`'s own `PatternMatching` observation without also
        // requiring `SymbolicExpr`.
        for repeated in [false, true] {
            let m = module_with_fn_body_value(
                FeatureManifest::from_features(&[Feature::PatternMatching]),
                Expr::SymReplaceAll {
                    expr: Box::new(Expr::IntLit {
                        value: 5,
                        span: s(),
                    }),
                    rules: vec![Expr::SymRule {
                        lhs: Box::new(Expr::IntLit {
                            value: 1,
                            span: s(),
                        }),
                        rhs: Box::new(Expr::IntLit {
                            value: 0,
                            span: s(),
                        }),
                        delayed: false,
                        span: s(),
                    }],
                    repeated,
                    span: s(),
                },
            );
            let r = validate(&m);
            assert!(
                r.is_ok(),
                "expected ok for repeated={}, got {:?}",
                repeated,
                r.issues
            );
        }
    }

    /// End-to-end version of the rejection tests above, in the same spirit
    /// as `backend.rs`'s `backend_rejects_module_whose_body_uses_array_lit_and_matmul`:
    /// a real module whose body nests `SymReplaceAll` around a `SymApply`
    /// and a `SymRule` with a `SymPatternNamed`/`SymPatternBlank` pattern —
    /// the full symbolic + pattern-matching vocabulary in one tree — with
    /// NO features declared, confirming every one of `SymbolicExpr` and
    /// `PatternMatching` is independently required.
    #[test]
    fn sym_replace_all_over_sym_apply_with_pattern_rule_requires_both_features() {
        // f(x) /. (x_ -> 0) — replace any argument matching the pattern
        // `x_` with 0 inside the symbolic application `f(x)`.
        let m = module_with_fn_body_value(
            FeatureManifest::new(),
            Expr::SymReplaceAll {
                expr: Box::new(Expr::SymApply {
                    head: Box::new(Expr::SymSymbol {
                        name: "f".into(),
                        span: s(),
                    }),
                    args: vec![Expr::SymSymbol {
                        name: "x".into(),
                        span: s(),
                    }],
                    span: s(),
                }),
                rules: vec![Expr::SymRule {
                    lhs: Box::new(Expr::SymPatternNamed {
                        name: "x".into(),
                        pattern: Box::new(Expr::SymPatternBlank {
                            head: None,
                            span: s(),
                        }),
                        span: s(),
                    }),
                    rhs: Box::new(Expr::IntLit {
                        value: 0,
                        span: s(),
                    }),
                    delayed: false,
                    span: s(),
                }],
                repeated: false,
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(!r.is_ok());
        assert!(
            r.errors().any(|i| i.message.contains("symbolic-expr")),
            "expected a symbolic-expr rejection, got {:?}",
            r.issues
        );
        assert!(
            r.errors().any(|i| i.message.contains("pattern-matching")),
            "expected a pattern-matching rejection, got {:?}",
            r.issues
        );
    }

    #[test]
    fn sym_replace_all_over_sym_apply_with_pattern_rule_valid_when_declared() {
        // The same tree as above, but with both required features
        // declared — must validate cleanly end to end.
        let m = module_with_fn_body_value(
            FeatureManifest::from_features(&[Feature::SymbolicExpr, Feature::PatternMatching]),
            Expr::SymReplaceAll {
                expr: Box::new(Expr::SymApply {
                    head: Box::new(Expr::SymSymbol {
                        name: "f".into(),
                        span: s(),
                    }),
                    args: vec![Expr::SymSymbol {
                        name: "x".into(),
                        span: s(),
                    }],
                    span: s(),
                }),
                rules: vec![Expr::SymRule {
                    lhs: Box::new(Expr::SymPatternNamed {
                        name: "x".into(),
                        pattern: Box::new(Expr::SymPatternBlank {
                            head: None,
                            span: s(),
                        }),
                        span: s(),
                    }),
                    rhs: Box::new(Expr::IntLit {
                        value: 0,
                        span: s(),
                    }),
                    delayed: false,
                    span: s(),
                }],
                repeated: false,
                span: s(),
            },
        );
        let r = validate(&m);
        assert!(r.is_ok(), "expected ok, got {:?}", r.issues);
    }
}
