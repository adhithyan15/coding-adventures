//! Binary linking — concatenate compiled function binaries and record offsets.
//!
//! After all functions have been compiled to native binary blobs by the
//! backend, they must be **linked** into a single flat code section before
//! being written to the `.aot` snapshot.
//!
//! # Design
//!
//! The linker is intentionally simple: it concatenates the per-function binary
//! blobs in the order they are supplied and records the byte offset at which
//! each function's code begins.  At load time, the runtime uses these offsets
//! to jump to the correct function.
//!
//! This mirrors the Python `link.link()` function exactly.
//!
//! # Output format
//!
//! ```text
//! ┌────────────────────────┬───────────────────┬─────────────┐
//! │  fn "main" binary (N)  │  fn "helper" (M)  │  …          │
//! └────────────────────────┴───────────────────┴─────────────┘
//!  offset 0                 offset N             offset N+M
//! ```
//!
//! # Example
//!
//! ```
//! use aot_core::link::{link, entry_point_offset};
//!
//! let binaries = vec![
//!     ("main".to_string(),   vec![0xDE, 0xAD]),
//!     ("helper".to_string(), vec![0xBE, 0xEF, 0x00]),
//! ];
//! let (code, offsets) = link(&binaries);
//! assert_eq!(code, vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00]);
//! assert_eq!(offsets["main"],   0);
//! assert_eq!(offsets["helper"], 2);
//!
//! // Entry point defaults to "main".
//! assert_eq!(entry_point_offset(&offsets, None), 0);
//! ```

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Concatenate function binaries into a single code section.
///
/// # Parameters
///
/// - `fn_binaries` — ordered list of `(name, binary_bytes)` pairs.  Order
///   determines layout in the output.
///
/// # Returns
///
/// A tuple of:
/// - `Vec<u8>` — the concatenated code section.
/// - `HashMap<String, usize>` — function name → byte offset in the code section.
///
/// # Empty input
///
/// If `fn_binaries` is empty, both the code vector and the offset map are empty.
pub fn link(fn_binaries: &[(String, Vec<u8>)]) -> (Vec<u8>, HashMap<String, usize>) {
    let mut code: Vec<u8> = Vec::new();
    let mut offsets: HashMap<String, usize> = HashMap::new();

    for (name, binary) in fn_binaries {
        let offset = code.len();
        offsets.insert(name.clone(), offset);
        code.extend_from_slice(binary);
    }

    (code, offsets)
}

/// Return the byte offset of the entry-point function in the linked code.
///
/// # Parameters
///
/// - `offsets` — the offset map returned by [`link`].
/// - `entry` — the name of the entry-point function.  Defaults to `"main"`
///   when `None`.
///
/// # Returns
///
/// The byte offset, or `0` if the entry-point function is not in the map.
/// Returning `0` when the function is absent is safe because the snapshot
/// header is still written with `entry_point_offset = 0`; the runtime handles
/// a missing entry gracefully.
pub fn entry_point_offset(offsets: &HashMap<String, usize>, entry: Option<&str>) -> usize {
    let name = entry.unwrap_or("main");
    *offsets.get(name).unwrap_or(&0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input() {
        let (code, offsets) = link(&[]);
        assert!(code.is_empty());
        assert!(offsets.is_empty());
    }

    #[test]
    fn single_function() {
        let bins = vec![("main".to_string(), vec![1u8, 2, 3])];
        let (code, offsets) = link(&bins);
        assert_eq!(code, vec![1, 2, 3]);
        assert_eq!(offsets["main"], 0);
    }

    #[test]
    fn two_functions_contiguous() {
        let bins = vec![
            ("main".to_string(),   vec![0xAA, 0xBB]),
            ("helper".to_string(), vec![0xCC, 0xDD, 0xEE]),
        ];
        let (code, offsets) = link(&bins);
        assert_eq!(code, vec![0xAA, 0xBB, 0xCC, 0xDD, 0xEE]);
        assert_eq!(offsets["main"],   0);
        assert_eq!(offsets["helper"], 2);
    }

    #[test]
    fn three_functions_offsets_cumulative() {
        let bins = vec![
            ("a".to_string(), vec![1, 2]),
            ("b".to_string(), vec![3, 4, 5]),
            ("c".to_string(), vec![6]),
        ];
        let (code, offsets) = link(&bins);
        assert_eq!(code.len(), 6);
        assert_eq!(offsets["a"], 0);
        assert_eq!(offsets["b"], 2);
        assert_eq!(offsets["c"], 5);
    }

    #[test]
    fn entry_point_default_main() {
        let bins = vec![
            ("setup".to_string(), vec![0xAA]),
            ("main".to_string(),  vec![0xBB]),
        ];
        let (_, offsets) = link(&bins);
        let ep = entry_point_offset(&offsets, None);
        assert_eq!(ep, offsets["main"]);
    }

    #[test]
    fn entry_point_custom_name() {
        let bins = vec![
            ("setup".to_string(), vec![0x01, 0x02, 0x03]),
            ("start".to_string(), vec![0x04]),
        ];
        let (_, offsets) = link(&bins);
        let ep = entry_point_offset(&offsets, Some("start"));
        assert_eq!(ep, 3);
    }

    #[test]
    fn entry_point_missing_returns_zero() {
        let offsets: HashMap<String, usize> = HashMap::new();
        assert_eq!(entry_point_offset(&offsets, Some("main")), 0);
    }

    #[test]
    fn empty_binaries_skipped() {
        // A function with zero bytes still gets an offset entry.
        let bins = vec![
            ("noop".to_string(), vec![]),
            ("real".to_string(), vec![0xFF]),
        ];
        let (code, offsets) = link(&bins);
        assert_eq!(code.len(), 1);
        assert_eq!(offsets["noop"], 0);
        assert_eq!(offsets["real"], 0);
    }
}
