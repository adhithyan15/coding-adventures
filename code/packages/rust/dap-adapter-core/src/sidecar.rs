//! [`SidecarIndex`] — `(file, line) ↔ VmLocation` lookups.
//!
//! Wraps [`debug_sidecar::DebugSidecarReader`] with two added conveniences:
//!
//! 1. **Bidirectional lookup that preserves all matches.**  The underlying
//!    `find_instr` returns just the lowest matching instruction, but a single
//!    source line can map to multiple instructions across multiple functions
//!    (especially in inlined or unrolled code).  We need *all* of them in
//!    order to install a breakpoint at every reachable VM location for the
//!    line.
//!
//! 2. **Forward lookup keyed on a [`VmLocation`].**  This is the natural
//!    shape for the DAP layer (call-stack frames already carry `(fn, idx)`).
//!
//! ## Algorithm
//!
//! Construction is O(N) where N is the total number of `LineRow`s — we walk
//! every function's row list once and populate two HashMaps:
//!
//! ```text
//!   line_to_locs:  (file, line) → Vec<VmLocation>
//!   loc_to_source: (fn, idx)    → SourceLocation
//! ```
//!
//! Both queries are then O(1) average.

use std::collections::HashMap;

use debug_sidecar::{DebugSidecarReader, SourceLocation};

use crate::vm_conn::VmLocation;

/// Forward + reverse index over a compiled debug sidecar.
#[derive(Debug)]
pub struct SidecarIndex {
    reader: DebugSidecarReader,
    /// `(file, line) → all VM locations on that line`.
    line_to_locs: HashMap<(String, u32), Vec<VmLocation>>,
    /// `(fn, idx) → source location` (memoised for O(1) lookup).
    loc_to_source: HashMap<VmLocation, SourceLocation>,
}

