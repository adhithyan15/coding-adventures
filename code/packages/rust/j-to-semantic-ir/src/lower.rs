//! The lowering pass from `coding_adventures_j_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **v0.1.0**.
//!
//! # J's CST shape (confirmed against `j-parser`'s own grammar and the
//! tree-walk `j-runtime::eval` already does over it — structurally almost
//! identical to `apl-to-semantic-ir`'s own APL CST, per MA06 §3's "two
//! nonterminals, reused from APL almost verbatim")
//!
//! ```text
//! program      = { line }
//! line         = statement NEWLINE | statement | NEWLINE  -- no `statement`
//!                child: blank/comment-only line, skip
//! statement    = assignment                                -- pure passthrough
//! assignment   = NAME ASSIGN_LOCAL assignment               -- 3 children: chained/actual
//!              | NAME ASSIGN_GLOBAL assignment              -- assignment (local vs global
//!                                                              is not behaviorally distinct
//!                                                              in this cut -- see below)
//!              | noun_expr                                  -- 1 child: base case
//! noun_expr    = term [ verb_expr noun_expr ]                -- 1 or 3 children
//!              | verb_expr noun_expr                         -- 2 children: monadic
//! term         = NUMBER { NUMBER }        -- 1+ stranded numbers
//!              | NAME
//!              | LPAREN noun_expr RPAREN
//! verb_expr    = simple_verb [ AT verb_expr ]   -- bare/adverbed primitive, optionally composed
//!              | LPAREN verb_train RPAREN       -- the one genuinely new production (trains)
//! simple_verb  = verb_primitive [ REDUCE | SCAN ]
//! verb_primitive = one of the 17 primitive glyph tokens
//! verb_train   = train_tooth train_tooth { train_tooth }   -- flat, 2+ teeth
//! train_tooth  = verb_expr | term
//! ```
//!
//! # Scope
//!
//! **Supported** (every construct `j-parser`'s grammar can produce):
//! - Number literals (`NUMBER`, underscore `_` negative sign — see "Negative
//!   literals" below), stranded literals (`1 2 3` → one rank-1
//!   [`Expr::ArrayLit`]), variables (`NAME`), parenthesised grouping.
//! - Assignment (`=.`/`=:`), including right-associative chained assignment
//!   (`a=.b=.3`) — mirrors `apl-to-semantic-ir`'s identical design (see its
//!   own module doc comment's "Chained assignment" section); J's two
//!   assignment operators are not given different lowering behaviour, since
//!   this cut has one flat whole-program scope with no local/global
//!   distinction to preserve (MA06 §4: "not meaningfully distinct in this
//!   cut").
//! - The 12 scalar dyadic atoms shared with APL
//!   (`+ - * % <. >. = ~: < > <: >:`), unconditionally lowered to
//!   [`Expr::ElementwiseOp`] — the same no-scalar/array-disambiguation
//!   simplification `apl-to-semantic-ir` already established (see that
//!   crate's own module doc comment), since none of these 12 has a
//!   non-elementwise reading in J either.
//! - The 6 of those 12 that have a monadic meaning (`+ - * % <. >.`), each
//!   mapped onto the *same* well-known [`Expr::BuiltinCall`] names
//!   `apl-to-semantic-ir` already introduced (`"neg"`/`"sign"`/`"recip"`/
//!   `"ceil"`/`"floor"`; `+` is a pass-through no-op).
//! - `$`/`i.`/`,` (shape-reshape, index-generator-index-of,
//!   ravel-catenate) — direct, field-for-field reuse of the exact SIR22
//!   addendum nodes APL's own `⍴`/`⍳`/`,` already map onto.
//! - `#` (tally/replicate) and `^` (exponential/power) — **genuinely new,
//!   no APL analogue or SIR22-addendum node**; see "Two new primitives"
//!   below.
//! - `/` (reduce) and `\` (scan), monadic-only, over any of the 12 scalar
//!   atoms — same restriction and SIR nodes as APL.
//! - `@` (compose/"atop") and parenthesised trains (`(f g)` hook, `(f g h)`/
//!   `(n g h)` fork) — **the one genuinely new production, no APL
//!   precedent**; see "Trains" below. Neither introduces a new SIR node —
//!   both lower to nested applications of the same node types, per MA06
//!   §5's own explicit instruction.
//! - Auto-print of a bare top-level noun expression (the same `"print"`
//!   [`Expr::BuiltinCall`] every SIR backend already implements).
//!
//! **Deliberately rejected** with a clean [`JLowerError`] (each is
//! syntactically constructible by `j-parser`'s grammar but semantically
//! invalid, exactly mirroring what `j-runtime::eval` discovers at
//! *runtime* and this frontend discovers at *lowering time* instead):
//! - The 6 comparison atoms (`= ~: < > <: >:`) used monadically.
//! - A reduce- or scan-decorated `simple_verb` used dyadically (`3+/4`).
//! - `$`/`i.`/`,`/`#`/`^` decorated with `/`/`\` — none of these 5 is "a
//!   scalar dyadic verb", mirroring `j-runtime::eval::require_scalar_binop`
//!   exactly (this is the reason `^`, despite mapping onto
//!   [`Expr::ElementwiseOp`] dyadically, is still classified as
//!   [`NonScalarAtom::Caret`] here rather than [`FnKind::Atom`] — see "Two
//!   new primitives" below).
//! - A hook or fork with a bare noun tooth anywhere except a fork's
//!   leading position (e.g. `(a b)`, two bare names/literals) — `j.grammar`
//!   itself explicitly documents that it does not encode this restriction
//!   syntactically, leaving it to this lowering pass (see that grammar
//!   file's own header comment).
//! - Trains/compose nested more than [`MAX_TRAIN_COMBINATOR_DEPTH`] levels
//!   deep, and a single train wider than [`MAX_TRAIN_TEETH`] teeth — see
//!   "Trains" below for why this cap exists and how it's sized.
//!
//! **Not applicable** (the grammar `j-parser` compiles literally cannot
//! produce these — boxing/nested arrays, the rank conjunction, user-defined
//! explicit verbs, control flow, unparenthesised trains, outer product —
//! MA06 §1/§4 — so there is no CST shape for this lowerer to ever reach).
//!
//! # No scalar/array disambiguation, chained assignment, auto-print
//!
//! Identical in every respect to `apl-to-semantic-ir`'s own design — see
//! that crate's module doc comment for the full rationale on each. J's
//! `noun_expr`/`term`/`assignment` productions are structurally identical
//! to APL's `value_expr`/`term`/`assignment` (MA06 §3 reuses them almost
//! verbatim, renamed), so the lowering logic for all three transfers
//! directly.
//!
//! # Two new primitives with no APL analogue: `#` and `^`
//!
//! `array_runtime::ops::BinOp` (the shared 12-variant scalar-dyadic type
//! both APL's and J's 12 atoms map onto) has no `Pow` variant at all, and
//! `#`'s monadic (tally) and dyadic (replicate) meanings are unrelated
//! structural-array operations, not a scalar dyadic function — so
//! `j-runtime::eval::JFn` categorises *both* as [`NonScalarAtom`] variants
//! (`Hash`/`Caret`), alongside `$`/`i.`/`,`, and this lowerer mirrors that
//! categorisation exactly (**not** folding `^` into [`FnKind::Atom`] even
//! though its dyadic meaning happens to fit [`Expr::ElementwiseOp`] — see
//! below): this is what correctly excludes both from reduce/scan
//! eligibility via [`Lowerer::require_scalar_atom`], matching
//! `j-runtime::eval::require_scalar_binop`'s identical real restriction, so
//! this frontend's *accepted surface* stays in lockstep with the reference
//! interpreter's (the whole point of the oracle-testing convention HML01
//! §"Verification" describes).
//!
//! - `#` monadic (tally) → `BuiltinCall("tally", [target])` (new). Dyadic
//!   (replicate, `x # y`) → `BuiltinCall("replicate", [x, y])` (new) — `x`
//!   is the per-item repeat-count vector, `y` the data, matching
//!   `j-runtime::builtins::replicate(x, y)`'s exact argument order.
//! - `^` monadic (natural exponential) → `BuiltinCall("exp", [target])`
//!   (new). Dyadic (power, `x ^ y`) → **does** lower to
//!   `Expr::ElementwiseOp { op: Pow, .. }` — `Pow` already exists in
//!   [`ElementwiseOpKind`] (added for MATLAB's `.^`, unused by APL's own
//!   12-atom cut), so at the *SIR* level (unlike `array_runtime::BinOp`,
//!   which has no such slot) J's dyadic `^` can reuse the existing
//!   elementwise representation directly, even though its categorisation
//!   for reduce/scan-eligibility purposes still mirrors `j-runtime`'s own
//!   `NonScalar` classification.
//!
//! # Trains: hooks, forks, compose (MA06 §3, no APL precedent)
//!
//! `j-runtime::eval::JFn` generalises `apl-runtime::eval::AplFn` by adding
//! exactly three variants — `Compose`/`Hook`/`Fork` — and this lowerer's own
//! [`FnKind`] does the same, building [`Expr`] trees instead of evaluating
//! [`array_runtime::Array`] values. The formulas
//! ([`Lowerer::apply_monadic`]/[`Lowerer::apply_dyadic`]'s own match arms
//! spell each one out) are the exact ones `j-runtime::eval` already
//! implements:
//!
//! | Combinator | Monadic (`y`) | Dyadic (`x`, `y`) |
//! |---|---|---|
//! | `f@g` (compose) | `f(g(y))` | `f(x g y)` |
//! | `(f g)` (hook) | `y f (g y)` | `x f (g y)` |
//! | `(f g h)` (fork) | `(f y) g (h y)` | `(x f y) g (x h y)` |
//! | `(n g h)` (fork, leading noun) | `n g (h y)` | `n g (x h y)` |
//!
//! A 4+-tooth train peels from the left, recursively, per MA06 §3's
//! corrected folding rule: `(a b c d) = (a (b c d))` — the peeled-off
//! leading tooth always plays a hook's `f` role (so it must be a verb,
//! never a bare noun — only a *fork's* leading position ever accepts one).
//! No new SIR node is needed for any of this — every combinator lowers to
//! ordinary nested [`Expr::ElementwiseOp`]/[`Expr::BuiltinCall`]/etc.
//! applications, exactly as MA06 §5 anticipated.
//!
//! ## Why trains get their own, much smaller depth guard
//!
//! Unlike every other recursive production in this file (and in
//! `apl-to-semantic-ir`, which never needed this at all — APL has no
//! trains), a hook or a *verb-left* fork **duplicates its noun operand(s)
//! in the emitted `Expr` tree**: `apply_monadic`'s `Hook` arm, for
//! instance, needs `y` twice (once as `f`'s left operand, once fed into
//! `g`) and — since this lowerer builds an owned `Expr` *tree*, not a
//! value the way `j-runtime::eval` does — the only way to use `y` twice is
//! to `.clone()` its already-lowered `Expr` subtree. A real interpreter
//! evaluates `y` once into an `Array` and cheaply reuses that value twice;
//! this lowerer, working over expression trees ahead of any evaluation,
//! re-embeds a full *copy* of the subtree each time.
//!
//! That duplication **compounds**: if `y` itself is (or contains) another
//! hook/fork — reachable either through a wide (4+-tooth) single train,
//! whose fold recurses through several nested `Hook`s, *or* through
//! explicitly parenthesised nested trains (`j.grammar` itself calls out
//! that trains "can nest, e.g. `((f g) h)`") — each additional combinator
//! level can again double how many copies of the innermost operand end up
//! embedded. `N` nested levels therefore bound the worst case at `2^N`
//! duplicated copies of whatever expression sits at the bottom, entirely
//! independent of the *general* [`MAX_EXPR_DEPTH`] guard (which bounds
//! ordinary CST-walk recursion for stack safety, not output-size
//! explosion, and is far too permissive a bound for this specific risk —
//! at `MAX_EXPR_DEPTH`'s own 256, `2^256` is obviously intractable). Note
//! that `@` (compose) never causes this: its formula uses each operand
//! exactly once, no `.clone()` involved — chaining many composes is
//! ordinary linear nesting, not multiplicative.
//!
//! [`MAX_TRAIN_COMBINATOR_DEPTH`] is a dedicated, much smaller cap
//! (`12`, bounding the worst case to `2^12 = 4096` duplicated copies — cheap
//! regardless of what the duplicated subtree contains) checked via
//! [`Lowerer::check_combinator_depth`] at **every** point a `Hook`/`Fork`
//! gets constructed: once per level when [`Lowerer::fold_train`] peels a
//! wide train (bounding a single train's own width), and once per
//! parenthesised sub-train encountered as a [`Lowerer::lower_verb_expr`]
//! tooth (bounding nesting achieved via explicit parentheses instead of
//! raw width) — the two mechanisms share one counter and one cap, since
//! both compound the identical risk. [`MAX_TRAIN_TEETH`] (`64`) is a
//! separate, purely defensive cap on raw tooth *count* — generous enough
//! that no realistic J program ever approaches it, existing only so this
//! lowerer never does O(tooth count) collection work for an implausibly
//! wide train before the real guard (`MAX_TRAIN_COMBINATOR_DEPTH`, tripped
//! during the fold a few levels in regardless) gets a chance to reject it.
//! Deferring true common-subexpression elimination (binding `y`/`x` to a
//! synthetic temporary once, referencing it thereafter instead of cloning)
//! to a follow-up is a deliberate scope decision, not an oversight — no
//! realistic J program in this cut's scope nests trains anywhere near this
//! cap, so the guard exists purely as a hard backstop against a
//! pathological input.

