//! # testbench-framework
//!
//! A Rust harness for writing and running tests against an HDL design.
//!
//! ## The mental model
//!
//! Think of this like a hardware lab bench:
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │  TestCase  (what to check)                  │
//! │    ↓                                        │
//! │  run(hir)  (wire up the lab)                │
//! │    ↓                                        │
//! │  HardwareVm  (the silicon simulator)        │
//! │    ↓                                        │
//! │  DutHandle  (probes + signal generators)    │
//! │    ↓                                        │
//! │  TestReport  (pass / fail log)              │
//! └─────────────────────────────────────────────┘
//! ```
//!
//! ## Quick start
//!
//! ```rust
//! use testbench_framework::{DutHandle, TestCase, run};
//!
//! let tc = TestCase::new("buf_high", |dut: &mut DutHandle| {
//!     dut.set("a", 1);
//!     assert_eq!(dut.get("a"), 1);
//! });
//!
//! // Construct a HIR and run:
//! // let report = run(hir, Some(vec![tc]));
//! // assert!(report.all_passed());
//! ```
//!
//! ## Stimulus helpers
//!
//! ```rust
//! use testbench_framework::{DutHandle, exhaustive, random_stimulus};
//! use std::collections::HashMap;
//!
//! // Drive all 2^n combinations (handy for small combinational circuits):
//! // exhaustive(dut, &[("a", 4), ("b", 4)], Some(|d: &mut DutHandle| { ... }));
//!
//! // Or drive N random vectors:
//! // random_stimulus(dut, &[("a", 4), ("b", 4)], 100, 42, Some(|d| { ... }));
//! ```

pub mod registry;
pub mod runner;
pub mod stimulus;

pub use registry::{clear_registry, discover, register_test};
pub use runner::{run, DutHandle, TestCase, TestReport};
pub use stimulus::{exhaustive, random_stimulus};
