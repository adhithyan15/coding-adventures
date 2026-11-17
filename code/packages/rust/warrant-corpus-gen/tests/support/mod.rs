//! Shared helpers for the integration tests: parse a document from text, and a tiny
//! boilerplate builder for synthetic documents.

#![allow(dead_code)]

use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{parse_document, ParserLimits, WarrantFile};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

/// Parse `src` and run `f` on the file.  (A `WarrantFile` borrows its source, so it
/// cannot be returned.)
pub fn with_file<R>(src: &str, f: impl FnOnce(&WarrantFile<'_>) -> R) -> R {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    f(&file)
}

pub const CT_COMPARE: &str = include_str!("../../../../../specs/ct-compare.warrant");
pub const GF256: &str = include_str!("../../../../../specs/gf256.warrant");
pub const CORE_V0: &str = include_str!("../fixtures/core_forms_v0.warrant");
pub const CORE_V1: &str = include_str!("../fixtures/core_forms_v1.warrant");
pub const ENUM_V1: &str = include_str!("../fixtures/enum_forms_v1.warrant");
pub const ENUM_PROBE: &str = include_str!("../fixtures/enum_probe.warrant");
pub const ENUM_V0: &str = include_str!("../fixtures/enum_forms_v0.warrant");

/// The contract boilerplate every operation needs (labels, input contracts, bounds),
/// for a synthetic document whose operations take only `Bytes` parameters named `a`
/// and `b` (so it can be appended to any such document).
pub fn bytes_op_contracts(ops: &[&str]) -> String {
    let mut s = String::from("  assume platform:\n    none\n  assume environment:\n    none\n");
    for op in ops {
        s += &format!(
            "  labels {op}:\n    a.contents secret\n    a.length public\n    b.contents secret\n    b.length public\n"
        );
    }
    for op in ops {
        for p in ["a", "b"] {
            s += &format!("  input {op}.{p}:\n    hostile\n    ensures no_panic, terminates\n");
        }
    }
    for op in ops {
        s += &format!("  bound {op}:\n    steps <= 64 * len(a) + 512\n    heap <= 2 * len(a)\n    depth <= 32\n");
    }
    s
}

/// A v0 or v1 document with one `op f(a: Bytes, b: Bytes) -> Bool`, the given extra
/// declarations (models, claims, ...) and the boilerplate.
pub fn doc_with(version_suffix: &str, extra: &str) -> String {
    format!(
        "warrant synth{version_suffix}\n  op f(a: Bytes, b: Bytes) -> Bool\n{}{extra}",
        bytes_op_contracts(&["f"])
    )
}

/// A document whose operations are all `(a: Bytes, b: Bytes)` operations (so one
/// boilerplate fits), given as declaration lines, plus extra declarations.
pub fn doc_ops(version_suffix: &str, op_lines: &[&str], extra: &str) -> String {
    let mut names: Vec<&str> = Vec::new();
    let mut s = format!("warrant synth{version_suffix}\n");
    for line in op_lines {
        s += &format!("  {line}\n");
        let name = line.trim_start_matches("op ").split('(').next().unwrap();
        names.push(name);
    }
    s += &bytes_op_contracts(&names);
    s += extra;
    s
}
