//! # `JitPromotionThreshold` — per-typing-tier JIT call-count thresholds.
//!
//! LANG22 §"JIT compilation pipeline" specifies:
//!
//! | Tier | `FullyTyped` | `PartiallyTyped` | `Untyped` |
//! |------|-------------|-------------------|-----------|
//! | Threshold (calls) | 0 (before first call) | 10 | 100 |
//!
//! The rationale:
//!
//! - **FullyTyped (threshold = 0)**: every type is known statically at
//!   compile time.  The JIT doesn't need to observe anything — it can
//!   emit specialised native code immediately, before the function ever
//!   executes in the interpreter.
//!
//! - **PartiallyTyped (threshold = 10)**: a handful of interpreter runs
//!   is enough to confirm the observed types match the declared types and
//!   to fill in the `"any"` slots.  Ten calls is a conservative minimum
//!   that avoids premature specialisation on atypical input.
//!
//! - **Untyped (threshold = 100)**: the profiler needs more observations
//!   to build confidence.  A function called only a few times is "cold"
//!   by definition and not worth JIT-compiling.  100 calls provides
//!   enough signal to distinguish monomorphic from polymorphic behaviour
//!   with high accuracy (V8 uses a similar value of 64–128).
//!
//! ## Partial-tier interpolation
//!
//! For `Partial(fraction)` tiers the threshold is **linearly interpolated**
//! between the boundaries:
//!
//! ```text
//! threshold = round(10 + (100 - 10) × (1.0 - fraction))
//!           = round(10 + 90 × (1.0 - fraction))
//! ```
//!
//! A `Partial(1.0)` function (effectively fully typed) gets threshold 10
//! rather than 0 because the JIT still needs a few observations to confirm
//! the partial annotations are consistent.  A `Partial(0.0)` function (just
//! barely not `Untyped`) gets threshold 100.
//!
//! ```
//! use typing_spectrum::threshold::JitPromotionThreshold;
//! use iir_type_checker::tier::TypingTier;
//!
//! assert_eq!(JitPromotionThreshold::for_tier(&TypingTier::FullyTyped).call_count, 0);
//! assert_eq!(JitPromotionThreshold::for_tier(&TypingTier::Partial(1.0)).call_count, 10);
//! assert_eq!(JitPromotionThreshold::for_tier(&TypingTier::Partial(0.0)).call_count, 100);
//! assert_eq!(JitPromotionThreshold::for_tier(&TypingTier::Untyped).call_count, 100);
//! ```

use iir_type_checker::tier::TypingTier;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Call-count threshold for [`TypingTier::FullyTyped`] functions.
///
/// Zero means "compile before the first interpreted call" — the JIT precompiles
/// the function lazily on first-touch (call_count was 0 at dispatch time).
pub const THRESHOLD_FULLY_TYPED: u32 = 0;

/// Call-count threshold for the anchor of the [`TypingTier::Partial`] range
/// (fraction = 1.0, effectively fully typed but annotated partially).
pub const THRESHOLD_PARTIAL_HI: u32 = 10;

/// Call-count threshold for the anchor of the [`TypingTier::Partial`] range
/// (fraction = 0.0, effectively untyped but annotated partially) and for
/// [`TypingTier::Untyped`].
pub const THRESHOLD_UNTYPED: u32 = 100;

// ---------------------------------------------------------------------------
// JitPromotionThreshold
// ---------------------------------------------------------------------------

/// The call count at which the JIT should promote a function from the
/// interpreter tier to native code.
///
/// Obtain via [`JitPromotionThreshold::for_tier`].  The `call_count` field
/// is the raw threshold; comparison is `>=` (promote when the function has
/// been called at least this many times).
///
/// # Example
///
/// ```
/// use typing_spectrum::threshold::JitPromotionThreshold;
/// use iir_type_checker::tier::TypingTier;
///
/// let t = JitPromotionThreshold::for_tier(&TypingTier::Untyped);
/// assert!(t.should_promote(100));
/// assert!(!t.should_promote(99));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JitPromotionThreshold {
    /// The minimum call count required before the JIT promotes this function.
    ///
    /// - `0` → promote immediately on first dispatch (no interpreter run).
    /// - `N > 0` → wait until the interpreter has executed the function `N` times.
    pub call_count: u32,
}

