//! SPICE workbench acceptance gate for native XAML waveform geometry.
//!
//! The package's waveform trace is a `list<list<number>>`; each row is one
//! adjacent-point segment. WinUI's typed DataTemplate compiler cannot bind an
//! indexer helper through `Owner`, so the backend must expose each coordinate
//! as a generated `double` property on the segment row VM.

use std::fs;
use std::path::PathBuf;

fn spice_workbench_src_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .map(|path| {
            path.join("mosaic")
                .join("mosaic-pkg-spice-workbench")
                .join("src")
        })
        .expect("derive SPICE workbench source root from CARGO_MANIFEST_DIR")
}

#[test]
fn spice_workbench_waveform_segments_lower_to_bound_xaml_lines() {
    let root = spice_workbench_src_root();
    let mil = fs::read_to_string(root.join("SpiceWorkbench.mil")).expect("read workbench MIL");
    let mll = fs::read_to_string(root.join("SpiceWorkbench.mll")).expect("read workbench MLL");
    let msl =
        fs::read_to_string(root.join("SpiceWorkbench.dark.msl")).expect("read workbench dark MSL");

    let interface = mosmodel_compiler::compile(&mil).expect("compile workbench interface");
    let layout = moslayout_compiler::compile(&mll, Some(&interface.descriptor_json))
        .expect("compile workbench layout");
    let style = mosstyle_compiler::compile(&msl, Some(&layout.part_map_json))
        .expect("compile workbench style");
    let result = mosaic_emit_xaml::from_pipeline(
        &interface.component,
        &layout.def,
        &style.def,
        None,
        &mosaic_emit_xaml::EmitOptions::default(),
    )
    .expect("emit workbench XAML");

    assert!(
        result.xaml.contains(
            "<Line X1=\"{x:Bind Expr_e8931b57Number, Mode=OneWay}\" \
             Y1=\"{x:Bind Expr_746e4cf4Number, Mode=OneWay}\" \
             X2=\"{x:Bind Expr_47a84ffdNumber, Mode=OneWay}\" \
             Y2=\"{x:Bind Expr_7649649aNumber, Mode=OneWay}\""
        ),
        "waveform segment must bind all four geometry coordinates:\n{}",
        result.xaml
    );
    assert!(
        result
            .code_behind
            .contains("internal double Expr_e8931b57Number(IReadOnlyList<double> Segment)"),
        "segment indexer must stay numeric in generated code-behind:\n{}",
        result.code_behind
    );
    assert!(
        result.for_view_models.iter().any(|view_model| {
            view_model
                .source
                .contains("public double Expr_e8931b57Number")
                && view_model
                    .source
                    .contains("Owner.Expr_e8931b57Number(Segment)")
        }),
        "typed segment VM must project numeric helpers locally"
    );
}
