//! # Lowering — Wolfram M-expression AST → `symbolic-ir`
//!
//! The W-3 [`wolfram-parser`](coding_adventures_wolfram_parser) hands us a
//! *generic* [`GrammarASTNode`] tree whose `rule_name`s mirror the grammar
//! (`assignment`, `replaceall`, `rule`, `additive`, `multiplicative`, `power`,
//! `postfix`, `atom`, `list`, …). The parser deliberately does **no** semantic
//! work — it just records which rule matched. This module is where Wolfram's
//! *meaning* is assigned: every surface construct is **desugared** into the
//! canonical [`symbolic_ir::IRNode`] head the [`symbolic-vm`](symbolic_vm)
//! already knows how to evaluate.
//!
//! ## "Everything is `head[args]`"
//!
//! Wolfram's defining idea (MA04 §1) is that *every* fragment is one expression
//! tree `head[arg, …]`: `2 + 3` is `Plus[2, 3]`, `{1, 2}` is `List[1, 2]`,
//! `x = 5` is `Set[x, 5]`. That maps directly onto `IRNode::Apply { head, args }`.
//! So lowering is, essentially, turning the surface operators back into their
//! heads.
//!
//! ## The head-name bridge
//!
//! The one subtlety (MA04 §7.1): the **surface** head names are not the **IR**
//! head names. The VM's handler table is keyed on `Add`/`Sub`/`Mul`/`Div`/`Pow`/
//! `Neg` (the `symbolic-ir` constants), but Wolfram speaks `Plus`/`Subtract`/
//! `Times`/`Divide`/`Power`/`Minus`. We bridge them in **both** directions of
//! entry, so the infix form and the explicit head-application form collapse to
//! the same IR:
//!
//! ```text
//!   1 + 2          ─┐
//!                   ├─► Add(1, 2)  ─► VM ─► 3
//!   Plus[1, 2]     ─┘
//! ```
//!
//! | Surface        | IR head | | Surface         | IR head |
//! |----------------|---------|-|-----------------|---------|
//! | `+` `Plus`     | `Add`   | | `==` `Equal`    | `Equal` |
//! | `-` `Subtract` | `Sub`   | | `<` `Less`      | `Less`  |
//! | `*` `Times`    | `Mul`   | | `&&` `And`      | `And`   |
//! | `/` `Divide`   | `Div`   | | `\|\|` `Or`     | `Or`    |
//! | `^` `Power`    | `Pow`   | | `!` `Not`       | `Not`   |
//! | unary `-`      | `Neg`   | | `{…}` `List`    | `List`  |
//! | `=` `Set`      | `Assign`| | `:=` `SetDelayed`| `Define`|
//!
//! `Sin`/`Cos`/`Exp`/`Log`/`Sqrt`/… are already the IR's own head names, so they
//! pass through untouched. Any *other* `f[…]` (an unknown head) also passes
//! through — Mathematica leaves `f[x]` unevaluated when `f` has no definition,
//! and so does the `SymbolicBackend`.
//!
//! ## Patterns and rules
//!
//! `_`, `x_`, `_h`, `x_h` lower to the exact node shapes
//! [`cas-pattern-matching`](cas_pattern_matching) expects — `Blank()`,
//! `Pattern(x, Blank())`, `Blank(h)`, `Pattern(x, Blank(h))` — and `a -> b` /
//! `a :> b` to `Rule`/`RuleDelayed`. `expr /. rules` is *not* lowered to a VM
//! head (the VM has no `ReplaceAll`); instead the runtime recognises a
//! `ReplaceAll` apply and dispatches it through `cas_pattern_matching::rewrite`.

use cas_pattern_matching::nodes::{
    BLANK, PATTERN, RULE as PM_RULE, RULE_DELAYED as PM_RULE_DELAYED,
};
use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use symbolic_ir::{
    apply, flt, int, str_node, sym, IRNode, ADD, AND, DEFINE, DIV, EQUAL, GREATER, GREATER_EQUAL,
    LESS, LESS_EQUAL, LIST, MUL, NEG, NOT, NOT_EQUAL, OR, POW, SUB,
};

