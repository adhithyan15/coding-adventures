//! `TypingTier` — the three-way typing classification for an IIR module.
//!
//! # The optional-typing spectrum
//!
//! InterpreterIR's `type_hint` field on each `IIRInstr` can be either a
//! **concrete type** (e.g. `"u8"`, `"bool"`) or the sentinel `"any"`.
//! This gives a continuous spectrum:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                  Typing spectrum                                │
//! │                                                                 │
//! │  Untyped ──────────────────────────────────── FullyTyped       │
//! │  (all "any")    Partial(0.3)    Partial(0.9)  (all concrete)   │
//! │                                                                 │
//! │  Typical:       Typical:        Typical:      Typical:         │
//! │  MRI Ruby,      TypeScript,     Hack/PHP,     Tetrad, Algol,   │
//! │  Twig, Lisp     Sorbet-Ruby     Dart, Swift   Oct, C           │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The tier drives compilation strategy:
//!
//! | Tier | AOT strategy | JIT strategy |
//! |------|-------------|--------------|
//! | `Untyped` | Link full `vm-runtime`; speculate from profile | Wait for profiling; speculate aggressively |
//! | `Partial` | AOT-compile typed functions; interpret untyped | JIT hot paths; defer untyped to interpreter |
//! | `FullyTyped` | AOT-compile everything; no `vm-runtime` link needed | Skip warmup; specialise immediately |

/// The typing tier of an `IIRModule`.
///
/// Calculated by [`crate::check::measure_tier`] after scanning all
/// `IIRInstr` objects in a module.
///
/// # Display
///
/// ```
/// use iir_type_checker::tier::TypingTier;
///
/// assert_eq!(TypingTier::Untyped.to_string(), "Untyped");
/// assert_eq!(TypingTier::Partial(0.5).to_string(), "Partial(50%)");
/// assert_eq!(TypingTier::FullyTyped.to_string(), "FullyTyped");
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum TypingTier {
    /// All instructions carry `type_hint = "any"`.
    ///
    /// Profile-guided speculation is the only path to specialised code.
    /// At least one observation cycle is required before AOT/JIT can
    /// emit typed CIR for any instruction.
    Untyped,

    /// Some instructions carry concrete type hints.
    ///
    /// The `f32` payload is the fraction of **data-flow** instructions
    /// (i.e. instructions with a destination register) that carry a
    /// concrete `type_hint`.  Control-flow instructions (`jmp`,
    /// `jmp_if_*`, `label`, `ret_void`) are excluded from the count
    /// because they never produce a typed value.
    ///
    /// - `0.0 < fraction < 1.0` always (boundary cases use `Untyped`
    ///   and `FullyTyped`).
    /// - Rounded to two decimal places for stable equality in tests.
    Partial(f32),

    /// Every data-flow instruction carries a concrete `type_hint`.
    ///
    /// No profiling is needed; AOT can compile the whole module without
    /// linking `vm-runtime`.
    FullyTyped,
}

impl TypingTier {
    /// Classify a pre-computed `typed_fraction`.
    ///
    /// ```
    /// use iir_type_checker::tier::TypingTier;
    ///
    /// assert_eq!(TypingTier::from_fraction(0.0), TypingTier::Untyped);
    /// assert_eq!(TypingTier::from_fraction(1.0), TypingTier::FullyTyped);
    /// let t = TypingTier::from_fraction(0.6);
    /// assert!(matches!(t, TypingTier::Partial(_)));
    /// ```
    pub fn from_fraction(fraction: f32) -> Self {
        if fraction <= 0.0 {
            TypingTier::Untyped
        } else if fraction >= 1.0 {
            TypingTier::FullyTyped
        } else {
            // Round to two decimal places so tests comparing fractions
            // are stable across floating-point arithmetic.
            let rounded = (fraction * 100.0).round() / 100.0;
            TypingTier::Partial(rounded)
        }
    }

