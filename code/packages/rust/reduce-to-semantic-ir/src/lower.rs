//! The lowering pass from `coding_adventures_reduce_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **v0.1.0**.
//!
//! This is the **fourth** frontend to target
//! [SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
//! symbolic-expression/pattern-matching domain extension of the SIR10
//! narrow-waist IR (Stream B of
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)). The first,
//! `wolfram-to-semantic-ir`, and the second, `macsyma-to-semantic-ir`, are
//! this crate's design templates for the "everything is data" decision —
//! read `wolfram-to-semantic-ir::lower`'s module doc comment first. The
//! third, `derive-to-semantic-ir`, is the closer template: everything
//! below assumes that context and only calls out where Reduce's grammar
//! differs.
//!
//! # Retargeting `reduce-runtime`, not starting from scratch
//!
//! `reduce-runtime` already walks this exact CST and compiles it to
//! `symbolic_ir::IRNode` (`Symbol`/`Integer`/`Float`/`Apply` — see that
//! crate's `src/lower.rs`, in particular its module doc comment). **Much
//! of this module is a direct copy of `derive-runtime::lower`'s shape**
//! (Reduce's own module doc comment says so explicitly): Reduce, like
//! Derive, has no `f[x]`-universal-application syntax (ordinary parens,
//! `f(x)`, double as grouping, call, AND array-subscript read) and no
//! pattern/rewrite-rule vocabulary in this subset (MA08 §4 defers `let`
//! rules to later work). So this crate's job is the same mechanical
//! retarget `derive-to-semantic-ir` already did: walk the same CST,
//! dispatch on the same rule names `reduce-runtime::lower::lower_node`
//! already uses, and construct `semantic_ir::Expr::{SymSymbol,SymApply}`
//! wherever that lowering constructs `symbolic_ir::IRNode::{Symbol,Apply}`.
//! Literals (`Integer`/`Float`) reuse SIR10/SIR16's `IntLit`/`FloatLit`
//! directly — Reduce has no `STRING` token at all (`reduce.tokens`'s
//! `escapes: none`), so, like Derive, this crate never constructs
//! `Expr::StrLit` either.
//!
//! Unlike Derive, Reduce's grammar has three genuinely new constructs with
//! no Derive analogue at all: an **expression-shaped `if`** (`if_expr`,
//! [`Lowerer::lower_if`]), a **group statement** `<< s1; s2; ... >>`
//! (`group_expr`, [`Lowerer::lower_group_expr`], MA08 §3's
//! `CompoundExpression`), and **cons** (`a . b`, `cons`,
//! [`Lowerer::lower_cons`]/[`Lowerer::fold_cons`]). All three retarget
//! `reduce-runtime::lower`'s own `lower_if`/`lower_group_expr`/
//! `lower_cons`/`fold_cons` functions, whose exact logic is mirrored here.
//! Reduce also has lists (`{a, b, c}`, curly braces — MA08 §1/§3, NOT
//! Derive's square-bracket `[a,b,c]` vector/matrix literal), which are
//! flat only (no row/matrix shape the way Derive's `vector`/`row` split
//! needs — matrices are out of Reduce's scope, MA08 §4), so
//! [`Lowerer::lower_list_literal`] reuses [`Lowerer::lower_arglist`]
//! directly instead of Derive's row-counting logic.
//!
//! # Scope (v0.1.0) — no pattern-matching or rewrite-rule syntax at all
//!
//! **Verified empirically against `reduce.grammar`/`reduce.tokens`
//! (`code/grammars/reduce/`), not just trusted from `reduce-runtime`'s own
//! doc comment**, exactly mirroring `derive-to-semantic-ir`'s own
//! verification discipline for Derive: grepping both files for
//! pattern-syntax tokens (`_` blank, `->`/`:>` rule arrows, `/.`/`//.`
//! replacement operators) finds none — `reduce.tokens` declares no such
//! token, and `reduce.grammar`'s only assignment-shaped construct is
//! `assignment = logical_or [ ASSIGN expr ]` (`:=`, used for both plain
//! assignment and procedure definition — see below). This confirms
//! `reduce-runtime`'s own claim ("no pattern/rewrite-rule vocabulary in
//! this subset (MA08 §4 defers `let` rules)") is accurate for the grammar
//! as it currently ships.
//!
//! This crate therefore **only ever constructs [`Expr::SymSymbol`] and
//! [`Expr::SymApply`]** (plus the reused `IntLit`/`FloatLit` literal
//! nodes) — it never constructs `SymPatternBlank`/`SymPatternNamed`/
//! `SymRule`/`SymReplaceAll`, and it never observes
//! `Feature::PatternMatching`. One concrete consequence, mirroring
//! `derive-to-semantic-ir`'s identical note: [`measure_depth_iterative`]/
//! [`drop_iterative`] below only need a match arm for [`Expr::SymApply`]
//! (recursing into `head` and `args`) — every other `Expr` variant is a
//! leaf for this crate's purposes. `if`/`<< ... >>`/cons all lower to
//! *this same* `SymApply` variant (only the head symbol's name differs —
//! `If`/`CompoundExpression`/`Cons` are not new `Expr` variants), so this
//! holds even though this crate covers more surface than Derive's.
//!
//! # A REAL divergence from MA08 §3's own prose: arithmetic head *names*
//!
//! MA08 §3's table spells the "Lowers to" column for arithmetic as
//! `Plus`/`Subtract`/`Times`/`Power`, and even expands `a / b` to
//! `Times[a, Power[b, -1]]` and `-a` to `Times[-1, a]`. **None of those
//! spellings exist in `symbolic-ir`** (confirmed by grepping it directly:
//! `grep -n '"Plus"\|"Subtract"\|"Times"\|"Power"' symbolic-ir/src/lib.rs`
//! returns nothing) — `reduce-runtime`'s own module doc comment discloses
//! this exact divergence and confirms it empirically the same way. What
//! actually exists, and what `symbolic_vm::handlers::build_handler_table`
//! actually wires handlers for, is [`ADD`]/[`SUB`]/[`MUL`]/[`DIV`]/
//! [`POW`]/[`NEG`] — the *exact* heads `derive-to-semantic-ir` and
//! `macsyma-to-semantic-ir` already lower `+`/`-`/`*`/`/`/`^`/unary-`-` to,
//! and the exact heads `reduce-runtime::lower` itself uses (not the
//! literal `Plus`/`Subtract`/`Times`/`Power`/expansion MA08 §3's prose
//! describes). This crate uses those SAME real heads, reusing the
//! identical `symbolic_ir` constants `reduce-runtime` imports, so all four
//! symbolic-CAS SIR23 frontends (Wolfram, Macsyma, Derive, Reduce) agree
//! on every arithmetic result — a disclosed, deliberate divergence from
//! the spec's literal prose (already corrected in MA08's own
//! changelog-style note), not new-head invention.
//!
//! # A REAL gap: several MA08 §3 heads have no handler in `symbolic-vm` at all
//!
//! `reduce-runtime`'s own module doc comment discloses that
//! `CompoundExpression`, `First`, `Second`, `Third`, `Rest`, `Part`,
//! `Append`, `Reverse` (and `Cons`, for the one cons shape that doesn't
//! fold away — see [`Lowerer::fold_cons`]) have **no** evaluation handler
//! in the shared `symbolic_vm::handlers::build_handler_table` — `reduce-
//! runtime` does not build a bespoke `Backend` to add them, reusing the
//! shared backend unchanged per its own design mandate, so these calls
//! evaluate as an ordinary unknown-head no-op fallback at runtime. This is
//! **largely moot for this crate**: this frontend never evaluates
//! anything (per the "everything is data" design shared with every SIR23
//! frontend), so a `SymApply{head: "First", ...}` node is valid,
//! executable SIR23 data regardless of whether any *runtime* currently
//! has a handler for it. What DOES matter here is spelling: this crate
//! reuses the exact same head spelling `reduce-runtime` uses for these,
//! via locally-defined `pub const`s ([`COMPOUND_EXPRESSION`], [`CONS`],
//! [`FIRST`], [`SECOND`], [`THIRD`], [`REST`], [`PART`], [`APPEND`],
//! [`REVERSE`]) rather than string literals scattered through the match
//! arms below — `symbolic_ir` doesn't export these (they are not
//! `Backend`-agnostic canonical heads the way `ADD`/`SUB`/… are), and
//! neither does `reduce-runtime` itself (this crate does not depend on
//! `reduce-runtime`, per this repo's SIR23-frontend convention of
//! consuming only the parser + shared IR crates), so each is redefined
//! locally, spelled to match `reduce-runtime`'s own constants for future
//! consistency — exactly the same "locally-defined pub const, spelled to
//! match a sibling crate's constant" pattern `macsyma-to-semantic-ir`
//! needed for its own `WHILE_HEAD`/`FOR_EACH_HEAD`/`BLOCK_HEAD`/
//! `RETURN_HEAD` constants (per `derive-to-semantic-ir`'s own module doc
//! comment note about that).
//!
//! # `:=` disambiguation has no operator to branch on
//!
//! Like `derive-runtime`'s and `reduce-runtime`'s own identical note:
//! Reduce's grammar has exactly ONE assignment token, `ASSIGN` (`:=`) —
//! `x := 5` and `h(l, m) := l - 2*m` are syntactically identical until
//! this lowering step. [`Lowerer::lower_assignment`] disambiguates purely
//! by the *lowered LHS's shape*: `SymApply{head: SymSymbol(_), ..}` →
//! `Define`, anything else → `Assign`. Unlike `derive.grammar`'s
//! self-referential `assignment = logical_or [ ASSIGN assignment ]`,
//! `reduce.grammar`'s right-hand side is the WIDER `expr` production
//! (`assignment = logical_or [ ASSIGN expr ]`) — a deliberate,
//! grammar-level divergence `reduce.grammar`'s own comment discloses, not
//! an oversight this crate needs to work around: Reduce's `if`/`<<...>>`
//! are genuinely usable as expressions (MA08 §3, with no Derive analogue
//! at all), so `x := if a>0 then 1 else -1` and `x := << a:=1; a+1 >>`
//! both parse and lower directly through the same dispatch this module
//! already has for `if_expr`/`group_expr` — no special-casing needed in
//! [`Lowerer::lower_assignment`] itself, since it just lowers whatever
//! node sits in the RHS child slot, whatever rule matched there.
//!
//! # Recursion-depth hardening — carried over proactively, not discovered
//!
//! `wolfram-to-semantic-ir`'s `CHANGELOG.md` documents four rounds of
//! security review that each found a real, adversarially-confirmed native
//! stack-overflow gap, and every sibling SIR23 frontend since
//! (`macsyma-to-semantic-ir`, `derive-to-semantic-ir`) carries every one
//! of those hardening mechanisms over from day one rather than
//! rediscovering them. This crate does the same, even though neither
//! `reduce-parser` nor `reduce-runtime` (the retarget source) applies any
//! of these guards themselves — they are a `*-to-semantic-ir`-frontend-
//! specific defense, not part of the native pipeline:
//!
//! - [`MAX_EXPR_DEPTH`] bounds this crate's own CST-walking recursion.
//! - [`Lowerer::check_chain_length`] caps every flat, same-precedence
//!   operator-chain fold (`additive`/`multiplicative`/`logical_or`/
//!   `logical_and`) before any tree is built — `reduce-parser`'s own
//!   `MAX_RULE_DEPTH` doc comment confirms these ARE flat EBNF
//!   repetitions in this grammar (not right-recursion), measured directly
//!   against an uncapped parser (zero crashes up to one million repeated
//!   items — width alone is not a recursion-depth risk in the *parser*,
//!   but folding N flat operands into an N-deep binary `Expr` tree here
//!   still is, for every later recursive pass over that tree).
//! - [`Lowerer::check_postfix_chain_length`] caps chained call
//!   application (`f(x)(y)(z)…`). Like Derive's `postfix` (and unlike
//!   Wolfram's, which also has `[[…]]` Part-indexing multiplying against
//!   it), Reduce's `postfix` has only ONE suffix shape — a call — so a
//!   single per-chain count of call groups is already an exact bound.
//! - [`Lowerer::check_apply_arg_count`] caps `arglist`/`list_literal`
//!   element counts AND `group_expr`'s flat `{ (SEMI|DOLLAR) expr }`
//!   statement-sequence length — flat-`Vec` allocation-size backstops,
//!   not stack guards (mirrors `derive-to-semantic-ir`'s identical reuse
//!   of this one guard for both `arglist` and vector-row counts).
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
//! Reduce's five genuinely *self-referential* (right-recursive)
//! productions — parenthesised `group` nesting, the `:=` chain, the
//! `if`/`else` chain, the cons (`.`) chain, and the power (`^`) chain —
//! need NO additional lowering-side guard beyond the ordinary `depth`
//! parameter threaded through [`Lowerer::lower_node`]: `reduce-parser`'s
//! own `MAX_RULE_DEPTH` (128) already bounds how deep any of these can
//! nest in the CST this crate ever receives (measured directly in that
//! crate's own doc comment — the binding constraint is the cons chain's
//! 179-rule-frame floor, ~28.5% margin below the 128 cap), so a tree this
//! deep can never even reach this crate's lowering in the first place.
//! This mirrors exactly why `derive-to-semantic-ir` needs no explicit
//! guard on `assignment`'s own right-recursive `[ ASSIGN assignment ]`
//! continuation either — the risk there is bounded by the parser, not by
//! this module.
//!
//! # `compile` vs. `compile_source`
//!
//! This module's [`compile`] is pure lowering over an already-parsed
//! tree — see `src/lib.rs`'s `compile_source` doc comment for why, like
//! `macsyma-to-semantic-ir`/`derive-to-semantic-ir` (and unlike
//! `wolfram-to-semantic-ir`), this crate's `compile_source` does not need
//! to spawn an enlarged-stack worker thread: `reduce-parser`'s own
//! `MAX_RULE_DEPTH` (128) is already documented safe on a bare default
//! (~2 MiB) stack with comfortable margin.

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span, Stmt,
};
use symbolic_ir::{
    ADD, AND, ASSIGN, DEFINE, DIV, EQUAL, GREATER, GREATER_EQUAL, IF, LESS, LESS_EQUAL, LIST, MUL,
    NEG, NOT, NOT_EQUAL, OR, POW, SUB,
};

