//! # The Axiom evaluator — an AST-walking interpreter over `axiom-parser`'s CST
//!
//! ## Why this is an interpreter, not a two-phase "lower, then evaluate" pass
//!
//! Every other CAS-family runtime in this repo (`derive-runtime`,
//! `reduce-runtime`, `maple-runtime`) works in **two clean phases**: lower
//! the whole parsed tree to a `symbolic_ir::IRNode` first (no evaluation),
//! then hand the finished tree to `symbolic_vm::VM::eval` once. That works
//! because every one of their surface constructs has a direct `IRNode` head
//! to lower to.
//!
//! Axiom's `::` (coercion), `:` (declaration), and `has` (category query)
//! do **not** — MA13 §2's own central finding is that `symbolic-ir` has no
//! domain/category concept at all, so there is no `IRNode` head "Coerce" or
//! "Declare" to lower to. Worse, `coercion` sits *inside* the ordinary
//! arithmetic cascade (`a + b :: Float` is legal, MA13 §4), so a coercion
//! can appear as a genuine sub-expression anywhere arithmetic can, not just
//! at a statement's top level. That rules out a clean lower-then-evaluate
//! split: this module instead walks the CST **top-down, evaluating eagerly**
//! (an ordinary tree-walking interpreter) so that by the time it reaches a
//! `coercion`/`declaration`/`has_query` node, it already has a concrete,
//! evaluated [`AxiomValue`] in hand to check against the fixed domain table
//! (`crate::domains`) — exactly the "evaluated entirely within
//! `axiom-runtime`'s own dispatcher, never inside `symbolic-vm` itself"
//! shape MA13 §2/§5 calls for.
//!
//! Pure arithmetic (`+ - * / ^` and comparisons) is still **reused
//! unchanged**: each fold step builds one small `IRNode::Apply` and hands it
//! to [`symbolic_vm::VM::eval`] — no new math, exactly MA13 §2's finding.
//! Evaluating one fold step at a time (rather than building the whole
//! nested tree first) also means a long flat chain (`1+1+1+…`) never
//! becomes a single deep `IRNode` the VM must recurse through in one call —
//! each step is O(1) VM-recursion depth, sidestepping the "flat repetition
//! folds into a deep tree" DoS vector by construction, not by a
//! token-count patch (see `crate::MAX_STATEMENT_TOKENS`'s own doc comment
//! for the remaining, narrower reason that cap still exists as
//! defense-in-depth).
//!
//! ## Function bodies are the one place this module still *lowers* rather
//! than evaluates
//!
//! A held function body must **not** be evaluated at definition time (MA13
//! §4: `f(x: T, ...): T == e` — the body is stored, substituted, and
//! evaluated fresh at *call* time, "duck-typed... since this is an
//! interpreter, not Axiom's own compiler"). This module therefore reuses
//! `symbolic_vm`'s own `Define`/user-function-call machinery **unchanged**
//! (the exact mechanism Derive/Reduce/Maple already use for their own
//! user-defined functions) rather than inventing a second, bespoke
//! recursive-call mechanism of its own — which would need its own
//! call-depth guard against unbounded native recursion (an infinitely
//! self-recursive function). Reusing the shared mechanism means Axiom's own
//! user functions carry exactly the same (lack of an extra) recursion-depth
//! guard every sibling CAS-family runtime here already has, matching that
//! established convention rather than introducing a new one.
//!
//! The trade-off, disclosed rather than silently accepted: since `::`/`:`/
//! `has` have **no** `IRNode` representation at all, a function body cannot
//! contain them (there would be nothing for the shared VM's substitution
//! mechanism to evaluate them *as*) — [`lower_pure_body`] structurally
//! lowers a body through the arithmetic/comparison/`if`/call/list subset
//! only, and cleanly rejects `:=`/`:`/`::`/`has`/a `;`-sequenced block
//! inside a body with an [`EvalError`]. This is a real, disclosed
//! narrowing, not a silent gap: it matches MA13 §4's own single confirmed
//! function-definition example (`power(x: Integer, n: NonNegativeInteger):
//! Integer == x ** n`, a pure arithmetic expression) exactly, and every
//! sibling CAS-family runtime's own function bodies are equally
//! single-expression-only.

use crate::domains::{
    coerce_value, domain_has_category, resolve_category, resolve_domain, AxiomDomain, DomainError,
};
use crate::value::{infer_domain, print_axiom, AxiomValue};
use crate::{builtins, EvalContext};
use lexer::token::Token;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
use symbolic_ir::{apply, flt, int, str_node, sym, IRNode};

