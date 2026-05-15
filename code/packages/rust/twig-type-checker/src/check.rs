//! Two-pass type-checking walker for Twig programs (TW05-B).
//!
//! ## Overview of the two passes
//!
//! ```text
//! Program
//!   │
//!   ▼ Pass 1: collect_forms
//! TypeEnv  ─────────────────────────────────┐
//!   │                                        │
//!   ▼ Pass 2: check_forms                   │
//! (errors appended, kinds inferred)  ◄──────┘
//! ```
//!
//! ### Pass 1 — declaration collection
//!
//! Walk every top-level `Form` once.  For each:
//!
//! - `TypeAlias` → register in `env.aliases`.
//! - `RecordDef` → register in `env.records`; expose constructor in `env.globals`.
//! - `UnionDef` → register in `env.unions`; expose variant constructors.
//! - `Define` → classify as `Function { arity }` or `Any` (or use annotation)
//!   and register in `env.globals`.
//!
//! The purpose of Pass 1 is to make every top-level name visible *before*
//! any body is walked, so mutual recursion and forward references work
//! correctly.
//!
//! ### Pass 2 — expression walking
//!
//! Walk every `Form::Define` and `Form::Expr` body with `infer_expr`.
//! `infer_expr` is mutually recursive with itself:
//!
//! - `Lambda` pushes a frame, walks the body, pops the frame.
//! - `Let` evaluates RHS in the outer scope, pushes a frame, walks the body.
//! - `Match` checks exhaustiveness, then walks each arm in its own frame.
//! - `Apply` checks arity when the callee resolves to a `Function`.
//!
//! ## Ownership and borrowing
//!
//! `TypeEnv` is shared immutably by all recursive calls to `infer_expr` —
//! it's fully built before Pass 2 starts.  `ScopeStack` is mutable and
//! threaded through each recursive call.  `errors` is a mutable `Vec`
//! that any sub-call can append to.

use lang_refinement_protocol::{
    check_call_site_refinements, compute_if_narrowing, RefinementMode,
};
use twig_parser::{
    Apply, Begin, Define, Expr, Form, If, Lambda, Let,
    // LANG52: sequential let* bindings
    LetStar,
    Match, MatchPat, Program, TypedMode,
};
use type_checker_protocol::TypeErrorDiagnostic;

use crate::arity::check_arity;
use crate::bridge::TwigRefinementBridge;
use crate::env::{ScopeStack, TypeEnv};
use crate::exhaustiveness::check_exhaustiveness;
use crate::kinds::{annotation_to_refined_type, type_annotation_to_kind, TwigKind};

// ---------------------------------------------------------------------------
// Pass 1 — declaration collection
// ---------------------------------------------------------------------------

/// Walk all top-level `Form`s and populate `env` with every declared name.
///
/// After this function returns, `env.globals`, `env.aliases`, `env.records`,
/// and `env.unions` are fully populated.  No errors are generated in Pass 1 —
/// even if a type annotation references an unknown name, we wait until Pass 2
/// to report it.
///
/// The `mode` parameter is not used in Pass 1 (error generation is a Pass 2
/// concern), but is threaded through for symmetry.
pub fn collect_forms(program: &Program, env: &mut TypeEnv, _mode: &TypedMode) {
    // (_mode is kept for API symmetry and future strict-mode annotation checking.)
    for form in &program.forms {
        match form {
            Form::TypeAlias(t) => {
                env.register_alias(t.name.clone(), t.expr.clone());
            }
            Form::RecordDef(r) => {
                env.register_record(r);
            }
            Form::UnionDef(u) => {
                env.register_union(u);
            }
            Form::Define(def) => {
                classify_define(def, env);
            }
            Form::Expr(_) => {
                // Bare top-level expressions are not named bindings.
            }
        }
    }
}