/// The synthetic head the runtime uses to mark a `/.` application. The VM has no
/// `ReplaceAll` handler, so the runtime intercepts this head *before* evaluation
/// and runs `cas_pattern_matching::rewrite` instead. Held conceptually, but we
/// never hand it to the VM, so it needs no entry in the handler/held tables.
pub const REPLACE_ALL: &str = "ReplaceAll";

/// A failure while lowering the surface tree to IR. These are *structural*
/// errors — a node shape the lowering did not expect — not user syntax errors
/// (those are caught earlier by the parser). In practice they should never fire
/// for any tree the W-3 grammar can actually produce; they exist so a malformed
/// or future-grammar node fails loudly with a message rather than panicking.
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
/// The W-3 grammar's root is `program = { statement_line }`, where each
/// `statement_line` wraps a `statement` (plus an optional `NEWLINE`/`SEMI`
/// terminator) or is terminator-only (a blank line). We keep only the lines that
/// actually carry a `statement` and lower each one; blank lines contribute
/// nothing, exactly as in a Wolfram notebook.
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
        // A statement_line is `statement (NEWLINE|SEMI) | statement | NEWLINE |
        // SEMI`; only the first two carry an inner `statement` node.
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
/// not actually apply its operator the parser still emits the level's node with a
/// single child. [`unwrap_single`] peels those away so we dispatch on the first
/// rule that genuinely shapes the tree.
fn lower_node(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    match unwrap_single(node) {
        Unwrapped::Token(token) => lower_token(token),
        Unwrapped::Node(node) => match node.rule_name.as_str() {
            "program" => Err(LowerError::new("nested program node is not an expression")),
            "statement_line" | "statement" | "expr" => lower_first_node(node),
            "assignment" => lower_assignment(node),
            "replaceall" => lower_replaceall(node),
            "rule" => lower_rule(node),
            "logical_or" => lower_logical_chain(node, OR),
            "logical_and" => lower_logical_chain(node, AND),
            "logical_not" => lower_logical_not(node),
            "comparison" => lower_comparison(node),
            "additive" | "multiplicative" => lower_binary_chain(node),
            "unary" => lower_unary(node),
            "power" => lower_power(node),
            "postfix" => lower_postfix(node),
            "atom" => lower_atom(node),
            "list" => lower_list(node),
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
        "STRING" => Ok(str_node(strip_quotes(&token.value))),
        // A lone `_` is `Blank()`. The `atom` rule for a bare blank has a single
        // BLANK token child, which `unwrap_single` peels down to here before
        // `lower_atom` ever sees it — so the canonical place to interpret a lone
        // blank is at the token level.
        "BLANK" => Ok(apply(sym(BLANK), vec![])),
        other => Err(LowerError::new(format!(
            "unexpected token `{other}` = {:?}",
            token.value
        ))),
    }
}

/// Parse a `NUMBER` lexeme into an `Integer` or `Float` IR literal.
///
/// The W-2 lexer's `NUMBER` regex is `[0-9]+\.?[0-9]*([eE][+-]?[0-9]+)?`, so a
/// `.`, `e`, or `E` means it is a real; otherwise it is an integer. We reject an
/// integer that overflows `i64` (the IR's integer width) rather than silently
/// wrapping.
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

/// `assignment = replaceall [ ( SET | SETDELAYED ) assignment ]`.
///
/// `x = e` is `Set[x, e]`, lowered to the VM's `Assign` (a held head that binds
/// `x` to the evaluated `e`). `f[x_] := e` is `SetDelayed[…]`, lowered to the
/// VM's `Define` so the right-hand side is *held* and re-substituted per call.
/// When the optional operator is absent the node is a transparent wrapper.
fn lower_assignment(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let Some(op_index) = node
        .children
        .iter()
        .position(|c| as_token(c).is_some_and(|t| matches!(token_type(t), "SET" | "SETDELAYED")))
    else {
        return lower_first_node(node);
    };
    if op_index == 0 || op_index + 1 >= node.children.len() {
        return Err(LowerError::new("malformed assignment node"));
    }
    let lhs = lower_child(&node.children[op_index - 1])?;
    let rhs = lower_child(&node.children[op_index + 1])?;
    let op = token_type(as_token(&node.children[op_index]).unwrap());

    if op == "SETDELAYED" {
        // `f[x_] := body` — a function definition. Lower the LHS apply into the
        // `Define(head, List(params…), body)` record the VM's define_handler
        // expects. A bare `x := body` (LHS is a plain symbol) becomes a
        // zero-parameter Define, i.e. a held binding.
        //
        // The VM's `apply_user_function` binds parameters by *plain symbol name*,
        // so a Wolfram parameter `x_` (which lowers to `Pattern(x, Blank())`) is
        // reduced here to the bare bound symbol `x`. The blank's optional type
        // constraint (`x_Integer`) is dropped — the simple substitution VM does
        // not enforce argument types — which is the honest subset behaviour
        // (MA04 §4). A non-pattern parameter is passed through unchanged.
        if let IRNode::Apply(app) = &lhs {
            if matches!(&app.head, IRNode::Symbol(_)) {
                let params: Vec<IRNode> = app.args.iter().map(param_binding_symbol).collect();
                return Ok(apply(
                    sym(DEFINE),
                    vec![app.head.clone(), apply(sym(LIST), params), rhs],
                ));
            }
        }
        return Ok(apply(sym(DEFINE), vec![lhs, apply(sym(LIST), vec![]), rhs]));
    }
    // `x = e` — immediate assignment.
    Ok(assign(lhs, rhs))
}

/// Build a `Set`/`Assign` apply. Factored out so the pretty-printer side has a
/// single definition of the assignment head to mirror.
fn assign(lhs: IRNode, rhs: IRNode) -> IRNode {
    apply(sym(symbolic_ir::ASSIGN), vec![lhs, rhs])
}

/// Reduce a `SetDelayed` LHS parameter to the bare symbol the VM binds against.
///
/// A Wolfram parameter is written `x_` (`Pattern(x, Blank())`) or `x_h`
/// (`Pattern(x, Blank(h))`); the VM's `apply_user_function` binds by plain symbol
/// name, so we extract the captured name `x`. A parameter that is already a plain
/// symbol (or anything else) is returned unchanged.
fn param_binding_symbol(param: &IRNode) -> IRNode {
    if let IRNode::Apply(app) = param {
        if let IRNode::Symbol(head) = &app.head {
            if head == PATTERN && !app.args.is_empty() {
                // Pattern(name, _) — bind by `name`.
                if let IRNode::Symbol(name) = &app.args[0] {
                    return sym(name);
                }
            }
        }
    }
    param.clone()
}

/// `replaceall = rule { REPLACEALL rule }` — left-associative `/.`.
///
/// `e /. r1 /. r2` is `ReplaceAll[ReplaceAll[e, r1], r2]`. We fold left into the
/// synthetic [`REPLACE_ALL`] head; the runtime intercepts it and runs
/// `cas_pattern_matching::rewrite` rather than handing it to the VM.
fn lower_replaceall(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let mut operands = child_nodes(node);
    let first = operands
        .next()
        .ok_or_else(|| LowerError::new("empty replaceall node"))?;
    let mut result = lower_node(first)?;
    for rhs in operands {
        result = apply(sym(REPLACE_ALL), vec![result, lower_node(rhs)?]);
    }
    Ok(result)
}

/// `rule = logical_or [ ( RULE | RULEDELAYED ) rule ]` — right-associative.
///
/// `a -> b` is `Rule[a, b]`, `a :> b` is `RuleDelayed[a, b]` (the
/// `cas-pattern-matching` node shapes). Because the grammar recurses on the RHS,
/// the natural recursive lowering is already right-associative:
/// `a -> b -> c` ⇒ `Rule[a, Rule[b, c]]`.
fn lower_rule(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let Some(op_index) = node
        .children
        .iter()
        .position(|c| as_token(c).is_some_and(|t| matches!(token_type(t), "RULE" | "RULEDELAYED")))
    else {
        return lower_first_node(node);
    };
    if op_index == 0 || op_index + 1 >= node.children.len() {
        return Err(LowerError::new("malformed rule node"));
    }
    let lhs = lower_child(&node.children[op_index - 1])?;
    let rhs = lower_child(&node.children[op_index + 1])?;
    let head = match token_type(as_token(&node.children[op_index]).unwrap()) {
        "RULE" => PM_RULE,
        _ => PM_RULE_DELAYED,
    };
    // Wolfram binds a pattern name on the LHS (`t_`) and refers to it as a *bare*
    // symbol on the RHS (`-> t`). `cas-pattern-matching`'s `substitute`, however,
    // only fills in `Pattern(name, …)` reference nodes — a bare `Symbol("t")` is
    // left as-is. So we rewrite every RHS occurrence of a name bound on the LHS
    // into the `Pattern(name, Blank())` reference form the matcher understands.
    let bound = collect_pattern_names(&lhs);
    let rhs = bind_pattern_refs(rhs, &bound);
    Ok(apply(sym(head), vec![lhs, rhs]))
}

/// Gather the names captured by `Pattern(name, …)` nodes anywhere in `node`.
fn collect_pattern_names(node: &IRNode) -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();
    fn walk(node: &IRNode, names: &mut std::collections::HashSet<String>) {
        if let IRNode::Apply(app) = node {
            if let IRNode::Symbol(head) = &app.head {
                if head == PATTERN && !app.args.is_empty() {
                    if let IRNode::Symbol(name) = &app.args[0] {
                        names.insert(name.clone());
                    }
                }
            }
            walk(&app.head, names);
            for arg in &app.args {
                walk(arg, names);
            }
        }
    }
    walk(node, &mut names);
    names
}

