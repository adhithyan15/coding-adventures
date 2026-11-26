// ============================================================================
// Hooks.swift -- sabotage seams for the negative controls
// ============================================================================
//
// A conformance suite that cannot fail proves nothing (the repo's "LH-003 false
// pass"). These `internal` seams let the tests *break* the engine on purpose
// (`@testable import TestBootstrap`) and assert that the corpus run notices.
// Production code always reads the defaults; tests restore them with `reset()`
// in a `defer`.

enum Hooks {
    /// The bisection candidate: `current - floor(diff / 2)`.
    static var bisectCandidate: (BigInt, BigInt) -> BigInt = { current, diff in
        current - diff.shiftedRight(1)
    }

    /// The unit-step candidate: one step toward the target.
    static var unitCandidate: (BigInt, BigInt) -> BigInt = { current, diff in
        current - BigInt(diff.signum)
    }

    /// When true the single-deletion phase of sequence shrinking is skipped.
    static var skipDeletionPhase = false

    /// How every public entry point builds its generator from a seed.
    static var newRng: (UInt64) -> SplitMix64 = { SplitMix64(seed: $0) }

    static func reset() {
        bisectCandidate = { current, diff in current - diff.shiftedRight(1) }
        unitCandidate = { current, diff in current - BigInt(diff.signum) }
        skipDeletionPhase = false
        newRng = { SplitMix64(seed: $0) }
        ShrinkBudget.defaultMaxWork = Limits.maxShrinkWork
    }
}
