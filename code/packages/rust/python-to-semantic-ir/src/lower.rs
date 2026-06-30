//! The lowering pass from `python_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **milestone M3**.
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
//! # What M3 adds
//!
//! M3 adds **control flow**.  A `statement` may now wrap a
//! `compound_stmt` (`if_stmt` / `while_stmt` / `for_stmt`) in addition to
//! a `simple_stmt`:
//!
//! - `if_stmt` — the parser flattens `if`/`elif`/`else` into one node; we
//!   collect the `(cond, suite)` clauses + optional `else`, then fold
//!   right-to-left into nested [`Expr::If`].  A trailing `if` becomes the
//!   block value; otherwise it is wrapped as a `Stmt::ExprStmt`.
//! - `while_stmt` → [`Stmt::While`].
//! - `for_stmt` → [`Stmt::ForRange`] when the iterable is a literal
//!   `range(...)` call (arity 1/2/3 → `start`/`stop`/`step`), else
//!   [`Stmt::ForEach`].
//!
//! Each suite lowers to a [`Block`] via [`Lowerer::lower_suite`].  The
//! lowerer's declared-name table became a **stack** (was a `HashSet`) so
//! block-local names — including the loop variable — are scoped exactly
//! as the SIR validator scopes them (`mark`/`rewind`), keeping lowering
//! and validation in lock-step.  A `MAX_BLOCK_DEPTH` guard bounds nested
//! control-flow recursion.
//!
//! ## Still deferred (later milestones)
//!
//! - calls / functions / `def` / `lambda`                → M4+
//! - collections (lists / dicts / indexing)              → M5+
//! - tuple `for` targets, `with` / `try`, `global` /
//!   `nonlocal`, multi-target assignment                 → deferred
//!
//! Unhandled rules produce a clear `PythonLowerError` rather than
//! silently dropping source, so later milestones can slot their
//! extractors in exactly where the error is raised today.

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Scope, Span, Stmt,
};

/// Maximum expression-nesting depth the lowerer will descend before
/// bailing with an error.  The expression-precedence chain is ~20 levels
/// deep for a *bare* literal, and explicit grouping/unary operators add a
/// level each, so a healthy human-written expression sits far below this.
/// The cap exists purely to turn pathologically deep (but parseable)
/// input — `((((…42…))))`, `------…42`, `a and a and a and …` — into a
/// clean `PythonLowerError` instead of a native stack overflow (which
/// aborts unrecoverably and cannot be caught in Rust).
const MAX_EXPR_DEPTH: usize = 256;

/// Maximum *statement-block* nesting depth (M3).  Each `if` / `while` /
/// `for` body re-enters [`Lowerer::lower_suite`] one level deeper, so a
/// pathological tower of `while c:\n while c:\n …` would recurse without
/// bound.  Mirroring [`MAX_EXPR_DEPTH`]'s role for expressions, this cap
/// turns deeply nested control flow into a clean positioned
/// `PythonLowerError` instead of a native (uncatchable) stack overflow.
/// It is generous: real source nests a handful of levels, far below this.
const MAX_BLOCK_DEPTH: usize = 256;

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
    /// Names bound in the scope chain *so far*, as a stack (M3).  Drives
    /// first-occurrence detection: the first `x = …` declares (`LetStar`),
    /// a later `x = …` re-assigns (`Assign`), and a `VarRef` to a name in
    /// this stack resolves as `Scope::Local`.
    ///
    /// Why a stack rather than a flat `HashSet` (M2's design)?  M3 adds
    /// nested statement *blocks* — loop and `if`-branch bodies.  The SIR
    /// validator scopes block-local names with `mark()`/`rewind()`: a name
    /// bound inside a loop body (or the loop variable itself) is **not**
    /// visible once the body ends.  We mirror that exactly here with
    /// [`Self::scope_mark`] / [`Self::scope_rewind`] so the names the
    /// lowerer resolves and the names the validator accepts stay in
    /// lock-step — otherwise a lowered module could fail its own
    /// round-trip validation.  Membership is by linear scan (scopes are
    /// tiny in practice), matching the validator's `LocalEnv`.
    declared: Vec<String>,
}