/// A failure while evaluating Axiom source — a coercion/declaration-mismatch
/// (the book's own confirmed error shape), an unresolved domain/category
/// name, a malformed AST node, or an unsupported construct inside a
/// function body. Not a Rust panic: every evaluation error surfaces as a
/// clean `Err` all the way out to `AxiomSession::feed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalError {
    message: String,
}

impl EvalError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        EvalError {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for EvalError {}

impl From<DomainError> for EvalError {
    fn from(e: DomainError) -> Self {
        EvalError::new(e.0)
    }
}

/// Evaluate any parsed node (typically the `program` root `axiom-parser`
/// hands back) and return its [`AxiomValue`].
///
/// Transparent wrapper rules (`program`, `expr`, `define`, `atom`, and every
/// precedence-cascade rule that did not apply its own operator) are peeled
/// away by [`unwrap_single`] before dispatch, so this is safe to call on any
/// node in the tree, not just a top-level `program`/`expr` — every internal
/// recursive call in this module goes through this same entry point.
pub fn eval_expr(ctx: &mut EvalContext, node: &GrammarASTNode) -> Result<AxiomValue, EvalError> {
    match unwrap_single(node) {
        Unwrapped::Token(token) => eval_token(ctx, token),
        Unwrapped::Node(node) => match node.rule_name.as_str() {
            "if_expr" => eval_if(ctx, node),
            "declared_define" => eval_declared_define(ctx, node),
            "undeclared_define" => eval_undeclared_define(ctx, node),
            "assignment" => eval_assignment(ctx, node),
            "declaration" => eval_declaration(ctx, node),
            "has_query" => eval_has_query(node),
            "comparison" => eval_comparison(ctx, node),
            "coercion" => eval_coercion(ctx, node),
            "additive" => eval_binary_chain(ctx, node, additive_head),
            "multiplicative" => eval_binary_chain(ctx, node, multiplicative_head),
            "unary" => eval_unary(ctx, node),
            "power" => eval_power(ctx, node),
            "postfix" => eval_postfix(ctx, node),
            "list_literal" => eval_list_literal(ctx, node),
            "group" => eval_group(ctx, node),
            other => Err(EvalError::new(format!("no evaluation for rule `{other}`"))),
        },
    }
}

// ---------------------------------------------------------------------------
// Literal / symbol tokens
// ---------------------------------------------------------------------------

fn eval_token(ctx: &mut EvalContext, token: &Token) -> Result<AxiomValue, EvalError> {
    match token_type(token) {
        "NUMBER" => {
            let node = parse_number(&token.value)?;
            Ok(AxiomValue::inferred(ctx.vm.eval(node)))
        }
        "STRING" => Ok(AxiomValue::inferred(str_node(token.value.clone()))),
        "NAME" => {
            let result = ctx.vm.eval(sym(&token.value));
            let domain = ctx
                .declared
                .get(&token.value)
                .cloned()
                .or_else(|| infer_domain(&result));
            Ok(AxiomValue { node: result, domain })
        }
        other => Err(EvalError::new(format!(
            "unexpected token `{other}` = {:?}",
            token.value
        ))),
    }
}

fn parse_number(text: &str) -> Result<IRNode, EvalError> {
    if text.contains('.') || text.contains('e') || text.contains('E') {
        text.parse::<f64>()
            .map(flt)
            .map_err(|e| EvalError::new(format!("invalid real literal {text:?}: {e}")))
    } else {
        text.parse::<i64>()
            .map(int)
            .map_err(|e| EvalError::new(format!("invalid integer literal {text:?}: {e}")))
    }
}

// ---------------------------------------------------------------------------
// if p then e1 else e2
// ---------------------------------------------------------------------------

fn eval_if(ctx: &mut EvalContext, node: &GrammarASTNode) -> Result<AxiomValue, EvalError> {
    let exprs: Vec<&GrammarASTNode> = child_nodes(node).filter(|n| n.rule_name == "expr").collect();
    if exprs.len() != 3 {
        return Err(EvalError::new("malformed `if` node"));
    }
    let predicate = eval_expr(ctx, exprs[0])?;
    match &predicate.node {
        IRNode::Symbol(s) if s == "True" => eval_expr(ctx, exprs[1]),
        IRNode::Symbol(s) if s == "False" => eval_expr(ctx, exprs[2]),
        other => Err(EvalError::new(format!(
            "`if` predicate must evaluate to Boolean, got: {}",
            print_axiom(other)
        ))),
    }
}

// ---------------------------------------------------------------------------
// Function definition -- `==`, held body (reuses the shared VM's Define
// mechanism unchanged; see the module doc comment for why)
// ---------------------------------------------------------------------------

fn eval_declared_define(ctx: &mut EvalContext, node: &GrammarASTNode) -> Result<AxiomValue, EvalError> {
    // declared_define = NAME LPAREN [ typed_param_list ] RPAREN COLON type_expr DEFINE expr
    let name = first_token_value(node, "NAME")
        .ok_or_else(|| EvalError::new("malformed function definition: missing name"))?;

    let params = child_nodes(node)
        .find(|n| n.rule_name == "typed_param_list")
        .map(typed_param_names)
        .transpose()?
        .unwrap_or_default();

    // The return-type annotation is a DIRECT child of `declared_define`
    // (not nested inside `typed_param_list`), so this filter cannot
    // accidentally pick up a parameter's own type_expr.
    let return_type_node = child_nodes(node)
        .find(|n| n.rule_name == "type_expr")
        .ok_or_else(|| EvalError::new("malformed function definition: missing return type"))?;
    resolve_domain(&builtins::parse_type_spec(return_type_node))?;

    let body_node = child_nodes(node)
        .find(|n| n.rule_name == "expr")
        .ok_or_else(|| EvalError::new("malformed function definition: missing body"))?;
    let body_ir = lower_pure_body(body_node)?;

    register_function(ctx, &name, params, body_ir)
}

fn eval_undeclared_define(ctx: &mut EvalContext, node: &GrammarASTNode) -> Result<AxiomValue, EvalError> {
    // undeclared_define = NAME NAME DEFINE expr
    let names: Vec<String> = node
        .children
        .iter()
        .filter_map(as_token)
        .filter(|t| token_type(t) == "NAME")
        .map(|t| t.value.clone())
        .collect();
    let [name, param] = names.as_slice() else {
        return Err(EvalError::new("malformed undeclared function definition"));
    };
    let body_node = child_nodes(node)
        .find(|n| n.rule_name == "expr")
        .ok_or_else(|| EvalError::new("malformed function definition: missing body"))?;
    let body_ir = lower_pure_body(body_node)?;
    register_function(ctx, name, vec![param.clone()], body_ir)
}

fn register_function(
    ctx: &mut EvalContext,
    name: &str,
    params: Vec<String>,
    body: IRNode,
) -> Result<AxiomValue, EvalError> {
    let params_list = apply(sym(symbolic_ir::LIST), params.into_iter().map(sym).collect());
    let define_node = apply(sym(symbolic_ir::DEFINE), vec![sym(name), params_list, body]);
    let result = ctx.vm.eval(define_node);
    Ok(AxiomValue::inferred(result))
}

/// `typed_param_list = typed_param { COMMA typed_param } ;  typed_param =
/// NAME COLON type_expr`. Extracts each parameter's name, validating that
/// its declared type resolves against the fixed domain table (mirroring the
/// book's own confirmed `Polynomial(String)`-is-invalid rejection) -- the
/// annotation itself is not enforced against call arguments (MA13 §4: the
/// undeclared form is duck-typed per call, and this cut does not build a
/// full static type-checker for the declared form either), only checked for
/// being a well-formed reference to this cut's fixed table.
fn typed_param_names(node: &GrammarASTNode) -> Result<Vec<String>, EvalError> {
    child_nodes(node)
        .filter(|n| n.rule_name == "typed_param")
        .map(|param| {
            let name = first_token_value(param, "NAME")
                .ok_or_else(|| EvalError::new("malformed parameter: missing name"))?;
            let type_expr = child_nodes(param)
                .find(|n| n.rule_name == "type_expr")
                .ok_or_else(|| EvalError::new("malformed parameter: missing type"))?;
            resolve_domain(&builtins::parse_type_spec(type_expr))?;
            Ok(name)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// x := e -- immediate assignment, domain-checked against a prior `:` (MA13 §3/§4)
// ---------------------------------------------------------------------------

fn eval_assignment(ctx: &mut EvalContext, node: &GrammarASTNode) -> Result<AxiomValue, EvalError> {
    let name = first_token_value(node, "NAME")
        .ok_or_else(|| EvalError::new("malformed assignment: missing name"))?;
    let rhs_node = child_nodes(node)
        .find(|n| n.rule_name == "expr")
        .ok_or_else(|| EvalError::new("malformed assignment: missing right-hand side"))?;
    let rhs = eval_expr(ctx, rhs_node)?;

    let declared = ctx.declared.get(&name).cloned();
    let (stored_node, domain) = if let Some(domain) = &declared {
        match coerce_value(&rhs.node, domain) {
            Some(coerced) => (coerced, declared),
            None => {
                // The book's own confirmed error shape (MA13 §3), quoted
                // verbatim: "Cannot convert right-hand side of assignment
                // ... to an object of the type Integer of the left-hand
                // side."
                return Err(EvalError::new(format!(
                    "Cannot convert right-hand side of assignment {} to an object of the type {} of the left-hand side.",
                    print_axiom(&rhs.node),
                    domain.display_name()
                )));
            }
        }
    } else {
        (rhs.node, rhs.domain)
    };

    ctx.vm.backend.bind(&name, stored_node.clone());
    Ok(AxiomValue {
        node: stored_node,
        domain,
    })
}

// ---------------------------------------------------------------------------
// a : T / (a, b, c) : T -- declaration (MA13 §3/§4)
// ---------------------------------------------------------------------------

fn eval_declaration(ctx: &mut EvalContext, node: &GrammarASTNode) -> Result<AxiomValue, EvalError> {
    let decl_target = child_nodes(node)
        .find(|n| n.rule_name == "decl_target")
        .ok_or_else(|| EvalError::new("malformed declaration: missing target"))?;
    let type_expr = child_nodes(node)
        .find(|n| n.rule_name == "type_expr")
        .ok_or_else(|| EvalError::new("malformed declaration: missing type"))?;
    let domain = resolve_domain(&builtins::parse_type_spec(type_expr))?;

    for name in decl_target_names(decl_target) {
        ctx.declared.insert(name, domain.clone());
    }

    // A pure declaration has no value of its own in real Axiom's own
    // interactive session (it restricts a name's domain, nothing else) --
    // this crate's own disclosed presentation convention echoes `true`
    // (Boolean) to confirm the declaration was accepted, since MA13 §3/§4
    // does not show what a bare declaration itself "evaluates to."
    Ok(AxiomValue::with_domain(sym("True"), AxiomDomain::Boolean))
}

/// `decl_target = NAME | LPAREN name_list RPAREN ;  name_list = NAME {
/// COMMA NAME }`.
fn decl_target_names(node: &GrammarASTNode) -> Vec<String> {
    if let Some(name_list) = child_nodes(node).find(|n| n.rule_name == "name_list") {
        return name_list
            .children
            .iter()
            .filter_map(as_token)
            .filter(|t| token_type(t) == "NAME")
            .map(|t| t.value.clone())
            .collect();
    }
    node.children
        .iter()
        .filter_map(as_token)
        .filter(|t| token_type(t) == "NAME")
        .map(|t| t.value.clone())
        .collect()
}

// ---------------------------------------------------------------------------
// D has C -- category-membership query (MA13 §3/§4)
// ---------------------------------------------------------------------------

fn eval_has_query(node: &GrammarASTNode) -> Result<AxiomValue, EvalError> {
    let type_exprs: Vec<&GrammarASTNode> = child_nodes(node)
        .filter(|n| n.rule_name == "type_expr")
        .collect();
    if type_exprs.len() != 2 {
        return Err(EvalError::new("malformed `has` query"));
    }
    let domain = resolve_domain(&builtins::parse_type_spec(type_exprs[0]))?;
    let category = resolve_category(&builtins::parse_type_spec(type_exprs[1]))?;
    let result = domain_has_category(&domain, category);
    Ok(AxiomValue::with_domain(
        if result { sym("True") } else { sym("False") },
        AxiomDomain::Boolean,
    ))
}

// ---------------------------------------------------------------------------
// comparison = coercion [ (EQ|NE|LE|LESS|GREATER|GE) coercion ]
// ---------------------------------------------------------------------------

fn eval_comparison(ctx: &mut EvalContext, node: &GrammarASTNode) -> Result<AxiomValue, EvalError> {
    let op_index = node
        .children
        .iter()
        .position(|c| as_token(c).is_some_and(|t| comparison_head(token_type(t)).is_some()))
        .ok_or_else(|| EvalError::new("malformed comparison node"))?;
    if op_index == 0 || op_index + 1 >= node.children.len() {
        return Err(EvalError::new("malformed comparison node"));
    }
    let head = comparison_head(token_type(as_token(&node.children[op_index]).unwrap())).unwrap();
    let lhs = eval_expr_child(ctx, &node.children[op_index - 1])?;
    let rhs = eval_expr_child(ctx, &node.children[op_index + 1])?;
    let result = ctx.vm.eval(apply(sym(head), vec![lhs.node, rhs.node]));
    Ok(AxiomValue::inferred(result))
}

// ---------------------------------------------------------------------------
// coercion = additive [ COERCE type_expr ]
// ---------------------------------------------------------------------------

fn eval_coercion(ctx: &mut EvalContext, node: &GrammarASTNode) -> Result<AxiomValue, EvalError> {
    let additive_node = child_nodes(node)
        .find(|n| n.rule_name == "additive")
        .ok_or_else(|| EvalError::new("malformed coercion: missing left-hand side"))?;
    let type_expr = child_nodes(node)
        .find(|n| n.rule_name == "type_expr")
        .ok_or_else(|| EvalError::new("malformed coercion: missing target type"))?;

    let lhs = eval_expr(ctx, additive_node)?;
    let target = resolve_domain(&builtins::parse_type_spec(type_expr))?;

    match coerce_value(&lhs.node, &target) {
        Some(coerced) => Ok(AxiomValue::with_domain(coerced, target)),
        None => {
            // Adapted from the book's own confirmed assignment-mismatch
            // phrase (MA13 §3) for the standalone `::` case, which has no
            // "left-hand side" of its own to name -- disclosed here as an
            // adaptation, not an independently-verified-to-the-byte second
            // quotation.
            Err(EvalError::new(format!(
                "Cannot convert {} to an object of the type {}.",
                print_axiom(&lhs.node),
                target.display_name()
            )))
        }
    }
}

// ---------------------------------------------------------------------------
// additive / multiplicative -- iterative left-associative fold
// ---------------------------------------------------------------------------

fn additive_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "PLUS" => Some(symbolic_ir::ADD),
        "MINUS" => Some(symbolic_ir::SUB),
        _ => None,
    }
}

fn multiplicative_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "TIMES" => Some(symbolic_ir::MUL),
        "SLASH" => Some(symbolic_ir::DIV),
        _ => None,
    }
}

