//! The lowering pass from `python_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **milestone M2**.
//!
//! # What M1 covered (still supported)
//!
//! Only *literals* at the top level.  The Python parser emits a deeply
//! nested generic CST (every precedence level of the expression
//! grammar is its own rule), so the bulk of M1's work was *peeling*
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
//!               walrus_expr → or_expr → and_expr → not_expr
//!                 → comparison → bitwise_or → … → arith → term
//!                   → factor → power → await_expr → primary → atom
//!                     TOKEN type_name="INT"  value="42"     ⇒ IntLit
//!                     TOKEN type_name="FLOAT" value="3.25"  ⇒ FloatLit
//!                     TOKEN Keyword "True"/"False"          ⇒ BoolLit
//!                     TOKEN Keyword "None"                  ⇒ NilLit
//!                     TOKEN String  "hi"                    ⇒ StrLit
//!                     TOKEN Name    "x"                     ⇒ VarRef (M2)
//! ```
//!
//! When the source is a *bare* literal, every rule between `expression`
//! and `atom` is a single-child wrapper, so we collapse the chain
//! generically rather than naming all ~20 levels.
//!
//! # What M2 adds
//!
//! M2 turns the precedence-rule onion from a pure *peel* into a small
//! *operator recogniser*.  Each precedence rule, when it actually
//! *branches* (more than one child node, with operator tokens between),
//! is lowered to the matching SIR node:
//!
//! | rule           | branching shape                       | SIR node                  |
//! |----------------|---------------------------------------|---------------------------|
//! | `or_expr`      | `a or b or …`                         | `LogicalOr` (left-nested) |
//! | `and_expr`     | `a and b and …`                       | `LogicalAnd` (left-nested)|
//! | `not_expr`     | `not x`                               | `BuiltinCall("not", [x])` |
//! | `comparison`   | `a < b`, `a == b`, …                  | `BuiltinCall("<"/"="/…)`  |
//! | `arith`        | `a + b`, `a - b`                      | `BuiltinCall("+"/"-")`    |
//! | `term`         | `a * b`, `a / b`, `a % b`             | `BuiltinCall("*"/"/"/"%") |
//! | `factor`       | `-x`, `+x`                            | `BuiltinCall("neg")` / id |
//!
//! Comparison maps `==`→`"="` and `!=`→`"!="` per SIR17; the rest use
//! their literal spelling.  `and`/`or` become the dedicated
//! short-circuit nodes (`LogicalAnd`/`LogicalOr`) rather than
//! `BuiltinCall`, because the latter would eagerly evaluate both sides.
//!
//! M2 also adds **variable references** and **assignment**:
//!
//! - a bare `Name` token (`x`) becomes a `VarRef` whose `scope` is
//!   resolved against the names bound so far in the current (module/
//!   `main`) scope — `Scope::Local` when bound, otherwise a positioned
//!   "unresolved name" error.
//! - `x = expr` performs *first-occurrence detection*: the first time a
//!   name is assigned in a scope it is *declared* (we emit a
//!   `LetStarBinding`); a later assignment to an already-declared name
//!   *mutates* it (we emit an `Assign`).  Python has no `let`/`var`
//!   keyword, so this mirrors how the JS and Ruby sibling frontends
//!   decide bind-vs-reassign.
//!
//! Why `LetStarBinding` and not `LetBinding`?  The SIR validator treats
//! a *run* of consecutive `LetBinding`s as a **parallel**-let group:
//! every RHS is checked in the scope *before* the group, so a later
//! binding cannot see an earlier one (`x = 1` then `y = x + 1` would
//! fail to resolve `x`).  `LetStarBinding` has **sequential** semantics
//! — each RHS sees the prior bindings — which is exactly Python's
//! top-to-bottom execution model.
//!
//! ## Constant-folded unary minus (carried from M1)
//!
//! `-7` parses as `factor( Minus, factor(…INT 7…) )`.  When the operand
//! is a numeric *literal* we still fold the negation in place
//! (`IntLit{-7}`), because the spec lists `-7 ⇒ IntLit { value }`.  For a
//! non-literal operand (`-x`) we now emit `BuiltinCall("neg", [x])`
//! instead of erroring.
//!
//! ## Still deferred (later milestones)
//!
//! - calls / functions / `def` / `lambda`                → M3+
//! - control flow (`if` / `while` / `for`)               → M3+
//! - collections (lists / dicts / indexing)              → M3+
//! - `global` / `nonlocal`, multi-target assignment      → deferred
//!
//! Unhandled rules produce a clear `PythonLowerError` rather than
//! silently dropping source, so later milestones can slot their
//! extractors in exactly where the error is raised today.

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope, Span, Stmt,
};
use std::collections::HashSet;