/// Determine the `TwigKind` for a `Define` and register it in `env.globals`.
///
/// Priority:
/// 1. If `def.expr` is a `Lambda` → `Function { arity: params.len() }`.
///    Also registers per-parameter `RefinedType`s in `fn_param_refinements`
///    for any annotated parameters (TW05-C).
/// 2. If `def.type_annotation` is `Some(ann)` → use the annotation.
/// 3. Otherwise → `Any` (no static info available yet).
fn classify_define(def: &Define, env: &mut TypeEnv) {
    let kind = match &def.expr {
        Expr::Lambda(lam) => {
            // TW05-C: build the per-parameter refinement vec and store it.
            // `param_annotations[i]` is `Some(ann)` when the i-th parameter
            // carries a type annotation; `None` (or absent) otherwise.
            let refinements: Vec<Option<lang_refined_types::RefinedType>> = (0..lam.params.len())
                .map(|i| {
                    lam.param_annotations
                        .get(i)
                        .and_then(|opt_ann| opt_ann.as_ref())
                        .and_then(|ann| annotation_to_refined_type(ann))
                })
                .collect();
            env.register_fn_refinements(def.name.clone(), refinements);

            TwigKind::Function {
                arity: lam.params.len(),
            }
        }
        _ => match &def.type_annotation {
            Some(ann) => type_annotation_to_kind(ann, env),
            None => TwigKind::Any,
        },
    };
    env.bind_global(def.name.clone(), kind);
}

// ---------------------------------------------------------------------------
// Pass 2 — expression walking
// ---------------------------------------------------------------------------

/// Walk all `Form::Define` and `Form::Expr` bodies, collecting type errors.
///
/// After this function returns, `errors` contains all type violations found
/// in the program under `mode`.
pub fn check_forms(
    program: &Program,
    env: &TypeEnv,
    scope: &mut ScopeStack,
    mode: &TypedMode,
    errors: &mut Vec<TypeErrorDiagnostic>,
) {
    for form in &program.forms {
        match form {
            Form::Define(def) => {
                // Check the body expression, but ignore the returned kind — the
                // caller already registered the declared kind for this name.
                infer_expr(&def.expr, env, scope, mode, errors);
            }
            Form::Expr(expr) => {
                infer_expr(expr, env, scope, mode, errors);
            }
            // Declarations were processed in Pass 1; nothing to do here.
            Form::TypeAlias(_) | Form::RecordDef(_) | Form::UnionDef(_) => {}
        }
    }
}

// ---------------------------------------------------------------------------
// infer_expr
// ---------------------------------------------------------------------------

/// Infer the `TwigKind` of an expression, appending any type errors found.
///
/// This function recurses over the entire expression tree.  It is the heart
/// of TW05-B.  Each arm documents what it checks and what kind it returns.
pub fn infer_expr(
    expr: &Expr,
    env: &TypeEnv,
    scope: &mut ScopeStack,
    mode: &TypedMode,
    errors: &mut Vec<TypeErrorDiagnostic>,
) -> TwigKind {
    match expr {
        // ── Atoms — trivial kinds ────────────────────────────────────────────
        Expr::IntLit(_) => TwigKind::Int,
        Expr::BoolLit(_) => TwigKind::Bool,
        Expr::NilLit(_) => TwigKind::Nil,
        Expr::SymLit(_) => TwigKind::Symbol,
        // LANG51: string literals have kind Str.
        Expr::StrLit(_) => TwigKind::Str,

        // ── Variable reference ───────────────────────────────────────────────
        Expr::VarRef(v) => infer_var_ref(v.name.as_str(), v.line, v.column, env, scope, mode, errors),

        // ── Lambda ──────────────────────────────────────────────────────────
        Expr::Lambda(lam) => infer_lambda(lam, env, scope, mode, errors),

        // ── Function application ─────────────────────────────────────────────
        Expr::Apply(app) => infer_apply(app, env, scope, mode, errors),

        // ── Conditional ─────────────────────────────────────────────────────
        Expr::If(if_expr) => infer_if(if_expr, env, scope, mode, errors),

        // ── Let binding ─────────────────────────────────────────────────────
        Expr::Let(let_expr) => infer_let(let_expr, env, scope, mode, errors),

        // ── Sequential let* binding (LANG52) ─────────────────────────────────
        //
        // For type checking purposes, let* behaves like a chain of nested lets.
        // We walk each binding's RHS in the scope that includes all prior
        // bindings, then walk the body in the full accumulated scope.
        Expr::LetStar(let_star) => infer_let_star(let_star, env, scope, mode, errors),

        // ── Sequencing ──────────────────────────────────────────────────────
        Expr::Begin(begin) => infer_begin(begin, env, scope, mode, errors),

        // ── Pattern matching ─────────────────────────────────────────────────
        Expr::Match(m) => infer_match(m, env, scope, mode, errors),
    }
}