fn comparison_head(token_type: &str) -> Option<&'static str> {
    match token_type {
        "EQ" => Some(symbolic_ir::EQUAL),
        "NE" => Some(symbolic_ir::NOT_EQUAL),
        "LE" => Some(symbolic_ir::LESS_EQUAL),
        "LESS" => Some(symbolic_ir::LESS),
        "GREATER" => Some(symbolic_ir::GREATER),
        "GE" => Some(symbolic_ir::GREATER_EQUAL),
        _ => None,
    }
}

/// Fold a flat `operand { op operand }` repetition **iteratively**, one
/// `symbolic_vm::VM::eval` call per step (see the module doc comment for
/// why this sidesteps the "flat chain folds into one deep tree" DoS vector
/// by construction).
fn eval_binary_chain(
    ctx: &mut EvalContext,
    node: &GrammarASTNode,
    head_of: fn(&str) -> Option<&'static str>,
) -> Result<AxiomValue, EvalError> {
    let mut children = node.children.iter();
    let first = children
        .next()
        .ok_or_else(|| EvalError::new("empty binary chain"))?;
    let mut acc = eval_expr_child(ctx, first)?;
    while let Some(op_child) = children.next() {
        let head = as_token(op_child)
            .and_then(|t| head_of(token_type(t)))
            .ok_or_else(|| EvalError::new("expected a binary operator"))?;
        let rhs_child = children
            .next()
            .ok_or_else(|| EvalError::new("binary operator with no right operand"))?;
        let rhs = eval_expr_child(ctx, rhs_child)?;
        let result = ctx.vm.eval(apply(sym(head), vec![acc.node, rhs.node]));
        acc = AxiomValue::inferred(result);
    }
    Ok(acc)
}

