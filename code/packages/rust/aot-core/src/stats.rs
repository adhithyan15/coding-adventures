//! `AOTStats` — compilation statistics for a single `AOTCore` session.
//!
//! `AOTStats` is a value-type struct that accumulates counters across all
//! `compile()` calls on a single [`AOTCore`](crate::core::AOTCore) instance.
//! Callers retrieve a snapshot with `AOTCore::stats()`.
//!
//! # Fields
//!
//! | Field | Meaning |
//! |---|---|
//! | `functions_compiled` | How many functions were compiled to native binary |
//! | `functions_untyped` | How many fell back to the IIR table (type remained `"any"`) |
//! | `compilation_time_ns` | Wall-clock nanoseconds spent inside `compile()` |
//! | `total_binary_size` | Bytes emitted by the backend across all compiled functions |
//! | `optimization_level` | Optimization level passed at construction time |
//!
//! # Example
//!
//! ```
//! use aot_core::stats::AOTStats;
//!
//! let mut s = AOTStats::new(2);
//! s.functions_compiled = 3;
//! s.functions_untyped  = 1;
//! s.total_binary_size  = 1024;
//!
//! assert_eq!(s.total_functions(), 4);
//! assert!(s.typed_ratio() > 0.5);
//! ```

// ---------------------------------------------------------------------------
// AOTStats
// ---------------------------------------------------------------------------

/// Compilation statistics accumulated across one `AOTCore` session.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AOTStats {
    /// Number of functions successfully compiled to native binary.
    pub functions_compiled: usize,

    /// Number of functions that could not be typed and were emitted into the
    /// IIR table instead.
    pub functions_untyped: usize,

    /// Wall-clock nanoseconds spent in compilation across all `compile()` calls.
    pub compilation_time_ns: u64,

    /// Total bytes emitted by the backend across all compiled functions.
    pub total_binary_size: usize,

    /// Optimization level (`0` = none, `1` = fold+DCE, `2` = fold+DCE+AOT passes).
    pub optimization_level: u8,
}

impl AOTStats {
    /// Create a fresh `AOTStats` with all counters zeroed.
    ///
    /// `optimization_level` is stored verbatim — it never changes during a
    /// session and is recorded here for display / diagnostics.
    pub fn new(optimization_level: u8) -> Self {
        AOTStats {
            optimization_level,
            ..AOTStats::default()
        }
    }

    /// Total functions encountered by the compiler (`compiled + untyped`).
    pub fn total_functions(&self) -> usize {
        self.functions_compiled + self.functions_untyped
    }

    /// Fraction of functions that were fully compiled (0.0 – 1.0).
    ///
    /// Returns `1.0` when no functions have been seen (vacuously perfect).
    pub fn typed_ratio(&self) -> f64 {
        let total = self.total_functions();
        if total == 0 {
            1.0
        } else {
            self.functions_compiled as f64 / total as f64
        }
    }

    /// Compilation throughput: bytes per nanosecond (0.0 when no time elapsed).
    pub fn bytes_per_ns(&self) -> f64 {
        if self.compilation_time_ns == 0 {
            0.0
        } else {
            self.total_binary_size as f64 / self.compilation_time_ns as f64
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_zeroed() {
        let s = AOTStats::new(2);
        assert_eq!(s.functions_compiled, 0);
        assert_eq!(s.functions_untyped, 0);
        assert_eq!(s.compilation_time_ns, 0);
        assert_eq!(s.total_binary_size, 0);
        assert_eq!(s.optimization_level, 2);
    }

    #[test]
    fn total_functions() {
        let mut s = AOTStats::new(1);
        s.functions_compiled = 3;
        s.functions_untyped = 2;
        assert_eq!(s.total_functions(), 5);
    }

    #[test]
    fn typed_ratio_all_compiled() {
        let mut s = AOTStats::new(0);
        s.functions_compiled = 10;
        assert!((s.typed_ratio() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn typed_ratio_none_compiled() {
        let mut s = AOTStats::new(0);
        s.functions_untyped = 5;
        assert!((s.typed_ratio()).abs() < 1e-9);
    }

    #[test]
    fn typed_ratio_empty() {
        let s = AOTStats::new(0);
        assert!((s.typed_ratio() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn bytes_per_ns_zero_time() {
        let mut s = AOTStats::new(0);
        s.total_binary_size = 1000;
        assert!((s.bytes_per_ns()).abs() < 1e-9);
    }

    #[test]
    fn bytes_per_ns_positive() {
        let mut s = AOTStats::new(0);
        s.total_binary_size = 1000;
        s.compilation_time_ns = 1000;
        assert!((s.bytes_per_ns() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn clone_eq() {
        let mut s = AOTStats::new(2);
        s.functions_compiled = 5;
        assert_eq!(s.clone(), s);
    }
}
