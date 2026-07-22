//! The lowering pass from `coding_adventures_q_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **v0.1.0**.
//!
//! # Q's CST shape (confirmed against `code/grammars/q/q.grammar` and the
//! tree-walk `q-runtime::eval` already does over it — structurally almost
//! identical to `j-to-semantic-ir`'s own J CST, per MA11 §3's "reused
//! UNCHANGED... this is the easy, mechanical part")
//!
//! ```text
//! program          = { line }
//! line             = statement NEWLINE | statement | NEWLINE
//! statement        = assignment                                -- passthrough
//! assignment       = NAME COLON assignment                     -- 3 children: chained/actual
//!                   | noun_expr                                 -- 1 child: base case
//! noun_expr        = term [ verb_expr noun_expr | noun_expr ]   -- 1, 2, or 3 children
//!                   | verb_expr noun_expr                        -- (2-child ambiguity resolved
//!                                                                    by inspecting kids[0].rule_name)
//! term             = NUMBER { NUMBER }
//!                   | NAME
//!                   | function_literal
//!                   | LPAREN noun_expr RPAREN
//!                   | list_literal
//! list_literal     = LPAREN noun_expr SEMICOLON noun_expr { SEMICOLON noun_expr } RPAREN
//! function_literal = LBRACE [ LBRACKET param_list RBRACKET ] stmt_seq RBRACE
//! param_list       = NAME { SEMICOLON NAME }
//! stmt_seq         = statement { SEMICOLON statement }
//! verb_expr        = verb_primitive [ EACH | REDUCE | SCAN ]
//!                   | NAME
//!                   | function_literal
//! verb_primitive   = one of the 17 primitive glyph tokens
//! ```
//!
//! # Scope
//!
//! **Supported** (every construct `q-parser`'s grammar can produce):
//! - Number literals, stranded literals (`1 2 3` → one rank-1
//!   [`Expr::ArrayLit`] wrapped in [`Expr::Ravel`] — the identical
//!   column-major-storage bug fix `j-to-semantic-ir`'s/`apl-to-semantic-ir`'s
//!   own `lower_term` already carries, see "Stranded literals" below),
//!   variables, parenthesised grouping.
//! - Assignment (`:`), including right-associative chained assignment
//!   (`a:b:3`) — see "Top-level scope is Global, not Local" below for why
//!   this crate's own convention genuinely differs from `j-to-semantic-ir`'s.
//! - The 12 scalar dyadic primitives shared with APL/J
//!   (`+ - * % & | = <> < <= >= >`), unconditionally lowered to
//!   [`Expr::ElementwiseOp`].
//! - The 6 of those 12 that have a monadic meaning in Q (`- % & | ~` is
//!   NOT the same 6 as J's — Q's own primitive table pairs each glyph with a
//!   genuinely different monadic meaning, MA11 §4) plus 5 bespoke primitives
//!   with no scalar-dyadic mapping at all (`! , # _ ~`) — see "The 17
//!   primitives" below for the complete table.
//! - `'` (each), `/` (reduce), `\` (scan) — same restriction and SIR nodes
//!   as APL/J: reduce/scan only apply to the 12 scalar-dyadic primitives;
//!   each degenerates to direct application for the primitives whose direct
//!   meaning is already elementwise, and is a clean error otherwise (MA11 §4
//!   / `q-runtime::builtins`'s own `each_monadic_supported`/
//!   `each_dyadic_supported`, mirrored exactly here).
//! - **List literals, dual syntax** (`1 2 3` stranding, `(a;b;c)` explicit)
//!   — both lower to the identical `Ravel(ArrayLit(..))` shape.
//! - **Function literals** (`{[x;y] ...}`, and the bracket-omitted implicit
//!   `x`/`y`/`z` form) — the one genuinely new lowering surface relative to
//!   `j-to-semantic-ir`'s own model; see "Function literals" below.
//! - Auto-print of a bare top-level noun expression (the same `"print"`
//!   [`Expr::BuiltinCall`] every SIR backend already implements).
//!
//! **Deliberately rejected** with a clean [`QLowerError`], mirroring what
//! `q-runtime::eval`/`q-runtime::builtins` reject at *runtime* (this
//! frontend catches the same things at *lowering* time instead, wherever
//! that is statically decidable — see "What stays genuinely dynamic" below
//! for the one case that is not):
//! - The 6 comparison primitives (`= <> < <= >= >`) used monadically.
//! - A reduce- or scan-decorated primitive used dyadically.
//! - `! , # _ ~` decorated with `/`/`\` — none of these 5 is "a scalar
//!   dyadic verb", mirroring `q-runtime::builtins::require_scalar_binop`
//!   exactly.
//! - Dyadic `!` (dict creation) — explicitly deferred, MA11 §4.
//! - A list literal containing a directly-syntactic function-literal or
//!   non-scalar (stranded/nested-list) element — `array_runtime::Array` has
//!   no boxed/heterogeneous representation (MA11 §4), mirroring
//!   `q-runtime::eval::eval_list_literal`'s identical rejection.
//! - A **nested** function-literal definition (one `{...}` appearing
//!   anywhere inside another's own parameter list or body) — MA11 §4
//!   explicitly puts this out of scope; mirrors
//!   `q-runtime::eval::Interpreter::build_lambda`'s identical
//!   `inside_a_call` rejection.
//! - A direct call (`f x` / `x f y`) to a function whose own declared
//!   parameter list is SHORTER than the call site's arity (1 for monadic,
//!   2 for dyadic) — mirrors `q-runtime::eval::Interpreter::call_lambda`'s
//!   "function takes at most N parameter(s)" rejection. See "Function
//!   literals" below for the (disclosed, narrower) direction this crate
//!   simplifies the opposite case.
//!
//! **Not applicable** (the grammar `q-parser` compiles literally cannot
//! produce these — booleans, symbols, strings, temporal literals,
//! dictionaries, tables, q-SQL, `?`/`.`/`@`, each-prior/each-right/
//! each-left, recursion — MA11 §4 — so there is no CST shape for this
//! lowerer to ever reach).
//!
//! # Stranded literals: the same `Ravel`-wrap fix APL/J already carry
//!
//! Identical bug, identical fix, to `apl-to-semantic-ir`'s/
//! `j-to-semantic-ir`'s own `lower_term`: a bare `ArrayLit { rows:
//! vec![row], .. }` is a genuinely rank-2 `[1, n]` value under this IR's
//! column-major storage convention (`Feature::ArrayColumnMajor`), not the
//! rank-1 `[n]` vector a stranded literal (or an explicit `(a;b;c)` list
//! literal, MA11 §3 bullet 3) actually is. Wrapping in [`Expr::Ravel`]
//! flattens any input rank down to a genuine rank-1 result — invisible to Q
//! source, only the IR shape changes.
//!
//! # Top-level scope is Global, not Local — a REAL divergence from J/APL
//!
//! `j-to-semantic-ir`/`apl-to-semantic-ir` lower every top-level name to a
//! `main`-local `Scope::Local` binding, because J/APL's entire program lives
//! inside the single synthesized `main` function and nothing else ever
//! needs to read a top-level name from outside it. Q genuinely breaks this
//! assumption: MA11 §2/§5 and `q-runtime::eval::Lambda`'s own doc comment
//! are explicit that a function literal's body "resolves any non-parameter
//! name against the *global* frame at call time" — meaning a Q function can
//! read (and this crate must therefore make visible to) a plain array
//! variable assigned at the top level, from inside a **separate**,
//! independently-compiled SIR [`Function`]. A `main`-local JS `let` is
//! invisible to a sibling JS function, so every top-level Q variable
//! becomes a genuine [`semantic_ir::Global`] (`init_function: "main"`,
//! `Scope::Global` everywhere it's read or written) instead — mirroring how
//! `semantic-ir-to-javascript::emit_globals` documents "globals are
//! module-level `let`s", visible from any function in the same file. Since
//! `emit_globals` pre-declares every global as `let name = null;` before any
//! function runs, this crate never needs the `LetStarBinding`-vs-`Assign`
//! first-occurrence distinction J/APL use for their own `Local` bindings —
//! every top-level write is simply `Stmt::Assign { scope: Global, .. }`,
//! first write or tenth.
//!
//! A **local** assignment made *inside* a function-literal body (MA11 §4:
//! "local to that call only") is a genuine `Scope::Local` (or `Scope::Param`
//! if it reassigns one of the function's own parameters), scoped to that
//! one synthesized [`Function`] alone — the ordinary `LetStarBinding`-vs-
//! `Assign` first-occurrence distinction applies there exactly as it does
//! in every sibling frontend.
//!
//! # Function literals: the one lowering surface with no APL/J precedent
//!
//! `{[x;y] stmt; stmt; ...}` (MA11 §2/§3 bullet 1) is the one genuinely new
//! *lowering* problem this crate's model (`j-to-semantic-ir`) never had to
//! solve, because J's/APL's in-scope grammars are expression-only — a train
//! *recombines* existing primitives, it never introduces a brand-new
//! parameter name a body can reference. `semantic_ir`'s own core already has
//! exactly the machinery general-purpose-language frontends (`python-to-
//! semantic-ir`, `ruby-to-semantic-ir`) use for this: a genuine
//! [`Function`] with named [`Param`]s, referenced by callers via
//! [`Expr::DirectCall`] (a statically-known callee), [`Expr::MakeClosure`]
//! (a bare reference to a named function used as a *value*, not called),
//! and [`Expr::IndirectCall`] (a call through a value whose identity is not
//! statically known — a closure handle). Since Q's own `QFn::Lambda` has
//! **no captures at all** (`q-runtime::eval::Lambda`'s own doc comment:
//! "stores no captured environment ... resolves any non-parameter name
//! against the *global* frame at call time"), this crate's design is
//! considerably SIMPLER than Python's/Ruby's own lambda-lifting machinery
//! (no free-variable analysis, no capture list ever populated) — every
//! synthesized [`Function`] always has `captures: vec![]`.
//!
//! ## Three call-site shapes, one dispatch decision
//!
//! Mirrors `q-runtime::eval`'s own "one dispatch site" framing (that
//! module's top doc comment: `apply_monadic`/`apply_dyadic` are the ONE
//! place every `QFn` variant is applied) at the *lowering* level instead of
//! the *evaluation* level:
//!
//! 1. **A NAME that resolves (at lowering time) to a function literal
//!    directly assigned to it at the top level** (`f:{x+y}` then `2 f 3`) —
//!    lowers to [`FnKind::KnownFn`], i.e. [`Expr::DirectCall`]. This is the
//!    common case and the one every function-chaining test in
//!    `q-runtime`'s own test suite exercises.
//! 2. **An inline function literal used immediately as a callee**
//!    (`{x*2} 5`) — synthesizes a fresh, anonymously-named top-level
//!    [`Function`] and *also* lowers to [`FnKind::KnownFn`]
//!    ([`Expr::DirectCall`] to the synthesized name) — semantically
//!    equivalent to "make a closure, then immediately call through it," but
//!    skipping the indirection since the callee is statically known the
//!    moment it's written.
//! 3. **Anything else** — a NAME that is a plain variable (a parameter, or
//!    a top-level array), or a parenthesised/list-literal/numeric term used
//!    as a callee — lowers to [`FnKind::Dynamic`], i.e. [`Expr::IndirectCall`]
//!    through whatever [`Expr`] that term evaluated to. This is what makes
//!    the genuinely dynamic, higher-order case work correctly with **no**
//!    special-casing: `q-runtime`'s own test suite has a real example
//!    (`passing_a_function_value_as_an_argument_to_another_function`:
//!    `apply:{[g] g 5}`, then `apply inc` calls `apply` with `inc` bound to
//!    `g`, and `g 5` inside `apply`'s body applies whatever `g` turns out to
//!    hold at runtime) — `g` is an ordinary [`Scope::Param`] [`Expr::VarRef`],
//!    dispatched via [`Expr::IndirectCall`], with **no static knowledge**
//!    of what it holds. `semantic-ir-to-javascript`'s own `applyClosure`
//!    runtime helper throws a clean `TypeError` if the value it receives
//!    isn't a genuine closure — the same "clean error, never silent
//!    misbehavior" class of failure `q-runtime::eval::as_callable` raises
//!    for the identical mistake (`applying_a_plain_array_value_as_a_function_is_a_clean_error`
//!    in that crate's own test suite), just discovered at a different
//!    (later, but still catchable) point.
//!
//! The dispatch decision itself is a single, shared helper
//! ([`Lowerer::expr_to_fnkind`]): lower the callee position the *ordinary*
//! way (exactly as if it were a ordinary value-producing `term`/`NAME`), and
//! then inspect whether the result is literally an [`Expr::MakeClosure`] —
//! if so, unwrap its `fn_name` into [`FnKind::KnownFn`] (the DirectCall
//! optimization); otherwise, keep the `Expr` as-is and dispatch dynamically.
//! `q-runtime`'s own grammar has no operator-precedence distinction between
//! `verb_expr`'s `NAME`/`function_literal` alternatives and `noun_expr`'s
//! "apply a bare `term`" fallback (MA11 §3's own header note on `q.grammar`)
//! — both alternatives are, at the *evaluation* level, resolved by the
//! identical `lookup` + `as_callable` pair. This crate's single shared
//! dispatch helper is the direct lowering-time mirror of that same real
//! unification, not an independent design choice.
//!
//! ## Disclosed simplification: declared arity vs. call-site arity
//!
//! `q-runtime::eval::Interpreter::call_lambda` binds a call's arguments to
//! a function's parameters **positionally**, and does NOT require every
//! declared parameter to receive an argument — a function declared with
//! more parameters than a given call site supplies (most commonly the
//! bracket-omitted implicit `x`/`y`/`z` form, called monadically with only
//! one argument) simply leaves the extra parameters **unbound** for that
//! call, erroring only if the body actually references one of them
//! (confirmed directly against `q-parser`'s own doc-comment example, `f 2
//! 3` calling a `{[x;y] ...}`-shaped `f` **monadically** with the single
//! two-element vector argument `2 3`, since adjacent NUMBER stranding
//! always wins over the dyadic-application alternative at that grammar
//! position — leaving `y` unbound for that call). `semantic_ir::Function`'s
//! own default-parameter model (SIR10) has no "declared but deliberately
//! left unbound, error only if referenced" concept — a parameter is either
//! required (arity floor) or has a real fallback *value*. This crate
//! resolves the mismatch by giving every parameter **after the first** a
//! default of `IntLit(0)` (arbitrary, always-valid-as-a-scalar-array
//! sentinel — never `Feature`-gated beyond `Feature::DefaultParams`), so a
//! call supplying fewer arguments than the callee declares is accepted (the
//! trailing parameters silently take the value `0` rather than truly
//! staying unbound) instead of rejected. This is a genuine, disclosed
//! behavioral divergence from `q-runtime`'s own semantics for the narrow
//! case where the body actually *reads* an unsupplied trailing parameter
//! (a real Q program doing this is arguably already a bug in the source,
//! since real q-runtime errors on it too) — but it is what lets the
//! overwhelmingly common, well-formed case (a function declaring exactly
//! the parameters its own body uses, called with matching arity) compile
//! and run correctly, rather than rejecting every use of the implicit
//! `x`/`y`/`z` convenience called at anything other than full ternary
//! arity. The OTHER direction — a call site supplying *more* arguments than
//! the callee declares — is still a hard, disclosed lowering error (see
//! [`Lowerer::apply_monadic`]/[`Lowerer::apply_dyadic`]'s `KnownFn` arm),
//! mirroring `call_lambda`'s own "function takes at most N parameter(s)"
//! rejection exactly, since there is no SIR concept of "extra argument,
//! silently ignored" to fall back on either.
//!
//! # The 17 primitives (MA11 §4's full table)
//!
//! | Glyph | Monadic | Dyadic |
//! |---|---|---|
//! | `+` | flip → **identity** (this cut can never construct a rank-2 value at all — no reshape/matrix-literal primitive exists in scope, MA11 §4 — so "transpose if rank 2, else identity" reduces to unconditional identity for every value this frontend can ever produce) | add → `ElementwiseOp(Add)` |
//! | `-` | negate → `BuiltinCall("neg")` (the exact same builtin `apl-to-semantic-ir`'s monadic `-` already uses) | subtract → `ElementwiseOp(Sub)` |
//! | `*` | first → `BuiltinCall("q_first")` (new — Q's "first item," genuinely unlike J's monadic `*` = sign) | multiply → `ElementwiseOp(Mul)` |
//! | `%` | reciprocal → `BuiltinCall("recip")` (reused from `apl-to-semantic-ir`) | divide → `ElementwiseOp(Div)` |
//! | `!` | til (0-based) → `Expr::IndexGenerator` + a `- 1` correction (the identical fix `j-to-semantic-ir::zero_base_index` already applies for J's own 0-based `i.`, since the shared JS runtime's `indexGenerator` hardcodes APL's 1-based convention) | dyadic `!` (dict creation) — **explicitly deferred, MA11 §4**: a clean lowering error, never attempted |
//! | `,` | enlist → `Expr::Ravel` (see "Enlist reuses Ravel" below for why these coincide exactly in this cut) | join → `Expr::Catenate` (see "Join reuses Catenate" below) |
//! | `#` | tally → `BuiltinCall("tally")` (reused **as-is** from `j-to-semantic-ir` — `ArrayRt.tally`'s existing rank0→1/rank1→n/rank2→r convention already matches `q-runtime::builtins::tally` exactly) | take → `BuiltinCall("q_take")` (new — Q's `#` is *take*, unlike J's dyadic `#` = *replicate*) |
//! | `_` | floor → `BuiltinCall("floor")` (reused from `apl-to-semantic-ir`) | drop → `BuiltinCall("q_drop")` (new) |
//! | `&` | where → `BuiltinCall("q_where")` (new — indices of nonzero elements) | min → `ElementwiseOp(Min)` |
//! | `\|` | reverse → `BuiltinCall("q_reverse")` (new) | max → `ElementwiseOp(Max)` |
//! | `~` | not → `BuiltinCall("q_not")` (new — elementwise 0/1, NOT the generic boolean `"not"` builtin, which returns a native JS boolean and isn't elementwise) | match → `BuiltinCall("q_match")` (new — deep equality, producing a scalar, the one dyadic primitive here that is NOT elementwise) |
//! | `=` `<>` `<` `<=` `>=` `>` | none (clean error — dyadic-only) | `ElementwiseOp(Eq/Ne/Lt/Le/Ge/Gt)` |
//!
//! ## Enlist reuses `Ravel`
//!
//! Real Q's monadic `,` (enlist) and APL/J's monadic `,` (ravel) are
//! genuinely different operations in general (enlist adds ONE level of list
//! nesting even to an already-list-shaped argument; ravel flattens to rank
//! 1 unconditionally) — but this cut's value model
//! (`array_runtime::Array`, MA11 §4: "arrays only, dense and numeric," no
//! nested/boxed representation) can only ever hold rank 0 or rank 1 values
//! (no primitive in scope can construct rank 2), and on exactly that
//! reachable subset the two operations coincide completely: a rank-0
//! scalar becomes a rank-1 single-element vector under EITHER reading, and
//! a rank-1 vector is returned unchanged under EITHER reading. Confirmed
//! directly against `q_runtime::builtins::enlist`'s own doc comment, which
//! discloses the identical simplification on the *runtime* side ("the
//! closest sound approximation available within this value model, and is
//! exact for enlist's single most common use"). This crate reuses
//! [`Expr::Ravel`] rather than inventing a redundant `BuiltinCall` that
//! would compute the identical result through a second code path.
//!
//! ## Join reuses `Catenate`
//!
//! `q_runtime::builtins::join`'s own case-by-case shape (scalar-scalar →
//! 2-vector; scalar-vector/vector-scalar → prepend/append; vector-vector →
//! concatenate; matrix-matrix with equal row counts → column-join) is,
//! case for case, IDENTICAL to `apl_runtime::builtins::catenate`'s (the
//! kernel `Expr::Catenate`'s shared JS runtime implementation already
//! computes) — confirmed by direct side-by-side comparison of both Rust
//! source functions. This crate reuses [`Expr::Catenate`] rather than
//! introducing a `BuiltinCall("q_join", ..)` that would just re-implement
//! the identical logic a second time.
//!
//! # Recursion-depth guard
//!
//! [`MAX_EXPR_DEPTH`] (256) bounds this crate's own lowering recursion —
//! defense in depth, exactly mirroring `j-to-semantic-ir::MAX_EXPR_DEPTH`'s/
//! `apl-to-semantic-ir::MAX_EXPR_DEPTH`'s identical rationale:
//! `q-parser`'s own `MAX_RULE_DEPTH` (32) already bounds how deep a CST
//! built from untrusted source can possibly be, so this can never actually
//! trip on a tree from `try_parse_q`; it exists purely so a hand-built
//! `GrammarASTNode` (or a future change to `q-parser`'s own cap) can't turn
//! a deep-but-technically-parseable input into an uncatchable native stack
//! overflow while walking it here. Unlike J's own trains, Q has no
//! combinator-shaped construct that duplicates an operand in the emitted
//! tree (MA11 §4: no trains, no `@` compose) — so, unlike
//! `j-to-semantic-ir`, this crate needs no *second*, smaller
//! combinator-depth guard.

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, ElementwiseOpKind, Expr, Feature, FeatureManifest, Function, Global,
    Metadata, Module, Param, ParamKind, Scope, Span, Stmt,
};
use std::collections::{HashMap, HashSet};

