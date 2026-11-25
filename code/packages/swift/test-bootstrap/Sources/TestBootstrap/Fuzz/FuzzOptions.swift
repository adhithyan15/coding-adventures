// ============================================================================
// FuzzOptions.swift -- the bounds of a campaign (CWE-400: nothing unbounded)
// ============================================================================

/// Bounds a campaign; every field is validated against its ceiling.
///
/// `minimizeBudget` is the most target executions spent minimising ONE novel
/// crash. On top of it the whole campaign spends at most `maxIterations`
/// minimisation executions, so minimisation can at most double a campaign's
/// work however many distinct crashes it finds.
///
/// PRIVACY: the seed is deliberately absent from every textual rendering
/// (`description`, `debugDescription`, the reflection mirror): a seed plus a
/// public target is a working reproducer (VER06 section 6).
public struct FuzzOptions: CustomStringConvertible, CustomDebugStringConvertible, CustomReflectable {
    public var maxIterations: Int
    public var seed: UInt64
    public var maxInputLen: Int
    public var baseEnergy: Int
    public var minimizeBudget: Int
    public var stepBudget: Int
    public var memBudget: Int

    public static let maxStepBudget = 100_000_000
    public static let maxMemBudget = 1 << 30
    public static let maxMinimizeBudget = 100_000
    public static let defaultMinimizeBudget = 2000
    public static let defaultStepBudget = 200_000
    public static let defaultMemBudget = 64 << 20

    public init(maxIterations: Int = 1000, seed: UInt64 = 42, maxInputLen: Int = 4096, baseEnergy: Int = 10,
                minimizeBudget: Int = FuzzOptions.defaultMinimizeBudget,
                stepBudget: Int = FuzzOptions.defaultStepBudget,
                memBudget: Int = FuzzOptions.defaultMemBudget) {
        self.maxIterations = maxIterations
        self.seed = seed
        self.maxInputLen = maxInputLen
        self.baseEnergy = baseEnergy
        self.minimizeBudget = minimizeBudget
        self.stepBudget = stepBudget
        self.memBudget = memBudget
    }

    /// The limits only.
    public var description: String {
        "FuzzOptions{maxIterations=\(maxIterations) maxInputLen=\(maxInputLen) baseEnergy=\(baseEnergy)"
            + " minimizeBudget=\(minimizeBudget) stepBudget=\(stepBudget) memBudget=\(memBudget)}"
    }
    public var debugDescription: String { description }
    public var customMirror: Mirror {
        Mirror(self, children: ["limits": description], displayStyle: .struct)
    }

    /// Checks every field against its range.
    public func validate() throws {
        try FuzzOptions.check("maxIterations", maxIterations, Mutate.maxIterations)
        try FuzzOptions.check("maxInputLen", maxInputLen, Mutate.maxInputBytes)
        try FuzzOptions.check("baseEnergy", baseEnergy, Mutate.maxIterations)
        try FuzzOptions.check("minimizeBudget", minimizeBudget, FuzzOptions.maxMinimizeBudget)
        try FuzzOptions.check("stepBudget", stepBudget, FuzzOptions.maxStepBudget)
        try FuzzOptions.check("memBudget", memBudget, FuzzOptions.maxMemBudget)
    }

    private static func check(_ name: String, _ v: Int, _ hi: Int) throws {
        if v < 0 || v > hi { throw FuzzError.invalidArgument("\(name) must be in [0, \(hi)]") }
    }
}
