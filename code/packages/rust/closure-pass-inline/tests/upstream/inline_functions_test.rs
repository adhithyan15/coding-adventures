//! Ported from `InlineFunctionsTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! CLOC12 port for the `inline` pass. Upstream `InlineFunctions` is a
//! large pass; our `InlinePass` implements the provably-sound core:
//! it substitutes the body of a `return <expr>;` function at its call
//! site(s) — single-use always, multi-use under a conservative size
//! budget — when every argument is safe to substitute and the body has
//! no free identifiers beyond the parameters. See the crate docs.
//!
//! Like the crate's own behaviour tests, each case drives the real
//! `source → bridge → inline → emit` chain (closurec's SIMPLE path) and
//! asserts on the emitted string — the same surface upstream uses
//! (`test("function f(){return 1} f()", "1")`). Two caveats vs. the Java
//! oracle, both intrinsic to running ONLY this pass:
//!
//!   1. The dead callee declaration is left in place — `remove-unused-
//!      vars` / `treeshake` delete it downstream. So where upstream
//!      shows just the inlined call site, we also see the retained
//!      `function …{…};` prefix.
//!   2. No constant-folding runs after, so `d(2)` inlines to `2*2`
//!      (not `4`) — folding is `constant-fold`'s job, a separate pass.
//!
//! Behaviors upstream supports that our slice does not are recorded as
//! `#[ignore = "blocked on gap-NNN"]` placeholders pinned to
//! `code/specs/CLOC12-gaps.md` (gap-127 … gap-131).

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_closure_pass_inline::InlinePass;
use coding_adventures_closure_pass_pipeline::{Pass, PassContext};
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_parser::{bridge, parse_javascript_typed};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

/// Parse `src`, bridge to a typed `Program`, run **only** `InlinePass`,
/// and emit the result as minified JS — the same chain closurec's SIMPLE
/// level uses. (Copied from the crate's own test helper; integration
/// tests can't see the private one.)
fn inline_source(src: &str) -> String {
    let es = EsVersion::Es2025;
    let node = parse_javascript_typed(src, es).expect("parse");
    let prog = bridge::grammar_to_program(&node, es).expect("bridge");

    let pass = InlinePass::new();
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    let out = pass
        .run(PassContext {
            program: &prog,
            sidecar: &sidecar,
            cv: &mut cv,
        })
        .expect("inline");

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
// Active ports — behaviors `InlinePass` supports today.
// =====================================================================

#[test]
fn inlines_zero_param_constant_return() {
    // upstream `inlineFunctions1`-style: `function f(){return 1} f()`
    // → the call becomes `1`.
    assert_eq!(
        inline_source("function f() { return 1; } g(f());"),
        "function f(){return 1};g(1);"
    );
}

#[test]
fn inlines_string_literal_return() {
    assert_eq!(
        inline_source("function s() { return \"hi\"; } g(s());"),
        "function s(){return\"hi\"};g(\"hi\");"
    );
}

#[test]
fn inlines_zero_param_body_at_two_sites() {
    // A tiny (1-node) body is under the multi-use budget, so both call
    // sites are inlined.
    assert_eq!(
        inline_source("function one() { return 1; } a(one()); b(one());"),
        "function one(){return 1};a(1);b(1);"
    );
}

#[test]
fn substitutes_argument_into_member_object() {
    // `o` → `arr`, but `.length` (the property name) is untouched.
    assert_eq!(
        inline_source("function len(o) { return o.length; } use(len(arr));"),
        "function len(o){return o.length};use(arr.length);"
    );
}

#[test]
fn inlines_call_nested_in_binary_expression() {
    assert_eq!(
        inline_source("function id(v) { return v; } use(id(a) + id(b));"),
        "function id(v){return v};use(a+b);"
    );
}

#[test]
fn does_not_inline_when_a_use_is_not_a_call() {
    // `f` used as a value (`keep(f)`) blocks inlining: substituting the
    // calls would leave `f` referenced, so the decl couldn't be removed.
    assert_eq!(
        inline_source("function f(x) { return x * 2; } a(f(1)); keep(f);"),
        "function f(x){return x*2};a(f(1));keep(f);"
    );
}

#[test]
fn does_not_inline_multi_use_over_budget() {
    // `x * x * x` (5 nodes) exceeds the 1-param budget (2 + 1), so two
    // call sites are declined to avoid output growth.
    assert_eq!(
        inline_source("function cube(x) { return x * x * x; } a(cube(p)); b(cube(q));"),
        "function cube(x){return x*x*x};a(cube(p));b(cube(q));"
    );
}

// =====================================================================
// Not-yet-supported upstream behaviors — pinned to CLOC12-gaps.md.
// =====================================================================

#[test]
#[ignore = "blocked on gap-127: inlining a function with local declarations (var/let in the body)"]
fn inlines_function_with_local_variable() {
    // upstream: `function f(x){ var y = x + 1; return y; } g(f(2));`
    // inlines to the equivalent expression. Our slice only handles a
    // single `return <expr>;` body with no locals.
}

#[test]
#[ignore = "blocked on gap-128: inlining a method that references `this`"]
fn inlines_method_using_this() {
    // upstream inlines simple `this`-using method bodies at call sites
    // where the receiver is known. Our slice bails on any free
    // identifier (including `this`).
}

#[test]
#[ignore = "blocked on gap-129: inlining a function expression / arrow assigned to a variable"]
fn inlines_function_expression_binding() {
    // upstream: `var f = function(x){return x*2}; g(f(3));` and the
    // arrow form inline. Our slice only recognizes `function` *declarations*.
}

#[test]
#[ignore = "blocked on gap-130: inlining a void (no-return) function called for its side effect only"]
fn inlines_void_function_statement_call() {
    // upstream: `function log(x){ console.log(x); } log(v);` inlines the
    // body as a statement. Our slice targets value-position `return`
    // bodies; broader void-statement inlining is future work.
}

#[test]
#[ignore = "blocked on gap-131: inlining declines must cover a self-referential (recursive) callee explicitly"]
fn does_not_inline_recursive_function() {
    // upstream leaves `function f(x){return f(x)} g(f(1));` alone. Our
    // slice happens to decline it (the body's free `f` reference fails
    // the no-free-identifier gate), but there is no dedicated recursion
    // guard — pinned so a future change that widens the free-identifier
    // rule can't silently start inlining a recursive body.
}

#[test]
#[ignore = "blocked on gap-132: inlining a COMPOUND (non-leaf) argument expression"]
fn inlines_compound_argument_with_precedence_parens() {
    // GAP FOUND BY THIS PORT. upstream inlines
    // `function d(x){return x*2} g(d(a+b));` → `g((a+b)*2);`, adding the
    // precedence-preserving parens. Our slice substitutes only *simple*
    // arguments (a bare identifier or a literal); a compound argument
    // like `a + b` is declined and the call is left intact
    // (`g(d(a+b));`) — a conservative miss, not a miscompile. Closing
    // this needs: (1) allow a compound arg when its parameter is used
    // exactly once in the body (so it isn't duplicated / re-evaluated),
    // and (2) parenthesize the substituted expression against the
    // surrounding operator's precedence. See `code/specs/CLOC12-gaps.md`.
    //
    // When implemented, this becomes:
    //   assert_eq!(
    //       inline_source("function d(x) { return x * 2; } g(d(a + b));"),
    //       "function d(x){return x*2};g((a+b)*2);"
    //   );
}
