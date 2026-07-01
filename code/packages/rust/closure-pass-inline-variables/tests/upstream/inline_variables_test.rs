//! Ported from `InlineVariablesTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! The **first** CLOC12 port into `closure-pass-inline-variables`.
//! Upstream `InlineVariables` inlines any *effectively-constant* variable
//! (a `var`/`let`/`const` assigned exactly once and never reassigned),
//! substituting its value at every reference and then deleting the
//! declaration.
//!
//! closurec's `InlineVariablesPass` implements the provably-sound
//! **const-literal** core of that: it replaces uses of a `const` bound to
//! a **literal** with that literal, under a multi-use size budget, with
//! TDZ and shadowing soundness guards — and (a deliberate divergence) it
//! only *propagates*, leaving the now-dead `const X = …;` husk for the
//! downstream `remove-unused-vars` pass to delete. So the active cases
//! assert the husk **remains**, and the whole-`InlineVariables` behaviors
//! (single-assignment `let`/`var`, alias initializers, husk removal) are
//! pinned as `#[ignore = "blocked on gap-NNN"]` placeholders.
//!
//! ## Harness
//!
//! The crate carries `javascript-parser` + `closure-emitter` dev-deps, so
//! each case drives the **real** source → `grammar_to_program` bridge →
//! `InlineVariablesPass` → `emit` roundtrip. Each is
//! `assert_eq!(propagate(src), expected)` on emitted JS.
//!
//! NOTE on emit: assertions are against raw `closure-emitter` output —
//! binary operators keep spaces, statements end in `;`, and booleans use
//! Closure shorthand (`true` → `!0`, `false` → `!1`).

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_closure_pass_inline_variables::InlineVariablesPass;
use coding_adventures_closure_pass_pipeline::{Pass, PassContext};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_parser::{bridge, parse_javascript_typed};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support harness
// =====================================================================

/// Parse `src`, bridge to a typed `Program`, run `InlineVariablesPass`,
/// and emit the result as minified JS. Returns the emitted string.
fn propagate(src: &str) -> String {
    let es = EsVersion::Es2025;
    let node = parse_javascript_typed(src, es).expect("parse");
    let prog = bridge::grammar_to_program(&node, es).expect("bridge");

    let pass = InlineVariablesPass::new();
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    let out = pass
        .run(PassContext {
            program: &prog,
            sidecar: &sidecar,
            cv: &mut cv,
        })
        .expect("inline-variables");

    let mut cv2 = CVLog::new(false);
    let opts = EmitOptions {
        source_map: false,
        ..Default::default()
    };
    emit(&out.program, &sidecar, &mut cv2, &opts)
        .expect("emit")
        .code
}

// =====================================================================
// Active — const-literal propagation our pass performs today.
// (The dead `const X=…;` husk is EXPECTED to remain — see module docs.)
// =====================================================================

/// Upstream `testSimpleInlineConst` (const-literal subset): a single use
/// of a literal `const` is replaced by the literal.
#[test]
fn inline_single_use_const_literal() {
    assert_eq!(propagate("const RATE = 2; use(RATE);"), "const RATE=2;use(2);");
}

/// The literal is substituted inside a larger expression.
#[test]
fn inline_const_into_expression() {
    assert_eq!(
        propagate("const RATE = 2; total(base * RATE);"),
        "const RATE=2;total(base*2);"
    );
}

/// A short literal is worth duplicating across multiple sites.
#[test]
fn inline_short_literal_at_multiple_sites() {
    assert_eq!(
        propagate("const N = 3; a(N); b(N); c(N);"),
        "const N=3;a(3);b(3);c(3);"
    );
}

/// Boolean and null literals propagate (booleans via Closure shorthand).
#[test]
fn inline_boolean_and_null_literals() {
    assert_eq!(
        propagate("const ON = true; const NONE = null; f(ON, NONE);"),
        "const ON=!0;const NONE=null;f(!0,null);"
    );
}

/// Upstream `testNoInlineConstantWithComplexValue`-style budget: a long
/// literal used at MULTIPLE sites is not worth duplicating — declined.
#[test]
fn does_not_inline_long_literal_at_multiple_sites() {
    assert_eq!(
        propagate("const MSG = \"a long message value\"; a(MSG); b(MSG);"),
        "const MSG=\"a long message value\";a(MSG);b(MSG);"
    );
}