impl JitPromotionThreshold {
    /// Compute the promotion threshold for the given [`TypingTier`].
    ///
    /// See the [module-level docs](self) for the interpolation formula.
    ///
    /// ```
    /// use typing_spectrum::threshold::JitPromotionThreshold;
    /// use iir_type_checker::tier::TypingTier;
    ///
    /// assert_eq!(JitPromotionThreshold::for_tier(&TypingTier::FullyTyped).call_count, 0);
    /// assert_eq!(JitPromotionThreshold::for_tier(&TypingTier::Partial(0.5)).call_count, 55);
    /// assert_eq!(JitPromotionThreshold::for_tier(&TypingTier::Untyped).call_count, 100);
    /// ```
    pub fn for_tier(tier: &TypingTier) -> Self {
        let call_count = match tier {
            TypingTier::FullyTyped => THRESHOLD_FULLY_TYPED,
            TypingTier::Untyped   => THRESHOLD_UNTYPED,
            TypingTier::Partial(fraction) => {
                // Linear interpolation: 10 at fraction=1.0, 100 at fraction=0.0.
                // threshold = round(10 + 90 × (1.0 - fraction))
                let frac = fraction.clamp(0.0, 1.0) as f64;
                let raw = THRESHOLD_PARTIAL_HI as f64 + 90.0 * (1.0 - frac);
                raw.round() as u32
            }
        };
        JitPromotionThreshold { call_count }
    }

    /// Return `true` when the given call count has reached or exceeded the
    /// promotion threshold.
    ///
    /// For a threshold of 0, this is always `true` (the function is promoted
    /// before its first interpreted call).
    ///
    /// ```
    /// use typing_spectrum::threshold::JitPromotionThreshold;
    ///
    /// let t = JitPromotionThreshold { call_count: 10 };
    /// assert!(t.should_promote(10));
    /// assert!(t.should_promote(100));
    /// assert!(!t.should_promote(9));
    ///
    /// // threshold = 0: always ready
    /// let eager = JitPromotionThreshold { call_count: 0 };
    /// assert!(eager.should_promote(0));
    /// ```
    pub fn should_promote(&self, actual_call_count: u32) -> bool {
        actual_call_count >= self.call_count
    }

    /// Human-readable label for use in advisory reports.
    ///
    /// ```
    /// use typing_spectrum::threshold::JitPromotionThreshold;
    ///
    /// assert_eq!(JitPromotionThreshold { call_count: 0 }.label(), "compile-before-first-call");
    /// assert_eq!(JitPromotionThreshold { call_count: 10 }.label(), "after-10-calls");
    /// assert_eq!(JitPromotionThreshold { call_count: 100 }.label(), "after-100-calls");
    /// ```
    pub fn label(&self) -> String {
        if self.call_count == 0 {
            "compile-before-first-call".to_string()
        } else {
            format!("after-{}-calls", self.call_count)
        }
    }
}

impl std::fmt::Display for JitPromotionThreshold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fully_typed_threshold_is_zero() {
        let t = JitPromotionThreshold::for_tier(&TypingTier::FullyTyped);
        assert_eq!(t.call_count, THRESHOLD_FULLY_TYPED);
    }

    #[test]
    fn untyped_threshold_is_hundred() {
        let t = JitPromotionThreshold::for_tier(&TypingTier::Untyped);
        assert_eq!(t.call_count, THRESHOLD_UNTYPED);
    }

    #[test]
    fn partial_at_one_is_ten() {
        let t = JitPromotionThreshold::for_tier(&TypingTier::Partial(1.0));
        assert_eq!(t.call_count, 10);
    }

    #[test]
    fn partial_at_zero_is_hundred() {
        let t = JitPromotionThreshold::for_tier(&TypingTier::Partial(0.0));
        assert_eq!(t.call_count, 100);
    }

    #[test]
    fn partial_at_half_is_fifty_five() {
        let t = JitPromotionThreshold::for_tier(&TypingTier::Partial(0.5));
        // 10 + 90 × 0.5 = 55
        assert_eq!(t.call_count, 55);
    }

    #[test]
    fn partial_interpolates_monotonically() {
        // Higher typed fraction → lower threshold.
        let fracs = [0.0f32, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0];
        let mut prev = u32::MAX;
        for &f in &fracs {
            let t = JitPromotionThreshold::for_tier(&TypingTier::Partial(f));
            assert!(t.call_count <= prev,
                "threshold non-monotonic at fraction={f}: {} > {prev}", t.call_count);
            prev = t.call_count;
        }
    }

    #[test]
    fn should_promote_at_exactly_threshold() {
        let t = JitPromotionThreshold { call_count: 10 };
        assert!(t.should_promote(10));
        assert!(!t.should_promote(9));
    }

    #[test]
    fn should_promote_zero_threshold_always_true() {
        let t = JitPromotionThreshold { call_count: 0 };
        assert!(t.should_promote(0));
        assert!(t.should_promote(1_000_000));
    }

    #[test]
    fn label_matches_expected_strings() {
        assert_eq!(
            JitPromotionThreshold { call_count: 0 }.label(),
            "compile-before-first-call"
        );
        assert_eq!(
            JitPromotionThreshold { call_count: 10 }.label(),
            "after-10-calls"
        );
        assert_eq!(
            JitPromotionThreshold { call_count: 100 }.label(),
            "after-100-calls"
        );
    }
}
