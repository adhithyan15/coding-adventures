//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **arrow-function** printing cases — the `=>`
//! form in value position. This is the ninth CodePrinter port into
//! `closure-emitter` (after core / declarations / trailing-comma / numbers /
//! string-escape / ascii-escape / object-literal / function-expression) and
//! isolates the `emit_arrow_function_expression` + precedence surface that
//! landed with `Expression::ArrowFunctionExpression` (CLOC12.151).
//!
//! ## How the emitter prints an arrow function (recap)
//!
//! ```text
//!   x => x                one param, concise body   → x=>x
//!   (a, b) => a           params, concise body      → (a,b)=>a
//!   () => 1               zero params, concise      → ()=>1
//!   () => {}              empty block body          → ()=>{}
//!   x => { return x; }    block body                → x=>{return x}
//!   async x => x          async, concise            → async x=>x
//! ```
//!
//! A single *plain identifier* param drops its parens (`x=>x`); zero and
//! two-or-more params keep them. A concise body that is an **object literal**
//! is wrapped so its leading `{` is not read as a block: `()=>({})`.
//!
//! Two contexts mis-parse a *bare* embedded arrow and so trigger a paren wrap
//! (an arrow is `PREC_ASSIGNMENT`, the loosest expression):
//!
//! ```text
//!   (()=>{})();           as a call callee  (()=>{}() is invalid)
//!   (()=>{}).x            as a member object (()=>{}.x is invalid)
//! ```
//!
//! In an already-unambiguous context — a call *argument* — no parens are
//! added, and unlike a function expression an arrow at expression-statement
//! *start* needs no wrap (`x=>x;` is a valid statement).
//!
//! ## Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (upstream lex/parses
//! source). That lets the port exercise **block-bodied** arrows
//! (`ArrowBody::Block`) which the emitter prints correctly even though the
//! current grammar can't yet parse them into the bridge (see CLOC12-gaps
//! gap-156) — the emitter is the unit under test here, not the parser.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    ArrowBody, ArrowFunctionExpression, BlockStatement, CallExpression, Expression,
    ExpressionStatement, FunctionParam, Identifier, MemberExpression, NumericLiteral,
    ObjectExpression, Program, ProgramItem, ReturnStatement, SourceType, Statement,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
// =====================================================================

fn ident(name: &str) -> Expression {
    Expression::Identifier(Identifier { cv: None, name: name.to_string() })
}

fn num(v: f64) -> Expression {
    Expression::NumericLiteral(NumericLiteral {
        cv: None,
        value: v,
        raw: format!("{}", v as i64),
    })
}

fn params(names: &[&str]) -> Vec<FunctionParam> {
    names
        .iter()
        .map(|p| FunctionParam::Identifier(Identifier { cv: None, name: p.to_string() }))
        .collect()
}

/// An arrow with a concise (expression) body.
fn arrow_concise(ps: &[&str], body: Expression) -> Expression {
    Expression::ArrowFunctionExpression(ArrowFunctionExpression {
        cv: None,
        params: params(ps),
        body: ArrowBody::Expression(Box::new(body)),
        is_async: false,
    })
}

/// An arrow with a block body.
fn arrow_block(ps: &[&str], body: Vec<Statement>) -> Expression {
    Expression::ArrowFunctionExpression(ArrowFunctionExpression {
        cv: None,
        params: params(ps),
        body: ArrowBody::Block(BlockStatement { cv: None, body }),
        is_async: false,
    })
}

fn call(callee: Expression, args: Vec<Expression>) -> Expression {
    Expression::CallExpression(CallExpression {
        cv: None,
        callee: Box::new(callee),
        arguments: args,
    })
}

fn member(object: Expression, property: Expression) -> Expression {
    Expression::MemberExpression(MemberExpression {
        cv: None,
        object: Box::new(object),
        property: Box::new(property),
        computed: false,
    })
}

fn stmt(expr: Expression) -> ProgramItem {
    ProgramItem::Statement(Statement::expression_statement(ExpressionStatement {
        cv: None,
        expression: expr,
    }))
}