/// Rewrite bare `Symbol(name)` references that are bound LHS pattern names into
/// `Pattern(name, Blank())` reference nodes, so `cas-pattern-matching::substitute`
/// fills them in. Symbols not in `bound` (and all literals) pass through.
fn bind_pattern_refs(node: IRNode, bound: &std::collections::HashSet<String>) -> IRNode {
    match node {
        IRNode::Symbol(name) if bound.contains(&name) => {
            apply(sym(PATTERN), vec![sym(name), apply(sym(BLANK), vec![])])
        }
        IRNode::Apply(app) => {
            let head = bind_pattern_refs(app.head, bound);
            let args = app
                .args
                .into_iter()
                .map(|a| bind_pattern_refs(a, bound))
                .collect();
            apply(head, args)
        }
        other => other,
    }
}

/// `logical_or`/`logical_and` — fold the operands into an n-ary `Or`/`And`. A
/// single operand (no operator present) is a transparent wrapper.
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

/// `logical_not = NOT logical_not | comparison`. A leading `!` wraps the operand
/// in `Not`; otherwise it is the inner comparison.
fn lower_logical_not(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let has_not = node
        .children
        .iter()
        .any(|c| as_token(c).is_some_and(|t| token_type(t) == "NOT"));
    if !has_not {
        return lower_first_node(node);
    }
    let inner = child_nodes(node)
        .next()
        .ok_or_else(|| LowerError::new("`!` with no operand"))?;
    Ok(apply(sym(NOT), vec![lower_node(inner)?]))
}

