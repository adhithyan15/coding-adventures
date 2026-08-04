//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **static initialization block** printing cases —
//! a `static { … }` member (`ClassMember::StaticBlock`), the third (and last)
//! kind of class member alongside a method (`ClassMember::Method`) and a field
//! (`ClassMember::Field`). This is the twenty-third CodePrinter port into
//! `closure-emitter` (companion to the class-*expression* port
//! `code_printer_class_test.rs`, the class-*declaration* port
//! `code_printer_class_declaration_test.rs`, and the class-*field* port
//! `code_printer_class_field_test.rs`) and isolates `emit_static_block` + the
//! shared `emit_class_tail` member loop that grew a `StaticBlock` arm with
//! `ClassMember::StaticBlock` (CLOC12.176 PR1).
//!
//! # How the emitter prints a static block (recap)
//!
//! Inside a class body a static block prints `static{<statements>}`:
//!
//! ```text
//!   static { }         → static{}
//!   static { x }       → static{x}
//!   static { x = 1 }   → static{x=1}
//!   static { x; y }    → static{x;y}
//! ```
//!
//! The `static` keyword abuts the `{` with **no** space (the brace is a hard
//! token boundary, so `static{…}` parses). The body is emitted by the shared
//! `emit_block_statement`, which prints the `{ … }` and the `;`-separated
//! statement list exactly as a function body does. Like a *method* (and unlike a
//! *field*), a static block is **brace-terminated** — the `}` self-terminates,
//! so it needs no trailing `;` and abuts the following member directly.
//!
//! Note there is no key, no `static` *modifier* flag, and no parameter list: a
//! static block is `static` + a block, nothing else. (Unlike a method/field, the
//! `static` here is the block's own leading keyword, not an `is_static` flag on a
//! keyed member.)
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (the emitter is the unit
//! under test). The bridge conversion of a static block landed in CLOC12.176 PR2
//! and is exercised separately in `javascript-parser` + a `closurec` e2e diff
//! fixture; here the emitter is driven from hand-constructed AST so this port
//! does not depend on the bridge.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    AssignmentExpression, AssignmentOperator, AssignmentTarget, BlockStatement, ClassDeclaration,
    ClassMember, Declaration, Expression, ExpressionStatement, FunctionExpression, Identifier,
    MethodDefinition, MethodKind, NumericLiteral, Program, ProgramItem, PropertyDefinition,
    PropertyKey, SourceType, Statement,
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

/// A `PropertyKey::Identifier` key.
fn ident_key(name: &str) -> PropertyKey {
    PropertyKey::Identifier(Identifier { cv: None, name: name.to_string() })
}

/// An expression statement wrapping `expr` (`expr;`).
fn expr_stmt(expr: Expression) -> Statement {
    Statement::expression_statement(ExpressionStatement { cv: None, expression: expr })
}

/// A simple assignment `target=value` as an expression.
fn assign(target: &str, value: Expression) -> Expression {
    Expression::AssignmentExpression(AssignmentExpression {
        cv: None,
        operator: AssignmentOperator::Eq,
        left: AssignmentTarget::Identifier(Identifier { cv: None, name: target.to_string() }),
        right: Box::new(value),
    })
}

/// Build one `static { <statements> }` member.
fn static_block(stmts: Vec<Statement>) -> ClassMember {
    ClassMember::StaticBlock(BlockStatement { cv: None, body: stmts })
}

/// A no-op method value `(){}` (no params, empty body).
fn plain_method_value() -> FunctionExpression {
    FunctionExpression {
        cv: None,
        id: None,
        params: vec![],
        body: BlockStatement { cv: None, body: vec![] },
        generator: false,
        is_async: false,
    }
}

/// Build one plain method member (used to prove static-block/method interleave).
fn method(name: &str) -> ClassMember {
    ClassMember::Method(MethodDefinition {
        cv: None,
        key: ident_key(name),
        kind: MethodKind::Method,
        value: plain_method_value(),
        computed: false,
        is_static: false,
    })
}

/// Build one class **field** member (used to prove the three member kinds coexist).
fn field(name: &str, value: Option<Expression>) -> ClassMember {
    ClassMember::Field(PropertyDefinition {
        cv: None,
        key: ident_key(name),
        value,
        computed: false,
        is_static: false,
    })
}

/// Emit `class C{<members>}` as a top-level declaration, returning the code.
fn emit_body(body: Vec<ClassMember>) -> String {
    let decl = Declaration::ClassDeclaration(ClassDeclaration {
        cv: None,
        id: Identifier { cv: None, name: "C".to_string() },
        super_class: None,
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

/// Upstream `assertPrint(input, expected)` reshaped: emit a class `C` with the
/// given members and assert the emitted code equals `expected`.
fn assert_emits(body: Vec<ClassMember>, expected: &str) {
    let code = emit_body(body);
    assert_eq!(
        code, expected,
        "static-block emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — an empty static block prints `static{}`
// =====================================================================

/// `class C{static{}}` — the `static` keyword abuts the `{` with no space, and
/// the empty block is brace-terminated (no trailing `;`).
#[test]
fn empty_static_block() {
    assert_emits(vec![static_block(vec![])], "class C{static{}};");
}

/// The `static` keyword and the opening brace are adjacent — a regression guard
/// that no space slips between them (`static {}` would be wrong for minified
/// output).
#[test]
fn static_and_brace_are_adjacent() {
    let code = emit_body(vec![static_block(vec![])]);
    assert!(code.contains("static{"), "expected `static{{` with no gap, got {code:?}");
    assert!(!code.contains("static {"), "a space slipped between `static` and `{{`: {code:?}");
}

// =====================================================================
// Active — a body of statements prints inside the block
// =====================================================================

/// `class C{static{x}}` — a single expression statement prints inside the block.
#[test]
fn static_block_with_statement() {
    assert_emits(vec![static_block(vec![expr_stmt(ident("x"))])], "class C{static{x}};");
}

/// `class C{static{x=1}}` — a real initializer assignment, the canonical use of
/// a static block. The `=` has no surrounding spaces (minified).
#[test]
fn static_block_with_assignment() {
    assert_emits(
        vec![static_block(vec![expr_stmt(assign("x", num(1.0, "1")))])],
        "class C{static{x=1}};",
    );
}

/// `class C{static{x;y}}` — two statements are `;`-separated inside the block
/// (the same statement-list separator a function body uses).
#[test]
fn static_block_with_two_statements() {
    assert_emits(
        vec![static_block(vec![expr_stmt(ident("x")), expr_stmt(ident("y"))])],
        "class C{static{x;y}};",
    );
}

// =====================================================================
// Active — a static block is brace-terminated (no trailing `;`)
// =====================================================================

/// A static block, like a method and unlike a field, needs no `;` separator: the
/// `}` self-terminates. `class C{static{}m(){}}` — the block abuts the following
/// method with no separator.
#[test]
fn static_block_then_method_no_separator() {
    assert_emits(vec![static_block(vec![]), method("m")], "class C{static{}m(){}};");
}

/// `class C{m(){}static{}}` — a method then a static block: the method's `}`
/// abuts the `static` keyword with no separator.
#[test]
fn method_then_static_block_no_separator() {
    assert_emits(vec![method("m"), static_block(vec![])], "class C{m(){}static{}};");
}

/// Two static blocks back-to-back — `class C{static{}static{}}`. Each is
/// brace-terminated, so they abut with no separator (a legal ES2022 shape: a
/// class may hold multiple static blocks).
#[test]
fn two_static_blocks_back_to_back() {
    assert_emits(vec![static_block(vec![]), static_block(vec![])], "class C{static{}static{}};");
}

// =====================================================================
// Active — all three member kinds coexist in source order
// =====================================================================

/// `class C{x=1;static{y=2}m(){}}` — a field, a static block, and a method in
/// source order. Each prints its own terminator: the field's `;`, the block's
/// `}` (self-terminating), the method's `}`. Proves the shared `emit_class_tail`
/// member loop dispatches all three `ClassMember` arms correctly.
#[test]
fn field_static_block_method_interleave() {
    assert_emits(
        vec![
            field("x", Some(num(1.0, "1"))),
            static_block(vec![expr_stmt(assign("y", num(2.0, "2")))]),
            method("m"),
        ],
        "class C{x=1;static{y=2}m(){}};",
    );
}