fn emit_default(expr: Expression) -> String {
    let prog =
        Program::new_untraced(EsVersion::Es2025, SourceType::Module).with_body(vec![stmt(expr)]);
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    emit(&prog, &sidecar, &mut cv, &EmitOptions::default())
        .expect("emit failed")
        .code
}

/// Upstream `assertPrint(input, expected)` reshaped: emit the expression as a
/// single-statement program and assert the emitted code equals `expected`.
fn assert_emits(expr: Expression, expected: &str) {
    let code = emit_default(expr);
    assert_eq!(
        code, expected,
        "arrow-function emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — params + concise body
// =====================================================================

/// A single plain-identifier param drops its parens; an arrow at
/// expression-statement start needs NO wrap (unlike a function expression).
#[test]
fn single_param_concise_body_no_parens() {
    assert_emits(arrow_concise(&["x"], ident("x")), "x=>x;");
}

/// Zero params keep the empty parens; a concise body prints bare.
#[test]
fn zero_param_concise_body() {
    assert_emits(arrow_concise(&[], num(1.0)), "()=>1;");
}

/// Two-or-more params are parenthesised and comma-separated with no interior
/// whitespace.
#[test]
fn multi_param_concise_body() {
    assert_emits(arrow_concise(&["a", "b"], ident("a")), "(a,b)=>a;");
}

/// A concise body that is an object literal is wrapped so the leading `{`
/// isn't read as a block body.
#[test]
fn object_literal_concise_body_is_wrapped() {
    let obj = Expression::ObjectExpression(ObjectExpression { cv: None, properties: vec![] });
    assert_emits(arrow_concise(&[], obj), "()=>({});");
}

// =====================================================================
// Active — block body
// =====================================================================

/// An empty block body prints `{}` with no trailing `;` after the arrow.
#[test]
fn empty_block_body() {
    assert_emits(arrow_block(&[], vec![]), "()=>{};");
}

/// A block body with a `return`: the last statement drops its trailing `;` in
/// compact mode and the arrow adds none after `}`.
#[test]
fn block_body_return() {
    let body = vec![Statement::return_statement(ReturnStatement {
        cv: None,
        argument: Some(ident("x")),
    })];
    assert_emits(arrow_block(&["x"], body), "x=>{return x};");
}

// =====================================================================
// Active — call-callee / member-object parenthesisation
// =====================================================================

/// An IIFE arrow: the callee is wrapped because `()=>{}()` is a syntax error.
#[test]
fn iife_wraps_the_callee() {
    assert_emits(call(arrow_block(&[], vec![]), vec![]), "(()=>{})();");
}

/// An IIFE arrow with an argument.
#[test]
fn iife_with_argument() {
    assert_emits(call(arrow_concise(&["x"], ident("x")), vec![num(1.0)]), "(x=>x)(1);");
}

/// An arrow as a member object is wrapped — `()=>{}.x` is invalid.
#[test]
fn member_object_is_wrapped() {
    assert_emits(member(arrow_block(&[], vec![]), ident("x")), "(()=>{}).x;");
}

// =====================================================================
// Active — unambiguous contexts add no parens
// =====================================================================

/// As a call *argument* the arrow needs no parens (the loosest context).
#[test]
fn call_argument_is_not_parenthesised() {
    assert_emits(call(ident("g"), vec![arrow_concise(&["x"], ident("x"))]), "g(x=>x);");
}

// =====================================================================
// Active — async prefix
// =====================================================================

/// An `async` arrow needs a separating space before an unparenthesised
/// identifier param (`async x=>x`), else `async` and the param would merge.
#[test]
fn async_single_param() {
    let mut a = arrow_concise(&["x"], ident("x"));
    if let Expression::ArrowFunctionExpression(af) = &mut a {
        af.is_async = true;
    }
    assert_emits(a, "async x=>x;");
}

/// An `async` arrow with parenthesised params needs no space before `(`
/// (`async()=>{}`); shown as a call argument to avoid also exercising any
/// statement-start behaviour.
#[test]
fn async_paren_params_in_call_argument() {
    let mut a = arrow_block(&[], vec![]);
    if let Expression::ArrowFunctionExpression(af) = &mut a {
        af.is_async = true;
    }
    assert_emits(call(ident("h"), vec![a]), "h(async()=>{});");
}