// ---------------------------------------------------------------------------
// unary = MINUS unary | power
// ---------------------------------------------------------------------------

fn eval_unary(ctx: &mut EvalContext, node: &GrammarASTNode) -> Result<AxiomValue, EvalError> {
    let operand_node = child_nodes(node)
        .next()
        .ok_or_else(|| EvalError::new("unary `-` with no operand"))?;
    let operand = eval_expr(ctx, operand_node)?;
    let result = ctx.vm.eval(apply(sym(symbolic_ir::NEG), vec![operand.node]));
    Ok(AxiomValue::inferred(result))
}

// ---------------------------------------------------------------------------
// power = postfix [ (CARET|POW) unary ]
// ---------------------------------------------------------------------------

fn eval_power(ctx: &mut EvalContext, node: &GrammarASTNode) -> Result<AxiomValue, EvalError> {
    let nodes: Vec<&GrammarASTNode> = child_nodes(node).collect();
    if nodes.len() != 2 {
        return Err(EvalError::new("malformed power node"));
    }
    let base = eval_expr(ctx, nodes[0])?;
    let exp = eval_expr(ctx, nodes[1])?;
    let result = ctx
        .vm
        .eval(apply(sym(symbolic_ir::POW), vec![base.node, exp.node]));
    Ok(AxiomValue::inferred(result))
}