// ---------------------------------------------------------------------------
// VarRef resolution
// ---------------------------------------------------------------------------

fn infer_var_ref(
    name: &str,
    line: usize,
    column: usize,
    env: &TypeEnv,
    scope: &ScopeStack,
    _mode: &TypedMode, // Reserved: future strict vs lenient differentiation within resolution
    errors: &mut Vec<TypeErrorDiagnostic>,
) -> TwigKind {
    // Resolution order: local scope first, then globals.
    if let Some(kind) = scope.lookup(name) {
        return kind.clone();
    }
    if let Some(kind) = env.lookup_global(name) {
        return kind.clone();
    }
    // Name not found.  Both Strict and Lenient modes collect the diagnostic;
    // the difference is that Strict sets ok:false while Lenient keeps ok:true.
    // Off mode never reaches here (check_program returns early for Off).
    errors.push(TypeErrorDiagnostic {
        message: format!("unresolved variable `{name}`"),
        line,
        column,
    });
    TwigKind::Any
}

// ---------------------------------------------------------------------------
// Lambda
// ---------------------------------------------------------------------------

/// Infer a `lambda` expression.
///
/// 1. Push a new scope frame.
/// 2. Bind each parameter with its annotation kind (or `Any` if unannotated).
/// 3. Infer each body expression; keep only the last kind.
/// 4. Pop the frame.
/// 5. Return `Function { arity: params.len() }`.
///
/// The return kind of the lambda body is not yet propagated — in TW05-B we
/// don't track declared return types in the kind system.  TW05-C will use the
/// `return_annotation` field for that.
fn infer_lambda(
    lam: &Lambda,
    env: &TypeEnv,
    scope: &mut ScopeStack,
    mode: &TypedMode,
    errors: &mut Vec<TypeErrorDiagnostic>,
) -> TwigKind {
    scope.push_frame();

    // Bind each parameter.  Annotation kinds come from `param_annotations`;
    // parameters without an annotation get `Any`.
    for (i, param_name) in lam.params.iter().enumerate() {
        let kind = lam
            .param_annotations
            .get(i)
            .and_then(|a| a.as_ref())
            .map(|ann| type_annotation_to_kind(ann, env))
            .unwrap_or(TwigKind::Any);
        scope.bind(param_name, kind);
    }

    // Infer body; take the kind of the last expression.
    let mut body_kind = TwigKind::Any;
    for e in &lam.body {
        body_kind = infer_expr(e, env, scope, mode, errors);
    }

    scope.pop_frame();

    // The lambda itself is a callable — its kind is Function, regardless of
    // what the body evaluates to.
    let _ = body_kind; // Not propagated in TW05-B.
    TwigKind::Function {
        arity: lam.params.len(),
    }
}

// ---------------------------------------------------------------------------
// Apply
// ---------------------------------------------------------------------------

