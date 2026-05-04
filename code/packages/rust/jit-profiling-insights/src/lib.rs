//! `jit-profiling-insights` — developer feedback from the JIT compiler.
//!
//! This crate surfaces the JIT compiler's internal decisions to the developer
//! as actionable advice.  Instead of "this function took 40ms" (profiler output),
//! it says "this expression causes 1 type guard per call because `n` is declared
//! untyped — annotating it as `u8` eliminates the branch and saves ~8% runtime".
//!
//! # Public API
//!
//! - [`analyze`] — the main entry point.  Accepts a slice of [`IIRFunction`]
//!   objects whose instructions carry profiler annotations (`observed_type`,
//!   `observation_count`) and returns a structured [`ProfilingReport`].
//!
//! - [`ProfilingReport`] — the top-level result: `total_instructions_executed`,
//!   ranked [`TypeSite`] list, and [`ProfilingReport::format_text`] /
//!   [`ProfilingReport::format_json`] renderers.
//!
//! - [`TypeSite`] — one instruction-level hotspot: which register is untyped,
//!   what the profiler observed, how expensive the dispatch is, and what to do.
//!
//! - [`DispatchCost`] — four-level enum: `None` / `Guard` / `GenericCall` /
//!   `Deopt`.  Drives the impact formula (`call_count × cost_weight`).
//!
//! # Quick start
//!
//! ```
//! use jit_profiling_insights::{analyze, DispatchCost};
//! use interpreter_ir::{IIRFunction, IIRInstr, Operand};
//!
//! // Build a function with one untyped type_assert instruction (Guard cost).
//! let mut instr = IIRInstr::new(
//!     "type_assert",
//!     None,
//!     vec![Operand::Var("r0".into())],
//!     "any",
//! );
//! instr.observation_count = 1_000_000;
//!
//! let fn_ = IIRFunction::new("fibonacci", vec![], "any", vec![instr]);
//! let report = analyze(&[fn_], "fibonacci", 1);
//!
//! assert_eq!(report.sites.len(), 1);
//! assert_eq!(report.sites[0].dispatch_cost, DispatchCost::Guard);
//! assert_eq!(report.sites[0].call_count, 1_000_000);
//!
//! // Human-readable output:
//! let text = report.format_text();
//! assert!(text.contains("fibonacci"));
//! assert!(text.contains("GUARD"));
//! ```

pub mod analyze;
pub mod classify;
pub mod rank;
pub mod types;

pub use analyze::analyze;
pub use types::{DispatchCost, ProfilingReport, TypeSite};
