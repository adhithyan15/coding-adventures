//! `SlotState` and `SlotKind` — the V8 Ignition-style feedback slot.
//!
//! A *feedback slot* is a per-instruction record of which runtime types have
//! been observed flowing through one dynamically-typed IIR instruction.  A
//! JIT uses the slot's *kind* to decide whether to specialise the instruction
//! on its observed type, emit a small dispatch table, or give up and fall back
//! to generic runtime calls.
//!
//! # State machine
//!
//! Every slot walks this monotonic progression — it can only move forward:
//!
//! ```text
//! UNINITIALIZED  ──(first observation)──►  MONOMORPHIC
//! MONOMORPHIC    ──(same type again)──────► MONOMORPHIC
//! MONOMORPHIC    ──(2nd distinct type)───►  POLYMORPHIC
//! POLYMORPHIC    ──(3rd or 4th distinct)──► POLYMORPHIC
//! POLYMORPHIC    ──(5th distinct type)───►  MEGAMORPHIC
//! MEGAMORPHIC    ──(any observation)──────► MEGAMORPHIC
//! ```
//!
//! The cap at four stored observations (transitioning to MEGAMORPHIC on the
//! fifth distinct type) is borrowed verbatim from V8's inline-cache machinery.
//! It bounds the `observations` vec to O(1) size — a megamorphic site in a
//! long-running program cannot grow memory without bound.
//!
//! # Language agnosticism
//!
//! The type strings stored in `observations` are defined by the language
//! frontend, not by vm-core.  For Tetrad everything is `"u8"`; for a Lisp
//! frontend a slot might see `["cons", "nil", "symbol"]`; for a Python
//! frontend `["int", "str", "list"]`.  The state machine only compares
//! strings — it never interprets them.
//!
//! # Example
//!
//! ```
//! use interpreter_ir::slot_state::{SlotState, SlotKind};
//!
//! let mut slot = SlotState::new();
//! assert_eq!(slot.kind, SlotKind::Uninitialized);
//!
//! slot.record("u8");
//! assert_eq!(slot.kind, SlotKind::Monomorphic);
//! assert_eq!(slot.dominant_type(), Some("u8"));
//!
//! slot.record("u8");   // same type — still mono
//! assert_eq!(slot.count, 2);
//!
//! slot.record("u16");  // second distinct type
//! assert_eq!(slot.kind, SlotKind::Polymorphic);
//! ```

/// Maximum number of distinct types to track before going megamorphic.
///
/// Four is the V8 Ignition value and the one used historically in this
/// pipeline.  Exported so downstream code can import it rather than
/// hard-coding the literal `4`.
pub const MAX_POLYMORPHIC_OBSERVATIONS: usize = 4;

// ---------------------------------------------------------------------------
// SlotKind
// ---------------------------------------------------------------------------

/// The four states of a feedback slot's type profile.
///
/// The progression is strictly monotonic: a slot only moves forward.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotKind {
    /// Never reached; slot was just allocated.  JIT waits — no data yet.
    Uninitialized,
    /// Exactly one type seen across all observations.
    /// JIT specialises aggressively on that type.
    Monomorphic,
    /// Two to four distinct types seen.
    /// JIT emits a small dispatch table with type guards per arm.
    Polymorphic,
    /// Five or more distinct types seen (observations list discarded to cap
    /// memory).  JIT skips specialisation entirely.
    Megamorphic,
}

// ---------------------------------------------------------------------------
// SlotState
// ---------------------------------------------------------------------------

/// Runtime type-profile for one feedback slot.
#[derive(Debug, Clone)]
pub struct SlotState {
    /// Current IC state — see [`SlotKind`].
    pub kind: SlotKind,

    /// Ordered list of distinct type strings seen so far.
    ///
    /// Bounded by [`MAX_POLYMORPHIC_OBSERVATIONS`]; discarded entirely when
    /// the slot transitions to `Megamorphic` so a long-running megamorphic
    /// site does not grow memory without bound.
    pub observations: Vec<String>,

    /// Total number of observations recorded (including revisits of the same
    /// type).  Used by JITs to decide whether the profile is warm enough to
    /// trust.
    pub count: u32,
}

impl SlotState {
    /// Create a new, uninitialised feedback slot.
    pub fn new() -> Self {
        SlotState {
            kind: SlotKind::Uninitialized,
            observations: Vec::new(),
            count: 0,
        }
    }

    // ------------------------------------------------------------------
    // State transitions
    // ------------------------------------------------------------------