/// Infer a function-application expression `(fn arg0 arg1 …)`.
///
/// If the function position resolves to `Function { arity }`, the actual
/// argument count is compared and an arity error is emitted on mismatch.
///
/// ## TW05-C: call-site refinement checking
///
/// When the callee is a named function with registered parameter refinements
/// (`env.fn_param_refinements`), we run a per-binding proof obligation for
/// each argument that corresponds to an annotated parameter:
///
/// | Argument kind | Evidence | Outcome |
/// |---------------|----------|---------|
/// | `IntLit(n)` | `Concrete(n)` | checked exactly |
/// | `VarRef → RefinedInt(p)` in scope | `Predicated([p])` | checked under assumption `p` |
/// | `VarRef → Int` or `Any` | `Unconstrained` | always `Unknown` |
/// | anything else | `Unconstrained` | always `Unknown` |
///
/// `Unknown` in lenient mode → silent.  In strict mode → error.
/// `ProvenUnsafe(cx)` → error in all modes.
///
/// The return kind is always `Any` — return-type annotation checking is
/// handled by `iir-refinement-pass` (LANG42) at the IIR level.
fn infer_apply(
    app: &Apply,
    env: &TypeEnv,
    scope: &mut ScopeStack,
    mode: &TypedMode,
    errors: &mut Vec<TypeErrorDiagnostic>,
) -> TwigKind {
    // Infer the function-position expression first.
    let fn_kind = infer_expr(&app.fn_expr, env, scope, mode, errors);

    // Extract the function name (for error messages) if the callee is a simple VarRef.
    let fn_name: Option<&str> = match app.fn_expr.as_ref() {
        Expr::VarRef(v) => Some(v.name.as_str()),
        _ => None,
    };

    // Infer all argument expressions (collect their kinds for refinement checking).
    let arg_kinds: Vec<TwigKind> = app.args
        .iter()
        .map(|arg| infer_expr(arg, env, scope, mode, errors))
        .collect();

    // Arity check: only fires when the function kind has a known arity.
    if let TwigKind::Function { arity: expected } = fn_kind {
        check_arity(fn_name, expected, app.args.len(), app.line, app.column, errors);
    }

    // TW05-C / LANG54: call-site refinement checking via the generic protocol.
    // Only runs when the callee is a VarRef with registered param refinements.
    if let Some(callee_name) = fn_name {
        if let Some(param_refinements) = env.fn_param_refinements.get(callee_name) {
            // Map TypedMode → RefinementMode for the protocol.
            let ref_mode = match mode {
                TypedMode::Strict  => RefinementMode::Strict,
                // Lenient and Off both use Lenient — Off never reaches here
                // (check_program returns early), but be defensive.
                _ => RefinementMode::Lenient,
            };

            // Delegate to the generic free function via TwigRefinementBridge.
            // All evidence classification, Checker invocation, and outcome
            // handling is now in lang-refinement-protocol.
            let diags = check_call_site_refinements(
                &TwigRefinementBridge,
                callee_name,
                app.line,
                app.column,
                &app.args,
                &arg_kinds,
                param_refinements,
                ref_mode,
            );
            for d in diags {
                errors.push(TypeErrorDiagnostic {
                    message: d.message,
                    line: d.line,
                    column: d.column,
                });
            }
        }
    }

    // Return kind unknown without a return-type annotation.
    TwigKind::Any
}

// ---------------------------------------------------------------------------
// If
// ---------------------------------------------------------------------------

