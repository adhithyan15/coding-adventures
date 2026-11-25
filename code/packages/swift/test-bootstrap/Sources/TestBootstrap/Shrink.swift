// ============================================================================
// Shrink.swift -- integrated shrinking (VER05 section 4, VER05e section 4)
// ============================================================================
//
// A property that fails on [913, -4, 77, 0, 2] has told you nothing; one that
// fails on [0] has told you what is wrong. Shrinking walks from the first
// failing candidate toward the smallest one that still fails.
//
// CONVENTIONS
// -----------
//   * A predicate named `prop` is true when the property HOLDS (the corpus
//     convention). A predicate named `falsifies` is true when a candidate still
//     FAILS the property. Shrinkers take `falsifies`.
//   * `steps` counts ACCEPTED moves only. The target probe is one move.
//   * Swift arrays are values (copy-on-write), so a candidate handed to the
//     property can never corrupt the shrinker's state. The budget still charges
//     for the copy a mutation would make: the charge, not the aliasing, is what
//     bounds the work.

/// The pair returned by `Shrink.simulateReplay`.
public struct Replay: Equatable {
    public let initialFail: BigInt
    public let minimal: BigInt
}

public enum Shrink {
    private static func inBounds(_ v: BigInt, _ lo: BigInt?, _ hi: BigInt?) -> Bool {
        (lo == nil || lo! <= v) && (hi == nil || v <= hi!)
    }

    /// A step count returned by a NESTED (possibly user-written) shrinker is
    /// untrusted: clamp it to [0, remaining] so `steps += n` can neither trap on
    /// overflow nor go negative.
    static func clampAdd(_ n: Int, _ remaining: Int) -> Int { min(max(n, 0), max(remaining, 0)) }

    static func checkBudget(_ maxSteps: Int) throws {
        if maxSteps < 0 || maxSteps > Limits.maxShrinkSteps {
            throw PbtError.invalidArgument("maxSteps \(maxSteps) outside [0, \(Limits.maxShrinkSteps)]")
        }
    }

    /// Clamps a caller-supplied budget into [0, maxShrinkSteps] so internal
    /// arithmetic can never overflow (the public entries throw instead).
    static func clampSteps(_ n: Int) -> Int { min(max(n, 0), Limits.maxShrinkSteps) }

    /// Values at the API boundary must fit `BigInt.apiMaxBits`, so internal
    /// arithmetic (a difference, a span) stays under the hard 1100-bit ceiling.
    static func checkBits(_ values: [BigInt], _ what: String) throws {
        for v in values where v.bitWidth > BigInt.apiMaxBits {
            throw PbtError.invalidArgument("\(what) wider than \(BigInt.apiMaxBits) bits")
        }
    }

    // MARK: integers ----------------------------------------------------------

    /// Bisects then unit-steps `current` toward `target`.
    ///
    /// Each turn tries, in order,
    ///   1. the bisection candidate `current - floor(diff / 2)`, where
    ///      `diff = current - target`, when `|diff| > 1`;
    ///   2. the unit candidate: one step toward the target.
    /// A candidate is accepted only if it is inside [lo, hi] (nil = unbounded),
    /// differs from `current` and still falsifies. No acceptable candidate ends
    /// the walk.
    ///
    /// Worked example (corpus `pbt/shrink_int/54321_le_100`, property x <= 100):
    /// 54321 -> 27161 -> 13581 -> ... -> 101 in 15 accepted moves. 101 is the
    /// smallest value that still fails: from 101 the bisection candidate
    /// 101 - 50 = 51 satisfies the property (refused) and so does the unit step
    /// 100 (refused), so the walk ends.
    static func intWalk(
        current start: BigInt, target: BigInt, lo: BigInt?, hi: BigInt?,
        falsifies: (BigInt) -> Bool, budget: Int
    ) -> Shrunk<BigInt> {
        var current = start
        var steps = 0
        while steps < budget && current != target {
            let diff = current - target
            var moved = false
            if diff.magnitude > .one {
                let cand = Hooks.bisectCandidate(current, diff)
                if cand != current && inBounds(cand, lo, hi) && falsifies(cand) {
                    current = cand
                    steps += 1
                    moved = true
                }
            }
            if !moved {
                let cand = Hooks.unitCandidate(current, diff)
                if cand != current && inBounds(cand, lo, hi) && falsifies(cand) {
                    current = cand
                    steps += 1
                    moved = true
                }
            }
            if !moved { break }
        }
        return Shrunk(value: current, steps: steps)
    }

