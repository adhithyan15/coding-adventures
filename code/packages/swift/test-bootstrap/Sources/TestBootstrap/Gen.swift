// ============================================================================
// Gen.swift -- a generator: a sampler plus an integrated shrinker
// ============================================================================
//
// A `Gen<T>` knows two things about the values it makes:
//
//   1. HOW TO SAMPLE one from a stream of random words (`sampler`);
//   2. HOW TO SHRINK one that made a property fail (`shrinker`).
//
// Keeping both in one value is what "integrated shrinking" means (VER05 section
// 4): the shrinker belongs to the generator, so a shrunk `integer(5, 10)` can
// never wander out to 0. Combinators that cannot invert themselves (`map`,
// `andThen`, `oneOf`, `frequency`) simply have no shrinker.

/// A value together with the number of accepted shrink moves that produced it.
public struct Shrunk<T> {
    public var value: T
    public var steps: Int
    public init(value: T, steps: Int) {
        self.value = value
        self.steps = steps
    }
}

/// A shrinker takes a falsifying value, a predicate that is true when a
/// candidate STILL falsifies the property, and a step budget; it returns the
/// smallest falsifying value it found with the count of accepted moves.
public typealias Shrinker<T> = (_ value: T, _ falsifies: @escaping (T) -> Bool, _ maxSteps: Int) -> Shrunk<T>

/// Thrown by a sampler that cannot produce a value (a `filter` that never
/// accepts). The runner reports it as `property-exhausted`.
public struct ExhaustedError: Error, Equatable {
    public let reason: String
    public init(_ reason: String) { self.reason = reason }
}

public struct Gen<T> {
    public let sampler: (inout SplitMix64) throws -> T
    public let shrinker: Shrinker<T>?

    public init(sampler: @escaping (inout SplitMix64) throws -> T, shrinker: Shrinker<T>? = nil) {
        self.sampler = sampler
        self.shrinker = shrinker
    }

    /// Draws one value.
    public func sample(_ rng: inout SplitMix64) throws -> T { try sampler(&rng) }

    /// Shrinks `value` as a PUBLIC entry point: opens a child scope of the
    /// ambient budget with a fresh local cap and charges every evaluation of
    /// `falsifies` (VER05e section 4.1).
    public func shrink(_ value: T, falsifies: @escaping (T) -> Bool, maxSteps: Int) throws -> Shrunk<T> {
        try Shrink.checkBudget(maxSteps)
        let depth = ShrinkBudget.enterNesting()
        defer { ShrinkBudget.leaveNesting() }
        if depth > ShrinkBudget.maxNesting { return Shrunk(value: value, steps: 0) }
        let budget = ShrinkBudget.enterChild(maxSteps: maxSteps)
        defer { budget.close() }
        if !budget.charge(1) { return Shrunk(value: value, steps: 0) }   // entry is charged up the chain
        return ShrinkBudget.bind(budget) { run(value, budget.charged(falsifies), maxSteps) }
    }

    /// Shrinks as an INTERNAL combinator step: joins the enclosing scope.
    func shrinkNested(_ value: T, _ falsifies: @escaping (T) -> Bool, _ maxSteps: Int) -> Shrunk<T> {
        // `checkBudget` cannot fail for the internal callers (they pass a
        // remaining budget derived from a validated one); a bad value is
        // treated as "no budget".
        if maxSteps < 0 || maxSteps > Limits.maxShrinkSteps { return Shrunk(value: value, steps: 0) }
        let budget = ShrinkBudget.enter(maxSteps: maxSteps)
        defer { budget.close() }
        return run(value, falsifies, maxSteps)
    }

    private func run(_ value: T, _ falsifies: @escaping (T) -> Bool, _ maxSteps: Int) -> Shrunk<T> {
        guard let shrinker else { return Shrunk(value: value, steps: 0) }
        return shrinker(value, falsifies, maxSteps)
    }
}