/// Maximum ordinary CST-walk recursion depth. See this file's module doc
/// comment's "Recursion-depth guard" section.
const MAX_EXPR_DEPTH: usize = 256;

/// Synthetic file name used for all spans (the CST does not carry the
/// original path).
const FILE: &str = "<q>";

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// An error encountered during Q → SIR lowering.
///
/// Mirrors `JLowerError`/`AplLowerError`'s shape exactly (a `message` plus
/// 1-based `line`/`column`) so tooling can treat every SIR frontend
/// uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for QLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "QLowerError at {}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for QLowerError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed Q CST (rooted at the `program` rule) into a SIR module.
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<semantic_ir::Module, QLowerError> {
    Lowerer::new(module_name).lower_file(tree)
}

// ---------------------------------------------------------------------------
// Primitive-verb representation (MA11 §4's full table)
// ---------------------------------------------------------------------------

/// One of `q.tokens`' 17 primitive verb glyphs, kept as a small `Copy` enum
/// (not the raw token) so error messages can name the actual glyph (`"!"`,
/// not `"BANG"`) — mirrors `q_runtime::builtins::Prim` field-for-field
/// (this crate does not depend on `q-runtime` in its non-dev dependencies,
/// so the enum is redefined here rather than imported, exactly as
/// `j-to-semantic-ir::NonScalarAtom` redefines its own analogue of
/// `j_runtime::eval::NonScalarAtom` rather than importing it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Prim {
    Plus,
    Minus,
    Star,
    Percent,
    Bang,
    Comma,
    Hash,
    Underscore,
    Amp,
    Pipe,
    Tilde,
    Eq,
    Ne,
    Lt,
    Le,
    Ge,
    Gt,
}

