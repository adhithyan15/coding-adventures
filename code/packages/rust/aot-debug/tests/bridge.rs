//! Bridge-layer tests for `aot-debug`.
//!
//! Confirms that:
//!
//! 1. A simple IIRModule with a few `SourceLoc`s produces a sidecar blob
//!    that reads back via `DebugSidecarReader`.
//! 2. Synthetic source locations are skipped (no garbage line/col rows).
//! 3. The function entry's `param_count` round-trips.
//! 4. `artifact_info_pe` rejects offsets that don't fit in `u32`.

use std::collections::HashMap;

use aot_debug::{
    artifact_info_elf_or_macho, artifact_info_pe, build_sidecar_from_iir,
};
use debug_sidecar::DebugSidecarReader;
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, SourceLoc};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build an IIRFunction with `instructions` at the given `locs` (lockstep).
fn fn_with_locs(name: &str, instrs: Vec<IIRInstr>, locs: Vec<SourceLoc>) -> IIRFunction {
    assert_eq!(instrs.len(), locs.len(), "fn_with_locs: parallel arrays");
    let mut f = IIRFunction::new(name, vec![], "void", instrs);
    f.source_map = locs;
    f
}

fn single(f: IIRFunction) -> IIRModule {
    IIRModule {
        name: "test".into(),
        functions: vec![f],
        entry_point: None,
        language: "test".into(),
        exports: vec![],
        imports: vec![],
    }
}

// ===========================================================================
// 1. build_sidecar_from_iir — minimal happy path
// ===========================================================================

#[test]
fn builds_sidecar_with_one_function_and_one_real_location() {
    let f = fn_with_locs(
        "main",
        vec![
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
        vec![SourceLoc::new(10, 3)],
    );
    let bytes = build_sidecar_from_iir(&single(f), "hello.bas");
    assert!(!bytes.is_empty(), "sidecar bytes must not be empty");

    // Re-parse: at minimum the writer's `finish()` must produce something
    // the reader can ingest.
    DebugSidecarReader::new(&bytes).expect("sidecar must parse back");
}

// ===========================================================================
// 2. Synthetic SourceLocs are skipped
// ===========================================================================

#[test]
fn synthetic_locs_are_skipped() {
    // 3 instructions, but only the middle one has a real source location.
    let f = fn_with_locs(
        "f",
        vec![
            IIRInstr::new("const", Some("v".into()),
                vec![interpreter_ir::Operand::Int(0)], "i32"),
            IIRInstr::new("mov", Some("w".into()),
                vec![interpreter_ir::Operand::Var("v".into())], "i32"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
        vec![
            SourceLoc::SYNTHETIC,
            SourceLoc::new(5, 1),
            SourceLoc::SYNTHETIC,
        ],
    );
    let bytes = build_sidecar_from_iir(&single(f), "demo.basic");
    let reader = DebugSidecarReader::new(&bytes).expect("parse");

    // Walk the line-table rows for "f"; only one should be present.
    let rows: Vec<_> = reader.raw_line_rows("f").iter().collect();
    assert_eq!(
        rows.len(),
        1,
        "expected exactly 1 line-row (the non-synthetic middle one); got {rows:?}"
    );
    assert_eq!(rows[0].line, 5);
    assert_eq!(rows[0].col, 1);
    assert_eq!(rows[0].instr_index, 1);
}

// ===========================================================================
// 3. Function range round-trips
// ===========================================================================
//
// The reader doesn't expose `param_count` directly (it's stored but kept
// crate-private).  We assert on the public surface instead: the function
// must appear in `function_names()` and its `function_range` must reflect
// the instruction count we passed.

#[test]
fn function_range_propagates() {
    let mut f = IIRFunction::new(
        "add",
        vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
        "i32",
        vec![
            IIRInstr::new("ret", None,
                vec![interpreter_ir::Operand::Var("a".into())], "i32"),
        ],
    );
    f.source_map = vec![SourceLoc::new(1, 1)];
    let bytes = build_sidecar_from_iir(&single(f), "/tmp/code.bas");
    let reader = DebugSidecarReader::new(&bytes).expect("parse");

    assert!(
        reader.function_names().contains(&"add"),
        "expected `add` in function_names; got: {:?}",
        reader.function_names()
    );
    let (start, end) = reader.function_range("add").expect("range for `add`");
    assert_eq!(start, 0, "start_instr should be 0");
    assert_eq!(end, 1, "end_instr should be the instruction count (1)");
}

// ===========================================================================
// 4. Tolerant of source_map / instructions length mismatch
// ===========================================================================
//
// A frontend bug could leave source_map shorter than instructions (or
// vice-versa).  We must not panic on that — walk the shorter prefix.

#[test]
fn tolerates_shorter_source_map() {
    let mut f = IIRFunction::new(
        "f",
        vec![],
        "void",
        vec![
            IIRInstr::new("const", Some("v".into()),
                vec![interpreter_ir::Operand::Int(0)], "i32"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    f.source_map = vec![SourceLoc::new(7, 1)]; // only 1 entry for 2 instrs
    let bytes = build_sidecar_from_iir(&single(f), "x.bas");
    let reader = DebugSidecarReader::new(&bytes).expect("parse");
    let rows: Vec<_> = reader.raw_line_rows("f").iter().collect();
    assert_eq!(rows.len(), 1, "must walk min(map, instrs) — got {rows:?}");
}

// ===========================================================================
// 5. artifact_info_elf_or_macho — happy shape
// ===========================================================================

#[test]
fn artifact_info_elf_or_macho_smoke() {
    let mut offsets = HashMap::new();
    offsets.insert("main".to_string(), 0usize);
    offsets.insert("helper".to_string(), 128usize);
    let info = artifact_info_elf_or_macho("linux", 0x400000, &offsets, 256);
    assert_eq!(info.target, "linux");
    assert_eq!(info.load_address, 0x400000);
    assert_eq!(info.code_size, 256);
    assert_eq!(info.symbol_table_u64.len(), 2);
    assert_eq!(info.symbol_table_u64["main"], 0);
    assert_eq!(info.symbol_table_u64["helper"], 128);
    assert!(info.symbol_table_u32.is_empty(),
        "elf_or_macho path should NOT populate the u32 table");
}

// ===========================================================================
// 6. artifact_info_pe — happy + overflow rejection
// ===========================================================================

#[test]
fn artifact_info_pe_happy() {
    let mut offsets = HashMap::new();
    offsets.insert("main".to_string(), 0usize);
    let info = artifact_info_pe(0x140000000, &offsets, 0x1000, 1)
        .expect("PE artifact info should build");
    assert_eq!(info.target, "windows");
    assert_eq!(info.image_base, 0x140000000);
    assert_eq!(info.code_rva, 0x1000);
    assert_eq!(info.symbol_table_u32["main"], 0);
}

#[test]
fn artifact_info_pe_rejects_oversized_offset() {
    let mut offsets = HashMap::new();
    // u32::MAX + 1 — definitely doesn't fit.
    offsets.insert("ghost".to_string(), (u32::MAX as usize) + 1);
    // Manual `match` rather than `.expect_err`: `ArtifactInfo` doesn't
    // implement `Debug`, and `Result::expect_err` requires `T: Debug`.
    let err = match artifact_info_pe(0x140000000, &offsets, 0, 1) {
        Ok(_) => panic!("oversized offset should have been rejected"),
        Err(e) => e,
    };
    assert!(
        err.contains("ghost") && err.contains("u32::MAX"),
        "error should name the symbol and explain the cap; got: {err}"
    );
}