/// Infer an `if` expression `(if cond then else)`.
///
/// Both branches are inferred.  If they agree on a kind, that kind is
/// returned.  Otherwise `TwigKind::unify` widens them (see `kinds::unify`).
///
/// ## TW05-C / LANG54: flow-sensitive narrowing via the generic protocol
///
/// We call [`compute_if_narrowing`] (from `lang-refinement-protocol`) with
/// [`TwigRefinementBridge`].  The generic function:
///
/// 1. Calls `bridge.narrowing_facts(cond)` → per-variable predicates implied
///    by the guard being true.
/// 2. Looks up each variable's current kind via the provided `scope_lookup`
///    closure.
/// 3. Returns [`NarrowedBindings`] with `true_branch` and `false_branch`
///    binding lists.
///
/// We then push a scope frame, apply the bindings, infer the branch, and pop.
///
/// ### Example
///
/// ```scheme
/// (define (f n)
///   (if (< n 128)
///     (ascii-info n)   ;; true branch: n narrowed to RefinedInt(n < 128)
///     0))              ;; false branch: n narrowed to RefinedInt(NOT(n < 128))
/// ```
///
/// The condition is also inferred (to collect errors in it) but its kind
/// is not checked — Twig uses dynamic truthiness, not a static bool
/// requirement.
fn infer_if(
    if_expr: &If,
    env: &TypeEnv,
    scope: &mut ScopeStack,
    mode: &TypedMode,
    errors: &mut Vec<TypeErrorDiagnostic>,
) -> TwigKind {
    // Always infer the condition for error collection.
    infer_expr(&if_expr.cond, env, scope, mode, errors);

    // LANG54: delegate narrowing computation to the generic protocol.
    // `compute_if_narrowing` calls TwigRefinementBridge::narrowing_facts and
    // TwigRefinementBridge::narrow_kind internally — no Twig-specific logic here.
    let narrowed = compute_if_narrowing(
        &TwigRefinementBridge,
        &if_expr.cond,
        // Scope lookup: local scope first, then module globals.
        // Globals are typically not refined kinds (module-level names are always
        // Function / Any / Int etc., never narrowed by a guard), but we include
        // them here for completeness.
        |var| scope.lookup(var).cloned().or_else(|| env.lookup_global(var).cloned()),
    );

    // ── True branch ──────────────────────────────────────────────────────────
    scope.push_frame();
    for (var, kind) in narrowed.true_branch {
        scope.bind(&var, kind);
    }
    let then_kind = infer_expr(&if_expr.then_branch, env, scope, mode, errors);
    scope.pop_frame();

    // ── False branch ─────────────────────────────────────────────────────────
    scope.push_frame();
    for (var, kind) in narrowed.false_branch {
        scope.bind(&var, kind);
    }
    let else_kind = infer_expr(&if_expr.else_branch, env, scope, mode, errors);
    scope.pop_frame();

    TwigKind::unify(then_kind, else_kind)
}

// ---------------------------------------------------------------------------
// Let
// ---------------------------------------------------------------------------

/// Infer a `(let ((x e) …) body+)` expression.
///
/// Twig uses Scheme-style `let` (not `let*`): all RHS expressions are
/// evaluated in the *outer* scope, then all bindings are introduced together.
/// This means `(let ((x 1) (y x)) y)` uses the outer `x`, not the `x` just
/// bound by this `let`.
///
/// Steps:
/// 1. Infer each RHS in the *current* scope (before pushing the new frame).
/// 2. Push a new frame.
/// 3. Bind each name to its inferred RHS kind.
/// 4. Infer each body expression in the new frame.
/// 5. Pop the frame.
/// 6. Return the last body expression's kind.
fn infer_let(
    let_expr: &Let,
    env: &TypeEnv,
    scope: &mut ScopeStack,
    mode: &TypedMode,
    errors: &mut Vec<TypeErrorDiagnostic>,
) -> TwigKind {
    // Step 1: infer all RHS in the outer scope.
    let binding_kinds: Vec<(String, TwigKind)> = let_expr
        .bindings
        .iter()
        .map(|(name, rhs)| {
            let kind = infer_expr(rhs, env, scope, mode, errors);
            (name.clone(), kind)
        })
        .collect();

    // Step 2 & 3: push frame and bind.
    scope.push_frame();
    for (name, kind) in binding_kinds {
        scope.bind(&name, kind);
    }

    // Step 4: infer body.
    let mut last_kind = TwigKind::Any;
    for e in &let_expr.body {
        last_kind = infer_expr(e, env, scope, mode, errors);
    }

    // Step 5: pop frame.
    scope.pop_frame();

    last_kind
}

// ---------------------------------------------------------------------------
// LetStar (LANG52)
// ---------------------------------------------------------------------------

