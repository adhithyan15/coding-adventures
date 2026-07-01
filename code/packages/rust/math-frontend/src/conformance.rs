//! A **shared conformance harness** every frontend can be run through.
//!
//! It enforces the three invariants the [`MathFrontend`](crate::MathFrontend) contract
//! promises, so a consumer can trust any registered frontend:
//!
//! 1. **Total & panic-free** — parsing a sample never panics (caught via
//!    [`std::panic::catch_unwind`]); a failure must be a returned `Err`, not a crash.
//! 2. **Well-formed errors** — every `FrontendError` names this frontend and carries an
//!    in-range, non-inverted byte span.
//! 3. **Honest capabilities** — the frontend never emits a neutral construct it did not
//!    advertise in [`capabilities`](crate::MathFrontend::capabilities). (Declaring a
//!    capability you simply didn't exercise on the sample set is fine — under-claiming is
//!    safe; over-emitting is the bug.)
//!
//! Each frontend ships its own notation-specific golden tests on top of this.

use crate::expr::MathExpr;
use crate::frontend::{Capabilities, MathFrontend};
use std::panic::AssertUnwindSafe;

/// The outcome of running a frontend through the harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceReport {
    pub frontend: String,
    pub samples_checked: usize,
    /// Empty iff the frontend conformed on every sample.
    pub issues: Vec<String>,
}

impl ConformanceReport {
    pub fn passed(&self) -> bool {
        self.issues.is_empty()
    }
}

/// Run `frontend` over `samples`, checking the three contract invariants. Never panics —
/// even if the frontend under test does (that panic becomes a recorded issue).
pub fn check_frontend(frontend: &dyn MathFrontend, samples: &[&str]) -> ConformanceReport {
    let name = frontend.name().to_string();
    let declared = frontend.capabilities();
    let mut issues = Vec::new();

    for &sample in samples {
        // (1) total & panic-free
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| frontend.parse(sample)));
        let parsed = match result {
            Ok(r) => r,
            Err(_) => {
                issues.push(format!("panicked while parsing {sample:?}"));
                continue;
            }
        };

        match parsed {
            Ok(expr) => {
                // (3) honest capabilities
                let mut used = Capabilities::none();
                collect_used(&expr, &mut used);
                for missing in over_emitted(used, declared) {
                    issues.push(format!(
                        "{sample:?}: emits `{missing}` but capabilities().{missing} is false"
                    ));
                }
            }
            Err(e) => {
                // (2) well-formed errors
                if e.frontend != name {
                    issues.push(format!(
                        "{sample:?}: error names frontend {:?}, expected {:?}",
                        e.frontend, name
                    ));
                }
                let (s, end) = e.span;
                if s > end {
                    issues.push(format!("{sample:?}: inverted error span {s}..{end}"));
                }
                if end > sample.len() {
                    issues.push(format!(
                        "{sample:?}: error span end {end} exceeds source length {}",
                        sample.len()
                    ));
                }
            }
        }
    }

    ConformanceReport {
        frontend: name,
        samples_checked: samples.len(),
        issues,
    }
}

/// Names of capabilities that `used` requires but `declared` did not advertise.
fn over_emitted(used: Capabilities, declared: Capabilities) -> Vec<&'static str> {
    let mut v = Vec::new();
    let pairs: [(&str, bool, bool); 14] = [
        ("fractions", used.fractions, declared.fractions),
        ("roots", used.roots, declared.roots),
        ("powers", used.powers, declared.powers),
        ("functions", used.functions, declared.functions),
        ("big_operators", used.big_operators, declared.big_operators),
        ("relations", used.relations, declared.relations),
        ("matrices", used.matrices, declared.matrices),
        ("implicit_mul", used.implicit_mul, declared.implicit_mul),
        ("text", used.text, declared.text),
        ("plusminus", used.plusminus, declared.plusminus),
        ("binomials", used.binomials, declared.binomials),
        ("accents", used.accents, declared.accents),
        ("oversets", used.oversets, declared.oversets),
        ("sequences", used.sequences, declared.sequences),
    ];
    for (label, used_it, declared_it) in pairs {
        if used_it && !declared_it {
            v.push(label);
        }
    }
    v
}

