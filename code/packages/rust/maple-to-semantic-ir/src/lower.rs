//! The lowering pass from `coding_adventures_maple_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **v0.1.0**.
//!
//! This is the **fifth and final** frontend to target
//! [SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
//! symbolic-expression/pattern-matching domain extension of the SIR10
//! narrow-waist IR (Stream B of
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)). The first,
//! `wolfram-to-semantic-ir`, and the second, `macsyma-to-semantic-ir`, are
//! this crate's design templates for the "everything is data" decision —
//! read `wolfram-to-semantic-ir::lower`'s module doc comment first. The
//! fourth, `reduce-to-semantic-ir`, is the closest sibling (both languages
//! are "surface operators + `head(args)` calls" with no pattern/
//! rewrite-rule vocabulary) — everything below assumes that context and
//! only calls out where Maple's grammar genuinely differs.
//!
//! # Retargeting `maple-runtime`, not starting from scratch
//!
//! `maple-runtime` already walks this exact CST and compiles it to
//! `symbolic_ir::IRNode` (`Symbol`/`Integer`/`Float`/`Apply` — see that
//! crate's `src/lower.rs`, in particular its module doc comment). This
//! crate's job is the same mechanical retarget `reduce-to-semantic-ir`
//! already did for `reduce-runtime`: walk the same CST, dispatch on the
//! same rule names `maple-runtime::lower::lower_node` already uses, and
//! construct `semantic_ir::Expr::{SymSymbol,SymApply}` wherever that
//! lowering constructs `symbolic_ir::IRNode::{Symbol,Apply}`. Literals
//! (`Integer`/`Float`) reuse SIR10/SIR16's `IntLit`/`FloatLit` directly —
//! Maple has no `STRING` token at all (`maple.tokens`'s `escapes: none`),
//! so, like Reduce and Derive, this crate never constructs `Expr::StrLit`
//! either.
//!
//! # A REAL structural difference from Reduce: the dispatch is SPLIT
//!
//! `reduce.grammar`'s `expr = if_expr | group_expr | assignment` sits at
//! the very TOP of Reduce's expression grammar, so `if`/`<<...>>`/`:=` are
//! reachable from *every* position an `expr` can appear — nested inside an
//! arithmetic operand, a function argument, anywhere. `maple.grammar`
//! draws a hard line Reduce's own grammar does not (see that grammar's own
//! "statements vs. expressions" design-decision comment, reused here only
//! by reference, not copied): `statement = if_expr | assignment` sits in
//! its OWN nonterminal, never reachable from `expr` at all. `expr` itself
//! is just `logical_or` — Chapter 3 "Maple Expressions" — with no
//! alternative that ever leads back to `if_expr`/`assignment` — Chapter 5
//! "Maple Statements".
//!
//! Concretely, this crate's [`Lowerer::lower_node`] is still ONE dispatch
//! table (mirroring `maple-runtime::lower::lower_node`'s own single
//! `match`, not two separate Rust functions) — but the grammar's own
//! reachability graph, not a Rust-level split, is what enforces the real
//! divide: [`Lowerer::lower_if`] and [`Lowerer::lower_assignment`] are
//! only ever reached via `lower_node`'s own `"if_expr"`/`"assignment"`
//! match arms, which in turn are only ever reached from
//! [`Lowerer::lower_file`]'s top-level statement loop (`statement_line`/
//! `statement`) — never nested inside an arithmetic/comparison/logical
//! operand the way Reduce's can be. `x := if a then 1 else 2 end if;` and
//! `a := b := c;` are both syntax errors in Maple's grammar (confirmed
//! directly against `maple.grammar`, not just trusted from
//! `maple-runtime`'s doc comment) — so no `Assign`/`Define` node can ever
//! appear nested inside another `SymApply`'s argument list the way
//! Reduce's `If`/`CompoundExpression` can. This crate's dispatch table
//! reflects that same shape by simply retargeting `maple-runtime`'s own
//! rule-name arms one-for-one, rather than blindly copying
//! `reduce-to-semantic-ir`'s unified-reachability dispatch.
//!
//! # Scope (v0.1.0) — no pattern-matching or rewrite-rule syntax at all
//!
//! **Verified empirically against `maple.grammar`/`maple.tokens`
//! (`code/grammars/maple/`), not just trusted from `maple-runtime`'s own
//! doc comment**, exactly mirroring `reduce-to-semantic-ir`'s own
//! verification discipline: grepping both files for pattern-syntax tokens
//! (`_` blank, `->`/`:>` rule arrows, `/.`/`//.` replacement operators)
//! finds none — `maple.tokens` declares no such token, and `maple.grammar`
//! DOES have an `ARROW` (`->`) token, but it appears in exactly ONE
//! production, `arrow_def = arrow_params ARROW expr` (MA09 §3's
//! general-purpose function-definition spelling, `f := x -> x^2`), never
//! as a pattern-rule arrow the way Wolfram's `->`/`:>` are. This confirms
//! `maple-runtime`'s own claim ("no pattern/rewrite-rule vocabulary in
//! this subset — MA09 §4 defers `patmatch`/`match`, ordinary library
//! calls, not surface grammar") is accurate for the grammar as it
//! currently ships.
//!
//! This crate therefore **only ever constructs [`Expr::SymSymbol`] and
//! [`Expr::SymApply`]** (plus the reused `IntLit`/`FloatLit` literal
//! nodes) — it never constructs `SymPatternBlank`/`SymPatternNamed`/
//! `SymRule`/`SymReplaceAll`, and it never observes
//! `Feature::PatternMatching`. One concrete consequence, mirroring
//! `reduce-to-semantic-ir`'s identical note: [`measure_depth_iterative`]/
//! [`drop_iterative`] below only need a match arm for [`Expr::SymApply`]
//! (recursing into `head` and `args`) — every other `Expr` variant is a
//! leaf for this crate's purposes. `If`/`Assign`/`Define`/`Set` all lower
//! to *this same* `SymApply` variant (only the head symbol's name or the
//! bracket differs), so this holds even though this crate covers the
//! full statement/expression split.
//!
//! # Assignment: a bare `NAME` LHS, never a call shape — plus a genuinely
//! new `arrow_def`/`arrow_params` path
//!
//! `assignment = NAME ASSIGN ( arrow_def | expr ) | expr` — deliberately
//! NARROWER than `reduce-to-semantic-ir::lower_assignment`'s
//! `SymApply{head: SymSymbol(_), ..}`-shaped disambiguation.
//! `maple.grammar`'s own "assignment's left-hand side" design-decision
//! comment explains why: Maple's identical-looking `f(x) := expr`
//! spelling means something NARROWER and DIFFERENT in real Maple (a
//! remember-table specific-value patch onto an ALREADY-EXISTING
//! procedure, MA09 §1/§4) than Reduce's/Derive's own general-definition
//! idiom of the same shape — so `maple.grammar` makes `f(x) := expr` fail
//! to *parse* at all, and this crate's [`Lowerer::lower_assignment`] never
//! needs to distinguish "was the LHS call-shaped" the way Reduce's/
//! Derive's own `lower_assignment` does. By the time `lower_node`
//! dispatches to [`Lowerer::lower_assignment`], a genuine `assignment`
//! node always has the 3-child `[NAME, ASSIGN, (arrow_def | expr)]` shape
//! — the bare-`expr` alternative dissolves away (via [`unwrap_single`])
//! before ever reaching this function; [`Lowerer::lower_first_node`]'s
//! fallback there is defensive only, mirroring `maple-runtime::
//! lower_assignment`'s identical defensive shape.
//!
//! Instead, Maple has a SEPARATE production, `arrow_def = arrow_params
//! ARROW expr` (MA09 §3's own two worked spellings: `f := (x, y) -> x + y`
//! and `f := x -> x^2`), for general function definition —
//! [`Lowerer::lower_arrow_def`] lowers this to `Define[f, List[params...],
//! body]`, the same `Define` shape Derive's/Reduce's own
//! (differently-spelled) general-definition idioms already use.
//! [`Lowerer::lower_arrow_params`] collects every `NAME` token among the
//! node's children in order — `arrow_params = NAME | LPAREN [ NAME {
//! COMMA NAME } ] RPAREN` — both the bare-single-parameter and the
//! parenthesised-list shapes reduce to the identical "collect NAME
//! tokens" walk, so there is no need to branch on which alternative
//! matched; the `LPAREN`/`COMMA`/`RPAREN` tokens present in the
//! parenthesised form are harmlessly filtered out. Zero parameters (`()
//! -> e`) falls out of the optional inner list for free.
//!
//! # `if`/`elif`/`else` — a right-fold, mirroring Macsyma's elif chain,
//! NOT Reduce's simpler 2-or-3-child `if`
//!
//! `if_expr = "if" expr "then" statement { "elif" expr "then" statement }
//! [ "else" statement ] ( "end" "if" | "fi" )` — unlike Reduce's `if_expr
//! = "if" expr "then" expr [ "else" expr ]` (a plain 2-or-3-children
//! count, since Reduce's grammar has no `elseif` repetition at all), Maple
//! has a flat `{ "elif" expr "then" statement }` EBNF repetition that must
//! be folded right-to-left into nested `If` applications — the *same*
//! shape `macsyma-runtime`'s own elif-chain fold already handles for
//! Macsyma, and the *same* fold [`maple-runtime::lower_if`] already
//! implements, retargeted directly here.
//!
//! Because Maple requires an explicit close (`end if` or `fi`) for every
//! `if_expr`, there is no dangling-else ambiguity the way Reduce's
//! `if`/`else` chain has: a nested `if_expr` inside a branch must run all
//! the way to its OWN close before the outer `if_expr`'s own `elif`/
//! `else`/close is ever reached — structurally only one place an outer
//! `else` can attach, by construction. Every child *node* of an `if_expr`
//! (ignoring the keyword tokens — `Group`/`Alternation` splice whichever
//! branch matched directly into the parent `Sequence`, they never
//! synthesize a wrapper node, confirmed directly against
//! `maple-parser`'s compiled grammar) appears in strict alternating
//! `(cond, body)` pairs, with one optional trailing lone `body` node for a
//! final `else` — [`Lowerer::lower_if`] collects `child_nodes` in order
//! and walks it two at a time (an odd-length list signals a trailing
//! `else`), exactly mirroring `maple-runtime::lower_if`'s identical
//! collection-and-fold logic, retargeted onto `semantic_ir::Expr`.
//!
//! The `{ "elif" ... }` repetition is a FLAT list of sibling children (not
//! right-recursive — confirmed directly against `maple-parser`'s own
//! `MAX_RULE_DEPTH` doc comment, which measures every `{ x }` EBNF
//! repetition in this grammar as costing zero native parser stack
//! regardless of width), so folding N `elif` arms right-to-left still
//! builds an N-deep nested `If` `Expr` tree — [`Lowerer::
//! check_elif_chain_length`] bounds `elif`-arm count before any fold
//! happens, the same "flat CST repetition folds into a deep tree" DoS
//! shape [`Lowerer::check_chain_length`] already guards for
//! `additive`/`multiplicative`/`logical_or`/`logical_and`.
//!
//! # `Set` — a canonical head genuinely new to this repo (MA09 §5)
//!
//! Maple is the first language in this repo with **two** distinct
//! bracketed aggregate literals: `[a, b, c]` (ordered, `List` — a shared,
//! already-existing head every CAS-family sibling here reuses) and `{a,
//! b, c}` (unordered, `Set` — MA09 §3/§5). `symbolic-vm`'s shared handler
//! table has no handler for a `Set` head (confirmed by `maple-runtime`'s
//! own doc comment, which greps `symbolic_vm::handlers::
//! build_handler_table` directly) — so [`SET`] is a `pub const` defined
//! LOCALLY in this crate, exactly the same pattern
//! `reduce-to-semantic-ir`/`reduce-runtime` used for their own new
//! `COMPOUND_EXPRESSION`/`CONS`/`FIRST`/… constants (a locally-defined
//! `pub const`, not added to shared `symbolic-ir`, since it's not a
//! `Backend`-agnostic canonical head), spelled to match `maple-runtime`'s
//! own identically-named constant. This is largely moot for this crate:
//! it never evaluates anything (the "everything is data" design every
//! SIR23 frontend shares), so a `SymApply{head: "Set", ...}` node is
//! valid, executable SIR23 data regardless of whether any *runtime*
//! currently has a handler for it.
//!
//! # `diff`/`int` — thin bridges to already-shared calculus handlers
//!
//! `diff(f, x)`/`int(f, x)` bridge to the canonical `D`/`Integrate` heads
//! — the same idea as Derive's `DIF`/`INT` bridge and
//! `maple-runtime::surface_head_to_ir`'s own identical table, just
//! lowercase surface spelling (MA09 §3: Maple's builtin names are
//! conventionally lowercase, unlike Derive's uppercase convention). A
//! plain name→canonical-head bridge table entry, no new logic — unlike
//! `reduce-to-semantic-ir`'s much larger list-accessor bridge table, MA09
//! §3 documents no function-call spelling for list/set construction
//! (Maple already has literal `[...]`/`{...}` syntax for both) and this
//! subset deliberately does not bridge elementary-function names (`sin`,
//! `log`, …) either.
//!
//! # Booleans — the first literal `true`/`false` TOKENS in this CAS family
//!
//! Neither `derive.grammar`'s nor `reduce.grammar`'s own grammar has a
//! dedicated boolean literal token (their booleans only arise from
//! comparison/logic results) — `maple.grammar`'s `atom` rule is the first
//! to include `"true"`/`"false"` as their own alternatives (MA09 §3,
//! citing the `type/truefalseFAIL` Help page). `maple-runtime::
//! lower_token` bridges these to the shared backend's pre-bound `True`/
//! `False` symbols (`symbolic_vm::backend::BaseBackend::new` pre-binds
//! exactly those two names) — the same bridge `macsyma-compiler::
//! lower_token` already uses for its own boolean keywords (`"KEYWORD" if
//! token.value == "true" => Ok(sym("True"))`). [`Lowerer::lower_token`]
//! retargets this exact bridge. **Verified directly against
//! `symbolic-ir/src/lib.rs`** (per this repo's verify-before-implementing
//! discipline, not assumed): that crate exports no `TRUE`/`FALSE`
//! constants at all (`grep -n '"True"\|"False"\|pub const TRUE\|pub
//! const FALSE'` turns up nothing but a stray test literal) — every
//! sibling CAS-family lowering that needs these two names uses bare
//! string literals, so this crate does too (`self.sym_symbol("True"
//! .to_string(), span)`), rather than inventing constants no shared crate
//! defines.
//!
//! # The `;`-vs-`:` statement terminator is a runtime/session concept,
//! not something this frontend replicates
//!
//! `maple-runtime`'s own `LoweredStatement`/`Display` types tag every
//! lowered statement with whether `MapleSession`'s evaluation loop should
//! print a `Display` line — MA09 §3's own statement-separator row is
//! explicit this is "a display flag on the surrounding session, not an IR
//! node." This SIR23 frontend has no interactive-session/display concept
//! at all (mirrors how neither `derive-to-semantic-ir` nor
//! `reduce-to-semantic-ir` replicate their own native runtimes' prompt/
//! display machinery either) — [`Lowerer::lower_file`] just emits each
//! statement's lowered `Expr` as a plain `Stmt::ExprStmt`, exactly like
//! every sibling SIR23 frontend, and ignores the `;`/`:` distinction
//! entirely. No `LoweredStatement`-with-display-flag type exists here —
//! that shape is specific to REPL rendering, out of scope for this crate.
//!
//! # `postfix` is NOT chainable — no `check_postfix_chain_length`-
//! equivalent guard exists here at all
//!
//! `reduce-to-semantic-ir`'s (and `derive-to-semantic-ir`'s) `postfix`
//! production is `atom { LPAREN [ arglist ] RPAREN }` — a REPEATED call
//! suffix, so `f(x)(y)(z)…` parses and needs a chain-length guard before
//! lowering folds it into a deep nested-application tree.
//! `maple.grammar`'s own `postfix = atom [ LPAREN [ arglist ] RPAREN ] ;`
//! (verified directly against the grammar file, not assumed) has a single
//! OPTIONAL suffix — `[ ... ]`, not `{ ... }` — so `f(x)(y)` is not valid
//! Maple in this subset at all: after the first `(x)` is consumed,
//! `postfix`'s own production has nowhere left to attach a second call
//! group, and the leftover `(y)` fails whatever comes next (confirmed by
//! this crate's own `postfix_call_is_not_chainable` regression test,
//! which asserts `compile_source` rejects `f(x)(y);\n` as a parse error).
//! [`Lowerer::lower_postfix`] therefore has no chain-counting loop and no
//! analogous guard at all — the axis `check_postfix_chain_length` bounds
//! in the sibling crates is structurally impossible here, not merely
//! bounded by a cap.
//!
//! # Recursion-depth hardening — carried over proactively, not discovered
//!
//! `wolfram-to-semantic-ir`'s `CHANGELOG.md` documents four rounds of
//! security review that each found a real, adversarially-confirmed native
//! stack-overflow gap, and every sibling SIR23 frontend since
//! (`macsyma-to-semantic-ir`, `derive-to-semantic-ir`,
//! `reduce-to-semantic-ir`) carries every one of those hardening
//! mechanisms over from day one rather than rediscovering them. This
//! crate does the same, even though neither `maple-parser` nor
//! `maple-runtime` (the retarget source) applies any of these guards
//! themselves — they are a `*-to-semantic-ir`-frontend-specific defense,
//! not part of the native pipeline:
//!
//! - [`MAX_EXPR_DEPTH`] bounds this crate's own CST-walking recursion.
//! - [`Lowerer::check_chain_length`] caps every flat, same-precedence
//!   operator-chain fold (`additive`/`multiplicative`/`logical_or`/
//!   `logical_and`) before any tree is built — `maple-parser`'s own
//!   `MAX_RULE_DEPTH` doc comment confirms these ARE flat EBNF
//!   repetitions in this grammar (not right-recursion), costing zero
//!   native parser stack regardless of width.
//! - [`Lowerer::check_elif_chain_length`] caps the `elif`-arm count in an
//!   `if_expr` before [`Lowerer::lower_if`]'s right-fold runs — the same
//!   "flat repetition folds into a deep tree" shape, applied to the one
//!   construct genuinely new relative to Reduce's simpler `if`.
//! - **No `check_postfix_chain_length`-equivalent guard exists** — see
//!   the section above for why: `postfix`'s single OPTIONAL call suffix
//!   makes chained application structurally impossible in this grammar.
//! - [`Lowerer::check_apply_arg_count`] caps `arglist` element counts
//!   (shared by a call's arguments, `list_literal`, and `set_literal`)
//!   AND `arrow_params`' flat parameter-name count — flat-`Vec`
//!   allocation-size backstops, not stack guards.
//! - [`measure_depth_iterative`] is the authoritative,
//!   construction-composition-independent check: an iterative (never
//!   recursive) walk of an already-built `Expr`, called once per
//!   top-level statement in [`Lowerer::lower_file`].
//! - [`drop_iterative`] tears down a tree [`measure_depth_iterative`] just
//!   rejected, using an explicit work stack rather than the ordinary
//!   recursive `Drop` glue, for the same reason every sibling SIR23
//!   frontend documents (detecting an oversized tree and then letting it
//!   drop normally just relocates the same stack overflow from "walking
//!   forward" to "walking backward").
//!
//! Maple's six genuinely *self-referential* (right-recursive or
//! prefix-recursive) productions — parenthesised `group` nesting,
//! list-/set-literal nesting, a `not`-prefix chain, a unary-minus-prefix
//! chain, the power (`^`) chain, and nested `if`/`end if` (or `fi`) — need
//! NO additional lowering-side guard beyond the ordinary `depth`
//! parameter threaded through [`Lowerer::lower_node`]: `maple-parser`'s
//! own `MAX_RULE_DEPTH` (150) already bounds how deep any of these can
//! nest in the CST this crate ever receives (measured directly in that
//! crate's own doc comment across all six shapes independently — the
//! binding constraint is the `not`-chain's floor of 218 rule frames, a
//! ~31.2% margin below the 150 cap). This mirrors exactly why
//! `reduce-to-semantic-ir` needs no explicit guard on its own five
//! self-referential productions either — the risk there is bounded by the
//! parser, not by this module.
//!
//! # `compile` vs. `compile_source`
//!
//! This module's [`compile`] is pure lowering over an already-parsed
//! tree — see `src/lib.rs`'s `compile_source` doc comment for why, like
//! `macsyma-to-semantic-ir`/`derive-to-semantic-ir`/
//! `reduce-to-semantic-ir` (and unlike `wolfram-to-semantic-ir`), this
//! crate's `compile_source` does not need to spawn an enlarged-stack
//! worker thread: `maple-parser`'s own `MAX_RULE_DEPTH` (150) is already
//! documented safe on a bare default (~2 MiB) stack with comfortable
//! margin.

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span, Stmt,
};
use symbolic_ir::{
    ADD, AND, ASSIGN, D, DEFINE, DIV, EQUAL, GREATER, GREATER_EQUAL, IF, INTEGRATE, LESS,
    LESS_EQUAL, LIST, MUL, NEG, NOT, NOT_EQUAL, OR, POW, SUB,
};

