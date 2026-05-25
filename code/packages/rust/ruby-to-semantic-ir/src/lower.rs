//! Ruby `GrammarASTNode` → `semantic_ir::Module` lowering.
//!
//! See [the crate README](../README.md) for the v0 scope.  The
//! lowering is deliberately tiny because the v0 ruby-parser grammar
//! itself is tiny (six rules: `program`, `statement`, `assignment`,
//! `method_call`, `expression_stmt`, `expression`, `term`, `factor`).
//! Anything Ruby can write that isn't covered by those rules either
//! fails to parse or reaches us as a more general shape that we
//! still pattern-match against.

use std::collections::HashSet;

use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, Effect, EffectSet, ExportName, Expr, Feature, FeatureManifest, Function, Metadata,
    Module, Param, Scope, Span, Stmt,
};

/// A failure encountered during Ruby → SIR lowering.
///
/// Carries 1-based line/column so callers can produce IDE-friendly
/// diagnostics.  When the position is unknown (e.g. the AST node
/// had no recorded span), the fields are zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RubyLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for RubyLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RubyLowerError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for RubyLowerError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed Ruby program into a `semantic_ir::Module`.
///
/// The root node must be a `program` rule node — that's what
/// [`coding_adventures_ruby_parser::parse_ruby`] always emits.  The
/// `module_name` becomes the SIR module identifier (typically the
/// source file's stem).
pub fn compile(program: &GrammarASTNode, module_name: &str) -> Result<Module, RubyLowerError> {
    if program.rule_name != "program" {
        return Err(RubyLowerError {
            message: format!(
                "expected root rule `program`, got `{}`",
                program.rule_name
            ),
            line: program.start_line.unwrap_or(0),
            column: program.start_column.unwrap_or(0),
        });
    }

    let mut lw = Lowerer {
        file_name: module_name.to_string(),
        declared_locals: HashSet::new(),
        current_params: HashSet::new(),
        user_functions: Vec::new(),
        features_used: HashSet::new(),
        block_counter: 0,
    };
    // Phase 6a: hoist `def name(params) … end` declarations to
    // top-level Functions BEFORE walking the rest of the program so
    // the main-body lowerer knows which names resolve as
    // `DirectCall` targets vs. unknown builtins.
    lw.collect_def_statements(program)?;
    let block = lw.lower_program(program)?;

    let main = Function {
        name: "main".to_string(),
        params: Vec::new(),
        return_type: None,
        captures: Vec::new(),
        body: block,
        effects: EffectSet::PURE,
        metadata: Metadata::new(),
        span: lw.span_of(program),
    };

    // User-defined functions come first, then `main`.  The SIR
    // validator doesn't care about ordering — backends that emit
    // forward declarations will still see `main` exported.
    let mut functions = std::mem::take(&mut lw.user_functions);
    functions.push(main);

    // SIR's validator requires the manifest to *exactly* match
    // usage (declared-but-unused is a warning, used-but-undeclared
    // is an error).  We've been tallying features as we lowered;
    // here we materialise them into the manifest in a stable
    // chronological order.
    let mut manifest = FeatureManifest::new();
    for f in [
        Feature::DynamicTyping,
        Feature::MutableBindings,
        Feature::Loops,
        Feature::Sequences,
        Feature::Maps,
        Feature::Symbols,
        Feature::Closures,
        // Phase 6l — method-call chains synthesise a `StrLit` for the
        // method name when packing into the `__method__` envelope.
        // StrLit usage triggers the `Strings` feature.
        Feature::Strings,
        // Phase 6z — float literals (`1.5`, `1e10`, `1.5e-3`) lower
        // to `Expr::FloatLit` and trigger the `Floats` feature.
        Feature::Floats,
    ] {
        if lw.features_used.contains(&f) {
            manifest.add(f);
        }
    }

    Ok(Module {
        name: module_name.to_string(),
        manifest,
        imports: Vec::new(),
        // `main` is the conventional entry point — exporting it lets
        // SIR backends recognise it as such.
        exports: vec![ExportName {
            name: "main".to_string(),
            span: Span::synthetic(),
        }],
        functions,
        globals: Vec::new(),
        metadata: Metadata::new(),
        span: lw.span_of(program),
    })
}

// ---------------------------------------------------------------------------
// Lowerer
// ---------------------------------------------------------------------------

struct Lowerer {
    file_name: String,
    /// Names already bound by a prior `Stmt::LetBinding` in the
    /// current scope.  Drives the `LetBinding` vs `Assign` choice:
    /// first occurrence binds, subsequent occurrences re-assign.
    declared_locals: HashSet<String>,
    /// Phase 6a: parameter names visible in the *current* function
    /// scope.  Empty at the top level (main).  When a Name token is
    /// emitted as a `VarRef`, this set decides whether the `scope`
    /// is `Scope::Param` (the validator's expectation for function
    /// parameters) or `Scope::Local`.
    current_params: HashSet<String>,
    /// Phase 6a: user-defined functions collected from
    /// `def name(params) … end` declarations.  Filled by
    /// `collect_def_statements` (a top-level hoisting pass) before
    /// the main-body lowerer runs.
    user_functions: Vec<Function>,
    /// Phase 6b: SIR features actually exercised by this lowering.
    /// The SIR validator requires manifests to *exactly* match
    /// usage (declared-but-unused is a warning, used-but-undeclared
    /// is an error), so we track on-demand instead of unconditionally
    /// declaring every feature.
    features_used: HashSet<semantic_ir::Feature>,
    /// Phase 6g: monotonically-increasing counter for synthesised
    /// closure-function names.  Each `method_with_block` increments
    /// it once to mint a fresh `__block_<n>` name for the trailing
    /// block's hoisted Function.
    block_counter: usize,
}

impl Lowerer {
    /// Build a `Span` from a node's recorded start/end positions.
    /// Missing positions fall back to a `point` at (0, 0) — fine for
    /// SIR validation purposes.
    fn span_of(&self, node: &GrammarASTNode) -> Span {
        let sl = node.start_line.unwrap_or(0);
        let sc = node.start_column.unwrap_or(0);
        let el = node.end_line.unwrap_or(sl);
        let ec = node.end_column.unwrap_or(sc);
        Span::new(&self.file_name, sl, sc, el, ec)
    }

    fn span_of_token(&self, t: &Token) -> Span {
        Span::point(&self.file_name, t.line, t.column)
    }

    // -------------------------------------------------------------------
    // program → Block
    // -------------------------------------------------------------------

