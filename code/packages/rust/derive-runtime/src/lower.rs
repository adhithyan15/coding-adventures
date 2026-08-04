//! # Lowering — Derive syntax tree → `symbolic-ir`
//!
//! The D-3 [`derive-parser`](coding_adventures_derive_parser) hands us a
//! *generic* [`GrammarASTNode`] tree whose `rule_name`s mirror the grammar
//! (`assignment`, `additive`, `multiplicative`, `power`, `postfix`, `atom`,
//! …). The parser deliberately does **no** semantic work — it just records
//! which rule matched. This module is where Derive's *meaning* is assigned:
//! every surface construct is **desugared** into the canonical
//! [`symbolic_ir::IRNode`] head [`symbolic-vm`](symbolic_vm) already knows
//! how to evaluate.
//!
//! ## Much thinner than Wolfram's/Macsyma's lowering
//!
//! Derive (MA07 §1) has no `f[x]`-universal-application syntax and no
//! pattern/rewrite-rule vocabulary (`_`, `->`, `/.`) — every "transform this
//! expression" operation (`DIF`, `INT`, `IF`, …) is an ordinary named
//! function call with ordinary parentheses, ambiguity-free with the infix
//! operators. So this module needs none of the pattern-lowering,
//! `ReplaceAll`/`ReplaceRepeated` interception, or ordinary-vs-pure-function
//! machinery `wolfram-runtime::lower` carries — just arithmetic, comparison,
//! logic, assignment/definition, and function application.
//!
//! ## The head-name bridge — and why Derive needs a BIGGER one than Wolfram
//!
//! Wolfram already spells its canonical heads in the IR's own casing
//! (`Sin`, `Plus`, …), so its bridge only covers the handful of operators
//! with a *different* long-form name (`Plus` → `Add`, `Set` → `Assign`, …).
//! Derive's built-ins are conventionally **UPPERCASE** (`SIN`, `DIF`, `INT`,
//! `IF` — MA07 §3), and `IRNode::Symbol` equality is case-sensitive, so
//! *every* elementary/hyperbolic function and every renamed calculus/control
//! head needs an explicit surface→IR entry here — not just the handful that
//! differ semantically. An unrecognised head (a user-defined function name,
//! or a builtin not spelled in the exact uppercase convention) passes
//! through unchanged, exactly like Wolfram's unknown-head fallthrough.
//!
//! ## `:=` disambiguation has no operator to branch on
//!
//! Wolfram (`=`/`:=`) and Macsyma (`:`/`::=`) each have TWO distinct
//! assignment operators, so their lowering picks `Assign` vs `Define` by
//! *which token fired*. Derive's grammar has exactly ONE token, `ASSIGN`
//! (`:=`), reaching the `assignment` rule (derive-lexer/derive-parser's own
//! READMEs) — `x := 5` and `F(x) := x^2 + 1` are syntactically identical
//! until this lowering step. So [`lower_assignment`] disambiguates purely by
//! the *lowered LHS's shape*: `Apply(Symbol(_), _)` → `Define`, anything
//! else → `Assign`. Derive also has no pattern syntax, so (unlike Wolfram's
//! `param_binding_symbol`) a function's parameters need no unwrapping — a
//! bare `NAME` in `F(x, y) := …`'s argument position already lowers straight
//! to a plain `Symbol`, the exact shape `define_handler` binds against.
//!
//! ## Vectors/matrices as structural `List` data (D-5)
//!
//! `derive-parser` parses `[a, b, c]` / `[a, b; c, d]` as a single `vector`
//! rule — `vector = LBRACKET row { SEMI row } RBRACKET`, `row = expr { COMMA
//! expr }` — with no separate grammar rule distinguishing "vector" from
//! "matrix" shape. [`lower_vector`] draws that distinction purely by
//! *counting* how many `row` children were parsed (per the grammar file's
//! own comment on `vector`): exactly one `row` lowers to a flat
//! `List(elems…)` (a vector); more than one lowers to a `List` of per-row
//! `List`s (a matrix), mirroring Wolfram's `{a, b}` → `List[a, b]`. Per MA07
//! §2/§4, this is *structural* only — no linear-algebra evaluation (matrix
//! multiply, determinant, …) is wired here; that is separate, later work.

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use symbolic_ir::{
    apply, flt, int, sym, IRNode, ACOS, ACOSH, ADD, AND, ASIN, ASINH, ATAN, ATANH, COS, COSH, COTH,
    CSCH, D, DEFINE, DIV, EQUAL, EXP, GREATER, GREATER_EQUAL, IF, INTEGRATE, LESS, LESS_EQUAL,
    LIST, LOG, MUL, NEG, NOT, OR, POW, SECH, SIN, SINH, SQRT, SUB, TAN, TANH,
};

