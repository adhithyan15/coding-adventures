//! JavaScript `GrammarASTNode` (CST) → `semantic_ir::Module` lowering.
//!
//! # What this file does (milestones M1 + M2 + M3)
//!
//! The [`javascript-parser`](coding_adventures_javascript_parser) crate
//! hands us a *concrete syntax tree* (CST): a [`GrammarASTNode`] whose
//! shape mirrors the ECMAScript grammar one-for-one.  Even a bare
//! literal like `42;` produces a deep spine of single-child wrapper
//! nodes — `program → source_element → statement →
//! expression_statement → expression → assignment_expression → … →
//! primary_expression → <token>`.  Twenty-odd rule layers, all of which
//! exist only to encode operator precedence, and *none* of which carry
//! information until one of them *branches* (an operator with two
//! operands, an assignment with a target and value, …).
//!
//! ## Milestone history
//!
//! - **M1** lowered **literals only**: walk a statement down to its
//!   single leaf token, classify the token, emit the matching SIR
//!   literal.  Anything non-literal was rejected.
//! - **M2** (this build) adds **variables and operators**.  The
//!   single-child *peel* in [`Lowerer::lower_expression`] is still the
//!   entry point: when it stops at a *branching* node it now dispatches
//!   on the rule name (`additive_expression`, `relational_expression`,
//!   `logical_and_expression`, `unary_expression`, …) instead of
//!   rejecting.  And `program` now threads a per-scope set of declared
//!   names so a bare identifier resolves to a [`VarRef`] and a
//!   `let`/`const`/`var` becomes a binding statement.
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
//! ## The operator truth table (M2 — learned by probing the parser)
//!
//! Each row's *rule node* is where the precedence spine **branches**:
//! the children are `[lhs, op_token, rhs, op_token, rhs, …]` (binary
//! rules are flat, left-associative chains), or `[op_token, operand]`
//! for a prefix `unary_expression`.  The middle/first token's *value*
//! (not its `TokenType`, which varies) carries the operator spelling.
//!
//! | JS source     | branching rule              | op value | SIR node                       |
//! |---------------|-----------------------------|----------|--------------------------------|
//! | `a + b`       | `additive_expression`       | `+`      | `BuiltinCall("+", [a, b])`     |
//! | `a - b`       | `additive_expression`       | `-`      | `BuiltinCall("-", [a, b])`     |
//! | `a * b`       | `multiplicative_expression` | `*`      | `BuiltinCall("*", [a, b])`     |
//! | `a / b`       | `multiplicative_expression` | `/`      | `BuiltinCall("/", [a, b])`     |
//! | `a % b`       | `multiplicative_expression` | `%`      | `BuiltinCall("%", [a, b])`     |
//! | `a < b`       | `relational_expression`     | `<`      | `BuiltinCall("<", [a, b])`     |
//! | `a > b`       | `relational_expression`     | `>`      | `BuiltinCall(">", [a, b])`     |
//! | `a <= b`      | `relational_expression`     | `<=`     | `BuiltinCall("<=", [a, b])`    |
//! | `a >= b`      | `relational_expression`     | `>=`     | `BuiltinCall(">=", [a, b])`    |
//! | `a == b`      | `equality_expression`       | `==`     | `BuiltinCall("=", [a, b])`     |
//! | `a === b`     | `equality_expression`       | `===`    | `BuiltinCall("=", [a, b])`     |
//! | `a != b`      | `equality_expression`       | `!=`     | `BuiltinCall("!=", [a, b])`    |
//! | `a !== b`     | `equality_expression`       | `!==`    | `BuiltinCall("!=", [a, b])`    |
//! | `a && b`      | `logical_and_expression`    | `&&`     | `LogicalAnd { a, b }`          |
//! | `a \|\| b`    | `logical_or_expression`     | `\|\|`   | `LogicalOr { a, b }`           |
//! | `!a`          | `unary_expression`          | `!`      | `BuiltinCall("not", [a])`      |
//! | `-a`          | `unary_expression`          | `-`      | `BuiltinCall("neg", [a])`      |
//!
//! ### Equality normalisation (a deliberate semantic change)
//!
//! JS has *two* equality families: loose (`==`/`!=`, with coercion) and
//! strict (`===`/`!==`, no coercion).  The IR has a single `=` /`!=`.
//! We map **both** JS families to the strict-shaped IR comparison
//! (`BuiltinCall("=")` / `BuiltinCall("!=")`).  This *changes semantics*
//! for the coercion cases — `null == undefined` is `true` in JS but
//! `false` under strict comparison.  This loss is spec-sanctioned for v0
//! (see SIR19 "Equality normalisation"); programs relying on loose
//! coercion are out of scope.
//!
//! ### Unary `-` on a numeric literal (constant fold)
//!
//! `-5` parses as a prefix `unary_expression` whose operand is the
//! literal `5`.  Rather than emit `BuiltinCall("neg", [IntLit(5)])` we
//! *constant-fold* it to `IntLit(-5)` (and `-3.25` to `FloatLit(-3.25)`).
//! This keeps the spec's `-7 → IntLit` row exact.  Unary `-` on any
//! *non-literal* operand (e.g. `-x`) stays `BuiltinCall("neg", [x])`.
//!
//! ## Variable model (M2)
//!
//! Everything lives inside the synthetic top-level `main`, so every
//! top-level binding is a *local* of `main` (`Scope::Local`).  We track a
//! `declared_locals` set as we lower statements in source order:
//!
//! - First sighting of a name as `let`/`const`/`var x = …` (or a bare
//!   `x = …` that has no prior binding) emits a binding statement and
//!   records the name.
//! - A subsequent `x = …` to an already-declared name emits
//!   `Stmt::Assign` (`Feature::MutableBindings`).
//! - A bare identifier reference resolves to `VarRef { scope: Local }`
//!   if it is declared, to `NilLit` for the exact spelling `undefined`,
//!   and otherwise to a positioned "unresolved name" [`JsLowerError`].
//!
//! Bindings lower to [`Stmt::LetStarBinding`] (sequential `let*`), **not**
//! [`Stmt::LetBinding`].  The SIR validator treats a run of consecutive
//! `LetBinding`s as a *parallel* group whose right-hand sides may not see
//! one another; JS `let`/`const` are sequentially scoped, so a perfectly
//! ordinary `let x = 1; const y = x + 1;` must validate.  `let*`'s
//! sequential semantics match JS exactly.  (The SIR19 spec coverage table
//! writes "LetBinding" generically for both kinds; this divergence is
//! noted there.)  `const` vs `let` vs `var` are not distinguished in v0
//! (the IR models no immutability constraint).
//!
//! ## Control flow (M3 — learned by probing the parser)
//!
//! M3 adds the four counting/branching control-flow shapes.  The CST
//! rule names and child layouts (precedence-wrapper layers elided):
//!
//! | JS source                                   | CST rule           | children                                                                     |
//! |---------------------------------------------|--------------------|------------------------------------------------------------------------------|
//! | `if (c) S`                                  | `if_statement`     | `[Kw("if"), (, expression, ), statement]`                                    |
//! | `if (c) S else T`                           | `if_statement`     | `[…, statement, Kw("else"), statement]`                                      |
//! | `while (c) S`                               | `while_statement`  | `[Kw("while"), (, expression, ), statement]`                                 |
//! | `for (let i=0; c; u) S`                     | `for_statement`    | `[Kw("for"), (, Kw("let"), binding_list, ;, expr(cond), ;, expr(update), ), statement]` |
//! | `for (const x of xs) S`                     | `for_of_statement` | `[Kw("for"), (, Kw("const"), binding_element, Name("of"), assignment_expression, ), statement]` |
//! | `{ S1; S2; }`                               | `block`            | `[{, statement*, }]`                                                          |
//!
//! ### `if` → [`Expr::If`]
//!
//! The IR's conditional is an *expression* ([`Expr::If`]) with `then_branch`
//! and `else_branch` [`Block`]s — there is no statement-level `if`.  So a JS
//! `if` *statement* lowers to a `Stmt::ExprStmt` wrapping an `Expr::If`.  A
//! missing `else` becomes a synthetic nil-valued empty `Block`.  An
//! **else-if chain** (`else if (…)`) is just the grammar nesting another
//! `if_statement` inside the `else` `statement`, so it recurses naturally
//! into a *nested* `Expr::If` living in the outer `else_branch`'s tail value.
//!
//! ### `while` → [`Stmt::While`]
//!
//! Direct: lower the condition expression and the body block.
//!
//! ### C-style `for` → [`Stmt::ForRange`] (canonical counting loops only)
//!
//! The IR has no general three-clause `for`; it has a half-open counting
//! [`Stmt::ForRange`] (`for var in range(start, stop, step)`).  We accept a
//! C-style `for` **only** when it matches the canonical counting shape and
//! extract `var`/`start`/`stop`/`step`:
//!
//! - **init** must be `let i = <start>` (a single `lexical_binding`/`var`
//!   declaration binding `i` to the start expression).
//! - **cond** must be `i < <stop>` or `i <= <stop>` on the *same* `i`.
//!   `<=` is rewritten to a half-open `<` by bumping the stop to
//!   `<stop> + 1` (`BuiltinCall("+", [stop, IntLit(1)])`).
//! - **update** must increment `i` by a constant `step` in one of:
//!   `i = i + <step>`, `i += <step>`, or `i++` (step = 1).
//!
//! Anything else — a different loop variable across clauses, a decrementing
//! or multiplicative update, a missing clause, a multi-binding init — is a
//! *non-canonical* loop we cannot faithfully represent as a `ForRange`, so
//! it is a positioned [`JsLowerError`] (deferred), never silently mangled.
//!
//! ### `for … of` → [`Stmt::ForEach`]
//!
//! `for (const x of xs)` binds `x` over the iterable `xs`.  Only the
//! single-identifier binding form is supported (destructuring is deferred).
//!
//! ### Block scoping
//!
//! A `{ … }` block and every control-flow body lower to a [`Block`].  Names
//! bound *inside* a body are block-scoped: we snapshot `declared_locals`
//! before lowering a body and restore it afterwards, so an inner `let` does
//! not leak to the enclosing scope.  This mirrors the SIR validator, which
//! marks/rewinds its `LocalEnv` around each `Block` and around a loop's
//! body (with the loop variable added only for that body).  The loop
//! variable is likewise bound into the body scope only.
//!
//! ### Recursion bound
//!
//! Statement-block nesting is bounded by [`MAX_STMT_DEPTH`] exactly as
//! operator recursion is bounded by [`MAX_EXPR_DEPTH`]: each nested body is
//! lowered with `depth + 1`, and an over-deep nest becomes an ordinary
//! positioned error rather than a stack overflow.

