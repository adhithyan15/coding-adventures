//! `CoverageRecorder` — subscribes to a `HardwareVm` and accumulates
//! toggle, coverpoint, and cross coverage data.
//!
//! ## Thread-safety and ownership
//!
//! The `HardwareVm::subscribe` API requires callbacks that are `Send + 'static`.
//! But the recorder's internal state needs to be mutated from both the callback
//! and the public API (e.g., `add_coverpoint`).
//!
//! Solution: all mutable state lives inside `Arc<Mutex<RecorderInner>>`.
//! - The **callback** clones the `Arc` and locks the mutex on each event.
//! - The **recorder struct** holds its own `Arc` clone to lock for API calls.
//!
//! This means calls to `add_coverpoint` / `report` are quick mutex lock/unlock
//! operations — safe and predictable.
//!
//! ## Toggle coverage explained
//!
//! For each enabled signal we count:
//! - **rising**: transitions where old_value = 0 and new_value ≠ 0
//! - **falling**: transitions where old_value ≠ 0 and new_value = 0
//!
//! Full toggle = at least one rising AND at least one falling.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use hardware_vm::{Event, HardwareVm};

use crate::coverpoint::Coverpoint;
use crate::cross::CrossPoint;

// ---------------------------------------------------------------------------
// ToggleStats
// ---------------------------------------------------------------------------

/// Per-signal transition counts.
#[derive(Debug, Clone, Default)]
pub struct ToggleStats {
    /// Number of 0 → non-zero transitions.
    pub rising: u64,
    /// Number of non-zero → 0 transitions.
    pub falling: u64,
}

// ---------------------------------------------------------------------------
// CoverageReport — a snapshot
// ---------------------------------------------------------------------------

/// Snapshot of all coverage data produced by [`CoverageRecorder::report`].
#[derive(Debug, Clone)]
pub struct CoverageReport {
    /// Per-coverpoint hit counts: `report.coverpoints["cp_name"]["bin_name"]`.
    pub coverpoints: HashMap<String, HashMap<String, u64>>,
    /// Per-cross hit counts: `report.crosses["cross_name"][&["bin_a", "bin_b"]]`.
    pub crosses: HashMap<String, HashMap<Vec<String>, u64>>,
    /// Per-signal toggle statistics: `report.toggle["signal_name"].rising`.
    pub toggle: HashMap<String, ToggleStats>,
}

// ---------------------------------------------------------------------------
// RecorderInner — guarded by Mutex
// ---------------------------------------------------------------------------

struct RecorderInner {
    coverpoints:     HashMap<String, Coverpoint>,
    crosses:         HashMap<String, CrossPoint>,
    toggle_signals:  std::collections::HashSet<String>,
    toggle:          HashMap<String, ToggleStats>,
    last_values:     HashMap<String, i64>,
}

impl RecorderInner {
    fn new() -> Self {
        RecorderInner {
            coverpoints:    HashMap::new(),
            crosses:        HashMap::new(),
            toggle_signals: std::collections::HashSet::new(),
            toggle:         HashMap::new(),
            last_values:    HashMap::new(),
        }
    }

    fn on_event(&mut self, event: &Event) {
        let sig      = &event.signal;
        let new_val  = event.new_value;
        let old_val  = event.old_value;

        // Track last-seen value for cross sampling.
        self.last_values.insert(sig.clone(), new_val);

        // Toggle counting.
        if self.toggle_signals.contains(sig) {
            let stats = self.toggle.entry(sig.clone()).or_default();
            if old_val == 0 && new_val != 0 {
                stats.rising  += 1;
            } else if old_val != 0 && new_val == 0 {
                stats.falling += 1;
            }
        }

        // Sample coverpoints watching this signal.
        for cp in self.coverpoints.values_mut() {
            if cp.signal == *sig {
                cp.sample(new_val);
            }
        }

        // Forward last_values to all crosses (they read these on sample_cross()).
        for cross in self.crosses.values_mut() {
            cross.last_values.insert(sig.clone(), new_val);
        }
    }
}