// ---------------------------------------------------------------------------
// postfix = atom [ call_args ] -- function application
// ---------------------------------------------------------------------------

fn eval_postfix(ctx: &mut EvalContext, node: &GrammarASTNode) -> Result<AxiomValue, EvalError> {
    let atom_node = child_nodes(node)
        .next()
        .ok_or_else(|| EvalError::new("postfix has no base"))?;
    let call_args_node = child_nodes(node)
        .find(|n| n.rule_name == "call_args")
        .ok_or_else(|| EvalError::new("malformed postfix node"))?;

    let head = postfix_head(ctx, atom_node)?;
    let arg_nodes = call_args_exprs(call_args_node);
    let mut args = Vec::with_capacity(arg_nodes.len());
    for a in arg_nodes {
        args.push(eval_expr(ctx, a)?.node);
    }
    let result = ctx.vm.eval(apply(head, args));
    Ok(AxiomValue::inferred(result))
}

/// The call's head -- a bare `NAME` atom is read as a raw, **unevaluated**
/// `Symbol`, not looked up first: the shared VM's own `eval_apply` decides
/// how to resolve a `Symbol` head (a bound `Define` record vs. a free
/// symbol) itself, and evaluating it here first would strip that
/// information away before the VM ever sees it (mirrors
/// `derive-runtime::lower::lower_postfix`'s identical "don't evaluate the
/// callee" design).
fn postfix_head(ctx: &mut EvalContext, atom_node: &GrammarASTNode) -> Result<IRNode, EvalError> {
    if let Some(name) = bare_name(atom_node) {
        return Ok(sym(name));
    }
    Ok(eval_expr(ctx, atom_node)?.node)
}