/// The canonical head for a `<< s1; s2; ... >>` group statement (MA08 §3).
///
/// Not exported by `symbolic-ir` (see the module doc comment's "REAL gap"
/// section) — defined locally, spelled to match `reduce-runtime::lower`'s
/// own identically-named constant, so the one place this crate needs the
/// spelling has a name, not a repeated string literal.
pub const COMPOUND_EXPRESSION: &str = "CompoundExpression";

/// The canonical head for a non-foldable `a . b` cons (MA08 §3). See
/// [`Lowerer::fold_cons`] — this head is only ever produced when the
/// right-hand side isn't structurally a literal `List`, the one case MA08
/// §3 does not document a fold for.
pub const CONS: &str = "Cons";

/// The canonical heads for Reduce's list accessors/constructors (MA08 §3).
/// Spelled to match `reduce-runtime::lower`'s own identically-named
/// constants (which are in turn spelled to match `cas-list-operations`'
/// own `FIRST`/`REST`/`APPEND`/`REVERSE`/`PART` constants) — this crate
/// does not depend on either crate, but keeps the *spelling* identical on
/// purpose.
pub const FIRST: &str = "First";
pub const SECOND: &str = "Second";
pub const THIRD: &str = "Third";
pub const REST: &str = "Rest";
pub const PART: &str = "Part";
pub const APPEND: &str = "Append";
pub const REVERSE: &str = "Reverse";

