//! The lowering pass from `twig_parser::Program` → `semantic_ir::Module`.
//!
//! Two phases:
//!
//! 1. **Top-level collection** — split forms into `value_defines`,
//!    `fn_defines`, and `bare_exprs`.  Build the set of known
//!    function names and global names.
//!
//! 2. **Lowering** — walk each function / lambda / bare expression,
//!    producing SIR nodes.  Free-variable analysis is run per
//!    `lambda` form to compute its capture set.  Lambdas are
//!    promoted to fresh top-level `semantic_ir::Function`s with
//!    gensym'd names; the source-position lambda becomes a
//!    `MakeClosure` referencing the synthesised function.
//!
//! ## Scope resolution
//!
//! The lowerer maintains a per-function context with three sets:
//!
//! - `params`   — parameter names of the current function.
//! - `captures` — capture names of the current function (empty for
//!                top-level non-closure functions).
//! - `locals`   — a *stack* of let-bound names; pushed when entering
//!                `let` / `let*` groups, popped on exit.
//!
//! Name resolution at any point walks these in order:
//! locals (innermost-first) → params → captures → module globals →
//! module function names → builtins → unresolved (lowering error).

use std::collections::HashSet;

use semantic_ir::{
    Block, CaptureValue, Effect, EffectSet, ExportName, Expr, Feature, FeatureManifest, Function,
    Global, Import, ImportName, Metadata, Module, Param, Scope, Span, Stmt,
};
use twig_parser::{
    Apply, Begin, BoolLit, Expr as TExpr, Form, If as TIf, IntLit, Lambda as TLambda,
    Let as TLet, LetStar as TLetStar, NilLit, Program, StrLit, SymLit, VarRef,
};

use crate::builtins;

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// An error encountered during Twig → SIR lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwigLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for TwigLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TwigLowerError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for TwigLowerError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed Twig program into a SIR module.
pub fn compile(program: &Program, module_name: &str) -> Result<Module, TwigLowerError> {
    let mut lw = Lowerer::new(module_name);
    lw.lower_program(program)
}

// ---------------------------------------------------------------------------
// The lowerer
// ---------------------------------------------------------------------------

const FILE: &str = "<twig>";

struct Lowerer {
    module_name: String,
    /// Top-level function names declared in this module (including
    /// synthesised lambdas, `_init`, and `main`).
    function_names: HashSet<String>,
    /// Top-level value defines.
    global_names: HashSet<String>,
    /// Synthesised closure-body functions (one per `(lambda ...)`).
    synthesised: Vec<Function>,
    /// Gensym counter for synthesised lambda names.
    lambda_counter: usize,
    /// Observed features — appended to throughout lowering.
    observed: FeatureManifest,
}