/// The canonical head for a Maple set literal `{a, b, c}` (MA09 §3/§5).
///
/// Not exported by `symbolic-ir` (see the module doc comment's "`Set`"
/// section) — defined locally, spelled to match `maple-runtime::lower`'s
/// own identically-named constant, so the one place this crate needs the
/// spelling has a name, not a repeated string literal.
pub const SET: &str = "Set";

/// Maximum expression-nesting depth for *this crate's own* lowering
/// recursion — distinct from (and independent of) `maple-parser`'s own
/// `MAX_RULE_DEPTH` grammar-nesting guard (150), which bounds the CST this
/// crate walks. Mirrors `wolfram-to-semantic-ir`'s, `macsyma-to-semantic-
/// ir`'s, `derive-to-semantic-ir`'s, and `reduce-to-semantic-ir`'s
/// identically-named, identically-valued guard — kept at 256 for
/// consistency across the whole SIR23 frontend family rather than
/// inventing a new value, even though `maple-parser`'s own cap (150) is
/// lower: this constant bounds a DIFFERENT axis (this crate's own
/// chain-folding/tree-depth budget, exercised by e.g. a long flat `+`
/// chain that parses as ONE CST node regardless of nesting depth), not
/// the CST-nesting axis `maple-parser`'s own cap already bounds.
const MAX_EXPR_DEPTH: usize = 256;

