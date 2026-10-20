//! package_compiles — smoke test for the mosaic-pkg-grid package.
//!
//! Asserts that the package (manifest + its components, each with .mil /
//! .mll and, when it renders anything, a light and a dark .msl) is
//! internally consistent at the language-frontend layer:
//!
//!   1. The manifest parses as TOML and exports exactly the interfaces in
//!      `src/*.mil`; README.md's export table names exactly those exports.
//!   2. Each <Component>.mil compiles via mosmodel_compiler::compile.
//!   3. Each <Component>.mll compiles via moslayout_compiler::compile,
//!      validated against the matching .mil interface descriptor.
//!   4. Each <Component>.{light,dark}.msl compiles via
//!      mosstyle_compiler::compile, validated against the matching .mll
//!      part map. A component ships both themes or neither.
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

/// The two themes a styled component ships.
const THEMES: &[&str] = &["light", "dark"];

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

/// The exported components, read from `mosaic-package.toml`'s
/// `[components].exports`. The manifest is the one list: this test used to
/// keep its own copy, which is how `RowHeaderGrid`'s stylesheets came to be
/// exported without ever being compiled here. The manifest is checked
/// against the files on disk instead (`manifest_declares_expected_exports`)
/// and against the README (`readme_describes_every_export`).
fn exported_components() -> Vec<String> {
    let manifest_src = fs::read_to_string(package_root().join("mosaic-package.toml"))
        .expect("manifest mosaic-package.toml must exist at the package root");
    let value: toml::Value =
        toml::from_str(&manifest_src).expect("mosaic-package.toml must parse as valid TOML");
    value
        .get("components")
        .and_then(|c| c.get("exports"))
        .and_then(|e| e.as_array())
        .expect("[components].exports must be an array")
        .iter()
        .map(|v| {
            v.as_str()
                .expect("[components].exports entries must be strings")
                .to_string()
        })
        .collect()
}

