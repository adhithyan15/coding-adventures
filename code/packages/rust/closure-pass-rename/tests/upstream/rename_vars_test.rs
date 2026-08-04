//! Ported from `RenameVarsTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! # What this file covers
//!
//! The **first** CLOC12 port into `closure-pass-rename`. Upstream
//! `RenameVars` is a whole-program variable renamer: it shortens every
//! non-externed binding — globals, all nested function scopes, and the
//! function *names* themselves — with a frequency-biased name generator.
//!
//! closurec deliberately **splits** that job. `RenamePass` (this crate)
//! is the conservative, provably-sound **local** renamer: it shortens
//! parameters and uniquely-bound `var`/`let`/`const` locals of **leaf**
//! functions (functions with no nested function), leaving globals to the
//! separate `rename-globals` pass. So this port mirrors upstream
//! `RenameVars` intent *restricted to a single leaf scope* for the
//! active cases, and pins the remaining whole-program behaviors as
//! `#[ignore = "blocked on gap-NNN"]` placeholders.
//!
//! ## Harness
//!
//! The rename crate carries `javascript-parser` + `closure-emitter` as
//! dev-dependencies, so — unlike the hand-built-AST `dce` /
//! `remove-unused-vars` ports — these cases drive the **real** source →
//! `grammar_to_program` bridge → `RenamePass` → `emit` roundtrip, the
//! exact chain closurec's SIMPLE level uses. Each case is
//! `assert_eq!(rename(src), expected)` on the emitted string.
//!
//! NOTE on whitespace: assertions are against raw `closure-emitter`
//! output — binary operators keep spaces and function declarations get a
//! trailing `;`. What the port pins is *which identifiers were renamed*,
//! not the (separate) WHITESPACE_ONLY tightening.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_closure_pass_pipeline::{Pass, PassContext};
use coding_adventures_closure_pass_rename::RenamePass;
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_parser::{bridge, parse_javascript_typed};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;

// =====================================================================
// Test-support harness
// =====================================================================

