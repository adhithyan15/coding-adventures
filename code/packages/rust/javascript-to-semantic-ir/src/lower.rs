//! JavaScript `GrammarASTNode` (CST) → `semantic_ir::Module` lowering.
//!
//! # What this file does (milestone M1)
//!
//! The [`javascript-parser`](coding_adventures_javascript_parser) crate
//! hands us a *concrete syntax tree* (CST): a [`GrammarASTNode`] whose
//! shape mirrors the ECMAScript grammar one-for-one.  Even a bare
//! literal like `42;` produces a deep spine of single-child wrapper
//! nodes — `program → source_element → statement →
//! expression_statement → expression → assignment_expression → … →
//! primary_expression → <token>`.  Twenty-odd rule layers, all of which
//! exist only to encode operator precedence, and *none* of which carry
//! information once you've reached the leaf.
//!
//! M1 lowers **literals only**.  The strategy is therefore deliberately
//! blunt: walk a statement down to its single leaf token, classify the
//! token, and emit the matching SIR literal.  Anything that *isn't* a
//! single-leaf literal statement (an operator, a name reference, a
//! declaration, a call) is **out of scope** and rejected with a clear
//! [`JsLowerError`] so later milestones can slot their handling in at
//! exactly the right place.
//!
//! ## The literal truth table (learned by probing the real parser)
//!
//! | JS source     | leaf token                                  | SIR node            |
//! |---------------|---------------------------------------------|---------------------|
//! | `42`          | `Number` value `"42"`  (no `.`/`e`)         | `IntLit { 42 }`     |
//! | `3.25`        | `Number` value `"3.25"` (has `.`)           | `FloatLit { 3.25 }` |
//! | `1e3`         | `Number` value `"1e3"` (has `e`)            | `FloatLit { 1000 }` |
//! | `true`        | `Keyword` value `"true"`                    | `BoolLit { true }`  |
//! | `false`       | `Keyword` value `"false"`                   | `BoolLit { false }` |
//! | `null`        | `Keyword` value `"null"`                    | `NilLit`            |
//! | `undefined`   | `Name`   value `"undefined"` (an ident!)    | `NilLit`            |
//! | `"hi"`/`'hi'` | `String` value `hi` (already unescaped)     | `StrLit { "hi" }`   |
//!
//! Note two JS-specific lossy mappings, both spec-sanctioned for v0:
//!   * `null` **and** `undefined` collapse to `NilLit` — the IR has one
//!     "absence" value, so the JS distinction is lost.
//!   * an integer-shaped number becomes `IntLit`, everything else
//!     `FloatLit`.  JS has a single `number` type (an IEEE-754 double);
//!     splitting it lets integer-heavy code round-trip through SIR
//!     backends that *do* distinguish.

use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, ExportName, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span,
    CURRENT_SIR_VERSION,
};

/// A failure encountered during JavaScript → SIR lowering.
///
/// Carries 1-based `line`/`column` so callers can produce IDE-friendly
/// diagnostics.  When the position is unknown (the AST node had no
/// recorded span), the fields are zero.  Mirrors the error shape used by
/// the sibling [`ruby-to-semantic-ir`](https://example.invalid) and
/// [`twig-to-semantic-ir`] frontends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for JsLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "JsLowerError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for JsLowerError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed JavaScript program into a [`semantic_ir::Module`].
///
/// The root node must be a `program` rule node — that's what
/// [`coding_adventures_javascript_parser::parse_javascript`] always
/// emits.  The `module_name` becomes the SIR module identifier
/// (typically the source file's stem).
///
/// In M1 the body may contain only literal expression statements; any
/// other statement shape produces a [`JsLowerError`] (see module docs).
pub fn compile(program: &GrammarASTNode, module_name: &str) -> Result<Module, JsLowerError> {
    if program.rule_name != "program" {
        return Err(JsLowerError {
            message: format!("expected root rule `program`, got `{}`", program.rule_name),
            line: program.start_line.unwrap_or(0),
            column: program.start_column.unwrap_or(0),
        });
    }

    let mut lw = Lowerer {
        file_name: module_name.to_string(),
        features_used: FeatureManifest::new(),
    };

    let block = lw.lower_program(program)?;

    // Every JS source becomes a synthetic `main` whose body is the
    // top-level statement sequence — matching SIR17 (Python) and the
    // Ruby frontend.  `main` has no params, so it never triggers the
    // validator's `DynamicTyping` observation; we only declare features
    // that the literals themselves use (`Strings`, `Floats`).
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

    // Materialise the manifest in a stable order.  The SIR validator
    // requires the manifest to *exactly* match what the body uses:
    // used-but-undeclared is an error, declared-but-unused a warning.
    // We tallied features while lowering, so we just hand the
    // accumulator over.
    let manifest = lw.features_used.clone();

    let metadata = Metadata::new()
        .with_source_language("javascript")
        .with_sir_version(CURRENT_SIR_VERSION);

    Ok(Module {
        name: module_name.to_string(),
        manifest,
        imports: Vec::new(),
        // `main` is the conventional entry point — exporting it lets SIR
        // backends recognise it as such.
        exports: vec![ExportName {
            name: "main".to_string(),
            span: Span::synthetic(),
        }],
        functions: vec![main],
        globals: Vec::new(),
        metadata,
        span: lw.span_of(program),
    })
}

