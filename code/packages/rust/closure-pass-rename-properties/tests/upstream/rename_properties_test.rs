//! Ported from `RenamePropertiesTest.java` in `google/closure-compiler`,
//! Apache-2.0. Upstream SHA: see `tests/upstream/UPSTREAM_SHA`.
//!
//! Translation policy: see
//! `code/specs/CLOC12-upstream-test-suite-port.md`.
//!
//! This is a CLOC12 port for the `rename-properties` pass. Upstream
//! `RenameProperties` is heap-/type-aware: it can rename same-named properties
//! on unrelated objects independently and packs the hottest properties into the
//! shortest names. Our `RenamePropertiesPass` implements the sound name-based
//! slice: rename **dotted, unquoted** property names (member accesses `o.prop`
//! and object-literal keys `{prop: v}`) to the shortest fresh names
//! `a`, `b`, `c`, … in first-appearance order, applied consistently to every
//! occurrence of the same name. It leaves untouched a name accessed via a
//! computed/quoted subscript anywhere (`o["prop"]`), a name already one
//! character long, a curated set of built-in / DOM names, and any externs
//! do-not-rename entry.
//!
//! So the file splits in two:
//!
//! - **Active `#[test]`s** — behaviors our pass supports today, driving the real
//!   `source → bridge → rename → emit` chain and asserting on the emitted string
//!   (the pass exposes a source-string surface through public crate APIs, so —
//!   like the `rename-globals` port — we assert on strings exactly as upstream's
//!   `test(js, expected)`).
//! - **`#[ignore = "blocked on gap-NNN"]` placeholders** — upstream intent our
//!   name-based pass does not cover (type-aware disambiguation, cross-module
//!   renaming, frequency-ordered assignment), each pinned to a `gap-NNN` entry
//!   in `code/specs/CLOC12-gaps.md`.
//!
//! Every active test that *disagrees* with our pass is a real closurec defect,
//! not a translation artifact — the whole point of the port.

use coding_adventures_closure_emitter::{emit, EmitOptions};
use coding_adventures_closure_pass_pipeline::{Pass, PassContext};
use coding_adventures_closure_pass_rename_properties::RenamePropertiesPass;
use coding_adventures_correlation_vector::CVLog;
use coding_adventures_javascript_parser::{bridge, parse_javascript_typed};
use coding_adventures_javascript_tokens::EsVersion;
use coding_adventures_type_sidecar::Sidecar;
use std::collections::HashSet;