/// Parse `src`, bridge it to a typed `Program`, run `RenamePass`, and
/// emit the result as minified JS. Returns the emitted string.
fn rename(src: &str) -> String {
    let es = EsVersion::Es2025;
    let node = parse_javascript_typed(src, es).expect("parse");
    let prog = bridge::grammar_to_program(&node, es).expect("bridge");

    let pass = RenamePass::new();
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    let out = pass
        .run(PassContext {
            program: &prog,
            sidecar: &sidecar,
            cv: &mut cv,
        })
        .expect("rename");

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
// Active — behaviors our local renamer supports today
// =====================================================================

/// Upstream `testRenameSimple` (restricted to a leaf scope): a single
/// local `var` is shortened at its declaration and its use.
#[test]
fn rename_simple_local_var() {
    assert_eq!(
        rename("function f() { var longName = 1; return longName; }"),
        "function f(){var a=1;return a};"
    );
}

/// Upstream `testRenameLocals`: parameters are locals too, shortened at
/// the declaration and every use site.
#[test]
fn rename_leaf_parameter() {
    assert_eq!(
        rename("function f(longName) { return longName + 1; }"),
        "function f(a){return a+1};"
    );
}

/// Multiple parameters get distinct fresh names, in declaration order.
#[test]
fn rename_multiple_params_distinctly() {
    assert_eq!(
        rename("function f(first, second) { return first * second; }"),
        "function f(a,b){return a*b};"
    );
}

/// `const` and `let` locals are renamed just like `var`.
#[test]
fn rename_local_const_and_let() {
    assert_eq!(
        rename("function f() { const total = 1; let partial = 2; return total + partial; }"),
        "function f(){const a=1;let b=2;return a+b};"
    );
}

/// Parameter and local together — both uniquely bound, both shortened,
/// param first (declaration order).
#[test]
fn rename_param_and_local_together() {
    assert_eq!(
        rename("function f(input) { var doubled = input * 2; return doubled; }"),
        "function f(a){var b=a*2;return b};"
    );
}

/// Upstream `testRenameLocalsWithNamesReservedForGlobals`: a fresh short
/// name must never capture a referenced free global. Here the body reads
/// global `a`, so the parameter cannot become `a` — it gets `b`.
#[test]
fn fresh_name_avoids_referenced_global() {
    assert_eq!(
        rename("function f(longName) { return a + longName; }"),
        "function f(b){return a+b};"
    );
}

/// A property access (`obj.longName`) is NOT a variable reference — the
/// member name stays. The receiver param `obj` IS renamed.
#[test]
fn does_not_rename_property_access() {
    assert_eq!(
        rename("function f(obj) { return obj.longName; }"),
        "function f(a){return a.longName};"
    );
}

/// A non-computed object-literal key is a property name, never renamed.
#[test]
fn does_not_rename_object_literal_key() {
    assert_eq!(
        rename("function f(val) { return { keyName: val }; }"),
        "function f(a){return{keyName:a}};"
    );
}

/// A computed member `obj[idx]` IS a use position — `idx` is renamed.
#[test]
fn renames_computed_member_index() {
    assert_eq!(
        rename("function f(obj, idx) { return obj[idx]; }"),
        "function f(a,b){return a[b]};"
    );
}

/// A single-character name is already minimal — the generator can't
/// shrink it, so nothing changes.
#[test]
fn single_char_name_left_alone() {
    assert_eq!(
        rename("function f(x) { return x + 1; }"),
        "function f(x){return x+1};"
    );
}

/// Soundness (catch bindings are reserved): a caught name is never used
/// as a fresh short name. The catch param here is literally `a`, so the
/// function param `longName` must become `b`, not alias the caught value.
#[test]
fn fresh_name_avoids_catch_binding() {
    assert_eq!(
        rename("function f(longName) { try { risky(); } catch (a) { use(a, longName); } }"),
        "function f(b){try{risky()}catch(a){use(a,b)}};"
    );
}

/// Soundness: a name declared twice (function-scope `var` + block-scope
/// `let`) is two distinct bindings; renaming "every use" would conflate
/// them, so the name is skipped entirely. `keep`, declared once, is
/// renamed.
#[test]
fn skips_name_declared_twice() {
    assert_eq!(
        rename(
            "function f() { var total = 1; { let total = 2; sink(total); } var keep = total; return keep; }"
        ),
        "function f(){var total=1;{let total=2;sink(total)}var a=total;return a};"
    );
}

// =====================================================================
// Ignored — whole-program `RenameVars` behaviors we do not do yet.
// Each is pinned to a gap in code/specs/CLOC12-gaps.md. The `expected`
// strings encode the upstream behavior we are aiming for, so flipping
// `#[ignore]` off is a one-line change once the gap closes.
// =====================================================================

/// Upstream `testRenameGlobals`: a top-level `var` is shortened. Ours
/// leaves globals to the separate `rename-globals` pass, so the local
/// renamer is a no-op here today.
#[test]
#[ignore = "blocked on gap-144: RenamePass does not rename globals (rename-globals owns that)"]
fn rename_global_var() {
    assert_eq!(rename("var longName = 1; use(longName);"), "var a=1;use(a);");
}

/// Upstream renames parameters of nested (non-leaf) functions too. Ours
/// only touches leaf functions, so `outer`'s param `param` stays.
#[test]
#[ignore = "blocked on gap-145: non-leaf (nesting) function params not renamed"]
fn rename_non_leaf_function_param() {
    assert_eq!(
        rename(
            "function outer(param) { function inner() { return 1; } return inner() + param; }"
        ),
        "function outer(a){function inner(){return 1};return inner()+a};"
    );
}

/// Upstream shortens the function *name* itself. Ours preserves
/// declaration names (they may be externally referenced).
#[test]
#[ignore = "blocked on gap-146: function declaration names not renamed"]
fn rename_function_declaration_name() {
    assert_eq!(
        rename("function longFnName() { return 1; } longFnName();"),
        "function a(){return 1};a();"
    );
}

/// Upstream's frequency-biased generator hands the 1-char name to the
/// most-referenced variable. Here `rare` is used once and `often` three
/// times, so upstream would make `often` → `a` and `rare` → `b`; ours
/// allocates in declaration order (`rare` → `a`, `often` → `b`).
#[test]
#[ignore = "blocked on gap-147: name allocation is declaration-order, not frequency-biased"]
fn frequency_biased_name_allocation() {
    assert_eq!(
        rename(
            "function f() { var rare = 1; var often = 2; return often + often + often + rare; }"
        ),
        "function f(){var b=1;var a=2;return a+a+a+b};"
    );
}
