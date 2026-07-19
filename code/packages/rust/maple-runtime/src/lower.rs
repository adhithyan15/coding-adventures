//! # Lowering — Maple syntax tree → `symbolic-ir`
//!
//! The MP-3 [`maple-parser`](coding_adventures_maple_parser) hands us a
//! *generic* [`GrammarASTNode`] tree whose `rule_name`s mirror
//! `maple.grammar` (`statement`, `if_expr`, `assignment`, `arrow_def`,
//! `logical_or`, `comparison`, `additive`, `postfix`, `atom`, …). The parser
//! deliberately does **no** semantic work — it just records which rule
//! matched. This module is where Maple's *meaning* is assigned: every
//! surface construct is **desugared** into the canonical
//! [`symbolic_ir::IRNode`] head [`symbolic-vm`](symbolic_vm) already knows
//! how to evaluate, per MA09 §3's table.
//!
//! ## Much of this mirrors `reduce-runtime::lower`'s shape — with one
//! structural difference `reduce-parser` doesn't have
//!
//! Maple, like Reduce, is a "surface operators + `head(args)` calls"
//! language with no pattern/rewrite-rule vocabulary in this subset (MA09 §4
//! defers `patmatch`/`match` — they're ordinary library calls, not surface
//! grammar). But `maple.grammar` draws a hard line REDUCE's own grammar does
//! not: `statement = if_expr | assignment` sits in its own nonterminal,
//! never reachable from `expr` (see `maple.grammar`'s own "statements vs.
//! expressions" design-decision comment) — so, unlike
//! `reduce-runtime::lower_node`'s single dispatch table covering both
//! statement-shaped and expression-shaped rules uniformly, this module's
//! dispatch mirrors that same split: [`lower_node`] handles the ordinary
//! expression cascade (`logical_or` down to `atom`), while [`lower_if`] and
//! [`lower_assignment`] are the two `statement`-only forms, reached only via
//! `lower_node`'s own `"if_expr"`/`"assignment"` match arms (never nested
//! inside an arithmetic/comparison/logical operand the way Reduce's
//! `if_expr`/`assignment` can be).
//!
//! ## `Set` — a canonical head new to this repo (MA09 §5)
//!
//! Maple is the first language in this repo with **two** distinct bracketed
//! aggregate literals: `[a, b, c]` (ordered, `List`) and `{a, b, c}`
//! (unordered, `Set` — MA09 §3/§5). `symbolic-vm`'s shared handler table has
//! no handler for a `Set` head (confirmed by grepping
//! `symbolic_vm::handlers::build_handler_table` — see this crate's own
//! README/CHANGELOG for the full accounting), so [`SET`] is defined *locally
//! to this crate*, exactly the way `reduce-runtime::lower` defines its own
//! new `COMPOUND_EXPRESSION`/`CONS`/`FIRST`/… constants rather than adding
//! them to the shared `symbolic-ir` crate. `Set` is **not** a held head
//! (unlike `Assign`/`Define`/`If`/`Assume`/`Forget` — see
//! `symbolic_vm::backend::BaseBackend::new`), so its arguments *do* evaluate
//! before the VM discovers there is no handler; `on_unknown_head`'s default
//! fallback then leaves the call itself unevaluated — the *exact* same
//! disclosed gap MA09 §5 documents by name ("this subset's `{a, b, c}` set
//! literal lowers to the structurally-correct `Set[a, b, c]` ... but
//! evaluates as an unresolved call today"). Real Maple's set semantics
//! (unordered, duplicates silently removed) are therefore not yet enforced
//! at evaluation time — only the *shape* is correct, ready for a future item
//! that adds a real handler (to the shared table, or to a narrowly-scoped
//! Maple `Backend`) without any lowering change at all.
//!
//! ## `diff`/`int` — thin bridges to already-shared calculus handlers
//!
//! `diff(f, x)` and `int(f, x)` bridge to the canonical `D`/`Integrate`
//! heads — the *same* handlers Derive's `DIF`/`INT` and Wolfram's
//! `D`/`Integrate` already call under their own names (confirmed by
//! grepping `symbolic_vm::handlers::build_handler_table`: both are wired
//! whenever `SymbolicBackend::new()` builds the table, which it always does
//! with `simplify: true`). This crate reimplements no calculus — it is a
//! surface-name bridge only, the same shape `derive-runtime::lower`'s own
//! `"DIF" => D, "INT" => INTEGRATE` bridge uses. Multi-argument forms
//! (`diff(f, x, y)`, `int(f, x, a, b)`) are out of this subset's scope
//! (MA09 §4) — this lowering does not special-case argument counts at all,
//! so a wrong-arity call simply reaches `derivative_handler`/
//! `integrate_handler`'s own panic ("D expects 2 arguments, got N"), caught
//! by [`crate`]'s own worker-thread `catch_unwind` boundary exactly like any
//! other reused-handler panic.
//!
//! ## Booleans — the first literal `true`/`false` tokens in this CAS family
//!
//! Neither `reduce.grammar` nor `derive.grammar` has a dedicated boolean
//! *literal* token (their booleans arise purely from comparison/logic
//! results) — `maple.grammar`'s `atom` rule is the first to include `"true"`
//! and `"false"` as their own alternatives (MA09 §3, citing the
//! `type/truefalseFAIL` Help page). The established precedent for bridging a
//! literal boolean keyword to the shared backend's pre-bound `True`/`False`
//! symbols (`symbolic_vm::backend::BaseBackend::new` pre-binds exactly those
//! two names) is `macsyma-compiler`'s own `lower_token`: `"KEYWORD" if
//! token.value == "true" => Ok(sym("True"))` (and the `"false"` mirror) —
//! [`lower_token`] below reuses that exact bridge.

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use symbolic_ir::{
    apply, flt, int, sym, IRNode, ADD, AND, ASSIGN, DEFINE, DIV, D, EQUAL, GREATER, GREATER_EQUAL,
    IF, INTEGRATE, LESS, LESS_EQUAL, LIST, MUL, NEG, NOT, NOT_EQUAL, OR, POW, SUB,
};

