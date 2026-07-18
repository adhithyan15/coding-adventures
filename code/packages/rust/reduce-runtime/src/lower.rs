//! # Lowering — Reduce syntax tree → `symbolic-ir`
//!
//! The R-3 [`reduce-parser`](coding_adventures_reduce_parser) hands us a
//! *generic* [`GrammarASTNode`] tree whose `rule_name`s mirror
//! `reduce.grammar` (`assignment`, `additive`, `multiplicative`, `power`,
//! `postfix`, `atom`, `if_expr`, `group_expr`, …). The parser deliberately
//! does **no** semantic work — it just records which rule matched. This
//! module is where Reduce's *meaning* is assigned: every surface construct
//! is **desugared** into the canonical [`symbolic_ir::IRNode`] head
//! [`symbolic-vm`](symbolic_vm) already knows how to evaluate, per MA08 §3's
//! table.
//!
//! ## Much of this is a direct copy of `derive-runtime::lower`'s shape
//!
//! Reduce (MA08 §1) is, like Derive, a "surface operators + `head(args)`
//! calls" language with no `f[x]`-universal-application syntax and no
//! pattern/rewrite-rule vocabulary in this subset (MA08 §4 defers `let`
//! rules) — so this module needs none of Wolfram's pattern-lowering or
//! `ReplaceAll` machinery, just arithmetic, comparison, logic, assignment/
//! definition, function application, and (new relative to Derive) lists,
//! cons, and two expression-shaped control-flow forms (`if`, `<< ... >>`).
//!
//! ## A REAL divergence from MA08 §3's own prose: arithmetic head *names*
//!
//! MA08 §3's table spells the "Lowers to" column for arithmetic as `Plus`/
//! `Subtract`/`Times`/`Power`, and even expands `a / b` to
//! `Times[a, Power[b, -1]]` and `-a` to `Times[-1, a]`. **None of those
//! spellings exist in `symbolic-ir`** — grep it yourself:
//! `grep -n '"Plus"\|"Subtract"\|"Times"\|"Power"' symbolic-ir/src/lib.rs`
//! returns nothing. What actually exists, and what
//! `symbolic_vm::handlers::build_handler_table` actually wires handlers for,
//! is [`ADD`]/[`SUB`]/[`MUL`]/[`DIV`]/[`POW`]/[`NEG`] — the *exact* heads
//! `derive-runtime::lower` and `macsyma-compiler` already lower `+`/`-`/`*`/
//! `/`/`^`/unary-`-` to. Using the shared `Div`/`Neg` handlers directly
//! (rather than literally expanding division/negation into `Times`/`Power`
//! applications, which would sidestep the very handlers MA08 §5 says this
//! milestone reuses, and would make Reduce's `1/2` print/compare
//! differently from Derive's or Macsyma's identical expression) is *more*
//! faithful to "the exact same functions, so all four languages agree on
//! every result" (MA08 §5) than following the spec prose literally would
//! have been. This is a disclosed, deliberate divergence — MA08 §3's table
//! has been corrected to match (see the spec's own changelog-style note),
//! and is called out in R-4's commit message.
//!
//! ## A REAL gap: several MA08 §3 heads have no handler in `symbolic-vm` at all
//!
//! Unlike the arithmetic renaming above (a naming mismatch with a working
//! handler underneath), grepping `symbolic-vm::handlers::build_handler_table`
//! for `CompoundExpression`, `First`, `Second`, `Third`, `Rest`, `Part`,
//! `Append`, and `Reverse` turns up **nothing** — no handler is registered
//! for any of them. MA08 §5's claim that these are "already implemented for
//! Macsyma/Wolfram/Derive — the exact same functions" does not hold for the
//! *shared* `SymbolicBackend`/`build_handler_table` this crate is required
//! to reuse *unchanged*: Macsyma's `first`/`rest`/`append`/`reverse` are
//! real (`cas-list-operations`), and Wolfram's `CompoundExpression` is real,
//! but both are wired through a **bespoke `Backend`** specific to that
//! language (`macsyma-runtime`/`wolfram-runtime`'s own `builtins.rs`), which
//! is precisely the thing MA08 §2/§5 says R-4 must *not* build ("reused
//! *unchanged*, with no custom `Backend` at all"). Per this crate's own
//! marching orders ("if any is missing, that's a real gap to flag ...  not
//! something to invent new engine code for beyond what's needed"), this
//! lowering still produces the structurally-correct heads MA08 §3
//! documents (so the parsed *shape* is right, and a future item that adds a
//! `cas-list-operations`-backed handler — or a documented, narrow custom
//! `Backend` — to the shared table needs no lowering changes at all), but
//! evaluating one of these calls does **not** perform the list operation:
//! the arguments evaluate (so side effects inside them, e.g. an `Assign`,
//! still happen), and [`Backend::on_unknown_head`]'s default fallback
//! leaves the call itself unevaluated, exactly like calling an
//! undefined user function. `Cons` is the same story for the one shape MA08
//! §3 does *not* say to fold away (see [`fold_cons`]).
//!
//! [`Backend::on_unknown_head`]: symbolic_vm::backend::Backend::on_unknown_head

