#![forbid(unsafe_code)]
//! # `fsrs` — a zero-dependency, forward-only FSRS-6 scheduler
//!
//! FSRS ("Free Spaced Repetition Scheduler") is the algorithm modern Anki uses
//! to decide *when* you should next see a flashcard. It models your memory of a
//! card with two numbers:
//!
//! - **stability (`S`)** — how many days until your chance of recalling the card
//!   falls to 90%. Bigger `S` = the memory lasts longer.
//! - **difficulty (`D`)** — how hard the card is for you, on a 1–10 scale.
//!   Harder cards grow their stability more slowly.
//!
//! Every time you review a card you press one of four buttons — **Again (1),
//! Hard (2), Good (3), Easy (4)**. FSRS takes your *current* `(S, D)`, how many
//! days have elapsed since the last review, and the button you pressed, and
//! produces a *new* `(S, D)` plus the number of days to wait before the next
//! review. That whole computation is the "forward pass", and it is all this
//! crate does.
//!
//! ## Why this crate exists
//!
//! The upstream [`fsrs`](https://crates.io/crates/fsrs) crate is excellent, but
//! it pulls in the `burn` tensor framework (and, transitively, dozens of other
//! crates) because it *also* implements **training** — fitting the 21 model
//! parameters to your personal review history via gradient descent. Engram
//! never trains; it only schedules. The scheduling path in the upstream crate is
//! pure scalar `f32` arithmetic — no tensors — so we reimplement exactly that
//! path here, from scratch, with **no third-party dependencies**, to honour the
//! repository's zero-dependency policy.
//!
//! The formulas, constants, and *order of operations* below are transcribed
//! faithfully from upstream `fsrs` 6.6.1 (`model.rs` + `inference.rs`), so this
//! crate reproduces its `next_states` / `memory_state_from_sm2` /
//! `current_retrievability` outputs bit-for-bit. Before the upstream crate was
//! dropped, a throwaway cross-check asserted exactly that across 5,900+
//! comparisons against the live crate; the exact upstream outputs for
//! representative cases are frozen as the unit-test snapshots below.
//!
//! ## What is *not* here
//!
//! Training, the optimizer, batch/tensor code, evaluation metrics, and the
//! simulation harness. Those are the parts that need `burn`. If Engram ever
//! wants to *train* parameters it would be a separate, opt-in concern.

/// The number of tunable weights in an FSRS-6 parameter set.
///
/// FSRS-4/5 used fewer (17 and 19 respectively); FSRS-6 added a learnable decay
/// term, bringing the total to 21. [`check_and_fill_parameters`] upgrades the
/// shorter legacy sets to this length.
pub const PARAMETER_COUNT: usize = 21;

/// The decay exponent FSRS-5 used for its forgetting curve (a fixed constant,
/// since FSRS-5 did not learn it).
pub const FSRS5_DEFAULT_DECAY: f32 = 0.5;

/// The *default* decay exponent for FSRS-6. Unlike FSRS-5 this is a learnable
/// weight (`w[20]`), but this constant is the starting/default value and is also
/// what callers pass to [`current_retrievability`] when they have no better
/// estimate.
pub const FSRS6_DEFAULT_DECAY: f32 = 0.1542;

/// The 21 default FSRS-6 weights, as shipped by upstream `fsrs` 6.6.1.
///
/// A brand-new deck with no personalised training uses exactly these. Index 20
/// is the decay term and equals [`FSRS6_DEFAULT_DECAY`].
pub static DEFAULT_PARAMETERS: [f32; PARAMETER_COUNT] = [
    0.212,
    1.2931,
    2.3065,
    8.2956,
    6.4133,
    0.8334,
    3.0194,
    0.001,
    1.8722,
    0.1666,
    0.796,
    1.4835,
    0.0614,
    0.2629,
    1.6483,
    0.6014,
    1.8729,
    0.5425,
    0.0912,
    0.0658,
    FSRS6_DEFAULT_DECAY,
];

// ---------------------------------------------------------------------------
// Clamp bounds. Stability and difficulty are kept inside sane physical ranges
// at every step so a pathological parameter set can never send them to
// infinity or negative values.
// ---------------------------------------------------------------------------