/// The canonical head for a Maple set literal `{a, b, c}` (MA09 §3/§5).
///
/// Not exported by `symbolic-ir` — see this module's own doc comment's
/// "`Set`" section for the full accounting of why (no shared handler exists
/// yet, mirroring `reduce-runtime::lower`'s identical treatment of its own
/// new `COMPOUND_EXPRESSION`/`CONS`/list-accessor heads).
pub const SET: &str = "Set";

/// A failure while lowering the surface tree to IR. These are *structural*
/// errors — a node shape the lowering did not expect — not user syntax
/// errors (those are caught earlier by the parser).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LowerError {
    message: String,
}

impl LowerError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// The human-readable message.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for LowerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LowerError {}

/// One lowered top-level statement, tagged with whether its result should be
/// displayed.
///
/// MA09 §3's own statement-separator row is explicit that the `;`-vs-`:`
/// distinction is "a display flag on the surrounding session, not an IR
/// node" — so, unlike every other surface construct in this module, the
/// terminator choice is carried *alongside* the lowered [`IRNode`] rather
/// than folded into it. [`crate::MapleSession`] evaluates every statement
/// regardless (so `:`-suppressed side effects like `x := 5:` still bind
/// `x`), but only renders a displayed line for `Display` statements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoweredStatement {
    pub node: IRNode,
    pub display: Display,
}

/// Whether a lowered statement's evaluated result should be printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    /// `;`-terminated (or the optional final terminator-less statement,
    /// kept displayed for interactive convenience — mirroring
    /// `reduce-parser`'s/`macsyma-parser`'s identical trailing-statement
    /// allowance).
    Show,
    /// `:`-terminated (MA09 §3 — Programming Guide §5.3).
    Suppress,
}

/// Lower a parsed `program` node into one [`LoweredStatement`] per source
/// statement.
///
/// `maple.grammar`'s root is `program = { statement_line } [ statement ]`:
/// zero or more `;`/`:`-terminated `statement_line`s, plus an optional final
/// bare `statement` with no terminator (so a source file need not end with
/// one) — the identical shape `reduce.grammar`'s own `program` production
/// documents, for the identical reason (see that grammar's own comment,
/// reused verbatim by `maple.grammar`).
pub fn lower_program(root: &GrammarASTNode) -> Result<Vec<LoweredStatement>, LowerError> {
    if root.rule_name != "program" {
        return Err(LowerError::new(format!(
            "expected `program` root, got `{}`",
            root.rule_name
        )));
    }
    let mut statements = Vec::new();
    for child in child_nodes(root) {
        match child.rule_name.as_str() {
            "statement_line" => {
                let statement = child_nodes(child)
                    .find(|n| n.rule_name == "statement")
                    .ok_or_else(|| LowerError::new("statement_line has no statement"))?;
                let display = match statement_terminator(child) {
                    Some("COLON") => Display::Suppress,
                    _ => Display::Show,
                };
                statements.push(LoweredStatement {
                    node: lower_node(statement)?,
                    display,
                });
            }
            "statement" => statements.push(LoweredStatement {
                node: lower_node(child)?,
                display: Display::Show,
            }),
            _ => {}
        }
    }
    Ok(statements)
}

/// Find the `SEMI`/`COLON` terminator token type of a `statement_line` node.
fn statement_terminator(node: &GrammarASTNode) -> Option<&str> {
    node.children.iter().find_map(as_token).map(token_type)
}

/// Lower a single arbitrary node.
///
/// Most grammar rules are "transparent wrappers" — `expr` is just
/// `logical_or`, and every precedence tier that did not apply its own
/// operator still emits its own node with a single child.
/// [`unwrap_single`] peels those away so we dispatch on the first rule
/// that genuinely shapes the tree.
fn lower_node(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    match unwrap_single(node) {
        Unwrapped::Token(token) => lower_token(token),
        Unwrapped::Node(node) => match node.rule_name.as_str() {
            "program" => Err(LowerError::new("nested program node is not an expression")),
            "statement_line" | "statement" | "expr" => lower_first_node(node),
            "if_expr" => lower_if(node),
            "assignment" => lower_assignment(node),
            "logical_or" => lower_logical_chain(node, OR),
            "logical_and" => lower_logical_chain(node, AND),
            "logical_not" => lower_logical_not(node),
            "comparison" => lower_comparison(node),
            "additive" | "multiplicative" => lower_binary_chain(node),
            "unary" => lower_unary(node),
            "power" => lower_power(node),
            "postfix" => lower_postfix(node),
            "atom" => lower_atom(node),
            "arglist" => Err(LowerError::new(
                "an arglist cannot be lowered as a scalar expression",
            )),
            "list_literal" => lower_list_literal(node),
            "set_literal" => lower_set_literal(node),
            "group" => lower_group(node),
            other => Err(LowerError::new(format!("no lowering for rule `{other}`"))),
        },
    }
}

/// Lower a raw token (a literal or a bare symbol).
///
/// `"true"`/`"false"` are `KEYWORD` tokens (MA09 §3's own literal boolean
/// alternatives in `atom`) bridged to the shared backend's pre-bound
/// `True`/`False` symbols — see this module's own doc comment's "Booleans"
/// section for the `macsyma-compiler` precedent this mirrors.
fn lower_token(token: &Token) -> Result<IRNode, LowerError> {
    match token_type(token) {
        "NUMBER" => lower_number(&token.value),
        "NAME" => Ok(sym(&token.value)),
        "KEYWORD" if token.value == "true" => Ok(sym("True")),
        "KEYWORD" if token.value == "false" => Ok(sym("False")),
        other => Err(LowerError::new(format!(
            "unexpected token `{other}` = {:?}",
            token.value
        ))),
    }
}

