//! `vm-type-suggestions` — parameter type suggestions from the VM profiler.
//!
//! After running a program under vm-core, this crate reads the profiler
//! observations on each function's parameter-loading instructions and asks:
//!
//! > "Based on what the VM actually observed at runtime, what type annotations
//! >  should the developer add?"
//!
//! No JIT required.  No guard analysis.  Just: "here's what came in — you
//! should say so in the source code."
//!
//! # Public API
//!
//! - [`suggest`] — the main entry point.  Returns a [`SuggestionReport`].
//!
//! - [`SuggestionReport`] — top-level result: [`SuggestionReport::actionable`]
//!   (CERTAIN suggestions only), [`SuggestionReport::by_function`] (grouped view),
//!   [`SuggestionReport::format_text`], [`SuggestionReport::format_json`].
//!
//! - [`ParamSuggestion`] — one parameter: name, observed type, confidence,
//!   suggestion string.
//!
//! - [`Confidence`] — `Certain` / `Mixed` / `NoData`.
//!
//! # Quick start
//!
//! ```
//! use vm_type_suggestions::{suggest, Confidence};
//! use interpreter_ir::{IIRFunction, IIRInstr, Operand};
//!
//! // A function with one untyped parameter and a load_mem instruction.
//! let mut load = IIRInstr::new(
//!     "load_mem",
//!     Some("r0".into()),
//!     vec![Operand::Var("arg[0]".into())],
//!     "any",
//! );
//! load.observation_count = 1_000_000;
//! load.observed_type = Some("u8".into());
//!
//! let fn_ = IIRFunction::new(
//!     "add",
//!     vec![("n".into(), "any".into())],
//!     "any",
//!     vec![load],
//! );
//!
//! let report = suggest(&[fn_], "fibonacci");
//! assert_eq!(report.suggestions.len(), 1);
//! assert_eq!(report.suggestions[0].confidence, Confidence::Certain);
//! assert_eq!(
//!     report.suggestions[0].suggestion.as_deref(),
//!     Some("declare 'n: u8'"),
//! );
//!
//! println!("{}", report.format_text());
//! ```

pub mod suggest;
pub mod types;

pub use suggest::suggest;
pub use types::{Confidence, ParamSuggestion, SuggestionReport};
