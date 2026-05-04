//! [`SidecarIndex`] — fast offset ↔ source-location lookups.
//!
//! ## Implementation plan (LS03 PR A)
//!
//! Wraps the `debug-sidecar` query API with a caching layer.
//!
//! PREREQUISITE: verify debug-sidecar's Rust public API before implementing.
//! File: code/packages/rust/debug-sidecar/src/lib.rs
//!       OR code/packages/python/debug-sidecar/src/debug_sidecar/ (Python version)
//!
//! The sidecar maps: instruction_offset (u64) ↔ (source_file, line, column).
//! Spec reference: 05d §"Query API".
//!
//! If the Rust debug-sidecar doesn't exist yet (only Python):
//! - Either port the query reader to Rust (preferred; add it to the existing crate).
//! - Or shell out to a Python helper (temporary workaround).
//!
//! ### from_bytes(sidecar_bytes: &[u8]) → Result<SidecarIndex, String>
//!
//! Parse the sidecar binary format (spec 05d) into an in-memory index.
//! Build two HashMaps for O(1) bidirectional lookup:
//!   offset_to_source: HashMap<u64, (PathBuf, u32, u32)>
//!   source_to_offsets: HashMap<(PathBuf, u32), Vec<u64>>

use std::path::{Path, PathBuf};

/// Bidirectional index over a compiled debug sidecar.
///
/// ## TODO — implement (LS03 PR A)
pub struct SidecarIndex {
    // TODO: offset_to_source: HashMap<u64, (PathBuf, u32, u32)>
    //       source_to_offsets: HashMap<(PathBuf, u32), Vec<u64>>
}

impl SidecarIndex {
    /// Build a sidecar index from raw sidecar bytes.
    ///
    /// ## TODO — implement (LS03 PR A)
    pub fn from_bytes(_bytes: &[u8]) -> Result<Self, String> {
        Err("SidecarIndex::from_bytes: not yet implemented (LS03 PR A)".into())
    }

    /// Resolve an instruction offset to (source_file, line, column).
    ///
    /// Returns None if the offset is not in the sidecar.
    ///
    /// ## TODO — implement (LS03 PR A)
    pub fn offset_to_source(&self, _offset: u64) -> Option<(PathBuf, u32, u32)> {
        None
    }

    /// Resolve a source line to the set of instruction offsets on that line.
    ///
    /// Returns an empty vec if no offsets map to this line.
    ///
    /// ## TODO — implement (LS03 PR A)
    pub fn source_to_offsets(&self, _file: &Path, _line: u32) -> Vec<u64> {
        vec![]
    }
}
