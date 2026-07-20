//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **class-expression** printing cases — the
//! `class` keyword in value position (`ClassExpression`). This is the
//! twentieth CodePrinter port into `closure-emitter` (after core /
//! declarations / trailing-comma / numbers / string-escape / ascii-escape /
//! object-literal / function-expression / arrow-function / template / update /
//! new / sequence / tagged-template / spread / yield / await / this / super)
//! and isolates `emit_class` + `emit_class_member` + the `PREC_UNARY`
//! classification that landed with `Expression::ClassExpression` (CLOC12.173).
//!
//! # How the emitter prints a class expression (recap)
//!
//! A class expression prints `class[ id][ extends S]{members}` with **no**
//! separators between members (each carries its own `{…}`). One
//! [`MethodDefinition`] prints
//! `[static ][get|set ][async ][*]key(params){body}` — prefixes in grammar
//! order.
//!
//! ```text
//!   class {}                      → class{}
//!   class C {}                    → class C{}
//!   class C extends B {}          → class C extends B{}
//!   class { m() {} }              → class{m(){}}
//!   class { static get x() {} }   → class{static get x(){}}
//!   class { [k]() {} }            → class{[k](){}}
//! ```
//!
//! **Precedence.** `expr_prec` tags a class expression at `PREC_UNARY`
//! (exactly like a function expression) — below the `PREC_PRIMARY` at which a
//! member/call parent emits its base. So a class as a member object
//! (`(class{}).x`) or a call callee (`(class{})()`) is wrapped, while looser
//! assignment/binary parents leave it bare (`x=class{}`, `class{}+1`). At the
//! *start* of an expression statement a leading `class` parses as a class
//! *declaration*, so `emit_expression_statement` wraps it: `(class{});`. The
//! `extends` operand is emitted at `PREC_PRIMARY`: an identifier / member /
//! call heritage stays bare, a looser conditional heritage is wrapped.
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (the emitter is the unit
//! under test). The bridge conversion of `class_expression` (gap-167) lands in
//! CLOC12.173 PR2 and is exercised separately in `javascript-parser`; here the
//! emitter is driven from hand-constructed AST so this port does not depend on
//! the bridge. Building the AST directly also lets the port exercise shapes the
//! grammar cannot yet parse (generator / async / computed-key methods,
//! multi-member classes without `;` separators — see `CLOC12-gaps.md`).

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    BinaryExpression, BinaryOperator, BlockStatement, CallExpression, ClassExpression, ClassMember,
    ConditionalExpression, Expression, ExpressionStatement, FunctionExpression, FunctionParam,
    Identifier, MemberExpression, MethodDefinition, MethodKind, NumericLiteral, Program,
    ProgramItem, PropertyKey, ReturnStatement, SourceType, Statement,
};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support helpers
// =====================================================================

fn ident(name: &str) -> Expression {
    Expression::Identifier(Identifier { cv: None, name: name.to_string() })
}

fn num(value: f64, raw: &str) -> Expression {
    Expression::NumericLiteral(NumericLiteral { cv: None, value, raw: raw.to_string() })
}

fn member(object: Expression, property: &str) -> Expression {
    Expression::MemberExpression(MemberExpression {
        cv: None,
        object: Box::new(object),
        property: Box::new(ident(property)),
        computed: false,
    })
}

fn binary(op: BinaryOperator, left: Expression, right: Expression) -> Expression {
    Expression::BinaryExpression(BinaryExpression {
        cv: None,
        operator: op,
        left: Box::new(left),
        right: Box::new(right),
    })
}

fn call(callee: Expression, arguments: Vec<Expression>) -> Expression {
    Expression::CallExpression(CallExpression { cv: None, callee: Box::new(callee), arguments })
}

fn cond(test: Expression, consequent: Expression, alternate: Expression) -> Expression {
    Expression::ConditionalExpression(ConditionalExpression {
        cv: None,
        test: Box::new(test),
        consequent: Box::new(consequent),
        alternate: Box::new(alternate),
    })
}

fn ret(arg: Option<Expression>) -> Statement {
    Statement::return_statement(ReturnStatement { cv: None, argument: arg })
}

/// Build a method body as a `FunctionExpression` (params + block), with the
/// `generator` / `is_async` flags a method head may carry.
fn func(params: &[&str], body: Vec<Statement>, generator: bool, is_async: bool) -> FunctionExpression {
    FunctionExpression {
        cv: None,
        id: None,
        params: params
            .iter()
            .map(|p| FunctionParam::Identifier(Identifier { cv: None, name: p.to_string() }))
            .collect(),
        body: BlockStatement { cv: None, body },
        generator,
        is_async,
    }
}

