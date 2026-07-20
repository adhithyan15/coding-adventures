//! The lowering pass from `coding_adventures_derive_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **v0.1.0**.
//!
//! This is the **third** frontend to target
//! [SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
//! symbolic-expression/pattern-matching domain extension of the SIR10
//! narrow-waist IR (Stream B of
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)). The first,
//! `wolfram-to-semantic-ir`, and the second, `macsyma-to-semantic-ir`, are
//! this crate's design templates — read `wolfram-to-semantic-ir::lower`'s
//! module doc comment first for the "everything is data" design decision;
//! everything below assumes that context and only calls out where
//! Derive's grammar differs.
//!
//! # Retargeting `derive-runtime`, not starting from scratch
//!
//! `derive-runtime` already walks this exact CST and compiles it to
//! `symbolic_ir::IRNode` (`Symbol`/`Integer`/`Float`/`Apply` — see that
//! crate's `src/lower.rs`, in particular its module doc comment). Derive
//! has no `f[x]`-universal-application syntax (ordinary parens, `F(x)`,
//! double as both grouping and application) and — unlike Wolfram — no
//! pattern/rewrite-rule vocabulary at all (`_`, `->`, `/.`), so
//! `derive-runtime`'s own lowering is already "just" arithmetic,
//! comparison, logic, assignment/definition, function application, and
//! vector/matrix literals, with no pattern-lowering or
//! `ReplaceAll`/`ReplaceRepeated` interception to carry over. This crate's
//! job is therefore mechanical, exactly like `macsyma-to-semantic-ir`'s
//! own "retarget, don't redesign" precedent: walk the same CST, dispatch
//! on the same rule names `derive-runtime::lower::lower_node` already
//! uses, and construct `semantic_ir::Expr::{SymSymbol,SymApply}` wherever
//! that lowering constructs `symbolic_ir::IRNode::{Symbol,Apply}`.
//! Literals (`Integer`/`Float`) reuse SIR10/SIR16's `IntLit`/`FloatLit`
//! directly, per the SIR23 spec — Derive has no `STRING` token at all (see
//! `derive.tokens`'s `escapes: none`), so unlike Wolfram/Macsyma this
//! crate never constructs `Expr::StrLit` either.
//!
//! # Scope (v0.1.0) — no pattern-matching or rewrite-rule syntax at all
//!
//! **Verified empirically against `derive.grammar`/`derive.tokens`
//! (`code/grammars/derive/`), not just trusted from `derive-runtime`'s own
//! doc comment**: grepping both files for pattern-syntax tokens (`_`
//! blank, `->`/`:>` rule arrows, `/.`/`//.` replacement operators) finds
//! none — `derive.tokens` declares no such token at all, and
//! `derive.grammar`'s only assignment-shaped construct is `assignment =
//! logical_or [ ASSIGN assignment ]` (`:=`, used for both plain
//! assignment and function definition — see below). This confirms
//! `derive-runtime`'s own claim ("Derive... has no pattern/rewrite-rule
//! vocabulary (`_`, `->`, `/.`)") is accurate for the grammar as it
//! currently ships, matching this repo's "verify before implementing"
//! discipline (see `lessons.md`) rather than taking the doc comment on
//! faith.
//!
//! This crate therefore **only ever constructs [`Expr::SymSymbol`] and
//! [`Expr::SymApply`]** (plus the reused `IntLit`/`FloatLit` literal
//! nodes) — it never constructs `SymPatternBlank`/`SymPatternNamed`/
//! `SymRule`/`SymReplaceAll`, and it never observes
//! `Feature::PatternMatching`. This is a disclosed scope boundary
//! matching the grammar's actual surface, not an oversight: a future
//! Derive grammar revision that adds pattern-matching syntax would need a
//! new grammar rule before this crate could ever emit those nodes. One
//! concrete consequence, mirroring `macsyma-to-semantic-ir`'s identical
//! note: [`measure_depth_iterative`]/[`drop_iterative`] below only need a
//! match arm for [`Expr::SymApply`] (recursing into `head` and `args`) —
//! every other `Expr` variant is a leaf for this crate's purposes.
//!
//! Every one of `derive.grammar`'s productions is covered: literals,
//! arithmetic (`+ - * /`, unary `-`, `^`), comparisons (`= <= < > >=` —
//! Derive has no `!=`/not-equal operator token at all, unlike Macsyma's
//! `#`), logic (`AND`/`OR`/`NOT`, case-sensitive reserved keywords),
//! vectors/matrices (`[…]`/`[…;…]`), function application (`F(x)`,
//! chainable `F(x)(y)` for grammar fidelity though Derive has no idiom
//! that produces one), and assignment/definition (the single `:=`
//! operator, disambiguated by LHS shape — see [`Lowerer::lower_assignment`]).
//! Derive has **no** control-flow grammar productions at all (no
//! `if`/`while`/`for`/`block`/`return` rules, unlike Macsyma) — `IF(…)` is
//! an ordinary UPPERCASE builtin call bridged through
//! [`Self::build_application`]'s [`standard_function`] table exactly like
//! `SIN`/`DIF`/`INT`, not a special grammar form, so this crate needs none
//! of Macsyma's synthetic `WHILE_HEAD`/`FOR_EACH_HEAD`/`BLOCK_HEAD`/
//! `RETURN_HEAD` local constants (and can import `IF` directly from
//! `symbolic_ir`, which already exports it, unlike those Macsyma-only
//! synthetic heads).
//!
//! # A BIGGER surface→canonical bridge than Wolfram's
//!
//! Wolfram already spells its canonical heads in the IR's own casing
//! (`Sin`, `Plus`, …), so its bridge only covers the operators whose
//! long-form name differs. Derive's built-ins are conventionally
//! **UPPERCASE** (`SIN`, `DIF`, `INT`, `IF` — MA07 §3), and `SymSymbol`
//! equality is case-sensitive, so *every* elementary/hyperbolic function
//! and every renamed calculus/control head needs an explicit entry in
//! [`standard_function`] — not just the handful that differ semantically.
//! This mirrors `derive-runtime::lower::surface_head_to_ir`'s table
//! exactly (same set of surface names, same canonical targets), reusing
//! the identical `symbolic_ir` head-name constants so the two lowerings
//! (native-eval and SIR23) can never drift apart on what a given builtin
//! canonicalizes to. An unrecognised head (a user-defined function, or a
//! builtin not spelled in the exact uppercase convention) passes through
//! unchanged, exactly like `derive-runtime`'s own fallthrough.
//!
//! # `:=` disambiguation has no operator to branch on
//!
//! Like `derive-runtime::lower::lower_assignment`'s own note: Derive's
//! grammar has exactly ONE assignment token, `ASSIGN` (`:=`) — `x := 5`
//! and `F(x) := x^2 + 1` are syntactically identical until this lowering
//! step. [`Lowerer::lower_assignment`] disambiguates purely by the
//! *lowered LHS's shape*: `SymApply{head: SymSymbol(_), ..}` → `Define`,
//! anything else → `Assign`. Derive also has no pattern syntax, so a
//! function's parameters need no unwrapping — a bare `NAME` in `F(x, y)
//! := …`'s argument position already lowers straight to a plain
//! `SymSymbol`, the exact shape a `Define` handler binds against.
//!
//! # Vectors/matrices as structural `List` data (D-5)
//!
//! `derive-parser` parses `[a, b, c]` / `[a, b; c, d]` as a single
//! `vector` rule — `vector = LBRACKET row { SEMI row } RBRACKET`, `row =
//! expr { COMMA expr }` — with no separate grammar rule distinguishing
//! "vector" from "matrix" shape. [`Lowerer::lower_vector`] draws that
//! distinction purely by *counting* how many `row` children were parsed,
//! mirroring `derive-runtime::lower::lower_vector`'s identical logic and
//! Wolfram's `{a, b}` → `List[a, b]`: exactly one `row` lowers to a flat
//! `SymApply{head: List, args: elems…}` (a vector); more than one lowers
//! to a `List` of per-row `List`s (a matrix). Per MA07 §2/§4, this is
//! *structural* only — no linear-algebra evaluation is wired here.
//!
//! # Recursion-depth hardening — carried over proactively, not discovered
//!
//! `wolfram-to-semantic-ir`'s `CHANGELOG.md` documents four rounds of
//! security review that each found a real, adversarially-confirmed native
//! stack-overflow gap, and `macsyma-to-semantic-ir` carried every one of
//! those hardening mechanisms over from day one rather than rediscovering
//! them. This crate does the same, even though neither `derive-parser`
//! nor `derive-runtime` (the retarget source) applies any of these
//! guards themselves — they are a `*-to-semantic-ir`-frontend-specific
//! defense established by the two sibling crates, not part of the native
//! pipeline:
//!
//! - [`MAX_EXPR_DEPTH`] bounds this crate's own CST-walking recursion.
//! - [`Lowerer::check_chain_length`] caps every flat, same-precedence
//!   operator-chain fold (`additive`/`multiplicative`/`logical_or`/
//!   `logical_and`) before any tree is built.
//! - [`Lowerer::check_postfix_chain_length`] caps chained call
//!   application (`F(x)(y)(z)…`). Like Macsyma's `postfix` (and unlike
//!   Wolfram's, which also has `[[…]]` Part-indexing multiplying against
//!   it), Derive's `postfix` has only ONE suffix shape — a call — so a
//!   single per-chain count of call groups is already an exact bound.
//! - [`Lowerer::check_apply_arg_count`] caps `arglist`/vector-`row`
//!   element counts and vector row counts — flat-`Vec` allocation-size
//!   backstops, not stack guards.
//! - [`measure_depth_iterative`] is the authoritative,
//!   construction-composition-independent check: an iterative (never
//!   recursive) walk of an already-built `Expr`, called once per
//!   top-level statement in [`Lowerer::lower_file`].
//! - [`drop_iterative`] tears down a tree [`measure_depth_iterative`] just
//!   rejected, using an explicit work stack rather than the ordinary
//!   recursive `Drop` glue, for the same reason `macsyma-to-semantic-ir`
//!   documents (detecting an oversized tree and then letting it drop
//!   normally just relocates the same stack overflow from "walking
//!   forward" to "walking backward").
//!
//! # `compile` vs. `compile_source`
//!
//! This module's [`compile`] is pure lowering over an already-parsed
//! tree — see `src/lib.rs`'s `compile_source` doc comment for why, like
//! `macsyma-to-semantic-ir` (and unlike `wolfram-to-semantic-ir`), this
//! crate's `compile_source` does not need to spawn an enlarged-stack
//! worker thread: `derive-parser`'s own `MAX_RULE_DEPTH` (200) is already
//! documented safe on a bare default (~2 MiB) stack with comfortable
//! margin.

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span, Stmt,
};
use symbolic_ir::{
    ACOS, ACOSH, ADD, AND, ASIN, ASINH, ASSIGN, ATAN, ATANH, COS, COSH, COTH, CSCH, D, DEFINE, DIV,
    EQUAL, EXP, GREATER, GREATER_EQUAL, IF, INTEGRATE, LESS, LESS_EQUAL, LIST, LOG, MUL, NEG, NOT,
    OR, POW, SECH, SIN, SINH, SQRT, SUB, TAN, TANH,
};