    /// The target probe followed by the walk (VER05 section 4.2).
    static func shrinkIntCore(
        initial: BigInt, target: BigInt, lo: BigInt, hi: BigInt,
        falsifies: (BigInt) -> Bool, maxSteps rawMaxSteps: Int
    ) -> Shrunk<BigInt> {
        let maxSteps = clampSteps(rawMaxSteps)
        if maxSteps == 0 { return Shrunk(value: initial, steps: 0) }   // no budget, no shrinking
        if inBounds(target, lo, hi) && target != initial && falsifies(target) {
            return Shrunk(value: target, steps: 1)                    // the target itself fails
        }
        return intWalk(current: initial, target: target, lo: lo, hi: hi, falsifies: falsifies, budget: maxSteps)
    }

    /// A byte shrunk toward 0 by the integer walk.
    static let byteWalk: Shrinker<UInt8> = { value, falsifies, budget in
        let r = intWalk(current: BigInt(Int(value)), target: .zero, lo: .zero, hi: BigInt(255),
                        falsifies: { falsifies(UInt8($0.asInt!)) }, budget: budget)
        return Shrunk(value: UInt8(r.value.asInt!), steps: r.steps)
    }

    // MARK: sequences ---------------------------------------------------------

    /// Shrinks a falsifying sequence (VER05 section 4.3).
    ///
    ///   0. probe: with minLen == 0, does the empty list fail? It is the answer
    ///      (one step). With minLen > 0 probe the length-minLen prefix.
    ///   1. bisection: keep the left half, else the right half, while one fails.
    ///   2. single deletion, right to left.
    ///   3. element-wise, left to right, through `shrinkElem` (nil = none).
    ///
    /// Worked example (corpus `list_sum_le_20`, initial [10 5 8 4]): the empty
    /// list satisfies the property; both halves ([10 5], [8 4]) do too; deleting
    /// index 3 leaves [10 5 8] (sum 23, still failing); the element walk takes
    /// 10 -> 8, giving the 1-minimal [8 5 8].
    ///
    /// Every candidate is charged BEFORE it is built, so a spent budget never
    /// costs another copy; when the budget is spent the loops simply end.
    static func shrinkSequence<E>(
        _ initial: [E], falsifies: @escaping ([E]) -> Bool, maxSteps: Int, minLen: Int,
        shrinkElem: Shrinker<E>?
    ) -> Shrunk<[E]> {
        let maxSteps = clampSteps(maxSteps)
        if maxSteps == 0 { return Shrunk(value: initial, steps: 0) }
        // One budget for the whole shrink, shared with every nested shrinker.
        let budget = ShrinkBudget.enter(maxSteps: maxSteps)
        defer { budget.close() }
        if !budget.chargeCopy(initial.count) { return Shrunk(value: initial, steps: 0) }
        return sequence(budget, initial, falsifies, maxSteps, minLen, shrinkElem)
    }

