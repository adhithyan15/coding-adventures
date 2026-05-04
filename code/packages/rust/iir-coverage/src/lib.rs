//! # `iir-coverage` — IIR-level coverage projection.
//!
//! **LANG dev-tools D-4** (mirrors LANG18's `tetrad-runtime` line-coverage
//! layer).  First consumer of the [`IIRFunction::source_map`] populated by
//! D-1 ([PR #1834](https://github.com/adhithyan15/coding-adventures/pull/1834)).
//!
//! ## What this crate does
//!
//! Given:
//! - an [`IIRModule`] (with each `IIRFunction` carrying its `source_map:
//!   Vec<SourceLoc>`, lockstep with `instructions`), and
//! - an **execution trace** (per-function set of IIR instruction indices
//!   that were reached during execution),
//!
//! …this crate projects the trace back to **source lines** and returns a
//! [`LineCoverageReport`].  The report answers "which source lines of my
//! program were executed?" without any knowledge of the interpreter, the
//! JIT, or the original source text — just the IR + the trace.
//!
//! ## Why a separate crate
//!
//! The dispatcher (`vm-core` / `lispy-runtime` / `twig-vm`) doesn't need
//! to know about coverage data structures, and the coverage report
//! doesn't need to know about the dispatcher.  Decoupling means:
//!
//! - The dispatcher records an opaque trace (just a `HashMap<String,
//!   HashSet<usize>>`); no allocation per instruction except on the
//!   first hit.
//! - The report is computed off the hot path, on demand, after
//!   execution completes.
//! - Multiple consumers (JSON exporter, terminal formatter, LSP code-lens
//!   provider, …) all build on the same projection.
//! - Tests exercise the projection without spinning up a real VM —
//!   hand-built `IIRModule` + hand-built trace.
//!
//! ## Granularity
//!
//! Per LANG18 §"D1 IIR-level granularity":
//!
//! - The trace is a **set** of IIR instruction indices, not a count.
//!   "Was this IIR step reached at least once" — *not* "how many times".
//!   Loop-iteration counts belong to LANG17's `BranchStats`, a separate
//!   layer.
//! - [`CoveredLine::iir_hit_count`] reports the number of *distinct*
//!   IIR instructions at that source line that ran.  A single source
//!   line typically lowers to several IIR ops (e.g. `y := x + 1` →
//!   `load x`, `add 1`, `store y`).  If all three IIR instructions
//!   for that line ran, `iir_hit_count == 3`.  This is *not* an
//!   execution frequency.
//!
//! ## Synthetic source positions
//!
//! Compiler-synthesised IIR instructions carry [`SourceLoc::SYNTHETIC`]
//! (line `0`).  The projection drops these — synthetic instructions
//! correspond to no source line, so they neither contribute to coverage
//! totals nor appear in the report.
//!
//! ## Multi-file source attribution
//!
//! `IIRFunction.source_map` carries `(line, column)` per instruction but
//! not the source-file name.  In the LANG-VM v1 pipeline, every function
//! in a `IIRModule` belongs to a single source unit; callers pass the
//! source-file path to [`build_report`] and the report tags every covered
//! line with that path.  Multi-file modules (e.g. once Twig modules
//! lower to a single `IIRModule`) will need a richer mapping; we'll add
//! a `build_report_multi_file(module, trace, file_for_function)` overload
//! when the first such consumer appears.

#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use interpreter_ir::IIRModule;

// ---------------------------------------------------------------------------
// CoveredLine + LineCoverageReport
// ---------------------------------------------------------------------------

/// One source line that was reached during execution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CoveredLine {
    /// Source-file path, exactly as supplied to [`build_report`].
    pub file: String,
    /// 1-based source line number.
    pub line: u32,
    /// Number of *distinct* IIR instruction indices at this source line
    /// that were executed.  This is **not** an execution frequency —
    /// see the crate-level docs.
    pub iir_hit_count: u32,
}

/// Source-line coverage produced by composing an IIR execution trace
/// with `IIRFunction::source_map`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LineCoverageReport {
    /// All `(file, line)` pairs that were reached during execution,
    /// in `(file, line)` ascending order.
    covered_lines: Vec<CoveredLine>,
}