/// Infer a `(let* ((x e1) (y e2) …) body+)` expression.
///
/// Unlike `infer_let`, each binding's RHS is evaluated in the scope that
/// includes all *prior* bindings.  Modelled as a single push-frame followed
/// by incremental binding additions.
fn infer_let_star(
    let_star: &LetStar,
    env: &TypeEnv,
    scope: &mut ScopeStack,
    mode: &TypedMode,
    errors: &mut Vec<TypeErrorDiagnostic>,
) -> TwigKind {
    scope.push_frame();

    // Evaluate each RHS in the accumulated scope, then immediately bind.
    for (name, rhs) in &let_star.bindings {
        let kind = infer_expr(rhs, env, scope, mode, errors);
        scope.bind(name, kind);
    }

    // Infer body.
    let mut last_kind = TwigKind::Any;
    for e in &let_star.body {
        last_kind = infer_expr(e, env, scope, mode, errors);
    }

    scope.pop_frame();
    last_kind
}

// ---------------------------------------------------------------------------
// Begin
// ---------------------------------------------------------------------------

/// Infer a `(begin e1 e2 …)` expression.
///
/// Each sub-expression is inferred (to collect errors).  The kind returned
/// is that of the last expression; earlier expression kinds are discarded.
fn infer_begin(
    begin: &Begin,
    env: &TypeEnv,
    scope: &mut ScopeStack,
    mode: &TypedMode,
    errors: &mut Vec<TypeErrorDiagnostic>,
) -> TwigKind {
    let mut last = TwigKind::Any;
    for e in &begin.exprs {
        last = infer_expr(e, env, scope, mode, errors);
    }
    last
}

// ---------------------------------------------------------------------------
// Match
// ---------------------------------------------------------------------------

/// Infer a `(match scrutinee arm+)` expression.
///
/// Steps:
/// 1. Infer the scrutinee's kind.
/// 2. If the kind is `Union(name)`, check exhaustiveness.
/// 3. For each arm:
///    a. Push a new frame.
///    b. Bind pattern variables (`Variant` → binds field names as `Any`;
///       `Binding(n)` → binds the whole scrutinee as `Any`;
///       `Wildcard` → no binding).
///    c. Infer each body expression.
///    d. Pop the frame.
/// 4. Return `Any` (arm kinds not unified in TW05-B).
fn infer_match(
    m: &Match,
    env: &TypeEnv,
    scope: &mut ScopeStack,
    mode: &TypedMode,
    errors: &mut Vec<TypeErrorDiagnostic>,
) -> TwigKind {
    // Step 1: infer scrutinee.
    let scrutinee_kind = infer_expr(&m.scrutinee, env, scope, mode, errors);

    // Step 2: exhaustiveness check (only for union scrutinees in strict mode
    // or lenient mode — we always emit the diagnostic regardless of mode since
    // a non-exhaustive match is a semantic error in any mode).
    if let TwigKind::Union(ref union_name) = scrutinee_kind {
        check_exhaustiveness(
            union_name,
            &m.arms,
            env,
            m.line,
            m.column,
            errors,
        );
    }

    // Step 3: walk arms.
    for arm in &m.arms {
        scope.push_frame();

        match &arm.pat {
            MatchPat::Variant { bindings, .. } => {
                // Bind each field name as `Any` (TW05-B doesn't thread field
                // types through yet — that's a TW05-C follow-up).
                for b in bindings {
                    scope.bind(b, TwigKind::Any);
                }
            }
            MatchPat::Binding(name) => {
                // Bind the whole scrutinee value as `Any`.
                scope.bind(name, scrutinee_kind.clone());
            }
            MatchPat::Wildcard => {
                // No bindings introduced.
            }
        }

        // Infer body (collect errors, discard kinds).
        for e in &arm.body {
            infer_expr(e, env, scope, mode, errors);
        }

        scope.pop_frame();
    }

    // Return kind is `Any` — arm kinds are not unified in TW05-B.
    TwigKind::Any
}
