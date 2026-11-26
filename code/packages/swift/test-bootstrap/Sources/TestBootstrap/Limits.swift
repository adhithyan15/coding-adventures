// ============================================================================
// Limits.swift -- every cap the engine enforces (CWE-400: resource exhaustion)
// ============================================================================
//
// A property engine runs *other people's closures* on *generated data* inside
// loops. Every loop and every allocation therefore has an explicit ceiling,
// and the ceilings live in one place so they can be read (and audited) at once.

/// The engine's resource limits. Values match VER05e (Java) so every lane
/// refuses the same inputs.
public enum Limits {
    /// Most cases one `check` may run.
    public static let maxCases = 100_000
    /// Largest shrink budget accepted (VER05 section 4.4 says 1,000 by default).
    public static let maxShrinkSteps = 10_000
    /// Longest byte string a generator may be asked for.
    public static let maxInputBytes = 1 << 20
    /// Longest list a generator may be asked for.
    public static let maxListLen = 100_000
    /// Most values one corpus `gen_*` case may ask for.
    public static let maxGenCount = 1_000_000
    /// Shrink attempts allowed per step of budget...
    public static let attemptFactor = 16
    /// ...plus this slack, so a tiny budget still gets a few probes.
    public static let attemptSlack = 64
    /// Most elements one shrink may copy in total (64M).
    public static let maxShrinkWork = 64 * 1024 * 1024
    /// Total bytes a corpus `gen_bytes` / `gen_bounded` case may produce.
    public static let maxGenBytesTotal = 1 << 20
    /// `frequency` total weight ceiling (it uses the bounded draw).
    public static let maxFrequencyTotal: UInt64 = 1 << 32
    /// Most retries one `filter` may be given.
    public static let maxFilterRetries = 10_000
    /// Retries a `filter` gets when none is given (VER05 section 3.2).
    public static let defaultMaxRetries = 100

    /// The attempt cap of one shrink: `16 * maxSteps + 64`.
    public static func attemptBudget(_ maxSteps: Int) -> Int {
        // Clamped so a hostile `maxSteps` (Int.max) cannot overflow and trap.
        attemptFactor * min(max(maxSteps, 0), maxShrinkSteps) + attemptSlack
    }
}