impl LineCoverageReport {
    /// Borrow the underlying covered-line slice.
    pub fn covered_lines(&self) -> &[CoveredLine] {
        &self.covered_lines
    }

    /// Return the sorted list of covered line numbers for `path`.
    ///
    /// Returns an empty `Vec` if `path` does not appear in the report.
    pub fn lines_for_file(&self, path: &str) -> Vec<u32> {
        self.covered_lines
            .iter()
            .filter(|cl| cl.file == path)
            .map(|cl| cl.line)
            .collect()
    }

    /// Total number of distinct `(file, line)` pairs that were reached.
    pub fn total_lines_covered(&self) -> usize {
        self.covered_lines.len()
    }

    /// All distinct source-file paths that appear in the report,
    /// sorted ascending.
    pub fn files(&self) -> Vec<&str> {
        let mut out: Vec<&str> = self
            .covered_lines
            .iter()
            .map(|cl| cl.file.as_str())
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Sum of `iir_hit_count` across all covered lines.  Useful as a
    /// rough "instructions executed" metric.
    pub fn total_iir_hits(&self) -> u32 {
        self.covered_lines.iter().map(|cl| cl.iir_hit_count).sum()
    }
}

// ---------------------------------------------------------------------------
// The trace and the projection
// ---------------------------------------------------------------------------

/// One execution trace: per-function set of IIR instruction indices
/// (instruction pointers) that were reached at least once.
///
/// This is the shape a vm-core / lispy-runtime / twig-vm dispatcher
/// produces when coverage mode is enabled.  See LANG18 §"vm-core
/// coverage API" for the dispatcher-side contract.
pub type ExecutionTrace = HashMap<String, HashSet<usize>>;

/// Errors raised by [`build_report`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CoverageError {
    /// The trace referenced a function name that the [`IIRModule`] does
    /// not declare.  Usually indicates trace/module skew (different
    /// build than was traced).
    UnknownFunction {
        /// The offending function name.
        name: String,
    },
    /// The trace referenced an instruction index past the end of the
    /// function's instruction list.  Usually indicates trace/module
    /// skew.
    IpOutOfBounds {
        /// Function name.
        function: String,
        /// Out-of-bounds IP.
        ip: usize,
        /// The function's actual instruction count.
        instruction_count: usize,
    },
    /// The function's `source_map` length doesn't match its
    /// `instructions` length.  Should never happen for IR built via
    /// `twig_ir_compiler::FnCtx::emit` (lockstep is enforced by
    /// construction); fires only if someone builds an `IIRFunction`
    /// manually with mismatched vectors.
    SourceMapDriftedFromInstructions {
        /// Function name.
        function: String,
        /// Length of `instructions`.
        instructions_len: usize,
        /// Length of `source_map`.
        source_map_len: usize,
    },
}

impl std::fmt::Display for CoverageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoverageError::UnknownFunction { name } => {
                write!(f, "trace references unknown function `{name}`")
            }
            CoverageError::IpOutOfBounds { function, ip, instruction_count } => write!(
                f,
                "trace references IP {ip} in `{function}` but it has only {instruction_count} instructions"
            ),
            CoverageError::SourceMapDriftedFromInstructions {
                function,
                instructions_len,
                source_map_len,
            } => write!(
                f,
                "function `{function}` has {instructions_len} instructions but {source_map_len} source-map entries (lockstep violated)"
            ),
        }
    }
}

impl std::error::Error for CoverageError {}