use std::collections::HashSet;

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, ElementwiseOpKind, Expr, Feature, FeatureManifest, Function, Metadata,
    Module, Scope, Span, Stmt,
};

/// Maximum ordinary CST-walk recursion depth — defense in depth, exactly
/// mirroring `apl-to-semantic-ir::MAX_EXPR_DEPTH`'s own identical
/// rationale: `j-parser`'s own `MAX_RULE_DEPTH` (70) already bounds how
/// deep a CST built from untrusted source can possibly be, so this can
/// never actually trip on a tree from `try_parse_j`; it exists purely so a
/// hand-built `GrammarASTNode` (or a future change to `j-parser`'s own cap)
/// can't turn a deep-but-technically-parseable input into an uncatchable
/// native stack overflow while walking it here. This is a *different*
/// guard from [`MAX_TRAIN_COMBINATOR_DEPTH`] below — see this file's module
/// doc comment's "Why trains get their own, much smaller depth guard"
/// section for why 256 is nowhere near tight enough for that specific risk.
const MAX_EXPR_DEPTH: usize = 256;

/// Maximum `Hook`/`Fork`/`Compose` nesting depth — see this file's module
/// doc comment's "Why trains get their own, much smaller depth guard"
/// section for the exact duplication mechanism this bounds and why `12` is
/// large enough for any realistic program while keeping the worst case
/// (`2^12 = 4096` duplicated copies) cheap.
const MAX_TRAIN_COMBINATOR_DEPTH: usize = 12;

