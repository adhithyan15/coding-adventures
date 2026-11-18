//! # The one error type of the generator
//!
//! VER04r section 13 ("Slice S3 in detail") says what a generator does when it meets
//! something it cannot emit: it **refuses the whole document**, names the reason, and
//! writes nothing.  [`GenError`] is that refusal.
//!
//! ```text
//!   code      meaning                                      who raises it
//!   --------  -------------------------------------------  ----------------------
//!   "level"   a construct beyond `ir.core` (level named)    lowering
//!   "W0701"   a VER04r generation rule is violated          lowering, cost, arith
//!   "type"    the document does not type check              lowering
//!   "ir"      a corpus error of Appendix A.1 (a `match` that falls   lowering
//!             through, an unreachable arm, a duplicate or
//!             shadowing `bind`)
//!   "check"   the S2 checker refused the document           lowering
//!   "limit"   a resource bound of VER04r section 14         lowering, cost, profile
//!   "W0706"   a suite admits nothing / is over budget      enumeration
//!   "W0707"   a `Bytes<N>` wider than 4096                 domain resolution
//!   "domain"  a descriptor VER04r 16.2 leaves undecided    domain resolution
//!   "profile" `corpus.profile` is malformed                 profile reader
//!   "json"    a value cannot be rendered canonically        json writer
//!   "fault" "drift" "io"   faults and controls, --check, files (S3c)
//! ```
//!
//! A message **never echoes input bytes** (VER04r section 14, "no reproducing material
//! in public output"): it is one of this crate's own sentences, and `at` names a claim,
//! model, operation or example by the identifier the document itself declares.

use std::fmt;

/// The codes a [`GenError`] can carry, as constants so a test can match them without
/// typing the string twice.
pub mod code {
    /// A construct belongs to a feature level the generator has not implemented.
    pub const LEVEL: &str = "level";
    /// A VER04r generation rule (the `W0701` family of the v1 generator).
    pub const W0701: &str = "W0701";
    /// The document does not type check under the model IR's rules.
    pub const TYPE: &str = "type";
    /// The S2 checker (`warrant-grammar`) refused the document.
    pub const CHECK: &str = "check";
    /// A corpus error of VER04r Appendix A.1 (S10t-2), the generator's reading of the oracle's
    /// `corpus` code: a `match` that can fall through, an arm no value can reach, the same `bind`
    /// twice in one pattern, a `bind` that shadows a binder, a parameter, `result` or an outer
    /// `bind`.  Raised in this order of precedence (VER04r 16.10 R12): per pattern, duplicate and
    /// shadowing binds; then, per match, unreachable arms in arm order, then fall-through.
    pub const IR: &str = "ir";
    /// A resource bound was exceeded (IR depth, cost, work, profile size).
    pub const LIMIT: &str = "limit";
    /// A suite admits no case, or its product is over the budget (the v1 generator's code).
    pub const W0706: &str = "W0706";
    /// A `Bytes<N>` wider than `MAX_FIXED_WIDTH` (the v1 generator's code).
    pub const W0707: &str = "W0707";
    /// A binder descriptor that VER04r 16.2 leaves undecided (`nat_count`, a signed
    /// `fx_edge`, `FxEdge` at 16 bits, a list of byte strings): refused until S10 decides.
    pub const DOMAIN: &str = "domain";
    /// `corpus.profile` is malformed.
    pub const PROFILE: &str = "profile";
    /// A JSON value cannot be rendered by the canonical writer.
    pub const JSON: &str = "json";
    /// A fault cannot be instantiated, no claim can catch it, or every claim is blind to it.
    pub const FAULT: &str = "fault";
    /// An on-disk corpus differs from the regenerated one (`--check`), or the profile's
    /// fault names differ from the bindings'.
    pub const DRIFT: &str = "drift";
    /// A file could not be read or written, or a path was refused.
    pub const IO: &str = "io";
}

