//! Thread-local test registry.
//!
//! ## Why thread-local?
//!
//! Rust does not have Python's module-level mutable state, so we use a
//! *thread-local* `RefCell<Vec<TestCase>>`.  This means:
//!
//! - No `Mutex` overhead for single-threaded test runs.
//! - Each test thread gets its own registry, preventing cross-test pollution.
//! - The same test binary can register and run tests repeatedly by calling
//!   [`clear_registry`] between runs.
//!
//! ## Usage pattern
//!
//! ```rust
//! use testbench_framework::{register_test, discover, clear_registry, DutHandle};
//!
//! register_test("always_passes", |_dut: &mut DutHandle| {
//!     // nothing to assert → passes
//! });
//!
//! let cases = discover();
//! assert_eq!(cases.len(), 1);
//!
//! clear_registry();
//! assert!(discover().is_empty());
//! ```

use std::cell::RefCell;

use crate::runner::{DutHandle, TestCase};

thread_local! {
    static REGISTRY: RefCell<Vec<TestCase>> = const { RefCell::new(Vec::new()) };
}

/// Register a test function under `name`.
///
/// Equivalent to the Python `@test` decorator.
pub fn register_test<F>(name: impl Into<String>, f: F)
where
    F: Fn(&mut DutHandle) + Send + 'static,
{
    REGISTRY.with(|r| r.borrow_mut().push(TestCase::new(name, f)));
}

/// Return all currently registered tests, consuming the registry.
///
/// The registry is left empty after this call.  This mirrors Python's
/// `discover()`, which returns a snapshot and leaves the global list intact —
/// here we drain to avoid `TestCase` not implementing `Clone` (closures don't).
pub fn discover() -> Vec<TestCase> {
    REGISTRY.with(|r| r.borrow_mut().drain(..).collect())
}

/// Empty the registry without running any tests.
pub fn clear_registry() {
    REGISTRY.with(|r| r.borrow_mut().clear());
}