/// Smallest allowed stability: ~1.4 minutes expressed in days. Stability can
/// never be zero (it sits in a denominator).
const S_MIN: f32 = 0.001;
/// Largest allowed stability: 100 years in days. Beyond this the exact value is
/// irrelevant — the card is effectively "known forever".
const S_MAX: f32 = 36500.0;
/// Difficulty floor (easiest possible card).
const D_MIN: f32 = 1.0;
/// Difficulty ceiling (hardest possible card).
const D_MAX: f32 = 10.0;
/// Upper bound applied to the four *initial* stability weights when clipping a
/// parameter set. Distinct from `S_MAX` because a *fresh* card should not start
/// out believed-known-for-a-century.
const INIT_S_MAX: f32 = 100.0;

/// Errors from constructing an [`FSRS`] scheduler or requesting a state.
///
/// Deliberately tiny — the forward pass has only two ways to fail: a
/// wrong-length / non-finite parameter set, and inputs that push the arithmetic
/// to a non-finite result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsrsError {
    /// The supplied parameter slice was not a valid length (0/17/19/21) or
    /// contained a non-finite (`NaN`/`inf`) weight.
    InvalidParameters,
    /// A requested state came out non-finite (e.g. `memory_state_from_sm2`
    /// given degenerate inputs).
    InvalidInput,
}

/// A card's memory state: the `(stability, difficulty)` pair FSRS tracks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryState {
    /// Days until predicted recall probability drops to 90%.
    pub stability: f32,
    /// Card difficulty on a 1.0 (easy) – 10.0 (hard) scale.
    pub difficulty: f32,
}

/// The outcome of pressing one button: the resulting [`MemoryState`] and the
/// interval (in days) FSRS would schedule for it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemState {
    /// The memory state after this rating is applied.
    pub memory: MemoryState,
    /// Days to wait before showing the card again.
    pub interval: f32,
}

/// The four [`ItemState`]s, one per answer button, produced by
/// [`FSRS::next_states`]. The scheduler picks whichever field matches the button
/// the user actually pressed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NextStates {
    /// Result of pressing **Again** (rating 1).
    pub again: ItemState,
    /// Result of pressing **Hard** (rating 2).
    pub hard: ItemState,
    /// Result of pressing **Good** (rating 3).
    pub good: ItemState,
    /// Result of pressing **Easy** (rating 4).
    pub easy: ItemState,
}

// ===========================================================================
// The forgetting curve and interval maths.
// ===========================================================================

/// The **power forgetting curve** `R(t)` — probability you still recall a card
/// `t` days after review, given stability `s`.
///
/// FSRS-6 uses a *power law* rather than the classic exponential `e^{-t/s}`,
/// because empirically memory decays more slowly than an exponential in the long
/// tail. The shape is `R(t) = (1 + factor · t/s)^decay`, where `decay = -w[20]`
/// is negative (so larger `t` ⇒ smaller `R`), and `factor` is chosen so that
/// `R(s) = 0.9` exactly — i.e. by definition you have a 90% chance of recall
/// after exactly `s` days.
///
/// ```text
/// R(0) = 1            (just reviewed — certain recall)
/// R(s) = 0.9          (stability is the 90%-recall horizon, by construction)
/// R(t) → 0 as t → ∞   (everything is eventually forgotten)
/// ```
#[inline]
fn power_forgetting_curve(w: &[f32], t: f32, s: f32) -> f32 {
    let decay = -w[20];
    let factor = (0.9f32.ln() / decay).exp() - 1.0;
    (t / s * factor + 1.0).powf(decay)
}

/// Invert the forgetting curve: given a stability `s` and a *desired retention*
/// `r` (the recall probability you want to maintain, e.g. 0.90), return how many
/// days to wait so that `R(interval) = r`.
///
/// This is the number the scheduler ultimately cares about — "when do I show
/// this card again?" Higher desired retention ⇒ shorter interval (you review
/// more often to keep recall high).
#[inline]
fn next_interval(w: &[f32], stability: f32, desired_retention: f32) -> f32 {
    let decay = -w[20];
    let factor = (0.9f32.ln() / decay).exp() - 1.0;
    stability / factor * (desired_retention.powf(1.0 / decay) - 1.0)
}

