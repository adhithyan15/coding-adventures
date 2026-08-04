//! The lowering pass from `coding_adventures_macsyma_parser`'s generic
//! [`GrammarASTNode`] CST → [`semantic_ir::Module`], **v0.1.0**.
//!
//! This is the **second** frontend to target
//! [SIR23](../../../specs/SIR23-symbolic-pattern-semantic-ir.md), the
//! symbolic-expression/pattern-matching domain extension of the SIR10
//! narrow-waist IR (Stream B of
//! [`HML01`](../../../specs/HML01-math-to-semantic-ir.md)). The first,
//! `wolfram-to-semantic-ir`, is this crate's design template — read its
//! `lower.rs` module doc comment first; everything below assumes that
//! context and only calls out where Macsyma's grammar differs.
//!
//! # Retargeting `macsyma-compiler`, not starting from scratch
//!
//! `macsyma-compiler` already walks this exact CST and compiles it to
//! `symbolic_ir::IRNode` (`Symbol`/`Integer`/`Rational`/`Float`/`Str`/
//! `Apply` — see that crate's `Compiler::compile_node`). Every control-flow
//! construct Macsyma has — `if`/`elseif`/`else`, `while`, `for`, `block`,
//! `return` — already lowers there to plain `Apply` data with a synthetic
//! head symbol (`If`/`While`/`ForEach`/`ForRange`/`Block`/`Return`), not to
//! a host-language statement, because `symbolic_ir::IRNode` has no
//! statement/control-flow vocabulary at all: it is a **symbolic expression**
//! IR, and Macsyma's own native runtime (`macsyma-runtime`) interprets that
//! `Apply` data directly. SIR23 mirrors that same "everything is data"
//! shape (`SymSymbol`/`SymApply`) for exactly the same reason `wolfram-to-
//! semantic-ir` gives in its own module doc comment, so this crate's job is
//! mechanical: walk the same CST, dispatch on the same rule names
//! `macsyma-compiler::Compiler::compile_node` already uses, and construct
//! `semantic_ir::Expr::{SymSymbol,SymApply}` wherever that compiler
//! constructs `symbolic_ir::IRNode::{Symbol,Apply}`. Literals
//! (`Integer`/`Float`/`Str`) reuse SIR10/SIR16's `IntLit`/`FloatLit`/
//! `StrLit` directly, per the SIR23 spec.
//!
//! # Scope (v0.1.0) — no pattern-matching or rewrite-rule syntax at all
//!
//! Unlike Wolfram, Macsyma's currently-implemented grammar (all 24 rules in
//! `macsyma-parser`, see `code/grammars/macsyma/macsyma.grammar`) has **no**
//! pattern-matching or rewrite-rule surface syntax whatsoever:
//!
//! - No `_`/blank, no named-pattern (`x_`) shape.
//! - No `->`/`:>` rule arrow — the lexer tokenizes `ARROW` (`->`) but no
//!   parser rule ever consumes it (see `macsyma.tokens`'s `ARROW = "->"`
//!   and the grammar's own precedence cascade, which has no rule
//!   referencing `ARROW` at all).
//! - No `/.`/`//.` replacement operators, no `|`/`?`/`/;` pattern sugar.
//!
//! This crate therefore **only ever constructs [`Expr::SymSymbol`] and
//! [`Expr::SymApply`]** (plus the reused `IntLit`/`FloatLit`/`StrLit`
//! literal nodes) — it never constructs `SymPatternBlank`/
//! `SymPatternNamed`/`SymRule`/`SymReplaceAll`, and it never observes
//! `Feature::PatternMatching`. This is a disclosed scope boundary matching
//! the grammar's actual surface, not an oversight: a future Macsyma grammar
//! revision that adds `matchdeclare`/pattern-matching builtins would need a
//! new grammar rule before this crate could ever emit those nodes. One
//! concrete consequence: [`measure_depth_iterative`]/[`drop_iterative`]
//! below only need a match arm for [`Expr::SymApply`] (recursing into
//! `head` and `args`) — every other `Expr` variant is a leaf for this
//! crate's purposes, since a `SymPatternBlank`/`SymPatternNamed`/`SymRule`/
//! `SymReplaceAll` node can never appear in a tree this crate builds.
//!
//! Every one of Macsyma's 24 implemented grammar productions IS covered:
//! literals, arithmetic (`+ - * /`, unary, `^`/`**`), comparisons (`= # < >
//! <= >=`, non-chaining), logic (`and`/`or`/`not`), lists (`[…]`), function
//! application (`f(x)`, chainable `f(x)(y)`), assignment (`:`) and function
//! definition (`:=`), and the control-flow forms (`if`/`elseif`/`else`,
//! `while`, `for … in … do`, `for … thru/while/unless … do`, `block(…)`,
//! `return(…)`).
//!
//! # Recursion-depth hardening — carried over proactively, not discovered
//!
//! `wolfram-to-semantic-ir`'s `CHANGELOG.md` documents four rounds of
//! security review that each found a real, adversarially-confirmed native
//! stack-overflow gap (flat operator chains, chained postfix application,
//! a multiplicative bracket×index composition, and cross-`(...)`-boundary
//! composition) plus one further round that found its own fix's rejection
//! path re-crashed on `Drop`. Every one of those hardening mechanisms is
//! applied here from day one:
//!
//! - [`MAX_EXPR_DEPTH`] bounds this crate's own CST-walking recursion.
//! - [`Lowerer::check_chain_length`] caps every flat, same-precedence
//!   operator-chain fold (`additive`/`multiplicative`/`logical_or`/
//!   `logical_and`) before any tree is built — the same "flat CST node,
//!   deep folded tree" risk `wolfram-to-semantic-ir`'s identically-named
//!   guard documents.
//! - [`Lowerer::check_postfix_chain_length`] caps chained call application
//!   (`f(x)(y)(z)…`). Macsyma's `postfix` has only ONE suffix shape (a
//!   call) — there is no `[[…]]` Part-indexing sugar to multiply against —
//!   so, unlike Wolfram's `add_chain_depth` cumulative budget (needed
//!   because an `LDBRACKET` group there folds one `Part` per *index*,
//!   making groups × indices-per-group multiply), a single per-chain count
//!   of call groups is already an exact bound: each `(...)` group adds
//!   exactly one `SymApply` wrap to the tree regardless of its own
//!   argument count. See that method's doc comment for the full argument.
//! - [`Lowerer::check_if_chain_length`] caps the `if`/`elseif`/`else`
//!   chain — a construct Wolfram's grammar has no equivalent of at all.
//!   Macsyma's `if_expr` grammar rule folds a flat `{ elseif expr then
//!   expr }` repetition (cheap to parse — one CST node, however many
//!   clauses) into a nested nest of `If(cond, then, else)` `SymApply`s,
//!   one level per clause — exactly the same "flat source, deep IR" shape
//!   the other chain guards exist for, so it gets the identical treatment:
//!   reject the clause count *before* folding.
//! - [`Lowerer::check_apply_arg_count`] caps `arglist`/`list` element
//!   counts — a flat-`Vec` allocation-size backstop, not a stack guard.
//! - [`measure_depth_iterative`] is the authoritative, construction-
//!   composition-independent check: an iterative (never recursive, so it
//!   can never itself overflow) walk of an already-built `Expr`, called
//!   once per top-level statement in [`Lowerer::lower_file`] before it can
//!   reach the returned `Module`. Per-construct guards above are each
//!   scoped to one grammar node and do not compose across nested `(...)`
//!   boundaries (`wolfram-to-semantic-ir`'s security review found exactly
//!   this gap); this closes it regardless of how the tree was assembled.
//! - [`drop_iterative`] tears down a tree `measure_depth_iterative` just
//!   rejected, using an explicit work stack rather than the ordinary
//!   recursive `Drop` glue `semantic_ir::Expr` gets for free — detecting an
//!   oversized tree and then letting it fall out of scope normally would
//!   just relocate the same native stack overflow from "walking forward"
//!   to "walking backward" (the exact bug `wolfram-to-semantic-ir`'s
//!   fourth review round found).
//!
//! # `compile` vs. `compile_source`
//!
//! This module's [`compile`] is pure lowering over an already-parsed tree —
//! see `src/lib.rs`'s `compile_source` doc comment for why, unlike
//! `wolfram-to-semantic-ir`, this crate's `compile_source` does not need to
//! spawn an enlarged-stack worker thread: `macsyma-parser`'s own
//! `MAX_RULE_DEPTH` (200) is already documented safe on a bare default
//! (~2 MiB) stack with comfortable margin.

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use semantic_ir::{
    Block, EffectSet, Expr, Feature, FeatureManifest, Function, Metadata, Module, Span, Stmt,
};
use symbolic_ir::{
    ACOS, ACOSH, ADD, AND, ASIN, ASINH, ASSIGN, ATAN, ATANH, COS, COSH, D, DEFINE, DIV, EQUAL,
    EXP, GREATER, GREATER_EQUAL, INTEGRATE, LESS, LESS_EQUAL, LIST, LOG, MUL, NEG, NOT, NOT_EQUAL,
    OR, POW, SIN, SINH, SQRT, SUB, TAN, TANH,
};