/// `call_args = LPAREN [ arglist ] RPAREN | atom`. Returns the argument
/// expression nodes to evaluate, uniformly for both the explicit-parens
/// form and the paren-optional single-bare-atom form (`f a`).
fn call_args_exprs(node: &GrammarASTNode) -> Vec<&GrammarASTNode> {
    if let Some(arglist) = child_nodes(node).find(|n| n.rule_name == "arglist") {
        return child_nodes(arglist)
            .filter(|n| n.rule_name == "expr")
            .collect();
    }
    let has_lparen = node
        .children
        .iter()
        .any(|c| as_token(c).is_some_and(|t| token_type(t) == "LPAREN"));
    if has_lparen {
        Vec::new() // f() -- explicit, empty argument list
    } else {
        // f a -- the paren-optional single bare atom IS the one argument.
        child_nodes(node).filter(|n| n.rule_name == "atom").collect()
    }
}

// ---------------------------------------------------------------------------
// list_literal = LBRACKET [ elem_list ] RBRACKET
// ---------------------------------------------------------------------------

fn eval_list_literal(ctx: &mut EvalContext, node: &GrammarASTNode) -> Result<AxiomValue, EvalError> {
    let elems: Vec<&GrammarASTNode> = child_nodes(node)
        .find(|n| n.rule_name == "elem_list")
        .map(|elem_list| child_nodes(elem_list).filter(|n| n.rule_name == "expr").collect())
        .unwrap_or_default();
    let mut values = Vec::with_capacity(elems.len());
    for e in elems {
        values.push(eval_expr(ctx, e)?.node);
    }
    let result = ctx.vm.eval(apply(sym(symbolic_ir::LIST), values));
    Ok(AxiomValue::inferred(result))
}

// ---------------------------------------------------------------------------
// group = LPAREN expr { SEMI expr } RPAREN -- grouping OR a `;`-block
// ---------------------------------------------------------------------------

fn eval_group(ctx: &mut EvalContext, node: &GrammarASTNode) -> Result<AxiomValue, EvalError> {
    let exprs: Vec<&GrammarASTNode> = child_nodes(node).filter(|n| n.rule_name == "expr").collect();
    if exprs.is_empty() {
        return Err(EvalError::new("empty group `( )`"));
    }
    let mut last = None;
    for e in exprs {
        last = Some(eval_expr(ctx, e)?);
    }
    Ok(last.expect("checked non-empty above"))
}

