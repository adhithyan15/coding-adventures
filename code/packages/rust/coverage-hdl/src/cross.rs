//! `CrossPoint` — cross-product coverage across two or more coverpoints.
//!
//! ## What is cross coverage?
//!
//! Coverpoints measure one signal at a time.  Cross coverage asks whether
//! *every combination* of bins from two (or more) signals was observed.
//!
//! Example: coverpoint `cin` has bins `{zero, one}` and coverpoint `a`
//! has bins `{low, high}`.  The cross has 2 × 2 = 4 required combinations:
//!
//! ```text
//! (cin=zero, a=low)   — seen?
//! (cin=zero, a=high)  — seen?
//! (cin=one,  a=low)   — seen?
//! (cin=one,  a=high)  — seen?
//! ```
//!
//! ## How sampling works
//!
//! The `CrossPoint` does *not* subscribe to events itself.  Instead, the
//! `CoverageRecorder` keeps a per-signal `last_values` table and calls
//! `cross.sample()` when the user explicitly requests it (e.g., at a clock
//! edge).  `sample()` reads the last-seen bin for each constituent coverpoint,
//! and if all signals matched a bin, it increments the corresponding
//! `(bin_a, bin_b, ...)` tuple.

use std::collections::HashMap;

use crate::coverpoint::Coverpoint;

/// Records hit counts for every (bin-combination) tuple.
#[derive(Debug, Clone)]
pub struct CrossPoint {
    /// Human-readable label.
    pub name: String,
    /// The constituent coverpoints (in dimension order).
    pub coverpoints: Vec<Coverpoint>,
    /// Hit counts keyed by `Vec<bin_name>` (one name per coverpoint).
    pub hits: HashMap<Vec<String>, u64>,
    /// Last seen value per signal (updated externally by the recorder).
    pub last_values: HashMap<String, i64>,
}

impl CrossPoint {
    /// Create a cross.
    pub fn new(name: impl Into<String>, coverpoints: Vec<Coverpoint>) -> Self {
        CrossPoint {
            name: name.into(),
            coverpoints,
            hits: HashMap::new(),
            last_values: HashMap::new(),
        }
    }

    /// Record one sample using the current `last_values`.
    ///
    /// If any constituent signal has no last-seen value, or if its value
    /// matches no bin, the sample is silently dropped.
    pub fn sample(&mut self) {
        let mut bins_hit: Vec<String> = Vec::with_capacity(self.coverpoints.len());
        for cp in &self.coverpoints {
            let Some(&v) = self.last_values.get(&cp.signal) else { return; };
            let Some(bin_name) = cp.bins.iter().find(|b| (b.matcher)(v)).map(|b| b.name.clone())
                else { return; };
            bins_hit.push(bin_name);
        }
        *self.hits.entry(bins_hit).or_insert(0) += 1;
    }

    /// Fraction of expected (bin-combination) tuples that have been seen.
    ///
    /// Expected combinations = product of bin-counts across all coverpoints.
    /// Returns 1.0 when there are no constituent coverpoints.
    pub fn coverage(&self) -> f64 {
        if self.coverpoints.is_empty() {
            return 1.0;
        }
        let total_combos: usize = self.coverpoints.iter()
            .map(|cp| cp.bins.len().max(1))
            .product();
        let hit_combos = self.hits.values().filter(|&&c| c > 0).count();
        hit_combos as f64 / total_combos as f64
    }
}