impl Prim {
    /// The ASCII spelling of this glyph, for error messages. `Ne` spells
    /// `<>` — Q's own not-equal (MA11 §4), never `~=`/`#`.
    fn glyph(self) -> &'static str {
        match self {
            Prim::Plus => "+",
            Prim::Minus => "-",
            Prim::Star => "*",
            Prim::Percent => "%",
            Prim::Bang => "!",
            Prim::Comma => ",",
            Prim::Hash => "#",
            Prim::Underscore => "_",
            Prim::Amp => "&",
            Prim::Pipe => "|",
            Prim::Tilde => "~",
            Prim::Eq => "=",
            Prim::Ne => "<>",
            Prim::Lt => "<",
            Prim::Le => "<=",
            Prim::Ge => ">=",
            Prim::Gt => ">",
        }
    }

    /// The [`ElementwiseOpKind`] this primitive's **dyadic** meaning maps
    /// onto, if any. Exactly 12 of the 17 map onto one variant apiece; the
    /// remaining 5 (`! , # _ ~`) have no elementwise-scalar dyadic meaning
    /// at all — mirrors `q_runtime::builtins::Prim::to_binop` exactly.
    fn to_binop(self) -> Option<ElementwiseOpKind> {
        match self {
            Prim::Plus => Some(ElementwiseOpKind::Add),
            Prim::Minus => Some(ElementwiseOpKind::Sub),
            Prim::Star => Some(ElementwiseOpKind::Mul),
            Prim::Percent => Some(ElementwiseOpKind::Div),
            Prim::Amp => Some(ElementwiseOpKind::Min),
            Prim::Pipe => Some(ElementwiseOpKind::Max),
            Prim::Eq => Some(ElementwiseOpKind::Eq),
            Prim::Ne => Some(ElementwiseOpKind::Ne),
            Prim::Lt => Some(ElementwiseOpKind::Lt),
            Prim::Le => Some(ElementwiseOpKind::Le),
            Prim::Ge => Some(ElementwiseOpKind::Ge),
            Prim::Gt => Some(ElementwiseOpKind::Gt),
            Prim::Bang | Prim::Comma | Prim::Hash | Prim::Underscore | Prim::Tilde => None,
        }
    }

    /// Whether monadic `f'x` (each) has a well-defined, non-redundant
    /// meaning for this primitive — true only for the four primitives whose
    /// *monadic* meaning is itself an ordinary per-element scalar map.
    /// Mirrors `q_runtime::builtins::Prim::each_monadic_supported` exactly.
    fn each_monadic_supported(self) -> bool {
        matches!(self, Prim::Minus | Prim::Percent | Prim::Underscore | Prim::Tilde)
    }

    /// Whether dyadic `x f'y` (each) has a well-defined, non-redundant
    /// meaning — true exactly for the `ElementwiseOpKind`-mappable
    /// primitives. Mirrors `q_runtime::builtins::Prim::each_dyadic_supported`
    /// exactly.
    fn each_dyadic_supported(self) -> bool {
        self.to_binop().is_some()
    }
}

// ---------------------------------------------------------------------------
// Verb-expression / callee representation
// ---------------------------------------------------------------------------