use lexer::token::{Token, TokenType};
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, ExportName, Expr, Feature, FeatureManifest, Function, Metadata, Module,
    Scope, Span, Stmt, CURRENT_SIR_VERSION,
};
use std::collections::HashSet;

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

/// Hard ceiling on operator-expression recursion depth.
///
/// `lower_expression` descends the precedence spine iteratively, but each
/// operand of a branching node is lowered by a *recursive* call back into
/// `lower_expression`.  A pathological input — thousands of nested
/// parenthesised operators — could otherwise drive that recursion deep
/// enough to overflow the thread stack (an uncatchable abort, i.e. a DoS
/// for any host compiling untrusted source).  We cap the depth and turn
/// an over-deep tree into an ordinary positioned error.  The limit is
/// generous: real JavaScript almost never nests operators past a handful
/// of levels, and the CST's ~20 fixed precedence-wrapper layers per
/// "real" level are peeled iteratively (they do **not** count against
/// this budget — only genuine operand recursion does).
const MAX_EXPR_DEPTH: usize = 256;

/// Hard ceiling on *statement-block* nesting depth (M3).
///
/// Each control-flow body (`if`/`while`/`for` body, or a bare `{ … }`
/// block) is lowered by a recursive call that descends with `depth + 1`.
/// Deeply nested control flow — thousands of `if (c) { if (c) { … } }` —
/// could otherwise drive that recursion deep enough to overflow the thread
/// stack (an uncatchable abort, i.e. a DoS for any host compiling untrusted
/// source).  We cap the nesting and turn an over-deep tree into an ordinary
/// positioned error.  The limit is generous: real JavaScript almost never
/// nests blocks past a handful of levels.  This is the statement-side twin
/// of [`MAX_EXPR_DEPTH`].
const MAX_STMT_DEPTH: usize = 256;

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
/// M2 admits literal expression statements, `let`/`const`/`var`
/// bindings, re-assignments, variable references, and unary/binary
/// operators.  M3 adds control flow: `if`/`else`, `while`, the canonical
/// counting C-style `for`, `for … of`, and bare `{ … }` blocks.  Any
/// other statement or expression shape produces a [`JsLowerError`] (see
/// module docs).
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
        declared_locals: HashSet::new(),
    };

    let block = lw.lower_program(program)?;

    // Every JS source becomes a synthetic `main` whose body is the
    // top-level statement sequence — matching SIR17 (Python) and the
    // Ruby frontend.  `main` has no params, so it never triggers the
    // validator's `DynamicTyping` observation; we only declare features
    // the body actually uses.
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
// Lowerer — the small amount of mutable state M2 needs
// ---------------------------------------------------------------------------

struct Lowerer {
    /// Logical filename stamped into every [`Span`].  We use the module
    /// name because the parser CST doesn't carry the original path.
    file_name: String,
    /// Features accumulated as we lower.  `FeatureManifest::add` is
    /// idempotent, so repeated `StrLit`s add `Strings` exactly once.
    features_used: FeatureManifest,
    /// Names already bound in `main`'s top-level scope.  Drives the
    /// binding-vs-assignment choice (first sighting binds, later
    /// sightings re-assign) and lets a bare identifier resolve to a
    /// `Scope::Local` `VarRef`.  M2 has a single flat scope (no nested
    /// functions yet), so one set suffices.
    declared_locals: HashSet<String>,
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
    /// the program's result.  Following SIR17/Ruby, binding and
    /// assignment statements accumulate in `stmts`, and the **final**
    /// top-level *expression* statement becomes the tail `value`.
    /// Earlier bare expression statements are pure (M2 has no calls yet),
    /// hence unobservable, so we drop them.  An empty program yields a
    /// `NilLit` value.
    fn lower_program(&mut self, program: &GrammarASTNode) -> Result<Block, JsLowerError> {
        // The top-level statement sequence is `program`'s children, each a
        // `source_element` wrapping one `statement`.  Lowering it is exactly
        // lowering a statement list — the same routine used for `{ … }`
        // block bodies — at depth 0.
        self.lower_stmt_seq(&program.children, self.span_of(program), 0)
    }

