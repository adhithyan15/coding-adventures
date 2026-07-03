//! `Coverpoint` — a single signal under coverage observation.
//!
//! ## Anatomy of a coverpoint
//!
//! ```text
//! Coverpoint {
//!   name:   "a_value"           ← label for reporting
//!   signal: "a"                 ← which signal to watch
//!   bins:   [ zero, one ]       ← expected value ranges
//!   hits:   { "zero": 3, "one": 1 }  ← populated by sample()
//! }
//! ```
//!
//! ## Coverage percentage
//!
//! A bin is "hit" if its count is ≥ 1.  Coverage = hit_bins / total_bins.
//!
//! ```text
//! bins:  [ zero(0), one(0) ]   →  0/2 = 0 %
//! after sample(0):
//!        [ zero(1), one(0) ]   →  1/2 = 50 %
//! after sample(1):
//!        [ zero(1), one(1) ]   →  2/2 = 100 %
//! ```

use std::collections::HashMap;

use crate::bins::Bin;

/// Watches one signal; counts how many times each bin is matched.
#[derive(Debug, Clone)]
pub struct Coverpoint {
    /// Human-readable label.
    pub name: String,
    /// Name of the signal this coverpoint observes.
    pub signal: String,
    /// The bins (in first-match-wins order).
    pub bins: Vec<Bin>,
    /// Hit counts per bin name.  Initialised to 0 for every bin.
    pub hits: HashMap<String, u64>,
}

impl Coverpoint {
    /// Create a coverpoint.  Initialises all hit counts to 0.
    pub fn new(name: impl Into<String>, signal: impl Into<String>, bins: Vec<Bin>) -> Self {
        let mut hits = HashMap::new();
        let bins_copy: Vec<Bin> = bins.into_iter().collect();
        for b in &bins_copy {
            hits.entry(b.name.clone()).or_insert(0);
        }
        Coverpoint {
            name: name.into(),
            signal: signal.into(),
            bins: bins_copy,
            hits,
        }
    }

    /// Record one sample.  The first matching bin is incremented.
    ///
    /// If no bin matches (e.g., value outside all defined ranges), the sample
    /// is silently dropped — just like un-ranged values in SystemVerilog.
    pub fn sample(&mut self, value: i64) {
        for b in &self.bins {
            if (b.matcher)(value) {
                *self.hits.entry(b.name.clone()).or_insert(0) += 1;
                return; // first-match-wins
            }
        }
    }

    /// Fraction of bins that have been hit at least once.
    ///
    /// Returns 1.0 (100 %) when there are no bins defined (trivially covered).
    pub fn coverage(&self) -> f64 {
        if self.bins.is_empty() {
            return 1.0;
        }
        let hit_count = self.bins.iter()
            .filter(|b| self.hits.get(&b.name).copied().unwrap_or(0) > 0)
            .count();
        hit_count as f64 / self.bins.len() as f64
    }
}