/// Maximum raw tooth count for a single `verb_train` — a cheap, purely
/// defensive cap (not the main guard against duplication blowup, which is
/// [`MAX_TRAIN_COMBINATOR_DEPTH`] above) so this lowerer never spends
/// O(tooth count) work collecting an implausibly wide train's teeth before
/// the real guard gets a chance to reject it a few fold levels in. Mirrors
/// `j-runtime::eval::parse_verb_train`'s own identical "cap before doing
/// the O(tooth count) work" discipline, just against a much smaller
/// constant appropriate to this crate's own risk (that crate's own cap,
/// `builtins::MAX_ARRAY_LENGTH` = 1,000,000, guards a completely different
/// concern — evaluating a huge flat train's worth of *array* work, not
/// building an exponentially-duplicating *expression tree*).
const MAX_TRAIN_TEETH: usize = 64;

/// Synthetic file name used for all spans (the CST does not carry the
/// original path).
const FILE: &str = "<j>";

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// An error encountered during J → SIR lowering.
///
/// Mirrors `AplLowerError`/`MatlabLowerError`'s shape exactly (a `message`
/// plus 1-based `line`/`column`) so tooling can treat every SIR frontend
/// uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for JLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JLowerError at {}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for JLowerError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed J CST (rooted at the `program` rule) into a SIR module.
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<semantic_ir::Module, JLowerError> {
    Lowerer::new(module_name).lower_file(tree)
}

// ---------------------------------------------------------------------------
// Verb-expression representation
// ---------------------------------------------------------------------------

/// One of `j.tokens`' five bespoke (non-scalar-dyadic) primitive verbs, kept
/// around so error messages can name the actual glyph (`"$"`, not
/// `"DOLLAR"`) — mirrors `j-runtime::eval::NonScalarAtom` exactly, right
/// down to including `Caret` here despite `^`'s dyadic form lowering to an
/// `ElementwiseOp` (see this file's module doc comment's "Two new
/// primitives" section for why).
#[derive(Clone, Copy)]
enum NonScalarAtom {
    Dollar,
    Idot,
    Ravel,
    Hash,
    Caret,
}

impl NonScalarAtom {
    fn glyph(self) -> &'static str {
        match self {
            NonScalarAtom::Dollar => "$",
            NonScalarAtom::Idot => "i.",
            NonScalarAtom::Ravel => ",",
            NonScalarAtom::Hash => "#",
            NonScalarAtom::Caret => "^",
        }
    }
}

/// This lowerer's own representation of a `verb_expr`: "which verb, and
/// with which adverb/conjunction/train structure (if any) applied" —
/// generalises `apl-to-semantic-ir::FnKind` with `Compose`/`Hook`/`Fork`,
/// exactly mirroring how `j-runtime::eval::JFn` generalises
/// `apl-runtime::eval::AplFn` (MA06 §5's own explicit instruction). Unlike
/// APL, J has no outer-product operator in this cut's scope, so there is no
/// `Outer` variant here (mirrors `JFn` exactly).
enum FnKind {
    /// One of the 12 verbs that map onto [`ElementwiseOpKind`] (`+ - * % <.
    /// >. = ~: < > <: >:`).
    Atom(ElementwiseOpKind),
    /// `$`/`i.`/`,`/`#`/`^` — bespoke monadic+dyadic logic that does not
    /// fit "an operator over a scalar dyadic verb" at all, so none of these
    /// ever plug into reduce/scan.
    NonScalar(NonScalarAtom),
    /// A `BinOp`-mappable atom with `/` (reduce) applied — inherently
    /// monadic.
    Reduce(ElementwiseOpKind),
    /// A `BinOp`-mappable atom with `\` (scan) applied — also monadic.
    Scan(ElementwiseOpKind),
    /// `f@g` ("atop" compose) — monadic `f(g(y))`; dyadic `f(x g y)`.
    Compose(Box<FnKind>, Box<FnKind>),
    /// A 2-tooth train (hook) — monadic `y f (g y)`; dyadic `x f (g y)`.
    Hook(Box<FnKind>, Box<FnKind>),
    /// A 3-tooth train (fork), or a 4+-tooth train peeled down to this base
    /// case (see [`Lowerer::fold_train`]).
    Fork(ForkLeft, Box<FnKind>, Box<FnKind>),
}

/// The first ("left") tooth of a [`FnKind::Fork`]: either an ordinary verb,
/// or a literal noun constant (only meaningful in this leading position —
/// see [`Lowerer::fold_train`]/[`Lowerer::require_verb_tooth`]).
enum ForkLeft {
    Verb(Box<FnKind>),
    Noun(Expr),
}

/// What one `train_tooth` lowers to, before it is known whether the
/// overall train needs it to be a verb or (in a fork's leading position
/// only) accepts it as a literal noun.
enum ToothValue {
    Verb(FnKind),
    Noun(Expr),
}

// ---------------------------------------------------------------------------
// The lowerer
// ---------------------------------------------------------------------------

/// J scopes every variable to the *whole program* (there are no blocks,
/// loops, or user-defined explicit verbs in this cut — MA06 §1/§4) —
/// mirrors `apl-to-semantic-ir::Lowerer` exactly: one flat set of bound
/// names for its entire lifetime.
struct Lowerer {
    module_name: String,
    observed: FeatureManifest,
    locals: HashSet<String>,
}

