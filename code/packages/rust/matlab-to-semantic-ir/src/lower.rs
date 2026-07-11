//! The lowering pass from `coding_adventures_matlab_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **v0.1.0**.
//!
//! # Scope
//!
//! MATLAB is large; this first cut covers a well-defined, testable subset
//! and returns a clean [`MatlabLowerError`] for anything outside it, rather
//! than silently mis-lowering:
//!
//! **Supported:**
//! - Literals: `NUMBER` (int- or float-shaped by lexeme), `STRING`, matrix
//!   literals `[1 2; 3 4]` → [`Expr::ArrayLit`].
//! - Variables (`NAME`), assignment (`x = expr`; first occurrence →
//!   `LetStarBinding`, later re-assignment → `Assign`, mirroring every
//!   other SIR frontend).
//! - Arithmetic: `+`/`-` always lower to [`Expr::ElementwiseOp`] (MATLAB has
//!   no non-elementwise reading of these, per the SIR22 spec) *unless both
//!   operands are provably scalar* (see "Scalar/array disambiguation"
//!   below), in which case a plain `BuiltinCall` is emitted instead — both
//!   forms are semantically identical for scalars, so this is purely an
//!   optimisation that also happens to let scalar-only MATLAB programs
//!   round-trip through backends that do not yet implement SIR22 codegen.
//!   `.* ./ .\ .^` always lower to `ElementwiseOp` (unambiguous). Bare `*`
//!   disambiguates to [`Expr::MatMul`] vs. `ElementwiseOp` per the same
//!   scalar rule; bare `/`/`\` (mrdivide/mldivide — matrix division) are
//!   **unsupported** outside the scalar case, since `array-runtime` has no
//!   linear-solve kernel to map onto yet.
//! - Comparisons (`== ~= < > <= >=`), logical `&& || & &` (short-circuit
//!   and elementwise forms are **not** distinguished — both lower to the
//!   same `LogicalAnd`/`LogicalOr`, a disclosed simplification), unary
//!   `+ - ~`.
//! - Ranges `a:b` (as a value, and specialised for `for i = a:b`) →
//!   [`Expr::Range`].
//! - Transpose `'`/`.'` → [`Expr::Transpose`].
//! - Indexing `A(i, j, ...)` (read → [`Expr::IndexGet`], write →
//!   [`Stmt::IndexSet`]) with 1-based → 0-based translation at lowering
//!   time (SIR10's "disambiguation is the frontend's job"); `:` →
//!   [`IndexArg::Whole`].
//! - Control flow `if`/`elseif`/`else`, `while`.
//! - Function definitions `function [out =] name(params) ... end` (single
//!   or zero output only) and calls to them ([`Expr::DirectCall`]).
//! - `disp(x)` — the one recognised builtin, mapped onto the SIR `print`
//!   builtin every backend already implements (needed so a lowered
//!   program can produce any observable output at all). Every other
//!   identifier that is neither a known variable nor a known user
//!   function is rejected rather than guessed at.
//!
//! **Deliberately out of scope for v0.1.0** (each rejected with an explicit
//! [`MatlabLowerError`], not silently mis-lowered):
//! - Stepped (`a:step:b`) or matrix-valued `for` loops — only
//!   `for i = a:b` is supported; the exclusive-stop conversion (`b + 1`) is
//!   exact only for a unit step.
//! - `end`-relative indexing (`A(end)`, `A(end-1)`) — no `size`/`length`
//!   builtin is wired up yet to resolve it.
//! - Matrix power (`A^2` with a non-scalar base), matrix division `/`/`\`
//!   between non-scalars (mrdivide/mldivide — no backend kernel exists).
//! - Multi-output functions (`[a, b] = f(...)`), nested function
//!   definitions, `break`/`continue`/`return` (semantic-ir has no
//!   early-exit control-flow node at all yet — this is a whole-IR gap, not
//!   specific to this frontend), `switch`/`try`/`global`/`persistent`,
//!   cell arrays, anonymous functions (`@(...)  ...`), auto-vivification
//!   on indexed assignment to an undeclared variable, and chained
//!   assignment (`a = b = c`).
//!
//! # Scalar/array disambiguation
//!
//! MATLAB's `+ - * / \ ^` are polymorphic between scalar and matrix
//! operands at *runtime*; this frontend has no shape/type inference, so it
//! uses a conservative, purely syntactic heuristic on the **already-lowered**
//! operand [`Expr`]s (see [`expr_is_known_scalar`]): an operand is "known
//! scalar" iff it is a bare `IntLit`/`FloatLit`, or a `BuiltinCall` of
//! `+ - * / neg` whose own arguments are (transitively) known-scalar. This
//! correctly folds chains like `1 + 2 * 3` but does not attempt full
//! constant evaluation (e.g. it will not recognise `(2 + 3)` inside a
//! larger *variable* expression as scalar) — falling through to the
//! array-domain node in an ambiguous case is always semantically safe
//! (either correct, since `array_runtime`'s elementwise kernel already
//! broadcasts a genuine runtime scalar against anything, or an honest
//! "unsupported" error), just occasionally more conservative than a full
//! type-inference pass would be.

use std::collections::HashSet;

use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, ElementwiseOpKind, Expr, Feature, FeatureManifest, Function, IndexArg,
    Metadata, Module, Param, ParamKind, Scope, Span, Stmt,
};

/// Maximum expression-nesting depth. Mirrors every other SIR frontend's
/// identically-named, identically-justified guard: turns pathologically
/// deep (but parseable) input into a clean [`MatlabLowerError`] instead of
/// a native (uncatchable) stack overflow.
const MAX_EXPR_DEPTH: usize = 256;

/// Maximum statement-block nesting depth (each `if`/`while`/`for` body, or
/// a `function` body, re-enters the block lowerer one level deeper).
const MAX_BLOCK_DEPTH: usize = 256;

/// Synthetic file name used for all spans (the CST does not carry the
/// original path).
const FILE: &str = "<matlab>";

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// An error encountered during MATLAB → SIR lowering.
///
/// Mirrors `PythonLowerError`/`TwigLowerError`'s shape exactly (`message` +
/// 1-based `line`/`column`) so tooling can treat every SIR frontend
/// uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatlabLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for MatlabLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MatlabLowerError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for MatlabLowerError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed MATLAB CST (rooted at the `program` rule) into a SIR
/// module.
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, MatlabLowerError> {
    Lowerer::new(module_name).lower_file(tree)
}

// ---------------------------------------------------------------------------
// The lowerer
// ---------------------------------------------------------------------------

/// One lowered top-level / body statement: either a `Stmt` or a bare
/// expression (an expression statement).
enum Lowered {
    Stmt(Box<Stmt>),
    Expr(Expr),
}

/// Per-function name-resolution context. MATLAB scopes a variable to its
/// *whole enclosing function* (no block scoping), so — unlike a
/// closure-supporting frontend — there is no capture set here at all;
/// `locals` simply accumulates for the function's lifetime and is never
/// rewound when leaving an `if`/`while`/`for` body (that would be wrong
/// for MATLAB: a variable first assigned inside an `if` remains visible
/// afterward). The one place `locals` *is* temporarily extended and then
/// rewound is a `for`-loop variable, whose scope genuinely is the loop
/// (mirroring every other SIR frontend's identical loop-variable handling).
struct FunctionCtx {
    params: HashSet<String>,
    locals: Vec<String>,
}

impl FunctionCtx {
    fn new(params: HashSet<String>) -> Self {
        Self {
            params,
            locals: Vec::new(),
        }
    }

    fn top_level() -> Self {
        Self::new(HashSet::new())
    }
}

struct Lowerer {
    module_name: String,
    /// Features observed while lowering, used to build the manifest so it
    /// declares *exactly* what the module emits.
    observed: FeatureManifest,
    /// Every top-level `function` name, collected in a first pass so a call
    /// to a function defined later in the file resolves as
    /// [`Expr::DirectCall`] regardless of textual order.
    function_names: HashSet<String>,
    /// The lowered top-level functions, in definition order. `main` is
    /// appended last by [`Self::lower_file`].
    functions: Vec<Function>,
}