/// This lowerer's own representation of "which verb, and with which adverb
/// (if any) applied, or which callable value" — the direct lowering-time
/// analogue of `q_runtime::eval::QFn`. Unlike `j-to-semantic-ir::FnKind`,
/// there is no `Compose`/`Hook`/`Fork` here at all (Q has no trains and no
/// `@` compose, MA11 §3/§4) — every variant is a leaf dispatch.
enum FnKind {
    /// A bare primitive glyph, applied directly.
    Prim(Prim),
    /// A primitive with `'` (each) applied.
    Each(Prim),
    /// A primitive with `/` (reduce) applied — inherently monadic.
    Reduce(ElementwiseOpKind),
    /// A primitive with `\` (scan) applied — also monadic.
    Scan(ElementwiseOpKind),
    /// A statically-known top-level function (by its SIR name) — lowers to
    /// [`Expr::DirectCall`]. See this file's module doc comment's "Three
    /// call-site shapes" section.
    KnownFn(String),
    /// Anything else: a callable whose identity is not statically known —
    /// lowers to [`Expr::IndirectCall`] through this already-lowered
    /// [`Expr`]. See this file's module doc comment's "Three call-site
    /// shapes" section, case 3.
    Dynamic(Box<Expr>),
}

// ---------------------------------------------------------------------------
// Name resolution
// ---------------------------------------------------------------------------

/// What a bare `NAME` resolves to, once looked up against the current
/// function's own parameters/locals and then the top-level bindings table
/// (see [`Lowerer::resolve_name`]).
enum Resolved {
    /// A statically-known top-level function — its own SIR name.
    Function(String),
    /// An ordinary value binding, at the given [`Scope`].
    Var(Scope),
}

/// The local-name scope for whichever [`Function`] is currently being
/// lowered. The top level (`main`) uses an always-empty [`FnScope`] (MA11's
/// own convention has no true *local* binding at the top level — every
/// top-level name is a [`Scope::Global`], see this file's module doc
/// comment's "Top-level scope is Global, not Local" section); a
/// function-literal body's own [`FnScope`] is seeded with its parameter
/// names and grows its `locals` set as the body's own statements assign new
/// names, mirroring `q_runtime::eval::Interpreter`'s "local to that call
/// only" scoping (MA11 §4).
struct FnScope {
    params: HashSet<String>,
    locals: HashSet<String>,
}

impl FnScope {
    fn top_level() -> Self {
        FnScope { params: HashSet::new(), locals: HashSet::new() }
    }

    fn for_lambda(params: &[String]) -> Self {
        FnScope { params: params.iter().cloned().collect(), locals: HashSet::new() }
    }
}

/// What an `assignment` chain's base case resolved to — either an ordinary
/// value (the ordinary case) or a bare top-level function-literal
/// definition (see [`Lowerer::bare_function_literal_term`] and this file's
/// module doc comment's "Function literals" section).
enum AssignedValue {
    Value(Expr),
    Function(String),
}

// ---------------------------------------------------------------------------
// The lowerer
// ---------------------------------------------------------------------------

/// Q scopes a plain array variable to the *whole program* if assigned at
/// the top level (a genuine [`Scope::Global`], see this file's module doc
/// comment), or to one function call if assigned inside a function-literal
/// body (a [`Scope::Local`]/[`Scope::Param`], MA11 §4's "local to that call
/// only"). A user-defined function name is tracked separately
/// (`known_functions`) regardless of where it was defined, since MA11 §4
/// only ever allows a function literal to be *directly* assigned at the
/// top level (no nested function-literal definitions at all).
struct Lowerer {
    module_name: String,
    observed: FeatureManifest,
    /// Q source name → the synthesized SIR [`Function`]'s own name, for
    /// every name a bare function literal has been directly assigned to at
    /// the top level.
    known_functions: HashMap<String, String>,
    /// SIR function name → its own declared parameter count, used for the
    /// "too many arguments" arity check (see this file's module doc
    /// comment's "Disclosed simplification" section).
    known_function_arity: HashMap<String, usize>,
    /// Every top-level Q name currently bound to a plain (non-function)
    /// value — becomes a [`Global`] module entry. A `HashSet` for O(1)
    /// membership plus `global_order` (below) for deterministic output
    /// order; a name that is later reassigned to a function literal is
    /// removed from this set (see [`Lowerer::lower_assignment_chain`]).
    global_names: HashSet<String>,
    /// Insertion order for `global_names`, so the emitted `Module.globals`
    /// list is deterministic regardless of `HashSet`'s own iteration order.
    /// Filtered against `global_names` at finalization time, so a name
    /// later promoted to a function does not leave a stale entry.
    global_order: Vec<String>,
    /// Every synthesized function-literal [`Function`] (never `main`
    /// itself, appended separately in [`Lowerer::lower_file`]).
    functions: Vec<Function>,
    /// Monotonically increasing counter for synthesized function names
    /// (`q_lambda_<N>`) — every function literal gets one, regardless of
    /// whether it came from a direct top-level assignment or an inline,
    /// immediately-applied literal, so there is never a naming collision to
    /// resolve.
    lambda_counter: usize,
    /// Nesting depth of function-literal-body lowering currently in
    /// progress: `0` at the top level, `1` while lowering the *first*
    /// function literal's own body, etc. Used by
    /// [`Lowerer::register_function_literal`] to reject a **nested**
    /// function-literal definition (MA11 §4), mirroring
    /// `q_runtime::eval::Interpreter::inside_a_call`'s identical guard.
    inside_lambda_depth: usize,
}

