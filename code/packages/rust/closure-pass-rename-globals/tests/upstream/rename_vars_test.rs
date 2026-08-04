//! Ported from `RenameVarsTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! This is a CLOC12 port for the `rename-globals` pass. Upstream
//! `RenameVars` renames *every* variable — globals, locals, and parameters
//! — to short names, with several reservation / stability modes. Our
//! `RenameGlobalsPass` implements the provably-sound global slice: it
//! renames **GLOBAL-scope** bindings (top-level `function` names and
//! `var` / `let` / `const` targets) to the shortest fresh names
//! `a`, `b`, `c`, … in first-appearance order, and leaves everything else
//! alone (names already one character long, free/undeclared globals, dotted
//! property keys, and any do-not-rename extern).
//!
//! So the file splits in two:
//!
//! - **Active `#[test]`s** — the upstream behaviors our pass genuinely
//!   supports today, driving the real `source → bridge → rename → emit`
//!   chain (the pass exposes a source-string surface through public crate
//!   APIs, so — unlike the dce / remove-unused-vars AST-builder ports — we
//!   assert on the emitted string exactly as upstream's `test(js, expected)`
//!   does).
//! - **`#[ignore = "blocked on gap-NNN"]` placeholders** — upstream intent
//!   our global-only pass does not cover yet (local / parameter renaming,
//!   pseudo-name mode, short-name reuse across scopes), each pinned to a
//!   `gap-NNN` entry in `code/specs/CLOC12-gaps.md`. Run with
//!   `--include-ignored` to measure progress as those gaps close.
//!
//! Every active test that *disagrees* with our pass is a real closurec
//! defect, not a translation artifact — the whole point of the port.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_closure_pass_pipeline::{Pass, PassContext};
use coding_adventures_closure_pass_rename_globals::RenameGlobalsPass;
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_parser::{bridge, parse_javascript_typed};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;
use std::collections::HashSet;

/// Parse `src`, bridge to a typed `Program`, run `RenameGlobalsPass` with the
/// given do-not-rename (externs) set, and emit the minified result — the same
/// chain closurec's ADVANCED level uses. Returns the emitted string.
fn rename_with(src: &str, externs: &[&str]) -> String {
    let es = EsVersion::Es2025;
    let node = parse_javascript_typed(src, es).expect("parse");
    let prog = bridge::grammar_to_program(&node, es).expect("bridge");

    let set: HashSet<String> = externs.iter().map(|s| s.to_string()).collect();
    let pass = RenameGlobalsPass::new(set);
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    let out = pass
        .run(PassContext {
            program: &prog,
            sidecar: &sidecar,
            cv: &mut cv,
        })
        .expect("rename-globals");

    let mut cv2 = CVLog::new(false);
    let opts = EmitOptions {
        source_map: false,
        ..Default::default()
    };
    emit(&out.program, &sidecar, &mut cv2, &opts)
        .expect("emit")
        .code
}

fn rename(src: &str) -> String {
    rename_with(src, &[])
}

// ===================================================================
// Active ports — behaviors the global renamer supports today.
// ===================================================================

/// Upstream `testRenameSimple`: two distinct globals get the two shortest
/// names in appearance order.
#[test]
fn renames_two_globals_to_a_and_b() {
    assert_eq!(
        rename("var alpha = 1; var beta = 2; use(alpha, beta);"),
        "var a=1;var b=2;use(a,b);"
    );
}

/// Every use of a renamed global is rewritten consistently — the binding and
/// all reads collapse to the same short name.
#[test]
fn rewrites_all_uses_of_a_renamed_global() {
    assert_eq!(
        rename("var counter = 0; bump(counter); bump(counter);"),
        "var a=0;bump(a);bump(a);"
    );
}

/// A top-level `function` declaration is a global binding and is renamed like
/// any other (upstream renames function names too).
#[test]
fn renames_a_global_function_declaration() {
    assert_eq!(
        rename("function computeTotal() { return 1; } computeTotal();"),
        "function a(){return 1}a();"
    );
}