/// Walk an expression, switching on each capability flag the tree actually uses.
/// (`implicit_mul` cannot be detected structurally — `Mul` is `Mul` however it was
/// written — so it is not inferred here; it is a parser-behavior claim the frontend's
/// own golden tests cover.)
fn collect_used(e: &MathExpr, caps: &mut Capabilities) {
    match e {
        MathExpr::Number(_) | MathExpr::Symbol(_) => {}
        MathExpr::Bin(crate::BinOp::Pow, a, b) => {
            caps.powers = true;
            collect_used(a, caps);
            collect_used(b, caps);
        }
        MathExpr::Bin(crate::BinOp::PlusMinus | crate::BinOp::MinusPlus, a, b) => {
            caps.plusminus = true;
            collect_used(a, caps);
            collect_used(b, caps);
        }
        MathExpr::Bin(_, a, b) => {
            collect_used(a, caps);
            collect_used(b, caps);
        }
        MathExpr::Unary(_, a) => collect_used(a, caps),
        MathExpr::Frac(a, b) => {
            caps.fractions = true;
            collect_used(a, caps);
            collect_used(b, caps);
        }
        MathExpr::Binom(a, b) => {
            caps.binomials = true;
            collect_used(a, caps);
            collect_used(b, caps);
        }
        MathExpr::Root { degree, radicand } => {
            caps.roots = true;
            if let Some(d) = degree {
                collect_used(d, caps);
            }
            collect_used(radicand, caps);
        }
        MathExpr::Call { arg, .. } => {
            caps.functions = true;
            collect_used(arg, caps);
        }
        MathExpr::BigOp { lower, upper, body, .. } => {
            caps.big_operators = true;
            if let Some(l) = lower {
                collect_used(l, caps);
            }
            if let Some(u) = upper {
                collect_used(u, caps);
            }
            collect_used(body, caps);
        }
        MathExpr::Subscript(a, b) => {
            collect_used(a, caps);
            collect_used(b, caps);
        }
        MathExpr::Rel(_, a, b) => {
            caps.relations = true;
            collect_used(a, caps);
            collect_used(b, caps);
        }
        MathExpr::Group(a) => collect_used(a, caps),
        MathExpr::Text(_) => caps.text = true,
        MathExpr::Matrix(rows) => {
            caps.matrices = true;
            for row in rows {
                for cell in row {
                    collect_used(cell, caps);
                }
            }
        }
        MathExpr::Accent { body, .. } => {
            caps.accents = true;
            collect_used(body, caps);
        }
        MathExpr::Overset { over: a, base: b } | MathExpr::Underset { under: a, base: b } => {
            caps.oversets = true;
            collect_used(a, caps);
            collect_used(b, caps);
        }
        MathExpr::Sequence(items) => {
            caps.sequences = true;
            for item in items {
                collect_used(item, caps);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::{MathExpr, Number};
    use crate::frontend::{Capabilities, FrontendError};

    /// Honest frontend: emits only a Number, declares nothing extra → conforms.
    struct Honest;
    impl MathFrontend for Honest {
        fn name(&self) -> &str { "honest" }
        fn parse(&self, src: &str) -> Result<MathExpr, FrontendError> {
            Number::parse(src)
                .map(MathExpr::Number)
                .ok_or_else(|| FrontendError::new("honest", "nope", (0, src.len())))
        }
        fn capabilities(&self) -> Capabilities { Capabilities::none() }
    }

    /// Dishonest frontend: emits a Frac but declares no fractions capability.
    struct OverClaimer;
    impl MathFrontend for OverClaimer {
        fn name(&self) -> &str { "over" }
        fn parse(&self, _src: &str) -> Result<MathExpr, FrontendError> {
            Ok(MathExpr::Frac(
                Box::new(MathExpr::Number(Number::from_i64(1))),
                Box::new(MathExpr::Number(Number::from_i64(2))),
            ))
        }
        fn capabilities(&self) -> Capabilities { Capabilities::none() }
    }

    /// Buggy frontend: returns an out-of-range error span.
    struct BadSpan;
    impl MathFrontend for BadSpan {
        fn name(&self) -> &str { "badspan" }
        fn parse(&self, src: &str) -> Result<MathExpr, FrontendError> {
            Err(FrontendError::new("badspan", "x", (0, src.len() + 99)))
        }
        fn capabilities(&self) -> Capabilities { Capabilities::none() }
    }

    /// Crashing frontend: the harness must convert the panic into an issue, not crash.
    struct Panicker;
    impl MathFrontend for Panicker {
        fn name(&self) -> &str { "panic" }
        fn parse(&self, _src: &str) -> Result<MathExpr, FrontendError> {
            panic!("boom")
        }
        fn capabilities(&self) -> Capabilities { Capabilities::none() }
    }

    /// Dishonest frontend: emits ± / a binomial but declares neither capability.
    struct PmBinomOverClaimer;
    impl MathFrontend for PmBinomOverClaimer {
        fn name(&self) -> &str { "pmbinom" }
        fn parse(&self, src: &str) -> Result<MathExpr, FrontendError> {
            let one = || Box::new(MathExpr::Number(Number::from_i64(1)));
            if src == "pm" {
                Ok(MathExpr::Bin(crate::BinOp::PlusMinus, one(), one()))
            } else {
                Ok(MathExpr::Binom(one(), one()))
            }
        }
        fn capabilities(&self) -> Capabilities { Capabilities::none() }
    }

    #[test]
    fn honest_frontend_conforms() {
        let r = check_frontend(&Honest, &["1", "2", "x"]);
        assert!(r.passed(), "{:?}", r.issues);
        assert_eq!(r.samples_checked, 3);
    }

    #[test]
    fn over_claiming_plusminus_and_binomials_is_flagged() {
        let r = check_frontend(&PmBinomOverClaimer, &["pm", "binom"]);
        assert!(!r.passed());
        assert!(r.issues.iter().any(|i| i.contains("plusminus")));
        assert!(r.issues.iter().any(|i| i.contains("binomials")));
    }

    #[test]
    fn emitting_accent_without_declaring_is_flagged() {
        // A frontend that emits `\hat{x}` (an Accent) but declares `none()` must be flagged
        // for the `accents` capability — the conformance gate polices the new node too.
        struct AccentOverClaimer;
        impl MathFrontend for AccentOverClaimer {
            fn name(&self) -> &str { "accent" }
            fn parse(&self, _src: &str) -> Result<MathExpr, FrontendError> {
                Ok(MathExpr::Accent {
                    accent: "hat".into(),
                    body: Box::new(MathExpr::Symbol("x".into())),
                })
            }
            fn capabilities(&self) -> Capabilities { Capabilities::none() }
        }
        let r = check_frontend(&AccentOverClaimer, &["xhat"]);
        assert!(!r.passed());
        assert!(r.issues.iter().any(|i| i.contains("accents")));
    }

    #[test]
    fn emitting_overset_without_declaring_is_flagged() {
        // A frontend that emits `\overset{a}{b}` (an Overset) but declares `none()` must be
        // flagged for the `oversets` capability — the gate polices the new node too.
        struct OversetOverClaimer;
        impl MathFrontend for OversetOverClaimer {
            fn name(&self) -> &str { "overset" }
            fn parse(&self, _src: &str) -> Result<MathExpr, FrontendError> {
                Ok(MathExpr::Overset {
                    over: Box::new(MathExpr::Symbol("a".into())),
                    base: Box::new(MathExpr::Symbol("b".into())),
                })
            }
            fn capabilities(&self) -> Capabilities { Capabilities::none() }
        }
        let r = check_frontend(&OversetOverClaimer, &["aoverb"]);
        assert!(!r.passed());
        assert!(r.issues.iter().any(|i| i.contains("oversets")));
    }

    #[test]
    fn declaring_oversets_admits_overset_and_underset() {
        // Declaring `with_oversets()` makes emitting an Overset/Underset conforming.
        struct OversetHonest;
        impl MathFrontend for OversetHonest {
            fn name(&self) -> &str { "overset-honest" }
            fn parse(&self, src: &str) -> Result<MathExpr, FrontendError> {
                let s = |n: &str| Box::new(MathExpr::Symbol(n.into()));
                Ok(if src == "under" {
                    MathExpr::Underset { under: s("a"), base: s("b") }
                } else {
                    MathExpr::Overset { over: s("a"), base: s("b") }
                })
            }
            fn capabilities(&self) -> Capabilities { Capabilities::none().with_oversets() }
        }
        assert!(check_frontend(&OversetHonest, &["over", "under"]).passed());
    }

    #[test]
    fn emitting_sequence_without_declaring_is_flagged() {
        // A frontend that emits a `Sequence` but declares `none()` must be flagged for the
        // `sequences` capability — the gate polices the new node too.
        struct SequenceOverClaimer;
        impl MathFrontend for SequenceOverClaimer {
            fn name(&self) -> &str { "sequence" }
            fn parse(&self, _src: &str) -> Result<MathExpr, FrontendError> {
                Ok(MathExpr::Sequence(vec![
                    MathExpr::Symbol("a".into()),
                    MathExpr::Symbol("b".into()),
                ]))
            }
            fn capabilities(&self) -> Capabilities { Capabilities::none() }
        }
        let r = check_frontend(&SequenceOverClaimer, &["a,b"]);
        assert!(!r.passed());
        assert!(r.issues.iter().any(|i| i.contains("sequences")));
    }

    #[test]
    fn declaring_sequences_admits_sequence() {
        // Declaring `with_sequences()` makes emitting a Sequence conforming.
        struct SequenceHonest;
        impl MathFrontend for SequenceHonest {
            fn name(&self) -> &str { "sequence-honest" }
            fn parse(&self, _src: &str) -> Result<MathExpr, FrontendError> {
                Ok(MathExpr::Sequence(vec![
                    MathExpr::Symbol("a".into()),
                    MathExpr::Symbol("b".into()),
                    MathExpr::Symbol("c".into()),
                ]))
            }
            fn capabilities(&self) -> Capabilities { Capabilities::none().with_sequences() }
        }
        assert!(check_frontend(&SequenceHonest, &["a,b,c"]).passed());
    }

    #[test]
    fn declaring_accents_admits_accent() {
        // Declaring `with_accents()` makes emitting an Accent conforming.
        struct AccentHonest;
        impl MathFrontend for AccentHonest {
            fn name(&self) -> &str { "accent-honest" }
            fn parse(&self, _src: &str) -> Result<MathExpr, FrontendError> {
                Ok(MathExpr::Accent {
                    accent: "vec".into(),
                    body: Box::new(MathExpr::Symbol("v".into())),
                })
            }
            fn capabilities(&self) -> Capabilities { Capabilities::none().with_accents() }
        }
        assert!(check_frontend(&AccentHonest, &["vvec"]).passed());
    }

    #[test]
    fn all_caps_admits_plusminus_and_binom() {
        // A frontend declaring all() may emit ± and Binom without being flagged.
        struct Full;
        impl MathFrontend for Full {
            fn name(&self) -> &str { "full" }
            fn parse(&self, _src: &str) -> Result<MathExpr, FrontendError> {
                let one = || Box::new(MathExpr::Number(Number::from_i64(1)));
                Ok(MathExpr::Binom(
                    one(),
                    Box::new(MathExpr::Bin(crate::BinOp::MinusPlus, one(), one())),
                ))
            }
            fn capabilities(&self) -> Capabilities { Capabilities::all() }
        }
        assert!(check_frontend(&Full, &["x"]).passed());
    }

    #[test]
    fn over_claiming_capabilities_is_flagged() {
        let r = check_frontend(&OverClaimer, &["anything"]);
        assert!(!r.passed());
        assert!(r.issues.iter().any(|i| i.contains("fractions")));
    }

    #[test]
    fn out_of_range_error_span_is_flagged() {
        let r = check_frontend(&BadSpan, &["abc"]);
        assert!(!r.passed());
        assert!(r.issues.iter().any(|i| i.contains("exceeds source length")));
    }

    #[test]
    fn a_panic_becomes_an_issue_not_a_crash() {
        // Silence the default panic hook so the expected panic doesn't spam test output.
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let r = check_frontend(&Panicker, &["1"]);
        std::panic::set_hook(prev);
        assert!(!r.passed());
        assert!(r.issues.iter().any(|i| i.contains("panicked")));
    }
}