use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use symbolic_ir::{
    apply, flt, int, sym, IRNode, ADD, AND, ASSIGN, DEFINE, DIV, EQUAL, GREATER, GREATER_EQUAL, IF,
    LESS, LESS_EQUAL, LIST, MUL, NEG, NOT, NOT_EQUAL, OR, POW, SUB,
};

/// The canonical head for a `<< s1; s2; ... >>` group statement (MA08 §3).
///
/// Not exported by `symbolic-ir` (see the module doc comment's "REAL gap"
/// section) — defined locally so the one place this crate needs the
/// spelling has a name, not a repeated string literal.
pub const COMPOUND_EXPRESSION: &str = "CompoundExpression";

/// The canonical head for a non-foldable `a . b` cons (MA08 §3). See
/// [`fold_cons`] — this head is only ever produced when the right-hand side
/// isn't structurally a literal `List`, the one case MA08 §3 does not
/// document a fold for.
pub const CONS: &str = "Cons";

/// The canonical heads for Reduce's list accessors/constructors (MA08 §3).
/// Spelled to match `cas-list-operations`' own `FIRST`/`REST`/`APPEND`/
/// `REVERSE`/`PART` constants (that crate is not a dependency here — see
/// the module doc comment — but the *spelling* is kept identical on
/// purpose, so a future handler wiring either crate's constants into the
/// shared table needs no change on this crate's side).
pub const FIRST: &str = "First";
pub const SECOND: &str = "Second";
pub const THIRD: &str = "Third";
pub const REST: &str = "Rest";
pub const PART: &str = "Part";
pub const APPEND: &str = "Append";
pub const REVERSE: &str = "Reverse";

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
/// `reduce.grammar`'s root is `program = { statement_line } [ statement ]`:
/// zero or more `;`/`$`-terminated `statement_line`s, plus an optional final
/// bare `statement` with no terminator (so a source file need not end with
/// one). Both shapes contribute exactly one lowered statement each.
pub fn lower_program(root: &GrammarASTNode) -> Result<Vec<IRNode>, LowerError> {
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
                if let Some(statement) = child_nodes(child).find(|n| n.rule_name == "statement") {
                    statements.push(lower_node(statement)?);
                }
            }
            "statement" => statements.push(lower_node(child)?),
            _ => {}
        }
    }
    Ok(statements)
}