/// Maximum expression-nesting depth for *this crate's own* lowering
/// recursion — distinct from (and independent of) `macsyma-parser`'s own
/// `MAX_RULE_DEPTH` grammar-nesting guard, which bounds the CST this crate
/// walks. Mirrors every other SIR frontend's identically-named,
/// identically-justified guard (see `wolfram-to-semantic-ir::lower::
/// MAX_EXPR_DEPTH`'s doc comment for the full "why 256" reasoning, which
/// applies unchanged here — both grammars' native-stack crash floors were
/// independently measured to be nearly identical, ~275-278 `parse_rule`
/// frames on a 2 MiB stack, since both share the same generic
/// `GrammarParser` dispatch engine).
const MAX_EXPR_DEPTH: usize = 256;

/// Synthetic file name used for all spans (the CST does not carry the
/// original path).
const FILE: &str = "<macsyma>";

// ---------------------------------------------------------------------------
// Surface-only head names (not exported by `symbolic_ir`, since these are
// synthetic heads a Macsyma-family frontend introduces to represent
// control-flow-as-data, not part of the shared symbolic-IR vocabulary
// itself — duplicated locally rather than imported from `macsyma-compiler`,
// matching `wolfram-to-semantic-ir`'s own precedent of local synthetic
// heads even where a sibling crate happens to export an equivalent).
// ---------------------------------------------------------------------------

const IF_HEAD: &str = "If";
const WHILE_HEAD: &str = "While";
const FOR_EACH_HEAD: &str = "ForEach";
const FOR_RANGE_HEAD: &str = "ForRange";
const BLOCK_HEAD: &str = "Block";
const RETURN_HEAD: &str = "Return";

// ---------------------------------------------------------------------------
// Public error type
// ---------------------------------------------------------------------------