/// Parse a `NUMBER` lexeme into an `Integer` or `Float` IR literal.
///
/// `maple.tokens`' `NUMBER` regex (`[0-9]+\.?[0-9]*([eE][+-]?[0-9]+)?`) is
/// identical to Reduce's/Derive's/Macsyma's own, so a `.`, `e`, or `E` means
/// it is a real; otherwise it is an integer. We reject an integer that
/// overflows `i64` (the IR's integer width) rather than silently wrapping.
fn lower_number(text: &str) -> Result<IRNode, LowerError> {
    if text.contains('.') || text.contains('e') || text.contains('E') {
        text.parse::<f64>()
            .map(flt)
            .map_err(|e| LowerError::new(format!("invalid real literal {text:?}: {e}")))
    } else {
        text.parse::<i64>()
            .map(int)
            .map_err(|e| LowerError::new(format!("invalid integer literal {text:?}: {e}")))
    }
}

/// `if_expr = "if" expr "then" statement { "elif" expr "then" statement }
/// [ "else" statement ] ( "end" "if" | "fi" )` — MA09 §3: each `elif`
/// desugars to a nested `If`, folded right-to-left so the *innermost* If is
/// either the final `else` body (if present) or a bare 2-arg `If` (if not) —
/// exactly the shape the spec's own worked example describes: `If[b, s1,
/// If[b2, s2, s3]]`.
///
/// Every child *node* of an `if_expr` (ignoring the `"if"`/`"then"`/
/// `"elif"`/`"else"`/`"end"`/`"fi"` keyword tokens, confirmed by reading
/// `maple-parser`'s own compiled grammar: `Group`/`Alternation` never
/// synthesize a wrapper node, they splice whichever branch matched directly
/// into the parent `Sequence`) appears in strict alternating
/// `(cond, body)` pairs, with one optional trailing lone `body` node for a
/// final `else` — so collecting `child_nodes` in order and walking it two
/// at a time (with an odd-length list signalling a trailing `else`) recovers
/// the whole chain without needing to distinguish `expr` nodes from
/// `statement` nodes structurally.
fn lower_if(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let nodes: Vec<&GrammarASTNode> = child_nodes(node).collect();
    if nodes.len() < 2 {
        return Err(LowerError::new("if_expr must have at least one branch"));
    }
    let has_else = nodes.len() % 2 == 1;
    let branch_count = if has_else {
        (nodes.len() - 1) / 2
    } else {
        nodes.len() / 2
    };

    let mut branches = Vec::with_capacity(branch_count);
    for i in 0..branch_count {
        let cond = lower_node(nodes[2 * i])?;
        let body = lower_node(nodes[2 * i + 1])?;
        branches.push((cond, body));
    }
    let else_body = if has_else {
        Some(lower_node(nodes[nodes.len() - 1])?)
    } else {
        None
    };

    // Fold right-to-left: the innermost branch's "else slot" is the final
    // `else` body if present, otherwise absent (a bare 2-arg `If`); every
    // earlier `elif` branch wraps the accumulated result as its own 3-arg
    // `If`'s else-slot.
    let mut acc = else_body;
    for (cond, body) in branches.into_iter().rev() {
        acc = Some(match acc {
            Some(prev) => apply(sym(IF), vec![cond, body, prev]),
            None => apply(sym(IF), vec![cond, body]),
        });
    }
    acc.ok_or_else(|| LowerError::new("if_expr produced no branches"))
}

/// `assignment = NAME ASSIGN ( arrow_def | expr ) | expr` — deliberately
/// NARROWER than `reduce-runtime::lower_assignment`'s general
/// `Apply(Symbol(_), _)`-shaped left-hand side: `maple.grammar`'s own
/// "assignment's left-hand side" design-decision comment explains why the
/// LHS here is a bare `NAME` *token*, full stop — `f(x) := expr` (Maple's
/// narrower remember-table spelling, MA09 §1/§4) fails to *parse* at all in
/// this subset, so this function never needs to distinguish "was the LHS a
/// call" the way Reduce's/Derive's own `lower_assignment` does.
///
/// By the time `lower_node` dispatches here (via `unwrap_single`, which only
/// stops descending once a node's own child count isn't exactly 1), a
/// genuine `assignment` node always has the 3-child `[NAME, ASSIGN, (arrow_def
/// | expr)]` shape — the bare-`expr` alternative dissolves away before ever
/// reaching this function. The `lower_first_node` fallback below is
/// defensive only, mirroring `reduce-runtime::lower_assignment`'s identical
/// defensive shape.
fn lower_assignment(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let is_assign_form = node.children.len() == 3
        && as_token(&node.children[1]).is_some_and(|t| token_type(t) == "ASSIGN");
    if !is_assign_form {
        return lower_first_node(node);
    }
    let name = match as_token(&node.children[0]) {
        Some(t) if token_type(t) == "NAME" => t.value.clone(),
        _ => return Err(LowerError::new("assignment lhs must be a bare NAME")),
    };
    match &node.children[2] {
        ASTNodeOrToken::Node(n) if n.rule_name == "arrow_def" => lower_arrow_def(name, n),
        rhs => {
            let value = lower_child(rhs)?;
            Ok(apply(sym(ASSIGN), vec![sym(name), value]))
        }
    }
}

/// `arrow_def = arrow_params ARROW expr` — MA09 §3's general-purpose
/// function-definition spelling, `f := (x, y) -> e` / `f := x -> e`. Lowers
/// to `Define[f, List[params...], body]`, mirroring
/// `derive-runtime`'s/`reduce-runtime`'s identical `Define` shape for their
/// own (differently-spelled) general-definition idioms.
fn lower_arrow_def(name: String, node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    if node.children.len() != 3 {
        return Err(LowerError::new("malformed arrow_def node"));
    }
    let params_node = as_node(&node.children[0])
        .ok_or_else(|| LowerError::new("arrow_def is missing its parameter list"))?;
    let params = lower_arrow_params(params_node);
    let body = lower_child(&node.children[2])?;
    Ok(apply(
        sym(DEFINE),
        vec![sym(name), apply(sym(LIST), params), body],
    ))
}

