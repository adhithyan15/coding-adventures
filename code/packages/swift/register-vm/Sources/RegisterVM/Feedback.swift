// ============================================================================
// Feedback.swift — Inline-cache type feedback for the register VM
// ============================================================================
//
// BACKGROUND: INLINE CACHES AND TYPE FEEDBACK
// ────────────────────────────────────────────
// V8's Ignition interpreter records the runtime types it sees at each
// "polymorphic inline cache" (PIC) site.  A site is a single instruction
// that could dispatch differently depending on the types of its inputs —
// for example an `add` that might see int+int most of the time but
// occasionally float+int.
//
// The information collected here drives the *Turbofan* JIT compiler in the
// real V8; in this educational VM we collect it but don't use it to compile
// native code.  The purpose is to show how the feedback infrastructure works.
//
// FEEDBACK STATES (IC states)
// ────────────────────────────
//  uninitialized  — the site has never been reached; no info yet
//  monomorphic    — only one type combination seen (fast path in a JIT)
//  polymorphic    — 2–4 type combinations seen (medium complexity)
//  megamorphic    — more than 4 distinct combinations; give up specialising
//
// STATE MACHINE
// ─────────────
//  uninitialized ──(first call)──► monomorphic
//  monomorphic   ──(new type) ──► polymorphic   (up to 4 entries)
//  polymorphic   ──(5th type) ──► megamorphic
//  megamorphic   ─────────────► megamorphic (terminal state)
//
// ============================================================================

// ============================================================================
// MARK: - FeedbackSlot
// ============================================================================

/// The type-recording state for a single instruction site.
///
/// Each `CallFrame` carries a `[FeedbackSlot]` vector indexed by the
/// instruction's `feedbackSlot` operand.  When the interpreter executes a
/// type-recording opcode it updates the slot by calling one of the
/// `record…` helpers below.
public enum FeedbackSlot {
    /// No execution has passed through this site yet.
    case uninitialized

    /// The site has been reached with one or more type combinations, all of
    /// which fit in the `types` array (up to 4 entries).
    case monomorphic(types: [TypePair])

    /// The site has been reached with more than one type combination but
    /// fewer than five; the array holds all distinct pairs seen so far.
    case polymorphic(types: [TypePair])

    /// The site has seen more than four distinct type combinations.
    /// Further specialisation is not attempted.
    case megamorphic

    /// A pair of type-name strings describing the left and right operands
    /// of a binary operation (or the single operand for unary ops).
    public typealias TypePair = (String, String)

    /// Allocate a zero-filled feedback vector of the given size.
    ///
    /// - Parameter size: Number of slots (must equal `CodeObject.feedbackSlotCount`).
    /// - Returns: An array of `size` `.uninitialized` slots.
    public static func newVector(size: Int) -> [FeedbackSlot] {
        Array(repeating: .uninitialized, count: size)
    }
}

// ============================================================================
// MARK: - Hidden class counter
// ============================================================================

/// Module-level monotonically-increasing counter for hidden class IDs.
///
/// In a production engine this would be per-isolate; here a global counter
/// is fine because we only ever run one VM at a time.
private var _nextHiddenClassId = 0

/// Allocate the next unique hidden class identifier.
///
/// This is intentionally a free function rather than a static method so that
/// callers don't need to import any specific type.
///
/// - Returns: A unique `Int` identifier.
public func nextHiddenClassId() -> Int {
    defer { _nextHiddenClassId += 1 }
    return _nextHiddenClassId
}

// ============================================================================
// MARK: - valueType helper
// ============================================================================

/// Return a short type-name string for a `VMValue`, used as feedback keys.
///
/// These strings mirror the names that V8's Maglev/Turbofan use internally:
///
/// | `VMValue` case | returned string  |
/// |----------------|-----------------|
/// | `.integer`     | `"Smi"`         |
/// | `.float`       | `"Number"`      |
/// | `.string`      | `"String"`      |
/// | `.boolean`     | `"Boolean"`     |
/// | `.null`        | `"Null"`        |
/// | `.undefined`   | `"Undefined"`   |
/// | `.object`      | `"Object"`      |
/// | `.array`       | `"Array"`       |
/// | `.function`    | `"Function"`    |
///
/// - Parameter v: The value to classify.
/// - Returns: A short type-name string.
public func valueType(_ v: VMValue) -> String {
    switch v {
    case .integer:   return "Smi"
    case .float:     return "Number"
    case .string:    return "String"
    case .boolean:   return "Boolean"
    case .null:      return "Null"
    case .undefined: return "Undefined"
    case .object:    return "Object"
    case .array:     return "Array"
    case .function:  return "Function"
    }
}