    private static func sequence<E>(
        _ budget: ShrinkBudget, _ initial: [E], _ falsifies: @escaping ([E]) -> Bool,
        _ maxSteps: Int, _ minLen: Int, _ shrinkElem: Shrinker<E>?
    ) -> Shrunk<[E]> {
        var current = initial
        var steps = 0

        // Phase 0: the empty / minimum-length prefix probe.
        if current.count > minLen && budget.charge(minLen) {
            if falsifies(Array(current[0..<minLen])) {
                if minLen == 0 { return Shrunk(value: [], steps: 1) }
                current.removeSubrange(minLen..<current.count)
                steps = 1
            }
        }
        let floor = max(1, minLen)

        // Phase 1: bisection (left half first, then right half).
        while current.count > floor && steps < maxSteps {
            let mid = current.count / 2
            if mid >= minLen && budget.charge(mid) && falsifies(Array(current[0..<mid])) {
                current.removeSubrange(mid..<current.count)
                steps += 1
                continue
            }
            let rightLen = current.count - mid
            if rightLen >= minLen && budget.charge(rightLen) && falsifies(Array(current[mid..<current.count])) {
                current.removeSubrange(0..<mid)
                steps += 1
                continue
            }
            break
        }

        // Phase 2: single-element deletion, right to left.
        if !Hooks.skipDeletionPhase {
            var i = current.count - 1
            while i >= 0 && current.count > floor && steps < maxSteps && budget.hasRoom() {
                if !budget.charge(current.count - 1) { break }   // charge BEFORE copying
                var cand = current
                cand.remove(at: i)
                if falsifies(cand) {
                    current.remove(at: i)
                    steps += 1
                    i = min(i, current.count - 1)
                } else {
                    i -= 1
                }
            }
        }

        // Phase 3: element-wise, left to right.
        if let shrinkElem {
            let size = current.count
            for index in 0..<size {
                let remaining = maxSteps - steps
                if remaining <= 0 || !budget.hasRoom() { break }
                let snapshot = current
                let withElement: (E) -> Bool = { candidate in
                    if !budget.charge(snapshot.count) { return false }
                    var probe = snapshot
                    probe[index] = candidate
                    return falsifies(probe)
                }
                let r = shrinkElem(snapshot[index], withElement, remaining)
                let n = clampAdd(r.steps, remaining)
                if n > 0 {
                    current[index] = r.value
                    steps += n
                }
            }
        }
        return Shrunk(value: current, steps: steps)
    }

    /// Opens a nesting level for a public entry; false when too deep (refuse).
    private static func admit() -> Bool { ShrinkBudget.enterNesting() <= ShrinkBudget.maxNesting }

    // MARK: public entry points ----------------------------------------------

    /// Shrinks a falsifying integer toward `target` inside [lo, hi] (the corpus
    /// calls these min and max). `prop(x)` is true when the property holds.
    /// Throws `.notFalsifying` when `initial` satisfies `prop`, and
    /// `.invalidArgument` for lo > hi, an initial outside the bounds, or a
    /// budget outside [0, 10,000]. The result is never outside [lo, hi].
    public static func shrinkInt(
        initial: BigInt, target: BigInt, lo: BigInt, hi: BigInt,
        prop: @escaping (BigInt) -> Bool, maxSteps: Int
    ) throws -> Shrunk<BigInt> {
        try checkBudget(maxSteps)
        if lo > hi { throw PbtError.invalidArgument("min exceeds max") }
        if !inBounds(initial, lo, hi) { throw PbtError.invalidArgument("initial value must lie within [min, max]") }
        try checkBits([initial, target, lo, hi], "shrinkInt value")
        let admitted = admit()
        defer { ShrinkBudget.leaveNesting() }
        if !admitted { return Shrunk(value: initial, steps: 0) }
        let budget = ShrinkBudget.enterChild(maxSteps: maxSteps)
        defer { budget.close() }
        if !budget.charge(1) { return Shrunk(value: initial, steps: 0) }   // starved by an enclosing shrink
        return try ShrinkBudget.bind(budget) {
            if prop(initial) { throw PbtError.notFalsifying }
            return shrinkIntCore(initial: initial, target: target, lo: lo, hi: hi,
                                 falsifies: { budget.charge(1) && !prop($0) }, maxSteps: maxSteps)
        }
    }