/// `arrow_params = NAME | LPAREN [ NAME { COMMA NAME } ] RPAREN` — a single
/// bare parameter needs no parentheses, two-or-more do, and `()` (zero
/// parameters) falls out of the optional inner list for free (MA09 §3's own
/// note on `arrow_params`). Both shapes are handled uniformly by simply
/// collecting every `NAME` token among the node's children in order — the
/// `LPAREN`/`COMMA`/`RPAREN` tokens present in the parenthesised form are
/// harmlessly filtered out, so there is no need to branch on which
/// alternative matched.
fn lower_arrow_params(node: &GrammarASTNode) -> Vec<IRNode> {
    node.children
        .iter()
        .filter_map(as_token)
        .filter(|t| token_type(t) == "NAME")
        .map(|t| sym(t.value.clone()))
        .collect()
}

/// `logical_or`/`logical_and` — fold the operands into an n-ary `Or`/`And`.
/// Safe to fold n-ary (unlike `additive`/`multiplicative`) because every
/// step in one chain shares the SAME operator (`logical_or = logical_and {
/// "or" logical_and }` never mixes `or` and `and`) — mirrors
/// `reduce-runtime::lower_logical_chain` exactly.
fn lower_logical_chain(node: &GrammarASTNode, head: &str) -> Result<IRNode, LowerError> {
    let operands = child_nodes(node)
        .map(lower_node)
        .collect::<Result<Vec<_>, _>>()?;
    match operands.len() {
        0 => Err(LowerError::new("empty logical chain")),
        1 => Ok(operands.into_iter().next().unwrap()),
        _ => Ok(apply(sym(head), operands)),
    }
}

/// `logical_not = "not" logical_not | comparison`. A leading `not` wraps the
/// operand; otherwise it is the inner comparison. `and`/`or`/`not` are all
/// matched in the grammar as `maple.tokens`' own `KEYWORD` token type
/// (promoted from `NAME` by exact lowercase spelling), so — mirroring
/// `reduce-runtime::lower_logical_not`'s identical check — this checks the
/// token's literal *value*, not its type name alone (every keyword shares
/// that same type name).
fn lower_logical_not(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let has_not = node
        .children
        .iter()
        .any(|c| as_token(c).is_some_and(|t| t.value == "not"));
    if !has_not {
        return lower_first_node(node);
    }
    let inner = child_nodes(node)
        .next()
        .ok_or_else(|| LowerError::new("`not` with no operand"))?;
    Ok(apply(sym(NOT), vec![lower_node(inner)?]))
}

/// `comparison = additive [ ( EQ | NEQ | LESS | GREATER | LE | GE ) additive
/// ]` — a single (non-chained) comparison, per MA09 §3's own disclosed
/// simplification (the identical "one flat, non-chaining tier"
/// simplification `reduce.grammar`'s own `comparison` rule already made,
/// reused here rather than re-derived — see `maple.grammar`'s own "the
/// comparison tier is flat and non-chaining" design-decision comment). `=`
/// is Maple's *equation* operator (`Equal`), never assignment — `:=` alone
/// owns that role. Unlike Reduce's `neq` word-keyword, Maple's not-equal is
/// the symbolic `NEQ` (`<>`) token type, so no value-based disambiguation is
/// needed here.
fn lower_comparison(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let Some(op_index) = node
        .children
        .iter()
        .position(|c| as_token(c).is_some_and(|t| comparison_head(t).is_some()))
    else {
        return lower_first_node(node);
    };
    if op_index == 0 || op_index + 1 >= node.children.len() {
        return Err(LowerError::new("malformed comparison node"));
    }
    let head = comparison_head(as_token(&node.children[op_index]).unwrap()).unwrap();
    Ok(apply(
        sym(head),
        vec![
            lower_child(&node.children[op_index - 1])?,
            lower_child(&node.children[op_index + 1])?,
        ],
    ))
}

/// `additive`/`multiplicative` — a left-associative chain of `+`/`-` or
/// `*`/`/`. Must fold pairwise (not n-ary, unlike the logical chains) since a
/// single chain can mix operators: `a - b - c` folds left into
/// `Sub(Sub(a, b), c)`; `a + b - c` into `Sub(Add(a, b), c)`. Mirrors
/// `reduce-runtime::lower_binary_chain` exactly (identical grammar shape).
fn lower_binary_chain(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let mut children = node.children.iter();
    let first = children
        .next()
        .ok_or_else(|| LowerError::new("empty binary chain"))?;
    let mut result = lower_child(first)?;
    while let Some(op_child) = children.next() {
        let head = as_token(op_child)
            .and_then(|t| binary_head(token_type(t)))
            .ok_or_else(|| LowerError::new("expected a binary operator"))?;
        let rhs = children
            .next()
            .ok_or_else(|| LowerError::new("binary operator with no right operand"))?;
        result = apply(sym(head), vec![result, lower_child(rhs)?]);
    }
    Ok(result)
}

/// `unary = MINUS unary | power`. MA09 §3 lists only unary `-` (no unary
/// `+`, matching `reduce.grammar`'s/`derive.grammar`'s identical asymmetry).
fn lower_unary(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    if node.children.len() == 1 {
        return lower_child(&node.children[0]);
    }
    let operand = lower_child(
        node.children
            .get(1)
            .ok_or_else(|| LowerError::new("unary `-` with no operand"))?,
    )?;
    Ok(apply(sym(NEG), vec![operand]))
}

