//! # Resource bounds — the part that makes a preprocessor safe to point at hostile input.
//!
//! A preprocessor reads files chosen by the source it is preprocessing. That
//! makes it the one component in a compiler that an attacker can steer, so
//! every loop in the engine needs a finite budget.
//!
//! The organising principle of [`Bounds`] — and the thing most hand-written
//! preprocessors get wrong — is **bound work and bytes, not only shape.**
//! Counting nesting depth and emitted tokens feels thorough, and leaves three
//! doors wide open:
//!
//! ```text
//!   shape-only bound          what walks through it
//!   ─────────────────────     ──────────────────────────────────────────────
//!   "emitted tokens ≤ N"      expansion CONSUMED by a conditional is never
//!                             emitted.  A 40-deep doubling chain evaluated
//!                             inside `@if` makes 2^40 tokens the cap never sees.
//!
//!   "token count ≤ N"         stringize and paste grow BYTES while holding
//!                             the token count flat — one token can carry a
//!                             megabyte of text.
//!
//!   "include depth ≤ N"       a DAG bomb is not deep and is not a cycle:
//!   + "no cycles"             ten includes fanning out eight levels is 10^8
//!                             file reads at depth 8 with no repeat on the stack.
//! ```
//!
//! So the counters below are cumulative totals over the whole translation
//! unit, not per-nesting-level snapshots.
//!
//! ## Tighten-only
//!
//! Every field has a finite default and [`Bounds::tighten`] is the only way to
//! change one. A dialect *or an embedding host* may lower a bound; neither can
//! raise one or disable it. A configuration surface that can set a budget to
//! infinity is a configuration surface that will eventually be set to infinity.
//!
//! ## Fuel is the one that matters most
//!
//! Every other field bounds a dimension somebody thought of. [`Bounds::fuel`]
//! bounds the ones nobody thought of: it is decremented by every token copied,
//! every rescan, every file read. Preprocessor denial-of-service historically
//! arrives through whichever dimension was not enumerated, and no list —
//! including this one — is complete.

/// Finite, tighten-only resource budgets for one translation unit.
///
/// Construct with [`Bounds::default`] and narrow with [`Bounds::tighten`].
/// There is deliberately no way to widen one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    /// Maximum depth of the *active* include stack.
    ///
    /// C requires at least 15 nested includes to work at all, so this cannot
    /// be tiny. Depth alone stops a straight chain, not a fan-out.
    pub include_depth: u32,

    /// Maximum number of file inclusions over the whole translation unit.
    ///
    /// This is the one that stops a DAG bomb. Cycle detection is stack-based
    /// (see [`crate::engine`]) because a header may legitimately be included
    /// many times, so "not a cycle and not deep" is a reachable state with
    /// exponential cost. Until `@pragma once`-style deduplication exists, this
    /// counter is what holds the engine up.
    pub total_inclusions: u32,

    /// Maximum total bytes of source read, across every file.
    pub total_source_bytes: u64,

    /// Maximum bytes in any single included file, checked from the opened
    /// handle's metadata *before* the read, so an enormous file is refused
    /// rather than read and then rejected.
    pub bytes_per_file: u64,

    /// Maximum depth of macro expansion. (Unused until macros land, but
    /// declared here so the budget surface is stable.)
    pub macro_depth: u32,

    /// Maximum tokens **produced** — emitted, consumed by a conditional, or
    /// discarded. Not "emitted": see the module header.
    pub tokens_produced: u64,

    /// Maximum length of a single token's spelling.
    ///
    /// Guards the byte-growth channel that a token counter cannot see.
    pub token_spelling_bytes: u64,

    /// Maximum total bytes of token text the engine synthesises (as opposed to
    /// copying from source).
    pub synthesised_text_bytes: u64,

    /// Maximum grouping nesting inside a macro argument list.
    pub arg_group_depth: u32,

    /// Maximum grouping nesting inside a conditional's controlling expression.
    ///
    /// The engine pre-scans for this *before* handing the slice to a dialect,
    /// rather than trusting each dialect to police itself — see
    /// [`crate::dialect::Dialect::eval_condition`].
    pub condition_depth: u32,

    /// Maximum nesting of conditional groups.
    pub conditional_depth: u32,

    /// Total work budget. Decremented by every token copied, every rescan and
    /// every file read. The catch-all for dimensions not enumerated above.
    pub fuel: u64,

    /// Maximum number of diagnostics before the engine stops and reports a
    /// summary. A cascade that emits one diagnostic per token is itself a
    /// denial-of-service vector.
    pub max_diagnostics: u32,

    /// Maximum source text quoted into any one diagnostic.
    ///
    /// Distinct from the no-silent-truncation rule on token streams: truncating
    /// a *message* is fine, truncating a *program* is not.
    pub diagnostic_quote_bytes: u32,
}

