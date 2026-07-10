//! The lowering pass from `python_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **milestone M5**.
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
//! # What M5 adds — collections (lists & dicts)
//!
//! M5 is the last big Python-frontend milestone — it turns the
//! single-scalar IR into one that lowers real data programs:
//!
//! - **list display `[a, b, c]`** → [`Expr::SeqLit`] (the parser names
//!   this `atom → list_expr [ "[", list_body?, "]" ]`; `list_body` is a
//!   comma-separated run of `expression`s).  `[]` → an empty `SeqLit`.
//! - **dict display `{k: v, ...}`** → [`Expr::MapLit`] over
//!   [`semantic_ir::MapEntry`]s (parser: `atom → dict_or_set_expr [ "{",
//!   dict_or_set_body?, "}" ]`, where `dict_or_set_body → dict_body` is a
//!   comma-separated run of `dict_entry [ key, ":", value ]`).  `{}` → an
//!   empty `MapLit`.  A **set** display (`{1, 2}`) parses to a
//!   `dict_or_set_body` with *no* `dict_body` child — rejected (deferred).
//! - **subscription `x[i]`** → either [`Expr::SeqIndex`] or
//!   [`Expr::MapGet`], disambiguated by the index (see below).  The parser
//!   names a subscript a *trailing `suffix`* on a `primary`:
//!   `primary → atom suffix*`, where a subscript suffix is
//!   `[ "[", subscript, "]" ]` (a call suffix is `[ "(", arguments?, ")" ]`).
//!   Chained subscripts (`xs[i][j]`) are multiple suffixes, applied
//!   left-to-right.
//! - **`len(xs)`** → [`Expr::SeqLen`] (the SIR17 spec prefers the
//!   dedicated `SeqLen` node over `BuiltinCall("len")` so backends can
//!   emit native length access).  `len` with arity ≠ 1 is rejected.
//! - **subscript assignment `x[i] = v`** → [`Stmt::SeqSet`] /
//!   [`Stmt::MapSet`], mirroring the read-side disambiguation.
//!
//! ## Subscript disambiguation (list index vs dict key)
//!
//! Python uses one `[]` syntax for both list indexing and dict lookup; the
//! SIR17 spec lists `xs[i] → SeqIndex` and `d[k] → MapGet` but leaves the
//! *syntactic* rule that tells them apart open (the frontend has no type
//! information).  Mirroring the JS sibling's cut-line, M5 uses a purely
//! syntactic heuristic: **a string-literal index → `MapGet` / `MapSet`
//! (a map key); any other index → `SeqIndex` / `SeqSet` (a list index).**
//! This makes the canonical idioms (`xs[0]`, `d["name"]`) lower correctly;
//! a dict keyed by a computed/integer key (`d[k]`, `counts[n]`) lowers as
//! a sequence index, which the SIR runtime's duck-typed `[]` still
//! executes correctly (both route through `__getitem__`/`__setitem__`).
//! The choice only affects the manifest feature (`Sequences` vs `Maps`),
//! not runtime behaviour.
//!
//! ## Method calls → `__method__` dispatch (C2)
//!
//! A **method call** `recv.method(args…)` lowers to the shared SIR
//! method-dispatch envelope
//! `BuiltinCall("__method__", [recv, StrLit("method"), ...args])` — the
//! receiver at `args[0]`, the method name a `StrLit` at `args[1]`, call
//! args trailing.  This is the same encoding the Ruby frontend emits, and
//! the Python/TS backends already decode it and route it through
//! `sir-runtime-oop` (50+ collection methods: `append`/`push`,
//! `map`/`collect`, `select`/`filter`, `keys`, `values`, `upcase`, …), so
//! **no core IR or backend change is needed** — see
//! [`Lowerer::lower_method_call`].  A callable argument (`xs.map(f)`,
//! `lst.sort(key=lambda x: -x)`) is just another argument and lowers
//! through the ordinary call-arg path (a lambda → `MakeClosure`).
//!
//! ## Still deferred (later milestones / runtime-library work)
//!
//! - list/dict **comprehensions** (`[x for x in xs]`)            → deferred
//! - **slicing** (`xs[a:b]`, `xs[::2]`)                          → deferred
//! - **tuple** / **set** literals (`(1, 2)`, `{1, 2}`)           → deferred
//! - **attribute access as a value** (`obj.x` *not* followed by a call) —
//!   an attribute *read* has no v0 lowering (C2 covers method **calls**
//!   only)                                                        → deferred
//! - **unpacking** (`a, b = xs`, `*rest`)                        → deferred
//!
//! Each deferred form yields a positioned [`PythonLowerError`].
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
//! - collection *comprehensions* / *slicing*                  → deferred
//!   (list & dict literals / index / `len` / subscript-assign land in M5;
//!   method **calls** land in C2 — attribute-as-value stays deferred)
//! - `*args` / `**kwargs` rest parameters                      → deferred
//!   (positional/keyword **default** params land in P8; keyword-only
//!   params & keyword args land in KW8)
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
    Block, Capture, CaptureValue, EffectSet, Expr, Feature, FeatureManifest, Function, IndexArg,
    MapEntry, Metadata, Module, Param, ParamKind, Scope, Span, Stmt,
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

/// One extracted `def`-parameter spec: its name, its [`ParamKind`]
/// (`Required` for a positional param, `Keyword` for a keyword-only one
/// after the `*` boundary), and the still-unlowered CST node of its
/// default value (`Some` for an optional param, `None` for a required
/// one).  The default is borrowed from the `def` CST (lifetime `'a`) and
/// lowered by the caller in the enclosing scope.
type ParamSpec<'a> = (String, ParamKind, Option<&'a GrammarASTNode>);

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

/// What a trailing `primary` `suffix` denotes.  A `suffix` is one of a
/// *call* (`( … )`), a *subscript* (`[ … ]`), or an *attribute access*
/// (`.x`) — classified by [`Lowerer::suffix_kind`].
///
/// ## Attribute access and method calls (C2)
///
/// The grammar spells `recv.method(args)` as **two** suffixes on the
/// `primary`: an `Attr` suffix (`.method`) immediately followed by a
/// `Call` suffix (`(args)`).  The suffix fold in
/// [`Lowerer::try_primary_suffixes`] therefore special-cases an `Attr`
/// suffix by *looking ahead*: an `Attr` followed by a `Call` is a
/// **method call** and lowers to the shared SIR method-dispatch envelope
/// `BuiltinCall("__method__", [receiver, StrLit(method), ...args])` (see
/// [`Lowerer::lower_method_call`]).  A bare `Attr` with no trailing `Call`
/// (`obj.x` used as a *value*) remains deferred — attribute-as-value has
/// no v0 lowering.
enum SuffixKind {
    Call,
    Subscript,
    /// Attribute access `.name` — the method name of a following `Call`
    /// suffix, or a deferred bare attribute read.  Carries the lexeme so
    /// the fold can pack it as the dispatch method-name `StrLit`.
    Attr(String),
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
                // M5: a *subscript* target (`xs[i] = v` / `d[k] = v`)?
                // Everything else (attribute, tuple-unpack, …) is deferred.
                if let Some(stmt) = self.try_subscript_assign(assign, target_node, rhs_node, ctx)? {
                    return Ok(Lowered::Stmt(Box::new(stmt)));
                }
                return Err(self.err_at(
                    target_node,
                    "unsupported: assignment target is not a bare name (deferred)".to_string(),
                ));
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