/// `power = postfix [ CARET unary ]` — right-associative `^`. Unlike
/// `reduce-runtime::lower_power` (which accepts either `CARET` or `POW`,
/// since Reduce's manual documents both as one tier), Maple's own grammar
/// has no `**` synonym at all (MA09 §3/§4; `maple.tokens` has no `POW`
/// token), so this only ever matches `CARET`.
fn lower_power(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    match node.children.len() {
        1 => lower_child(&node.children[0]),
        3 => {
            let is_caret =
                as_token(&node.children[1]).is_some_and(|t| token_type(t) == "CARET");
            if !is_caret {
                return Err(LowerError::new("malformed power node: expected CARET"));
            }
            Ok(apply(
                sym(POW),
                vec![
                    lower_child(&node.children[0])?,
                    lower_child(&node.children[2])?,
                ],
            ))
        }
        _ => Err(LowerError::new("malformed power node")),
    }
}

/// `postfix = atom [ LPAREN [ arglist ] RPAREN ]` — a single OPTIONAL call
/// suffix, deliberately narrower than `reduce-runtime::lower_postfix`'s
/// REPEATED `{ LPAREN [arglist] RPAREN }` chain (`maple.grammar`'s own "one
/// call suffix, not a repeated chain" design-decision comment: MA09 §3
/// documents no chained-call/array-subscript convention for Maple the way
/// Reduce's manual does). One consequence worth noting for
/// [`crate::MAX_STATEMENT_TOKENS`]'s own doc comment: Maple therefore has
/// *no* "long chained postfix call `f(x)(x)(x)…`" deep-lowered-tree vector
/// at all — that vector is structurally impossible in this grammar, not
/// merely unlikely.
///
/// The head runs through [`canonical_head`] so `diff`/`int` become the IR
/// heads `symbolic-vm` already has handlers for; any other head (a
/// user-defined function, or a builtin this subset doesn't bridge, like
/// `sin`/`solve`/`piecewise`) passes through unchanged and evaluates via the
/// VM's ordinary "unknown head" fallback — the deferred `cas-*` surface
/// (MA09 §4) simply parses and lowers as an ordinary, currently-unresolved
/// call, exactly like any undefined user function.
fn lower_postfix(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let base = lower_child(
        node.children
            .first()
            .ok_or_else(|| LowerError::new("postfix has no base"))?,
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
        .map(lower_arglist)
        .transpose()?
        .unwrap_or_default();
    Ok(apply(canonical_head(base), args))
}

/// `arglist = expr { COMMA expr }` — lower each comma-separated argument.
fn lower_arglist(node: &GrammarASTNode) -> Result<Vec<IRNode>, LowerError> {
    child_nodes(node).map(lower_node).collect()
}

/// `atom = NUMBER | NAME | "true" | "false" | list_literal | set_literal |
/// group`. In practice `unwrap_single` already dissolves a single-child
/// `atom` node before `lower_node`'s dispatch ever sees rule_name `"atom"`
/// (every alternative here matches to exactly one child), so this function
/// mirrors `reduce-runtime::lower_atom`'s identical defensive shape rather
/// than being load-bearing for the common case.
fn lower_atom(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    if let Some(child) = child_nodes(node).next() {
        if matches!(child.rule_name.as_str(), "list_literal" | "set_literal" | "group") {
            return lower_node(child);
        }
    }
    let tokens: Vec<&Token> = node.children.iter().filter_map(as_token).collect();
    match tokens.as_slice() {
        [single] => lower_token(single),
        _ => Err(LowerError::new(format!(
            "unrecognised atom token shape: {:?}",
            tokens.iter().map(|t| &t.value).collect::<Vec<_>>()
        ))),
    }
}

/// `list_literal = LBRACKET [ arglist ] RBRACKET` — MA09 §3's `[a, b, c]`
/// (square brackets, ordered, duplicates kept). Lowers to `List[...]`, the
/// shared, already-handled head every CAS-family sibling in this repo
/// reuses.
fn lower_list_literal(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let args = child_nodes(node)
        .find(|n| n.rule_name == "arglist")
        .map(lower_arglist)
        .transpose()?
        .unwrap_or_default();
    Ok(apply(sym(LIST), args))
}

/// `set_literal = LBRACE [ arglist ] RBRACE` — MA09 §3's `{a, b, c}` (curly
/// braces, unordered, duplicates removed *in real Maple*). Lowers to the
/// new [`SET`] head — see this module's own doc comment's "`Set`" section
/// for the disclosed evaluation-time gap (arguments evaluate; the
/// dedup/unordered semantics are not yet enforced, since no shared handler
/// exists for this head).
fn lower_set_literal(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let args = child_nodes(node)
        .find(|n| n.rule_name == "arglist")
        .map(lower_arglist)
        .transpose()?
        .unwrap_or_default();
    Ok(apply(sym(SET), args))
}

/// `group = LPAREN expr RPAREN` — grouping only; lower the inner expression.
fn lower_group(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let inner = child_nodes(node)
        .next()
        .ok_or_else(|| LowerError::new("empty group `( )`"))?;
    lower_node(inner)
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn lower_child(child: &ASTNodeOrToken) -> Result<IRNode, LowerError> {
    match child {
        ASTNodeOrToken::Node(node) => lower_node(node),
        ASTNodeOrToken::Token(token) => lower_token(token),
    }
}

/// Lower the first *node* child (ignoring tokens). Used by transparent-
/// wrapper rules whose only meaningful content is a nested expression node.
fn lower_first_node(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let child = child_nodes(node)
        .next()
        .ok_or_else(|| LowerError::new(format!("`{}` has no expression child", node.rule_name)))?;
    lower_node(child)
}

/// Map an arithmetic token type to its IR head.
fn binary_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "PLUS" => Some(ADD),
        "MINUS" => Some(SUB),
        "TIMES" => Some(MUL),
        "SLASH" => Some(DIV),
        _ => None,
    }
}