impl Lowerer {
    fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            observed: FeatureManifest::new(),
            function_names: HashSet::new(),
            functions: Vec::new(),
        }
    }

    // -------------------------------------------------------------------
    // for-loop variable scope: mark/rewind (the ONE place MATLAB truly
    // does introduce a scope narrower than the whole function).
    // -------------------------------------------------------------------

    fn scope_mark(ctx: &FunctionCtx) -> usize {
        ctx.locals.len()
    }

    fn scope_rewind(ctx: &mut FunctionCtx, mark: usize) {
        ctx.locals.truncate(mark);
    }

    // -------------------------------------------------------------------
    // top level: `program` → collect function names, then lower
    // -------------------------------------------------------------------

    fn lower_file(&mut self, program: &GrammarASTNode) -> Result<Module, MatlabLowerError> {
        if program.rule_name != "program" {
            return Err(self.err_at(
                program,
                format!("expected `program` root, got `{}`", program.rule_name),
            ));
        }

        // Every value this frontend lowers has `sir_type: None` -- MATLAB
        // has no static type declarations anywhere -- which is itself what
        // the validator's ground truth treats as "using" dynamic typing
        // (see `semantic-ir/src/validator.rs`'s own comment to that
        // effect), so this is unconditionally true for every module.
        self.observed.add(Feature::DynamicTyping);

        self.collect_function_names(program)?;

        let mut ctx = FunctionCtx::top_level();
        let mut items: Vec<Lowered> = Vec::new();
        for stmt_line in child_nodes(program) {
            if stmt_line.rule_name != "statement_line" {
                continue;
            }
            let stmt = match self.first_child_named(stmt_line, "statement") {
                Some(s) => s,
                None => continue, // a bare terminator (blank line) -- nothing to lower
            };
            let kids = child_nodes(stmt);
            let inner = match kids.as_slice() {
                [only] => *only,
                _ => return Err(self.err_at(stmt, "malformed statement".to_string())),
            };
            if inner.rule_name == "func_def" {
                let f = self.lower_func_def(inner)?;
                self.functions.push(f);
                continue;
            }
            if let Some(item) = self.lower_statement_body_item(inner, &mut ctx, 0)? {
                items.push(item);
            }
        }

        let span = Span::point(FILE, 1, 1);
        let main_body =
            assemble_stmts_only(items, Expr::NilLit { span: span.clone() }, span.clone());
        let main = Function {
            name: "main".to_string(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: main_body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: span.clone(),
        };

        let mut functions = std::mem::take(&mut self.functions);
        functions.push(main);

        let metadata = Metadata::new()
            .with_source_language("matlab")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION);

        Ok(Module {
            name: self.module_name.clone(),
            manifest: self.observed.clone(),
            imports: vec![],
            exports: vec![],
            functions,
            globals: vec![],
            metadata,
            span,
        })
    }

    /// Pass 1: collect every top-level `function`'s name, so a call
    /// anywhere in the file — regardless of textual order — resolves as
    /// [`Expr::DirectCall`]. Nested function definitions are rejected (as
    /// an explicit error) when actually lowered, not here.
    fn collect_function_names(&mut self, program: &GrammarASTNode) -> Result<(), MatlabLowerError> {
        for stmt_line in child_nodes(program) {
            if stmt_line.rule_name != "statement_line" {
                continue;
            }
            let stmt = match self.first_child_named(stmt_line, "statement") {
                Some(s) => s,
                None => continue,
            };
            let kids = child_nodes(stmt);
            let inner = match kids.as_slice() {
                [only] => *only,
                _ => continue,
            };
            if inner.rule_name == "func_def" {
                let name = self.func_def_name(inner)?;
                self.function_names.insert(name);
            }
        }
        Ok(())
    }

    // -------------------------------------------------------------------
    // function definitions
    // -------------------------------------------------------------------

    /// The function's own name: a bare `NAME` token directly under
    /// `func_def` (distinct from the *output variable*'s name, which lives
    /// one level deeper inside `func_returns` and is therefore invisible to
    /// this direct scan).
    fn func_def_name(&self, def: &GrammarASTNode) -> Result<String, MatlabLowerError> {
        def.children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name => Some(t.value.clone()),
                _ => None,
            })
            .ok_or_else(|| self.err_at(def, "malformed function definition: no name".to_string()))
    }

    /// Lower a `func_def` into a top-level [`Function`]. MATLAB functions
    /// have no `return`-expression; the designated output variable's final
    /// value *is* the return value, so the body's trailing [`Block::value`]
    /// is synthesised as a `VarRef` to it (or `NilLit` for a function with
    /// no output).
    fn lower_func_def(&mut self, def: &GrammarASTNode) -> Result<Function, MatlabLowerError> {
        let span = self.span_of(def);
        let name = self.func_def_name(def)?;

        let mut output: Option<String> = None;
        if let Some(returns) = self.first_child_named(def, "func_returns") {
            if !child_nodes(returns).is_empty() {
                // `LBRACKET [name_list] RBRACKET EQ` shape -- multi-output.
                return Err(self.err_at(
                    returns,
                    "unsupported: multiple output arguments (`[a, b] = f(...)`) are out of scope for v0.1.0"
                        .to_string(),
                ));
            }
            let out_name = returns
                .children
                .iter()
                .find_map(|c| match c {
                    ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name => Some(t.value.clone()),
                    _ => None,
                })
                .ok_or_else(|| {
                    self.err_at(returns, "malformed function return clause".to_string())
                })?;
            output = Some(out_name);
        }

        let mut param_names: Vec<String> = Vec::new();
        if let Some(name_list) = self.first_child_named(def, "name_list") {
            param_names.extend(name_list.children.iter().filter_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name => Some(t.value.clone()),
                _ => None,
            }));
        }

        let body_node = self
            .first_child_named(def, "block_body")
            .ok_or_else(|| self.err_at(def, "malformed function: no body".to_string()))?;

        let mut ctx = FunctionCtx::new(param_names.iter().cloned().collect());
        let items = self.lower_body_items(body_node, &mut ctx, 0)?;

        let value = match &output {
            Some(out_name) => Expr::VarRef {
                name: out_name.clone(),
                scope: Scope::Local,
                span: span.clone(),
            },
            None => Expr::NilLit { span: span.clone() },
        };
        let body = assemble_stmts_only(items, value, span.clone());

        Ok(Function {
            name,
            params: param_names
                .into_iter()
                .map(|p| Param {
                    name: p,
                    sir_type: None,
                    kind: ParamKind::Required,
                    default: None,
                    span: span.clone(),
                })
                .collect(),
            return_type: None,
            captures: vec![],
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span,
        })
    }

    // -------------------------------------------------------------------
    // statement bodies (shared by `if`/`while`/`for`/`function` bodies)
    // -------------------------------------------------------------------

    fn lower_body_items(
        &mut self,
        body_node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Vec<Lowered>, MatlabLowerError> {
        let mut items = Vec::new();
        for stmt_line in child_nodes(body_node) {
            if stmt_line.rule_name == "statement_line" {
                if let Some(item) = self.lower_statement_line(stmt_line, ctx, depth)? {
                    items.push(item);
                }
            }
        }
        Ok(items)
    }

    fn lower_block_body(
        &mut self,
        block_body: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Block, MatlabLowerError> {
        if depth > MAX_BLOCK_DEPTH {
            return Err(self.err_at(
                block_body,
                format!("control-flow nesting too deep (exceeds {MAX_BLOCK_DEPTH} levels)"),
            ));
        }
        let items = self.lower_body_items(block_body, ctx, depth)?;
        let span = self.span_of(block_body);
        Ok(assemble_stmts_only(
            items,
            Expr::NilLit { span: span.clone() },
            span,
        ))
    }

    fn lower_statement_line(
        &mut self,
        stmt_line: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Lowered>, MatlabLowerError> {
        let stmt = match self.first_child_named(stmt_line, "statement") {
            Some(s) => s,
            None => return Ok(None), // a bare terminator (blank line)
        };
        let kids = child_nodes(stmt);
        let inner = match kids.as_slice() {
            [only] => *only,
            _ => return Err(self.err_at(stmt, "malformed statement".to_string())),
        };
        self.lower_statement_body_item(inner, ctx, depth)
    }

    /// Dispatch one `statement` alternative that is *not* a top-level
    /// `func_def` (that case is handled directly by [`Self::lower_file`]).
    /// Reached here, `func_def` can only mean a *nested* definition, which
    /// this frontend does not support.
    fn lower_statement_body_item(
        &mut self,
        inner: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Lowered>, MatlabLowerError> {
        if depth > MAX_BLOCK_DEPTH {
            return Err(self.err_at(
                inner,
                format!("control-flow nesting too deep (exceeds {MAX_BLOCK_DEPTH} levels)"),
            ));
        }
        match inner.rule_name.as_str() {
            "func_def" => Err(self.err_at(
                inner,
                "unsupported: nested function definitions are out of scope for v0.1.0".to_string(),
            )),
            "if_stmt" => Ok(Some(Lowered::Expr(self.lower_if(inner, ctx, depth)?))),
            "while_stmt" => Ok(Some(Lowered::Stmt(Box::new(
                self.lower_while(inner, ctx, depth)?,
            )))),
            "for_stmt" => Ok(Some(Lowered::Stmt(Box::new(
                self.lower_for(inner, ctx, depth)?,
            )))),
            "switch_stmt" => Err(self.err_at(
                inner,
                "unsupported: `switch` is out of scope for v0.1.0".to_string(),
            )),
            "try_stmt" => Err(self.err_at(
                inner,
                "unsupported: `try`/`catch` is out of scope for v0.1.0".to_string(),
            )),
            "break_stmt" => Err(self.err_at(
                inner,
                "unsupported: `break` has no SIR equivalent yet".to_string(),
            )),
            "continue_stmt" => Err(self.err_at(
                inner,
                "unsupported: `continue` has no SIR equivalent yet".to_string(),
            )),
            "return_stmt" => Err(self.err_at(
                inner,
                "unsupported: early `return` has no SIR equivalent yet (a function's output \
                 variable at its final statement is the only supported return mechanism)"
                    .to_string(),
            )),
            "global_stmt" => Err(self.err_at(
                inner,
                "unsupported: `global`/`persistent` are out of scope for v0.1.0".to_string(),
            )),
            _ => self.lower_statement_expr(inner, ctx, depth).map(Some),
        }
    }

    // -------------------------------------------------------------------
    // control flow
    // -------------------------------------------------------------------

    fn lower_if(
        &mut self,
        if_stmt: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Expr, MatlabLowerError> {
        struct Clause<'a> {
            cond: &'a GrammarASTNode,
            body: &'a GrammarASTNode,
        }
        let mut clauses: Vec<Clause> = Vec::new();
        let mut else_body: Option<&GrammarASTNode> = None;

        let kids = child_nodes(if_stmt);
        let mut it = kids.into_iter();
        let cond0 = it
            .next()
            .ok_or_else(|| self.err_at(if_stmt, "malformed if: no condition".to_string()))?;
        let body0 = it
            .next()
            .ok_or_else(|| self.err_at(if_stmt, "malformed if: no body".to_string()))?;
        clauses.push(Clause {
            cond: cond0,
            body: body0,
        });
        for rest in it {
            match rest.rule_name.as_str() {
                "elseif_clause" => match child_nodes(rest).as_slice() {
                    [c, b] => clauses.push(Clause { cond: c, body: b }),
                    _ => return Err(self.err_at(rest, "malformed elseif clause".to_string())),
                },
                "else_clause" => match child_nodes(rest).as_slice() {
                    [b] => else_body = Some(b),
                    _ => return Err(self.err_at(rest, "malformed else clause".to_string())),
                },
                other => {
                    return Err(self.err_at(
                        rest,
                        format!("unexpected `{other}` inside if statement"),
                    ))
                }
            }
        }

        let if_span = self.span_of(if_stmt);
        let mut else_branch: Block = match else_body {
            Some(b) => self.lower_block_body(b, ctx, depth + 1)?,
            None => empty_block(if_span.clone()),
        };
        for clause in clauses.into_iter().rev() {
            let cond = self.lower_expr(clause.cond, ctx)?;
            let then_branch = self.lower_block_body(clause.body, ctx, depth + 1)?;
            let span = cond.span().clone();
            let folded = Expr::If {
                cond: Box::new(cond),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
                span,
            };
            else_branch = value_block(folded);
        }
        match else_branch.value {
            Expr::If { .. } => Ok(else_branch.value),
            other => Ok(other),
        }
    }

    fn lower_while(
        &mut self,
        while_stmt: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Stmt, MatlabLowerError> {
        let (cond_node, body_node) = match child_nodes(while_stmt).as_slice() {
            [c, b] => (*c, *b),
            _ => {
                return Err(self.err_at(
                    while_stmt,
                    "malformed while: expected condition and body".to_string(),
                ))
            }
        };
        let cond = self.lower_expr(cond_node, ctx)?;
        let body = self.lower_block_body(body_node, ctx, depth + 1)?;
        self.observed.add(Feature::Loops);
        Ok(Stmt::While {
            cond,
            body,
            span: self.span_of(while_stmt),
        })
    }

    /// Lower `for NAME = a:b ... end` into [`Stmt::ForRange`]. Only the
    /// unit-step, two-operand range form is supported (see the module doc
    /// comment's scope note); `ForRange` is half-open, but MATLAB's `a:b`
    /// is inclusive, so the exclusive bound is `b + 1` — exact for any
    /// `a`/`b` (not just integers) precisely because the step is fixed at 1.
    fn lower_for(
        &mut self,
        for_stmt: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Stmt, MatlabLowerError> {
        let span = self.span_of(for_stmt);
        let var = for_stmt
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if t.type_ == TokenType::Name => Some(t.value.clone()),
                _ => None,
            })
            .ok_or_else(|| self.err_at(for_stmt, "malformed for: no loop variable".to_string()))?;

        let (iter_node, body_node) = match child_nodes(for_stmt).as_slice() {
            [i, b] => (*i, *b),
            _ => {
                return Err(self.err_at(
                    for_stmt,
                    "malformed for: expected range and body".to_string(),
                ))
            }
        };

        let range_node = self.peel_to_named(iter_node, "colon_expr", 0);
        let (start_n, stop_n) = match range_node {
            Some(r) => match child_nodes(r).as_slice() {
                [s, e] => (*s, *e),
                _ => {
                    return Err(self.err_at(
                        r,
                        "unsupported: stepped for-loop ranges (`for i = a:step:b`) are out of \
                         scope for v0.1.0"
                            .to_string(),
                    ))
                }
            },
            None => {
                return Err(self.err_at(
                    iter_node,
                    "unsupported: `for` over a non-range expression is out of scope for v0.1.0 \
                     (only `for NAME = a:b` is supported)"
                        .to_string(),
                ))
            }
        };

        let start = self.lower_expr(start_n, ctx)?;
        let stop_val = self.lower_expr(stop_n, ctx)?;
        let stop_span = stop_val.span().clone();
        let stop = Expr::BuiltinCall {
            name: "+".to_string(),
            args: vec![
                stop_val,
                Expr::IntLit {
                    value: 1,
                    span: stop_span.clone(),
                },
            ],
            effects: EffectSet::PURE,
            span: stop_span,
        };
        let step = Expr::IntLit {
            value: 1,
            span: span.clone(),
        };

        self.observed.add(Feature::Loops);
        let mark = Self::scope_mark(ctx);
        ctx.locals.push(var.clone());
        let body = self.lower_block_body(body_node, ctx, depth + 1)?;
        Self::scope_rewind(ctx, mark);

        Ok(Stmt::ForRange {
            var,
            start,
            stop,
            step,
            body,
            span,
        })
    }

    // -------------------------------------------------------------------
    // assignment
    // -------------------------------------------------------------------

    /// Lower a `statement`'s `expr` alternative: either a value expression
    /// (a bare function call, e.g. `disp(x)`) or an assignment. MATLAB's
    /// own grammar folds assignment *into* the expression precedence chain
    /// (`expr = assignment`, `assignment = logical_or [ EQ assignment ]`),
    /// so this peels down to the `assignment` rule specifically (wherever
    /// it lands in the collapsed tree) rather than assuming a fixed depth.
    fn lower_statement_expr(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Lowered, MatlabLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        if node.rule_name != "assignment" {
            return match child_nodes(node).as_slice() {
                [only] if node.children.len() == 1 => {
                    self.lower_statement_expr(only, ctx, depth + 1)
                }
                _ => {
                    let expr = self.lower_expr(node, ctx)?;
                    Ok(Lowered::Expr(expr))
                }
            };
        }
        match child_nodes(node).as_slice() {
            [lhs] if node.children.len() == 1 => {
                let expr = self.lower_expr(lhs, ctx)?;
                Ok(Lowered::Expr(expr))
            }
            [lhs, rhs] => {
                if rhs.rule_name == "assignment" && child_nodes(rhs).len() == 2 {
                    return Err(self.err_at(
                        rhs,
                        "unsupported: chained assignment (`a = b = c`) is out of scope for v0.1.0"
                            .to_string(),
                    ));
                }
                self.lower_assignment(node, lhs, rhs, ctx)
            }
            _ => Err(self.err_at(node, "malformed assignment".to_string())),
        }
    }

    fn lower_assignment(
        &mut self,
        assign_node: &GrammarASTNode,
        lhs: &GrammarASTNode,
        rhs: &GrammarASTNode,
        ctx: &mut FunctionCtx,
    ) -> Result<Lowered, MatlabLowerError> {
        let span = self.span_of(assign_node);

        if let Some(name) = self.bare_name(lhs) {
            let value = self.lower_expr(rhs, ctx)?;
            if ctx.locals.contains(&name) || ctx.params.contains(&name) {
                self.observed.add(Feature::MutableBindings);
                return Ok(Lowered::Stmt(Box::new(Stmt::Assign {
                    name,
                    scope: Scope::Local,
                    value,
                    span,
                })));
            }
            ctx.locals.push(name.clone());
            return Ok(Lowered::Stmt(Box::new(Stmt::LetStarBinding {
                name,
                sir_type: None,
                value,
                span,
            })));
        }

        if let Some((base_name, call_suffix)) = self.indexed_target(lhs) {
            if !(ctx.locals.contains(&base_name) || ctx.params.contains(&base_name)) {
                return Err(self.err_at(
                    lhs,
                    format!(
                        "cannot index-assign into `{base_name}`: not previously assigned \
                         (auto-vivification is out of scope for v0.1.0)"
                    ),
                ));
            }
            // A fresh statement's own expression tree gets its own
            // `MAX_EXPR_DEPTH` budget (depth 0), matching the `rhs` lowering
            // just below and every other statement-boundary expression in
            // this file -- see `lower_index_args`'s doc comment for why
            // *nested* index/call positions instead thread the caller's
            // depth rather than restarting.
            let indices = self.lower_index_args(call_suffix, ctx, 0)?;
            let value = self.lower_expr(rhs, ctx)?;
            return Ok(Lowered::Stmt(Box::new(Stmt::IndexSet {
                target: Box::new(Expr::VarRef {
                    name: base_name,
                    scope: Scope::Local,
                    span: span.clone(),
                }),
                indices,
                value: Box::new(value),
                span,
            })));
        }

        Err(self.err_at(
            lhs,
            "unsupported: assignment target is not a bare name or a simple index expression"
                .to_string(),
        ))
    }

    // -------------------------------------------------------------------
    // expressions: precedence dispatch
    // -------------------------------------------------------------------

    fn lower_expr(&mut self, node: &GrammarASTNode, ctx: &mut FunctionCtx) -> Result<Expr, MatlabLowerError> {
        self.lower_expr_d(node, ctx, 0)
    }

    fn lower_expr_d(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Expr, MatlabLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        match node.rule_name.as_str() {
            "logical_or" | "bit_or" => {
                if let Some(e) = self.try_logical(node, ctx, depth, true)? {
                    return Ok(e);
                }
            }
            "logical_and" | "bit_and" => {
                if let Some(e) = self.try_logical(node, ctx, depth, false)? {
                    return Ok(e);
                }
            }
            "comparison" => {
                if let Some(e) = self.try_comparison(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "colon_expr" => {
                if let Some(e) = self.lower_colon(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "additive" => {
                if let Some(e) = self.try_additive(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "multiplicative" => {
                if let Some(e) = self.try_multiplicative(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "unary" => {
                if let Some(e) = self.lower_unary(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "power" => {
                if let Some(e) = self.try_power(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "postfix" => {
                if let Some(e) = self.lower_postfix(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "assignment" => {
                // The RHS of a real assignment is itself grammatically
                // labelled `assignment` (`assignment = logical_or [ EQ
                // assignment ]`) even when it carries no further `=` --
                // that's an ordinary wrapper to peel through, not a nested
                // assignment attempt. Only an *actual* `[ EQ assignment ]`
                // suffix reaching here (assignment used where a plain value
                // is expected, e.g. as a function argument) is an error.
                return match child_nodes(node).as_slice() {
                    [only] if node.children.len() == 1 => {
                        self.lower_expr_d(only, ctx, depth + 1)
                    }
                    _ => Err(self.err_at(
                        node,
                        "unsupported: assignment used as a value expression".to_string(),
                    )),
                };
            }
            _ => {}
        }
        match child_nodes(node).as_slice() {
            [only] if node.children.len() == 1 => self.lower_expr_d(only, ctx, depth + 1),
            _ => Err(self.err_at(
                node,
                format!("unsupported: `{}` (deferred)", node.rule_name),
            )),
        }
    }

    fn try_logical(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
        is_or: bool,
    ) -> Result<Option<Expr>, MatlabLowerError> {
        let ops: &[&str] = if is_or { &["||", "|"] } else { &["&&", "&"] };
        let has_op = node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if ops.contains(&t.value.as_str())));
        if !has_op {
            return Ok(None);
        }
        self.check_chain_length(node)?;
        let mut acc: Option<Expr> = None;
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                let operand = self.lower_expr_d(n, ctx, depth + 1)?;
                acc = Some(match acc.take() {
                    None => operand,
                    Some(lhs) => {
                        let span = lhs.span().clone();
                        if is_or {
                            Expr::LogicalOr {
                                lhs: Box::new(lhs),
                                rhs: Box::new(operand),
                                span,
                            }
                        } else {
                            Expr::LogicalAnd {
                                lhs: Box::new(lhs),
                                rhs: Box::new(operand),
                                span,
                            }
                        }
                    }
                });
            }
        }
        match acc {
            Some(e) => Ok(Some(e)),
            None => Err(self.err_at(node, "empty logical expression".to_string())),
        }
    }

    fn try_comparison(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, MatlabLowerError> {
        const OPS: &[&str] = &["==", "~=", "<=", ">=", "<", ">"];
        let has_op = node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if OPS.contains(&t.value.as_str())));
        if !has_op {
            return Ok(None);
        }
        self.check_chain_length(node)?;
        let mut acc: Option<Expr> = None;
        let mut pending: Option<String> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if OPS.contains(&t.value.as_str()) => {
                    pending = Some(match t.value.as_str() {
                        "==" => "=".to_string(),
                        "~=" => "!=".to_string(),
                        other => other.to_string(),
                    });
                }
                ASTNodeOrToken::Node(n) => {
                    let operand = self.lower_expr_d(n, ctx, depth + 1)?;
                    acc = Some(match (acc.take(), pending.take()) {
                        (None, _) => operand,
                        (Some(lhs), Some(op)) => {
                            let span = lhs.span().clone();
                            Expr::BuiltinCall {
                                name: op,
                                args: vec![lhs, operand],
                                effects: EffectSet::PURE,
                                span,
                            }
                        }
                        (Some(_), None) => {
                            return Err(self.err_at(node, "malformed comparison".to_string()))
                        }
                    });
                }
                ASTNodeOrToken::Token(_) => {}
            }
        }
        match acc {
            Some(e) => Ok(Some(e)),
            None => Err(self.err_at(node, "empty comparison".to_string())),
        }
    }

    fn lower_colon(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, MatlabLowerError> {
        if node.rule_name != "colon_expr" {
            return Ok(None);
        }
        let kids = child_nodes(node);
        let span = self.span_of(node);
        match kids.as_slice() {
            [only] => self.lower_expr_d(only, ctx, depth + 1).map(Some),
            [start, stop] => {
                // `Expr::Range` only requires `Feature::NDArrays` per the
                // validator's own ground truth (a bare range is not itself
                // a matrix op).
                self.observed.add(Feature::NDArrays);
                let start_e = self.lower_expr_d(start, ctx, depth + 1)?;
                let stop_e = self.lower_expr_d(stop, ctx, depth + 1)?;
                Ok(Some(Expr::Range {
                    start: Box::new(start_e),
                    step: None,
                    stop: Box::new(stop_e),
                    span,
                }))
            }
            [start, step, stop] => {
                self.observed.add(Feature::NDArrays);
                let start_e = self.lower_expr_d(start, ctx, depth + 1)?;
                let step_e = self.lower_expr_d(step, ctx, depth + 1)?;
                let stop_e = self.lower_expr_d(stop, ctx, depth + 1)?;
                Ok(Some(Expr::Range {
                    start: Box::new(start_e),
                    step: Some(Box::new(step_e)),
                    stop: Box::new(stop_e),
                    span,
                }))
            }
            _ => Err(self.err_at(node, "malformed range expression".to_string())),
        }
    }

    fn try_additive(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, MatlabLowerError> {
        let has_op = node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.value == "+" || t.value == "-"));
        if !has_op {
            return Ok(None);
        }
        self.check_chain_length(node)?;
        // `acc` tracks the accumulated `Expr` *and* whether it is itself
        // known-scalar, updated incrementally (O(1) per fold step) rather
        // than re-derived by calling `expr_is_known_scalar` on the
        // ever-growing `lhs` at every step -- that re-walk would cost
        // O(chain length) stack on the final step of an ordinary flat
        // `1 + 1 + ... + 1` chain (no parens, so `MAX_EXPR_DEPTH`'s
        // grammar-nesting guard never engages), an uncatchable stack
        // overflow on a long enough chain.
        let mut acc: Option<(Expr, bool)> = None;
        let mut pending: Option<String> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if t.value == "+" || t.value == "-" => {
                    pending = Some(t.value.clone());
                }
                ASTNodeOrToken::Node(n) => {
                    let operand = self.lower_expr_d(n, ctx, depth + 1)?;
                    let operand_scalar = expr_is_known_scalar(&operand);
                    acc = Some(match (acc.take(), pending.take()) {
                        (None, _) => (operand, operand_scalar),
                        (Some((lhs, lhs_scalar)), Some(op)) => {
                            self.build_additive(lhs, lhs_scalar, operand, operand_scalar, &op)
                        }
                        (Some(_), None) => {
                            return Err(
                                self.err_at(node, "malformed additive expression".to_string())
                            )
                        }
                    });
                }
                ASTNodeOrToken::Token(_) => {}
            }
        }
        match acc {
            Some((e, _)) => Ok(Some(e)),
            None => Err(self.err_at(node, "empty additive expression".to_string())),
        }
    }

    /// Combine one fold step of an additive chain. `lhs_scalar`/`rhs_scalar`
    /// are the caller's already-known scalar-ness of each operand (see
    /// `try_additive`'s doc comment on why these are threaded rather than
    /// re-derived); returns the built `Expr` plus whether *it* is itself
    /// known-scalar (`lhs_scalar && rhs_scalar`, matching the `BuiltinCall`
    /// condition below), for the next fold step to reuse in turn.
    fn build_additive(
        &mut self,
        lhs: Expr,
        lhs_scalar: bool,
        rhs: Expr,
        rhs_scalar: bool,
        op: &str,
    ) -> (Expr, bool) {
        let span = lhs.span().clone();
        if lhs_scalar && rhs_scalar {
            (
                Expr::BuiltinCall {
                    name: op.to_string(),
                    args: vec![lhs, rhs],
                    effects: EffectSet::PURE,
                    span,
                },
                true,
            )
        } else {
            // `Expr::ElementwiseOp` requires `MatrixOps` + `ArrayColumnMajor`
            // per the validator's own ground truth -- not `NDArrays`.
            self.observed.add(Feature::MatrixOps);
            self.observed.add(Feature::ArrayColumnMajor);
            let kind = if op == "+" {
                ElementwiseOpKind::Add
            } else {
                ElementwiseOpKind::Sub
            };
            (
                Expr::ElementwiseOp {
                    op: kind,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span,
                },
                false,
            )
        }
    }

    fn try_multiplicative(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, MatlabLowerError> {
        const OPS: &[&str] = &["*", "/", "\\", ".*", "./", ".\\"];
        let has_op = node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if OPS.contains(&t.value.as_str())));
        if !has_op {
            return Ok(None);
        }
        self.check_chain_length(node)?;
        // See `try_additive`'s doc comment: `acc` tracks scalar-ness
        // incrementally alongside the accumulated `Expr` rather than
        // re-deriving it by re-walking the growing tree every fold step.
        let mut acc: Option<(Expr, bool)> = None;
        let mut pending: Option<String> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if OPS.contains(&t.value.as_str()) => {
                    pending = Some(t.value.clone());
                }
                ASTNodeOrToken::Node(n) => {
                    let operand = self.lower_expr_d(n, ctx, depth + 1)?;
                    let operand_scalar = expr_is_known_scalar(&operand);
                    acc = Some(match (acc.take(), pending.take()) {
                        (None, _) => (operand, operand_scalar),
                        (Some((lhs, lhs_scalar)), Some(op)) => self.build_multiplicative(
                            lhs,
                            lhs_scalar,
                            operand,
                            operand_scalar,
                            &op,
                            node,
                        )?,
                        (Some(_), None) => {
                            return Err(self.err_at(
                                node,
                                "malformed multiplicative expression".to_string(),
                            ))
                        }
                    });
                }
                ASTNodeOrToken::Token(_) => {}
            }
        }
        match acc {
            Some((e, _)) => Ok(Some(e)),
            None => Err(self.err_at(node, "empty multiplicative expression".to_string())),
        }
    }

    /// `lhs_scalar`/`rhs_scalar` are the caller's already-known scalar-ness
    /// of each operand -- see `try_additive`'s doc comment on why these are
    /// threaded rather than re-derived by calling `expr_is_known_scalar` on
    /// the growing accumulator at every fold step (the same unbounded-
    /// recursion hazard applies here identically). Returns the built `Expr`
    /// plus whether it is itself known-scalar, for the next fold step.
    fn build_multiplicative(
        &mut self,
        lhs: Expr,
        lhs_scalar: bool,
        rhs: Expr,
        rhs_scalar: bool,
        op: &str,
        node: &GrammarASTNode,
    ) -> Result<(Expr, bool), MatlabLowerError> {
        let span = lhs.span().clone();
        match op {
            ".*" => {
                if lhs_scalar && rhs_scalar {
                    Ok((
                        Expr::BuiltinCall {
                            name: "*".to_string(),
                            args: vec![lhs, rhs],
                            effects: EffectSet::PURE,
                            span,
                        },
                        true,
                    ))
                } else {
                    self.observed.add(Feature::MatrixOps);
                    self.observed.add(Feature::ArrayColumnMajor);
                    Ok((
                        Expr::ElementwiseOp {
                            op: ElementwiseOpKind::Mul,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                            span,
                        },
                        false,
                    ))
                }
            }
            "./" => {
                if lhs_scalar && rhs_scalar {
                    Ok((
                        Expr::BuiltinCall {
                            name: "/".to_string(),
                            args: vec![lhs, rhs],
                            effects: EffectSet::PURE,
                            span,
                        },
                        true,
                    ))
                } else {
                    self.observed.add(Feature::MatrixOps);
                    self.observed.add(Feature::ArrayColumnMajor);
                    Ok((
                        Expr::ElementwiseOp {
                            op: ElementwiseOpKind::Div,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                            span,
                        },
                        false,
                    ))
                }
            }
            ".\\" => {
                if lhs_scalar && rhs_scalar {
                    Ok((
                        Expr::BuiltinCall {
                            name: "/".to_string(),
                            args: vec![rhs, lhs],
                            effects: EffectSet::PURE,
                            span,
                        },
                        true,
                    ))
                } else {
                    self.observed.add(Feature::MatrixOps);
                    self.observed.add(Feature::ArrayColumnMajor);
                    Ok((
                        Expr::ElementwiseOp {
                            op: ElementwiseOpKind::Div,
                            lhs: Box::new(rhs),
                            rhs: Box::new(lhs),
                            span,
                        },
                        false,
                    ))
                }
            }
            "*" => {
                if lhs_scalar && rhs_scalar {
                    Ok((
                        Expr::BuiltinCall {
                            name: "*".to_string(),
                            args: vec![lhs, rhs],
                            effects: EffectSet::PURE,
                            span,
                        },
                        true,
                    ))
                } else if lhs_scalar || rhs_scalar {
                    self.observed.add(Feature::MatrixOps);
                    self.observed.add(Feature::ArrayColumnMajor);
                    Ok((
                        Expr::ElementwiseOp {
                            op: ElementwiseOpKind::Mul,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                            span,
                        },
                        false,
                    ))
                } else {
                    self.observed.add(Feature::MatrixOps);
                    self.observed.add(Feature::ArrayColumnMajor);
                    Ok((
                        Expr::MatMul {
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                            span,
                        },
                        false,
                    ))
                }
            }
            "/" => {
                if lhs_scalar && rhs_scalar {
                    Ok((
                        Expr::BuiltinCall {
                            name: "/".to_string(),
                            args: vec![lhs, rhs],
                            effects: EffectSet::PURE,
                            span,
                        },
                        true,
                    ))
                } else if lhs_scalar || rhs_scalar {
                    self.observed.add(Feature::MatrixOps);
                    self.observed.add(Feature::ArrayColumnMajor);
                    Ok((
                        Expr::ElementwiseOp {
                            op: ElementwiseOpKind::Div,
                            lhs: Box::new(lhs),
                            rhs: Box::new(rhs),
                            span,
                        },
                        false,
                    ))
                } else {
                    Err(self.err_at(
                        node,
                        "unsupported: matrix right division `/` (mrdivide) has no backend \
                         kernel yet"
                            .to_string(),
                    ))
                }
            }
            "\\" => {
                if lhs_scalar && rhs_scalar {
                    Ok((
                        Expr::BuiltinCall {
                            name: "/".to_string(),
                            args: vec![rhs, lhs],
                            effects: EffectSet::PURE,
                            span,
                        },
                        true,
                    ))
                } else {
                    Err(self.err_at(
                        node,
                        "unsupported: matrix left division `\\` (mldivide) has no backend \
                         kernel yet"
                            .to_string(),
                    ))
                }
            }
            other => Err(self.err_at(node, format!("unsupported multiplicative operator `{other}`"))),
        }
    }

    fn lower_unary(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, MatlabLowerError> {
        if node.rule_name != "unary" || node.children.len() != 2 {
            return Ok(None);
        }
        let sign = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) => Some(t.value.clone()),
                ASTNodeOrToken::Node(_) => None,
            })
            .ok_or_else(|| self.err_at(node, "malformed unary expression".to_string()))?;
        let inner_node = *child_nodes(node)
            .first()
            .ok_or_else(|| self.err_at(node, "malformed unary expression: no operand".to_string()))?;
        let operand = self.lower_expr_d(inner_node, ctx, depth + 1)?;
        let span = operand.span().clone();
        let result = match sign.as_str() {
            "+" => operand,
            "-" => match operand {
                Expr::IntLit { value, span } => Expr::IntLit {
                    value: value.wrapping_neg(),
                    span,
                },
                Expr::FloatLit { value, span } => Expr::FloatLit { value: -value, span },
                other => Expr::BuiltinCall {
                    name: "neg".to_string(),
                    args: vec![other],
                    effects: EffectSet::PURE,
                    span,
                },
            },
            "~" => Expr::BuiltinCall {
                name: "not".to_string(),
                args: vec![operand],
                effects: EffectSet::PURE,
                span,
            },
            other => return Err(self.err_at(node, format!("unsupported unary operator `{other}`"))),
        };
        Ok(Some(result))
    }

    fn try_power(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, MatlabLowerError> {
        if node.rule_name != "power" {
            return Ok(None);
        }
        let op = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.value == "^" || t.value == ".^" => Some(t.value.clone()),
            _ => None,
        });
        let op = match op {
            Some(o) => o,
            None => return Ok(None),
        };
        let (base, exp) = match child_nodes(node).as_slice() {
            [b, e] => (*b, *e),
            _ => return Err(self.err_at(node, "malformed power expression".to_string())),
        };
        let _ = op; // both `^` and `.^` lower identically (see module scope note)
        let base_e = self.lower_expr_d(base, ctx, depth + 1)?;
        let exp_e = self.lower_expr_d(exp, ctx, depth + 1)?;
        let span = base_e.span().clone();
        self.observed.add(Feature::MatrixOps);
        self.observed.add(Feature::ArrayColumnMajor);
        Ok(Some(Expr::ElementwiseOp {
            op: ElementwiseOpKind::Pow,
            lhs: Box::new(base_e),
            rhs: Box::new(exp_e),
            span,
        }))
    }

    // -------------------------------------------------------------------
    // postfix: transpose / call / index
    // -------------------------------------------------------------------

    fn lower_postfix(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, MatlabLowerError> {
        if node.rule_name != "postfix" {
            return Ok(None);
        }
        let kids = child_nodes(node);
        let (primary, suffixes) = match kids.split_first() {
            Some((p, rest)) => (*p, rest),
            None => return Err(self.err_at(node, "malformed postfix expression".to_string())),
        };
        if suffixes.is_empty() {
            return self.lower_primary(primary, ctx, depth + 1).map(Some);
        }

        let mut acc: Option<Expr> = None;
        let mut first = true;
        for suffix in suffixes {
            match suffix.rule_name.as_str() {
                "transpose_suffix" => {
                    let target = match acc.take() {
                        Some(e) => e,
                        None => self.lower_primary(primary, ctx, depth + 1)?,
                    };
                    let tok = suffix
                        .token()
                        .ok_or_else(|| self.err_at(suffix, "malformed transpose suffix".to_string()))?;
                    let conjugate = tok.value == "'";
                    let span = target.span().clone();
                    self.observed.add(Feature::MatrixOps);
                    self.observed.add(Feature::ArrayColumnMajor);
                    acc = Some(Expr::Transpose {
                        target: Box::new(target),
                        conjugate,
                        span,
                    });
                }
                "call_suffix" => {
                    if first {
                        let name = self.primary_bare_name(primary).ok_or_else(|| {
                            self.err_at(
                                primary,
                                "unsupported: call/index target is not a bare name".to_string(),
                            )
                        })?;
                        if ctx.locals.contains(&name) || ctx.params.contains(&name) {
                            let span = self.span_of(primary);
                            let indices = self.lower_index_args(suffix, ctx, depth + 1)?;
                            acc = Some(Expr::IndexGet {
                                target: Box::new(Expr::VarRef {
                                    name,
                                    scope: Scope::Local,
                                    span: span.clone(),
                                }),
                                indices,
                                span,
                            });
                        } else if name == "disp" {
                            // The one builtin this frontend recognises: MATLAB's
                            // `disp` maps onto the SIR `print` builtin every
                            // backend already implements, matching every other
                            // frontend's own "print"/"puts" convention. Without
                            // this there would be no way for a lowered MATLAB
                            // program to produce observable output at all.
                            let span = self.span_of(primary);
                            let args = self.lower_call_args(suffix, ctx, depth + 1)?;
                            if args.len() != 1 {
                                return Err(self.err_at(
                                    primary,
                                    "`disp` takes exactly one argument".to_string(),
                                ));
                            }
                            acc = Some(Expr::BuiltinCall {
                                name: "print".to_string(),
                                args,
                                effects: EffectSet::PURE,
                                span,
                            });
                        } else if self.function_names.contains(&name) {
                            let span = self.span_of(primary);
                            let args = self.lower_call_args(suffix, ctx, depth + 1)?;
                            acc = Some(Expr::DirectCall {
                                fn_name: name,
                                args,
                                effects: EffectSet::PURE,
                                span,
                            });
                        } else {
                            return Err(self.err_at(
                                primary,
                                format!(
                                    "unsupported: unknown identifier `{name}` (not a known \
                                     variable or user function)"
                                ),
                            ));
                        }
                    } else {
                        let base = acc
                            .take()
                            .expect("acc is set after the first suffix in the fold");
                        let span = base.span().clone();
                        let indices = self.lower_index_args(suffix, ctx, depth + 1)?;
                        acc = Some(Expr::IndexGet {
                            target: Box::new(base),
                            indices,
                            span,
                        });
                    }
                }
                "cell_suffix" | "field_suffix" => {
                    return Err(self.err_at(
                        suffix,
                        format!("unsupported: `{}` is out of scope for v0.1.0", suffix.rule_name),
                    ))
                }
                other => return Err(self.err_at(suffix, format!("unsupported postfix suffix `{other}`"))),
            }
            first = false;
        }
        Ok(acc)
    }

    fn lower_primary(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Expr, MatlabLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        if let Some(tok) = node.token() {
            let span = self.span_of(node);
            return match tok.type_ {
                TokenType::Number => Ok(number_literal_expr(tok, &span)),
                TokenType::String => Ok(Expr::StrLit {
                    value: tok.value.clone(),
                    span,
                }),
                TokenType::Name => {
                    let name = tok.value.clone();
                    if ctx.params.contains(&name) {
                        Ok(Expr::VarRef {
                            name,
                            scope: Scope::Param,
                            span,
                        })
                    } else if ctx.locals.contains(&name) {
                        Ok(Expr::VarRef {
                            name,
                            scope: Scope::Local,
                            span,
                        })
                    } else {
                        Err(self.err_at(
                            node,
                            format!("undefined variable `{name}` (not previously assigned)"),
                        ))
                    }
                }
                _ => Err(self.err_at(node, format!("unsupported literal token `{}`", tok.value))),
            };
        }
        let only = match child_nodes(node).as_slice() {
            [only] => *only,
            _ => return Err(self.err_at(node, "malformed primary expression".to_string())),
        };
        match only.rule_name.as_str() {
            "matrix_literal" => self.lower_matrix_literal(only, ctx, depth + 1),
            "cell_literal" => Err(self.err_at(
                only,
                "unsupported: cell arrays are out of scope for v0.1.0".to_string(),
            )),
            "lambda" => Err(self.err_at(
                only,
                "unsupported: anonymous functions (`@(...) ...`) are out of scope for v0.1.0"
                    .to_string(),
            )),
            "group" => match child_nodes(only).as_slice() {
                [inner] => self.lower_expr_d(inner, ctx, depth + 1),
                _ => Err(self.err_at(only, "malformed parenthesised expression".to_string())),
            },
            other => Err(self.err_at(only, format!("unsupported: `{other}` (deferred)"))),
        }
    }

    fn lower_matrix_literal(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Expr, MatlabLowerError> {
        if depth > MAX_BLOCK_DEPTH {
            return Err(self.err_at(
                node,
                format!("matrix literal nesting too deep (exceeds {MAX_BLOCK_DEPTH} levels)"),
            ));
        }
        // `Expr::ArrayLit` requires `NDArrays` + `ArrayColumnMajor` (not
        // `MatrixOps`) per the validator's own ground truth.
        self.observed.add(Feature::NDArrays);
        self.observed.add(Feature::ArrayColumnMajor);
        let span = self.span_of(node);
        let mut rows: Vec<Vec<Expr>> = Vec::new();
        if let Some(matrix_rows) = self.first_child_named(node, "matrix_rows") {
            for row in child_nodes(matrix_rows) {
                if row.rule_name == "matrix_row" {
                    let mut cells = Vec::new();
                    for cell in child_nodes(row) {
                        cells.push(self.lower_expr_d(cell, ctx, depth + 1)?);
                    }
                    rows.push(cells);
                }
            }
        }
        Ok(Expr::ArrayLit { rows, span })
    }

    // -------------------------------------------------------------------
    // indexing / call arguments
    // -------------------------------------------------------------------

    /// `depth` is the *enclosing expression's* depth, not a fresh count --
    /// each index argument is lowered via [`Self::lower_expr_d`] at
    /// `depth + 1`, not the depth-resetting [`Self::lower_expr`], so that a
    /// chain of nested indexing (`A(A(A(...))))`) actually accumulates
    /// against [`MAX_EXPR_DEPTH`] instead of each level silently restarting
    /// its own budget (the bug this comment exists to prevent regressing).
    fn lower_index_args(
        &mut self,
        call_suffix: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Vec<IndexArg>, MatlabLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                call_suffix,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        // `Expr::IndexGet`/`Stmt::IndexSet` only require `NDArrays` per the
        // validator's own ground truth.
        self.observed.add(Feature::NDArrays);
        let arg_list = match self.first_child_named(call_suffix, "arg_list") {
            Some(a) => a,
            None => return Ok(vec![]),
        };
        let mut out = Vec::new();
        for arg in child_nodes(arg_list) {
            if arg.rule_name == "arg" {
                out.push(self.lower_one_index_arg(arg, ctx, depth + 1)?);
            }
        }
        Ok(out)
    }

    /// Lower one index-position argument, translating 1-based MATLAB
    /// indexing to the IR's 0-based convention: a literal integer index
    /// constant-folds (`A(3)` → `Scalar(IntLit(2))`); anything else emits
    /// `BuiltinCall("-", [idx, 1])` (`A(i)` → `Scalar(i - 1)`). See
    /// [`Self::lower_index_args`] on why `depth` is threaded rather than
    /// restarted.
    fn lower_one_index_arg(
        &mut self,
        arg: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<IndexArg, MatlabLowerError> {
        if let Some(tok) = arg.token() {
            if tok.value == ":" {
                return Ok(IndexArg::Whole);
            }
        }
        let inner = match child_nodes(arg).as_slice() {
            [only] => *only,
            _ => return Err(self.err_at(arg, "malformed index argument".to_string())),
        };
        let idx = self.lower_expr_d(inner, ctx, depth)?;
        let span = idx.span().clone();
        let shifted = match idx {
            Expr::IntLit { value, .. } => Expr::IntLit {
                value: value - 1,
                span,
            },
            other => Expr::BuiltinCall {
                name: "-".to_string(),
                args: vec![
                    other,
                    Expr::IntLit {
                        value: 1,
                        span: span.clone(),
                    },
                ],
                effects: EffectSet::PURE,
                span,
            },
        };
        Ok(IndexArg::Scalar(Box::new(shifted)))
    }

    /// See [`Self::lower_index_args`] on why `depth` is the caller's
    /// enclosing depth (threaded via [`Self::lower_expr_d`]), not a fresh
    /// count restarted via the depth-resetting [`Self::lower_expr`].
    fn lower_call_args(
        &mut self,
        call_suffix: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Vec<Expr>, MatlabLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                call_suffix,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        let arg_list = match self.first_child_named(call_suffix, "arg_list") {
            Some(a) => a,
            None => return Ok(vec![]),
        };
        let mut out = Vec::new();
        for arg in child_nodes(arg_list) {
            if arg.rule_name != "arg" {
                continue;
            }
            if let Some(tok) = arg.token() {
                if tok.value == ":" {
                    return Err(self.err_at(
                        arg,
                        "unsupported: `:` is not a valid function-call argument".to_string(),
                    ));
                }
            }
            let inner = match child_nodes(arg).as_slice() {
                [only] => *only,
                _ => return Err(self.err_at(arg, "malformed call argument".to_string())),
            };
            out.push(self.lower_expr_d(inner, ctx, depth + 1)?);
        }
        Ok(out)
    }

    // -------------------------------------------------------------------
    // target resolution (assignment LHS)
    // -------------------------------------------------------------------

    fn bare_name(&self, node: &GrammarASTNode) -> Option<String> {
        let postfix = self.peel_to_named(node, "postfix", 0)?;
        match child_nodes(postfix).as_slice() {
            [primary] => self.primary_bare_name(primary),
            _ => None,
        }
    }

    fn indexed_target<'a>(
        &self,
        node: &'a GrammarASTNode,
    ) -> Option<(String, &'a GrammarASTNode)> {
        let postfix = self.peel_to_named(node, "postfix", 0)?;
        match child_nodes(postfix).as_slice() {
            [primary, suffix] if suffix.rule_name == "call_suffix" => {
                self.primary_bare_name(primary).map(|name| (name, *suffix))
            }
            _ => None,
        }
    }

    fn primary_bare_name(&self, primary: &GrammarASTNode) -> Option<String> {
        let tok = primary.token()?;
        if tok.type_ == TokenType::Name {
            Some(tok.value.clone())
        } else {
            None
        }
    }

    // -------------------------------------------------------------------
    // small tree helpers
    // -------------------------------------------------------------------

    /// Peel through a chain of single-Node-child wrapper rules until
    /// reaching a node named `name`, or return `None` if the chain
    /// branches or runs out of depth first.
    fn peel_to_named<'a>(
        &self,
        node: &'a GrammarASTNode,
        name: &str,
        depth: usize,
    ) -> Option<&'a GrammarASTNode> {
        if depth > MAX_EXPR_DEPTH {
            return None;
        }
        if node.rule_name == name {
            return Some(node);
        }
        match child_nodes(node).as_slice() {
            [only] if node.children.len() == 1 => self.peel_to_named(only, name, depth + 1),
            _ => None,
        }
    }

    fn first_child_named<'a>(
        &self,
        node: &'a GrammarASTNode,
        kind: &str,
    ) -> Option<&'a GrammarASTNode> {
        child_nodes(node).into_iter().find(|n| n.rule_name == kind)
    }

    fn span_of(&self, node: &GrammarASTNode) -> Span {
        Span::point(
            FILE,
            node.start_line.unwrap_or(1),
            node.start_column.unwrap_or(1),
        )
    }

    fn err_at(&self, node: &GrammarASTNode, message: String) -> MatlabLowerError {
        MatlabLowerError {
            message,
            line: node.start_line.unwrap_or(1),
            column: node.start_column.unwrap_or(1),
        }
    }

    /// Reject a same-precedence operator chain (`additive`/`multiplicative`/
    /// `comparison`/`logical_or`/`logical_and`) with more than
    /// `MAX_EXPR_DEPTH` operands.
    ///
    /// The MATLAB grammar collapses a flat run of `+`/`-`/`*`/... into ONE
    /// CST node with many children rather than nesting through parens, so a
    /// long unparenthesized chain never trips the ordinary grammar-nesting
    /// depth guard. But folding N operands left-associatively still builds
    /// an N-deep *binary* `Expr` tree — and that tree's own depth is what
    /// matters for every later recursive pass over it (this crate's own
    /// scalar-ness check, but just as much the shared validator, any
    /// backend's emit pass, and even plain `Drop`, none of which cap
    /// depth themselves). A 60,000-term chain was confirmed to overflow
    /// the native stack during security review, even after fixing this
    /// file's own O(1)-per-fold-step scalar tracking, precisely because
    /// the resulting tree was still 60,000 levels deep regardless of how
    /// cheaply each level was built. Capping the operand *count* here — not
    /// just the construction cost — is the only fix that actually bounds
    /// the tree, so this check is deliberately unconditional (it does not
    /// try to distinguish "still cheap to build" from "already too deep to
    /// ever safely walk again").
    fn check_chain_length(&self, node: &GrammarASTNode) -> Result<(), MatlabLowerError> {
        let operand_count = node
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(_)))
            .count();
        if operand_count > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!(
                    "expression chain too long ({operand_count} operands, exceeds \
                     {MAX_EXPR_DEPTH})"
                ),
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

