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
//! than evaluates -- and function *calls* are dispatched here, not by
//! `symbolic_vm`, because that is what makes recursion depth cappable
//!
//! A held function body must **not** be evaluated at definition time (MA13
//! §4: `f(x: T, ...): T == e` — the body is stored, substituted, and
//! evaluated fresh at *call* time, "duck-typed... since this is an
//! interpreter, not Axiom's own compiler"). [`lower_pure_body`] structurally
//! lowers a body through the arithmetic/comparison/`if`/call/list subset
//! only (never evaluating it), and cleanly rejects `:=`/`:`/`::`/`has`/a
//! `;`-sequenced block inside a body with an [`EvalError`] -- since those
//! constructs have **no** `IRNode` representation at all, there is nothing
//! for a stored body to represent them *as*. This is a real, disclosed
//! narrowing, not a silent gap: it matches MA13 §4's own single confirmed
//! function-definition example (`power(x: Integer, n: NonNegativeInteger):
//! Integer == x ** n`, a pure arithmetic expression) exactly.
//!
//! An earlier version of this crate registered the lowered body via
//! `symbolic_vm`'s own `Define`/user-function-call mechanism (the same
//! mechanism Derive/Reduce/Maple use for their own user-defined functions)
//! and simply handed a call to `VM::eval` unchanged. **This was a real,
//! since-fixed security gap, caught in review**: `VM::eval_apply`'s own
//! substitution-and-recurse path for a bound `Define` record calls
//! `self.eval(...)` *inside its own Rust function body*, so a
//! self-recursive user function (`fact(n) == if n = 0 then 1 else n *
//! fact(n - 1)`, then `fact(50000000)`) recurses natively through
//! `symbolic_vm`'s own call stack, entirely outside this crate's control --
//! there is no seam inside `VM::eval_apply` this crate can hook a
//! call-depth counter into without editing `symbolic-vm` itself, which
//! MA13 §2 rules out. A large worker-thread stack does not fix this: it
//! only raises how deep the recursion must go before crashing, and a
//! genuine native stack overflow is **not** catchable by `catch_unwind` --
//! Rust's runtime response to one is to abort the whole process, not
//! unwind a thread.
//!
//! The fix: this crate's own [`register_function`]/`call_user_function`/
//! [`eval_ir`] now dispatch every user-function call **themselves**, never
//! handing a call to a registered function to `VM::eval` at all (ordinary
//! arithmetic/comparison/`if`/`List` heads still go through `VM::eval`
//! exactly as before -- only the "is this Apply's head a user-defined
//! function?" branch is now this crate's own). `call_user_function`
//! increments [`EvalContext::call_depth`] on every invocation, at *every*
//! nesting position inside a body (not just the top level -- `eval_ir`
//! walks the *entire* substituted body itself, rather than handing it to
//! `VM::eval` in one shot, specifically so a recursive call buried inside
//! an `if`-branch or an arithmetic operand is intercepted too), and returns
//! a clean [`EvalError`] once [`MAX_CALL_DEPTH`] is exceeded -- turning
//! unbounded recursion into an ordinary `Err`, not a process abort.

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