// ===========================================================================
// Initial state for a brand-new card (first-ever review).
// ===========================================================================

/// Initial stability for a never-before-seen card, chosen by the first button
/// pressed. `w[0..=3]` are the four initial-stability weights for
/// Again/Hard/Good/Easy; a harder first rating ⇒ smaller starting stability.
///
/// `rating` is 1-based; we map it to `w[rating-1]`, clamped into `0..=3`.
#[inline]
fn init_stability(w: &[f32], rating: usize) -> f32 {
    w[rating.saturating_sub(1).min(3)]
}

/// Initial difficulty for a new card. `w[4]` anchors the Again-difficulty and
/// `w[5]` controls how much *easier* higher ratings start out:
/// `D0(rating) = w[4] − e^{w[5]·(rating−1)} + 1`.
///
/// Passing the sentinel `rating = 4` here also yields the "target" difficulty
/// used by [`mean_reversion`].
#[inline]
fn init_difficulty(w: &[f32], rating: usize) -> f32 {
    w[4] - (w[5] * rating.saturating_sub(1) as f32).exp() + 1.0
}

// ===========================================================================
// Difficulty update.
// ===========================================================================

/// Pull a freshly-computed difficulty back toward the "easy anchor"
/// `init_difficulty(4)`. Without this, difficulties would ratchet ever upward;
/// mean reversion (weight `w[7]`) keeps the population of cards from all drifting
/// to maximum difficulty over time.
#[inline]
fn mean_reversion(w: &[f32], new_d: f32) -> f32 {
    w[7] * (init_difficulty(w, 4) - new_d) + new_d
}

/// Scale a raw difficulty change by how much head-room is left before `D_MAX`.
///
/// Near `D_MAX` a card can barely get harder (the factor → 0); near `D_MIN` the
/// full change applies. This "linear damping" is what makes difficulty saturate
/// smoothly instead of slamming into the ceiling.
#[inline]
fn linear_damping(delta_d: f32, old_d: f32) -> f32 {
    (10.0 - old_d) * delta_d / 9.0
}

/// New difficulty after a review. Pressing an *easier* button (higher rating)
/// lowers difficulty and vice-versa: `Δ = −w[6]·(rating−3)`, damped by how close
/// we already are to the ceiling.
#[inline]
fn next_difficulty(w: &[f32], difficulty: f32, rating: f32) -> f32 {
    let delta_d = -w[6] * (rating - 3.0);
    difficulty + linear_damping(delta_d, difficulty)
}

// ===========================================================================
// Stability update — three regimes.
// ===========================================================================

/// New stability after a **successful** recall (Hard/Good/Easy) that happened
/// after a real gap (`delta_t > 0`).
///
/// The multiplier rewards you more when: the card is easy (`11 − D` large),
/// stability is currently low (`S^{-w[9]}` — low-stability memories have more
/// room to grow), and the review was *hard-won* — recall probability `r` was low
/// so `(e^{(1−r)·w[10]} − 1)` is large (spacing effect: reviewing right before
/// you'd forget grows memory most). Hard applies a penalty `w[15]`, Easy a bonus
/// `w[16]`.
#[inline]
fn stability_after_success(w: &[f32], last_s: f32, last_d: f32, r: f32, rating: f32) -> f32 {
    let hard_penalty = if rating == 2.0 { w[15] } else { 1.0 };
    let easy_bonus = if rating == 4.0 { w[16] } else { 1.0 };
    last_s
        * (w[8].exp()
            * (11.0 - last_d)
            * last_s.powf(-w[9])
            * (((1.0 - r) * w[10]).exp() - 1.0)
            * hard_penalty
            * easy_bonus
            + 1.0)
}