    /// Advance the state machine by one observation.
    ///
    /// `type_name` is whatever string the language frontend uses to identify
    /// the runtime type (`"u8"`, `"cons"`, `"Number"`, …).  The state machine
    /// does not interpret the string — it only compares it with `==`.
    pub fn record(&mut self, type_name: &str) {
        self.count += 1;

        // MEGAMORPHIC is terminal — no further state updates.
        if self.kind == SlotKind::Megamorphic {
            return;
        }

        // If we've already seen this type, just bump the count (done above).
        if self.observations.iter().any(|t| t == type_name) {
            return;
        }

        // First time seeing `type_name` — will we transition?
        if self.observations.len() >= MAX_POLYMORPHIC_OBSERVATIONS {
            // Fifth distinct type: discard the observation list to cap memory
            // and mark the slot megamorphic.
            self.observations.clear();
            self.kind = SlotKind::Megamorphic;
            return;
        }

        self.observations.push(type_name.to_string());

        // Re-derive the kind from the list length.
        self.kind = if self.observations.len() == 1 {
            SlotKind::Monomorphic
        } else {
            // 2, 3, or 4 distinct types.
            SlotKind::Polymorphic
        };
    }

    // ------------------------------------------------------------------
    // Read helpers
    // ------------------------------------------------------------------

    /// Return `true` when the slot has enough data to JIT-specialise on.
    ///
    /// Only `Monomorphic` slots are specialisable.  Polymorphic sites need
    /// a dispatch table (a different codegen path), and megamorphic sites
    /// should not be specialised at all.
    pub fn is_specialisable(&self) -> bool {
        self.kind == SlotKind::Monomorphic
    }

    /// Return `true` when the slot has gone megamorphic (≥ 5 distinct types).
    pub fn is_megamorphic(&self) -> bool {
        self.kind == SlotKind::Megamorphic
    }

    /// Return the single type string for a `Monomorphic` slot, else `None`.
    ///
    /// JITs use this to ask "can I specialise here?" in one call.
    pub fn dominant_type(&self) -> Option<&str> {
        if self.kind == SlotKind::Monomorphic {
            self.observations.first().map(|s| s.as_str())
        } else {
            None
        }
    }
}

impl Default for SlotState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_slot_is_uninitialized() {
        let slot = SlotState::new();
        assert_eq!(slot.kind, SlotKind::Uninitialized);
        assert_eq!(slot.count, 0);
        assert!(slot.observations.is_empty());
    }

    #[test]
    fn first_observation_goes_monomorphic() {
        let mut slot = SlotState::new();
        slot.record("u8");
        assert_eq!(slot.kind, SlotKind::Monomorphic);
        assert_eq!(slot.count, 1);
        assert_eq!(slot.dominant_type(), Some("u8"));
    }

    #[test]
    fn repeated_same_type_stays_monomorphic() {
        let mut slot = SlotState::new();
        for _ in 0..10 {
            slot.record("u8");
        }
        assert_eq!(slot.kind, SlotKind::Monomorphic);
        assert_eq!(slot.count, 10);
    }

    #[test]
    fn second_distinct_type_goes_polymorphic() {
        let mut slot = SlotState::new();
        slot.record("u8");
        slot.record("u16");
        assert_eq!(slot.kind, SlotKind::Polymorphic);
        assert_eq!(slot.observations.len(), 2);
        assert!(slot.dominant_type().is_none());
    }

    #[test]
    fn fifth_distinct_type_goes_megamorphic() {
        let mut slot = SlotState::new();
        for t in &["u8", "u16", "u32", "u64", "bool"] {
            slot.record(t);
        }
        assert_eq!(slot.kind, SlotKind::Megamorphic);
        // Observations list is cleared to cap memory.
        assert!(slot.observations.is_empty());
        assert_eq!(slot.count, 5);
    }

    #[test]
    fn megamorphic_is_terminal() {
        let mut slot = SlotState::new();
        for t in &["u8", "u16", "u32", "u64", "bool"] {
            slot.record(t);
        }
        slot.record("str");
        assert_eq!(slot.kind, SlotKind::Megamorphic);
        assert_eq!(slot.count, 6);
    }

    #[test]
    fn is_specialisable_only_for_mono() {
        let mut slot = SlotState::new();
        assert!(!slot.is_specialisable());
        slot.record("u8");
        assert!(slot.is_specialisable());
        slot.record("u16");
        assert!(!slot.is_specialisable());
    }
}