/// Register a held function body in this crate's own function table (see
/// the module doc comment for why this is *not* `symbolic_vm`'s own
/// `Define` mechanism). Redefining an existing name overwrites it, matching
/// the shared VM's own rebind-on-redefine behaviour every sibling CAS
/// runtime here relies on. Returns the bare name as the definition's
/// displayed value, exactly what `symbolic_vm::handlers::define_handler`
/// itself returns for a `Define`.
fn register_function(
    ctx: &mut EvalContext,
    name: &str,
    params: Vec<String>,
    body: IRNode,
) -> Result<AxiomValue, EvalError> {
    ctx.functions.insert(name.to_string(), (params, body));
    Ok(AxiomValue::inferred(sym(name)))
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

    // Self-referential reassignment guard -- see `symbolic_vm::handlers::
    // MAX_BOUND_VALUE_NODES`/`MAX_BOUND_VALUE_DEPTH`'s own doc comments for
    // the full incident this closes: repeated `a := a * a` doubles the
    // bound value's node count every step (reaching millions of nodes from
    // a few hundred bytes of source); repeated `a := a + a` ALSO doubles
    // nesting depth (via the shared `Add` handler's flatten-then-left-
    // associate canonicalization), which is independently dangerous
    // because a too-deep bound value can overflow the native stack on the
    // very NEXT lookup -- an uncatchable process abort, not a catchable
    // error -- before a node-count check on that next statement's result
    // would ever get a chance to run.
    //
    // This crate's plain `NAME ASSIGN expr` assignment does NOT lower to
    // `symbolic_ir::ASSIGN` and go through `symbolic_vm`'s shared
    // `assign_handler` the way every other CAS-family runtime's does (see
    // this module's own doc comment for why: Axiom is an eager AST-walking
    // interpreter, not a lower-then-`VM::eval`-once pipeline) -- it binds
    // directly through `ctx.vm.backend.bind` below, bypassing that shared
    // choke point entirely. Axiom's own `+`/`*`/etc. still fold through
    // `symbolic_vm::VM::eval` one step at a time (this module's own doc
    // comment, "Pure arithmetic ... is still reused unchanged"), so it hits
    // the identical shared `Add`/`Mul` handlers and is equally exposed to
    // both growth axes -- applying the identical, shared budget checks here
    // closes the same hole at Axiom's own bind site rather than leaving it
    // open behind a guard that only covers the other runtimes.
    if symbolic_vm::handlers::count_nodes_within_cap(
        &stored_node,
        symbolic_vm::handlers::MAX_BOUND_VALUE_NODES,
    )
    .is_none()
    {
        return Err(EvalError::new(format!(
            "Assign target '{name}' would bind a value exceeding {} nodes -- rejecting to \
             prevent unbounded growth from self-referential reassignment (e.g. repeated \
             '{name} := {name} * {name}')",
            symbolic_vm::handlers::MAX_BOUND_VALUE_NODES
        )));
    }
    if symbolic_vm::handlers::depth_within_cap(
        &stored_node,
        symbolic_vm::handlers::MAX_BOUND_VALUE_DEPTH,
    )
    .is_none()
    {
        return Err(EvalError::new(format!(
            "Assign target '{name}' would bind a value nested deeper than {} levels -- \
             rejecting to prevent unbounded growth from self-referential reassignment \
             (e.g. repeated '{name} := {name} + {name}')",
            symbolic_vm::handlers::MAX_BOUND_VALUE_DEPTH
        )));
    }

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
    // Carry the bare `IRNode` through the fold, not a domain-inferred
    // `AxiomValue` -- a review-caught O(N^2) cost: `AxiomValue::inferred`
    // calls `domains::is_polynomial_over_integers`, which walks the WHOLE
    // node structurally. Calling it on the accumulator at every one of N
    // fold steps re-walks an accumulator that grows by one step each time,
    // giving O(N^2) total work for one N-term chain -- despite this
    // function's own iterative, one-fold-at-a-time shape already being O(N)
    // for the actual arithmetic (the shape this module's doc comment
    // credits with "sidestepping the flat-repetition DoS vector by
    // construction" -- true for native-recursion STACK DEPTH, which this
    // fix doesn't change, but not for total CPU work, which it does).
    // Every intermediate accumulator's `.domain` is discarded anyway (only
    // `.node` ever feeds the next fold step), so there is no behavior
    // change: domain inference now runs exactly ONCE, on the final result.
    let mut acc = eval_expr_child(ctx, first)?.node;
    while let Some(op_child) = children.next() {
        let head = as_token(op_child)
            .and_then(|t| head_of(token_type(t)))
            .ok_or_else(|| EvalError::new("expected a binary operator"))?;
        let rhs_child = children
            .next()
            .ok_or_else(|| EvalError::new("binary operator with no right operand"))?;
        let rhs = eval_expr_child(ctx, rhs_child)?;
        acc = ctx.vm.eval(apply(sym(head), vec![acc, rhs.node]));
    }
    Ok(AxiomValue::inferred(acc))
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

    let arg_nodes = call_args_exprs(call_args_node);
    let mut args = Vec::with_capacity(arg_nodes.len());
    for a in arg_nodes {
        args.push(eval_expr(ctx, a)?.node);
    }

    // A bare `NAME` callee: check this crate's OWN function table first
    // (see the module doc comment for why user-function calls are
    // dispatched here, never handed to `VM::eval`, and never looked up as
    // an ordinary bound value first -- doing so would strip away the
    // "this is a call, not a value read" information before we get a
    // chance to check the function table).
    if let Some(name) = bare_name(atom_node) {
        if let Some((params, body)) = ctx.functions.get(&name).cloned() {
            let result = call_user_function(ctx, &name, &params, &body, args)?;
            return Ok(AxiomValue::inferred(result));
        }
        // Not a registered function -- an ordinary bound variable used as a
        // call head, or an unbound/free symbolic call -- delegate to the
        // shared VM exactly as before.
        let result = ctx.vm.eval(apply(sym(name), args));
        return Ok(AxiomValue::inferred(result));
    }

    // A non-bare-name callee (e.g. a parenthesised expression used as a
    // call head) -- evaluate it and delegate to the shared VM.
    let head = eval_expr(ctx, atom_node)?.node;
    let result = ctx.vm.eval(apply(head, args));
    Ok(AxiomValue::inferred(result))
}