/// New stability after a **lapse** (Again) following a real gap. A forgotten card
/// collapses to a much smaller "post-lapse stability", but never *above* its old
/// stability divided by `e^{w[17]·w[18]}` (the `new_s_min` cap) — forgetting
/// cannot leave you better off than a floor relative to where you were.
#[inline]
fn stability_after_failure(w: &[f32], last_s: f32, last_d: f32, r: f32) -> f32 {
    let new_s = w[11]
        * last_d.powf(-w[12])
        * ((last_s + 1.0).powf(w[13]) - 1.0)
        * ((1.0 - r) * w[14]).exp();
    let new_s_min = last_s / (w[17] * w[18]).exp();
    new_s.min(new_s_min)
}

/// New stability for a **same-day** re-review (`delta_t == 0`), e.g. clicking
/// through learning steps. There is no forgetting to exploit, so a distinct
/// "short-term" formula governed by `w[17..=19]` applies. For Hard/Good/Easy the
/// factor is floored at 1.0 (same-day success never *reduces* stability); for
/// Again it may reduce it.
#[inline]
fn stability_short_term(w: &[f32], last_s: f32, rating: f32) -> f32 {
    let sinc = (w[17] * (rating - 3.0 + w[18])).exp() * last_s.powf(-w[19]);
    last_s * if rating >= 2.0 { sinc.max(1.0) } else { sinc }
}

/// Advance a memory state by one review.
///
/// This is the heart of FSRS. Given the current `state`, the days elapsed
/// (`delta_t`), the button pressed (`rating`, 1–4), and `nth` (0 iff this is the
/// card's first-ever review), produce the next `(S, D)`.
///
/// The branching mirrors the three stability regimes plus the special cases:
/// - `delta_t == 0` → short-term (same-day) stability.
/// - first-ever review (`nth == 0 && S == 0`) → seed from `init_*`.
/// - `rating == 0` → a sentinel "no-op" review that leaves the state untouched.
fn step(w: &[f32], delta_t: f32, rating: f32, state: MemoryState, nth: usize) -> MemoryState {
    let last_s = state.stability.clamp(S_MIN, S_MAX);
    let last_d = state.difficulty.clamp(D_MIN, D_MAX);

    let retrievability = power_forgetting_curve(w, delta_t, last_s);
    let stability_after_success =
        stability_after_success(w, last_s, last_d, retrievability, rating);
    let stability_after_failure = stability_after_failure(w, last_s, last_d, retrievability);
    let stability_short_term = stability_short_term(w, last_s, rating);

    let mut new_s = if rating == 1.0 {
        stability_after_failure
    } else {
        stability_after_success
    };
    if delta_t == 0.0 {
        new_s = stability_short_term;
    }

    let mut new_d = next_difficulty(w, last_d, rating);
    new_d = mean_reversion(w, new_d).clamp(D_MIN, D_MAX);

    if nth == 0 && state.stability == 0.0 {
        let init_rating = (rating as u32).clamp(1, 4) as usize;
        new_s = init_stability(w, init_rating);
        new_d = init_difficulty(w, init_rating).clamp(D_MIN, D_MAX);
    }

    if rating == 0.0 {
        new_s = last_s;
        new_d = last_d;
    }

    MemoryState {
        stability: new_s.clamp(S_MIN, S_MAX),
        difficulty: new_d,
    }
}

/// Reject a state that has gone non-finite (defensive; the clamps in [`step`]
/// make this practically unreachable, but `memory_state_from_sm2` can produce
/// one from degenerate inputs).
fn validate_state(state: MemoryState) -> Result<MemoryState, FsrsError> {
    if !state.stability.is_finite() || !state.difficulty.is_finite() {
        Err(FsrsError::InvalidInput)
    } else {
        Ok(state)
    }
}

// ===========================================================================
// Parameter preparation: upgrade legacy sets and clip into valid ranges.
// ===========================================================================