impl Lowerer {
    fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            observed: FeatureManifest::new(),
            locals: HashSet::new(),
        }
    }

    // -------------------------------------------------------------------
    // top level: `program` → one `main` function
    // -------------------------------------------------------------------

    fn lower_file(&mut self, program: &GrammarASTNode) -> Result<Module, JLowerError> {
        if program.rule_name != "program" {
            return Err(self.err_at(
                program,
                format!("expected `program` root, got `{}`", program.rule_name),
            ));
        }

        self.observed.add(Feature::DynamicTyping);

        let mut stmts: Vec<Stmt> = Vec::new();
        for line in child_nodes(program) {
            if line.rule_name != "line" {
                continue;
            }
            // A `line` with no `statement` child (a blank line, or a
            // comment-only line -- `NB.` comments are already stripped by
            // the lexer's skip pattern) is a bare NEWLINE production; skip
            // it, don't error.
            let Some(stmt_node) = first_child_named(line, "statement") else {
                continue;
            };
            let assignment_node = only_node(stmt_node)
                .ok_or_else(|| self.err_at(stmt_node, "malformed statement".to_string()))?;
            let mut new_stmts = self.lower_top_level_statement(assignment_node, 0)?;
            stmts.append(&mut new_stmts);
        }

        let span = Span::point(FILE, 1, 1);
        let main = Function {
            name: "main".to_string(),
            params: vec![],
            return_type: None,
            captures: vec![],
            body: Block {
                stmts,
                value: Expr::NilLit { span: span.clone() },
                span: span.clone(),
            },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span: span.clone(),
        };

        let metadata = Metadata::new()
            .with_source_language("j")
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

    /// Dispatch one top-level `assignment` node (a `statement`'s sole
    /// child) into zero or more [`Stmt`]s.
    fn lower_top_level_statement(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Vec<Stmt>, JLowerError> {
        self.check_depth(node, depth)?;
        match node.children.len() {
            // Base case: a bare `noun_expr`, not an assignment. Real J
            // auto-print session semantics (MA06 §4, mirroring MA05 §4 --
            // see this file's module doc comment).
            1 => {
                let noun_expr_node = only_node(node)
                    .ok_or_else(|| self.err_at(node, "malformed noun_expr statement".to_string()))?;
                let v = self.lower_noun_expr(noun_expr_node, depth + 1)?;
                let span = v.span().clone();
                Ok(vec![Stmt::ExprStmt {
                    expr: Expr::BuiltinCall {
                        name: "print".to_string(),
                        args: vec![v],
                        effects: EffectSet::PURE,
                        span: span.clone(),
                    },
                    span,
                }])
            }
            // `NAME ASSIGN_LOCAL assignment` or `NAME ASSIGN_GLOBAL
            // assignment` -- an actual assignment (possibly chained).
            // Assignment is silent (MA06 §4): emit every statement the
            // chain unrolled into, and nothing else.
            3 => {
                let (stmts, _final_value) = self.lower_assignment_chain(node, depth)?;
                Ok(stmts)
            }
            n => Err(self.err_at(node, format!("malformed assignment with {n} children"))),
        }
    }

    // -------------------------------------------------------------------
    // assignment (including chained assignment)
    // -------------------------------------------------------------------

    /// Recursively lower an `assignment` node. Returns the statements the
    /// chain unrolled into (in dependency order) and an [`Expr`] the OUTER
    /// caller can reuse to reference "the value just bound" -- mirrors
    /// `apl-to-semantic-ir::Lowerer::lower_assignment_chain` exactly (see
    /// that crate's module doc comment's "Chained assignment" section for
    /// the full design rationale, which transfers unchanged).
    fn lower_assignment_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<(Vec<Stmt>, Expr), JLowerError> {
        self.check_depth(node, depth)?;
        match node.children.len() {
            1 => {
                let noun_expr_node = only_node(node).ok_or_else(|| {
                    self.err_at(node, "malformed noun_expr in assignment".to_string())
                })?;
                let v = self.lower_noun_expr(noun_expr_node, depth + 1)?;
                Ok((vec![], v))
            }
            3 => {
                let name = self.assignment_target_name(node)?;
                let inner = only_node(node).ok_or_else(|| {
                    self.err_at(node, "malformed assignment: no nested assignment".to_string())
                })?;
                let (mut stmts, inner_value) = self.lower_assignment_chain(inner, depth + 1)?;
                let span = self.span_of(node);
                if self.locals.insert(name.clone()) {
                    stmts.push(Stmt::LetStarBinding {
                        name: name.clone(),
                        sir_type: None,
                        value: inner_value,
                        span: span.clone(),
                    });
                } else {
                    self.observed.add(Feature::MutableBindings);
                    stmts.push(Stmt::Assign {
                        name: name.clone(),
                        scope: Scope::Local,
                        value: inner_value,
                        span: span.clone(),
                    });
                }
                Ok((stmts, Expr::VarRef { name, scope: Scope::Local, span }))
            }
            n => Err(self.err_at(node, format!("malformed assignment with {n} children"))),
        }
    }

    /// The `NAME` token of an actual assignment's target -- the first child
    /// of a 3-child `assignment` node. Deliberately does not inspect
    /// whether the second child is `ASSIGN_LOCAL` or `ASSIGN_GLOBAL` --
    /// this cut gives both the identical lowering (MA06 §4: "not
    /// meaningfully distinct").
    fn assignment_target_name(&self, node: &GrammarASTNode) -> Result<String, JLowerError> {
        match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NAME" => {
                Ok(t.value.clone())
            }
            _ => Err(self.err_at(node, "malformed assignment (missing target name)".to_string())),
        }
    }

    // -------------------------------------------------------------------
    // noun_expr / term
    // -------------------------------------------------------------------

    /// `noun_expr = term [ verb_expr noun_expr ] | verb_expr noun_expr` --
    /// mirrors `apl-to-semantic-ir::Lowerer::lower_value_expr`'s own
    /// dispatch-by-child-count exactly (both productions collapse to the
    /// same three shapes once tokens are filtered out).
    fn lower_noun_expr(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, JLowerError> {
        self.check_depth(node, depth)?;
        let span = self.span_of(node);
        let kids = child_nodes(node);
        match kids.as_slice() {
            [term] => self.lower_term(term, depth + 1),
            [vexpr, sub] => {
                // A fresh top-level verb application starts its own
                // combinator-depth budget at 0 -- nesting only compounds
                // *within* one train's own construction (see this file's
                // module doc comment's "Why trains get their own, much
                // smaller depth guard" section), not across unrelated
                // applications elsewhere in the program.
                let f = self.lower_verb_expr(vexpr, depth + 1, 0)?;
                let arg = self.lower_noun_expr(sub, depth + 1)?;
                self.apply_monadic(f, arg, span)
            }
            [lhs_term, vexpr, sub] => {
                let lhs = self.lower_term(lhs_term, depth + 1)?;
                let f = self.lower_verb_expr(vexpr, depth + 1, 0)?;
                let rhs = self.lower_noun_expr(sub, depth + 1)?;
                self.apply_dyadic(f, lhs, rhs, span)
            }
            other => Err(self.err_at(
                node,
                format!("malformed noun_expr with {} children", other.len()),
            )),
        }
    }

    /// `term = NUMBER { NUMBER } | NAME | LPAREN noun_expr RPAREN`.
    fn lower_term(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, JLowerError> {
        self.check_depth(node, depth)?;
        match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NUMBER" => {
                // "Stranding": one or more juxtaposed NUMBER tokens form a
                // single term -- `1 2 3` is one 3-element vector, a lone
                // `5` is a rank-0 scalar (MA06 §4, inherited from APL
                // unchanged).
                let numbers: Vec<&Token> = node
                    .children
                    .iter()
                    .filter_map(|c| match c {
                        ASTNodeOrToken::Token(tok) if tok.effective_type_name() == "NUMBER" => {
                            Some(tok)
                        }
                        _ => None,
                    })
                    .collect();
                if numbers.len() == 1 {
                    self.number_literal(numbers[0])
                } else {
                    self.observed.add(Feature::NDArrays);
                    self.observed.add(Feature::ArrayColumnMajor);
                    let span = self.span_of(node);
                    let row: Vec<Expr> = numbers
                        .iter()
                        .map(|tok| self.number_literal(tok))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(Expr::ArrayLit { rows: vec![row], span })
                }
            }
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NAME" => {
                let span = self.span_of(node);
                if !self.locals.contains(&t.value) {
                    return Err(self.err_at(
                        node,
                        format!("undefined variable `{}` (not previously assigned)", t.value),
                    ));
                }
                Ok(Expr::VarRef { name: t.value.clone(), scope: Scope::Local, span })
            }
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "LPAREN" => {
                let inner = only_node(node)
                    .ok_or_else(|| self.err_at(node, "malformed parenthesised term".to_string()))?;
                self.lower_noun_expr(inner, depth + 1)
            }
            _ => Err(self.err_at(node, "malformed term".to_string())),
        }
    }

    /// Convert one `NUMBER` token into an `Expr::IntLit`/`Expr::FloatLit`,
    /// observing `Feature::Floats` for the latter -- mirrors
    /// `apl-to-semantic-ir::Lowerer::number_literal` exactly.
    fn number_literal(&mut self, tok: &Token) -> Result<Expr, JLowerError> {
        let span = Span::point(FILE, tok.line, tok.column);
        let expr = number_literal_expr(&tok.value, &span).map_err(|message| JLowerError {
            message,
            line: tok.line,
            column: tok.column,
        })?;
        if matches!(expr, Expr::FloatLit { .. }) {
            self.observed.add(Feature::Floats);
        }
        Ok(expr)
    }

    // -------------------------------------------------------------------
    // verb_expr / simple_verb / verb_primitive
    // -------------------------------------------------------------------

    /// `verb_expr = simple_verb [ AT verb_expr ] | LPAREN verb_train
    /// RPAREN`. `combinator_depth` is passed through unchanged into the
    /// `AT` (compose) continuation -- composing never duplicates an
    /// operand, so chaining many composes is ordinary linear nesting, not
    /// something that needs to compound toward
    /// [`MAX_TRAIN_COMBINATOR_DEPTH`] on its own (see this file's module
    /// doc comment) -- but is incremented by one on entry to a
    /// parenthesised sub-train, since *that* is exactly the "nesting via
    /// explicit parentheses" mechanism that section describes.
    fn lower_verb_expr(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        combinator_depth: usize,
    ) -> Result<FnKind, JLowerError> {
        self.check_depth(node, depth)?;
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(simple)] if simple.rule_name == "simple_verb" => {
                self.lower_simple_verb(simple)
            }
            [ASTNodeOrToken::Node(simple), ASTNodeOrToken::Token(at), ASTNodeOrToken::Node(rest)]
                if simple.rule_name == "simple_verb" && at.effective_type_name() == "AT" =>
            {
                let f = self.lower_simple_verb(simple)?;
                let g = self.lower_verb_expr(rest, depth + 1, combinator_depth)?;
                Ok(FnKind::Compose(Box::new(f), Box::new(g)))
            }
            [ASTNodeOrToken::Token(lparen), ASTNodeOrToken::Node(train), ASTNodeOrToken::Token(_rparen)]
                if lparen.effective_type_name() == "LPAREN" =>
            {
                self.lower_verb_train(train, depth + 1, combinator_depth + 1)
            }
            _ => Err(self.err_at(node, "malformed verb_expr".to_string())),
        }
    }

    /// `simple_verb = verb_primitive [ REDUCE | SCAN ]`.
    fn lower_simple_verb(&self, node: &GrammarASTNode) -> Result<FnKind, JLowerError> {
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(prim)] => self.lower_verb_primitive(prim),
            [ASTNodeOrToken::Node(prim), ASTNodeOrToken::Token(adverb)] => {
                match adverb.effective_type_name() {
                    "REDUCE" => Ok(FnKind::Reduce(self.require_scalar_atom(prim, "reduce (/)")?)),
                    "SCAN" => Ok(FnKind::Scan(self.require_scalar_atom(prim, "scan (\\)")?)),
                    other => Err(self.err_at(node, format!("unexpected adverb token `{other}`"))),
                }
            }
            _ => Err(self.err_at(node, "malformed simple_verb".to_string())),
        }
    }

    /// `verb_primitive`: always exactly one child, a single token naming
    /// the primitive glyph -- mirrors `j-runtime::eval::parse_verb_primitive`'s
    /// exact token-to-variant mapping (confirmed against that function),
    /// including `FLOOR`→`Min`/`CEILING`→`Max` (the same target `apl.tokens`'
    /// own FLOOR/CEILING already use -- MA06 §4: only *which digraph* spells
    /// floor differs between J and APL, not the underlying op).
    fn lower_verb_primitive(&self, node: &GrammarASTNode) -> Result<FnKind, JLowerError> {
        let tok = match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) => t,
            _ => return Err(self.err_at(node, "malformed verb_primitive".to_string())),
        };
        Ok(match tok.effective_type_name() {
            "PLUS" => FnKind::Atom(ElementwiseOpKind::Add),
            "MINUS" => FnKind::Atom(ElementwiseOpKind::Sub),
            "STAR" => FnKind::Atom(ElementwiseOpKind::Mul),
            "PERCENT" => FnKind::Atom(ElementwiseOpKind::Div),
            "FLOOR" => FnKind::Atom(ElementwiseOpKind::Min),
            "CEILING" => FnKind::Atom(ElementwiseOpKind::Max),
            "EQ" => FnKind::Atom(ElementwiseOpKind::Eq),
            "NE" => FnKind::Atom(ElementwiseOpKind::Ne),
            "LT" => FnKind::Atom(ElementwiseOpKind::Lt),
            "LE" => FnKind::Atom(ElementwiseOpKind::Le),
            "GE" => FnKind::Atom(ElementwiseOpKind::Ge),
            "GT" => FnKind::Atom(ElementwiseOpKind::Gt),
            "DOLLAR" => FnKind::NonScalar(NonScalarAtom::Dollar),
            "IDOT" => FnKind::NonScalar(NonScalarAtom::Idot),
            "RAVEL" => FnKind::NonScalar(NonScalarAtom::Ravel),
            "HASH" => FnKind::NonScalar(NonScalarAtom::Hash),
            "CARET" => FnKind::NonScalar(NonScalarAtom::Caret),
            other => return Err(self.err_at(node, format!("unknown verb primitive `{other}`"))),
        })
    }

    /// Reduce/scan apply only to the 12 verbs that map onto
    /// [`ElementwiseOpKind`] -- `$`/`i.`/`,`/`#`/`^` are not "a scalar
    /// dyadic verb" at all, so stacking an adverb on one of them is a
    /// clean, explicit error, mirroring
    /// `j-runtime::eval::require_scalar_binop`'s identical restriction
    /// (including its rejection of `^`, despite `^`'s own dyadic meaning
    /// otherwise fitting `ElementwiseOp` -- see this file's module doc
    /// comment's "Two new primitives" section).
    fn require_scalar_atom(
        &self,
        atom: &GrammarASTNode,
        context: &str,
    ) -> Result<ElementwiseOpKind, JLowerError> {
        match self.lower_verb_primitive(atom)? {
            FnKind::Atom(op) => Ok(op),
            FnKind::NonScalar(a) => Err(self.err_at(
                atom,
                format!(
                    "{} is not a scalar dyadic verb and cannot take the {context} adverb",
                    a.glyph()
                ),
            )),
            FnKind::Reduce(_) | FnKind::Scan(_) | FnKind::Compose(_, _) | FnKind::Hook(_, _)
            | FnKind::Fork(_, _, _) => {
                unreachable!("lower_verb_primitive never itself produces an adverb/conjunction/train-bearing FnKind")
            }
        }
    }

    // -------------------------------------------------------------------
    // trains: verb_train / train_tooth / folding
    // -------------------------------------------------------------------

    /// `verb_train = train_tooth train_tooth { train_tooth }` -- a flat
    /// list of 2+ teeth. Lower each tooth first, then [`Self::fold_train`]
    /// applies MA06 §3's peel-from-the-left folding rule.
    fn lower_verb_train(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        combinator_depth: usize,
    ) -> Result<FnKind, JLowerError> {
        self.check_depth(node, depth)?;
        let tooth_nodes = child_nodes(node);
        // Cheap defensive cap on raw width, checked before any O(tooth
        // count) collection work -- see this file's module doc comment's
        // "Why trains get their own, much smaller depth guard" section for
        // why this is a *secondary* guard, not the main one.
        if tooth_nodes.len() > MAX_TRAIN_TEETH {
            return Err(self.err_at(
                node,
                format!(
                    "train has {} teeth, exceeding the cap of {MAX_TRAIN_TEETH}",
                    tooth_nodes.len()
                ),
            ));
        }
        let mut teeth = Vec::with_capacity(tooth_nodes.len());
        for tooth in tooth_nodes {
            teeth.push(self.lower_train_tooth(tooth, depth + 1, combinator_depth)?);
        }
        self.fold_train(teeth, combinator_depth, node)
    }

    /// `train_tooth = verb_expr | term`. A `term` tooth is a literal noun
    /// constant -- only meaningful in a fork's leading position, enforced
    /// by [`Self::fold_train`]/[`Self::require_verb_tooth`], not here (this
    /// grammar production itself does not encode that restriction -- see
    /// `j.grammar`'s own header comment, quoted in this file's module doc
    /// comment).
    fn lower_train_tooth(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        combinator_depth: usize,
    ) -> Result<ToothValue, JLowerError> {
        self.check_depth(node, depth)?;
        match node.children.first() {
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "verb_expr" => {
                Ok(ToothValue::Verb(self.lower_verb_expr(n, depth + 1, combinator_depth)?))
            }
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "term" => {
                Ok(ToothValue::Noun(self.lower_term(n, depth + 1)?))
            }
            _ => Err(self.err_at(node, "malformed train_tooth".to_string())),
        }
    }

    /// Fold a flat list of `verb_train` teeth into a [`FnKind`], following
    /// MA06 §3's corrected right-to-left, peel-from-the-left rule --
    /// mirrors `j-runtime::eval::fold_train`'s exact folding shape, just
    /// building an `FnKind`/`Expr` tree instead of evaluating a value.
    /// Checks [`MAX_TRAIN_COMBINATOR_DEPTH`] on every call (base case or
    /// recursive) since every call constructs exactly one `Hook`/`Fork`.
    fn fold_train(
        &self,
        mut teeth: Vec<ToothValue>,
        combinator_depth: usize,
        node: &GrammarASTNode,
    ) -> Result<FnKind, JLowerError> {
        self.check_combinator_depth(node, combinator_depth)?;
        match teeth.len() {
            0 | 1 => Err(self.err_at(
                node,
                format!("malformed train with {} teeth (need at least 2)", teeth.len()),
            )),
            2 => {
                let g = self.require_verb_tooth(teeth.pop().unwrap(), node, "a hook's second tooth")?;
                let f = self.require_verb_tooth(teeth.pop().unwrap(), node, "a hook's first tooth")?;
                Ok(FnKind::Hook(Box::new(f), Box::new(g)))
            }
            3 => {
                let h = self.require_verb_tooth(teeth.pop().unwrap(), node, "a fork's third tooth")?;
                let g = self.require_verb_tooth(teeth.pop().unwrap(), node, "a fork's second tooth")?;
                let left = match teeth.pop().unwrap() {
                    ToothValue::Verb(f) => ForkLeft::Verb(Box::new(f)),
                    ToothValue::Noun(n) => ForkLeft::Noun(n),
                };
                Ok(FnKind::Fork(left, Box::new(g), Box::new(h)))
            }
            _ => {
                // `(a b c d ...)` = `(a (b c d ...))` -- the peeled-off
                // leading tooth always plays a hook's `f` role, so (unlike
                // a fork's leading position) it must be a verb.
                let rest = teeth.split_off(1);
                let a = self.require_verb_tooth(
                    teeth.pop().unwrap(),
                    node,
                    "a train's leading tooth (width 4+)",
                )?;
                let g = self.fold_train(rest, combinator_depth + 1, node)?;
                Ok(FnKind::Hook(Box::new(a), Box::new(g)))
            }
        }
    }

    /// Unwrap a [`ToothValue`] that must be a verb (every train position
    /// except a fork's leading tooth), erroring cleanly on a bare noun --
    /// mirrors `j.grammar`'s own disclosed example (`(A B)`, two bare
    /// names, parses syntactically but is semantically invalid) and
    /// `j-runtime::eval::fold_train`'s identical runtime rejection.
    fn require_verb_tooth(
        &self,
        tooth: ToothValue,
        node: &GrammarASTNode,
        context: &str,
    ) -> Result<FnKind, JLowerError> {
        match tooth {
            ToothValue::Verb(f) => Ok(f),
            ToothValue::Noun(_) => Err(self.err_at(
                node,
                format!(
                    "{context} must be a verb, not a bare noun (a literal constant is only \
                     meaningful in a fork's leading position)"
                ),
            )),
        }
    }

    // -------------------------------------------------------------------
    // monadic / dyadic application
    // -------------------------------------------------------------------

    /// Apply a monadic (one-argument) verb-expression to `arg`.
    fn apply_monadic(&mut self, f: FnKind, arg: Expr, span: Span) -> Result<Expr, JLowerError> {
        match f {
            FnKind::Atom(op) => self.apply_monadic_scalar(op, arg),
            FnKind::NonScalar(NonScalarAtom::Dollar) => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::Shape { target: Box::new(arg), span })
            }
            FnKind::NonScalar(NonScalarAtom::Idot) => {
                self.observed.add(Feature::NDArrays);
                Ok(Expr::IndexGenerator { count: Box::new(arg), span })
            }
            FnKind::NonScalar(NonScalarAtom::Ravel) => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::Ravel { target: Box::new(arg), span })
            }
            // `#` monadic (tally) / `^` monadic (natural exponential) --
            // genuinely new builtins, no SIR22-addendum node fits either
            // (see this file's module doc comment's "Two new primitives").
            FnKind::NonScalar(NonScalarAtom::Hash) => Ok(wrap_builtin("tally", arg)),
            FnKind::NonScalar(NonScalarAtom::Caret) => Ok(wrap_builtin("exp", arg)),
            FnKind::Reduce(op) => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::Reduce { op, target: Box::new(arg), span })
            }
            FnKind::Scan(op) => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::Scan { op, target: Box::new(arg), span })
            }
            // Compose, monadic: `(f@g) y = f (g y)` -- `arg` used once, no
            // duplication.
            FnKind::Compose(f, g) => {
                let inner = self.apply_monadic(*g, arg, span.clone())?;
                self.apply_monadic(*f, inner, span)
            }
            // Hook, monadic: `(f g) y = y f (g y)` -- `arg` used TWICE
            // (bounded by MAX_TRAIN_COMBINATOR_DEPTH at construction time,
            // see this file's module doc comment).
            FnKind::Hook(f, g) => {
                let gy = self.apply_monadic(*g, arg.clone(), span.clone())?;
                self.apply_dyadic(*f, arg, gy, span)
            }
            // Fork, monadic: verb-left `(f g h) y = (f y) g (h y)`;
            // leading-noun `(n g h) y = n g (h y)` (the leading-noun case
            // uses `y` only once -- no duplication there).
            FnKind::Fork(left, g, h) => match left {
                ForkLeft::Verb(f) => {
                    let fy = self.apply_monadic(*f, arg.clone(), span.clone())?;
                    let hy = self.apply_monadic(*h, arg, span.clone())?;
                    self.apply_dyadic(*g, fy, hy, span)
                }
                ForkLeft::Noun(n) => {
                    let hy = self.apply_monadic(*h, arg, span.clone())?;
                    self.apply_dyadic(*g, n, hy, span)
                }
            },
        }
    }

    /// Apply a dyadic (two-argument) verb-expression to `lhs`/`rhs`.
    fn apply_dyadic(
        &mut self,
        f: FnKind,
        lhs: Expr,
        rhs: Expr,
        span: Span,
    ) -> Result<Expr, JLowerError> {
        match f {
            FnKind::Atom(op) => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::ElementwiseOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span })
            }
            FnKind::NonScalar(NonScalarAtom::Dollar) => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::Reshape { shape: Box::new(lhs), target: Box::new(rhs), span })
            }
            FnKind::NonScalar(NonScalarAtom::Idot) => {
                self.observed.add(Feature::NDArrays);
                Ok(Expr::IndexOf { haystack: Box::new(lhs), needle: Box::new(rhs), span })
            }
            FnKind::NonScalar(NonScalarAtom::Ravel) => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::Catenate { lhs: Box::new(lhs), rhs: Box::new(rhs), span })
            }
            // `x # y` (replicate): `x` is the per-item repeat-count vector,
            // `y` the data -- matches `j-runtime::builtins::replicate(x,
            // y)`'s exact argument order.
            FnKind::NonScalar(NonScalarAtom::Hash) => Ok(Expr::BuiltinCall {
                name: "replicate".to_string(),
                args: vec![lhs, rhs],
                effects: EffectSet::PURE,
                span,
            }),
            // `x ^ y` (power) -- unlike its monadic form, this DOES reuse
            // an existing SIR22 node: `Pow` already exists in
            // `ElementwiseOpKind` (added for MATLAB's `.^`), even though
            // `array_runtime::BinOp` has no such variant (see this file's
            // module doc comment's "Two new primitives" section for why
            // `^` is still classified `NonScalar`, not `Atom`, despite
            // this).
            FnKind::NonScalar(NonScalarAtom::Caret) => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::ElementwiseOp {
                    op: ElementwiseOpKind::Pow,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    span,
                })
            }
            FnKind::Reduce(_) => Err(self.err_at_span(
                &span,
                "/ (reduce) takes exactly one operand, but was applied dyadically".to_string(),
            )),
            FnKind::Scan(_) => Err(self.err_at_span(
                &span,
                "\\ (scan) takes exactly one operand, but was applied dyadically".to_string(),
            )),
            // Compose, dyadic: `x (f@g) y = f (x g y)` -- `x`/`y` each used
            // once, no duplication.
            FnKind::Compose(f, g) => {
                let inner = self.apply_dyadic(*g, lhs, rhs, span.clone())?;
                self.apply_monadic(*f, inner, span)
            }
            // Hook, dyadic: `x (f g) y = x f (g y)` -- `g` always applies
            // MONADICALLY to `y` alone, regardless of this call's own
            // dyadic arity; `x`/`y` each used once, no duplication (unlike
            // the monadic-hook case above).
            FnKind::Hook(f, g) => {
                let gy = self.apply_monadic(*g, rhs, span.clone())?;
                self.apply_dyadic(*f, lhs, gy, span)
            }
            // Fork, dyadic: verb-left `x (f g h) y = (x f y) g (x h y)` --
            // `x`/`y` each used TWICE (bounded by
            // MAX_TRAIN_COMBINATOR_DEPTH); leading-noun `x (n g h) y = n g
            // (x h y)` -- `x`/`y` each used once, no duplication.
            FnKind::Fork(left, g, h) => match left {
                ForkLeft::Verb(f) => {
                    let fxy = self.apply_dyadic(*f, lhs.clone(), rhs.clone(), span.clone())?;
                    let hxy = self.apply_dyadic(*h, lhs, rhs, span.clone())?;
                    self.apply_dyadic(*g, fxy, hxy, span)
                }
                ForkLeft::Noun(n) => {
                    let hxy = self.apply_dyadic(*h, lhs, rhs, span.clone())?;
                    self.apply_dyadic(*g, n, hxy, span)
                }
            },
        }
    }

    /// Monadic meaning of the six atoms that have one (MA06 §4, identical
    /// mapping to APL's own `apply_monadic_scalar`): `+` conjugate
    /// (pass-through, no complex numbers in this cut), `-` negate, `*`
    /// sign, `%` reciprocal, `<.` floor, `>.` ceiling. The six comparisons
    /// have **no** monadic meaning -- mirrors
    /// `j-runtime::eval::apply_monadic_scalar`'s identical restriction,
    /// just with J's own glyph spellings in the error text.
    fn apply_monadic_scalar(&mut self, op: ElementwiseOpKind, operand: Expr) -> Result<Expr, JLowerError> {
        match op {
            ElementwiseOpKind::Add => Ok(operand),
            ElementwiseOpKind::Sub => Ok(wrap_builtin("neg", operand)),
            ElementwiseOpKind::Mul => Ok(wrap_builtin("sign", operand)),
            ElementwiseOpKind::Div => Ok(wrap_builtin("recip", operand)),
            ElementwiseOpKind::Min => Ok(wrap_builtin("floor", operand)),
            ElementwiseOpKind::Max => Ok(wrap_builtin("ceil", operand)),
            ElementwiseOpKind::Eq
            | ElementwiseOpKind::Ne
            | ElementwiseOpKind::Lt
            | ElementwiseOpKind::Le
            | ElementwiseOpKind::Ge
            | ElementwiseOpKind::Gt => {
                let span = operand.span().clone();
                Err(self.err_at_span(
                    &span,
                    format!(
                        "no monadic form for {} (comparison atoms are dyadic-only in J)",
                        glyph_for_comparison(op)
                    ),
                ))
            }
            ElementwiseOpKind::Pow => unreachable!(
                "FnKind::Atom(Pow) is never constructed for J -- CARET lowers to \
                 FnKind::NonScalar(Caret) instead (see this file's module doc comment's \
                 \"Two new primitives\" section), so ElementwiseOpKind::Pow only ever reaches \
                 apply_dyadic's own NonScalar(Caret) arm directly, never this scalar-atom table"
            ),
        }
    }

    // -------------------------------------------------------------------
    // small helpers
    // -------------------------------------------------------------------

    fn span_of(&self, node: &GrammarASTNode) -> Span {
        Span::point(FILE, node.start_line.unwrap_or(1), node.start_column.unwrap_or(1))
    }

    fn err_at(&self, node: &GrammarASTNode, message: String) -> JLowerError {
        JLowerError {
            message,
            line: node.start_line.unwrap_or(1),
            column: node.start_column.unwrap_or(1),
        }
    }

    fn err_at_span(&self, span: &Span, message: String) -> JLowerError {
        JLowerError { message, line: span.start_line, column: span.start_col }
    }

    fn check_depth(&self, node: &GrammarASTNode, depth: usize) -> Result<(), JLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        Ok(())
    }

    fn check_combinator_depth(
        &self,
        node: &GrammarASTNode,
        combinator_depth: usize,
    ) -> Result<(), JLowerError> {
        if combinator_depth > MAX_TRAIN_COMBINATOR_DEPTH {
            return Err(self.err_at(
                node,
                format!(
                    "train/compose nesting too deep (exceeds {MAX_TRAIN_COMBINATOR_DEPTH} \
                     combinator levels) -- each hook/fork level can double how many times an \
                     operand is embedded in the emitted SIR tree, so this is capped \
                     independently of the general expression-depth limit"
                ),
            ));
        }
        Ok(())
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

/// The first *node* child named `kind`, or `None`.
fn first_child_named<'a>(node: &'a GrammarASTNode, kind: &str) -> Option<&'a GrammarASTNode> {
    child_nodes(node).into_iter().find(|n| n.rule_name == kind)
}

