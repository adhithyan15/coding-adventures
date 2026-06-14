//! # `iir-coverage-json` — JSON exporter for `LineCoverageReport`.
//!
//! Stable structured-data sibling of [`iir-coverage-lcov`].  Use
//! whichever fits your downstream tool: lcov for `genhtml` /
//! Codecov / SonarQube; JSON for in-house dashboards, custom CI
//! gates, JS/TS pipelines, or any tool that already speaks JSON.
//!
//! ## Why not bring in `serde_json`?
//!
//! Coding-adventures crates default to **zero dependencies**.  The
//! schema is small enough (one root object, one nested array) that
//! a hand-rolled writer is ~50 LOC and avoids pulling 400+ KLOC of
//! transitive deps just to format a report.
//!
//! ## Schema (v1)
//!
//! ```json
//! {
//!   "schema_version": 1,
//!   "files": [
//!     {
//!       "path": "src/main.twig",
//!       "lines_found": 42,
//!       "lines_hit": 37,
//!       "lines": [
//!         { "line": 1, "iir_hit_count": 3 },
//!         { "line": 2, "iir_hit_count": 1 }
//!       ]
//!     }
//!   ]
//! }
//! ```
//!
//! - `schema_version` — bumped on incompatible changes; consumers
//!   should refuse unknown values.
//! - `files` — sorted by `path` ascending for deterministic output.
//! - Within each file, `lines` is sorted by `line` ascending.
//! - `lines_hit` counts entries with `iir_hit_count > 0`.  Equal to
//!   `lines_found` for any report produced by `iir_coverage::build_report`
//!   (the projection only emits lines that were reached).
//! - `iir_hit_count` is the number of *distinct IIR instructions*
//!   at that source line that ran — see the
//!   [`iir-coverage`](../iir-coverage/) crate docs.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::collections::BTreeMap;

use iir_coverage::LineCoverageReport;

/// Schema version emitted by [`to_json`].  Consumers should refuse
/// JSON whose `schema_version` isn't a value they understand.
pub const SCHEMA_VERSION: u32 = 1;