/// Normalise a parameter slice to a full 21-weight FSRS-6 set.
///
/// Accepts the historical lengths and upgrades them:
/// - `0` → the [`DEFAULT_PARAMETERS`].
/// - `17` (FSRS-4) → convert the two short-term weights and append FSRS-5/6 tail.
/// - `19` (FSRS-5) → append the FSRS-6 decay pair.
/// - `21` (FSRS-6) → used as-is.
///
/// Any other length, or a non-finite weight, is [`FsrsError::InvalidParameters`].
fn check_and_fill_parameters(parameters: &[f32]) -> Result<Vec<f32>, FsrsError> {
    let parameters = match parameters.len() {
        0 => DEFAULT_PARAMETERS.to_vec(),
        17 => {
            let mut p = parameters.to_vec();
            // FSRS-4 → FSRS-5 conversion (transcribed from upstream).
            p[4] = p[5].mul_add(2.0, p[4]);
            p[5] = p[5].mul_add(3.0, 1.0).ln() / 3.0;
            p[6] += 0.5;
            p.extend_from_slice(&[0.0, 0.0, 0.0, FSRS5_DEFAULT_DECAY]);
            p
        }
        19 => {
            let mut p = parameters.to_vec();
            p.extend_from_slice(&[0.0, FSRS5_DEFAULT_DECAY]);
            p
        }
        21 => parameters.to_vec(),
        _ => return Err(FsrsError::InvalidParameters),
    };
    if parameters.iter().any(|w| !w.is_finite()) {
        return Err(FsrsError::InvalidParameters);
    }
    Ok(parameters)
}

/// Clamp every weight into the range that keeps the forward pass well-behaved.
///
/// Most bounds are fixed; the two short-term-stability ceilings (`w[17]`,
/// `w[18]`) depend on `num_relearning_steps` so that a lapse can never *increase*
/// stability once relearning steps are accounted for (see the derivation inline
/// upstream). Engram always constructs schedulers with `num_relearning_steps = 1`
/// and `enable_short_term = false`, which makes those ceilings a flat `2.0` and
/// the `w[19]` floor `0.0`.
fn clip_parameters_in_place(
    parameters: &mut [f32],
    num_relearning_steps: usize,
    enable_short_term: bool,
) {
    let w17_w18_ceiling = if num_relearning_steps > 1 {
        (-(parameters[11].ln() + (2.0f32.powf(parameters[13]) - 1.0).ln() + parameters[14] * 0.3)
            / num_relearning_steps as f32)
            .max(0.01)
            .sqrt()
            .min(2.0)
    } else {
        2.0
    };
    let w19_floor = if enable_short_term { 0.01 } else { 0.0 };
    let clamps: [(f32, f32); PARAMETER_COUNT] = [
        (S_MIN, INIT_S_MAX),
        (S_MIN, INIT_S_MAX),
        (S_MIN, INIT_S_MAX),
        (S_MIN, INIT_S_MAX),
        (D_MIN, D_MAX),
        (0.001, 4.0),
        (0.001, 4.0),
        (0.001, 0.75),
        (0.0, 4.5),
        (0.0, 0.8),
        (0.001, 3.5),
        (0.001, 5.0),
        (0.001, 0.25),
        (0.001, 0.9),
        (0.0, 4.0),
        (0.0, 1.0),
        (1.0, 6.0),
        (0.0, w17_w18_ceiling),
        (0.0, w17_w18_ceiling),
        (w19_floor, 0.8),
        (0.1, 0.8),
    ];
    for (w, (low, high)) in parameters.iter_mut().zip(clamps) {
        *w = w.clamp(low, high);
    }
}

/// Current recall probability for a stored memory state — the free function used
/// by search filters (`prop:r`) to rank how "due" cards are.
///
/// This is exactly [`power_forgetting_curve`] but written against an explicit
/// `decay` argument (rather than a parameter set), because the caller may be
/// scoring an *imported* card whose own decay differs from ours.
pub fn current_retrievability(state: MemoryState, days_elapsed: f32, decay: f32) -> f32 {
    let factor = 0.9f32.powf(1.0 / -decay) - 1.0;
    (days_elapsed / state.stability * factor + 1.0).powf(-decay)
}

// ===========================================================================
// The public scheduler.
// ===========================================================================

/// A ready-to-use FSRS-6 scheduler holding a validated, clipped 21-weight set.
///
/// Construct with [`FSRS::new`] (an empty slice uses [`DEFAULT_PARAMETERS`]),
/// then call [`FSRS::next_states`] for each review.
#[derive(Debug, Clone)]
pub struct FSRS {
    parameters: [f32; PARAMETER_COUNT],
}