/// Maximum number of nested user-function calls in progress at once,
/// checked by [`call_user_function`] on every invocation (at *any* nesting
/// position inside a body, via [`eval_ir`] -- not just the top level).
///
/// This is the guard against unbounded native recursion through a
/// self-recursive (or mutually recursive) user-defined function -- see the
/// module doc comment's "Function bodies" section for the full incident
/// this was added to close (a genuine, review-caught security gap: without
/// this, `fact(n) == if n = 0 then 1 else n * fact(n - 1)` then
/// `fact(50000000)` recurses natively until the process aborts on a stack
/// overflow, which `catch_unwind` cannot catch).
///
/// 500 is a conservative, generously-safe round number, not a
/// binary-searched floor the way `axiom-parser`'s `MAX_RULE_DEPTH` is: each
/// level costs `eval_ir`/`call_user_function`/`symbolic_vm::substitute`'s
/// own modest stack frames plus a cloned, `MAX_STATEMENT_TOKENS`-bounded
/// body tree, all comfortably within `crate::EVAL_STACK_SIZE`'s 512 MiB
/// worker-thread stack even at many times this depth (confirmed directly:
/// `deeply_recursive_call_is_rejected_before_native_overflow`, below, drives
/// recursion to 5,000 -- ten times the cap -- on a worker thread with a
/// deliberately small 8 MiB stack, and the cap still trips cleanly with
/// margin to spare). Ordinary hand-written recursive functions (factorial,
/// Fibonacci, list-style recursion over realistic inputs) stay far below it.
pub const MAX_CALL_DEPTH: usize = 500;

/// Call a registered user-defined function: substitute `args` for `params`
/// in `body`, then evaluate the substituted body via [`eval_ir`] -- entirely
/// within this crate's own control, so [`MAX_CALL_DEPTH`] can actually be
/// enforced (see the module doc comment).
fn call_user_function(
    ctx: &mut EvalContext,
    name: &str,
    params: &[String],
    body: &IRNode,
    args: Vec<IRNode>,
) -> Result<IRNode, EvalError> {
    if params.len() != args.len() {
        return Err(EvalError::new(format!(
            "`{name}` expects {} argument(s), got {}",
            params.len(),
            args.len()
        )));
    }

    ctx.call_depth += 1;
    if ctx.call_depth > MAX_CALL_DEPTH {
        ctx.call_depth -= 1;
        return Err(EvalError::new(format!(
            "recursion too deep calling `{name}`: exceeded {MAX_CALL_DEPTH} nested function calls"
        )));
    }

    let mapping: std::collections::HashMap<String, IRNode> =
        params.iter().cloned().zip(args).collect();
    let substituted = symbolic_vm::vm::substitute(body.clone(), &mapping);
    let result = eval_ir(ctx, &substituted);

    ctx.call_depth -= 1;
    result
}

/// Evaluate an already-lowered, pure [`IRNode`] tree (a substituted function
/// body) -- the counterpart to [`eval_expr`] for the one place this crate
/// evaluates `IRNode` directly rather than a `GrammarASTNode`.
///
/// Walks the **entire** tree itself (rather than handing it to
/// [`symbolic_vm::VM::eval`] in one shot) specifically so that a call to a
/// registered user function *at any position* inside the body -- an `if`
/// branch, an arithmetic operand, a list element -- is intercepted by this
/// same [`call_user_function`] depth-guarded path, not silently handed off
/// to the shared VM's own uncapped recursion. `If` is special-cased so only
/// the taken branch is evaluated (mirroring the shared VM's own held-head
/// treatment of `If`); every other head (arithmetic, comparison, `List`, an
/// unregistered/free call) has its arguments evaluated here first and is
/// then delegated to `VM::eval` for the actual operation, exactly as
/// `eval_expr`'s own arithmetic handling does.
fn eval_ir(ctx: &mut EvalContext, node: &IRNode) -> Result<IRNode, EvalError> {
    match node {
        IRNode::Apply(app) => {
            if let IRNode::Symbol(name) = &app.head {
                if let Some((params, body)) = ctx.functions.get(name).cloned() {
                    let mut arg_values = Vec::with_capacity(app.args.len());
                    for a in &app.args {
                        arg_values.push(eval_ir(ctx, a)?);
                    }
                    return call_user_function(ctx, name, &params, &body, arg_values);
                }
                if name == symbolic_ir::IF {
                    if app.args.len() != 3 {
                        return Err(EvalError::new("malformed `if` node"));
                    }
                    let predicate = eval_ir(ctx, &app.args[0])?;
                    return match &predicate {
                        IRNode::Symbol(s) if s == "True" => eval_ir(ctx, &app.args[1]),
                        IRNode::Symbol(s) if s == "False" => eval_ir(ctx, &app.args[2]),
                        other => Err(EvalError::new(format!(
                            "`if` predicate must evaluate to Boolean, got: {}",
                            print_axiom(other)
                        ))),
                    };
                }
            }
            let mut args = Vec::with_capacity(app.args.len());
            for a in &app.args {
                args.push(eval_ir(ctx, a)?);
            }
            Ok(ctx.vm.eval(apply(app.head.clone(), args)))
        }
        other => Ok(ctx.vm.eval(other.clone())),
    }
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