// ---------------------------------------------------------------------------
// Lowerer — the small amount of mutable state M1 needs
// ---------------------------------------------------------------------------

struct Lowerer {
    /// Logical filename stamped into every [`Span`].  We use the module
    /// name because the parser CST doesn't carry the original path.
    file_name: String,
    /// Features accumulated as we lower.  `FeatureManifest::add` is
    /// idempotent, so repeated `StrLit`s add `Strings` exactly once.
    features_used: FeatureManifest,
}

impl Lowerer {
    /// Build a [`Span`] from a node's recorded 1-based position.  Falls
    /// back to a zero point when the parser left positions unset.
    fn span_of(&self, node: &GrammarASTNode) -> Span {
        Span::new(
            self.file_name.clone(),
            node.start_line.unwrap_or(0),
            node.start_column.unwrap_or(0),
            node.end_line.unwrap_or_else(|| node.start_line.unwrap_or(0)),
            node.end_column.unwrap_or_else(|| node.start_column.unwrap_or(0)),
        )
    }

    /// Build a [`Span`] from a leaf [`Token`]'s 1-based position.  The
    /// width is one column (zero-width point), which is good enough for
    /// literal diagnostics.
    fn span_of_token(&self, tok: &Token) -> Span {
        Span::point(self.file_name.clone(), tok.line, tok.column)
    }

    // -----------------------------------------------------------------------
    // program → Block
    // -----------------------------------------------------------------------

    /// Lower the whole program into a single [`Block`].
    ///
    /// SIR `Block`s are "statements then a tail value": `Block.value` is
    /// the program's result.  Following SIR17/Ruby, the **final**
    /// top-level expression becomes the tail `value`; everything before
    /// it would become `stmts`.  In M1 the only statement kind is a
    /// literal expression statement, and a bare literal has no side
    /// effect, so any *non-final* literal statement is simply dropped (it
    /// can't be observed).  We therefore keep `stmts` empty and route the
    /// last literal — if any — into `value`.  An empty program yields a
    /// `NilLit` value, exactly like `ruby-to-semantic-ir` on `""`.
    fn lower_program(&mut self, program: &GrammarASTNode) -> Result<Block, JsLowerError> {
        // `program`'s children are `source_element` nodes (one per
        // top-level statement).  We collect the lowered expression for
        // each so the last becomes the block's tail value.
        let mut exprs: Vec<Expr> = Vec::new();
        for child in &program.children {
            match child {
                ASTNodeOrToken::Node(n) => {
                    // A `source_element` wraps a `statement`; descend.
                    let expr = self.lower_source_element(n)?;
                    exprs.push(expr);
                }
                // Stray tokens directly under `program` (there should be
                // none for well-formed input) are ignored.
                ASTNodeOrToken::Token(_) => {}
            }
        }

        let value = exprs.pop().unwrap_or(Expr::NilLit {
            span: self.span_of(program),
        });

        Ok(Block {
            // M1 produces no statements: literals are pure, so only the
            // tail value is observable.  Later milestones (let-bindings,
            // assignments, calls) will populate this.
            stmts: Vec::new(),
            value,
            span: self.span_of(program),
        })
    }

