//! [`DebugSidecarWriter`] — accumulates source-location data during compilation.
//!
//! The compiler calls this once per emitted `IIRInstr` to record the mapping
//! from instruction index to source location.  After compilation, [`finish`]
//! serialises the accumulated data to [`bytes::Bytes`] — actually a `Vec<u8>`
//! — that the reader can load.
//!
//! # Design
//!
//! The on-disk format is JSON UTF-8.  This is intentionally simpler than a
//! compact binary format and lets the entire pipeline work end-to-end before
//! investing in a more efficient encoding.  The [`crate::DebugSidecarReader`]
//! boundary is the only place that knows the format, so upgrading later is a
//! single-file change.
//!
//! The writer is append-only and **not** `Send` / `Sync` — each compilation
//! uses its own instance.
//!
//! # Example
//!
//! ```
//! use debug_sidecar::DebugSidecarWriter;
//!
//! let mut w = DebugSidecarWriter::new();
//! let file_id = w.add_source_file("fibonacci.tetrad", b"");
//!
//! w.begin_function("fibonacci", 0, 1);
//! w.declare_variable("fibonacci", 0, "n", "any", 0, 12);
//! w.record("fibonacci", 0, file_id, 3, 5);
//! w.end_function("fibonacci", 12);
//!
//! let sidecar: Vec<u8> = w.finish();
//! assert!(!sidecar.is_empty());
//! ```

use std::collections::HashMap;

use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Internal representation
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct LineRow {
    instr_index: usize,
    file_id: usize,
    line: u32,
    col: u32,
}

#[derive(Debug)]
struct FunctionEntry {
    start_instr: usize,
    end_instr: Option<usize>,
    param_count: usize,
}

#[derive(Debug)]
struct VariableEntry {
    reg_index: u32,
    name: String,
    type_hint: String,
    live_start: usize,
    live_end: usize,
}

// ---------------------------------------------------------------------------
// DebugSidecarWriter
// ---------------------------------------------------------------------------

/// Accumulates debug sidecar data during a single compilation.
///
/// Create one instance per compilation unit (module / source file), call the
/// `record*` / `declare_*` / `begin_*` / `end_*` methods as instructions are
/// emitted, then call [`finish`][`Self::finish`] to serialise.
#[derive(Debug)]
pub struct DebugSidecarWriter {
    source_files: Vec<(String, String)>, // (path, checksum_hex)
    source_file_index: HashMap<String, usize>,
    line_table: HashMap<String, Vec<LineRow>>,
    functions: HashMap<String, FunctionEntry>,
    variables: HashMap<String, Vec<VariableEntry>>,
}

impl DebugSidecarWriter {
    /// Create a new, empty writer.
    pub fn new() -> Self {
        Self {
            source_files: Vec::new(),
            source_file_index: HashMap::new(),
            line_table: HashMap::new(),
            functions: HashMap::new(),
            variables: HashMap::new(),
        }
    }

    // ------------------------------------------------------------------
    // Source files
    // ------------------------------------------------------------------

    /// Register a source file and return its `file_id`.
    ///
    /// Calling this multiple times with the same `path` is safe — subsequent
    /// calls return the same `file_id` without duplicating the entry.
    ///
    /// # Parameters
    ///
    /// - `path` — source file path (absolute or relative to the build directory).
    /// - `checksum` — optional SHA-256 bytes of the source at compile time.
    ///   Pass `b""` to omit.
    ///
    /// # Returns
    ///
    /// 0-based `file_id` for use in [`record`][`Self::record`].
    pub fn add_source_file(&mut self, path: &str, checksum: &[u8]) -> usize {
        if let Some(&id) = self.source_file_index.get(path) {
            return id;
        }
        let id = self.source_files.len();
        let checksum_hex: String = checksum.iter().map(|b| format!("{b:02x}")).collect();
        self.source_files.push((path.to_string(), checksum_hex));
        self.source_file_index.insert(path.to_string(), id);
        id
    }

    // ------------------------------------------------------------------
    // Line table
    // ------------------------------------------------------------------