/// An error encountered during Macsyma → SIR lowering.
///
/// Mirrors `WolframLowerError`/`MatlabLowerError`/`PythonLowerError`'s shape
/// exactly (`message` + 1-based `line`/`column`) so tooling can treat every
/// SIR frontend uniformly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacsymaLowerError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for MacsymaLowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "MacsymaLowerError at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for MacsymaLowerError {}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Lower a parsed Macsyma CST (rooted at the `program` rule) into a SIR
/// module.
///
/// This function does **not** itself guard against native stack overflow
/// on deeply-nested input beyond its own [`MAX_EXPR_DEPTH`] cap — it trusts
/// `tree` was already parsed under a suitable guard (`macsyma-parser`'s own
/// `MAX_RULE_DEPTH`). See `src/lib.rs`'s `compile_source` doc comment for
/// why, unlike `wolfram-to-semantic-ir::compile_source`, no worker-thread
/// stack enlargement is needed here: `macsyma-parser`'s `MAX_RULE_DEPTH`
/// (200) is already documented safe on a bare default stack.
pub fn compile(tree: &GrammarASTNode, module_name: &str) -> Result<Module, MacsymaLowerError> {
    Lowerer::new(module_name).lower_file(tree)
}

// ---------------------------------------------------------------------------
// The lowerer
// ---------------------------------------------------------------------------