    fn lower_program(&mut self, program: &GrammarASTNode) -> Result<Block, RubyLowerError> {
        // Collect the statement nodes (skip whitespace/newline
        // children that the parser may emit).
        let stmts_in: Vec<&GrammarASTNode> = program
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                _ => None,
            })
            .collect();

        if stmts_in.is_empty() {
            return Ok(Block {
                stmts: Vec::new(),
                value: Expr::NilLit { span: self.span_of(program) },
                span: self.span_of(program),
            });
        }

        // The last statement node *may* be promoted to the block's
        // `value` slot — but only if it's an `expression_stmt` (a
        // bare expression with no side-effecting structure around
        // it).  Assignments always stay as statements because they
        // bind a name, and method calls stay as statements because
        // their effects (printing, raising) are observed before any
        // value is consumed.  Exception: if the method call is the
        // sole tail of the program, we still promote it so the
        // module has a meaningful return value.
        let last_idx = stmts_in.len() - 1;
        let mut stmts_out: Vec<Stmt> = Vec::with_capacity(stmts_in.len());
        let mut value: Option<Expr> = None;

        for (i, s) in stmts_in.iter().enumerate() {
            let inner = self.first_node_child(s).ok_or_else(|| RubyLowerError {
                message: "statement node had no child rule".to_string(),
                line: s.start_line.unwrap_or(0),
                column: s.start_column.unwrap_or(0),
            })?;

            let is_tail = i == last_idx;
            let tail_kind = inner.rule_name.as_str();
            if is_tail && matches!(tail_kind, "expression_stmt" | "method_call" | "method_call_no_paren") {
                // Promote the tail expression to the block's value.
                let v = match tail_kind {
                    "expression_stmt" => {
                        let expr_node = self.first_node_child(inner).ok_or_else(|| {
                            RubyLowerError {
                                message: "expression_stmt had no expression child".to_string(),
                                line: inner.start_line.unwrap_or(0),
                                column: inner.start_column.unwrap_or(0),
                            }
                        })?;
                        self.lower_expression(expr_node)?
                    }
                    "method_call" | "method_call_no_paren" => self.lower_method_call(inner)?,
                    _ => unreachable!(),
                };
                value = Some(v);
            } else {
                // Phase 6r — use the multi-stmt dispatch wrapper so
                // `multi_assignment` nodes fan out into one SIR Stmt
                // per (lhs[i], rhs[i]) pair.
                stmts_out.extend(self.lower_statement_inner_multi(inner)?);
            }
        }

        let value = value.unwrap_or(Expr::NilLit { span: self.span_of(program) });
        Ok(Block {
            stmts: stmts_out,
            value,
            span: self.span_of(program),
        })
    }

    /// Phase 6r — multi-statement-emitting dispatch wrapper.
    ///
    /// Some Ruby source-statement forms lower to *multiple* SIR
    /// statements:
    ///
    /// - `multi_assignment` (`a, b = 1, 2`) → one `LetBinding`/`Assign`
    ///   per LHS-RHS pair.  The grammar groups them as a single
    ///   surface statement, but at the SIR level they're independent
    ///   bindings.
    ///
    /// Every other statement form produces exactly one SIR statement
    /// and is delegated to [`lower_statement_inner`].  The helper exists
    /// so callers walking a statement list (`lower_program`,
    /// `lower_clause_statements`, `lower_def_statement`, etc.) can
    /// uniformly `.extend(...)` the result instead of `.push(...)`-ing
    /// a single Stmt — keeping the single-stmt path lossless while
    /// permitting multi-stmt fan-out where the grammar warrants it.
    fn lower_statement_inner_multi(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Vec<Stmt>, RubyLowerError> {
        match node.rule_name.as_str() {
            "multi_assignment" => self.lower_multi_assignment(node),
            "begin_statement" => self.lower_begin_statement(node),
            _ => Ok(vec![self.lower_statement_inner(node)?]),
        }
    }

    /// Lower the inner rule node of a `statement` (one of
    /// `assignment` / `method_call` / `expression_stmt`) into a
    /// `Stmt`.
    fn lower_statement_inner(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Stmt, RubyLowerError> {
        match node.rule_name.as_str() {
            "assignment" => self.lower_assignment(node),
            "method_call" => {
                let expr = self.lower_method_call(node)?;
                Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                })
            }
            "method_call_no_paren" => {
                // Phase 6h: paren-less call.  Shape-compatible with
                // `method_call` (same callee + expression-arg layout
                // minus the LPAREN/RPAREN), so the existing
                // `lower_method_call` handles it transparently —
                // both shapes' `expression` children are collected
                // the same way.
                let expr = self.lower_method_call(node)?;
                Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                })
            }
            "expression_stmt" => {
                let expr_node = self.first_node_child(node).ok_or_else(|| RubyLowerError {
                    message: "expression_stmt had no expression child".to_string(),
                    line: node.start_line.unwrap_or(0),
                    column: node.start_column.unwrap_or(0),
                })?;
                let expr = self.lower_expression(expr_node)?;
                Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                })
            }
            "def_statement" => {
                // `def` declarations were hoisted to top-level
                // Functions in the pre-pass; here we drop them from
                // the main-body statement stream.  Returning a no-op
                // ExprStmt keeps the `Block.stmts` slot occupied but
                // valid SIR-wise.
                Ok(Stmt::ExprStmt {
                    expr: Expr::NilLit {
                        span: self.span_of(node),
                    },
                    span: self.span_of(node),
                })
            }
            "class_statement" | "module_statement" => {
                // Phase 6f: class/module declarations parse but don't
                // yet introduce a real namespace in SIR v0 (SIR has no
                // `class` / `namespace` node — that lands in a later
                // phase together with method dispatch).  We walk the
                // body so nested `def` declarations *are* hoisted to
                // top-level Functions (matching the def_statement
                // behaviour at the program level), then emit a no-op
                // ExprStmt(NilLit) in place of the class/module so
                // the main-body statement stream stays in sync with
                // the source line count.
                //
                // Caveat (documented for backends): the hoisted
                // methods land at top-level, not nested under the
                // class name.  In real Ruby, `class Foo; def bar`
                // makes `bar` an instance method of `Foo`.  v0 SIR
                // collapses the namespace; the validator still
                // accepts the result because every function has a
                // unique name and `main` is the only export.
                self.collect_def_statements_from_body(node)?;
                Ok(Stmt::ExprStmt {
                    expr: Expr::NilLit {
                        span: self.span_of(node),
                    },
                    span: self.span_of(node),
                })
            }
            "if_statement" | "unless_statement" => {
                // Phase 6b: SIR's `Expr::If` is an *expression* — it
                // always yields a value.  We wrap it in `Stmt::ExprStmt`
                // here so the body's value (or NilLit) propagates
                // through the SIR statement stream.
                let expr = self.lower_if_or_unless(node)?;
                Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                })
            }
            "case_statement" => {
                // Phase 6u — `case x; when v1[,v2,...] then body; else end`.
                //
                // Lower to a chained `Expr::If`:
                //
                //   case x
                //   when 1, 2 then a
                //   when 3    then b
                //   else c
                //   end
                //
                // becomes
                //
                //   if (x == 1 || x == 2) then a
                //   else if x == 3 then b
                //   else c
                //
                // wrapped in `Stmt::ExprStmt`.  Each `when_clause`
                // becomes a single `If` step; the else_clause (or
                // implicit `NilLit` block) terminates the chain.
                let expr = self.lower_case_statement(node)?;
                Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                })
            }
            "while_statement" | "until_statement" => {
                // Phase 6c: SIR's `Stmt::While` is the canonical
                // top-level loop — `until cond` lowers to
                // `while !cond` (wrap the condition in `not`).
                self.lower_while_or_until(node)
            }
            "method_with_block" => {
                // Phase 6g
                let expr = self.lower_method_with_block(node)?;
                return Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                });
            }
            "modifier_statement" => {
                // Phase 6q: trailing-modifier conditionals/loops.
                // `lhs if cond`    → ExprStmt(If(cond, [lhs], Nil))
                // `lhs unless cond`→ ExprStmt(If(not cond, [lhs], Nil))
                // `lhs while cond` → While(cond, [lhs])
                // `lhs until cond` → While(not cond, [lhs])
                self.lower_modifier_statement(node)
            }
            "yield_statement" => {
                // Phase 6t — `yield` keyword.
                //
                // Grammar shape:
                //   yield_statement = "yield" [ yield_args ] ;
                //   yield_args      = LPAREN [ call_arg { COMMA call_arg } ] RPAREN
                //                   | call_arg { COMMA call_arg } ;
                //
                // Lowering: `BuiltinCall("yield", lowered_args)` wrapped
                // in `Stmt::ExprStmt`.  The `yield_args` wrapper (when
                // present) holds the call_arg subnodes directly; we walk
                // either the statement node or the yield_args wrapper.
                //
                // Effects: PURE.  `yield` invokes the caller-supplied
                // block, whose effects bubble up through the call site's
                // effect set when the block is constructed.  Modelling
                // `yield` itself as PURE keeps the effect lattice from
                // double-counting block effects.
                let yield_args_node = self
                    .find_node_child(node, "yield_args");
                let call_arg_nodes: Vec<&GrammarASTNode> = if let Some(ya) = yield_args_node {
                    ya.children
                        .iter()
                        .filter_map(|c| match c {
                            ASTNodeOrToken::Node(n) if n.rule_name == "call_arg" => Some(n),
                            _ => None,
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                let args: Vec<Expr> = call_arg_nodes
                    .into_iter()
                    .map(|n| self.lower_call_arg(n))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Stmt::ExprStmt {
                    expr: Expr::BuiltinCall {
                        name: "yield".to_string(),
                        args,
                        effects: EffectSet::PURE,
                        span: self.span_of(node),
                    },
                    span: self.span_of(node),
                })
            }
            "return_statement" | "break_statement" | "next_statement" => {
                // Phase 6j: control-flow keywords lower to BuiltinCall
                // with Effect::Divergent.  Optional trailing expression
                // becomes the single arg; bare `return` carries NilLit.
                let name = match node.rule_name.as_str() {
                    "return_statement" => "return",
                    "break_statement" => "break",
                    "next_statement" => "next",
                    _ => unreachable!(),
                };
                let arg_node = self.find_node_child(node, "expression");
                let arg = match arg_node {
                    Some(n) => self.lower_expression(n)?,
                    None => Expr::NilLit { span: self.span_of(node) },
                };
                let expr = Expr::BuiltinCall {
                    name: name.to_string(),
                    args: vec![arg],
                    effects: EffectSet::PURE.with(Effect::Divergent),
                    span: self.span_of(node),
                };
                Ok(Stmt::ExprStmt {
                    expr,
                    span: self.span_of(node),
                })
            }
            other => Err(RubyLowerError {
                message: format!("unsupported statement form `{other}`"),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            }),
        }
    }

    // -------------------------------------------------------------------
    // Phase 6b — `if … else … end` / `unless … else … end`
    // -------------------------------------------------------------------

    /// Lower an `if_statement` or `unless_statement` node into an
    /// `Expr::If`.  Both rules have the same shape from the AST's
    /// perspective; the only difference is that `unless`'s
    /// condition is negated.  `elsif` chains nest right — the
    /// `else_branch` of the outermost `If` is itself an `If` for
    /// the first elsif, etc.
    fn lower_if_or_unless(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Expr, RubyLowerError> {
        let is_unless = node.rule_name == "unless_statement";
        // The first `expression` child is the condition.
        let cond_node = self
            .find_node_child(node, "expression")
            .ok_or_else(|| RubyLowerError {
                message: format!("{} missing condition expression", node.rule_name),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;
        let mut cond = self.lower_expression(cond_node)?;
        if is_unless {
            // `unless cond` is `if !cond` — wrap in `not` builtin.
            cond = Expr::BuiltinCall {
                name: "not".to_string(),
                args: vec![cond],
                effects: EffectSet::PURE,
                span: self.span_of(cond_node),
            };
        }

        // Then-branch body: every `statement` child *until* the
        // first elsif/else/end terminator.  Since the grammar
        // already segregates elsif/else into their own subnodes,
        // direct `statement` children of `node` are the then-body.
        let then_body = self.lower_clause_statements(node)?;

        // elsif chain — right-associative nesting.  Build the
        // tail starting from `else_clause` and unwind back through
        // any `elsif_clause` nodes in reverse order.
        let elsifs: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "elsif_clause" => Some(n),
                _ => None,
            })
            .collect();
        let else_clause: Option<&GrammarASTNode> = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "else_clause" => Some(n),
            _ => None,
        });

        // Start with the `else` body (or `NilLit` if absent).
        let mut tail = if let Some(ec) = else_clause {
            self.lower_clause_statements(ec)?
        } else {
            Block {
                stmts: Vec::new(),
                value: Expr::NilLit { span: self.span_of(node) },
                span: self.span_of(node),
            }
        };

        // Unwind elsif clauses in reverse order, each wrapping the
        // accumulated tail as its own else-branch.
        for ec in elsifs.iter().rev() {
            let ec_cond = self.find_node_child(ec, "expression").ok_or_else(|| {
                RubyLowerError {
                    message: "elsif_clause missing condition expression".to_string(),
                    line: ec.start_line.unwrap_or(0),
                    column: ec.start_column.unwrap_or(0),
                }
            })?;
            let ec_cond_expr = self.lower_expression(ec_cond)?;
            let ec_body = self.lower_clause_statements(ec)?;
            tail = Block {
                stmts: Vec::new(),
                value: Expr::If {
                    cond: Box::new(ec_cond_expr),
                    then_branch: Box::new(ec_body),
                    else_branch: Box::new(tail),
                    span: self.span_of(ec),
                },
                span: self.span_of(ec),
            };
        }

        Ok(Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_body),
            else_branch: Box::new(tail),
            span: self.span_of(node),
        })
    }

    /// Lower the `statement` children of a clause node (`if_statement`,
    /// `elsif_clause`, `else_clause`, `unless_statement`) into a
    /// `Block`.  Tail-expression promotion follows the same rule as
    /// `lower_program` — last bare `expression_stmt` / `method_call`
    /// becomes `value`, otherwise `value = NilLit`.
    fn lower_clause_statements(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Block, RubyLowerError> {
        // Phase 6b: each branch is an independent SIR `Block`.
        // Lock the declared-locals set to the outer-scope's snapshot
        // before lowering the body, then restore on exit.  Without
        // this, locals introduced in one `if`-branch would leak
        // into the other branch's scope and cause spurious
        // `Stmt::Assign` emissions (or vice versa).
        let saved_locals = self.declared_locals.clone();
        let stmts_in: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                _ => None,
            })
            .collect();
        if stmts_in.is_empty() {
            return Ok(Block {
                stmts: Vec::new(),
                value: Expr::NilLit { span: self.span_of(node) },
                span: self.span_of(node),
            });
        }
        let last_idx = stmts_in.len() - 1;
        let mut stmts_out: Vec<Stmt> = Vec::new();
        let mut value: Option<Expr> = None;
        for (i, s) in stmts_in.iter().enumerate() {
            let inner = self.first_node_child(s).ok_or_else(|| RubyLowerError {
                message: "statement node had no child rule".to_string(),
                line: s.start_line.unwrap_or(0),
                column: s.start_column.unwrap_or(0),
            })?;
            let is_tail = i == last_idx;
            let kind = inner.rule_name.as_str();
            if is_tail && matches!(kind, "expression_stmt" | "method_call" | "method_call_no_paren") {
                let v = match kind {
                    "expression_stmt" => {
                        let expr_node = self.first_node_child(inner).ok_or_else(|| {
                            RubyLowerError {
                                message: "expression_stmt had no expression child".to_string(),
                                line: inner.start_line.unwrap_or(0),
                                column: inner.start_column.unwrap_or(0),
                            }
                        })?;
                        self.lower_expression(expr_node)?
                    }
                    "method_call" | "method_call_no_paren" => self.lower_method_call(inner)?,
                    _ => unreachable!(),
                };
                value = Some(v);
            } else {
                // Phase 6r — multi-stmt fan-out for `multi_assignment`.
                stmts_out.extend(self.lower_statement_inner_multi(inner)?);
            }
        }
        let value = value.unwrap_or(Expr::NilLit { span: self.span_of(node) });
        // Restore the outer scope's declared locals.
        self.declared_locals = saved_locals;
        Ok(Block {
            stmts: stmts_out,
            value,
            span: self.span_of(node),
        })
    }

    // -------------------------------------------------------------------
    // Phase 6c — `while cond … end` / `until cond … end`
    // -------------------------------------------------------------------

    /// Lower a `while_statement` or `until_statement` into a
    /// `Stmt::While`.  `until cond` lowers to `while !cond`
    /// (condition wrapped in `BuiltinCall("not", ...)`).
    fn lower_while_or_until(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Stmt, RubyLowerError> {
        let is_until = node.rule_name == "until_statement";
        let cond_node = self
            .find_node_child(node, "expression")
            .ok_or_else(|| RubyLowerError {
                message: format!("{} missing condition expression", node.rule_name),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;
        let mut cond = self.lower_expression(cond_node)?;
        if is_until {
            cond = Expr::BuiltinCall {
                name: "not".to_string(),
                args: vec![cond],
                effects: EffectSet::PURE,
                span: self.span_of(cond_node),
            };
        }
        let body = self.lower_clause_statements(node)?;
        // Phase 6c: the SIR validator requires `loops` to be
        // declared whenever the module emits a `Stmt::While` /
        // `Stmt::ForRange` / `Stmt::ForEach`.
        self.features_used.insert(Feature::Loops);
        Ok(Stmt::While {
            cond,
            body,
            span: self.span_of(node),
        })
    }

    // -------------------------------------------------------------------
    // Phase 6u — `case … when … else … end`
    // -------------------------------------------------------------------

    /// Lower a `case_statement` node to a chained `Expr::If`.
    ///
    /// Grammar shape (per `ruby.grammar`):
    /// ```text
    /// case_statement = "case" expression { when_clause } [ else_clause ] "end" ;
    /// when_clause    = "when" expression { COMMA expression }
    ///                       { !"when" !"else" !"end" statement } ;
    /// ```
    ///
    /// Lowering rule:
    ///
    /// ```text
    /// case x
    /// when v1, v2 then body_a
    /// when v3     then body_b
    /// else body_c
    /// end
    /// ```
    ///
    /// becomes
    ///
    /// ```text
    /// if ((x == v1) || (x == v2)) then body_a
    /// else if (x == v3) then body_b
    /// else body_c
    /// ```
    ///
    /// Each `when_clause` produces one nested `If` step.  Multiple values
    /// in a single `when` (`when 1, 2, 3`) chain through `BuiltinCall("or", ...)`
    /// inside that step's condition.  The else terminator (or an empty
    /// `NilLit` block when absent) caps the chain.
    ///
    /// v0 caveats (deferred):
    /// - Ruby's `when` uses `===` (case-equality, class-aware) — this
    ///   v0 lowers to `==`.  Phase 7d adds full `case/in` pattern
    ///   matching with proper match semantics.
    /// - Range/Regex/Class values in `when` lists work syntactically
    ///   (they parse as expressions) but the `==` comparison won't
    ///   match Ruby's case-equality semantics.
    fn lower_case_statement(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Expr, RubyLowerError> {
        // 1. Scrutinee — the first `expression` direct child of the
        //    case_statement.  (subsequent `expression` children belong
        //    to when_clause descendants, but they're inside subnodes,
        //    not direct children.)
        let scrutinee_node = self
            .find_node_child(node, "expression")
            .ok_or_else(|| RubyLowerError {
                message: "case_statement missing scrutinee expression".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;
        let scrutinee = self.lower_expression(scrutinee_node)?;

        // 2. Collect every when_clause subnode.
        let when_clauses: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "when_clause" => Some(n),
                _ => None,
            })
            .collect();

        // 3. Find the optional else_clause (reused from if_statement).
        let else_clause = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "else_clause" => Some(n),
            _ => None,
        });

        // 4. Build the tail: the else block, or an empty NilLit block
        //    if no else clause was provided.
        let mut tail: Block = if let Some(ec) = else_clause {
            self.lower_clause_statements(ec)?
        } else {
            Block {
                stmts: Vec::new(),
                value: Expr::NilLit { span: self.span_of(node) },
                span: self.span_of(node),
            }
        };

        // 5. Unwind when_clauses in reverse, each wrapping the
        //    accumulated tail as its else-branch.
        for wc in when_clauses.iter().rev() {
            // Collect this clause's value expressions (all `expression`
            // Node children in order).
            let value_nodes: Vec<&GrammarASTNode> = wc
                .children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                    _ => None,
                })
                .collect();
            if value_nodes.is_empty() {
                return Err(RubyLowerError {
                    message: "when_clause missing value expression(s)".to_string(),
                    line: wc.start_line.unwrap_or(0),
                    column: wc.start_column.unwrap_or(0),
                });
            }

            // Build the condition: `(scrutinee == v1) || (scrutinee == v2) || ...`.
            // Lower each value, build an `==` BuiltinCall, then OR-fold
            // left-to-right (matches Ruby's left-to-right when evaluation).
            let span = self.span_of(wc);
            let mut cond: Option<Expr> = None;
            for vn in &value_nodes {
                let val = self.lower_expression(vn)?;
                // Clone the scrutinee fresh for every comparison —
                // SIR is tree-shaped, no shared subexpressions.
                let cmp = Expr::BuiltinCall {
                    name: "==".to_string(),
                    args: vec![scrutinee.clone(), val],
                    effects: EffectSet::PURE,
                    span: span.clone(),
                };
                cond = Some(match cond {
                    None => cmp,
                    Some(prev) => Expr::BuiltinCall {
                        name: "or".to_string(),
                        args: vec![prev, cmp],
                        effects: EffectSet::PURE,
                        span: span.clone(),
                    },
                });
            }
            let cond = cond.expect("at least one when value");

            // Lower the when body — same shape as `lower_clause_statements`
            // handles for if/elsif/else.
            let then_block = self.lower_clause_statements(wc)?;

            // Wrap into an If and let `tail` become the else.
            tail = Block {
                stmts: Vec::new(),
                value: Expr::If {
                    cond: Box::new(cond),
                    then_branch: Box::new(then_block),
                    else_branch: Box::new(tail),
                    span: span.clone(),
                },
                span,
            };
        }

        // The case expression is the chain's outermost If — which
        // currently sits as the `tail` Block's `value`.  Peel it out.
        Ok(tail.value)
    }

    // -------------------------------------------------------------------
    // Phase 6q — modifier conditionals/loops
    // -------------------------------------------------------------------

    /// Lower a `modifier_statement` node — Ruby's trailing-modifier
    /// surface syntax for one-line `if`/`unless`/`while`/`until`.
    ///
    /// Grammar shape (per `ruby.grammar`):
    /// ```text
    /// modifier_statement = ( assignment
    ///                      | method_call_no_paren
    ///                      | method_call
    ///                      | expression_stmt )
    ///                      ( "if_modifier" | "unless_modifier"
    ///                      | "while_modifier" | "until_modifier" )
    ///                      expression ;
    /// ```
    ///
    /// AST children layout: `[ lhs_node, modifier_kw_token, cond_node ]`
    /// — the leading group lands a single inner-rule node (one of the
    /// four LHS alternatives), then a keyword token whose value is
    /// `if_modifier`/`unless_modifier`/`while_modifier`/`until_modifier`
    /// (re-tagged by the lexer's `tag_modifier_keywords` post-pass),
    /// then the trailing `expression` node for the condition.
    ///
    /// Lowering table (the table form is reproduced in `ruby.grammar`):
    ///
    /// | Source              | Lowered SIR                                              |
    /// |---------------------|----------------------------------------------------------|
    /// | `lhs if cond`       | `Stmt::ExprStmt(Expr::If(cond, [lhs], Nil))`             |
    /// | `lhs unless cond`   | `Stmt::ExprStmt(Expr::If(not(cond), [lhs], Nil))`        |
    /// | `lhs while cond`    | `Stmt::While(cond, [lhs])`                               |
    /// | `lhs until cond`    | `Stmt::While(not(cond), [lhs])`                          |
    ///
    /// Lowering identity with the leading-keyword forms — same `Expr::If` /
    /// `Stmt::While` shapes — means every downstream emitter
    /// (semantic-ir-to-python / -rust / -typescript / -go) needs zero
    /// new code paths.  The Ruby user sees a syntactic shortcut; the
    /// SIR sees the canonical conditional/loop.
    ///
    /// The LHS body is wrapped in a single-statement `Block` (with
    /// `value: NilLit` — the modifier form is statement-position only,
    /// never tail-promoted to expression).
    fn lower_modifier_statement(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Stmt, RubyLowerError> {
        // 1. Find the LHS inner-rule node.  It's the first child that's
        //    one of the four LHS-eligible rules.
        let lhs_node = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n)
                    if matches!(
                        n.rule_name.as_str(),
                        "assignment"
                            | "method_call"
                            | "method_call_no_paren"
                            | "expression_stmt"
                    ) =>
                {
                    Some(n)
                }
                _ => None,
            })
            .ok_or_else(|| RubyLowerError {
                message: "modifier_statement missing LHS inner-rule node".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;

        // 2. Find the modifier keyword token value.  The lexer
        //    guarantees one of the four `*_modifier` values lives in
        //    a Keyword token between LHS and cond.
        let modifier_kw = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t)
                    if matches!(
                        t.value.as_str(),
                        "if_modifier"
                            | "unless_modifier"
                            | "while_modifier"
                            | "until_modifier"
                    ) =>
                {
                    Some(t.value.as_str())
                }
                _ => None,
            })
            .ok_or_else(|| RubyLowerError {
                message: "modifier_statement missing modifier keyword token".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;

        // 3. Find the cond expression — the LAST `expression` rule
        //    node among direct children.  (LHS may contain nested
        //    `expression` nodes, but those are grand-children of
        //    `modifier_statement`, not direct children.  Using the
        //    last-direct-child position is robust against future
        //    grammar tweaks that might insert intermediate nodes.)
        let cond_node = node
            .children
            .iter()
            .rev()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                _ => None,
            })
            .ok_or_else(|| RubyLowerError {
                message: "modifier_statement missing condition expression".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;

        // 4. Lower the LHS into a Stmt, then wrap in a single-stmt
        //    Block.  Block.value is NilLit — modifier forms never sit
        //    in tail position.
        let lhs_stmt = self.lower_statement_inner(lhs_node)?;
        let body_block = Block {
            stmts: vec![lhs_stmt],
            value: Expr::NilLit {
                span: self.span_of(node),
            },
            span: self.span_of(node),
        };

        // 5. Lower the condition.  For `unless_modifier` / `until_modifier`,
        //    wrap it in `not` — identical to the leading-keyword
        //    `unless_statement` / `until_statement` lowerings.
        let mut cond = self.lower_expression(cond_node)?;
        let negate =
            matches!(modifier_kw, "unless_modifier" | "until_modifier");
        if negate {
            cond = Expr::BuiltinCall {
                name: "not".to_string(),
                args: vec![cond],
                effects: EffectSet::PURE,
                span: self.span_of(cond_node),
            };
        }

        // 6. Emit If (conditional modifiers) or While (loop modifiers).
        match modifier_kw {
            "if_modifier" | "unless_modifier" => {
                let else_block = Block {
                    stmts: Vec::new(),
                    value: Expr::NilLit {
                        span: self.span_of(node),
                    },
                    span: self.span_of(node),
                };
                Ok(Stmt::ExprStmt {
                    expr: Expr::If {
                        cond: Box::new(cond),
                        then_branch: Box::new(body_block),
                        else_branch: Box::new(else_block),
                        span: self.span_of(node),
                    },
                    span: self.span_of(node),
                })
            }
            "while_modifier" | "until_modifier" => {
                self.features_used.insert(Feature::Loops);
                Ok(Stmt::While {
                    cond,
                    body: body_block,
                    span: self.span_of(node),
                })
            }
            // The token-value filter above already rejected anything
            // outside the four valid modifier values; this arm is
            // unreachable.
            other => Err(RubyLowerError {
                message: format!("unknown modifier keyword `{other}`"),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            }),
        }
    }

    // -------------------------------------------------------------------
    // Phase 6a — def_statement hoisting
    // -------------------------------------------------------------------

    /// Pre-pass: walk `program` children and lift every
    /// `def_statement` into a top-level `Function` on
    /// `self.user_functions`.  Method bodies are recursively
    /// lowered using a *fresh* declared-locals set so the outer
    /// program's let-bindings don't leak in.
    fn collect_def_statements(
        &mut self,
        program: &GrammarASTNode,
    ) -> Result<(), RubyLowerError> {
        for child in &program.children {
            let stmt = match child {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => n,
                _ => continue,
            };
            let inner = match self.first_node_child(stmt) {
                Some(n) => n,
                None => continue,
            };
            if inner.rule_name != "def_statement" {
                continue;
            }
            let func = self.lower_def_statement(inner)?;
            self.user_functions.push(func);
        }
        Ok(())
    }

    /// Phase 6f: walk the body of a `class_statement` / `module_statement`
    /// and hoist every nested `def_statement` to a top-level `Function`
    /// on `self.user_functions`.  This mirrors the program-level
    /// `collect_def_statements` pre-pass — same hoisting semantics,
    /// same scope-isolation behaviour (each method body gets a fresh
    /// `declared_locals` / `current_params`).  Called from
    /// `lower_statement_inner` when the class/module is first reached,
    /// not from the global pre-pass, so each nested `def` is hoisted
    /// exactly once.
    fn collect_def_statements_from_body(
        &mut self,
        body_owner: &GrammarASTNode,
    ) -> Result<(), RubyLowerError> {
        for child in &body_owner.children {
            let stmt = match child {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => n,
                _ => continue,
            };
            let inner = match self.first_node_child(stmt) {
                Some(n) => n,
                None => continue,
            };
            if inner.rule_name == "def_statement" {
                let func = self.lower_def_statement(inner)?;
                self.user_functions.push(func);
            } else if inner.rule_name == "class_statement"
                || inner.rule_name == "module_statement"
            {
                // Nested class/module — recurse so deeply-nested
                // `def`s still get hoisted.
                self.collect_def_statements_from_body(inner)?;
            }
        }
        Ok(())
    }

    fn lower_def_statement(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Function, RubyLowerError> {
        // Shape:
        //   KEYWORD("def") NAME [ LPAREN [ params ] RPAREN ]
        //                  { !"end" statement } KEYWORD("end")
        // The first child token is the `def` keyword itself; the
        // method name is the *Name* token that follows.  We can't
        // use `expect_first_name_token` because it accepts both
        // Name and Keyword — it would return "def".
        let name_token = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => Some(t),
            _ => None,
        });
        let name_token = name_token.ok_or_else(|| RubyLowerError {
            message: "def_statement missing method-name token".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })?;
        let name = name_token.value.clone();

        // Collect parameters.  The optional `params` rule node holds
        // a sequence of `param` subnodes (Phase 6s — each param is
        // wrapped in its own rule so the optional `*` / `**` splat
        // prefix can sit inside the param slot).  We walk each `param`,
        // detect the splat prefix from its leading Token (`*` or `**`,
        // both with `value` set), and extract the parameter Name.
        //
        // v0 limitation: the splat-ness of a param is LOST at the SIR
        // level.  Param has no variadic flag, so a splat param lowers
        // to a regular Param with the bare Name (no `*` prefix in
        // `name`).  Downstream emitters therefore treat the parameter
        // as positional rather than variadic — a deferred correctness
        // limitation tracked for a future SIR phase.  The grammar +
        // parse round-trip is correct; the lossy SIR shape only
        // matters when generating target source for a Ruby program
        // that actually relies on variadic semantics.
        let params_node = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "params" => Some(n),
            _ => None,
        });
        let params: Vec<Param> = if let Some(pn) = params_node {
            pn.children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Node(param_node) if param_node.rule_name == "param" => {
                        // Find the parameter Name token, skipping the
                        // optional `*`/`**` splat prefix.  Both the
                        // prefix and the identifier land on Name-typed
                        // tokens (the 1.8-baseline state machine
                        // coalesces `**` into one Name token with value
                        // `"**"`, and `*` is technically a Star token
                        // but defensive value-filter covers both).
                        param_node.children.iter().find_map(|cc| match cc {
                            ASTNodeOrToken::Token(t)
                                if matches!(t.type_, TokenType::Name)
                                    && t.value != "*"
                                    && t.value != "**" =>
                            {
                                Some(Param {
                                    name: t.value.clone(),
                                    sir_type: None,
                                    span: self.span_of_token(t),
                                })
                            }
                            _ => None,
                        })
                    }
                    _ => None,
                })
                .collect()
        } else {
            Vec::new()
        };

        // Phase 6b: any non-empty parameter list means we'll emit
        // untyped Params (sir_type=None), which the SIR validator
        // requires `dynamic-typing` to be declared for.
        if !params.is_empty() {
            self.features_used.insert(Feature::DynamicTyping);
        }

        // Lower the body using a fresh locals + params scope so the
        // outer program's bindings don't leak into the method.
        // Parameters are pre-declared as "locals" so a re-assignment
        // to a param routes through `Stmt::Assign` (SIR-correct),
        // *and* are tracked in `current_params` so any `VarRef` to
        // them inside the body gets `Scope::Param` (validator-correct).
        let saved_locals = std::mem::take(&mut self.declared_locals);
        let saved_params = std::mem::take(&mut self.current_params);
        for p in &params {
            self.declared_locals.insert(p.name.clone());
            self.current_params.insert(p.name.clone());
        }

        // The body is every `statement` child of the def_statement
        // that *isn't* the method's own def_statement (we already
        // matched that), in source order.
        let body_stmts: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                _ => None,
            })
            .collect();

        let mut stmts_out: Vec<Stmt> = Vec::new();
        let mut value: Option<Expr> = None;
        if body_stmts.is_empty() {
            value = Some(Expr::NilLit {
                span: self.span_of(node),
            });
        } else {
            let last_idx = body_stmts.len() - 1;
            for (i, s) in body_stmts.iter().enumerate() {
                let inner = self.first_node_child(s).ok_or_else(|| {
                    RubyLowerError {
                        message: "statement node had no child rule".to_string(),
                        line: s.start_line.unwrap_or(0),
                        column: s.start_column.unwrap_or(0),
                    }
                })?;
                let is_tail = i == last_idx;
                let kind = inner.rule_name.as_str();
                if is_tail && matches!(kind, "expression_stmt" | "method_call" | "method_call_no_paren") {
                    let v = match kind {
                        "expression_stmt" => {
                            let expr_node =
                                self.first_node_child(inner).ok_or_else(|| {
                                    RubyLowerError {
                                        message:
                                            "expression_stmt had no expression child"
                                                .to_string(),
                                        line: inner.start_line.unwrap_or(0),
                                        column: inner.start_column.unwrap_or(0),
                                    }
                                })?;
                            self.lower_expression(expr_node)?
                        }
                        "method_call" | "method_call_no_paren" => self.lower_method_call(inner)?,
                        _ => unreachable!(),
                    };
                    value = Some(v);
                } else {
                    // Phase 6r — multi-stmt fan-out for `multi_assignment`.
                    stmts_out.extend(self.lower_statement_inner_multi(inner)?);
                }
            }
        }
        let value = value.unwrap_or(Expr::NilLit {
            span: self.span_of(node),
        });

        // Restore the outer scope's locals + params so the rest of
        // the program lowers correctly.
        self.declared_locals = saved_locals;
        self.current_params = saved_params;

        Ok(Function {
            name,
            params,
            return_type: None,
            captures: Vec::new(),
            body: Block {
                stmts: stmts_out,
                value,
                span: self.span_of(node),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: self.span_of(node),
        })
    }

    // -------------------------------------------------------------------
    // assignment → LetBinding (first) or Assign (subsequent)
    // -------------------------------------------------------------------

    fn lower_assignment(&mut self, node: &GrammarASTNode) -> Result<Stmt, RubyLowerError> {
        // Shape (post-6p): NAME ( EQUALS | "+=" | "-=" | "*=" | "/=" | "||=" | "&&=" ) expression
        let (name, name_span) = self.expect_first_name_token(node)?;
        let expr_node = self.find_node_child(node, "expression").ok_or_else(|| {
            RubyLowerError {
                message: "assignment missing RHS expression".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            }
        })?;
        let rhs = self.lower_expression(expr_node)?;

        // Phase 6p — detect compound-assign operator.  The lexer
        // pre-fuses `+=`, `-=`, `*=`, `/=`, `||=`, `&&=` into single
        // Name-typed tokens; here we read the operator token (skipping
        // the leading NAME) to dispatch.
        let op_token = node.children.iter().skip(1).find_map(|c| match c {
            ASTNodeOrToken::Token(t) => {
                let v = t.value.as_str();
                if matches!(v, "+=" | "-=" | "*=" | "/=" | "||=" | "&&=") {
                    Some(v.to_string())
                } else {
                    None
                }
            }
            _ => None,
        });

        let span = self.span_of(node);
        // Build the effective RHS.  For plain `=`, it's just `rhs`.
        // For compound forms, wrap it as `BuiltinCall(op, [VarRef(x), rhs])`
        // where `op` is the underlying binary operator (`+` for `+=`,
        // `or` for `||=`, etc.).  Lowering identically to
        // `x = x op rhs` keeps downstream emitters simple — no new
        // compound-assign-aware code paths required.
        let value = if let Some(op) = op_token.as_deref() {
            let (builtin_name, effects) = match op {
                "+=" => ("+", EffectSet::PURE),
                "-=" => ("-", EffectSet::PURE),
                "*=" => ("*", EffectSet::PURE),
                "/=" => ("/", EffectSet::PURE),
                "||=" => ("or", EffectSet::PURE),
                "&&=" => ("and", EffectSet::PURE),
                _ => unreachable!("op_token matched only the six compound forms above"),
            };
            let lhs_ref = Expr::VarRef {
                name: name.clone(),
                scope: Scope::Local,
                span: span.clone(),
            };
            Expr::BuiltinCall {
                name: builtin_name.to_string(),
                args: vec![lhs_ref, rhs],
                effects,
                span: span.clone(),
            }
        } else {
            rhs
        };

        // A compound assignment ALWAYS reads then re-binds, so it
        // must emit `Stmt::Assign` (never `LetBinding`).  Plain `=`
        // keeps the original "first sighting → LetBinding, subsequent
        // → Assign" behaviour.
        let is_compound = op_token.is_some();
        if is_compound || self.declared_locals.contains(&name) {
            // Re-bind path: mutable-bindings feature required.
            self.features_used.insert(Feature::MutableBindings);
            // Compound `x ||= 1` without a prior `x = …` is still
            // valid Ruby (treats `x` as nil), but we record it as a
            // local so any subsequent `x = 1` doesn't re-binding-error.
            self.declared_locals.insert(name.clone());
            Ok(Stmt::Assign {
                name,
                scope: Scope::Local,
                value,
                span,
            })
        } else {
            self.declared_locals.insert(name.clone());
            Ok(Stmt::LetBinding {
                name,
                sir_type: None,
                value,
                span,
            })
        }
        // `name_span` is intentionally unused for now — the SIR Stmt
        // span covers the whole statement.  Keeping the binding so
        // the lookup helper stays useful for callers that need it
        // (e.g. error messages).
        .map(|s| {
            let _ = name_span;
            s
        })
    }

    // -------------------------------------------------------------------
    // Phase 6r — multi-assignment
    // -------------------------------------------------------------------

    /// Lower a `multi_assignment` node (`a, b = 1, 2`) into one SIR
    /// statement per (LHS, RHS) pair.
    ///
    /// Grammar shape (per `ruby.grammar`):
    /// ```text
    /// multi_assignment = NAME COMMA NAME { COMMA NAME }
    ///                    EQUALS
    ///                    expression { COMMA expression } ;
    /// ```
    ///
    /// AST layout: the leading `NAME` tokens and their separator
    /// `COMMA`s sit before the `EQUALS` token; after `EQUALS` come
    /// the `expression` rule nodes (also `COMMA`-separated, but the
    /// `COMMA` between expressions is a Token child while the
    /// `expression` itself is a Node child).  We walk the children
    /// linearly: NAME tokens encountered *before* EQUALS form the LHS
    /// list; `expression` nodes encountered *after* EQUALS form the
    /// RHS list.
    ///
    /// Lowering rule for each `(lhs[i], rhs[i])` pair: identical to a
    /// plain `lhs[i] = rhs[i]` assignment —
    /// - First sighting of `lhs[i]` in this scope → `Stmt::LetBinding`.
    /// - Subsequent sighting → `Stmt::Assign` (and the lowerer marks
    ///   `Feature::MutableBindings`, same as the assignment lowerer).
    ///
    /// **v0 restrictions** (documented in `ruby.grammar` and the
    /// changelog):
    ///
    /// - LHS count must equal RHS count.  Mismatched arities are
    ///   rejected with a `RubyLowerError` rather than silently
    ///   padding with `nil` / discarding extras.  This keeps the v0
    ///   semantics unambiguous; the more permissive Ruby semantics
    ///   (excess LHS gets `nil`, excess RHS is dropped) ride with a
    ///   future phase.
    /// - Single-RHS auto-unpack `a, b = arr` is NOT supported (the
    ///   grammar requires at least one RHS expression but the
    ///   lowerer will reject the count mismatch).
    /// - Splat targets `a, *b = 1, 2, 3` ride with Phase 6s.
    fn lower_multi_assignment(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Vec<Stmt>, RubyLowerError> {
        // Walk children, partitioning at the EQUALS token.
        let mut saw_equals = false;
        let mut lhs_names: Vec<(String, Span)> = Vec::new();
        let mut rhs_exprs: Vec<&GrammarASTNode> = Vec::new();
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) => {
                    if t.type_ == TokenType::Equals {
                        saw_equals = true;
                    } else if !saw_equals && t.type_ == TokenType::Name {
                        // LHS name.  (COMMAs are also Token children
                        // but they're not Name-typed, so this branch
                        // skips them naturally.)
                        lhs_names.push((t.value.clone(), self.span_of_token(t)));
                    }
                    // Tokens after EQUALS (COMMAs between RHS
                    // expressions) are dropped — we only care about
                    // the Node children for the RHS list.
                }
                ASTNodeOrToken::Node(n) => {
                    if saw_equals && n.rule_name == "expression" {
                        rhs_exprs.push(n);
                    }
                }
            }
        }

        // Sanity: the grammar guarantees at least two LHS names and
        // at least one RHS, and an EQUALS token between them.  Defend
        // against pathological inputs anyway.
        if lhs_names.len() < 2 {
            return Err(RubyLowerError {
                message: format!(
                    "multi_assignment expected ≥2 LHS names, got {}",
                    lhs_names.len()
                ),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            });
        }
        if !saw_equals {
            return Err(RubyLowerError {
                message: "multi_assignment missing EQUALS token".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            });
        }
        if lhs_names.len() != rhs_exprs.len() {
            return Err(RubyLowerError {
                message: format!(
                    "multi_assignment v0 requires LHS count == RHS count \
                     (got {} LHS, {} RHS); splat / single-RHS auto-unpack \
                     not yet supported",
                    lhs_names.len(),
                    rhs_exprs.len(),
                ),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            });
        }

        // Lower each RHS first — this matches Ruby's evaluation order
        // (RHS is fully evaluated, *then* the LHS bindings happen).
        // For the parallel-binding case (`a, b = b, a` swap), Ruby
        // collects all RHS values *before* writing any LHS.  Our v0
        // sequential lowering (`a = expr0; b = expr1`) is equivalent
        // for the common case where no LHS appears in the RHS — which
        // is the only shape we test for v0.  The swap case
        // (`a, b = b, a`) would silently mis-lower under v0; this is
        // documented as a deferred limitation.
        let lowered_rhs: Vec<Expr> = rhs_exprs
            .iter()
            .map(|e| self.lower_expression(e))
            .collect::<Result<_, _>>()?;

        // Emit one Stmt per pair.  Reuses the same LetBinding/Assign
        // decision rule as `lower_assignment`.
        let mut out: Vec<Stmt> = Vec::with_capacity(lhs_names.len());
        for ((name, name_span), value) in lhs_names.into_iter().zip(lowered_rhs.into_iter()) {
            let span = name_span.clone();
            let stmt = if self.declared_locals.contains(&name) {
                self.features_used.insert(Feature::MutableBindings);
                Stmt::Assign {
                    name: name.clone(),
                    scope: Scope::Local,
                    value,
                    span,
                }
            } else {
                self.declared_locals.insert(name.clone());
                Stmt::LetBinding {
                    name: name.clone(),
                    sir_type: None,
                    value,
                    span,
                }
            };
            out.push(stmt);
        }
        Ok(out)
    }

    // -------------------------------------------------------------------
    // Phase 6v — `begin … rescue … ensure … end`
    // -------------------------------------------------------------------

    /// Lower a `begin_statement` node to a sequence of SIR statements.
    ///
    /// Grammar shape (per `ruby.grammar`):
    /// ```text
    /// begin_statement = "begin"
    ///                   { !"rescue" !"ensure" !"end" statement }
    ///                   { rescue_clause }
    ///                   [ ensure_clause ]
    ///                   "end" ;
    /// rescue_clause   = "rescue" [ exception_list ] [ "=>" NAME ]
    ///                        { !"rescue" !"ensure" !"end" statement } ;
    /// exception_list  = NAME { COMMA NAME } ;
    /// ensure_clause   = "ensure" { !"end" statement } ;
    /// ```
    ///
    /// **v0 lossy lowering** — SIR has no exception-handling primitive.
    /// We lower body, rescue, and ensure blocks INLINE (concatenated)
    /// with synthetic `BuiltinCall` markers bracketing each rescue and
    /// ensure section so downstream emitters can detect the form:
    ///
    /// ```text
    /// begin
    ///   body_stmts
    /// rescue StandardError, IOError => e
    ///   rescue_stmts
    /// ensure
    ///   ensure_stmts
    /// end
    /// ```
    ///
    /// → SIR sequence:
    ///
    /// ```text
    /// body_stmts...
    /// ExprStmt(BuiltinCall("__rescue_marker__", [
    ///     StrLit("StandardError,IOError"),
    ///     StrLit("e"),
    /// ]))
    /// rescue_stmts...
    /// ExprStmt(BuiltinCall("__ensure_marker__", []))
    /// ensure_stmts...
    /// ```
    ///
    /// Semantics: the body always runs.  Without a real try/catch
    /// primitive, the rescue body is unreachable in the SIR model
    /// (exceptions can't propagate through SIR's effect lattice in
    /// v0).  The ensure body runs unconditionally after the rescue
    /// section (same effect as a plain sequence under "no exception"
    /// semantics).  Downstream emitters that target languages with
    /// real exceptions can re-stitch the form via the marker
    /// `BuiltinCall`s; this is documented as a deferred limitation.
    ///
    /// `__rescue_marker__` and `__ensure_marker__` carry the
    /// `Effect::MayThrow` tag so the validator allows exception-aware
    /// programs without forcing the user to declare extra features.
    fn lower_begin_statement(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Vec<Stmt>, RubyLowerError> {
        let mut out: Vec<Stmt> = Vec::new();
        let outer_span = self.span_of(node);

        // 1. Lower the body statements (direct `statement` children of
        //    `begin_statement`).  The grammar's negative-lookahead
        //    repetition stops collection at the first rescue/ensure/end,
        //    so direct `statement` children are exactly the body.
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                if n.rule_name == "statement" {
                    if let Some(inner) = self.first_node_child(n) {
                        out.extend(self.lower_statement_inner_multi(inner)?);
                    }
                }
            }
        }

        // 2. Process each rescue_clause in order.
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                if n.rule_name == "rescue_clause" {
                    // Collect the exception type list (if present).
                    let exc_list = n
                        .children
                        .iter()
                        .find_map(|c| match c {
                            ASTNodeOrToken::Node(en)
                                if en.rule_name == "exception_list" =>
                            {
                                Some(en)
                            }
                            _ => None,
                        })
                        .map(|en| {
                            en.children
                                .iter()
                                .filter_map(|cc| match cc {
                                    ASTNodeOrToken::Token(t)
                                        if t.type_ == TokenType::Name =>
                                    {
                                        Some(t.value.as_str())
                                    }
                                    _ => None,
                                })
                                .collect::<Vec<_>>()
                                .join(",")
                        })
                        .unwrap_or_default();
                    // Find the exception variable name (after `=>`).
                    // The grammar `[ "=>" NAME ]` puts both inside the
                    // rescue_clause's direct token children.  Walk to
                    // find the `=>` token, then take the next Name.
                    let var_name: String = {
                        let mut saw_arrow = false;
                        let mut found: Option<String> = None;
                        for cc in &n.children {
                            if let ASTNodeOrToken::Token(t) = cc {
                                if t.value == "=>" {
                                    saw_arrow = true;
                                } else if saw_arrow
                                    && t.type_ == TokenType::Name
                                {
                                    found = Some(t.value.clone());
                                    break;
                                }
                            }
                        }
                        found.unwrap_or_default()
                    };
                    // Emit the marker.
                    out.push(Stmt::ExprStmt {
                        expr: Expr::BuiltinCall {
                            name: "__rescue_marker__".to_string(),
                            args: vec![
                                Expr::StrLit {
                                    value: exc_list,
                                    span: outer_span.clone(),
                                },
                                Expr::StrLit {
                                    value: var_name,
                                    span: outer_span.clone(),
                                },
                            ],
                            effects: EffectSet::PURE.with(Effect::MayThrow),
                            span: outer_span.clone(),
                        },
                        span: outer_span.clone(),
                    });
                    // Strings feature is required because we emit StrLits.
                    self.features_used.insert(Feature::Strings);
                    // Lower the rescue body's statements inline.
                    for cc in &n.children {
                        if let ASTNodeOrToken::Node(nn) = cc {
                            if nn.rule_name == "statement" {
                                if let Some(inner) = self.first_node_child(nn)
                                {
                                    out.extend(
                                        self.lower_statement_inner_multi(inner)?,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Process the optional ensure_clause.
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                if n.rule_name == "ensure_clause" {
                    out.push(Stmt::ExprStmt {
                        expr: Expr::BuiltinCall {
                            name: "__ensure_marker__".to_string(),
                            args: vec![],
                            effects: EffectSet::PURE.with(Effect::MayThrow),
                            span: outer_span.clone(),
                        },
                        span: outer_span.clone(),
                    });
                    for cc in &n.children {
                        if let ASTNodeOrToken::Node(nn) = cc {
                            if nn.rule_name == "statement" {
                                if let Some(inner) = self.first_node_child(nn)
                                {
                                    out.extend(
                                        self.lower_statement_inner_multi(inner)?,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(out)
    }

    // -------------------------------------------------------------------
    // method_call → BuiltinCall (recognised names) or DirectCall
    // -------------------------------------------------------------------

    fn lower_method_call(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // Shapes (Phase 6s-aware):
        //   method_call          = (NAME|KEYWORD) LPAREN [ call_arg
        //                          (COMMA call_arg)* ] RPAREN { dot_call }
        //   method_call_no_paren = (NAME|KEYWORD) expression
        //                          (COMMA expression)*
        //
        // The two shapes use *different* arg encodings: parenned calls
        // wrap each arg in a `call_arg` rule (which admits `*`/`**`
        // splat prefixes — Phase 6s); paren-less calls keep bare
        // `expression` children (the call_arg wrapper would create a
        // grammar ambiguity with binary `*` at expression-start
        // position — `a * b` as a statement would parse as `a(splat b)`,
        // which is wrong).  Paren-less splat (`puts *arr`) is therefore
        // a v0 deferred limitation; users who need it can fall back to
        // the parenned form `puts(*arr)`.
        //
        // Phase 6l: trailing `dot_call` repetitions chain method calls
        // onto the result.  Args before the first dot_call belong to
        // the head call; args inside each dot_call belong to that step.
        let (callee, _callee_span) = self.expect_first_name_token(node)?;
        let args: Vec<Expr> = if node.rule_name == "method_call_no_paren" {
            // Legacy shape: bare `expression` children directly.
            node.children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                    _ => None,
                })
                .map(|n| self.lower_expression(n))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            // Phase 6s shape: `call_arg` wrappers (with optional splat
            // prefix), collected only from the head call's prefix
            // siblings.
            self.head_call_args(node)
                .into_iter()
                .map(|n| self.lower_call_arg(n))
                .collect::<Result<Vec<_>, _>>()?
        };

        let span = self.span_of(node);
        let head: Expr = if let Some(effects) = ruby_builtin_effects(&callee) {
            Expr::BuiltinCall {
                name: callee,
                args,
                effects,
                span,
            }
        } else {
            // Unrecognised name — fall back to DirectCall.  SIR
            // backends that can't resolve the name will surface a
            // diagnostic; this keeps the lowering total (no panics).
            Expr::DirectCall {
                fn_name: callee,
                args,
                effects: EffectSet::PURE,
                span,
            }
        };
        // Phase 6l — apply trailing `.method[(...)]` chain steps, if any.
        self.apply_dot_chain(head, node)
    }

    /// Phase 6l+6s helper — return the `call_arg` Node children of
    /// `method_call` that belong to the *head* call (i.e. those that
    /// come before any `dot_call` child).  Without this guard, args
    /// nested inside `dot_call` subtrees would leak into the head call.
    ///
    /// Phase 6s renamed the prior `head_call_expression_children`:
    /// `method_call`'s grammar now wraps each arg in a `call_arg` rule
    /// (so splat/double-splat prefixes have a slot).  Callers route
    /// each returned `call_arg` through [`lower_call_arg`] to unwrap
    /// the `*` / `**` envelope.
    fn head_call_args<'a>(
        &self,
        node: &'a GrammarASTNode,
    ) -> Vec<&'a GrammarASTNode> {
        let mut out = Vec::new();
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                if n.rule_name == "dot_call" {
                    break;
                }
                if n.rule_name == "call_arg" {
                    out.push(n);
                }
            }
        }
        out
    }

    /// Phase 6s — lower a single `call_arg` node.
    ///
    /// Grammar shape: `call_arg = [ "*" | "**" ] expression ;`
    ///
    /// Lowering:
    /// - No prefix → return the lowered `expression` as-is.
    /// - `*` prefix → wrap in `BuiltinCall("splat", [inner])` — a
    ///   semantic marker that downstream emitters can detect to expand
    ///   into target-language variadic forwarding.
    /// - `**` prefix → wrap in `BuiltinCall("double_splat", [inner])`
    ///   — same pattern, for keyword-argument spread.
    ///
    /// The BuiltinCall envelope preserves splat semantics through SIR
    /// (where the lossy v0 Param shape can't represent variadic
    /// parameters directly).  Callers downstream can pattern-match the
    /// builtin name to convert back to splat syntax in target source.
    fn lower_call_arg(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Expr, RubyLowerError> {
        // Detect the leading `*` / `**` token (if present).  Both
        // forms land on Token children with their value preserved
        // (the 1.8-baseline state machine coalesces `**` into one
        // Name-typed Op token; `*` is a Star token).
        let prefix = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t)
                if matches!(t.value.as_str(), "*" | "**") =>
            {
                Some(t.value.clone())
            }
            _ => None,
        });
        let expr_node = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                _ => None,
            })
            .ok_or_else(|| RubyLowerError {
                message: "call_arg missing expression child".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;
        let inner = self.lower_expression(expr_node)?;
        let span = self.span_of(node);
        Ok(match prefix.as_deref() {
            Some("*") => Expr::BuiltinCall {
                name: "splat".to_string(),
                args: vec![inner],
                effects: EffectSet::PURE,
                span,
            },
            Some("**") => Expr::BuiltinCall {
                name: "double_splat".to_string(),
                args: vec![inner],
                effects: EffectSet::PURE,
                span,
            },
            _ => inner,
        })
    }

    // -------------------------------------------------------------------
    // Phase 6g — method-with-block lowering
    // -------------------------------------------------------------------

    /// Lower a `method_with_block` node into the SIR shape:
    /// the call itself plus a synthesised `Expr::MakeClosure`
    /// appended as the call's last argument.  Block body becomes a
    /// new top-level `Function` named `__block_<n>` on
    /// `self.user_functions`.
    ///
    /// v0 simplification: block bodies see only their own params
    /// (no captures from the outer scope).  Bodies that reference
    /// outer locals will fail the SIR validator at the `VarRef` stage.
    /// Documented in the crate CHANGELOG as a known limitation.
    fn lower_method_with_block(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Expr, RubyLowerError> {
        // Shape:
        //   (NAME | KEYWORD) [LPAREN [expression (COMMA expression)*] RPAREN] block
        // The leading callee name comes first.
        let (callee, _callee_span) = self.expect_first_name_token(node)?;

        // Collect explicit argument expressions (direct `expression`
        // children of the method_with_block node — *not* inside the
        // block).  The block node holds its own `statement` children;
        // we route around it.
        let args: Vec<Expr> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                _ => None,
            })
            .map(|n| self.lower_expression(n))
            .collect::<Result<Vec<_>, _>>()?;

        // Find the trailing `block` subnode and lower it to a hoisted
        // Function.  The Function's name is `__block_<n>` where `n`
        // monotonically counts every block we've lowered so far —
        // unique across the whole module.
        let block_node = self
            .find_node_child(node, "block")
            .ok_or_else(|| RubyLowerError {
                message: "method_with_block missing block subnode".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;
        let fn_name = self.hoist_block_to_function(block_node)?;

        // Append `MakeClosure` as the trailing arg.  Captures are
        // empty in v0 (block bodies that reference outer locals are
        // a known limitation).
        let make_closure = Expr::MakeClosure {
            fn_name: fn_name.clone(),
            captures: Vec::new(),
            span: self.span_of(block_node),
        };
        self.features_used.insert(Feature::Closures);

        let mut all_args = args;
        all_args.push(make_closure);

        let span = self.span_of(node);
        if let Some(effects) = ruby_builtin_effects(&callee) {
            Ok(Expr::BuiltinCall {
                name: callee,
                args: all_args,
                effects,
                span,
            })
        } else {
            Ok(Expr::DirectCall {
                fn_name: callee,
                args: all_args,
                effects: EffectSet::PURE,
                span,
            })
        }
    }

    /// Hoist a `block` (with one `do_block` or `brace_block` child)
    /// into a synthesised top-level Function on `user_functions`.
    /// Returns the synthesised function name so the caller can refer
    /// to it via `MakeClosure { fn_name }`.
    fn hoist_block_to_function(
        &mut self,
        block_node: &GrammarASTNode,
    ) -> Result<String, RubyLowerError> {
        // Drill into the do_block / brace_block child.
        let inner = self.first_node_child(block_node).ok_or_else(|| RubyLowerError {
            message: "block missing do_block/brace_block child".to_string(),
            line: block_node.start_line.unwrap_or(0),
            column: block_node.start_column.unwrap_or(0),
        })?;

        // Extract block parameters (the `|x, y|` pipe form).  Each
        // Name token *that isn't a `|`* is a parameter.  The lexer
        // classifies bare `|` ops as Name tokens (see
        // ruby-lexer/src/lib.rs::classify_op_token), so we filter
        // them out by value, not by type.
        let params_node = self.find_node_child(inner, "block_params");
        let params: Vec<Param> = match params_node {
            Some(pn) => pn
                .children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Token(t)
                        if matches!(t.type_, TokenType::Name) && t.value != "|" =>
                    {
                        Some(Param {
                            name: t.value.clone(),
                            sir_type: None,
                            span: self.span_of_token(t),
                        })
                    }
                    _ => None,
                })
                .collect(),
            None => Vec::new(),
        };
        // Block params are untyped → declare dynamic-typing.
        if !params.is_empty() {
            self.features_used.insert(Feature::DynamicTyping);
        }

        // Lower the body with a fresh locals+params scope so the
        // outer program's bindings don't leak in (same pattern as
        // `lower_def_statement`).
        let saved_locals = std::mem::take(&mut self.declared_locals);
        let saved_params = std::mem::take(&mut self.current_params);
        for p in &params {
            self.declared_locals.insert(p.name.clone());
            self.current_params.insert(p.name.clone());
        }

        // Body statements: every direct `statement` child of the
        // inner do_block / brace_block, in source order.  Tail-
        // expression promotion follows the same rule as
        // `lower_program` / `lower_clause_statements`.
        let body_stmts: Vec<&GrammarASTNode> = inner
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                _ => None,
            })
            .collect();
        let mut stmts_out: Vec<Stmt> = Vec::new();
        let mut value: Option<Expr> = None;
        if body_stmts.is_empty() {
            value = Some(Expr::NilLit {
                span: self.span_of(inner),
            });
        } else {
            let last_idx = body_stmts.len() - 1;
            for (i, s) in body_stmts.iter().enumerate() {
                let inner_stmt = self.first_node_child(s).ok_or_else(|| RubyLowerError {
                    message: "statement node had no child rule".to_string(),
                    line: s.start_line.unwrap_or(0),
                    column: s.start_column.unwrap_or(0),
                })?;
                let is_tail = i == last_idx;
                let kind = inner_stmt.rule_name.as_str();
                if is_tail && matches!(kind, "expression_stmt" | "method_call") {
                    let v = match kind {
                        "expression_stmt" => {
                            let en = self.first_node_child(inner_stmt).ok_or_else(|| {
                                RubyLowerError {
                                    message: "expression_stmt had no expression child".to_string(),
                                    line: inner_stmt.start_line.unwrap_or(0),
                                    column: inner_stmt.start_column.unwrap_or(0),
                                }
                            })?;
                            self.lower_expression(en)?
                        }
                        "method_call" => self.lower_method_call(inner_stmt)?,
                        _ => unreachable!(),
                    };
                    value = Some(v);
                } else {
                    // Phase 6r — multi-stmt fan-out for `multi_assignment`.
                    stmts_out.extend(self.lower_statement_inner_multi(inner_stmt)?);
                }
            }
        }
        let value = value.unwrap_or(Expr::NilLit {
            span: self.span_of(inner),
        });

        // Restore outer scope.
        self.declared_locals = saved_locals;
        self.current_params = saved_params;

        // Mint the synthetic function name and push the hoisted
        // Function onto user_functions.  Underscore-prefixed names
        // are conventionally treated as "compiler-generated" by SIR
        // backends — they should not collide with user-declared
        // identifiers.
        let n = self.block_counter;
        self.block_counter += 1;
        let fn_name = format!("__block_{n}");

        self.user_functions.push(Function {
            name: fn_name.clone(),
            params,
            return_type: None,
            captures: Vec::new(),
            body: Block {
                stmts: stmts_out,
                value,
                span: self.span_of(inner),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: self.span_of(block_node),
        });

        Ok(fn_name)
    }

    // -------------------------------------------------------------------
    // Phase 6w — arrow-lambda literal `->(params){body}`
    // -------------------------------------------------------------------

    /// Lower a `lambda_literal` node (`->(params){body}`) into a
    /// `BuiltinCall("lambda", [MakeClosure])` expression.
    ///
    /// Grammar shape (per `ruby.grammar`):
    /// ```text
    /// lambda_literal = "->" [ LPAREN [ params ] RPAREN ] block ;
    /// ```
    ///
    /// The body is hoisted to a top-level `Function` (named `__block_<n>`,
    /// reusing the same counter as `method_with_block` blocks).  Params
    /// are extracted from the leading `params` subnode (Phase 6s — splat
    /// supported) rather than from `block_params` (the `|x|` form inside
    /// the block), because in `->` syntax the parens-list IS the
    /// parameter list.
    ///
    /// **v0 deferred limitations**:
    /// - Block bodies that reference outer locals lose them — captures
    ///   are NOT computed for v0 (same limitation as Phase 6g blocks).
    /// - If the user writes both `->(x) { |y| … }` (params in parens
    ///   AND a block_params header), the latter is silently ignored;
    ///   only the parens-list is honoured.
    /// - `lambda { … }` and `proc { … }` continue to lower via
    ///   `method_with_block` — they're regular keyword-led calls.
    ///   The SIR builtin table tags both as `BuiltinCall("lambda", …)`
    ///   so downstream emitters see a single closure-construction shape.
    fn lower_lambda_literal(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Expr, RubyLowerError> {
        // 1. Find the `block` subnode (mandatory).
        let block_node = self
            .find_node_child(node, "block")
            .ok_or_else(|| RubyLowerError {
                message: "lambda_literal missing block subnode".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;

        // 2. Extract arrow-lambda params from the optional `params`
        //    subnode (Phase 6s: param = [ "*"|"**" ] NAME).
        let params_node = self.find_node_child(node, "params");
        let params: Vec<Param> = if let Some(pn) = params_node {
            pn.children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Node(param_node)
                        if param_node.rule_name == "param" =>
                    {
                        // Skip splat-prefix tokens; pick the bare NAME.
                        param_node.children.iter().find_map(|cc| match cc {
                            ASTNodeOrToken::Token(t)
                                if matches!(t.type_, TokenType::Name)
                                    && t.value != "*"
                                    && t.value != "**" =>
                            {
                                Some(Param {
                                    name: t.value.clone(),
                                    sir_type: None,
                                    span: self.span_of_token(t),
                                })
                            }
                            _ => None,
                        })
                    }
                    _ => None,
                })
                .collect()
        } else {
            Vec::new()
        };
        if !params.is_empty() {
            self.features_used.insert(Feature::DynamicTyping);
        }

        // 3. Hoist the block body to a Function with these params.
        //    Reuse the same machinery as `hoist_block_to_function` but
        //    using OUR `params` (from the parens-list), not the inner
        //    block's `block_params` pipe form.
        let fn_name = self.hoist_lambda_body(block_node, params)?;

        // 4. Emit BuiltinCall("lambda", [MakeClosure]).  Closures
        //    feature auto-set so the validator accepts MakeClosure.
        self.features_used.insert(Feature::Closures);
        let span = self.span_of(node);
        Ok(Expr::BuiltinCall {
            name: "lambda".to_string(),
            args: vec![Expr::MakeClosure {
                fn_name,
                captures: Vec::new(),
                span: span.clone(),
            }],
            effects: EffectSet::PURE,
            span,
        })
    }

    /// Phase 6w helper — hoist a `block` (do_block/brace_block) body
    /// to a top-level Function, taking the params list from the caller
    /// (the arrow lambda's parens-list) rather than from the block's
    /// own `|...|` `block_params` header.
    ///
    /// This is structurally parallel to `hoist_block_to_function` but
    /// with the params source swapped.  Returns the synthesised
    /// function name.
    fn hoist_lambda_body(
        &mut self,
        block_node: &GrammarASTNode,
        params: Vec<Param>,
    ) -> Result<String, RubyLowerError> {
        let inner = self.first_node_child(block_node).ok_or_else(|| RubyLowerError {
            message: "block missing do_block/brace_block child".to_string(),
            line: block_node.start_line.unwrap_or(0),
            column: block_node.start_column.unwrap_or(0),
        })?;

        // Lower body with fresh scope (params pre-declared as locals
        // + tracked in current_params so VarRefs get Scope::Param).
        let saved_locals = std::mem::take(&mut self.declared_locals);
        let saved_params = std::mem::take(&mut self.current_params);
        for p in &params {
            self.declared_locals.insert(p.name.clone());
            self.current_params.insert(p.name.clone());
        }

        // Body statements (same tail-expression promotion rule as the
        // method_with_block hoister).
        let body_stmts: Vec<&GrammarASTNode> = inner
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                _ => None,
            })
            .collect();
        let mut stmts_out: Vec<Stmt> = Vec::new();
        let mut value: Option<Expr> = None;
        if body_stmts.is_empty() {
            value = Some(Expr::NilLit {
                span: self.span_of(inner),
            });
        } else {
            let last_idx = body_stmts.len() - 1;
            for (i, s) in body_stmts.iter().enumerate() {
                let inner_stmt = self.first_node_child(s).ok_or_else(|| RubyLowerError {
                    message: "statement node had no child rule".to_string(),
                    line: s.start_line.unwrap_or(0),
                    column: s.start_column.unwrap_or(0),
                })?;
                let is_tail = i == last_idx;
                let kind = inner_stmt.rule_name.as_str();
                if is_tail && matches!(kind, "expression_stmt" | "method_call") {
                    let v = match kind {
                        "expression_stmt" => {
                            let en = self.first_node_child(inner_stmt).ok_or_else(|| {
                                RubyLowerError {
                                    message: "expression_stmt had no expression child"
                                        .to_string(),
                                    line: inner_stmt.start_line.unwrap_or(0),
                                    column: inner_stmt.start_column.unwrap_or(0),
                                }
                            })?;
                            self.lower_expression(en)?
                        }
                        "method_call" => self.lower_method_call(inner_stmt)?,
                        _ => unreachable!(),
                    };
                    value = Some(v);
                } else {
                    stmts_out.extend(self.lower_statement_inner_multi(inner_stmt)?);
                }
            }
        }
        let value = value.unwrap_or(Expr::NilLit {
            span: self.span_of(inner),
        });

        // Restore outer scope.
        self.declared_locals = saved_locals;
        self.current_params = saved_params;

        // Mint a synthetic function name (shares the same counter as
        // method_with_block-hoisted blocks).
        let n = self.block_counter;
        self.block_counter += 1;
        let fn_name = format!("__block_{n}");

        self.user_functions.push(Function {
            name: fn_name.clone(),
            params,
            return_type: None,
            captures: Vec::new(),
            body: Block {
                stmts: stmts_out,
                value,
                span: self.span_of(inner),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: self.span_of(block_node),
        });

        Ok(fn_name)
    }

    // -------------------------------------------------------------------
    // expression / term / factor
    // -------------------------------------------------------------------

    fn lower_expression(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // Pass through wrapper rules transparently — the parser
        // sometimes nests `expression → sum → term → factor → expression`.
        match node.rule_name.as_str() {
            // Phase 6m: `expression` is now the top of the logical
            // chain.  It contains exactly one child node — a
            // `logical_or`.  Pass through transparently.
            //
            // The comparison-op chain that used to live directly under
            // `expression` (pre-6m) has moved to the dedicated
            // `comparison` rule (lowered the same way the old
            // `expression` was — via `lower_comparison_chain`).
            "expression" => {
                let inner = self.first_node_child(node).ok_or_else(|| RubyLowerError {
                    message: "expression had no inner rule".to_string(),
                    line: node.start_line.unwrap_or(0),
                    column: node.start_column.unwrap_or(0),
                })?;
                self.lower_expression(inner)
            }
            // Phase 6m — logical-OR chain: `a || b || c || …`.
            // Folds left-associatively into nested
            // `BuiltinCall("or", [lhs, rhs])`.  Operator forms `||`
            // (symbol) and `or` (keyword) lower identically — see the
            // grammar comment for the v0 simplification.
            // Phase 6o — ternary `cond ? a : b`.  Either a bare
            // `range` pass-through or an `Expr::If` with single-expression
            // branch blocks.  Lowers identically to `if cond then a else b end`
            // so downstream emitters need no new code path.
            "ternary" => self.lower_ternary(node),
            // Phase 6n — range expressions `a..b` (inclusive) and
            // `a...b` (exclusive).  Either a bare `logical_or` pass-through
            // or a `BuiltinCall("range", [start, end, BoolLit(exclusive)])`.
            "range" => self.lower_range(node),
            "logical_or" => self.lower_logical_chain(node, &["||", "or"], "or"),
            // Phase 6m — logical-AND chain: same pattern as logical_or.
            "logical_and" => self.lower_logical_chain(node, &["&&", "and"], "and"),
            // Phase 6m — `logical_not`.  Two shapes:
            //   - prefix `!` or `not` → BuiltinCall("not", [inner])
            //   - bare passthrough to `comparison` (no leading op)
            "logical_not" => self.lower_logical_not(node),
            // Phase 6m — the comparison chain rule (renamed from the
            // old `expression`).  Same lowering as before.
            "comparison" => self.lower_comparison_chain(node),
            "sum" => self.lower_binary_chain(node, &["PLUS", "MINUS"]),
            "term" => self.lower_binary_chain(node, &["STAR", "SLASH"]),
            "factor" => self.lower_factor(node),
            "unary_minus" => {
                // Phase 6k — `-x` → BuiltinCall("neg", [x]).
                let inner = self.first_node_child(node).ok_or_else(|| RubyLowerError {
                    message: "unary_minus had no factor child".to_string(),
                    line: node.start_line.unwrap_or(0),
                    column: node.start_column.unwrap_or(0),
                })?;
                let operand = self.lower_expression(inner)?;
                Ok(Expr::BuiltinCall {
                    name: "neg".to_string(),
                    args: vec![operand],
                    effects: EffectSet::PURE,
                    span: self.span_of(node),
                })
            }
            "array_literal" => self.lower_array_literal(node),
            "hash_literal" => self.lower_hash_literal(node),
            "symbol_literal" => self.lower_symbol_literal(node),
            // Phase 6l — `method_call` may now appear in expression
            // position because it's the first atom alternative inside
            // `factor`.  Reuse the statement-level lowerer; it handles
            // the trailing `{ dot_call }` chain transparently.
            "method_call" => self.lower_method_call(node),
            // Phase 6w — arrow-lambda literal `->(params){body}`.
            "lambda_literal" => self.lower_lambda_literal(node),
            // The parser sometimes wraps a bare token into an "expression_stmt"
            // when reached as the RHS of an assignment.  Recurse into it.
            "expression_stmt" => {
                let inner = self.first_node_child(node).ok_or_else(|| RubyLowerError {
                    message: "expression_stmt had no inner expression".to_string(),
                    line: node.start_line.unwrap_or(0),
                    column: node.start_column.unwrap_or(0),
                })?;
                self.lower_expression(inner)
            }
            other => Err(RubyLowerError {
                message: format!("unsupported expression shape `{other}`"),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            }),
        }
    }

    /// Lower a left-associative chain of binary operators.  Used for
    /// both `expression` (PLUS / MINUS) and `term` (STAR / SLASH) —
    /// the only difference is the operator set.
    fn lower_binary_chain(
        &mut self,
        node: &GrammarASTNode,
        ops: &[&str],
    ) -> Result<Expr, RubyLowerError> {
        // Walk children in order.  The first must be a sub-expression
        // node; subsequent pairs are (op-token, sub-expression node).
        let mut acc: Option<Expr> = None;
        let mut pending_op: Option<(String, Span)> = None;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(sub) => {
                    let expr = self.lower_expression(sub)?;
                    acc = Some(match (acc.take(), pending_op.take()) {
                        (None, _) => expr,
                        (Some(lhs), Some((op_name, op_span))) => Expr::BuiltinCall {
                            name: op_name,
                            args: vec![lhs, expr],
                            effects: EffectSet::PURE,
                            span: op_span,
                        },
                        (Some(lhs), None) => {
                            // Two sibling sub-expressions with no operator
                            // between them — should not happen with the
                            // v0 grammar; treat as an internal error.
                            return Err(RubyLowerError {
                                message: "two consecutive expression children without an operator"
                                    .to_string(),
                                line: sub.start_line.unwrap_or(0),
                                column: sub.start_column.unwrap_or(0),
                            }
                            .also(lhs));
                        }
                    });
                }
                ASTNodeOrToken::Token(tok) => {
                    if ops.iter().any(|op| token_type_name(tok.type_) == *op) {
                        pending_op = Some((
                            token_lexeme_for_op(tok.type_).to_string(),
                            self.span_of_token(tok),
                        ));
                    }
                    // Other tokens (whitespace, newline) are dropped.
                }
            }
        }

        acc.ok_or_else(|| RubyLowerError {
            message: "binary chain had no operands".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })
    }

    /// Lower the `expression` rule's comparison-operator chain.
    /// Phase 6i — supports `==`, `!=`, `<`, `>`, `<=`, `>=` as
    /// left-associative BuiltinCalls.
    ///
    /// The lexer's `classify_op_token` reclassifies most comparison
    /// operators as `Name`-type tokens (its catch-all branch — only
    /// `==` gets a dedicated `EqualsEquals` type).  So we identify
    /// comparison operators by *value*, not by token type — the same
    /// trick used for `=>` in `hash_entry`.  This means the helper is
    /// resilient to the lexer's classifier changing in the future.
    fn lower_comparison_chain(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Expr, RubyLowerError> {
        const COMPARISON_OPS: &[&str] = &["==", "!=", "<", ">", "<=", ">="];
        let mut acc: Option<Expr> = None;
        let mut pending_op: Option<(String, Span)> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(sub) => {
                    let expr = self.lower_expression(sub)?;
                    acc = Some(match (acc.take(), pending_op.take()) {
                        (None, _) => expr,
                        (Some(lhs), Some((op_name, op_span))) => Expr::BuiltinCall {
                            name: op_name,
                            args: vec![lhs, expr],
                            effects: EffectSet::PURE,
                            span: op_span,
                        },
                        (Some(lhs), None) => {
                            return Err(RubyLowerError {
                                message:
                                    "two consecutive sum sub-expressions without a comparison \
                                     operator between them"
                                        .to_string(),
                                line: sub.start_line.unwrap_or(0),
                                column: sub.start_column.unwrap_or(0),
                            }
                            .also(lhs));
                        }
                    });
                }
                ASTNodeOrToken::Token(tok) => {
                    // Match by lexeme — covers both EqualsEquals
                    // (`==`) and Name-classified operators (`<`, `>`,
                    // `<=`, `>=`, `!=`).
                    if COMPARISON_OPS.iter().any(|op| *op == tok.value) {
                        pending_op = Some((tok.value.clone(), self.span_of_token(tok)));
                    }
                    // Whitespace/newline tokens fall through silently.
                }
            }
        }
        acc.ok_or_else(|| RubyLowerError {
            message: "comparison chain had no operands".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })
    }

    // -------------------------------------------------------------------
    // Phase 6m — logical operators `&&`, `||`, `and`, `or`, `!`, `not`
    // -------------------------------------------------------------------

    /// Lower a left-associative logical chain (`logical_or` /
    /// `logical_and`).  `op_lexemes` is the set of accepted operator
    /// lexemes (e.g. `["||", "or"]`).  `builtin_name` is the SIR
    /// builtin name to emit (e.g. `"or"`).  Both the symbol form
    /// (`||`) and keyword form (`or`) collapse to the same builtin —
    /// see the grammar comment for why v0 doesn't distinguish them.
    fn lower_logical_chain(
        &mut self,
        node: &GrammarASTNode,
        op_lexemes: &[&str],
        builtin_name: &str,
    ) -> Result<Expr, RubyLowerError> {
        let mut acc: Option<Expr> = None;
        let mut pending_op_span: Option<Span> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(sub) => {
                    let expr = self.lower_expression(sub)?;
                    acc = Some(match (acc.take(), pending_op_span.take()) {
                        (None, _) => expr,
                        (Some(lhs), Some(op_span)) => Expr::BuiltinCall {
                            name: builtin_name.to_string(),
                            args: vec![lhs, expr],
                            effects: EffectSet::PURE,
                            span: op_span,
                        },
                        (Some(_), None) => {
                            return Err(RubyLowerError {
                                message: format!(
                                    "logical chain had two consecutive operands without `{}`",
                                    op_lexemes.join("/")
                                ),
                                line: sub.start_line.unwrap_or(0),
                                column: sub.start_column.unwrap_or(0),
                            });
                        }
                    });
                }
                ASTNodeOrToken::Token(tok) => {
                    // Match operator by lexeme — `||`/`&&` lex as Name
                    // tokens (catch-all in classify_op_token), `and`/`or`
                    // lex as Keyword tokens.  Both reach us by value.
                    if op_lexemes.iter().any(|l| *l == tok.value) {
                        pending_op_span = Some(self.span_of_token(tok));
                    }
                }
            }
        }
        acc.ok_or_else(|| RubyLowerError {
            message: "logical chain had no operands".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })
    }

    /// Lower a `logical_not` node.  Shape: `{ "!" | "not" } comparison`.
    /// Each leading `!` or `not` wraps the inner expression in another
    /// `BuiltinCall("not", …)` layer — so `!!x` produces `not(not(x))`.
    fn lower_logical_not(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Expr, RubyLowerError> {
        let not_count = node
            .children
            .iter()
            .filter(|c| matches!(
                c,
                ASTNodeOrToken::Token(t) if t.value == "!" || t.value == "not"
            ))
            .count();
        // The single Node child is the inner `comparison` expression.
        let inner = self.first_node_child(node).ok_or_else(|| RubyLowerError {
            message: "logical_not had no inner expression".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })?;
        let mut expr = self.lower_expression(inner)?;
        // Wrap once per leading `!` / `not` token.
        for _ in 0..not_count {
            expr = Expr::BuiltinCall {
                name: "not".to_string(),
                args: vec![expr],
                effects: EffectSet::PURE,
                span: self.span_of(node),
            };
        }
        Ok(expr)
    }

    // -------------------------------------------------------------------
    // Phase 6n — range expressions `..` / `...`
    // -------------------------------------------------------------------

    /// Lower a `range` node.  Grammar shape:
    ///
    ///   range = logical_or [ ( "..." | ".." ) logical_or ]
    ///
    /// Two cases:
    ///   - One operand child, no `..`/`...` token → pass through (the
    ///     range rule is just a transparent wrapper in this case).
    ///   - Two operand children with a `..` or `...` token between them
    ///     → emit `BuiltinCall("range", [start, end, BoolLit(exclusive)])`.
    ///     The third argument carries the inclusive/exclusive flag so a
    ///     single builtin handles both forms without name multiplication.
    ///     `..` → exclusive=false; `...` → exclusive=true.
    ///
    /// Range is pure: building a range doesn't observe or mutate any
    /// state.  (Iterating over one *would* run code, but that's a
    /// separate call.)
    fn lower_range(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // Collect operand sub-nodes (each a `logical_or`).
        let operands: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                _ => None,
            })
            .collect();

        // Find the `..` or `...` operator token (if present).
        let op_tok = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if t.value == ".." || t.value == "..." => Some(t),
            _ => None,
        });

        match (operands.len(), op_tok) {
            // Bare logical_or pass-through — no range operator.
            (1, None) => self.lower_expression(operands[0]),
            // Two operands separated by `..` or `...`.
            (2, Some(tok)) => {
                let start = self.lower_expression(operands[0])?;
                let end = self.lower_expression(operands[1])?;
                let exclusive = tok.value == "...";
                let op_span = self.span_of_token(tok);
                Ok(Expr::BuiltinCall {
                    name: "range".to_string(),
                    args: vec![
                        start,
                        end,
                        // The third arg is a flag — `true` means
                        // exclusive (`...`), `false` means inclusive
                        // (`..`).  Carrying it as data keeps the
                        // builtin's signature uniform.
                        Expr::BoolLit { value: exclusive, span: op_span.clone() },
                    ],
                    effects: EffectSet::PURE,
                    span: op_span,
                })
            }
            // Shouldn't happen given the grammar shape — but be
            // defensive: a missing operator with two operands or a
            // present operator with the wrong operand count points
            // at a grammar regeneration gone awry.
            (n, _) => Err(RubyLowerError {
                message: format!(
                    "range node had {n} operand(s) and op={:?} — expected (1, None) or (2, Some(..|...))",
                    op_tok.map(|t| t.value.clone()),
                ),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            }),
        }
    }

    // -------------------------------------------------------------------
    // Phase 6o — ternary `cond ? a : b`
    // -------------------------------------------------------------------

    /// Lower a `ternary` node.  Grammar shape:
    ///
    ///   ternary = range [ "?" expression ":" expression ]
    ///
    /// Two cases:
    ///   - One operand sub-node (just a `range`, no `?`) → pass through.
    ///   - Three operand sub-nodes (`range "?" expression ":" expression`)
    ///     → emit `Expr::If` wrapping each branch in a single-expression
    ///     `Block`.  Lowers identically to `if cond then a else b end`
    ///     so all downstream emitters (semantic-ir-to-python, etc.) reuse
    ///     existing if-lowering code paths.
    ///
    /// Right-associativity (`a ? b : c ? d : e` → `a ? b : (c ? d : e)`)
    /// is enforced by the grammar's recursion into `expression` for the
    /// false branch.  Each `expression` recursion bottoms back out at
    /// `ternary` at the top of the precedence pyramid, so the inner
    /// ternary appears as the else-branch's value.
    fn lower_ternary(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // Collect operand sub-nodes (each is an `expression`-shaped
        // subtree: the first is `range`, the trailing two are
        // `expression`).
        let operands: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                _ => None,
            })
            .collect();

        match operands.len() {
            // Bare range pass-through — no `?` operator.
            1 => self.lower_expression(operands[0]),
            // cond ? then : else — three operand sub-nodes.
            3 => {
                let cond = self.lower_expression(operands[0])?;
                let then_value = self.lower_expression(operands[1])?;
                let else_value = self.lower_expression(operands[2])?;
                let span = self.span_of(node);
                let then_block = Block {
                    stmts: Vec::new(),
                    value: then_value,
                    span: span.clone(),
                };
                let else_block = Block {
                    stmts: Vec::new(),
                    value: else_value,
                    span: span.clone(),
                };
                Ok(Expr::If {
                    cond: Box::new(cond),
                    then_branch: Box::new(then_block),
                    else_branch: Box::new(else_block),
                    span,
                })
            }
            n => Err(RubyLowerError {
                message: format!(
                    "ternary node had {n} operand sub-node(s) — expected 1 (pass-through) or 3 (cond/then/else)",
                ),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            }),
        }
    }

    /// Lower a `factor` node — the leaves of the expression tree.
    /// Phase 6e — `:foo` / `:"bar"` → `Expr::SymLit`.  The leading
    /// COLON is the syntactic marker; the symbol's *name* is the
    /// Name/Keyword/String token that follows.  For quoted forms
    /// (`:"hello world"`) the String token's value already strips
    /// the surrounding quotes, so we use it verbatim.
    fn lower_symbol_literal(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Expr, RubyLowerError> {
        let name_tok = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t)
                if matches!(
                    t.type_,
                    TokenType::Name | TokenType::Keyword | TokenType::String
                ) =>
            {
                Some(t)
            }
            _ => None,
        });
        let name_tok = name_tok.ok_or_else(|| RubyLowerError {
            message: "symbol_literal missing payload token".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })?;
        self.features_used.insert(Feature::Symbols);
        Ok(Expr::SymLit {
            name: name_tok.value.clone(),
            span: self.span_of(node),
        })
    }

    /// Phase 6d — `[a, b, c]` → `Expr::SeqLit`.
    fn lower_array_literal(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Expr, RubyLowerError> {
        let items: Vec<Expr> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                _ => None,
            })
            .map(|n| self.lower_expression(n))
            .collect::<Result<Vec<_>, _>>()?;
        // SIR's SeqLit allocates a runtime list — declare the
        // `sequences` feature so the validator accepts it.
        self.features_used.insert(Feature::Sequences);
        Ok(Expr::SeqLit {
            items,
            span: self.span_of(node),
        })
    }

    /// Phase 6d — `{a: 1, b => 2}` → `Expr::MapLit`.  Both the
    /// `NAME COLON expression` shorthand and the `expression => expression`
    /// hash-rocket form lower to the same node — the key becomes a
    /// `SymLit` for the shorthand (since `a:` is sugar for `:a =>`)
    /// or whatever the LHS expression evaluates to for the rocket
    /// form.
    fn lower_hash_literal(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Expr, RubyLowerError> {
        let entry_nodes: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "hash_entry" => Some(n),
                _ => None,
            })
            .collect();
        let mut entries: Vec<semantic_ir::nodes::MapEntry> = Vec::with_capacity(entry_nodes.len());
        for ent in &entry_nodes {
            entries.push(self.lower_hash_entry(ent)?);
        }
        self.features_used.insert(Feature::Maps);
        Ok(Expr::MapLit {
            entries,
            span: self.span_of(node),
        })
    }

    fn lower_hash_entry(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<semantic_ir::nodes::MapEntry, RubyLowerError> {
        // Two shapes are possible:
        //   1. `NAME COLON expression` — shorthand.  The Name token
        //      is the symbol key.
        //   2. `expression "=>" expression` — hash-rocket.  Two
        //      `expression` rule children.
        let expression_subnodes: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                _ => None,
            })
            .collect();
        if expression_subnodes.len() == 2 {
            // Rocket form.
            let key = self.lower_expression(expression_subnodes[0])?;
            let value = self.lower_expression(expression_subnodes[1])?;
            return Ok(semantic_ir::nodes::MapEntry { key, value });
        }
        // Shorthand form — find the leading Name token.
        let key_tok = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => Some(t),
            _ => None,
        });
        let key_tok = key_tok.ok_or_else(|| RubyLowerError {
            message: "hash_entry missing key Name token".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })?;
        let key = Expr::SymLit {
            name: key_tok.value.clone(),
            span: self.span_of_token(key_tok),
        };
        self.features_used.insert(Feature::Symbols);
        let value_node = expression_subnodes.first().ok_or_else(|| RubyLowerError {
            message: "hash_entry shorthand missing value expression".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })?;
        let value = self.lower_expression(value_node)?;
        Ok(semantic_ir::nodes::MapEntry { key, value })
    }

    // -------------------------------------------------------------------
    // Phase 7a — backtick command literal lowering
    // -------------------------------------------------------------------

    /// Lower a Ruby backtick command literal `` `cmd args` `` (Phase 7a).
    ///
    /// The lexer's Phase-4m `backtick_body` state emits the entire
    /// literal — including the surrounding backticks — as a single
    /// `TokenType::String` token whose `value` is `` `<body>` `` (the
    /// inner body wrapped back up).  This sentinel-by-prefix trick
    /// lets the parser route both plain strings and backtick literals
    /// through the same NUMBER/STRING/NAME factor alternation while
    /// preserving the distinction for the lowerer.
    ///
    /// ## SIR shape
    ///
    /// `BuiltinCall { name: "backtick", args: [StrLit(body)] }` — the
    /// inner body (backticks stripped) is carried as a `StrLit` arg.
    /// Effects are `MayBlock | MayPrint | MayThrow`: command execution
    /// can block (waiting for the child process), print (stdout/stderr
    /// from the child), and throw (`Errno::ENOENT` and friends).  Same
    /// marker-builtin pattern as Phase 6v's `__rescue_marker__`,
    /// Phase 6w's lambda construction, and Phase 6y's `__interp__`.
    ///
    /// ## v0 deferred limitations
    ///
    /// - Interpolation inside the body (`` `echo #{name}` ``) is NOT
    ///   split.  The body is emitted as a single `StrLit` with any
    ///   `#{...}` markers preserved verbatim.  A future phase will
    ///   reuse the Phase 6y interpolation splitter inside the body.
    /// - Escape sequences inside the body (`` \` ``, `\n`, etc.) are
    ///   resolved by the lexer (Phase 4m's body state) before reaching
    ///   us — we don't re-process them here.
    /// - Triggers `Feature::Strings` because we emit a `StrLit`.
    fn lower_backtick_command_literal(&mut self, raw: &str, span: Span) -> Expr {
        // Strip the surrounding backticks.  The lexer guarantees the
        // value is `` `<body>` ``, so the first and last bytes are
        // always ASCII `` ` `` (single-byte) — we can slice on bytes.
        // Defensive fallback: if either delimiter is missing (which
        // would be a lexer bug), treat the whole value as the body so
        // we don't panic on a malformed input.
        let body = if raw.len() >= 2
            && raw.starts_with('`')
            && raw.ends_with('`')
        {
            &raw[1..raw.len() - 1]
        } else {
            raw
        };
        self.features_used.insert(Feature::Strings);
        Expr::BuiltinCall {
            name: "backtick".to_string(),
            args: vec![Expr::StrLit {
                value: body.to_string(),
                span: span.clone(),
            }],
            // Backtick execution can block on the child process, print
            // its output, and throw if the command can't be invoked
            // (`Errno::ENOENT`, etc).
            effects: EffectSet::PURE
                .with(Effect::MayBlock)
                .with(Effect::MayPrint)
                .with(Effect::MayThrow),
            span,
        }
    }

    // -------------------------------------------------------------------
    // Phase 6z — numeric literal lowering (float / hex / bin / oct / dec)
    // -------------------------------------------------------------------

    /// Lower a Ruby numeric literal token (Phase 6z).
    ///
    /// The lexer's Phase-4k / Phase-4l post-passes fuse the source-level
    /// shapes below into a single `TokenType::Number` token whose value
    /// is the verbatim source text (with underscore separators preserved).
    /// This routine dispatches on the shape:
    ///
    /// | Source       | SIR shape                                |
    /// |--------------|------------------------------------------|
    /// | `42`         | `IntLit { value: 42 }`                   |
    /// | `1_000_000`  | `IntLit { value: 1000000 }`              |
    /// | `0x1F`       | `IntLit { value: 31 }` (radix 16)        |
    /// | `0xDEAD_BEEF`| `IntLit { value: 3735928559 }`           |
    /// | `0b1010`     | `IntLit { value: 10 }` (radix 2)         |
    /// | `0o17`       | `IntLit { value: 15 }` (radix 8)         |
    /// | `0d42`       | `IntLit { value: 42 }` (radix 10 explicit) |
    /// | `1.5`        | `FloatLit { value: 1.5 }`                |
    /// | `1e10`       | `FloatLit { value: 1e10 }`               |
    /// | `1.5e-3`     | `FloatLit { value: 0.0015 }`             |
    ///
    /// Float detection is a single pass over the cleaned (underscore-
    /// stripped) value: if `.` or `e` / `E` is present **anywhere**,
    /// the literal is a float; otherwise it's an integer.  Radix
    /// detection requires both the leading `0` *and* a radix-prefix
    /// letter as the second character.  These two checks are mutually
    /// exclusive in the Ruby grammar (radix prefixes start with a
    /// letter, floats start with a digit run + `.` / `e`), so the
    /// dispatch order doesn't matter — we test radix first because
    /// it's the cheaper check.
    ///
    /// ## v0 deferred
    ///
    /// - Ruby's `r` / `i` numeric suffixes (Rational / Complex, lexed
    ///   by Phase 4f) are still kept on the token as a trailing letter;
    ///   the lowerer currently rejects those, since SIR has no
    ///   Rational / Complex types.  A future phase will route those
    ///   into `BuiltinCall("rational", [...])` / `BuiltinCall("complex", [...])`
    ///   markers.
    /// - Negative literals are still handled by the unary-minus path
    ///   (Phase 6k); this routine sees only the magnitude.
    fn lower_numeric_literal(
        &mut self,
        raw: &str,
        span: Span,
        err_line: usize,
        err_column: usize,
    ) -> Result<Expr, RubyLowerError> {
        // Step 1: strip Ruby's `_` digit separators.  They're purely
        // cosmetic (Ruby allows `1_000_000` to mean `1000000`).  We
        // do this *before* the shape dispatch so both the float-parse
        // and the radix-parse see clean digit strings.
        let cleaned: String = raw.chars().filter(|c| *c != '_').collect();

        // Step 2: radix-prefix detection (Phase 4l).  A Ruby radix
        // literal is `0` followed by a radix letter then the digits:
        //   0x | 0X  -> base 16
        //   0b | 0B  -> base  2
        //   0o | 0O  -> base  8
        //   0d | 0D  -> base 10 (explicit decimal)
        // Anything else starting with `0` is plain decimal (e.g. `0`,
        // `017` would be Ruby's legacy octal — not supported in v0).
        let bytes = cleaned.as_bytes();
        if bytes.len() >= 3 && bytes[0] == b'0' {
            let (radix, body_start): (u32, usize) = match bytes[1] {
                b'x' | b'X' => (16, 2),
                b'b' | b'B' => (2, 2),
                b'o' | b'O' => (8, 2),
                b'd' | b'D' => (10, 2),
                _ => (0, 0),
            };
            if radix != 0 {
                let body = &cleaned[body_start..];
                // `i64::from_str_radix` rejects empty strings and bad
                // digits — both of which would already be lexer bugs
                // here, so propagate the error rather than panicking.
                let v = i64::from_str_radix(body, radix).map_err(|_| RubyLowerError {
                    message: format!("invalid radix-{} integer literal `{}`", radix, raw),
                    line: err_line,
                    column: err_column,
                })?;
                return Ok(Expr::IntLit { value: v, span });
            }
        }

        // Step 3: float detection (Phase 4k).  A Ruby float literal has
        // either a fractional part (`.` followed by digit) OR an
        // exponent (`e` / `E`).  Both can appear together (`1.5e-3`).
        // We use `contains` rather than `starts_with` because the dot
        // / exponent can appear anywhere in the body.
        //
        // Note we cannot use a bare `.` check because the lexer's
        // float fusion already guarantees the dot is between digits;
        // we don't need to re-validate that here.
        let has_fraction = cleaned.contains('.');
        let has_exponent = cleaned.contains(['e', 'E']);
        if has_fraction || has_exponent {
            self.features_used.insert(Feature::Floats);
            let v: f64 = cleaned.parse().map_err(|_| RubyLowerError {
                message: format!("invalid float literal `{}`", raw),
                line: err_line,
                column: err_column,
            })?;
            return Ok(Expr::FloatLit { value: v, span });
        }

        // Step 4: plain decimal integer (pre-Phase-6z behaviour).
        let v: i64 = cleaned.parse().map_err(|_| RubyLowerError {
            message: format!("invalid integer literal `{}`", raw),
            line: err_line,
            column: err_column,
        })?;
        Ok(Expr::IntLit { value: v, span })
    }

    // -------------------------------------------------------------------
    // Phase 6y — string interpolation lowering
    // -------------------------------------------------------------------

    /// Lower a Ruby string literal whose raw content may contain
    /// `#{...}` interpolation markers (Phase 6y).
    ///
    /// The lexer's Phase-3b state machine captures `"foo#{x}bar"` as a
    /// single `TokenType::String` token whose `value` is the inner
    /// content with the `#{...}` markers preserved verbatim and any
    /// `{` / `}` inside the interpolation already brace-balanced by
    /// the lexer's `interp_brace_depth` tracking.
    ///
    /// ## Split strategy
    ///
    /// Walk the content char-by-char.  When we hit `#{`, flush the
    /// accumulated literal text as a `StrLit` segment, then scan the
    /// interpolation body up to the matching `}` (tracking brace depth
    /// so `#{ {a: 1} }` works).  Each interpolation body lowers via
    /// [`lower_interp_expression`] — bare identifiers route to
    /// `VarRef`, anything else lowers as a `BuiltinCall("__interp__",
    /// [StrLit(raw)])` marker.  This matches the marker pattern used
    /// by Phase 6v rescue/ensure.
    ///
    /// ## Output shapes
    ///
    /// | Source              | Lowered SIR shape                                                              |
    /// |---------------------|--------------------------------------------------------------------------------|
    /// | `"plain"`           | `StrLit("plain")`                                                              |
    /// | `"#{x}"`            | `VarRef("x")` — single non-literal segment, no wrapper                         |
    /// | `"hi #{name}"`      | `BuiltinCall("string_concat", [StrLit("hi "), VarRef("name")])`                |
    /// | `"#{a}#{b}"`        | `BuiltinCall("string_concat", [VarRef("a"), VarRef("b")])`                     |
    /// | `"sum is #{1+2}"`   | `BuiltinCall("string_concat", [StrLit("sum is "), BuiltinCall("__interp__", [StrLit("1+2")])])` |
    ///
    /// ## v0 deferred
    ///
    /// - Complex interpolation expressions are kept as the `__interp__`
    ///   marker carrying the raw source text rather than being recursively
    ///   parsed; downstream Ruby emitters can still reconstruct the
    ///   original literal verbatim from the marker.  A future phase
    ///   will recursively invoke the Ruby parser/lowerer on the body so
    ///   the SIR carries proper semantic info.
    /// - Escape sequences inside the literal (`\n`, `\t`, `\\`, `\"`)
    ///   pass through unchanged — the lexer hasn't unescaped them yet.
    fn lower_string_literal_with_interp(
        &mut self,
        raw: &str,
        span: Span,
        err_line: usize,
        err_column: usize,
    ) -> Result<Expr, RubyLowerError> {
        let mut segments: Vec<Expr> = Vec::new();
        let mut text_buf = String::new();
        let mut chars = raw.char_indices().peekable();

        while let Some((_, ch)) = chars.next() {
            // Detect the `#{` interpolation opener — only when `#` is
            // immediately followed by `{`.  Bare `#` inside a string
            // (e.g. `"a#b"`) is just a literal character.
            if ch == '#' {
                if let Some(&(_, '{')) = chars.peek() {
                    // Consume the `{`.
                    chars.next();
                    // Flush whatever literal text we've accumulated so
                    // far as its own `StrLit` segment.  We do not push
                    // empty segments (saves allocations and keeps the
                    // emitted SIR clean for `"#{a}"`-style strings).
                    if !text_buf.is_empty() {
                        segments.push(Expr::StrLit {
                            value: std::mem::take(&mut text_buf),
                            span: span.clone(),
                        });
                    }
                    // Scan up to the matching closing `}`, tracking
                    // brace depth so nested `{...}` (e.g. inline hash
                    // or block in the interp) is balanced correctly.
                    let mut depth: usize = 1;
                    let mut interp = String::new();
                    let mut terminated = false;
                    for (_, c) in chars.by_ref() {
                        match c {
                            '{' => {
                                depth += 1;
                                interp.push(c);
                            }
                            '}' => {
                                depth -= 1;
                                if depth == 0 {
                                    terminated = true;
                                    break;
                                }
                                interp.push(c);
                            }
                            other => interp.push(other),
                        }
                    }
                    if !terminated {
                        // Defensive: the lexer's Phase-3b state machine
                        // would have rejected an unterminated `#{...`,
                        // but propagate as a lower-error rather than
                        // panicking if it ever slips through.
                        return Err(RubyLowerError {
                            message: format!(
                                "unterminated `#{{...` interpolation in string literal `\"{}\"`",
                                raw
                            ),
                            line: err_line,
                            column: err_column,
                        });
                    }
                    segments.push(self.lower_interp_expression(&interp, span.clone()));
                    continue;
                }
            }
            // Ordinary literal character.  push() copies one full
            // UTF-8 char (not a byte) so multi-byte content stays
            // intact.
            text_buf.push(ch);
        }
        // Flush any trailing literal text after the last interp.
        if !text_buf.is_empty() {
            segments.push(Expr::StrLit {
                value: text_buf,
                span: span.clone(),
            });
        }

        // Result-shape selection:
        // - Empty string literal (`""`): emit a single empty `StrLit`.
        // - Exactly one segment: hand it back directly (no concat
        //   wrapper needed — keeps `"plain"` and `"#{x}"` lean).
        // - Two or more segments: wrap in a `string_concat` builtin.
        //
        // Any path that emits one or more segments needs the `Strings`
        // feature flag because we're producing `StrLit` data.
        if segments.is_empty() {
            return Ok(Expr::StrLit {
                value: String::new(),
                span,
            });
        }
        self.features_used.insert(Feature::Strings);
        if segments.len() == 1 {
            return Ok(segments.into_iter().next().unwrap());
        }
        Ok(Expr::BuiltinCall {
            name: "string_concat".to_string(),
            args: segments,
            effects: EffectSet::PURE,
            span,
        })
    }

    /// Lower the body of a single `#{...}` interpolation segment
    /// (Phase 6y).
    ///
    /// v0 fast path: a bare identifier (no whitespace, no operators,
    /// no sigils) routes to `VarRef` with the same `Scope::Param` /
    /// `Scope::Local` dispatch as the regular factor-atom Name case.
    /// This covers the overwhelmingly common shape `"hello #{name}"`.
    ///
    /// v0 fallback: anything else — arithmetic, method calls, nested
    /// strings, sigil vars, etc. — lowers as a single marker
    /// `BuiltinCall("__interp__", [StrLit(raw_body)])`.  Downstream
    /// emitters that target Ruby can re-emit the marker as `#{<raw>}`
    /// verbatim; emitters that target other languages can flag the
    /// marker as a TODO for a future phase that re-parses the body.
    ///
    /// Same marker pattern as Phase 6v's `__rescue_marker__` /
    /// `__ensure_marker__` — a known-name `BuiltinCall` whose arg
    /// list carries the verbatim source text.
    fn lower_interp_expression(&mut self, raw: &str, span: Span) -> Expr {
        let trimmed = raw.trim();
        // Bare-identifier check: starts with `_` or ASCII letter,
        // and every following char is `_`/letter/digit.  We
        // intentionally reject sigil vars (`@x`, `$x`, `@@x`) here
        // because the Phase 6x routing happens at lex time, not at
        // interp-split time — those would need their own special
        // handling in a follow-up phase.
        let mut chars_iter = trimmed.chars();
        let is_bare_name = match chars_iter.next() {
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {
                chars_iter.all(|c| c.is_ascii_alphanumeric() || c == '_')
            }
            _ => false,
        };
        if is_bare_name {
            let scope = if self.current_params.contains(&trimmed.to_string()) {
                Scope::Param
            } else {
                Scope::Local
            };
            return Expr::VarRef {
                name: trimmed.to_string(),
                scope,
                span,
            };
        }
        // Fallback marker.  Triggers `Strings` because we embed the
        // raw text as a `StrLit`.
        self.features_used.insert(Feature::Strings);
        Expr::BuiltinCall {
            name: "__interp__".to_string(),
            args: vec![Expr::StrLit {
                value: raw.to_string(),
                span: span.clone(),
            }],
            effects: EffectSet::PURE,
            span,
        }
    }

    fn lower_factor(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // factor ::= ( atom ) { dot_call }
        //
        // Phase 6l — method receiver chains.  The atom is followed by
        // zero or more `dot_call` Node children (`.method[(args)]`).
        // We extract the atom first, then wrap it once per dot_call.
        //
        // Atom alternatives: NUMBER | STRING | NAME | KEYWORD |
        //   symbol_literal | array_literal | hash_literal |
        //   LPAREN expression RPAREN | unary_minus
        let atom = self.lower_factor_atom(node)?;
        self.apply_dot_chain(atom, node)
    }

    /// Extract the atom expression from a `factor` node, ignoring
    /// trailing `dot_call` Node children.  This is the pre-Phase-6l
    /// lowering logic, refactored into its own helper so that
    /// `lower_factor` can apply the dot-chain postfix on top.
    fn lower_factor_atom(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let span = self.span_of_token(tok);
                    match tok.type_ {
                        TokenType::Number => {
                            // Phase 6z — float / hex / bin / oct / decimal-explicit
                            // integer literal parsing.  The lexer (Phase 4k / 4l)
                            // fuses these into a single `Number` token whose
                            // value carries the verbatim source text.  The
                            // parser sees them all uniformly at the factor
                            // atom position — no grammar changes needed.
                            return self.lower_numeric_literal(
                                &tok.value,
                                span,
                                tok.line,
                                tok.column,
                            );
                        }
                        TokenType::String => {
                            // Phase 7a — backtick command literal dispatch.
                            // The lexer (Phase 4m) emits `` `cmd args` `` as
                            // a `String` token whose value is the verbatim
                            // source *including* the surrounding backticks
                            // — same lexeme-prefix sentinel trick the
                            // percent literals and heredocs use.  Detect by
                            // checking the leading byte.
                            if tok.value.starts_with('`') {
                                return Ok(self.lower_backtick_command_literal(
                                    &tok.value,
                                    span,
                                ));
                            }
                            // Phase 6y — string interpolation expression
                            // lowering.  The lexer (Phase 3b) emits the
                            // entire `"foo#{x}bar"` literal as a single
                            // `String` token whose `value` holds the inner
                            // content with `#{...}` markers preserved
                            // verbatim.  When markers are present we split
                            // into segments and emit a concat builtin;
                            // when absent we fall through to a plain
                            // `StrLit` (zero-cost fast path).
                            return self.lower_string_literal_with_interp(
                                &tok.value,
                                span,
                                tok.line,
                                tok.column,
                            );
                        }
                        TokenType::Name => {
                            // Inside a function body, parameter
                            // names lex as `VarRef` with
                            // `Scope::Param` so the SIR validator
                            // can verify they bind to a `Param`
                            // declaration.  At the top level
                            // (main) the params set is empty and
                            // every name falls through to
                            // `Scope::Local`.
                            //
                            // Phase 6x — Ruby sigil-prefixed variable refs
                            // (`@x` ivar, `@@x` cvar, `$x` gvar) come through
                            // as Name-typed tokens with the sigil preserved
                            // in `value` (the lexer's Phase-4i/4j states
                            // build a single-token form).
                            //
                            // v0 SIR limitation: there is no dedicated IVar /
                            // CVar / GVar scope.  Using `Scope::Global` for
                            // `$x` would require a matching `Global` decl on
                            // the module (the validator enforces this); we
                            // skip the auto-declaration and put all sigil
                            // vars on `Scope::Local` instead.  The leading
                            // sigil stays in the bound name, so downstream
                            // emitters that target Ruby (or any language
                            // with similar lookup) can detect the sigil and
                            // route the assignment / read appropriately.
                            //
                            // Documented as a deferred limitation; a follow-
                            // up phase will (a) add IVar/CVar scopes to SIR
                            // and/or (b) auto-emit `Global` declarations for
                            // `$x`-prefixed names so the validator-true
                            // mapping `$x` → `Scope::Global` becomes usable.
                            let scope = if self.current_params.contains(&tok.value) {
                                Scope::Param
                            } else {
                                Scope::Local
                            };
                            return Ok(Expr::VarRef {
                                name: tok.value.clone(),
                                scope,
                                span,
                            });
                        }
                        TokenType::Keyword => match tok.value.as_str() {
                            "nil" => return Ok(Expr::NilLit { span }),
                            "true" => return Ok(Expr::BoolLit { value: true, span }),
                            "false" => return Ok(Expr::BoolLit { value: false, span }),
                            _ => {
                                // Any other keyword used in factor position
                                // is an error in v0 — but the parser
                                // accepts NAME|KEYWORD as a fallback to
                                // method-call shapes.  Treat as a local.
                                return Ok(Expr::VarRef {
                                    name: tok.value.clone(),
                                    scope: Scope::Local,
                                    span,
                                });
                            }
                        },
                        _ => {
                            // Parens — skip the LPAREN/RPAREN tokens
                            // and recurse into the inner expression
                            // (which is a sibling Node child).
                        }
                    }
                }
                ASTNodeOrToken::Node(sub) => {
                    // Skip dot_call children — those are postfix-applied
                    // by `apply_dot_chain` after the atom is extracted.
                    if sub.rule_name == "dot_call" {
                        continue;
                    }
                    return self.lower_expression(sub);
                }
            }
        }
        Err(RubyLowerError {
            message: "factor node had no recognisable leaf".to_string(),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })
    }

    // -------------------------------------------------------------------
    // Phase 6l — dot-call chain postfix
    // -------------------------------------------------------------------

    /// Walk every `dot_call` Node child of `node` (in source order) and
    /// fold each one into a method-call expression with the running
    /// `recv` as receiver.  `foo.bar.baz` becomes:
    ///
    /// ```text
    /// __method__(recv = foo, "bar")    →  inner
    /// __method__(recv = inner, "baz")  →  outer
    /// ```
    ///
    /// The chosen SIR encoding is `Expr::BuiltinCall { name:
    /// "__method__", args: [receiver, StrLit(method_name), ...args] }`.
    /// This keeps the receiver as a first-class expression (preserving
    /// arbitrary nesting), the method name as data (so backends can
    /// dispatch by string), and avoids growing the shared SIR Expr enum.
    ///
    /// BuiltinCall (not DirectCall) is chosen because the validator
    /// checks DirectCall.fn_name against the module's function table,
    /// and our synthetic `__method__` envelope intentionally isn't a
    /// declared function — it's a wire-format tag for backends.
    ///
    /// Effects default to PURE — receiver-dispatched calls are
    /// type-erased at this layer; a later receiver-type analysis pass
    /// can widen as needed.  Callers wrapping I/O-flavored chains
    /// (e.g. `STDOUT.puts(...)`) can post-process.
    fn apply_dot_chain(
        &mut self,
        atom: Expr,
        node: &GrammarASTNode,
    ) -> Result<Expr, RubyLowerError> {
        let mut recv = atom;
        for child in &node.children {
            if let ASTNodeOrToken::Node(sub) = child {
                if sub.rule_name == "dot_call" {
                    recv = self.fold_one_dot_call(recv, sub)?;
                }
            }
        }
        Ok(recv)
    }

    /// Lower a single `dot_call` step.  Grammar shape (Phase 6s):
    ///     dot_call = "." ( NAME | KEYWORD ) [ LPAREN [ call_arg
    ///                  { COMMA call_arg } ] RPAREN ] ;
    fn fold_one_dot_call(
        &mut self,
        receiver: Expr,
        dot_node: &GrammarASTNode,
    ) -> Result<Expr, RubyLowerError> {
        // First Name/Keyword token under dot_node is the method name.
        let (method_name, name_span) = self.expect_first_name_token(dot_node)?;
        // Optional argument list — each arg is wrapped in `call_arg`
        // (Phase 6s) so the optional splat prefix has a slot.
        let args: Vec<Expr> = dot_node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "call_arg" => Some(n),
                _ => None,
            })
            .map(|n| self.lower_call_arg(n))
            .collect::<Result<Vec<_>, _>>()?;

        let span = self.span_of(dot_node);
        // Pack as BuiltinCall("__method__", [receiver, StrLit(method),
        // ...args]) — see apply_dot_chain doc for rationale.
        // The synthetic StrLit triggers the Strings feature, which the
        // post-pass adds to the manifest unconditionally.
        self.features_used.insert(Feature::Strings);
        let mut full_args = Vec::with_capacity(args.len() + 2);
        full_args.push(receiver);
        full_args.push(Expr::StrLit {
            value: method_name,
            span: name_span,
        });
        full_args.extend(args);
        Ok(Expr::BuiltinCall {
            name: "__method__".to_string(),
            args: full_args,
            effects: EffectSet::PURE,
            span,
        })
    }

    // -------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------

    fn first_node_child<'a>(&self, node: &'a GrammarASTNode) -> Option<&'a GrammarASTNode> {
        node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            _ => None,
        })
    }

    fn find_node_child<'a>(
        &self,
        node: &'a GrammarASTNode,
        rule_name: &str,
    ) -> Option<&'a GrammarASTNode> {
        node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == rule_name => Some(n),
            _ => None,
        })
    }

    /// Return the lexeme of the first `Name` or `Keyword` token
    /// directly under `node` along with its span.
    fn expect_first_name_token(
        &self,
        node: &GrammarASTNode,
    ) -> Result<(String, Span), RubyLowerError> {
        for child in &node.children {
            if let ASTNodeOrToken::Token(t) = child {
                if matches!(t.type_, TokenType::Name | TokenType::Keyword) {
                    return Ok((t.value.clone(), self.span_of_token(t)));
                }
            }
        }
        Err(RubyLowerError {
            message: format!(
                "expected first Name/Keyword token under `{}`",
                node.rule_name
            ),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        })
    }
}

