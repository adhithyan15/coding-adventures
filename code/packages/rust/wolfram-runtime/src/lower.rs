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

/// The W-20 pattern-construct heads the W-21 operator sugar desugars to. Each is
/// the *exact* surface name the W-20 runtime already evaluates (see
/// `wolfram-runtime/src/builtins.rs`): `a | b` → `Alternatives[a, b]`,
/// `patt /; test` → `Condition[patt, test]`, `patt ? fn` →
/// `PatternTest[patt, fn]`, and `expr //. rules` → `ReplaceRepeated[expr, rules]`.
/// W-21 introduces NO new evaluation — these heads are reused unchanged, so an
/// operator form and its `Head[args]` long form produce identical IR. Like
/// `ReplaceAll`/`ReplaceRepeated` they are recognised by the runtime before the
/// VM sees them; they need no handler/held-table entry here.
const ALTERNATIVES_HEAD: &str = "Alternatives";
const CONDITION_HEAD: &str = "Condition";
const PATTERN_TEST_HEAD: &str = "PatternTest";
/// `expr //. rules` desugars to this head, which W-20 (§22.4) already evaluates
/// with its hard iteration + growth caps. The operator surface inherits those
/// DoS bounds verbatim — lowering to `//.` is identical to writing
/// `ReplaceRepeated[…]`.
pub const REPLACE_REPEATED: &str = "ReplaceRepeated";

/// The W-5 head names the W-6 operator sugar desugars to. `f /@ x` lowers to
/// `Map[f, x]`, `f @@ x` to `Apply[f, x]`, and `x[[i]]` to `Part[x, i]` — the
/// *exact same* heads the [`WolframBackend`](crate::backend) built-in table
/// answers (keyed on these surface names), so a sugar form and its long form
/// produce byte-identical IR and therefore evaluate identically.
const MAP_HEAD: &str = "Map";
const APPLY_HEAD: &str = "Apply";
const PART_HEAD: &str = "Part";

/// The W-11 pure-function IR heads. A `#`/`#n` slot lowers to `Slot[n]`, `##` to
/// `SlotSequence[1]`, and a pure function (either `Function[params, body]` or the
/// `body &` slot form) to a `Function` apply. These are *surface* head names the
/// runtime recognises (via the [`WolframBackend`](crate::backend) rewrite rule)
/// and resolves by substitution when the function is applied — they are never
/// handed to the shared VM handler table, so they need no entry there.
pub(crate) const SLOT_HEAD: &str = "Slot";
pub(crate) const SLOT_SEQUENCE_HEAD: &str = "SlotSequence";
pub(crate) const FUNCTION_HEAD: &str = "Function";

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
            "condition" => lower_condition(node),
            "alternatives" => lower_alternatives(node),
            "patterntest" => lower_patterntest(node),
            "logical_or" => lower_logical_chain(node, OR),
            "logical_and" => lower_logical_chain(node, AND),
            "logical_not" => lower_logical_not(node),
            "comparison" => lower_comparison(node),
            "additive" | "multiplicative" => lower_binary_chain(node),
            "unary" => lower_unary(node),
            "power" => lower_power(node),
            "amp" => lower_amp(node),
            "mapapply" => lower_mapapply(node),
            "postfix" => lower_postfix(node),
            "atom" => lower_atom(node),
            "slot" => lower_slot(node),
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
        // A bare `#` (the `slot` rule with a single HASH child) is peeled down to
        // this token by `unwrap_single` before `lower_slot` runs, so a lone slot
        // is interpreted here — `#` ≡ `#1` → `Slot[1]`. A numbered `#n` keeps two
        // tokens (HASH + NUMBER), so it is NOT unwrapped and reaches `lower_slot`.
        "HASH" => Ok(apply(sym(SLOT_HEAD), vec![int(1)])),
        // Likewise a lone `##` → `SlotSequence[1]`.
        "SLOTSEQ" => Ok(apply(sym(SLOT_SEQUENCE_HEAD), vec![int(1)])),
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