impl Lowerer {
    fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            observed: FeatureManifest::new(),
            declared: Vec::new(),
        }
    }

    // -------------------------------------------------------------------
    // scope stack (M3): mirror the validator's `LocalEnv` mark/rewind so
    // block-local bindings (loop vars, names first-bound inside a body)
    // do not leak past the block — exactly as the validator scopes them.
    // -------------------------------------------------------------------

    /// Is `name` bound somewhere in the current scope chain?
    fn is_declared(&self, name: &str) -> bool {
        self.declared.iter().any(|n| n == name)
    }

    /// Bind `name` in the current (innermost) scope.
    fn declare(&mut self, name: String) {
        self.declared.push(name);
    }

    /// Remember the current scope depth before entering a nested block.
    fn scope_mark(&self) -> usize {
        self.declared.len()
    }

    /// Drop every name bound since `mark`, leaving the enclosing scope.
    fn scope_rewind(&mut self, mark: usize) {
        self.declared.truncate(mark);
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
                items.push(self.lower_statement(stmt, 0)?);
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
    // statement → assignment, expression, or compound (if/while/for)
    // -------------------------------------------------------------------

    /// Lower a `statement`.
    ///
    /// A `statement` wraps exactly one of:
    ///
    /// - a `simple_stmt` — an *assignment* (`x = expr`) or a bare
    ///   *expression statement*; or
    /// - a `compound_stmt` — control flow (`if` / `while` / `for`), added
    ///   in M3.
    ///
    /// `def` / `class` / `with` / `try` also arrive as `compound_stmt`
    /// children and are still rejected with a clear "unsupported" error;
    /// `global` / `nonlocal` take a different `small_stmt` branch and are
    /// likewise rejected.
    ///
    /// `depth` is the statement-block nesting depth — top-level statements
    /// are depth 0; each loop / `if`-branch body recurses one level deeper.
    /// It bounds [`MAX_BLOCK_DEPTH`] so pathologically nested control flow
    /// fails cleanly instead of overflowing the native stack.
    fn lower_statement(
        &mut self,
        stmt: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, PythonLowerError> {
        if depth > MAX_BLOCK_DEPTH {
            return Err(self.err_at(
                stmt,
                format!("control-flow nesting too deep (exceeds {MAX_BLOCK_DEPTH} levels)"),
            ));
        }
        if stmt.rule_name != "statement" {
            return Err(self.err_at(
                stmt,
                format!("expected `statement`, got `{}`", stmt.rule_name),
            ));
        }

        // `statement` has a single node child: either `simple_stmt` or
        // (M3) `compound_stmt`.
        let inner = match child_nodes(stmt).as_slice() {
            [only] => *only,
            _ => {
                return Err(self.err_at(
                    stmt,
                    "unsupported: statement with multiple parts (deferred)".to_string(),
                ))
            }
        };
        match inner.rule_name.as_str() {
            "simple_stmt" => self.lower_simple_stmt(inner),
            "compound_stmt" => self.lower_compound_stmt(inner, depth),
            other => Err(self.err_at(
                inner,
                format!("unsupported: {other} (deferred to a later milestone)"),
            )),
        }
    }

    /// Lower a `simple_stmt` — an assignment or a bare expression.
    fn lower_simple_stmt(&mut self, simple: &GrammarASTNode) -> Result<Lowered, PythonLowerError> {
        // Descend the fixed spine: simple_stmt → small_stmt → assign_stmt.
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

        if self.is_declared(&name) {
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
            self.declare(name.clone());
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
    // compound statements (M3): if / while / for
    // -------------------------------------------------------------------

    /// Lower a `compound_stmt` — control flow.  A `compound_stmt` wraps a
    /// single rule node: `if_stmt`, `while_stmt`, or `for_stmt` (M3).
    /// `def` / `class_def` / `with_stmt` / `try_stmt` also surface here and
    /// are deferred to later milestones with a clear error.
    ///
    /// `if` lowers to an [`Expr::If`] returned as a [`Lowered::Expr`], so a
    /// trailing `if` can become the block *value* (Python's `if` is a
    /// statement, but SIR models it as an expression — see [`Self::lower_if`]).
    /// `while` / `for` are pure statements ([`Lowered::Stmt`]).
    fn lower_compound_stmt(
        &mut self,
        compound: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, PythonLowerError> {
        let inner = match child_nodes(compound).as_slice() {
            [only] => *only,
            _ => {
                return Err(self.err_at(
                    compound,
                    "unsupported: compound statement with multiple parts (deferred)".to_string(),
                ))
            }
        };
        match inner.rule_name.as_str() {
            "if_stmt" => Ok(Lowered::Expr(self.lower_if(inner, depth)?)),
            "while_stmt" => Ok(Lowered::Stmt(Box::new(self.lower_while(inner, depth)?))),
            "for_stmt" => Ok(Lowered::Stmt(Box::new(self.lower_for(inner, depth)?))),
            other => Err(self.err_at(
                inner,
                format!("unsupported: {other} (deferred to a later milestone)"),
            )),
        }
    }

    /// Lower an `if_stmt` into a nested chain of [`Expr::If`].
    ///
    /// The parser flattens the whole `if` / `elif` / `else` construct into
    /// **one** `if_stmt` node whose children are an ordered token+node
    /// stream:
    ///
    /// ```text
    /// if_stmt:
    ///   KW "if"   expression  ":"  suite          ← the leading clause
    ///   KW "elif" expression  ":"  suite          ← zero or more elif clauses
    ///   …
    ///   KW "else" ":"  suite                       ← optional trailing else
    /// ```
    ///
    /// We walk that stream into a list of `(cond, suite)` clauses plus an
    /// optional `else` suite, then fold it **right-to-left** so each `elif`
    /// becomes the `else_branch` of the clause before it:
    ///
    /// ```text
    /// if c1: B1 elif c2: B2 else: B3
    ///   ⇒ If { c1, B1, else: If { c2, B2, else: B3 } }
    /// ```
    ///
    /// A missing `else` becomes an empty `else_branch` block whose value is
    /// `NilLit` (SIR requires both branches; an `if` with no `else` yields
    /// nil on the false path, matching Python where the suite simply does
    /// not run).  `if` adds no manifest feature — it is a SIR v0 construct.
    fn lower_if(&mut self, if_stmt: &GrammarASTNode, depth: usize) -> Result<Expr, PythonLowerError> {
        // Each clause is a guard expression paired with its suite; `else`
        // (if present) is a bare suite with no guard.
        struct Clause<'a> {
            cond: &'a GrammarASTNode,
            suite: &'a GrammarASTNode,
        }
        let mut clauses: Vec<Clause> = Vec::new();
        let mut else_suite: Option<&GrammarASTNode> = None;

        // Walk the flat child stream.  A keyword token (`if`/`elif`/`else`)
        // opens a clause; the next `expression` is its guard (absent for
        // `else`); the next `suite` is its body.
        let mut pending_cond: Option<&GrammarASTNode> = None;
        let mut in_else = false;
        for child in &if_stmt.children {
            match child {
                ASTNodeOrToken::Token(t)
                    if t.type_ == lexer::token::TokenType::Keyword
                        && (t.value == "if" || t.value == "elif") =>
                {
                    in_else = false;
                }
                ASTNodeOrToken::Token(t)
                    if t.type_ == lexer::token::TokenType::Keyword && t.value == "else" =>
                {
                    in_else = true;
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "expression" => {
                    pending_cond = Some(n);
                }
                ASTNodeOrToken::Node(n) if n.rule_name == "suite" => {
                    if in_else {
                        else_suite = Some(n);
                    } else {
                        let cond = pending_cond.take().ok_or_else(|| {
                            self.err_at(n, "malformed if: clause has no condition".to_string())
                        })?;
                        clauses.push(Clause { cond, suite: n });
                    }
                }
                // Other tokens (`:`) and any stray nodes are ignored.
                _ => {}
            }
        }

        if clauses.is_empty() {
            return Err(self.err_at(if_stmt, "malformed if: no clauses".to_string()));
        }

        let if_span = self.span_of(if_stmt);

        // Build the final `else` block first (it is the deepest branch).
        let mut else_branch: Block = match else_suite {
            Some(s) => self.lower_suite(s, depth + 1)?,
            None => empty_block(if_span.clone()),
        };

        // Fold clauses right-to-left so earlier `elif`s wrap later ones.
        for clause in clauses.into_iter().rev() {
            let cond = self.lower_expr(clause.cond)?;
            let then_branch = self.lower_suite(clause.suite, depth + 1)?;
            let span = cond.span().clone();
            let folded = Expr::If {
                cond: Box::new(cond),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
                span,
            };
            // The just-built `If` becomes the else-branch of the next
            // (outer) clause: wrap it in a one-value block.
            else_branch = value_block(folded);
        }

        // After the fold, `else_branch` is a block whose value is the
        // outermost `If`.  Unwrap it back to the bare `If` expression.
        match else_branch.value {
            Expr::If { .. } => Ok(else_branch.value),
            other => Ok(other),
        }
    }

    /// Lower a `while_stmt` into [`Stmt::While`].
    ///
    /// `while_stmt` children: `KW "while"`, `expression` (the condition),
    /// `:`, `suite` (the body).  The body is lowered into a [`Block`]; the
    /// loop adds [`Feature::Loops`].
    fn lower_while(
        &mut self,
        while_stmt: &GrammarASTNode,
        depth: usize,
    ) -> Result<Stmt, PythonLowerError> {
        let cond_node = self
            .first_child_named(while_stmt, "expression")
            .ok_or_else(|| self.err_at(while_stmt, "malformed while: no condition".to_string()))?;
        let suite = self
            .first_child_named(while_stmt, "suite")
            .ok_or_else(|| self.err_at(while_stmt, "malformed while: no body".to_string()))?;

        let cond = self.lower_expr(cond_node)?;
        let body = self.lower_suite(suite, depth + 1)?;
        self.observed.add(Feature::Loops);
        Ok(Stmt::While {
            cond,
            body,
            span: self.span_of(while_stmt),
        })
    }

    /// Lower a `for_stmt` into either [`Stmt::ForRange`] (when the iterable
    /// is a literal `range(...)` call) or [`Stmt::ForEach`] (any other
    /// iterable).
    ///
    /// `for_stmt` children: `KW "for"`, `target_list` (the loop var(s)),
    /// `KW "in"`, `expression_list` (the iterable), `:`, `suite` (body).
    ///
    /// M3 supports exactly one bare-name target (`for i in …`).  Tuple
    /// targets (`for k, v in …`) are deferred.  The loop variable is bound
    /// **inside the body's scope only** (mirroring the validator): we push
    /// a scope mark, declare the var, lower the body, then rewind.
    fn lower_for(&mut self, for_stmt: &GrammarASTNode, depth: usize) -> Result<Stmt, PythonLowerError> {
        let target_list = self
            .first_child_named(for_stmt, "target_list")
            .ok_or_else(|| self.err_at(for_stmt, "malformed for: no target".to_string()))?;
        let iter_list = self
            .first_child_named(for_stmt, "expression_list")
            .ok_or_else(|| self.err_at(for_stmt, "malformed for: no iterable".to_string()))?;
        let suite = self
            .first_child_named(for_stmt, "suite")
            .ok_or_else(|| self.err_at(for_stmt, "malformed for: no body".to_string()))?;

        // The target must be a single bare name.  `target_list` holds one
        // or more `target` nodes; we accept exactly one whose leaf is a
        // `Name`.
        let targets = child_nodes(target_list);
        let var = match targets.as_slice() {
            [one] => match self.target_name(one)? {
                Some(name) => name,
                None => {
                    return Err(self.err_at(
                        one,
                        "unsupported: for-loop target is not a bare name (deferred)".to_string(),
                    ))
                }
            },
            _ => {
                return Err(self.err_at(
                    target_list,
                    "unsupported: tuple for-loop target (deferred)".to_string(),
                ))
            }
        };

        // The iterable is the single expression in `expression_list`.
        let iter_expr_node = self.single_expr(iter_list)?;
        let span = self.span_of(for_stmt);

        // Is the iterable a literal `range(...)` call?  If so, lower to
        // `ForRange`; otherwise lower the iterable expression and emit
        // `ForEach`.  We classify *before* binding the loop var because the
        // iterable is evaluated in the *enclosing* scope (the loop var is
        // not yet in scope), exactly as the validator checks it.
        let range = self.try_range_call(iter_expr_node)?;

        self.observed.add(Feature::Loops);

        match range {
            Some((start, stop, step)) => {
                // Bind the loop var inside the body's scope only.
                let mark = self.scope_mark();
                self.declare(var.clone());
                let body = self.lower_suite_no_mark(suite, depth + 1)?;
                self.scope_rewind(mark);
                Ok(Stmt::ForRange {
                    var,
                    start,
                    stop,
                    step,
                    body,
                    span,
                })
            }
            None => {
                let iter = self.lower_expr(iter_expr_node)?;
                let mark = self.scope_mark();
                self.declare(var.clone());
                let body = self.lower_suite_no_mark(suite, depth + 1)?;
                self.scope_rewind(mark);
                Ok(Stmt::ForEach {
                    var,
                    iter,
                    body,
                    span,
                })
            }
        }
    }

    /// Recognise a literal `range(...)` call and lower its arguments into
    /// `(start, stop, step)` expressions per Python's `range` arities:
    ///
    /// | call form            | start | stop | step |
    /// |----------------------|-------|------|------|
    /// | `range(n)`           | `0`   | `n`  | `1`  |
    /// | `range(a, b)`        | `a`   | `b`  | `1`  |
    /// | `range(a, b, c)`     | `a`   | `b`  | `c`  |
    ///
    /// Returns `Ok(None)` when the iterable is *not* a `range(...)` call
    /// (so the caller falls back to `ForEach`).  A `range` call with zero
    /// arguments or more than three is rejected (`range` requires 1–3 args).
    fn try_range_call(
        &mut self,
        iter: &GrammarASTNode,
    ) -> Result<Option<(Expr, Expr, Expr)>, PythonLowerError> {
        // Peel single-child wrappers down to the `primary` that carries the
        // `atom`(Name) + `suffix`(call) shape.
        let primary = match self.peel_to_primary(iter) {
            Some(p) => p,
            None => return Ok(None),
        };

        // A call `primary` is `atom suffix` where `suffix` is `( args )`.
        let kids = child_nodes(primary);
        let (callee, suffix) = match kids.as_slice() {
            [callee, suffix] if suffix.rule_name == "suffix" => (*callee, *suffix),
            _ => return Ok(None),
        };

        // The callee must be the bare name `range`.
        match self.target_name(callee)? {
            Some(name) if name == "range" => {}
            _ => return Ok(None),
        }

        // Collect the call's `argument` nodes (commas are tokens).
        let suffix_kids = child_nodes(suffix);
        let args: Vec<&GrammarASTNode> = suffix_kids
            .into_iter()
            .filter(|n| n.rule_name == "arguments")
            .flat_map(child_nodes)
            .filter(|n| n.rule_name == "argument")
            .collect();

        let span = self.span_of(primary);
        let int = |v: i64| Expr::IntLit { value: v, span: span.clone() };

        // One `argument` wraps one `expression`; lower it.
        let arg_expr = |me: &mut Self, a: &GrammarASTNode| -> Result<Expr, PythonLowerError> {
            let expr = me.single_arg_expr(a)?;
            me.lower_expr(expr)
        };

        match args.as_slice() {
            [n] => {
                let stop = arg_expr(self, n)?;
                Ok(Some((int(0), stop, int(1))))
            }
            [a, b] => {
                let start = arg_expr(self, a)?;
                let stop = arg_expr(self, b)?;
                Ok(Some((start, stop, int(1))))
            }
            [a, b, c] => {
                let start = arg_expr(self, a)?;
                let stop = arg_expr(self, b)?;
                let step = arg_expr(self, c)?;
                Ok(Some((start, stop, step)))
            }
            other => Err(self.err_at(
                primary,
                format!(
                    "range() takes 1 to 3 arguments, got {} (range with wrong arity)",
                    other.len()
                ),
            )),
        }
    }

    /// Peel an expression node down to the `primary` rule (the level that
    /// carries call/index/attribute suffixes), following single-child
    /// wrappers.  Returns `None` if no `primary` with a non-trivial shape
    /// is reached (e.g. a bare name peels past `primary` to its `atom`).
    fn peel_to_primary<'a>(&self, node: &'a GrammarASTNode) -> Option<&'a GrammarASTNode> {
        let mut cur = node;
        let mut depth = 0usize;
        loop {
            if depth > MAX_EXPR_DEPTH {
                return None;
            }
            if cur.rule_name == "primary" {
                return Some(cur);
            }
            match child_nodes(cur).as_slice() {
                [only] if cur.children.len() == 1 => {
                    cur = only;
                    depth += 1;
                }
                _ => return None,
            }
        }
    }

    /// An `argument` node wraps a single `expression`; return it.
    fn single_arg_expr<'a>(
        &self,
        arg: &'a GrammarASTNode,
    ) -> Result<&'a GrammarASTNode, PythonLowerError> {
        child_nodes(arg)
            .into_iter()
            .find(|n| n.rule_name == "expression")
            .ok_or_else(|| self.err_at(arg, "malformed call argument".to_string()))
    }

    /// Lower a `suite` (an indented statement block) into a [`Block`].
    ///
    /// A `suite` is `Newline Indent statement+ Dedent`.  Each `statement`
    /// lowers via [`Self::lower_statement`] (so nested control flow works).
    /// Block-value semantics mirror `main`'s top level: the trailing item,
    /// **if it is a bare expression**, becomes the block's value; everything
    /// before becomes a `Stmt` (bare expressions become `ExprStmt`s).  A
    /// suite ending in an assignment / loop yields a `NilLit` value.
    ///
    /// This variant introduces its own scope mark/rewind so names bound
    /// inside the suite do not leak to the enclosing scope (matching the
    /// validator's `check_block`).
    fn lower_suite(&mut self, suite: &GrammarASTNode, depth: usize) -> Result<Block, PythonLowerError> {
        let mark = self.scope_mark();
        let block = self.lower_suite_no_mark(suite, depth)?;
        self.scope_rewind(mark);
        Ok(block)
    }

    /// Like [`Self::lower_suite`] but **without** pushing/popping a scope
    /// mark — the caller manages scope (used by `for`, which must bind the
    /// loop variable across the body in the *same* scope frame, then rewind
    /// once afterwards).
    fn lower_suite_no_mark(
        &mut self,
        suite: &GrammarASTNode,
        depth: usize,
    ) -> Result<Block, PythonLowerError> {
        if suite.rule_name != "suite" {
            return Err(self.err_at(
                suite,
                format!("expected `suite`, got `{}`", suite.rule_name),
            ));
        }

        let mut items: Vec<Lowered> = Vec::new();
        for child in &suite.children {
            if let ASTNodeOrToken::Node(stmt) = child {
                if stmt.rule_name == "statement" {
                    items.push(self.lower_statement(stmt, depth)?);
                }
            }
            // Token children (Newline / Indent / Dedent) are ignored.
        }

        let span = self.span_of(suite);

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

        Ok(Block { stmts, value, span })
    }

    /// First *node* child of `node` whose `rule_name == name`.
    fn first_child_named<'a>(
        &self,
        node: &'a GrammarASTNode,
        name: &str,
    ) -> Option<&'a GrammarASTNode> {
        child_nodes(node).into_iter().find(|n| n.rule_name == name)
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
    /// Scope model (per SIR17): the binding forms so far are a top-level
    /// (or block-level) assignment and a `for` loop variable, both of
    /// which become *locals*.  A name bound earlier in the current scope
    /// chain resolves as `Scope::Local`; an unbound name is an error
    /// (Python raises `NameError` at runtime, and we have no builtins
    /// wired up yet — `print`/`len` arrive with calls in M4).
    fn resolve_var(
        &mut self,
        node: &GrammarASTNode,
        name: &str,
        span: Span,
    ) -> Result<Expr, PythonLowerError> {
        if self.is_declared(name) {
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

/// An empty `Block` whose value is `NilLit` — used for an absent `else`
/// branch (SIR's `If` always carries both branches; a missing `else`
/// yields nil on the false path, matching Python's "the suite just doesn't
/// run").
fn empty_block(span: Span) -> Block {
    Block {
        stmts: vec![],
        value: Expr::NilLit { span: span.clone() },
        span,
    }
}

/// A `Block` with no statements whose value is `expr` — used to nest one
/// `If` as the `else_branch` of another (an `elif` chain).
fn value_block(expr: Expr) -> Block {
    let span = expr.span().clone();
    Block {
        stmts: vec![],
        value: expr,
        span,
    }
}