// ---------------------------------------------------------------------------
// CoverageRecorder — public API
// ---------------------------------------------------------------------------

/// Hooks into a [`HardwareVm`] to accumulate coverage data.
///
/// ## Usage
///
/// ```rust
/// use coverage_hdl::{CoverageRecorder, Coverpoint, bin_value};
///
/// // let mut recorder = CoverageRecorder::new(&mut vm);
/// // recorder.add_coverpoint(Coverpoint::new("a_bits", "a",
/// //     vec![bin_value("zero", 0), bin_value("one", 1)]));
/// // recorder.enable_toggle_coverage(&["a", "y"]);
/// // vm.set_input("a", 1).unwrap();
/// // let report = recorder.report();
/// ```
pub struct CoverageRecorder {
    inner: Arc<Mutex<RecorderInner>>,
}

impl CoverageRecorder {
    /// Create a new recorder and subscribe to `vm`'s value-change events.
    pub fn new(vm: &mut HardwareVm) -> Self {
        let inner = Arc::new(Mutex::new(RecorderInner::new()));
        let inner_cb = Arc::clone(&inner);

        vm.subscribe(move |event: &Event| {
            if let Ok(mut guard) = inner_cb.lock() {
                guard.on_event(event);
            }
        });

        CoverageRecorder { inner }
    }

    // ---- Registration ----

    /// Add a coverpoint.  The coverpoint will be sampled on every value-change
    /// event for its signal.
    pub fn add_coverpoint(&self, cp: Coverpoint) {
        let mut g = self.inner.lock().unwrap();
        g.coverpoints.insert(cp.name.clone(), cp);
    }

    /// Add a cross-coverage point.  Call `sample_cross()` manually at the
    /// appropriate time (e.g., each clock edge) to record samples.
    pub fn add_cross(&self, cross: CrossPoint) {
        let mut g = self.inner.lock().unwrap();
        g.crosses.insert(cross.name.clone(), cross);
    }

    /// Enable toggle coverage tracking for the named signals.
    pub fn enable_toggle_coverage(&self, signals: &[&str]) {
        let mut g = self.inner.lock().unwrap();
        for s in signals {
            g.toggle_signals.insert(s.to_string());
            g.toggle.entry(s.to_string()).or_default();
        }
    }

    // ---- Sampling ----

    /// Record a cross sample.
    ///
    /// - `cross_name = None` → sample every registered cross.
    /// - `cross_name = Some("name")` → sample only that cross.
    pub fn sample_cross(&self, cross_name: Option<&str>) {
        let mut g = self.inner.lock().unwrap();
        if let Some(name) = cross_name {
            if let Some(cross) = g.crosses.get_mut(name) {
                cross.sample();
            }
        } else {
            for cross in g.crosses.values_mut() {
                cross.sample();
            }
        }
    }

    // ---- Reporting ----

    /// Return a snapshot of all coverage data.
    pub fn report(&self) -> CoverageReport {
        let g = self.inner.lock().unwrap();
        CoverageReport {
            coverpoints: g.coverpoints.iter()
                .map(|(n, cp)| (n.clone(), cp.hits.clone()))
                .collect(),
            crosses: g.crosses.iter()
                .map(|(n, cr)| (n.clone(), cr.hits.clone()))
                .collect(),
            toggle: g.toggle.clone(),
        }
    }

    /// Average coverage fraction across all registered coverpoints and crosses.
    ///
    /// Returns 0.0 when nothing is registered.
    pub fn overall_coverage(&self) -> f64 {
        let g = self.inner.lock().unwrap();
        let mut items: Vec<f64> = Vec::new();
        for cp in g.coverpoints.values() {
            items.push(cp.coverage());
        }
        for cr in g.crosses.values() {
            items.push(cr.coverage());
        }
        if items.is_empty() {
            return 0.0;
        }
        items.iter().sum::<f64>() / items.len() as f64
    }
}
