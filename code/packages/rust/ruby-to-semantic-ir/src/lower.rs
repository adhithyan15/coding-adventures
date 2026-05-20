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
    Block, Effect, EffectSet, ExportName, Expr, FeatureManifest, Function, Metadata, Module,
    Scope, Span, Stmt,
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
    };
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

    Ok(Module {
        name: module_name.to_string(),
        manifest: FeatureManifest::new(),
        imports: Vec::new(),
        // `main` is the conventional entry point — exporting it lets
        // SIR backends recognise it as such.
        exports: vec![ExportName {
            name: "main".to_string(),
            span: Span::synthetic(),
        }],
        functions: vec![main],
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
            other => Err(RubyLowerError {
                message: format!("unsupported statement form `{other}`"),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            }),
        }
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
                            // `nil` / `true` / `false` would be Keyword
                            // tokens, not Name.  All Name tokens here
                            // are locals (or unresolved — backend will
                            // tell us).
                            return Ok(Expr::VarRef {
                                name: tok.value.clone(),
                                scope: Scope::Local,
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

