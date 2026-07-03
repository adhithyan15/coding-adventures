//! Integration test: end-to-end compile every component in
//! `mosaic-pkg-grid` through the XAML backend.
//!
//! This is the spec §17 PR-6 capstone test — the WinUI 3 / XAML
//! emitter is "done" in the spec sense when it can lower every
//! component in `mosaic-pkg-grid` (`Grid`, `Cell`, `Column`) without
//! hitting `UnsupportedPrimitive`, `UnsupportedExpression`, or any
//! other failure path.
//!
//! What this test verifies:
//!
//!   1. Each component's `.mil` / `.mll` / `.dark.msl` parses through
//!      the three IR compilers (mosmodel / moslayout / mosstyle).
//!   2. The triple lowers through `mosaic_emit_xaml::from_pipeline`
//!      without error.
//!   3. The XAML output contains a `<UserControl>` root.
//!   4. The code-behind output declares a `partial class
//!      {Component} : UserControl`.
//!   5. The Event union output declares `record {Component}Event`.
//!
//! What this test does NOT verify:
//!
//!   - That `dotnet build` accepts the emitted source (Windows-only;
//!     tracked as a follow-up that needs the MSBuild toolchain).
//!   - That the rendered XAML is visually correct (no headless XAML
//!     renderer in the CI matrix).
//!   - That the package's intended UI behaviour (selection, editing)
//!     wires correctly end-to-end.
//!
//! The companion `mosaic-pkg-grid::tests::package_compiles` test
//! verifies the same three-file shape against the three IR
//! compilers; this test extends it to the XAML emitter.
//!
//! Note on path resolution: mosaic-emit-xaml lives in
//! `code/packages/rust/mosaic-emit-xaml/` and mosaic-pkg-grid lives
//! in `code/packages/mosaic/mosaic-pkg-grid/`. We resolve relative to
//! `CARGO_MANIFEST_DIR` (set to the mosaic-emit-xaml crate root by
//! Cargo) and step up four directory levels to find the package.

use std::fs;
use std::path::PathBuf;

const COMPONENTS: &[&str] = &["Grid", "Cell", "Column"];

fn pkg_grid_src_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent() // mosaic-emit-xaml → rust/
        .and_then(|p| p.parent()) // rust → packages/
        .map(|p| p.join("mosaic").join("mosaic-pkg-grid").join("src"))
        .expect("derive mosaic-pkg-grid src root from CARGO_MANIFEST_DIR")
}

#[test]
fn pkg_grid_root_resolves_and_contains_each_component() {
    let root = pkg_grid_src_root();
    assert!(
        root.exists(),
        "mosaic-pkg-grid src root not found at {:?}",
        root
    );
    for c in COMPONENTS {
        assert!(
            root.join(format!("{c}.mil")).exists(),
            "missing {c}.mil under {:?}",
            root
        );
        assert!(
            root.join(format!("{c}.mll")).exists(),
            "missing {c}.mll under {:?}",
            root
        );
        // Only Grid and Cell ship a `.dark.msl`; Column is metadata-
        // only so it skips style. We don't assert presence here.
    }
}

/// Compile one component through the three IR compilers and then
/// through the XAML emitter. Returns the (xaml, code_behind, events)
/// triple as strings.
fn compile_component(name: &str) -> (String, String, String) {
    let root = pkg_grid_src_root();
    let mil = root.join(format!("{name}.mil"));
    let mll = root.join(format!("{name}.mll"));
    let msl = root.join(format!("{name}.dark.msl"));

    let mil_src = fs::read_to_string(&mil).unwrap_or_else(|e| panic!("read {:?}: {e}", mil));
    let mll_src = fs::read_to_string(&mll).unwrap_or_else(|e| panic!("read {:?}: {e}", mll));
    // .msl is optional — Column is metadata-only.
    let msl_src = fs::read_to_string(&msl).ok();

    // 1. mosmodel
    let mosmodel_out = mosmodel_compiler::compile(&mil_src)
        .unwrap_or_else(|errs| panic!("mosmodel {name}: {errs:?}"));

    // 2. moslayout (with interface validation)
    let layout_out = moslayout_compiler::compile(&mll_src, Some(&mosmodel_out.descriptor_json))
        .unwrap_or_else(|errs| panic!("moslayout {name}: {errs:?}"));

    // 3. mosstyle (empty when there's no .dark.msl)
    let style_def = match msl_src {
        Some(src) => {
            let style_out = mosstyle_compiler::compile(&src, Some(&layout_out.part_map_json))
                .unwrap_or_else(|errs| panic!("mosstyle {name}: {errs:?}"));
            style_out.def
        }
        None => mosstyle_compiler::StyleDef {
            component_name: name.to_string(),
            parts: Vec::new(),
        },
    };

    // 4. The XAML backend. We provide a `ComponentRegistry` that
    //    registers Grid's siblings (`Cell`, `Column`) so Grid.mll's
    //    `Cell` reference resolves to a component-ref `<pkg:Cell/>`
    //    rather than an `UnknownComponent` error.
    let mut reg = mosaic_emit_xaml::ComponentRegistry::new();
    reg.register(
        "Cell",
        "grid",
        "using:Mosaic.Package.Grid",
        "mosaic-pkg-grid",
    );
    reg.register(
        "Column",
        "grid",
        "using:Mosaic.Package.Grid",
        "mosaic-pkg-grid",
    );
    // Grid itself is the active component; we don't register Grid
    // because it would shadow the self-reference (it isn't ever
    // referenced from inside Grid.mll, only from VisiCalc's host).

    let opts = mosaic_emit_xaml::EmitOptions::default();
    let result = mosaic_emit_xaml::from_pipeline(
        &mosmodel_out.component,
        &layout_out.def,
        &style_def,
        Some(&reg),
        &opts,
    )
    .unwrap_or_else(|e| panic!("xaml emit {name}: {e}"));

    (result.xaml, result.code_behind, result.events)
}