    /// Record the source location of one emitted instruction.
    ///
    /// May be called out of order by `instr_index` — the reader sorts rows
    /// by index at load time.
    ///
    /// # Parameters
    ///
    /// - `fn_name` — function containing this instruction.
    /// - `instr_index` — 0-based index within the function body.
    /// - `file_id` — returned by [`add_source_file`][`Self::add_source_file`].
    /// - `line` — 1-based source line.
    /// - `col` — 1-based source column.
    pub fn record(
        &mut self,
        fn_name: &str,
        instr_index: usize,
        file_id: usize,
        line: u32,
        col: u32,
    ) {
        self.line_table
            .entry(fn_name.to_string())
            .or_default()
            .push(LineRow { instr_index, file_id, line, col });
    }

    // ------------------------------------------------------------------
    // Functions
    // ------------------------------------------------------------------

    /// Register the start of a function's instruction range.
    ///
    /// Must be paired with [`end_function`][`Self::end_function`].
    ///
    /// # Parameters
    ///
    /// - `fn_name` — function name (must match the `IIRFunction.name`).
    /// - `start_instr` — index of the first instruction in the function body.
    /// - `param_count` — number of parameters (for stack frame display).
    pub fn begin_function(&mut self, fn_name: &str, start_instr: usize, param_count: usize) {
        self.functions.insert(fn_name.to_string(), FunctionEntry {
            start_instr,
            end_instr: None,
            param_count,
        });
    }

    /// Record the end of a function's instruction range.
    ///
    /// # Parameters
    ///
    /// - `fn_name` — must match a prior [`begin_function`][`Self::begin_function`] call.
    /// - `end_instr` — one-past-last instruction index (exclusive upper bound).
    pub fn end_function(&mut self, fn_name: &str, end_instr: usize) {
        if let Some(entry) = self.functions.get_mut(fn_name) {
            entry.end_instr = Some(end_instr);
        }
    }

    // ------------------------------------------------------------------
    // Variables
    // ------------------------------------------------------------------

    /// Record a named variable binding for a register.
    ///
    /// # Parameters
    ///
    /// - `fn_name` — function containing this variable.
    /// - `reg_index` — IIR register index.
    /// - `name` — human-readable variable name.
    /// - `type_hint` — declared type (`"any"`, `"u8"`, …), or `""` for none.
    /// - `live_start` — first instruction index at which this binding is valid.
    /// - `live_end` — one-past-last instruction index (exclusive).
    #[allow(clippy::too_many_arguments)]
    pub fn declare_variable(
        &mut self,
        fn_name: &str,
        reg_index: u32,
        name: &str,
        type_hint: &str,
        live_start: usize,
        live_end: usize,
    ) {
        self.variables
            .entry(fn_name.to_string())
            .or_default()
            .push(VariableEntry {
                reg_index,
                name: name.to_string(),
                type_hint: type_hint.to_string(),
                live_start,
                live_end,
            });
    }

    // ------------------------------------------------------------------
    // Serialisation
    // ------------------------------------------------------------------

    /// Serialise all accumulated data to JSON UTF-8 bytes.
    ///
    /// The result is an opaque `Vec<u8>` that can be written to disk or passed
    /// directly to [`DebugSidecarReader::new`][`crate::DebugSidecarReader::new`].
    ///
    /// The format is:
    ///
    /// ```json
    /// {
    ///   "version": 1,
    ///   "source_files": [{"path": "...", "checksum": "..."}],
    ///   "line_table": {"fn": [{"instr_index": N, "file_id": F, "line": L, "col": C}]},
    ///   "functions": {"fn": {"start_instr": S, "end_instr": E, "param_count": P}},
    ///   "variables": {"fn": [{"reg_index": R, "name": "...", "type_hint": "...", ...}]}
    /// }
    /// ```
    pub fn finish(&self) -> Vec<u8> {
        // Build JSON for source_files.
        let sf_json: Vec<Value> = self.source_files.iter().map(|(path, checksum)| {
            json!({"path": path, "checksum": checksum})
        }).collect();

        // Build JSON for line_table — sort each function's rows by instr_index.
        let mut lt_json = serde_json::Map::new();
        for (fn_name, rows) in &self.line_table {
            let mut sorted: Vec<&LineRow> = rows.iter().collect();
            sorted.sort_by_key(|r| r.instr_index);
            let rows_json: Vec<Value> = sorted.iter().map(|r| {
                json!({
                    "instr_index": r.instr_index,
                    "file_id": r.file_id,
                    "line": r.line,
                    "col": r.col,
                })
            }).collect();
            lt_json.insert(fn_name.clone(), Value::Array(rows_json));
        }

        // Build JSON for functions.
        let mut fn_json = serde_json::Map::new();
        for (fn_name, entry) in &self.functions {
            fn_json.insert(fn_name.clone(), json!({
                "start_instr": entry.start_instr,
                "end_instr": entry.end_instr,
                "param_count": entry.param_count,
            }));
        }

        // Build JSON for variables.
        let mut var_json = serde_json::Map::new();
        for (fn_name, vars) in &self.variables {
            let arr: Vec<Value> = vars.iter().map(|v| {
                json!({
                    "reg_index": v.reg_index,
                    "name": v.name,
                    "type_hint": v.type_hint,
                    "live_start": v.live_start,
                    "live_end": v.live_end,
                })
            }).collect();
            var_json.insert(fn_name.clone(), Value::Array(arr));
        }

        let payload = json!({
            "version": 1,
            "source_files": sf_json,
            "line_table": Value::Object(lt_json),
            "functions": Value::Object(fn_json),
            "variables": Value::Object(var_json),
        });

        serde_json::to_vec(&payload).expect("sidecar JSON serialisation is infallible")
    }
}

