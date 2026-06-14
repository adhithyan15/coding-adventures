//! [`DebugSidecarReader`] — queries source-location data at runtime.
//!
//! The reader is the consumer side of the debug sidecar pipeline.  It loads
//! the bytes produced by [`DebugSidecarWriter::finish`][`crate::DebugSidecarWriter::finish`]
//! and answers three kinds of questions:
//!
//! 1. **Offset → source** (debugger paused):
//!    [`lookup`][`DebugSidecarReader::lookup`] → `Option<SourceLocation>`
//!
//! 2. **Source → offset** (setting a breakpoint):
//!    [`find_instr`][`DebugSidecarReader::find_instr`] → `Option<usize>`
//!
//! 3. **Variable inspection** (Variables panel):
//!    [`live_variables`][`DebugSidecarReader::live_variables`] → `Vec<Variable>`
//!
//! # Example
//!
//! ```
//! use debug_sidecar::{DebugSidecarWriter, DebugSidecarReader};
//!
//! let mut w = DebugSidecarWriter::new();
//! let fid = w.add_source_file("fib.tetrad", b"");
//! w.begin_function("fib", 0, 1);
//! w.record("fib", 3, fid, 7, 5);
//! w.record("fib", 7, fid, 10, 1);
//! w.end_function("fib", 12);
//! w.declare_variable("fib", 0, "n", "u8", 0, 12);
//!
//! let reader = DebugSidecarReader::new(&w.finish()).unwrap();
//!
//! let loc = reader.lookup("fib", 3).unwrap();
//! assert_eq!(loc.to_string(), "fib.tetrad:7:5");
//!
//! let idx = reader.find_instr("fib.tetrad", 7).unwrap();
//! assert_eq!(idx, 3);
//!
//! let vars = reader.live_variables("fib", 5);
//! assert_eq!(vars.len(), 1);
//! assert_eq!(vars[0].name, "n");
//! ```

use serde_json::Value;

use crate::types::{SourceLocation, Variable};

// ---------------------------------------------------------------------------
// Internal row types (deserialized from JSON)
// ---------------------------------------------------------------------------

/// A single row in the line number table (exposed for `native-debug-info`).
#[derive(Debug, Clone)]
pub struct LineRow {
    /// 0-based instruction index within the function body.
    pub instr_index: usize,
    /// 0-based file ID (index into the source_files list).
    pub file_id: usize,
    /// 1-based line number.
    pub line: u32,
    /// 1-based column number.
    pub col: u32,
}

#[derive(Debug, Clone)]
struct FunctionEntry {
    start_instr: usize,
    end_instr: Option<usize>,
    #[allow(dead_code)]
    param_count: usize,
}

#[derive(Debug, Clone)]
struct RawVariable {
    reg_index: u32,
    name: String,
    type_hint: String,
    live_start: usize,
    live_end: usize,
}

// ---------------------------------------------------------------------------
// DebugSidecarReader
// ---------------------------------------------------------------------------

/// Answers debug queries from a compiled sidecar.
///
/// Parse errors are exposed as [`SidecarError`].
///
/// The line-number lookup in [`lookup`][`Self::lookup`] uses a binary-search
/// "last row whose index ≤ N" algorithm identical to the DWARF lookup
/// semantics — a generated instruction that maps to no explicit row still
/// gets the location of the nearest preceding recorded instruction.
#[derive(Debug)]
pub struct DebugSidecarReader {
    source_files: Vec<String>,        // path for each file_id
    line_table: std::collections::HashMap<String, Vec<LineRow>>,
    functions: std::collections::HashMap<String, FunctionEntry>,
    variables: std::collections::HashMap<String, Vec<RawVariable>>,
    /// Pre-sorted instr_index lists for bisect lookups.
    sorted_indices: std::collections::HashMap<String, Vec<usize>>,
}

/// Error returned when sidecar bytes cannot be parsed.
#[derive(Debug)]
pub struct SidecarError(pub String);

impl std::fmt::Display for SidecarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SidecarError: {}", self.0)
    }
}

impl std::error::Error for SidecarError {}