#[test]
fn cell_component_lowers_to_xaml_without_error() {
    let (xaml, code_behind, events) = compile_component("Cell");
    assert!(
        xaml.contains("<UserControl"),
        "Cell XAML missing UserControl root"
    );
    assert!(
        code_behind.contains("public sealed partial class Cell : UserControl"),
        "Cell code-behind missing partial class"
    );
    assert!(
        events.contains("CellEvent"),
        "Cell events file missing CellEvent record"
    );
}

#[test]
fn column_component_lowers_to_xaml_without_error() {
    // Column is metadata-only — slot-bearing but no children. The
    // XAML should still be syntactically valid (UserControl with an
    // empty body or a self-closing root tag).
    let (xaml, code_behind, _events) = compile_component("Column");
    assert!(xaml.contains("<UserControl"));
    assert!(
        code_behind.contains("public sealed partial class Column : UserControl"),
        "Column code-behind missing partial class"
    );
}

#[test]
fn grid_component_lowers_to_xaml_without_error() {
    // Grid is the headline component — uses HostTable + sub-tags +
    // For + Cell (component reference). All five PR-1..PR-5
    // pieces of the spec's §17 roadmap must work together for this
    // test to pass.
    let (xaml, code_behind, events) = compile_component("Grid");
    assert!(xaml.contains("<UserControl"));
    // HostTable lowered.
    assert!(
        xaml.contains("<Grid>") || xaml.contains("<Grid "),
        "Grid XAML missing <Grid> from HostTable lowering, got:\n{xaml}"
    );
    // For block lowered to ItemsRepeater.
    assert!(
        xaml.contains("<ItemsRepeater"),
        "Grid XAML missing <ItemsRepeater> from For lowering, got:\n{xaml}"
    );
    // Cell reference lowered with the registered xmlns prefix.
    assert!(
        xaml.contains("<grid:Cell"),
        "Grid XAML missing <grid:Cell> component reference, got:\n{xaml}"
    );
    // xmlns declaration on UserControl.
    assert!(
        xaml.contains("xmlns:grid=\"using:Mosaic.Package.Grid\""),
        "Grid XAML missing xmlns:grid declaration, got:\n{xaml}"
    );
    // Code-behind has the partial class.
    assert!(
        code_behind.contains("public sealed partial class Grid : UserControl"),
        "Grid code-behind missing partial class"
    );
    // Three emits (onNavigate, onEditCommit, onEditCancel) → three records.
    assert!(events.contains("Navigate"));
    assert!(events.contains("EditCommit"));
    assert!(events.contains("EditCancel"));
}

#[test]
fn grid_emits_for_view_models() {
    // Grid.mll uses a `For` block over `viewport-rows`. The XAML
    // emitter should produce a generated RowVm record for it.
    let root = pkg_grid_src_root();
    let mil_src = fs::read_to_string(root.join("Grid.mil")).unwrap();
    let mll_src = fs::read_to_string(root.join("Grid.mll")).unwrap();
    let msl_src = fs::read_to_string(root.join("Grid.dark.msl")).unwrap();

    let mosmodel_out = mosmodel_compiler::compile(&mil_src).unwrap();
    let layout_out =
        moslayout_compiler::compile(&mll_src, Some(&mosmodel_out.descriptor_json)).unwrap();
    let style_out = mosstyle_compiler::compile(&msl_src, Some(&layout_out.part_map_json)).unwrap();

    let mut reg = mosaic_emit_xaml::ComponentRegistry::new();
    reg.register(
        "Cell",
        "grid",
        "using:Mosaic.Package.Grid",
        "mosaic-pkg-grid",
    );

    let result = mosaic_emit_xaml::from_pipeline(
        &mosmodel_out.component,
        &layout_out.def,
        &style_out.def,
        Some(&reg),
        &mosaic_emit_xaml::EmitOptions::default(),
    )
    .unwrap();

    assert!(
        !result.for_view_models.is_empty(),
        "Grid should produce at least one RowVm from its For block"
    );
}