/// The first (and, for every rule this crate lowers, only) *node* child.
fn only_node(node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    node.children.iter().find_map(|c| match c {
        ASTNodeOrToken::Node(n) => Some(n),
        ASTNodeOrToken::Token(_) => None,
    })
}

/// Wrap `operand` in a well-known, pure, single-argument
/// [`Expr::BuiltinCall`] named `name`, reusing `operand`'s own span.
fn wrap_builtin(name: &str, operand: Expr) -> Expr {
    let span = operand.span().clone();
    Expr::BuiltinCall {
        name: name.to_string(),
        args: vec![operand],
        effects: EffectSet::PURE,
        span,
    }
}

/// The glyph for one of the six comparison [`ElementwiseOpKind`]s -- used
/// only to name the offending atom in the "no monadic form" error message.
/// J's own spellings (`~:`/`<:`/`>:` for ne/le/ge) differ from APL's.
fn glyph_for_comparison(op: ElementwiseOpKind) -> &'static str {
    match op {
        ElementwiseOpKind::Eq => "=",
        ElementwiseOpKind::Ne => "~:",
        ElementwiseOpKind::Lt => "<",
        ElementwiseOpKind::Le => "<:",
        ElementwiseOpKind::Ge => ">:",
        ElementwiseOpKind::Gt => ">",
        other => unreachable!("glyph_for_comparison called with non-comparison op {other:?}"),
    }
}