/// The lowering pass's only mutable state: the module name (fixed at
/// construction) and the set of SIR features observed while lowering (used
/// to build the manifest so it declares *exactly* what the module emits —
/// see `semantic-ir/src/validator.rs`'s `check_expr`, the ground truth this
/// must match node-kind-for-node-kind).
///
/// Like `wolfram-to-semantic-ir`'s `Lowerer`, there is no per-function
/// name-resolution context here at all: under the "everything is data"
/// design this crate inherits from `macsyma-compiler` (see the module doc
/// comment), there are no host variables, parameters, or scopes to
/// resolve — even a function's formal parameters lower to plain
/// `SymSymbol`s inside a `List`, not to bound names. This lowerer is a
/// near-stateless recursive descent over the CST.
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
    // top level: `program = { statement }`
    // -------------------------------------------------------------------

    fn lower_file(&mut self, program: &GrammarASTNode) -> Result<Module, MacsymaLowerError> {
        if program.rule_name != "program" {
            return Err(self.err_at(
                program,
                format!("expected `program` root, got `{}`", program.rule_name),
            ));
        }

        let mut stmts: Vec<Stmt> = Vec::new();
        for stmt_node in child_nodes(program) {
            if stmt_node.rule_name != "statement" {
                continue;
            }
            let expr = self.lower_node(stmt_node, 0)?;
            if measure_depth_iterative(&expr).is_none() {
                let err = self.err_at(
                    stmt_node,
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
            .with_source_language("macsyma")
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
    /// that did not apply its own operator still emits its own node with a
    /// single child. [`unwrap_single`] peels those away so we dispatch on
    /// the first rule that genuinely shapes the tree (mirrors
    /// `wolfram-to-semantic-ir::lower::unwrap_single` and `macsyma-
    /// compiler`'s own `unwrap_node`, which this crate's dispatch table is
    /// otherwise a direct retarget of).
    fn lower_node(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MacsymaLowerError> {
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
                "statement" | "expression" => self.lower_first_node(node, depth),
                "assign" => self.lower_assign(node, depth),
                "logical_or" => self.lower_logical_chain(node, depth, OR),
                "logical_and" => self.lower_logical_chain(node, depth, AND),
                "logical_not" => self.lower_logical_not(node, depth),
                "comparison" => self.lower_comparison(node, depth),
                "additive" | "multiplicative" => self.lower_binary_chain(node, depth),
                "unary" => self.lower_unary(node, depth),
                "power" => self.lower_power(node, depth),
                "postfix" => self.lower_postfix(node, depth),
                "atom" => self.lower_first_node(node, depth),
                "group" => self.lower_group(node, depth),
                "list" => self.lower_list(node, depth),
                "if_expr" => self.lower_if(node, depth),
                "while_expr" => self.lower_while(node, depth),
                "for_expr" => self.lower_first_node(node, depth),
                "for_each_expr" => self.lower_for_each(node, depth),
                "for_range_expr" => self.lower_for_range(node, depth),
                "block_expr" => self.lower_block(node, depth),
                "return_expr" => self.lower_return(node, depth),
                "arglist" => Err(self.err_at(
                    node,
                    "an arglist cannot be lowered as a scalar expression".to_string(),
                )),
                other => Err(self.err_at(node, format!("no lowering for rule `{other}`"))),
            },
        }
    }

    /// Lower a raw token (a literal or a bare symbol).
    fn lower_token(&mut self, token: &Token) -> Result<Expr, MacsymaLowerError> {
        let span = self.token_span(token);
        match token_type(token) {
            "NUMBER" => Ok(self.number_literal_expr(&token.value, span)),
            "NAME" => Ok(self.sym_symbol(token.value.clone(), span)),
            // `GrammarLexer` already strips the surrounding quotes from any
            // STRING-typed token during tokenization, so `token.value` is
            // already the bare string content — no quote-stripping helper
            // needed (see the module doc comment; `macsyma-compiler::
            // Compiler::compile_token`'s identical `str_node(&token.value)`
            // confirms this is the established convention).
            "STRING" => Ok(Expr::StrLit {
                value: token.value.clone(),
                span,
            }),
            "KEYWORD" if token.value == "true" => Ok(self.sym_symbol("True".to_string(), span)),
            "KEYWORD" if token.value == "false" => Ok(self.sym_symbol("False".to_string(), span)),
            other => Err(MacsymaLowerError {
                message: format!("unexpected token `{other}` = {:?}", token.value),
                line: token.line,
                column: token.column,
            }),
        }
    }

    /// `assign = logical_or [ ( COLON | COLONEQ ) assign ] ;` —
    /// right-associative.
    ///
    /// `x : e` (`:`) lowers to `SymApply{head: Assign, args: [x, e]}` — a
    /// plain 2-argument equation/binding, pure data. `f(x, y) := body`
    /// (`:=`) lowers to a **3-argument** `SymApply{head: Define, args:
    /// [SymSymbol("f"), List(x, y), body]}`, mirroring `macsyma-compiler::
    /// Compiler::compile_assign`'s own shape exactly — this is deliberately
    /// NOT Wolfram's 2-argument `Define(Apply(f, params), body)` shape; the
    /// function name and its parameter list are separate arguments here. A
    /// bare `name := body` (no call-shaped LHS) falls back to
    /// `Define(name, List([]), body)` — an empty parameter list — again
    /// matching `macsyma-compiler`'s own existing behaviour exactly.
    fn lower_assign(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MacsymaLowerError> {
        let Some(op_index) = node
            .children
            .iter()
            .position(|c| as_token(c).is_some_and(|t| matches!(token_type(t), "COLON" | "COLONEQ")))
        else {
            return self.lower_first_node(node, depth);
        };
        if op_index == 0 || op_index + 1 >= node.children.len() {
            return Err(self.err_at(node, "malformed assign node".to_string()));
        }
        let lhs = self.lower_child(&node.children[op_index - 1], depth + 1)?;
        let rhs = self.lower_child(&node.children[op_index + 1], depth + 1)?;
        let op = token_type(as_token(&node.children[op_index]).unwrap());
        let span = self.span_of(node);

        if op == "COLONEQ" {
            if let Expr::SymApply { head, args, .. } = &lhs {
                if matches!(head.as_ref(), Expr::SymSymbol { .. }) {
                    let list_span = span.clone();
                    let params = self.sym_apply(
                        self.sym_symbol_bare(LIST, list_span.clone()),
                        args.clone(),
                        list_span,
                    );
                    return Ok(self.sym_apply(
                        self.sym_symbol_bare(DEFINE, span.clone()),
                        vec![(**head).clone(), params, rhs],
                        span,
                    ));
                }
            }
            let list_span = span.clone();
            let empty_params = self.sym_apply(self.sym_symbol_bare(LIST, list_span.clone()), vec![], list_span);
            return Ok(self.sym_apply(
                self.sym_symbol_bare(DEFINE, span.clone()),
                vec![lhs, empty_params, rhs],
                span,
            ));
        }
        Ok(self.sym_apply(self.sym_symbol_bare(ASSIGN, span.clone()), vec![lhs, rhs], span))
    }

    /// `logical_or`/`logical_and` — fold operands into an n-ary `And`/`Or`
    /// `SymApply` (a single flat apply carrying every operand at this
    /// precedence level, not a nested binary chain — mirrors `wolfram-to-
    /// semantic-ir::lower::lower_logical_chain` and `macsyma-compiler::
    /// Compiler::compile_logical_chain` exactly).
    fn lower_logical_chain(
        &mut self,
        node: &GrammarASTNode,
        depth: usize,
        head: &str,
    ) -> Result<Expr, MacsymaLowerError> {
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
    fn lower_logical_not(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MacsymaLowerError> {
        let has_not = node
            .children
            .iter()
            .any(|c| as_token(c).is_some_and(|t| t.value == "not"));
        if !has_not {
            return self.lower_first_node(node, depth);
        }
        let inner = child_nodes(node)
            .into_iter()
            .next()
            .ok_or_else(|| self.err_at(node, "`not` with no operand".to_string()))?;
        let operand = self.lower_node(inner, depth + 1)?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(NOT, span.clone()), vec![operand], span))
    }

    /// `comparison = additive [ ( EQ | HASH | LT | GT | LEQ | GEQ ) additive ] ;`
    /// — non-chaining: `a < b < c` is a parse error in Macsyma, so there is
    /// at most one comparison operator per node and no `check_chain_length`
    /// call is needed here.
    fn lower_comparison(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MacsymaLowerError> {
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
    /// `+`/`-`/`*`/`/`.
    ///
    /// Like Wolfram's (and MATLAB's) grammar, Macsyma's grammar collapses a
    /// flat run of same-precedence operators into ONE CST node with many
    /// children rather than nesting through parens — see
    /// [`Self::check_chain_length`] for why this needs its own cap
    /// independent of `MAX_EXPR_DEPTH`.
    fn lower_binary_chain(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MacsymaLowerError> {
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

    /// `unary = ( MINUS | PLUS ) unary | power ;`
    ///
    /// An exhaustive match on `children.len()` (mirroring `lower_power`'s
    /// own shape) rather than an early-return-then-index — a security
    /// review caught the prior version indexing `node.children[0]`
    /// unconditionally in the 2-child path with no defense against a
    /// hypothetical 0-child node (impossible under the current grammar, but
    /// every structurally analogous function in this file defends against
    /// that "shouldn't happen" shape explicitly rather than assuming it
    /// away, so this one now does too).
    fn lower_unary(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MacsymaLowerError> {
        match node.children.len() {
            1 => self.lower_child(&node.children[0], depth + 1),
            2 => {
                let op = token_type(
                    as_token(&node.children[0])
                        .ok_or_else(|| self.err_at(node, "unary op must be a token".to_string()))?,
                );
                let operand = self.lower_child(&node.children[1], depth + 1)?;
                if op == "MINUS" {
                    let span = self.span_of(node);
                    Ok(self.sym_apply(self.sym_symbol_bare(NEG, span.clone()), vec![operand], span))
                } else {
                    Ok(operand) // unary plus is a no-op
                }
            }
            _ => Err(self.err_at(node, "malformed unary node".to_string())),
        }
    }

    /// `power = postfix [ ( CARET | STAREQ ) unary ] ;` — right-associative
    /// `^`/`**` (the compiler normalizes both spellings to the same `Pow`
    /// head; the grammar's own precedence trick of routing the RHS back
    /// through `unary` — which itself falls through to `power` — is what
    /// gives `a^b^c` its right-associative shape, not any special-casing
    /// here).
    fn lower_power(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MacsymaLowerError> {
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
    /// application, chainable (`f(x)(y)(z)…`).
    ///
    /// Unlike Wolfram's `postfix` (which has a second suffix shape,
    /// `[[…]]` Part-indexing, that multiplies against the call-argument
    /// count — see [`Self::check_postfix_chain_length`]'s doc comment),
    /// Macsyma's `postfix` has only this one suffix shape, so a plain
    /// linear cap on the number of chained call groups is exact.
    fn lower_postfix(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MacsymaLowerError> {
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

    /// Apply `head` to `args`, bridging a lowercase built-in surface
    /// function name (`sin`, `diff`, …) to its canonical IR head via
    /// [`standard_function`] — mirrors `macsyma-compiler::Compiler::
    /// canonical_call_head` exactly. Unlike Wolfram's `build_application`,
    /// there is no associative n-ary left-fold here: Macsyma has no
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

    /// `arglist = expression { COMMA expression } ;` — lower each
    /// comma-separated argument. An arglist is a flat `Vec`, not a folded
    /// tree, so it has no stack-recursion risk analogous to the
    /// binary-chain rules — [`Self::check_apply_arg_count`] still bounds
    /// its raw length as a modest defense-in-depth cap on allocation size.
    fn lower_arglist(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Vec<Expr>, MacsymaLowerError> {
        self.lower_child_nodes(node, depth)
    }

    /// `group = LPAREN expression RPAREN ;` — grouping only.
    fn lower_group(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MacsymaLowerError> {
        let inner = child_nodes(node)
            .into_iter()
            .next()
            .ok_or_else(|| self.err_at(node, "empty group `( )`".to_string()))?;
        self.lower_node(inner, depth + 1)
    }

    /// `list = LBRACKET [ arglist ] RBRACKET ;` → `SymApply{head: List, …}`.
    fn lower_list(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MacsymaLowerError> {
        let args = self.lower_list_elements(node, depth)?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(LIST, span.clone()), args, span))
    }

    /// The raw element list of a `list` node, without wrapping in `List`
    /// (used both by [`Self::lower_list`] and by [`Self::lower_block`],
    /// which needs to distinguish "first argument is itself a list" from
    /// "no locals declaration").
    fn lower_list_elements(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Vec<Expr>, MacsymaLowerError> {
        let args = match child_nodes(node).into_iter().find(|n| n.rule_name == "arglist") {
            Some(arglist_node) => self.lower_arglist(arglist_node, depth + 1)?,
            None => vec![],
        };
        self.check_apply_arg_count(node, args.len())?;
        Ok(args)
    }

    /// `if_expr = "if" expression "then" expression { "elseif" expression
    /// "then" expression } [ "else" expression ] ;`
    ///
    /// Right-folds the flat `elseif` repetition into a nested chain of
    /// `SymApply{head: If, args: [cond, then, else]}`, one nesting level
    /// per clause, with the base `if`/`then` ending up as the OUTERMOST
    /// wrap and the optional `else` (or a synthetic `False` symbol when
    /// there is none) as the innermost fallback — mirrors `macsyma-
    /// compiler::Compiler::compile_if`'s exact fold order.
    /// [`Self::check_if_chain_length`] runs first, before any branch is
    /// even lowered — see that method's doc comment for the DoS reasoning.
    fn lower_if(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MacsymaLowerError> {
        self.check_if_chain_length(node)?;
        let branch_nodes = child_nodes(node);
        if branch_nodes.len() < 2 {
            return Err(self.err_at(node, "if expression needs a condition and a then-branch".to_string()));
        }
        let has_else = branch_nodes.len() % 2 == 1;

        let mut branches = Vec::with_capacity(branch_nodes.len());
        for b in branch_nodes {
            branches.push(self.lower_node(b, depth + 1)?);
        }

        let span = self.span_of(node);
        let mut fallback = if has_else {
            branches.pop().unwrap()
        } else {
            self.sym_symbol_bare("False", span.clone())
        };

        // `branches` now holds an even run of [cond1, then1, cond2, then2,
        // …]; popping two at a time from the END processes the LAST
        // (cond, then) pair first, nesting it as the innermost `If` around
        // the current fallback, and works backward to the FIRST pair last
        // — so the base `if`/`then` ends up wrapping everything else, and
        // each earlier `elseif` nests one level further in. This matches
        // `compile_if`'s own right-to-left index walk without needing to
        // clone any branch value.
        while let Some(then_branch) = branches.pop() {
            let cond = branches
                .pop()
                .ok_or_else(|| self.err_at(node, "malformed if/elseif chain".to_string()))?;
            fallback = self.sym_apply(
                self.sym_symbol_bare(IF_HEAD, span.clone()),
                vec![cond, then_branch, fallback],
                span.clone(),
            );
        }
        Ok(fallback)
    }

    /// `while_expr = "while" expression "do" expression ;`
    fn lower_while(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MacsymaLowerError> {
        let branches = child_nodes(node);
        if branches.len() != 2 {
            return Err(self.err_at(node, "while expression needs a condition and a body".to_string()));
        }
        let cond = self.lower_node(branches[0], depth + 1)?;
        let body = self.lower_node(branches[1], depth + 1)?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(WHILE_HEAD, span.clone()), vec![cond, body], span))
    }

    /// `for_each_expr = "for" NAME "in" expression "do" expression ;`
    fn lower_for_each(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MacsymaLowerError> {
        let variable = node
            .children
            .iter()
            .find_map(|c| as_token(c).filter(|t| token_type(t) == "NAME").map(|t| t.value.clone()));
        let Some(variable) = variable else {
            return Err(self.err_at(node, "for-each expression missing loop variable".to_string()));
        };
        let branches = child_nodes(node);
        if branches.len() != 2 {
            return Err(self.err_at(node, "for-each expression malformed".to_string()));
        }
        let iter = self.lower_node(branches[0], depth + 1)?;
        let body = self.lower_node(branches[1], depth + 1)?;
        let span = self.span_of(node);
        let var_sym = self.sym_symbol_bare(variable, span.clone());
        Ok(self.sym_apply(
            self.sym_symbol_bare(FOR_EACH_HEAD, span.clone()),
            vec![var_sym, iter, body],
            span,
        ))
    }

    /// `for_range_expr = "for" NAME [ ":" expression ] [ "step" expression ]
    /// ( "thru" | "while" | "unless" ) expression "do" expression ;`
    ///
    /// `start`/`step` default to `1` when omitted, matching the grammar's
    /// own documented defaults and `macsyma-compiler::Compiler::
    /// compile_for_range`'s exact fallback shape.
    fn lower_for_range(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MacsymaLowerError> {
        let variable = node
            .children
            .iter()
            .find_map(|c| as_token(c).filter(|t| token_type(t) == "NAME").map(|t| t.value.clone()));
        let Some(variable) = variable else {
            return Err(self.err_at(node, "for-range expression missing loop variable".to_string()));
        };
        let branch_nodes = child_nodes(node);
        if branch_nodes.len() < 2 {
            return Err(self.err_at(node, "for-range expression malformed".to_string()));
        }
        let mut branches = Vec::with_capacity(branch_nodes.len());
        for b in branch_nodes {
            branches.push(self.lower_node(b, depth + 1)?);
        }
        let span = self.span_of(node);
        let (start, step, end, body) = match branches.as_slice() {
            [end, body] => (
                Expr::IntLit { value: 1, span: span.clone() },
                Expr::IntLit { value: 1, span: span.clone() },
                end.clone(),
                body.clone(),
            ),
            [start, end, body] => (
                start.clone(),
                Expr::IntLit { value: 1, span: span.clone() },
                end.clone(),
                body.clone(),
            ),
            [start, step, end, body, ..] => (start.clone(), step.clone(), end.clone(), body.clone()),
            _ => return Err(self.err_at(node, "for-range expression malformed".to_string())),
        };
        let var_sym = self.sym_symbol_bare(variable, span.clone());
        Ok(self.sym_apply(
            self.sym_symbol_bare(FOR_RANGE_HEAD, span.clone()),
            vec![var_sym, start, step, end, body],
            span,
        ))
    }

    /// `block_expr = "block" "(" [ arglist ] ")" ;`
    ///
    /// `block([x: 0, y], stmt1, stmt2, …)` → `Block(List(locals…), stmt1,
    /// …)` when the first argument is itself a `[…]` list literal (the
    /// locals declaration); `block(stmt1, stmt2, …)` → `Block(List(),
    /// stmt1, …)` otherwise — mirrors `macsyma-compiler::Compiler::
    /// compile_block`'s exact heuristic.
    fn lower_block(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MacsymaLowerError> {
        let span = self.span_of(node);
        let Some(args_node) = child_nodes(node).into_iter().find(|n| n.rule_name == "arglist") else {
            let empty_locals = self.sym_apply(self.sym_symbol_bare(LIST, span.clone()), vec![], span.clone());
            return Ok(self.sym_apply(self.sym_symbol_bare(BLOCK_HEAD, span.clone()), vec![empty_locals], span));
        };
        let args = self.lower_arglist(args_node, depth + 1)?;
        self.check_apply_arg_count(node, args.len())?;
        if args.first().is_some_and(is_list_apply) {
            Ok(self.sym_apply(self.sym_symbol_bare(BLOCK_HEAD, span.clone()), args, span))
        } else {
            let mut with_locals = vec![self.sym_apply(self.sym_symbol_bare(LIST, span.clone()), vec![], span.clone())];
            with_locals.extend(args);
            Ok(self.sym_apply(self.sym_symbol_bare(BLOCK_HEAD, span.clone()), with_locals, span))
        }
    }

    /// `return_expr = "return" "(" expression ")" ;`
    fn lower_return(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MacsymaLowerError> {
        let inner = child_nodes(node)
            .into_iter()
            .next()
            .ok_or_else(|| self.err_at(node, "return expression missing value".to_string()))?;
        let value = self.lower_node(inner, depth + 1)?;
        let span = self.span_of(node);
        Ok(self.sym_apply(self.sym_symbol_bare(RETURN_HEAD, span.clone()), vec![value], span))
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

    /// Build a `SymSymbol` for a *head* name, a loop variable, or any other
    /// internally-constructed symbol that is always immediately wrapped in
    /// a [`Self::sym_apply`] call — which itself observes the feature — so
    /// this helper does not need to (identical shape to [`Self::sym_symbol`],
    /// named separately only so call sites make their intent legible;
    /// mirrors `wolfram-to-semantic-ir::lower::Lowerer::sym_symbol_bare`).
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

    /// Parse a `NUMBER` lexeme into an `IntLit` or `FloatLit` (a `.`, `e`,
    /// or `E` means a real; otherwise an integer, matching `macsyma-
    /// compiler::parse_number`'s identical rule). An integer lexeme too
    /// large for `i64` falls back to a float rather than silently
    /// truncating.
    ///
    /// **Must** be an instance method, not a free function: every branch
    /// that constructs a `FloatLit` calls `self.observed.add(Feature::
    /// Floats)` immediately. This is a confirmed, currently-shipped bug in
    /// both `matlab-to-semantic-ir` and `wolfram-to-semantic-ir` (their
    /// number-literal helpers are free functions with no access to
    /// `observed`, so a float-literal-only module fails `semantic_ir::
    /// validate()` — the manifest never declares `Feature::Floats` even
    /// though the validator's own `check_expr` requires it for every
    /// `Expr::FloatLit` node) — this crate does not propagate that bug.
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
    /// Macsyma's grammar, like Wolfram's and MATLAB's, collapses a flat run
    /// of same-precedence operators into ONE CST node with many children
    /// rather than nesting through parens, so a long unparenthesized chain
    /// (`1 + 1 + ... + 1`, thousands of terms) never trips the ordinary
    /// grammar-nesting depth guard (`macsyma-parser`'s `MAX_RULE_DEPTH`,
    /// which counts *nesting*, not the length of one flat repetition). But
    /// folding N operands left-associatively still builds an N-deep
    /// *binary* `Expr` tree, and that tree's own depth is what every later
    /// recursive pass over it pays for regardless of how cheaply each fold
    /// step was — the exact bug class `matlab-to-semantic-ir` discovered
    /// the hard way and `wolfram-to-semantic-ir`'s identically-named guard
    /// documents; applied here from day one rather than retrofitted.
    fn check_chain_length(&self, node: &GrammarASTNode) -> Result<(), MacsymaLowerError> {
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
    /// `postfix` node at `MAX_EXPR_DEPTH`.
    ///
    /// Wolfram's analogous guard (`add_chain_depth`) needs a *cumulative*
    /// budget shared across an entire chain, because Wolfram's `postfix`
    /// has TWO suffix shapes — `[...]` call application and `[[...]]` Part
    /// indexing — and an `[[...]]` group folds one `Part` per *index*, so N
    /// chained groups each carrying M indices builds N×M real nesting
    /// levels (the two axes multiply, not add). Macsyma's `postfix` has
    /// only ONE suffix shape: a call `(...)`. A call group always adds
    /// exactly ONE level of `SymApply` nesting to `result`, regardless of
    /// how many arguments it carries — `f(a, b, c)` is one wrap, not
    /// three — so there is no second axis to multiply against, and a plain
    /// count of chained call groups, capped at `MAX_EXPR_DEPTH`, already
    /// bounds the real nesting depth this loop can build, one-to-one.
    fn check_postfix_chain_length(&self, node: &GrammarASTNode) -> Result<(), MacsymaLowerError> {
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

    /// Reject an `if`/`elseif`/`else` chain with more than `MAX_EXPR_DEPTH`
    /// (cond, then) branches, checked BEFORE any branch is lowered or any
    /// `If` `SymApply` is built.
    ///
    /// This is a DoS-risk class Wolfram's grammar has no equivalent of at
    /// all (it has no `if_expr` production), but the same fundamental shape
    /// [`Self::check_chain_length`] guards against for the flat operator
    /// chains: Macsyma's `if_expr` grammar rule folds a flat `{ elseif expr
    /// then expr }` repetition — ONE CST node, cheap to parse regardless of
    /// how many `elseif` clauses it lists — into a NESTED chain of `If`
    /// `SymApply`s, one nesting level per clause (see
    /// [`Lowerer::lower_if`]). A source file with thousands of `elseif`
    /// clauses is therefore cheap to produce and parse, but would build a
    /// proportionally deep lowered tree that every later recursive pass
    /// (the validator, a backend's emit pass, even plain `Drop`) pays for —
    /// so, exactly like the flat-chain guards, this rejects the clause
    /// count up front, before the (already-too-deep) tree is ever built.
    fn check_if_chain_length(&self, node: &GrammarASTNode) -> Result<(), MacsymaLowerError> {
        let branch_count = child_nodes(node).len();
        let has_else = branch_count % 2 == 1;
        let pair_count = if has_else { (branch_count - 1) / 2 } else { branch_count / 2 };
        if pair_count > MAX_EXPR_DEPTH {
            return Err(self.err_at(
                node,
                format!("if/elseif chain too long ({pair_count} branches, exceeds {MAX_EXPR_DEPTH})"),
            ));
        }
        Ok(())
    }

    /// Cap the argument count of a single `f(…)`/`[…]` application or list
    /// literal. Unlike [`Self::check_chain_length`], an arglist does not
    /// fold into a nested tree (it stays a flat `Vec<Expr>`), so this is
    /// not a stack-recursion guard — it is a modest defense-in-depth cap on
    /// a single allocation's size, using the same `MAX_EXPR_DEPTH` bound
    /// for consistency rather than inventing a second unrelated constant.
    fn check_apply_arg_count(&self, node: &GrammarASTNode, count: usize) -> Result<(), MacsymaLowerError> {
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

    fn lower_first_node(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Expr, MacsymaLowerError> {
        let child = child_nodes(node)
            .into_iter()
            .next()
            .ok_or_else(|| self.err_at(node, format!("`{}` has no expression child", node.rule_name)))?;
        self.lower_node(child, depth + 1)
    }

    fn lower_child(&mut self, child: &ASTNodeOrToken, depth: usize) -> Result<Expr, MacsymaLowerError> {
        match child {
            ASTNodeOrToken::Node(node) => self.lower_node(node, depth),
            ASTNodeOrToken::Token(token) => self.lower_token(token),
        }
    }

    fn lower_child_nodes(&mut self, node: &GrammarASTNode, depth: usize) -> Result<Vec<Expr>, MacsymaLowerError> {
        child_nodes(node)
            .into_iter()
            .map(|n| self.lower_node(n, depth))
            .collect()
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

    fn err_at(&self, node: &GrammarASTNode, message: String) -> MacsymaLowerError {
        MacsymaLowerError {
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
fn child_nodes(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    node.children
        .iter()
        .filter_map(|c| match c {
            ASTNodeOrToken::Node(n) => Some(n),
            ASTNodeOrToken::Token(_) => None,
        })
        .collect()
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

/// Map an arithmetic token type to its canonical IR head.
fn binary_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "PLUS" => Some(ADD),
        "MINUS" => Some(SUB),
        "STAR" => Some(MUL),
        "SLASH" => Some(DIV),
        _ => None,
    }
}

/// Map a comparison token type to its canonical IR head. Macsyma's `#`
/// means "not equal" — an idiosyncrasy of the language, unrelated to
/// Wolfram's unrelated `#` (`Slot`) meaning; there is no `Slot` concept in
/// Macsyma at all.
fn comparison_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "EQ" => Some(EQUAL),
        "HASH" => Some(NOT_EQUAL),
        "LT" => Some(LESS),
        "GT" => Some(GREATER),
        "LEQ" => Some(LESS_EQUAL),
        "GEQ" => Some(GREATER_EQUAL),
        _ => None,
    }
}

/// Bridge a lowercase Macsyma built-in call-head name to the canonical IR
/// head (`sin` → `Sin`, …). Returns `None` for anything else — an
/// unrecognised name (a user-defined function, or a Macsyma builtin like
/// `sum`/`solve`/`factor` with no `symbolic-ir` canonical constant) passes
/// through unchanged as a plain `SymSymbol` head, exactly as-is (lowercase,
/// verbatim) — mirrors `macsyma-compiler::standard_function`'s table,
/// restricted to the subset that already has a canonical `symbolic-ir`
/// constant (per this crate's scope: no new canonical heads are invented
/// for the builtins that don't have one).
fn standard_function(name: &str) -> Option<&'static str> {
    match name {
        "diff" => Some(D),
        "integrate" => Some(INTEGRATE),
        "sin" => Some(SIN),
        "cos" => Some(COS),
        "tan" => Some(TAN),
        "asin" => Some(ASIN),
        "acos" => Some(ACOS),
        "atan" => Some(ATAN),
        "sinh" => Some(SINH),
        "cosh" => Some(COSH),
        "tanh" => Some(TANH),
        "asinh" => Some(ASINH),
        "acosh" => Some(ACOSH),
        "atanh" => Some(ATANH),
        "log" => Some(LOG),
        "exp" => Some(EXP),
        "sqrt" => Some(SQRT),
        _ => None,
    }
}

/// True if `expr` is a `SymApply{head: List, ..}` — used by
/// [`Lowerer::lower_block`] to detect a locals declaration (`block([x: 0,
/// y], …)`) vs. a plain first statement.
fn is_list_apply(expr: &Expr) -> bool {
    matches!(expr, Expr::SymApply { head, .. } if matches!(head.as_ref(), Expr::SymSymbol { name, .. } if name == LIST))
}

/// Measure `expr`'s true tree depth **iteratively**, using an explicit
/// heap-allocated work stack rather than native recursion, so calling this
/// can never itself overflow the stack no matter how deep `expr` already
/// is. Building a deeply-nested `Box`-based tree only costs heap space
/// (each construction step is O(1) stack); the risk this guards against is
/// only in *walking* it recursively afterward.
///
/// Returns `None` as soon as the depth is certain to exceed
/// `MAX_EXPR_DEPTH`, `Some(depth)` otherwise.
///
/// Only needs a match arm for [`Expr::SymApply`] (recursing into `head`
/// and `args`) — every other `Expr` variant is a leaf for this crate's
/// purposes, since (per the module doc comment's scope note) this crate
/// can never construct a `SymPatternBlank`/`SymPatternNamed`/`SymRule`/
/// `SymReplaceAll` node in the first place (Macsyma's grammar has no
/// pattern-matching or rewrite-rule syntax at all).
///
/// This is the authoritative depth check every other guard in this file
/// (`MAX_EXPR_DEPTH`'s recursion-depth parameter, [`Lowerer::
/// check_chain_length`], [`Lowerer::check_postfix_chain_length`],
/// [`Lowerer::check_if_chain_length`]) is only an early, cheap
/// approximation of — those guards are each scoped to one grammar node and
/// do not compose across nested `(...)` boundaries (see `wolfram-to-
/// semantic-ir`'s `CHANGELOG.md` for the security-review finding this
/// mirrors). Called once per top-level statement in [`Lowerer::
/// lower_file`], so no tree this crate hands to a caller can ever actually
/// exceed `MAX_EXPR_DEPTH`, regardless of how its construction was
/// composed.
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
/// ordinary *recursive* compiler-derived `Drop` glue (`semantic_ir::Expr`
/// has no custom `Drop` impl of its own). `wolfram-to-semantic-ir`'s
/// security-review history confirmed this is a real, exploitable crash
/// (empirically, via an isolated subprocess) — moving a pathologically
/// deep tree past [`measure_depth_iterative`]'s detection only to then let
/// it drop normally just relocates the same native stack overflow from
/// "walking the tree forward" to "walking it backward".
///
/// The technique: take ownership of `expr`, and for the one nested
/// recursive field this crate's trees can ever have (`SymApply`'s `head`/
/// `args`), *move* those fields out onto an explicit heap-allocated work
/// stack instead of leaving them in place to be dropped as part of the
/// outer match's scrutinee. Each loop iteration therefore drops only one
/// node's own non-recursive fields (strings, spans) — the same technique a
/// hand-written `impl Drop for List` uses to avoid overflowing on a long
/// linked list, generalised from a list to a tree. Only needs the
/// `Expr::SymApply` arm for the same scope reason [`measure_depth_iterative`]
/// documents.
fn drop_iterative(expr: Expr) {
    let mut stack: Vec<Expr> = vec![expr];
    while let Some(node) = stack.pop() {
        if let Expr::SymApply { head, args, .. } = node {
            stack.push(*head);
            stack.extend(args);
        }
        // `node`'s own shell drops here — shallowly, since its only nested
        // `Expr` field (if any) was already moved out onto `stack` above.
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
/// `wolfram-to-semantic-ir::lower::unwrap_single` and `macsyma-compiler::
/// unwrap_node`).
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