/// Lower a single arbitrary node.
///
/// Most grammar rules are "transparent wrappers" — a `statement` is just an
/// `expr`, an `expr` is just whichever of `if_expr`/`group_expr`/
/// `assignment` matched, and when a precedence level did not actually apply
/// its operator the parser still emits the level's node with a single
/// child. [`unwrap_single`] peels those away so we dispatch on the first
/// rule that genuinely shapes the tree.
fn lower_node(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    match unwrap_single(node) {
        Unwrapped::Token(token) => lower_token(token),
        Unwrapped::Node(node) => match node.rule_name.as_str() {
            "program" => Err(LowerError::new("nested program node is not an expression")),
            "statement_line" | "statement" | "expr" => lower_first_node(node),
            "if_expr" => lower_if(node),
            "group_expr" => lower_group_expr(node),
            "assignment" => lower_assignment(node),
            "logical_or" => lower_logical_chain(node, OR),
            "logical_and" => lower_logical_chain(node, AND),
            "logical_not" => lower_logical_not(node),
            "comparison" => lower_comparison(node),
            "cons" => lower_cons(node),
            "additive" | "multiplicative" => lower_binary_chain(node),
            "unary" => lower_unary(node),
            "power" => lower_power(node),
            "postfix" => lower_postfix(node),
            "atom" => lower_atom(node),
            "list_literal" => lower_list_literal(node),
            "arglist" => Err(LowerError::new(
                "an arglist cannot be lowered as a scalar expression",
            )),
            "group" => lower_group(node),
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
/// The R-2 lexer's `NUMBER` regex is `[0-9]+\.?[0-9]*([eE][+-]?[0-9]+)?`
/// (identical to Derive's/Macsyma's own), so a `.`, `e`, or `E` means it is
/// a real; otherwise it is an integer. We reject an integer that overflows
/// `i64` (the IR's integer width) rather than silently wrapping.
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

/// `if_expr = "if" expr "then" expr [ "else" expr ]` — MA08 §3: `If[b, s1,
/// s2]`, or (no `else`) `If[b, s1]`, which `symbolic_vm::handlers`' own
/// `if_handler` already accepts (2 *or* 3 args — the 2-arg case returns the
/// `False` symbol when the condition doesn't hold, since there is no
/// `else`-branch value to produce), so this needs no special-casing beyond
/// counting how many `expr` children were parsed.
fn lower_if(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let branches: Vec<&GrammarASTNode> = child_nodes(node).collect();
    match branches.len() {
        2 => Ok(apply(
            sym(IF),
            vec![lower_node(branches[0])?, lower_node(branches[1])?],
        )),
        3 => Ok(apply(
            sym(IF),
            vec![
                lower_node(branches[0])?,
                lower_node(branches[1])?,
                lower_node(branches[2])?,
            ],
        )),
        n => Err(LowerError::new(format!(
            "if_expr expected 2 or 3 `expr` children (cond/then[/else]), got {n}"
        ))),
    }
}

/// `group_expr = GROUP_OPEN expr { ( SEMI | DOLLAR ) expr } GROUP_CLOSE` —
/// MA08 §3's `<< s1; s2; ... >>`, lowered to `CompoundExpression[s1, s2,
/// ...]`. See the module doc comment's "REAL gap" section: `symbolic-vm`'s
/// shared handler table has no handler for this head, so — while each
/// `s_i` still evaluates in order (the VM evaluates every argument of an
/// unheld `Apply` before dispatching, and `CompoundExpression` is not in
/// `BaseBackend`'s held-head set), so `<< x := 1; x + 1 >>` really does bind
/// `x` and really does compute `2` for its second statement — the
/// *overall* result is the unevaluated `CompoundExpression(1, 2)`, not bare
/// `2`, because there is no handler to collapse it to "the last statement's
/// value" the way MA08 §3 describes. A disclosed gap, not a silent bug.
fn lower_group_expr(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let exprs: Vec<&GrammarASTNode> = child_nodes(node).collect();
    if exprs.is_empty() {
        return Err(LowerError::new("empty group statement `<< >>`"));
    }
    let lowered = exprs
        .into_iter()
        .map(lower_node)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(apply(sym(COMPOUND_EXPRESSION), lowered))
}

/// `assignment = logical_or [ ASSIGN expr ]` — right-associative (manual
/// §2.7: "a:=b:=c evaluates as a:=(b:=c)"), disambiguated purely by the
/// lowered LHS's *shape* since — exactly like Derive (see
/// `derive-runtime::lower`'s identical note) — Reduce's grammar has only
/// ONE assignment token reaching this rule: `Apply(Symbol(_), _)` → a
/// `h(l, m) := e` procedure definition (`Define`); anything else → an
/// ordinary `x := e` variable assignment (`Assign`).
///
/// Unlike `derive.grammar`'s `assignment = logical_or [ ASSIGN assignment
/// ]` (whose RHS recurses back into `assignment` itself),
/// `reduce.grammar`'s RHS is the wider `expr` — so `x := if a>0 then 1 else
/// -1` and `x := << a:=1; a+1 >>` parse (and lower) directly, since
/// Reduce's `if`/`<< ... >>` are genuinely usable as expressions (MA08 §3),
/// unlike anything in Derive's grammar.
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
            // h(l, m) := e — a procedure definition. Reduce has no pattern
            // syntax either, so each parameter already lowered to a plain
            // `Symbol` (or, for a malformed definition like `h(1) := e`,
            // whatever the caller wrote — `define_handler` and the VM's
            // parameter binder own validating that, not this lowering).
            return Ok(apply(
                sym(DEFINE),
                vec![app.head.clone(), apply(sym(LIST), app.args.clone()), rhs],
            ));
        }
    }
    // x := e — variable assignment.
    Ok(apply(sym(ASSIGN), vec![lhs, rhs]))
}

/// `logical_or`/`logical_and` — fold the operands into an n-ary `Or`/`And`.
/// Safe to fold n-ary (unlike `additive`/`multiplicative`) because every
/// step in one chain shares the SAME operator (`logical_or = logical_and {
/// "or" logical_and }` never mixes `or` and `and`). A single operand (no
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

/// `logical_not = "not" logical_not | comparison`. A leading `not` wraps the
/// operand; otherwise it is the inner comparison.
///
/// `and`/`or`/`not`/`neq`/`if`/`then`/`else` are all matched in the grammar
/// as `reduce.tokens`' own `KEYWORD` token type (promoted from `NAME` by
/// exact lowercase spelling — see `reduce.tokens`'s header), so — mirroring
/// `derive-runtime::lower_logical_not`'s identical check for its own
/// `Literal`-matched `"NOT"` — this checks the token's literal *value*, not
/// `effective_type_name()` alone (every keyword shares that same type
/// name).
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

