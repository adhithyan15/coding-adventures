//! Ported from `CodePrinterTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! Upstream `CodePrinterTest`'s **object spread** `{...o}` printing cases — the
//! object-spread member (`ObjectMember::Spread`, ES2018). This is the
//! twenty-third CodePrinter port into `closure-emitter` (after core /
//! declarations / trailing-comma / numbers / string-escape / ascii-escape /
//! object-literal / function-expression / arrow-function / template / update /
//! new / sequence / tagged-template / spread / yield / await / this / super /
//! new.target / import.meta / import-expression) and isolates
//! `emit_object_spread` + the member-iteration in `emit_object` that landed with
//! `ObjectMember` (CLOC12.170).
//!
//! # How the emitter prints an object spread (recap)
//!
//! An object literal's `properties` list is a `Vec<ObjectMember>` intermixing
//! normal `Property` members and `Spread` members. A spread prints `...` then
//! its `argument` at `PREC_ASSIGNMENT` with no interior space — identical in
//! shape to the call/array spread. Two facts drive the printing:
//!
//!   1. **The argument sits in a member (assignment) position.** Everything at
//!      or above assignment strength prints bare (`{...a}`, `{...a.b}`,
//!      `{...f()}`); only a looser *sequence* argument must wrap (`{...(a,b)}`),
//!      because a bare `...a,b` would spread only `a` and leave `,b` as a second
//!      (empty-keyed, invalid) member slot.
//!   2. **Member order is preserved.** `{a: 1, ...b}` and `{...a, b: 1}` print
//!      their members in source order — the interleaving is observable, since a
//!      later member overrides an earlier key.
//!
//! ```text
//!   {...a}          → {...a}          bare spread member
//!   {...a, b: 1}    → {...a,b:1}      spread then a normal member
//!   {a: 1, ...b}    → {a:1,...b}      normal member then a spread
//!   {...f()}        → {...f()}        a call binds tighter than a comma → bare
//!   {...(a, b)}     → {...(a,b)}      a sequence must wrap
//! ```
//!
//! An object at statement-start needs parens (a leading `{` would parse as a
//! block), so the cases below assert the parenthesised `({...})` form — the
//! `...` printing is what they isolate.
//!
//! # Note on hand-constructed inputs
//!
//! These assertions build typed-AST inputs directly (the emitter is the unit
//! under test). The bridge conversion of `{...o}` (gap-SpreadProperty) lands in
//! CLOC12.170 PR2 and is exercised separately in `javascript-parser`; here the
//! emitter is driven from hand-constructed AST so this port does not depend on
//! the bridge.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_ast::{
    CallExpression, Expression, ExpressionStatement, Identifier, NumericLiteral, ObjectExpression,
    ObjectMember, Program, ProgramItem, Property, PropertyKey, PropertyKind, SequenceExpression,
    SourceType, SpreadElement, Statement,
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

/// A `...arg` spread member.
fn spread(arg: Expression) -> ObjectMember {
    ObjectMember::Spread(SpreadElement { cv: None, argument: Box::new(arg) })
}

/// A plain `name: value` init member.
fn init(name: &str, value: Expression) -> ObjectMember {
    ObjectMember::Property(Property {
        cv: None,
        kind: PropertyKind::Init,
        key: PropertyKey::Identifier(Identifier { cv: None, name: name.to_string() }),
        value: Box::new(value),
        computed: false,
        shorthand: false,
        method: false,
    })
}

fn object(members: Vec<ObjectMember>) -> Expression {
    Expression::ObjectExpression(ObjectExpression { cv: None, properties: members })
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

/// Upstream `assertPrint(input, expected)` reshaped: emit the object literal as
/// a single-statement program and assert the emitted code equals `expected`.
/// (The object at statement start is parenthesised — a leading `{` would parse
/// as a block — so `expected` carries the `(...)`.)
fn assert_emits(members: Vec<ObjectMember>, expected: &str) {
    let code = emit_default(object(members));
    assert_eq!(
        code, expected,
        "object-spread emit output did not match\n  actual:   {:?}\n  expected: {:?}",
        code, expected
    );
}

// =====================================================================
// Active — the spread argument prints at PREC_ASSIGNMENT inside the braces
// =====================================================================

/// `{...a}` — a bare identifier spread member.
#[test]
fn object_spread_sole_member_is_bare() {
    assert_emits(vec![spread(ident("a"))], "({...a});");
}

/// `{...a, b: 1}` — a spread then a normal member; source order preserved.
#[test]
fn object_spread_before_property() {
    assert_emits(vec![spread(ident("a")), init("b", num(1.0, "1"))], "({...a,b:1});");
}

/// `{a: 1, ...b}` — a normal member then a spread.
#[test]
fn object_spread_after_property() {
    assert_emits(vec![init("a", num(1.0, "1")), spread(ident("b"))], "({a:1,...b});");
}

/// `{...f()}` — a call binds tighter than the member comma, so no wrap.
#[test]
fn object_spread_call_argument_is_bare() {
    let call = Expression::CallExpression(CallExpression {
        cv: None,
        callee: Box::new(ident("f")),
        arguments: vec![],
    });
    assert_emits(vec![spread(call)], "({...f()});");
}

// =====================================================================
// Active — a looser (sequence) argument must wrap
// =====================================================================

/// `{...(a, b)}` — a sequence is looser than the member comma, so it must wrap;
/// a bare `...a,b` would spread only `a` and leave `,b` as a second (invalid)
/// member slot.
#[test]
fn object_spread_sequence_argument_wraps() {
    let seq = Expression::SequenceExpression(SequenceExpression {
        cv: None,
        expressions: vec![ident("a"), ident("b")],
    });
    assert_emits(vec![spread(seq)], "({...(a,b)});");
}
