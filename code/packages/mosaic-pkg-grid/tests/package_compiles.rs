//! package_compiles — smoke test for the mosaic-pkg-grid package.
//!
//! Asserts that the four-file shape of the package (manifest + three
//! components, each with .mil / .mll / .msl) is internally consistent at
//! the language-frontend layer:
//!
//!   1. The manifest parses as TOML and declares the expected exports.
//!   2. Each <Component>.mil compiles via mosmodel_compiler::compile.
//!   3. Each <Component>.mll compiles via moslayout_compiler::compile,
//!      validated against the matching .mil interface descriptor.
//!   4. Each <Component>.dark.msl compiles via mosstyle_compiler::compile,
//!      validated against the matching .mll part map.
//!
//! Expected v0.1.0 state — what passes and what is documented to fail
//! ------------------------------------------------------------------
//! * Cell + Column .mil / .mll / .msl: all compile clean.
//! * Grid.mil: compiles clean.
//! * Grid.mll: relies on kernel primitives whose backend lowerings (and
//!   in some cases their full grammar-level resolver semantics) are
//!   landing in parallel UI29 PRs.  Today the moslayout compiler treats
//!   every tag as a NAME token and validates only slot / emit references
//!   against the .mil, so Grid.mll's `For` / `If` / `HostTable` /
//!   `HostTableHead` / `HostTableBody` / `Cell` nodes all PARSE and the
//!   slot / emit references all resolve.  Should that change as the
//!   resolver gains more checks (e.g. forbidding unknown primitives, or
//!   requiring expression syntax for `is-editing: slot: edit-row` to
//!   become `r == slot: edit-row`), this test should be revisited — the
//!   point is to document the expected current behaviour, not lock the
//!   package into one specific compiler state.
//! * Grid.dark.msl: compiles clean (its parts — `sheet`, `data-row` —
//!   match the Grid.mll part declarations).
//!
//! The test deliberately uses simple assertions and prints helpful
//! diagnostics so a future contributor running it against an updated
//! compiler can immediately see what changed and where.

use std::fs;
use std::path::PathBuf;

/// The list of exported components, in the order they appear in the
/// manifest.  Used to drive the per-component compile loop.
const COMPONENTS: &[&str] = &["Grid", "Cell", "Column"];

/// Path helpers — anchor everything to the package root (the directory
/// containing this crate's Cargo.toml).  CARGO_MANIFEST_DIR is set by
/// Cargo when running the test, so the path is deterministic regardless
/// of the directory `cargo test` was invoked from.
fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn src_path(name: &str) -> PathBuf {
    package_root().join("src").join(name)
}

