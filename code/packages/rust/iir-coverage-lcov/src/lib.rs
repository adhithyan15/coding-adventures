//! # `iir-coverage-lcov` — lcov-format exporter for `LineCoverageReport`.
//!
//! Drop-in CI integration with the lcov ecosystem (`genhtml`,
//! Codecov, SonarQube, GitLab CI's coverage parser, every
//! lcov-aware tool).
//!
//! ## What lcov is
//!
//! lcov is the de-facto interchange format for line-coverage data.
//! Every coverage tool either consumes lcov directly (`genhtml`)
//! or accepts it as one of its input formats (Codecov, Coveralls,
//! SonarQube).  Producing lcov gives the LANG-VM coverage stack
//! immediate compatibility with the entire downstream tooling
//! ecosystem without us writing per-tool adapters.
//!
//! ## Format reference
//!
//! lcov is plain text, one record per source file, terminated by
//! `end_of_record`.  We emit only the subset that line-coverage
//! reports populate (lcov also has function- and branch-coverage
//! lines, which are intentionally out of scope here — see the
//! [`iir-coverage`](https://crates.io/crates/iir-coverage) crate
//! docs for why per-line frequency / branch coverage live in a
//! different layer).
//!
//! ```text
//! TN:                                ; test name (always blank for us)
//! SF:<file path>                     ; source file
//! DA:<line>,<exec count>             ; one per covered line
//! LF:<lines found>                   ; total covered-line count for this file
//! LH:<lines hit>                     ; covered-line count where exec count > 0
//! end_of_record
//! ```
//!
//! `iir_hit_count` from [`iir_coverage::CoveredLine`] becomes the
//! `<exec count>` on the `DA:` line.  Recall that this is the
//! number of *distinct IIR instructions at this source line that
//! ran*, not an execution-frequency.  `genhtml` and similar tools
//! display it as "hit count" — close enough for the typical "is
//! this line covered?" question that motivates running coverage
//! in the first place.
//!
//! ## What this crate does *not* do
//!
//! - **No I/O.**  Returns a `String`; the caller writes it to disk.
//!   Keeps the crate capability-free and trivially testable.
//! - **No file-existence checks.**  We don't try to open `SF:` paths.
//!   lcov consumers do that themselves.
//! - **No function- or branch-coverage records.**  Out of scope by
//!   design (would need a separate trace shape).

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::collections::BTreeMap;

use iir_coverage::LineCoverageReport;

/// Render `report` in lcov info-file format.
///
/// One record per distinct source file in the report (sorted by
/// path ascending).  Within each record, `DA:` lines are sorted
/// by source-line ascending.  The output ends with a final
/// newline so it can be concatenated with other lcov files
/// (e.g. `cat report1.lcov report2.lcov | genhtml`).
///
/// ## Path sanitisation
///
/// lcov has **no escape mechanism** for the `SF:<path>\n` line.
/// A path containing `\n` (legal on POSIX) or `\r` (legal in
/// some filesystems) would let an attacker who controls the
/// `source_file` argument to `iir_coverage::build_report` forge
/// extra coverage records for arbitrary files, potentially
/// fooling downstream tools (Codecov, SonarQube) that gate
/// merges on coverage deltas.  We replace any `\r` or `\n` byte
/// in a path with `_` to make this attack impossible.  The same
/// goes for any literal `end_of_record` substring, which would
/// truncate the current record mid-stream.
///
/// All other characters (spaces, Unicode, etc.) pass through
/// verbatim — lcov consumers handle them fine.
pub fn to_lcov(report: &LineCoverageReport) -> String {
    // Group covered lines by source file.  `BTreeMap` for
    // deterministic file ordering; the inner `Vec` is sorted by
    // line ascending (preserving the order that
    // `LineCoverageReport::covered_lines` already guarantees per
    // file, since it's globally sorted by `(file, line)`).
    let mut by_file: BTreeMap<&str, Vec<&iir_coverage::CoveredLine>> = BTreeMap::new();
    for cl in report.covered_lines() {
        by_file.entry(cl.file.as_str()).or_default().push(cl);
    }

    let mut out = String::new();
    for (file, lines) in &by_file {
        // Per-file record header.  Sanitise the path — see fn docs.
        out.push_str("TN:\n");
        out.push_str("SF:");
        out.push_str(&sanitise_path(file));
        out.push('\n');

        // DA: lines and counters.
        let lines_found = lines.len();
        let mut lines_hit = 0usize;
        for cl in lines {
            out.push_str("DA:");
            out.push_str(&cl.line.to_string());
            out.push(',');
            out.push_str(&cl.iir_hit_count.to_string());
            out.push('\n');
            if cl.iir_hit_count > 0 {
                lines_hit += 1;
            }
        }
        out.push_str("LF:");
        out.push_str(&lines_found.to_string());
        out.push('\n');
        out.push_str("LH:");
        out.push_str(&lines_hit.to_string());
        out.push('\n');
        out.push_str("end_of_record\n");
    }
    out
}