/// Upstream `testDoNotRenameExterns`: the reserved name `apiHandler` is never
/// touched, while the ordinary global `helper` is renamed to the first free
/// short name `a` — at both its declaration and its call inside the reserved
/// function's body.
#[test]
fn does_not_rename_reserved_extern() {
    assert_eq!(
        rename_with(
            "function apiHandler() { return helper(); } function helper() { return 1; }",
            &["apiHandler"],
        ),
        "function apiHandler(){return a()}function a(){return 1};",
    );
}

/// A free/undeclared global referenced but never bound (`window`) is left
/// exactly as written — the renamer only touches names it can see a binding
/// for.
#[test]
fn leaves_free_undeclared_globals_untouched() {
    assert_eq!(
        rename("function greet() { console.log(window); } greet();"),
        "function a(){console.log(window)}a();"
    );
}

/// Upstream keeps property accesses stable: `obj.total` is a member key, not a
/// variable reference, so it is never renamed even when a same-named global
/// exists.
#[test]
fn does_not_rename_dotted_property_keys() {
    assert_eq!(
        rename("var total = 1; read(obj.total);"),
        "var a=1;read(obj.total);"
    );
}

/// A computed member `obj[key]` *does* reference the global `key`, so the
/// index is renamed while the (undeclared) `obj` stays.
#[test]
fn renames_global_used_as_computed_member_index() {
    assert_eq!(
        rename("var key = 2; read(obj[key]);"),
        "var a=2;read(obj[a]);"
    );
}

/// Upstream never lengthens a name: a global already one character long has no
/// shorter form available, so it is left as-is.
#[test]
fn does_not_rename_already_single_char_global() {
    assert_eq!(rename("function f() { return 1; } f();"), "function f(){return 1}f();");
}

// ===================================================================
// Ignored ports — upstream intent the global-only pass does not cover.
// Each is pinned to a gap in code/specs/CLOC12-gaps.md.
// ===================================================================

/// Upstream `RenameVars` also renames LOCAL variables inside function bodies.
/// Our pass only renames globals, so `inner` is left untouched today.
#[test]
#[ignore = "blocked on gap-134: rename-globals does not rename function-local variables"]
fn renames_a_local_variable() {
    assert_eq!(
        rename("function f() { var innerLongName = 1; return innerLongName; } f();"),
        "function a(){var b=1;return b};a();"
    );
}

/// Upstream renames function PARAMETERS. Our pass leaves parameter names alone.
#[test]
#[ignore = "blocked on gap-135: rename-globals does not rename function parameters"]
fn renames_a_function_parameter() {
    assert_eq!(
        rename("function f(longParam) { return longParam; } f(1);"),
        "function a(b){return b};a(1);"
    );
}

/// Upstream can re-use a freed short name across two DISJOINT local scopes
/// (both locals may become `a`). Our global-only pass never allocates local
/// names at all.
#[test]
#[ignore = "blocked on gap-136: rename-globals does not reuse short names across disjoint local scopes"]
fn reuses_short_names_across_disjoint_scopes() {
    assert_eq!(
        rename("function f() { var longOne = 1; return longOne; } function g() { var longTwo = 2; return longTwo; }"),
        "function a(){var c=1;return c};function b(){var c=2;return c}"
    );
}

/// Upstream's pseudo-name mode maps each original name to a stable
/// human-readable placeholder rather than a minimal short name. Our pass has
/// no pseudo-name mode.
#[test]
#[ignore = "blocked on gap-137: rename-globals has no pseudo-name / stable-name mode"]
fn pseudo_name_mode_uses_stable_placeholders() {
    // Upstream `$longName$$` style placeholder; unsupported today.
    assert_eq!(
        rename("var longName = 1; use(longName);"),
        "var $longName$$=1;use($longName$$);"
    );
}