    /// Lower a slice of CST children that are statement-bearing nodes into a
    /// single [`Block`] (statements then a tail value).
    ///
    /// This is the shared workhorse for the top-level program body and every
    /// `{ … }` block / control-flow body.  Each child is lowered to a
    /// [`Lowered`]:
    ///
    /// - a [`Lowered::Stmt`] is pushed onto `stmts` (flushing any pending
    ///   bare-expression value as an `ExprStmt` first, so evaluation order
    ///   and side effects are preserved);
    /// - a [`Lowered::Expr`] becomes the *candidate* tail value, superseding
    ///   any earlier pure bare-expression value.
    ///
    /// The final candidate tail value becomes `Block.value`; an empty
    /// sequence yields a `NilLit` tail (matching SIR's "every block produces
    /// a value" rule).  `block_span` stamps the resulting `Block`.
    fn lower_stmt_seq(
        &mut self,
        children: &[ASTNodeOrToken],
        block_span: Span,
        depth: usize,
    ) -> Result<Block, JsLowerError> {
        let mut stmts: Vec<Stmt> = Vec::new();
        // The most recent bare-expression value seen.  Whatever it holds
        // at the end becomes the block's tail value.
        let mut tail: Option<Expr> = None;

        for child in children {
            match child {
                ASTNodeOrToken::Node(n) => {
                    match self.lower_source_element(n, depth)? {
                        Lowered::Stmt(s) => {
                            // A statement makes any pending tail
                            // expression unobservable as a *value*, but it
                            // may have a side effect — keep it as an
                            // `ExprStmt` so evaluation order is preserved.
                            if let Some(prev) = tail.take() {
                                let span = prev.span().clone();
                                stmts.push(Stmt::ExprStmt { expr: prev, span });
                            }
                            stmts.push(*s);
                        }
                        Lowered::Expr(e) => {
                            // A new bare expression supersedes the prior
                            // one as the candidate tail value; the prior
                            // one, being pure, is dropped.
                            tail = Some(e);
                        }
                    }
                }
                // Stray tokens (the `{`/`}` of a block, the `source_element`
                // separators, etc.) carry no statement; skip them.
                ASTNodeOrToken::Token(_) => {}
            }
        }

        let value = tail.unwrap_or(Expr::NilLit {
            span: block_span.clone(),
        });

        Ok(Block {
            stmts,
            value,
            span: block_span,
        })
    }

    /// Lower one statement-bearing item (a `source_element`, a `statement`
    /// wrapper, or a concrete statement node) to a [`Lowered`].
    ///
    /// `depth` bounds control-flow body nesting (see [`MAX_STMT_DEPTH`]); a
    /// nested body recurses with `depth + 1`.
    fn lower_source_element(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Lowered, JsLowerError> {
        // `source_element` → `statement` → `<concrete statement>`.
        // Descend through the single-child wrappers until we reach a
        // statement we recognise.
        let inner = single_child_node(node).unwrap_or(node);
        match inner.rule_name.as_str() {
            // `source_element` and `statement` are both single-child
            // wrappers; recurse through them to the concrete statement.
            "statement" => self.lower_source_element(inner, depth),
            other => self.lower_statement_inner(inner, other, depth),
        }
    }

    /// Lower a concrete statement node, dispatching on its `rule_name`.
    fn lower_statement_inner(
        &mut self,
        node: &GrammarASTNode,
        rule_name: &str,
        depth: usize,
    ) -> Result<Lowered, JsLowerError> {
        match rule_name {
            "expression_statement" => self.lower_expression_statement(node),
            "lexical_declaration" => self
                .lower_lexical_declaration(node)
                .map(|s| Lowered::Stmt(Box::new(s))),
            "variable_statement" => self
                .lower_variable_statement(node)
                .map(|s| Lowered::Stmt(Box::new(s))),
            // ── M3: control flow ────────────────────────────────────
            "if_statement" => self.lower_if(node, depth).map(Lowered::Expr),
            "while_statement" => self
                .lower_while(node, depth)
                .map(|s| Lowered::Stmt(Box::new(s))),
            "for_statement" => self
                .lower_for(node, depth)
                .map(|s| Lowered::Stmt(Box::new(s))),
            "for_of_statement" => self
                .lower_for_of(node, depth)
                .map(|s| Lowered::Stmt(Box::new(s))),
            "block" => self.lower_block(node, depth).map(|b| {
                // A bare `{ … }` block is a value-producing expression in
                // SIR (`Expr::Block`); at statement position its tail value
                // is unobservable but its statements run for effect.
                Lowered::Expr(Expr::Block(Box::new(b)))
            }),
            // deferred to a later milestone: function_declaration,
            // return_statement, switch, try, do-while, labeled, …
            other => Err(self.unsupported(node, other)),
        }
    }

    // -----------------------------------------------------------------------
    // M3: control-flow lowering
    // -----------------------------------------------------------------------

    /// Lower an `if_statement` to an [`Expr::If`].
    ///
    /// CST (probed): `[Kw("if"), (, expression, ), statement]` with no else,
    /// or `[…, statement, Kw("else"), statement]` with one.  The `then`/
    /// `else` `statement`s are each a block body (a `{ … }` block or a
    /// single statement).  A missing `else` becomes a synthetic empty
    /// nil-valued [`Block`].  An `else if` chain is the grammar nesting
    /// another `if_statement` inside the else `statement`, so it recurses
    /// into a nested `Expr::If` automatically.
    fn lower_if(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, JsLowerError> {
        self.check_stmt_depth(node, depth)?;
        let span = self.span_of(node);

        // Collect the direct child *nodes* in order: [cond_expr, then_stmt]
        // or [cond_expr, then_stmt, else_stmt].  The `if`/`else` keywords,
        // parens, etc. are tokens we skip.
        let nodes = child_nodes(node);
        let cond_node = nodes.first().ok_or_else(|| self.unsupported(node, "if (no condition)"))?;
        let then_node = nodes.get(1).ok_or_else(|| self.unsupported(node, "if (no then branch)"))?;

        let cond = self.lower_expression(cond_node, 0)?;
        let then_branch = self.lower_body(then_node, depth)?;
        let else_branch = match nodes.get(2) {
            Some(else_node) => self.lower_body(else_node, depth)?,
            // No `else`: an empty, nil-valued block.
            None => Block {
                stmts: Vec::new(),
                value: Expr::NilLit { span: span.clone() },
                span: span.clone(),
            },
        };

        Ok(Expr::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
            span,
        })
    }

    /// Lower a `while_statement` to a [`Stmt::While`].
    ///
    /// CST: `[Kw("while"), (, expression, ), statement]`.
    fn lower_while(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Stmt, JsLowerError> {
        self.check_stmt_depth(node, depth)?;
        let span = self.span_of(node);
        let nodes = child_nodes(node);
        let cond_node = nodes.first().ok_or_else(|| self.unsupported(node, "while (no condition)"))?;
        let body_node = nodes.get(1).ok_or_else(|| self.unsupported(node, "while (no body)"))?;

        let cond = self.lower_expression(cond_node, 0)?;
        let body = self.lower_body(body_node, depth)?;
        // The validator observes `Feature::Loops` for every loop statement;
        // declare it so the manifest matches the body exactly.
        self.features_used.add(Feature::Loops);
        Ok(Stmt::While { cond, body, span })
    }

    /// Lower a `for_of_statement` to a [`Stmt::ForEach`].
    ///
    /// CST: `[Kw("for"), (, Kw("let|const|var"), binding_element,
    /// Name("of"), assignment_expression(iter), ), statement]`.  Only the
    /// single-identifier binding (`for (const x of xs)`) is supported;
    /// destructuring (`for (const [a, b] of …)`) is deferred.  The loop
    /// variable `x` is bound into the body scope only.
    fn lower_for_of(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Stmt, JsLowerError> {
        self.check_stmt_depth(node, depth)?;
        let span = self.span_of(node);

        // The binding name lives in the `binding_element`'s single Name
        // token.  Anything else (a destructuring pattern) is deferred.
        let binding = child_node_named(node, "binding_element")
            .ok_or_else(|| self.unsupported(node, "for-of (no binding_element)"))?;
        let var_tok = single_leaf_token(binding)
            .filter(|t| matches!(t.type_, TokenType::Name))
            .ok_or_else(|| JsLowerError {
                message: "for-of destructuring binding is deferred (only `for (const x of …)`)"
                    .to_string(),
                line: span.start_line,
                column: span.start_col,
            })?;
        let var = var_tok.value.clone();

        // The iterable is the `assignment_expression` child — the only
        // expression-shaped node (the `binding_element` is the binding).
        let iter_node = child_node_named(node, "assignment_expression")
            .ok_or_else(|| self.unsupported(node, "for-of (no iterable)"))?;
        let iter = self.lower_expression(iter_node, 0)?;

        let body = self.lower_loop_body_scoped(&var, node, depth)?;
        self.features_used.add(Feature::Loops);
        Ok(Stmt::ForEach { var, iter, body, span })
    }

    /// Lower a canonical C-style `for_statement` to a [`Stmt::ForRange`].
    ///
    /// CST: `[Kw("for"), (, Kw("let"), binding_list, ;, expr(cond), ;,
    /// expr(update), ), statement]`.  We accept **only** the canonical
    /// counting shape (see module docs):
    ///
    ///   * init `let i = <start>` (single binding of `i`),
    ///   * cond `i < <stop>` or `i <= <stop>` on the same `i`,
    ///   * update `i = i + <step>`, `i += <step>`, or `i++` (step 1).
    ///
    /// `<=` is rewritten half-open by bumping `stop` to `stop + 1`.  Any
    /// non-canonical shape is a positioned [`JsLowerError`] (deferred).
    fn lower_for(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Stmt, JsLowerError> {
        self.check_stmt_depth(node, depth)?;
        let span = self.span_of(node);

        // The non-canonical bail-out, factored so every rejection carries
        // the loop's position and a uniform "deferred" message.
        let reject = |why: &str| JsLowerError {
            message: format!(
                "non-canonical C-style `for` ({why}) is deferred; only the counting form \
                 `for (let i = <start>; i < <stop>; i++/i += <step>)` is supported"
            ),
            line: span.start_line,
            column: span.start_col,
        };

        // ── init: `let i = <start>` ─────────────────────────────────────
        // The init clause is a `binding_list` (for `let`/`const`) sitting
        // directly under the `for_statement` (the probe shows it is *not*
        // wrapped in a `lexical_declaration` here — the `let` keyword and
        // `binding_list` are direct children).  `var` would surface a
        // `variable_declaration_list` instead; we accept either.
        let (loop_var, start) = self.extract_for_init(node).ok_or_else(|| {
            reject("init is not a single `let i = <start>` binding")
        })?;

        // ── cond: `i < <stop>` / `i <= <stop>` ──────────────────────────
        // The condition is the first `expression` child after the init's
        // terminating `;`.  We need the *branching* relational node.
        let cond_expr = self
            .for_clause_expr(node, 0)
            .ok_or_else(|| reject("missing condition clause"))?;
        let stop = self.extract_for_cond(cond_expr, &loop_var).ok_or_else(|| {
            reject("condition is not `i < <stop>` or `i <= <stop>` on the loop variable")
        })?;

        // ── update: `i = i + <step>` / `i += <step>` / `i++` ────────────
        let update_expr = self
            .for_clause_expr(node, 1)
            .ok_or_else(|| reject("missing update clause"))?;
        let step = self.extract_for_step(update_expr, &loop_var).ok_or_else(|| {
            reject("update is not an increment of the loop variable by a constant step")
        })?;

        // ── body (loop variable scoped into it) ─────────────────────────
        let body = self.lower_loop_body_scoped(&loop_var, node, depth)?;

        self.features_used.add(Feature::Loops);
        Ok(Stmt::ForRange {
            var: loop_var,
            start,
            stop,
            step,
            body,
            span,
        })
    }

    /// Extract `(var, start_expr)` from a C-`for` init clause, or `None` if
    /// it is not a single `let|const|var i = <start>` binding.
    fn extract_for_init(&mut self, for_node: &GrammarASTNode) -> Option<(String, Expr)> {
        // `let`/`const` → `binding_list[ lexical_binding[ Name, =, init ] ]`.
        // `var`         → `variable_declaration_list[ variable_declaration ]`.
        let (list_name, binding_name) =
            if child_node_named(for_node, "binding_list").is_some() {
                ("binding_list", "lexical_binding")
            } else {
                ("variable_declaration_list", "variable_declaration")
            };
        let list = child_node_named(for_node, list_name)?;
        let bindings = children_nodes_named(list, binding_name);
        if bindings.len() != 1 {
            return None; // multi-variable init is non-canonical.
        }
        let binding = bindings[0];
        let name_tok = binding.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => Some(t),
            _ => None,
        })?;
        let init_node = binding.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })?;
        let start = self.lower_expression(init_node, 0).ok()?;
        Some((name_tok.value.clone(), start))
    }