/// `comparison = cons [ ( EQ | "neq" | LESS | GREATER | LE | GE ) cons ]` —
/// a single (non-chained) comparison, per MA08 §3's own disclosed
/// simplification ("this subset treats the whole group as one flat,
/// non-chaining comparison tier"). `=` is Reduce's *equation* operator
/// (`Equal`), never assignment — `:=` alone owns that role (MA08 §3,
/// manual §3.4).
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

/// `cons = additive [ DOT cons ] ` — right-associative (`cons`'s own
/// optional continuation references itself: `a . b . {c}` is `a . (b .
/// {c})`, so lowering the RHS recursively folds inside-out before this
/// call ever sees it — see [`fold_cons`]).
fn lower_cons(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let Some(dot_index) = node
        .children
        .iter()
        .position(|c| as_token(c).is_some_and(|t| token_type(t) == "DOT"))
    else {
        return lower_first_node(node);
    };
    if dot_index == 0 || dot_index + 1 >= node.children.len() {
        return Err(LowerError::new("malformed cons node"));
    }
    let lhs = lower_child(&node.children[dot_index - 1])?;
    let rhs = lower_child(&node.children[dot_index + 1])?;
    Ok(fold_cons(lhs, rhs))
}

/// Fold `a . rhs` — MA08 §3's own words: "R-4 folds a `Cons` onto a literal
/// `List` immediately into one `List`". When `rhs` lowered to a
/// *structurally* literal `List(...)` application, prepend `lhs` directly
/// into a new flat `List` — no `Cons` head is ever produced for this case,
/// so it needs no VM handler at all (`List`'s own handler, already shared
/// and reused, does all the work). This is the ONLY shape MA08 §3's table
/// documents a lowering for.
///
/// A right-hand side that ISN'T structurally a `List` at lowering time
/// (`a . b`, where `b` is a bound variable, a function call, or another
/// not-yet-resolved expression — lowering runs once, before any VM
/// evaluation, so it cannot know what `b` will turn out to be) has no such
/// fold available; MA08 §3's table is silent on this case. Rather than
/// reject it outright, it lowers to a plain `Cons[a, b]` application — the
/// same "structurally correct, but no handler evaluates it further" gap as
/// `First`/`Rest`/etc (see the module doc comment).
fn fold_cons(lhs: IRNode, rhs: IRNode) -> IRNode {
    if let IRNode::Apply(app) = &rhs {
        if matches!(&app.head, IRNode::Symbol(s) if s == LIST) {
            let mut elems = vec![lhs];
            elems.extend(app.args.iter().cloned());
            return apply(sym(LIST), elems);
        }
    }
    apply(sym(CONS), vec![lhs, rhs])
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

/// `unary = MINUS unary | power`. MA08 §3 lists only unary `-` (no unary
/// `+`, matching `derive.grammar`'s identical asymmetry) — a leading `-` is
/// `Neg`; otherwise it is the inner `power`.
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

/// `power = postfix [ ( CARET | POW ) unary ]` — right-associative `^`/`**`
/// (MA08 §3: "manual §2.7's own precedence table lists them as one tier" —
/// `reduce.tokens` keeps `CARET`/`POW` as two distinct *token* types, but
/// this grammar tier already collapses both onto the one `power` production,
/// so lowering just needs to accept either token type here).
fn lower_power(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    match node.children.len() {
        1 => lower_child(&node.children[0]),
        3 => {
            let is_power_op = as_token(&node.children[1])
                .is_some_and(|t| matches!(token_type(t), "CARET" | "POW"));
            if !is_power_op {
                return Err(LowerError::new(
                    "malformed power node: expected CARET or POW",
                ));
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

/// `postfix = atom { LPAREN [arglist] RPAREN }` — function/procedure/
/// array-subscript application, left-associative and chainable. MA08 §3's
/// single call-shaped production covers `f(a, b)`, a `Define` LHS like
/// `h(l, m)`, and `a(5)`/`b(i, q)` (array-subscript *reads* — array
/// declaration/indexed *write* are out of scope, MA08 §4) all at once.
///
/// The head runs through [`canonical_head`] so a builtin surface name like
/// `list`/`first`/`rest`/`append`/`reverse` becomes the IR head the VM
/// dispatches (or, per the module doc comment's "REAL gap" section,
/// structurally produces but cannot yet further evaluate); an unrecognised
/// head (a user-defined operator/procedure) passes through unchanged.
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

/// `atom = NUMBER | NAME | list_literal | group`.
fn lower_atom(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    if let Some(child) = child_nodes(node).next() {
        if matches!(child.rule_name.as_str(), "list_literal" | "group") {
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

/// `list_literal = LBRACE [ arglist ] RBRACE` — MA08 §3's `{a, b, c}`
/// (curly braces, NOT Derive's square brackets). Reduce's list is always
/// flat here (no row/matrix shape — matrices are out of scope, MA08 §4), so
/// this reuses [`lower_arglist`] directly, unlike `derive-runtime`'s
/// row-counting `vector`/`row` split.
fn lower_list_literal(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let args = child_nodes(node)
        .find(|n| n.rule_name == "arglist")
        .map(lower_arglist)
        .transpose()?
        .unwrap_or_default();
    Ok(apply(sym(LIST), args))
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

/// Map an arithmetic token type to its IR head. See the module doc
/// comment's naming-divergence note: these are `Add`/`Sub`/`Mul`/`Div`, not
/// MA08 §3's literal (and non-existent) `Plus`/`Subtract`/`Times`.
fn binary_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "PLUS" => Some(ADD),
        "MINUS" => Some(SUB),
        "TIMES" => Some(MUL),
        "SLASH" => Some(DIV),
        _ => None,
    }
}

/// Map a comparison token to its IR head. `neq` is a `KEYWORD`-typed token
/// (see `lower_logical_not`'s identical note), matched by literal value
/// alongside the four symbolic comparison token *types*.
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

/// Bridge a Reduce *surface* builtin name (lowercase, per manual convention
/// — `list`, `first`, `second`, `third`, `rest`, `part`, `append`,
/// `reverse`) to the canonical IR head. A head not in this table (a
/// user-defined operator/procedure, or a builtin spelled with different
/// casing) is returned unchanged, so it evaluates through the VM's ordinary
/// user-function path and an unrecognised spelling stays a harmless
/// unevaluated symbolic call — exactly `derive-runtime::lower::canonical_head`'s
/// same fallthrough contract.
fn canonical_head(head: IRNode) -> IRNode {
    if let IRNode::Symbol(name) = &head {
        if let Some(canonical) = surface_head_to_ir(name) {
            return sym(canonical);
        }
    }
    head
}

/// The surface→IR head dictionary for R-4's scope (MA08 §3): the
/// function-call spelling of a list literal (`list(...)` → `List`) and the
/// list accessors/constructors. Every one of these except `List` has no
/// handler in the shared `symbolic-vm` table (see the module doc comment's
/// "REAL gap" section) — they are bridged here anyway so the lowered
/// *shape* is exactly what MA08 §3's table says, ready for a future item
/// that wires the handlers without touching this lowering at all.
fn surface_head_to_ir(name: &str) -> Option<&'static str> {
    Some(match name {
        "list" => LIST,
        "first" => FIRST,
        "second" => SECOND,
        "third" => THIRD,
        "rest" => REST,
        "part" => PART,
        "append" => APPEND,
        "reverse" => REVERSE,
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
/// ordered-choice rule like `expr = if_expr | group_expr | assignment` once
/// it has committed to one alternative — `unwrap_single` skips straight to
/// the rule that actually matters.
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
    use coding_adventures_reduce_parser::parse_reduce;

    /// Lower a single-statement source to its one IR node.
    fn lower_one(src: &str) -> IRNode {
        let ast = parse_reduce(src);
        let mut stmts = lower_program(&ast).expect("lowering failed");
        assert_eq!(stmts.len(), 1, "expected exactly one statement for {src:?}");
        stmts.pop().unwrap()
    }

    #[test]
    fn integer_and_real_literals() {
        assert_eq!(lower_one("42;\n"), int(42));
        assert_eq!(lower_one("1.5;\n"), flt(1.5));
    }

    #[test]
    fn bare_symbol() {
        assert_eq!(lower_one("foo;\n"), sym("foo"));
    }

    #[test]
    fn bare_trailing_statement_with_no_terminator_lowers() {
        assert_eq!(lower_one("42"), int(42));
    }

    // --- Arithmetic (MA08 §3 row: `a+b`,`a-b`->Plus/Subtract; `a*b`->Times;
    //     `a/b`->Times[a,Power[b,-1]] -- this crate instead reuses Add/Sub/
    //     Mul/Div directly; see the module doc comment's divergence note) --

    #[test]
    fn additive_lowers_to_add() {
        assert_eq!(lower_one("1 + 2;\n"), apply(sym(ADD), vec![int(1), int(2)]));
    }

    #[test]
    fn subtraction_lowers_to_sub_left_assoc() {
        // a - b - c  ->  Sub(Sub(a, b), c)
        assert_eq!(
            lower_one("a - b - c;\n"),
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
            lower_one("a + b - c;\n"),
            apply(
                sym(SUB),
                vec![apply(sym(ADD), vec![sym("a"), sym("b")]), sym("c")]
            )
        );
    }

    #[test]
    fn multiplication_and_division_use_mul_and_div_directly() {
        assert_eq!(
            lower_one("a * b;\n"),
            apply(sym(MUL), vec![sym("a"), sym("b")])
        );
        // NOT Times(a, Pow(b, -1)) -- see the module doc comment.
        assert_eq!(
            lower_one("a / b;\n"),
            apply(sym(DIV), vec![sym("a"), sym("b")])
        );
    }

    #[test]
    fn power_operator_and_double_star_both_lower_to_pow() {
        assert_eq!(
            lower_one("a ^ b;\n"),
            apply(sym(POW), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one("a ** b;\n"),
            apply(sym(POW), vec![sym("a"), sym("b")])
        );
    }

    #[test]
    fn power_is_right_associative() {
        // a ^ b ^ c  ->  Pow(a, Pow(b, c))
        assert_eq!(
            lower_one("a ^ b ^ c;\n"),
            apply(
                sym(POW),
                vec![sym("a"), apply(sym(POW), vec![sym("b"), sym("c")])]
            )
        );
    }

    #[test]
    fn unary_minus_lowers_to_neg_and_binds_looser_than_power() {
        // -x^2  ->  Neg(Pow(x, 2)) -- NOT Times(-1, Pow(x, 2)).
        assert_eq!(
            lower_one("-x^2;\n"),
            apply(sym(NEG), vec![apply(sym(POW), vec![sym("x"), int(2)])])
        );
    }

    // --- Comparison / equation (MA08 §3) -----------------------------------

    #[test]
    fn eq_lowers_to_equal_not_assign() {
        assert_eq!(
            lower_one("x = 4;\n"),
            apply(sym(EQUAL), vec![sym("x"), int(4)])
        );
    }

    #[test]
    fn every_comparison_operator_lowers_to_its_head() {
        assert_eq!(
            lower_one("a < b;\n"),
            apply(sym(LESS), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one("a > b;\n"),
            apply(sym(GREATER), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one("a <= b;\n"),
            apply(sym(LESS_EQUAL), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one("a >= b;\n"),
            apply(sym(GREATER_EQUAL), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one("a neq b;\n"),
            apply(sym(NOT_EQUAL), vec![sym("a"), sym("b")])
        );
    }

    // --- Logic (MA08 §3) ---------------------------------------------------

    #[test]
    fn boolean_keywords_lower_to_and_or_not() {
        assert_eq!(
            lower_one("a and b;\n"),
            apply(sym(AND), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one("a or b;\n"),
            apply(sym(OR), vec![sym("a"), sym("b")])
        );
        assert_eq!(lower_one("not a;\n"), apply(sym(NOT), vec![sym("a")]));
    }

    #[test]
    fn logical_or_chain_folds_n_ary() {
        assert_eq!(
            lower_one("a or b or c;\n"),
            apply(sym(OR), vec![sym("a"), sym("b"), sym("c")])
        );
    }

    #[test]
    fn uppercase_keyword_spellings_are_not_special_cased_here() {
        // reduce.tokens' keywords are lowercase-only, so `AND` etc lex as a
        // plain NAME at the R-2 layer already; this crate never sees an
        // uppercase logical keyword to bridge (the mirror image of Derive's
        // uppercase-only AND/OR/NOT).
        assert_eq!(lower_one("AND;\n"), sym("AND"));
    }

    // --- Grouping ------------------------------------------------------------

    #[test]
    fn grouping_parens_lower_transparently() {
        assert_eq!(
            lower_one("(1 + 2) * 3;\n"),
            apply(
                sym(MUL),
                vec![apply(sym(ADD), vec![int(1), int(2)]), int(3)]
            )
        );
    }

    // --- Function/procedure application (MA08 §3) --------------------------

    #[test]
    fn function_application_of_unknown_head_passes_through() {
        assert_eq!(
            lower_one("f(a, b);\n"),
            apply(sym("f"), vec![sym("a"), sym("b")])
        );
    }

    #[test]
    fn array_subscript_read_shares_the_call_production() {
        // a(5) / b(i, q) -- MA08 §3: reads exactly like an ordinary call.
        assert_eq!(lower_one("a(5);\n"), apply(sym("a"), vec![int(5)]));
        assert_eq!(
            lower_one("b(i, q);\n"),
            apply(sym("b"), vec![sym("i"), sym("q")])
        );
    }

    #[test]
    fn nested_function_calls_lower_correctly() {
        assert_eq!(
            lower_one("log(exp(x));\n"),
            apply(sym("log"), vec![apply(sym("exp"), vec![sym("x")])])
        );
    }

    // --- Assignment / procedure definition (MA08 §3) -----------------------

    #[test]
    fn variable_assignment_lowers_to_assign() {
        assert_eq!(
            lower_one("x := 5;\n"),
            apply(sym(ASSIGN), vec![sym("x"), int(5)])
        );
    }

    #[test]
    fn procedure_definition_lowers_to_define() {
        // h(l, m) := l - 2*m  ->  Define(h, List(l, m), Sub(l, Mul(2, m)))
        assert_eq!(
            lower_one("h(l, m) := l - 2*m;\n"),
            apply(
                sym(DEFINE),
                vec![
                    sym("h"),
                    apply(sym(LIST), vec![sym("l"), sym("m")]),
                    apply(
                        sym(SUB),
                        vec![sym("l"), apply(sym(MUL), vec![int(2), sym("m")])]
                    ),
                ]
            )
        );
    }

    #[test]
    fn assign_vs_define_disambiguated_by_lhs_shape_not_operator() {
        assert!(matches!(
            lower_one("x := 5;\n"),
            IRNode::Apply(a) if matches!(&a.head, IRNode::Symbol(s) if s == ASSIGN)
        ));
        assert!(matches!(
            lower_one("h(x) := x;\n"),
            IRNode::Apply(a) if matches!(&a.head, IRNode::Symbol(s) if s == DEFINE)
        ));
    }

    #[test]
    fn assignment_right_associates() {
        // a := b := 5  ->  Assign(a, Assign(b, 5))
        assert_eq!(
            lower_one("a := b := 5;\n"),
            apply(
                sym(ASSIGN),
                vec![sym("a"), apply(sym(ASSIGN), vec![sym("b"), int(5)])]
            )
        );
    }

    #[test]
    fn if_is_usable_as_an_assignment_rhs() {
        // x := if a>0 then 1 else -1
        let lowered = lower_one("x := if a>0 then 1 else -1;\n");
        assert!(matches!(
            &lowered,
            IRNode::Apply(a) if matches!(&a.head, IRNode::Symbol(s) if s == ASSIGN)
        ));
        if let IRNode::Apply(a) = lowered {
            assert!(matches!(
                &a.args[1],
                IRNode::Apply(inner) if matches!(&inner.head, IRNode::Symbol(s) if s == IF)
            ));
        }
    }

    // --- `if` (MA08 §3) ------------------------------------------------------

    #[test]
    fn if_then_else_lowers_to_three_arg_if() {
        assert_eq!(
            lower_one("if a > b then a else b;\n"),
            apply(
                sym(IF),
                vec![
                    apply(sym(GREATER), vec![sym("a"), sym("b")]),
                    sym("a"),
                    sym("b"),
                ]
            )
        );
    }

    #[test]
    fn if_then_with_no_else_lowers_to_two_arg_if() {
        assert_eq!(
            lower_one("if a then b;\n"),
            apply(sym(IF), vec![sym("a"), sym("b")])
        );
    }

    #[test]
    fn dangling_else_attaches_to_the_nearest_if() {
        // if a then if b then c else d
        //   -> If(a, If(b, c, d))          -- NOT If(If(a, If(b,c)), d)
        assert_eq!(
            lower_one("if a then if b then c else d;\n"),
            apply(
                sym(IF),
                vec![sym("a"), apply(sym(IF), vec![sym("b"), sym("c"), sym("d")])]
            )
        );
    }

    // --- Group statement `<< ... >>` (MA08 §3) ------------------------------

    #[test]
    fn group_statement_lowers_to_compound_expression() {
        assert_eq!(
            lower_one("<< a := 1; a + 1 >>;\n"),
            apply(
                sym(COMPOUND_EXPRESSION),
                vec![
                    apply(sym(ASSIGN), vec![sym("a"), int(1)]),
                    apply(sym(ADD), vec![sym("a"), int(1)]),
                ]
            )
        );
    }

    #[test]
    fn group_statement_with_a_single_statement_lowers() {
        assert_eq!(
            lower_one("<< a + 1 >>;\n"),
            apply(
                sym(COMPOUND_EXPRESSION),
                vec![apply(sym(ADD), vec![sym("a"), int(1)])]
            )
        );
    }

    #[test]
    fn group_statement_is_usable_as_an_assignment_rhs() {
        let lowered = lower_one("x := << a := 1; a + 1 >>;\n");
        assert!(matches!(
            &lowered,
            IRNode::Apply(a) if matches!(&a.head, IRNode::Symbol(s) if s == ASSIGN)
        ));
    }

    // --- Lists (MA08 §3) -----------------------------------------------------

    #[test]
    fn brace_list_literal_lowers_to_list() {
        assert_eq!(
            lower_one("{a, b, c};\n"),
            apply(sym(LIST), vec![sym("a"), sym("b"), sym("c")])
        );
    }

    #[test]
    fn empty_brace_list_literal_lowers_to_empty_list() {
        assert_eq!(lower_one("{};\n"), apply(sym(LIST), vec![]));
    }

    #[test]
    fn list_function_call_spelling_lowers_the_same_as_braces() {
        assert_eq!(
            lower_one("list(a, b, c);\n"),
            apply(sym(LIST), vec![sym("a"), sym("b"), sym("c")])
        );
    }

    #[test]
    fn list_accessor_and_constructor_calls_bridge_to_canonical_heads() {
        assert_eq!(lower_one("first(l);\n"), apply(sym(FIRST), vec![sym("l")]));
        assert_eq!(
            lower_one("second(l);\n"),
            apply(sym(SECOND), vec![sym("l")])
        );
        assert_eq!(lower_one("third(l);\n"), apply(sym(THIRD), vec![sym("l")]));
        assert_eq!(lower_one("rest(l);\n"), apply(sym(REST), vec![sym("l")]));
        assert_eq!(
            lower_one("part(l, n);\n"),
            apply(sym(PART), vec![sym("l"), sym("n")])
        );
        assert_eq!(
            lower_one("append(l1, l2);\n"),
            apply(sym(APPEND), vec![sym("l1"), sym("l2")])
        );
        assert_eq!(
            lower_one("reverse(l);\n"),
            apply(sym(REVERSE), vec![sym("l")])
        );
    }

    // --- Cons (MA08 §3) ------------------------------------------------------

    #[test]
    fn cons_onto_a_literal_list_folds_into_one_list() {
        // a . {b, c}  ->  List(a, b, c) -- NOT Cons(a, List(b, c)).
        assert_eq!(
            lower_one("a . {b, c};\n"),
            apply(sym(LIST), vec![sym("a"), sym("b"), sym("c")])
        );
    }

    #[test]
    fn cons_is_right_associative_and_folds_through_every_link() {
        // a . b . {c}  ->  a . (b . {c}) -> a . List(b, c) -> List(a, b, c)
        assert_eq!(
            lower_one("a . b . {c};\n"),
            apply(sym(LIST), vec![sym("a"), sym("b"), sym("c")])
        );
    }

    #[test]
    fn cons_onto_a_non_list_lowers_to_a_bare_cons_head() {
        // a . b  (b not structurally a literal list) -> Cons(a, b) -- a
        // disclosed, documented gap (see the module doc comment); this does
        // not crash, it just cannot be folded away at lowering time.
        assert_eq!(
            lower_one("a . b;\n"),
            apply(sym(CONS), vec![sym("a"), sym("b")])
        );
    }

    #[test]
    fn cons_binds_looser_than_additive_but_tighter_than_comparison() {
        // 1+2 . {3,4} = 4  ->  Equal(List(Add(1,2), 3, 4), 4)
        assert_eq!(
            lower_one("1+2 . {3,4} = 4;\n"),
            apply(
                sym(EQUAL),
                vec![
                    apply(
                        sym(LIST),
                        vec![apply(sym(ADD), vec![int(1), int(2)]), int(3), int(4)]
                    ),
                    int(4),
                ]
            )
        );
    }

    // --- Multi-statement programs -------------------------------------------

    #[test]
    fn multi_statement_program_lowers_each_line() {
        let ast = parse_reduce("x := 1; y := 2; x + y;\n");
        let stmts = lower_program(&ast).expect("lowering failed");
        assert_eq!(stmts.len(), 3);
        assert_eq!(stmts[0], apply(sym(ASSIGN), vec![sym("x"), int(1)]));
        assert_eq!(stmts[1], apply(sym(ASSIGN), vec![sym("y"), int(2)]));
        assert_eq!(stmts[2], apply(sym(ADD), vec![sym("x"), sym("y")]));
    }

    #[test]
    fn semi_and_dollar_terminated_statements_both_lower() {
        let ast = parse_reduce("x := 1$ y := 2;\n");
        let stmts = lower_program(&ast).expect("lowering failed");
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn a_small_reduce_program_lowers_end_to_end() {
        let ast = parse_reduce("h(x) := x*x; h(5);\n");
        let stmts = lower_program(&ast).expect("lowering failed");
        assert_eq!(stmts.len(), 2);
        assert!(matches!(
            &stmts[0],
            IRNode::Apply(a) if matches!(&a.head, IRNode::Symbol(s) if s == DEFINE)
        ));
        assert_eq!(stmts[1], apply(sym("h"), vec![int(5)]));
    }
}