// ---------------------------------------------------------------------------
// Token plumbing
// ---------------------------------------------------------------------------

fn token_type_name(t: TokenType) -> &'static str {
    // We only need names for the operator tokens that appear in
    // expression chains.  Anything else returns a placeholder; the
    // caller never compares it against the operator list.
    match t {
        TokenType::Plus => "PLUS",
        TokenType::Minus => "MINUS",
        TokenType::Star => "STAR",
        TokenType::Slash => "SLASH",
        _ => "OTHER",
    }
}

fn token_lexeme_for_op(t: TokenType) -> &'static str {
    match t {
        TokenType::Plus => "+",
        TokenType::Minus => "-",
        TokenType::Star => "*",
        TokenType::Slash => "/",
        _ => "?",
    }
}

// ---------------------------------------------------------------------------
// Ruby builtins
// ---------------------------------------------------------------------------

/// Effect set for a recognised Ruby builtin.  Returns `None` for any
/// name we don't know — the caller falls back to `DirectCall`.
///
/// The v0 list is intentionally tiny: just the I/O and error-raising
/// builtins that nearly every Ruby program touches.  Later phases
/// will grow this as the lowering matures.
fn ruby_builtin_effects(name: &str) -> Option<EffectSet> {
    match name {
        "puts" | "print" | "p" => {
            Some(EffectSet::PURE.with(Effect::MayPrint))
        }
        "gets" => {
            // Reads from stdin — modelled as a blocking effect.  Not
            // strictly pure, but `MayBlock` is the closest tag we
            // have in the SIR v0 effect lattice.
            Some(EffectSet::PURE.with(Effect::MayBlock))
        }
        "raise" => {
            // `raise` is divergent (the call doesn't return) and
            // also throws — backends use both tags to suppress
            // unreachable-code warnings and to emit `throw`/`return`
            // shapes correctly.
            Some(EffectSet::PURE.with(Effect::MayThrow).with(Effect::Divergent))
        }
        // Phase 6g: block-taking iterators.  These all accept a
        // trailing block (closure) as their last argument and invoke
        // it zero or more times.  v0 models them as pure builtins —
        // their effect set is the *closure's* effect set lifted, but
        // SIR's effect inference handles that at the call site, so
        // we just declare PURE here.  Adding them to the builtin
        // table makes `each { … }` lower cleanly without forcing
        // every consumer to declare `each` as a user function.
        // Phase 6w — explicit closure-construction builtins.  `lambda { ... }`
        // and `proc { ... }` go through `method_with_block` and pass their
        // hoisted closure as the trailing arg.  Tagging both as known
        // builtins (PURE) gives downstream emitters a single
        // closure-construction shape — same as Phase 6w's arrow-lambda.
        "lambda" | "proc" => Some(EffectSet::PURE),
        "each" | "map" | "select" | "reject" | "filter"
        | "each_with_index" | "each_with_object" | "times"
        | "tap" | "then" | "yield_self" | "loop"
        | "collect" | "find" | "detect" | "any?" | "all?"
        | "none?" | "count" | "reduce" | "inject" | "sort_by"
        | "group_by" | "min_by" | "max_by" | "flat_map"
        | "partition" | "each_slice" | "each_cons" => {
            Some(EffectSet::PURE)
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Misc
// ---------------------------------------------------------------------------

impl RubyLowerError {
    /// Chain a value onto the error (used inside `?`-style fallbacks
    /// where we need to "consume" a value without otherwise using it).
    fn also<T>(self, _: T) -> Self {
        self
    }
}