/// Synthetic file name used for all spans (the CST does not carry the
/// original path).
const FILE: &str = "<maple>";

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// An error encountered during Maple → SIR lowering.
///
/// Mirrors `ReduceLowerError`/`DeriveLowerError`/`MacsymaLowerError`/
/// `WolframLowerError`'s shape exactly (`message` + 1-based
/// `line`/`column`) so tooling can treat every SIR frontend uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapleLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for MapleLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MapleLowerError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for MapleLowerError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed Maple CST (rooted at the `program` rule) into a SIR
/// module.
///
/// This function does **not** itself guard against native stack overflow
/// on deeply-nested input beyond its own [`MAX_EXPR_DEPTH`] cap — it
/// trusts `tree` was already parsed under a suitable guard
/// (`maple-parser`'s own `MAX_RULE_DEPTH`). See `src/lib.rs`'s
/// `compile_source` doc comment for why no worker-thread stack
/// enlargement is needed here.
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, MapleLowerError> {
    Lowerer::new(module_name).lower_file(tree)
}

// ---------------------------------------------------------------------------
// The lowerer
// ---------------------------------------------------------------------------

/// The lowering pass's only mutable state: the module name (fixed at
/// construction) and the set of SIR features observed while lowering
/// (used to build the manifest so it declares *exactly* what the module
/// emits — see `semantic-ir/src/validator.rs`'s `check_expr`, the ground
/// truth this must match node-kind-for-node-kind).
///
/// Like every sibling SIR23 frontend's `Lowerer`, there is no per-function
/// name-resolution context here at all: under the "everything is data"
/// design inherited from `maple-runtime` (see the module doc comment),
/// there are no host variables, parameters, or scopes to resolve — even
/// an arrow-definition's formal parameters lower to plain `SymSymbol`s
/// inside a `List`, not to bound names. This lowerer is a near-stateless
/// recursive descent over the CST.
struct Lowerer {
    module_name: String,
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
    // top level: `program = { statement_line } [ statement ] ;`
    // `statement_line = statement ( SEMI | COLON ) ;`
    // -------------------------------------------------------------------