/// `comparison = additive [ op additive ]` — a single (non-chained) comparison.
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

/// `additive`/`multiplicative` — a left-associative chain of `+`/`-` or `*`/`/`.
///
/// `a - b - c` folds left into `Sub(Sub(a, b), c)`; `a / b` into `Div(a, b)`. The
/// VM's handlers simplify these (`Sub(x, 0)` → `x`, numeric folds, …).
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

/// `unary = ( MINUS | PLUS ) unary | power`. A leading `-` is `Neg`; a leading
/// `+` is a no-op (`+x` is `x`); no prefix means the inner `power`.
fn lower_unary(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    if node.children.len() == 1 {
        return lower_child(&node.children[0]);
    }
    let op = token_type(
        as_token(&node.children[0]).ok_or_else(|| LowerError::new("unary op must be a token"))?,
    );
    let operand = lower_child(
        node.children
            .get(1)
            .ok_or_else(|| LowerError::new("unary op with no operand"))?,
    )?;
    if op == "MINUS" {
        Ok(apply(sym(NEG), vec![operand]))
    } else {
        Ok(operand) // unary plus
    }
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

/// `postfix = atom { LBRACKET [ arglist ] RBRACKET }` — function application,
/// left-associative and chainable (`f[x][y]` is `(f[x])[y]`).
///
/// The head is the lowered base; the args are the lowered `arglist`. Crucially
/// we run the head through [`canonical_head`] so a built-in surface head like
/// `Plus[…]` or `Sin[…]` becomes the IR head (`Add`, `Sin`) the VM dispatches —
/// the *same* head the infix `+` lowers to (MA04 §7.1).
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
        if token_type(token) != "LBRACKET" {
            i += 1;
            continue;
        }
        // The next child is the optional `arglist` node (absent for `f[]`).
        let args = node
            .children
            .get(i + 1)
            .and_then(as_node)
            .filter(|n| n.rule_name == "arglist")
            .map(lower_arglist)
            .transpose()?
            .unwrap_or_default();
        result = build_application(result, args);
        i += 1;
    }
    Ok(result)
}

