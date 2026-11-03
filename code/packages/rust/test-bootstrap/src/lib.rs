//! # test-bootstrap — the bottom rung of the testing ladder (TF01)
//!
//! Every testing tool in the verification program (VER00) is checked by the
//! layer beneath it: generated tests by the property engine, the property
//! engine by its corpus, and so on down. This crate is the layer with nothing
//! beneath it, so it is built to be checkable another way:
//!
//! 1. **Small enough to verify by reading.** No dependencies, a few hundred
//!    lines, every rule written down next to the code that enforces it.
//! 2. **Pinned by data.** `code/specs/fixtures/test-bootstrap-v1/cases.tfv`
//!    lists inputs and the verdicts expected for them. A separate Python
//!    reference (`code/scripts/bootstrap_harness_conformance.py`) re-derives
//!    every one, so passing the corpus means agreeing with an independent
//!    implementation, not just with ourselves.
//! 3. **Proved able to fail.** The corpus contains cases whose verdict must
//!    be *fail*, and the BUILD runs the harness a second time in sabotage mode
//!    (see [`run::Options::sabotage`]) and checks, from outside, that it
//!    reports failures.
//!
//! ## The four pieces
//!
//! ```text
//!   cases.tfv ──► tfv::parse_file ──► Records ──┐
//!                                               ▼
//!   prng (SplitMix64, FNV-1a, shuffle) ──► run::Suite ──► TAP v14 on stdout
//!                                               ▲            exit 0 / 1
//!                        verdict::{deep_equal, error_is}
//! ```
//!
//! - [`tfv`] reads TFV, the line format corpora travel in.
//! - [`verdict`] is the pure core: two functions that decide pass or fail.
//! - [`prng`] is the deterministic randomness that orders a run.
//! - [`run`] registers tests, shuffles them, runs them and prints TAP.
//!
//! What it deliberately lacks (fixtures, hooks, parallelism, rich assertions)
//! is listed in TF01 §7; all of it belongs to the layers built on this one.

pub mod prng;
pub mod run;
pub mod tfv;
pub mod verdict;

pub use run::{Ctx, Options, Suite, Summary};
pub use tfv::{Record, TfvError, Value};
pub use verdict::Verdict;