    /// Like `reduce-to-semantic-ir::lower_file`, Maple's grammar adds an
    /// OPTIONAL final bare `statement` outside the repetition (so a
    /// source file need not end with a trailing `;`/`:`) — the identical
    /// shape `maple.grammar`'s own `program` comment documents (reused
    /// nearly verbatim from `reduce.grammar`'s own comment, per that
    /// grammar file's own note). The `;`-vs-`:` display distinction is
    /// deliberately NOT tracked here — see the module doc comment's
    /// "statement terminator" section.
    fn lower_file(&mut self, program: &GrammarASTNode) -> Result<Module, MapleLowerError> {
        if program.rule_name != "program" {
            return Err(self.err_at(
                program,
                format!("expected `program` root, got `{}`", program.rule_name),
            ));
        }

        let mut stmts: Vec<Stmt> = Vec::new();
        for child in child_nodes(program) {
            let statement_node = match child.rule_name.as_str() {
                "statement_line" => child_nodes(child).find(|n| n.rule_name == "statement"),
                "statement" => Some(child),
                _ => None,
            };
            let Some(statement_node) = statement_node else {
                continue;
            };
            let expr = self.lower_node(statement_node, 0)?;
            if measure_depth_iterative(&expr).is_none() {
                let err = self.err_at(
                    statement_node,
                    format!("expression tree too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
                );
                drop_iterative(expr);
                return Err(err);
            }
            let span = expr.span().clone();
            stmts.push(Stmt::ExprStmt { expr, span });
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
            .with_source_language("maple")
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
    // Dispatch
    // -------------------------------------------------------------------

    /// Lower a single arbitrary node, one level deeper than `depth`.
    ///
    /// One shared dispatch table — mirroring `maple-runtime::lower::
    /// lower_node`'s own single `match` exactly (see the module doc
    /// comment's "dispatch is SPLIT" section for why this is still
    /// correct despite the grammar's statement/expression divide: the
    /// divide is enforced by *reachability*, not by two Rust functions).
    /// Most grammar rules are "transparent wrappers" — a precedence level
    /// that did not apply its own operator still emits its own node with
    /// a single child, and so does `statement = if_expr | assignment`
    /// once it has committed to one alternative. [`unwrap_single`] peels
    /// those away so we dispatch on the first rule that genuinely shapes
    /// the tree.
    fn lower_node(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MapleLowerError> {
        if depth > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("expression nesting too deep (exceeds {MAX_EXPR_DEPTH} levels)"),
            ));
        }
        match unwrap_single(node) {
            Unwrapped::Token(token) => self.lower_token(token),
            Unwrapped::Node(node) => match node.rule_name.as_str() {
                "program" => Err(self.err_at(node, "nested program node is not an expression".to_string())),
                "statement_line" | "statement" | "expr" => self.lower_first_node(node, depth),
                "if_expr" => self.lower_if(node, depth),
                "assignment" => self.lower_assignment(node, depth),
                "logical_or" => self.lower_logical_chain(node, depth, OR),
                "logical_and" => self.lower_logical_chain(node, depth, AND),
                "logical_not" => self.lower_logical_not(node, depth),
                "comparison" => self.lower_comparison(node, depth),
                "additive" | "multiplicative" => self.lower_binary_chain(node, depth),
                "unary" => self.lower_unary(node, depth),
                "power" => self.lower_power(node, depth),
                "postfix" => self.lower_postfix(node, depth),
                "atom" => self.lower_atom(node, depth),
                "arglist" => Err(self.err_at(
                    node,
                    "an arglist cannot be lowered as a scalar expression".to_string(),
                )),
                "list_literal" => self.lower_list_literal(node, depth),
                "set_literal" => self.lower_set_literal(node, depth),
                "group" => self.lower_group(node, depth),
                other => Err(self.err_at(node, format!("no lowering for rule `{other}`"))),
            },
        }
    }

    /// Lower a raw token (a literal or a bare symbol). `and`/`or`/`not`/
    /// `if`/`then`/`elif`/`else`/`end`/`fi` are always consumed by their
    /// own grammar rule before reaching here as a bare leaf token (they
    /// are matched by literal spelling inside `if_expr`/`logical_and`/
    /// `logical_or`/`logical_not`'s own productions) — so, mirroring
    /// `maple-runtime::lower_token`, this only ever needs `NUMBER`/
    /// `NAME`/the two boolean `KEYWORD` arms.
    fn lower_token(&mut self, token: &Token) -> Result<Expr, MapleLowerError> {
        let span = self.token_span(token);
        match token_type(token) {
            "NUMBER" => Ok(self.number_literal_expr(&token.value, span)),
            "NAME" => Ok(self.sym_symbol(token.value.clone(), span)),
            // See the module doc comment's "Booleans" section — the
            // shared backend pre-binds exactly these two symbol names,
            // and `symbolic-ir` exports no `TRUE`/`FALSE` constant
            // (verified directly), so this uses bare string literals,
            // matching `maple-runtime`'s/`macsyma-compiler`'s own bridge.
            "KEYWORD" if token.value == "true" => Ok(self.sym_symbol("True".to_string(), span)),
            "KEYWORD" if token.value == "false" => Ok(self.sym_symbol("False".to_string(), span)),
            other => Err(MapleLowerError {
                message: format!("unexpected token `{other}` = {:?}", token.value),
                line: token.line,
                column: token.column,
            }),
        }
    }

    /// `if_expr = "if" expr "then" statement { "elif" expr "then"
    /// statement } [ "else" statement ] ( "end" "if" | "fi" )` — see the
    /// module doc comment's "if/elif/else" section for the full
    /// right-fold explanation. Retargets `maple-runtime::lower_if`'s
    /// exact collection-and-fold logic onto `semantic_ir::Expr`, with
    /// [`Self::check_elif_chain_length`] added as this crate's own
    /// proactive DoS guard (the native runtime does not need one, since
    /// it evaluates eagerly and never builds an unbounded tree ahead of
    /// use the way this frontend's pure-data lowering does).
    fn lower_if(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MapleLowerError> {
        let nodes: Vec<&GrammarASTNode> = child_nodes(node).collect();
        if nodes.len() < 2 {
            return Err(self.err_at(node, "if_expr must have at least one branch".to_string()));
        }
        let has_else = nodes.len() % 2 == 1;
        let branch_count = if has_else { (nodes.len() - 1) / 2 } else { nodes.len() / 2 };
        self.check_elif_chain_length(node, branch_count)?;

        let mut branches = Vec::with_capacity(branch_count);
        for i in 0..branch_count {
            let cond = self.lower_node(nodes[2 * i], depth + 1)?;
            let body = self.lower_node(nodes[2 * i + 1], depth + 1)?;
            branches.push((cond, body));
        }
        let else_body = if has_else {
            Some(self.lower_node(nodes[nodes.len() - 1], depth + 1)?)
        } else {
            None
        };

        let span = self.span_of(node);
        // Fold right-to-left: the innermost branch's "else slot" is the
        // final `else` body if present, otherwise absent (a bare 2-arg
        // `If`); every earlier `elif` branch wraps the accumulated
        // result as its own 3-arg `If`'s else-slot.
        let mut acc = else_body;
        for (cond, body) in branches.into_iter().rev() {
            acc = Some(match acc {
                Some(prev) => self.sym_apply(
                    self.sym_symbol_bare(IF, span.clone()),
                    vec![cond, body, prev],
                    span.clone(),
                ),
                None => self.sym_apply(self.sym_symbol_bare(IF, span.clone()), vec![cond, body], span.clone()),
            });
        }
        acc.ok_or_else(|| self.err_at(node, "if_expr produced no branches".to_string()))
    }

    /// `assignment = NAME ASSIGN ( arrow_def | expr ) | expr` — see the
    /// module doc comment's "Assignment" section for why this LHS is a
    /// bare `NAME` token, full stop, unlike Reduce's/Derive's own
    /// call-shaped-LHS disambiguation. By the time `lower_node` dispatches
    /// here (via `unwrap_single`), a genuine `assignment` node always has
    /// the 3-child `[NAME, ASSIGN, (arrow_def | expr)]` shape — the
    /// bare-`expr` alternative dissolves away before ever reaching this
    /// function. [`Self::lower_first_node`]'s fallback is defensive only,
    /// mirroring `maple-runtime::lower_assignment`'s identical shape.
    fn lower_assignment(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MapleLowerError> {
        let is_assign_form = node.children.len() == 3
            && as_token(&node.children[1]).is_some_and(|t| token_type(t) == "ASSIGN");
        if !is_assign_form {
            return self.lower_first_node(node, depth);
        }
        let name = match as_token(&node.children[0]) {
            Some(t) if token_type(t) == "NAME" => t.value.clone(),
            _ => return Err(self.err_at(node, "assignment lhs must be a bare NAME".to_string())),
        };
        let span = self.span_of(node);
        match &node.children[2] {
            ASTNodeOrToken::Node(n) if n.rule_name == "arrow_def" => self.lower_arrow_def(name, n, depth + 1),
            rhs => {
                let value = self.lower_child(rhs, depth + 1)?;
                Ok(self.sym_apply(
                    self.sym_symbol_bare(ASSIGN, span.clone()),
                    vec![self.sym_symbol_bare(name, span.clone()), value],
                    span,
                ))
            }
        }
    }

    /// `arrow_def = arrow_params ARROW expr` — MA09 §3's general-purpose
    /// function-definition spelling, `f := (x, y) -> e` / `f := x -> e`.
    /// Lowers to `Define[f, List[params...], body]`, mirroring
    /// `derive-to-semantic-ir`'s/`reduce-to-semantic-ir`'s identical
    /// `Define` shape for their own (differently-spelled) definition
    /// idioms — see the module doc comment's "Assignment" section.
    fn lower_arrow_def(
        &mut self,
        name: String,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Expr, MapleLowerError> {
        if node.children.len() != 3 {
            return Err(self.err_at(node, "malformed arrow_def node".to_string()));
        }
        let params_node = as_node(&node.children[0])
            .ok_or_else(|| self.err_at(node, "arrow_def is missing its parameter list".to_string()))?;
        let params = self.lower_arrow_params(params_node)?;
        let body = self.lower_child(&node.children[2], depth)?;
        let span = self.span_of(node);
        let params_list = self.sym_apply(self.sym_symbol_bare(LIST, span.clone()), params, span.clone());
        Ok(self.sym_apply(
            self.sym_symbol_bare(DEFINE, span.clone()),
            vec![self.sym_symbol_bare(name, span.clone()), params_list, body],
            span,
        ))
    }

    /// `arrow_params = NAME | LPAREN [ NAME { COMMA NAME } ] RPAREN` — a
    /// single bare parameter needs no parentheses, two-or-more do, and
    /// `()` (zero parameters) falls out of the optional inner list for
    /// free. Both shapes are handled uniformly by simply collecting every
    /// `NAME` token among the node's children in order — the
    /// `LPAREN`/`COMMA`/`RPAREN` tokens present in the parenthesised form
    /// are harmlessly filtered out, mirroring `maple-runtime::
    /// lower_arrow_params`'s identical logic. A flat `Vec`, not a folded
    /// tree, so [`Self::check_apply_arg_count`] bounds its length as an
    /// allocation-size backstop only, not a stack-recursion guard.
    fn lower_arrow_params(&mut self, node: &GrammarASTNode) -> Result<Vec<Expr>, MapleLowerError> {
        let names: Vec<&Token> = node
            .children
            .iter()
            .filter_map(as_token)
            .filter(|t| token_type(t) == "NAME")
            .collect();
        self.check_apply_arg_count(node, names.len())?;
        let mut params = Vec::with_capacity(names.len());
        for t in names {
            let span = self.token_span(t);
            params.push(self.sym_symbol(t.value.clone(), span));
        }
        Ok(params)
    }

    /// `logical_or`/`logical_and` — fold operands into an n-ary `Or`/`And`
    /// `SymApply` (a single flat apply carrying every operand at this
    /// precedence level, not a nested binary chain — safe to fold n-ary
    /// because every step in one chain shares the SAME operator).
    /// Mirrors `maple-runtime::lower_logical_chain`/`reduce-to-semantic-
    /// ir::lower_logical_chain` exactly, with [`Self::check_chain_length`]
    /// added proactively.
    fn lower_logical_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        head: &str,
    ) -> Result<Expr, MapleLowerError> {
        self.check_chain_length(node)?;
        let operands = self.lower_child_nodes(node, depth + 1)?;
        match operands.len() {
            0 => Err(self.err_at(node, "empty logical chain".to_string())),
            1 => Ok(operands.into_iter().next().unwrap()),
            _ => {
                let span = self.span_of(node);
                Ok(self.sym_apply(self.sym_symbol_bare(head, span.clone()), operands, span))
            }
        }
    }

    /// `logical_not = "not" logical_not | comparison ;`
    ///
    /// `and`/`or`/`not`/`if`/`then`/`elif`/`else`/`end`/`fi` are all
    /// matched in the grammar as `maple.tokens`' own `KEYWORD` token type
    /// (promoted from `NAME` by exact lowercase spelling — MA09 §3), so —
    /// mirroring `maple-runtime::lower_logical_not`'s identical check —
    /// this checks the token's literal *value*, not `effective_type_name()`
    /// (every keyword shares that one type name).
    fn lower_logical_not(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MapleLowerError> {
        let has_not = node
            .children
            .iter()
            .any(|c| as_token(c).is_some_and(|t| t.value == "not"));
        if !has_not {
            return self.lower_first_node(node, depth);
        }
        let inner = child_nodes(node)
            .next()
            .ok_or_else(|| self.err_at(node, "`not` with no operand".to_string()))?;
        let operand = self.lower_node(inner, depth + 1)?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(NOT, span.clone()), vec![operand], span))
    }

    /// `comparison = additive [ ( EQ | NEQ | LESS | GREATER | LE | GE )
    /// additive ] ;` — a single (non-chained) comparison, per MA09 §3's
    /// own disclosed simplification (reusing `reduce.grammar`'s own
    /// "flat, non-chaining tier" precedent rather than re-deriving one —
    /// see `maple.grammar`'s own design-decision comment). `=` is Maple's
    /// *equation* operator (`Equal`), never assignment — `:=` alone owns
    /// that role. Unlike Reduce's word-keyword `neq`, Maple's not-equal
    /// is the symbolic `NEQ` (`<>`) token TYPE, so — unlike
    /// `reduce-to-semantic-ir::comparison_head`'s value-based check for
    /// `neq` — every arm here is purely type-based.
    fn lower_comparison(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MapleLowerError> {
        let Some(op_index) = node
            .children
            .iter()
            .position(|c| as_token(c).is_some_and(|t| comparison_head(t).is_some()))
        else {
            return self.lower_first_node(node, depth);
        };
        if op_index == 0 || op_index + 1 >= node.children.len() {
            return Err(self.err_at(node, "malformed comparison node".to_string()));
        }
        let head = comparison_head(as_token(&node.children[op_index]).unwrap()).unwrap();
        let lhs = self.lower_child(&node.children[op_index - 1], depth + 1)?;
        let rhs = self.lower_child(&node.children[op_index + 1], depth + 1)?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(head, span.clone()), vec![lhs, rhs], span))
    }

    /// `additive`/`multiplicative` — a left-associative binary chain of
    /// `+`/`-`/`*`/`/`. Must fold pairwise (not n-ary, unlike the logical
    /// chains) since a single chain can mix operators: `a - b - c` folds
    /// left into `Sub(Sub(a, b), c)`; `a + b - c` into `Sub(Add(a, b),
    /// c)`. Mirrors `maple-runtime::lower_binary_chain`/`reduce-to-
    /// semantic-ir::lower_binary_chain` exactly (identical grammar
    /// shape). [`Self::check_chain_length`] guards the fold, same
    /// reasoning as [`Self::lower_logical_chain`]'s.
    fn lower_binary_chain(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MapleLowerError> {
        self.check_chain_length(node)?;
        let mut children = node.children.iter();
        let first = children
            .next()
            .ok_or_else(|| self.err_at(node, "empty binary chain".to_string()))?;
        let mut result = self.lower_child(first, depth + 1)?;
        while let Some(op_child) = children.next() {
            let head = as_token(op_child)
                .and_then(|t| binary_head(token_type(t)))
                .ok_or_else(|| self.err_at(node, "expected a binary operator".to_string()))?;
            let rhs = children
                .next()
                .ok_or_else(|| self.err_at(node, "binary operator with no right operand".to_string()))?;
            let rhs_expr = self.lower_child(rhs, depth + 1)?;
            let span = self.span_of(node);
            result = self.sym_apply(self.sym_symbol_bare(head, span.clone()), vec![result, rhs_expr], span);
        }
        Ok(result)
    }

    /// `unary = MINUS unary | power ;` — MA09 §3 lists only unary `-` (no
    /// unary `+`, matching Reduce's/Derive's identical asymmetry) — a
    /// leading `-` is `Neg`; otherwise it is the inner `power`. Genuinely
    /// self-referential (prefix-recursive), bounded by `maple-parser`'s
    /// own depth cap — see the module doc comment's recursion-hardening
    /// section for why no additional chain-length guard is needed here.
    fn lower_unary(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MapleLowerError> {
        match node.children.len() {
            1 => self.lower_child(&node.children[0], depth + 1),
            2 => {
                let operand = self.lower_child(&node.children[1], depth + 1)?;
                let span = self.span_of(node);
                Ok(self.sym_apply(self.sym_symbol_bare(NEG, span.clone()), vec![operand], span))
            }
            _ => Err(self.err_at(node, "malformed unary node".to_string())),
        }
    }

    /// `power = postfix [ CARET unary ] ;` — right-associative `^`.
    /// Unlike `reduce-to-semantic-ir::lower_power` (which accepts either
    /// `CARET` or `POW`, since Reduce's manual documents both as one
    /// tier), Maple's own grammar has no `**` synonym at all (MA09 §3/§4;
    /// `maple.tokens` has no `POW` token), so this only ever matches
    /// `CARET` — mirroring `maple-runtime::lower_power`'s identical
    /// single-token acceptance.
    fn lower_power(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MapleLowerError> {
        match node.children.len() {
            1 => self.lower_child(&node.children[0], depth + 1),
            3 => {
                let is_caret = as_token(&node.children[1]).is_some_and(|t| token_type(t) == "CARET");
                if !is_caret {
                    return Err(self.err_at(node, "malformed power node: expected CARET".to_string()));
                }
                let lhs = self.lower_child(&node.children[0], depth + 1)?;
                let rhs = self.lower_child(&node.children[2], depth + 1)?;
                let span = self.span_of(node);
                Ok(self.sym_apply(self.sym_symbol_bare(POW, span.clone()), vec![lhs, rhs], span))
            }
            _ => Err(self.err_at(node, "malformed power node".to_string())),
        }
    }

    /// `postfix = atom [ LPAREN [ arglist ] RPAREN ] ;` — a single
    /// OPTIONAL call suffix, deliberately narrower than Reduce's/Derive's
    /// REPEATED `{ LPAREN [arglist] RPAREN }` chain. See the module doc
    /// comment's "postfix is NOT chainable" section: because at most one
    /// call suffix can ever appear, there is no
    /// `check_postfix_chain_length`-equivalent guard anywhere in this
    /// function — the axis it would guard is structurally impossible in
    /// this grammar.
    ///
    /// The head runs through [`Self::build_application`] so `diff`/`int`
    /// become the IR heads `symbolic-vm` already has handlers for; any
    /// other head (a user-defined function, or a builtin this subset
    /// doesn't bridge, like `sin`/`solve`/`piecewise`) passes through
    /// unchanged and stays a harmless unevaluated symbolic call — mirrors
    /// `maple-runtime::lower_postfix`'s identical head-bridging step.
    fn lower_postfix(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MapleLowerError> {
        let base = self.lower_child(
            node.children
                .first()
                .ok_or_else(|| self.err_at(node, "postfix has no base".to_string()))?,
            depth + 1,
        )?;

        let has_call = node
            .children
            .get(1)
            .and_then(as_token)
            .is_some_and(|t| token_type(t) == "LPAREN");
        if !has_call {
            return Ok(base);
        }

        let args = node
            .children
            .get(2)
            .and_then(as_node)
            .filter(|n| n.rule_name == "arglist")
            .map(|n| self.lower_arglist(n, depth + 1))
            .transpose()?
            .unwrap_or_default();
        self.check_apply_arg_count(node, args.len())?;
        Ok(self.build_application(base, args, node))
    }

    /// Apply `head` to `args`, bridging a lowercase builtin surface
    /// function name (`diff`, `int`) to its canonical IR head via
    /// [`standard_function`] — mirrors `maple-runtime::lower_postfix`'s
    /// `canonical_head` step exactly. There is no associative n-ary
    /// left-fold here (unlike Wolfram's `build_application`): Maple has
    /// no explicit-head-application sugar analogous to Wolfram's
    /// `Plus[1, 2, 3]`, so this is a plain wrap.
    fn build_application(&mut self, head: Expr, args: Vec<Expr>, node: &GrammarASTNode) -> Expr {
        let span = self.span_of(node);
        let canonical_head = match &head {
            Expr::SymSymbol { name, span: head_span } => standard_function(name)
                .map(|c| self.sym_symbol_bare(c, head_span.clone()))
                .unwrap_or_else(|| head.clone()),
            _ => head,
        };
        self.sym_apply(canonical_head, args, span)
    }

    /// `arglist = expr { COMMA expr } ;` — lower each comma-separated
    /// argument. An arglist is a flat `Vec`, not a folded tree, so it has
    /// no stack-recursion risk analogous to the binary-chain rules — the
    /// caller applies [`Self::check_apply_arg_count`] to bound its raw
    /// length as a modest defense-in-depth cap on allocation size.
    fn lower_arglist(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Vec<Expr>, MapleLowerError> {
        self.lower_child_nodes(node, depth)
    }

    /// `atom = NUMBER | NAME | "true" | "false" | list_literal |
    /// set_literal | group ;` In practice [`unwrap_single`] already
    /// dissolves a single-child `atom` node before `lower_node`'s
    /// dispatch ever sees rule_name `"atom"` (every alternative here
    /// matches to exactly one child), so this function mirrors
    /// `maple-runtime::lower_atom`'s identical defensive shape rather
    /// than being load-bearing for the common case.
    fn lower_atom(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MapleLowerError> {
        if let Some(child) = child_nodes(node).next() {
            if matches!(child.rule_name.as_str(), "list_literal" | "set_literal" | "group") {
                return self.lower_node(child, depth + 1);
            }
        }
        let tokens: Vec<&Token> = node.children.iter().filter_map(as_token).collect();
        match tokens.as_slice() {
            [single] => self.lower_token(single),
            _ => Err(self.err_at(
                node,
                format!(
                    "unrecognised atom token shape: {:?}",
                    tokens.iter().map(|t| &t.value).collect::<Vec<_>>()
                ),
            )),
        }
    }

    /// `list_literal = LBRACKET [ arglist ] RBRACKET ;` — MA09 §3's `[a,
    /// b, c]` (square brackets, ordered, duplicates kept). Lowers to
    /// `List[...]`, the shared, already-handled head every CAS-family
    /// sibling in this repo reuses — mirrors `maple-runtime::
    /// lower_list_literal`'s identical logic.
    fn lower_list_literal(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MapleLowerError> {
        let args = match child_nodes(node).find(|n| n.rule_name == "arglist") {
            Some(arglist_node) => self.lower_arglist(arglist_node, depth + 1)?,
            None => vec![],
        };
        self.check_apply_arg_count(node, args.len())?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(LIST, span.clone()), args, span))
    }

    /// `set_literal = LBRACE [ arglist ] RBRACE ;` — MA09 §3's `{a, b,
    /// c}` (curly braces, unordered, duplicates removed *in real Maple*).
    /// Lowers to the new [`SET`] head — see the module doc comment's
    /// "`Set`" section for the disclosed evaluation-time gap (this
    /// frontend never evaluates anything, so the gap is moot here; only
    /// the *shape* matters for pure data construction/codegen). Mirrors
    /// `maple-runtime::lower_set_literal`'s identical logic, differing
    /// from [`Self::lower_list_literal`] only in which head/bracket.
    fn lower_set_literal(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MapleLowerError> {
        let args = match child_nodes(node).find(|n| n.rule_name == "arglist") {
            Some(arglist_node) => self.lower_arglist(arglist_node, depth + 1)?,
            None => vec![],
        };
        self.check_apply_arg_count(node, args.len())?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(SET, span.clone()), args, span))
    }

    /// `group = LPAREN expr RPAREN ;` — grouping only.
    fn lower_group(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MapleLowerError> {
        let inner = child_nodes(node)
            .next()
            .ok_or_else(|| self.err_at(node, "empty group `( )`".to_string()))?;
        self.lower_node(inner, depth + 1)
    }

    // -------------------------------------------------------------------
    // Small constructors (each observes the feature the validator's own
    // `check_expr` requires for that node kind -- see `semantic-ir/src/
    // validator.rs`, the ground truth this must match exactly).
    // -------------------------------------------------------------------

    fn sym_symbol(&mut self, name: String, span: Span) -> Expr {
        self.observed.add(Feature::SymbolicExpr);
        Expr::SymSymbol { name, span }
    }

    /// Build a `SymSymbol` for a *head* name, or any other
    /// internally-constructed symbol that is always immediately wrapped
    /// in a [`Self::sym_apply`] call — which itself observes the
    /// feature — so this helper does not need to (identical shape to
    /// [`Self::sym_symbol`], named separately only so call sites make
    /// their intent legible; mirrors every sibling SIR23 frontend's
    /// identically-named helper).
    fn sym_symbol_bare(&self, name: impl Into<String>, span: Span) -> Expr {
        Expr::SymSymbol {
            name: name.into(),
            span,
        }
    }

    fn sym_apply(&mut self, head: Expr, args: Vec<Expr>, span: Span) -> Expr {
        self.observed.add(Feature::SymbolicExpr);
        Expr::SymApply {
            head: Box::new(head),
            args,
            span,
        }
    }

    /// Parse a `NUMBER` lexeme into an `IntLit` or `FloatLit` (a `.`,
    /// `e`, or `E` means a real; otherwise an integer, matching
    /// `maple-runtime::lower::lower_number`'s identical rule — the
    /// `maple.tokens` `NUMBER` regex is identical to Reduce's/Derive's/
    /// Macsyma's own). An integer lexeme too large for `i64` falls back
    /// to a float rather than silently truncating.
    ///
    /// **Must** be an instance method, not a free function: every branch
    /// that constructs a `FloatLit` calls `self.observed.add(Feature::
    /// Floats)` immediately. This is a confirmed, previously-shipped bug
    /// in both `matlab-to-semantic-ir` and `wolfram-to-semantic-ir` (their
    /// number-literal helpers were free functions with no access to
    /// `observed`, so a float-literal-only module failed
    /// `semantic_ir::validate()`), fixed proactively in
    /// `macsyma-to-semantic-ir`/`derive-to-semantic-ir`/
    /// `reduce-to-semantic-ir` and carried forward here.
    fn number_literal_expr(&mut self, text: &str, span: Span) -> Expr {
        if text.contains('.') || text.contains('e') || text.contains('E') {
            self.observed.add(Feature::Floats);
            Expr::FloatLit {
                value: text.parse::<f64>().unwrap_or(0.0),
                span,
            }
        } else {
            match text.parse::<i64>() {
                Ok(v) => Expr::IntLit { value: v, span },
                Err(_) => {
                    self.observed.add(Feature::Floats);
                    Expr::FloatLit {
                        value: text.parse::<f64>().unwrap_or(0.0),
                        span,
                    }
                }
            }
        }
    }

    // -------------------------------------------------------------------
    // Guards
    // -------------------------------------------------------------------

    /// Reject a same-precedence operator chain (`additive`/
    /// `multiplicative`/`logical_or`/`logical_and`) with more than
    /// `MAX_EXPR_DEPTH` operands.
    ///
    /// Maple's grammar, like Wolfram's, Macsyma's, Derive's, and Reduce's,
    /// collapses a flat run of same-precedence operators into ONE CST
    /// node with many children rather than nesting through parens, so a
    /// long unparenthesized chain (`1 + 1 + ... + 1`, thousands of terms)
    /// never trips the ordinary grammar-nesting depth guard
    /// (`maple-parser`'s `MAX_RULE_DEPTH`, which counts *nesting*, not
    /// the length of one flat repetition — confirmed directly in that
    /// crate's own doc comment). But folding N operands
    /// left-associatively still builds an N-deep *binary* `Expr` tree,
    /// and that tree's own depth is what every later recursive pass over
    /// it pays for regardless of how cheaply each fold step was.
    fn check_chain_length(&self, node: &GrammarASTNode) -> Result<(), MapleLowerError> {
        let operand_count = node
            .children
            .iter()
            .filter(|c| matches!(c, ASTNodeOrToken::Node(_)))
            .count();
        if operand_count > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!(
                    "expression chain too long ({operand_count} operands, exceeds \
                     {MAX_EXPR_DEPTH})"
                ),
            ));
        }
        Ok(())
    }

    /// Reject an `if_expr` with more than `MAX_EXPR_DEPTH` total branches
    /// (the initial `if` branch plus every `elif` branch). See the module
    /// doc comment's "if/elif/else" section: the `{ "elif" ... }`
    /// repetition is a flat CST shape (zero parser-stack cost regardless
    /// of width), but [`Self::lower_if`]'s right-fold still builds a
    /// `branch_count`-deep nested `If` `Expr` tree — the identical DoS
    /// shape [`Self::check_chain_length`] guards for the flat operator
    /// chains, applied here to the one construct genuinely new relative
    /// to `reduce-to-semantic-ir`'s simpler 2-or-3-child `if`.
    fn check_elif_chain_length(&self, node: &GrammarASTNode, branch_count: usize) -> Result<(), MapleLowerError> {
        if branch_count > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("if/elif chain too long ({branch_count} branches, exceeds {MAX_EXPR_DEPTH})"),
            ));
        }
        Ok(())
    }