// ---------------------------------------------------------------------------
// lower_pure_body -- structural (non-evaluating) lowering for held function
// bodies. See the module doc comment for why this subset exists.
// ---------------------------------------------------------------------------

fn lower_pure_body(node: &GrammarASTNode) -> Result<IRNode, EvalError> {
    match unwrap_single(node) {
        Unwrapped::Token(token) => lower_pure_token(token),
        Unwrapped::Node(node) => match node.rule_name.as_str() {
            "if_expr" => {
                let exprs: Vec<&GrammarASTNode> =
                    child_nodes(node).filter(|n| n.rule_name == "expr").collect();
                if exprs.len() != 3 {
                    return Err(EvalError::new("malformed `if` node"));
                }
                Ok(apply(
                    sym(symbolic_ir::IF),
                    vec![
                        lower_pure_body(exprs[0])?,
                        lower_pure_body(exprs[1])?,
                        lower_pure_body(exprs[2])?,
                    ],
                ))
            }
            "comparison" => lower_pure_binary(node, comparison_head),
            "coercion" => {
                if has_coerce_token(node) {
                    Err(EvalError::new(
                        "`::` coercion is not supported inside a function body this cut -- \
                         write it only at the top level or inside an `if`'s branches",
                    ))
                } else {
                    lower_pure_first(node)
                }
            }
            "additive" => lower_pure_chain(node, additive_head),
            "multiplicative" => lower_pure_chain(node, multiplicative_head),
            "unary" => {
                let operand = child_nodes(node)
                    .next()
                    .ok_or_else(|| EvalError::new("unary `-` with no operand"))?;
                Ok(apply(sym(symbolic_ir::NEG), vec![lower_pure_body(operand)?]))
            }
            "power" => {
                let nodes: Vec<&GrammarASTNode> = child_nodes(node).collect();
                if nodes.len() != 2 {
                    return Err(EvalError::new("malformed power node"));
                }
                Ok(apply(
                    sym(symbolic_ir::POW),
                    vec![lower_pure_body(nodes[0])?, lower_pure_body(nodes[1])?],
                ))
            }
            "postfix" => lower_pure_postfix(node),
            "list_literal" => {
                let elems: Vec<&GrammarASTNode> = child_nodes(node)
                    .find(|n| n.rule_name == "elem_list")
                    .map(|el| child_nodes(el).filter(|n| n.rule_name == "expr").collect())
                    .unwrap_or_default();
                let values = elems
                    .into_iter()
                    .map(lower_pure_body)
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(apply(sym(symbolic_ir::LIST), values))
            }
            "group" => {
                let exprs: Vec<&GrammarASTNode> =
                    child_nodes(node).filter(|n| n.rule_name == "expr").collect();
                if exprs.len() == 1 {
                    lower_pure_body(exprs[0])
                } else {
                    Err(EvalError::new(
                        "a `;`-sequenced block is not supported inside a function body this cut",
                    ))
                }
            }
            "declaration" => Err(EvalError::new(
                "`:` declaration is not supported inside a function body this cut",
            )),
            "has_query" => Err(EvalError::new(
                "`has` is not supported inside a function body this cut",
            )),
            "assignment" => Err(EvalError::new(
                "`:=` assignment is not supported inside a function body this cut -- \
                 write a single expression, matching MA13's own confirmed function-definition example",
            )),
            "declared_define" | "undeclared_define" => {
                Err(EvalError::new("nested function definitions are not supported"))
            }
            other => Err(EvalError::new(format!(
                "no lowering for rule `{other}` inside a function body"
            ))),
        },
    }
}

fn lower_pure_token(token: &Token) -> Result<IRNode, EvalError> {
    match token_type(token) {
        "NUMBER" => parse_number(&token.value),
        "STRING" => Ok(str_node(token.value.clone())),
        "NAME" => Ok(sym(&token.value)),
        other => Err(EvalError::new(format!(
            "unexpected token `{other}` = {:?}",
            token.value
        ))),
    }
}

fn lower_pure_first(node: &GrammarASTNode) -> Result<IRNode, EvalError> {
    let child = child_nodes(node)
        .next()
        .ok_or_else(|| EvalError::new(format!("`{}` has no child", node.rule_name)))?;
    lower_pure_body(child)
}