    /// M5: lower a **subscript assignment** `base[index] = rhs` into
    /// [`Stmt::SeqSet`] (list) or [`Stmt::MapSet`] (map), mirroring the
    /// read-side disambiguation (string-literal index → map).  Returns
    /// `Ok(None)` when the LHS is *not* a subscript target (so the caller
    /// reports the generic "not a bare name" deferral).
    ///
    /// The `base` is everything left of the final subscript: for
    /// `xs[i] = v` it is `xs`; for a chained `m[a][b] = v` it is `m[a]`
    /// (itself lowered as a `SeqIndex`/`MapGet` value).  The base is
    /// lowered as an ordinary expression (it must resolve to a value in
    /// scope), the final index/key and the RHS likewise.
    fn try_subscript_assign(
        &mut self,
        assign: &GrammarASTNode,
        target_node: &GrammarASTNode,
        rhs_node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
    ) -> Result<Option<Stmt>, PythonLowerError> {
        // Peel the target expression to its `primary` (the rule that
        // carries `atom suffix*`); a non-subscript target peels to a bare
        // atom (no suffix) and is not ours.
        let primary = match self.peel_to_primary(target_node) {
            Some(p) => p,
            None => return Ok(None),
        };
        let kids = child_nodes(primary);
        let (atom, suffixes) = match kids.split_first() {
            Some((atom, rest)) if !rest.is_empty() && rest[0].rule_name == "suffix" => {
                (*atom, rest)
            }
            _ => return Ok(None),
        };
        // The *final* suffix must be the subscript being assigned.  Earlier
        // suffixes (if any) form the base value `m[a]` / `g()`.
        let (last, leading) = suffixes.split_last().expect("split_first left ≥ 1 suffix");
        if !matches!(self.suffix_kind(last)?, SuffixKind::Subscript) {
            // `f(x) = v` is not a valid assignment target.
            return Err(self.err_at(
                primary,
                "unsupported: assignment to a call result (deferred)".to_string(),
            ));
        }

        // Build the base value: the atom, with every leading suffix
        // applied (calls / subscripts).  No leading suffix → the base is
        // just the atom (`xs`).
        let span = self.span_of(assign);
        let mut base = self.lower_expr(atom, ctx)?;
        for suffix in leading {
            base = self.apply_value_suffix(base, suffix, ctx, 0, &span)?;
        }

        let index_node = self.subscript_index(last)?;
        let index = self.lower_expr(index_node, ctx)?;
        let value = self.lower_expr(rhs_node, ctx)?;

        if is_str_lit(&index) {
            self.observed.add(Feature::Maps);
            Ok(Some(Stmt::MapSet {
                map: base,
                key: index,
                value,
                span,
            }))
        } else {
            self.observed.add(Feature::Sequences);
            Ok(Some(Stmt::SeqSet {
                seq: base,
                index,
                value,
                span,
            }))
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
            "while_stmt" => Ok(Lowered::Stmt(Box::new(
                self.lower_while(inner, ctx, depth)?,
            ))),
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
        // Resolve `(name, optional default-CST)` for each parameter, then
        // lower every default **in the enclosing scope** (Python evaluates
        // defaults at `def` time in the scope the `def` is written inside).
        // The bounded `lower_expr_in` reuses the `MAX_EXPR_DEPTH`-capped
        // expression walk, so a pathologically deep default fails cleanly.
        let specs = self.def_param_specs(def)?;
        let mut params: Vec<(String, ParamKind, Option<Expr>)> = Vec::with_capacity(specs.len());
        let mut any_default = false;
        let mut any_keyword = false;
        for (pname, kind, default_node) in specs {
            if kind == ParamKind::Keyword {
                any_keyword = true;
            }
            let default = match default_node {
                Some(node) => {
                    any_default = true;
                    Some(self.lower_expr_in(node, enclosing, depth + 1)?)
                }
                None => None,
            };
            params.push((pname, kind, default));
        }
        if any_default {
            // A `Param.default = Some(_)` is exactly what the validator
            // observes as `DefaultParams`; declare it here so the manifest
            // matches.
            self.observed.add(Feature::DefaultParams);
        }
        if any_keyword {
            // A `Param.kind == Keyword` is exactly what the validator
            // observes as `KeywordParams`; declare it so the manifest
            // matches (mirrors the `DefaultParams` line above).
            self.observed.add(Feature::KeywordParams);
        }
        let suite = self
            .first_child_named(def, "suite")
            .ok_or_else(|| self.err_at(def, "malformed def: missing body".to_string()))?;
        let span = self.span_of(def);

        self.lower_callable(&name, &params, suite, enclosing, depth, span)
    }

    /// Extract a `def_stmt`'s parameters as `(name, kind, optional
    /// default-value CST node)` triples.
    ///
    /// A *plain* positional parameter (`a`) yields
    /// `(name, ParamKind::Required, None)`.  A *defaulted* one (`b = 10`)
    /// yields `(name, ParamKind::Required, Some(<expression node>))` — the
    /// `param_with_default` node carries an extra `=` token and an
    /// `expression` child, which the caller lowers (in the **enclosing**
    /// scope) into the IR's `Param.default`.
    ///
    /// ## The keyword-only `*` boundary (KW8)
    ///
    /// Python's grammar splits a parameter list at a bare `*` (or `*args`):
    /// every parameter that follows is **keyword-only** — it can only be
    /// supplied by name at the call site.  The parser models this split
    /// structurally: positional params are `param_with_default` children of
    /// `parameter_list` **directly**, while the keyword-only params live as
    /// `param_with_default` children of a nested **`star_params`** node
    /// (`* [NAME] (, param_with_default)*  [, double_star_param]`).  So the
    /// `*`-boundary is not a token we hunt for — it is *the tree shape*:
    ///
    /// ```text
    ///   def f(a, *, x, y=1):
    ///   parameter_list
    ///     param_with_default(a)          ← positional  → Required
    ///     star_params
    ///       *                            ← the boundary marker
    ///       param_with_default(x)        ← keyword-only → Keyword, default None
    ///       param_with_default(y=1)      ← keyword-only → Keyword, default Some(1)
    /// ```
    ///
    /// We therefore emit `ParamKind::Required` for every `param_with_default`
    /// that is a *direct* child of `parameter_list`, and `ParamKind::Keyword`
    /// for every `param_with_default` nested inside a `star_params`.  A
    /// keyword param with no default (`x`) is a **required** keyword; one
    /// with a default (`y=1`) is an **optional** keyword — the required-ness
    /// rides entirely on the `default` field, exactly as it does for
    /// positional optionals (there is no separate "is-required" flag).
    ///
    /// The `*args` positional-rest name and the `**kwargs` keyword-rest that
    /// may bracket the keyword-only region are outside the KW8 subset (the
    /// crate does not yet model `Rest`/`KwRest` params) and are rejected
    /// with a positioned error rather than silently dropped.
    ///
    /// ## Python def-time semantics vs. the IR's call-time model
    ///
    /// Python evaluates a default **once, at `def` time, in the enclosing
    /// scope** — so a default cannot reference another parameter
    /// (`def f(a, b=a)` is a `NameError`).  The IR's `Param.default` is a
    /// *call-time*, param-scope model (a superset).  For the constant /
    /// enclosing-reference defaults Python actually permits, the two
    /// coincide, so lowering the Python default straight into
    /// `Param.default` is faithful.  The one observable divergence is a
    /// *mutable* default (`def f(x=[])`): Python shares one list across
    /// calls; under the IR it is re-evaluated per call.  That is a
    /// deliberate, documented v0 choice (see the crate README).
    fn def_param_specs<'a>(
        &self,
        def: &'a GrammarASTNode,
    ) -> Result<Vec<ParamSpec<'a>>, PythonLowerError> {
        let parameters = match self.first_child_named(def, "parameters") {
            Some(p) => p,
            None => return Ok(vec![]), // `def f():` — no params.
        };
        // `parameters → parameter_list → param_with_default+ [star_params]`.
        let list = self
            .first_child_named(parameters, "parameter_list")
            .unwrap_or(parameters);
        let mut specs = Vec::new();
        for child in child_nodes(list) {
            match child.rule_name.as_str() {
                // Positional param (before any `*`): Required.
                "param_with_default" => {
                    let (name, default) = self.param_spec(child)?;
                    specs.push((name, ParamKind::Required, default));
                }
                // Everything after a bare `*` / `*args` is keyword-only.
                // The `star_params` node holds the boundary `*`, an
                // optional `*args` NAME, the keyword-only params, and an
                // optional `**kwargs`.
                "star_params" => self.collect_keyword_only_params(child, &mut specs)?,
                // `slash_params` (positional-only `/`) is not in the KW8
                // subset — reject rather than mis-lower its params.
                "slash_params" => {
                    return Err(self.err_at(
                        child,
                        "unsupported: positional-only `/` parameters (deferred)".to_string(),
                    ))
                }
                // A top-level `**kwargs` (no preceding `*`) — KwRest is not
                // modelled by this crate yet.
                "double_star_param" => {
                    return Err(self.err_at(
                        child,
                        "unsupported: **kwargs keyword-rest parameter (deferred)".to_string(),
                    ))
                }
                _ => {}
            }
        }
        Ok(specs)
    }

    /// Harvest the keyword-only parameters out of a `star_params` node,
    /// appending `(name, ParamKind::Keyword, default)` for each.
    ///
    /// A `star_params` looks like `* [NAME] (, param_with_default)* [, **kw]`.
    /// The leading `*` is the keyword-only boundary; an optional bare NAME
    /// immediately after it is the `*args` positional-rest, which this crate
    /// does not yet model — its presence is rejected.  Any nested
    /// `double_star_param` (`**kwargs`) is likewise rejected.  Every
    /// `param_with_default` **inside** this node is keyword-only, so it maps
    /// to `ParamKind::Keyword` (required if it has no default, optional if
    /// it does).
    fn collect_keyword_only_params<'a>(
        &self,
        star: &'a GrammarASTNode,
        specs: &mut Vec<ParamSpec<'a>>,
    ) -> Result<(), PythonLowerError> {
        for child in &star.children {
            match child {
                // A bare NAME token directly under `star_params` is the
                // `*args` rest name (`def f(*args, x): …`).  Rest params are
                // outside the KW8 subset.
                ASTNodeOrToken::Token(t)
                    if matches!(t.type_, lexer::token::TokenType::Name)
                        && t.type_name.is_none() =>
                {
                    return Err(self.err_at(
                        star,
                        "unsupported: *args positional-rest parameter (deferred)".to_string(),
                    ))
                }
                ASTNodeOrToken::Token(_) => {} // `*` / `,` separators.
                ASTNodeOrToken::Node(n) => match n.rule_name.as_str() {
                    "param_with_default" => {
                        let (name, default) = self.param_spec(n)?;
                        specs.push((name, ParamKind::Keyword, default));
                    }
                    "double_star_param" => {
                        return Err(self.err_at(
                            n,
                            "unsupported: **kwargs keyword-rest parameter (deferred)".to_string(),
                        ))
                    }
                    _ => {}
                },
            }
        }
        Ok(())
    }

    /// Extract one parameter's `(name, optional default node)` from a
    /// `param_with_default`.
    ///
    /// The CST shapes are:
    ///   - plain `a`     → `[NAME]`                       → `(name, None)`
    ///   - default `b=1` → `[NAME, EQUALS, expression]`   → `(name, Some(expr))`
    ///
    /// This is `ParamKind`-agnostic: the caller stamps `Required` (a direct
    /// `parameter_list` child) or `Keyword` (nested in `star_params`) onto
    /// the result.  A type-annotation `COLON expression` (`def f(a: int)`)
    /// is not in the subset, but were one present its `expression` child
    /// would be mistaken for a default — so we bind `default` only to the
    /// `expression` that follows an `EQUALS` token.
    fn param_spec<'a>(
        &self,
        pwd: &'a GrammarASTNode,
    ) -> Result<(String, Option<&'a GrammarASTNode>), PythonLowerError> {
        let mut name: Option<String> = None;
        let mut default: Option<&GrammarASTNode> = None;
        // Only the `expression` that follows an `=` is a default.  We track
        // whether the last token seen was `=` so a (subset-external) type
        // annotation `NAME : expression` could never be mistaken for one.
        let mut seen_equals = false;
        for child in &pwd.children {
            match child {
                ASTNodeOrToken::Token(t) => {
                    if matches!(t.type_, lexer::token::TokenType::Name)
                        && t.type_name.is_none()
                        && name.is_none()
                    {
                        name = Some(t.value.clone());
                    } else if matches!(t.type_, lexer::token::TokenType::Equals) {
                        seen_equals = true;
                    }
                }
                // The default-value `expression` node — present only once an
                // `=` has been seen (`name = expr`).
                ASTNodeOrToken::Node(n) => {
                    if seen_equals {
                        default = Some(n);
                    }
                }
            }
        }
        match name {
            Some(name) => Ok((name, default)),
            None => Err(self.err_at(pwd, "malformed parameter".to_string())),
        }
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

        let (captures, capture_values) = self.resolve_captures(&free, enclosing, &span)?;

        // Lower the body in the synthesised function's own context.
        let mut inner = FunctionCtx::new(bound.clone(), captures.iter().cloned().collect());
        let value = self.lower_expr_in(body_expr, &mut inner, depth + 1)?;
        let body = Block {
            stmts: vec![],
            value,
            span: span.clone(),
        };
        // Lambdas in the M5 subset never carry defaults or keyword-only
        // markers (both rejected in `lambda_params`), so every param maps to
        // `(name, Required, None)`.
        let lambda_params: Vec<(String, ParamKind, Option<Expr>)> = params
            .iter()
            .map(|n| (n.clone(), ParamKind::Required, None))
            .collect();
        self.push_function(&fn_name, &lambda_params, &captures, body, span.clone());

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
        params: &[(String, ParamKind, Option<Expr>)],
        suite: &GrammarASTNode,
        enclosing: &mut FunctionCtx,
        depth: usize,
        span: Span,
    ) -> Result<Expr, PythonLowerError> {
        // ── Free-variable analysis over the suite. ──
        // Names bound *within* the body — the params plus every name the
        // body assigns / `for`-binds — are body-local, not captures.  Note
        // a default expression sees the *enclosing* scope (it was already
        // lowered there by the caller), so it never contributes to the
        // body's free/bound sets here.
        let mut bound: HashSet<String> = params.iter().map(|(n, _, _)| n.clone()).collect();
        self.collect_suite_bound_names(suite, &mut bound)?;
        let mut free = Vec::new();
        let mut seen = HashSet::new();
        self.collect_free_names(suite, &bound, &mut free, &mut seen, 0)?;

        let (captures, capture_values) = self.resolve_captures(&free, enclosing, &span)?;

        // ── Lower the body in the function's own context. ──
        let mut inner = FunctionCtx::new(
            params.iter().map(|(n, _, _)| n.clone()).collect(),
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
        params: &[(String, ParamKind, Option<Expr>)],
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
            self.fn_captures.insert(name.to_string(), captures.to_vec());
        }
        let f = Function {
            name: name.to_string(),
            params: params
                .iter()
                .map(|(pname, kind, default)| Param {
                    name: pname.clone(),
                    sir_type: None,
                    // The `kind` is decided by the parameter's position
                    // relative to the keyword-only `*` boundary: `Required`
                    // for a positional param (a direct `parameter_list`
                    // child) and `Keyword` for a keyword-only one (nested in
                    // `star_params`).  Required-vs-optional does NOT ride on
                    // the kind — it rides on `default` (a positional or
                    // keyword param with `default: Some(_)` is optional).
                    kind: *kind,
                    default: default.clone().map(Box::new),
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
                        if let Ok(Some(name)) =
                            self.single_expr(list).and_then(|e| self.target_name(e))
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
            if let Ok(specs) = self.def_param_specs(node) {
                for (pname, _kind, default_node) in specs {
                    // A *default expression* is evaluated in the **enclosing**
                    // scope (Python def-time semantics), so any name it
                    // references is free against the *outer* `bound` set — not
                    // shadowed by the params.  Collect those before adding the
                    // param itself to `inner`.  The keyword-only `_kind` does
                    // not affect free-variable analysis (a keyword param binds
                    // its name in the body exactly as a positional one does).
                    if let Some(default) = default_node {
                        self.collect_free_names(default, bound, free, seen, depth + 1)?;
                    }
                    inner.insert(pname);
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
                // A `primary` with trailing `suffix`es is a call and/or
                // subscript chain (`f(x)`, `xs[i]`, `xs[i][j]`, `g()[0]`).
                if let Some(e) = self.try_primary_suffixes(node, ctx, depth)? {
                    return Ok(e);
                }
            }
            // M5: list display `[a, b, c]` → SeqLit.
            "list_expr" => return self.lower_list_expr(node, ctx, depth),
            // M5: dict display `{k: v, ...}` → MapLit (a set display is
            // rejected inside this handler).
            "dict_or_set_expr" => return self.lower_dict_or_set_expr(node, ctx, depth),
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

    /// `primary` with trailing `suffix`es → a call / subscript chain
    /// (`f(x)`, `xs[i]`, `xs[i][j]`, `g()[0]`).  Returns `Ok(None)` when
    /// the `primary` has no suffix (a bare atom) so the generic peel
    /// handles it.
    ///
    /// The suffixes are applied **left to right** as a fold over an
    /// accumulated [`Expr`].  The *first* suffix is special-cased because
    /// the base is a bare `atom` (a name): a **call** there resolves with
    /// the full name semantics (builtin / `DirectCall` / `IndirectCall`),
    /// and a **`len(...)`** call is intercepted as [`Expr::SeqLen`].  Once
    /// the accumulator is a *computed value*, a further call suffix is an
    /// [`Expr::IndirectCall`] and a subscript suffix a `SeqIndex`/`MapGet`.
    ///
    /// Each suffix application is depth-bounded (it lowers the suffix's
    /// argument/index expressions at `depth + 1`), so a pathological
    /// subscript tower `xs[xs[xs[...]]]` (deep *index* expressions) fails
    /// cleanly via [`MAX_EXPR_DEPTH`].
    fn try_primary_suffixes(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Option<Expr>, PythonLowerError> {
        let kids = child_nodes(node);
        // `primary → atom suffix*`.  No suffix → not our shape.
        let (atom, suffixes) = match kids.split_first() {
            Some((atom, rest)) if !rest.is_empty() && rest[0].rule_name == "suffix" => {
                (*atom, rest)
            }
            _ => return Ok(None),
        };

        let span = self.span_of(node);

        // Fold the suffixes left to right over an accumulated `Expr`.  Most
        // suffixes consume one node, but an **attribute** suffix (`.method`)
        // looks ahead one slot: `.method` immediately followed by a `(args)`
        // call suffix is a *method call* and consumes **both** suffixes,
        // producing a `__method__` dispatch envelope (C2).  So the loop is
        // index-based rather than a plain `for` — an attribute+call pair
        // advances the cursor by two.
        //
        // The *first* suffix is special only when it is a `Call`/`Subscript`
        // on the bare-name atom: a call there carries full name semantics
        // (builtin / `len`→`SeqLen` / `DirectCall` / `IndirectCall`) and a
        // subscript indexes the atom-as-value.  An attribute *first* suffix
        // (`recv.method(…)` where `recv` is a bare name) needs the atom as an
        // ordinary *value* receiver, which `handle_attr_suffix` lowers.
        let mut i = 0usize;
        let mut acc: Option<Expr> = None;
        while i < suffixes.len() {
            let suffix = suffixes[i];
            match self.suffix_kind(suffix)? {
                SuffixKind::Attr(method) => {
                    // Look ahead: `.method (args)` → method call; a bare
                    // `.method` (no following call) is deferred.
                    let next_is_call = match suffixes.get(i + 1) {
                        Some(s) => matches!(self.suffix_kind(s)?, SuffixKind::Call),
                        None => false,
                    };
                    if !next_is_call {
                        return Err(self.err_at(
                            suffix,
                            "unsupported: attribute access as a value \
                             (deferred to a later milestone)"
                                .to_string(),
                        ));
                    }
                    // Receiver: the accumulated value, or — for a leading
                    // attribute — the bare atom lowered as a value.
                    let receiver = match acc.take() {
                        Some(recv) => recv,
                        None => self.lower_expr_d(atom, ctx, depth + 1)?,
                    };
                    let call_suffix = suffixes[i + 1];
                    let name_span = self.span_of(suffix);
                    acc = Some(self.lower_method_call(
                        receiver,
                        method,
                        name_span,
                        call_suffix,
                        ctx,
                        depth,
                        &span,
                    )?);
                    i += 2;
                }
                SuffixKind::Call | SuffixKind::Subscript => {
                    acc = Some(match acc.take() {
                        // Chained suffix on a computed value.
                        Some(base) => self.apply_value_suffix(base, suffix, ctx, depth, &span)?,
                        // The very first suffix carries bare-name semantics.
                        None => self.apply_first_suffix(atom, suffix, ctx, depth, &span)?,
                    });
                    i += 1;
                }
            }
        }
        // `suffixes` is non-empty (checked above), so `acc` is always set.
        Ok(acc)
    }

    /// Apply the **first** trailing `suffix` to the bare-name `atom`.  A
    /// call suffix resolves the callee with full name semantics
    /// (builtin / `len`→`SeqLen` / `DirectCall` / `IndirectCall`); a
    /// subscript suffix lowers the atom as a value first, then indexes it.
    fn apply_first_suffix(
        &mut self,
        atom: &GrammarASTNode,
        suffix: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
        span: &Span,
    ) -> Result<Expr, PythonLowerError> {
        match self.suffix_kind(suffix)? {
            SuffixKind::Call => self.lower_call_suffix(atom, suffix, ctx, depth, span),
            SuffixKind::Subscript => {
                let base = self.lower_expr_d(atom, ctx, depth + 1)?;
                self.lower_subscript_suffix(base, suffix, ctx, depth, span)
            }
            // The suffix fold routes every `Attr` suffix through
            // `lower_method_call` (with look-ahead) *before* calling this
            // helper, so an attribute never reaches here.
            SuffixKind::Attr(_) => Err(self.err_at(
                suffix,
                "internal: attribute suffix reached apply_first_suffix".to_string(),
            )),
        }
    }

    /// Apply a trailing `suffix` to an already-computed value (a chained
    /// suffix: `g()[0]`, `xs[i][j]`, `f(x)(y)`).  A call here is always an
    /// [`Expr::IndirectCall`] (the value is a closure handle); a subscript
    /// is a `SeqIndex`/`MapGet`.
    fn apply_value_suffix(
        &mut self,
        base: Expr,
        suffix: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
        span: &Span,
    ) -> Result<Expr, PythonLowerError> {
        match self.suffix_kind(suffix)? {
            SuffixKind::Call => {
                let args = self.lower_call_args(suffix, ctx, depth)?;
                self.observed.add(Feature::Closures);
                Ok(Expr::IndirectCall {
                    target: Box::new(base),
                    args,
                    effects: EffectSet::PURE,
                    span: span.clone(),
                })
            }
            SuffixKind::Subscript => self.lower_subscript_suffix(base, suffix, ctx, depth, span),
            // Attribute suffixes are consumed by the fold's look-ahead
            // (`.method (args)` → method dispatch) before reaching here.
            SuffixKind::Attr(_) => Err(self.err_at(
                suffix,
                "internal: attribute suffix reached apply_value_suffix".to_string(),
            )),
        }
    }

    /// Lower a **method call** `receiver.method(args…)` (C2) to the shared
    /// SIR method-dispatch envelope.
    ///
    /// ## The `__method__` convention
    ///
    /// Receiver-dispatched calls are *not* growing the core `Expr` enum;
    /// instead every frontend packs them into a synthetic
    ///
    /// ```text
    /// BuiltinCall { name: "__method__",
    ///               args: [ receiver, StrLit("method"), arg1, arg2, … ] }
    /// ```
    ///
    /// so the **receiver is always `args[0]`**, the **method name is always a
    /// `StrLit` at `args[1]`**, and the call's own arguments follow.  This is
    /// exactly what the Ruby frontend emits (see
    /// `ruby-to-semantic-ir::fold_one_dot_call`) and exactly what the
    /// Python/TS backends already decode and route through
    /// `sir-runtime-oop`'s `call_method` (50+ collection methods:
    /// `append`/`push`, `map`/`collect`, `select`/`filter`, `keys`,
    /// `values`, `upcase`, …).  Because the shape is a plain `BuiltinCall`,
    /// **no new core IR node, backend change, or feature flag is required** —
    /// the validator accepts `BuiltinCall`, and the only feature the envelope
    /// introduces is [`Feature::Strings`] for the synthetic method-name
    /// literal (declared here, matching the Ruby frontend).  There is
    /// deliberately **no** `MethodDispatch` feature — that is a later
    /// (Phase-2) milestone; matching the existing pipeline keeps validation
    /// and the Python backend happy.
    ///
    /// ## Higher-order arguments
    ///
    /// A callable argument (`lst.sort(key=lambda x: -x)`, `xs.map(f)`) is
    /// **just another argument**: Python has no trailing-block syntax, so the
    /// lambda/closure lowers through the ordinary [`Self::lower_call_args`] →
    /// [`Self::lower_expr_d`] path (a lambda becomes an [`Expr::MakeClosure`],
    /// a bare name a closure `VarRef`) and lands in the dispatch args.  The
    /// backend/runtime detects a trailing `Closure` and applies it as the
    /// block, so `xs.map(fn)` runs `fn` per element with no special-casing
    /// here.
    ///
    /// Effects default to `PURE` — the receiver type is erased at this layer,
    /// mirroring the Ruby frontend; a later receiver-type analysis pass can
    /// widen effects if needed.
    #[allow(clippy::too_many_arguments)]
    fn lower_method_call(
        &mut self,
        receiver: Expr,
        method: String,
        name_span: Span,
        call_suffix: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
        span: &Span,
    ) -> Result<Expr, PythonLowerError> {
        // Lower the call's arguments first (a lambda arg becomes a
        // `MakeClosure` here — see the doc note on higher-order args).
        let call_args = self.lower_call_args(call_suffix, ctx, depth)?;

        // Pack `[receiver, StrLit(method), ...call_args]`.  The synthetic
        // method-name literal is the reason this envelope declares
        // `Feature::Strings`.
        self.observed.add(Feature::Strings);
        let mut args = Vec::with_capacity(call_args.len() + 2);
        args.push(receiver);
        args.push(Expr::StrLit {
            value: method,
            span: name_span,
        });
        args.extend(call_args);

        Ok(Expr::BuiltinCall {
            name: "__method__".to_string(),
            args,
            effects: EffectSet::PURE,
            span: span.clone(),
        })
    }

    /// Classify a `suffix`: a call `( … )`, a subscript `[ … ]`, or an
    /// attribute access `.name`.  The grammar's `suffix` rule is
    /// `DOT NAME | "[" subscript "]" | "(" arguments? ")"`, so the first
    /// *token* child discriminates: `(` → call, `[` → subscript, `.` →
    /// attribute (whose NAME lexeme is captured for method dispatch).
    fn suffix_kind(&self, suffix: &GrammarASTNode) -> Result<SuffixKind, PythonLowerError> {
        match suffix.children.first() {
            Some(ASTNodeOrToken::Token(t)) if t.value == "(" => Ok(SuffixKind::Call),
            Some(ASTNodeOrToken::Token(t)) if t.value == "[" => Ok(SuffixKind::Subscript),
            Some(ASTNodeOrToken::Token(t)) if t.value == "." => {
                let name = self.attr_name(suffix)?;
                Ok(SuffixKind::Attr(name))
            }
            _ => Err(self.err_at(
                suffix,
                "unsupported: unrecognised suffix (deferred to a later milestone)".to_string(),
            )),
        }
    }

    /// Extract the attribute NAME lexeme from a `DOT NAME` attribute
    /// suffix (`.method`).  The suffix's second token child is the NAME.
    fn attr_name(&self, suffix: &GrammarASTNode) -> Result<String, PythonLowerError> {
        suffix
            .children
            .iter()
            .filter_map(|c| match c {
                ASTNodeOrToken::Token(t) if matches!(t.type_, lexer::token::TokenType::Name) => {
                    Some(t.value.clone())
                }
                _ => None,
            })
            .next()
            .ok_or_else(|| self.err_at(suffix, "malformed attribute suffix".to_string()))
    }

    /// Lower a call `suffix` applied to a bare-name `callee` atom, with
    /// full name semantics: a builtin (`print`/`range`) → `BuiltinCall`;
    /// `len(x)` → the dedicated [`Expr::SeqLen`] node (SIR17 prefers it
    /// over `BuiltinCall("len")`); a known function → `DirectCall`; a
    /// local/param/capture value → `IndirectCall`.
    fn lower_call_suffix(
        &mut self,
        callee: &GrammarASTNode,
        suffix: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
        span: &Span,
    ) -> Result<Expr, PythonLowerError> {
        let args = self.lower_call_args(suffix, ctx, depth)?;

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

        // `len(x)` → SeqLen (preferred over BuiltinCall("len")).  Arity
        // must be exactly 1; a value of the same name in scope shadows the
        // builtin (then it is an ordinary indirect call).
        if name == "len" && !ctx.is_enclosing_value(&name) {
            if args.len() != 1 {
                return Err(self.err_at(
                    suffix,
                    format!("len() takes exactly 1 argument, got {}", args.len()),
                ));
            }
            self.observed.add(Feature::Sequences);
            return Ok(Expr::SeqLen {
                seq: Box::new(args.into_iter().next().expect("arity checked == 1")),
                span: span.clone(),
            });
        }

        // Other builtins (`print` / `range`).
        if BUILTIN_CALLS.contains(&name.as_str()) && !ctx.is_enclosing_value(&name) {
            return Ok(Expr::BuiltinCall {
                name,
                args,
                effects: EffectSet::PURE,
                span: span.clone(),
            });
        }
        // Known function (top-level or nested-lifted) → DirectCall, but
        // only if it is *not* shadowed by an enclosing value of the same
        // name (a local/param/capture closure handle wins).
        if self.function_names.contains(&name) && !ctx.is_enclosing_value(&name) {
            return Ok(Expr::DirectCall {
                fn_name: name,
                args,
                effects: EffectSet::PURE,
                span: span.clone(),
            });
        }
        // Otherwise the name must be a value (closure handle) — resolve
        // it and emit an IndirectCall.
        let target = self.resolve_var_in(ctx, &name, span.clone())?;
        self.observed.add(Feature::Closures);
        Ok(Expr::IndirectCall {
            target: Box::new(target),
            args,
            effects: EffectSet::PURE,
            span: span.clone(),
        })
    }

    /// Lower the argument expressions of a call `suffix` (`( a, b, … )`).
    ///
    /// ## Keyword arguments (KW8) — `f(1, y=2)`
    ///
    /// The grammar gives an `argument` node one of four shapes; the two that
    /// matter here are
    ///
    /// ```text
    ///   argument → expression                 (a POSITIONAL argument)
    ///   argument → NAME EQUALS expression      (a KEYWORD argument: `y=2`)
    /// ```
    ///
    /// A positional argument lowers to its bare `Expr` (unchanged from
    /// before).  A keyword argument lowers to an [`Expr::KeywordArg`] wrapper
    /// `{ name, value }` that carries the parameter *name* alongside the
    /// lowered value, and is appended to the same `args` vec — the core IR
    /// models keyword arguments as ordinary `args` elements that trail the
    /// positionals (the validator enforces the trailing rule), rather than a
    /// parallel `kwargs` field.  Whenever any keyword argument is produced we
    /// declare [`Feature::KeywordParams`] so the manifest matches what the
    /// validator observes.
    ///
    /// ## Keyword arg vs. `**dict` splat — a deliberate distinction
    ///
    /// Only the explicit `NAME = value` spelling becomes a `KeywordArg`.  The
    /// grammar's other two `argument` forms — `STAR expression` (`*seq`) and
    /// `DOUBLE_STAR expression` (`**dict`, keyword-rest splat) — are NOT
    /// keyword arguments: `**dict` names no single parameter, it unpacks a
    /// mapping.  Those splat forms keep their existing (subset-external)
    /// treatment via [`Self::single_arg_expr`], which returns the inner
    /// `expression`; we detect the `NAME EQUALS` shape *first* by the leading
    /// NAME + `=` tokens so a `**dict` never masquerades as a keyword arg.
    fn lower_call_args(
        &mut self,
        suffix: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Vec<Expr>, PythonLowerError> {
        let arg_nodes = self.call_arguments(suffix);
        let mut args = Vec::with_capacity(arg_nodes.len());
        for a in &arg_nodes {
            match self.keyword_arg_name(a) {
                // `NAME = expression` → a keyword argument.
                Some(kw_name) => {
                    let value_node = self.single_arg_expr(a)?;
                    let value = self.lower_expr_d(value_node, ctx, depth + 1)?;
                    let span = self.span_of(a);
                    self.observed.add(Feature::KeywordParams);
                    args.push(Expr::KeywordArg {
                        name: kw_name,
                        value: Box::new(value),
                        span,
                    });
                }
                // A bare positional argument (or a `*`/`**` splat, whose
                // token is dropped by `single_arg_expr` — unchanged v0).
                None => {
                    let e = self.single_arg_expr(a)?;
                    args.push(self.lower_expr_d(e, ctx, depth + 1)?);
                }
            }
        }
        Ok(args)
    }

    /// If this `argument` node is the keyword form `NAME EQUALS expression`,
    /// return the keyword name; otherwise `None`.
    ///
    /// The CST for `y=2` is `argument[ NAME("y"), EQUALS, expression ]`.  We
    /// require **both** a leading bare `NAME` token and an `EQUALS` token so
    /// that neither a positional `expression` (whose first descendant may be
    /// a NAME atom, but which has no `EQUALS` *token child of the argument*)
    /// nor a `**dict` splat (a `DOUBLE_STAR` token, no `EQUALS`) is ever
    /// mistaken for a keyword argument.
    fn keyword_arg_name(&self, arg: &GrammarASTNode) -> Option<String> {
        let mut name: Option<String> = None;
        let mut has_equals = false;
        for child in &arg.children {
            match child {
                ASTNodeOrToken::Token(t)
                    if matches!(t.type_, lexer::token::TokenType::Name)
                        && t.type_name.is_none()
                        && name.is_none() =>
                {
                    name = Some(t.value.clone());
                }
                ASTNodeOrToken::Token(t) if matches!(t.type_, lexer::token::TokenType::Equals) => {
                    has_equals = true;
                }
                _ => {}
            }
        }
        match (name, has_equals) {
            (Some(n), true) => Some(n),
            _ => None,
        }
    }

    /// Lower a subscript `suffix` (`[ index ]`) applied to `base`,
    /// disambiguating list-index ([`Expr::SeqIndex`]) from dict-lookup
    /// ([`Expr::MapGet`]) by the index: a **string-literal** index is a
    /// map key; anything else is a sequence index (see the module-level
    /// "Subscript disambiguation" note).  Slicing (`a:b`) is rejected.
    fn lower_subscript_suffix(
        &mut self,
        base: Expr,
        suffix: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
        span: &Span,
    ) -> Result<Expr, PythonLowerError> {
        let index_node = self.subscript_index(suffix)?;
        let index = self.lower_expr_d(index_node, ctx, depth + 1)?;
        if is_str_lit(&index) {
            self.observed.add(Feature::Maps);
            Ok(Expr::MapGet {
                map: Box::new(base),
                key: Box::new(index),
                span: span.clone(),
            })
        } else {
            self.observed.add(Feature::Sequences);
            Ok(Expr::SeqIndex {
                seq: Box::new(base),
                index: Box::new(index),
                span: span.clone(),
            })
        }
    }

    /// Extract the single index `expression` from a subscript `suffix`
    /// (`suffix → "[" subscript "]"`, `subscript → subscript_item →
    /// expression`).  A *slice* (`a:b`, with a `Colon`) or multi-item
    /// subscript is rejected (deferred).
    fn subscript_index<'a>(
        &self,
        suffix: &'a GrammarASTNode,
    ) -> Result<&'a GrammarASTNode, PythonLowerError> {
        let subscript = self
            .first_child_named(suffix, "subscript")
            .ok_or_else(|| self.err_at(suffix, "malformed subscript".to_string()))?;
        // A slice surfaces as a `Colon` token somewhere in the subscript
        // (e.g. `xs[a:b]`); reject it explicitly as deferred.
        if has_colon_token(subscript) {
            return Err(self.err_at(
                subscript,
                "unsupported: slicing (deferred to a later milestone)".to_string(),
            ));
        }
        let items: Vec<&GrammarASTNode> = child_nodes(subscript)
            .into_iter()
            .filter(|n| n.rule_name == "subscript_item")
            .collect();
        let item = match items.as_slice() {
            [only] => *only,
            _ => {
                return Err(self.err_at(
                    subscript,
                    "unsupported: multi-element subscript (deferred)".to_string(),
                ))
            }
        };
        self.first_child_named(item, "expression").ok_or_else(|| {
            self.err_at(
                item,
                "unsupported: non-expression subscript (deferred)".to_string(),
            )
        })
    }

    /// Lower a list display `list_expr → "[" list_body? "]"` into
    /// [`Expr::SeqLit`].  The elements live in `list_body` as a
    /// comma-separated run of `expression`s; an empty list (`[]`) has no
    /// `list_body` child.  A comprehension (`[x for x in xs]`) carries a
    /// `for`/`comp_for` node instead — rejected as deferred.  Each element
    /// is lowered at `depth + 1`, so a deep `[[[…]]]` tower fails cleanly.
    fn lower_list_expr(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Expr, PythonLowerError> {
        let span = self.span_of(node);
        let mut items = Vec::new();
        if let Some(body) = self.first_child_named(node, "list_body") {
            // A comprehension body carries a `comp_for` / `for`-bearing
            // node rather than a plain expression run — reject it.
            if self.is_comprehension_body(body) {
                return Err(self.err_at(
                    body,
                    "unsupported: list comprehension (deferred to a later milestone)".to_string(),
                ));
            }
            for el in child_nodes(body) {
                if el.rule_name == "expression" {
                    items.push(self.lower_expr_d(el, ctx, depth + 1)?);
                } else {
                    return Err(self.err_at(
                        el,
                        format!("unsupported list element `{}` (deferred)", el.rule_name),
                    ));
                }
            }
        }
        self.observed.add(Feature::Sequences);
        Ok(Expr::SeqLit { items, span })
    }

    /// Lower a dict display `dict_or_set_expr → "{" dict_or_set_body? "}"`
    /// into [`Expr::MapLit`].  The body wraps a `dict_body` whose children
    /// are `dict_entry [ key, ":", value ]` nodes (comma-separated).  An
    /// empty `{}` has no body.  A **set** display (`{1, 2}`) parses to a
    /// `dict_or_set_body` with no `dict_body` (a bare expression run) and is
    /// rejected; a comprehension likewise.  Each key/value is lowered at
    /// `depth + 1` so a deep `{a: {b: …}}` tower fails cleanly.
    fn lower_dict_or_set_expr(
        &mut self,
        node: &GrammarASTNode,
        ctx: &mut FunctionCtx,
        depth: usize,
    ) -> Result<Expr, PythonLowerError> {
        let span = self.span_of(node);
        let body = match self.first_child_named(node, "dict_or_set_body") {
            // Empty `{}` is an empty map.
            None => return Ok(self.empty_map(span)),
            Some(b) => b,
        };
        let dict_body = match self.first_child_named(body, "dict_body") {
            Some(db) => db,
            // No `dict_body` ⇒ a *set* display (or set comprehension) —
            // deferred (sets are not an SIR17 collection in v0).
            None => {
                return Err(self.err_at(
                    body,
                    "unsupported: set literal / comprehension (deferred to a later milestone)"
                        .to_string(),
                ))
            }
        };
        if self.is_comprehension_body(dict_body) {
            return Err(self.err_at(
                dict_body,
                "unsupported: dict comprehension (deferred to a later milestone)".to_string(),
            ));
        }
        let mut entries = Vec::new();
        for entry in child_nodes(dict_body) {
            if entry.rule_name != "dict_entry" {
                return Err(self.err_at(
                    entry,
                    format!("unsupported dict element `{}` (deferred)", entry.rule_name),
                ));
            }
            // `dict_entry → key_expression ":" value_expression`.  A `**d`
            // spread entry has no plain `[key, value]` expression pair.
            let exprs: Vec<&GrammarASTNode> = child_nodes(entry)
                .into_iter()
                .filter(|n| n.rule_name == "expression")
                .collect();
            let (k, v) = match exprs.as_slice() {
                [k, v] => (*k, *v),
                _ => {
                    return Err(self.err_at(
                        entry,
                        "unsupported: dict spread / non key-value entry (deferred)".to_string(),
                    ))
                }
            };
            let key = self.lower_expr_d(k, ctx, depth + 1)?;
            let value = self.lower_expr_d(v, ctx, depth + 1)?;
            entries.push(MapEntry { key, value });
        }
        self.observed.add(Feature::Maps);
        Ok(Expr::MapLit { entries, span })
    }

    /// An empty [`Expr::MapLit`] (the lowering of `{}`).  Declares `Maps`.
    fn empty_map(&mut self, span: Span) -> Expr {
        self.observed.add(Feature::Maps);
        Expr::MapLit {
            entries: vec![],
            span,
        }
    }

    /// Does a list/dict body carry a comprehension (`for`/`comp_for`)
    /// rather than a plain element run?  Best-effort: a comprehension's
    /// CST contains a `comp_for` node or a `for` keyword token.
    fn is_comprehension_body(&self, body: &GrammarASTNode) -> bool {
        body.children.iter().any(|c| match c {
            ASTNodeOrToken::Node(n) => {
                n.rule_name == "comp_for" || n.rule_name.contains("comprehension")
            }
            ASTNodeOrToken::Token(t) => {
                t.type_ == lexer::token::TokenType::Keyword && t.value == "for"
            }
        })
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
            self.err_at(
                node,
                "malformed `not` expression (missing operand)".to_string(),
            )
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

/// Is a lowered index expression a string literal?  This is the M5
/// subscript-disambiguation predicate: a string-literal subscript index is
/// treated as a **map key** (`MapGet`/`MapSet`); any other index is a
/// **sequence index** (`SeqIndex`/`SeqSet`).  See the module-level
/// "Subscript disambiguation" note for the rationale and its limits.
fn is_str_lit(expr: &Expr) -> bool {
    matches!(expr, Expr::StrLit { .. })
}

/// Does `node` carry a `Colon` token among its *direct* children?  Used to
/// detect a slice subscript (`xs[a:b]`), which surfaces as a `Colon` token
/// inside the `subscript` node and is rejected (deferred) in M5.
fn has_colon_token(node: &GrammarASTNode) -> bool {
    node.children
        .iter()
        .any(|c| matches!(c, ASTNodeOrToken::Token(t) if t.type_ == lexer::token::TokenType::Colon))
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
            start,
            stop,
            step,
            body,
            ..
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
        Stmt::SeqSet {
            seq, index, value, ..
        } => {
            collect_callees_expr(seq, out);
            collect_callees_expr(index, out);
            collect_callees_expr(value, out);
        }
        Stmt::MapSet {
            map, key, value, ..
        } => {
            collect_callees_expr(map, out);
            collect_callees_expr(key, out);
            collect_callees_expr(value, out);
        }
        Stmt::IndexSet {
            target,
            indices,
            value,
            ..
        } => {
            collect_callees_expr(target, out);
            for idx in indices {
                match idx {
                    IndexArg::Scalar(e) | IndexArg::Range(e) => collect_callees_expr(e, out),
                    IndexArg::Whole => {}
                }
            }
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
        // SIR26 conversion (not currently emitted by this frontend) — recurse.
        Expr::Convert { value, .. } => collect_callees_expr(value, out),
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
        // KW1 compile-compat stub: recurse into the `KeywordArg`'s inner
        // `value` (its runtime meaning) so callees nested in a keyword
        // argument are still collected.  Real support pending KW2–KW8.
        Expr::KeywordArg { value, .. } => collect_callees_expr(value, out),
        // SIR22 array/matrix nodes: the Python frontend never emits these
        // (no lowering path constructs them today), but a `DirectCall` could
        // in principle appear nested inside one (e.g. as a row element or a
        // range bound), so recurse into every child `Expr` slot, matching
        // the treatment of every other compound node above.
        Expr::ArrayLit { rows, .. } => {
            for row in rows {
                for cell in row {
                    collect_callees_expr(cell, out);
                }
            }
        }
        Expr::Range {
            start, step, stop, ..
        } => {
            collect_callees_expr(start, out);
            if let Some(step) = step {
                collect_callees_expr(step, out);
            }
            collect_callees_expr(stop, out);
        }
        Expr::MatMul { lhs, rhs, .. } => {
            collect_callees_expr(lhs, out);
            collect_callees_expr(rhs, out);
        }
        Expr::ElementwiseOp { lhs, rhs, .. } => {
            collect_callees_expr(lhs, out);
            collect_callees_expr(rhs, out);
        }
        Expr::Transpose { target, .. } => collect_callees_expr(target, out),
        Expr::IndexGet {
            target, indices, ..
        } => {
            collect_callees_expr(target, out);
            for idx in indices {
                match idx {
                    IndexArg::Scalar(e) | IndexArg::Range(e) => collect_callees_expr(e, out),
                    IndexArg::Whole => {}
                }
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