/// A failure while lowering the surface tree to IR. These are *structural*
/// errors — a node shape the lowering did not expect, or a construct
/// deliberately deferred to a later item — not user syntax errors (those are
/// caught earlier by the parser).
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

/// Lower a parsed `program` node into one IR statement per source statement.
///
/// The D-3 grammar's root is `program = { statement_line }`, where each
/// `statement_line` wraps a `statement` plus a `NEWLINE` terminator, or is
/// terminator-only (a blank line). We keep only the lines that actually
/// carry a `statement` and lower each one; blank lines contribute nothing.
pub fn lower_program(root: &GrammarASTNode) -> Result<Vec<IRNode>, LowerError> {
    if root.rule_name != "program" {
        return Err(LowerError::new(format!(
            "expected `program` root, got `{}`",
            root.rule_name
        )));
    }
    let mut statements = Vec::new();
    for line in child_nodes(root) {
        if line.rule_name != "statement_line" {
            continue;
        }
        if let Some(statement) = child_nodes(line).find(|n| n.rule_name == "statement") {
            statements.push(lower_node(statement)?);
        }
    }
    Ok(statements)
}

/// Lower a single arbitrary node.
///
/// Most grammar rules are "transparent wrappers" — a `statement` is just an
/// `expr`, an `expr` is just an `assignment`, and when a precedence level did
/// not actually apply its operator the parser still emits the level's node
/// with a single child. [`unwrap_single`] peels those away so we dispatch on
/// the first rule that genuinely shapes the tree.
fn lower_node(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    match unwrap_single(node) {
        Unwrapped::Token(token) => lower_token(token),
        Unwrapped::Node(node) => match node.rule_name.as_str() {
            "program" => Err(LowerError::new("nested program node is not an expression")),
            "statement_line" | "statement" | "expr" => lower_first_node(node),
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
            "vector" => lower_vector(node),
            "row" => Err(LowerError::new(
                "a `row` node must be lowered via `lower_vector`'s row-counting logic, \
                 not `lower_node` directly",
            )),
            "group" => lower_group(node),
            "arglist" => Err(LowerError::new(
                "an arglist cannot be lowered as a scalar expression",
            )),
            other => Err(LowerError::new(format!("no lowering for rule `{other}`"))),
        },
    }
}

/// Lower a raw token (a literal or a bare symbol).
fn lower_token(token: &Token) -> Result<IRNode, LowerError> {
    match token_type(token) {
        "NUMBER" => lower_number(&token.value),
        "NAME" => Ok(sym(&token.value)),
        other => Err(LowerError::new(format!(
            "unexpected token `{other}` = {:?}",
            token.value
        ))),
    }
}

/// Parse a `NUMBER` lexeme into an `Integer` or `Float` IR literal.
///
/// The D-2 lexer's `NUMBER` regex is `[0-9]+\.?[0-9]*([eE][+-]?[0-9]+)?`, so
/// a `.`, `e`, or `E` means it is a real; otherwise it is an integer. We
/// reject an integer that overflows `i64` (the IR's integer width) rather
/// than silently wrapping.
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