/// `replaceall = rule { ( REPLACEALL | REPLACEREPEATED ) rule }` —
/// left-associative `/.` *and* (W-21) `//.`.
///
/// `e /. r1 /. r2` is `ReplaceAll[ReplaceAll[e, r1], r2]`; `e //. r` is
/// `ReplaceRepeated[e, r]` (W-21). Both operators share this one precedence level
/// and fold strictly left, so a mixed chain `e /. a //. b` is
/// `ReplaceRepeated[ReplaceAll[e, a], b]`. We must therefore walk the children
/// *including the operator tokens* (not just `child_nodes`, which drops them) to
/// pick `REPLACE_ALL` vs `REPLACE_REPEATED` per step. The runtime intercepts both
/// synthetic heads before the VM: `/.` runs `cas_pattern_matching::rewrite` once,
/// `//.` (§22.4) iterates to a fixed point under its hard iteration + growth caps.
fn lower_replaceall(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    // Fast path: a transparent single-operand wrapper (no replace operator).
    if node.children.len() == 1 {
        return lower_child(&node.children[0]);
    }
    let mut children = node.children.iter();
    let first = children
        .next()
        .ok_or_else(|| LowerError::new("empty replaceall node"))?;
    let mut result = lower_child(first)?;
    while let Some(op_child) = children.next() {
        let head = match as_token(op_child).map(token_type) {
            Some("REPLACEALL") => REPLACE_ALL,
            Some("REPLACEREPEATED") => REPLACE_REPEATED,
            _ => return Err(LowerError::new("expected a `/.` or `//.` operator")),
        };
        let rhs = children
            .next()
            .ok_or_else(|| LowerError::new("`/.`/`//.` with no right operand"))?;
        result = apply(sym(head), vec![result, lower_child(rhs)?]);
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

/// `condition = alternatives [ CONDITION condition ]` — the W-21 `/;` operator.
///
/// `patt /; test` lowers to `Condition[patt, test]`, the W-20 head §22.3 already
/// evaluates. Right-associative (the grammar recurses on the RHS), so the rare
/// nested `a /; b /; c` is `Condition[a, Condition[b, c]]`.
///
/// Crucially — UNLIKE `lower_rule` — the test keeps its **bare** named-symbol
/// references. The W-20 `Condition` handler substitutes the match's named
/// bindings (`Symbol("x")`) into the test before evaluating it (§22.3,
/// `substitute_bound_symbols`), so `x_ /; x > 2` must lower to
/// `Condition[Pattern[x, Blank[]], Greater[x, 2]]` with a *bare* `x` in the test.
/// We therefore do NOT run `bind_pattern_refs` on the test.
fn lower_condition(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let Some(op_index) = node
        .children
        .iter()
        .position(|c| as_token(c).is_some_and(|t| token_type(t) == "CONDITION"))
    else {
        // No `/;` — a transparent wrapper over the single `alternatives` operand.
        return lower_first_node(node);
    };
    if op_index == 0 || op_index + 1 >= node.children.len() {
        return Err(LowerError::new("malformed condition node"));
    }
    let patt = lower_child(&node.children[op_index - 1])?;
    let test = lower_child(&node.children[op_index + 1])?;
    Ok(apply(sym(CONDITION_HEAD), vec![patt, test]))
}

/// `alternatives = logical_or { ALTERNATIVES logical_or }` — the W-21 `|` operator.
///
/// `a | b | c` lowers to the single n-ary `Alternatives[a, b, c]` the W-20 head
/// §22.2 evaluates (first alternative that matches wins). Like `+`/`&&` the whole
/// run folds into ONE flat head, not nested binary applies; a lone operand (no
/// `|`) passes straight through as a transparent wrapper.
fn lower_alternatives(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let operands = child_nodes(node)
        .map(lower_node)
        .collect::<Result<Vec<_>, _>>()?;
    match operands.len() {
        0 => Err(LowerError::new("empty alternatives node")),
        1 => Ok(operands.into_iter().next().unwrap()),
        _ => Ok(apply(sym(ALTERNATIVES_HEAD), operands)),
    }
}

/// `patterntest = postfix { PATTERNTEST postfix }` — the W-21 `?` operator.
///
/// `patt ? fn` lowers to `PatternTest[patt, fn]`, the W-20 head §22.3 evaluates
/// (`fn[subject]` must be `True`). Infix, left-associative, so a chain
/// `_?IntegerQ?Positive` folds left into
/// `PatternTest[PatternTest[Blank[], IntegerQ], Positive]`. A lone `postfix`
/// (no `?`) is a transparent single-child wrapper.
fn lower_patterntest(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    // Fast path: no `?` present — a transparent wrapper over one operand.
    if node.children.len() == 1 {
        return lower_child(&node.children[0]);
    }
    let mut children = node.children.iter();
    let first = children
        .next()
        .ok_or_else(|| LowerError::new("empty patterntest node"))?;
    let mut result = lower_child(first)?;
    while let Some(op_child) = children.next() {
        if as_token(op_child).map(token_type) != Some("PATTERNTEST") {
            return Err(LowerError::new("expected a `?` operator"));
        }
        let rhs = children
            .next()
            .ok_or_else(|| LowerError::new("`?` with no right operand"))?;
        result = apply(sym(PATTERN_TEST_HEAD), vec![result, lower_child(rhs)?]);
    }
    Ok(result)
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

/// `mapapply = postfix { ( MAP | APPLY ) postfix }` — the W-6 `/@` / `@@`
/// operator sugar, infix and left-associative.
///
/// `f /@ x` lowers to `Map[f, x]` and `f @@ x` to `Apply[f, x]` — the same
/// W-5 heads (`Map`/`Apply`) the long forms produce, so `f /@ {1, 2}` is IR-
/// identical to `Map[f, {1, 2}]` (MA04 §9). `/@` and `@@` share one
/// left-associative precedence level, so a mixed chain folds strictly left:
/// `g @@ f /@ x` ⇒ `(g @@ f) /@ x` ⇒ `Map[Apply[g, f], x]` (parenthesise when
/// mixing — the subset does not give `@@` Wolfram's higher precedence). When no
/// operator is present the rule is a transparent single-child wrapper, handled
/// by the fast path.
fn lower_mapapply(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    // Fast path: no `/@`/`@@` present — a transparent wrapper over one operand.
    if node.children.len() == 1 {
        return lower_child(&node.children[0]);
    }
    let mut children = node.children.iter();
    let first = children
        .next()
        .ok_or_else(|| LowerError::new("empty mapapply node"))?;
    let mut result = lower_child(first)?;
    while let Some(op_child) = children.next() {
        let head = match as_token(op_child).map(token_type) {
            Some("MAP") => MAP_HEAD,
            Some("APPLY") => APPLY_HEAD,
            _ => return Err(LowerError::new("expected a `/@` or `@@` operator")),
        };
        let rhs = children
            .next()
            .ok_or_else(|| LowerError::new("`/@`/`@@` with no right operand"))?;
        // Map[f, x] / Apply[f, x] — the W-5 built-in heads, NOT bridged through
        // `canonical_head` (they are not arithmetic), so they reach the
        // WolframBackend handler table verbatim.
        result = apply(sym(head), vec![result, lower_child(rhs)?]);
    }
    Ok(result)
}

/// `postfix = atom { LBRACKET [ arglist ] RBRACKET | LDBRACKET arglist RBRACKET RBRACKET }`
/// — function application *and* the W-6 `[[ … ]]` part sugar, both postfix,
/// left-associative and chainable (`f[x][y]` is `(f[x])[y]`,
/// `x[[1]][[2]]` is `Part[Part[x, 1], 2]`).
///
/// For `f[…]` the head is the lowered base and the args the lowered `arglist`;
/// we run the head through [`canonical_head`] so a built-in surface head like
/// `Plus[…]` or `Sin[…]` becomes the IR head (`Add`, `Sin`) the VM dispatches —
/// the *same* head the infix `+` lowers to (MA04 §7.1).
///
/// For `x[[i]]` we emit `Part[x, i]` — the W-5 `Part` head (MA04 §9). A
/// multi-index `x[[i, j]]` is folded into nested parts `Part[Part[x, i], j]`
/// (Wolfram's `Part[x, i, j]` semantics), one `Part` per index, so it reuses the
/// single-index `Part` handler unchanged.
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
        match token_type(token) {
            "LBRACKET" => {
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
            }
            "LDBRACKET" => {
                // `[[ arglist ]]` — the grammar guarantees a non-empty `arglist`
                // (a bare `x[[]]` does not parse). Fold each index into a nested
                // `Part`, so `x[[i, j]]` becomes `Part[Part[x, i], j]`.
                let indices = node
                    .children
                    .get(i + 1)
                    .and_then(as_node)
                    .filter(|n| n.rule_name == "arglist")
                    .map(lower_arglist)
                    .transpose()?
                    .unwrap_or_default();
                for index in indices {
                    result = apply(sym(PART_HEAD), vec![result, index]);
                }
            }
            _ => {}
        }
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
        // W-11: normalise the *named* long form `Function[p, body]` so its
        // parameter list is always a `List`. `Function[x, body]` (a single
        // symbol param) becomes `Function[List[x], body]`; `Function[{x, y},
        // body]` already has a `List` first arg and is left as-is. The
        // slot-based one-arg form `Function[body]` is also left as-is (it has no
        // parameter list — its body refers to slots). This single normalisation
        // lets the backend application rule treat every named Function uniformly
        // as `Function[List(params…), body]`.
        if name == FUNCTION_HEAD && args.len() == 2 && !is_list_node(&args[0]) {
            let mut it = args.into_iter();
            let param = it.next().unwrap();
            let body = it.next().unwrap();
            return apply(sym(FUNCTION_HEAD), vec![apply(sym(LIST), vec![param]), body]);
        }
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
    // Sub-rule (list / group / slot): delegate.
    if let Some(child) = child_nodes(node).next() {
        if matches!(child.rule_name.as_str(), "list" | "group" | "slot") {
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

/// `slot = HASH [ NUMBER ] | SLOTSEQ` — a pure-function argument slot (W-11).
///
/// | Surface | IR                  | meaning                          |
/// |---------|---------------------|----------------------------------|
/// | `#`     | `Slot[1]`           | the first argument (`#` ≡ `#1`)  |
/// | `#n`    | `Slot[n]`           | the n-th argument                |
/// | `##`    | `SlotSequence[1]`   | all arguments, spliced           |
///
/// A `#n` is `HASH` followed by the ordinary `NUMBER` token (there is no
/// dedicated slot-number token), so we read the optional number here. A bare
/// `#` defaults to slot 1. `##` carries no number in this subset (real Wolfram's
/// `##n` is out of scope) and lowers to `SlotSequence[1]`.
fn lower_slot(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let tokens: Vec<&Token> = node.children.iter().filter_map(as_token).collect();
    match tokens.as_slice() {
        // `##` — SlotSequence (all args). The subset has no `##n`.
        [s] if token_type(s) == "SLOTSEQ" => Ok(apply(sym(SLOT_SEQUENCE_HEAD), vec![int(1)])),
        // `#` — the first slot.
        [h] if token_type(h) == "HASH" => Ok(apply(sym(SLOT_HEAD), vec![int(1)])),
        // `#n` — the n-th slot. The number must be a positive integer.
        [h, n] if token_type(h) == "HASH" && token_type(n) == "NUMBER" => {
            let idx = n
                .value
                .parse::<i64>()
                .map_err(|e| LowerError::new(format!("invalid slot number {:?}: {e}", n.value)))?;
            if idx < 1 {
                return Err(LowerError::new(format!(
                    "slot number must be >= 1, got {idx}"
                )));
            }
            Ok(apply(sym(SLOT_HEAD), vec![int(idx)]))
        }
        _ => Err(LowerError::new(format!(
            "unrecognised slot token shape: {:?}",
            tokens.iter().map(|t| &t.value).collect::<Vec<_>>()
        ))),
    }
}

/// `amp = power AMP { AMP } { amp_apply } | power` — the W-11 `&` pure-function
/// postfix (and the optional immediate application that may follow it).
///
/// With no `&` present the rule is a transparent single-child wrapper over
/// `power`. With one or more trailing `&`, each `&` wraps the running expression
/// into a slot-based `Function[body]` (so `expr & &` wraps twice). Then each
/// trailing `amp_apply` (`[args]` or `[[i]]`) applies the resulting function —
/// `(#^2)&[5]` is `(Function[#^2])[5]`, reusing the same `build_application` /
/// `Part` lowering ordinary postfix application uses, so the backend's
/// `Function`-application rewrite rule fires on it at eval time.
fn lower_amp(node: &GrammarASTNode) -> Result<IRNode, LowerError> {
    // Fast path: no `&` — a transparent wrapper over one `power` child.
    let amp_count = node
        .children
        .iter()
        .filter(|c| as_token(c).is_some_and(|t| token_type(t) == "AMP"))
        .count();
    if amp_count == 0 {
        return lower_first_node(node);
    }

    // The body is the first (and only) `power` child, before the `&` run.
    let body = lower_first_node(node)?;

    // Wrap once per `&`: `expr &` → Function[expr]; `expr & &` → Function[Function[expr]].
    let mut result = body;
    for _ in 0..amp_count {
        result = apply(sym(FUNCTION_HEAD), vec![result]);
    }

    // Apply any trailing `amp_apply` suffixes (`[args]` / `[[i]]`) to the function.
    for suffix in child_nodes(node).filter(|n| n.rule_name == "amp_apply") {
        result = lower_amp_apply(result, suffix)?;
    }
    Ok(result)
}

/// Apply one `amp_apply` suffix to an already-built pure function.
///
/// `amp_apply = LBRACKET [ arglist ] RBRACKET | LDBRACKET arglist RBRACKET RBRACKET`
/// — the same two postfix forms `postfix` handles. `f & [a, b]` builds the
/// application `f[a, b]` (which the backend rule then resolves by substitution);
/// `f & [[i]]` folds into nested `Part`s exactly as ordinary part sugar does.
fn lower_amp_apply(func: IRNode, suffix: &GrammarASTNode) -> Result<IRNode, LowerError> {
    let is_part = suffix
        .children
        .iter()
        .any(|c| as_token(c).is_some_and(|t| token_type(t) == "LDBRACKET"));
    let args = child_nodes(suffix)
        .find(|n| n.rule_name == "arglist")
        .map(lower_arglist)
        .transpose()?
        .unwrap_or_default();
    if is_part {
        let mut result = func;
        for index in args {
            result = apply(sym(PART_HEAD), vec![result, index]);
        }
        Ok(result)
    } else {
        Ok(build_application(func, args))
    }
}

/// True if `node` is a `List(...)` apply — used to detect an already-list
/// `Function` parameter list (`Function[{x, y}, body]`) vs a single-symbol one.
fn is_list_node(node: &IRNode) -> bool {
    matches!(node, IRNode::Apply(app) if matches!(&app.head, IRNode::Symbol(s) if s == LIST))
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

/// Build a canonical application from a head and args — the W-5 runtime entry to
/// the *same* bridge + associative-fold W-4 lowering uses (see
/// [`build_application`]). The list/functional built-ins `Map`/`Apply` construct
/// a fresh `f[…]` at evaluation time and route it through this so, e.g.,
/// `Apply[Plus, {1, 2, 3}]` becomes the left-folded `Add(Add(1, 2), 3)` the VM
/// then sums to `6`, identical to the infix `1 + 2 + 3`.
pub(crate) fn build_canonical_application(head: IRNode, args: Vec<IRNode>) -> IRNode {
    build_application(head, args)
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

    // ----- W-21 pattern operator sugar lowering --------------------------------

    #[test]
    fn alternatives_operator_folds_to_one_nary_head() {
        // a | b | c  ->  Alternatives(a, b, c)  (one flat head, like + / &&)
        assert_eq!(
            lower_one("a | b | c\n"),
            apply(sym(ALTERNATIVES_HEAD), vec![sym("a"), sym("b"), sym("c")])
        );
        // A lone operand (no `|`) passes straight through.
        assert_eq!(lower_one("a\n"), sym("a"));
    }

    #[test]
    fn condition_operator_lowers_to_condition_head_with_bare_test() {
        // x_ /; x > 2  ->  Condition(Pattern(x, Blank()), Greater(x, 2))
        // The test keeps a BARE `x` (the W-20 Condition handler substitutes the
        // named binding into it), so we must NOT rewrite it into a Pattern node.
        assert_eq!(
            lower_one("x_ /; x > 2\n"),
            apply(
                sym(CONDITION_HEAD),
                vec![
                    apply(sym(PATTERN), vec![sym("x"), apply(sym(BLANK), vec![])]),
                    apply(sym(GREATER), vec![sym("x"), int(2)]),
                ]
            )
        );
    }

    #[test]
    fn condition_binds_looser_than_alternatives() {
        // a | b /; t  ->  Condition(Alternatives(a, b), t)  (| tighter than /;)
        assert_eq!(
            lower_one("a | b /; t\n"),
            apply(
                sym(CONDITION_HEAD),
                vec![
                    apply(sym(ALTERNATIVES_HEAD), vec![sym("a"), sym("b")]),
                    sym("t"),
                ]
            )
        );
    }

    #[test]
    fn patterntest_operator_lowers_to_patterntest_head() {
        // _?EvenQ  ->  PatternTest(Blank(), EvenQ)
        assert_eq!(
            lower_one("_?EvenQ\n"),
            apply(
                sym(PATTERN_TEST_HEAD),
                vec![apply(sym(BLANK), vec![]), sym("EvenQ")]
            )
        );
        // Left-associative chain: _?IntegerQ?Positive
        //   ->  PatternTest(PatternTest(Blank(), IntegerQ), Positive)
        assert_eq!(
            lower_one("_?IntegerQ?Positive\n"),
            apply(
                sym(PATTERN_TEST_HEAD),
                vec![
                    apply(
                        sym(PATTERN_TEST_HEAD),
                        vec![apply(sym(BLANK), vec![]), sym("IntegerQ")]
                    ),
                    sym("Positive"),
                ]
            )
        );
    }

    #[test]
    fn replacerepeated_operator_lowers_to_replacerepeated_head_left_assoc() {
        // x //. a -> b  ->  ReplaceRepeated(x, Rule(a, b))
        assert_eq!(
            lower_one("x //. a -> b\n"),
            apply(
                sym(REPLACE_REPEATED),
                vec![sym("x"), apply(sym(PM_RULE), vec![sym("a"), sym("b")])]
            )
        );
        // Mixed chain folds strictly left: x /. a //. b
        //   ->  ReplaceRepeated(ReplaceAll(x, a), b)
        assert_eq!(
            lower_one("x /. a //. b\n"),
            apply(
                sym(REPLACE_REPEATED),
                vec![
                    apply(sym(REPLACE_ALL), vec![sym("x"), sym("a")]),
                    sym("b"),
                ]
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
    fn map_sugar_lowers_to_map_head() {
        // f /@ x  ->  Map[f, x]  — IR-identical to the long form Map[f, x].
        assert_eq!(
            lower_one("f /@ x\n"),
            apply(sym(MAP_HEAD), vec![sym("f"), sym("x")])
        );
        assert_eq!(lower_one("f /@ x\n"), lower_one("Map[f, x]\n"));
        // Over a list literal.
        assert_eq!(lower_one("f /@ {1, 2}\n"), lower_one("Map[f, {1, 2}]\n"));
    }

    #[test]
    fn apply_sugar_lowers_to_apply_head() {
        // f @@ x  ->  Apply[f, x]  — IR-identical to Apply[f, x].
        assert_eq!(
            lower_one("f @@ x\n"),
            apply(sym(APPLY_HEAD), vec![sym("f"), sym("x")])
        );
        assert_eq!(
            lower_one("Plus @@ {1, 2, 3}\n"),
            lower_one("Apply[Plus, {1, 2, 3}]\n")
        );
    }

    #[test]
    fn mapapply_chain_folds_left() {
        // `/@` and `@@` share one left-associative precedence level, so a mixed
        // chain folds strictly left: `g @@ f /@ x` is `(g @@ f) /@ x` =
        // Map[Apply[g, f], x]. (Real Wolfram gives `@@` higher precedence; the
        // subset does not, per MA04 §9 — parenthesise when mixing.)
        assert_eq!(
            lower_one("g @@ f /@ x\n"),
            apply(
                sym(MAP_HEAD),
                vec![apply(sym(APPLY_HEAD), vec![sym("g"), sym("f")]), sym("x")]
            )
        );
        // A same-operator chain is unambiguously left: `f /@ g /@ x` =
        // Map[Map[f, g], x].
        assert_eq!(
            lower_one("f /@ g /@ x\n"),
            apply(
                sym(MAP_HEAD),
                vec![apply(sym(MAP_HEAD), vec![sym("f"), sym("g")]), sym("x")]
            )
        );
    }

    #[test]
    fn double_bracket_sugar_lowers_to_part_head() {
        // x[[i]]  ->  Part[x, i]  — IR-identical to Part[x, i].
        assert_eq!(
            lower_one("x[[2]]\n"),
            apply(sym(PART_HEAD), vec![sym("x"), int(2)])
        );
        assert_eq!(lower_one("x[[2]]\n"), lower_one("Part[x, 2]\n"));
        assert_eq!(
            lower_one("{a, b, c}[[2]]\n"),
            lower_one("Part[{a, b, c}, 2]\n")
        );
    }

    #[test]
    fn chained_and_multi_index_part_nest() {
        // m[[1]][[2]]  ->  Part[Part[m, 1], 2]
        assert_eq!(
            lower_one("m[[1]][[2]]\n"),
            apply(
                sym(PART_HEAD),
                vec![apply(sym(PART_HEAD), vec![sym("m"), int(1)]), int(2)]
            )
        );
        // Multi-index m[[1, 2]] folds the same way: Part[Part[m, 1], 2].
        assert_eq!(lower_one("m[[1, 2]]\n"), lower_one("m[[1]][[2]]\n"));
    }

    #[test]
    fn part_sugar_interleaves_with_application() {
        // f[x][[1]]  ->  Part[f[x], 1]
        assert_eq!(
            lower_one("f[x][[1]]\n"),
            apply(
                sym(PART_HEAD),
                vec![apply(sym("f"), vec![sym("x")]), int(1)]
            )
        );
        // x[[1]][y]  ->  (Part[x, 1])[y]
        assert_eq!(
            lower_one("x[[1]][y]\n"),
            apply(
                apply(sym(PART_HEAD), vec![sym("x"), int(1)]),
                vec![sym("y")]
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

    // --- W-11 pure functions: slots, & postfix, Function normalisation -----

    #[test]
    fn bare_and_numbered_slots_lower_to_slot_head() {
        // #  →  Slot[1]   (# ≡ #1)
        assert_eq!(lower_one("#\n"), apply(sym(SLOT_HEAD), vec![int(1)]));
        // #2 →  Slot[2]
        assert_eq!(lower_one("#2\n"), apply(sym(SLOT_HEAD), vec![int(2)]));
        // ## →  SlotSequence[1]
        assert_eq!(
            lower_one("##\n"),
            apply(sym(SLOT_SEQUENCE_HEAD), vec![int(1)])
        );
    }

    #[test]
    fn amp_postfix_wraps_body_in_a_slot_function() {
        // (#^2)&  →  Function[Pow[Slot[1], 2]]  (slot-based, one-arg Function).
        assert_eq!(
            lower_one("#^2 &\n"),
            apply(
                sym(FUNCTION_HEAD),
                vec![apply(
                    sym(POW),
                    vec![apply(sym(SLOT_HEAD), vec![int(1)]), int(2)]
                )]
            )
        );
    }

    #[test]
    fn amp_precedence_puts_power_inside_the_function_body() {
        // The pinned precedence: `#^2 &` is `(#^2)&`, NOT `#^(2&)`. So the body
        // must be a Pow whose base is Slot[1] — the `&` captured the whole `#^2`.
        let ir = lower_one("#^2 &\n");
        let IRNode::Apply(func) = &ir else {
            panic!("expected a Function apply, got {ir:?}");
        };
        assert_eq!(func.head, sym(FUNCTION_HEAD));
        assert_eq!(func.args.len(), 1, "slot-based Function has one (body) arg");
        // The single body arg is a Pow, not a bare Slot with a Function exponent.
        let IRNode::Apply(body) = &func.args[0] else {
            panic!("body should be a Pow apply");
        };
        assert_eq!(body.head, sym(POW), "the `^` is inside the function body");
    }

    #[test]
    fn double_ampersand_wraps_twice() {
        // `# & &`  →  Function[Function[Slot[1]]].
        assert_eq!(
            lower_one("# & &\n"),
            apply(
                sym(FUNCTION_HEAD),
                vec![apply(
                    sym(FUNCTION_HEAD),
                    vec![apply(sym(SLOT_HEAD), vec![int(1)])]
                )]
            )
        );
    }

    #[test]
    fn named_function_long_form_normalises_params_to_a_list() {
        // Function[x, x^2]  →  Function[List[x], Pow[x, 2]]  (single param wrapped).
        assert_eq!(
            lower_one("Function[x, x^2]\n"),
            apply(
                sym(FUNCTION_HEAD),
                vec![
                    apply(sym(LIST), vec![sym("x")]),
                    apply(sym(POW), vec![sym("x"), int(2)])
                ]
            )
        );
        // Function[{x, y}, x + y]  →  Function[List[x, y], Add[x, y]]  (list kept).
        assert_eq!(
            lower_one("Function[{x, y}, x + y]\n"),
            apply(
                sym(FUNCTION_HEAD),
                vec![
                    apply(sym(LIST), vec![sym("x"), sym("y")]),
                    apply(sym(ADD), vec![sym("x"), sym("y")])
                ]
            )
        );
    }

    #[test]
    fn pure_function_applied_lowers_to_application_of_a_function() {
        // (#^2)&[5]  →  (Function[Pow[Slot[1], 2]])[5]  — an Apply whose head is
        // the Function node; the backend rule resolves it at eval time.
        let ir = lower_one("(#^2)&[5]\n");
        let IRNode::Apply(outer) = &ir else {
            panic!("expected an application, got {ir:?}");
        };
        assert_eq!(outer.args, vec![int(5)], "the arg list is [5]");
        let IRNode::Apply(func) = &outer.head else {
            panic!("the head must be the Function node");
        };
        assert_eq!(func.head, sym(FUNCTION_HEAD));
    }

    #[test]
    fn slot_number_must_be_positive() {
        // `#0` is not a valid slot (slots are 1-indexed) — lowering errors.
        let ast = parse_wolfram("#0\n");
        assert!(lower_program(&ast).is_err());
    }
}