/// A long literal at a SINGLE site is always worth it.
#[test]
fn inline_long_literal_at_single_site() {
    assert_eq!(
        propagate("const MSG = \"a long message value\"; a(MSG);"),
        "const MSG=\"a long message value\";a(\"a long message value\");"
    );
}

/// `let` / `var` are reassignable — never propagated by this pass.
#[test]
fn does_not_inline_let_or_var() {
    assert_eq!(propagate("let X = 5; use(X);"), "let X=5;use(X);");
    assert_eq!(propagate("var Y = 5; use(Y);"), "var Y=5;use(Y);");
}

/// A non-literal initializer (identifier alias / call) is declined —
/// could be reassigned or have side effects.
#[test]
fn does_not_inline_non_literal_value() {
    assert_eq!(propagate("const X = other; use(X);"), "const X=other;use(X);");
    assert_eq!(propagate("const X = make(); use(X);"), "const X=make();use(X);");
}

/// Upstream `testNoInlineWithShadow`: a name declared twice (const plus a
/// function parameter of the same name) is declined — a use could resolve
/// to either binding.
#[test]
fn does_not_inline_shadowed_name() {
    assert_eq!(
        propagate("const RATE = 2; function f(RATE) { return RATE; }"),
        "const RATE=2;function f(RATE){return RATE};"
    );
}

/// A property name (`obj.RATE`) is not a use of the const — not replaced.
#[test]
fn does_not_replace_property_name() {
    assert_eq!(
        propagate("const RATE = 2; use(obj.RATE);"),
        "const RATE=2;use(obj.RATE);"
    );
}

/// A computed member `obj[RATE]` IS a use position — replaced.
#[test]
fn replaces_computed_member_index() {
    assert_eq!(
        propagate("const RATE = 2; use(obj[RATE]);"),
        "const RATE=2;use(obj[2]);"
    );
}

/// Soundness (TDZ): with an inert prefix (only literal `const`s before
/// it), a `const` is safe to propagate.
#[test]
fn inline_through_inert_const_prefix() {
    assert_eq!(
        propagate("const A = 1; const X = 2; f(A, X);"),
        "const A=1;const X=2;f(1,2);"
    );
}

/// Soundness (TDZ): a top-level call `g()` runs before `const X`
/// initializes; `g` could read `X` in its temporal dead zone and throw.
/// Propagating the literal would erase that throw — declined.
#[test]
fn does_not_inline_when_code_runs_before_declaration() {
    assert_eq!(propagate("g(); const X = 5; use(X);"), "g();const X=5;use(X);");
}

// =====================================================================
// Ignored — whole-`InlineVariables` behaviors we do not do yet.
// The `expected` strings encode the upstream target, so flipping
// `#[ignore]` off is a one-line change once the gap closes.
// =====================================================================

/// Upstream inlines a single-assignment `let`/`var` (assigned once, never
/// reassigned) and removes the declaration. Ours only propagates `const`.
#[test]
#[ignore = "blocked on gap-148: only const literals propagate; single-assignment let/var not inlined"]
fn inline_single_assignment_let() {
    assert_eq!(propagate("let X = 5; use(X);"), "use(5);");
}

/// Upstream inlines an identifier-alias initializer (`const A = B` → uses
/// of `A` become `B`) when the alias target is not reassigned.
#[test]
#[ignore = "blocked on gap-149: identifier-alias initializers not inlined"]
fn inline_identifier_alias() {
    assert_eq!(propagate("const A = B; use(A);"), "use(B);");
}

/// Upstream removes the dead declaration once every reference is inlined.
/// Ours leaves the husk for `remove-unused-vars` (so today the const
/// remains — see the active `inline_single_use_const_literal`).
#[test]
#[ignore = "blocked on gap-150: dead const declaration husk not removed (remove-unused-vars owns that)"]
fn removes_dead_const_after_inlining() {
    assert_eq!(propagate("const R = 2; use(R);"), "use(2);");
}