/// Render `report` as a JSON document.  Schema documented at
/// crate level.
pub fn to_json(report: &LineCoverageReport) -> String {
    // Group covered lines by source file.  `BTreeMap` for
    // deterministic file ordering.
    let mut by_file: BTreeMap<&str, Vec<&iir_coverage::CoveredLine>> = BTreeMap::new();
    for cl in report.covered_lines() {
        by_file.entry(cl.file.as_str()).or_default().push(cl);
    }

    let mut out = String::new();
    out.push('{');
    out.push_str(r#""schema_version":"#);
    out.push_str(&SCHEMA_VERSION.to_string());
    out.push_str(r#","files":["#);
    let mut first_file = true;
    for (file, lines) in &by_file {
        if !first_file {
            out.push(',');
        }
        first_file = false;
        let lines_found = lines.len();
        let lines_hit = lines.iter().filter(|cl| cl.iir_hit_count > 0).count();
        out.push('{');
        out.push_str(r#""path":"#);
        push_json_string(&mut out, file);
        out.push_str(r#","lines_found":"#);
        out.push_str(&lines_found.to_string());
        out.push_str(r#","lines_hit":"#);
        out.push_str(&lines_hit.to_string());
        out.push_str(r#","lines":["#);
        let mut first_line = true;
        for cl in lines {
            if !first_line {
                out.push(',');
            }
            first_line = false;
            out.push('{');
            out.push_str(r#""line":"#);
            out.push_str(&cl.line.to_string());
            out.push_str(r#","iir_hit_count":"#);
            out.push_str(&cl.iir_hit_count.to_string());
            out.push('}');
        }
        out.push_str("]}");
    }
    out.push_str("]}");
    out
}

/// Append `s` to `out` as a JSON string literal — RFC 8259 escapes:
/// `"`, `\`, control chars (`
/// shorthand.  Other UTF-8 passes through verbatim (allowed by RFC
/// 8259 §7).
///
/// This is the **only** place untrusted input crosses into the
/// output; correctness here keeps the emitter injection-proof.
fn push_json_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str(r#"\""#),
            '\\' => out.push_str(r"\\"),
            '\n' => out.push_str(r"\n"),
            '\r' => out.push_str(r"\r"),
            '\t' => out.push_str(r"\t"),
            '\x08' => out.push_str(r"\b"),
            '\x0c' => out.push_str(r"\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use iir_coverage::{build_report, ExecutionTrace};
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, SourceLoc};
    use std::collections::{HashMap, HashSet};

    fn instr() -> IIRInstr {
        IIRInstr::new("nop", None, vec![], "any")
    }

    fn func_with_source_map(name: &str, locs: Vec<SourceLoc>) -> IIRFunction {
        let instructions: Vec<IIRInstr> = locs.iter().map(|_| instr()).collect();
        let mut f = IIRFunction::new(name, vec![], "any", instructions);
        f.source_map = locs;
        f
    }

    fn module_with(functions: Vec<IIRFunction>) -> IIRModule {
        let mut m = IIRModule::new("test", "test");
        m.functions = functions;
        m
    }

    fn trace_with(fn_name: &str, ips: &[usize]) -> ExecutionTrace {
        let mut t: ExecutionTrace = HashMap::new();
        t.insert(fn_name.to_owned(), ips.iter().copied().collect::<HashSet<_>>());
        t
    }

    #[test]
    fn empty_report_yields_well_formed_envelope() {
        let report = LineCoverageReport::default();
        let json = to_json(&report);
        assert_eq!(json, r#"{"schema_version":1,"files":[]}"#);
    }

    #[test]
    fn single_file_three_lines() {
        let module = module_with(vec![func_with_source_map(
            "f",
            vec![
                SourceLoc::new(1, 1),
                SourceLoc::new(2, 1),
                SourceLoc::new(3, 1),
            ],
        )]);
        let trace = trace_with("f", &[0, 2]); // skip line 2
        let report = build_report(&module, &trace, "demo.twig").unwrap();
        let json = to_json(&report);
        let expected = r#"{"schema_version":1,"files":[{"path":"demo.twig","lines_found":2,"lines_hit":2,"lines":[{"line":1,"iir_hit_count":1},{"line":3,"iir_hit_count":1}]}]}"#;
        assert_eq!(json, expected);
    }

    #[test]
    fn multi_iir_per_line_records_iir_hit_count() {
        let module = module_with(vec![func_with_source_map(
            "f",
            vec![
                SourceLoc::new(5, 1),
                SourceLoc::new(5, 7),
                SourceLoc::new(5, 12),
            ],
        )]);
        let trace = trace_with("f", &[0, 1, 2]);
        let report = build_report(&module, &trace, "src.twig").unwrap();
        let json = to_json(&report);
        assert!(json.contains(r#""iir_hit_count":3"#), "got:\n{json}");
        assert!(json.contains(r#""lines_found":1"#));
        assert!(json.contains(r#""lines_hit":1"#));
    }

    #[test]
    fn lines_sorted_ascending_in_json_output() {
        let module = module_with(vec![func_with_source_map(
            "f",
            vec![
                SourceLoc::new(7, 1),
                SourceLoc::new(2, 1),
                SourceLoc::new(5, 1),
            ],
        )]);
        let trace = trace_with("f", &[0, 1, 2]);
        let report = build_report(&module, &trace, "src.twig").unwrap();
        let json = to_json(&report);
        let p2 = json.find(r#""line":2"#).unwrap();
        let p5 = json.find(r#""line":5"#).unwrap();
        let p7 = json.find(r#""line":7"#).unwrap();
        assert!(p2 < p5 && p5 < p7);
    }

    // ---------- JSON-string escaping (security) ----------

    #[test]
    fn json_escapes_quote_and_backslash_in_path() {
        let module = module_with(vec![func_with_source_map(
            "f",
            vec![SourceLoc::new(1, 1)],
        )]);
        let trace = trace_with("f", &[0]);
        // Path containing both " and \ — must be escaped, not break the JSON.
        let report = build_report(&module, &trace, r#"a"b\c.twig"#).unwrap();
        let json = to_json(&report);
        assert!(json.contains(r#""path":"a\"b\\c.twig""#), "got:\n{json}");
    }

    #[test]
    fn json_escapes_newlines_and_control_chars_in_path() {
        let module = module_with(vec![func_with_source_map(
            "f",
            vec![SourceLoc::new(1, 1)],
        )]);
        let trace = trace_with("f", &[0]);
        let path = "evil\n\t\r\x07.twig"; // newline/tab/CR/bell
        let report = build_report(&module, &trace, path).unwrap();
        let json = to_json(&report);
        // No literal control chars.
        assert!(!json.contains('\n'));
        assert!(!json.contains('\t'));
        assert!(!json.contains('\r'));
        assert!(!json.contains('\x07'));
        // \n/\t/\r appear as escape sequences; bell encoded as .
        assert!(json.contains(r#"\n"#));
        assert!(json.contains(r#"\t"#));
        assert!(json.contains(r#"\r"#));
        assert!(json.contains("\\u0007"), "got:\n{json}");
    }

    #[test]
    fn unicode_passes_through_verbatim() {
        // RFC 8259 allows raw UTF-8 in strings (only `"`, `\`, and
        // control chars must be escaped).
        let module = module_with(vec![func_with_source_map(
            "f",
            vec![SourceLoc::new(1, 1)],
        )]);
        let trace = trace_with("f", &[0]);
        let report = build_report(&module, &trace, "漢字 🦀.twig").unwrap();
        let json = to_json(&report);
        assert!(json.contains(r#""path":"漢字 🦀.twig""#));
    }

    #[test]
    fn schema_version_is_emitted() {
        let report = LineCoverageReport::default();
        let json = to_json(&report);
        assert!(json.starts_with(r#"{"schema_version":1,"#));
    }

    #[test]
    fn output_is_compact_no_whitespace() {
        // We don't pretty-print.  Consumers that want pretty output
        // should pipe through `jq .` or similar.  Tight check that
        // we don't accidentally emit spaces.
        let module = module_with(vec![func_with_source_map(
            "f",
            vec![SourceLoc::new(1, 1)],
        )]);
        let trace = trace_with("f", &[0]);
        let report = build_report(&module, &trace, "x").unwrap();
        let json = to_json(&report);
        assert!(!json.contains(": "));
        assert!(!json.contains(", "));
    }

    #[test]
    fn realistic_two_file_report() {
        // Trick: build one report with multiple files by hand — but
        // the v1 build_report only tags everything with one file.
        // Run it twice and merge the covered lines into a single
        // LineCoverageReport via the public constructor… which
        // doesn't exist.  Settle for a single-file realistic case
        // here; multi-file lands when iir_coverage adds multi-file
        // support.
        let module = module_with(vec![
            func_with_source_map("main", vec![SourceLoc::new(1, 1), SourceLoc::new(2, 1)]),
            func_with_source_map("helper", vec![SourceLoc::new(10, 1), SourceLoc::new(11, 1)]),
        ]);
        let mut trace: ExecutionTrace = HashMap::new();
        trace.insert("main".into(), HashSet::from([0_usize, 1_usize]));
        trace.insert("helper".into(), HashSet::from([0_usize]));
        let report = build_report(&module, &trace, "demo.twig").unwrap();
        let json = to_json(&report);
        assert!(json.contains(r#""path":"demo.twig""#));
        assert!(json.contains(r#""lines_found":3"#));
        assert!(json.contains(r#""lines_hit":3"#));
    }
}
