//! The lowering pass from `python_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **milestone M1**.
//!
//! # What M1 covers
//!
//! Only *literals* at the top level.  The Python parser emits a deeply
//! nested generic CST (every precedence level of the expression
//! grammar is its own rule), so the bulk of M1's work is *peeling*
//! that onion down to the single `atom` token that carries a literal.
//!
//! ```text
//! file
//!   statement
//!     simple_stmt
//!       small_stmt
//!         assign_stmt
//!           expression_list
//!             expression
//!               walrus_expr → or_expr → … → power → await_expr
//!                 → primary → atom
//!                   TOKEN type_name="INT"  value="42"     ⇒ IntLit
//!                   TOKEN type_name="FLOAT" value="3.25"  ⇒ FloatLit
//!                   TOKEN Keyword "True"/"False"          ⇒ BoolLit
//!                   TOKEN Keyword "None"                  ⇒ NilLit
//!                   TOKEN String  "hi"                    ⇒ StrLit
//! ```
//!
//! Every rule between `expression` and `atom` is a *single-child
//! wrapper* when the source is a bare literal, so we collapse the
//! chain generically (`unwrap_single_node`) rather than naming all
//! ~20 levels.
//!
//! ## Unary minus
//!
//! `-7` parses as `factor( Minus, factor(…INT 7…) )`.  When `factor`
//! has the shape `[Token("-"), Node]` and the inner node lowers to an
//! `IntLit`/`FloatLit`, we negate it in place — a trivial constant
//! fold, kept because the spec lists `-7 ⇒ IntLit { value }`.
//!
//! ## Everything else is deferred
//!
//! - variable references (`x`)                 → deferred to M2
//! - assignment (`x = 1`, `assign_suffix`)     → deferred to M2
//! - operators / calls / collections / control → deferred to M3+
//!
//! Unhandled rules produce a clear `PythonLowerError`
//! (`"unsupported in M1: <rule>"`) rather than silently dropping
//! source, so later milestones can slot their extractors in exactly
//! where the M1 error is raised today.

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span, Stmt,
};

/// Maximum expression-nesting depth the lowerer will descend before
/// bailing with an error.  The expression-precedence chain is ~20 levels
/// deep for a *bare* literal, and explicit grouping/unary-minus add a
/// level each, so a healthy human-written literal sits far below this.
/// The cap exists purely to turn pathologically deep (but parseable)
/// input — `((((…42…))))`, `------…42` — into a clean `PythonLowerError`
/// instead of a native stack overflow (which aborts unrecoverably).
const MAX_EXPR_DEPTH: usize = 256;

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// An error encountered during Python → SIR lowering.
///
/// Mirrors `TwigLowerError`'s shape exactly (`message` + 1-based
/// `line`/`column`) so tooling can treat all SIR frontends uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PythonLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for PythonLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PythonLowerError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for PythonLowerError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Synthetic file name used for all spans (the CST does not carry the
/// original path).
const FILE: &str = "<python>";

/// Lower a parsed Python CST into a SIR module (M1: literals only).
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, PythonLowerError> {
    Lowerer::new(module_name).lower_file(tree)
}

// ---------------------------------------------------------------------------
// The lowerer
// ---------------------------------------------------------------------------

struct Lowerer {
    module_name: String,
    /// Features observed while lowering, used to build the manifest so
    /// it declares *exactly* what the module emits.
    observed: FeatureManifest,
}