/// Apply `head` to `args`, bridging built-in surface heads to their canonical IR
/// head. `Sin[x]` → `Sin(x)`, `f[x]` → `f(x)`.
///
/// For the *associative* arithmetic/logic heads (`Add`/`Mul`/`And`/`Or`) we
/// additionally **left-fold** a 3-or-more-argument application into a binary
/// chain — `Plus[1, 2, 3]` → `Add(Add(1, 2), 3)`. This matters because the VM's
/// handlers fold *binary* `Add`/`Mul` numerically (and flatten nested ones) but
/// leave a direct n-ary `Add(1, 2, 3)` untouched; folding here makes the explicit
/// head-application `Plus[1, 2, 3]` evaluate identically to the infix `1 + 2 + 3`
/// (MA04 §7.1).
fn build_application(head: IRNode, args: Vec<IRNode>) -> IRNode {
    let canonical = canonical_head(head);
    if let IRNode::Symbol(name) = &canonical {
        if matches!(name.as_str(), ADD | MUL | AND | OR) && args.len() > 2 {
            let mut iter = args.into_iter();
            let mut acc = iter.next().unwrap();
            for next in iter {
                acc = apply(sym(name.clone()), vec![acc, next]);
            }
            return acc;
        }
    }
    apply(canonical, args)
}

/// `arglist = expr { COMMA expr }` — lower each comma-separated argument.
fn lower_arglist(node: &GrammarASTNode) -> Result<Vec<IRNode>, LowerError> {
    child_nodes(node).map(lower_node).collect()
}

/// `atom` — one of: NUMBER, STRING, a symbol, a pattern blank, a list, or a
/// parenthesised group. The pattern-blank forms (`x_`, `_h`, `x_h`) are the only
/// atoms that combine multiple tokens; everything else delegates.
///
/// `atom = NUMBER | STRING | NAME [ BLANK [ NAME ] ] | BLANK [ NAME ] | list | group`
fn lower_atom(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    // Sub-rule (list / group): delegate.
    if let Some(child) = child_nodes(node).next() {
        if matches!(child.rule_name.as_str(), "list" | "group") {
            return lower_node(child);
        }
    }

    // Otherwise the atom is a run of tokens. Read them in order. The blank/
    // pattern arms must come *before* the lone-token arm, because a lone `_` is a
    // single BLANK token that means `Blank()`, not a literal/symbol.
    let tokens: Vec<&Token> = node.children.iter().filter_map(as_token).collect();
    match tokens.as_slice() {
        // `_`  → Blank()        ;  `_h` → Blank(h)
        [b, rest @ ..] if token_type(b) == "BLANK" => Ok(lower_blank(rest)),
        // `x_` → Pattern(x, Blank()) ; `x_h` → Pattern(x, Blank(h))
        [name, b, rest @ ..] if token_type(name) == "NAME" && token_type(b) == "BLANK" => {
            let inner = lower_blank(rest);
            Ok(apply(sym(PATTERN), vec![sym(&name.value), inner]))
        }
        // A lone literal or bare symbol.
        [single] => lower_token(single),
        _ => Err(LowerError::new(format!(
            "unrecognised atom token shape: {:?}",
            tokens.iter().map(|t| &t.value).collect::<Vec<_>>()
        ))),
    }
}