/// Parse `src`, bridge to a typed `Program`, run `RenamePropertiesPass` with the
/// given externs (do-not-rename) set, and emit the minified result — the same
/// chain closurec's ADVANCED level uses. Returns the emitted string.
fn rename_with(src: &str, externs: &[&str]) -> String {
    let es = EsVersion::Es2025;
    let node = parse_javascript_typed(src, es).expect("parse");
    let prog = bridge::grammar_to_program(&node, es).expect("bridge");

    let set: HashSet<String> = externs.iter().map(|s| s.to_string()).collect();
    let pass = RenamePropertiesPass::new(set);
    let sidecar = Sidecar::new();
    let mut cv = CVLog::new(false);
    let out = pass
        .run(PassContext {
            program: &prog,
            sidecar: &sidecar,
            cv: &mut cv,
        })
        .expect("rename-properties");

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
// Active ports — behaviors the property renamer supports today.
// ===================================================================

/// Upstream `testRenameProperties` core: a program-private property is renamed
/// to the shortest name, consistently at every dotted read.
#[test]
fn renames_a_private_property_consistently() {
    assert_eq!(
        rename("read(a.renderMode); read(b.renderMode);"),
        "read(a.a);read(b.a);"
    );
}

/// The rename spans dotted reads *and* object-literal keys of the same name —
/// they collapse to one short name.
#[test]
fn renames_reads_and_object_literal_keys_alike() {
    assert_eq!(
        rename("read(a.renderMode); var c = { renderMode: 3 };"),
        "read(a.a);var c={a:3};"
    );
}

/// Distinct property names get distinct short names in first-appearance order,
/// including down a member chain.
#[test]
fn distinct_properties_get_distinct_names() {
    assert_eq!(rename("read(a.outerField.innerField);"), "read(a.a.b);");
}

/// Upstream `testQuotedProperty`: a property accessed via a quoted subscript
/// anywhere (`o["mode"]`) poisons the rename of its dotted form — otherwise the
/// two spellings would desync. Both are left as written.
#[test]
fn quoted_access_poisons_the_rename() {
    assert_eq!(
        rename("read(obj.mode); read(other[\"mode\"]);"),
        "read(obj.mode);read(other[\"mode\"]);"
    );
}

/// A built-in / DOM property name is never renamed; a program-private one
/// alongside it still is.
#[test]
fn leaves_builtin_names_untouched() {
    assert_eq!(
        rename("var n = arr.length; s.toString(); var o = { tally: 1 };"),
        "var n=arr.length;s.toString();var o={a:1};"
    );
}

/// A single-character property name has no shorter form, so it is left as-is.
#[test]
fn does_not_rename_single_char_property() {
    assert_eq!(rename("read(obj.x); read(obj.x);"), "read(obj.x);read(obj.x);");
}

/// A computed subscript `obj[idx]` is not a property *name* position, so `idx`
/// is untouched while a sibling dotted `.field` is renamed.
#[test]
fn computed_subscript_index_is_not_renamed() {
    assert_eq!(
        rename("var v = obj[idx]; read(obj.field);"),
        "var v=obj[idx];read(obj.a);"
    );
}

/// Upstream `testExterns`: a property listed in the externs do-not-rename set is
/// preserved, while a program-private sibling is still renamed.
#[test]
fn does_not_rename_externs_property() {
    assert_eq!(
        rename_with("read(el.innerHTML); read(el.secretField);", &["innerHTML"]),
        "read(el.innerHTML);read(el.a);"
    );
}

// ===================================================================
// Ignored ports — upstream intent the name-based pass does not cover.
// Each is pinned to a gap in code/specs/CLOC12-gaps.md.
// ===================================================================

/// Upstream is type-aware: the same property name on two UNRELATED object types
/// can be renamed to two different short names. Our name-based pass renames a
/// name once, globally, so both `.state` reads collapse to the same `a`.
#[test]
#[ignore = "blocked on gap-138: rename-properties is name-based, not type-/heap-aware disambiguation"]
fn type_aware_disambiguation_of_same_name() {
    // Upstream (heap-aware) could give Widget#state and Store#state distinct
    // short names; our pass cannot.
    assert_eq!(
        rename("new Widget().state; new Store().state;"),
        "new Widget().a;new Store().b;"
    );
}

/// Upstream orders short-name assignment by property FREQUENCY, so the most-used
/// property gets `a`. Our pass assigns by first appearance regardless of count.
#[test]
#[ignore = "blocked on gap-139: rename-properties assigns short names by appearance order, not usage frequency"]
fn frequency_ordered_short_name_assignment() {
    // `hot` is used 3x, `cold` 1x; upstream gives `hot`→a. Ours gives the
    // first-seen `cold`→a.
    assert_eq!(
        rename("o.cold; o.hot; o.hot; o.hot;"),
        "o.b;o.a;o.a;o.a;"
    );
}

/// Upstream can rename a property consistently across separately-compiled
/// modules via a shared rename map. Our single-program pass has no cross-module
/// map.
#[test]
#[ignore = "blocked on gap-140: rename-properties has no cross-module shared rename map"]
fn cross_module_consistent_renaming() {
    // Placeholder: exercised here as a single program, but the intent is a
    // stable map reused across compilations.
    assert_eq!(
        rename("moduleA.sharedProp; moduleB.sharedProp;"),
        "moduleA.a;moduleB.a;"
    );
}
