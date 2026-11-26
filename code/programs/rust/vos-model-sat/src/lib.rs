//! # `vos-model-sat`: the host model-SAT lane, encoder half (VOS-040a, VOS31)
//!
//! The `sched` contract (`code/packages/rust/vos-sched-contract`) states what a scheduler MEANS
//! as a Warrant model (`warrant/contract.part`) and ten lemmas about it (`warrant/lemmas.part`).
//! An exhaustive evaluation of a lemma stops at N = 8.  This crate is the first half of the lane
//! that goes further with SAT: an **N-parametric encoder** that turns the model and each lemma
//! into an and-inverter graph (`bit-blaster`) at any N from 1 to 64.  It **does no solving**:
//! solving, DRAT certificates and the committed table are VOS-040b.
//!
//! ```text
//!   contract.part ──(hand transcription, VOS-D072)──> SchedModel / Encoder        src/model.rs
//!   mutants/*.part ─────────────────────────────────> Mutant (one per lemma)      src/mutants.rs
//!   lemmas.part ────────────────────────────────────> lemma_circuit(id, model)    src/lemmas.rs
//!
//!   the hand transcription is the trust problem, so three cross-checks (src/crosscheck.rs):
//!     (a) lemma check     vs the Warrant evaluator, N = 8, base + mutants, failing SETS equal
//!     (b) function check  vs the Warrant evaluator, N = 8, each model function
//!     (c) width check     vs a plain Rust reference (src/reference.rs), N = 1 ..= 10
//! ```
//!
//! ## The hard constraint (VOS31 section 3)
//!
//! This crate DEPENDS ON `bit-blaster`, `warrant-grammar`, `vos-link` and the contract package,
//! and edits none of them: the capacity recording of VOS30 records their tree ids.
//!
//! ## What a result here means
//!
//! Nothing in this crate proves anything about an implementation.  The labels of [`labels`] say
//! "lemma", and the wider widths say the encoder is checked at N = 8 only.

pub mod cli;
pub mod crosscheck;
pub mod labels;
pub mod lane;
pub mod lemmas;
pub mod model;
pub mod mutants;
pub mod oracle;
pub mod probes;
pub mod reference;
pub mod solve;
pub mod sweep;
pub mod types;

pub use crosscheck::{Mismatch, Report};
pub use lemmas::{lemma, lemma_circuit, LemmaCircuit, LemmaId, UnknownLemma};
pub use model::{Encoder, SchedModel};
pub use mutants::{Fault, Faulty, Mutant};
pub use types::{Out, PickEnc, Ready, Step, UpdEnc, Val};

/// The largest N the encoder accepts.  A thread id is a `U8` and 255 is the idle sentinel, so N
/// must stay below 255; 64 is the top of the ladder (VOS31 section 4) and bounds every loop.
pub const MAX_N: usize = 64;

/// A width outside `1 ..= MAX_N`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BadWidth(pub usize);

/// Run the whole cross-check on the contract crate in `files`, returning the lines to print and
/// whether every check was clean.  Used by the CLI and by the tests.
pub fn run_cross_checks(files: &oracle::ContractFiles) -> Result<(Vec<String>, bool), String> {
    let ev = crosscheck::gather(files)?;
    Ok(run_cross_checks_with(&ev))
}

/// [`run_cross_checks`] on evidence already gathered (the lane also needs it for replay).
pub fn run_cross_checks_with(ev: &crosscheck::Evidence) -> (Vec<String>, bool) {
    let (base, mutant) = crosscheck::real_encoders();
    let mut lines = Vec::new();
    let mut ok = true;
    let mut note = |name: &str, r: Report| {
        ok &= r.is_clean();
        lines.push(format!(
            "{}: {} comparisons, {} disagreements{}",
            name,
            r.compared,
            r.disagreements,
            if r.is_clean() { "" } else { "  FAILED" }
        ));
        for s in &r.samples {
            lines.push(format!(
                "    {} {} {:?}: {}",
                s.check, s.subject, s.case, s.detail
            ));
        }
    };
    note(
        "evaluator side (a) lemmas and (b) functions, N = 8, base and mutants",
        crosscheck::check_evaluator_side(ev, &base, &mutant),
    );
    note(
        "reference against the evaluator, N = 8",
        crosscheck::check_reference_vs_evaluator(&ev.base),
    );
    note(
        "(c) width check, encoder against the reference, N = 1..=10",
        crosscheck::check_all_widths(&crosscheck::honest_at),
    );
    (lines, ok)
}