// ============================================================================
// MARK: - recordBinaryOp
// ============================================================================

/// Update the feedback slot for a binary operation.
///
/// Call this from the `add`, `sub`, `mul`, etc. handlers after computing the
/// result but before dispatching to the next instruction.
///
/// - Parameters:
///   - vector: The mutable feedback vector of the current `CallFrame`.
///   - slot: The slot index specified by the instruction's `feedbackSlot` operand.
///   - left: The left-hand-side value.
///   - right: The right-hand-side value (often the accumulator).
public func recordBinaryOp(
    vector: inout [FeedbackSlot],
    slot: Int,
    left: VMValue,
    right: VMValue
) {
    guard slot >= 0 && slot < vector.count else { return }
    let pair: FeedbackSlot.TypePair = (valueType(left), valueType(right))
    vector[slot] = updateSlot(vector[slot], pair: pair)
}

// ============================================================================
// MARK: - recordPropertyLoad
// ============================================================================

/// Update the feedback slot for a property load (`ldaNamedProperty`).
///
/// The slot records which hidden class IDs have been seen at this load site.
/// We encode a hidden class as a string `"HiddenClass:<id>"` so it fits into
/// the same `TypePair` infrastructure.
///
/// - Parameters:
///   - vector: The mutable feedback vector.
///   - slot: Index of the slot to update.
///   - hiddenClassId: The hidden class ID of the object whose property was read.
public func recordPropertyLoad(
    vector: inout [FeedbackSlot],
    slot: Int,
    hiddenClassId: Int
) {
    guard slot >= 0 && slot < vector.count else { return }
    let pair: FeedbackSlot.TypePair = ("HiddenClass:\(hiddenClassId)", "")
    vector[slot] = updateSlot(vector[slot], pair: pair)
}

// ============================================================================
// MARK: - recordCallSite
// ============================================================================

/// Update the feedback slot for a call site (`callAnyReceiver`, `callProperty`).
///
/// Tracking what function type is called lets a JIT inline the callee.
///
/// - Parameters:
///   - vector: The mutable feedback vector.
///   - slot: The slot index.
///   - calleeType: A string describing the callee (e.g., `"Function"`, `"Closure"`).
public func recordCallSite(
    vector: inout [FeedbackSlot],
    slot: Int,
    calleeType: String
) {
    guard slot >= 0 && slot < vector.count else { return }
    let pair: FeedbackSlot.TypePair = (calleeType, "")
    vector[slot] = updateSlot(vector[slot], pair: pair)
}

// ============================================================================
// MARK: - updateSlot (private)
// ============================================================================

/// Transition a single `FeedbackSlot` according to the IC state machine.
///
/// State transitions:
/// - `uninitialized` → `monomorphic([pair])`
/// - `monomorphic` where `pair` already present → unchanged
/// - `monomorphic` where new `pair` → `polymorphic([existing..., pair])`
/// - `polymorphic` where `pair` already present → unchanged
/// - `polymorphic` with count < 4 → `polymorphic([existing..., pair])`
/// - `polymorphic` with count == 4 → `megamorphic`
/// - `megamorphic` → `megamorphic`
///
/// - Parameters:
///   - current: The slot's current state.
///   - pair: The new type pair observed at this site.
/// - Returns: The updated slot state.
private func updateSlot(_ current: FeedbackSlot, pair: FeedbackSlot.TypePair) -> FeedbackSlot {
    // Helper to check equality for tuples (not Equatable by default).
    func pairEqual(_ a: FeedbackSlot.TypePair, _ b: FeedbackSlot.TypePair) -> Bool {
        a.0 == b.0 && a.1 == b.1
    }

    switch current {
    case .uninitialized:
        return .monomorphic(types: [pair])

    case .monomorphic(let types):
        if types.contains(where: { pairEqual($0, pair) }) {
            return current  // already recorded
        }
        return .polymorphic(types: types + [pair])

    case .polymorphic(let types):
        if types.contains(where: { pairEqual($0, pair) }) {
            return current  // already recorded
        }
        if types.count >= 4 {
            return .megamorphic
        }
        return .polymorphic(types: types + [pair])

    case .megamorphic:
        return .megamorphic  // terminal state
    }
}