impl FSRS {
    /// Build a scheduler from a parameter slice.
    ///
    /// The slice may be empty (⇒ defaults) or a legacy length; it is upgraded by
    /// [`check_and_fill_parameters`] and then clipped into valid ranges exactly
    /// as upstream does for a default `ModelConfig` (`num_relearning_steps = 1`,
    /// short-term stability disabled).
    pub fn new(parameters: &[f32]) -> Result<Self, FsrsError> {
        let mut parameters = check_and_fill_parameters(parameters)?;
        clip_parameters_in_place(&mut parameters, 1, false);
        let parameters: [f32; PARAMETER_COUNT] = parameters
            .try_into()
            .map_err(|_| FsrsError::InvalidParameters)?;
        Ok(Self { parameters })
    }

    /// The clipped weights backing this scheduler.
    pub fn parameters(&self) -> &[f32; PARAMETER_COUNT] {
        &self.parameters
    }

    /// Interval (days) to schedule for a given stability and desired retention.
    fn next_interval_for_stability(&self, stability: f32, desired_retention: f32) -> f32 {
        next_interval(&self.parameters, stability, desired_retention)
    }

    /// Initial stability for a new card given the first rating (1–4).
    pub fn init_stability(&self, rating: u32) -> f32 {
        init_stability(&self.parameters, rating as usize)
    }

    /// Compute the four next states (one per button) from the current memory
    /// state.
    ///
    /// Pass `current_memory_state = None` for a brand-new card (first review);
    /// otherwise pass the stored `(S, D)`. `days_elapsed` is the gap since the
    /// last review. Returns [`FsrsError::InvalidInput`] only if the arithmetic
    /// produces a non-finite state.
    pub fn next_states(
        &self,
        current_memory_state: Option<MemoryState>,
        desired_retention: f32,
        days_elapsed: u32,
    ) -> Result<NextStates, FsrsError> {
        // `nth = 0` marks a first-ever review, which triggers the `init_*`
        // seeding branch inside `step`. An existing card is `nth = 1`.
        let (state, nth) = match current_memory_state {
            Some(state) => (state, 1),
            None => (
                MemoryState {
                    stability: 0.0,
                    difficulty: 0.0,
                },
                0,
            ),
        };

        let for_rating = |rating: u32| -> Result<ItemState, FsrsError> {
            let memory = validate_state(step(
                &self.parameters,
                days_elapsed as f32,
                rating as f32,
                state,
                nth,
            ))?;
            let interval = self.next_interval_for_stability(memory.stability, desired_retention);
            Ok(ItemState { memory, interval })
        };

        Ok(NextStates {
            again: for_rating(1)?,
            hard: for_rating(2)?,
            good: for_rating(3)?,
            easy: for_rating(4)?,
        })
    }