/// Maximum expression-nesting depth the lowerer will descend before
/// bailing with an error.  The expression-precedence chain is ~20 levels
/// deep for a *bare* literal, and explicit grouping/unary operators add a
/// level each, so a healthy human-written expression sits far below this.
/// The cap exists purely to turn pathologically deep (but parseable)
/// input — `((((…42…))))`, `------…42`, `a and a and a and …` — into a
/// clean `PythonLowerError` instead of a native stack overflow (which
/// aborts unrecoverably and cannot be caught in Rust).
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

/// Lower a parsed Python CST into a SIR module (M2: literals, variable
/// references, assignment, unary/binary operators).
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, PythonLowerError> {
    Lowerer::new(module_name).lower_file(tree)
}

// ---------------------------------------------------------------------------
// The lowerer
// ---------------------------------------------------------------------------

/// One lowered top-level statement: either a `Stmt` (an assignment) or a
/// bare expression (an expression statement / the final value).
///
/// `Stmt` is boxed because it is by far the largest variant (it holds a
/// full `Stmt`, which transitively contains `Block`s); boxing keeps the
/// enum small and silences `clippy::large_enum_variant`.
enum Lowered {
    Stmt(Box<Stmt>),
    Expr(Expr),
}

struct Lowerer {
    module_name: String,
    /// Features observed while lowering, used to build the manifest so
    /// it declares *exactly* what the module emits.
    observed: FeatureManifest,
    /// Names already bound in the current (module / `main`) scope.  Drives
    /// first-occurrence detection: the first `x = …` declares (`LetStar`),
    /// later `x = …` re-assign (`Assign`), and a `VarRef` to a name in
    /// this set resolves as `Scope::Local`.
    declared: HashSet<String>,
}