/// Build a `Blank()` / `Blank(h)` from the (possibly empty) trailing head-name
/// token of a blank pattern.
fn lower_blank(rest: &[&Token]) -> IRNode {
    match rest.first() {
        Some(head) if token_type(head) == "NAME" => apply(sym(BLANK), vec![sym(&head.value)]),
        _ => apply(sym(BLANK), vec![]),
    }
}

/// `list = LBRACE [ arglist ] RBRACE` → `List(elem…)`.
fn lower_list(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let mut args = Vec::new();
    for child in child_nodes(node) {
        if child.rule_name == "arglist" {
            args.extend(lower_arglist(child)?);
        }
    }
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

/// Lower the first *node* child (ignoring tokens). Used by transparent-wrapper
/// rules whose only meaningful content is a nested expression node.
fn lower_first_node(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let child = child_nodes(node)
        .next()
        .ok_or_else(|| LowerError::new(format!("`{}` has no expression child", node.rule_name)))?;
    lower_node(child)
}

/// Map an arithmetic/separator token type to its IR head.
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
        "EQUAL" => Some(EQUAL),
        "UNEQUAL" => Some(NOT_EQUAL),
        "LESS" => Some(LESS),
        "GREATER" => Some(GREATER),
        "LE" => Some(LESS_EQUAL),
        "GE" => Some(GREATER_EQUAL),
        _ => None,
    }
}

/// Bridge a Wolfram *surface* head to the canonical IR head for built-ins.
///
/// `Plus`/`Times`/`Power`/… are the names a user types as an explicit
/// head-application; the VM's handler table is keyed on `Add`/`Mul`/`Pow`/…. A
/// head that is already an IR head (`Sin`, `Cos`, `Log`, …) — or any *unknown*
/// head — is returned unchanged, so `Sin[x]` evaluates and `myFunc[x]` passes
/// through. Only a bare `Symbol` head is bridged; a computed head (e.g. the head
/// of `f[x][y]`) is left as-is.
fn canonical_head(head: IRNode) -> IRNode {
    if let IRNode::Symbol(name) = &head {
        if let Some(canonical) = surface_head_to_ir(name) {
            return sym(canonical);
        }
    }
    head
}

/// The surface→IR head dictionary for the operators that have *both* an infix
/// form and a long head name. Returning `None` means "not a renamed built-in" —
/// the head (whether an already-canonical `Sin` or a user symbol) stays as typed.
fn surface_head_to_ir(name: &str) -> Option<&'static str> {
    Some(match name {
        "Plus" => ADD,
        "Subtract" => SUB,
        "Times" => MUL,
        "Divide" => DIV,
        "Power" => POW,
        "Minus" => NEG,
        "Equal" => EQUAL,
        "Unequal" => NOT_EQUAL,
        "Less" => LESS,
        "Greater" => GREATER,
        "LessEqual" => LESS_EQUAL,
        "GreaterEqual" => GREATER_EQUAL,
        "And" => AND,
        "Or" => OR,
        "Not" => NOT,
        "List" => LIST,
        "Set" => symbolic_ir::ASSIGN,
        "SetDelayed" => DEFINE,
        _ => return None,
    })
}

fn token_type(token: &Token) -> &str {
    token.effective_type_name()
}