    /// Cap the argument count of a single `f(…)` application, the
    /// element count of a `[…]`/`{…}` list/set literal, or the parameter
    /// count of an `arrow_def`'s `arrow_params`. None of these fold into
    /// a nested tree (all stay a flat `Vec<Expr>`), so this is not a
    /// stack-recursion guard — it is a modest defense-in-depth cap on a
    /// single allocation's size, using the same `MAX_EXPR_DEPTH` bound
    /// for consistency rather than inventing new constants per call site
    /// (mirrors `reduce-to-semantic-ir::check_apply_arg_count`'s
    /// identical reuse across `arglist`/`list_literal`/`group_expr`).
    fn check_apply_arg_count(&self, node: &GrammarASTNode, count: usize) -> Result<(), MapleLowerError> {
        if count > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("too many arguments ({count}, exceeds {MAX_EXPR_DEPTH})"),
            ));
        }
        Ok(())
    }

    // -------------------------------------------------------------------
    // Small helpers
    // -------------------------------------------------------------------

    fn lower_first_node(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MapleLowerError> {
        let child = child_nodes(node)
            .next()
            .ok_or_else(|| self.err_at(node, format!("`{}` has no expression child", node.rule_name)))?;
        self.lower_node(child, depth + 1)
    }

    fn lower_child(&mut self, child: &ASTNodeOrToken, depth: usize) -> Result<Expr, MapleLowerError> {
        match child {
            ASTNodeOrToken::Node(node) => self.lower_node(node, depth),
            ASTNodeOrToken::Token(token) => self.lower_token(token),
        }
    }

    fn lower_child_nodes(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Vec<Expr>, MapleLowerError> {
        child_nodes(node).map(|n| self.lower_node(n, depth)).collect()
    }

    fn span_of(&self, node: &GrammarASTNode) -> Span {
        Span::new(
            FILE,
            node.start_line.unwrap_or(1),
            node.start_column.unwrap_or(1),
            node.end_line.unwrap_or(node.start_line.unwrap_or(1)),
            node.end_column.unwrap_or(node.start_column.unwrap_or(1)),
        )
    }

    fn token_span(&self, token: &Token) -> Span {
        Span::point(FILE, token.line, token.column)
    }

    fn err_at(&self, node: &GrammarASTNode, message: String) -> MapleLowerError {
        MapleLowerError {
            message,
            line: node.start_line.unwrap_or(1),
            column: node.start_column.unwrap_or(1),
        }
    }
}

// ---------------------------------------------------------------------------
// Free helpers (no `&mut self` needed)
// ---------------------------------------------------------------------------

/// Collect the *node* children of `node` (dropping tokens).
fn child_nodes(node: &GrammarASTNode) -> impl Iterator<Item = &GrammarASTNode> {
    node.children.iter().filter_map(as_node)
}

fn as_node(child: &ASTNodeOrToken) -> Option<&GrammarASTNode> {
    match child {
        ASTNodeOrToken::Node(node) => Some(node),
        ASTNodeOrToken::Token(_) => None,
    }
}

fn as_token(child: &ASTNodeOrToken) -> Option<&Token> {
    match child {
        ASTNodeOrToken::Token(token) => Some(token),
        ASTNodeOrToken::Node(_) => None,
    }
}

fn token_type(token: &Token) -> &str {
    token.effective_type_name()
}

/// Map an arithmetic token type to its canonical IR head — the exact
/// heads `symbolic_vm::handlers::build_handler_table` wires and
/// `maple-runtime::lower` itself already uses. Note `TIMES`, matching
/// `maple.tokens`'s own spelling of the multiplication token (same as
/// Reduce's/Derive's `TIMES`, not Macsyma's `STAR`).
fn binary_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "PLUS" => Some(ADD),
        "MINUS" => Some(SUB),
        "TIMES" => Some(MUL),
        "SLASH" => Some(DIV),
        _ => None,
    }
}

