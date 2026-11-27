// ============================================================================
// Options.swift, FailureIds, Verdict -- the runner's vocabulary
// ============================================================================

/// Whether the run is "CI / hosted" (redact) or "local" (verbose).
///
/// `.unset` is treated as CI: the engine FAILS CLOSED. A seed plus a public
/// property is a working exploit reproducer (VER00 section 7.3), so leaking is
/// opt-in (`.off`), never the accident of forgetting to configure.
public enum CiMode: Equatable {
    case unset, on, off
}

public struct Options: Equatable {
    public var numTests: Int
    public var maxShrinkSteps: Int
    public var ci: CiMode

    public init(numTests: Int = 100, maxShrinkSteps: Int = 1000, ci: CiMode = .unset) {
        self.numTests = numTests
        self.maxShrinkSteps = maxShrinkSteps
        self.ci = ci
    }

    /// A pure helper: the caller passes a lookup (for example
    /// `{ ProcessInfo.processInfo.environment[$0] }`); the package never reads
    /// the environment itself. `CI` set to anything except empty, `0` or
    /// `false` (trimmed, case-insensitive) means CI.
    public static func ciMode(fromLookup lookup: (String) -> String?) -> CiMode {
        guard let raw = lookup("CI") else { return .off }
        let v = raw.trimmingCharacters(in: [" ", "\t", "\n", "\r"]).lowercased()
        return (v.isEmpty || v == "0" || v == "false") ? .off : .on
    }
}

private extension String {
    func trimmingCharacters(in set: Set<Character>) -> String {
        var s = Substring(self)
        while let f = s.first, set.contains(f) { s.removeFirst() }
        while let l = s.last, set.contains(l) { s.removeLast() }
        return String(s)
    }
}

/// The closed failure vocabulary of VER05 section 5.1.
public enum FailureIds {
    public static let counterexample = "property-counterexample"
    public static let exhausted = "property-exhausted"
    /// Registered, never produced (no wall-clock timeout exists; VER05 section 7.4 #5).
    public static let timeout = "property-timeout"
    public static let panic = "property-panic"
    public static let all = [counterexample, exhausted, timeout, panic]

    /// True only for the four registered strings.
    public static func isPbtFailureId(_ s: String) -> Bool { all.contains(s) }
}

/// The outcome of one `Runner.check`.
public struct Verdict {
    public let passed: Bool
    public let failureId: String?
    public let message: String?
    /// Cases that held (or were evaluated when the run stopped).
    public let numTests: Int
    public let iterations: Int
    public let shrinkSteps: Int
    /// nil when redacted.
    public let seed: UInt64?
    /// The first falsifying value; nil when redacted or not applicable.
    public let initial: Any?
    /// The re-verified minimal witness; nil when redacted or not applicable.
    public let minimal: Any?
    /// `repro-fnv` correlation id, present only in redacted counterexamples.
    public let digest: String?
}