/// `assignment = logical_or [ ASSIGN assignment ]` — right-associative.
///
/// See the module doc comment's "`:=` disambiguation" section: there is only
/// one operator token, so the LHS's own *lowered shape* decides `Assign` vs
/// `Define`. `F(x, y) := body` (LHS lowers to `Apply(Symbol(F), [x, y])`)
/// becomes `Define(F, List(x, y), body)`; a bare `x := body` (LHS is a plain
/// symbol) becomes a zero-parameter `Define` — no, becomes `Assign(x,
/// body)`, an ordinary variable binding.
fn lower_assignment(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let Some(op_index) = node
        .children
        .iter()
        .position(|c| as_token(c).is_some_and(|t| token_type(t) == "ASSIGN"))
    else {
        return lower_first_node(node);
    };
    if op_index == 0 || op_index + 1 >= node.children.len() {
        return Err(LowerError::new("malformed assignment node"));
    }
    let lhs = lower_child(&node.children[op_index - 1])?;
    let rhs = lower_child(&node.children[op_index + 1])?;

    if let IRNode::Apply(app) = &lhs {
        if matches!(&app.head, IRNode::Symbol(_)) {
            // F(x, y) := body — a function definition. Derive has no pattern
            // syntax, so each parameter already lowered to a plain `Symbol`
            // (or, for a malformed definition like `F(1) := …`, whatever the
            // caller wrote — `define_handler` and the VM's parameter binder
            // own validating that, not this lowering, mirroring how
            // Wolfram's own lowering does not validate parameter shapes
            // either).
            return Ok(apply(
                sym(DEFINE),
                vec![app.head.clone(), apply(sym(LIST), app.args.clone()), rhs],
            ));
        }
    }
    // x := e — variable assignment.
    Ok(apply(sym(symbolic_ir::ASSIGN), vec![lhs, rhs]))
}

/// `logical_or`/`logical_and` — fold the operands into an n-ary `Or`/`And`.
/// Safe to fold n-ary (unlike `additive`/`multiplicative`) because every
/// step in one chain shares the SAME operator (`logical_or = logical_and {
/// "OR" logical_and }` never mixes `OR` and `AND`). A single operand (no
/// operator present) is a transparent wrapper.
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

/// `logical_not = NOT logical_not | comparison`. A leading `NOT` wraps the
/// operand; otherwise it is the inner comparison.
///
/// `NOT` is matched in the grammar as a `Literal` (a keyword the D-2 lexer
/// promotes from a plain `NAME`, not a distinct regex-declared token type
/// like `PLUS`/`EQ`), so — mirroring `macsyma-compiler::compile_logical_not`'s
/// identical check for its own `Literal`-matched `"not"` — this checks the
/// token's literal *value*, not `effective_type_name()` (which stays
/// whatever the lexer classified the underlying lexeme as).
fn lower_logical_not(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let has_not = node
        .children
        .iter()
        .any(|c| as_token(c).is_some_and(|t| t.value == "NOT"));
    if !has_not {
        return lower_first_node(node);
    }
    let inner = child_nodes(node)
        .next()
        .ok_or_else(|| LowerError::new("`NOT` with no operand"))?;
    Ok(apply(sym(NOT), vec![lower_node(inner)?]))
}

/// `comparison = additive [ (EQ|LE|LESS|GREATER|GE) additive ]` — a single
/// (non-chained) comparison. `=` is Derive's *equation* operator (`Equal`),
/// never assignment — `:=` alone owns that role (MA07 §3).
fn lower_comparison(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let Some(op_index) = node
        .children
        .iter()
        .position(|c| as_token(c).is_some_and(|t| comparison_head(token_type(t)).is_some()))
    else {
        return lower_first_node(node);
    };
    if op_index == 0 || op_index + 1 >= node.children.len() {
        return Err(LowerError::new("malformed comparison node"));
    }
    let head = comparison_head(token_type(as_token(&node.children[op_index]).unwrap())).unwrap();
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
/// `Sub(Sub(a, b), c)`; `a + b - c` into `Sub(Add(a, b), c)`.
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

/// `unary = MINUS unary | power`. Derive's grammar (unlike Wolfram's) has no
/// unary-plus alternative — a leading `-` is `Neg`; otherwise it is the
/// inner `power`.
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

/// `power = postfix [ POWER unary ]` — right-associative `^`.
fn lower_power(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    match node.children.len() {
        1 => lower_child(&node.children[0]),
        3 => Ok(apply(
            sym(POW),
            vec![
                lower_child(&node.children[0])?,
                lower_child(&node.children[2])?,
            ],
        )),
        _ => Err(LowerError::new("malformed power node")),
    }
}

/// `postfix = atom { LPAREN [arglist] RPAREN }` — function application,
/// left-associative and chainable (`F(x)(y)` is `(F(x))(y)`, though Derive
/// has no idiom that actually produces one — included for grammar fidelity).
///
/// The head runs through [`canonical_head`] so a builtin surface name like
/// `SIN`/`DIF`/`IF` becomes the IR head (`Sin`/`D`/`If`) the VM dispatches;
/// an unrecognised head (a user-defined function) passes through unchanged.
fn lower_postfix(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let mut result = lower_child(
        node.children
            .first()
            .ok_or_else(|| LowerError::new("postfix has no base"))?,
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
                .map(lower_arglist)
                .transpose()?
                .unwrap_or_default();
            result = apply(canonical_head(result), args);
        }
        i += 1;
    }
    Ok(result)
}