fn read_source(name: &str) -> String {
    let path = src_path(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

// ---------------------------------------------------------------------------
// 1. Manifest
// ---------------------------------------------------------------------------

/// Parses `mosaic-package.toml` and asserts the expected exports.
///
/// The schema this test enforces is the UI29 §4.2 minimum surface:
///   [package].name / .version / .description / .license
///   [components].exports = [...]
///   [dependencies] (may be empty)
///   [kernel].version
#[test]
fn manifest_declares_expected_exports() {
    let manifest_src = fs::read_to_string(package_root().join("mosaic-package.toml"))
        .expect("manifest mosaic-package.toml must exist at the package root");

    let value: toml::Value = toml::from_str(&manifest_src)
        .expect("mosaic-package.toml must parse as valid TOML");

    // [package].name
    let name = value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .expect("[package].name must be set");
    assert_eq!(name, "mosaic-pkg-grid", "[package].name mismatch");

    // [package].version
    let version = value
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .expect("[package].version must be set");
    assert_eq!(version, "0.2.0", "[package].version must be 0.2.0 for the UI28-1 cell-and-column composition release");

    // [components].exports
    let exports = value
        .get("components")
        .and_then(|c| c.get("exports"))
        .and_then(|e| e.as_array())
        .expect("[components].exports must be an array");
    let export_names: Vec<&str> = exports.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(
        export_names,
        vec!["Grid", "Cell", "Column"],
        "[components].exports must list exactly Grid, Cell, Column in that order"
    );

    // [kernel].version
    let kernel_version = value
        .get("kernel")
        .and_then(|k| k.get("version"))
        .and_then(|v| v.as_str())
        .expect("[kernel].version must be set");
    assert_eq!(kernel_version, "1", "[kernel].version must target UI29 kernel v1");
}

// ---------------------------------------------------------------------------
// 2. mosmodel — .mil compilation
// ---------------------------------------------------------------------------

/// Each component's `.mil` must compile via mosmodel_compiler.  We hold
/// onto the descriptor_json output to thread into the .mll compile step.
#[test]
fn each_mil_compiles() {
    for component in COMPONENTS {
        let src = read_source(&format!("{}.mil", component));
        let result = mosmodel_compiler::compile(&src);
        match result {
            Ok(out) => {
                assert_eq!(
                    out.component.component, *component,
                    "{}.mil declared component name must equal '{}'",
                    component, component
                );
            }
            Err(errs) => panic!(
                "{}.mil failed to compile via mosmodel_compiler:\n{:#?}",
                component, errs
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// 3. moslayout — .mll compilation
// ---------------------------------------------------------------------------

/// Compiles each `<Component>.mll` against its `<Component>.mil` interface
/// descriptor.  For Cell and Column the layout is trivially kernel-only
/// (`Box` is a real primitive) and validates clean.  For Grid we still
/// expect a clean compile under the current moslayout-compiler — the
/// tag-name validation is intentionally lax today; should a future R2 PR
/// tighten it to reject unknown primitives, this assertion should flip
/// to `expect_err` for Grid until U29-R2 + all U29-K-* resolvers cover
/// `HostTable` / `For` / `If` / `Cell`.
#[test]
fn each_mll_compiles_against_its_interface() {
    for component in COMPONENTS {
        let mil_src = read_source(&format!("{}.mil", component));
        let mil_out = mosmodel_compiler::compile(&mil_src)
            .unwrap_or_else(|e| panic!("{}.mil precompile failed: {:#?}", component, e));

        let mll_src = read_source(&format!("{}.mll", component));
        let mll_result =
            moslayout_compiler::compile(&mll_src, Some(&mil_out.descriptor_json));

        match mll_result {
            Ok(_) => { /* expected — see test docstring above */ }
            Err(errs) => panic!(
                "{}.mll failed to compile via moslayout_compiler:\n{:#?}\n\
                 NOTE: this MAY be the expected state if the resolver has\n\
                 been tightened to reject `HostTable` / `For` / `If` / userland\n\
                 component references — in which case flip this test to\n\
                 `expect_err` for Grid and document the gap in CHANGELOG.md.",
                component, errs
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// 4. mosstyle — .msl compilation
// ---------------------------------------------------------------------------

/// Compiles each available `<Component>.dark.msl` against the part map
/// produced by `<Component>.mll`.  Column ships no .msl (it has no
/// visible output), so we only check Grid and Cell.
#[test]
fn each_msl_compiles_against_its_part_map() {
    for component in ["Grid", "Cell"] {
        let msl_filename = format!("{}.dark.msl", component);
        let msl_path = src_path(&msl_filename);
        assert!(
            msl_path.exists(),
            "{} must ship a dark-theme stylesheet at {}",
            component,
            msl_path.display()
        );

        // We need the part map produced by the .mll.  Compile the .mll
        // (with no interface) to get a part map; mosstyle then validates
        // its `part X { ... }` declarations against that map.
        let mll_src = read_source(&format!("{}.mll", component));
        let mll_out = moslayout_compiler::compile(&mll_src, None)
            .unwrap_or_else(|e| panic!("{}.mll precompile failed: {:#?}", component, e));

        let msl_src = read_source(&msl_filename);
        let msl_result = mosstyle_compiler::compile(&msl_src, Some(&mll_out.part_map_json));

        if let Err(errs) = msl_result {
            panic!(
                "{} failed to compile via mosstyle_compiler:\n{:#?}",
                msl_filename, errs
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 5. Source-shape sanity
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// 6. UI28-1 v0.2.0 shape — additive guards
//
// These tests pin the v0.2.0-specific surface: the new slots on Cell and
// Grid, the nested-For body shape in Grid.mll, and the expression-driven
// per-cell predicates. A future refactor that drops any of these is the
// kind of regression the v0.2.0 release commits to avoiding.
// ---------------------------------------------------------------------------

#[test]
fn ui28_1_cell_declares_is_selected_slot() {
    // v0.1.0 had no `is-selected` slot. v0.2.0 adds it so Grid can
    // thread per-cell selection state down without sharing its
    // internal coordinate-comparison logic with the host.
    let src = read_source("Cell.mil");
    assert!(
        src.contains("slot is-selected"),
        "Cell.mil must declare an `is-selected` slot (added in v0.2.0)"
    );
    let mil = mosmodel_compiler::compile(&src).expect("Cell.mil compiles");
    let names: Vec<&str> = mil.component.slots.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"is-selected"),
        "Cell mosmodel descriptor must expose `is-selected`, got: {:?}",
        names
    );
}

#[test]
fn ui28_1_grid_declares_column_headers_and_widths_slots() {
    // v0.1.0 had no column-metadata slots (header row was empty).
    // v0.2.0 adds parallel-array slots so the header renders + the
    // <colgroup> carries widths.
    let src = read_source("Grid.mil");
    assert!(
        src.contains("slot column-headers"),
        "Grid.mil must declare `column-headers: list<text>` (v0.2.0)"
    );
    assert!(
        src.contains("slot column-widths"),
        "Grid.mil must declare `column-widths: list<number>` (v0.2.0)"
    );
    let mil = mosmodel_compiler::compile(&src).expect("Grid.mil compiles");
    let names: Vec<&str> = mil.component.slots.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"column-headers") && names.contains(&"column-widths"),
        "Grid mosmodel descriptor must expose column-headers + column-widths, got: {:?}",
        names
    );
}

#[test]
fn ui28_1_grid_mll_uses_nested_for_via_u29_3_4_scope() {
    // The body must contain TWO `For` blocks (outer over rows, inner
    // over each row's cells). Without UI29 §3.4 (PR #4398) the inner
    // `each: row` would be rejected; the moslayout-compiler accepting
    // this layout end-to-end is the regression guard for that PR.
    let src = read_source("Grid.mll");
    let for_count = src.matches("For (").count();
    assert!(
        for_count >= 3,
        "Grid.mll must contain at least 3 For blocks (colgroup, head, body-outer, body-inner — found {}), got source:\n{}",
        for_count,
        src
    );
    // The inner For specifically uses the outer's `as: row` as its
    // `each:` value. That's the UI29 §3.4 shape we depend on.
    assert!(
        src.contains("For ( each: row"),
        "Grid.mll body must nest `For ( each: row, ... )` to iterate each row's cells (UI29 §3.4)"
    );

    // Round-trip the file through the validator. This is the
    // language-frontend assertion that PR #4398's scope-walker is in
    // place AND that v0.2.0's nested-For is well-formed.
    let mil_src = read_source("Grid.mil");
    let mil_out = mosmodel_compiler::compile(&mil_src).expect("Grid.mil compiles");
    moslayout_compiler::compile(&src, Some(&mil_out.descriptor_json))
        .expect("Grid.mll must validate against Grid.mil; if this fails the §3.4 scope walker may have regressed");
}

#[test]
fn ui28_1_grid_mll_predicate_uses_expression_in_slot_binding() {
    // Per UI28-1 §2 constraint 4 (encapsulation), Grid computes
    // is-editing / is-selected at the Cell call site via Expr — the
    // host pushes plain coordinate numbers, not pre-computed masks.
    let src = read_source("Grid.mll");
    // The Expr shape is `( r == editRow && c == editCol )`. We pin the
    // operator structure rather than exact spacing; the moslayout
    // parser collapses to a reconstructed-source Expr either way.
    assert!(
        src.contains("is-editing:") && src.contains("editRow") && src.contains("editCol"),
        "Grid.mll must compose is-editing per-cell via expression-in-slot-binding"
    );
    assert!(
        src.contains("is-selected:") && src.contains("selectedRow") && src.contains("selectedCol"),
        "Grid.mll must compose is-selected per-cell via expression-in-slot-binding"
    );
}

#[test]
fn ui28_1_grid_mll_has_colgroup_section_for_column_widths() {
    let src = read_source("Grid.mll");
    assert!(
        src.contains("HostTableColGroup"),
        "Grid.mll must use HostTableColGroup to thread per-column widths (UI28-1 §3.3)"
    );
    assert!(
        src.contains("each: slot: column-widths"),
        "Grid.mll's HostTableColGroup must iterate the `column-widths` slot"
    );
}

#[test]
fn ui28_1_grid_mll_header_for_renders_column_headers() {
    let src = read_source("Grid.mll");
    assert!(
        src.contains("each: slot: column-headers"),
        "Grid.mll's HostTableHead must iterate the `column-headers` slot — v0.1.0 left the header empty, v0.2.0 fills it"
    );
}

#[test]
fn ui28_1_no_new_kernel_primitive_was_added() {
    // Sanity guard: ensure nothing in Grid.mll / Cell.mll references a
    // tag that isn't already in the UI29 kernel + this package's
    // userland Cell / Column. The currently-allowed tag set is:
    //
    //   - 21 UI29 kernel primitives + UI31 HostTable family
    //   - Userland: Grid, Cell, Column
    //
    // If a future change introduces e.g. `HostCell` or `HostTableRow`
    // as a new tag, this test fires loud — Cell-as-component is the
    // architectural contract per UI28-1 §2 constraint 1.
    let allowed_tags: std::collections::HashSet<&str> = [
        // Containers + leaves + control flow (UI29 kernel)
        "Box", "Row", "Column", "Stack", "Text", "Image", "Spacer",
        "Divider", "Icon", "If", "Else", "For",
        // Host primitives (UI29 + UI29-1/2/4)
        "HostInput", "HostButton", "HostTable", "HostScroll", "HostDialog",
        "HostCheckbox", "HostRadio", "HostLink", "HostTooltip", "HostNumberInput",
        // HostTable structural sub-tags + Col (UI31)
        "HostTableColGroup", "HostTableHead", "HostTableBody", "HostTableFoot",
        "Col",
        // Legacy (still in PRIMITIVES, scheduled removal in U29-X1)
        "Grid", "Scroll",
        // Userland (this package)
        "Cell", "Column",
    ]
    .iter()
    .copied()
    .collect();

    for component in ["Grid", "Cell", "Column"] {
        let src = read_source(&format!("{}.mll", component));
        // Naive tag-extraction: any PascalCase identifier at the start of
        // a line (possibly preceded by whitespace) is a tag candidate.
        // Misses the rare inline tag, but the common shape catches all
        // surface-level new primitives.
        for raw_line in src.lines() {
            let line = raw_line.trim_start();
            // Strip leading `}` from "} Else {" etc.
            let line = line.trim_start_matches('}').trim_start();
            // Get the first identifier-looking token.
            let token: String = line
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if token.is_empty() {
                continue;
            }
            let first = token.chars().next().unwrap();
            if !first.is_ascii_uppercase() {
                continue;
            }
            assert!(
                allowed_tags.contains(token.as_str()),
                "{}.mll references unknown tag `{}` — if this is a new \
                 primitive, add it to the kernel list (and review whether \
                 UI28-1 §2 constraint 1 is being violated)",
                component,
                token
            );
        }
    }
}

/// Belt-and-suspenders check: every file the package promises is on disk.
#[test]
fn source_tree_has_expected_shape() {
    let expected = [
        "Grid.mil",
        "Grid.mll",
        "Grid.dark.msl",
        "Cell.mil",
        "Cell.mll",
        "Cell.dark.msl",
        "Column.mil",
        "Column.mll",
    ];
    for name in expected {
        let path = src_path(name);
        assert!(
            path.exists(),
            "expected package source file missing: {}",
            path.display()
        );
    }

    // Column has no .msl on purpose — guard the negative case too.
    let column_msl = src_path("Column.dark.msl");
    assert!(
        !column_msl.exists(),
        "Column is metadata-only and must not ship a .msl ({} exists unexpectedly)",
        column_msl.display()
    );
}