/// Where in the language a refused construct lives: the feature level it needs.
///
/// The first four are the levels of VER04r section 4.1.  The last four are not levels
/// but *features* the generator also cannot emit yet (`inout`/`ensures` are slice S12,
/// `vectors` tables and `assume` rows have no corpus representation in this document);
/// they are named in the error exactly as a level is, so a census can group by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Beyond {
    /// `ir.fixed`: integer built-ins (`wrap`, `shl`, `bit_and`, `be_bytes`, ...).
    IrFixed,
    /// `ir.tuple`: tuple types, the tuple literal, `project`, and `match` on a `Result`
    /// (VER04r 4.1, [D20, D35]).  Lowered, costed and evaluated since slice S10t-2; the level is
    /// still named here because a *consumer* that does not implement it must name it, and for
    /// the faults whose evaluation is gated (VER04r 16.10 R12).
    IrTuple,
    /// `ir.data`: records, variants, `Option` anywhere but under `unwrap_or`, lists, `match` on
    /// anything but a `Result`, constructor and record projection.
    IrData,
    /// `ir.seq`: the reserved WX-D `fold`/`map`/`all`/`any`/`count` level.
    IrSeq,
    /// `ir.inout.records`: reserved record-shaped `inout` host forms.
    IrInoutRecords,
    /// WX-G `inout` parameters (slice S12).
    Inout,
    /// WX-G `ensures` rows (slice S12).
    Ensures,
    /// WX-E `vectors` tables (no VER04r representation yet).
    Vectors,
    /// `assume platform` / `assume environment` rows that assume something.
    Assume,
}

impl Beyond {
    /// The name written in an error and in a census.
    pub fn name(self) -> &'static str {
        match self {
            Beyond::IrFixed => "ir.fixed",
            Beyond::IrTuple => "ir.tuple",
            Beyond::IrData => "ir.data",
            Beyond::IrSeq => "ir.seq",
            Beyond::IrInoutRecords => "ir.inout.records",
            Beyond::Inout => "inout",
            Beyond::Ensures => "ensures",
            Beyond::Vectors => "vectors",
            Beyond::Assume => "assume",
        }
    }
}

/// A named refusal.  See the module documentation for the codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenError {
    /// One of the [`code`] constants.
    pub code: &'static str,
    /// For `code == "level"`: the [`Beyond::name`] of the refused construct.
    pub level: Option<&'static str>,
    /// The claim, model, operation, example or declaration the refusal is about
    /// (an identifier the document declares, or empty when the document as a whole is
    /// at fault).
    pub at: String,
    /// One of this crate's own sentences; never input bytes.
    pub message: String,
    /// Raised while checking a subject call's arguments against its op's `admit` for one
    /// admitted case ([D19]): the enumerator appends that case's ordinal.  A flag, not a
    /// message match, so rewording a sentence can never silently drop the ordinal.
    pub during_case: bool,
    /// Raised by one of the two `Bytes` size limits of VER04r 4.2 rule 6 (a literal, or a
    /// `concat`, past `MAX_IR_VALUE`), as the oracle's `SizeError`.  It is what an
    /// `omit_example` line may excuse as `value-over-limit` (VER04r 8.4, [D34]); an
    /// integer-window overflow is not a size error.  Always `W0701` as before.
    pub size: bool,
}

impl GenError {
    /// A refusal with `code` about `at`.
    pub fn new(code: &'static str, at: impl Into<String>, message: impl Into<String>) -> GenError {
        GenError {
            code,
            level: None,
            at: at.into(),
            message: message.into(),
            during_case: false,
            size: false,
        }
    }

    /// A `Bytes` value over `MAX_IR_VALUE`: a `W0701` that is also a *size* error
    /// ([`GenError::size`]).
    pub fn size_error(message: impl Into<String>) -> GenError {
        GenError {
            size: true,
            ..GenError::new(code::W0701, "", message)
        }
    }