    /// Lower one `source_element` (top-level item) to an [`Expr`].
    fn lower_source_element(&mut self, node: &GrammarASTNode) -> Result<Expr, JsLowerError> {
        // `source_element` → `statement` → `expression_statement`.
        // Descend through the single-child wrappers until we reach a
        // statement we recognise.  In M1 only `expression_statement` is
        // supported; a `function_declaration`, `variable_statement`,
        // `if_statement`, etc. reaches us here and is rejected.
        let stmt = single_child_node(node).unwrap_or(node);
        match stmt.rule_name.as_str() {
            "statement" => {
                let inner = single_child_node(stmt).unwrap_or(stmt);
                self.lower_statement_inner(inner)
            }
            "expression_statement" => self.lower_expression_statement(stmt),
            other => Err(self.unsupported(stmt, other)),
        }
    }

    /// Lower the node *inside* a `statement` wrapper.
    fn lower_statement_inner(&mut self, node: &GrammarASTNode) -> Result<Expr, JsLowerError> {
        match node.rule_name.as_str() {
            "expression_statement" => self.lower_expression_statement(node),
            // deferred to M2+: variable_statement, if_statement,
            // iteration_statement, function_declaration, return_statement…
            other => Err(self.unsupported(node, other)),
        }
    }

    /// Lower an `expression_statement` (`<expression> ;`) to an [`Expr`].
    fn lower_expression_statement(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Expr, JsLowerError> {
        // Children are the `expression` node followed by a `Semicolon`
        // token.  Find the first child node (the expression) and lower
        // it; the trailing semicolon carries no value.
        let expr_node = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                ASTNodeOrToken::Token(_) => None,
            })
            .ok_or_else(|| JsLowerError {
                message: "empty expression statement".to_string(),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            })?;
        self.lower_expression(expr_node)
    }

    // -----------------------------------------------------------------------
    // expression → Expr (M1: literals only)
    // -----------------------------------------------------------------------

    /// Lower a JS `expression` to a SIR [`Expr`].
    ///
    /// The CST spine from `expression` down to the leaf is a chain of
    /// single-child precedence wrappers (see module docs).  We walk that
    /// spine to its bottom.  If the bottom is a single leaf token, it's a
    /// literal we can lower.  If instead we hit a node that *branches*
    /// (more than one meaningful child — i.e. an actual operator,
    /// arguments list, member access, …), that's a non-literal
    /// expression and out of M1 scope.
    fn lower_expression(&mut self, expr: &GrammarASTNode) -> Result<Expr, JsLowerError> {
        // Descend through single-child wrapper nodes.
        let mut cur = expr;
        loop {
            // A leaf node (exactly one child, and that child is a token)
            // is a literal — classify and emit.
            if let Some(tok) = cur.token() {
                return self.lower_literal_token(tok);
            }
            // Otherwise, if there's exactly one *node* child and no other
            // meaningful children, keep descending.
            match single_child_node(cur) {
                Some(next) => cur = next,
                None => {
                    // We reached a branching node (an operator, call,
                    // member access, …).  Not a literal → out of M1.
                    return Err(self.unsupported(cur, &cur.rule_name));
                }
            }
        }
    }

    /// Classify a leaf literal token and build the matching SIR literal.
    ///
    /// See the truth table in the module docs for the full mapping.
    fn lower_literal_token(&mut self, tok: &Token) -> Result<Expr, JsLowerError> {
        let span = self.span_of_token(tok);
        match tok.type_ {
            // ── number ──────────────────────────────────────────────
            // JS has a single numeric type.  We split on textual shape:
            // a literal with no `.` and no exponent marker is an
            // integer; anything else is a float.  This lets integer code
            // round-trip cleanly through backends that distinguish.
            TokenType::Number => self.lower_number(&tok.value, span),

            // ── keyword literals: true / false / null ───────────────
            TokenType::Keyword => match tok.value.as_str() {
                "true" => Ok(Expr::BoolLit { value: true, span }),
                "false" => Ok(Expr::BoolLit { value: false, span }),
                // `null` → the IR's single absence value.
                "null" => Ok(Expr::NilLit { span }),
                other => Err(JsLowerError {
                    message: format!("unsupported keyword literal `{other}` (M1 supports only true/false/null)"),
                    line: tok.line,
                    column: tok.column,
                }),
            },

            // ── string ──────────────────────────────────────────────
            // The lexer already resolved escape sequences, so `tok.value`
            // is the final string content.  NOTE: template literals
            // (backtick strings) are a *different* token/rule and are
            // deferred to a later milestone (see CHANGELOG "Deferred").
            TokenType::String => {
                self.features_used.add(Feature::Strings);
                Ok(Expr::StrLit {
                    value: tok.value.clone(),
                    span,
                })
            }

            // ── `undefined` ─────────────────────────────────────────
            // `undefined` is not a keyword in JS — it's a global
            // identifier, so it arrives as a `Name` token.  We special-
            // case that exact spelling to `NilLit` (the JS null/undefined
            // distinction is intentionally lost in v0).  Any *other*
            // identifier is a variable reference, which is out of M1
            // scope (deferred to M2).
            TokenType::Name if tok.value == "undefined" => Ok(Expr::NilLit { span }),
            TokenType::Name => Err(JsLowerError {
                message: format!(
                    "variable reference `{}` is out of scope for M1 (literals only)",
                    tok.value
                ),
                line: tok.line,
                column: tok.column,
            }),

            other => Err(JsLowerError {
                message: format!("unsupported literal token {other:?} (M1 supports literals only)"),
                line: tok.line,
                column: tok.column,
            }),
        }
    }

    /// Lower a numeric literal's *text* into `IntLit` or `FloatLit`.
    ///
    /// A literal is treated as an integer iff it parses as an `i64` and
    /// its text contains neither a decimal point nor an exponent marker
    /// (`e`/`E`).  Otherwise it's a float.  Hex/octal/binary integer
    /// forms (`0x…`, `0o…`, `0b…`) and `BigInt` (`10n`) are deferred to a
    /// later milestone.
    fn lower_number(&mut self, text: &str, span: Span) -> Result<Expr, JsLowerError> {
        let looks_float = text.contains('.') || text.contains('e') || text.contains('E');
        // Reject the non-decimal integer forms in M1: their `i64` parse
        // would fail, and silently falling through to a `FloatLit` would
        // be wrong.  Detecting them explicitly yields a clear error.
        let non_decimal = text.len() > 1
            && text.starts_with('0')
            && matches!(text.as_bytes()[1], b'x' | b'X' | b'o' | b'O' | b'b' | b'B');
        if non_decimal || text.ends_with('n') {
            return Err(JsLowerError {
                message: format!(
                    "numeric literal `{text}` form (hex/octal/binary/BigInt) is deferred past M1"
                ),
                line: span.start_line,
                column: span.start_col,
            });
        }

        if !looks_float {
            if let Ok(value) = text.parse::<i64>() {
                return Ok(Expr::IntLit { value, span });
            }
            // Integer-shaped but doesn't fit i64 (e.g. very large).  Fall
            // through to float so we don't lose the program; JS would
            // hold it as a double anyway.
        }

        match text.parse::<f64>() {
            Ok(value) => {
                self.features_used.add(Feature::Floats);
                Ok(Expr::FloatLit { value, span })
            }
            Err(_) => Err(JsLowerError {
                message: format!("could not parse numeric literal `{text}`"),
                line: span.start_line,
                column: span.start_col,
            }),
        }
    }

    /// Build the standard "out of M1 scope" error for a node.
    fn unsupported(&self, node: &GrammarASTNode, what: &str) -> JsLowerError {
        JsLowerError {
            message: format!(
                "`{what}` is out of scope for M1 (literals only); deferred to a later milestone"
            ),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        }
    }
}

// ---------------------------------------------------------------------------
// Free helpers
// ---------------------------------------------------------------------------

/// If `node` has exactly one child and that child is a nested node,
/// return it.  This is the workhorse for descending the CST's precedence
/// spine: each precedence layer that wasn't "used" appears as a wrapper
/// with a single node child.  Returns `None` when the node branches
/// (multiple children) or when its single child is a token (a leaf).
fn single_child_node(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    if node.children.len() == 1 {
        match &node.children[0] {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        }
    } else {
        None
    }
}