    /// Return the `n`-th `expression` clause node under a `for_statement`
    /// (cond = 0, update = 1).  These are the `expression` rule nodes that
    /// sit between the clause-separating `;`/`)` tokens.
    fn for_clause_expr<'a>(
        &self,
        for_node: &'a GrammarASTNode,
        n: usize,
    ) -> Option<&'a GrammarASTNode> {
        for_node
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(node) if node.rule_name == "expression" => Some(node),
                _ => None,
            })
            .nth(n)
    }

    /// Extract the `stop` expression from a canonical loop condition.
    ///
    /// Accepts `i < S` (→ `S`) and `i <= S` (→ half-open `S + 1`), where the
    /// left operand is exactly the loop variable `var`.  Returns `None` for
    /// any other comparison (wrong variable, `>`/`>=`, RHS-anchored, …).
    fn extract_for_cond(
        &mut self,
        cond_node: &GrammarASTNode,
        var: &str,
    ) -> Option<Expr> {
        // Peel to the branching `relational_expression`: `[lhs, op, rhs]`.
        let branch = peel_to_branch(cond_node);
        if branch.rule_name != "relational_expression" || branch.children.len() != 3 {
            return None;
        }
        // children = [lhs_node, op_token, rhs_node].
        let lhs = match &branch.children[0] {
            ASTNodeOrToken::Node(n) => n,
            _ => return None,
        };
        let op = match &branch.children[1] {
            ASTNodeOrToken::Token(t) => t.value.as_str(),
            _ => return None,
        };
        let rhs = match &branch.children[2] {
            ASTNodeOrToken::Node(n) => n,
            _ => return None,
        };
        // LHS must be exactly the loop variable.
        let lhs_tok = single_leaf_token(lhs)?;
        if !matches!(lhs_tok.type_, TokenType::Name) || lhs_tok.value != var {
            return None;
        }
        let stop = self.lower_expression(rhs, 0).ok()?;
        match op {
            "<" => Some(stop),
            "<=" => {
                // Half-open rewrite: `i <= S` ⇔ `i < S + 1`.
                let span = stop.span().clone();
                Some(Expr::BuiltinCall {
                    name: "+".to_string(),
                    args: vec![stop, Expr::IntLit { value: 1, span: span.clone() }],
                    effects: EffectSet::PURE,
                    span,
                })
            }
            _ => None,
        }
    }

    /// Extract the `step` expression from a canonical loop update clause.
    ///
    /// Accepts (on the loop variable `var`):
    ///
    ///   * `i++`           → `IntLit(1)` (postfix increment),
    ///   * `i += <step>`   → `<step>`,
    ///   * `i = i + <step>`→ `<step>`.
    ///
    /// Returns `None` for decrements, `*=`, a different variable, etc.
    fn extract_for_step(
        &mut self,
        update_node: &GrammarASTNode,
        var: &str,
    ) -> Option<Expr> {
        let branch = peel_to_branch(update_node);
        match branch.rule_name.as_str() {
            // ── `i++` : postfix_expression[ lhs, Name("++") ] ───────────
            "postfix_expression" => {
                let nodes = child_nodes(branch);
                let target = nodes.first()?;
                let t = single_leaf_token(target)?;
                if !matches!(t.type_, TokenType::Name) || t.value != var {
                    return None;
                }
                // The operator token must be `++` (reject `i--`).
                let op_ok = branch.children.iter().any(|c| {
                    matches!(c, ASTNodeOrToken::Token(tok) if tok.value == "++")
                });
                if !op_ok {
                    return None;
                }
                Some(Expr::IntLit { value: 1, span: self.span_of(branch) })
            }
            // ── `i += s` or `i = i + s` :
            //    assignment_expression[ lhs, op, rhs ] ─────────────────
            "assignment_expression" if branch.children.len() == 3 => {
                let lhs = match &branch.children[0] {
                    ASTNodeOrToken::Node(n) => n,
                    _ => return None,
                };
                let lhs_tok = single_leaf_token(lhs)?;
                if !matches!(lhs_tok.type_, TokenType::Name) || lhs_tok.value != var {
                    return None;
                }
                // The assignment operator value (`=`, `+=`, …).
                let op = match &branch.children[1] {
                    ASTNodeOrToken::Node(n) => single_leaf_token(n)?.value.clone(),
                    ASTNodeOrToken::Token(t) => t.value.clone(),
                };
                let rhs = match &branch.children[2] {
                    ASTNodeOrToken::Node(n) => n,
                    _ => return None,
                };
                match op.as_str() {
                    // `i += s` → step is `s`.
                    "+=" => self.lower_expression(rhs, 0).ok(),
                    // `i = i + s` → the RHS must be `i + s`; step is `s`.
                    "=" => self.extract_plus_step(rhs, var),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// From an RHS shaped `i + <step>` (an `additive_expression` whose left
    /// operand is the loop variable and whose single operator is `+`),
    /// extract `<step>`.  Returns `None` for anything else (e.g. `i - 1`,
    /// `i * 2`, `s + i`).
    fn extract_plus_step(&mut self, rhs: &GrammarASTNode, var: &str) -> Option<Expr> {
        let branch = peel_to_branch(rhs);
        // `i + s` is one `additive_expression` with children
        // `[i_node, Plus, s_node]`.
        if branch.rule_name != "additive_expression" || branch.children.len() != 3 {
            return None;
        }
        let lhs = match &branch.children[0] {
            ASTNodeOrToken::Node(n) => n,
            _ => return None,
        };
        let op = match &branch.children[1] {
            ASTNodeOrToken::Token(t) => t.value.as_str(),
            _ => return None,
        };
        if op != "+" {
            return None;
        }
        let lhs_tok = single_leaf_token(lhs)?;
        if !matches!(lhs_tok.type_, TokenType::Name) || lhs_tok.value != var {
            return None;
        }
        let step_node = match &branch.children[2] {
            ASTNodeOrToken::Node(n) => n,
            _ => return None,
        };
        self.lower_expression(step_node, 0).ok()
    }

    // -----------------------------------------------------------------------
    // M3: shared body / block helpers
    // -----------------------------------------------------------------------

    /// Lower a control-flow *body* `statement` (an `if`/`while` branch or a
    /// `for` body) into a [`Block`].
    ///
    /// The body is either a `{ … }` block (→ its statement sequence) or a
    /// single statement (→ a one-item block).  Either way names bound inside
    /// it are block-scoped: we snapshot `declared_locals` before lowering
    /// and restore it after, so an inner `let` does not leak outward.
    fn lower_body(
        &mut self,
        body_stmt: &GrammarASTNode,
        depth: usize,
    ) -> Result<Block, JsLowerError> {
        let saved = self.declared_locals.clone();
        let result = self.lower_body_inner(body_stmt, depth);
        // Restore the outer scope regardless of success so a partially
        // mutated set never leaks (defensive; on `Err` we abort anyway).
        self.declared_locals = saved;
        result
    }

    /// Lower a loop body with `loop_var` bound into the body scope only.
    ///
    /// Mirrors the validator, which adds the loop variable to its `LocalEnv`
    /// for the body and rewinds afterwards.  The variable must resolve to a
    /// `Scope::Local` `VarRef` inside the body but be invisible after the
    /// loop.  We add it to `declared_locals` over the body and then restore
    /// the snapshot (which excludes it).
    fn lower_loop_body_scoped(
        &mut self,
        loop_var: &str,
        for_node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Block, JsLowerError> {
        let body_stmt = self
            .loop_body_node(for_node)
            .ok_or_else(|| self.unsupported(for_node, "loop (no body)"))?;
        let saved = self.declared_locals.clone();
        self.declared_locals.insert(loop_var.to_string());
        let result = self.lower_body_inner(body_stmt, depth);
        self.declared_locals = saved;
        result
    }

    /// The body `statement` of a loop is its **last** direct child node
    /// (after the header tokens / clause expressions).
    fn loop_body_node<'a>(&self, for_node: &'a GrammarASTNode) -> Option<&'a GrammarASTNode> {
        child_nodes(for_node).into_iter().next_back()
    }

    /// Inner body-lowering shared by [`lower_body`] and
    /// [`lower_loop_body_scoped`] (which own the scope save/restore).
    fn lower_body_inner(
        &mut self,
        body_stmt: &GrammarASTNode,
        depth: usize,
    ) -> Result<Block, JsLowerError> {
        // Descend the `statement` wrapper to the concrete body node.
        let inner = single_child_node(body_stmt).unwrap_or(body_stmt);
        if inner.rule_name == "block" {
            return self.lower_block(inner, depth);
        }
        // A single (unbraced) statement body, e.g. `if (c) x = 1;`.  Lower
        // the one statement and fold it into a one-element `Block`, reusing
        // the same Stmt/Expr → (stmts, tail) routing as a block.
        let span = self.span_of(body_stmt);
        let mut stmts: Vec<Stmt> = Vec::new();
        let value = match self.lower_source_element(body_stmt, depth + 1)? {
            Lowered::Stmt(s) => {
                stmts.push(*s);
                Expr::NilLit { span: span.clone() }
            }
            Lowered::Expr(e) => e,
        };
        Ok(Block { stmts, value, span })
    }

    /// Lower a `block` (`{ stmt* }`) into a [`Block`].  The `{`/`}` tokens
    /// are skipped by [`lower_stmt_seq`].  Recurses with `depth + 1` so the
    /// nesting guard catches pathological depth.
    fn lower_block(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Block, JsLowerError> {
        self.check_stmt_depth(node, depth)?;
        let span = self.span_of(node);
        self.lower_stmt_seq(&node.children, span, depth + 1)
    }

    /// Enforce the [`MAX_STMT_DEPTH`] nesting bound; error if exceeded.
    fn check_stmt_depth(&self, node: &GrammarASTNode, depth: usize) -> Result<(), JsLowerError> {
        if depth > MAX_STMT_DEPTH {
            return Err(JsLowerError {
                message: format!(
                    "control-flow nests deeper than the supported limit ({MAX_STMT_DEPTH})"
                ),
                line: node.start_line.unwrap_or(0),
                column: node.start_column.unwrap_or(0),
            });
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Declarations: let / const / var  →  binding or assignment statement
    // -----------------------------------------------------------------------

    /// Lower a `lexical_declaration` (`let`/`const` form).
    ///
    /// Shape (from the probe):
    /// `lexical_declaration[ Keyword(let|const), binding_list, Semicolon ]`
    /// where `binding_list` holds one or more `lexical_binding`s, each
    /// `lexical_binding[ Name, Equals, assignment_expression ]`.
    ///
    /// M2 supports the common single-binding case; a comma-separated
    /// multi-binding list (`let a = 1, b = 2;`) is rejected (deferred) so
    /// the lossy behaviour is explicit rather than silent.
    fn lower_lexical_declaration(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Stmt, JsLowerError> {
        let list = child_node_named(node, "binding_list")
            .ok_or_else(|| self.unsupported(node, "lexical_declaration (no binding_list)"))?;
        let bindings = children_nodes_named(list, "lexical_binding");
        self.lower_single_binding(node, &bindings, "lexical_binding")
    }

    /// Lower a `variable_statement` (`var` form).
    ///
    /// Shape: `variable_statement[ Keyword(var), variable_declaration_list,
    /// Semicolon ]`, the list holding `variable_declaration`s each shaped
    /// `[ Name, Equals, assignment_expression ]`.  `var` hoisting is NOT
    /// modelled (SIR19 spec "`var` hoisting"): we emit the binding at its
    /// source position, exactly like `let`.
    fn lower_variable_statement(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Stmt, JsLowerError> {
        let list = child_node_named(node, "variable_declaration_list")
            .ok_or_else(|| self.unsupported(node, "variable_statement (no declaration list)"))?;
        let decls = children_nodes_named(list, "variable_declaration");
        self.lower_single_binding(node, &decls, "variable_declaration")
    }

    /// Shared core for `let`/`const`/`var`: lower exactly one binding of
    /// the form `[ Name, Equals, <init expr> ]`.
    fn lower_single_binding(
        &mut self,
        decl_node: &GrammarASTNode,
        bindings: &[&GrammarASTNode],
        what: &str,
    ) -> Result<Stmt, JsLowerError> {
        if bindings.len() != 1 {
            return Err(JsLowerError {
                message: format!(
                    "multi-binding `{what}` (`let a = 1, b = 2;`) is deferred past M2"
                ),
                line: decl_node.start_line.unwrap_or(0),
                column: decl_node.start_column.unwrap_or(0),
            });
        }
        let binding = bindings[0];

        // The binding name is the first `Name` token child.
        let name_tok = binding
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) if matches!(t.type_, TokenType::Name) => Some(t),
                _ => None,
            })
            .ok_or_else(|| {
                self.unsupported(binding, &format!("{what} (destructuring/no name)"))
            })?;
        let name = name_tok.value.clone();
        let span = self.span_of(decl_node);

        // The initialiser is the single expression-shaped child node.
        // A declaration with no initialiser (`let x;`) is deferred — the
        // IR has no "uninitialised binding" and inventing a `NilLit`
        // would mask the source's intent.
        let init_node = binding
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                ASTNodeOrToken::Token(_) => None,
            })
            .ok_or_else(|| JsLowerError {
                message: format!(
                    "uninitialised binding `{name}` (`{what}` with no `= …`) is deferred past M2"
                ),
                line: span.start_line,
                column: span.start_col,
            })?;
        let value = self.lower_expression(init_node, 0)?;

        // First sighting → `let*` binding; a re-declaration of an
        // already-declared name (legal for `var`, a redeclare error for
        // `let`/`const` in real JS but we don't enforce that) becomes an
        // `Assign` to keep validation honest.
        if self.declared_locals.contains(&name) {
            self.features_used.add(Feature::MutableBindings);
            Ok(Stmt::Assign {
                name,
                scope: Scope::Local,
                value,
                span,
            })
        } else {
            self.declared_locals.insert(name.clone());
            // Sequential `let*` (not parallel `let`): see module docs.
            Ok(Stmt::LetStarBinding {
                name,
                sir_type: None,
                value,
                span,
            })
        }
    }

    // -----------------------------------------------------------------------
    // expression_statement  →  bare expression OR re-assignment statement
    // -----------------------------------------------------------------------

    /// Lower an `expression_statement` (`<expression> ;`).
    ///
    /// Two cases distinguished by the inner expression's shape:
    ///   * a top-level `assignment_expression` with an `=` operator
    ///     (`x = …`) becomes a binding/assignment **statement**;
    ///   * anything else is a value-producing expression returned as
    ///     [`Lowered::Expr`].
    fn lower_expression_statement(
        &mut self,
        node: &GrammarASTNode,
    ) -> Result<Lowered, JsLowerError> {
        // Children are the `expression` node followed by a `Semicolon`.
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

        // Peek for a top-level assignment: descend the single-child
        // spine until something branches; if that something is an
        // `assignment_expression` with three children (`lhs op rhs`),
        // it's a statement-level assignment.
        let branch = peel_to_branch(expr_node);
        if branch.rule_name == "assignment_expression" && branch.children.len() == 3 {
            return self.lower_assignment(branch).map(|s| Lowered::Stmt(Box::new(s)));
        }
        self.lower_expression(expr_node, 0).map(Lowered::Expr)
    }

    /// Lower a statement-level `assignment_expression` (`x = expr`).
    ///
    /// Shape: `assignment_expression[ left_hand_side_expression,
    /// assignment_operator, assignment_expression ]`.  M2 supports only
    /// the plain `=` operator on a bare identifier target; compound
    /// assignment (`+=`, …) and assignment to a member/index
    /// (`obj.x = …`, `xs[i] = …`) are deferred.
    fn lower_assignment(&mut self, node: &GrammarASTNode) -> Result<Stmt, JsLowerError> {
        let span = self.span_of(node);

        // children[0] = LHS target, children[1] = assignment_operator,
        // children[2] = RHS value.
        let lhs = match &node.children[0] {
            ASTNodeOrToken::Node(n) => n,
            ASTNodeOrToken::Token(_) => {
                return Err(self.unsupported(node, "assignment (token LHS)"))
            }
        };
        let op = &node.children[1];
        // Only the plain `=` operator is supported.
        let op_is_plain_eq = match op {
            ASTNodeOrToken::Node(n) => single_leaf_token(n)
                .map(|t| t.value == "=")
                .unwrap_or(false),
            ASTNodeOrToken::Token(t) => t.value == "=",
        };
        if !op_is_plain_eq {
            return Err(JsLowerError {
                message: "compound assignment (`+=`, `-=`, …) is deferred past M2".to_string(),
                line: span.start_line,
                column: span.start_col,
            });
        }

        // The target must be a bare identifier.  Peel the LHS spine to
        // its leaf token; anything that branches (member access, index)
        // is deferred.
        let target_tok = single_leaf_token(peel_to_branch(lhs)).ok_or_else(|| JsLowerError {
            message: "assignment to a non-identifier target (member/index) is deferred past M2"
                .to_string(),
            line: span.start_line,
            column: span.start_col,
        })?;
        if !matches!(target_tok.type_, TokenType::Name) {
            return Err(self.unsupported(lhs, "assignment target (not a name)"));
        }
        let name = target_tok.value.clone();

        let rhs = match &node.children[2] {
            ASTNodeOrToken::Node(n) => n,
            ASTNodeOrToken::Token(_) => {
                return Err(self.unsupported(node, "assignment (token RHS)"))
            }
        };
        let value = self.lower_expression(rhs, 0)?;

        // First sighting of a never-declared name via bare `x = …`
        // creates a top-level binding (JS implicitly creates a global on
        // assignment without a declarator).  A subsequent `x = …`
        // re-assigns.
        if self.declared_locals.contains(&name) {
            self.features_used.add(Feature::MutableBindings);
            Ok(Stmt::Assign {
                name,
                scope: Scope::Local,
                value,
                span,
            })
        } else {
            self.declared_locals.insert(name.clone());
            Ok(Stmt::LetStarBinding {
                name,
                sir_type: None,
                value,
                span,
            })
        }
    }

    // -----------------------------------------------------------------------
    // expression → Expr  (M2: literals, var refs, unary/binary operators)
    // -----------------------------------------------------------------------

    /// Lower a JS `expression` to a SIR [`Expr`].
    ///
    /// The CST spine from `expression` down to the leaf is a chain of
    /// single-child precedence wrappers (see module docs).  We walk that
    /// spine to its bottom iteratively.  Two outcomes:
    ///   * the bottom is a single leaf token → a literal or variable
    ///     reference;
    ///   * we hit a node that *branches* (an operator with operands) →
    ///     dispatch on the rule name to build the matching SIR node.
    ///
    /// `depth` bounds the *operand* recursion (each branch lowers its
    /// children with `depth + 1`); the iterative spine-peel itself does
    /// not consume the budget.
    fn lower_expression(
        &mut self,
        expr: &GrammarASTNode,
        depth: usize,
    ) -> Result<Expr, JsLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(JsLowerError {
                message: format!(
                    "expression nests deeper than the supported limit ({MAX_EXPR_DEPTH})"
                ),
                line: expr.start_line.unwrap_or(0),
                column: expr.start_column.unwrap_or(0),
            });
        }

        // Descend through single-child wrapper nodes.
        let mut cur = expr;
        loop {
            // A leaf node (exactly one child, a token) is a literal or a
            // variable reference — classify and emit.
            if let Some(tok) = cur.token() {
                return self.lower_leaf_token(tok);
            }
            match single_child_node(cur) {
                Some(next) => cur = next,
                None => {
                    // A branching node — an operator (or, in M2's
                    // still-unsupported set, a call/member access).
                    return self.lower_branch(cur, depth);
                }
            }
        }
    }

    /// Dispatch a *branching* precedence node to its operator handler.
    fn lower_branch(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Expr, JsLowerError> {
        match node.rule_name.as_str() {
            // ── flat left-associative binary chains ─────────────────
            // children = [lhs, op, rhs, op, rhs, …]
            "additive_expression"
            | "multiplicative_expression"
            | "relational_expression"
            | "equality_expression" => self.lower_binary_chain(node, depth),

            // ── short-circuit logical chains ────────────────────────
            "logical_and_expression" => self.lower_logical_chain(node, depth, true),
            "logical_or_expression" => self.lower_logical_chain(node, depth, false),

            // ── prefix unary ────────────────────────────────────────
            // children = [op_token, operand]
            "unary_expression" => self.lower_unary(node, depth),

            // ── still unsupported in M2 (calls, member access, …) ───
            other => Err(self.unsupported(node, other)),
        }
    }

    /// Lower a flat, left-associative binary chain to nested
    /// `BuiltinCall`s.  `a + b - c` (one `additive_expression` with
    /// children `[a, +, b, -, c]`) folds left into
    /// `BuiltinCall("-", [BuiltinCall("+", [a, b]), c])`.
    fn lower_binary_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Expr, JsLowerError> {
        // Split children into operand nodes and operator tokens.  The
        // grammar guarantees the alternating shape `[n, t, n, t, n, …]`.
        let mut acc: Option<Expr> = None;
        let mut pending_op: Option<&Token> = None;

        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(n) => {
                    let operand = self.lower_expression(n, depth + 1)?;
                    match (acc.take(), pending_op.take()) {
                        (None, _) => acc = Some(operand),
                        (Some(lhs), Some(op)) => {
                            acc = Some(self.build_binary_op(op, lhs, operand)?);
                        }
                        (Some(_), None) => {
                            // Two operands with no operator between —
                            // shouldn't happen for a well-formed CST.
                            return Err(self.unsupported(node, &node.rule_name));
                        }
                    }
                }
                ASTNodeOrToken::Token(t) => pending_op = Some(t),
            }
        }

        acc.ok_or_else(|| self.unsupported(node, &node.rule_name))
    }

    /// Build one binary `BuiltinCall` from an operator token and its two
    /// already-lowered operands, applying equality normalisation.
    fn build_binary_op(
        &mut self,
        op: &Token,
        lhs: Expr,
        rhs: Expr,
    ) -> Result<Expr, JsLowerError> {
        // Normalise the operator spelling to the IR builtin name.  Both
        // loose and strict equality collapse to the strict-shaped IR
        // comparison — a deliberate semantic change (see module docs).
        let builtin = match op.value.as_str() {
            "+" | "-" | "*" | "/" | "%" | "<" | ">" | "<=" | ">=" => op.value.as_str(),
            "==" | "===" => "=",
            "!=" | "!==" => "!=",
            other => {
                return Err(JsLowerError {
                    message: format!("unsupported binary operator `{other}`"),
                    line: op.line,
                    column: op.column,
                })
            }
        };
        let span = self.span_of_token(op);
        Ok(Expr::BuiltinCall {
            name: builtin.to_string(),
            args: vec![lhs, rhs],
            // Arithmetic/comparison builtins are pure.
            effects: EffectSet::PURE,
            span,
        })
    }

    /// Lower a logical chain (`&&` / `||`) to nested short-circuit nodes.
    /// `a && b && c` folds left into `And(And(a, b), c)`.  These are
    /// **not** builtins: `LogicalAnd`/`LogicalOr` carry short-circuit
    /// semantics the validator records as `Feature::ShortCircuit`.
    fn lower_logical_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        is_and: bool,
    ) -> Result<Expr, JsLowerError> {
        // Short-circuit nodes are observed by the validator as
        // `Feature::ShortCircuit`; declare it so the manifest matches.
        self.features_used.add(Feature::ShortCircuit);
        let mut acc: Option<Expr> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(n) => {
                    let operand = self.lower_expression(n, depth + 1)?;
                    acc = Some(match acc.take() {
                        None => operand,
                        Some(lhs) => {
                            let span = lhs.span().clone();
                            if is_and {
                                Expr::LogicalAnd {
                                    lhs: Box::new(lhs),
                                    rhs: Box::new(operand),
                                    span,
                                }
                            } else {
                                Expr::LogicalOr {
                                    lhs: Box::new(lhs),
                                    rhs: Box::new(operand),
                                    span,
                                }
                            }
                        }
                    });
                }
                // The `&&` / `||` operator token carries no operand.
                ASTNodeOrToken::Token(_) => {}
            }
        }
        acc.ok_or_else(|| self.unsupported(node, &node.rule_name))
    }

    /// Lower a prefix `unary_expression` (`!x`, `-x`).
    ///
    /// Children are `[op_token, operand]`.  `!` → `BuiltinCall("not")`,
    /// `-` → `BuiltinCall("neg")` — except `-<numeric literal>` is
    /// constant-folded into a negative literal (see module docs).  Other
    /// prefix operators (`+`, `~`, `typeof`, `void`, `delete`) are
    /// deferred.
    fn lower_unary(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Expr, JsLowerError> {
        // The operator is the leading token; the operand is the node.
        let op = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Token(t) => Some(t),
                ASTNodeOrToken::Node(_) => None,
            })
            .ok_or_else(|| self.unsupported(node, "unary_expression (no operator)"))?;
        let operand_node = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) => Some(n),
                ASTNodeOrToken::Token(_) => None,
            })
            .ok_or_else(|| self.unsupported(node, "unary_expression (no operand)"))?;

        match op.value.as_str() {
            "!" => {
                let operand = self.lower_expression(operand_node, depth + 1)?;
                Ok(Expr::BuiltinCall {
                    name: "not".to_string(),
                    args: vec![operand],
                    effects: EffectSet::PURE,
                    span: self.span_of_token(op),
                })
            }
            "-" => {
                // Constant-fold `-<numeric literal>`: peel the operand
                // spine; if it bottoms out at a single Number token, emit
                // a negative literal directly (keeps the spec's `-7 →
                // IntLit` row exact).
                if let Some(tok) = single_leaf_token(peel_to_branch(operand_node)) {
                    if matches!(tok.type_, TokenType::Number) {
                        return self.lower_number(&format!("-{}", tok.value), self.span_of_token(op));
                    }
                }
                let operand = self.lower_expression(operand_node, depth + 1)?;
                Ok(Expr::BuiltinCall {
                    name: "neg".to_string(),
                    args: vec![operand],
                    effects: EffectSet::PURE,
                    span: self.span_of_token(op),
                })
            }
            other => Err(JsLowerError {
                message: format!(
                    "unary operator `{other}` is deferred past M2 (only `!` and `-` supported)"
                ),
                line: op.line,
                column: op.column,
            }),
        }
    }

    /// Classify a leaf token and build the matching SIR atom.
    ///
    /// Covers M1 literals plus M2 variable references.  See the truth
    /// tables in the module docs.
    fn lower_leaf_token(&mut self, tok: &Token) -> Result<Expr, JsLowerError> {
        let span = self.span_of_token(tok);
        match tok.type_ {
            // ── number ──────────────────────────────────────────────
            TokenType::Number => self.lower_number(&tok.value, span),

            // ── keyword literals: true / false / null ───────────────
            TokenType::Keyword => match tok.value.as_str() {
                "true" => Ok(Expr::BoolLit { value: true, span }),
                "false" => Ok(Expr::BoolLit { value: false, span }),
                "null" => Ok(Expr::NilLit { span }),
                other => Err(JsLowerError {
                    message: format!(
                        "keyword `{other}` is not a value expression supported in M2"
                    ),
                    line: tok.line,
                    column: tok.column,
                }),
            },

            // ── string ──────────────────────────────────────────────
            TokenType::String => {
                self.features_used.add(Feature::Strings);
                Ok(Expr::StrLit {
                    value: tok.value.clone(),
                    span,
                })
            }

            // ── identifier: undefined / variable reference ──────────
            // `undefined` is a global identifier, not a keyword, so it
            // arrives as a `Name` token; collapse it to `NilLit` (the
            // JS null/undefined distinction is intentionally lost in v0).
            TokenType::Name if tok.value == "undefined" => Ok(Expr::NilLit { span }),
            // Any other identifier is a variable reference.  Resolve it
            // against the declared-locals set; an undeclared name is a
            // positioned "unresolved name" error (SIR19 "Error model").
            TokenType::Name => {
                if self.declared_locals.contains(&tok.value) {
                    Ok(Expr::VarRef {
                        name: tok.value.clone(),
                        scope: Scope::Local,
                        span,
                    })
                } else {
                    Err(JsLowerError {
                        message: format!("unresolved name reference `{}`", tok.value),
                        line: tok.line,
                        column: tok.column,
                    })
                }
            }

            other => Err(JsLowerError {
                message: format!("unsupported token {other:?} in expression position"),
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
    /// forms (`0x…`, `0o…`, `0b…`) and `BigInt` (`10n`) are deferred.
    ///
    /// A leading `-` (from a constant-folded unary minus) is permitted
    /// and parsed as part of the literal.
    fn lower_number(&mut self, text: &str, span: Span) -> Result<Expr, JsLowerError> {
        let looks_float = text.contains('.') || text.contains('e') || text.contains('E');
        // Detect the non-decimal integer forms after any leading sign.
        let digits = text.strip_prefix('-').unwrap_or(text);
        let non_decimal = digits.len() > 1
            && digits.starts_with('0')
            && matches!(digits.as_bytes()[1], b'x' | b'X' | b'o' | b'O' | b'b' | b'B');
        if non_decimal || text.ends_with('n') {
            return Err(JsLowerError {
                message: format!(
                    "numeric literal `{text}` form (hex/octal/binary/BigInt) is deferred past M2"
                ),
                line: span.start_line,
                column: span.start_col,
            });
        }

        if !looks_float {
            if let Ok(value) = text.parse::<i64>() {
                return Ok(Expr::IntLit { value, span });
            }
            // Integer-shaped but doesn't fit i64.  Fall through to float
            // so we don't lose the program; JS holds it as a double.
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

    /// Build the standard "out of scope" error for a node.
    fn unsupported(&self, node: &GrammarASTNode, what: &str) -> JsLowerError {
        JsLowerError {
            message: format!(
                "`{what}` is out of scope for this milestone; deferred to a later one"
            ),
            line: node.start_line.unwrap_or(0),
            column: node.start_column.unwrap_or(0),
        }
    }
}

// ---------------------------------------------------------------------------
// Lowered — a top-level item is either a statement or a tail expression
// ---------------------------------------------------------------------------

/// The result of lowering one top-level `source_element`.
///
/// Bindings and assignments are [`Stmt`]s that accumulate in the block;
/// a bare expression is a candidate tail value.  Keeping the distinction
/// explicit (rather than always wrapping in `ExprStmt`) lets
/// [`Lowerer::lower_program`] route the final expression into the block's
/// `value` slot, matching the SIR "statements then a value" block shape.
///
/// `Stmt` is boxed: it is substantially larger than `Expr` (it embeds
/// loop/class/try variants), so an unboxed enum would size every
/// `Lowered` to the largest variant — clippy's `large_enum_variant`.
enum Lowered {
    Stmt(Box<Stmt>),
    Expr(Expr),
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

/// Peel a node's single-child precedence spine down to the first node
/// that *branches* (more than one child) or is a leaf (single token
/// child).  Returns that node.  Unlike [`single_child_node`] this keeps
/// descending; it's used to classify an expression's "real" shape
/// without lowering it (e.g. is the LHS of an `expression_statement` an
/// assignment?).
fn peel_to_branch(node: &GrammarASTNode) -> &GrammarASTNode {
    let mut cur = node;
    while let Some(next) = single_child_node(cur) {
        cur = next;
    }
    cur
}

/// If `node` is a leaf wrapper bottoming out at a single token, return
/// that token.  Peels the precedence spine first, so
/// `single_leaf_token(primary_expression-wrapping-`x`)` yields the `x`
/// token.  Returns `None` if the bottom node branches.
fn single_leaf_token(node: &GrammarASTNode) -> Option<&Token> {
    peel_to_branch(node).token()
}

/// Return every direct child of `node` that is a *node* (dropping the
/// interleaved tokens), in source order.  Used by the control-flow lowerers
/// to read a statement's operand nodes positionally — e.g. an
/// `if_statement`'s `[cond, then, else]` or a loop's trailing body node —
/// without having to thread past the keyword/paren tokens.
fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
}

/// Return the first direct child node of `node` whose `rule_name` is
/// `name`, if any.
fn child_node_named<'a>(node: &'a GrammarASTNode, name: &str) -> Option<&'a GrammarASTNode> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) if n.rule_name == name => Some(n),
        _ => None,
    })
}

/// Return every direct child node of `node` whose `rule_name` is `name`.
fn children_nodes_named<'a>(node: &'a GrammarASTNode, name: &str) -> Vec<&'a GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == name => Some(n),
            _ => None,
        })
        .collect()
}
