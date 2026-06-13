//! Core runner types: `DutHandle`, `TestCase`, `TestReport`, and `run()`.
//!
//! ## How panic catching works
//!
//! Hardware tests written in Rust naturally express assertions with `assert_eq!`
//! or `assert!`, which *panic* on failure.  Rust's `std::panic::catch_unwind`
//! traps panics without unwinding the whole process, letting the runner record
//! a failure message and continue to the next test — exactly like Python's
//! `except AssertionError`.
//!
//! We wrap the call in `AssertUnwindSafe` because `DutHandle` contains a
//! `HardwareVm` which is not automatically `UnwindSafe` (it has interior
//! mutable state).  This is safe here because we throw the VM away after each
//! test; there's no way to observe a half-updated state.
//!
//! ## Fresh VM per test
//!
//! Each `TestCase` receives a *brand-new* `HardwareVm` built from the same
//! `Hir`.  This mirrors the Python implementation and prevents signal values
//! from leaking between tests — the hardware equivalent of "clear the bench
//! between experiments."

use std::panic::AssertUnwindSafe;
use std::time::Instant;

use hardware_vm::{HardwareVm, VmError};
use hdl_ir::Hir;

use crate::registry::discover;

// ---------------------------------------------------------------------------
// DutHandle — the "test probe"
// ---------------------------------------------------------------------------

/// A handle to the Device Under Test.
///
/// Wraps a [`HardwareVm`] and exposes a simple `get` / `set` API.
/// Think of it as the oscilloscope probe and signal generator on a lab bench.
pub struct DutHandle {
    pub(crate) vm: HardwareVm,
}

impl DutHandle {
    /// Wrap an existing `HardwareVm` in a `DutHandle`.
    ///
    /// Useful when you want to construct a DUT outside `run()`,
    /// e.g., to attach a [`CoverageRecorder`](coverage_hdl::CoverageRecorder)
    /// before handing it to a test.
    pub fn new(vm: HardwareVm) -> Self {
        DutHandle { vm }
    }

    /// Read the current value of any signal (input, output, or internal net).
    ///
    /// Returns 0 if the signal name is unknown (consistent with a default
    /// undriven net reading as logic-0).
    pub fn get(&self, name: &str) -> i64 {
        self.vm.read(name)
    }

    /// Drive an input port to `value`.
    ///
    /// Panics with a descriptive message if `name` is not a writable port,
    /// so test failures surface immediately rather than silently propagating.
    pub fn set(&mut self, name: &str, value: i64) {
        self.vm.set_input(name, value).unwrap_or_else(|e| {
            panic!("DutHandle::set({name:?}, {value}): {e}");
        });
    }

    /// Expose the inner VM for advanced use (e.g., coverage recording).
    pub fn vm_mut(&mut self) -> &mut HardwareVm {
        &mut self.vm
    }
}

// ---------------------------------------------------------------------------
// TestCase — one named experiment
// ---------------------------------------------------------------------------

/// A single test: a name, a closure, optional timeout, and a pass/fail sense.
///
/// The closure receives a `DutHandle` and may:
/// - Drive inputs with `dut.set(name, value)`
/// - Read outputs with `dut.get(name)`
/// - Assert correctness with `assert_eq!` / `assert!`
///
/// ## `should_fail`
///
/// Set `should_fail = true` for negative tests — cases where the circuit is
/// *expected* to produce wrong results (e.g., testing that a broken design
/// fails).  The runner inverts pass/fail logic for these.
pub struct TestCase {
    pub name: String,
    /// The test body.  Must be `Send` so the registry can cross thread
    /// boundaries if needed, and `'static` so it can be stored globally.
    pub func: Box<dyn Fn(&mut DutHandle) + Send + 'static>,
    /// Wall-clock budget for this test.  Not yet enforced (v0.2.0 adds
    /// timeout via a watcher thread), but recorded for documentation.
    pub timeout_s: f64,
    /// If true, a test that *panics* is counted as passed, and one that
    /// *does not panic* is counted as failed.
    pub should_fail: bool,
}

impl TestCase {
    /// Create a test case with default settings (5 s timeout, pass sense).
    pub fn new<F>(name: impl Into<String>, f: F) -> Self
    where
        F: Fn(&mut DutHandle) + Send + 'static,
    {
        TestCase {
            name: name.into(),
            func: Box::new(f),
            timeout_s: 5.0,
            should_fail: false,
        }
    }

    /// Override the timeout (informational in v0.1.0).
    pub fn with_timeout(mut self, timeout_s: f64) -> Self {
        self.timeout_s = timeout_s;
        self
    }

    /// Mark this test as expected to fail (negative test).
    pub fn expect_fail(mut self) -> Self {
        self.should_fail = true;
        self
    }
}

// ---------------------------------------------------------------------------
// TestReport — the lab notebook
// ---------------------------------------------------------------------------

/// Collects test outcomes from a single `run()` call.
#[derive(Debug, Default)]
pub struct TestReport {
    /// Names of tests that passed.
    pub passed: Vec<String>,
    /// `(name, error_message)` pairs for failed tests.
    pub failed: Vec<(String, String)>,
    /// Names of tests that were skipped (reserved for future use).
    pub skipped: Vec<String>,
    /// Wall-clock time for the entire run.
    pub duration_s: f64,
}

impl TestReport {
    /// `true` if no tests failed.
    pub fn all_passed(&self) -> bool {
        self.failed.is_empty()
    }

    /// One-line human-readable summary.
    ///
    /// Example: `"3 passed, 1 failed, 0 skipped in 0.012s"`
    pub fn summary(&self) -> String {
        format!(
            "{} passed, {} failed, {} skipped in {:.3}s",
            self.passed.len(),
            self.failed.len(),
            self.skipped.len(),
            self.duration_s,
        )
    }
}

// ---------------------------------------------------------------------------
// run() — the orchestrator
// ---------------------------------------------------------------------------

/// Run all given tests (or all registered tests if `tests` is `None`)
/// against a fresh `HardwareVm` built from `hir`.
///
/// Each test gets its own VM instance — signal state never leaks between tests.
///
/// Panics inside test closures are caught and recorded as failures, so one
/// broken test never aborts the rest of the run.
pub fn run(hir: Hir, tests: Option<Vec<TestCase>>) -> TestReport {
    let tests = tests.unwrap_or_else(discover);
    let mut report = TestReport::default();
    let start = Instant::now();

    for tc in tests {
        // Build a fresh simulator for this test.
        let vm = match HardwareVm::new(hir.clone()) {
            Ok(v) => v,
            Err(VmError::TopModuleNotFound(n)) => {
                report.failed.push((tc.name, format!("VM init: top module {n:?} not found")));
                continue;
            }
            Err(e) => {
                report.failed.push((tc.name, format!("VM init: {e}")));
                continue;
            }
        };
        let mut dut = DutHandle { vm };

        // Catch panics so one failing test does not abort the run.
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            (tc.func)(&mut dut);
        }));

        match result {
            Ok(()) => {
                if tc.should_fail {
                    report.failed.push((tc.name, "expected failure but test passed".into()));
                } else {
                    report.passed.push(tc.name);
                }
            }
            Err(payload) => {
                // Extract a human-readable message from the panic payload.
                let msg = if let Some(s) = payload.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = payload.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "unknown panic".to_string()
                };

                if tc.should_fail {
                    report.passed.push(tc.name);
                } else {
                    report.failed.push((tc.name, format!("assertion failed: {msg}")));
                }
            }
        }
    }

    report.duration_s = start.elapsed().as_secs_f64();
    report
}
