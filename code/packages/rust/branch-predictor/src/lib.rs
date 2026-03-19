/// # Branch Predictor -- Pluggable branch prediction algorithms for CPU simulation
///
/// This crate implements several branch prediction strategies, from simple
/// static predictors to dynamic two-bit saturating counters, plus a Branch
/// Target Buffer (BTB) for caching branch target addresses.
///
/// ## Modules
/// - `prediction` - `Prediction` struct and `BranchPredictor` trait
/// - `static_pred` - `AlwaysTaken`, `AlwaysNotTaken`, `BTFNT`
/// - `one_bit` - `OneBitPredictor` (1-bit per branch)
/// - `two_bit` - `TwoBitPredictor` with `TwoBitState` enum
/// - `btb` - `BranchTargetBuffer` for target address caching
/// - `stats` - `PredictionStats` for accuracy tracking
///
/// ## Quick start
/// ```
/// use branch_predictor::{TwoBitPredictor, TwoBitState, BranchPredictor};
///
/// let mut pred = TwoBitPredictor::new(1024, TwoBitState::WeaklyNotTaken);
/// let p = pred.predict(0x100);
/// assert!(!p.taken); // starts not-taken
/// pred.update(0x100, true, None);
/// let p = pred.predict(0x100);
/// assert!(p.taken); // learned from outcome
/// ```
pub mod btb;
pub mod one_bit;
pub mod prediction;
pub mod static_pred;
pub mod stats;
pub mod two_bit;

// Re-export the main types at the crate root for convenient access.
pub use btb::{BTBEntry, BranchTargetBuffer};
pub use one_bit::OneBitPredictor;
pub use prediction::{BranchPredictor, Prediction};
pub use static_pred::{AlwaysNotTakenPredictor, AlwaysTakenPredictor, BackwardTakenForwardNotTaken};
pub use stats::PredictionStats;
pub use two_bit::{TwoBitPredictor, TwoBitState};