impl Lowerer {
    fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
            observed: FeatureManifest::new(),
            known_functions: HashMap::new(),
            known_function_arity: HashMap::new(),
            global_names: HashSet::new(),
            global_order: Vec::new(),
            functions: Vec::new(),
            lambda_counter: 0,
            inside_lambda_depth: 0,
        }
    }

    // -------------------------------------------------------------------
    // top level: `program` → one `main` function (+ synthesized lambdas)
    // -------------------------------------------------------------------

    fn lower_file(&mut self, program: &GrammarASTNode) -> Result<Module, QLowerError> {
        if program.rule_name != "program" {
            return Err(self.err_at(
                program,
                format!("expected `program` root, got `{}`", program.rule_name),
            ));
        }

        self.observed.add(Feature::DynamicTyping);

        let mut top_scope = FnScope::top_level();
        let mut stmts: Vec<Stmt> = Vec::new();
        for line in child_nodes(program) {
            if line.rule_name != "line" {
                continue;
            }
            // A `line` with no `statement` child (a blank line, or a
            // comment-only line -- `/`-comments are already stripped by
            // `q-lexer`'s pre-tokenize hook) is a bare NEWLINE production;
            // skip it, don't error.
            let Some(stmt_node) = first_child_named(line, "statement") else {
                continue;
            };
            let assignment_node = only_node(stmt_node)
                .ok_or_else(|| self.err_at(stmt_node, "malformed statement".to_string()))?;
            let mut new_stmts =
                self.lower_top_level_statement(assignment_node, 0, &mut top_scope)?;
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

        // Filtered against the CURRENT `global_names` set (not just
        // insertion order) so a name later promoted from a plain variable
        // to a function (re-assigned to a bare function literal after an
        // earlier plain-value assignment) does not leave a stale `Global`
        // entry behind -- see `lower_assignment_chain`'s `Function` arm.
        let globals: Vec<Global> = self
            .global_order
            .iter()
            .filter(|name| self.global_names.contains(*name))
            .map(|name| Global {
                name: name.clone(),
                sir_type: None,
                init_function: "main".to_string(),
                span: span.clone(),
            })
            .collect();

        let mut functions = std::mem::take(&mut self.functions);
        functions.push(main);

        let metadata = Metadata::new()
            .with_source_language("q")
            .with_sir_version(semantic_ir::CURRENT_SIR_VERSION);

        Ok(Module {
            name: self.module_name.clone(),
            manifest: self.observed.clone(),
            imports: vec![],
            exports: vec![],
            functions,
            globals,
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
        top_scope: &mut FnScope,
    ) -> Result<Vec<Stmt>, QLowerError> {
        self.check_depth(node, depth)?;
        match node.children.len() {
            // Base case: a bare `noun_expr`, not an assignment. Real Q
            // auto-print session semantics (MA11 §4, mirroring
            // `j-to-semantic-ir`'s/`apl-to-semantic-ir`'s identical
            // convention).
            1 => {
                let noun_expr_node = only_node(node)
                    .ok_or_else(|| self.err_at(node, "malformed noun_expr statement".to_string()))?;
                let v = self.lower_noun_expr(noun_expr_node, depth + 1, top_scope)?;
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
            // `NAME COLON assignment` -- an actual assignment (possibly
            // chained). Assignment is silent (MA11 §4): emit every
            // statement the chain unrolled into, and nothing else.
            3 => {
                let (stmts, _final_value) =
                    self.lower_assignment_chain(node, depth, top_scope, true)?;
                Ok(stmts)
            }
            n => Err(self.err_at(node, format!("malformed assignment with {n} children"))),
        }
    }

    // -------------------------------------------------------------------
    // assignment (including chained assignment) -- shared by top-level
    // (Global scope) and function-literal-body-internal (Local/Param
    // scope) lowering, see this file's module doc comment's "Top-level
    // scope is Global, not Local" section.
    // -------------------------------------------------------------------

    /// Recursively lower an `assignment` node. Returns the statements the
    /// chain unrolled into (in dependency order) and what the chain's
    /// innermost RHS resolved to -- either an ordinary value, or (MA11 §3
    /// bullet 1) a bare function-literal definition, in which case NO
    /// `Stmt` is emitted for it at all (the function becomes its own
    /// top-level [`Function`], tracked in `known_functions` -- not an
    /// ordinary value binding).
    ///
    /// `is_top_level` selects `Scope::Global` (every top-level write,
    /// first occurrence or tenth -- no `LetStarBinding`-vs-`Assign`
    /// distinction is needed there, see the module doc comment) vs. the
    /// ordinary `Scope::Param`/`Scope::Local` first-occurrence convention
    /// every sibling frontend uses for its own function-body-local
    /// bindings.
    fn lower_assignment_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        fn_scope: &mut FnScope,
        is_top_level: bool,
    ) -> Result<(Vec<Stmt>, AssignedValue), QLowerError> {
        self.check_depth(node, depth)?;
        match node.children.len() {
            1 => {
                let noun_expr_node = only_node(node).ok_or_else(|| {
                    self.err_at(node, "malformed noun_expr in assignment".to_string())
                })?;
                if let Some(lit_node) = bare_function_literal_term(noun_expr_node) {
                    let sir_name = self.register_function_literal(lit_node, depth + 1)?;
                    return Ok((vec![], AssignedValue::Function(sir_name)));
                }
                let v = self.lower_noun_expr(noun_expr_node, depth + 1, fn_scope)?;
                Ok((vec![], AssignedValue::Value(v)))
            }
            3 => {
                let name = self.assignment_target_name(node)?;
                let inner = only_node(node).ok_or_else(|| {
                    self.err_at(node, "malformed assignment: no nested assignment".to_string())
                })?;
                let (mut stmts, inner_value) =
                    self.lower_assignment_chain(inner, depth + 1, fn_scope, is_top_level)?;
                let span = self.span_of(node);
                match inner_value {
                    AssignedValue::Function(sir_name) => {
                        // Promoting `name` to a function: if it was
                        // previously a plain global, drop it from that set
                        // so `lower_file` doesn't emit a stale `Global`.
                        self.global_names.remove(&name);
                        self.known_functions.insert(name.clone(), sir_name.clone());
                        Ok((stmts, AssignedValue::Function(sir_name)))
                    }
                    AssignedValue::Value(value) => {
                        // Demoting `name` to a plain value: if it was
                        // previously a known function, it no longer is.
                        self.known_functions.remove(&name);
                        if is_top_level {
                            if self.global_names.insert(name.clone()) {
                                self.global_order.push(name.clone());
                            }
                            self.observed.add(Feature::Globals);
                            self.observed.add(Feature::MutableBindings);
                            stmts.push(Stmt::Assign {
                                name: name.clone(),
                                scope: Scope::Global,
                                value,
                                span: span.clone(),
                            });
                            Ok((stmts, AssignedValue::Value(Expr::VarRef {
                                name,
                                scope: Scope::Global,
                                span,
                            })))
                        } else if fn_scope.params.contains(&name) {
                            // Reassigning one of the function's own
                            // parameters -- still a plain `Assign`, never a
                            // fresh binding (mirrors
                            // `q_runtime::eval::Interpreter::assign`'s
                            // "always write to the top frame" rule, where
                            // a param and a call-local variable share one
                            // frame).
                            self.observed.add(Feature::MutableBindings);
                            stmts.push(Stmt::Assign {
                                name: name.clone(),
                                scope: Scope::Param,
                                value,
                                span: span.clone(),
                            });
                            Ok((stmts, AssignedValue::Value(Expr::VarRef {
                                name,
                                scope: Scope::Param,
                                span,
                            })))
                        } else if fn_scope.locals.insert(name.clone()) {
                            stmts.push(Stmt::LetStarBinding {
                                name: name.clone(),
                                sir_type: None,
                                value,
                                span: span.clone(),
                            });
                            Ok((stmts, AssignedValue::Value(Expr::VarRef {
                                name,
                                scope: Scope::Local,
                                span,
                            })))
                        } else {
                            self.observed.add(Feature::MutableBindings);
                            stmts.push(Stmt::Assign {
                                name: name.clone(),
                                scope: Scope::Local,
                                value,
                                span: span.clone(),
                            });
                            Ok((stmts, AssignedValue::Value(Expr::VarRef {
                                name,
                                scope: Scope::Local,
                                span,
                            })))
                        }
                    }
                }
            }
            n => Err(self.err_at(node, format!("malformed assignment with {n} children"))),
        }
    }

    /// The `NAME` token of an actual assignment's target -- the first child
    /// of a 3-child `assignment` node.
    fn assignment_target_name(&self, node: &GrammarASTNode) -> Result<String, QLowerError> {
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

    /// `noun_expr = term [ verb_expr noun_expr | noun_expr ] | verb_expr noun_expr`.
    ///
    /// - 1 child `[term]` -- a bare term.
    /// - 3 children `[term, verb_expr, noun_expr]` -- ordinary dyadic
    ///   application.
    /// - 2 children -- ambiguous by count alone (the one wrinkle
    ///   `q.grammar` has that `j.grammar`/`apl.grammar` never needed, see
    ///   this file's module doc comment): `[verb_expr, noun_expr]` is
    ///   ordinary monadic primitive application (`-5`); `[term, noun_expr]`
    ///   is "apply a callable term" (`f 5`, or `{x*2} 5`). Disambiguated by
    ///   inspecting `kids[0].rule_name` -- exactly the check
    ///   `q_runtime::eval::Interpreter::eval_noun_expr` makes.
    fn lower_noun_expr(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        fn_scope: &FnScope,
    ) -> Result<Expr, QLowerError> {
        self.check_depth(node, depth)?;
        let span = self.span_of(node);
        let kids = child_nodes(node);
        match kids.as_slice() {
            [term] => self.lower_term(term, depth + 1, fn_scope),
            [first, sub] if first.rule_name == "verb_expr" => {
                let f = self.lower_verb_expr(first, depth + 1, fn_scope)?;
                let arg = self.lower_noun_expr(sub, depth + 1, fn_scope)?;
                self.apply_monadic(f, arg, span)
            }
            [first, sub] if first.rule_name == "term" => {
                let callee_expr = self.lower_term(first, depth + 1, fn_scope)?;
                let f = self.expr_to_fnkind(callee_expr);
                let arg = self.lower_noun_expr(sub, depth + 1, fn_scope)?;
                self.apply_monadic(f, arg, span)
            }
            [lhs_term, vexpr, sub] => {
                let lhs = self.lower_term(lhs_term, depth + 1, fn_scope)?;
                let f = self.lower_verb_expr(vexpr, depth + 1, fn_scope)?;
                let rhs = self.lower_noun_expr(sub, depth + 1, fn_scope)?;
                self.apply_dyadic(f, lhs, rhs, span)
            }
            other => Err(self.err_at(
                node,
                format!("malformed noun_expr with {} children", other.len()),
            )),
        }
    }

    /// `term = NUMBER { NUMBER } | NAME | function_literal | LPAREN noun_expr RPAREN | list_literal`.
    fn lower_term(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        fn_scope: &FnScope,
    ) -> Result<Expr, QLowerError> {
        self.check_depth(node, depth)?;
        match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NUMBER" => {
                // Stranding: one or more juxtaposed NUMBER tokens form a
                // single term (MA11 §4, inherited unchanged from APL/J).
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
                    // Same `Ravel`-wrap fix `j-to-semantic-ir`'s/
                    // `apl-to-semantic-ir`'s own `lower_term` carries --
                    // see this file's module doc comment's "Stranded
                    // literals" section.
                    self.observed.add(Feature::NDArrays);
                    self.observed.add(Feature::ArrayColumnMajor);
                    self.observed.add(Feature::MatrixOps);
                    let span = self.span_of(node);
                    let row: Vec<Expr> = numbers
                        .iter()
                        .map(|tok| self.number_literal(tok))
                        .collect::<Result<Vec<_>, _>>()?;
                    let array_lit = Expr::ArrayLit { rows: vec![row], span: span.clone() };
                    Ok(Expr::Ravel { target: Box::new(array_lit), span })
                }
            }
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "NAME" => {
                self.lower_name(t, node, fn_scope)
            }
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "function_literal" => {
                let sir_name = self.register_function_literal(n, depth + 1)?;
                let span = self.span_of(node);
                Ok(Expr::MakeClosure { fn_name: sir_name, captures: vec![], span })
            }
            Some(ASTNodeOrToken::Token(t)) if t.effective_type_name() == "LPAREN" => {
                let inner = only_node(node)
                    .ok_or_else(|| self.err_at(node, "malformed parenthesised term".to_string()))?;
                self.lower_noun_expr(inner, depth + 1, fn_scope)
            }
            Some(ASTNodeOrToken::Node(n)) if n.rule_name == "list_literal" => {
                self.lower_list_literal(n, depth + 1, fn_scope)
            }
            _ => Err(self.err_at(node, "malformed term".to_string())),
        }
    }

    /// Resolve a bare `NAME` token against `fn_scope` (params, then
    /// locals), then the top-level bindings tables (a known function, then
    /// a plain global), erroring cleanly if the name was never bound.
    /// Shared by [`Lowerer::lower_term`]'s `NAME` alternative and
    /// [`Lowerer::lower_verb_expr`]'s `NAME` alternative -- exactly the
    /// unification this file's module doc comment's "Three call-site
    /// shapes" section describes.
    fn lower_name(
        &mut self,
        tok: &Token,
        node: &GrammarASTNode,
        fn_scope: &FnScope,
    ) -> Result<Expr, QLowerError> {
        let span = Span::point(FILE, tok.line, tok.column);
        match self.resolve_name(&tok.value, fn_scope) {
            Some(Resolved::Function(sir_name)) => {
                self.observed.add(Feature::Closures);
                Ok(Expr::MakeClosure { fn_name: sir_name, captures: vec![], span })
            }
            Some(Resolved::Var(scope)) => {
                Ok(Expr::VarRef { name: tok.value.clone(), scope, span })
            }
            None => Err(self.err_at(node, format!("undefined variable '{}'", tok.value))),
        }
    }

    /// The actual 3-tier lookup [`Lowerer::lower_name`] uses: the current
    /// function's own parameters, then its own locals, then a top-level
    /// known function, then a top-level plain global.
    fn resolve_name(&self, name: &str, fn_scope: &FnScope) -> Option<Resolved> {
        if fn_scope.params.contains(name) {
            return Some(Resolved::Var(Scope::Param));
        }
        if fn_scope.locals.contains(name) {
            return Some(Resolved::Var(Scope::Local));
        }
        if let Some(sir_name) = self.known_functions.get(name) {
            return Some(Resolved::Function(sir_name.clone()));
        }
        if self.global_names.contains(name) {
            return Some(Resolved::Var(Scope::Global));
        }
        None
    }

    /// Convert an already-lowered value [`Expr`] into a [`FnKind`] callee:
    /// a literal [`Expr::MakeClosure`] is a statically-known function
    /// ([`FnKind::KnownFn`], a `DirectCall` optimization); anything else is
    /// dispatched dynamically ([`FnKind::Dynamic`]). See this file's module
    /// doc comment's "Three call-site shapes" section.
    fn expr_to_fnkind(&self, expr: Expr) -> FnKind {
        match expr {
            Expr::MakeClosure { fn_name, .. } => FnKind::KnownFn(fn_name),
            other => FnKind::Dynamic(Box::new(other)),
        }
    }

    /// `list_literal = LPAREN noun_expr SEMICOLON noun_expr { SEMICOLON noun_expr } RPAREN`
    /// (MA11 §3 bullet 3 / §4). Lowers to the identical shape stranding
    /// produces (`Ravel(ArrayLit([elements]))`) -- MA11 §3 bullet 3: "both
    /// lower to the same list value."
    ///
    /// Rejects (cleanly, at lowering time) any element that is
    /// *syntactically, provably* non-scalar or function-valued -- a
    /// directly-nested list/stranded literal always lowers to
    /// `Expr::Ravel` (which, in this cut, always denotes a genuinely
    /// non-scalar rank-1 result -- see this file's module doc comment's
    /// "Enlist reuses Ravel" section), and a directly-nested function
    /// reference always lowers to `Expr::MakeClosure` -- mirroring
    /// `q_runtime::eval::Interpreter::eval_list_literal`'s identical
    /// rejection of a `QValue::Arr` with rank >= 1 or a `QValue::Fn`
    /// element. An element that is merely some OTHER arbitrary expression
    /// (e.g. a variable reference that might, at runtime, hold a
    /// non-scalar or function value) is not statically decidable here any
    /// more than it is in `q-runtime` itself, so it is not rejected -- it
    /// is simply embedded as-is, exactly like every other array-domain
    /// frontend's `ArrayLit` element.
    fn lower_list_literal(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        fn_scope: &FnScope,
    ) -> Result<Expr, QLowerError> {
        self.check_depth(node, depth)?;
        let elems = child_nodes(node);
        let span = self.span_of(node);
        let mut row: Vec<Expr> = Vec::with_capacity(elems.len());
        for e in elems {
            let v = self.lower_noun_expr(e, depth + 1, fn_scope)?;
            match &v {
                Expr::MakeClosure { .. } => {
                    return Err(self.err_at(
                        e,
                        "a list literal containing a function-valued element has no \
                         representation in this cut's dense-numeric-only value model (MA11 §4)"
                            .to_string(),
                    ));
                }
                Expr::Ravel { .. } => {
                    return Err(self.err_at(
                        e,
                        "a list literal with a non-scalar element has no representation in \
                         this cut's dense-numeric-only value model (MA11 §4)"
                            .to_string(),
                    ));
                }
                _ => {}
            }
            row.push(v);
        }
        self.observed.add(Feature::NDArrays);
        self.observed.add(Feature::ArrayColumnMajor);
        self.observed.add(Feature::MatrixOps);
        let array_lit = Expr::ArrayLit { rows: vec![row], span: span.clone() };
        Ok(Expr::Ravel { target: Box::new(array_lit), span })
    }

    /// Convert one `NUMBER` token's source text into an `Expr::IntLit`/
    /// `Expr::FloatLit`. Unlike J (whose `NUMBER` folds a leading
    /// underscore into ASCII `-` before parsing), a Q `NUMBER` token's own
    /// value is already plain, standard numeric syntax --
    /// `q-lexer::fold_negative_number_literals`'s post-tokenize hook
    /// already prepends an ordinary ASCII `-` directly onto the token's
    /// `value` string when a negative literal is recognized (MA11 §3
    /// bullet 2), so no translation is needed here at all.
    fn number_literal(&mut self, tok: &Token) -> Result<Expr, QLowerError> {
        let span = Span::point(FILE, tok.line, tok.column);
        let text = &tok.value;
        let invalid = || QLowerError {
            message: format!("invalid number literal '{text}'"),
            line: tok.line,
            column: tok.column,
        };
        let expr = if text.contains('.') || text.contains('e') || text.contains('E') {
            let value = text.parse::<f64>().map_err(|_| invalid())?;
            self.observed.add(Feature::Floats);
            Expr::FloatLit { value, span }
        } else {
            match text.parse::<i64>() {
                Ok(v) => Expr::IntLit { value: v, span },
                Err(_) => {
                    let value = text.parse::<f64>().map_err(|_| invalid())?;
                    self.observed.add(Feature::Floats);
                    Expr::FloatLit { value, span }
                }
            }
        };
        Ok(expr)
    }

    // -------------------------------------------------------------------
    // function literals -- MA11 §2/§3 bullet 1's headline novelty
    // -------------------------------------------------------------------

    /// Build a synthesized top-level [`Function`] from a `function_literal`
    /// node (`{[x;y] stmt; stmt; ...}` or the bracket-omitted implicit
    /// `x`/`y`/`z` form), registers it in `self.functions`/
    /// `self.known_function_arity`, and returns its synthesized SIR name.
    ///
    /// **Rejects nested function literals** (MA11 §4): if this method is
    /// reached while already lowering another function literal's own body
    /// (`self.inside_lambda_depth > 0`), this is a function literal
    /// appearing textually inside another one's body -- mirrors
    /// `q_runtime::eval::Interpreter::build_lambda`'s identical
    /// unconditional rejection (see that method's own doc comment for the
    /// full rationale: even a nested literal that is merely
    /// constructed-and-returned, never invoked within the same call, would
    /// need real lexical closure capture this crate deliberately never
    /// implements).
    fn register_function_literal(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<String, QLowerError> {
        if self.inside_lambda_depth > 0 {
            return Err(self.err_at(
                node,
                "nested function literals are not supported in this cut (MA11 §4 -- every \
                 function body in scope calls only primitives and already-defined functions, \
                 with no nested function-literal definitions of its own)"
                    .to_string(),
            ));
        }
        self.check_depth(node, depth)?;

        let param_list_node = node.children.iter().find_map(|c| match c {
            ASTNodeOrToken::Node(n) if n.rule_name == "param_list" => Some(n),
            _ => None,
        });
        let params: Vec<String> = match param_list_node {
            Some(pl) => {
                let names: Vec<String> = pl
                    .children
                    .iter()
                    .filter_map(|c| match c {
                        ASTNodeOrToken::Token(t) if t.effective_type_name() == "NAME" => {
                            Some(t.value.clone())
                        }
                        _ => None,
                    })
                    .collect();
                if names.is_empty() {
                    return Err(
                        self.err_at(node, "malformed function_literal (empty param_list)".to_string())
                    );
                }
                names
            }
            // The bracket-omitted implicit-parameter convenience (MA11 §3
            // bullet 1 / §4): defaults to the well-documented `x`/`y`/`z`
            // names.
            None => vec!["x".to_string(), "y".to_string(), "z".to_string()],
        };

        let stmt_seq_node = node
            .children
            .iter()
            .find_map(|c| match c {
                ASTNodeOrToken::Node(n) if n.rule_name == "stmt_seq" => Some(n),
                _ => None,
            })
            .ok_or_else(|| {
                self.err_at(node, "malformed function_literal (missing stmt_seq)".to_string())
            })?;
        let statements: Vec<&GrammarASTNode> = child_nodes(stmt_seq_node)
            .into_iter()
            .filter(|n| n.rule_name == "statement")
            .collect();
        if statements.is_empty() {
            return Err(self.err_at(node, "malformed function_literal (empty body)".to_string()));
        }

        self.lambda_counter += 1;
        let sir_name = format!("q_lambda_{}", self.lambda_counter);
        let span = self.span_of(node);

        self.inside_lambda_depth += 1;
        let mut fn_scope = FnScope::for_lambda(&params);
        let body_result = self.lower_function_body(&statements, depth + 1, &mut fn_scope);
        self.inside_lambda_depth -= 1;
        let (body_stmts, body_value) = body_result?;

        // Every parameter after the first gets a disclosed sentinel
        // default (see this file's module doc comment's "Disclosed
        // simplification" section) so a call supplying fewer arguments
        // than declared (the common case for the implicit x/y/z
        // convenience, called monadically or dyadically) is still
        // accepted by SIR's own arity model.
        let sir_params: Vec<Param> = params
            .iter()
            .enumerate()
            .map(|(i, p)| Param {
                name: p.clone(),
                sir_type: None,
                kind: ParamKind::Required,
                default: if i == 0 {
                    None
                } else {
                    Some(Box::new(Expr::IntLit { value: 0, span: span.clone() }))
                },
                span: span.clone(),
            })
            .collect();
        if sir_params.len() > 1 {
            self.observed.add(Feature::DefaultParams);
        }
        self.observed.add(Feature::Closures);

        let arity = sir_params.len();
        self.known_function_arity.insert(sir_name.clone(), arity);
        self.functions.push(Function {
            name: sir_name.clone(),
            params: sir_params,
            return_type: None,
            captures: vec![],
            body: Block { stmts: body_stmts, value: body_value, span: span.clone() },
            effects: EffectSet::PURE,
            metadata: Metadata::new(),
            span,
        });
        Ok(sir_name)
    }

    /// Lower a function-literal body's `stmt_seq` (one or more
    /// `statement`s): every statement but the last is lowered for its
    /// (possible) side-effecting evaluation only and discarded (mirroring
    /// `q_runtime::eval::Interpreter::call_lambda`'s own "evaluate every
    /// body statement in order, keeping only the last" behavior); the
    /// **last** statement's value becomes the function's own return value
    /// -- never wrapped in `print` the way a top-level bare statement is
    /// (MA11 §4: auto-print is a top-level-only convention; a function
    /// body's own intermediate/final values are never printed).
    fn lower_function_body(
        &mut self,
        statements: &[&GrammarASTNode],
        depth: usize,
        fn_scope: &mut FnScope,
    ) -> Result<(Vec<Stmt>, Expr), QLowerError> {
        let mut stmts = Vec::new();
        let last_index = statements.len() - 1;
        let mut value: Option<Expr> = None;
        for (i, stmt) in statements.iter().enumerate() {
            let assignment_node = only_node(stmt)
                .ok_or_else(|| self.err_at(stmt, "malformed statement".to_string()))?;
            if assignment_node.children.len() == 1 {
                let noun_expr_node = only_node(assignment_node).ok_or_else(|| {
                    self.err_at(assignment_node, "malformed noun_expr statement".to_string())
                })?;
                let v = self.lower_noun_expr(noun_expr_node, depth + 1, fn_scope)?;
                if i == last_index {
                    value = Some(v);
                } else {
                    let span = v.span().clone();
                    stmts.push(Stmt::ExprStmt { expr: v, span });
                }
            } else {
                let (mut s, assigned) =
                    self.lower_assignment_chain(assignment_node, depth + 1, fn_scope, false)?;
                stmts.append(&mut s);
                if i == last_index {
                    match assigned {
                        AssignedValue::Value(v) => value = Some(v),
                        AssignedValue::Function(_) => {
                            return Err(self.err_at(
                                assignment_node,
                                "a function literal cannot be the last statement's own \
                                 directly-assigned value in this cut (MA11 §4 -- no nested \
                                 function-literal definitions)"
                                    .to_string(),
                            ));
                        }
                    }
                }
            }
        }
        Ok((stmts, value.expect("non-empty statements guarantees a value")))
    }

    // -------------------------------------------------------------------
    // verb_expr / verb_primitive
    // -------------------------------------------------------------------

    /// `verb_expr = verb_primitive [ EACH | REDUCE | SCAN ] | NAME | function_literal`.
    fn lower_verb_expr(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        fn_scope: &FnScope,
    ) -> Result<FnKind, QLowerError> {
        self.check_depth(node, depth)?;
        match node.children.as_slice() {
            [ASTNodeOrToken::Node(prim)] if prim.rule_name == "verb_primitive" => {
                Ok(FnKind::Prim(self.lower_verb_primitive(prim)?))
            }
            [ASTNodeOrToken::Node(prim), ASTNodeOrToken::Token(adverb)]
                if prim.rule_name == "verb_primitive" =>
            {
                let p = self.lower_verb_primitive(prim)?;
                match adverb.effective_type_name() {
                    "EACH" => Ok(FnKind::Each(p)),
                    "REDUCE" => Ok(FnKind::Reduce(self.require_scalar_atom(p, node, "/ (reduce)")?)),
                    "SCAN" => Ok(FnKind::Scan(self.require_scalar_atom(p, node, "\\ (scan)")?)),
                    other => Err(self.err_at(node, format!("unexpected adverb token '{other}'"))),
                }
            }
            [ASTNodeOrToken::Token(t)] if t.effective_type_name() == "NAME" => {
                let value_expr = self.lower_name(t, node, fn_scope)?;
                Ok(self.expr_to_fnkind(value_expr))
            }
            [ASTNodeOrToken::Node(fl)] if fl.rule_name == "function_literal" => {
                let sir_name = self.register_function_literal(fl, depth + 1)?;
                Ok(FnKind::KnownFn(sir_name))
            }
            _ => Err(self.err_at(node, "malformed verb_expr".to_string())),
        }
    }

    /// `verb_primitive`: always exactly one child, a single token naming
    /// the primitive glyph -- mirrors `q_runtime::eval::parse_verb_primitive`'s
    /// exact token-to-variant mapping.
    fn lower_verb_primitive(&self, node: &GrammarASTNode) -> Result<Prim, QLowerError> {
        let tok = match node.children.first() {
            Some(ASTNodeOrToken::Token(t)) => t,
            _ => return Err(self.err_at(node, "malformed verb_primitive".to_string())),
        };
        Ok(match tok.effective_type_name() {
            "PLUS" => Prim::Plus,
            "MINUS" => Prim::Minus,
            "STAR" => Prim::Star,
            "PERCENT" => Prim::Percent,
            "BANG" => Prim::Bang,
            "COMMA" => Prim::Comma,
            "HASH" => Prim::Hash,
            "UNDERSCORE" => Prim::Underscore,
            "AMP" => Prim::Amp,
            "PIPE" => Prim::Pipe,
            "TILDE" => Prim::Tilde,
            "EQ" => Prim::Eq,
            "LT" => Prim::Lt,
            "GT" => Prim::Gt,
            "LE" => Prim::Le,
            "GE" => Prim::Ge,
            "NE" => Prim::Ne,
            other => return Err(self.err_at(node, format!("unknown verb primitive '{other}'"))),
        })
    }

    /// Reduce/scan apply only to the 12 primitives that map onto an
    /// [`ElementwiseOpKind`] -- `! , # _ ~` are not "a scalar dyadic verb"
    /// at all, so stacking an adverb onto one of them is a clean, explicit
    /// scope error, mirroring `q_runtime::builtins::require_scalar_binop`
    /// exactly.
    fn require_scalar_atom(
        &self,
        p: Prim,
        node: &GrammarASTNode,
        context: &str,
    ) -> Result<ElementwiseOpKind, QLowerError> {
        p.to_binop().ok_or_else(|| {
            self.err_at(node, format!("{context}: {} is not a scalar dyadic verb", p.glyph()))
        })
    }

    // -------------------------------------------------------------------
    // monadic / dyadic application
    // -------------------------------------------------------------------

    /// Apply a monadic (one-argument) callable to `arg` -- the single
    /// dispatch site for every [`FnKind`] variant, mirroring
    /// `q_runtime::eval::Interpreter::apply_monadic`'s identical role.
    fn apply_monadic(&mut self, f: FnKind, arg: Expr, span: Span) -> Result<Expr, QLowerError> {
        match f {
            FnKind::Prim(p) => self.apply_monadic_prim(p, arg, span),
            FnKind::Each(p) => {
                if !p.each_monadic_supported() {
                    return Err(self.err_at_span(
                        &span,
                        format!(
                            "' (each) has no well-defined per-element meaning for '{}' \
                             monadically in this cut's flat, dense-array-only value model \
                             (MA11 §4)",
                            p.glyph()
                        ),
                    ));
                }
                self.apply_monadic_prim(p, arg, span)
            }
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
            FnKind::KnownFn(sir_name) => {
                let arity = *self
                    .known_function_arity
                    .get(&sir_name)
                    .expect("KnownFn always names a function this crate itself registered");
                if arity < 1 {
                    return Err(self.err_at_span(
                        &span,
                        format!("function takes at most {arity} parameter(s), called with 1"),
                    ));
                }
                Ok(Expr::DirectCall {
                    fn_name: sir_name,
                    args: vec![arg],
                    effects: EffectSet::PURE,
                    span,
                })
            }
            FnKind::Dynamic(callee) => {
                self.observed.add(Feature::Closures);
                Ok(Expr::IndirectCall { target: callee, args: vec![arg], effects: EffectSet::PURE, span })
            }
        }
    }

    /// Apply a dyadic (two-argument) callable to `lhs`/`rhs`.
    fn apply_dyadic(
        &mut self,
        f: FnKind,
        lhs: Expr,
        rhs: Expr,
        span: Span,
    ) -> Result<Expr, QLowerError> {
        match f {
            FnKind::Prim(p) => self.apply_dyadic_prim(p, lhs, rhs, span),
            FnKind::Each(p) => {
                if !p.each_dyadic_supported() {
                    return Err(self.err_at_span(
                        &span,
                        format!(
                            "' (each) has no well-defined per-element meaning for '{}' \
                             dyadically in this cut's flat, dense-array-only value model \
                             (MA11 §4)",
                            p.glyph()
                        ),
                    ));
                }
                self.apply_dyadic_prim(p, lhs, rhs, span)
            }
            FnKind::Reduce(_) => Err(self.err_at_span(
                &span,
                "/ (reduce) takes exactly one operand, but was applied dyadically".to_string(),
            )),
            FnKind::Scan(_) => Err(self.err_at_span(
                &span,
                "\\ (scan) takes exactly one operand, but was applied dyadically".to_string(),
            )),
            FnKind::KnownFn(sir_name) => {
                let arity = *self
                    .known_function_arity
                    .get(&sir_name)
                    .expect("KnownFn always names a function this crate itself registered");
                if arity < 2 {
                    return Err(self.err_at_span(
                        &span,
                        format!("function takes at most {arity} parameter(s), called with 2"),
                    ));
                }
                Ok(Expr::DirectCall {
                    fn_name: sir_name,
                    args: vec![lhs, rhs],
                    effects: EffectSet::PURE,
                    span,
                })
            }
            FnKind::Dynamic(callee) => {
                self.observed.add(Feature::Closures);
                Ok(Expr::IndirectCall {
                    target: callee,
                    args: vec![lhs, rhs],
                    effects: EffectSet::PURE,
                    span,
                })
            }
        }
    }

    /// Monadic dispatch for a bare primitive glyph (MA11 §4's full table --
    /// see this file's module doc comment's "The 17 primitives" section for
    /// the complete mapping and the rationale behind each reused/new node).
    fn apply_monadic_prim(&mut self, p: Prim, arg: Expr, span: Span) -> Result<Expr, QLowerError> {
        match p {
            // Flip: identity in this cut -- no primitive can ever
            // construct a rank-2 value (see the module doc comment).
            Prim::Plus => Ok(arg),
            Prim::Minus => Ok(wrap_builtin("neg", arg)),
            Prim::Star => Ok(wrap_builtin("q_first", arg)),
            Prim::Percent => Ok(wrap_builtin("recip", arg)),
            Prim::Bang => {
                self.observed.add(Feature::NDArrays);
                Ok(self.zero_base_index(
                    Expr::IndexGenerator { count: Box::new(arg), span: span.clone() },
                    span,
                ))
            }
            Prim::Comma => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::Ravel { target: Box::new(arg), span })
            }
            Prim::Hash => Ok(wrap_builtin("tally", arg)),
            Prim::Underscore => Ok(wrap_builtin("floor", arg)),
            Prim::Amp => Ok(wrap_builtin("q_where", arg)),
            Prim::Pipe => Ok(wrap_builtin("q_reverse", arg)),
            Prim::Tilde => Ok(wrap_builtin("q_not", arg)),
            Prim::Eq | Prim::Ne | Prim::Lt | Prim::Le | Prim::Ge | Prim::Gt => Err(
                self.err_at_span(
                    &span,
                    format!(
                        "no monadic form for {} (comparison atoms are dyadic-only in Q)",
                        p.glyph()
                    ),
                ),
            ),
        }
    }

    /// Dyadic dispatch for a bare primitive glyph.
    fn apply_dyadic_prim(
        &mut self,
        p: Prim,
        lhs: Expr,
        rhs: Expr,
        span: Span,
    ) -> Result<Expr, QLowerError> {
        if let Some(op) = p.to_binop() {
            self.observed.add(Feature::MatrixOps);
            self.observed.add(Feature::ArrayColumnMajor);
            return Ok(Expr::ElementwiseOp { op, lhs: Box::new(lhs), rhs: Box::new(rhs), span });
        }
        match p {
            // MA11 §4: "dyadic `!` (dict creation, and its other real
            // overloads) is deferred" -- explicitly out of scope, never
            // silently misinterpreted as something else.
            Prim::Bang => Err(self.err_at_span(
                &span,
                "dyadic ! (dict creation) is not yet implemented -- explicitly deferred, MA11 §4"
                    .to_string(),
            )),
            Prim::Comma => {
                self.observed.add(Feature::MatrixOps);
                self.observed.add(Feature::ArrayColumnMajor);
                Ok(Expr::Catenate { lhs: Box::new(lhs), rhs: Box::new(rhs), span })
            }
            Prim::Hash => Ok(Expr::BuiltinCall {
                name: "q_take".to_string(),
                args: vec![lhs, rhs],
                effects: EffectSet::PURE,
                span,
            }),
            Prim::Underscore => Ok(Expr::BuiltinCall {
                name: "q_drop".to_string(),
                args: vec![lhs, rhs],
                effects: EffectSet::PURE,
                span,
            }),
            Prim::Tilde => Ok(Expr::BuiltinCall {
                name: "q_match".to_string(),
                args: vec![lhs, rhs],
                effects: EffectSet::PURE,
                span,
            }),
            _ => unreachable!(
                "every primitive not covered by `to_binop` is handled by one of the three \
                 explicit arms above (Bang/Comma/Hash/Underscore/Tilde -- exactly the five \
                 `to_binop` returns None for)"
            ),
        }
    }

    /// Convert an `Expr::IndexGenerator` result (SIR22-addendum, shared
    /// with APL/J) from its hardcoded 1-based APL convention to Q's own
    /// 0-based `!` convention -- the identical fix
    /// `j-to-semantic-ir::Lowerer::zero_base_index` already applies for
    /// J's own 0-based `i.`, since `semantic-ir-to-javascript`'s codegen
    /// for this node hardcodes APL's `⍳` semantics with no
    /// parameterization. The fix is the same pure arithmetic identity: for
    /// every element, `q_value = apl_value - 1` holds (found: `apl_value`
    /// is the 1-based position `k+1` for Q's 0-based `k`).
    fn zero_base_index(&mut self, apl_convention: Expr, span: Span) -> Expr {
        self.observed.add(Feature::MatrixOps);
        self.observed.add(Feature::ArrayColumnMajor);
        Expr::ElementwiseOp {
            op: ElementwiseOpKind::Sub,
            lhs: Box::new(apl_convention),
            rhs: Box::new(Expr::IntLit { value: 1, span: span.clone() }),
            span,
        }
    }

    // -------------------------------------------------------------------
    // small helpers
    // -------------------------------------------------------------------

    fn span_of(&self, node: &GrammarASTNode) -> Span {
        Span::point(FILE, node.start_line.unwrap_or(1), node.start_column.unwrap_or(1))
    }

    fn err_at(&self, node: &GrammarASTNode, message: String) -> QLowerError {
        QLowerError {
            message,
            line: node.start_line.unwrap_or(1),
            column: node.start_column.unwrap_or(1),
        }
    }

    fn err_at_span(&self, span: &Span, message: String) -> QLowerError {
        QLowerError { message, line: span.start_line, column: span.start_col }
    }

    fn check_depth(&self, node: &GrammarASTNode, depth: usize) -> Result<(), QLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
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

/// If `noun_expr_node` (a 1-child `noun_expr`, i.e. a bare `term`) is
/// directly a `function_literal` -- not wrapped in parens, not part of a
/// larger expression -- return that `function_literal` node. Used to
/// detect "this assignment's RHS is a bare function-literal definition"
/// (MA11 §3 bullet 1), the one case that registers a top-level
/// [`Function`] instead of emitting an ordinary value-binding [`Stmt`].
fn bare_function_literal_term(noun_expr_node: &GrammarASTNode) -> Option<&GrammarASTNode> {
    let term_node = only_node(noun_expr_node)?;
    if term_node.rule_name != "term" {
        return None;
    }
    match term_node.children.as_slice() {
        [ASTNodeOrToken::Node(n)] if n.rule_name == "function_literal" => Some(n),
        _ => None,
    }
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