/// Map a comparison token to its canonical IR head. Every comparison
/// operator in this subset is a symbolic (punctuation) token TYPE —
/// unlike Reduce's `neq` keyword (matched by literal *value*), Maple
/// spells not-equal `<>` (`NEQ`), so no value-based check is needed
/// alongside the type-based ones — mirrors `maple-runtime::
/// comparison_head`'s identical purely-type-based table.
fn comparison_head(token: &Token) -> Option<&'static str> {
    match token_type(token) {
        "EQ" => Some(EQUAL),
        "NEQ" => Some(NOT_EQUAL),
        "LESS" => Some(LESS),
        "GREATER" => Some(GREATER),
        "LE" => Some(LESS_EQUAL),
        "GE" => Some(GREATER_EQUAL),
        _ => None,
    }
}

/// Bridge a Maple *surface* builtin call-head name to the canonical IR
/// head it's already implemented under. Per the module doc comment's
/// "diff/int" section, this subset's *only* such bridge is calculus
/// (`diff`→`D`, `int`→`Integrate`) — mirrors `maple-runtime::
/// surface_head_to_ir`'s identical two-entry table exactly. A head not
/// in this table (a user-defined function, or any of MA09 §4's deferred
/// `cas-*` surface, e.g. `sin`/`solve`/`piecewise`) is returned
/// unchanged, so it stays a harmless unevaluated symbolic call.
fn standard_function(name: &str) -> Option<&'static str> {
    match name {
        "diff" => Some(D),
        "int" => Some(INTEGRATE),
        _ => None,
    }
}