/// Strip the surrounding double-quotes from a `STRING` lexeme. The W-2 lexer
/// keeps the quotes (and raw backslash escapes) in the token value; the runtime
/// is responsible for decoding. For this subset we only remove the delimiters
/// and leave escapes as raw text (matching the lexer's `escapes: none`).
fn strip_quotes(text: &str) -> &str {
    text.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(text)
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

/// Peel away single-child wrapper nodes until we reach a node with structure (or
/// a leaf token). A precedence-cascade rule that did not apply its operator still
/// emits its own node with exactly one child — `unwrap_single` skips straight to
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
    use coding_adventures_wolfram_parser::parse_wolfram;

    /// Lower a single-statement source to its one IR node.
    fn lower_one(src: &str) -> IRNode {
        let ast = parse_wolfram(src);
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
    fn string_literal_loses_its_quotes() {
        assert_eq!(lower_one("\"hello\"\n"), str_node("hello"));
    }

    #[test]
    fn bare_symbol() {
        assert_eq!(lower_one("foo\n"), sym("foo"));
    }

    #[test]
    fn additive_lowers_to_add() {
        // 1 + 2  ->  Add(1, 2)
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
    fn unary_minus_is_neg_and_plus_is_noop() {
        assert_eq!(lower_one("-x\n"), apply(sym(NEG), vec![sym("x")]));
        assert_eq!(lower_one("+x\n"), sym("x"));
    }

    #[test]
    fn list_literal_lowers_to_list_head() {
        assert_eq!(
            lower_one("{1, 2, 3}\n"),
            apply(sym(LIST), vec![int(1), int(2), int(3)])
        );
        assert_eq!(lower_one("{}\n"), apply(sym(LIST), vec![]));
    }

    #[test]
    fn application_of_unknown_head_passes_through() {
        // f[x, y]  ->  f(x, y)
        assert_eq!(
            lower_one("f[x, y]\n"),
            apply(sym("f"), vec![sym("x"), sym("y")])
        );
        assert_eq!(lower_one("f[]\n"), apply(sym("f"), vec![]));
    }

    #[test]
    fn builtin_head_application_is_bridged_to_ir_head() {
        // Plus[1, 2, 3]  ->  Add(Add(1, 2), 3)  (left-folded so the VM evaluates
        // it identically to the infix 1 + 2 + 3).
        assert_eq!(
            lower_one("Plus[1, 2, 3]\n"),
            apply(
                sym(ADD),
                vec![apply(sym(ADD), vec![int(1), int(2)]), int(3)]
            )
        );
        // Times / Power likewise
        assert_eq!(
            lower_one("Times[a, b]\n"),
            apply(sym(MUL), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one("Power[2, 10]\n"),
            apply(sym(POW), vec![int(2), int(10)])
        );
    }

    #[test]
    fn sin_is_already_canonical() {
        // Sin[x]  ->  Sin(x)  (Sin is already an IR head — not renamed)
        assert_eq!(lower_one("Sin[x]\n"), apply(sym("Sin"), vec![sym("x")]));
    }

    #[test]
    fn nested_application_is_left_associative() {
        // f[x][y]  ->  (f(x))(y)
        assert_eq!(
            lower_one("f[x][y]\n"),
            apply(apply(sym("f"), vec![sym("x")]), vec![sym("y")])
        );
    }

    #[test]
    fn comparisons_lower_to_their_heads() {
        assert_eq!(
            lower_one("a == b\n"),
            apply(sym(EQUAL), vec![sym("a"), sym("b")])
        );
        assert_eq!(
            lower_one("a <= b\n"),
            apply(sym(LESS_EQUAL), vec![sym("a"), sym("b")])
        );
    }

    #[test]
    fn logic_chains_and_not() {
        // a && b || c  ->  Or(And(a, b), c)
        assert_eq!(
            lower_one("a && b || c\n"),
            apply(
                sym(OR),
                vec![apply(sym(AND), vec![sym("a"), sym("b")]), sym("c")]
            )
        );
        assert_eq!(lower_one("!x\n"), apply(sym(NOT), vec![sym("x")]));
    }

    #[test]
    fn pattern_blanks_lower_to_cas_node_shapes() {
        // _   ->  Blank()
        assert_eq!(lower_one("_\n"), apply(sym(BLANK), vec![]));
        // _Integer  ->  Blank(Integer)
        assert_eq!(
            lower_one("_Integer\n"),
            apply(sym(BLANK), vec![sym("Integer")])
        );
        // x_  ->  Pattern(x, Blank())
        assert_eq!(
            lower_one("x_\n"),
            apply(sym(PATTERN), vec![sym("x"), apply(sym(BLANK), vec![])])
        );
        // x_Integer  ->  Pattern(x, Blank(Integer))
        assert_eq!(
            lower_one("x_Integer\n"),
            apply(
                sym(PATTERN),
                vec![sym("x"), apply(sym(BLANK), vec![sym("Integer")])]
            )
        );
    }

    #[test]
    fn rules_lower_to_pattern_matching_heads() {
        // a -> b  ->  Rule(a, b)
        assert_eq!(
            lower_one("a -> b\n"),
            apply(sym(PM_RULE), vec![sym("a"), sym("b")])
        );
        // a :> b  ->  RuleDelayed(a, b)
        assert_eq!(
            lower_one("a :> b\n"),
            apply(sym(PM_RULE_DELAYED), vec![sym("a"), sym("b")])
        );
        // Right-associativity:  a -> b -> c  ->  Rule(a, Rule(b, c))
        assert_eq!(
            lower_one("a -> b -> c\n"),
            apply(
                sym(PM_RULE),
                vec![sym("a"), apply(sym(PM_RULE), vec![sym("b"), sym("c")])]
            )
        );
    }

    #[test]
    fn replaceall_uses_the_synthetic_head_left_assoc() {
        // x /. a -> b  ->  ReplaceAll(x, Rule(a, b))
        assert_eq!(
            lower_one("x /. a -> b\n"),
            apply(
                sym(REPLACE_ALL),
                vec![sym("x"), apply(sym(PM_RULE), vec![sym("a"), sym("b")])]
            )
        );
    }

    #[test]
    fn set_and_setdelayed() {
        // x = 5  ->  Assign(x, 5)
        assert_eq!(
            lower_one("x = 5\n"),
            apply(sym(symbolic_ir::ASSIGN), vec![sym("x"), int(5)])
        );
        // f[x_] := x  ->  Define(f, List(x), x)  — the param `x_` is reduced to
        // the bare bound symbol `x` for the VM's symbol-based parameter binding.
        assert_eq!(
            lower_one("f[x_] := x\n"),
            apply(
                sym(DEFINE),
                vec![sym("f"), apply(sym(LIST), vec![sym("x")]), sym("x")]
            )
        );
    }

    #[test]
    fn grouping_changes_precedence() {
        // (a + b) * c  ->  Mul(Add(a, b), c)
        assert_eq!(
            lower_one("(a + b) * c\n"),
            apply(
                sym(MUL),
                vec![apply(sym(ADD), vec![sym("a"), sym("b")]), sym("c")]
            )
        );
    }

    #[test]
    fn empty_program_lowers_to_no_statements() {
        let ast = parse_wolfram("\n");
        assert!(lower_program(&ast).unwrap().is_empty());
    }

    #[test]
    fn multiple_statements_lower_in_order() {
        let ast = parse_wolfram("1\n2\n3\n");
        assert_eq!(lower_program(&ast).unwrap(), vec![int(1), int(2), int(3)]);
    }

    #[test]
    fn non_program_root_is_rejected() {
        // A hand-built non-program node should be refused.
        let bogus = GrammarASTNode {
            rule_name: "atom".to_string(),
            children: vec![],
            start_line: None,
            start_column: None,
            end_line: None,
            end_column: None,
        };
        assert!(lower_program(&bogus).is_err());
    }

    #[test]
    fn assign_helper_builds_an_assign_apply() {
        assert_eq!(
            assign(sym("x"), int(1)),
            apply(sym(symbolic_ir::ASSIGN), vec![sym("x"), int(1)])
        );
    }
}
