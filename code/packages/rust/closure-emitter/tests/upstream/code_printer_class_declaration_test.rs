//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **class-declaration** printing cases — the
//! `class` keyword in *statement* position (`ClassDeclaration`). This is the
//! twenty-first CodePrinter port into `closure-emitter` (the companion to the
//! class-*expression* port, `code_printer_class_test.rs`) and isolates
//! `emit_class_declaration` + the shared `emit_class_tail` helper that landed
//! with `Declaration::ClassDeclaration` (CLOC12.174 PR1).
//!
//! # How the emitter prints a class declaration (recap)
//!
//! A class declaration prints `class <id>[ extends S]{members}` — the same body
//! shape as a class *expression*, with three deliberate differences, each the
//! mirror of the `FunctionDeclaration` vs `FunctionExpression` split:
//!
//! ```text
//!   class C {}                    → class C{}
//!   class C extends B {}          → class C extends B{}
//!   class C { m() {} }            → class C{m(){}}
//!   class C { static get x(){} }  → class C{static get x(){}}
//!   class C { [k]() {} }          → class C{[k](){}}
//! ```
//!
//!   1. **`id` always prints** (a declaration must bind a name), with a
//!      mandatory space after `class`.
//!   2. **No precedence wrap / no statement-start parenthesis.** A class
//!      *expression* at statement start is wrapped (`(class{});`) because a
//!      leading `class` parses as a *declaration* — which is exactly what this
//!      node IS. So the declaration form is emitted bare.
//!   3. **No trailing `;`.** A `function` declaration appends a normalising `;`
//!      (gap-030); a `class` declaration terminates with its `}` alone.
//!
//! The `extends` operand is emitted at `PREC_PRIMARY`: an identifier / member /
//! call heritage stays bare, a looser conditional heritage is wrapped.
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (the emitter is the unit
//! under test). The bridge conversion of `class_declaration` lands in
//! CLOC12.174 PR2 and is exercised separately in `javascript-parser`; here the
//! emitter is driven from hand-constructed AST so this port does not depend on
//! the bridge. Building the AST directly also lets the port exercise member
//! shapes the grammar cannot yet parse (generator / async / computed-key
//! methods, multi-member classes without `;` separators — see `CLOC12-gaps.md`).

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    BinaryExpression, BinaryOperator, BlockStatement, CallExpression, ClassDeclaration, ClassMember,
    ConditionalExpression, Declaration, Expression, FunctionExpression, FunctionParam, Identifier,
    MemberExpression, MethodDefinition, MethodKind, NumericLiteral, Program, ProgramItem,
    PropertyKey, ReturnStatement, SourceType, Statement,
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
fn func(
    params: &[&str],
    body: Vec<Statement>,
    generator: bool,
    is_async: bool,
) -> FunctionExpression {
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

/// A no-op method value `(){}` (no params, empty body, not generator/async).
fn plain() -> FunctionExpression {
    func(&[], vec![], false, false)
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

/// Build one method member with a *computed* key `[expr]`.
fn computed_method(key: Expression, value: FunctionExpression) -> ClassMember {
    ClassMember::Method(MethodDefinition {
        cv: None,
        key: PropertyKey::Expression(Box::new(key)),
        kind: MethodKind::Method,
        value,
        computed: true,
        is_static: false,
    })
}

/// Emit `class <id>[ extends S]{members}` as a top-level **declaration** program
/// item, returning the emitted code.
fn emit_decl(id: &str, super_class: Option<Expression>, body: Vec<ClassMember>) -> String {
    let decl = Declaration::ClassDeclaration(ClassDeclaration {
        cv: None,
        id: Identifier { cv: None, name: id.to_string() },
        super_class: super_class.map(Box::new),
        body,
    });
    let prog = Program::new_untraced(EsVersion::Es2025, SourceType::Module)
        .with_body(vec![ProgramItem::Declaration(decl)]);
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    emit(&prog, &sidecar, &mut cv, &EmitOptions::default())
        .expect("emit failed")
        .code
}

/// Upstream `assertPrint(input, expected)` reshaped: emit the class declaration
/// as a single-item program and assert the emitted code equals `expected`.
fn assert_emits(id: &str, super_class: Option<Expression>, body: Vec<ClassMember>, expected: &str) {
    let code = emit_decl(id, super_class, body);
    assert_eq!(
        code, expected,
        "class-declaration emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — the declaration is bare (no wrap, no trailing `;`)
// =====================================================================

/// `class C{}` — the minimal declaration. Emitted bare: the name prints after
/// `class` with a mandatory space, and the `}` terminates it with **no**
/// trailing `;` (unlike a function declaration) and **no** wrapping paren
/// (unlike the class *expression* form, which at statement start is `(class …);`).
#[test]
fn empty_class_declaration_is_bare() {
    assert_emits("C", None, vec![], "class C{}");
}

/// The bare shape is exactly the emitted string — a regression guard on both
/// the missing trailing `;` and the missing wrap, together.
#[test]
fn empty_declaration_has_no_semicolon_and_no_paren() {
    let code = emit_decl("C", None, vec![]);
    assert!(code.ends_with('}'), "must terminate with }}, got {code:?}");
    assert!(!code.ends_with("};"), "must NOT append a trailing ;, got {code:?}");
    assert!(!code.starts_with('('), "must NOT be wrapped in parens, got {code:?}");
}

// =====================================================================
// Active — `extends` heritage (operand emitted at PREC_PRIMARY)
// =====================================================================

/// `class C extends B{}` — an identifier heritage prints bare with the
/// mandatory spaces around `extends`.
#[test]
fn extends_identifier() {
    assert_emits("C", Some(ident("B")), vec![], "class C extends B{}");
}

/// `class C extends ns.B{}` — a member heritage binds at primary and stays bare.
#[test]
fn extends_member_is_bare() {
    assert_emits("C", Some(member(ident("ns"), "B")), vec![], "class C extends ns.B{}");
}

/// `class C extends mixin(B){}` — a call heritage (`extends mixin(Base)`) binds
/// at primary strength and stays bare.
#[test]
fn extends_call_is_bare() {
    assert_emits(
        "C",
        Some(call(ident("mixin"), vec![ident("B")])),
        vec![],
        "class C extends mixin(B){}",
    );
}

/// `class C extends (a?b:c){}` — a conditional heritage binds looser than
/// `PREC_PRIMARY`, so it is wrapped (with the mandatory space after `extends`).
#[test]
fn extends_conditional_is_wrapped() {
    assert_emits(
        "C",
        Some(cond(ident("a"), ident("b"), ident("c"))),
        vec![],
        "class C extends (a?b:c){}",
    );
}

// =====================================================================
// Active — members (`emit_class_member`, shared with the expression form)
// =====================================================================

/// `class C{m(){}}` — one plain method, no separators.
#[test]
fn one_method() {
    assert_emits("C", None, vec![method("m", MethodKind::Method, plain(), false)], "class C{m(){}}");
}

/// `class C{m(x){return x}}` — params + a body statement print through the
/// shared function-emit path.
#[test]
fn method_with_params_and_body() {
    assert_emits(
        "C",
        None,
        vec![method("m", MethodKind::Method, func(&["x"], vec![ret(Some(ident("x")))], false, false), false)],
        "class C{m(x){return x}}",
    );
}

/// `class C{static m(){}}` — a `static` member prints the keyword first.
#[test]
fn static_method() {
    assert_emits("C", None, vec![method("m", MethodKind::Method, plain(), true)], "class C{static m(){}}");
}

/// `class C{get x(){}}` — a getter accessor.
#[test]
fn getter() {
    assert_emits("C", None, vec![method("x", MethodKind::Get, plain(), false)], "class C{get x(){}}");
}

/// `class C{set x(v){}}` — a setter accessor with its one parameter.
#[test]
fn setter() {
    assert_emits(
        "C",
        None,
        vec![method("x", MethodKind::Set, func(&["v"], vec![], false, false), false)],
        "class C{set x(v){}}",
    );
}

/// `class C{constructor(){}}` — the constructor prints with no keyword prefix
/// (its `kind` matters only to the passes, not the emitter).
#[test]
fn constructor() {
    assert_emits(
        "C",
        None,
        vec![method("constructor", MethodKind::Constructor, plain(), false)],
        "class C{constructor(){}}",
    );
}

/// `class C{static get x(){}}` — stacked `static` + accessor prefixes, in
/// grammar order (`static` first, then `get`).
#[test]
fn static_getter() {
    assert_emits(
        "C",
        None,
        vec![method("x", MethodKind::Get, plain(), true)],
        "class C{static get x(){}}",
    );
}

/// `class C{*m(){}}` — a generator method prints the `*` before the key.
#[test]
fn generator_method() {
    assert_emits(
        "C",
        None,
        vec![method("m", MethodKind::Method, func(&[], vec![], true, false), false)],
        "class C{*m(){}}",
    );
}

/// `class C{async m(){}}` — an async method prints the `async` keyword (with a
/// space) before the key.
#[test]
fn async_method() {
    assert_emits(
        "C",
        None,
        vec![method("m", MethodKind::Method, func(&[], vec![], false, true), false)],
        "class C{async m(){}}",
    );
}

/// `class C{[k](){}}` — a computed key is bracketed.
#[test]
fn computed_key_method() {
    assert_emits("C", None, vec![computed_method(ident("k"), plain())], "class C{[k](){}}");
}

/// `class C{a(){}b(){}}` — two members print back-to-back with no separator
/// (each carries its own `{…}`). A grammar-unparseable shape the port covers
/// via hand-constructed AST.
#[test]
fn two_members_back_to_back() {
    assert_emits(
        "C",
        None,
        vec![
            method("a", MethodKind::Method, plain(), false),
            method("b", MethodKind::Method, plain(), false),
        ],
        "class C{a(){}b(){}}",
    );
}

// =====================================================================
// Active — the whole shape together
// =====================================================================

/// `class C extends B{m(){}}` — name + heritage + one member in one assertion.
#[test]
fn full_shape_named_heritage_and_member() {
    assert_emits(
        "C",
        Some(ident("B")),
        vec![method("m", MethodKind::Method, plain(), false)],
        "class C extends B{m(){}}",
    );
}

/// A numeric-literal computed key `[0]` (exercising the `num` helper) prints
/// the literal inside the brackets.
#[test]
fn computed_numeric_key() {
    assert_emits(
        "C",
        None,
        vec![computed_method(num(0.0, "0"), plain())],
        "class C{[0](){}}",
    );
}

/// A `+`-binary computed key `[a+b]` — the key expression prints inside the
/// brackets. Confirms the computed-key path recurses into an arbitrary
/// expression, using the `binary` helper.
#[test]
fn computed_binary_key() {
    assert_emits(
        "C",
        None,
        vec![computed_method(
            Expression::BinaryExpression(BinaryExpression {
                cv: None,
                operator: BinaryOperator::Add,
                left: Box::new(ident("a")),
                right: Box::new(ident("b")),
            }),
            plain(),
        )],
        "class C{[a+b](){}}",
    );
}