impl DebugSidecarReader {
    /// Parse a sidecar produced by [`DebugSidecarWriter::finish`][`crate::DebugSidecarWriter::finish`].
    ///
    /// # Errors
    ///
    /// Returns [`SidecarError`] if the bytes are not valid UTF-8 JSON, or if
    /// the `"version"` field is not `1`.
    pub fn new(data: &[u8]) -> Result<Self, SidecarError> {
        let payload: Value = serde_json::from_slice(data)
            .map_err(|e| SidecarError(format!("invalid sidecar data: {e}")))?;

        match payload.get("version").and_then(|v| v.as_u64()) {
            Some(1) => {}
            v => return Err(SidecarError(format!("unsupported sidecar version: {v:?}"))),
        }

        // Parse source_files.
        let source_files: Vec<String> = payload
            .get("source_files")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|e| e.get("path")?.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // Parse line_table.
        let mut line_table: std::collections::HashMap<String, Vec<LineRow>> =
            std::collections::HashMap::new();
        if let Some(lt) = payload.get("line_table").and_then(|v| v.as_object()) {
            for (fn_name, rows_val) in lt {
                let rows: Vec<LineRow> = rows_val
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|r| {
                        Some(LineRow {
                            instr_index: r.get("instr_index")?.as_u64()? as usize,
                            file_id: r.get("file_id")?.as_u64()? as usize,
                            line: r.get("line")?.as_u64()? as u32,
                            col: r.get("col")?.as_u64()? as u32,
                        })
                    })
                    .collect();
                line_table.insert(fn_name.clone(), rows);
            }
        }

        // Parse functions.
        let mut functions: std::collections::HashMap<String, FunctionEntry> =
            std::collections::HashMap::new();
        if let Some(fns) = payload.get("functions").and_then(|v| v.as_object()) {
            for (fn_name, fv) in fns {
                let start_instr = fv.get("start_instr").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let end_instr = fv.get("end_instr").and_then(|v| v.as_u64()).map(|v| v as usize);
                let param_count = fv.get("param_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                functions.insert(fn_name.clone(), FunctionEntry { start_instr, end_instr, param_count });
            }
        }

        // Parse variables.
        let mut variables: std::collections::HashMap<String, Vec<RawVariable>> =
            std::collections::HashMap::new();
        if let Some(vars) = payload.get("variables").and_then(|v| v.as_object()) {
            for (fn_name, varr) in vars {
                let vs: Vec<RawVariable> = varr
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|v| {
                        Some(RawVariable {
                            reg_index: v.get("reg_index")?.as_u64()? as u32,
                            name: v.get("name")?.as_str()?.to_string(),
                            type_hint: v.get("type_hint")?.as_str()?.to_string(),
                            live_start: v.get("live_start")?.as_u64()? as usize,
                            live_end: v.get("live_end")?.as_u64()? as usize,
                        })
                    })
                    .collect();
                variables.insert(fn_name.clone(), vs);
            }
        }

        // Pre-build sorted indices for bisect lookups.
        let sorted_indices: std::collections::HashMap<String, Vec<usize>> = line_table
            .iter()
            .map(|(fn_name, rows)| {
                (fn_name.clone(), rows.iter().map(|r| r.instr_index).collect())
            })
            .collect();