/// Measure `expr`'s true tree depth **iteratively**, using an explicit
/// heap-allocated work stack rather than native recursion, so calling
/// this can never itself overflow the stack no matter how deep `expr`
/// already is. Building a deeply-nested `Box`-based tree only costs heap
/// space (each construction step is O(1) stack); the risk this guards
/// against is only in *walking* it recursively afterward.
///
/// Returns `None` as soon as the depth is certain to exceed
/// `MAX_EXPR_DEPTH`, `Some(depth)` otherwise.
///
/// Only needs a match arm for [`Expr::SymApply`] (recursing into `head`
/// and `args`) — every other `Expr` variant is a leaf for this crate's
/// purposes, since (per the module doc comment's scope note) this crate
/// can never construct a `SymPatternBlank`/`SymPatternNamed`/`SymRule`/
/// `SymReplaceAll` node in the first place (Maple's grammar has no
/// pattern-matching or rewrite-rule syntax at all). `If`/`Assign`/
/// `Define`/`Set` are all `SymApply` with a different head symbol or
/// bracket, not new `Expr` variants, so this one match arm already
/// covers them.
///
/// This is the authoritative depth check every other guard in this file
/// (`MAX_EXPR_DEPTH`'s recursion-depth parameter, [`Lowerer::
/// check_chain_length`], [`Lowerer::check_elif_chain_length`]) is only
/// an early, cheap approximation of — those guards are each scoped to
/// one grammar node and do not compose across nested `(...)` boundaries
/// (see `wolfram-to-semantic-ir`'s `CHANGELOG.md` for the security-review
/// finding this mirrors). Called once per top-level statement in
/// [`Lowerer::lower_file`], so no tree this crate hands to a caller can
/// ever actually exceed `MAX_EXPR_DEPTH`, regardless of how its
/// construction was composed.
fn measure_depth_iterative(expr: &Expr) -> Option<usize> {
    let mut stack: Vec<(&Expr, usize)> = vec![(expr, 0)];
    let mut max_depth = 0;
    while let Some((node, d)) = stack.pop() {
        if d > MAX_EXPR_DEPTH {
            return None;
        }
        max_depth = max_depth.max(d);
        if let Expr::SymApply { head, args, .. } = node {
            stack.push((head, d + 1));
            for a in args {
                stack.push((a, d + 1));
            }
        }
    }
    Some(max_depth)
}