    /// Shrinks a falsifying list of integers: probe, halves, single deletion,
    /// then each element toward `elemTarget` (unbounded).
    public static func shrinkList(
        initial: [BigInt], elemTarget: BigInt, prop: @escaping ([BigInt]) -> Bool, maxSteps: Int
    ) throws -> Shrunk<[BigInt]> {
        try checkBudget(maxSteps)
        try checkBits(initial + [elemTarget], "shrinkList element")
        let admitted = admit()
        defer { ShrinkBudget.leaveNesting() }
        if !admitted { return Shrunk(value: initial, steps: 0) }
        let budget = ShrinkBudget.enterChild(maxSteps: maxSteps)
        defer { budget.close() }
        if !budget.chargeCopy(2 * initial.count) || !budget.charge(1) { return Shrunk(value: initial, steps: 0) }
        return try ShrinkBudget.bind(budget) {
            if prop(initial) { throw PbtError.notFalsifying }
            let walk: Shrinker<BigInt> = { value, falsifies, steps in
                intWalk(current: value, target: elemTarget, lo: nil, hi: nil, falsifies: falsifies, budget: steps)
            }
            return shrinkSequence(initial, falsifies: { !prop($0) }, maxSteps: maxSteps, minLen: 0, shrinkElem: walk)
        }
    }

    /// Shrinks falsifying bytes: sequence shrinking over byte values toward 0x00.
    public static func shrinkBytes(
        initial: [UInt8], prop: @escaping ([UInt8]) -> Bool, maxSteps: Int
    ) throws -> Shrunk<[UInt8]> {
        try checkBudget(maxSteps)
        let admitted = admit()
        defer { ShrinkBudget.leaveNesting() }
        if !admitted { return Shrunk(value: initial, steps: 0) }
        let budget = ShrinkBudget.enterChild(maxSteps: maxSteps)
        defer { budget.close() }
        // The clone, the list conversion and the array conversion back (the
        // Java lane's accounting, kept so every lane charges the same work).
        if !budget.chargeCopy(3 * initial.count) || !budget.charge(1) { return Shrunk(value: initial, steps: 0) }
        return try ShrinkBudget.bind(budget) {
            if prop(initial) { throw PbtError.notFalsifying }
            // Each attempt also converts its candidate back: charge that copy on
            // top of the attempt's own charge.
            return shrinkSequence(initial, falsifies: { budget.chargeCopy($0.count) && !prop($0) },
                                  maxSteps: maxSteps, minLen: 0, shrinkElem: byteWalk)
        }
    }

    /// Replays a failure from nothing but `seed`: draws up to 1000 values
    /// `bounded(1000)`; the first that falsifies `prop` is the initial failure,
    /// which is then shrunk toward 0 over [0, 1000]. The same seed gives the
    /// same pair forever. Throws if no draw falsified `prop`.
    public static func simulateReplay(seed: UInt64, prop: @escaping (BigInt) -> Bool) throws -> Replay {
        let admitted = admit()
        defer { ShrinkBudget.leaveNesting() }
        if !admitted { throw PbtError.invalidArgument("shrinks nested deeper than \(ShrinkBudget.maxNesting)") }
        let budget = ShrinkBudget.enterChild(maxSteps: 1000)
        defer { budget.close() }
        return try ShrinkBudget.bind(budget) { try replayBody(seed, prop, budget) }
    }

    private static func replayBody(_ seed: UInt64, _ prop: @escaping (BigInt) -> Bool, _ budget: ShrinkBudget) throws -> Replay {
        var rng = Hooks.newRng(seed)
        var initialFail: BigInt?
        var i = 0
        while i < 1000 && budget.charge(1) {
            let v = BigInt(try rng.bounded(1000))
            if !prop(v) { initialFail = v; break }
            i += 1
        }
        guard let fail = initialFail else {
            throw PbtError.invalidArgument("property was not falsified within 1000 draws")
        }
        let r = try shrinkInt(initial: fail, target: .zero, lo: .zero, hi: BigInt(1000), prop: prop, maxSteps: 1000)
        return Replay(initialFail: fail, minimal: r.value)
    }
}