impl Lowerer {
    fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            observed: FeatureManifest::new(),
        }
    }

    // -------------------------------------------------------------------
    // top level: `file` → synthesise `main`
    // -------------------------------------------------------------------

    /// The CST root is a `file` rule whose children are top-level
    /// `statement` nodes interleaved with stray `Newline` tokens.
    ///
    /// In M1 every statement must be a bare literal expression.  We
    /// lower each to an `Expr`; the *last* one becomes `main`'s block
    /// value (Python's "last expression's value" REPL semantics), and
    /// any earlier ones become `ExprStmt`s so they are still evaluated.
    /// An empty program yields `main` returning `NilLit`.
    fn lower_file(&mut self, file: &GrammarASTNode) -> Result<Module, PythonLowerError> {
        if file.rule_name != "file" {
            return Err(self.err_at(
                file,
                format!("expected `file` root, got `{}`", file.rule_name),
            ));
        }

        // Collect the lowered expression for each top-level statement.
        let mut exprs: Vec<Expr> = Vec::new();
        for child in &file.children {
            if let ASTNodeOrToken::Node(stmt) = child {
                // Skip purely structural empty `statement` wrappers that
                // carry no expression (defensive — none observed in M1).
                if let Some(expr) = self.lower_statement(stmt)? {
                    exprs.push(expr);
                }
            }
            // Token children at file level are stray NEWLINEs — ignore.
        }

        let span = Span::point(FILE, 1, 1);

        // Split into leading ExprStmts + final value expression.
        let value = exprs.pop().unwrap_or(Expr::NilLit { span: span.clone() });
        let stmts: Vec<Stmt> = exprs
            .into_iter()
            .map(|expr| {
                let s = expr.span().clone();
                Stmt::ExprStmt { expr, span: s }
            })
            .collect();

        let main = Function {
            name: "main".to_string(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts,
                value,
                span: span.clone(),
            },
            effects: semantic_ir::EffectSet::PURE,
            metadata: Metadata::new(),
            span: span.clone(),
        };

        let metadata = Metadata::new()
            .with_source_language("python")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION);

        Ok(Module {
            name: self.module_name.clone(),
            manifest: self.observed.clone(),
            imports: vec![],
            exports: vec![],
            functions: vec![main],
            globals: vec![],
            metadata,
            span,
        })
    }

    // -------------------------------------------------------------------
    // statement → expression
    // -------------------------------------------------------------------

    /// Lower a top-level `statement`.  In M1 the only supported shape
    /// is an *expression statement* (a bare literal).  Assignments,
    /// `def`/`class`, imports, control flow, etc. are rejected with a
    /// clear `unsupported in M1` error.
    fn lower_statement(&mut self, stmt: &GrammarASTNode) -> Result<Option<Expr>, PythonLowerError> {
        // Descend through the statement wrappers:
        //   statement → simple_stmt → small_stmt → assign_stmt
        // (compound statements like `if`/`def`/`for` take a different
        // branch and are unsupported in M1.)
        let simple = self.expect_single_named(stmt, "statement", &["simple_stmt"])?;
        let small = self.expect_single_named(simple, "simple_stmt", &["small_stmt"])?;
        let assign = self.expect_single_named(small, "small_stmt", &["assign_stmt"])?;

        // `assign_stmt` carries an `expression_list` and, *only for real
        // assignments*, an `assign_suffix` (`= rhs`).  An `assign_suffix`
        // means `x = ...`, which is deferred to M2.
        let node_children: Vec<&GrammarASTNode> = assign
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                ASTNodeOrToken::Token(_) => None,
            })
            .collect();
        if node_children.iter().any(|n| n.rule_name == "assign_suffix") {
            return Err(self.err_at(
                assign,
                "unsupported in M1: assignment (deferred to M2)".to_string(),
            ));
        }

        let expr_list = self.expect_single_named(assign, "assign_stmt", &["expression_list"])?;
        // An `expression_list` of one element is a single expression;
        // multi-element (tuple / multi-target) lists are deferred.
        let expr_nodes: Vec<&GrammarASTNode> = expr_list
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                ASTNodeOrToken::Token(_) => None,
            })
            .collect();
        if expr_nodes.len() != 1 {
            return Err(self.err_at(
                expr_list,
                "unsupported in M1: multi-element expression list (deferred)".to_string(),
            ));
        }

        let expr = self.lower_expr(expr_nodes[0])?;
        Ok(Some(expr))
    }

    // -------------------------------------------------------------------
    // expression → literal
    // -------------------------------------------------------------------

    /// Lower an expression node by peeling the precedence-rule onion
    /// down to the literal `atom`.
    ///
    /// Two structural cases are handled before the generic peel:
    ///
    /// 1. `factor( "-", factor(...) )` — unary minus on a numeric
    ///    literal, constant-folded into a negative `IntLit`/`FloatLit`.
    /// 2. any other multi-child node — unsupported in M1 (it would be
    ///    an operator, call, subscript, etc.).
    fn lower_expr(&mut self, node: &GrammarASTNode) -> Result<Expr, PythonLowerError> {
        self.lower_expr_d(node, 0)
    }

    /// Depth-tracked core of [`Self::lower_expr`].
    ///
    /// Python's precedence grammar wraps every expression in a deep
    /// single-child chain (`expression → … → atom`, ~20 levels), and
    /// source can nest arbitrarily (`((((42))))`, `------42`).  The
    /// generic peel below and `try_unary_minus` both recurse, so the
    /// recursion depth tracks the *input* nesting depth.  Without a cap,
    /// pathologically deep (but parseable) input would exhaust the native
    /// stack — an unrecoverable abort, since a Rust stack overflow cannot
    /// be caught.  `compile` is a public entry point taking an arbitrary
    /// CST, so we must bound this ourselves: past `MAX_EXPR_DEPTH` we
    /// return a positioned error instead of recursing further.
    fn lower_expr_d(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, PythonLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }

        // Case 1: unary-minus literal.  `factor` with [Minus, factor].
        if node.rule_name == "factor" {
            if let Some(folded) = self.try_unary_minus(node, depth)? {
                return Ok(folded);
            }
        }

        // Case 2: a `leaf` node — exactly one child that is a Token.
        // This is where every literal bottoms out (`atom` is a leaf).
        if let Some(tok) = node.token() {
            return self.lower_literal_token(node, tok);
        }

        // Generic peel: a single Node child → recurse.  Any other shape
        // (zero children, or >1 node children, or a Token sibling) is a
        // non-literal construct we do not handle in M1.
        let node_children: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                ASTNodeOrToken::Token(_) => None,
            })
            .collect();

        match node_children.as_slice() {
            [only] if node.children.len() == 1 => self.lower_expr_d(only, depth + 1),
            _ => Err(self.err_at(
                node,
                format!("unsupported in M1: {} (only literals are lowered)", node.rule_name),
            )),
        }
    }

    /// Recognise `factor( Token("-"), Node )` and constant-fold the
    /// negation when the inner node is a numeric literal.  Returns
    /// `Ok(None)` if `node` is not a unary-minus factor, letting the
    /// caller fall through to the generic peel.
    fn try_unary_minus(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Option<Expr>, PythonLowerError> {
        // Exactly two children: a leading Token and an inner Node.
        if node.children.len() != 2 {
            return Ok(None);
        }
        let (lead, inner) = (&node.children[0], &node.children[1]);
        let is_minus = matches!(lead, ASTNodeOrToken::Token(t) if t.value == "-");
        let inner = match inner {
            ASTNodeOrToken::Node(n) if is_minus => n,
            _ => return Ok(None),
        };

        // Lower the operand, then negate iff it is numeric.  Carry the
        // depth forward (+1) so a chain of unary minuses (`----42`) is
        // bounded by the same `MAX_EXPR_DEPTH` budget as the peel.
        let operand = self.lower_expr_d(inner, depth + 1)?;
        match operand {
            Expr::IntLit { value, span } => Ok(Some(Expr::IntLit {
                value: value.wrapping_neg(),
                span,
            })),
            Expr::FloatLit { value, span } => Ok(Some(Expr::FloatLit {
                value: -value,
                span,
            })),
            // `-x`, `-True`, etc. are real unary operators → deferred.
            _ => Err(self.err_at(
                node,
                "unsupported in M1: unary minus on non-literal (deferred)".to_string(),
            )),
        }
    }

    /// Turn a leaf `atom`/literal token into the matching SIR literal.
    ///
    /// Token classification (learned by inspecting real parses):
    ///
    /// | token                                   | SIR node            |
    /// |-----------------------------------------|---------------------|
    /// | `type_name == "INT"`                    | `IntLit`            |
    /// | `type_name == "FLOAT"`                  | `FloatLit` (+Floats)|
    /// | Keyword `True` / `False`                | `BoolLit`           |
    /// | Keyword `None`                          | `NilLit`            |
    /// | String token                            | `StrLit` (+Strings) |
    fn lower_literal_token(
        &mut self,
        node: &GrammarASTNode,
        tok: &lexer::token::Token,
    ) -> Result<Expr, PythonLowerError> {
        let span = self.span_of(node);
        let type_name = tok.type_name.as_deref();

        match (type_name, tok.type_, tok.value.as_str()) {
            (Some("INT"), _, text) => {
                let value: i64 = text.parse().map_err(|_| {
                    self.err_at(node, format!("invalid integer literal `{text}`"))
                })?;
                Ok(Expr::IntLit { value, span })
            }
            (Some("FLOAT"), _, text) => {
                let value: f64 = text.parse().map_err(|_| {
                    self.err_at(node, format!("invalid float literal `{text}`"))
                })?;
                self.observed.add(Feature::Floats);
                Ok(Expr::FloatLit { value, span })
            }
            (_, lexer::token::TokenType::Keyword, "True") => {
                Ok(Expr::BoolLit { value: true, span })
            }
            (_, lexer::token::TokenType::Keyword, "False") => {
                Ok(Expr::BoolLit { value: false, span })
            }
            (_, lexer::token::TokenType::Keyword, "None") => Ok(Expr::NilLit { span }),
            (_, lexer::token::TokenType::String, text) => {
                self.observed.add(Feature::Strings);
                // The lexer already resolved escapes (`"\n"` → newline),
                // so the token value is the final string content.
                Ok(Expr::StrLit {
                    value: text.to_string(),
                    span,
                })
            }
            // A bare `Name` token (e.g. `x`) is a variable reference,
            // deferred to M2; anything else is genuinely unsupported.
            _ => Err(self.err_at(
                node,
                format!(
                    "unsupported in M1: token `{}` (only int/float/bool/None/string literals)",
                    tok.value
                ),
            )),
        }
    }

    // -------------------------------------------------------------------
    // helpers
    // -------------------------------------------------------------------

    /// Assert `node.rule_name == expected` and that it has exactly one
    /// child *node* whose rule name is in `allowed`; return that child.
    /// Used to walk the fixed `statement → … → assign_stmt` spine while
    /// emitting precise errors when an unexpected (compound-statement)
    /// shape shows up.
    fn expect_single_named<'a>(
        &self,
        node: &'a GrammarASTNode,
        expected: &str,
        allowed: &[&str],
    ) -> Result<&'a GrammarASTNode, PythonLowerError> {
        if node.rule_name != expected {
            return Err(self.err_at(
                node,
                format!("expected `{}`, got `{}`", expected, node.rule_name),
            ));
        }
        let node_children: Vec<&GrammarASTNode> = node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                ASTNodeOrToken::Token(_) => None,
            })
            .collect();
        match node_children.as_slice() {
            [child] if allowed.contains(&child.rule_name.as_str()) => Ok(child),
            [child] => Err(self.err_at(
                child,
                format!("unsupported in M1: {} (deferred)", child.rule_name),
            )),
            _ => Err(self.err_at(
                node,
                format!("unsupported in M1: {} with multiple parts (deferred)", expected),
            )),
        }
    }

    /// Build a `Span` from a node's start position, defaulting to 1:1
    /// when the parser did not record one.
    fn span_of(&self, node: &GrammarASTNode) -> Span {
        Span::point(
            FILE,
            node.start_line.unwrap_or(1),
            node.start_column.unwrap_or(1),
        )
    }

    /// Build a `PythonLowerError` anchored at `node`'s start position.
    fn err_at(&self, node: &GrammarASTNode, message: String) -> PythonLowerError {
        PythonLowerError {
            message,
            line: node.start_line.unwrap_or(1),
            column: node.start_column.unwrap_or(1),
        }
    }
}