/// The component interfaces that exist in `src/`, by file stem, sorted.
fn source_components() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(package_root().join("src"))
        .expect("src/ must be readable")
        .map(|entry| entry.expect("src/ entry must be readable").path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("mil"))
        .map(|path| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .expect("a .mil file name must be UTF-8")
                .to_string()
        })
        .collect();
    names.sort();
    names
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
    assert_eq!(version, "0.2.3", "[package].version must be 0.2.3 for the Cell onClick row/col payload (UI37)");

    // [components].exports names exactly the interfaces in src/: an
    // unexported .mil is dead source, and an export with no .mil fails
    // every consumer. Sorted, because the manifest's order is the
    // author's, not part of the contract.
    let mut export_names = exported_components();
    export_names.sort();
    assert_eq!(
        export_names,
        source_components(),
        "[components].exports must name exactly the src/*.mil interfaces"
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
    for component in &exported_components() {
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
    for component in &exported_components() {
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

/// Compiles every exported component's `<Component>.{light,dark}.msl`
/// against the part map produced by `<Component>.mll`.
///
/// This used to compile `Grid.dark.msl` and `Cell.dark.msl` only, a list
/// written when those were the package's only stylesheets. When the light
/// theme and `RowHeaderGrid` arrived, four of the six stylesheets were
/// never compiled by the package's own test. The loop now follows the
/// manifest, and a component ships both themes or neither (Column renders
/// nothing, so it ships none; `source_tree_has_expected_shape` pins that).
#[test]
fn each_msl_compiles_against_its_part_map() {
    for component in &exported_components() {
        let present: Vec<&str> = THEMES
            .iter()
            .copied()
            .filter(|theme| src_path(&format!("{component}.{theme}.msl")).exists())
            .collect();
        assert!(
            present.is_empty() || present.len() == THEMES.len(),
            "{component} ships {present:?} but not every theme in {THEMES:?}: a styled \
             component must ship both, or a consumer of the missing theme renders unstyled"
        );

        // We need the part map produced by the .mll.  Compile the .mll
        // (with no interface) to get a part map; mosstyle then validates
        // its `part X { ... }` declarations against that map.
        let mll_src = read_source(&format!("{}.mll", component));
        let mll_out = moslayout_compiler::compile(&mll_src, None)
            .unwrap_or_else(|e| panic!("{}.mll precompile failed: {:#?}", component, e));

        for theme in present {
            let msl_filename = format!("{component}.{theme}.msl");
            let msl_src = read_source(&msl_filename);
            if let Err(errs) = mosstyle_compiler::compile(&msl_src, Some(&mll_out.part_map_json)) {
                panic!(
                    "{} failed to compile via mosstyle_compiler:\n{:#?}",
                    msl_filename, errs
                );
            }
        }
    }
}

/// README.md's export table is prose a person reads, so it cannot be
/// generated, but it can be checked: its `| `Name` | … |` rows must name
/// exactly the manifest's exports. It once said "Three components" and
/// listed three, while the manifest exported four.
#[test]
fn readme_describes_every_export() {
    let readme = fs::read_to_string(package_root().join("README.md"))
        .expect("README.md must exist at the package root");
    let section = readme
        .split("\n## ")
        .find(|s| s.starts_with("What this package exports"))
        .expect("README.md must have a `## What this package exports` section");
    let mut described: Vec<String> = section
        .lines()
        .filter_map(|line| line.strip_prefix("| `"))
        .filter_map(|rest| rest.split_once('`').map(|(name, _)| name.to_string()))
        .collect();
    described.sort();
    let mut exports = exported_components();
    exports.sort();
    assert_eq!(
        described, exports,
        "README.md's export table must name exactly [components].exports"
    );
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

/// v0.2.1 regression guard. Cell.mil declares `emit onClick`, and every call
/// site (Grid.mll's own `Cell(onClick: emit: onNavigate, ...)`) supplies a
/// handler for it — but v0.2.0's Cell.mll never referenced `emit: onClick`
/// anywhere in its own body, so a real click did nothing. The bug shipped
/// silently because no application had driven Grid through a real emitter
/// pipeline until the task-app sheet view did. This pins the fix: Cell's
/// root Box must wire onClick to its own declared emit, so the package
/// resolver's rewrite_bindings has something to substitute the call site's
/// binding into.
#[test]
fn ui28_1_cell_wires_its_own_onclick_emit() {
    let src = read_source("Cell.mll");
    let collapsed: String = src.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        collapsed.contains("onClick: emit: onClick"),
        "Cell.mll's Box must fire its own declared onClick emit — a click on \
         a cell otherwise does nothing, regardless of what the call site wires:\n{src}"
    );

    // And it must actually compile against Cell.mil's interface, proving
    // `onClick` really is a declared emit the resolver can bind to.
    let mil_src = read_source("Cell.mil");
    let mil_out = mosmodel_compiler::compile(&mil_src).expect("Cell.mil compiles");
    moslayout_compiler::compile(&src, Some(&mil_out.descriptor_json))
        .expect("Cell.mll must validate against Cell.mil");
}

/// v0.2.3 / UI37 regression guard. onClick firing (v0.2.2) is necessary but
/// not sufficient — Grid's documented `onNavigate(row, col)` contract needs
/// Cell to actually carry a coordinate. Pins: Cell.mil declares onClick with
/// a (row, col) payload and matching row/col slots; Cell.mll threads them
/// onto the Box; Grid.mll supplies them at the call site via the same
/// expression-in-slot-binding shape as is-editing/is-selected.
#[test]
fn ui37_cell_onclick_carries_row_and_col() {
    let cell_mil_src = read_source("Cell.mil");
    assert!(
        cell_mil_src.contains("slot row") && cell_mil_src.contains("slot col"),
        "Cell.mil must declare row/col slots:\n{cell_mil_src}"
    );
    let mil_out = mosmodel_compiler::compile(&cell_mil_src).expect("Cell.mil compiles");
    let on_click = mil_out
        .component
        .emits
        .iter()
        .find(|e| e.name == "onClick")
        .expect("Cell.mil must declare onClick");
    let param_names: Vec<&str> = on_click.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        param_names,
        vec!["row", "col"],
        "onClick must declare (row, col) in that order, got: {param_names:?}"
    );

    let cell_mll_src = read_source("Cell.mll");
    let collapsed_cell: String = cell_mll_src.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        collapsed_cell.contains("row: slot: row") && collapsed_cell.contains("col: slot: col"),
        "Cell.mll's Box must thread row/col onto itself:\n{cell_mll_src}"
    );

    let grid_mll_src = read_source("Grid.mll");
    let collapsed_grid: String = grid_mll_src.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        collapsed_grid.contains("row: ( r )") && collapsed_grid.contains("col: ( c )"),
        "Grid.mll must supply row/col at the Cell call site from its own loop bindings:\n{grid_mll_src}"
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

    for component in ["Grid", "Cell", "Column", "RowHeaderGrid"] {
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

/// Every file in `src/` belongs to an export: `<Export>.mil`, `.mll`,
/// `.{light,dark}.msl` or `.stories.json`, plus the crate's `lib.rs`. A
/// stray file (a renamed component's old stylesheet, say) is source no
/// build reads, so it fails here instead of drifting.
#[test]
fn source_tree_has_expected_shape() {
    let exports = exported_components();
    let entries = fs::read_dir(package_root().join("src")).expect("src/ must be readable");
    for entry in entries {
        let name = entry.expect("src/ entry must be readable").file_name();
        let name = name.to_str().expect("a src/ file name must be UTF-8");
        if name == "lib.rs" {
            continue;
        }
        let (stem, suffix) = name.split_once('.').unwrap_or((name, ""));
        assert!(
            exports.iter().any(|e| e == stem)
                && ["mil", "mll", "light.msl", "dark.msl", "stories.json"].contains(&suffix),
            "src/{name} is not a source file of any export in {exports:?}"
        );
    }
    for component in &exports {
        for ext in ["mil", "mll"] {
            let path = src_path(&format!("{component}.{ext}"));
            assert!(path.exists(), "expected package source file missing: {}", path.display());
        }
    }

    // Column has no .msl on purpose — guard the negative case too.
    for theme in THEMES {
        let column_msl = src_path(&format!("Column.{theme}.msl"));
        assert!(
            !column_msl.exists(),
            "Column is metadata-only and must not ship a .msl ({} exists unexpectedly)",
            column_msl.display()
        );
    }
}

// ---------------------------------------------------------------------------
// 7. #15048 -- the inline editor must not change the row's height
//
// Cell swaps a `Text` for a `HostInput` when `is-editing` is set. A native
// text input is not a neutral box: the user agent gives it padding, a
// border and a font of its own, none of which a `<span>` carries. Before
// this part existed the editor was emitted with no geometry at all, so
// those UA defaults applied in full.
//
// That matters because a grid row is only as tall as its tallest cell.
// Measured in headless Chrome against this package's own dark theme
// (`part cell`: padding 4px, no pinned height), the editing row came out
// 6px taller than a display row -- at 100%, 150% AND 200% text scale, so
// this is not a rounding artifact of one zoom level:
//
//     scale   display   editing(bare)   delta
//     100%      26px        32px         +6
//     150%      36px        42px         +6
//     200%      45px        51px         +6
//
// A 6px step is far outside the 0.5px tolerance in the React runtime's
// `table_capacity.ts`, which refuses uniform viewport capacity the moment
// any row's height differs from the first. So merely focusing a cell
// silently downgraded the grid's scrolling, which is the defect.
//
// Zeroing the box and inheriting the font leaves the input contributing
// only its line box, exactly as the span does. Re-measured with the part
// applied: delta 0 at all three scales, guard clean.
//
// WHAT THIS TEST DOES NOT CLAIM: it does not re-measure pixels -- there is
// no browser in this harness. It pins the authored contract that the
// measurement validated, so that deleting the part fails here rather than
// silently in a product months later.
#[test]
fn ui15048_cell_editor_part_matches_display_row_geometry() {
    let mll_src = read_source("Cell.mll");
    let mll_out = moslayout_compiler::compile(&mll_src, None)
        .unwrap_or_else(|e| panic!("Cell.mll precompile failed: {:#?}", e));

    // 1. The .mll must NAME the editor. Without a part name the style
    //    below has nothing to attach to and the input goes out bare.
    assert!(
        mll_out.part_map_json.contains("cell-editor"),
        "Cell.mll must give its HostInput the `cell-editor` part so the \
         editor's geometry can be authored; part map was: {}",
        mll_out.part_map_json
    );

    // 2. The theme must author that part.
    let msl_src = read_source("Cell.dark.msl");
    let style = mosstyle_compiler::compile(&msl_src, Some(&mll_out.part_map_json))
        .unwrap_or_else(|e| panic!("Cell.dark.msl failed to compile: {:#?}", e));

    let editor = style
        .def
        .parts
        .iter()
        .find(|p| p.name == "cell-editor")
        .expect("Cell.dark.msl must style the `cell-editor` part");

    // 3. Each property here was load-bearing in the measurement: padding
    //    and border are the UA box that made the row taller, `font`
    //    stops the input using its own smaller face, and border-box
    //    keeps `width: 100%` from overflowing the cell's own padding.
    //
    //    `color` is deliberately absent -- see the .msl. Every backend
    //    already colours the editor, and setting one here broke Compose
    //    (transparent) and Qt (duplicate property, a QML compile error).
    for (name, value) in [
        ("padding", "0px"),
        ("border", "0px"),
        ("font", "inherit"),
        ("box-sizing", "border-box"),
    ] {
        let prop = editor
            .base
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| {
                panic!(
                    "`cell-editor` must set `{}` -- without it the editing row \
                     grows and trips the uniform-capacity guard. Declared: {:?}",
                    name,
                    editor.base.iter().map(|p| &p.name).collect::<Vec<_>>()
                )
            });
        assert_eq!(
            prop.value.trim(),
            value,
            "`cell-editor` sets `{}: {}` but the measured geometry needs `{}`",
            name,
            prop.value,
            value
        );
    }

    // 4. `color` must stay ABSENT. This is not a style preference -- both
    //    ways of setting it are build-breaking, and neither shows up in
    //    this package's own tests, which is exactly why it is pinned here:
    //
    //      `color: inherit`  -> Color.Transparent on Compose, Color.clear
    //                           on SwiftUI, `Foreground="Inherit"` on XAML
    //      `color: <literal>` -> Qt writes `color` twice on the same
    //                           TextInput, a hard qmlcachegen error that
    //                           broke the Trestle task-app build three
    //                           packages downstream
    //                           (task-app -> pkg-sheet -> pkg-grid)
    //
    //    Every backend already colours the editor from its text context,
    //    so the correct value here is none at all.
    assert!(
        !editor.base.iter().any(|p| p.name == "color"),
        "`cell-editor` must not set `color`: `inherit` lowers to a \
         transparent brush on Compose/SwiftUI, and a literal makes Qt emit \
         `color` twice on one TextInput (a QML compile error that breaks \
         task-app). Every backend already colours the editor. Declared: {:?}",
        editor.base.iter().map(|p| &p.name).collect::<Vec<_>>()
    );

    // 5. Partition guard: the display cell's own part must survive. An
    //    edit that replaced `cell` with `cell-editor` would satisfy every
    //    assertion above and still break the grid.
    assert!(
        style.def.parts.iter().any(|p| p.name == "cell"),
        "the display cell's `cell` part must still be styled; parts are {:?}",
        style.def.parts.iter().map(|p| &p.name).collect::<Vec<_>>()
    );
}
