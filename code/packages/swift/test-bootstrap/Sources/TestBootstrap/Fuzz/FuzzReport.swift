// ============================================================================
// FuzzReport.swift -- outcomes, crash records and the campaign report
// ============================================================================
//
// PRIVACY (VER06 section 6): minimal crashing inputs and the corpus are PRIVATE.
// They are reachable only through the explicit accessors `minimalInput()` and
// `corpus()`, never through `description`, `debugDescription`, a `Mirror`
// (`dump`, `Canon.render`) or `String(describing:)`. Both types are CLASSES with
// private storage for exactly that reason, and the report does not store the
// campaign seed at all.

/// How one execution ended. `crash` carries a normalised kind and frames only:
/// never a message and never input bytes.
public enum Outcome: Equatable {
    case pass
    case crash(kind: String, frames: [String])
    case timeout
    case oom
}

/// One deduplicated crash.
public final class CrashRecord: CustomStringConvertible, CustomDebugStringConvertible, CustomReflectable {
    /// The 8-hex-digit signature.
    public let signature: String
    /// The normalised crash kind (a type name).
    public let kind: String
    /// The normalised frames, innermost first.
    public let frames: [String]
    /// How many campaign executions hit this signature (triage included).
    public private(set) var hits: Int
    /// False when minimisation was skipped or cut short by a budget, so the
    /// minimal input is only the shortest input seen, not a fixed point.
    public let minimized: Bool
    /// True when the retained-bytes cap forced the input to be dropped.
    public let inputDropped: Bool
    private var minimal: [UInt8]

    init(signature: String, kind: String, frames: [String], minimal: [UInt8], minimized: Bool, inputDropped: Bool) {
        self.signature = signature
        self.kind = kind
        self.frames = frames
        self.hits = 1
        self.minimal = minimal
        self.minimized = minimized
        self.inputDropped = inputDropped
    }

    /// A copy of the shortest input known to reproduce the signature. PRIVATE: never log it.
    public func minimalInput() -> [UInt8] { minimal }

    var minimalLength: Int { minimal.count }
    func hit() { hits += 1 }
    func replaceMinimal(_ shorter: [UInt8]) { minimal = shorter }

    /// Signature, kind, frames and counts; never input bytes.
    public var description: String {
        "CrashRecord{sig=\(signature) kind=\(kind) frames=[\(frames.joined(separator: ", "))] hits=\(hits) minimized=\(minimized)}"
    }
    public var debugDescription: String { description }
    public var customMirror: Mirror { Mirror(self, children: ["summary": description], displayStyle: .class) }
}

/// What a campaign found. Every execution lands in exactly one bucket.
/// `iterations` counts MUTATION executions (triage is not counted); crashes,
/// timeouts and OOMs count triage and mutation executions; anything that passed
/// is in the corpus or was redundant.
public final class FuzzReport: CustomStringConvertible, CustomDebugStringConvertible, CustomReflectable {
    /// Edges were reported by the target.
    public static let tierReported = "caller-reported"
    /// No edge was collected, so there was no feedback.
    public static let tierBlackBox = "black-box-uninstrumented"

    public let iterations: Int
    public let admittedSeeds: Int
    public let totalEdges: Int
    /// One record per distinct signature (at most `FuzzEngine.maxCrashes`).
    public let crashes: [CrashRecord]
    public let timeouts: Int
    public let ooms: Int
    /// DISTINCT novel signatures beyond `maxCrashes` (re-hits of a dropped one count once; the
    /// memory of dropped signatures is bounded, past which each counts once), so a cap never silently loses a failure.
    public let crashesDropped: Int
    /// Corpus admissions refused by the retained-bytes cap.
    public let admissionsDropped: Int
    public let coverageTier: String
    private let seeds: [[UInt8]]

    init(iterations: Int, admittedSeeds: Int, totalEdges: Int, crashes: [CrashRecord], timeouts: Int, ooms: Int,
         crashesDropped: Int, admissionsDropped: Int, coverageTier: String, corpus: [[UInt8]]) {
        self.iterations = iterations
        self.admittedSeeds = admittedSeeds
        self.totalEdges = totalEdges
        self.crashes = crashes
        self.timeouts = timeouts
        self.ooms = ooms
        self.crashesDropped = crashesDropped
        self.admissionsDropped = admissionsDropped
        self.coverageTier = coverageTier
        self.seeds = corpus
    }

    /// Whether the campaign can be read as "found nothing": at least one mutation
    /// iteration ran and no execution, triage included, crashed, timed out or ran
    /// out of memory. A campaign that never got past triage fuzzed nothing.
    public var isClean: Bool { iterations > 0 && crashes.isEmpty && timeouts == 0 && ooms == 0 && crashesDropped == 0 }

    /// The number of distinct crash signatures seen.
    public var uniqueCrashes: Int { crashes.count + crashesDropped }

    /// Copies of the admitted seeds. PRIVATE, like `CrashRecord.minimalInput()`.
    public func corpus() -> [[UInt8]] { seeds }

    /// Counts, never seeds or bytes.
    public var description: String {
        "FuzzReport{iterations=\(iterations) admitted=\(admittedSeeds) edges=\(totalEdges) crashes=\(crashes.count)"
            + " timeouts=\(timeouts) ooms=\(ooms) dropped=\(crashesDropped) admissionsDropped=\(admissionsDropped)"
            + " tier=\(coverageTier) clean=\(isClean)}"
    }
    public var debugDescription: String { description }
    public var customMirror: Mirror { Mirror(self, children: ["summary": description], displayStyle: .class) }
}