/// Collect the *node* children of `node` (dropping tokens).
fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

/// A `NUMBER` lexeme is a float if it has a decimal point or exponent,
/// otherwise an int; an integer lexeme too large for `i64` falls back to a
/// float rather than silently truncating or erroring.
fn number_literal_expr(tok: &Token, span: &Span) -> Expr {
    let text = &tok.value;
    if text.contains('.') || text.contains('e') || text.contains('E') {
        Expr::FloatLit {
            value: text.parse::<f64>().unwrap_or(0.0),
            span: span.clone(),
        }
    } else {
        match text.parse::<i64>() {
            Ok(v) => Expr::IntLit {
                value: v,
                span: span.clone(),
            },
            Err(_) => Expr::FloatLit {
                value: text.parse::<f64>().unwrap_or(0.0),
                span: span.clone(),
            },
        }
    }
}

/// Is `e` provably a scalar? See the module doc comment's "Scalar/array
/// disambiguation" section — this is a syntactic, non-evaluating check on
/// the *lowered* expression tree, not full constant folding.
fn expr_is_known_scalar(e: &Expr) -> bool {
    expr_is_known_scalar_d(e, 0)
}

/// Depth-capped core of [`expr_is_known_scalar`]. This is defense in depth,
/// not the primary fix for deep recursion here: every call site that folds a
/// *chain* of same-precedence operators (`build_additive`/
/// `build_multiplicative`) tracks each operand's scalar-ness incrementally
/// instead of re-deriving it by re-walking the whole accumulated left-hand
/// tree on every fold step -- re-deriving it that way would cost O(depth)
/// stack per step (O(chain length) at the final step) for an ordinary flat
/// `1 + 1 + 1 + ... + 1` chain, which has no bound at all from
/// `MAX_EXPR_DEPTH` (that guard counts *grammar nesting*, not the length of
/// one flat repetition -- a long unparenthesized chain never nests at the
/// CST level). This cap only protects a caller that (incorrectly) invokes
/// `expr_is_known_scalar` on a re-walked accumulator in the future;
/// returning `false` past the cap is always semantically safe, since a
/// "not provably scalar" verdict only ever falls through to the equally
/// correct array-domain node.
fn expr_is_known_scalar_d(e: &Expr, depth: usize) -> bool {
    if depth > MAX_EXPR_DEPTH {
        return false;
    }
    match e {
        Expr::IntLit { .. } | Expr::FloatLit { .. } => true,
        Expr::BuiltinCall { name, args, .. }
            if matches!(name.as_str(), "+" | "-" | "*" | "/" | "neg") =>
        {
            args.iter().all(|a| expr_is_known_scalar_d(a, depth + 1))
        }
        _ => false,
    }
}

/// An empty `Block` whose value is `NilLit`.
fn empty_block(span: Span) -> Block {
    Block {
        stmts: vec![],
        value: Expr::NilLit { span: span.clone() },
        span,
    }
}

/// A `Block` with no statements whose value is `expr`.
fn value_block(expr: Expr) -> Block {
    let span = expr.span().clone();
    Block {
        stmts: vec![],
        value: expr,
        span,
    }
}

/// Assemble a list of lowered items into a `Block` whose every item is a
/// statement (bare expressions wrapped as `ExprStmt`) and whose value is
/// always `value` — unlike a script-oriented frontend, MATLAB has no
/// "trailing expression is the result" convention at any body level
/// (scripts have no return value; only a function's designated output
/// variable does, and that is threaded in explicitly by the caller).
fn assemble_stmts_only(items: Vec<Lowered>, value: Expr, span: Span) -> Block {
    let stmts: Vec<Stmt> = items
        .into_iter()
        .map(|item| match item {
            Lowered::Stmt(s) => *s,
            Lowered::Expr(expr) => {
                let s = expr.span().clone();
                Stmt::ExprStmt { expr, span: s }
            }
        })
        .collect();
    Block { stmts, value, span }
}