/// Convert one `NUMBER` token's lexeme text into an `Expr::IntLit` (a whole
/// number that fits `i64`) or `Expr::FloatLit` -- mirrors
/// `apl-to-semantic-ir::lower::number_literal_expr`'s own int-vs-float
/// convention exactly.
///
/// J's lexer spells a negative literal's sign with a leading underscore
/// (`_5`, `1.5E_3`) rather than APL's high-minus `¯` -- ASCII has no
/// dedicated glyph, and a bare `-` is already the `MINUS` verb token
/// (`j.tokens` SECTION 4). The underscore is translated to `-` first,
/// exactly as `j-runtime::eval::parse_j_number` already does
/// (`s.replace('_', "-")`), before either numeric parser ever sees the
/// text -- a global replace, so both a mantissa underscore and an exponent
/// underscore (`1.5E_3`) are handled in one pass.
///
/// Returns `Err` (rather than silently substituting `0.0`) if `raw_text`
/// fails to parse as a number at all -- unreachable via `compile_source`
/// (the lexer's own `NUMBER` rule guarantees a parseable lexeme), but
/// `compile` is also a public entry point over a hand-built
/// `GrammarASTNode`, so this matches every other malformed-input case in
/// this file, which is rejected explicitly rather than silently coerced.
fn number_literal_expr(raw_text: &str, span: &Span) -> Result<Expr, String> {
    let text = raw_text.replace('_', "-");
    let invalid = || format!("invalid number literal `{raw_text}`");
    if text.contains('.') || text.contains('e') || text.contains('E') {
        let value = text.parse::<f64>().map_err(|_| invalid())?;
        Ok(Expr::FloatLit { value, span: span.clone() })
    } else {
        match text.parse::<i64>() {
            Ok(v) => Ok(Expr::IntLit { value: v, span: span.clone() }),
            Err(_) => {
                let value = text.parse::<f64>().map_err(|_| invalid())?;
                Ok(Expr::FloatLit { value, span: span.clone() })
            }
        }
    }
}