impl Lowerer {
    fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            function_names: HashSet::new(),
            global_names: HashSet::new(),
            synthesised: Vec::new(),
            lambda_counter: 0,
            observed: FeatureManifest::new(),
        }
    }

    // -------------------------------------------------------------------
    // top-level pipeline
    // -------------------------------------------------------------------

    fn lower_program(&mut self, p: &Program) -> Result<Module, TwigLowerError> {
        // 1. Reject deferred LANG48 forms up front.  v0 of the SIR
        //    has no node kinds for these.
        for form in &p.forms {
            match form {
                Form::TypeAlias(t) => {
                    return Err(TwigLowerError {
                        message: "type aliases not supported in SIR v0".into(),
                        line: t.line,
                        column: t.column,
                    });
                }
                Form::RecordDef(r) => {
                    return Err(TwigLowerError {
                        message: "record declarations not supported in SIR v0".into(),
                        line: r.line,
                        column: r.column,
                    });
                }
                Form::UnionDef(u) => {
                    return Err(TwigLowerError {
                        message: "union declarations not supported in SIR v0".into(),
                        line: u.line,
                        column: u.column,
                    });
                }
                _ => {}
            }
        }

        // 2. First pass: collect names.  Function names include
        //    user-declared functions plus the synthesised `_init` and
        //    `main`.
        for form in &p.forms {
            if let Form::Define(def) = form {
                if let TExpr::Lambda(_) = &def.expr {
                    self.function_names.insert(def.name.clone());
                } else {
                    self.global_names.insert(def.name.clone());
                }
            }
        }
        self.function_names.insert("_init".to_string());
        self.function_names.insert("main".to_string());

        // 3. Second pass: lower each form.
        let mut user_fns: Vec<Function> = Vec::new();
        let mut globals: Vec<Global> = Vec::new();
        let mut init_stmts: Vec<Stmt> = Vec::new();
        let mut main_stmts: Vec<Stmt> = Vec::new();
        let mut main_value: Option<Expr> = None;

        for form in &p.forms {
            match form {
                Form::Define(def) => {
                    if let TExpr::Lambda(lam) = &def.expr {
                        let f = self.lower_top_level_lambda(&def.name, lam)?;
                        user_fns.push(f);
                    } else {
                        // Value define → Global + init statement.
                        let span = self.span_at(def.line, def.column);
                        globals.push(Global {
                            name: def.name.clone(),
                            sir_type: None,
                            init_function: "_init".to_string(),
                            span: span.clone(),
                        });
                        let mut ctx = FunctionCtx::for_top_level(
                            self.global_names.clone(),
                            self.function_names.clone(),
                        );
                        let value = self.lower_expr(&def.expr, &mut ctx)?;
                        // The synthesised global_set call uses a
                        // SymLit for the global's name — note that
                        // in the manifest so the validator doesn't
                        // flag it as undeclared use of Symbols.
                        self.observed.add(Feature::Symbols);
                        init_stmts.push(Stmt::ExprStmt {
                            expr: Expr::BuiltinCall {
                                name: "global_set".into(),
                                args: vec![
                                    Expr::SymLit { name: def.name.clone(), span: span.clone() },
                                    value,
                                ],
                                effects: builtins::effects_for("global_set"),
                                span: span.clone(),
                            },
                            span,
                        });
                    }
                }
                Form::Expr(e) => {
                    let mut ctx = FunctionCtx::for_top_level(
                        self.global_names.clone(),
                        self.function_names.clone(),
                    );
                    let lowered = self.lower_expr(e, &mut ctx)?;
                    // The *last* bare expression becomes main's value;
                    // earlier ones are ExprStmts.  We accumulate
                    // expressions and convert the last one at the end.
                    if let Some(prev) = main_value.take() {
                        let span = prev.span().clone();
                        main_stmts.push(Stmt::ExprStmt { expr: prev, span });
                    }
                    main_value = Some(lowered);
                }
                Form::TypeAlias(_) | Form::RecordDef(_) | Form::UnionDef(_) => {
                    unreachable!("rejected in stage 1");
                }
            }
        }

        // 4. Synthesise `_init` (only if there are value defines).
        let synthetic_span = Span::point(FILE, 0, 0);
        if !init_stmts.is_empty() {
            let init = Function {
                name: "_init".to_string(),
                params: vec![],
                return_type: None,
                captures: vec![],
                body: Block {
                    stmts: init_stmts,
                    value: Expr::NilLit { span: synthetic_span.clone() },
                    span: synthetic_span.clone(),
                },
                effects: EffectSet::PURE,
                metadata: Metadata::new(),
                span: synthetic_span.clone(),
            };
            user_fns.push(init);
        }

        // 5. Synthesise `main`.  Its body is the accumulated bare
        //    expressions; the value is the last bare expression or
        //    `nil` if none.
        let main_body_value =
            main_value.unwrap_or(Expr::NilLit { span: synthetic_span.clone() });
        let main = Function {
            name: "main".to_string(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts: main_stmts,
                value: main_body_value,
                span: synthetic_span.clone(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: synthetic_span.clone(),
        };
        user_fns.push(main);

        // 6. Combine user fns + synthesised lambda fns.
        let mut functions: Vec<Function> = Vec::with_capacity(user_fns.len() + self.synthesised.len());
        functions.append(&mut self.synthesised);
        functions.append(&mut user_fns);

        // 7. Manifest fixups: mutual recursion is conservative true
        //    whenever there's more than one user function.  Globals
        //    & DynamicTyping are auto-deduced via the per-node logic.
        if user_fns_with_at_least_two_user_defines(&functions) {
            self.observed.add(Feature::MutualRecursion);
        }
        if !globals.is_empty() {
            self.observed.add(Feature::Globals);
        }
        if functions.iter().any(|f| f.params.iter().any(|p| p.sir_type.is_none())) {
            self.observed.add(Feature::DynamicTyping);
        }

        // 8. Build module.
        let module_span = Span::point(FILE, 1, 1);
        let mut metadata = Metadata::new()
            .with_source_language("twig")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION);
        if let Some(mi) = &p.module_info {
            metadata = metadata.with_extra("module-path", mi.name.clone());
        }
        let exports = p
            .module_info
            .as_ref()
            .map(|mi| {
                mi.exports
                    .iter()
                    .map(|n| ExportName {
                        name: n.clone(),
                        span: module_span.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let imports = p
            .module_info
            .as_ref()
            .map(|mi| {
                mi.imports
                    .iter()
                    .map(|path| Import {
                        module_path: path.clone(),
                        names: Vec::<ImportName>::new(),
                        span: module_span.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(Module {
            name: self.module_name.clone(),
            manifest: self.observed.clone(),
            imports,
            exports,
            functions,
            globals,
            metadata,
            span: module_span,
        })
    }

    // -------------------------------------------------------------------
    // Functions and lambdas
    // -------------------------------------------------------------------

    fn lower_top_level_lambda(
        &mut self,
        name: &str,
        lam: &TLambda,
    ) -> Result<Function, TwigLowerError> {
        // Top-level functions have no captures — references to outer
        // bindings are resolved as Global / Builtin.  A free-name
        // analysis still runs for two purposes:
        // 1. Validate that all referenced names resolve (catches
        //    typos at lowering time, not just at runtime).
        // 2. Surface a clear error message at lowering time when a
        //    name escapes.
        let mut ctx = FunctionCtx::new(
            lam.params.clone(),
            HashSet::new(), // no captures at top level
            self.global_names.clone(),
            self.function_names.clone(),
        );
        let body = self.lower_lambda_body(&lam.body, &mut ctx, lam.line, lam.column)?;
        Ok(Function {
            name: name.to_string(),
            params: lam.params.iter().map(|p| Param {
                name: p.clone(),
                sir_type: None,
                span: self.span_at(lam.line, lam.column),
            }).collect(),
            return_type: None,
            captures: vec![],
            body,
            effects: EffectSet::PURE, // conservative; v0 doesn't propagate
            metadata: Metadata::new(),
            span: self.span_at(lam.line, lam.column),
        })
    }

    fn lower_lambda_body(
        &mut self,
        body: &[TExpr],
        ctx: &mut FunctionCtx,
        line: usize,
        column: usize,
    ) -> Result<Block, TwigLowerError> {
        if body.is_empty() {
            return Err(TwigLowerError {
                message: "lambda body is empty".into(),
                line,
                column,
            });
        }
        let span = self.span_at(line, column);
        let mut stmts = Vec::with_capacity(body.len() - 1);
        for e in &body[..body.len() - 1] {
            let lowered = self.lower_expr(e, ctx)?;
            let s = lowered.span().clone();
            stmts.push(Stmt::ExprStmt { expr: lowered, span: s });
        }
        let value = self.lower_expr(body.last().unwrap(), ctx)?;
        Ok(Block { stmts, value, span })
    }

    fn fresh_lambda_name(&mut self) -> String {
        let name = format!("__lambda_{}", self.lambda_counter);
        self.lambda_counter += 1;
        self.function_names.insert(name.clone());
        name
    }

    // -------------------------------------------------------------------
    // Expression lowering
    // -------------------------------------------------------------------

    fn lower_expr(&mut self, e: &TExpr, ctx: &mut FunctionCtx) -> Result<Expr, TwigLowerError> {
        match e {
            TExpr::IntLit(IntLit { value, line, column }) => Ok(Expr::IntLit {
                value: *value,
                span: self.span_at(*line, *column),
            }),
            TExpr::BoolLit(BoolLit { value, line, column }) => Ok(Expr::BoolLit {
                value: *value,
                span: self.span_at(*line, *column),
            }),
            TExpr::NilLit(NilLit { line, column }) => Ok(Expr::NilLit {
                span: self.span_at(*line, *column),
            }),
            TExpr::SymLit(SymLit { name, line, column }) => {
                self.observed.add(Feature::Symbols);
                Ok(Expr::SymLit {
                    name: name.clone(),
                    span: self.span_at(*line, *column),
                })
            }
            TExpr::StrLit(StrLit { value, line, column }) => {
                self.observed.add(Feature::Strings);
                Ok(Expr::StrLit {
                    value: value.clone(),
                    span: self.span_at(*line, *column),
                })
            }
            TExpr::VarRef(VarRef { name, line, column }) => {
                let span = self.span_at(*line, *column);
                self.resolve_varref(name, ctx, &span)
            }
            TExpr::If(TIf { cond, then_branch, else_branch, line, column, .. }) => {
                let span = self.span_at(*line, *column);
                let lowered_cond = self.lower_expr(cond, ctx)?;
                let then_b = self.lower_branch(then_branch, ctx)?;
                let else_b = self.lower_branch(else_branch, ctx)?;
                Ok(Expr::If {
                    cond: Box::new(lowered_cond),
                    then_branch: Box::new(then_b),
                    else_branch: Box::new(else_b),
                    span,
                })
            }
            TExpr::Let(TLet { bindings, body, line, column }) => {
                self.lower_let(bindings, body, ctx, *line, *column, false)
            }
            TExpr::LetStar(TLetStar { bindings, body, line, column }) => {
                self.lower_let(bindings, body, ctx, *line, *column, true)
            }
            TExpr::Begin(Begin { exprs, line, column }) => {
                self.lower_begin(exprs, ctx, *line, *column)
            }
            TExpr::Lambda(lam) => self.lower_inline_lambda(lam, ctx),
            TExpr::Apply(Apply { fn_expr, args, line, column }) => {
                self.lower_apply(fn_expr, args, ctx, *line, *column)
            }
            TExpr::Match(m) => Err(TwigLowerError {
                message: "match expressions not supported in SIR v0".into(),
                line: m.line,
                column: m.column,
            }),
        }
    }

    fn lower_branch(&mut self, e: &TExpr, ctx: &mut FunctionCtx) -> Result<Block, TwigLowerError> {
        let lowered = self.lower_expr(e, ctx)?;
        let span = lowered.span().clone();
        Ok(Block {
            stmts: vec![],
            value: lowered,
            span,
        })
    }

    fn lower_let(
        &mut self,
        bindings: &[(String, TExpr)],
        body: &[TExpr],
        ctx: &mut FunctionCtx,
        line: usize,
        column: usize,
        sequential: bool,
    ) -> Result<Expr, TwigLowerError> {
        if body.is_empty() {
            return Err(TwigLowerError {
                message: format!(
                    "{} body is empty",
                    if sequential { "let*" } else { "let" }
                ),
                line,
                column,
            });
        }
        let span = self.span_at(line, column);

        // Lower the RHS expressions.  For `let`, all RHS use the
        // outer scope (no bindings added yet) — we lower them first.
        // For `let*`, each RHS uses prior bindings in turn.
        let mut stmts: Vec<Stmt> = Vec::with_capacity(bindings.len() + body.len() - 1);
        if sequential {
            for (name, rhs) in bindings {
                let v = self.lower_expr(rhs, ctx)?;
                let s = v.span().clone();
                stmts.push(Stmt::LetStarBinding {
                    name: name.clone(),
                    sir_type: None,
                    value: v,
                    span: s,
                });
                ctx.push_local(name.clone());
            }
        } else {
            // Parallel: lower all RHS in outer scope first.
            let mut lowered_pairs: Vec<(String, Expr)> = Vec::with_capacity(bindings.len());
            for (name, rhs) in bindings {
                let v = self.lower_expr(rhs, ctx)?;
                lowered_pairs.push((name.clone(), v));
            }
            for (name, v) in lowered_pairs {
                let s = v.span().clone();
                stmts.push(Stmt::LetBinding {
                    name: name.clone(),
                    sir_type: None,
                    value: v,
                    span: s,
                });
                ctx.push_local(name);
            }
        }

        // Lower the body in the augmented scope.
        for e in &body[..body.len() - 1] {
            let lowered = self.lower_expr(e, ctx)?;
            let s = lowered.span().clone();
            stmts.push(Stmt::ExprStmt { expr: lowered, span: s });
        }
        let value = self.lower_expr(body.last().unwrap(), ctx)?;

        // Pop the locals we added.
        for _ in 0..bindings.len() {
            ctx.pop_local();
        }

        Ok(Expr::Block(Box::new(Block { stmts, value, span })))
    }

    fn lower_begin(
        &mut self,
        exprs: &[TExpr],
        ctx: &mut FunctionCtx,
        line: usize,
        column: usize,
    ) -> Result<Expr, TwigLowerError> {
        if exprs.is_empty() {
            return Err(TwigLowerError {
                message: "begin body is empty".into(),
                line,
                column,
            });
        }
        let span = self.span_at(line, column);
        let mut stmts = Vec::with_capacity(exprs.len() - 1);
        for e in &exprs[..exprs.len() - 1] {
            let lowered = self.lower_expr(e, ctx)?;
            let s = lowered.span().clone();
            stmts.push(Stmt::ExprStmt { expr: lowered, span: s });
        }
        let value = self.lower_expr(exprs.last().unwrap(), ctx)?;
        Ok(Expr::Block(Box::new(Block { stmts, value, span })))
    }

    fn lower_inline_lambda(
        &mut self,
        lam: &TLambda,
        ctx: &mut FunctionCtx,
    ) -> Result<Expr, TwigLowerError> {
        self.observed.add(Feature::Closures);
        let span = self.span_at(lam.line, lam.column);
        // 1. Free-variable analysis.
        let bound_in_lambda: HashSet<String> = lam.params.iter().cloned().collect();
        let mut free = Vec::<String>::new();
        let mut seen = HashSet::<String>::new();
        for e in &lam.body {
            collect_free(e, &bound_in_lambda, &mut free, &mut seen);
        }
        // Order captures deterministically — alphabetical on names —
        // to make output reproducible.
        free.sort();

        // 2. Filter: captures don't include names that resolve to
        //    Global / Function / Builtin in the enclosing scope.
        //    Those are reachable directly from the inner function
        //    body without needing capture.
        let mut captures: Vec<String> = Vec::new();
        let mut capture_values: Vec<Expr> = Vec::new();
        for name in free {
            if ctx.locals.iter().any(|n| n == &name)
                || ctx.params.contains(&name)
                || ctx.captures.contains(&name)
            {
                let v = self.resolve_varref(&name, ctx, &span)?;
                captures.push(name.clone());
                capture_values.push(v);
            } else {
                // Global / function / builtin — no capture needed.
            }
        }

        // 3. Synthesise the closure-body function.
        let fn_name = self.fresh_lambda_name();
        let mut inner_ctx = FunctionCtx::new(
            lam.params.clone(),
            captures.iter().cloned().collect(),
            self.global_names.clone(),
            self.function_names.clone(),
        );
        let body = self.lower_lambda_body(&lam.body, &mut inner_ctx, lam.line, lam.column)?;
        let f = Function {
            name: fn_name.clone(),
            params: lam.params.iter().map(|p| Param {
                name: p.clone(),
                sir_type: None,
                span: span.clone(),
            }).collect(),
            return_type: None,
            captures: captures.iter().map(|n| semantic_ir::Capture {
                name: n.clone(),
                sir_type: None,
            }).collect(),
            body,
            effects: EffectSet::PURE.with(Effect::MayAllocate),
            metadata: Metadata::new(),
            span: span.clone(),
        };
        self.synthesised.push(f);

        // 4. Emit MakeClosure at the source position.
        Ok(Expr::MakeClosure {
            fn_name,
            captures: captures
                .into_iter()
                .zip(capture_values)
                .map(|(name, value)| CaptureValue { name, value })
                .collect(),
            span,
        })
    }

    fn lower_apply(
        &mut self,
        fn_expr: &TExpr,
        args: &[TExpr],
        ctx: &mut FunctionCtx,
        line: usize,
        column: usize,
    ) -> Result<Expr, TwigLowerError> {
        let span = self.span_at(line, column);

        // Lower args eagerly.
        let mut lowered_args: Vec<Expr> = Vec::with_capacity(args.len());
        for a in args {
            lowered_args.push(self.lower_expr(a, ctx)?);
        }

        // Three cases for the function position:
        if let TExpr::VarRef(VarRef { name, .. }) = fn_expr {
            if builtins::is_builtin(name) {
                return Ok(Expr::BuiltinCall {
                    name: name.clone(),
                    args: lowered_args,
                    effects: builtins::effects_for(name),
                    span,
                });
            }
            if self.function_names.contains(name) {
                return Ok(Expr::DirectCall {
                    fn_name: name.clone(),
                    args: lowered_args,
                    effects: EffectSet::PURE,
                    span,
                });
            }
            // Otherwise the name must resolve to a *value* (local /
            // param / capture / global) — emit IndirectCall on that
            // value.
            self.observed.add(Feature::Closures);
            let target = self.resolve_varref(name, ctx, &span)?;
            return Ok(Expr::IndirectCall {
                target: Box::new(target),
                args: lowered_args,
                effects: EffectSet::PURE,
                span,
            });
        }

        // The function position is an arbitrary expression.  Lower
        // it and emit IndirectCall.
        self.observed.add(Feature::Closures);
        let target = self.lower_expr(fn_expr, ctx)?;
        Ok(Expr::IndirectCall {
            target: Box::new(target),
            args: lowered_args,
            effects: EffectSet::PURE,
            span,
        })
    }

    // -------------------------------------------------------------------
    // Name resolution
    // -------------------------------------------------------------------

    fn resolve_varref(
        &mut self,
        name: &str,
        ctx: &FunctionCtx,
        span: &Span,
    ) -> Result<Expr, TwigLowerError> {
        // Local (innermost-first).  We iterate in reverse so inner
        // shadowing wins.
        if ctx.locals.iter().rev().any(|n| n == name) {
            return Ok(Expr::VarRef {
                name: name.to_string(),
                scope: Scope::Local,
                span: span.clone(),
            });
        }
        // Param.
        if ctx.params.contains(name) {
            return Ok(Expr::VarRef {
                name: name.to_string(),
                scope: Scope::Param,
                span: span.clone(),
            });
        }
        // Capture.
        if ctx.captures.contains(name) {
            return Ok(Expr::VarRef {
                name: name.to_string(),
                scope: Scope::Capture,
                span: span.clone(),
            });
        }
        // Global (value-define or top-level function name).
        if ctx.global_names.contains(name) || ctx.function_names.contains(name) {
            return Ok(Expr::VarRef {
                name: name.to_string(),
                scope: Scope::Global,
                span: span.clone(),
            });
        }
        // Builtin.
        if builtins::is_builtin(name) {
            return Ok(Expr::VarRef {
                name: name.to_string(),
                scope: Scope::Builtin,
                span: span.clone(),
            });
        }
        Err(TwigLowerError {
            message: format!("unresolved name `{}`", name),
            line: span.start_line,
            column: span.start_col,
        })
    }

    // -------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------

    fn span_at(&self, line: usize, column: usize) -> Span {
        Span::point(FILE, line, column)
    }
}

// ---------------------------------------------------------------------------
// FunctionCtx
// ---------------------------------------------------------------------------

/// Per-function lowering context — what is in scope right now for
/// name resolution.
struct FunctionCtx {
    params: HashSet<String>,
    captures: HashSet<String>,
    /// LIFO stack of let / let*-bound names.
    locals: Vec<String>,
    /// Snapshot of module-level globals (for resolution).
    global_names: HashSet<String>,
    /// Snapshot of module-level function names (for resolution).
    function_names: HashSet<String>,
}

impl FunctionCtx {
    fn new(
        params: Vec<String>,
        captures: HashSet<String>,
        global_names: HashSet<String>,
        function_names: HashSet<String>,
    ) -> Self {
        Self {
            params: params.into_iter().collect(),
            captures,
            locals: Vec::new(),
            global_names,
            function_names,
        }
    }

    /// Context for `_init` / `main` and other top-level synthesised
    /// bodies — no params, no captures, but the module's global and
    /// function tables are visible for resolution.
    fn for_top_level(
        global_names: HashSet<String>,
        function_names: HashSet<String>,
    ) -> Self {
        Self::new(vec![], HashSet::new(), global_names, function_names)
    }

    fn push_local(&mut self, name: String) {
        self.locals.push(name);
    }

    fn pop_local(&mut self) {
        self.locals.pop();
    }
}

// ---------------------------------------------------------------------------
// Free-variable analysis
// ---------------------------------------------------------------------------

/// Walk `expr` collecting every `VarRef` name that is *not* in
/// `bound`.  Results are appended to `free` (preserving first-seen
/// order); `seen` deduplicates.
fn collect_free(
    expr: &TExpr,
    bound: &HashSet<String>,
    free: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    match expr {
        TExpr::IntLit(_) | TExpr::BoolLit(_) | TExpr::NilLit(_) | TExpr::SymLit(_) | TExpr::StrLit(_) => {}
        TExpr::VarRef(VarRef { name, .. }) => {
            if !bound.contains(name) && seen.insert(name.clone()) {
                free.push(name.clone());
            }
        }
        TExpr::If(TIf { cond, then_branch, else_branch, .. }) => {
            collect_free(cond, bound, free, seen);
            collect_free(then_branch, bound, free, seen);
            collect_free(else_branch, bound, free, seen);
        }
        TExpr::Let(TLet { bindings, body, .. }) => {
            // Parallel let: every RHS sees `bound` (no new names);
            // the body sees `bound ∪ binding names`.
            for (_, rhs) in bindings {
                collect_free(rhs, bound, free, seen);
            }
            let mut inner = bound.clone();
            for (n, _) in bindings {
                inner.insert(n.clone());
            }
            for e in body {
                collect_free(e, &inner, free, seen);
            }
        }
        TExpr::LetStar(TLetStar { bindings, body, .. }) => {
            let mut inner = bound.clone();
            for (n, rhs) in bindings {
                collect_free(rhs, &inner, free, seen);
                inner.insert(n.clone());
            }
            for e in body {
                collect_free(e, &inner, free, seen);
            }
        }
        TExpr::Begin(Begin { exprs, .. }) => {
            for e in exprs {
                collect_free(e, bound, free, seen);
            }
        }
        TExpr::Lambda(lam) => {
            // Lambda binds its own params in the body.  Outer-free
            // names are anything else.
            let mut inner = bound.clone();
            for p in &lam.params {
                inner.insert(p.clone());
            }
            for e in &lam.body {
                collect_free(e, &inner, free, seen);
            }
        }
        TExpr::Apply(Apply { fn_expr, args, .. }) => {
            collect_free(fn_expr, bound, free, seen);
            for a in args {
                collect_free(a, bound, free, seen);
            }
        }
        TExpr::Match(_) => {
            // Match is rejected at lowering; treat as no-op for
            // free-var purposes.
        }
    }
}

// ---------------------------------------------------------------------------
// Manifest helpers
// ---------------------------------------------------------------------------

/// `true` iff the function list contains at least two user-defined
/// functions (we exclude `_init` and `main` from the count — they
/// are always synthesised).
fn user_fns_with_at_least_two_user_defines(fns: &[Function]) -> bool {
    fns.iter()
        .filter(|f| f.name != "_init" && f.name != "main")
        .count()
        >= 2
}