impl Lowerer {
    fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            observed: FeatureManifest::new(),
            declared: HashSet::new(),
        }
    }

    // -------------------------------------------------------------------
    // top level: `file` → synthesise `main`
    // -------------------------------------------------------------------

    /// The CST root is a `file` rule whose children are top-level
    /// `statement` nodes interleaved with stray `Newline` tokens.
    ///
    /// Each statement lowers to either an *assignment* statement
    /// (`LetStarBinding` / `Assign`) or a bare *expression*.  Python's
    /// REPL "last expression is the value" rule means the trailing item,
    /// **if it is a bare expression**, becomes `main`'s block value;
    /// everything before it becomes a `Stmt` (assignments stay as-is,
    /// bare expressions become `ExprStmt`s so side effects still run).
    /// A program whose last statement is an assignment has block value
    /// `NilLit` (assignment yields no value in Python).  An empty program
    /// yields `main` returning `NilLit`.
    fn lower_file(&mut self, file: &GrammarASTNode) -> Result<Module, PythonLowerError> {
        if file.rule_name != "file" {
            return Err(self.err_at(
                file,
                format!("expected `file` root, got `{}`", file.rule_name),
            ));
        }

        // Collect the lowered form of each top-level statement, in order.
        let mut items: Vec<Lowered> = Vec::new();
        for child in &file.children {
            if let ASTNodeOrToken::Node(stmt) = child {
                items.push(self.lower_statement(stmt)?);
            }
            // Token children at file level are stray NEWLINEs — ignore.
        }

        let span = Span::point(FILE, 1, 1);

        // The block value is the *last* item iff it is a bare expression;
        // otherwise (assignment last, or empty) the value is `NilLit` and
        // every item becomes a statement.
        let value = match items.last() {
            Some(Lowered::Expr(_)) => match items.pop() {
                Some(Lowered::Expr(e)) => e,
                _ => unreachable!("just matched Expr"),
            },
            _ => Expr::NilLit { span: span.clone() },
        };

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
            effects: EffectSet::PURE,
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
    // statement → assignment or expression
    // -------------------------------------------------------------------

    /// Lower a top-level `statement`.
    ///
    /// The supported shapes are an *assignment* (`x = expr`) and a bare
    /// *expression statement*.  Compound statements (`if`/`def`/`for`),
    /// `global`/`nonlocal`, imports, etc. take a different `small_stmt`
    /// branch (e.g. `global_stmt`) and are rejected with a clear
    /// "unsupported" error.
    fn lower_statement(&mut self, stmt: &GrammarASTNode) -> Result<Lowered, PythonLowerError> {
        // Descend the fixed statement spine:
        //   statement → simple_stmt → small_stmt → assign_stmt
        let simple = self.expect_single_named(stmt, "statement", &["simple_stmt"])?;
        let small = self.expect_single_named(simple, "simple_stmt", &["small_stmt"])?;
        let assign = self.expect_single_named(small, "small_stmt", &["assign_stmt"])?;

        // `assign_stmt` is `expression_list (assign_suffix)?`.  When an
        // `assign_suffix` (`= rhs`) is present it's a real assignment;
        // otherwise it's a bare expression statement.
        let node_children: Vec<&GrammarASTNode> = child_nodes(assign);
        let suffix = node_children
            .iter()
            .find(|n| n.rule_name == "assign_suffix")
            .copied();

        // The first `expression_list` is the assignment *target* (LHS) for
        // an assignment, or the whole expression for a bare statement.
        let lhs_list = self.expect_single_kind(assign, "expression_list")?;

        match suffix {
            None => {
                // Bare expression statement.
                let expr = self.lower_expr(self.single_expr(lhs_list)?)?;
                Ok(Lowered::Expr(expr))
            }
            Some(suffix) => self.lower_assignment(assign, lhs_list, suffix),
        }
    }

    /// Lower `target = rhs` from its `assign_stmt`, target `expression_list`,
    /// and `assign_suffix` (`= <expression_list>`).
    ///
    /// First-occurrence detection: the first assignment to a name emits a
    /// `LetStarBinding` (declares it); a later assignment to an
    /// already-declared name emits an `Assign` (re-binds, sets
    /// `Feature::MutableBindings` via the validator).
    fn lower_assignment(
        &mut self,
        assign: &GrammarASTNode,
        lhs_list: &GrammarASTNode,
        suffix: &GrammarASTNode,
    ) -> Result<Lowered, PythonLowerError> {
        // The RHS lives in the suffix's `expression_list`.
        let rhs_list = self.expect_single_kind(suffix, "expression_list")?;

        // M2 supports exactly one target = one value.  Tuple/chained
        // assignment (`a, b = …`, `a = b = …`) is deferred.
        let target_node = self.single_expr(lhs_list)?;
        let rhs_node = self.single_expr(rhs_list)?;

        // The target must be a bare name (`x`).  Attribute/subscript
        // targets (`obj.x = …`, `xs[i] = …`) are deferred.
        let name = match self.target_name(target_node)? {
            Some(name) => name,
            None => {
                return Err(self.err_at(
                    target_node,
                    "unsupported: assignment target is not a bare name (deferred)".to_string(),
                ))
            }
        };

        // Lower the RHS *before* declaring the name so a self-referential
        // first binding (`x = x`) correctly sees `x` as still-unbound and
        // raises an unresolved-name error (Python's behaviour).
        let value = self.lower_expr(rhs_node)?;
        let span = self.span_of(assign);

        if self.declared.contains(&name) {
            // Re-assignment to an already-declared local.  Emitting an
            // `Assign` means the module mutates a binding, so declare the
            // feature (the validator also observes it; declaring it here
            // keeps the manifest comparison balanced).
            self.observed.add(Feature::MutableBindings);
            Ok(Lowered::Stmt(Box::new(Stmt::Assign {
                name,
                scope: Scope::Local,
                value,
                span,
            })))
        } else {
            // First occurrence: declare via sequential `let*` so later
            // statements (and later RHS) can see it.
            self.declared.insert(name.clone());
            Ok(Lowered::Stmt(Box::new(Stmt::LetStarBinding {
                name,
                sir_type: None,
                value,
                span,
            })))
        }
    }

    /// If `node` is an expression that is *just* a bare `Name` atom,
    /// return that name.  Used to recognise an assignment target.  Returns
    /// `Ok(None)` for any non-trivial expression (an operator, literal,
    /// call, …), which the caller reports as an unsupported target.
    fn target_name(&self, node: &GrammarASTNode) -> Result<Option<String>, PythonLowerError> {
        // Peel single-child wrappers down to the leaf, exactly like the
        // expression peeler but without lowering — we only want to know if
        // the bottom is a single `Name` token.
        let mut cur = node;
        let mut depth = 0usize;
        loop {
            if depth > MAX_EXPR_DEPTH {
                return Err(self.err_at(node, "assignment target nesting too deep".to_string()));
            }
            if let Some(tok) = cur.token() {
                if matches!(tok.type_, lexer::token::TokenType::Name)
                    && tok.type_name.is_none()
                {
                    return Ok(Some(tok.value.clone()));
                }
                return Ok(None);
            }
            let kids = child_nodes(cur);
            match kids.as_slice() {
                [only] if cur.children.len() == 1 => {
                    cur = only;
                    depth += 1;
                }
                _ => return Ok(None),
            }
        }
    }

    // -------------------------------------------------------------------
    // expression → Expr
    // -------------------------------------------------------------------

    /// Lower an expression node, recognising operators and peeling the
    /// precedence-rule onion down to literals / variable references.
    fn lower_expr(&mut self, node: &GrammarASTNode) -> Result<Expr, PythonLowerError> {
        self.lower_expr_d(node, 0)
    }

    /// Depth-tracked core of [`Self::lower_expr`].
    ///
    /// Python's precedence grammar wraps every expression in a deep
    /// single-child chain, and operators / grouping / source nesting all
    /// add depth.  Every recursive call below increments `depth`, so the
    /// recursion depth tracks the *input* nesting depth.  `compile` is a
    /// public entry point taking an arbitrary CST, so we bound this
    /// ourselves: past `MAX_EXPR_DEPTH` we return a positioned error
    /// instead of recursing further (a Rust stack overflow cannot be
    /// caught, so this is the only way to keep `compile` total).
    ///
    /// The recursion is structural — each call descends into a strictly
    /// smaller sub-tree of the (finite) CST — so it always terminates; the
    /// depth cap only guards against the *native stack* on pathologically
    /// deep input, never against non-termination.
    fn lower_expr_d(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Expr, PythonLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }

        // Operator rules: handle each *branching* precedence level.  Each
        // returns `Ok(Some(expr))` when it matched the operator shape, or
        // `Ok(None)` to fall through to the generic peel (the rule was a
        // single-child wrapper this time).
        match node.rule_name.as_str() {
            "or_expr" => {
                if let Some(e) = self.try_logical(node, depth, "or")? {
                    return Ok(e);
                }
            }
            "and_expr" => {
                if let Some(e) = self.try_logical(node, depth, "and")? {
                    return Ok(e);
                }
            }
            "not_expr" => {
                if let Some(e) = self.try_not(node, depth)? {
                    return Ok(e);
                }
            }
            "comparison" => {
                if let Some(e) = self.try_comparison(node, depth)? {
                    return Ok(e);
                }
            }
            "arith" | "term" => {
                if let Some(e) = self.try_binary_arith(node, depth)? {
                    return Ok(e);
                }
            }
            "factor" => {
                if let Some(e) = self.try_unary_factor(node, depth)? {
                    return Ok(e);
                }
            }
            _ => {}
        }

        // A `leaf` node — exactly one child that is a Token.  This is
        // where every literal and bare name bottoms out.
        if let Some(tok) = node.token() {
            return self.lower_leaf_token(node, tok);
        }

        // Generic peel: a single Node child → recurse.  Any other shape
        // (zero children, or >1 node children, or a Token sibling) is a
        // construct M2 does not handle (call, subscript, attribute, …).
        let kids = child_nodes(node);
        match kids.as_slice() {
            [only] if node.children.len() == 1 => self.lower_expr_d(only, depth + 1),
            _ => Err(self.err_at(
                node,
                format!("unsupported: {} (deferred to a later milestone)", node.rule_name),
            )),
        }
    }

    /// `or_expr` / `and_expr`: `a (op b)+` where `op` is the keyword
    /// `or`/`and`.  Lowers to left-nested `LogicalOr`/`LogicalAnd`
    /// short-circuit nodes (so `a and b and c` is `(a and b) and c`).
    /// Returns `Ok(None)` when the node is a single-child wrapper (no
    /// operator present at this level).
    fn try_logical(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        keyword: &str,
    ) -> Result<Option<Expr>, PythonLowerError> {
        // Operands are the node children; operators are the `keyword`
        // tokens between them.  A bare wrapper has one node child and no
        // operator token → fall through.
        let operands = child_nodes(node);
        let has_kw = node.children.iter().any(|c| {
            matches!(c, ASTNodeOrToken::Token(t)
                if t.type_ == lexer::token::TokenType::Keyword && t.value == keyword)
        });
        if !has_kw {
            return Ok(None);
        }
        if operands.len() < 2 {
            // A keyword token but fewer than two operands is malformed.
            return Err(self.err_at(node, format!("malformed `{keyword}` expression")));
        }

        // Fold left: ((o0 op o1) op o2) …  Each operand is one precedence
        // level lower, so descend with depth+1.
        let mut acc = self.lower_expr_d(operands[0], depth + 1)?;
        for operand in &operands[1..] {
            let rhs = self.lower_expr_d(operand, depth + 1)?;
            let span = acc.span().clone();
            acc = if keyword == "and" {
                Expr::LogicalAnd {
                    lhs: Box::new(acc),
                    rhs: Box::new(rhs),
                    span,
                }
            } else {
                Expr::LogicalOr {
                    lhs: Box::new(acc),
                    rhs: Box::new(rhs),
                    span,
                }
            };
        }
        self.observed.add(Feature::ShortCircuit);
        Ok(Some(acc))
    }

    /// `not_expr`: `not <not_expr>` → `BuiltinCall("not", [operand])`.
    /// Returns `Ok(None)` when there is no leading `not` keyword (the rule
    /// was a single-child wrapper around a `comparison`).
    fn try_not(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Option<Expr>, PythonLowerError> {
        let leads_with_not = matches!(
            node.children.first(),
            Some(ASTNodeOrToken::Token(t))
                if t.type_ == lexer::token::TokenType::Keyword && t.value == "not"
        );
        if !leads_with_not {
            return Ok(None);
        }
        // The operand is the single node child after the keyword.
        let operand_node = child_nodes(node).into_iter().next().ok_or_else(|| {
            self.err_at(node, "malformed `not` expression (missing operand)".to_string())
        })?;
        let operand = self.lower_expr_d(operand_node, depth + 1)?;
        let span = self.span_of(node);
        Ok(Some(Expr::BuiltinCall {
            name: "not".to_string(),
            args: vec![operand],
            effects: EffectSet::PURE,
            span,
        }))
    }

    /// `comparison`: `a comp_op b (comp_op c)…` → left-nested
    /// `BuiltinCall(op, [lhs, rhs])`.  Each `comp_op` wraps the operator
    /// token; we map `==`→`"="`, `!=`→`"!="`, and the ordering ops to
    /// their literal spelling.  Returns `Ok(None)` for a bare wrapper.
    fn try_comparison(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Option<Expr>, PythonLowerError> {
        // children: operand, comp_op, operand, comp_op, operand, …
        // We need at least one `comp_op` to be an actual comparison.
        let has_comp_op = node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "comp_op"));
        if !has_comp_op {
            return Ok(None);
        }

        // Walk the children left to right, alternating operand / comp_op.
        let mut acc: Option<Expr> = None;
        let mut pending_op: Option<String> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(n) if n.rule_name == "comp_op" => {
                    pending_op = Some(self.comp_op_name(n)?);
                }
                ASTNodeOrToken::Node(n) => {
                    let operand = self.lower_expr_d(n, depth + 1)?;
                    acc = Some(match (acc.take(), pending_op.take()) {
                        // First operand: nothing to combine yet.
                        (None, _) => operand,
                        // lhs op operand → builtin compare.
                        (Some(lhs), Some(op)) => {
                            let span = lhs.span().clone();
                            Expr::BuiltinCall {
                                name: op,
                                args: vec![lhs, operand],
                                effects: EffectSet::PURE,
                                span,
                            }
                        }
                        // operand with no preceding comp_op — malformed.
                        (Some(_), None) => {
                            return Err(self.err_at(node, "malformed comparison".to_string()))
                        }
                    });
                }
                ASTNodeOrToken::Token(_) => {
                    // No bare operator tokens directly under `comparison`
                    // (they live inside `comp_op`); ignore defensively.
                }
            }
        }
        match acc {
            Some(e) => Ok(Some(e)),
            None => Err(self.err_at(node, "empty comparison".to_string())),
        }
    }

    /// Map a `comp_op` node's inner operator token to its SIR builtin
    /// name.  `==`→`"="` and `!=`→`"!="`; the ordering operators keep
    /// their literal spelling.
    fn comp_op_name(&self, comp_op: &GrammarASTNode) -> Result<String, PythonLowerError> {
        let tok = comp_op.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some(t),
            ASTNodeOrToken::Node(_) => None,
        });
        let tok = tok.ok_or_else(|| {
            self.err_at(comp_op, "comparison operator missing token".to_string())
        })?;
        let name = match tok.value.as_str() {
            "==" => "=",
            "!=" => "!=",
            "<" => "<",
            ">" => ">",
            "<=" => "<=",
            ">=" => ">=",
            other => {
                return Err(self.err_at(
                    comp_op,
                    format!("unsupported comparison operator `{other}` (deferred)"),
                ))
            }
        };
        Ok(name.to_string())
    }

    /// `arith` (`+`/`-`) and `term` (`*`/`/`/`%`): a left-associative run
    /// of binary operators with operator tokens between operands.  Lowers
    /// to left-nested `BuiltinCall(op, [lhs, rhs])`.  Returns `Ok(None)`
    /// for a bare single-operand wrapper.
    fn try_binary_arith(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Option<Expr>, PythonLowerError> {
        // Need at least one operator token to be a real binary op.
        let has_op = node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Token(t) if is_arith_op(&t.value)));
        if !has_op {
            return Ok(None);
        }

        let mut acc: Option<Expr> = None;
        let mut pending_op: Option<String> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Token(t) if is_arith_op(&t.value) => {
                    pending_op = Some(t.value.clone());
                }
                ASTNodeOrToken::Node(n) => {
                    let operand = self.lower_expr_d(n, depth + 1)?;
                    acc = Some(match (acc.take(), pending_op.take()) {
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
                            return Err(self.err_at(node, "malformed arithmetic expression".to_string()))
                        }
                    });
                }
                ASTNodeOrToken::Token(_) => {
                    // A non-operator token under arith/term is unexpected.
                }
            }
        }
        match acc {
            Some(e) => Ok(Some(e)),
            None => Err(self.err_at(node, "empty arithmetic expression".to_string())),
        }
    }

    /// `factor`: `(- | + | ~) <factor>`.  We handle unary `-` and `+`:
    ///
    /// - `-<numeric literal>` constant-folds to a negative literal
    ///   (carried from M1, because the spec lists `-7 ⇒ IntLit`).
    /// - `-<non-literal>` → `BuiltinCall("neg", [operand])`.
    /// - `+<operand>` is the identity — we drop it and return the operand
    ///   unchanged (no SIR node for unary plus).
    /// - `~<operand>` (bitwise NOT) is deferred.
    ///
    /// Returns `Ok(None)` for a bare single-child `factor` (no unary op).
    fn try_unary_factor(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Option<Expr>, PythonLowerError> {
        // Unary shape is exactly `[Token(op), Node(inner)]`.
        if node.children.len() != 2 {
            return Ok(None);
        }
        let (lead, inner) = (&node.children[0], &node.children[1]);
        let op = match lead {
            ASTNodeOrToken::Token(t) => t.value.as_str(),
            ASTNodeOrToken::Node(_) => return Ok(None),
        };
        let inner = match inner {
            ASTNodeOrToken::Node(n) => n,
            ASTNodeOrToken::Token(_) => return Ok(None),
        };

        let operand = self.lower_expr_d(inner, depth + 1)?;
        match op {
            "-" => match operand {
                // Constant-fold negation of numeric literals.
                Expr::IntLit { value, span } => Ok(Some(Expr::IntLit {
                    value: value.wrapping_neg(),
                    span,
                })),
                Expr::FloatLit { value, span } => Ok(Some(Expr::FloatLit {
                    value: -value,
                    span,
                })),
                // `-x` on a non-literal → builtin negate.
                other => {
                    let span = other.span().clone();
                    Ok(Some(Expr::BuiltinCall {
                        name: "neg".to_string(),
                        args: vec![other],
                        effects: EffectSet::PURE,
                        span,
                    }))
                }
            },
            // Unary plus is the identity in SIR — return the operand as-is.
            "+" => Ok(Some(operand)),
            // `~` (bitwise NOT) and any other unary token are deferred.
            other => Err(self.err_at(
                node,
                format!("unsupported unary operator `{other}` (deferred)"),
            )),
        }
    }

    /// Turn a leaf token into the matching SIR expression — a literal, or
    /// (M2) a variable reference for a bare `Name`.
    ///
    /// Token classification (learned by inspecting real parses):
    ///
    /// | token                                   | SIR node               |
    /// |-----------------------------------------|------------------------|
    /// | `type_name == "INT"`                    | `IntLit`               |
    /// | `type_name == "FLOAT"`                  | `FloatLit` (+Floats)   |
    /// | Keyword `True` / `False`                | `BoolLit`              |
    /// | Keyword `None`                          | `NilLit`               |
    /// | String token                            | `StrLit` (+Strings)    |
    /// | `Name` (no `type_name`)                 | `VarRef` (scope rslv)  |
    fn lower_leaf_token(
        &mut self,
        node: &GrammarASTNode,
        tok: &lexer::token::Token,
    ) -> Result<Expr, PythonLowerError> {
        let span = self.span_of(node);
        let type_name = tok.type_name.as_deref();

        match (type_name, tok.type_, tok.value.as_str()) {
            (Some("INT"), _, text) => {
                let value: i64 = text
                    .parse()
                    .map_err(|_| self.err_at(node, format!("invalid integer literal `{text}`")))?;
                Ok(Expr::IntLit { value, span })
            }
            (Some("FLOAT"), _, text) => {
                let value: f64 = text
                    .parse()
                    .map_err(|_| self.err_at(node, format!("invalid float literal `{text}`")))?;
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
            // A bare `Name` token is a variable reference.  Resolve its
            // scope against the names bound so far in this scope.
            (None, lexer::token::TokenType::Name, name) => self.resolve_var(node, name, span),
            // Anything else is genuinely unsupported.
            _ => Err(self.err_at(
                node,
                format!("unsupported token `{}` (deferred to a later milestone)", tok.value),
            )),
        }
    }

    /// Resolve a bare name reference to a scoped `VarRef`.
    ///
    /// M2 scope model (per SIR17): the only binding form so far is a
    /// top-level assignment, which becomes a *local* of `main`.  So a name
    /// bound earlier resolves as `Scope::Local`; an unbound name is an
    /// error (Python raises `NameError` at runtime, and we have no
    /// builtins wired up in M2 — `print`/`len`/`range` arrive with calls
    /// in M3).
    fn resolve_var(
        &mut self,
        node: &GrammarASTNode,
        name: &str,
        span: Span,
    ) -> Result<Expr, PythonLowerError> {
        if self.declared.contains(name) {
            Ok(Expr::VarRef {
                name: name.to_string(),
                scope: Scope::Local,
                span,
            })
        } else {
            Err(self.err_at(node, format!("unresolved name `{name}`")))
        }
    }

    // -------------------------------------------------------------------
    // helpers
    // -------------------------------------------------------------------

    /// Assert `node.rule_name == expected` and that it has exactly one
    /// child *node* whose rule name is in `allowed`; return that child.
    /// Used to walk the fixed `statement → … → assign_stmt` spine while
    /// emitting precise errors when an unexpected (compound-statement /
    /// `global`/`nonlocal`) shape shows up.
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
        let kids = child_nodes(node);
        match kids.as_slice() {
            [child] if allowed.contains(&child.rule_name.as_str()) => Ok(child),
            [child] => Err(self.err_at(
                child,
                format!("unsupported: {} (deferred to a later milestone)", child.rule_name),
            )),
            _ => Err(self.err_at(
                node,
                format!("unsupported: {} with multiple parts (deferred)", expected),
            )),
        }
    }

    /// Find the single child node of `node` whose `rule_name == kind`.
    /// Errors if absent.  (`assign_stmt` and `assign_suffix` both carry an
    /// `expression_list` child this way.)
    fn expect_single_kind<'a>(
        &self,
        node: &'a GrammarASTNode,
        kind: &str,
    ) -> Result<&'a GrammarASTNode, PythonLowerError> {
        child_nodes(node)
            .into_iter()
            .find(|n| n.rule_name == kind)
            .ok_or_else(|| self.err_at(node, format!("expected a `{kind}` child")))
    }

    /// An `expression_list` of exactly one element yields that single
    /// `expression`; multi-element lists (tuples / multi-target) are
    /// deferred.
    fn single_expr<'a>(
        &self,
        list: &'a GrammarASTNode,
    ) -> Result<&'a GrammarASTNode, PythonLowerError> {
        let exprs = child_nodes(list);
        match exprs.as_slice() {
            [only] => Ok(only),
            _ => Err(self.err_at(
                list,
                "unsupported: multi-element expression list (deferred)".to_string(),
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

/// Is `value` one of the binary arithmetic operator spellings the lowerer
/// recognises under `arith` (`+`/`-`) and `term` (`*`/`/`/`%`)?
fn is_arith_op(value: &str) -> bool {
    matches!(value, "+" | "-" | "*" | "/" | "%")
}