/// Project an execution trace through `module`'s source maps and
/// return a [`LineCoverageReport`] that tags every covered line with
/// `source_file`.
///
/// Synthetic source positions ([`SourceLoc::SYNTHETIC`]) are dropped:
/// they correspond to no source line.
///
/// Errors:
/// - [`CoverageError::UnknownFunction`] — trace mentions a function not
///   in `module` (build skew).
/// - [`CoverageError::IpOutOfBounds`] — trace mentions an IP past the
///   end of a function's instruction list (build skew).
/// - [`CoverageError::SourceMapDriftedFromInstructions`] — `source_map`
///   length disagrees with `instructions` length on a function (would
///   never happen for IR built via the canonical compiler path).
pub fn build_report(
    module: &IIRModule,
    trace: &ExecutionTrace,
    source_file: &str,
) -> Result<LineCoverageReport, CoverageError> {
    // Index functions by name once for O(1) lookup during projection.
    let by_name: HashMap<&str, &interpreter_ir::IIRFunction> = module
        .functions
        .iter()
        .map(|f| (f.name.as_str(), f))
        .collect();

    // Aggregate hits per line, keeping IPs as a set so we count each
    // distinct IIR instruction once even if a function is hit by
    // multiple call sites.  `BTreeMap` so the final report is sorted
    // by line ascending without an extra sort step.
    let mut per_line: BTreeMap<u32, BTreeSet<(String, usize)>> = BTreeMap::new();

    for (fn_name, ips) in trace {
        let func = by_name
            .get(fn_name.as_str())
            .copied()
            .ok_or_else(|| CoverageError::UnknownFunction { name: fn_name.clone() })?;

        if func.instructions.len() != func.source_map.len() {
            return Err(CoverageError::SourceMapDriftedFromInstructions {
                function: fn_name.clone(),
                instructions_len: func.instructions.len(),
                source_map_len: func.source_map.len(),
            });
        }

        for &ip in ips {
            if ip >= func.instructions.len() {
                return Err(CoverageError::IpOutOfBounds {
                    function: fn_name.clone(),
                    ip,
                    instruction_count: func.instructions.len(),
                });
            }
            let loc = func.source_map[ip];
            // Drop synthetic positions — they don't map to any source line.
            if loc.is_synthetic() {
                continue;
            }
            // De-dup by (function, IP) so a given IIR instruction
            // counts at most once even if the trace adds it twice
            // (sets shouldn't, but defensive).
            per_line
                .entry(loc.line)
                .or_default()
                .insert((fn_name.clone(), ip));
        }
    }

    let covered_lines = per_line
        .into_iter()
        .map(|(line, ips)| CoveredLine {
            file: source_file.to_owned(),
            line,
            // Saturate at `u32::MAX`.  Pathologically unreachable
            // (would require >4B distinct IIR instructions all
            // mapped to one source line) but we prefer a defined
            // saturate over silent wrap-around.
            iir_hit_count: u32::try_from(ips.len()).unwrap_or(u32::MAX),
        })
        .collect();

    Ok(LineCoverageReport { covered_lines })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, SourceLoc};

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

    fn empty_module() -> IIRModule {
        IIRModule::new("test", "test")
    }

    fn trace() -> ExecutionTrace {
        HashMap::new()
    }

    fn add(t: &mut ExecutionTrace, fn_name: &str, ips: &[usize]) {
        t.entry(fn_name.to_owned())
            .or_default()
            .extend(ips.iter().copied());
    }

    // ---------- Empty cases ----------

    #[test]
    fn empty_trace_yields_empty_report() {
        let module = empty_module();
        let report = build_report(&module, &trace(), "x.twig").unwrap();
        assert_eq!(report.total_lines_covered(), 0);
        assert!(report.lines_for_file("x.twig").is_empty());
    }

    #[test]
    fn empty_module_with_empty_trace_is_ok() {
        let module = empty_module();
        let report = build_report(&module, &trace(), "x.twig").unwrap();
        assert!(report.covered_lines().is_empty());
        assert_eq!(report.total_iir_hits(), 0);
    }

    // ---------- Basic projection ----------

    #[test]
    fn single_function_single_line() {
        let module = module_with(vec![func_with_source_map(
                "f",
                vec![
                    SourceLoc::new(3, 1),
                    SourceLoc::new(3, 5),
                    SourceLoc::new(3, 9),
                ],
            )]);
        let mut t = trace();
        add(&mut t, "f", &[0, 1, 2]);
        let report = build_report(&module, &t, "src.twig").unwrap();
        assert_eq!(
            report.covered_lines(),
            &[CoveredLine { file: "src.twig".into(), line: 3, iir_hit_count: 3 }]
        );
    }

    #[test]
    fn lines_sorted_ascending() {
        let module = module_with(vec![func_with_source_map(
                "f",
                vec![SourceLoc::new(7, 1), SourceLoc::new(2, 1), SourceLoc::new(5, 1)],
            )]);
        let mut t = trace();
        add(&mut t, "f", &[0, 1, 2]);
        let report = build_report(&module, &t, "src.twig").unwrap();
        let lines: Vec<u32> = report.covered_lines().iter().map(|cl| cl.line).collect();
        assert_eq!(lines, vec![2, 5, 7]);
    }

    #[test]
    fn unhit_instructions_dont_appear() {
        let module = module_with(vec![func_with_source_map(
                "f",
                vec![SourceLoc::new(1, 1), SourceLoc::new(2, 1), SourceLoc::new(3, 1)],
            )]);
        let mut t = trace();
        add(&mut t, "f", &[0, 2]); // Skip IP 1 → line 2 not covered.
        let report = build_report(&module, &t, "src.twig").unwrap();
        let lines: Vec<u32> = report.covered_lines().iter().map(|cl| cl.line).collect();
        assert_eq!(lines, vec![1, 3]);
    }

    // ---------- Synthetic positions are dropped ----------

    #[test]
    fn synthetic_positions_excluded() {
        let module = module_with(vec![func_with_source_map(
                "f",
                vec![
                    SourceLoc::new(1, 1),
                    SourceLoc::SYNTHETIC,
                    SourceLoc::new(2, 1),
                    SourceLoc::SYNTHETIC,
                ],
            )]);
        let mut t = trace();
        add(&mut t, "f", &[0, 1, 2, 3]);
        let report = build_report(&module, &t, "src.twig").unwrap();
        let lines: Vec<u32> = report.covered_lines().iter().map(|cl| cl.line).collect();
        assert_eq!(lines, vec![1, 2]);
        // IIR hit count reflects only the non-synthetic ones (1 each).
        assert_eq!(report.total_iir_hits(), 2);
    }

    #[test]
    fn report_with_only_synthetic_hits_is_empty() {
        let module = module_with(vec![func_with_source_map(
                "f",
                vec![SourceLoc::SYNTHETIC, SourceLoc::SYNTHETIC],
            )]);
        let mut t = trace();
        add(&mut t, "f", &[0, 1]);
        let report = build_report(&module, &t, "src.twig").unwrap();
        assert_eq!(report.total_lines_covered(), 0);
    }

    // ---------- Multi-line aggregation ----------

    #[test]
    fn iir_hit_count_aggregates_distinct_ips_at_same_line() {
        // Three IIR instructions at line 5; all three ran.
        let module = module_with(vec![func_with_source_map(
                "f",
                vec![
                    SourceLoc::new(5, 1),
                    SourceLoc::new(5, 7),
                    SourceLoc::new(5, 12),
                    SourceLoc::new(6, 1),
                ],
            )]);
        let mut t = trace();
        add(&mut t, "f", &[0, 1, 2, 3]);
        let report = build_report(&module, &t, "src.twig").unwrap();
        let line5 = report
            .covered_lines()
            .iter()
            .find(|cl| cl.line == 5)
            .unwrap();
        assert_eq!(line5.iir_hit_count, 3);
        let line6 = report
            .covered_lines()
            .iter()
            .find(|cl| cl.line == 6)
            .unwrap();
        assert_eq!(line6.iir_hit_count, 1);
    }

    #[test]
    fn multiple_functions_share_lines_correctly() {
        // Two functions both with an IIR instruction at line 10 — the
        // (function, ip) tuple keys the dedup so both contribute.
        let module = module_with(vec![
                func_with_source_map("f", vec![SourceLoc::new(10, 1)]),
                func_with_source_map("g", vec![SourceLoc::new(10, 5)]),
            ]);
        let mut t = trace();
        add(&mut t, "f", &[0]);
        add(&mut t, "g", &[0]);
        let report = build_report(&module, &t, "src.twig").unwrap();
        let line10 = &report.covered_lines()[0];
        assert_eq!(line10.line, 10);
        assert_eq!(line10.iir_hit_count, 2);
    }

    // ---------- Helper queries ----------

    #[test]
    fn lines_for_file_returns_sorted_lines() {
        let module = module_with(vec![func_with_source_map(
                "f",
                vec![SourceLoc::new(3, 1), SourceLoc::new(7, 1), SourceLoc::new(1, 1)],
            )]);
        let mut t = trace();
        add(&mut t, "f", &[0, 1, 2]);
        let report = build_report(&module, &t, "src.twig").unwrap();
        assert_eq!(report.lines_for_file("src.twig"), vec![1, 3, 7]);
        assert!(report.lines_for_file("other.twig").is_empty());
    }

    #[test]
    fn files_returns_distinct_paths() {
        // Single-file v1: every line tagged with the same file.
        let module = module_with(vec![func_with_source_map("f", vec![SourceLoc::new(1, 1)])]);
        let mut t = trace();
        add(&mut t, "f", &[0]);
        let report = build_report(&module, &t, "src.twig").unwrap();
        assert_eq!(report.files(), vec!["src.twig"]);
    }

    // ---------- Error cases ----------

    #[test]
    fn unknown_function_in_trace_errors() {
        let module = empty_module();
        let mut t = trace();
        add(&mut t, "missing", &[0]);
        let err = build_report(&module, &t, "src.twig").unwrap_err();
        assert!(matches!(err, CoverageError::UnknownFunction { name } if name == "missing"));
    }

    #[test]
    fn ip_out_of_bounds_errors() {
        let module = module_with(vec![func_with_source_map(
                "f",
                vec![SourceLoc::new(1, 1), SourceLoc::new(2, 1)],
            )]);
        let mut t = trace();
        add(&mut t, "f", &[5]); // function only has 2 instructions
        let err = build_report(&module, &t, "src.twig").unwrap_err();
        assert!(matches!(
            err,
            CoverageError::IpOutOfBounds { ip: 5, instruction_count: 2, .. }
        ));
    }

    #[test]
    fn source_map_drift_errors() {
        // Hand-build a function with mismatched source_map vs instructions.
        let mut func = IIRFunction::new("f", vec![], "any", vec![instr(), instr()]);
        func.source_map = vec![SourceLoc::new(1, 1)]; // length mismatch
        let module = module_with(vec![func]);
        let mut t = trace();
        add(&mut t, "f", &[0]);
        let err = build_report(&module, &t, "src.twig").unwrap_err();
        assert!(matches!(
            err,
            CoverageError::SourceMapDriftedFromInstructions {
                instructions_len: 2,
                source_map_len: 1,
                ..
            }
        ));
    }

    #[test]
    fn coverage_error_displays() {
        let e = CoverageError::UnknownFunction { name: "ghost".into() };
        assert_eq!(e.to_string(), "trace references unknown function `ghost`");
        let e = CoverageError::IpOutOfBounds {
            function: "f".into(),
            ip: 9,
            instruction_count: 3,
        };
        assert_eq!(
            e.to_string(),
            "trace references IP 9 in `f` but it has only 3 instructions"
        );
    }

    // ---------- Realistic mini-trace ----------

    #[test]
    fn realistic_two_function_trace() {
        // main: 3 lines (1, 2, 3) — all hit.
        // helper: 2 lines (10, 11) — only first hit.
        let module = module_with(vec![
                func_with_source_map(
                    "main",
                    vec![
                        SourceLoc::new(1, 1),
                        SourceLoc::new(2, 1),
                        SourceLoc::new(2, 5),
                        SourceLoc::new(3, 1),
                    ],
                ),
                func_with_source_map(
                    "helper",
                    vec![SourceLoc::new(10, 1), SourceLoc::new(11, 1)],
                ),
            ]);
        let mut t = trace();
        add(&mut t, "main", &[0, 1, 2, 3]);
        add(&mut t, "helper", &[0]);
        let report = build_report(&module, &t, "demo.twig").unwrap();
        assert_eq!(report.lines_for_file("demo.twig"), vec![1, 2, 3, 10]);
        assert_eq!(report.total_lines_covered(), 4);
        // Line 2 has 2 IIR hits, the rest 1.
        let line2 = report.covered_lines().iter().find(|cl| cl.line == 2).unwrap();
        assert_eq!(line2.iir_hit_count, 2);
    }
}