/// `arglist = expr { COMMA expr }` — lower each comma-separated argument.
fn lower_arglist(node: &GrammarASTNode) -> Result<Vec<IRNode>, LowerError> {
    child_nodes(node).map(lower_node).collect()
}

/// `vector = LBRACKET row { SEMI row } RBRACKET` (D-5, MA07 §2/§3).
///
/// A vector `[a, b, c]` parses as exactly one `row`; a matrix `[a, b, c; d,
/// e, f]` parses as more than one — the grammar has no separate rule for the
/// two shapes (see `derive.grammar`'s own comment on `vector`), so this is
/// where they're told apart, purely by counting `row` children: one row
/// lowers to a flat `List(elems…)`, more than one lowers to a `List` of
/// per-row `List`s — `[a,b,c]` → `List[a,b,c]`, `[a,b,c;d,e,f]` →
/// `List[List[a,b,c], List[d,e,f]]`, matching MA07 §3's table exactly.
fn lower_vector(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let rows: Vec<&GrammarASTNode> = child_nodes(node).filter(|n| n.rule_name == "row").collect();
    if rows.len() == 1 {
        Ok(apply(sym(LIST), lower_row(rows[0])?))
    } else {
        let row_lists = rows
            .into_iter()
            .map(|row| Ok(apply(sym(LIST), lower_row(row)?)))
            .collect::<Result<Vec<IRNode>, LowerError>>()?;
        Ok(apply(sym(LIST), row_lists))
    }
}

/// `row = expr { COMMA expr }` — lower each comma-separated element (mirrors
/// [`lower_arglist`]'s identical shape).
fn lower_row(node: &GrammarASTNode) -> Result<Vec<IRNode>, LowerError> {
    child_nodes(node).map(lower_node).collect()
}

