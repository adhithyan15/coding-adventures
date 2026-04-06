//! # Type-Feedback Vectors
//!
//! V8 Ignition records *what types actually flow through each operation site*
//! at runtime.  This information — stored in per-function *feedback vectors*
//! — is later consumed by the optimising compiler (Turbofan / Maglev) to
//! generate specialised machine code.
//!
//! ## Feedback lifecycle
//!
//! ```text
//! first call  → slot is Uninitialized
//! one type pair seen → Monomorphic   (fast path in optimised code)
//! two–four type pairs → Polymorphic  (inline cache with a few checks)
//! five+ type pairs → Megamorphic     (fall back to generic slow path)
//! ```
//!
//! We only simulate the *recording* side here (no actual specialisation), but
//! the feedback data is fully inspectable from tests so you can verify that
//! the VM correctly observes types.
//!
//! ## Hidden-class IDs
//!
//! Every [`crate::types::VMObject`] has a `hidden_class_id`.  Property-load
//! sites record which hidden class they observed; if the same class appears
//! every time, the load is *monomorphic* and a real JIT could hardcode the
//! property offset.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Global counter used to assign unique hidden-class IDs.
///
/// Using an atomic counter means we don't need a mutex, and IDs are unique
/// even across multiple [`crate::vm::VM`] instances in the same process (which
/// matters for tests that run several VMs in parallel).
static NEXT_HIDDEN_CLASS_ID: AtomicUsize = AtomicUsize::new(0);

/// Allocates and returns a fresh, globally unique hidden-class ID.
///
/// This is called by [`crate::types::VMObject::new`] and `set_property` each
/// time an object's shape changes.
pub fn next_hidden_class_id() -> usize {
    NEXT_HIDDEN_CLASS_ID.fetch_add(1, Ordering::SeqCst)
}

/// A `TypePair` records the type names of the two operands at a binary
/// operation site (e.g. `("number", "number")` for `42 + 1`).
pub type TypePair = (String, String);

/// `FeedbackSlot` — the type-recording state for one instrumented site.
///
/// The state machine is strictly monotone: we never go backwards.
///
/// ```text
/// Uninitialized
///     │ first observation
///     ▼
/// Monomorphic([pair])       ← hot path for uniform code
///     │ second distinct pair
///     ▼
/// Polymorphic([pair, pair, …]) ← still specialisable, but needs branching
///     │ 5th distinct pair
///     ▼
/// Megamorphic               ← give up; use generic code
/// ```
#[derive(Debug, Clone)]
pub enum FeedbackSlot {
    /// No execution through this site yet.
    Uninitialized,

    /// All executions so far saw the same type pair.
    /// The inner `Vec` stores the distinct pairs seen (length 1–4 in practice).
    Monomorphic(Vec<TypePair>),

    /// Multiple distinct type pairs have been seen (2–4).
    Polymorphic(Vec<TypePair>),

    /// Too many distinct type pairs — optimisation is not worthwhile.
    Megamorphic,
}

/// Creates a feedback vector of the given size, all slots `Uninitialized`.
///
/// # Arguments
/// * `size` — number of slots, equal to `CodeObject::feedback_slot_count`.
pub fn new_vector(size: usize) -> Vec<FeedbackSlot> {
    vec![FeedbackSlot::Uninitialized; size]
}

/// Returns a short string identifying the runtime type of `v`.
///
/// Used when building [`TypePair`]s for feedback recording.
///
/// ```
/// use register_vm::{feedback, types::VMValue};
/// assert_eq!(feedback::value_type(&VMValue::Integer(1)), "integer");
/// assert_eq!(feedback::value_type(&VMValue::Str("hi".into())), "string");
/// ```
pub fn value_type(v: &crate::types::VMValue) -> String {
    use crate::types::VMValue;
    match v {
        VMValue::Integer(_) => "integer".to_string(),
        VMValue::Float(_) => "float".to_string(),
        VMValue::Str(_) => "string".to_string(),
        VMValue::Bool(_) => "boolean".to_string(),
        VMValue::Null => "null".to_string(),
        VMValue::Undefined => "undefined".to_string(),
        VMValue::Object(_) => "object".to_string(),
        VMValue::Array(_) => "array".to_string(),
        VMValue::Function(_, _) => "function".to_string(),
    }
}

/// Records the types of the two operands at a binary-operation feedback slot.
///
/// Transitions the slot through Uninitialized → Monomorphic → Polymorphic →
/// Megamorphic as more distinct type pairs are observed.
///
/// # Arguments
/// * `vector` — the mutable feedback vector of the current call frame.
/// * `slot` — index within `vector` for this operation site.
/// * `left` — left-hand operand value (used to derive its type name).
/// * `right` — right-hand operand value.
pub fn record_binary_op(
    vector: &mut Vec<FeedbackSlot>,
    slot: usize,
    left: &crate::types::VMValue,
    right: &crate::types::VMValue,
) {
    if slot >= vector.len() {
        return;
    }
    let pair = (value_type(left), value_type(right));
    let current = vector[slot].clone();
    vector[slot] = update_slot(&current, pair);
}

/// Records the hidden-class ID observed at a property-load site.
///
/// We encode the hidden-class id as a `TypePair` where both elements carry
/// the class id string.  This is a simplification; a real engine would store
/// (hidden_class, property_offset) pairs.
///
/// # Arguments
/// * `vector` — the mutable feedback vector.
/// * `slot` — index of this property-load site.
/// * `hidden_class_id` — the object's current hidden class.
pub fn record_property_load(
    vector: &mut Vec<FeedbackSlot>,
    slot: usize,
    hidden_class_id: usize,
) {
    if slot >= vector.len() {
        return;
    }
    let id_str = hidden_class_id.to_string();
    let pair = (id_str.clone(), id_str);
    let current = vector[slot].clone();
    vector[slot] = update_slot(&current, pair);
}

/// Records the type of the callee at a call site.
///
/// # Arguments
/// * `vector` — the mutable feedback vector.
/// * `slot` — index of the call site.
/// * `callee_type` — e.g. `"function"`, `"builtin"`.
pub fn record_call_site(
    vector: &mut Vec<FeedbackSlot>,
    slot: usize,
    callee_type: &str,
) {
    if slot >= vector.len() {
        return;
    }
    let pair = (callee_type.to_string(), callee_type.to_string());
    let current = vector[slot].clone();
    vector[slot] = update_slot(&current, pair);
}

/// Computes the next state for a feedback slot given a newly observed pair.
///
/// The threshold of 4 distinct pairs before going Megamorphic mirrors V8's
/// inline-cache behaviour closely enough for educational purposes.
fn update_slot(current: &FeedbackSlot, pair: TypePair) -> FeedbackSlot {
    match current {
        FeedbackSlot::Megamorphic => FeedbackSlot::Megamorphic,
        FeedbackSlot::Uninitialized => FeedbackSlot::Monomorphic(vec![pair]),
        FeedbackSlot::Monomorphic(pairs) => {
            let mut new_pairs = pairs.clone();
            if !new_pairs.contains(&pair) {
                new_pairs.push(pair);
            }
            if new_pairs.len() == 1 {
                FeedbackSlot::Monomorphic(new_pairs)
            } else {
                FeedbackSlot::Polymorphic(new_pairs)
            }
        }
        FeedbackSlot::Polymorphic(pairs) => {
            let mut new_pairs = pairs.clone();
            if !new_pairs.contains(&pair) {
                new_pairs.push(pair);
            }
            if new_pairs.len() >= 5 {
                FeedbackSlot::Megamorphic
            } else {
                FeedbackSlot::Polymorphic(new_pairs)
            }
        }
    }
}
