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
            if is_tail && matches!(tail_kind, "expression_stmt" | "method_call") {
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
                    "method_call" => self.lower_method_call(inner)?,
                    _ => unreachable!(),
                };
                value = Some(v);
            } else {
                stmts_out.push(self.lower_statement_inner(inner)?);
            }
        }

        let value = value.unwrap_or(Expr::NilLit { span: self.span_of(program) });
        Ok(Block {
            stmts: stmts_out,
            value,
            span: self.span_of(program),
        })
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
            "while_statement" | "until_statement" => {
                // Phase 6c: SIR's `Stmt::While` is the canonical
                // top-level loop — `until cond` lowers to
                // `while !cond` (wrap the condition in `not`).
                self.lower_while_or_until(node)
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
            if is_tail && matches!(kind, "expression_stmt" | "method_call") {
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
                    "method_call" => self.lower_method_call(inner)?,
                    _ => unreachable!(),
                };
                value = Some(v);
            } else {
                stmts_out.push(self.lower_statement_inner(inner)?);
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

        // Collect parameters.  The optional `params` rule node lists
        // each parameter name as a sequence of Name tokens separated
        // by COMMA tokens — we only care about the names.
        let params_node = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "params" => Some(n),
            _ => None,
        });
        let params: Vec<Param> = if let Some(pn) = params_node {
            pn.children
                .iter()
                .filter_map(|c| match c {
                    ASTNodeOrToken::Token(t)
                        if matches!(t.type_, TokenType::Name) =>
                    {
                        Some(Param {
                            name: t.value.clone(),
                            sir_type: None,
                            span: self.span_of_token(t),
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
                if is_tail && matches!(kind, "expression_stmt" | "method_call") {
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
                        "method_call" => self.lower_method_call(inner)?,
                        _ => unreachable!(),
                    };
                    value = Some(v);
                } else {
                    stmts_out.push(self.lower_statement_inner(inner)?);
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
        // Shape: NAME EQUALS expression
        let (name, name_span) = self.expect_first_name_token(node)?;
        let expr_node = self.find_node_child(node, "expression").ok_or_else(|| {
            RubyLowerError {
                message: "assignment missing RHS expression".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            }
        })?;
        let value = self.lower_expression(expr_node)?;

        let span = self.span_of(node);
        if self.declared_locals.contains(&name) {
            // Phase 6b: `Stmt::Assign` re-binds an existing local —
            // the SIR validator requires the manifest to declare
            // `mutable-bindings` whenever this node appears.
            self.features_used.insert(Feature::MutableBindings);
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
    // method_call → BuiltinCall (recognised names) or DirectCall
    // -------------------------------------------------------------------

    fn lower_method_call(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // Shape: (NAME | KEYWORD) LPAREN [expression (COMMA expression)*] RPAREN
        let (callee, _callee_span) = self.expect_first_name_token(node)?;
        // Collect argument expressions: every `expression`-rule child of
        // this node.
        let args: Vec<Expr> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "expression" => Some(n),
                _ => None,
            })
            .map(|n| self.lower_expression(n))
            .collect::<Result<Vec<_>, _>>()?;

        let span = self.span_of(node);
        if let Some(effects) = ruby_builtin_effects(&callee) {
            Ok(Expr::BuiltinCall {
                name: callee,
                args,
                effects,
                span,
            })
        } else {
            // Unrecognised name — fall back to DirectCall.  SIR
            // backends that can't resolve the name will surface a
            // diagnostic; this keeps the lowering total (no panics).
            Ok(Expr::DirectCall {
                fn_name: callee,
                args,
                effects: EffectSet::PURE,
                span,
            })
        }
    }

    // -------------------------------------------------------------------
    // expression / term / factor
    // -------------------------------------------------------------------

    fn lower_expression(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // Pass through wrapper rules transparently — the parser
        // sometimes nests `expression → term → factor → expression`.
        match node.rule_name.as_str() {
            "expression" => self.lower_binary_chain(node, &["PLUS", "MINUS"]),
            "term" => self.lower_binary_chain(node, &["STAR", "SLASH"]),
            "factor" => self.lower_factor(node),
            "array_literal" => self.lower_array_literal(node),
            "hash_literal" => self.lower_hash_literal(node),
            "symbol_literal" => self.lower_symbol_literal(node),
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

    fn lower_factor(&mut self, node: &GrammarASTNode) -> Result<Expr, RubyLowerError> {
        // factor ::= NUMBER | STRING | NAME | KEYWORD | LPAREN expression RPAREN
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(tok) => {
                    let span = self.span_of_token(tok);
                    match tok.type_ {
                        TokenType::Number => {
                            let cleaned: String =
                                tok.value.chars().filter(|c| *c != '_').collect();
                            let v: i64 = cleaned.parse().map_err(|_| RubyLowerError {
                                message: format!("invalid integer literal `{}`", tok.value),
                                line: tok.line,
                                column: tok.column,
                            })?;
                            return Ok(Expr::IntLit { value: v, span });
                        }
                        TokenType::String => {
                            return Ok(Expr::StrLit {
                                value: tok.value.clone(),
                                span,
                            });
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