impl Default for DebugSidecarWriter {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_source_file_deduplicates() {
        let mut w = DebugSidecarWriter::new();
        let id0 = w.add_source_file("fib.tetrad", b"");
        let id1 = w.add_source_file("fib.tetrad", b"");
        assert_eq!(id0, id1);
        assert_eq!(w.source_files.len(), 1);
    }

    #[test]
    fn add_two_different_files() {
        let mut w = DebugSidecarWriter::new();
        let id0 = w.add_source_file("a.t", b"");
        let id1 = w.add_source_file("b.t", b"");
        assert_ne!(id0, id1);
        assert_eq!(w.source_files.len(), 2);
    }

    #[test]
    fn record_stores_row() {
        let mut w = DebugSidecarWriter::new();
        w.record("fib", 3, 0, 10, 5);
        assert_eq!(w.line_table["fib"].len(), 1);
        assert_eq!(w.line_table["fib"][0].line, 10);
    }

    #[test]
    fn begin_end_function() {
        let mut w = DebugSidecarWriter::new();
        w.begin_function("fib", 0, 2);
        w.end_function("fib", 15);
        let entry = &w.functions["fib"];
        assert_eq!(entry.start_instr, 0);
        assert_eq!(entry.end_instr, Some(15));
        assert_eq!(entry.param_count, 2);
    }

    #[test]
    fn end_function_with_no_begin_is_noop() {
        let mut w = DebugSidecarWriter::new();
        w.end_function("ghost", 5);
        assert!(!w.functions.contains_key("ghost"));
    }

    #[test]
    fn declare_variable_stored() {
        let mut w = DebugSidecarWriter::new();
        w.declare_variable("fib", 0, "n", "u8", 0, 12);
        assert_eq!(w.variables["fib"].len(), 1);
        assert_eq!(w.variables["fib"][0].name, "n");
    }

    #[test]
    fn finish_produces_non_empty_bytes() {
        let mut w = DebugSidecarWriter::new();
        w.add_source_file("f.t", b"");
        w.begin_function("main", 0, 0);
        w.end_function("main", 3);
        let bytes = w.finish();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn finish_valid_json_with_version_1() {
        let w = DebugSidecarWriter::new();
        let bytes = w.finish();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["version"], 1);
    }

    #[test]
    fn finish_line_table_sorted() {
        let mut w = DebugSidecarWriter::new();
        w.add_source_file("f.t", b"");
        // Insert out of order.
        w.record("fib", 5, 0, 10, 1);
        w.record("fib", 2, 0, 8, 1);
        let bytes = w.finish();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let rows = parsed["line_table"]["fib"].as_array().unwrap();
        let idx0 = rows[0]["instr_index"].as_u64().unwrap();
        let idx1 = rows[1]["instr_index"].as_u64().unwrap();
        assert!(idx0 < idx1, "rows must be sorted by instr_index");
    }

    #[test]
    fn checksum_stored_as_hex() {
        let mut w = DebugSidecarWriter::new();
        w.add_source_file("f.t", &[0xDE, 0xAD]);
        let bytes = w.finish();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let cs = parsed["source_files"][0]["checksum"].as_str().unwrap();
        assert_eq!(cs, "dead");
    }
}