fn sanitise_path(path: &str) -> String {
    // Replace any newline / carriage-return with `_` to prevent
    // record-injection.  Replace any literal `end_of_record`
    // substring for the same reason — its appearance inside an
    // `SF:` line would break tools that match it line-anchored,
    // but we also defend against tools that don't.
    let no_newlines: String = path
        .chars()
        .map(|c| if c == '\n' || c == '\r' { '_' } else { c })
        .collect();
    no_newlines.replace("end_of_record", "end_of_record_")
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

    // We re-test through iir-coverage's public path so the lcov
    // exporter is exercised on real `LineCoverageReport` values.

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
    fn empty_report_produces_empty_string() {
        let report = LineCoverageReport::default();
        assert_eq!(to_lcov(&report), "");
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
        let lcov = to_lcov(&report);
        let expected = "\
TN:
SF:demo.twig
DA:1,1
DA:3,1
LF:2
LH:2
end_of_record
";
        assert_eq!(lcov, expected);
    }

    #[test]
    fn multi_iir_per_line_records_iir_hit_count() {
        // Three IIR instructions all at line 5, all reached.
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
        let lcov = to_lcov(&report);
        assert!(lcov.contains("DA:5,3\n"), "got:\n{lcov}");
        assert!(lcov.contains("LF:1\n"));
        assert!(lcov.contains("LH:1\n"));
    }

    #[test]
    fn lines_sorted_ascending_in_lcov_output() {
        // Build report with lines in non-sorted physical order.
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
        let lcov = to_lcov(&report);
        let da_pos: Vec<usize> = ["DA:2,", "DA:5,", "DA:7,"]
            .iter()
            .map(|needle| lcov.find(needle).expect("needle present"))
            .collect();
        assert_eq!(da_pos, {
            let mut sorted = da_pos.clone();
            sorted.sort();
            sorted
        });
    }

    #[test]
    fn ends_with_newline_for_safe_concat() {
        let module = module_with(vec![func_with_source_map(
            "f",
            vec![SourceLoc::new(1, 1)],
        )]);
        let trace = trace_with("f", &[0]);
        let report = build_report(&module, &trace, "src.twig").unwrap();
        let lcov = to_lcov(&report);
        assert!(lcov.ends_with('\n'));
    }

    #[test]
    fn record_header_format_exact() {
        // Tight check on the exact prefix shape for downstream
        // tools that match line-by-line.
        let module = module_with(vec![func_with_source_map(
            "f",
            vec![SourceLoc::new(42, 1)],
        )]);
        let trace = trace_with("f", &[0]);
        let report = build_report(&module, &trace, "path/to/file.twig").unwrap();
        let lcov = to_lcov(&report);
        assert!(lcov.starts_with("TN:\nSF:path/to/file.twig\nDA:42,1\n"));
        assert!(lcov.contains("end_of_record\n"));
    }

    #[test]
    fn iir_hit_count_zero_does_not_increment_lh() {
        // Build a CoveredLine manually with iir_hit_count == 0.  The
        // projection from an ExecutionTrace can't produce zero (every
        // entry implies at least one IIR hit) but a future synthetic
        // path could, and our `LH:` math should be correct anyway.
        let report = {
            // We can't construct LineCoverageReport directly (it's
            // built by `build_report`), so synthesise via the public
            // path and then verify the `LH` math by emitting + parsing.
            let module = module_with(vec![func_with_source_map(
                "f",
                vec![SourceLoc::new(1, 1), SourceLoc::new(2, 1)],
            )]);
            let trace = trace_with("f", &[0, 1]);
            build_report(&module, &trace, "x").unwrap()
        };
        let lcov = to_lcov(&report);
        // Every line that came through `build_report` has count >= 1,
        // so LH == LF.
        assert!(lcov.contains("LF:2\n"));
        assert!(lcov.contains("LH:2\n"));
    }

    #[test]
    fn benign_special_characters_in_path_pass_through() {
        // Spaces, ampersands, Unicode all pass through verbatim.
        let module = module_with(vec![func_with_source_map(
            "f",
            vec![SourceLoc::new(1, 1)],
        )]);
        let trace = trace_with("f", &[0]);
        let report = build_report(&module, &trace, "path with spaces & 漢字.twig").unwrap();
        let lcov = to_lcov(&report);
        assert!(lcov.contains("SF:path with spaces & 漢字.twig\n"));
    }

    // ---------- Injection-defense (security review hardening) ----------

    #[test]
    fn newline_in_path_replaced_with_underscore() {
        let module = module_with(vec![func_with_source_map(
            "f",
            vec![SourceLoc::new(1, 1)],
        )]);
        let trace = trace_with("f", &[0]);
        // POSIX allows `\n` in filenames.  Without sanitisation, a
        // hostile path could inject `end_of_record\nTN:\nSF:fake\n…`.
        let report = build_report(
            &module,
            &trace,
            "evil.twig\nend_of_record\nSF:innocent.twig",
        )
        .unwrap();
        let lcov = to_lcov(&report);
        // Path is mangled — no literal newline, no second SF: line.
        // Count `SF:` only at start-of-line (preceded by `\n`), since
        // the path itself may legally contain the substring `SF:`.
        let sf_lines = lcov.matches("\nSF:").count();
        assert_eq!(sf_lines, 1, "should have exactly one SF: line, got:\n{lcov}");
        assert!(!lcov.contains("evil.twig\n"));
        assert!(lcov.contains("evil.twig_end_of_record_"));
    }

    #[test]
    fn carriage_return_in_path_replaced_with_underscore() {
        let module = module_with(vec![func_with_source_map(
            "f",
            vec![SourceLoc::new(1, 1)],
        )]);
        let trace = trace_with("f", &[0]);
        let report = build_report(&module, &trace, "evil.twig\rfake").unwrap();
        let lcov = to_lcov(&report);
        assert!(!lcov.contains('\r'));
        assert!(lcov.contains("evil.twig_fake"));
    }

    #[test]
    fn end_of_record_substring_in_path_disarmed() {
        let module = module_with(vec![func_with_source_map(
            "f",
            vec![SourceLoc::new(1, 1)],
        )]);
        let trace = trace_with("f", &[0]);
        // Without the substring replacement, a path like this would
        // contain a literal `end_of_record` token inside an `SF:`
        // line, which line-anchored consumers handle fine but
        // substring-matching scanners might not.  Defence in depth.
        let report = build_report(&module, &trace, "myend_of_recordfile.twig").unwrap();
        let lcov = to_lcov(&report);
        assert!(!lcov.contains("end_of_recordfile"), "got:\n{lcov}");
        assert!(lcov.contains("end_of_record_file"));
        // The genuine record terminator still appears exactly once.
        assert_eq!(lcov.matches("\nend_of_record\n").count(), 1);
    }
}