impl SidecarIndex {
    /// Build an index from raw sidecar bytes (the bytes produced by
    /// `DebugSidecarWriter::finish` in the compiler).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let reader = DebugSidecarReader::new(bytes)
            .map_err(|e| format!("sidecar parse: {e}"))?;
        let mut idx = SidecarIndex {
            reader,
            line_to_locs:  HashMap::new(),
            loc_to_source: HashMap::new(),
        };
        idx.build();
        Ok(idx)
    }

    /// Internal: populate the two HashMaps from the underlying reader.
    fn build(&mut self) {
        let fns: Vec<String> = self.reader.function_names()
            .into_iter().map(|s| s.to_string()).collect();
        let files: Vec<String> = self.reader.source_files().to_vec();

        for fn_name in &fns {
            for row in self.reader.raw_line_rows(fn_name) {
                let file = match files.get(row.file_id) {
                    Some(f) => f.clone(),
                    None    => continue,
                };
                let loc = VmLocation::new(fn_name.clone(), row.instr_index);
                let src = SourceLocation {
                    file: file.clone(),
                    line: row.line,
                    col:  row.col,
                };
                self.line_to_locs
                    .entry((file, row.line))
                    .or_default()
                    .push(loc.clone());
                self.loc_to_source.insert(loc, src);
            }
        }
    }

    /// Resolve a VM location to its source position.
    ///
    /// Falls back to the underlying reader's DWARF-style "last row ≤ idx"
    /// lookup if there is no exact-match row in our HashMap — this covers
    /// instructions that the compiler didn't emit an explicit row for.
    pub fn loc_to_source(&self, loc: &VmLocation) -> Option<SourceLocation> {
        if let Some(src) = self.loc_to_source.get(loc) {
            return Some(src.clone());
        }
        self.reader.lookup(&loc.function, loc.instr_index)
    }

    /// Return every VM location reachable on `(file, line)`.
    ///
    /// Used by `setBreakpoints`: the adapter installs a VM breakpoint at
    /// each returned location so any path through the line will trip.
    pub fn source_to_locs(&self, file: &str, line: u32) -> Vec<VmLocation> {
        self.line_to_locs
            .get(&(file.to_string(), line))
            .cloned()
            .unwrap_or_default()
    }

    /// Convenience: list every source file referenced by the sidecar.
    pub fn source_files(&self) -> Vec<String> {
        self.reader.source_files().to_vec()
    }

    /// Borrow the wrapped reader (for live-variable queries etc.).
    pub fn reader(&self) -> &DebugSidecarReader {
        &self.reader
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use debug_sidecar::DebugSidecarWriter;

    /// Build a small sidecar fixture for testing.
    ///
    /// Layout:
    /// - file `prog.tw` with 4 lines
    /// - function `main`:
    ///     instr 0 → line 1
    ///     instr 1 → line 2
    ///     instr 2 → line 2 (second instruction on same line)
    ///     instr 3 → line 4
    /// - function `helper`:
    ///     instr 0 → line 3
    ///     instr 1 → line 4 (cross-function shared line)
    fn fixture_bytes() -> Vec<u8> {
        let mut w = DebugSidecarWriter::new();
        let fid = w.add_source_file("prog.tw", b"");

        w.begin_function("main", 0, 1);
        w.record("main", 0, fid, 1, 1);
        w.record("main", 1, fid, 2, 1);
        w.record("main", 2, fid, 2, 5);
        w.record("main", 3, fid, 4, 1);
        w.end_function("main", 4);

        w.begin_function("helper", 0, 0);
        w.record("helper", 0, fid, 3, 1);
        w.record("helper", 1, fid, 4, 1);
        w.end_function("helper", 2);

        w.finish()
    }

    #[test]
    fn build_from_bytes_succeeds() {
        let bytes = fixture_bytes();
        let _ = SidecarIndex::from_bytes(&bytes).expect("ok");
    }

    #[test]
    fn from_bytes_rejects_garbage() {
        let err = SidecarIndex::from_bytes(b"not json").unwrap_err();
        assert!(err.contains("sidecar parse"));
    }

    #[test]
    fn loc_to_source_resolves_main() {
        let idx = SidecarIndex::from_bytes(&fixture_bytes()).unwrap();
        let src = idx.loc_to_source(&VmLocation::new("main", 0)).unwrap();
        assert_eq!(src.line, 1);
        assert_eq!(src.file, "prog.tw");
    }

    #[test]
    fn loc_to_source_resolves_helper() {
        let idx = SidecarIndex::from_bytes(&fixture_bytes()).unwrap();
        let src = idx.loc_to_source(&VmLocation::new("helper", 0)).unwrap();
        assert_eq!(src.line, 3);
    }

    #[test]
    fn loc_to_source_unknown_function_returns_none() {
        let idx = SidecarIndex::from_bytes(&fixture_bytes()).unwrap();
        assert!(idx.loc_to_source(&VmLocation::new("nope", 0)).is_none());
    }

    #[test]
    fn source_to_locs_finds_all_on_line() {
        let idx = SidecarIndex::from_bytes(&fixture_bytes()).unwrap();
        // Line 2 has two instructions in `main`.
        let mut locs = idx.source_to_locs("prog.tw", 2);
        locs.sort_by_key(|l| l.instr_index);
        assert_eq!(locs, vec![
            VmLocation::new("main", 1),
            VmLocation::new("main", 2),
        ]);
    }

    #[test]
    fn source_to_locs_finds_cross_function_matches() {
        let idx = SidecarIndex::from_bytes(&fixture_bytes()).unwrap();
        // Line 4 has instructions in BOTH `main` and `helper`.
        let mut locs = idx.source_to_locs("prog.tw", 4);
        locs.sort_by(|a, b| a.function.cmp(&b.function)
            .then(a.instr_index.cmp(&b.instr_index)));
        assert_eq!(locs, vec![
            VmLocation::new("helper", 1),
            VmLocation::new("main",   3),
        ]);
    }

    #[test]
    fn source_to_locs_unknown_returns_empty() {
        let idx = SidecarIndex::from_bytes(&fixture_bytes()).unwrap();
        assert!(idx.source_to_locs("prog.tw", 99).is_empty());
        assert!(idx.source_to_locs("missing.tw", 1).is_empty());
    }

    #[test]
    fn source_files_lists_registered() {
        let idx = SidecarIndex::from_bytes(&fixture_bytes()).unwrap();
        assert_eq!(idx.source_files(), vec!["prog.tw"]);
    }

    #[test]
    fn reader_accessor_works() {
        let idx = SidecarIndex::from_bytes(&fixture_bytes()).unwrap();
        // Smoke: the underlying reader can resolve via its own API too.
        let names = idx.reader().function_names();
        assert!(names.contains(&"main"));
        assert!(names.contains(&"helper"));
    }
}