    /// Approximate an FSRS memory state from legacy SM-2 values.
    ///
    /// When Anki migrates a card that has only SM-2 history (an ease factor and
    /// an interval, but no FSRS `(S, D)`), FSRS back-solves a plausible starting
    /// memory state so scheduling can continue smoothly. `sm2_retention` is the
    /// retention the SM-2 interval was assumed to target.
    pub fn memory_state_from_sm2(
        &self,
        ease_factor: f32,
        interval: f32,
        sm2_retention: f32,
    ) -> Result<MemoryState, FsrsError> {
        let w = &self.parameters;
        let decay = -w[20];
        let factor = 0.9f32.powf(1.0 / decay) - 1.0;
        let stability = interval.max(S_MIN) * factor / (sm2_retention.powf(1.0 / decay) - 1.0);
        let difficulty = 11.0
            - (ease_factor - 1.0)
                / (w[8].exp() * stability.powf(-w[9]) * ((1.0 - sm2_retention) * w[10]).exp_m1());
        if !stability.is_finite() || !difficulty.is_finite() {
            Err(FsrsError::InvalidInput)
        } else {
            Ok(MemoryState {
                stability,
                difficulty: difficulty.clamp(D_MIN, D_MAX),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two `f32`s are "equal" for our purposes if they agree to a tight relative
    /// tolerance. Because we reproduce upstream's exact operation order the
    /// values are in fact bit-identical, but a relative epsilon keeps the tests
    /// robust to any future harmless reassociation.
    fn approx(a: f32, b: f32) {
        let diff = (a - b).abs();
        let scale = a.abs().max(b.abs()).max(1.0);
        assert!(diff <= 1e-5 * scale, "expected {a} ≈ {b} (diff {diff})");
    }

    #[test]
    fn default_parameters_are_the_fsrs6_set() {
        assert_eq!(DEFAULT_PARAMETERS.len(), PARAMETER_COUNT);
        assert_eq!(DEFAULT_PARAMETERS[20], FSRS6_DEFAULT_DECAY);
    }

    #[test]
    fn new_rejects_bad_lengths_and_nonfinite() {
        assert_eq!(
            FSRS::new(&[1.0; 20]).err(),
            Some(FsrsError::InvalidParameters)
        );
        let mut bad = DEFAULT_PARAMETERS.to_vec();
        bad[0] = f32::NAN;
        assert_eq!(FSRS::new(&bad).err(), Some(FsrsError::InvalidParameters));
    }

    #[test]
    fn empty_slice_uses_defaults() {
        let a = FSRS::new(&[]).unwrap();
        let b = FSRS::new(&DEFAULT_PARAMETERS).unwrap();
        assert_eq!(a.parameters(), b.parameters());
    }

    // ---- Frozen numeric snapshots -----------------------------------------
    //
    // These expected values were produced by the real `fsrs` 6.6.1 crate (via a
    // throwaway cross-check that compared this crate against the live upstream
    // one across 5,900+ inputs, then was removed with the dev-dependency). They
    // are the frozen numeric gate for this reimplementation.

    #[test]
    fn new_card_next_states_match_frozen_snapshot() {
        let fsrs = FSRS::new(&DEFAULT_PARAMETERS).unwrap();
        let s = fsrs.next_states(None, 0.9, 0).unwrap();
        // Initial stabilities are exactly w[0..=3].
        approx(s.again.memory.stability, 0.212);
        approx(s.hard.memory.stability, 1.2931);
        approx(s.good.memory.stability, 2.3065);
        approx(s.easy.memory.stability, 8.2956);
        // Good's initial difficulty = w[4] - e^{w[5]*2} + 1, clamped to [1,10].
        approx(s.good.memory.difficulty, 2.118_104);
        // Good's interval at 90% retention.
        approx(s.good.interval, 2.306_5);
    }

    #[test]
    fn review_with_memory_and_elapsed_days_match_frozen_snapshot() {
        let fsrs = FSRS::new(&DEFAULT_PARAMETERS).unwrap();
        let current = MemoryState {
            stability: 7.0,
            difficulty: 5.0,
        };
        let s = fsrs.next_states(Some(current), 0.9, 3).unwrap();
        approx(s.good.memory.stability, 15.452_645);
        approx(s.good.memory.difficulty, 4.990_228);
        approx(s.good.interval, 15.452_645);
        approx(s.again.memory.stability, 1.0663601);
        approx(s.easy.memory.stability, 22.830_96);
    }

    #[test]
    fn memory_state_from_sm2_matches_frozen_snapshot() {
        let fsrs = FSRS::new(&DEFAULT_PARAMETERS).unwrap();
        let m = fsrs.memory_state_from_sm2(2.5, 10.0, 0.9).unwrap();
        approx(m.stability, 10.0);
        approx(m.difficulty, 6.9140563);
    }

    #[test]
    fn current_retrievability_decays_from_one() {
        let state = MemoryState {
            stability: 10.0,
            difficulty: 5.0,
        };
        // At t=0 recall is certain; at t=stability it is 0.9 by construction.
        approx(current_retrievability(state, 0.0, FSRS6_DEFAULT_DECAY), 1.0);
        approx(
            current_retrievability(state, 10.0, FSRS6_DEFAULT_DECAY),
            0.9,
        );
    }
}