/// Map a comparison token to its IR head. Every comparison operator in this
/// subset is a symbolic (punctuation) token type — unlike Reduce's `neq`
/// keyword, Maple spells not-equal `<>` (`NEQ`), so no value-based check is
/// needed alongside the type-based ones.
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

/// Bridge a Maple *surface* builtin call-head name to the canonical IR head
/// it's already implemented under. Per MA09 §2's own MP-4 bullet, this
/// subset's *only* such bridge is calculus (`diff`→`D`, `int`→`Integrate`) —
/// unlike `reduce-runtime`'s equivalent table (which also bridges
/// `list`/`first`/`second`/…), MA09 §3 documents no function-call spelling
/// for list/set construction (Maple already has literal `[...]`/`{...}`
/// syntax for both), and this subset deliberately does not bridge
/// elementary-function names (`sin`, `log`, …) the way it does not for
/// Reduce either — a head not in this table (a user-defined function, or
/// any of MA09 §4's deferred `cas-*` surface) is returned unchanged, so it
/// evaluates through the VM's ordinary user-function path and stays a
/// harmless unevaluated symbolic call.
fn canonical_head(head: IRNode) -> IRNode {
    if let IRNode::Symbol(name) = &head {
        if let Some(canonical) = surface_head_to_ir(name) {
            return sym(canonical);
        }
    }
    head
}

fn surface_head_to_ir(name: &str) -> Option<&'static str> {
    Some(match name {
        "diff" => D,
        "int" => INTEGRATE,
        _ => return None,
    })
}