impl Default for Bounds {
    fn default() -> Self {
        Bounds {
            include_depth: 200,
            total_inclusions: 10_000,
            total_source_bytes: 256 * 1024 * 1024,
            bytes_per_file: 16 * 1024 * 1024,
            macro_depth: 200,
            tokens_produced: 64_000_000,
            token_spelling_bytes: 64 * 1024,
            synthesised_text_bytes: 64 * 1024 * 1024,
            arg_group_depth: 200,
            condition_depth: 200,
            conditional_depth: 200,
            fuel: 1 << 30,
            max_diagnostics: 100,
            diagnostic_quote_bytes: 1024,
        }
    }
}

impl Bounds {
    /// Narrow this budget by taking the minimum of each field.
    ///
    /// This is the only mutator, and it is why no caller can widen a bound:
    /// `tighten` is a pointwise `min`, so the result is never larger than
    /// either input in any dimension. A host that wants "unlimited" cannot
    /// express it.
    #[must_use]
    pub fn tighten(self, other: Bounds) -> Bounds {
        Bounds {
            include_depth: self.include_depth.min(other.include_depth),
            total_inclusions: self.total_inclusions.min(other.total_inclusions),
            total_source_bytes: self.total_source_bytes.min(other.total_source_bytes),
            bytes_per_file: self.bytes_per_file.min(other.bytes_per_file),
            macro_depth: self.macro_depth.min(other.macro_depth),
            tokens_produced: self.tokens_produced.min(other.tokens_produced),
            token_spelling_bytes: self.token_spelling_bytes.min(other.token_spelling_bytes),
            synthesised_text_bytes: self.synthesised_text_bytes.min(other.synthesised_text_bytes),
            arg_group_depth: self.arg_group_depth.min(other.arg_group_depth),
            condition_depth: self.condition_depth.min(other.condition_depth),
            conditional_depth: self.conditional_depth.min(other.conditional_depth),
            fuel: self.fuel.min(other.fuel),
            max_diagnostics: self.max_diagnostics.min(other.max_diagnostics),
            diagnostic_quote_bytes: self.diagnostic_quote_bytes.min(other.diagnostic_quote_bytes),
        }
    }
}

/// A running tally checked against [`Bounds`] as the engine works.
///
/// Separate from `Bounds` so the budget stays immutable and the spend is
/// obviously the mutable part.
#[derive(Debug, Default, Clone)]
pub struct Spend {
    pub inclusions: u32,
    pub source_bytes: u64,
    pub tokens_produced: u64,
    pub synthesised_text_bytes: u64,
    pub fuel_used: u64,
    pub diagnostics: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_default_is_finite_and_nonzero() {
        // The point of this test is not the specific numbers -- it is that
        // nobody can land a bound of 0 (which would refuse all input) or leave
        // one conceptually "unlimited" by maxing it out.
        let b = Bounds::default();
        assert!(b.include_depth > 0 && b.include_depth < u32::MAX);
        assert!(b.total_inclusions > 0 && b.total_inclusions < u32::MAX);
        assert!(b.total_source_bytes > 0 && b.total_source_bytes < u64::MAX);
        assert!(b.bytes_per_file > 0 && b.bytes_per_file < u64::MAX);
        assert!(b.tokens_produced > 0 && b.tokens_produced < u64::MAX);
        assert!(b.token_spelling_bytes > 0 && b.token_spelling_bytes < u64::MAX);
        assert!(b.synthesised_text_bytes > 0 && b.synthesised_text_bytes < u64::MAX);
        assert!(b.fuel > 0 && b.fuel < u64::MAX);
        assert!(b.max_diagnostics > 0);
        assert!(b.diagnostic_quote_bytes > 0);
    }

    #[test]
    fn c_requires_at_least_fifteen_nested_includes() {
        // Not an arbitrary number: a preprocessor that cannot nest 15 deep
        // cannot compile conforming C, so this bound has a real floor.
        assert!(Bounds::default().include_depth >= 15);
    }

    #[test]
    fn tighten_narrows_and_cannot_widen() {
        let base = Bounds::default();
        let narrow = Bounds { fuel: 10, include_depth: 3, ..Bounds::default() };

        let t = base.tighten(narrow);
        assert_eq!(t.fuel, 10);
        assert_eq!(t.include_depth, 3);

        // The interesting direction: asking for MORE than the default gets you
        // the default, not the request.
        let greedy = Bounds { fuel: u64::MAX, include_depth: u32::MAX, ..Bounds::default() };
        let t2 = base.tighten(greedy);
        assert_eq!(t2.fuel, base.fuel);
        assert_eq!(t2.include_depth, base.include_depth);
    }

    #[test]
    fn tighten_is_order_independent() {
        // Pointwise min is commutative, so a host and a dialect narrowing the
        // same budget cannot depend on which one ran first.
        let a = Bounds { fuel: 500, conditional_depth: 9, ..Bounds::default() };
        let b = Bounds { fuel: 700, conditional_depth: 4, ..Bounds::default() };
        assert_eq!(a.tighten(b), b.tighten(a));
    }
}