/// Maximum expression-nesting depth for *this crate's own* lowering
/// recursion — distinct from (and independent of) `derive-parser`'s own
/// `MAX_RULE_DEPTH` grammar-nesting guard, which bounds the CST this
/// crate walks. Mirrors `wolfram-to-semantic-ir`'s and
/// `macsyma-to-semantic-ir`'s identically-named, identically-valued guard
/// (see `wolfram-to-semantic-ir::lower::MAX_EXPR_DEPTH`'s doc comment for
/// the full "why 256" reasoning). `derive-parser`'s own measured
/// bare-stack crash floor (298 `parse_rule` frames — see that crate's
/// `MAX_RULE_DEPTH` doc comment) is even higher than `macsyma-parser`'s/
/// `wolfram-parser`'s (~275-278), since all three share the same generic
/// `GrammarParser` dispatch engine, so 256 remains a conservative,
/// consistent value to reuse here rather than inventing a new one.
const MAX_EXPR_DEPTH: usize = 256;

/// Synthetic file name used for all spans (the CST does not carry the
/// original path).
const FILE: &str = "<derive>";

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// An error encountered during Derive → SIR lowering.
///
/// Mirrors `WolframLowerError`/`MacsymaLowerError`/`MatlabLowerError`'s
/// shape exactly (`message` + 1-based `line`/`column`) so tooling can
/// treat every SIR frontend uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeriveLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for DeriveLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeriveLowerError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for DeriveLowerError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed Derive CST (rooted at the `program` rule) into a SIR
/// module.
///
/// This function does **not** itself guard against native stack overflow
/// on deeply-nested input beyond its own [`MAX_EXPR_DEPTH`] cap — it
/// trusts `tree` was already parsed under a suitable guard
/// (`derive-parser`'s own `MAX_RULE_DEPTH`). See `src/lib.rs`'s
/// `compile_source` doc comment for why no worker-thread stack
/// enlargement is needed here.
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, DeriveLowerError> {
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
/// Like `wolfram-to-semantic-ir`'s and `macsyma-to-semantic-ir`'s
/// `Lowerer`s, there is no per-function name-resolution context here at
/// all: under the "everything is data" design inherited from
/// `derive-runtime` (see the module doc comment), there are no host
/// variables, parameters, or scopes to resolve — even a function's formal
/// parameters lower to plain `SymSymbol`s inside a `List`, not to bound
/// names. This lowerer is a near-stateless recursive descent over the
/// CST.
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
    // top level: `program = { statement_line } ;`
    // `statement_line = statement NEWLINE | statement | NEWLINE ;`
    // -------------------------------------------------------------------

    fn lower_file(&mut self, program: &GrammarASTNode) -> Result<Module, DeriveLowerError> {
        if program.rule_name != "program" {
            return Err(self.err_at(
                program,
                format!("expected `program` root, got `{}`", program.rule_name),
            ));
        }

        let mut stmts: Vec<Stmt> = Vec::new();
        for line in child_nodes(program) {
            if line.rule_name != "statement_line" {
                continue;
            }
            // A blank/terminator-only line has no `statement` child at
            // all — skip it, mirroring `derive-runtime::lower::
            // lower_program`'s identical filter.
            let Some(statement_node) = child_nodes(line).find(|n| n.rule_name == "statement")
            else {
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
            .with_source_language("derive")
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
    /// a single child. [`unwrap_single`] peels those away so we dispatch
    /// on the first rule that genuinely shapes the tree (mirrors
    /// `wolfram-to-semantic-ir::lower::unwrap_single` and
    /// `derive-runtime::lower::unwrap_single`, which this crate's
    /// dispatch table is otherwise a direct retarget of).
    fn lower_node(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
    ) -> Result<Expr, DeriveLowerError> {
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
                "vector" => self.lower_vector(node, depth),
                "row" => Err(self.err_at(
                    node,
                    "a `row` node must be lowered via `lower_vector`'s row-counting logic, \
                     not `lower_node` directly"
                        .to_string(),
                )),
                "group" => self.lower_group(node, depth),
                "arglist" => Err(self.err_at(
                    node,
                    "an arglist cannot be lowered as a scalar expression".to_string(),
                )),
                other => Err(self.err_at(node, format!("no lowering for rule `{other}`"))),
            },
        }
    }

    /// Lower a raw token (a literal or a bare symbol). Derive has no
    /// `STRING` token in this grammar (`derive.tokens`'s `escapes: none`)
    /// and no `true`/`false` literal keywords (only `AND`/`OR`/`NOT`,
    /// handled at the `logical_*` rules, never reaching here as a bare
    /// leaf token) — so unlike `macsyma-to-semantic-ir::lower_token`,
    /// this only ever needs `NUMBER`/`NAME` arms.
    fn lower_token(&mut self, token: &Token) -> Result<Expr, DeriveLowerError> {
        let span = self.token_span(token);
        match token_type(token) {
            "NUMBER" => Ok(self.number_literal_expr(&token.value, span)),
            "NAME" => Ok(self.sym_symbol(token.value.clone(), span)),
            other => Err(DeriveLowerError {
                message: format!("unexpected token `{other}` = {:?}", token.value),
                line: token.line,
                column: token.column,
            }),
        }
    }

    /// `assignment = logical_or [ ASSIGN assignment ] ;` —
    /// right-associative.
    ///
    /// See the module doc comment's "`:=` disambiguation" section: there
    /// is only ONE assignment token (`ASSIGN`), so the LHS's own
    /// *lowered shape* decides `Assign` vs `Define` — exactly mirroring
    /// `derive-runtime::lower::lower_assignment`. `F(x, y) := body`
    /// (LHS lowers to `SymApply{head: SymSymbol(F), args: [x, y]}`)
    /// becomes `Define(F, List(x, y), body)`; a bare `x := body` (LHS is
    /// a plain symbol, or anything else call-shaped with a non-`SymSymbol`
    /// head) becomes `Assign(x, body)`.
    fn lower_assignment(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, DeriveLowerError> {
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
                // F(x, y) := body — a function definition. Derive has no
                // pattern syntax, so each parameter already lowered to a
                // plain `SymSymbol` (or, for a malformed definition like
                // `F(1) := …`, whatever the caller wrote — a later pass
                // owns validating parameter shapes, not this lowering,
                // mirroring how `derive-runtime`'s own lowering does not
                // validate parameter shapes either).
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
    /// `derive-runtime::lower::lower_logical_chain` and
    /// `macsyma-to-semantic-ir::lower::lower_logical_chain` exactly).
    fn lower_logical_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        head: &str,
    ) -> Result<Expr, DeriveLowerError> {
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

    /// `logical_not = "NOT" logical_not | comparison ;`
    ///
    /// `NOT` is matched in the grammar as a literal keyword (a reserved
    /// word the D-2 lexer promotes from a plain `NAME` via
    /// `derive.tokens`'s `keywords:` block, not a distinct regex-declared
    /// token type like `PLUS`/`EQ`), so — mirroring
    /// `derive-runtime::lower::lower_logical_not`'s and
    /// `macsyma-to-semantic-ir::lower::lower_logical_not`'s identical
    /// checks for their own keyword tokens — this checks the token's
    /// literal *value*, not `effective_type_name()`.
    fn lower_logical_not(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, DeriveLowerError> {
        let has_not = node
            .children
            .iter()
            .any(|c| as_token(c).is_some_and(|t| t.value == "NOT"));
        if !has_not {
            return self.lower_first_node(node, depth);
        }
        let inner = child_nodes(node)
            .next()
            .ok_or_else(|| self.err_at(node, "`NOT` with no operand".to_string()))?;
        let operand = self.lower_node(inner, depth + 1)?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(NOT, span.clone()), vec![operand], span))
    }

    /// `comparison = additive [ (EQ|LE|LESS|GREATER|GE) additive ] ;` — a
    /// single (non-chained) comparison. `=` is Derive's *equation*
    /// operator (`Equal`), never assignment — `:=` alone owns that role
    /// (MA07 §3). Unlike Macsyma's `#` (not-equal), Derive's grammar has
    /// no not-equal comparison token at all.
    fn lower_comparison(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, DeriveLowerError> {
        let Some(op_index) = node
            .children
            .iter()
            .position(|c| as_token(c).is_some_and(|t| comparison_head(token_type(t)).is_some()))
        else {
            return self.lower_first_node(node, depth);
        };
        if op_index == 0 || op_index + 1 >= node.children.len() {
            return Err(self.err_at(node, "malformed comparison node".to_string()));
        }
        let head = comparison_head(token_type(as_token(&node.children[op_index]).unwrap())).unwrap();
        let lhs = self.lower_child(&node.children[op_index - 1], depth + 1)?;
        let rhs = self.lower_child(&node.children[op_index + 1], depth + 1)?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(head, span.clone()), vec![lhs, rhs], span))
    }

    /// `additive`/`multiplicative` — a left-associative binary chain of
    /// `+`/`-`/`*`/`/`. Must fold pairwise (not n-ary, unlike the logical
    /// chains) since a single chain can mix operators: `a - b - c` folds
    /// left into `Sub(Sub(a, b), c)`; `a + b - c` into `Sub(Add(a, b),
    /// c)`.
    ///
    /// Like Wolfram's and Macsyma's grammar, Derive's grammar collapses a
    /// flat run of same-precedence operators into ONE CST node with many
    /// children rather than nesting through parens — see
    /// [`Self::check_chain_length`] for why this needs its own cap
    /// independent of `MAX_EXPR_DEPTH`.
    fn lower_binary_chain(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, DeriveLowerError> {
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

    /// `unary = MINUS unary | power ;` — Derive's grammar (unlike
    /// Wolfram's and Macsyma's) has NO unary-plus alternative at all: a
    /// leading `-` is `Neg`; otherwise it is the inner `power`.
    fn lower_unary(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, DeriveLowerError> {
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

    /// `power = postfix [ POWER unary ] ;` — right-associative `^`. The
    /// grammar's own precedence trick of routing the RHS back through
    /// `unary` (which itself falls through to `power` again absent a
    /// leading `-`) is what gives `a^b^c` its right-associative shape, not
    /// any special-casing here.
    fn lower_power(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, DeriveLowerError> {
        match node.children.len() {
            1 => self.lower_child(&node.children[0], depth + 1),
            3 => {
                let lhs = self.lower_child(&node.children[0], depth + 1)?;
                let rhs = self.lower_child(&node.children[2], depth + 1)?;
                let span = self.span_of(node);
                Ok(self.sym_apply(self.sym_symbol_bare(POW, span.clone()), vec![lhs, rhs], span))
            }
            _ => Err(self.err_at(node, "malformed power node".to_string())),
        }
    }

    /// `postfix = atom { LPAREN [ arglist ] RPAREN } ;` — function
    /// application, left-associative and chainable (`F(x)(y)` is
    /// `(F(x))(y)`, though Derive has no idiom that actually produces
    /// one — included for grammar fidelity, mirrors
    /// `derive-runtime::lower::lower_postfix`'s identical comment).
    ///
    /// Like Macsyma's `postfix` (and unlike Wolfram's, whose second suffix
    /// shape `[[…]]` Part-indexing multiplies against the call-argument
    /// count), Derive's `postfix` has only this one suffix shape, so
    /// [`Self::check_postfix_chain_length`]'s plain linear cap on chained
    /// call groups is exact.
    fn lower_postfix(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, DeriveLowerError> {
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

    /// Apply `head` to `args`, bridging a UPPERCASE builtin surface
    /// function name (`SIN`, `DIF`, `IF`, …) to its canonical IR head via
    /// [`standard_function`] — mirrors `derive-runtime::lower::
    /// lower_postfix`'s `canonical_head` step and
    /// `macsyma-to-semantic-ir::lower::build_application`'s identical
    /// shape. Unlike Wolfram's `build_application`, there is no
    /// associative n-ary left-fold here: Derive has no explicit-head-
    /// application sugar analogous to Wolfram's `Plus[1, 2, 3]` (a call is
    /// always just a call), so this is a plain wrap.
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
    fn lower_arglist(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Vec<Expr>, DeriveLowerError> {
        self.lower_child_nodes(node, depth)
    }

    /// `vector = LBRACKET row { SEMI row } RBRACKET ;` (D-5, MA07 §2/§3).
    ///
    /// A vector `[a, b, c]` parses as exactly one `row`; a matrix `[a, b,
    /// c; d, e, f]` parses as more than one — the grammar has no separate
    /// rule for the two shapes (see `derive.grammar`'s own comment on
    /// `vector`), so this is where they're told apart, purely by counting
    /// `row` children: one row lowers to a flat `SymApply{head: List,
    /// args: elems…}`, more than one lowers to a `List` of per-row
    /// `List`s — mirrors `derive-runtime::lower::lower_vector` exactly.
    fn lower_vector(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, DeriveLowerError> {
        let rows: Vec<&GrammarASTNode> = child_nodes(node).filter(|n| n.rule_name == "row").collect();
        self.check_apply_arg_count(node, rows.len())?;
        let span = self.span_of(node);
        if rows.len() == 1 {
            let elems = self.lower_row(rows[0], depth + 1)?;
            Ok(self.sym_apply(self.sym_symbol_bare(LIST, span.clone()), elems, span))
        } else {
            let mut row_lists = Vec::with_capacity(rows.len());
            for row in rows {
                let elems = self.lower_row(row, depth + 1)?;
                let row_span = self.span_of(row);
                row_lists.push(self.sym_apply(self.sym_symbol_bare(LIST, row_span.clone()), elems, row_span));
            }
            Ok(self.sym_apply(self.sym_symbol_bare(LIST, span.clone()), row_lists, span))
        }
    }

    /// `row = expr { COMMA expr } ;` — lower each comma-separated element
    /// (mirrors [`Self::lower_arglist`]'s identical shape).
    fn lower_row(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Vec<Expr>, DeriveLowerError> {
        let elems = self.lower_child_nodes(node, depth)?;
        self.check_apply_arg_count(node, elems.len())?;
        Ok(elems)
    }

    /// `atom = NUMBER | NAME | vector | group ;`
    fn lower_atom(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, DeriveLowerError> {
        if let Some(child) = child_nodes(node).next() {
            if matches!(child.rule_name.as_str(), "vector" | "group") {
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

    /// `group = LPAREN expr RPAREN ;` — grouping only.
    fn lower_group(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, DeriveLowerError> {
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
    /// their intent legible; mirrors `wolfram-to-semantic-ir::lower::
    /// Lowerer::sym_symbol_bare` and `macsyma-to-semantic-ir`'s
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
    /// `derive-runtime::lower::lower_number`'s identical rule). An
    /// integer lexeme too large for `i64` falls back to a float rather
    /// than silently truncating.
    ///
    /// **Must** be an instance method, not a free function: every branch
    /// that constructs a `FloatLit` calls `self.observed.add(Feature::
    /// Floats)` immediately. This is a confirmed, previously-shipped bug
    /// in both `matlab-to-semantic-ir` and `wolfram-to-semantic-ir`
    /// (their number-literal helpers were free functions with no access
    /// to `observed`, so a float-literal-only module failed
    /// `semantic_ir::validate()`), fixed proactively in
    /// `macsyma-to-semantic-ir` and carried forward here.
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
    /// Derive's grammar, like Wolfram's and Macsyma's, collapses a flat
    /// run of same-precedence operators into ONE CST node with many
    /// children rather than nesting through parens, so a long
    /// unparenthesized chain (`1 + 1 + ... + 1`, thousands of terms)
    /// never trips the ordinary grammar-nesting depth guard
    /// (`derive-parser`'s `MAX_RULE_DEPTH`, which counts *nesting*, not
    /// the length of one flat repetition). But folding N operands
    /// left-associatively still builds an N-deep *binary* `Expr` tree,
    /// and that tree's own depth is what every later recursive pass over
    /// it pays for regardless of how cheaply each fold step was.
    fn check_chain_length(&self, node: &GrammarASTNode) -> Result<(), DeriveLowerError> {
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

    /// Cap the number of chained call groups (`F(x)(y)(z)…`) in a single
    /// `postfix` node at `MAX_EXPR_DEPTH`. Derive's `postfix` has only
    /// ONE suffix shape — a call `(...)` — so a plain count of chained
    /// call groups already bounds the real nesting depth this loop can
    /// build, one-to-one (see `macsyma-to-semantic-ir::lower::
    /// check_postfix_chain_length`'s doc comment for the fuller
    /// comparison against Wolfram's cumulative-budget variant, needed
    /// there only because Wolfram's `postfix` has a second suffix shape
    /// that multiplies against this one).
    fn check_postfix_chain_length(&self, node: &GrammarASTNode) -> Result<(), DeriveLowerError> {
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

    /// Cap the argument count of a single `F(…)` application or the
    /// element/row count of a `[…]` vector/matrix literal. Unlike
    /// [`Self::check_chain_length`], neither an arglist nor a vector row
    /// list folds into a nested tree (both stay a flat `Vec<Expr>`), so
    /// this is not a stack-recursion guard — it is a modest
    /// defense-in-depth cap on a single allocation's size, using the same
    /// `MAX_EXPR_DEPTH` bound for consistency rather than inventing a
    /// second unrelated constant.
    fn check_apply_arg_count(&self, node: &GrammarASTNode, count: usize) -> Result<(), DeriveLowerError> {
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

    fn lower_first_node(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, DeriveLowerError> {
        let child = child_nodes(node)
            .next()
            .ok_or_else(|| self.err_at(node, format!("`{}` has no expression child", node.rule_name)))?;
        self.lower_node(child, depth + 1)
    }

    fn lower_child(&mut self, child: &ASTNodeOrToken, depth: usize) -> Result<Expr, DeriveLowerError> {
        match child {
            ASTNodeOrToken::Node(node) => self.lower_node(node, depth),
            ASTNodeOrToken::Token(token) => self.lower_token(token),
        }
    }

    fn lower_child_nodes(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Vec<Expr>, DeriveLowerError> {
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

    fn err_at(&self, node: &GrammarASTNode, message: String) -> DeriveLowerError {
        DeriveLowerError {
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

/// Map an arithmetic token type to its canonical IR head. Note `TIMES`,
/// not Macsyma's `STAR` — `derive.tokens` spells the multiplication token
/// `TIMES`.
fn binary_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "PLUS" => Some(ADD),
        "MINUS" => Some(SUB),
        "TIMES" => Some(MUL),
        "SLASH" => Some(DIV),
        _ => None,
    }
}

/// Map a comparison token type to its canonical IR head. Derive has no
/// not-equal comparison token at all (unlike Macsyma's `#`), so this
/// table is one entry shorter than `macsyma-to-semantic-ir::lower::
/// comparison_head`'s.
fn comparison_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "EQ" => Some(EQUAL),
        "LE" => Some(LESS_EQUAL),
        "LESS" => Some(LESS),
        "GREATER" => Some(GREATER),
        "GE" => Some(GREATER_EQUAL),
        _ => None,
    }
}

/// Bridge a Derive *surface* head (conventionally UPPERCASE — MA07 §3) to
/// the canonical IR head. `SymSymbol` equality is case-sensitive, so
/// every builtin needs an explicit entry here — not just the ones that
/// differ semantically (unlike Wolfram's bridge, which only needs to
/// rename the few operators whose long-form name isn't already the IR's
/// own). Mirrors `derive-runtime::lower::surface_head_to_ir`'s table
/// exactly (same surface names, same canonical `symbolic_ir` targets),
/// reusing the identical constants so the native-eval and SIR23 lowerings
/// can never drift apart on what a builtin canonicalizes to. A head not
/// in this table (a user-defined function, or a builtin not spelled in
/// the exact uppercase convention) is returned unchanged, so `F(x)` stays
/// a harmless unevaluated symbolic call and an unrecognised spelling
/// (e.g. lowercase `sin(x)`) is just an ordinary user symbol/call, not
/// the builtin.
///
/// `LIM`/`SOLVE`/`SUM`/`PRODUCT`/`TAYLOR` are deliberately absent, exactly
/// matching `derive-runtime`'s own disclosed scope boundary (MA07 §4,
/// "Honest scope") — the shared VM/IR has no existing canonical head for
/// them, so wiring them here would be new-head invention, not reuse.
fn standard_function(name: &str) -> Option<&'static str> {
    match name {
        "DIF" => Some(D),
        "INT" => Some(INTEGRATE),
        "IF" => Some(IF),
        "SIN" => Some(SIN),
        "COS" => Some(COS),
        "TAN" => Some(TAN),
        "SQRT" => Some(SQRT),
        "EXP" => Some(EXP),
        "LOG" => Some(LOG),
        "ATAN" => Some(ATAN),
        "ASIN" => Some(ASIN),
        "ACOS" => Some(ACOS),
        "SINH" => Some(SINH),
        "COSH" => Some(COSH),
        "TANH" => Some(TANH),
        "ASINH" => Some(ASINH),
        "ACOSH" => Some(ACOSH),
        "ATANH" => Some(ATANH),
        "COTH" => Some(COTH),
        "SECH" => Some(SECH),
        "CSCH" => Some(CSCH),
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
/// `SymReplaceAll` node in the first place (Derive's grammar has no
/// pattern-matching or rewrite-rule syntax at all).
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
/// `wolfram-to-semantic-ir::lower::unwrap_single` and
/// `derive-runtime::lower::unwrap_single`).
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