        Ok(Self {
            source_files,
            line_table,
            functions,
            variables,
            sorted_indices,
        })
    }

    // ------------------------------------------------------------------
    // Source location lookup (offset → source)
    // ------------------------------------------------------------------

    /// Return the source location of instruction `instr_index` in `fn_name`.
    ///
    /// Uses the last row whose index is ≤ `instr_index`, matching the DWARF
    /// line-number lookup algorithm.  A generated instruction with no explicit
    /// record still gets the location of the nearest preceding recorded row.
    ///
    /// Returns `None` if the function has no debug information or if
    /// `instr_index` is before the first recorded instruction.
    pub fn lookup(&self, fn_name: &str, instr_index: usize) -> Option<SourceLocation> {
        let rows = self.line_table.get(fn_name)?;
        let indices = self.sorted_indices.get(fn_name)?;

        // Binary search: find the last index ≤ instr_index.
        // partition_point gives the first index > instr_index, so subtract 1.
        let pos = indices.partition_point(|&i| i <= instr_index);
        if pos == 0 {
            return None;
        }
        let row = &rows[pos - 1];
        let path = self.source_files.get(row.file_id)?.clone();
        Some(SourceLocation { file: path, line: row.line, col: row.col })
    }

    // ------------------------------------------------------------------
    // Reverse lookup (source → offset)
    // ------------------------------------------------------------------

    /// Return the first instruction index that maps to `(file, line)`.
    ///
    /// Scans all functions for a row matching the given file path and line
    /// number.  Returns the lowest matching instruction index, or `None` if
    /// the source line is not reachable.
    pub fn find_instr(&self, file: &str, line: u32) -> Option<usize> {
        // Resolve file path to file_id.
        let file_id = self.source_files.iter().position(|p| p == file)?;

        let mut best: Option<usize> = None;
        for rows in self.line_table.values() {
            for row in rows {
                if row.file_id == file_id && row.line == line {
                    let idx = row.instr_index;
                    best = Some(best.map_or(idx, |b: usize| b.min(idx)));
                }
            }
        }
        best
    }

    // ------------------------------------------------------------------
    // Variable inspection
    // ------------------------------------------------------------------

    /// Return all variables live at instruction `at_instr` in `fn_name`.
    ///
    /// A variable is live when `live_start <= at_instr < live_end`.
    /// The result is sorted by `reg_index`.
    pub fn live_variables(&self, fn_name: &str, at_instr: usize) -> Vec<Variable> {
        let raw = match self.variables.get(fn_name) {
            Some(v) => v,
            None => return Vec::new(),
        };
        let mut result: Vec<Variable> = raw
            .iter()
            .filter(|v| v.live_start <= at_instr && at_instr < v.live_end)
            .map(|v| Variable {
                reg_index: v.reg_index,
                name: v.name.clone(),
                type_hint: v.type_hint.clone(),
                live_start: v.live_start,
                live_end: v.live_end,
            })
            .collect();
        result.sort_by_key(|v| v.reg_index);
        result
    }

    // ------------------------------------------------------------------
    // Metadata
    // ------------------------------------------------------------------

    /// Return the list of source file paths registered in this sidecar.
    pub fn source_files(&self) -> &[String] {
        &self.source_files
    }

    /// Return the list of function names that have debug information.
    pub fn function_names(&self) -> Vec<&str> {
        self.functions.keys().map(|s| s.as_str()).collect()
    }

    /// Return the `(start_instr, end_instr)` range for `fn_name`.
    ///
    /// Returns `None` if the function was not registered or if
    /// `end_function()` was never called.
    pub fn function_range(&self, fn_name: &str) -> Option<(usize, usize)> {
        let entry = self.functions.get(fn_name)?;
        let end = entry.end_instr?;
        Some((entry.start_instr, end))
    }

    /// Return the raw sorted line rows for a function (used by native-debug-info).
    ///
    /// Returns an empty slice if the function is not in the line table.
    pub fn raw_line_rows(&self, fn_name: &str) -> &[LineRow] {
        self.line_table.get(fn_name).map(|v| v.as_slice()).unwrap_or(&[])
    }
}


// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DebugSidecarWriter;

    fn round_trip_writer() -> (DebugSidecarWriter, usize) {
        let mut w = DebugSidecarWriter::new();
        let fid = w.add_source_file("fib.tetrad", b"");
        w.begin_function("fib", 0, 1);
        w.record("fib", 3, fid, 7, 5);
        w.record("fib", 7, fid, 10, 1);
        w.end_function("fib", 12);
        w.declare_variable("fib", 0, "n", "u8", 0, 12);
        (w, fid)
    }

    fn make_reader() -> DebugSidecarReader {
        let (w, _) = round_trip_writer();
        DebugSidecarReader::new(&w.finish()).unwrap()
    }

    #[test]
    fn reader_parses_valid_data() {
        let r = make_reader();
        assert!(r.source_files.contains(&"fib.tetrad".to_string()));
    }

    #[test]
    fn reader_invalid_json_returns_error() {
        let result = DebugSidecarReader::new(b"not json");
        assert!(result.is_err());
    }

    #[test]
    fn reader_wrong_version_returns_error() {
        let json = r#"{"version":2,"source_files":[],"line_table":{},"functions":{},"variables":{}}"#;
        let result = DebugSidecarReader::new(json.as_bytes());
        assert!(result.is_err());
    }

    #[test]
    fn lookup_exact_match() {
        let r = make_reader();
        let loc = r.lookup("fib", 3).unwrap();
        assert_eq!(loc.line, 7);
        assert_eq!(loc.col, 5);
        assert_eq!(loc.file, "fib.tetrad");
    }

    #[test]
    fn lookup_between_rows_returns_preceding() {
        let r = make_reader();
        // instr 5 is between rows 3 and 7 — should return row at 3
        let loc = r.lookup("fib", 5).unwrap();
        assert_eq!(loc.line, 7);
    }

    #[test]
    fn lookup_at_second_row() {
        let r = make_reader();
        let loc = r.lookup("fib", 7).unwrap();
        assert_eq!(loc.line, 10);
    }

    #[test]
    fn lookup_before_first_row_returns_none() {
        let r = make_reader();
        // instr 0,1,2 are before the first row at 3
        assert!(r.lookup("fib", 0).is_none());
        assert!(r.lookup("fib", 2).is_none());
    }

    #[test]
    fn lookup_unknown_function_returns_none() {
        let r = make_reader();
        assert!(r.lookup("unknown_fn", 0).is_none());
    }

    #[test]
    fn find_instr_known_line() {
        let r = make_reader();
        let idx = r.find_instr("fib.tetrad", 7).unwrap();
        assert_eq!(idx, 3);
    }

    #[test]
    fn find_instr_unknown_file_returns_none() {
        let r = make_reader();
        assert!(r.find_instr("other.t", 7).is_none());
    }

    #[test]
    fn find_instr_unknown_line_returns_none() {
        let r = make_reader();
        assert!(r.find_instr("fib.tetrad", 999).is_none());
    }

    #[test]
    fn live_variables_at_start() {
        let r = make_reader();
        let vars = r.live_variables("fib", 0);
        assert_eq!(vars.len(), 1);
        assert_eq!(vars[0].name, "n");
    }

    #[test]
    fn live_variables_at_end_exclusive() {
        let r = make_reader();
        let vars = r.live_variables("fib", 12); // live_end=12, not live
        assert!(vars.is_empty());
    }

    #[test]
    fn live_variables_unknown_function() {
        let r = make_reader();
        let vars = r.live_variables("unknown", 5);
        assert!(vars.is_empty());
    }

    #[test]
    fn live_variables_sorted_by_reg_index() {
        let mut w = DebugSidecarWriter::new();
        w.declare_variable("f", 5, "z", "", 0, 10);
        w.declare_variable("f", 1, "a", "", 0, 10);
        w.declare_variable("f", 3, "m", "", 0, 10);
        let r = DebugSidecarReader::new(&w.finish()).unwrap();
        let vars = r.live_variables("f", 5);
        assert_eq!(vars[0].reg_index, 1);
        assert_eq!(vars[1].reg_index, 3);
        assert_eq!(vars[2].reg_index, 5);
    }

    #[test]
    fn source_files_returns_paths() {
        let r = make_reader();
        assert!(r.source_files().contains(&"fib.tetrad".to_string()));
    }

    #[test]
    fn function_range_returns_start_end() {
        let r = make_reader();
        let range = r.function_range("fib").unwrap();
        assert_eq!(range, (0, 12));
    }

    #[test]
    fn function_range_unknown_returns_none() {
        let r = make_reader();
        assert!(r.function_range("ghost").is_none());
    }

    #[test]
    fn function_names_includes_registered() {
        let r = make_reader();
        let names = r.function_names();
        assert!(names.contains(&"fib"));
    }
}
