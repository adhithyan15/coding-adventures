//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **function-expression** printing cases —
//! a function used in *value* position (`testFunctionExpression*`, the
//! IIFE / parenthesisation cases, and the `function`-at-statement-start
//! disambiguation). This is the eighth CodePrinter port into
//! `closure-emitter` (alongside core / declarations / trailing-comma /
//! numbers / string-escape / ascii-escape / object-literal) and isolates
//! the `emit_function_expression` + precedence surface that landed with
//! `Expression::FunctionExpression` (CLOC12.149) and became reachable
//! end-to-end once the bridge converted `function_expression`
//! (gap-153, CLOC12.149 PR2).
//!
//! ## How the emitter prints a function expression (recap)
//!
//! ```text
//!   function () {}          anonymous, empty        → function(){}
//!   function f () {}        named (body-local)      → function f(){}
//!   function (a, b) {}      params                  → function(a,b){}
//!   function () { return 1; }  a body               → function(){return 1}
//!   function* () {}         generator               → function*(){}
//!   async function () {}    async                   → async function(){}
//! ```
//!
//! Two contexts mis-parse a *bare* leading/embedded function expression
//! and so trigger a paren wrap:
//!
//! ```text
//!   (function(){});          at expression-statement start (else a decl)
//!   (function(){})();        as a call callee  (function(){}() is invalid)
//!   (function(){}).x         as a member object (function(){}.x is invalid)
//! ```
//!
//! In a context that is already unambiguous — a call *argument*, an
//! assignment RHS — no parens are added:
//!
//! ```text
//!   g(function(){});         call argument            → g(function(){})
//! ```
//!
//! A function expression never prints a trailing `;` after its body `}`
//! (that normalisation is a function *declaration* rule); the only `;`
//! seen here belongs to the enclosing expression statement.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    BlockStatement, CallExpression, Expression, ExpressionStatement, FunctionExpression,
    FunctionParam, Identifier, MemberExpression, NumericLiteral, Program, ProgramItem,
    ReturnStatement, SourceType, Statement,
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

fn ret(arg: Option<Expression>) -> Statement {
    Statement::return_statement(ReturnStatement { cv: None, argument: arg })
}

/// Build a `FunctionExpression`: optional name, identifier params, body.
fn fexpr(id: Option<&str>, params: &[&str], body: Vec<Statement>) -> Expression {
    Expression::FunctionExpression(FunctionExpression {
        cv: None,
        id: id.map(|n| Identifier { cv: None, name: n.to_string() }),
        params: params
            .iter()
            .map(|p| FunctionParam::Identifier(Identifier { cv: None, name: p.to_string() }))
            .collect(),
        body: BlockStatement { cv: None, body },
        generator: false,
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

/// Upstream `assertPrint(input, expected)` reshaped: emit the expression
/// as a single-statement program and assert the emitted code equals
/// `expected`.
fn assert_emits(expr: Expression, expected: &str) {
    let code = emit_default(expr);
    assert_eq!(
        code, expected,
        "function-expression emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — statement-start parenthesisation
// =====================================================================

/// A bare anonymous function expression at the start of an expression
/// statement is wrapped, else the leading `function` parses as a
/// declaration.
#[test]
fn anonymous_at_statement_start_is_parenthesised() {
    assert_emits(fexpr(None, &[], vec![]), "(function(){});");
}

/// A *named* function expression at statement start is wrapped the same
/// way; the name is printed (body-local).
#[test]
fn named_at_statement_start_is_parenthesised() {
    assert_emits(fexpr(Some("f"), &[], vec![]), "(function f(){});");
}

// =====================================================================
// Active — params and body
// =====================================================================

/// Parameters are comma-separated with no interior whitespace.
#[test]
fn params_are_comma_separated() {
    assert_emits(fexpr(None, &["a", "b"], vec![]), "(function(a,b){});");
}

/// A body statement prints inside the braces; the last statement drops
/// its trailing `;` in compact mode, and the function expression itself
/// adds no `;` after `}`.
#[test]
fn return_body_prints_without_trailing_semicolons() {
    assert_emits(
        fexpr(None, &[], vec![ret(Some(num(1.0)))]),
        "(function(){return 1});",
    );
}

/// A bare `return;` inside the body.
#[test]
fn bare_return_body() {
    assert_emits(fexpr(None, &[], vec![ret(None)]), "(function(){return});");
}

// =====================================================================
// Active — call-callee / member-object parenthesisation (IIFE etc.)
// =====================================================================

/// An IIFE: the function-expression callee is wrapped because
/// `function(){}()` is a syntax error.
#[test]
fn iife_wraps_the_callee() {
    assert_emits(call(fexpr(None, &[], vec![]), vec![]), "(function(){})();");
}

/// An IIFE with an argument.
#[test]
fn iife_with_argument() {
    assert_emits(
        call(fexpr(None, &["a"], vec![]), vec![num(1.0)]),
        "(function(a){})(1);",
    );
}

/// A function expression as the object of a member access is wrapped —
/// `function(){}.x` is invalid.
#[test]
fn member_object_is_wrapped() {
    assert_emits(member(fexpr(None, &[], vec![]), ident("x")), "(function(){}).x;");
}

// =====================================================================
// Active — unambiguous contexts add no parens
// =====================================================================

/// As a call *argument* the function expression needs no parens, and no
/// stray `;` is appended after its body.
#[test]
fn call_argument_is_not_parenthesised() {
    assert_emits(
        call(ident("g"), vec![fexpr(None, &[], vec![])]),
        "g(function(){});",
    );
}

/// A *named* function expression as a call argument prints its name and
/// stays unparenthesised.
#[test]
fn named_call_argument_prints_name() {
    assert_emits(
        call(
            ident("use"),
            vec![fexpr(Some("f"), &["a"], vec![ret(Some(ident("a")))])],
        ),
        "use(function f(a){return a});",
    );
}

// =====================================================================
// Active — generator / async prefixes
// =====================================================================

/// A generator function expression prints `function*` (the `*` fuses
/// with no separating space).
#[test]
fn generator_prefix() {
    let mut g = fexpr(None, &[], vec![]);
    if let Expression::FunctionExpression(f) = &mut g {
        f.generator = true;
    }
    assert_emits(g, "(function*(){});");
}

/// An async function expression prints an `async` prefix (a required
/// space follows). Shown in a call argument to avoid also exercising the
/// statement-start wrap.
#[test]
fn async_prefix() {
    let mut a = fexpr(None, &[], vec![]);
    if let Expression::FunctionExpression(f) = &mut a {
        f.is_async = true;
    }
    assert_emits(call(ident("h"), vec![a]), "h(async function(){});");
}
