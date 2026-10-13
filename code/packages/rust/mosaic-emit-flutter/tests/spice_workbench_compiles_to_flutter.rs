//! SPICE workbench acceptance gate for native Flutter waveform geometry (#16926).
//!
//! SpiceWorkbench draws its schematic grid, wires, terminals and waveform
//! with `Path` inside `For` loops, so every coordinate is bound to a list
//! element (`x1: ( segment[0] )`). Until #16926 this backend accepted only
//! literal `Path` coordinates, so the whole component failed to compile and
//! nothing recorded it: MosaicBook's gate compiles stories for the browser
//! backends only. This test compiles the real package source and pins the
//! bound geometry, so a regression fails here instead of silently.

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

fn emit_workbench() -> String {
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
    mosaic_emit_flutter::from_pipeline(&interface.component, &layout.def, &style.def)
        .expect("emit workbench Flutter")
        .output
}

#[test]
fn spice_workbench_waveform_segments_lower_to_bound_flutter_lines() {
    let out = emit_workbench();
    for needle in ["start: Offset(((double v) => v.isFinite ? v : 0.0)((( segment [ 0 ] )).toDouble()), ((double v) => v.isFinite ? v : 0.0)((( segment [ 1 ] )).toDouble()))", "end: Offset(((double v) => v.isFinite ? v : 0.0)((( segment [ 2 ] )).toDouble()), ((double v) => v.isFinite ? v : 0.0)((( segment [ 3 ] )).toDouble()))"] {
        assert!(out.contains(needle), "missing `{needle}` in:\n{out}");
    }
}

#[test]
fn spice_workbench_terminals_lower_to_bound_flutter_circles() {
    let out = emit_workbench();
    for needle in ["Positioned(left: ((double v) => v.isFinite ? v : 0.0)((( point [ 0 ] )).toDouble()) - 5, top: ((double v) => v.isFinite ? v : 0.0)((( point [ 1 ] )).toDouble()) - 5,"] {
        assert!(out.contains(needle), "missing `{needle}` in:\n{out}");
    }
}