/// Build one method member with an identifier key.
fn method(name: &str, kind: MethodKind, value: FunctionExpression, is_static: bool) -> ClassMember {
    ClassMember::Method(MethodDefinition {
        cv: None,
        key: PropertyKey::Identifier(Identifier { cv: None, name: name.to_string() }),
        kind,
        value,
        computed: false,
        is_static,
    })
}

/// Build a class expression: optional name, optional `extends` operand, members.
fn class(id: Option<&str>, super_class: Option<Expression>, body: Vec<ClassMember>) -> Expression {
    Expression::ClassExpression(ClassExpression {
        cv: None,
        id: id.map(|n| Identifier { cv: None, name: n.to_string() }),
        super_class: super_class.map(Box::new),
        body,
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
        "class emit output did not match\n  actual:   {:?}\n  expected: {:?};",
        code, expected
    );
}

// =====================================================================
// Active — statement-start parenthesisation
// =====================================================================

/// A bare anonymous class expression at the start of an expression statement
/// is wrapped, else the leading `class` parses as a class *declaration*.
#[test]
fn anonymous_at_statement_start_is_parenthesised() {
    assert_emits(class(None, None, vec![]), "(class{});");
}

/// A *named* class expression at statement start is wrapped the same way; the
/// name prints after `class` with a mandatory space.
#[test]
fn named_at_statement_start_is_parenthesised() {
    assert_emits(class(Some("C"), None, vec![]), "(class C{});");
}

// =====================================================================
// Active — the surface shape (driven from a call argument to avoid the
// statement-start wrap, so the bare class print is isolated)
// =====================================================================

/// `f(class{})` — an anonymous empty class as a call argument prints bare
/// (the argument context binds looser than the class's `PREC_UNARY`).
#[test]
fn empty_class_as_call_argument_is_bare() {
    assert_emits(call(ident("f"), vec![class(None, None, vec![])]), "f(class{});");
}

/// `f(class C{})` — a named class carries its name.
#[test]
fn named_class_as_call_argument() {
    assert_emits(call(ident("f"), vec![class(Some("C"), None, vec![])]), "f(class C{});");
}

// =====================================================================
// Active — `extends` heritage (operand emitted at PREC_PRIMARY)
// =====================================================================

/// `class C extends B{}` — an identifier heritage prints bare with the
/// mandatory spaces around `extends`.
#[test]
fn extends_identifier() {
    assert_emits(
        call(ident("f"), vec![class(Some("C"), Some(ident("B")), vec![])]),
        "f(class C extends B{});",
    );
}

/// `class extends ns.B{}` — a member heritage binds at primary and stays bare.
#[test]
fn extends_member_is_bare() {
    assert_emits(
        call(ident("f"), vec![class(None, Some(member(ident("ns"), "B")), vec![])]),
        "f(class extends ns.B{});",
    );
}

/// `class extends mixin(B){}` — a call heritage (`extends mixin(Base)`) binds
/// at primary strength and stays bare.
#[test]
fn extends_call_is_bare() {
    assert_emits(
        call(ident("f"), vec![class(None, Some(call(ident("mixin"), vec![ident("B")])), vec![])]),
        "f(class extends mixin(B){});",
    );
}

/// `class extends (a?b:c){}` — a conditional heritage binds looser than the
/// `PREC_PRIMARY` the operand is emitted at, so it is wrapped. The mandatory
/// space after `extends` is still printed before the wrapping paren.
#[test]
fn extends_conditional_is_wrapped() {
    assert_emits(
        call(
            ident("f"),
            vec![class(None, Some(cond(ident("a"), ident("b"), ident("c"))), vec![])],
        ),
        "f(class extends (a?b:c){});",
    );
}

// =====================================================================
// Active — member shapes
// =====================================================================

/// `class{m(){}}` — a single empty method prints inside the braces with no
/// separators.
#[test]
fn method_empty_body() {
    assert_emits(
        call(ident("f"), vec![class(None, None, vec![method("m", MethodKind::Method, func(&[], vec![], false, false), false)])]),
        "f(class{m(){}});",
    );
}

/// `class{m(a,b){return 1}}` — params are comma-separated and the body prints
/// between the braces.
#[test]
fn method_with_params_and_body() {
    assert_emits(
        call(
            ident("f"),
            vec![class(
                None,
                None,
                vec![method(
                    "m",
                    MethodKind::Method,
                    func(&["a", "b"], vec![ret(Some(num(1.0, "1")))], false, false),
                    false,
                )],
            )],
        ),
        "f(class{m(a,b){return 1}});",
    );
}

/// `class{static m(){}}` — a static member prints the `static ` prefix.
#[test]
fn static_method() {
    assert_emits(
        call(ident("f"), vec![class(None, None, vec![method("m", MethodKind::Method, func(&[], vec![], false, false), true)])]),
        "f(class{static m(){}});",
    );
}

/// `class{get x(){}}` — a getter prints the `get ` keyword before the key.
#[test]
fn getter() {
    assert_emits(
        call(ident("f"), vec![class(None, None, vec![method("x", MethodKind::Get, func(&[], vec![], false, false), false)])]),
        "f(class{get x(){}});",
    );
}

/// `class{set x(v){}}` — a setter prints the `set ` keyword and its one param.
#[test]
fn setter() {
    assert_emits(
        call(ident("f"), vec![class(None, None, vec![method("x", MethodKind::Set, func(&["v"], vec![], false, false), false)])]),
        "f(class{set x(v){}});",
    );
}

/// `class{constructor(){}}` — the constructor prints like a plain method (no
/// keyword prefix); its `kind` only matters to the passes.
#[test]
fn constructor() {
    assert_emits(
        call(ident("f"), vec![class(None, None, vec![method("constructor", MethodKind::Constructor, func(&[], vec![], false, false), false)])]),
        "f(class{constructor(){}});",
    );
}

/// `class{static get x(){}}` — prefixes stack in grammar order: `static`, then
/// the accessor keyword, then the key.
#[test]
fn static_getter() {
    assert_emits(
        call(ident("f"), vec![class(None, None, vec![method("x", MethodKind::Get, func(&[], vec![], false, false), true)])]),
        "f(class{static get x(){}});",
    );
}

/// `class{*m(){}}` — a generator method prints the `*` before the key. (The
/// grammar cannot yet parse a generator method, so the AST is hand-built.)
#[test]
fn generator_method() {
    assert_emits(
        call(ident("f"), vec![class(None, None, vec![method("m", MethodKind::Method, func(&[], vec![], true, false), false)])]),
        "f(class{*m(){}});",
    );
}

/// `class{async m(){}}` — an async method prints the `async ` prefix. (The
/// grammar cannot yet parse an async method, so the AST is hand-built.)
#[test]
fn async_method() {
    assert_emits(
        call(ident("f"), vec![class(None, None, vec![method("m", MethodKind::Method, func(&[], vec![], false, true), false)])]),
        "f(class{async m(){}});",
    );
}

/// `class{[k](){}}` — a computed-key method brackets the key expression.
#[test]
fn computed_key_method() {
    let member = ClassMember::Method(MethodDefinition {
        cv: None,
        key: PropertyKey::Expression(Box::new(ident("k"))),
        kind: MethodKind::Method,
        value: func(&[], vec![], false, false),
        computed: true,
        is_static: false,
    });
    assert_emits(
        call(ident("f"), vec![class(None, None, vec![member])]),
        "f(class{[k](){}});",
    );
}

/// `class{a(){}b(){}}` — two members print back-to-back with no separator
/// (each carries its own `{…}`). (The grammar requires `;` between members, so
/// this shape is hand-built — see `CLOC12-gaps.md`.)
#[test]
fn multiple_members_back_to_back() {
    assert_emits(
        call(
            ident("f"),
            vec![class(
                None,
                None,
                vec![
                    method("a", MethodKind::Method, func(&[], vec![], false, false), false),
                    method("b", MethodKind::Method, func(&[], vec![], false, false), false),
                ],
            )],
        ),
        "f(class{a(){}b(){}});",
    );
}

// =====================================================================
// Active — the whole node's precedence (class tags at PREC_UNARY)
// =====================================================================

/// `(class{}).x` — a member parent binds at primary strength; the looser class
/// object is wrapped.
#[test]
fn class_member_object_is_wrapped() {
    assert_emits(member(class(None, None, vec![]), "x"), "(class{}).x;");
}

/// `(class{})()` — a call callee likewise wraps the class.
#[test]
fn class_call_callee_is_wrapped() {
    assert_emits(call(class(None, None, vec![]), vec![]), "(class{})();");
}

/// `class{}+1` — a binary parent binds looser than the class's `PREC_UNARY`,
/// so the class stays bare on the left.
#[test]
fn class_under_binary_parent_is_bare() {
    assert_emits(binary(BinaryOperator::Add, class(None, None, vec![]), num(1.0, "1")), "class{}+1;");
}
