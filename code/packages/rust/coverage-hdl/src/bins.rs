//! Bin definitions: the basic building block of functional coverage.
//!
//! ## What is a bin?
//!
//! A *bin* is a named predicate on a signal value.  When a signal is sampled
//! and falls into a bin, that bin's hit count increments.  Coverage = fraction
//! of bins hit at least once.
//!
//! ## Bin kinds (truth table)
//!
//! | Constructor       | Matches when...             | Example |
//! |-------------------|-----------------------------|---------|
//! | `bin_value(n, v)` | `signal == v`               | `bin_value("max", 255)` |
//! | `bin_range(n, lo, hi)` | `lo ≤ signal ≤ hi`    | `bin_range("mid", 64, 192)` |
//! | `bin_default()`   | always (catch-all)          | catches anything not in other bins |
//!
//! ## First-match-wins
//!
//! When a coverpoint has multiple bins, only the *first* bin whose predicate
//! matches is incremented.  This lets you model mutually exclusive cases
//! without worrying about double-counting.

use std::sync::Arc;

/// A named value-predicate.
///
/// The `matcher` is stored as an `Arc<dyn Fn>` so `Bin` can be cloned freely
/// (useful when the same bin spec is shared across coverpoints or cross tables)
/// without re-allocating the predicate.
#[derive(Clone)]
pub struct Bin {
    /// Unique name within its coverpoint.
    pub name: String,
    /// Returns `true` when a sampled value should be counted in this bin.
    pub matcher: Arc<dyn Fn(i64) -> bool + Send + Sync>,
}

impl std::fmt::Debug for Bin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Bin({:?})", self.name)
    }
}

/// Bin matching exactly one integer value.
///
/// ```rust
/// use coverage_hdl::bin_value;
/// let b = bin_value("high_byte", 255);
/// assert!((b.matcher)(255));
/// assert!(!(b.matcher)(0));
/// ```
pub fn bin_value(name: impl Into<String>, value: i64) -> Bin {
    Bin {
        name: name.into(),
        matcher: Arc::new(move |v| v == value),
    }
}

/// Bin matching any value in the inclusive range `[min, max]`.
///
/// ```rust
/// use coverage_hdl::bin_range;
/// let b = bin_range("mid", 64, 192);
/// assert!((b.matcher)(64));
/// assert!((b.matcher)(128));
/// assert!((b.matcher)(192));
/// assert!(!(b.matcher)(63));
/// assert!(!(b.matcher)(193));
/// ```
pub fn bin_range(name: impl Into<String>, min: i64, max: i64) -> Bin {
    Bin {
        name: name.into(),
        matcher: Arc::new(move |v| v >= min && v <= max),
    }
}

/// Catch-all bin: matches every value.
///
/// Place this last in a coverpoint's bin list to capture "everything else."
///
/// ```rust
/// use coverage_hdl::bin_default;
/// let b = bin_default();
/// assert!((b.matcher)(0));
/// assert!((b.matcher)(i64::MAX));
/// assert!((b.matcher)(i64::MIN));
/// ```
pub fn bin_default() -> Bin {
    Bin {
        name: "default".into(),
        matcher: Arc::new(|_| true),
    }
}