/// Maximum expression-nesting depth for *this crate's own* lowering
/// recursion — distinct from (and independent of) `reduce-parser`'s own
/// `MAX_RULE_DEPTH` grammar-nesting guard (128), which bounds the CST this
/// crate walks. Mirrors `wolfram-to-semantic-ir`'s, `macsyma-to-semantic-
/// ir`'s, and `derive-to-semantic-ir`'s identically-named,
/// identically-valued guard — kept at 256 for consistency across the
/// whole SIR23 frontend family rather than inventing a new value, even
/// though `reduce-parser`'s own cap (128) is lower than `derive-parser`'s
/// (200): this constant bounds a DIFFERENT axis (this crate's own
/// chain-folding/tree-depth budget, exercised by e.g. a long flat `+`
/// chain that parses as ONE CST node regardless of nesting depth), not
/// the CST-nesting axis `reduce-parser`'s own cap already bounds.
const MAX_EXPR_DEPTH: usize = 256;

/// Synthetic file name used for all spans (the CST does not carry the
/// original path).
const FILE: &str = "<reduce>";

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// An error encountered during Reduce → SIR lowering.
///
/// Mirrors `DeriveLowerError`/`MacsymaLowerError`/`WolframLowerError`'s
/// shape exactly (`message` + 1-based `line`/`column`) so tooling can
/// treat every SIR frontend uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReduceLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for ReduceLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ReduceLowerError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for ReduceLowerError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed Reduce CST (rooted at the `program` rule) into a SIR
/// module.
///
/// This function does **not** itself guard against native stack overflow
/// on deeply-nested input beyond its own [`MAX_EXPR_DEPTH`] cap — it
/// trusts `tree` was already parsed under a suitable guard
/// (`reduce-parser`'s own `MAX_RULE_DEPTH`). See `src/lib.rs`'s
/// `compile_source` doc comment for why no worker-thread stack
/// enlargement is needed here.
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, ReduceLowerError> {
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
/// design inherited from `reduce-runtime` (see the module doc comment),
/// there are no host variables, parameters, or scopes to resolve — even a
/// procedure's formal parameters lower to plain `SymSymbol`s inside a
/// `List`, not to bound names. This lowerer is a near-stateless recursive
/// descent over the CST.
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
    // `statement_line = statement ( SEMI | DOLLAR ) ;`
    // -------------------------------------------------------------------

    /// Unlike Derive's `program = { statement_line }` (every statement is
    /// terminator-wrapped), Reduce's grammar adds an OPTIONAL final bare
    /// `statement` outside the repetition (`code/grammars/reduce/
    /// reduce.grammar`'s own comment explains why: so a source file need
    /// not end with a trailing `;`/`$`) — mirrors `reduce-runtime::lower::
    /// lower_program`'s identical two-shape loop.
    fn lower_file(&mut self, program: &GrammarASTNode) -> Result<Module, ReduceLowerError> {
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
            .with_source_language("reduce")
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
    /// Most grammar rules are "transparent wrappers" — a precedence level
    /// that did not apply its own operator still emits its own node with
    /// a single child, and so does the `expr = if_expr | group_expr |
    /// assignment` ordered-choice once it has committed to one
    /// alternative. [`unwrap_single`] peels those away so we dispatch on
    /// the first rule that genuinely shapes the tree (mirrors
    /// `wolfram-to-semantic-ir::lower::unwrap_single` and `reduce-
    /// runtime::lower::unwrap_single`, which this crate's dispatch table
    /// is otherwise a direct retarget of).
    fn lower_node(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Expr, ReduceLowerError> {
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
                "group_expr" => self.lower_group_expr(node, depth),
                "assignment" => self.lower_assignment(node, depth),
                "logical_or" => self.lower_logical_chain(node, depth, OR),
                "logical_and" => self.lower_logical_chain(node, depth, AND),
                "logical_not" => self.lower_logical_not(node, depth),
                "comparison" => self.lower_comparison(node, depth),
                "cons" => self.lower_cons(node, depth),
                "additive" | "multiplicative" => self.lower_binary_chain(node, depth),
                "unary" => self.lower_unary(node, depth),
                "power" => self.lower_power(node, depth),
                "postfix" => self.lower_postfix(node, depth),
                "atom" => self.lower_atom(node, depth),
                "list_literal" => self.lower_list_literal(node, depth),
                "arglist" => Err(self.err_at(
                    node,
                    "an arglist cannot be lowered as a scalar expression".to_string(),
                )),
                "group" => self.lower_group(node, depth),
                other => Err(self.err_at(node, format!("no lowering for rule `{other}`"))),
            },
        }
    }

    /// Lower a raw token (a literal or a bare symbol). Reduce has no
    /// `STRING` token in this grammar (`reduce.tokens`'s `escapes: none`)
    /// and its word-spelled operators/keywords (`and`/`or`/`not`/`neq`/
    /// `if`/`then`/`else`) are always consumed by their own grammar rule
    /// before reaching here as a bare leaf token — so, like
    /// `derive-runtime::lower_token`, this only ever needs `NUMBER`/`NAME`
    /// arms.
    fn lower_token(&mut self, token: &Token) -> Result<Expr, ReduceLowerError> {
        let span = self.token_span(token);
        match token_type(token) {
            "NUMBER" => Ok(self.number_literal_expr(&token.value, span)),
            "NAME" => Ok(self.sym_symbol(token.value.clone(), span)),
            other => Err(ReduceLowerError {
                message: format!("unexpected token `{other}` = {:?}", token.value),
                line: token.line,
                column: token.column,
            }),
        }
    }

    /// `if_expr = "if" expr "then" expr [ "else" expr ] ;` — MA08 §3:
    /// `If[b, s1, s2]`, or (no `else`) `If[b, s1]`. Reduce's `if_expr` has
    /// no `elseif` repetition the way Macsyma's does (nesting an `if`
    /// inside another `if`'s `else` branch is ordinary grammar recursion,
    /// bounded by `reduce-parser`'s own depth cap — see the module doc
    /// comment), so this needs only a plain 2-or-3-children count, unlike
    /// `macsyma-to-semantic-ir::lower_if`'s flat-chain fold.
    fn lower_if(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, ReduceLowerError> {
        let branches: Vec<&GrammarASTNode> = child_nodes(node).collect();
        let span = self.span_of(node);
        match branches.len() {
            2 => {
                let cond = self.lower_node(branches[0], depth + 1)?;
                let then_branch = self.lower_node(branches[1], depth + 1)?;
                Ok(self.sym_apply(self.sym_symbol_bare(IF, span.clone()), vec![cond, then_branch], span))
            }
            3 => {
                let cond = self.lower_node(branches[0], depth + 1)?;
                let then_branch = self.lower_node(branches[1], depth + 1)?;
                let else_branch = self.lower_node(branches[2], depth + 1)?;
                Ok(self.sym_apply(
                    self.sym_symbol_bare(IF, span.clone()),
                    vec![cond, then_branch, else_branch],
                    span,
                ))
            }
            n => Err(self.err_at(
                node,
                format!("if_expr expected 2 or 3 `expr` children (cond/then[/else]), got {n}"),
            )),
        }
    }

    /// `group_expr = GROUP_OPEN expr { ( SEMI | DOLLAR ) expr } GROUP_CLOSE ;`
    /// — MA08 §3's `<< s1; s2; ... >>`, lowered to
    /// `CompoundExpression[s1, s2, ...]`. See the module doc comment's
    /// "REAL gap" section: `symbolic-vm`'s shared handler table has no
    /// handler for this head, so this lowers the structurally-correct
    /// shape without claiming it evaluates to "the last statement's
    /// value" the way MA08 §3 describes — moot for this crate anyway,
    /// since it never evaluates anything. The `{ ... }` repetition is a
    /// FLAT list of sibling `expr` children (not right-recursive), so
    /// [`Self::check_apply_arg_count`] bounds its length as an
    /// allocation-size backstop, mirroring how [`Self::lower_arglist`] is
    /// bounded.
    fn lower_group_expr(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, ReduceLowerError> {
        let exprs: Vec<&GrammarASTNode> = child_nodes(node).collect();
        if exprs.is_empty() {
            return Err(self.err_at(node, "empty group statement `<< >>`".to_string()));
        }
        self.check_apply_arg_count(node, exprs.len())?;
        let mut lowered = Vec::with_capacity(exprs.len());
        for e in exprs {
            lowered.push(self.lower_node(e, depth + 1)?);
        }
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(COMPOUND_EXPRESSION, span.clone()), lowered, span))
    }

    /// `assignment = logical_or [ ASSIGN expr ] ;` — right-associative
    /// (manual §2.7: "a:=b:=c evaluates as a:=(b:=c)").
    ///
    /// See the module doc comment's "`:=` disambiguation" section: there
    /// is only ONE assignment token (`ASSIGN`), so the LHS's own
    /// *lowered shape* decides `Assign` vs `Define` — exactly mirroring
    /// `derive-to-semantic-ir::lower_assignment`/`reduce-runtime::lower::
    /// lower_assignment`. `h(l, m) := body` (LHS lowers to
    /// `SymApply{head: SymSymbol(h), ..}`) becomes
    /// `Define(h, List(l, m), body)`; a bare `x := body` (LHS is a plain
    /// symbol, or anything else call-shaped with a non-`SymSymbol` head)
    /// becomes `Assign(x, body)`.
    fn lower_assignment(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, ReduceLowerError> {
        let Some(op_index) = node
            .children
            .iter()
            .position(|c| as_token(c).is_some_and(|t| token_type(t) == "ASSIGN"))
        else {
            return self.lower_first_node(node, depth);
        };
        if op_index == 0 || op_index + 1 >= node.children.len() {
            return Err(self.err_at(node, "malformed assignment node".to_string()));
        }
        let lhs = self.lower_child(&node.children[op_index - 1], depth + 1)?;
        let rhs = self.lower_child(&node.children[op_index + 1], depth + 1)?;
        let span = self.span_of(node);

        if let Expr::SymApply { head, args, .. } = &lhs {
            if matches!(head.as_ref(), Expr::SymSymbol { .. }) {
                // h(l, m) := e — a procedure definition. Reduce has no
                // pattern syntax either, so each parameter already
                // lowered to a plain `SymSymbol` (or, for a malformed
                // definition like `h(1) := e`, whatever the caller wrote
                // — a later pass owns validating parameter shapes, not
                // this lowering, mirroring `reduce-runtime`'s own
                // lowering, which does not validate parameter shapes
                // either).
                let params = self.sym_apply(
                    self.sym_symbol_bare(LIST, span.clone()),
                    args.clone(),
                    span.clone(),
                );
                return Ok(self.sym_apply(
                    self.sym_symbol_bare(DEFINE, span.clone()),
                    vec![(**head).clone(), params, rhs],
                    span,
                ));
            }
        }
        // x := e — variable assignment.
        Ok(self.sym_apply(self.sym_symbol_bare(ASSIGN, span.clone()), vec![lhs, rhs], span))
    }

    /// `logical_or`/`logical_and` — fold operands into an n-ary `Or`/`And`
    /// `SymApply` (a single flat apply carrying every operand at this
    /// precedence level, not a nested binary chain — safe to fold n-ary
    /// because every step in one chain shares the SAME operator, mirrors
    /// `derive-to-semantic-ir::lower_logical_chain`/`reduce-runtime::
    /// lower::lower_logical_chain` exactly).
    fn lower_logical_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        head: &str,
    ) -> Result<Expr, ReduceLowerError> {
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
    /// `and`/`or`/`not`/`neq`/`if`/`then`/`else` are all matched in the
    /// grammar as `reduce.tokens`' own `KEYWORD` token type (promoted from
    /// `NAME` by EXACT lowercase spelling — see `reduce.tokens`'s header;
    /// the mirror image of `derive.tokens`'s UPPERCASE-only keyword rule),
    /// so — mirroring `derive-to-semantic-ir::lower_logical_not`'s
    /// identical check for its own uppercase `"NOT"` — this checks the
    /// token's literal *value*, not `effective_type_name()` (every
    /// keyword shares that one type name).
    fn lower_logical_not(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, ReduceLowerError> {
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

    /// `comparison = cons [ ( EQ | "neq" | LESS | GREATER | LE | GE ) cons ] ;`
    /// — a single (non-chained) comparison, per MA08 §3's own disclosed
    /// simplification. `=` is Reduce's *equation* operator (`Equal`),
    /// never assignment — `:=` alone owns that role (MA08 §3, manual
    /// §3.4). Unlike Derive (no not-equal token at all), Reduce has `neq`
    /// — a `KEYWORD`-typed token matched by literal value, exactly
    /// mirroring [`Self::lower_logical_not`]'s identical convention.
    fn lower_comparison(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, ReduceLowerError> {
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

    /// `cons = additive [ DOT cons ] ;` — right-associative (`cons`'s own
    /// optional continuation references itself: `a . b . {c}` is
    /// `a . (b . {c})`, so lowering the RHS recursively folds inside-out
    /// before [`Self::fold_cons`] ever sees it). This is genuine
    /// right-recursion in the grammar (not a flat repetition), bounded by
    /// `reduce-parser`'s own depth cap — see the module doc comment's
    /// recursion-hardening section for why no additional chain-length
    /// guard is needed here.
    fn lower_cons(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, ReduceLowerError> {
        let Some(dot_index) = node
            .children
            .iter()
            .position(|c| as_token(c).is_some_and(|t| token_type(t) == "DOT"))
        else {
            return self.lower_first_node(node, depth);
        };
        if dot_index == 0 || dot_index + 1 >= node.children.len() {
            return Err(self.err_at(node, "malformed cons node".to_string()));
        }
        let lhs = self.lower_child(&node.children[dot_index - 1], depth + 1)?;
        let rhs = self.lower_child(&node.children[dot_index + 1], depth + 1)?;
        let span = self.span_of(node);
        Ok(self.fold_cons(lhs, rhs, span))
    }

    /// Fold `lhs . rhs` — MA08 §3's own words: "R-4 folds a `Cons` onto a
    /// literal `List` immediately into one `List`". When `rhs` lowered to
    /// a *structurally* literal `List(...)` application, prepend `lhs`
    /// directly into a new flat `List` — no `Cons` head is ever produced
    /// for this case, so it needs no VM handler at all (`List`'s own
    /// handler, already shared and reused, does all the work). This is
    /// the ONLY shape MA08 §3's table documents a lowering for.
    ///
    /// A right-hand side that ISN'T structurally a `List` at lowering
    /// time (`a . b`, where `b` is a bound variable, a function call, or
    /// another not-yet-resolved expression — lowering runs once, before
    /// any evaluation, so it cannot know what `b` will turn out to be)
    /// has no such fold available; MA08 §3's table is silent on this
    /// case. Rather than reject it outright, it lowers to a plain
    /// `Cons[lhs, rhs]` application — the same "structurally correct, but
    /// no handler evaluates it further" gap as `First`/`Rest`/etc (see
    /// the module doc comment). Exactly mirrors `reduce-runtime::lower::
    /// fold_cons`'s identical logic, retargeted onto `semantic_ir::Expr`.
    fn fold_cons(&mut self, lhs: Expr, rhs: Expr, span: Span) -> Expr {
        match rhs {
            Expr::SymApply { head, args, .. } if matches!(head.as_ref(), Expr::SymSymbol { name, .. } if name == LIST) => {
                let mut elems = Vec::with_capacity(args.len() + 1);
                elems.push(lhs);
                elems.extend(args);
                self.sym_apply(self.sym_symbol_bare(LIST, span.clone()), elems, span)
            }
            rhs => self.sym_apply(self.sym_symbol_bare(CONS, span.clone()), vec![lhs, rhs], span),
        }
    }

    /// `additive`/`multiplicative` — a left-associative binary chain of
    /// `+`/`-`/`*`/`/`. Must fold pairwise (not n-ary, unlike the logical
    /// chains) since a single chain can mix operators: `a - b - c` folds
    /// left into `Sub(Sub(a, b), c)`; `a + b - c` into `Sub(Add(a, b),
    /// c)`.
    ///
    /// Like Wolfram's, Macsyma's, and Derive's grammars, Reduce's grammar
    /// collapses a flat run of same-precedence operators into ONE CST
    /// node with many children rather than nesting through parens — see
    /// [`Self::check_chain_length`] for why this needs its own cap
    /// independent of `MAX_EXPR_DEPTH`.
    fn lower_binary_chain(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, ReduceLowerError> {
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

    /// `unary = MINUS unary | power ;` — MA08 §3 lists only unary `-` (no
    /// unary `+`, matching `derive.grammar`'s identical asymmetry) — a
    /// leading `-` is `Neg`; otherwise it is the inner `power`.
    fn lower_unary(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, ReduceLowerError> {
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

    /// `power = postfix [ ( CARET | POW ) unary ] ;` — right-associative
    /// `^`/`**`. `reduce.tokens` keeps `CARET`/`POW` as two distinct
    /// *token* types (manual §2.7's own precedence table lists them as
    /// one tier), so — mirroring `reduce-runtime::lower_power`'s identical
    /// acceptance of either — this accepts either token type here, at the
    /// parser-grammar tier where the two spellings actually collapse onto
    /// one production.
    fn lower_power(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, ReduceLowerError> {
        match node.children.len() {
            1 => self.lower_child(&node.children[0], depth + 1),
            3 => {
                let is_power_op = as_token(&node.children[1]).is_some_and(|t| matches!(token_type(t), "CARET" | "POW"));
                if !is_power_op {
                    return Err(self.err_at(node, "malformed power node: expected CARET or POW".to_string()));
                }
                let lhs = self.lower_child(&node.children[0], depth + 1)?;
                let rhs = self.lower_child(&node.children[2], depth + 1)?;
                let span = self.span_of(node);
                Ok(self.sym_apply(self.sym_symbol_bare(POW, span.clone()), vec![lhs, rhs], span))
            }
            _ => Err(self.err_at(node, "malformed power node".to_string())),
        }
    }

    /// `postfix = atom { LPAREN [ arglist ] RPAREN } ;` — function/
    /// procedure/array-subscript application, left-associative and
    /// chainable. MA08 §3's single call-shaped production covers
    /// `f(a, b)`, a `Define` LHS like `h(l, m)`, and `a(5)`/`b(i, q)`
    /// (array-subscript *reads* — array declaration/indexed *write* are
    /// out of scope, MA08 §4) all at once, mirroring `reduce-runtime::
    /// lower_postfix`'s identical single production.
    ///
    /// Like Derive's `postfix` (and unlike Wolfram's, whose second suffix
    /// shape `[[…]]` Part-indexing multiplies against the call-argument
    /// count), Reduce's `postfix` has only this one suffix shape, so
    /// [`Self::check_postfix_chain_length`]'s plain linear cap on chained
    /// call groups is exact.
    fn lower_postfix(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, ReduceLowerError> {
        self.check_postfix_chain_length(node)?;
        let mut result = self.lower_child(
            node.children
                .first()
                .ok_or_else(|| self.err_at(node, "postfix has no base".to_string()))?,
            depth + 1,
        )?;

        let mut i = 1;
        while i < node.children.len() {
            let Some(token) = as_token(&node.children[i]) else {
                i += 1;
                continue;
            };
            if token_type(token) == "LPAREN" {
                let args = node
                    .children
                    .get(i + 1)
                    .and_then(as_node)
                    .filter(|n| n.rule_name == "arglist")
                    .map(|n| self.lower_arglist(n, depth + 1))
                    .transpose()?
                    .unwrap_or_default();
                self.check_apply_arg_count(node, args.len())?;
                result = self.build_application(result, args, node);
            }
            i += 1;
        }
        Ok(result)
    }

    /// Apply `head` to `args`, bridging a lowercase builtin surface
    /// function name (`list`, `first`, `rest`, …) to its canonical IR
    /// head via [`standard_function`] — mirrors `derive-to-semantic-ir::
    /// build_application`'s/`reduce-runtime::lower_postfix`'s
    /// `canonical_head` step exactly. Unlike Wolfram's `build_application`,
    /// there is no associative n-ary left-fold here: Reduce has no
    /// explicit-head-application sugar analogous to Wolfram's
    /// `Plus[1, 2, 3]` (a call is always just a call), so this is a plain
    /// wrap.
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
    /// no stack-recursion risk analogous to the binary-chain rules —
    /// [`Self::check_apply_arg_count`] still bounds its raw length as a
    /// modest defense-in-depth cap on allocation size.
    fn lower_arglist(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Vec<Expr>, ReduceLowerError> {
        self.lower_child_nodes(node, depth)
    }

    /// `atom = NUMBER | NAME | list_literal | group ;`
    fn lower_atom(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, ReduceLowerError> {
        if let Some(child) = child_nodes(node).next() {
            if matches!(child.rule_name.as_str(), "list_literal" | "group") {
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

    /// `list_literal = LBRACE [ arglist ] RBRACE ;` — MA08 §3's `{a, b,
    /// c}` (curly braces, NOT Derive's square brackets). Reduce's list is
    /// always flat here (no row/matrix shape — matrices are out of
    /// scope, MA08 §4), so this reuses [`Self::lower_arglist`] directly,
    /// unlike `derive-to-semantic-ir::lower_vector`'s row-counting split
    /// — mirrors `reduce-runtime::lower_list_literal`'s identical logic.
    fn lower_list_literal(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, ReduceLowerError> {
        let args = match child_nodes(node).find(|n| n.rule_name == "arglist") {
            Some(arglist_node) => self.lower_arglist(arglist_node, depth + 1)?,
            None => vec![],
        };
        self.check_apply_arg_count(node, args.len())?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(LIST, span.clone()), args, span))
    }

    /// `group = LPAREN expr RPAREN ;` — grouping only.
    fn lower_group(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, ReduceLowerError> {
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
    /// `reduce-runtime::lower::lower_number`'s identical rule). An
    /// integer lexeme too large for `i64` falls back to a float rather
    /// than silently truncating.
    ///
    /// **Must** be an instance method, not a free function: every branch
    /// that constructs a `FloatLit` calls `self.observed.add(Feature::
    /// Floats)` immediately. This is a confirmed, previously-shipped bug
    /// in both `matlab-to-semantic-ir` and `wolfram-to-semantic-ir` (their
    /// number-literal helpers were free functions with no access to
    /// `observed`, so a float-literal-only module failed
    /// `semantic_ir::validate()`), fixed proactively in
    /// `macsyma-to-semantic-ir`/`derive-to-semantic-ir` and carried
    /// forward here.
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
    /// Reduce's grammar, like Wolfram's, Macsyma's, and Derive's,
    /// collapses a flat run of same-precedence operators into ONE CST
    /// node with many children rather than nesting through parens, so a
    /// long unparenthesized chain (`1 + 1 + ... + 1`, thousands of terms)
    /// never trips the ordinary grammar-nesting depth guard
    /// (`reduce-parser`'s `MAX_RULE_DEPTH`, which counts *nesting*, not
    /// the length of one flat repetition — confirmed directly in that
    /// crate's own doc comment: an uncapped parser accepted one million
    /// repeated items with zero crashes). But folding N operands
    /// left-associatively still builds an N-deep *binary* `Expr` tree,
    /// and that tree's own depth is what every later recursive pass over
    /// it pays for regardless of how cheaply each fold step was.
    fn check_chain_length(&self, node: &GrammarASTNode) -> Result<(), ReduceLowerError> {
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

    /// Cap the number of chained call groups (`f(x)(y)(z)…`) in a single
    /// `postfix` node at `MAX_EXPR_DEPTH`. Reduce's `postfix` has only
    /// ONE suffix shape — a call `(...)` — so a plain count of chained
    /// call groups already bounds the real nesting depth this loop can
    /// build, one-to-one (see `macsyma-to-semantic-ir::lower::
    /// check_postfix_chain_length`'s doc comment for the fuller
    /// comparison against Wolfram's cumulative-budget variant, needed
    /// there only because Wolfram's `postfix` has a second suffix shape
    /// that multiplies against this one).
    fn check_postfix_chain_length(&self, node: &GrammarASTNode) -> Result<(), ReduceLowerError> {
        let chain_len = node
            .children
            .iter()
            .filter(|c| as_token(c).is_some_and(|t| token_type(t) == "LPAREN"))
            .count();
        if chain_len > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!(
                    "chained function-call application too deep ({chain_len} groups, exceeds \
                     {MAX_EXPR_DEPTH})"
                ),
            ));
        }
        Ok(())
    }

    /// Cap the argument count of a single `f(…)` application, the element
    /// count of a `{…}` list literal, or the statement count of a
    /// `<< … >>` group statement. None of these fold into a nested tree
    /// (all stay a flat `Vec<Expr>`), so this is not a stack-recursion
    /// guard — it is a modest defense-in-depth cap on a single
    /// allocation's size, using the same `MAX_EXPR_DEPTH` bound for
    /// consistency rather than inventing new constants per call site
    /// (mirrors `derive-to-semantic-ir::check_apply_arg_count`'s identical
    /// reuse across `arglist` AND vector/matrix row/element counts).
    fn check_apply_arg_count(&self, node: &GrammarASTNode, count: usize) -> Result<(), ReduceLowerError> {
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

    fn lower_first_node(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, ReduceLowerError> {
        let child = child_nodes(node)
            .next()
            .ok_or_else(|| self.err_at(node, format!("`{}` has no expression child", node.rule_name)))?;
        self.lower_node(child, depth + 1)
    }

    fn lower_child(&mut self, child: &ASTNodeOrToken, depth: usize) -> Result<Expr, ReduceLowerError> {
        match child {
            ASTNodeOrToken::Node(node) => self.lower_node(node, depth),
            ASTNodeOrToken::Token(token) => self.lower_token(token),
        }
    }

    fn lower_child_nodes(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Vec<Expr>, ReduceLowerError> {
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

    fn err_at(&self, node: &GrammarASTNode, message: String) -> ReduceLowerError {
        ReduceLowerError {
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

/// Map an arithmetic token type to its canonical IR head. See the module
/// doc comment's "REAL divergence" section: these are `Add`/`Sub`/`Mul`/
/// `Div` (`symbolic_ir::ADD`/`SUB`/`MUL`/`DIV`), NOT MA08 §3's literal
/// (and non-existent) `Plus`/`Subtract`/`Times`. Note `TIMES`, matching
/// `reduce.tokens`'s own spelling of the multiplication token (same as
/// Derive's `TIMES`, not Macsyma's `STAR`).
fn binary_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "PLUS" => Some(ADD),
        "MINUS" => Some(SUB),
        "TIMES" => Some(MUL),
        "SLASH" => Some(DIV),
        _ => None,
    }
}

/// Map a comparison token to its canonical IR head. `neq` is a
/// `KEYWORD`-typed token (see [`Lowerer::lower_logical_not`]'s identical
/// note), matched by literal value alongside the four symbolic comparison
/// token *types* — needs the full `&Token` (not just its type name),
/// unlike Derive's/Macsyma's `comparison_head(token_type: &str)`, since
/// `neq`'s type alone (`KEYWORD`) is shared with `and`/`or`/`not`/`if`/
/// `then`/`else`.
fn comparison_head(token: &Token) -> Option<&'static str> {
    match token_type(token) {
        "EQ" => Some(EQUAL),
        "LESS" => Some(LESS),
        "GREATER" => Some(GREATER),
        "LE" => Some(LESS_EQUAL),
        "GE" => Some(GREATER_EQUAL),
        "KEYWORD" if token.value == "neq" => Some(NOT_EQUAL),
        _ => None,
    }
}

/// Bridge a Reduce *surface* builtin name (lowercase, per manual
/// convention — `list`, `first`, `second`, `third`, `rest`, `part`,
/// `append`, `reverse`) to the canonical IR head. Mirrors `reduce-
/// runtime::lower::surface_head_to_ir`'s table exactly (same surface
/// names, same canonical head spellings — `LIST` from `symbolic_ir`, the
/// rest from this crate's own locally-defined constants, see the module
/// doc comment's "REAL gap" section). A head not in this table (a
/// user-defined operator/procedure, or a builtin spelled with different
/// casing) is returned unchanged, so `f(x)` stays a harmless unevaluated
/// symbolic call and an unrecognised spelling (e.g. uppercase `LIST(x)`)
/// is just an ordinary user symbol/call, not the builtin — mirrors
/// `derive-to-semantic-ir::standard_function`'s identical fallthrough
/// contract, one direction reversed (Derive bridges UPPERCASE only,
/// Reduce bridges lowercase only).
///
/// Unlike Derive's much larger bridge table (every elementary/hyperbolic
/// function, plus `DIF`/`INT`/`IF`), Reduce's R-4 scope (MA08 §3) has NO
/// trig/calculus bridging at all — confirmed directly against `reduce-
/// runtime::lower::surface_head_to_ir`'s own table, which covers only the
/// list constructor/accessors. `IF` needs no entry here at all, unlike
/// Derive's `IF(...)` call-shaped builtin, because Reduce's `if` is its
/// own dedicated `if_expr` grammar production (see
/// [`Lowerer::lower_if`]), not an ordinary call.
fn standard_function(name: &str) -> Option<&'static str> {
    match name {
        "list" => Some(LIST),
        "first" => Some(FIRST),
        "second" => Some(SECOND),
        "third" => Some(THIRD),
        "rest" => Some(REST),
        "part" => Some(PART),
        "append" => Some(APPEND),
        "reverse" => Some(REVERSE),
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
/// `SymReplaceAll` node in the first place (Reduce's grammar has no
/// pattern-matching or rewrite-rule syntax at all). `If`/
/// `CompoundExpression`/`Cons` are all `SymApply` with a different head
/// *symbol*, not new `Expr` variants, so this one match arm already
/// covers them.
///
/// This is the authoritative depth check every other guard in this file
/// (`MAX_EXPR_DEPTH`'s recursion-depth parameter, [`Lowerer::
/// check_chain_length`], [`Lowerer::check_postfix_chain_length`]) is only
/// an early, cheap approximation of — those guards are each scoped to one
/// grammar node and do not compose across nested `(...)` boundaries (see
/// `wolfram-to-semantic-ir`'s `CHANGELOG.md` for the security-review
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
/// `wolfram-to-semantic-ir::lower::unwrap_single` and `reduce-runtime::
/// lower::unwrap_single`).
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