fn lower_pure_binary(
    node: &GrammarASTNode,
    head_of: fn(&str) -> Option<&'static str>,
) -> Result<IRNode, EvalError> {
    let op_index = node
        .children
        .iter()
        .position(|c| as_token(c).is_some_and(|t| head_of(token_type(t)).is_some()))
        .ok_or_else(|| EvalError::new("malformed binary node"))?;
    if op_index == 0 || op_index + 1 >= node.children.len() {
        return Err(EvalError::new("malformed binary node"));
    }
    let head = head_of(token_type(as_token(&node.children[op_index]).unwrap())).unwrap();
    Ok(apply(
        sym(head),
        vec![
            lower_pure_child(&node.children[op_index - 1])?,
            lower_pure_child(&node.children[op_index + 1])?,
        ],
    ))
}

fn lower_pure_chain(
    node: &GrammarASTNode,
    head_of: fn(&str) -> Option<&'static str>,
) -> Result<IRNode, EvalError> {
    let mut children = node.children.iter();
    let first = children
        .next()
        .ok_or_else(|| EvalError::new("empty binary chain"))?;
    let mut result = lower_pure_child(first)?;
    while let Some(op_child) = children.next() {
        let head = as_token(op_child)
            .and_then(|t| head_of(token_type(t)))
            .ok_or_else(|| EvalError::new("expected a binary operator"))?;
        let rhs = children
            .next()
            .ok_or_else(|| EvalError::new("binary operator with no right operand"))?;
        result = apply(sym(head), vec![result, lower_pure_child(rhs)?]);
    }
    Ok(result)
}

fn lower_pure_postfix(node: &GrammarASTNode) -> Result<IRNode, EvalError> {
    let atom_node = child_nodes(node)
        .next()
        .ok_or_else(|| EvalError::new("postfix has no base"))?;
    let Some(call_args_node) = child_nodes(node).find(|n| n.rule_name == "call_args") else {
        return lower_pure_body(atom_node);
    };
    let head = match bare_name(atom_node) {
        Some(name) => sym(name),
        None => lower_pure_body(atom_node)?,
    };
    let args = call_args_exprs(call_args_node)
        .into_iter()
        .map(lower_pure_body)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(apply(head, args))
}

fn has_coerce_token(node: &GrammarASTNode) -> bool {
    node.children
        .iter()
        .any(|c| as_token(c).is_some_and(|t| token_type(t) == "COERCE"))
}

fn lower_pure_child(child: &ASTNodeOrToken) -> Result<IRNode, EvalError> {
    match child {
        ASTNodeOrToken::Node(node) => lower_pure_body(node),
        ASTNodeOrToken::Token(token) => lower_pure_token(token),
    }
}

// ---------------------------------------------------------------------------
// Small shared helpers
// ---------------------------------------------------------------------------

fn eval_expr_child(ctx: &mut EvalContext, child: &ASTNodeOrToken) -> Result<AxiomValue, EvalError> {
    match child {
        ASTNodeOrToken::Node(node) => eval_expr(ctx, node),
        ASTNodeOrToken::Token(token) => eval_token(ctx, token),
    }
}

/// If `node` peels down (via [`unwrap_single`]) to a bare `NAME` token,
/// return its text -- used to decide whether a call's head/callee should be
/// read as a raw, unevaluated `Symbol` (see [`postfix_head`]).
fn bare_name(node: &GrammarASTNode) -> Option<String> {
    match unwrap_single(node) {
        Unwrapped::Token(token) if token_type(token) == "NAME" => Some(token.value.clone()),
        _ => None,
    }
}

/// Find the first direct-or-transparently-nested `NAME` token of the given
/// declared token type among `node`'s IMMEDIATE children only (never
/// recursing into child *nodes*) -- used for a rule whose grammar guarantees
/// its own name token is a direct child (`declared_define`, `assignment`).
fn first_token_value(node: &GrammarASTNode, token_ty: &str) -> Option<String> {
    node.children
        .iter()
        .filter_map(as_token)
        .find(|t| token_type(t) == token_ty)
        .map(|t| t.value.clone())
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

fn child_nodes(node: &GrammarASTNode) -> impl Iterator<Item = &GrammarASTNode> {
    node.children.iter().filter_map(as_node)
}

enum Unwrapped<'a> {
    Node(&'a GrammarASTNode),
    Token(&'a Token),
}

/// Peel away single-child transparent wrapper nodes (`program`, `expr`,
/// `define`, `atom`, and any precedence-cascade rule that did not apply its
/// own operator) until reaching a node with real structure, or a leaf
/// token. Mirrors `derive-runtime::lower::unwrap_single` exactly.
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