    /// Return `true` when AOT can compile every function without
    /// relying on the runtime interpreter as a fallback.
    ///
    /// ```
    /// use iir_type_checker::tier::TypingTier;
    ///
    /// assert!(TypingTier::FullyTyped.is_fully_typed());
    /// assert!(!TypingTier::Untyped.is_fully_typed());
    /// assert!(!TypingTier::Partial(0.9).is_fully_typed());
    /// ```
    pub fn is_fully_typed(&self) -> bool {
        matches!(self, TypingTier::FullyTyped)
    }

    /// Return `true` when no static type information is available at all.
    ///
    /// ```
    /// use iir_type_checker::tier::TypingTier;
    ///
    /// assert!(TypingTier::Untyped.is_untyped());
    /// assert!(!TypingTier::Partial(0.1).is_untyped());
    /// assert!(!TypingTier::FullyTyped.is_untyped());
    /// ```
    pub fn is_untyped(&self) -> bool {
        matches!(self, TypingTier::Untyped)
    }

    /// Return the typed fraction in `[0.0, 1.0]`.
    ///
    /// ```
    /// use iir_type_checker::tier::TypingTier;
    ///
    /// assert_eq!(TypingTier::Untyped.typed_fraction(), 0.0);
    /// assert_eq!(TypingTier::FullyTyped.typed_fraction(), 1.0);
    /// assert!((TypingTier::Partial(0.4).typed_fraction() - 0.4).abs() < 1e-4);
    /// ```
    pub fn typed_fraction(&self) -> f32 {
        match self {
            TypingTier::Untyped => 0.0,
            TypingTier::FullyTyped => 1.0,
            TypingTier::Partial(f) => *f,
        }
    }
}

impl std::fmt::Display for TypingTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypingTier::Untyped => write!(f, "Untyped"),
            TypingTier::FullyTyped => write!(f, "FullyTyped"),
            TypingTier::Partial(frac) => write!(f, "Partial({:.0}%)", frac * 100.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_fraction_zero_is_untyped() {
        assert_eq!(TypingTier::from_fraction(0.0), TypingTier::Untyped);
    }

    #[test]
    fn from_fraction_one_is_fully_typed() {
        assert_eq!(TypingTier::from_fraction(1.0), TypingTier::FullyTyped);
    }

    #[test]
    fn from_fraction_negative_clamps_to_untyped() {
        assert_eq!(TypingTier::from_fraction(-0.5), TypingTier::Untyped);
    }

    #[test]
    fn from_fraction_over_one_clamps_to_fully_typed() {
        assert_eq!(TypingTier::from_fraction(1.5), TypingTier::FullyTyped);
    }

    #[test]
    fn partial_rounds_to_two_decimals() {
        let t = TypingTier::from_fraction(0.666_666);
        assert_eq!(t, TypingTier::Partial(0.67));
    }

    #[test]
    fn display_untyped() {
        assert_eq!(TypingTier::Untyped.to_string(), "Untyped");
    }

    #[test]
    fn display_fully_typed() {
        assert_eq!(TypingTier::FullyTyped.to_string(), "FullyTyped");
    }

    #[test]
    fn display_partial() {
        assert_eq!(TypingTier::Partial(0.5).to_string(), "Partial(50%)");
    }

    #[test]
    fn typed_fraction_boundaries() {
        assert_eq!(TypingTier::Untyped.typed_fraction(), 0.0);
        assert_eq!(TypingTier::FullyTyped.typed_fraction(), 1.0);
        assert!((TypingTier::Partial(0.3).typed_fraction() - 0.3).abs() < 1e-4);
    }

    #[test]
    fn is_fully_typed_only_for_fullytyped() {
        assert!(TypingTier::FullyTyped.is_fully_typed());
        assert!(!TypingTier::Untyped.is_fully_typed());
        assert!(!TypingTier::Partial(0.99).is_fully_typed());
    }

    #[test]
    fn is_untyped_only_for_untyped() {
        assert!(TypingTier::Untyped.is_untyped());
        assert!(!TypingTier::FullyTyped.is_untyped());
        assert!(!TypingTier::Partial(0.01).is_untyped());
    }
}
