//! The lowering pass from `python_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **milestone M4**.
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
//! is lowered to the matching SIR node (`+`/`-`/`*`/`/`/`%` →
//! `BuiltinCall`, `and`/`or` → `LogicalAnd`/`LogicalOr`, comparisons →
//! `BuiltinCall("<"/"="/…)`, `not`/unary `-`/`+`).  M2 also adds
//! **variable references** (bare `Name` → `VarRef` with resolved scope)
//! and **assignment** (`x = …` → `LetStarBinding` first time / `Assign`
//! after — first-occurrence detection).
//!
//! # What M3 adds
//!
//! M3 adds **control flow**: `if`/`elif`/`else` → nested [`Expr::If`];
//! `while c: body` → [`Stmt::While`]; `for x in range(...): body` →
//! [`Stmt::ForRange`] (arity 1/2/3 → start/stop/step) and any other
//! iterable → [`Stmt::ForEach`].  Each suite lowers to a [`Block`] via
//! [`Lowerer::lower_suite`].  The declared-name table is a **stack** so
//! block-local names (loop vars, names first-bound inside a body) are
//! scoped exactly as the SIR validator scopes them (`mark`/`rewind`).
//!
//! # What M4 adds — functions, calls, closures
//!
//! M4 is the milestone that turns the single-`main` module into a real
//! function table:
//!
//! - **`def f(params): suite`** → a top-level [`Function`] named `f`.
//!   Lowering is **two-pass**: a first pass collects *every* function
//!   name (top-level and nested) so calls resolve and mutual recursion
//!   works; the second pass lowers each body.  Each `def` lowers as if
//!   it stood alone, with its parameters in scope as `Scope::Param`.
//! - **`return expr`** — a function body is a [`Block`] whose `.value`
//!   IS the return value, so a *tail* `return expr` sets `body.value =
//!   expr`, and falling off the end (no `return`) synthesises
//!   `body.value = NilLit` (Python's implicit `None`).  A **non-tail**
//!   (early) `return` is **rejected** with a positioned error per the
//!   SIR17 spec — the IR has no `Return` node, so early returns would
//!   need a control-flow lift that v0 does not perform.
//! - **`lambda params: expr`** → a fresh top-level synthesised
//!   [`Function`] named `__lambda_<N>` plus an [`Expr::MakeClosure`] at
//!   the use site.  Free variables (names the body reads that are not
//!   its own params/locals and not globals/top-level functions/builtins)
//!   become **captures**; inside the synthesised function they resolve
//!   as [`Scope::Capture`].
//! - **nested `def`** → the same treatment as a lambda: lifted to a
//!   top-level synthesised function with computed captures.  A bare
//!   reference to the nested function's name yields an
//!   [`Expr::MakeClosure`] (so the closure value can be returned/passed).
//! - **calls** `f(args)` — `f` a known function name → [`Expr::DirectCall`];
//!   `f` a builtin (`print`/`len`/`range`) → [`Expr::BuiltinCall`]; `f`
//!   a local/param/captured value (a closure handle) → an
//!   [`Expr::IndirectCall`] through that `VarRef`.
//!
//! The manifest declares `Closures` when any `MakeClosure` / capture /
//! `IndirectCall` is emitted, and `MutualRecursion` when two top-level
//! functions call each other.
//!
//! ## Free-variable analysis (how captures are computed)
//!
//! We scan the lambda/nested-`def` **body subtree of the CST** for bare
//! `Name` references, subtracting the names bound *within* that body
//! (its own params and names it assigns).  Any remaining name that is a
//! *local/param/capture of the enclosing function* becomes a capture
//! (its value is the enclosing reference, evaluated at the
//! `MakeClosure` site).  Names that resolve to a global / top-level
//! function / builtin need no capture — they are reachable directly
//! from the synthesised body.  This mirrors the Twig and Ruby frontends.
//!
//! ## Still deferred (later milestones)
//!
//! - collections (lists / dicts / indexing / comprehensions)  → M5+
//! - `*args` / keyword & default arguments                    → deferred
//! - decorators / classes / `with` / `try` / generators       → deferred
//! - `global` / `nonlocal`, multi-target assignment           → deferred
//!
//! Unhandled rules produce a clear `PythonLowerError` rather than
//! silently dropping source.
//!
//! See `code/specs/SIR17-python-to-semantic-ir.md` for the full
//! lowering table and the deferred-form roadmap.

use std::collections::HashSet;

use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, Capture, CaptureValue, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata,
    Module, Param, ParamKind, Scope, Span, Stmt,
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
/// `for` body — and (M4) each `def` / `lambda` body — re-enters the
/// suite/expression lowerer one level deeper, so a pathological tower of
/// nested bodies would recurse without bound.  Mirroring
/// [`MAX_EXPR_DEPTH`]'s role for expressions, this cap turns deeply
/// nested control flow / closures into a clean positioned
/// `PythonLowerError` instead of a native (uncatchable) stack overflow.
/// It is generous: real source nests a handful of levels, far below this.
const MAX_BLOCK_DEPTH: usize = 256;

/// The builtin function names M4 recognises in call position.  `range`
/// is also recognised structurally inside `for` headers (M3); here it is
/// a general expression-position builtin (`range(n)` outside a `for`).
const BUILTIN_CALLS: &[&str] = &["print", "len", "range"];

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

/// Lower a parsed Python CST into a SIR module (M4: literals, variable
/// references, assignment, operators, control flow, functions, calls,
/// and closures).
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, PythonLowerError> {
    Lowerer::new(module_name).lower_file(tree)
}

// ---------------------------------------------------------------------------
// The lowerer
// ---------------------------------------------------------------------------

/// One lowered top-level / suite statement: either a `Stmt` or a bare
/// expression (an expression statement / the trailing block value).
///
/// `Stmt` is boxed because it is by far the largest variant (it holds a
/// full `Stmt`, which transitively contains `Block`s); boxing keeps the
/// enum small and silences `clippy::large_enum_variant`.
enum Lowered {
    Stmt(Box<Stmt>),
    Expr(Expr),
}

/// Per-function name-resolution context (M4).
///
/// `main` and every user / synthesised `def`/`lambda` get their own
/// `FunctionCtx`.  It tracks the names visible *inside this function*:
///
/// - `params` — the function's parameters (resolve as [`Scope::Param`]).
/// - `captures` — names captured from an enclosing function (resolve as
///   [`Scope::Capture`]); empty for top-level functions and `main`.
/// - `locals` — a LIFO stack of `let*`-bound / loop-variable names,
///   mirroring the SIR validator's `LocalEnv` (block-scoped via
///   `mark`/`rewind`).
///
/// Module-level function names and builtins live on the [`Lowerer`]
/// itself (they are the same for every function), so they are not
/// duplicated here.
struct FunctionCtx {
    params: HashSet<String>,
    captures: HashSet<String>,
    locals: Vec<String>,
}

impl FunctionCtx {
    /// A context for a function with the given params and captures.
    fn new(params: HashSet<String>, captures: HashSet<String>) -> Self {
        Self {
            params,
            captures,
            locals: Vec::new(),
        }
    }

    /// The top-level / `main` context: no params, no captures.
    fn top_level() -> Self {
        Self::new(HashSet::new(), HashSet::new())
    }

    /// Is `name` an enclosing-scope value (local / param / capture) of
    /// *this* function?  Such a name, when read inside a nested
    /// `lambda`/`def`, must be **captured**.
    fn is_enclosing_value(&self, name: &str) -> bool {
        self.locals.iter().any(|n| n == name)
            || self.params.contains(name)
            || self.captures.contains(name)
    }
}

struct Lowerer {
    module_name: String,
    /// Features observed while lowering, used to build the manifest so
    /// it declares *exactly* what the module emits.
    observed: FeatureManifest,
    /// Every function name known to the module: user `def`s (top-level
    /// and nested) plus synthesised `__lambda_<N>` names.  Collected in
    /// a *first pass* (see [`Self::collect_function_names`]) so a call to
    /// a function defined later in the file — and mutual recursion —
    /// resolve as [`Expr::DirectCall`].
    function_names: HashSet<String>,
    /// The synthesised + user functions accumulated during lowering, in
    /// definition order.  `main` is appended last by [`Self::lower_file`].
    functions: Vec<Function>,
    /// Counter for `__lambda_<N>` gensym names.
    lambda_counter: usize,
    /// For each *lifted* function (nested `def` / lambda) that carries
    /// captures, its ordered capture-name list.  When a **bare reference**
    /// to such a function name appears (constructing a closure handle
    /// without calling it), the `MakeClosure` must re-thread those
    /// captures from the *currently visible* enclosing values — this map
    /// records what to thread.  A function with no captures is absent
    /// (its bare reference is a zero-capture `MakeClosure`).
    fn_captures: std::collections::HashMap<String, Vec<String>>,
    /// Call graph among *top-level* functions: `caller → callees` by
    /// name.  Used after lowering to detect [`Feature::MutualRecursion`]
    /// (a cycle of length ≥ 2 — two functions that call each other).
    call_graph: Vec<(String, HashSet<String>)>,
}