fn token_type(token: &Token) -> &str {
    token.effective_type_name()
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

/// Iterate a node's direct *node* children (skipping tokens).
fn child_nodes(node: &GrammarASTNode) -> impl Iterator<Item = &GrammarASTNode> {
    node.children.iter().filter_map(as_node)
}

enum Unwrapped<'a> {
    Node(&'a GrammarASTNode),
    Token(&'a Token),
}

/// Peel away single-child wrapper nodes until we reach a node with structure
/// (or a leaf token). A precedence-cascade rule that did not apply its
/// operator still emits its own node with exactly one child, and so does an
/// ordered-choice rule like `expr = logical_or` once it has committed to its
/// one alternative — `unwrap_single` skips straight to the rule that
/// actually matters. Mirrors `reduce-runtime::unwrap_single` exactly (the
/// shared `parser::GrammarParser` engine's node shape is identical across
/// every grammar built on it).
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

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_maple_parser::parse_maple;

    /// Lower a single-statement source to its one `(IRNode, Display)` pair.
    fn lower_one(src: &str) -> LoweredStatement {
        let ast = parse_maple(src);
        let mut stmts = lower_program(&ast).expect("lowering failed");
        assert_eq!(stmts.len(), 1, "expected exactly one statement for {src:?}");
        stmts.pop().unwrap()
    }

    fn lower_one_node(src: &str) -> IRNode {
        lower_one(src).node
    }

    #[test]
    fn integer_and_real_literals() {
        assert_eq!(lower_one_node("42;\n"), int(42));
        assert_eq!(lower_one_node("1.5;\n"), flt(1.5));
    }

    #[test]
    fn bare_symbol() {
        assert_eq!(lower_one_node("foo;\n"), sym("foo"));
    }

    #[test]
    fn bare_trailing_statement_with_no_terminator_lowers() {
        let stmt = lower_one("42");
        assert_eq!(stmt.node, int(42));
        assert_eq!(stmt.display, Display::Show);
    }

    // --- Display flag: `;` shows, `:` suppresses (MA09 §3) ------------------

    #[test]
    fn semicolon_terminator_displays() {
        assert_eq!(lower_one("1 + 1;\n").display, Display::Show);
    }

    #[test]
    fn colon_terminator_suppresses() {
        assert_eq!(lower_one("1 + 1:\n").display, Display::Suppress);
    }

    #[test]
    fn mixed_terminators_in_one_program_are_tagged_independently() {
        let ast = parse_maple("1: 2; 3:\n");
        let stmts = lower_program(&ast).expect("lowering failed");
        assert_eq!(stmts.len(), 3);
        assert_eq!(stmts[0].display, Display::Suppress);
        assert_eq!(stmts[1].display, Display::Show);
        assert_eq!(stmts[2].display, Display::Suppress);
    }

    // --- Booleans (MA09 §3) --------------------------------------------------

    #[test]
    fn boolean_literals_bridge_to_the_shared_true_false_symbols() {
        assert_eq!(lower_one_node("true;\n"), sym("True"));
        assert_eq!(lower_one_node("false;\n"), sym("False"));
    }

    // --- Arithmetic (MA09 §3) ------------------------------------------------

    #[test]
    fn additive_lowers_to_add() {
        assert_eq!(
            lower_one_node("1 + 2;\n"),
            apply(sym(ADD), vec![int(1), int(2)])
        );
    }

    #[test]
    fn subtraction_lowers_to_sub_left_assoc() {
        assert_eq!(
            lower_one_node("a - b - c;\n"),
            apply(
                sym(SUB),
                vec![apply(sym(SUB), vec![sym("a"), sym("b")]), sym("c")]
            )
        );
    }

    #[test]
    fn mixed_additive_chain_folds_left_by_operator() {
        assert_eq!(
            lower_one_node("a + b - c;\n"),
            apply(
                sym(SUB),
                vec![apply(sym(ADD), vec![sym("a"), sym("b")]), sym("c")]
            )
        );
    }

    #[test]
    fn multiplication_requires_explicit_star_and_lowers_to_mul() {
        assert_eq!(
            lower_one_node("a * b;\n"),
            apply(sym(MUL), vec![sym("a"), sym("b")])
        );
    }

    #[test]
    fn division_lowers_to_div() {
        assert_eq!(
            lower_one_node("a / b;\n"),
            apply(sym(DIV), vec![sym("a"), sym("b")])
        );
    }

    #[test]
    fn power_operator_lowers_to_pow() {
        assert_eq!(
            lower_one_node("a ^ b;\n"),
            apply(sym(POW), vec![sym("a"), sym("b")])
        );
    }

    #[test]
    fn power_is_right_associative() {
        assert_eq!(
            lower_one_node("a ^ b ^ c;\n"),
            apply(
                sym(POW),
                vec![sym("a"), apply(sym(POW), vec![sym("b"), sym("c")])]
            )
        );
    }

    #[test]
    fn unary_minus_lowers_to_neg_and_binds_looser_than_power() {
        assert_eq!(
            lower_one_node("-x^2;\n"),
            apply(sym(NEG), vec![apply(sym(POW), vec![sym("x"), int(2)])])
        );
    }

    // --- Comparison / equation (MA09 §3) -------------------------------------

    #[test]
    fn eq_lowers_to_equal_not_assign() {
        assert_eq!(
            lower_one_node("x = 4;\n"),
            apply(sym(EQUAL), vec![sym("x"), int(4)])
        );
    }

    #[test]
    fn every_comparison_operator_lowers_to_its_head() {
        assert_eq!(
            lower_one_node("a < b;\n"),
            apply(sym(LESS), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one_node("a > b;\n"),
            apply(sym(GREATER), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one_node("a <= b;\n"),
            apply(sym(LESS_EQUAL), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one_node("a >= b;\n"),
            apply(sym(GREATER_EQUAL), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one_node("a <> b;\n"),
            apply(sym(NOT_EQUAL), vec![sym("a"), sym("b")])
        );
    }

    // --- Logic (MA09 §3) ------------------------------------------------------

    #[test]
    fn boolean_keywords_lower_to_and_or_not() {
        assert_eq!(
            lower_one_node("a and b;\n"),
            apply(sym(AND), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one_node("a or b;\n"),
            apply(sym(OR), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one_node("not a;\n"),
            apply(sym(NOT), vec![sym("a")])
        );
    }

    #[test]
    fn logical_or_chain_folds_n_ary() {
        assert_eq!(
            lower_one_node("a or b or c;\n"),
            apply(sym(OR), vec![sym("a"), sym("b"), sym("c")])
        );
    }

    // --- Grouping --------------------------------------------------------------

    #[test]
    fn grouping_parens_lower_transparently() {
        assert_eq!(
            lower_one_node("(1 + 2) * 3;\n"),
            apply(
                sym(MUL),
                vec![apply(sym(ADD), vec![int(1), int(2)]), int(3)]
            )
        );
    }

    // --- Function application (MA09 §3) -----------------------------------

    #[test]
    fn function_application_of_unknown_head_passes_through() {
        assert_eq!(
            lower_one_node("f(a, b);\n"),
            apply(sym("f"), vec![sym("a"), sym("b")])
        );
    }

    #[test]
    fn nested_function_calls_lower_correctly() {
        assert_eq!(
            lower_one_node("log(exp(x));\n"),
            apply(sym("log"), vec![apply(sym("exp"), vec![sym("x")])])
        );
    }

    #[test]
    fn call_with_no_arguments_lowers_to_an_empty_arg_apply() {
        assert_eq!(lower_one_node("f();\n"), apply(sym("f"), vec![]));
    }

    // --- diff/int bridge to D/Integrate (MA09 §2/§5) ------------------------

    #[test]
    fn diff_bridges_to_d() {
        assert_eq!(
            lower_one_node("diff(x^2, x);\n"),
            apply(sym(D), vec![apply(sym(POW), vec![sym("x"), int(2)]), sym("x")])
        );
    }

    #[test]
    fn int_bridges_to_integrate() {
        assert_eq!(
            lower_one_node("int(x, x);\n"),
            apply(sym(INTEGRATE), vec![sym("x"), sym("x")])
        );
    }

    #[test]
    fn elementary_function_names_are_not_bridged() {
        // Unlike diff/int -- MA09 §2/§5 names only the calculus bridge, so
        // `sin` stays lowercase and unresolved (mirroring reduce-runtime's
        // identical non-bridging of elementary function names).
        assert_eq!(lower_one_node("sin(x);\n"), apply(sym("sin"), vec![sym("x")]));
    }

    // --- Assignment / arrow-operator definition (MA09 §3) --------------------

    #[test]
    fn variable_assignment_lowers_to_assign() {
        assert_eq!(
            lower_one_node("x := 5;\n"),
            apply(sym(ASSIGN), vec![sym("x"), int(5)])
        );
    }

    #[test]
    fn arrow_definition_with_two_params_lowers_to_define() {
        assert_eq!(
            lower_one_node("f := (x, y) -> x + y;\n"),
            apply(
                sym(DEFINE),
                vec![
                    sym("f"),
                    apply(sym(LIST), vec![sym("x"), sym("y")]),
                    apply(sym(ADD), vec![sym("x"), sym("y")]),
                ]
            )
        );
    }

    #[test]
    fn arrow_definition_with_one_bare_param_lowers_to_define() {
        assert_eq!(
            lower_one_node("f := x -> x^2;\n"),
            apply(
                sym(DEFINE),
                vec![
                    sym("f"),
                    apply(sym(LIST), vec![sym("x")]),
                    apply(sym(POW), vec![sym("x"), int(2)]),
                ]
            )
        );
    }

    #[test]
    fn arrow_definition_with_zero_params_lowers_to_define_with_empty_list() {
        assert_eq!(
            lower_one_node("f := () -> 5;\n"),
            apply(
                sym(DEFINE),
                vec![sym("f"), apply(sym(LIST), vec![]), int(5)]
            )
        );
    }

    #[test]
    fn plain_assignment_of_a_variable_does_not_produce_define() {
        assert_eq!(
            lower_one_node("f := x;\n"),
            apply(sym(ASSIGN), vec![sym("f"), sym("x")])
        );
    }

    // --- `if`/`elif`/`else`/`end if` (MA09 §3) -------------------------------

    #[test]
    fn if_then_end_if_lowers_to_two_arg_if() {
        assert_eq!(
            lower_one_node("if a > 0 then 1 end if;\n"),
            apply(
                sym(IF),
                vec![apply(sym(GREATER), vec![sym("a"), int(0)]), int(1)]
            )
        );
    }

    #[test]
    fn if_then_else_end_if_lowers_to_three_arg_if() {
        assert_eq!(
            lower_one_node("if a > 0 then 1 else -1 end if;\n"),
            apply(
                sym(IF),
                vec![
                    apply(sym(GREATER), vec![sym("a"), int(0)]),
                    int(1),
                    apply(sym(NEG), vec![int(1)]),
                ]
            )
        );
    }

    #[test]
    fn fi_closing_spelling_lowers_identically_to_end_if() {
        let end_if = lower_one_node("if a > 0 then 1 else -1 end if;\n");
        let fi = lower_one_node("if a > 0 then 1 else -1 fi;\n");
        assert_eq!(end_if, fi);
    }

    #[test]
    fn elif_chain_desugars_to_nested_if() {
        // if a then 1 elif b then 2 else 3 end if
        //   -> If(a, 1, If(b, 2, 3))
        assert_eq!(
            lower_one_node("if a then 1 elif b then 2 else 3 end if;\n"),
            apply(
                sym(IF),
                vec![
                    sym("a"),
                    int(1),
                    apply(sym(IF), vec![sym("b"), int(2), int(3)]),
                ]
            )
        );
    }

    #[test]
    fn elif_chain_with_no_final_else_leaves_the_innermost_if_two_armed() {
        // if a then 1 elif b then 2 end if -> If(a, 1, If(b, 2))
        assert_eq!(
            lower_one_node("if a then 1 elif b then 2 end if;\n"),
            apply(
                sym(IF),
                vec![sym("a"), int(1), apply(sym(IF), vec![sym("b"), int(2)])]
            )
        );
    }

    #[test]
    fn multiple_elif_arms_fold_right_to_left() {
        assert_eq!(
            lower_one_node("if a then 1 elif b then 2 elif c then 3 else 4 end if;\n"),
            apply(
                sym(IF),
                vec![
                    sym("a"),
                    int(1),
                    apply(
                        sym(IF),
                        vec![
                            sym("b"),
                            int(2),
                            apply(sym(IF), vec![sym("c"), int(3), int(4)]),
                        ]
                    ),
                ]
            )
        );
    }

    #[test]
    fn if_can_branch_to_an_assignment() {
        assert_eq!(
            lower_one_node("if a then x := 1 else x := 2 end if;\n"),
            apply(
                sym(IF),
                vec![
                    sym("a"),
                    apply(sym(ASSIGN), vec![sym("x"), int(1)]),
                    apply(sym(ASSIGN), vec![sym("x"), int(2)]),
                ]
            )
        );
    }

    #[test]
    fn nested_if_resolves_unambiguously() {
        // if a then if b then c end if else d end if
        //   -> If(a, If(b, c), d)
        assert_eq!(
            lower_one_node("if a then if b then c end if else d end if;\n"),
            apply(
                sym(IF),
                vec![sym("a"), apply(sym(IF), vec![sym("b"), sym("c")]), sym("d")]
            )
        );
    }

    // --- Lists / sets (MA09 §3/§5) --------------------------------------------

    #[test]
    fn square_bracket_list_literal_lowers_to_list() {
        assert_eq!(
            lower_one_node("[a, b, c];\n"),
            apply(sym(LIST), vec![sym("a"), sym("b"), sym("c")])
        );
    }

    #[test]
    fn empty_list_literal_lowers_to_empty_list() {
        assert_eq!(lower_one_node("[];\n"), apply(sym(LIST), vec![]));
    }

    #[test]
    fn curly_brace_set_literal_lowers_to_the_new_set_head() {
        assert_eq!(
            lower_one_node("{a, b, c};\n"),
            apply(sym(SET), vec![sym("a"), sym("b"), sym("c")])
        );
    }

    #[test]
    fn empty_set_literal_lowers_to_empty_set() {
        assert_eq!(lower_one_node("{};\n"), apply(sym(SET), vec![]));
    }

    #[test]
    fn list_and_set_are_genuinely_distinct_heads_for_the_same_elements() {
        let list = lower_one_node("[a, b];\n");
        let set = lower_one_node("{a, b};\n");
        assert_ne!(list, set);
        assert_eq!(list, apply(sym(LIST), vec![sym("a"), sym("b")]));
        assert_eq!(set, apply(sym(SET), vec![sym("a"), sym("b")]));
    }

    // --- Multi-statement programs ---------------------------------------------

    #[test]
    fn multi_statement_program_lowers_each_line() {
        let ast = parse_maple("x := 1; y := 2; x + y;\n");
        let stmts = lower_program(&ast).expect("lowering failed");
        assert_eq!(stmts.len(), 3);
        assert_eq!(stmts[0].node, apply(sym(ASSIGN), vec![sym("x"), int(1)]));
        assert_eq!(stmts[1].node, apply(sym(ASSIGN), vec![sym("y"), int(2)]));
        assert_eq!(stmts[2].node, apply(sym(ADD), vec![sym("x"), sym("y")]));
    }

    #[test]
    fn a_small_maple_program_lowers_end_to_end() {
        let ast = parse_maple("f := x -> x*x; f(5);\n");
        let stmts = lower_program(&ast).expect("lowering failed");
        assert_eq!(stmts.len(), 2);
        assert!(matches!(
            &stmts[0].node,
            IRNode::Apply(a) if matches!(&a.head, IRNode::Symbol(s) if s == DEFINE)
        ));
        assert_eq!(stmts[1].node, apply(sym("f"), vec![int(5)]));
    }
}
