//! # coverage-hdl
//!
//! HDL coverage measurement — answers "which parts of the design space did
//! our test vectors actually exercise?"
//!
//! ## What is coverage?
//!
//! When you simulate a digital circuit, you drive thousands of input
//! combinations, but did you actually hit every interesting corner?  Coverage
//! lets you *define* what "interesting" means and then *measure* how often
//! each interesting case occurred.
//!
//! Three flavours are supported:
//!
//! | Flavour | Question answered |
//! |---------|-------------------|
//! | **Toggle** | Did every signal flip 0→1 *and* 1→0 at least once? |
//! | **Functional (coverpoint)** | Did each signal value fall into every defined bin? |
//! | **Cross** | Did every *combination* of bins across two+ signals occur? |
//!
//! ## Architecture
//!
//! ```text
//! HardwareVm ──subscribe──► CoverageRecorder ──report()──► CoverageReport
//!                │                │
//!                │          ┌─────▼──────────────┐
//!                │          │  RecorderInner      │
//!                │          │  ├─ coverpoints     │
//!                │          │  ├─ crosses         │
//!                │          │  ├─ toggle signals  │
//!                │          │  └─ last_values     │
//!                │          └─────────────────────┘
//!                │
//!           Arc<Mutex<RecorderInner>>
//! ```
//!
//! The `CoverageRecorder` registers a callback with the VM.  Every time a
//! signal changes, the callback fires and updates all registered coverpoints,
//! crosses, and toggle counters.
//!
//! ## Quick start
//!
//! ```rust
//! use coverage_hdl::{bin_range, bin_value, bin_default, Coverpoint, CoverageRecorder};
//!
//! // (assumes you have a HardwareVm `vm` already constructed)
//! // let mut recorder = CoverageRecorder::new(&mut vm);
//! //
//! // recorder.add_coverpoint(Coverpoint::new(
//! //     "a_vals", "a",
//! //     vec![bin_value("zero", 0), bin_value("one", 1)],
//! // ));
//! // recorder.enable_toggle_coverage(&["a", "y"]);
//! //
//! // vm.set_input("a", 1).unwrap();
//! //
//! // let report = recorder.report();
//! // assert_eq!(report.toggle["a"].rising, 1);
//! ```

pub mod bins;
pub mod coverpoint;
pub mod cross;
pub mod recorder;

pub use bins::{bin_default, bin_range, bin_value, Bin};
pub use coverpoint::Coverpoint;
pub use cross::CrossPoint;
pub use recorder::{CoverageRecorder, CoverageReport, ToggleStats};
