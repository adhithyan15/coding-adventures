//! # Labels: what is printed, and what is never printed (VOS31 section 11)
//!
//! A lemma has no published strength (VOS-D005), so a SAT result about it must never read as
//! evidence about a scheduler.  The label is therefore part of the artifact, and these strings
//! are exact: a test asserts each, and a second test asserts that none of the forbidden words
//! appears in any label.
//!
//! ```text
//!   N = 8        lemma, SAT-certified at N = 8; encoder checked against the evaluator on the whole domain
//!   N = 16/32/64 lemma family member, SAT-certified at N = <N>; the encoder is checked against the
//!                evaluator at N = 8 only; no shipped component has this width
//!   closure_at_m trivial (inv is true)                       (and it is not counted)
//!   a collapse   the row above, plus  collapsed (<reason>)
//!   spike        lowered helper
//! ```

/// The label of an N = 8 row.
pub const LABEL_N8: &str =
    "lemma, SAT-certified at N = 8; encoder checked against the evaluator on the whole domain";

/// The label of a trivial lemma (`closure_at_m`): printed, and not counted.
pub const LABEL_TRIVIAL: &str = "trivial (inv is true)";

/// The label of a successfully lowered helper of the Rust-lowering spike (VOS-040d).
pub const LABEL_LOWERED_HELPER: &str = "lowered helper";

/// Words no label may contain (VOS31 section 11, "never printed"), checked on the label with the
/// one honest phrase `no shipped component has this width` removed.
pub const FORBIDDEN_WORDS: [&str; 4] = ["proved", "verified", "shipped", "X3"];

/// The phrase of the wider-row label that says no component has that width.
pub const NO_SHIPPED_PHRASE: &str = "no shipped component has this width";

/// The label of a row at width `n`.
pub fn label_for(n: usize) -> String {
    if n == 8 {
        LABEL_N8.to_string()
    } else {
        format!(
            "lemma family member, SAT-certified at N = {n}; the encoder is checked against the evaluator at N = 8 only; {NO_SHIPPED_PHRASE}"
        )
    }
}

/// Why an obligation collapsed: the CLOSED set of VOS31 7.5 (Q7).  A reason is a value of this
/// enum, never free text, so a label can only ever carry one of these two spellings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollapseReason {
    /// `holds` is the constant true edge: the negation is the constant false.
    ConstantFold,
    /// The CNF is not trivial, but unit propagation alone refutes it.
    PropagationOnly,
}

impl CollapseReason {
    /// Both reasons, for tests.
    pub const ALL: [CollapseReason; 2] = [
        CollapseReason::ConstantFold,
        CollapseReason::PropagationOnly,
    ];

    /// The spelling the table and the label use.
    pub fn as_str(self) -> &'static str {
        match self {
            CollapseReason::ConstantFold => "constant-fold",
            CollapseReason::PropagationOnly => "propagation-only",
        }
    }

    /// The reason spelled `s`, if it is one of the two.
    pub fn parse(s: &str) -> Option<CollapseReason> {
        CollapseReason::ALL.into_iter().find(|r| r.as_str() == s)
    }
}

/// The label with `collapsed (<reason>)` appended, for a collapsed obligation.
pub fn label_collapsed(n: usize, reason: CollapseReason) -> String {
    format!("{}; collapsed ({})", label_for(n), reason.as_str())
}

/// The label as one table token: spaces become `_` (VOS31 8.2), so a row stays one line.
pub fn label_token(label: &str) -> String {
    label.replace(' ', "_")
}