impl Lowerer {
    fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            observed: FeatureManifest::new(),
            function_names: HashSet::new(),
            functions: Vec::new(),
            lambda_counter: 0,
            fn_captures: std::collections::HashMap::new(),
            call_graph: Vec::new(),
        }
    }

    // -------------------------------------------------------------------
    // scope stack: mirror the validator's `LocalEnv` mark/rewind so
    // block-local bindings (loop vars, names first-bound inside a body)
    // do not leak past the block — exactly as the validator scopes them.
    // -------------------------------------------------------------------

    /// Remember the current local-stack depth before entering a block.
    fn scope_mark(ctx: &FunctionCtx) -> usize {
        ctx.locals.len()
    }

    /// Drop every local bound since `mark`, leaving the enclosing scope.
    fn scope_rewind(ctx: &mut FunctionCtx, mark: usize) {
        ctx.locals.truncate(mark);
    }

    // -------------------------------------------------------------------
    // top level: `file` → collect function names, then synthesise `main`
    // -------------------------------------------------------------------

    /// The CST root is a `file` rule whose children are top-level
    /// `statement` nodes interleaved with stray `Newline` tokens.
    ///
    /// M4 makes this two-pass:
    ///
    /// 1. **Collect** every function name (top-level and nested `def`s)
    ///    so calls and mutual recursion resolve regardless of textual
    ///    order.
    /// 2. **Lower** each top-level statement.  A `def` lowers to a
    ///    [`Function`] (appended to the function table, *not* a `main`
    ///    statement); every other statement contributes to `main`'s body
    ///    exactly as in M3 (the trailing bare expression becomes the
    ///    block value).
    fn lower_file(&mut self, file: &GrammarASTNode) -> Result<Module, PythonLowerError> {
        if file.rule_name != "file" {
            return Err(self.err_at(
                file,
                format!("expected `file` root, got `{}`", file.rule_name),
            ));
        }

        // ── Pass 1: collect all function names (top-level + nested). ──
        for child in &file.children {
            if let ASTNodeOrToken::Node(stmt) = child {
                self.collect_function_names(stmt, 0)?;
            }
        }

        // ── Pass 2: lower each top-level statement. ──
        let mut ctx = FunctionCtx::top_level();
        let mut items: Vec<Lowered> = Vec::new();
        for child in &file.children {
            if let ASTNodeOrToken::Node(stmt) = child {
                if let Some(item) = self.lower_top_statement(stmt, &mut ctx, 0)? {
                    items.push(item);
                }
            }
            // Token children at file level are stray NEWLINEs — ignore.
        }

        let span = Span::point(FILE, 1, 1);
        let body = Self::assemble_block(items, &span);

        let main = Function {
            name: "main".to_string(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: span.clone(),
        };

        // User + synthesised functions first, then `main`.  The
        // validator does not care about ordering.
        let mut functions = std::mem::take(&mut self.functions);
        functions.push(main);

        // Detect mutual recursion among top-level functions.
        if self.has_mutual_recursion() {
            self.observed.add(Feature::MutualRecursion);
        }

        let metadata = Metadata::new()
            .with_source_language("python")
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

    /// Assemble a list of lowered items into a [`Block`].  The trailing
    /// item, **iff it is a bare expression**, becomes the block's value;
    /// otherwise (statement last, or empty) the value is `NilLit` and
    /// every item becomes a statement (bare expressions → `ExprStmt`).
    fn assemble_block(mut items: Vec<Lowered>, span: &Span) -> Block {
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
        Block {
            stmts,
            value,
            span: span.clone(),
        }
    }

    // -------------------------------------------------------------------
    // Pass 1: collect every function name (top-level + nested)
    // -------------------------------------------------------------------

    /// Recursively collect every `def`'s name into `function_names`, so
    /// the second (lowering) pass can resolve calls — including
    /// forward references and mutual recursion — to `DirectCall`.
    ///
    /// We descend into `def` suites too, so *nested* `def` names are
    /// known before their use; nested defs are lifted to top-level
    /// synthesised functions during lowering, but their names live in the
    /// same flat table.  `lambda` names are *not* collected here — they
    /// are gensym'd on the fly during lowering (a lambda has no source
    /// name to forward-reference).
    ///
    /// `depth` bounds the *suite-nesting* recursion: this pass-1 walk runs
    /// **before** the depth-guarded lowering, so a pathological tower of
    /// nested `def`s (or compounds) would otherwise overflow the native
    /// (uncatchable) stack here.  Past [`MAX_BLOCK_DEPTH`] we return a
    /// clean positioned `PythonLowerError`, mirroring the lowering guard.
    fn collect_function_names(
        &mut self,
        stmt: &GrammarASTNode,
        depth: usize,
    ) -> Result<(), PythonLowerError> {
        if depth > MAX_BLOCK_DEPTH {
            return Err(self.err_at(
                stmt,
                format!("control-flow nesting too deep (exceeds {MAX_BLOCK_DEPTH} levels)"),
            ));
        }
        // Only `statement` nodes carry defs; recurse structurally.
        if let Some(def) = self.as_def_stmt(stmt) {
            let name = self.def_name(def)?;
            self.function_names.insert(name);
            // Descend into the def's suite for nested defs.
            if let Some(suite) = self.first_child_named(def, "suite") {
                for child in &suite.children {
                    if let ASTNodeOrToken::Node(inner) = child {
                        if inner.rule_name == "statement" {
                            self.collect_function_names(inner, depth + 1)?;
                        }
                    }
                }
            }
        } else {
            // Non-def compound statements (if/while/for) may *contain*
            // nested defs in their suites — but Python forbids `def`
            // inside an `if`/loop at our v0 subset's depth resolution
            // semantics only at lowering time.  Still, scan suites so a
            // `def` nested under control flow is name-collected.
            for suite in self.descendant_suites(stmt) {
                for child in &suite.children {
                    if let ASTNodeOrToken::Node(inner) = child {
                        if inner.rule_name == "statement" {
                            self.collect_function_names(inner, depth + 1)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// If `stmt` is a `statement` wrapping a `compound_stmt` wrapping a
    /// `def_stmt`, return the `def_stmt` node.
    fn as_def_stmt<'a>(&self, stmt: &'a GrammarASTNode) -> Option<&'a GrammarASTNode> {
        if stmt.rule_name != "statement" {
            return None;
        }
        let compound = child_nodes(stmt)
            .into_iter()
            .find(|n| n.rule_name == "compound_stmt")?;
        child_nodes(compound)
            .into_iter()
            .find(|n| n.rule_name == "def_stmt")
    }

    /// The direct `suite` children of any `compound_stmt` inside
    /// `statement` *other than* a `def` — used by name collection to find
    /// defs nested under `if`/`while`/`for`.  (A `def`'s own suite is
    /// handled separately so its name is recorded first.)
    fn descendant_suites<'a>(&self, stmt: &'a GrammarASTNode) -> Vec<&'a GrammarASTNode> {
        let mut out = Vec::new();
        if stmt.rule_name != "statement" {
            return out;
        }
        if let Some(compound) = child_nodes(stmt)
            .into_iter()
            .find(|n| n.rule_name == "compound_stmt")
        {
            for inner in child_nodes(compound) {
                if inner.rule_name == "def_stmt" {
                    continue;
                }
                for s in child_nodes(inner) {
                    if s.rule_name == "suite" {
                        out.push(s);
                    }
                }
            }
        }
        out
    }

    /// Extract a `def_stmt`'s declared name (the `NAME` token after the
    /// `def` keyword).
    fn def_name(&self, def: &GrammarASTNode) -> Result<String, PythonLowerError> {
        for child in &def.children {
            if let ASTNodeOrToken::Token(t) = child {
                if matches!(t.type_, lexer::token::TokenType::Name) && t.type_name.is_none() {
                    return Ok(t.value.clone());
                }
            }
        }
        Err(self.err_at(def, "malformed def: missing function name".to_string()))
    }

    // -------------------------------------------------------------------
    // Pass 2 — statement → assignment / expression / compound / def
    // -------------------------------------------------------------------

    /// Lower a top-level (`main`-body) `statement`.  Returns `None` when
    /// the statement is a `def` (which is lifted to the function table and
    /// contributes nothing to `main`'s body), or `Some(item)` otherwise.
    fn lower_top_statement(
        &mut self,
        stmt: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Lowered>, PythonLowerError> {
        if let Some(def) = self.as_def_stmt(stmt) {
            // A top-level def: lift to the function table and discard the
            // definition-site closure value (nothing consumes it at the
            // top level, and a top-level def captures nothing).  We drop
            // the returned `MakeClosure` without retaining it, so it does
            // not contribute a `Closures` feature.
            self.lower_def(def, ctx, depth)?;
            return Ok(None);
        }
        Ok(Some(self.lower_statement(stmt, ctx, depth)?))
    }

    /// Lower a `statement` (already known *not* to be a top-level `def`
    /// the caller wants lifted; a `def` reaching here is a nested def and
    /// lowers to a `MakeClosure`-yielding lifted function).
    ///
    /// `depth` is the statement-block nesting depth; it bounds
    /// [`MAX_BLOCK_DEPTH`] so pathologically nested control flow / bodies
    /// fail cleanly instead of overflowing the native stack.
    fn lower_statement(
        &mut self,
        stmt: &GrammarASTNode,
        ctx: &mut FunctionCtx,
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
            "simple_stmt" => self.lower_simple_stmt(inner, ctx),
            "compound_stmt" => self.lower_compound_stmt(inner, ctx, depth),
            other => Err(self.err_at(
                inner,
                format!("unsupported: {other} (deferred to a later milestone)"),
            )),
        }
    }

    /// Lower a `simple_stmt` — an assignment, a `return`, or a bare
    /// expression.  A `return` reaching here is a **tail** return (the
    /// suite lowerer special-cases the final statement); a `return` in
    /// any other position is an early return and is rejected.
    fn lower_simple_stmt(
        &mut self,
        simple: &GrammarASTNode,
        ctx: &mut FunctionCtx,
    ) -> Result<Lowered, PythonLowerError> {
        let small = self.expect_single_named(simple, "simple_stmt", &["small_stmt"])?;
        // `small_stmt` wraps the actual statement: `assign_stmt` (M2) or
        // (M4) `return_stmt`.  Anything else is deferred/unsupported.
        let inner = match child_nodes(small).as_slice() {
            [only] => *only,
            _ => {
                return Err(self.err_at(
                    small,
                    "unsupported: small statement with multiple parts (deferred)".to_string(),
                ))
            }
        };
        match inner.rule_name.as_str() {
            "assign_stmt" => self.lower_assign_stmt(inner, ctx),
            "return_stmt" => {
                // A `return` reaching the *general* statement lowerer is
                // an early (non-tail) return — rejected per spec.  Tail
                // returns are intercepted by `lower_suite_no_mark`.
                Err(self.err_at(
                    inner,
                    "early return not supported in v0 (return must be the last statement of a function)"
                        .to_string(),
                ))
            }
            other => Err(self.err_at(
                inner,
                format!("unsupported: {other} (deferred to a later milestone)"),
            )),
        }
    }

    /// Lower an `assign_stmt` — `expression_list (assign_suffix)?`.  With
    /// an `assign_suffix` (`= rhs`) present it is a real assignment;
    /// otherwise it is a bare expression statement.
    fn lower_assign_stmt(
        &mut self,
        assign: &GrammarASTNode,
        ctx: &mut FunctionCtx,
    ) -> Result<Lowered, PythonLowerError> {
        let node_children: Vec<&GrammarASTNode> = child_nodes(assign);
        let suffix = node_children
            .iter()
            .find(|n| n.rule_name == "assign_suffix")
            .copied();

        let lhs_list = self.expect_single_kind(assign, "expression_list")?;

        match suffix {
            None => {
                let expr = self.lower_expr(self.single_expr(lhs_list)?, ctx)?;
                Ok(Lowered::Expr(expr))
            }
            Some(suffix) => self.lower_assignment(assign, lhs_list, suffix, ctx),
        }
    }

    /// Lower `target = rhs`.  First-occurrence detection: the first
    /// assignment to a name emits a `LetStarBinding`; a later assignment
    /// to an already-declared name emits an `Assign`.
    fn lower_assignment(
        &mut self,
        assign: &GrammarASTNode,
        lhs_list: &GrammarASTNode,
        suffix: &GrammarASTNode,
        ctx: &mut FunctionCtx,
    ) -> Result<Lowered, PythonLowerError> {
        let rhs_list = self.expect_single_kind(suffix, "expression_list")?;

        let target_node = self.single_expr(lhs_list)?;
        let rhs_node = self.single_expr(rhs_list)?;

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
        // first binding (`x = x`) correctly sees `x` as still-unbound.
        let value = self.lower_expr(rhs_node, ctx)?;
        let span = self.span_of(assign);

        if ctx.locals.iter().any(|n| n == &name) {
            self.observed.add(Feature::MutableBindings);
            Ok(Lowered::Stmt(Box::new(Stmt::Assign {
                name,
                scope: Scope::Local,
                value,
                span,
            })))
        } else {
            ctx.locals.push(name.clone());
            Ok(Lowered::Stmt(Box::new(Stmt::LetStarBinding {
                name,
                sir_type: None,
                value,
                span,
            })))
        }
    }

    /// If `node` is an expression that is *just* a bare `Name` atom,
    /// return that name (used to recognise an assignment / call target).
    fn target_name(&self, node: &GrammarASTNode) -> Result<Option<String>, PythonLowerError> {
        let mut cur = node;
        let mut depth = 0usize;
        loop {
            if depth > MAX_EXPR_DEPTH {
                return Err(self.err_at(node, "assignment target nesting too deep".to_string()));
            }
            if let Some(tok) = cur.token() {
                if matches!(tok.type_, lexer::token::TokenType::Name) && tok.type_name.is_none() {
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
    // compound statements: if / while / for / def
    // -------------------------------------------------------------------

    /// Lower a `compound_stmt` — control flow or a nested `def`.
    fn lower_compound_stmt(
        &mut self,
        compound: &GrammarASTNode,
        ctx: &mut FunctionCtx,
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
            "if_stmt" => Ok(Lowered::Expr(self.lower_if(inner, ctx, depth)?)),
            "while_stmt" => Ok(Lowered::Stmt(Box::new(self.lower_while(inner, ctx, depth)?))),
            "for_stmt" => Ok(Lowered::Stmt(Box::new(self.lower_for(inner, ctx, depth)?))),
            "def_stmt" => {
                // A nested `def` reaching the general statement lowerer:
                // lift it to a top-level synthesised function with
                // captures, then yield a `MakeClosure` so the closure
                // value is produced at this position (e.g. as a block
                // value to be `return`ed).
                let mc = self.lower_def(inner, ctx, depth)?;
                // This `MakeClosure` is retained (it becomes the block
                // value) → the module uses closures.
                self.observed.add(Feature::Closures);
                Ok(Lowered::Expr(mc))
            }
            other => Err(self.err_at(
                inner,
                format!("unsupported: {other} (deferred to a later milestone)"),
            )),
        }
    }

    /// Lower an `if_stmt` into a nested chain of [`Expr::If`].  (Unchanged
    /// from M3 except for threading the [`FunctionCtx`].)
    fn lower_if(
        &mut self,
        if_stmt: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Expr, PythonLowerError> {
        struct Clause<'a> {
            cond: &'a GrammarASTNode,
            suite: &'a GrammarASTNode,
        }
        let mut clauses: Vec<Clause> = Vec::new();
        let mut else_suite: Option<&GrammarASTNode> = None;

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
                _ => {}
            }
        }

        if clauses.is_empty() {
            return Err(self.err_at(if_stmt, "malformed if: no clauses".to_string()));
        }

        let if_span = self.span_of(if_stmt);

        let mut else_branch: Block = match else_suite {
            Some(s) => self.lower_suite(s, ctx, depth + 1)?,
            None => empty_block(if_span.clone()),
        };

        for clause in clauses.into_iter().rev() {
            let cond = self.lower_expr(clause.cond, ctx)?;
            let then_branch = self.lower_suite(clause.suite, ctx, depth + 1)?;
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

    /// Lower a `while_stmt` into [`Stmt::While`].
    fn lower_while(
        &mut self,
        while_stmt: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Stmt, PythonLowerError> {
        let cond_node = self
            .first_child_named(while_stmt, "expression")
            .ok_or_else(|| self.err_at(while_stmt, "malformed while: no condition".to_string()))?;
        let suite = self
            .first_child_named(while_stmt, "suite")
            .ok_or_else(|| self.err_at(while_stmt, "malformed while: no body".to_string()))?;

        let cond = self.lower_expr(cond_node, ctx)?;
        let body = self.lower_suite(suite, ctx, depth + 1)?;
        self.observed.add(Feature::Loops);
        Ok(Stmt::While {
            cond,
            body,
            span: self.span_of(while_stmt),
        })
    }

    /// Lower a `for_stmt` into [`Stmt::ForRange`] (literal `range(...)`
    /// iterable) or [`Stmt::ForEach`] (any other iterable).
    fn lower_for(
        &mut self,
        for_stmt: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Stmt, PythonLowerError> {
        let target_list = self
            .first_child_named(for_stmt, "target_list")
            .ok_or_else(|| self.err_at(for_stmt, "malformed for: no target".to_string()))?;
        let iter_list = self
            .first_child_named(for_stmt, "expression_list")
            .ok_or_else(|| self.err_at(for_stmt, "malformed for: no iterable".to_string()))?;
        let suite = self
            .first_child_named(for_stmt, "suite")
            .ok_or_else(|| self.err_at(for_stmt, "malformed for: no body".to_string()))?;

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

        let iter_expr_node = self.single_expr(iter_list)?;
        let span = self.span_of(for_stmt);

        let range = self.try_range_call(iter_expr_node, ctx)?;
        self.observed.add(Feature::Loops);

        match range {
            Some((start, stop, step)) => {
                let mark = Self::scope_mark(ctx);
                ctx.locals.push(var.clone());
                let body = self.lower_suite_no_mark(suite, ctx, depth + 1)?;
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
            None => {
                let iter = self.lower_expr(iter_expr_node, ctx)?;
                let mark = Self::scope_mark(ctx);
                ctx.locals.push(var.clone());
                let body = self.lower_suite_no_mark(suite, ctx, depth + 1)?;
                Self::scope_rewind(ctx, mark);
                Ok(Stmt::ForEach {
                    var,
                    iter,
                    body,
                    span,
                })
            }
        }
    }

    /// Recognise a literal `range(...)` call inside a `for` header and
    /// lower its 1/2/3 arguments into `(start, stop, step)`.
    fn try_range_call(
        &mut self,
        iter: &GrammarASTNode,
        ctx: &mut FunctionCtx,
    ) -> Result<Option<(Expr, Expr, Expr)>, PythonLowerError> {
        let primary = match self.peel_to_primary(iter) {
            Some(p) => p,
            None => return Ok(None),
        };

        let kids = child_nodes(primary);
        let (callee, suffix) = match kids.as_slice() {
            [callee, suffix] if suffix.rule_name == "suffix" => (*callee, *suffix),
            _ => return Ok(None),
        };

        match self.target_name(callee)? {
            Some(name) if name == "range" => {}
            _ => return Ok(None),
        }

        let args = self.call_arguments(suffix);

        let span = self.span_of(primary);
        let int = |v: i64| Expr::IntLit {
            value: v,
            span: span.clone(),
        };

        let arg_expr = |me: &mut Self,
                        a: &GrammarASTNode,
                        ctx: &mut FunctionCtx|
         -> Result<Expr, PythonLowerError> {
            let expr = me.single_arg_expr(a)?;
            me.lower_expr(expr, ctx)
        };

        match args.as_slice() {
            [n] => {
                let stop = arg_expr(self, n, ctx)?;
                Ok(Some((int(0), stop, int(1))))
            }
            [a, b] => {
                let start = arg_expr(self, a, ctx)?;
                let stop = arg_expr(self, b, ctx)?;
                Ok(Some((start, stop, int(1))))
            }
            [a, b, c] => {
                let start = arg_expr(self, a, ctx)?;
                let stop = arg_expr(self, b, ctx)?;
                let step = arg_expr(self, c, ctx)?;
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

    /// Collect the `argument` nodes from a call `suffix` (`( args )`).
    fn call_arguments<'a>(&self, suffix: &'a GrammarASTNode) -> Vec<&'a GrammarASTNode> {
        child_nodes(suffix)
            .into_iter()
            .filter(|n| n.rule_name == "arguments")
            .flat_map(child_nodes)
            .filter(|n| n.rule_name == "argument")
            .collect()
    }

    /// Peel an expression node down to the `primary` rule.
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

    // -------------------------------------------------------------------
    // def / lambda → lifted Function (+ MakeClosure)
    // -------------------------------------------------------------------

    /// Lower a `def_stmt` into a top-level [`Function`].
    ///
    /// `enclosing` is the context the `def` is *written inside* (the
    /// top-level `main` ctx, or another function's ctx for a nested
    /// `def`).  The returned [`Expr`] is an [`Expr::MakeClosure`] that
    /// constructs the closure value at the definition site — captures are
    /// evaluated against `enclosing`.  For a *top-level* `def` the
    /// captures are empty (nothing to capture) and the caller discards
    /// the `MakeClosure`; for a *nested* `def` the `MakeClosure` becomes
    /// the value produced where the `def` appeared (so it can be
    /// `return`ed or assigned).
    fn lower_def(
        &mut self,
        def: &GrammarASTNode,
        enclosing: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Expr, PythonLowerError> {
        let name = self.def_name(def)?;
        let params = self.def_params(def)?;
        let suite = self
            .first_child_named(def, "suite")
            .ok_or_else(|| self.err_at(def, "malformed def: missing body".to_string()))?;
        let span = self.span_of(def);

        self.lower_callable(&name, &params, suite, enclosing, depth, span)
    }

    /// Extract a `def_stmt`'s parameter names (rejecting defaults / `*args`
    /// / `**kwargs`, which are deferred).
    fn def_params(&self, def: &GrammarASTNode) -> Result<Vec<String>, PythonLowerError> {
        let parameters = match self.first_child_named(def, "parameters") {
            Some(p) => p,
            None => return Ok(vec![]), // `def f():` — no params.
        };
        // `parameters → parameter_list → param_with_default+`.
        let list = self
            .first_child_named(parameters, "parameter_list")
            .unwrap_or(parameters);
        let mut names = Vec::new();
        for pwd in child_nodes(list) {
            if pwd.rule_name != "param_with_default" {
                continue;
            }
            names.push(self.param_name(pwd)?);
        }
        Ok(names)
    }

    /// Extract one parameter's name from a `param_with_default`, rejecting
    /// a default value (`a=1`) — the node then carries an extra `EQUALS`
    /// token / default `expression`, which v0 does not model.
    fn param_name(&self, pwd: &GrammarASTNode) -> Result<String, PythonLowerError> {
        // A plain parameter is exactly one `NAME` token.  A default adds
        // an `EQUALS` token (+ a default expression node).
        let has_default = pwd.children.iter().any(|c| {
            matches!(c, ASTNodeOrToken::Token(t) if t.value == "=")
                || matches!(c, ASTNodeOrToken::Node(_))
        });
        if has_default {
            return Err(self.err_at(
                pwd,
                "unsupported: default parameter value (deferred)".to_string(),
            ));
        }
        for child in &pwd.children {
            if let ASTNodeOrToken::Token(t) = child {
                if matches!(t.type_, lexer::token::TokenType::Name) && t.type_name.is_none() {
                    return Ok(t.value.clone());
                }
            }
        }
        Err(self.err_at(pwd, "malformed parameter".to_string()))
    }

    /// Lower a `lambda_expr` into a fresh top-level synthesised
    /// [`Function`] plus an [`Expr::MakeClosure`] at the use site.
    fn lower_lambda(
        &mut self,
        lambda: &GrammarASTNode,
        enclosing: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Expr, PythonLowerError> {
        let span = self.span_of(lambda);
        let params = self.lambda_params(lambda)?;
        let body_expr = self
            .first_child_named(lambda, "expression")
            .ok_or_else(|| self.err_at(lambda, "malformed lambda: missing body".to_string()))?;

        let fn_name = self.fresh_lambda_name();

        // A lambda's body is a single expression; compute its free
        // variables, lower it in a fresh ctx with those captures, and
        // emit the closure body + a MakeClosure.
        let bound: HashSet<String> = params.iter().cloned().collect();
        let mut free = Vec::new();
        let mut seen = HashSet::new();
        self.collect_free_names(body_expr, &bound, &mut free, &mut seen, 0)?;

        let (captures, capture_values) =
            self.resolve_captures(&free, enclosing, &span)?;

        // Lower the body in the synthesised function's own context.
        let mut inner = FunctionCtx::new(bound.clone(), captures.iter().cloned().collect());
        let value = self.lower_expr_in(body_expr, &mut inner, depth + 1)?;
        let body = Block {
            stmts: vec![],
            value,
            span: span.clone(),
        };
        self.push_function(&fn_name, &params, &captures, body, span.clone());

        // A lambda always yields a retained closure value.
        self.observed.add(Feature::Closures);
        Ok(self.make_closure(fn_name, captures, capture_values, span))
    }

    /// Extract a `lambda_expr`'s parameter names (rejecting `*`/`**` and
    /// defaults).
    fn lambda_params(&self, lambda: &GrammarASTNode) -> Result<Vec<String>, PythonLowerError> {
        let params = match self.first_child_named(lambda, "lambda_params") {
            Some(p) => p,
            None => return Ok(vec![]), // `lambda: expr` — no params.
        };
        let mut names = Vec::new();
        for lp in child_nodes(params) {
            if lp.rule_name != "lambda_param" {
                continue;
            }
            // A default / `*`/`**` lambda param carries an extra token /
            // node beyond the lone NAME — reject.
            let has_extra = lp.children.iter().any(|c| {
                matches!(c, ASTNodeOrToken::Token(t) if t.value == "=" || t.value == "*" || t.value == "**")
                    || matches!(c, ASTNodeOrToken::Node(_))
            });
            if has_extra {
                return Err(self.err_at(
                    lp,
                    "unsupported: lambda default / *args / **kwargs (deferred)".to_string(),
                ));
            }
            for child in &lp.children {
                if let ASTNodeOrToken::Token(t) = child {
                    if matches!(t.type_, lexer::token::TokenType::Name) && t.type_name.is_none() {
                        names.push(t.value.clone());
                    }
                }
            }
        }
        Ok(names)
    }

    /// The shared back-end for `def` and (analogously) a lambda's body:
    /// given a function `name`, its `params`, and a `suite`, compute the
    /// captures (against `enclosing`), lower the suite in a fresh
    /// context, append the [`Function`] to the table, and return the
    /// [`Expr::MakeClosure`] for the definition site.
    fn lower_callable(
        &mut self,
        name: &str,
        params: &[String],
        suite: &GrammarASTNode,
        enclosing: &mut FunctionCtx,
        depth: usize,
        span: Span,
    ) -> Result<Expr, PythonLowerError> {
        // ── Free-variable analysis over the suite. ──
        // Names bound *within* the body — the params plus every name the
        // body assigns / `for`-binds — are body-local, not captures.
        let mut bound: HashSet<String> = params.iter().cloned().collect();
        self.collect_suite_bound_names(suite, &mut bound)?;
        let mut free = Vec::new();
        let mut seen = HashSet::new();
        self.collect_free_names(suite, &bound, &mut free, &mut seen, 0)?;

        let (captures, capture_values) = self.resolve_captures(&free, enclosing, &span)?;

        // ── Lower the body in the function's own context. ──
        let mut inner = FunctionCtx::new(
            params.iter().cloned().collect(),
            captures.iter().cloned().collect(),
        );
        let body = self.lower_function_suite(suite, &mut inner, depth + 1)?;

        self.push_function(name, params, &captures, body, span.clone());

        Ok(self.make_closure(name.to_string(), captures, capture_values, span))
    }

    /// Build a [`Function`] and append it to the table; also record its
    /// top-level call edges for mutual-recursion detection.
    fn push_function(
        &mut self,
        name: &str,
        params: &[String],
        captures: &[String],
        body: Block,
        span: Span,
    ) {
        // Any param with no annotation → DynamicTyping (Python has no
        // parameter type annotations in our subset).
        if !params.is_empty() {
            self.observed.add(Feature::DynamicTyping);
        }
        // A function that carries captures is a closure body — the
        // validator declares `Closures` for it, so we must too.
        if !captures.is_empty() {
            self.observed.add(Feature::Closures);
            // Record the capture list so a *bare reference* to this
            // function name re-threads the captures (a nested closure
            // returned/passed by name).
            self.fn_captures
                .insert(name.to_string(), captures.to_vec());
        }
        let f = Function {
            name: name.to_string(),
            params: params
                .iter()
                .map(|p| Param {
                    name: p.clone(),
                    sir_type: None,
                    kind: ParamKind::Required,
                    span: span.clone(),
                })
                .collect(),
            return_type: None,
            captures: captures
                .iter()
                .map(|n| Capture {
                    name: n.clone(),
                    sir_type: None,
                })
                .collect(),
            body,
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span,
        };
        // Record DirectCall edges out of this function (top-level call
        // graph) for mutual-recursion detection.
        let mut callees = HashSet::new();
        collect_direct_callees(&f.body, &mut callees);
        self.call_graph.push((name.to_string(), callees));
        self.functions.push(f);
    }

    /// Build the `MakeClosure` expression for a definition site, zipping
    /// capture names with their (enclosing-scope) values.
    ///
    /// This does **not** itself declare [`Feature::Closures`]: a
    /// top-level `def`'s `MakeClosure` is *discarded* (it never lands in a
    /// body), so the feature is declared by the *retaining* caller (a
    /// nested `def` / `lambda` / bare function-name reference) and by
    /// [`Self::push_function`] for any function that actually carries
    /// captures — mirroring exactly what the validator observes.
    fn make_closure(
        &mut self,
        fn_name: String,
        captures: Vec<String>,
        capture_values: Vec<Expr>,
        span: Span,
    ) -> Expr {
        Expr::MakeClosure {
            fn_name,
            captures: captures
                .into_iter()
                .zip(capture_values)
                .map(|(name, value)| CaptureValue { name, value })
                .collect(),
            span,
        }
    }

    /// Resolve the free names of a closure body against the `enclosing`
    /// context, partitioning them into *captures* (names that are an
    /// enclosing local/param/capture — these must be threaded through the
    /// closure handle) and non-captures (globals / top-level functions /
    /// builtins, reachable directly from the synthesised body).  Returns
    /// `(capture_names, capture_value_exprs)` in deterministic
    /// (alphabetical) order.
    fn resolve_captures(
        &mut self,
        free: &[String],
        enclosing: &mut FunctionCtx,
        span: &Span,
    ) -> Result<(Vec<String>, Vec<Expr>), PythonLowerError> {
        let mut names: Vec<String> = free
            .iter()
            .filter(|n| enclosing.is_enclosing_value(n))
            .cloned()
            .collect();
        names.sort();
        names.dedup();

        let mut values = Vec::with_capacity(names.len());
        for n in &names {
            // The capture *value* is the enclosing reference, resolved in
            // the enclosing context.
            values.push(self.resolve_var_in(enclosing, n, span.clone())?);
        }
        Ok((names, values))
    }

    fn fresh_lambda_name(&mut self) -> String {
        let name = format!("__lambda_{}", self.lambda_counter);
        self.lambda_counter += 1;
        self.function_names.insert(name.clone());
        name
    }

    /// Collect every name *bound* (assigned) inside a suite into `bound`,
    /// so the free-variable scan treats body-local assignments as bound
    /// rather than as captures.  Bare `x = …` targets are collected;
    /// nested-`def` names too (they shadow).
    fn collect_suite_bound_names(
        &self,
        suite: &GrammarASTNode,
        bound: &mut HashSet<String>,
    ) -> Result<(), PythonLowerError> {
        for child in &suite.children {
            if let ASTNodeOrToken::Node(stmt) = child {
                self.collect_stmt_bound_names(stmt, bound)?;
            }
        }
        Ok(())
    }

    fn collect_stmt_bound_names(
        &self,
        stmt: &GrammarASTNode,
        bound: &mut HashSet<String>,
    ) -> Result<(), PythonLowerError> {
        // assignment target?
        if let Some(def) = self.as_def_stmt(stmt) {
            if let Ok(n) = self.def_name(def) {
                bound.insert(n);
            }
            return Ok(());
        }
        // `for x in …:` binds `x`; descend into suites of compounds.
        for n in self.descendant_assign_targets(stmt)? {
            bound.insert(n);
        }
        Ok(())
    }

    /// Collect assignment targets and `for`-loop variables anywhere
    /// within `stmt` (descending suites), for body-local bound-name
    /// analysis.  Best-effort and conservative — over-binding only ever
    /// *reduces* captures, and a missing capture would surface as a
    /// validator error, so a name we fail to collect here cannot silently
    /// corrupt output.
    fn descendant_assign_targets(
        &self,
        stmt: &GrammarASTNode,
    ) -> Result<Vec<String>, PythonLowerError> {
        let mut out = Vec::new();
        self.walk_for_targets(stmt, &mut out, 0)?;
        Ok(out)
    }

    /// Recursively scan `node`'s subtree for assignment / `for`-target
    /// names.  This is a pre-lowering walk (it feeds free-variable
    /// analysis before the depth-guarded lowering runs), so it bounds its
    /// own recursion: it descends into *every* node child, so its depth
    /// tracks the *expression* nesting depth and is capped at
    /// [`MAX_EXPR_DEPTH`] — past which it returns a clean positioned error
    /// rather than overflowing the native stack.
    fn walk_for_targets(
        &self,
        node: &GrammarASTNode,
        out: &mut Vec<String>,
        depth: usize,
    ) -> Result<(), PythonLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        match node.rule_name.as_str() {
            "assign_stmt" => {
                // First expression_list is the LHS; if there's an
                // assign_suffix it's a real assignment target.
                let has_suffix = child_nodes(node)
                    .iter()
                    .any(|n| n.rule_name == "assign_suffix");
                if has_suffix {
                    if let Some(list) = self.first_child_named(node, "expression_list") {
                        if let Ok(Some(name)) = self
                            .single_expr(list)
                            .and_then(|e| self.target_name(e))
                        {
                            out.push(name);
                        }
                    }
                }
            }
            "for_stmt" => {
                if let Some(tl) = self.first_child_named(node, "target_list") {
                    for t in child_nodes(tl) {
                        if let Ok(Some(name)) = self.target_name(t) {
                            out.push(name);
                        }
                    }
                }
            }
            _ => {}
        }
        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                self.walk_for_targets(n, out, depth + 1)?;
            }
        }
        Ok(())
    }

    /// Collect free `Name` references in a CST subtree: every bare `Name`
    /// token that is *not* in `bound`, appended to `free` in first-seen
    /// order (`seen` deduplicates).  A `Name` that immediately precedes a
    /// call `suffix` is still collected (a called name can be a captured
    /// closure).  Nested `lambda`/`def` bodies bind their own params, so
    /// we extend `bound` when descending into them — but their *free*
    /// names that escape to *this* scope still surface (capture chaining
    /// at one level: an inner lambda referencing an outer-outer local is a
    /// documented v0 cut-line, but a single level works).
    ///
    /// `depth` bounds this pre-lowering walk's recursion.  Like the
    /// expression lowerer, it descends into *every* node child, so its
    /// depth tracks the *expression* nesting depth and is capped at
    /// [`MAX_EXPR_DEPTH`]; past the cap we return a clean positioned
    /// `PythonLowerError` instead of overflowing the native (uncatchable)
    /// stack on a pathologically deep input via the public `compile`.
    fn collect_free_names(
        &self,
        node: &GrammarASTNode,
        bound: &HashSet<String>,
        free: &mut Vec<String>,
        seen: &mut HashSet<String>,
        depth: usize,
    ) -> Result<(), PythonLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        // A nested lambda binds its params; descend with them added.
        if node.rule_name == "lambda_expr" {
            let mut inner = bound.clone();
            if let Ok(ps) = self.lambda_params(node) {
                for p in ps {
                    inner.insert(p);
                }
            }
            if let Some(body) = self.first_child_named(node, "expression") {
                self.collect_free_names(body, &inner, free, seen, depth + 1)?;
            }
            return Ok(());
        }
        // A nested def binds its name + params; descend into its suite
        // with those added.
        if node.rule_name == "def_stmt" {
            let mut inner = bound.clone();
            if let Ok(n) = self.def_name(node) {
                inner.insert(n);
            }
            if let Ok(ps) = self.def_params(node) {
                for p in ps {
                    inner.insert(p);
                }
            }
            if let Some(suite) = self.first_child_named(node, "suite") {
                self.collect_suite_bound_names(suite, &mut inner)?;
                self.collect_free_names(suite, &inner, free, seen, depth + 1)?;
            }
            return Ok(());
        }

        // A bare `Name` token (no type_name) is a reference.
        if let Some(tok) = node.token() {
            if matches!(tok.type_, lexer::token::TokenType::Name) && tok.type_name.is_none() {
                let name = &tok.value;
                if !bound.contains(name) && seen.insert(name.clone()) {
                    free.push(name.clone());
                }
            }
            return Ok(());
        }

        for child in &node.children {
            if let ASTNodeOrToken::Node(n) = child {
                self.collect_free_names(n, bound, free, seen, depth + 1)?;
            }
        }
        Ok(())
    }

    // -------------------------------------------------------------------
    // suites
    // -------------------------------------------------------------------

    /// Lower a *function* body `suite` into a [`Block`], intercepting a
    /// **tail** `return`.  The body's `value` IS the return value, so:
    ///
    /// - a final `return expr` → `body.value = expr`;
    /// - a final `return` (bare) or falling off the end → `value = NilLit`
    ///   (Python's implicit `None`);
    /// - a `return` in any *non-final* position → positioned error.
    fn lower_function_suite(
        &mut self,
        suite: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Block, PythonLowerError> {
        if suite.rule_name != "suite" {
            return Err(self.err_at(
                suite,
                format!("expected `suite`, got `{}`", suite.rule_name),
            ));
        }
        let stmt_nodes: Vec<&GrammarASTNode> = suite
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "statement" => Some(n),
                _ => None,
            })
            .collect();

        let span = self.span_of(suite);
        if stmt_nodes.is_empty() {
            // An empty function body (only `pass`-like) returns nil.
            return Ok(Block {
                stmts: vec![],
                value: Expr::NilLit { span: span.clone() },
                span,
            });
        }

        let mark = Self::scope_mark(ctx);

        // Lower every statement *except the last* with the normal rule —
        // a `return` here is non-tail and errors out.
        let mut items: Vec<Lowered> = Vec::new();
        for stmt in &stmt_nodes[..stmt_nodes.len() - 1] {
            items.push(self.lower_statement(stmt, ctx, depth)?);
        }

        // The last statement may be a tail `return`.
        let last = stmt_nodes[stmt_nodes.len() - 1];
        if let Some(ret) = self.as_return_stmt(last) {
            let value = self.lower_return_value(ret, ctx)?;
            let block = Self::assemble_block(items, &span);
            Self::scope_rewind(ctx, mark);
            return Ok(Block {
                stmts: block.stmts,
                value,
                span,
            });
        }

        // The last statement may be a tail `if` whose branches end in
        // `return`s — the canonical `if cond: return a else: return b`
        // shape.  Each branch is itself in *function-tail* position, so we
        // lower it as a function suite (its tail `return` becomes the
        // branch's block value) and fold into an `if`-expression.  This is
        // how an early-looking `return` inside a *tail* `if` is accepted
        // (it is not actually early — control reaches the function end).
        if let Some(if_stmt) = self.as_if_stmt(last) {
            let value = self.lower_if_tail(if_stmt, ctx, depth)?;
            let block = Self::assemble_block(items, &span);
            Self::scope_rewind(ctx, mark);
            return Ok(Block {
                stmts: block.stmts,
                value,
                span,
            });
        }

        // No tail return: the last statement lowers normally (its
        // trailing-expr value, if any, becomes the body value; else nil).
        items.push(self.lower_statement(last, ctx, depth)?);
        let block = Self::assemble_block(items, &span);
        Self::scope_rewind(ctx, mark);
        Ok(block)
    }

    /// If `stmt` is a `statement → compound_stmt → if_stmt`, return the
    /// `if_stmt` node.
    fn as_if_stmt<'a>(&self, stmt: &'a GrammarASTNode) -> Option<&'a GrammarASTNode> {
        if stmt.rule_name != "statement" {
            return None;
        }
        let compound = child_nodes(stmt)
            .into_iter()
            .find(|n| n.rule_name == "compound_stmt")?;
        child_nodes(compound)
            .into_iter()
            .find(|n| n.rule_name == "if_stmt")
    }

    /// Lower a **tail-position** `if_stmt` into a nested [`Expr::If`],
    /// lowering each branch suite as a *function suite* so a tail `return`
    /// inside a branch becomes that branch's block value.  Identical in
    /// structure to [`Self::lower_if`] except the branch lowerer is
    /// [`Self::lower_function_suite`] (tail-aware) rather than
    /// [`Self::lower_suite`].  A missing `else` yields a `NilLit` branch
    /// (Python's implicit `None` on the untaken path).
    fn lower_if_tail(
        &mut self,
        if_stmt: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Expr, PythonLowerError> {
        struct Clause<'a> {
            cond: &'a GrammarASTNode,
            suite: &'a GrammarASTNode,
        }
        let mut clauses: Vec<Clause> = Vec::new();
        let mut else_suite: Option<&GrammarASTNode> = None;
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
                _ => {}
            }
        }
        if clauses.is_empty() {
            return Err(self.err_at(if_stmt, "malformed if: no clauses".to_string()));
        }

        let if_span = self.span_of(if_stmt);
        let mut else_branch: Block = match else_suite {
            Some(s) => self.lower_function_suite(s, ctx, depth + 1)?,
            None => empty_block(if_span.clone()),
        };
        for clause in clauses.into_iter().rev() {
            let cond = self.lower_expr(clause.cond, ctx)?;
            let then_branch = self.lower_function_suite(clause.suite, ctx, depth + 1)?;
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

    /// If `stmt` is a `statement → simple_stmt → small_stmt → return_stmt`,
    /// return the `return_stmt` node.
    fn as_return_stmt<'a>(&self, stmt: &'a GrammarASTNode) -> Option<&'a GrammarASTNode> {
        if stmt.rule_name != "statement" {
            return None;
        }
        let simple = child_nodes(stmt)
            .into_iter()
            .find(|n| n.rule_name == "simple_stmt")?;
        let small = child_nodes(simple)
            .into_iter()
            .find(|n| n.rule_name == "small_stmt")?;
        child_nodes(small)
            .into_iter()
            .find(|n| n.rule_name == "return_stmt")
    }

    /// Lower a tail `return_stmt`'s value: `return expr` → `expr`; a bare
    /// `return` → `NilLit` (Python's implicit `None`).
    fn lower_return_value(
        &mut self,
        ret: &GrammarASTNode,
        ctx: &mut FunctionCtx,
    ) -> Result<Expr, PythonLowerError> {
        match self.first_child_named(ret, "expression_list") {
            Some(list) => {
                let expr = self.single_expr(list)?;
                self.lower_expr(expr, ctx)
            }
            None => Ok(Expr::NilLit {
                span: self.span_of(ret),
            }),
        }
    }

    /// Lower a control-flow `suite` (loop / branch body) into a [`Block`]
    /// with its own scope mark/rewind.  A `return` inside such a suite is
    /// always non-tail (it is nested in control flow, not the function
    /// tail) and is rejected by [`Self::lower_statement`].
    fn lower_suite(
        &mut self,
        suite: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Block, PythonLowerError> {
        let mark = Self::scope_mark(ctx);
        let block = self.lower_suite_no_mark(suite, ctx, depth)?;
        Self::scope_rewind(ctx, mark);
        Ok(block)
    }

    /// Like [`Self::lower_suite`] but without pushing/popping a scope mark
    /// (the caller manages scope, e.g. `for` binding its loop variable).
    fn lower_suite_no_mark(
        &mut self,
        suite: &GrammarASTNode,
        ctx: &mut FunctionCtx,
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
                    items.push(self.lower_statement(stmt, ctx, depth)?);
                }
            }
        }
        let span = self.span_of(suite);
        Ok(Self::assemble_block(items, &span))
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

    /// Lower an expression node in `ctx`.
    fn lower_expr(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
    ) -> Result<Expr, PythonLowerError> {
        self.lower_expr_in(node, ctx, 0)
    }

    /// Lower an expression with an explicit starting depth (used by
    /// closure-body lowering so the depth guard accounts for the
    /// enclosing block nesting).
    fn lower_expr_in(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Expr, PythonLowerError> {
        self.lower_expr_d(node, ctx, depth)
    }

    /// Depth-tracked core of expression lowering.  Bounded by
    /// [`MAX_EXPR_DEPTH`] so pathologically deep input fails cleanly
    /// instead of overflowing the native stack.
    fn lower_expr_d(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Expr, PythonLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }

        // A `lambda` can appear at any expression position.
        if node.rule_name == "lambda_expr" {
            return self.lower_lambda(node, ctx, depth);
        }

        match node.rule_name.as_str() {
            "or_expr" => {
                if let Some(e) = self.try_logical(node, ctx, depth, "or")? {
                    return Ok(e);
                }
            }
            "and_expr" => {
                if let Some(e) = self.try_logical(node, ctx, depth, "and")? {
                    return Ok(e);
                }
            }
            "not_expr" => {
                if let Some(e) = self.try_not(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "comparison" => {
                if let Some(e) = self.try_comparison(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "arith" | "term" => {
                if let Some(e) = self.try_binary_arith(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "factor" => {
                if let Some(e) = self.try_unary_factor(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            "primary" => {
                // A `primary` with a call `suffix` is a call expression.
                if let Some(e) = self.try_call(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            _ => {}
        }

        if let Some(tok) = node.token() {
            return self.lower_leaf_token(node, tok, ctx);
        }

        let kids = child_nodes(node);
        match kids.as_slice() {
            [only] if node.children.len() == 1 => self.lower_expr_d(only, ctx, depth + 1),
            _ => Err(self.err_at(
                node,
                format!(
                    "unsupported: {} (deferred to a later milestone)",
                    node.rule_name
                ),
            )),
        }
    }

    /// `primary` with a trailing call `suffix` → a call expression.
    /// Resolves the callee: a known function name → [`Expr::DirectCall`];
    /// a builtin (`print`/`len`/`range`) → [`Expr::BuiltinCall`]; a
    /// local/param/captured value → [`Expr::IndirectCall`] through that
    /// `VarRef`.  Returns `Ok(None)` when the `primary` is not a call
    /// (no `suffix`) so the generic peel handles it.
    fn try_call(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, PythonLowerError> {
        let kids = child_nodes(node);
        let (callee, suffix) = match kids.as_slice() {
            [callee, suffix] if suffix.rule_name == "suffix" => (*callee, *suffix),
            _ => return Ok(None),
        };
        // Only a *call* suffix `( … )` — not indexing `[ … ]` or
        // attribute `.x` (deferred).  A call suffix's first token is `(`.
        let is_call = matches!(
            suffix.children.first(),
            Some(ASTNodeOrToken::Token(t)) if t.value == "("
        );
        if !is_call {
            return Err(self.err_at(
                suffix,
                "unsupported: indexing / attribute access (deferred to a later milestone)"
                    .to_string(),
            ));
        }

        let span = self.span_of(node);

        // Lower the arguments first.
        let arg_nodes = self.call_arguments(suffix);
        let mut args = Vec::with_capacity(arg_nodes.len());
        for a in &arg_nodes {
            let e = self.single_arg_expr(a)?;
            args.push(self.lower_expr_d(e, ctx, depth + 1)?);
        }

        // The callee must be a bare name (no method calls in v0).
        let name = match self.target_name(callee)? {
            Some(n) => n,
            None => {
                return Err(self.err_at(
                    callee,
                    "unsupported: call of a non-name expression (deferred)".to_string(),
                ))
            }
        };

        // Builtin?
        if BUILTIN_CALLS.contains(&name.as_str()) {
            return Ok(Some(Expr::BuiltinCall {
                name,
                args,
                effects: EffectSet::PURE,
                span,
            }));
        }
        // Known function (top-level or nested-lifted) → DirectCall, but
        // only if it is *not* shadowed by an enclosing value of the same
        // name (a local/param/capture closure handle wins).
        if self.function_names.contains(&name) && !ctx.is_enclosing_value(&name) {
            return Ok(Some(Expr::DirectCall {
                fn_name: name,
                args,
                effects: EffectSet::PURE,
                span,
            }));
        }
        // Otherwise the name must be a value (closure handle) — resolve
        // it and emit an IndirectCall.
        let target = self.resolve_var_in(ctx, &name, span.clone())?;
        self.observed.add(Feature::Closures);
        Ok(Some(Expr::IndirectCall {
            target: Box::new(target),
            args,
            effects: EffectSet::PURE,
            span,
        }))
    }

    fn try_logical(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
        keyword: &str,
    ) -> Result<Option<Expr>, PythonLowerError> {
        let operands = child_nodes(node);
        let has_kw = node.children.iter().any(|c| {
            matches!(c, ASTNodeOrToken::Token(t)
                if t.type_ == lexer::token::TokenType::Keyword && t.value == keyword)
        });
        if !has_kw {
            return Ok(None);
        }
        if operands.len() < 2 {
            return Err(self.err_at(node, format!("malformed `{keyword}` expression")));
        }

        let mut acc = self.lower_expr_d(operands[0], ctx, depth + 1)?;
        for operand in &operands[1..] {
            let rhs = self.lower_expr_d(operand, ctx, depth + 1)?;
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

    fn try_not(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
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
        let operand_node = child_nodes(node).into_iter().next().ok_or_else(|| {
            self.err_at(node, "malformed `not` expression (missing operand)".to_string())
        })?;
        let operand = self.lower_expr_d(operand_node, ctx, depth + 1)?;
        let span = self.span_of(node);
        Ok(Some(Expr::BuiltinCall {
            name: "not".to_string(),
            args: vec![operand],
            effects: EffectSet::PURE,
            span,
        }))
    }

    fn try_comparison(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, PythonLowerError> {
        let has_comp_op = node
            .children
            .iter()
            .any(|c| matches!(c, ASTNodeOrToken::Node(n) if n.rule_name == "comp_op"));
        if !has_comp_op {
            return Ok(None);
        }

        let mut acc: Option<Expr> = None;
        let mut pending_op: Option<String> = None;
        for child in &node.children {
            match child {
                ASTNodeOrToken::Node(n) if n.rule_name == "comp_op" => {
                    pending_op = Some(self.comp_op_name(n)?);
                }
                ASTNodeOrToken::Node(n) => {
                    let operand = self.lower_expr_d(n, ctx, depth + 1)?;
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

    fn comp_op_name(&self, comp_op: &GrammarASTNode) -> Result<String, PythonLowerError> {
        let tok = comp_op.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Token(t) => Some(t),
            ASTNodeOrToken::Node(_) => None,
        });
        let tok = tok
            .ok_or_else(|| self.err_at(comp_op, "comparison operator missing token".to_string()))?;
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

    fn try_binary_arith(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, PythonLowerError> {
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
                    let operand = self.lower_expr_d(n, ctx, depth + 1)?;
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
                            return Err(
                                self.err_at(node, "malformed arithmetic expression".to_string())
                            )
                        }
                    });
                }
                ASTNodeOrToken::Token(_) => {}
            }
        }
        match acc {
            Some(e) => Ok(Some(e)),
            None => Err(self.err_at(node, "empty arithmetic expression".to_string())),
        }
    }

    fn try_unary_factor(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, PythonLowerError> {
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

        let operand = self.lower_expr_d(inner, ctx, depth + 1)?;
        match op {
            "-" => match operand {
                Expr::IntLit { value, span } => Ok(Some(Expr::IntLit {
                    value: value.wrapping_neg(),
                    span,
                })),
                Expr::FloatLit { value, span } => Ok(Some(Expr::FloatLit {
                    value: -value,
                    span,
                })),
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
            "+" => Ok(Some(operand)),
            other => Err(self.err_at(
                node,
                format!("unsupported unary operator `{other}` (deferred)"),
            )),
        }
    }

    /// Turn a leaf token into the matching SIR expression — a literal, or
    /// a variable reference for a bare `Name`.
    fn lower_leaf_token(
        &mut self,
        node: &GrammarASTNode,
        tok: &lexer::token::Token,
        ctx: &mut FunctionCtx,
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
                Ok(Expr::StrLit {
                    value: text.to_string(),
                    span,
                })
            }
            (None, lexer::token::TokenType::Name, name) => self.resolve_var(node, name, ctx, span),
            _ => Err(self.err_at(
                node,
                format!(
                    "unsupported token `{}` (deferred to a later milestone)",
                    tok.value
                ),
            )),
        }
    }

    /// Resolve a bare name reference (the node-anchored variant used by
    /// leaf-token lowering, which can report a precise error position).
    ///
    /// A bare reference to a **known function name** is a *closure value*:
    /// referencing a function by name (without calling it) constructs a
    /// closure handle.  A top-level function with no captures lowers to a
    /// zero-capture `MakeClosure`; a nested function likewise (its
    /// captures were computed where it was *defined*, threaded there — a
    /// bare reference to a nested function name re-constructs the closure
    /// from the *currently visible* values of those captures).
    fn resolve_var(
        &mut self,
        node: &GrammarASTNode,
        name: &str,
        ctx: &mut FunctionCtx,
        span: Span,
    ) -> Result<Expr, PythonLowerError> {
        // Local / param / capture (a value) wins over a same-named fn.
        if ctx.is_enclosing_value(name) {
            return self.resolve_var_in(ctx, name, span);
        }
        // A bare reference to a function name → a closure value.  If the
        // function carries captures (a nested closure), re-thread them
        // from the currently visible enclosing values; otherwise it is a
        // zero-capture closure handle.
        if self.function_names.contains(name) {
            self.observed.add(Feature::Closures);
            let cap_names = self.fn_captures.get(name).cloned().unwrap_or_default();
            let mut capture_values = Vec::with_capacity(cap_names.len());
            for cn in &cap_names {
                capture_values.push(self.resolve_var_in(ctx, cn, span.clone())?);
            }
            return Ok(self.make_closure(name.to_string(), cap_names, capture_values, span));
        }
        Err(self.err_at(node, format!("unresolved name `{name}`")))
    }

    /// Resolve a name that is *expected* to be in scope as a value
    /// (local/param/capture/global/builtin), producing the appropriately
    /// scoped [`Expr::VarRef`].  Errors if unresolved.
    fn resolve_var_in(
        &self,
        ctx: &FunctionCtx,
        name: &str,
        span: Span,
    ) -> Result<Expr, PythonLowerError> {
        if ctx.locals.iter().rev().any(|n| n == name) {
            return Ok(Expr::VarRef {
                name: name.to_string(),
                scope: Scope::Local,
                span,
            });
        }
        if ctx.params.contains(name) {
            return Ok(Expr::VarRef {
                name: name.to_string(),
                scope: Scope::Param,
                span,
            });
        }
        if ctx.captures.contains(name) {
            return Ok(Expr::VarRef {
                name: name.to_string(),
                scope: Scope::Capture,
                span,
            });
        }
        Err(PythonLowerError {
            message: format!("unresolved name `{name}`"),
            line: span.start_line,
            column: span.start_col,
        })
    }

    // -------------------------------------------------------------------
    // mutual recursion detection
    // -------------------------------------------------------------------

    /// Is there a cycle of length ≥ 2 in the top-level call graph (two
    /// functions that transitively call each other)?  A self-recursive
    /// function (a 1-cycle) is *not* mutual recursion.
    fn has_mutual_recursion(&self) -> bool {
        use std::collections::HashMap;
        let graph: HashMap<&str, &HashSet<String>> = self
            .call_graph
            .iter()
            .map(|(n, callees)| (n.as_str(), callees))
            .collect();
        // For each function f, see if any callee g (g != f) can reach
        // back to f — that is a mutual-recursion cycle.
        for (f, callees) in &self.call_graph {
            for g in callees.iter() {
                if g == f {
                    continue; // self-recursion is not mutual
                }
                if reaches(&graph, g, f) {
                    return true;
                }
            }
        }
        false
    }

    // -------------------------------------------------------------------
    // helpers
    // -------------------------------------------------------------------

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
                format!(
                    "unsupported: {} (deferred to a later milestone)",
                    child.rule_name
                ),
            )),
            _ => Err(self.err_at(
                node,
                format!("unsupported: {} with multiple parts (deferred)", expected),
            )),
        }
    }

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

    fn span_of(&self, node: &GrammarASTNode) -> Span {
        Span::point(
            FILE,
            node.start_line.unwrap_or(1),
            node.start_column.unwrap_or(1),
        )
    }

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

/// Walk a lowered `Block`, collecting the names of every `DirectCall`
/// target into `out` — the call edges out of one function, for
/// mutual-recursion detection.
fn collect_direct_callees(block: &Block, out: &mut HashSet<String>) {
    for s in &block.stmts {
        collect_callees_stmt(s, out);
    }
    collect_callees_expr(&block.value, out);
}

fn collect_callees_stmt(stmt: &Stmt, out: &mut HashSet<String>) {
    match stmt {
        Stmt::LetBinding { value, .. }
        | Stmt::LetStarBinding { value, .. }
        | Stmt::ExprStmt { expr: value, .. }
        | Stmt::Assign { value, .. } => collect_callees_expr(value, out),
        Stmt::While { cond, body, .. } => {
            collect_callees_expr(cond, out);
            collect_direct_callees(body, out);
        }
        Stmt::ForRange {
            start, stop, step, body, ..
        } => {
            collect_callees_expr(start, out);
            collect_callees_expr(stop, out);
            collect_callees_expr(step, out);
            collect_direct_callees(body, out);
        }
        Stmt::ForEach { iter, body, .. } => {
            collect_callees_expr(iter, out);
            collect_direct_callees(body, out);
        }
        Stmt::SeqSet { seq, index, value, .. } => {
            collect_callees_expr(seq, out);
            collect_callees_expr(index, out);
            collect_callees_expr(value, out);
        }
        Stmt::MapSet { map, key, value, .. } => {
            collect_callees_expr(map, out);
            collect_callees_expr(key, out);
            collect_callees_expr(value, out);
        }
        Stmt::ClassDef { body, .. }
        | Stmt::ModuleDef { body, .. }
        | Stmt::SingletonClassDef { body, .. } => {
            for s in body {
                collect_callees_stmt(s, out);
            }
        }
        Stmt::TryCatch {
            body,
            rescues,
            ensure_body,
            ..
        } => {
            for s in body {
                collect_callees_stmt(s, out);
            }
            for r in rescues {
                for s in &r.body {
                    collect_callees_stmt(s, out);
                }
            }
            if let Some(eb) = ensure_body {
                for s in eb {
                    collect_callees_stmt(s, out);
                }
            }
        }
    }
}

fn collect_callees_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        Expr::DirectCall { fn_name, args, .. } => {
            out.insert(fn_name.clone());
            for a in args {
                collect_callees_expr(a, out);
            }
        }
        Expr::IndirectCall { target, args, .. } => {
            collect_callees_expr(target, out);
            for a in args {
                collect_callees_expr(a, out);
            }
        }
        Expr::BuiltinCall { args, .. } => {
            for a in args {
                collect_callees_expr(a, out);
            }
        }
        Expr::MakeClosure { captures, .. } => {
            for c in captures {
                collect_callees_expr(&c.value, out);
            }
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            collect_callees_expr(cond, out);
            collect_direct_callees(then_branch, out);
            collect_direct_callees(else_branch, out);
        }
        Expr::Block(b) => collect_direct_callees(b, out),
        Expr::LogicalAnd { lhs, rhs, .. } | Expr::LogicalOr { lhs, rhs, .. } => {
            collect_callees_expr(lhs, out);
            collect_callees_expr(rhs, out);
        }
        Expr::SeqLit { items, .. } => {
            for i in items {
                collect_callees_expr(i, out);
            }
        }
        Expr::SeqIndex { seq, index, .. } => {
            collect_callees_expr(seq, out);
            collect_callees_expr(index, out);
        }
        Expr::SeqLen { seq, .. } => collect_callees_expr(seq, out),
        Expr::MapLit { entries, .. } => {
            for e in entries {
                collect_callees_expr(&e.key, out);
                collect_callees_expr(&e.value, out);
            }
        }
        Expr::MapGet { map, key, .. } => {
            collect_callees_expr(map, out);
            collect_callees_expr(key, out);
        }
        Expr::StrConcat { parts, .. } => {
            for p in parts {
                collect_callees_expr(p, out);
            }
        }
        Expr::Intrinsic { args, .. } => {
            for a in args {
                collect_callees_expr(a, out);
            }
        }
        // Atoms and references bind nothing.
        Expr::IntLit { .. }
        | Expr::BoolLit { .. }
        | Expr::NilLit { .. }
        | Expr::SymLit { .. }
        | Expr::StrLit { .. }
        | Expr::FloatLit { .. }
        | Expr::VarRef { .. } => {}
    }
}

/// Can `from` transitively reach `target` in the top-level call graph?
fn reaches(
    graph: &std::collections::HashMap<&str, &HashSet<String>>,
    from: &str,
    target: &str,
) -> bool {
    let mut stack = vec![from.to_string()];
    let mut visited = HashSet::new();
    while let Some(cur) = stack.pop() {
        if cur == target {
            return true;
        }
        if !visited.insert(cur.clone()) {
            continue;
        }
        if let Some(callees) = graph.get(cur.as_str()) {
            for c in callees.iter() {
                stack.push(c.clone());
            }
        }
    }
    false
}