    /// A refusal because a construct needs `beyond` (a level or an unimplemented
    /// feature).
    pub fn level(beyond: Beyond, at: impl Into<String>, message: impl Into<String>) -> GenError {
        GenError {
            code: code::LEVEL,
            level: Some(beyond.name()),
            at: at.into(),
            message: message.into(),
            during_case: false,
            size: false,
        }
    }

    /// This refusal belongs to one admitted case (see [`GenError::during_case`]).
    pub fn in_case(mut self) -> GenError {
        self.during_case = true;
        self
    }

    /// A refusal for a state the code's own invariants rule out (a bug here, never the
    /// document's fault): `W0701` with an `internal:` message, so it can never be mistaken for
    /// a user-facing rule.
    pub fn internal(message: &str) -> GenError {
        GenError::new(code::W0701, "", format!("internal: {message}"))
    }

    /// Fill in `at` if the refusal was raised deep inside an expression and has not
    /// been attributed yet.  An existing attribution is kept.
    pub fn at(mut self, at: &str) -> GenError {
        if self.at.is_empty() {
            self.at = at.to_string();
        }
        self
    }

    /// Is this a refusal for a feature level (as opposed to a malformed document)?
    pub fn is_level(&self) -> bool {
        self.code == code::LEVEL
    }
}

impl fmt::Display for GenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code)?;
        if let Some(level) = self.level {
            write!(f, " [{level}]")?;
        }
        if !self.at.is_empty() {
            write!(f, " at `{}`", self.at)?;
        }
        write!(f, ": {}", self.message)
    }
}

impl std::error::Error for GenError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_beyond_has_its_documented_name() {
        let names: Vec<&str> = [
            Beyond::IrFixed,
            Beyond::IrTuple,
            Beyond::IrData,
            Beyond::IrSeq,
            Beyond::IrInoutRecords,
            Beyond::Inout,
            Beyond::Ensures,
            Beyond::Vectors,
            Beyond::Assume,
        ]
        .iter()
        .map(|b| b.name())
        .collect();
        assert_eq!(
            names,
            [
                "ir.fixed",
                "ir.tuple",
                "ir.data",
                "ir.seq",
                "ir.inout.records",
                "inout",
                "ensures",
                "vectors",
                "assume"
            ]
        );
    }

    #[test]
    fn display_names_code_level_and_place_but_only_our_own_words() {
        let e = GenError::level(Beyond::IrFixed, "m", "needs ir.fixed");
        assert_eq!(e.to_string(), "level [ir.fixed] at `m`: needs ir.fixed");
        assert!(e.is_level());
        let e = GenError::new(code::W0701, "", "a rule");
        assert_eq!(e.to_string(), "W0701: a rule");
        assert!(!e.is_level());
        let e = GenError::new(code::LIMIT, "c", "too big");
        assert_eq!(e.to_string(), "limit at `c`: too big");
    }

    #[test]
    fn an_internal_refusal_says_so() {
        let e = GenError::internal("an invariant broke");
        assert_eq!(e.code, code::W0701);
        assert_eq!(e.message, "internal: an invariant broke");
        assert_eq!(e.at("claim").at, "claim");
    }

    #[test]
    fn only_a_size_error_is_flagged_and_it_is_still_w0701() {
        let e = GenError::size_error("a Bytes value is too long");
        assert!(e.size);
        assert_eq!((e.code, e.message.as_str()), (code::W0701, "a Bytes value is too long"));
        assert!(!GenError::new(code::W0701, "", "x").size);
        assert!(!GenError::level(Beyond::IrFixed, "", "x").size);
        assert!(!GenError::internal("x").size);
        // attribution keeps the flag
        assert!(e.at("claim").size);
    }

    #[test]
    fn at_fills_an_unattributed_error_and_keeps_an_attributed_one() {
        let e = GenError::new(code::TYPE, "", "x").at("claim");
        assert_eq!(e.at, "claim");
        let e = e.at("other");
        assert_eq!(e.at, "claim");
    }
}