/// `atom = NUMBER | NAME | vector | group`.
fn lower_atom(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    if let Some(child) = child_nodes(node).next() {
        if matches!(child.rule_name.as_str(), "vector" | "group") {
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

/// Map a comparison token type to its IR head.
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

/// Bridge a Derive *surface* head (conventionally uppercase — MA07 §3) to the
/// canonical IR head. `Symbol` equality is case-sensitive, so every builtin
/// this D-4 milestone wires needs an explicit entry — not just the ones that
/// differ semantically (unlike Wolfram's bridge, which only needs to rename
/// the few operators whose long-form name isn't already the IR's own). A
/// head not in this table (a user-defined function, or a builtin not spelled
/// in the exact uppercase convention) is returned unchanged, so `F(x)`
/// evaluates through the VM's ordinary user-function path and an unrecognised
/// spelling stays a harmless unevaluated symbolic call.
fn canonical_head(head: IRNode) -> IRNode {
    if let IRNode::Symbol(name) = &head {
        if let Some(canonical) = surface_head_to_ir(name) {
            return sym(canonical);
        }
    }
    head
}

/// The surface→IR head dictionary for D-4's scope: the base calculus forms
/// (`DIF`→`D`, `INT`→`Integrate` — both already fully implemented in
/// `symbolic_vm::handlers::build_handler_table`, so this milestone adds no
/// new engine code, per MA07 §5's reuse strategy), `IF`→`If`, and the
/// elementary/hyperbolic functions (case-bridged only — `symbolic-ir`
/// already names these `Sin`/`Cos`/…, so no other transformation is needed).
/// `LIM`/`SOLVE`/`SUM`/`PRODUCT`/`TAYLOR` are deliberately absent — MA07 §4
/// ("Honest scope") defers them to their own follow-on items, since (unlike
/// `DIF`/`INT`) the shared VM has no existing handler for them; wiring them
/// here would be new engine code, not reuse.
fn surface_head_to_ir(name: &str) -> Option<&'static str> {
    Some(match name {
        "DIF" => D,
        "INT" => INTEGRATE,
        "IF" => IF,
        "SIN" => SIN,
        "COS" => COS,
        "TAN" => TAN,
        "SQRT" => SQRT,
        "EXP" => EXP,
        "LOG" => LOG,
        "ATAN" => ATAN,
        "ASIN" => ASIN,
        "ACOS" => ACOS,
        "SINH" => SINH,
        "COSH" => COSH,
        "TANH" => TANH,
        "ASINH" => ASINH,
        "ACOSH" => ACOSH,
        "ATANH" => ATANH,
        "COTH" => COTH,
        "SECH" => SECH,
        "CSCH" => CSCH,
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
/// operator still emits its own node with exactly one child —
/// `unwrap_single` skips straight to the rule that actually matters.
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
    use coding_adventures_derive_parser::parse_derive;

    /// Lower a single-statement source to its one IR node.
    fn lower_one(src: &str) -> IRNode {
        let ast = parse_derive(src);
        let mut stmts = lower_program(&ast).expect("lowering failed");
        assert_eq!(stmts.len(), 1, "expected exactly one statement for {src:?}");
        stmts.pop().unwrap()
    }

    #[test]
    fn integer_and_real_literals() {
        assert_eq!(lower_one("42\n"), int(42));
        assert_eq!(lower_one("1.5\n"), flt(1.5));
    }

    #[test]
    fn bare_symbol() {
        assert_eq!(lower_one("foo\n"), sym("foo"));
    }

    #[test]
    fn additive_lowers_to_add() {
        assert_eq!(lower_one("1 + 2\n"), apply(sym(ADD), vec![int(1), int(2)]));
    }

    #[test]
    fn subtraction_lowers_to_sub_left_assoc() {
        // a - b - c  ->  Sub(Sub(a, b), c)
        assert_eq!(
            lower_one("a - b - c\n"),
            apply(
                sym(SUB),
                vec![apply(sym(SUB), vec![sym("a"), sym("b")]), sym("c")]
            )
        );
    }

    #[test]
    fn mixed_additive_chain_folds_left_by_operator() {
        // a + b - c  ->  Sub(Add(a, b), c)
        assert_eq!(
            lower_one("a + b - c\n"),
            apply(
                sym(SUB),
                vec![apply(sym(ADD), vec![sym("a"), sym("b")]), sym("c")]
            )
        );
    }

    #[test]
    fn multiplication_and_division() {
        assert_eq!(
            lower_one("a * b\n"),
            apply(sym(MUL), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one("a / b\n"),
            apply(sym(DIV), vec![sym("a"), sym("b")])
        );
    }

    #[test]
    fn power_is_right_associative() {
        // a ^ b ^ c  ->  Pow(a, Pow(b, c))
        assert_eq!(
            lower_one("a ^ b ^ c\n"),
            apply(
                sym(POW),
                vec![sym("a"), apply(sym(POW), vec![sym("b"), sym("c")])]
            )
        );
    }

    #[test]
    fn unary_minus_binds_looser_than_power() {
        // -x^2  ->  Neg(Pow(x, 2))  (unary wraps power, not the reverse)
        assert_eq!(
            lower_one("-x^2\n"),
            apply(sym(NEG), vec![apply(sym(POW), vec![sym("x"), int(2)])])
        );
    }

    #[test]
    fn eq_lowers_to_equal_not_assign() {
        // `=` is Derive's equation operator, never assignment.
        assert_eq!(
            lower_one("x = 4\n"),
            apply(sym(EQUAL), vec![sym("x"), int(4)])
        );
    }

    #[test]
    fn comparisons_lower_to_their_heads() {
        assert_eq!(
            lower_one("a <= b\n"),
            apply(sym(LESS_EQUAL), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one("a < b\n"),
            apply(sym(LESS), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one("a > b\n"),
            apply(sym(GREATER), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one("a >= b\n"),
            apply(sym(GREATER_EQUAL), vec![sym("a"), sym("b")])
        );
    }

    #[test]
    fn boolean_keywords_lower_to_and_or_not() {
        assert_eq!(
            lower_one("a AND b\n"),
            apply(sym(AND), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one("a OR b\n"),
            apply(sym(OR), vec![sym("a"), sym("b")])
        );
        assert_eq!(lower_one("NOT a\n"), apply(sym(NOT), vec![sym("a")]));
    }

    #[test]
    fn logical_or_chain_folds_n_ary() {
        // a OR b OR c  ->  Or(a, b, c) — homogeneous chain, safe to fold flat.
        assert_eq!(
            lower_one("a OR b OR c\n"),
            apply(sym(OR), vec![sym("a"), sym("b"), sym("c")])
        );
    }

    #[test]
    fn grouping_parens_lower_transparently() {
        assert_eq!(
            lower_one("(1 + 2) * 3\n"),
            apply(
                sym(MUL),
                vec![apply(sym(ADD), vec![int(1), int(2)]), int(3)]
            )
        );
    }

    #[test]
    fn function_application_of_unknown_head_passes_through() {
        // F(a, b)  ->  F(a, b) — a user-defined function, no bridging.
        assert_eq!(
            lower_one("F(a, b)\n"),
            apply(sym("F"), vec![sym("a"), sym("b")])
        );
    }

    #[test]
    fn builtin_uppercase_calls_are_bridged_to_canonical_ir_heads() {
        assert_eq!(
            lower_one("DIF(u, x)\n"),
            apply(sym(D), vec![sym("u"), sym("x")])
        );
        assert_eq!(
            lower_one("INT(u, x)\n"),
            apply(sym(INTEGRATE), vec![sym("u"), sym("x")])
        );
        assert_eq!(
            lower_one("INT(u, x, a, b)\n"),
            apply(sym(INTEGRATE), vec![sym("u"), sym("x"), sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one("IF(a, b, c)\n"),
            apply(sym(IF), vec![sym("a"), sym("b"), sym("c")])
        );
        assert_eq!(lower_one("SIN(x)\n"), apply(sym(SIN), vec![sym("x")]));
        assert_eq!(lower_one("SQRT(x)\n"), apply(sym(SQRT), vec![sym("x")]));
    }

    #[test]
    fn lowercase_spelling_is_not_bridged() {
        // Only the exact uppercase convention is bridged (case-sensitive,
        // matching IRNode::Symbol equality) — a different casing is just an
        // ordinary user symbol/call, not the builtin.
        assert_eq!(lower_one("sin(x)\n"), apply(sym("sin"), vec![sym("x")]));
    }

    #[test]
    fn variable_assignment_lowers_to_assign() {
        assert_eq!(
            lower_one("x := 5\n"),
            apply(sym(symbolic_ir::ASSIGN), vec![sym("x"), int(5)])
        );
    }

    #[test]
    fn function_definition_lowers_to_define() {
        // F(x) := x^2 + 1  ->  Define(F, List(x), Add(Pow(x, 2), 1))
        assert_eq!(
            lower_one("F(x) := x^2 + 1\n"),
            apply(
                sym(DEFINE),
                vec![
                    sym("F"),
                    apply(sym(LIST), vec![sym("x")]),
                    apply(
                        sym(ADD),
                        vec![apply(sym(POW), vec![sym("x"), int(2)]), int(1)]
                    ),
                ]
            )
        );
    }

    #[test]
    fn multi_param_function_definition() {
        // F(x, y) := x + y  ->  Define(F, List(x, y), Add(x, y))
        assert_eq!(
            lower_one("F(x, y) := x + y\n"),
            apply(
                sym(DEFINE),
                vec![
                    sym("F"),
                    apply(sym(LIST), vec![sym("x"), sym("y")]),
                    apply(sym(ADD), vec![sym("x"), sym("y")]),
                ]
            )
        );
    }

    #[test]
    fn assign_vs_define_disambiguated_by_lhs_shape_not_operator() {
        // Both use the identical `:=` token; only the parsed LHS shape
        // decides Assign vs Define (there is no SET/SETDELAYED distinction
        // in this grammar, unlike Wolfram/Macsyma).
        assert!(matches!(
            lower_one("x := 5\n"),
            IRNode::Apply(a) if matches!(&a.head, IRNode::Symbol(s) if s == symbolic_ir::ASSIGN)
        ));
        assert!(matches!(
            lower_one("F(x) := x\n"),
            IRNode::Apply(a) if matches!(&a.head, IRNode::Symbol(s) if s == DEFINE)
        ));
    }

    #[test]
    fn vector_literal_lowers_to_flat_list() {
        // [a, b, c] -> List(a, b, c) — one row.
        assert_eq!(
            lower_one("[a, b, c]\n"),
            apply(sym(LIST), vec![sym("a"), sym("b"), sym("c")])
        );
    }

    #[test]
    fn matrix_literal_lowers_to_list_of_row_lists() {
        // [a, b; c, d] -> List(List(a, b), List(c, d)) — two rows.
        assert_eq!(
            lower_one("[a, b; c, d]\n"),
            apply(
                sym(LIST),
                vec![
                    apply(sym(LIST), vec![sym("a"), sym("b")]),
                    apply(sym(LIST), vec![sym("c"), sym("d")]),
                ]
            )
        );
    }

    #[test]
    fn single_element_vector_lowers_to_singleton_list() {
        assert_eq!(lower_one("[5]\n"), apply(sym(LIST), vec![int(5)]));
    }

    #[test]
    fn three_row_matrix_lowers_to_three_row_lists() {
        // [1; 2; 3] -> List(List(1), List(2), List(3)) — three one-element rows.
        assert_eq!(
            lower_one("[1; 2; 3]\n"),
            apply(
                sym(LIST),
                vec![
                    apply(sym(LIST), vec![int(1)]),
                    apply(sym(LIST), vec![int(2)]),
                    apply(sym(LIST), vec![int(3)]),
                ]
            )
        );
    }

    #[test]
    fn vector_of_expressions_lowers_each_element() {
        // [x + 1, x * 2] -> List(Add(x, 1), Times(x, 2))
        assert_eq!(
            lower_one("[x + 1, x * 2]\n"),
            apply(
                sym(LIST),
                vec![
                    apply(sym(ADD), vec![sym("x"), int(1)]),
                    apply(sym(MUL), vec![sym("x"), int(2)]),
                ]
            )
        );
    }

    #[test]
    fn vector_assigned_to_a_variable() {
        // v := [1, 2, 3]  ->  Assign(v, List(1, 2, 3))
        assert_eq!(
            lower_one("v := [1, 2, 3]\n"),
            apply(
                sym(symbolic_ir::ASSIGN),
                vec![sym("v"), apply(sym(LIST), vec![int(1), int(2), int(3)])]
            )
        );
    }

    #[test]
    fn nested_function_calls_lower_correctly() {
        // SIN(COS(x))  ->  Sin(Cos(x))
        assert_eq!(
            lower_one("SIN(COS(x))\n"),
            apply(sym(SIN), vec![apply(sym(COS), vec![sym("x")])])
        );
    }

    #[test]
    fn multi_statement_program_lowers_each_line() {
        let ast = parse_derive("F(x) := DIF(SIN(x), x)\nF(0)\n");
        let stmts = lower_program(&ast).expect("lowering failed");
        assert_eq!(stmts.len(), 2);
        assert!(matches!(
            &stmts[0],
            IRNode::Apply(a) if matches!(&a.head, IRNode::Symbol(s) if s == DEFINE)
        ));
        assert_eq!(stmts[1], apply(sym("F"), vec![int(0)]));
    }
}