/// Tear down a rejected, pathologically-deep `Expr` tree **iteratively**,
/// so freeing it can never itself overflow the stack — unlike simply
/// letting `expr` fall out of scope, which invokes `Expr`/`Box<Expr>`'s
/// ordinary *recursive* compiler-derived `Drop` glue. `wolfram-to-
/// semantic-ir`'s security-review history confirmed this is a real,
/// exploitable crash (empirically, via an isolated subprocess) — moving a
/// pathologically deep tree past [`measure_depth_iterative`]'s detection
/// only to then let it drop normally just relocates the same native stack
/// overflow from "walking the tree forward" to "walking it backward".
///
/// The technique: take ownership of `expr`, and for the one nested
/// recursive field this crate's trees can ever have (`SymApply`'s
/// `head`/`args`), *move* those fields out onto an explicit heap-
/// allocated work stack instead of leaving them in place to be dropped as
/// part of the outer match's scrutinee. Only needs the `Expr::SymApply`
/// arm for the same scope reason [`measure_depth_iterative`] documents.
fn drop_iterative(expr: Expr) {
    let mut stack: Vec<Expr> = vec![expr];
    while let Some(node) = stack.pop() {
        if let Expr::SymApply { head, args, .. } = node {
            stack.push(*head);
            stack.extend(args);
        }
        // `node`'s own shell drops here — shallowly, since its only
        // nested `Expr` field (if any) was already moved out onto
        // `stack` above.
    }
}

enum Unwrapped<'a> {
    Node(&'a GrammarASTNode),
    Token(&'a Token),
}

/// Peel away single-child wrapper nodes until we reach a node with
/// structure (or a leaf token). A precedence-cascade rule that did not
/// apply its operator still emits its own node with exactly one child —
/// this skips straight to the rule that actually matters (mirrors
/// `maple-runtime::unwrap_single`/`reduce-to-semantic-ir::unwrap_single`
/// exactly — the shared `parser::GrammarParser` engine's node shape is
/// identical across every grammar built on it).
fn unwrap_single(mut node: &GrammarASTNode) -> Unwrapped<'_> {
    loop {
        if node.children.len() != 1 {
            return Unwrapped::Node(node);
        }
        match &node.children[0] {
            ASTNodeOrToken::Node(child) => node = child,
            ASTNodeOrToken::Token(token) => return Unwrapped::Token(token),
        }
    }
}
