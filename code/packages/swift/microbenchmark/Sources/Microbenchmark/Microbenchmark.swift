// ============================================================================
// Microbenchmark.swift — Paired collection, analysis, and report replay
// ============================================================================
//
// The experiment compares two implementations on adjacent batches of equal
// work. Calibration is shared so both sides use one batch size. Every reported
// ratio is reconstructed from retained raw durations, and replay recomputes
// the analysis instead of trusting the JSON's claimed verdict.
//
// ============================================================================

import Foundation
import RankStatistics

public enum MicrobenchmarkError: Error, Equatable, LocalizedError {
    case invalidConfiguration(String)
    case invalidObservation(String)
    case invalidClock
    case nonPositiveDuration
    case checksumMismatch
    case calibrationExhausted
    case malformedReport(String)

    public var errorDescription: String? {
        switch self {
        case .invalidConfiguration(let message), .invalidObservation(let message), .malformedReport(let message):
            return message
        case .invalidClock:
            return "clock is negative or regressed"
        case .nonPositiveDuration:
            return "measured durations must be positive"
        case .checksumMismatch:
            return "workload checksum mismatch"
        case .calibrationExhausted:
            return "calibration limit exhausted"
        }
    }
}

public struct Configuration: Codable, Equatable {
    public var pairs: Int64
    public var confidencePercent: Int64
    public var minimumFactor: Double
    public var seed: Int64
    public var targetNs: Int64
    public var maxBatch: Int64

    public init(
        pairs: Int64 = 40,
        confidencePercent: Int64 = 95,
        minimumFactor: Double = 1.05,
        seed: Int64 = 0,
        targetNs: Int64 = 100_000,
        maxBatch: Int64 = 65_536
    ) {
        self.pairs = pairs
        self.confidencePercent = confidencePercent
        self.minimumFactor = minimumFactor
        self.seed = seed
        self.targetNs = targetNs
        self.maxBatch = maxBatch
    }

    private enum CodingKeys: String, CodingKey {
        case pairs
        case confidencePercent = "confidence_percent"
        case minimumFactor = "minimum_factor"
        case seed
        case targetNs = "target_ns"
        case maxBatch = "max_batch"
    }

    public init(from decoder: Decoder) throws {
        try rejectUnknownKeys(decoder, allowed: ["pairs", "confidence_percent", "minimum_factor", "seed", "target_ns", "max_batch"])
        let container = try decoder.container(keyedBy: CodingKeys.self)
        pairs = try container.decode(Int64.self, forKey: .pairs)
        confidencePercent = try container.decode(Int64.self, forKey: .confidencePercent)
        minimumFactor = try container.decode(Double.self, forKey: .minimumFactor)
        seed = try container.decode(Int64.self, forKey: .seed)
        targetNs = try container.decode(Int64.self, forKey: .targetNs)
        maxBatch = try container.decode(Int64.self, forKey: .maxBatch)
    }
}

public struct Pair: Codable, Equatable {
    public let baselineNs: Int64
    public let candidateNs: Int64
    public let order: String

    public init(baselineNs: Int64, candidateNs: Int64, order: String) {
        self.baselineNs = baselineNs
        self.candidateNs = candidateNs
        self.order = order
    }

    private enum CodingKeys: String, CodingKey {
        case baselineNs = "baseline_ns"
        case candidateNs = "candidate_ns"
        case order
    }

    public init(from decoder: Decoder) throws {
        try rejectUnknownKeys(decoder, allowed: ["baseline_ns", "candidate_ns", "order"])
        let container = try decoder.container(keyedBy: CodingKeys.self)
        baselineNs = try container.decode(Int64.self, forKey: .baselineNs)
        candidateNs = try container.decode(Int64.self, forKey: .candidateNs)
        order = try container.decode(String.self, forKey: .order)
    }
}

public struct Interval: Codable, Equatable {
    public let low: Double
    public let high: Double

    public init(low: Double, high: Double) {
        self.low = low
        self.high = high
    }

    public init(from decoder: Decoder) throws {
        var container = try decoder.unkeyedContainer()
        low = try container.decode(Double.self)
        high = try container.decode(Double.self)
        guard container.isAtEnd else {
            throw MicrobenchmarkError.malformedReport("analysis interval must have exactly two endpoints")
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.unkeyedContainer()
        try container.encode(low)
        try container.encode(high)
    }
}

public struct Analysis: Codable, Equatable {
    public let ratio: Double
    public let interval: Interval?
    public let verdict: String

    public init(ratio: Double, interval: Interval?, verdict: String) {
        self.ratio = ratio
        self.interval = interval
        self.verdict = verdict
    }

    private enum CodingKeys: String, CodingKey {
        case ratio
        case interval
        case verdict
    }

    public init(from decoder: Decoder) throws {
        try rejectUnknownKeys(decoder, allowed: ["ratio", "interval", "verdict"])
        let container = try decoder.container(keyedBy: CodingKeys.self)
        guard container.contains(.interval) else {
            throw MicrobenchmarkError.malformedReport("analysis.interval is required")
        }
        ratio = try container.decode(Double.self, forKey: .ratio)
        interval = try container.decodeIfPresent(Interval.self, forKey: .interval)
        verdict = try container.decode(String.self, forKey: .verdict)
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(ratio, forKey: .ratio)
        try container.encode(interval, forKey: .interval)
        try container.encode(verdict, forKey: .verdict)
    }
}

public struct Report: Codable, Equatable {
    public let config: Configuration
    public let batchSize: Int64
    public let pairs: [Pair]
    public let analysis: Analysis
    public var workload: String?
    public var invocation: [String]?
    public var revision: String?
    public var runtime: String?
    public var os: String?
    public var arch: String?
    public var profile: String?

    public init(
        config: Configuration,
        batchSize: Int64,
        pairs: [Pair],
        analysis: Analysis,
        workload: String? = nil,
        invocation: [String]? = nil,
        revision: String? = nil,
        runtime: String? = nil,
        os: String? = nil,
        arch: String? = nil,
        profile: String? = nil
    ) {
        self.config = config
        self.batchSize = batchSize
        self.pairs = pairs
        self.analysis = analysis
        self.workload = workload
        self.invocation = invocation
        self.revision = revision
        self.runtime = runtime
        self.os = os
        self.arch = arch
        self.profile = profile
    }

    private enum CodingKeys: String, CodingKey {
        case config
        case batchSize = "batch_size"
        case pairs
        case analysis
        case workload
        case invocation
        case revision
        case runtime
        case os
        case arch
        case profile
    }

    public init(from decoder: Decoder) throws {
        try rejectUnknownKeys(decoder, allowed: ["config", "batch_size", "pairs", "analysis", "workload", "invocation", "revision", "runtime", "os", "arch", "profile"])
        let container = try decoder.container(keyedBy: CodingKeys.self)
        config = try container.decode(Configuration.self, forKey: .config)
        batchSize = try container.decode(Int64.self, forKey: .batchSize)
        pairs = try container.decode([Pair].self, forKey: .pairs)
        analysis = try container.decode(Analysis.self, forKey: .analysis)
        workload = container.contains(.workload) ? try container.decode(String.self, forKey: .workload) : nil
        invocation = container.contains(.invocation) ? try container.decode([String].self, forKey: .invocation) : nil
        revision = container.contains(.revision) ? try container.decode(String.self, forKey: .revision) : nil
        runtime = container.contains(.runtime) ? try container.decode(String.self, forKey: .runtime) : nil
        os = container.contains(.os) ? try container.decode(String.self, forKey: .os) : nil
        arch = container.contains(.arch) ? try container.decode(String.self, forKey: .arch) : nil
        profile = container.contains(.profile) ? try container.decode(String.self, forKey: .profile) : nil
    }
}

private struct DynamicCodingKey: CodingKey {
    let stringValue: String
    let intValue: Int?

    init?(stringValue: String) {
        self.stringValue = stringValue
        intValue = nil
    }

    init?(intValue: Int) {
        stringValue = String(intValue)
        self.intValue = intValue
    }
}

private func rejectUnknownKeys(_ decoder: Decoder, allowed: Set<String>) throws {
    let container = try decoder.container(keyedBy: DynamicCodingKey.self)
    if let unknown = container.allKeys.first(where: { !allowed.contains($0.stringValue) }) {
        throw MicrobenchmarkError.malformedReport("unknown report field: \(unknown.stringValue)")
    }
}

/// Validates a public configuration before any user callback is invoked.
public func validate(_ config: Configuration) throws {
    guard (1...10_000).contains(config.pairs) else {
        throw MicrobenchmarkError.invalidConfiguration("pairs must be from 1 through 10000")
    }
    guard (1...99).contains(config.confidencePercent) else {
        throw MicrobenchmarkError.invalidConfiguration("confidence must be from 1 through 99")
    }
    guard config.minimumFactor.isFinite, config.minimumFactor > 1 else {
        throw MicrobenchmarkError.invalidConfiguration("minimum_factor must be finite and greater than 1")
    }
    guard config.targetNs > 0, config.maxBatch > 0 else {
        throw MicrobenchmarkError.invalidConfiguration("target_ns and max_batch must be positive")
    }
}

/// Computes a median paired ratio, exact order-statistic interval, and verdict.
public func analyze(_ pairs: [Pair], confidencePercent: Int64 = 95, minimumFactor: Double = 1.05) throws -> Analysis {
    guard !pairs.isEmpty else {
        throw MicrobenchmarkError.invalidObservation("pairs must not be empty")
    }
    guard pairs.count <= 10_000 else {
        throw MicrobenchmarkError.invalidObservation("pair count exceeds 10000")
    }
    guard (1...99).contains(confidencePercent) else {
        throw MicrobenchmarkError.invalidConfiguration("confidence must be from 1 through 99")
    }
    guard minimumFactor.isFinite, minimumFactor > 1 else {
        throw MicrobenchmarkError.invalidConfiguration("minimum_factor must be finite and greater than 1")
    }

    var ratios: [Double] = []
    ratios.reserveCapacity(pairs.count)
    for pair in pairs {
        guard pair.baselineNs > 0, pair.candidateNs > 0 else {
            throw MicrobenchmarkError.invalidObservation("durations must be positive")
        }
        guard pair.order == "baseline-first" || pair.order == "candidate-first" else {
            throw MicrobenchmarkError.invalidObservation("pair order must be baseline-first or candidate-first")
        }
        ratios.append(Double(pair.candidateNs) / Double(pair.baselineNs))
    }

    let ratio = try RankStatistics.median(ratios)
    guard let bound = try RankStatistics.medianInterval(ratios, confidencePercent: Int(confidencePercent)) else {
        return Analysis(ratio: ratio, interval: nil, verdict: "inconclusive")
    }
    let interval = Interval(low: bound.low, high: bound.high)
    let fasterBoundary = 1 / minimumFactor
    let verdict: String
    if interval.high < fasterBoundary {
        verdict = "faster"
    } else if interval.low > minimumFactor {
        verdict = "slower"
    } else if interval.low >= fasterBoundary, interval.high <= minimumFactor {
        verdict = "equivalent"
    } else {
        verdict = "inconclusive"
    }
    return Analysis(ratio: ratio, interval: interval, verdict: verdict)
}

public typealias Batch = (Int64) throws -> Int64
public typealias Clock = () -> Int64

/// Creates a monotonic nanosecond clock whose values fit signed Int64.
public func monotonicClock() -> Clock {
    let origin = DispatchTime.now().uptimeNanoseconds
    return {
        let current = DispatchTime.now().uptimeNanoseconds
        guard current >= origin else {
            return -1
        }
        let elapsed = current - origin
        guard elapsed <= UInt64(Int64.max) else {
            return -1
        }
        return Int64(elapsed)
    }
}

private struct SplitMix64 {
    private var state: UInt64

    init(seed: Int64) {
        state = UInt64(bitPattern: seed)
    }

    mutating func next() -> UInt64 {
        state &+= 0x9e3779b97f4a7c15
        var value = state
        value = (value ^ (value >> 30)) &* 0xbf58476d1ce4e5b9
        value = (value ^ (value >> 27)) &* 0x94d049bb133111eb
        return value ^ (value >> 31)
    }

    mutating func bounded(_ bound: UInt64) -> UInt64 {
        precondition(bound > 0)
        let threshold = (0 &- bound) % bound
        while true {
            let value = next()
            if value >= threshold {
                return value % bound
            }
        }
    }
}

private func orderSchedule(count: Int64, seed: Int64) -> [String] {
    var schedule = Array(repeating: "baseline-first", count: Int((count + 1) / 2))
    schedule.append(contentsOf: Array(repeating: "candidate-first", count: Int(count / 2)))
    var random = SplitMix64(seed: seed)
    for index in stride(from: schedule.count, through: 2, by: -1) {
        let other = Int(random.bounded(UInt64(index)))
        schedule.swapAt(index - 1, other)
    }
    return schedule
}

private func expectedBatch(_ checksum: Int64, _ operations: Int64) throws -> Int64 {
    let (result, overflow) = checksum.multipliedReportingOverflow(by: operations)
    guard operations > 0, !overflow else {
        throw MicrobenchmarkError.invalidObservation("batch checksum arithmetic exceeds signed 64-bit range")
    }
    return result
}

/// Verifies, calibrates, and measures adjacent paired batches with one shared size.
public func collect(
    baseline: @escaping Batch,
    candidate: @escaping Batch,
    expectedChecksum: Int64,
    config: Configuration = Configuration(),
    clock suppliedClock: Clock? = nil
) throws -> Report {
    try validate(config)
    let clock = suppliedClock ?? monotonicClock()
    let baselineWarmup = try baseline(1)
    let candidateWarmup = try candidate(1)
    guard baselineWarmup == expectedChecksum, candidateWarmup == expectedChecksum else {
        throw MicrobenchmarkError.checksumMismatch
    }

    var previous: Int64?
    func tick() throws -> Int64 {
        let now = clock()
        guard now >= 0, previous.map({ now >= $0 }) ?? true else {
            throw MicrobenchmarkError.invalidClock
        }
        previous = now
        return now
    }

    func measure(_ batch: Batch, operations: Int64) throws -> Int64 {
        let expected = try expectedBatch(expectedChecksum, operations)
        let start = try tick()
        let observed = try batch(operations)
        let end = try tick()
        guard observed == expected else {
            throw MicrobenchmarkError.checksumMismatch
        }
        return end - start
    }

    var batchSize: Int64 = 1
    while true {
        let baselineNs = try measure(baseline, operations: batchSize)
        let candidateNs = try measure(candidate, operations: batchSize)
        if baselineNs >= config.targetNs, candidateNs >= config.targetNs {
            break
        }
        guard batchSize < config.maxBatch else {
            throw MicrobenchmarkError.calibrationExhausted
        }
        batchSize = batchSize > config.maxBatch / 2 ? config.maxBatch : batchSize * 2
    }

    var pairs: [Pair] = []
    pairs.reserveCapacity(Int(config.pairs))
    for order in orderSchedule(count: config.pairs, seed: config.seed) {
        let baselineNs: Int64
        let candidateNs: Int64
        if order == "baseline-first" {
            baselineNs = try measure(baseline, operations: batchSize)
            candidateNs = try measure(candidate, operations: batchSize)
        } else {
            candidateNs = try measure(candidate, operations: batchSize)
            baselineNs = try measure(baseline, operations: batchSize)
        }
        guard baselineNs > 0, candidateNs > 0 else {
            throw MicrobenchmarkError.nonPositiveDuration
        }
        pairs.append(Pair(baselineNs: baselineNs, candidateNs: candidateNs, order: order))
    }
    return Report(
        config: config,
        batchSize: batchSize,
        pairs: pairs,
        analysis: try analyze(pairs, confidencePercent: config.confidencePercent, minimumFactor: config.minimumFactor)
    )
}

/// Recomputes and validates every report claim before encoding JSON.
public func validated(_ report: Report) throws -> Report {
    try validate(report.config)
    guard report.batchSize > 0, report.batchSize <= report.config.maxBatch else {
        throw MicrobenchmarkError.malformedReport("batch_size must be positive and no greater than max_batch")
    }
    guard Int64(report.pairs.count) == report.config.pairs else {
        throw MicrobenchmarkError.malformedReport("report pair count does not match config")
    }
    let baselineFirst = report.pairs.filter { $0.order == "baseline-first" }.count
    guard Int64(baselineFirst) == (report.config.pairs + 1) / 2 else {
        throw MicrobenchmarkError.malformedReport("report pair order counts are not balanced")
    }
    let metadataPresence = [report.workload != nil, report.invocation != nil,
                            report.revision != nil, report.runtime != nil,
                            report.os != nil, report.arch != nil]
    let metadataCount = metadataPresence.filter { $0 }.count
    guard metadataCount == 0 || metadataCount == metadataPresence.count else {
        throw MicrobenchmarkError.malformedReport("driver metadata must include all six fields")
    }
    if let workload = report.workload, !isValidWorkloadIdentifier(workload) {
        throw MicrobenchmarkError.malformedReport("workload must be nonblank and contain no NUL")
    }
    for (name, value) in [("revision", report.revision), ("runtime", report.runtime),
                          ("os", report.os), ("arch", report.arch)] {
        if let value, value.isEmpty || containsNUL(value) {
            throw MicrobenchmarkError.malformedReport("\(name) must be nonempty and contain no NUL")
        }
    }
    if let invocation = report.invocation, invocation.contains(where: containsNUL) {
        throw MicrobenchmarkError.malformedReport("invocation arguments must not contain NUL")
    }
    if let profile = report.profile {
        guard metadataCount == metadataPresence.count else {
            throw MicrobenchmarkError.malformedReport("profile requires complete driver metadata")
        }
        guard !profile.isEmpty, !containsNUL(profile) else {
            throw MicrobenchmarkError.malformedReport("profile must be nonempty and contain no NUL")
        }
    }
    let expected = try analyze(
        report.pairs,
        confidencePercent: report.config.confidencePercent,
        minimumFactor: report.config.minimumFactor
    )
    guard expected == report.analysis else {
        throw MicrobenchmarkError.malformedReport("reported analysis does not match raw observations")
    }
    return Report(
        config: report.config,
        batchSize: report.batchSize,
        pairs: report.pairs,
        analysis: expected,
        workload: report.workload,
        invocation: report.invocation,
        revision: report.revision,
        runtime: report.runtime,
        os: report.os,
        arch: report.arch,
        profile: report.profile
    )
}

private func isValidWorkloadIdentifier(_ workload: String) -> Bool {
    guard !workload.isEmpty, !workload.unicodeScalars.contains(where: { $0.value == 0 }) else {
        return false
    }
    return workload.unicodeScalars.contains { scalar in
        let value = scalar.value
        return value != 0x20 && !(0x09...0x0D).contains(value)
    }
}

private func containsNUL(_ value: String) -> Bool {
    value.unicodeScalars.contains { $0.value == 0 }
}

public func encodeReport(_ report: Report) throws -> String {
    let valid = try validated(report)
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    let data = try encoder.encode(valid)
    guard data.count <= 4 * 1024 * 1024 else {
        throw MicrobenchmarkError.malformedReport("encoded report exceeds the 4 MiB size limit")
    }
    guard let json = String(data: data, encoding: .utf8) else {
        throw MicrobenchmarkError.malformedReport("JSON encoder returned non-UTF-8 data")
    }
    return json
}

public func replayReport(_ json: String) throws -> Report {
    guard json.utf8.count <= 4 * 1024 * 1024 else {
        throw MicrobenchmarkError.malformedReport("report JSON exceeds the 4 MiB input limit")
    }
    guard let data = json.data(using: .utf8) else {
        throw MicrobenchmarkError.malformedReport("report JSON must be UTF-8")
    }
    do {
        try rejectDuplicateObjectKeys(in: data)
        let report = try JSONDecoder().decode(Report.self, from: data)
        return try validated(report)
    } catch let error as MicrobenchmarkError {
        throw error
    } catch {
        throw MicrobenchmarkError.malformedReport(error.localizedDescription)
    }
}

/// Scans only JSON structure and object-key spellings before Codable decoding.
/// Foundation remains responsible for validating complete JSON and decoding
/// the typed report, preserving integer tokens as signed Int64 values.
private struct DuplicateKeyScanner {
    private static let maximumDepth = 128
    private static let maximumValues = 65_536

    private let bytes: [UInt8]
    private var index = 0
    private var valueCount = 0
    private var decodedKeyCache: [Data: Data] = [:]

    init(data: Data) {
        bytes = Array(data)
    }

    mutating func scan() throws {
        try scanValue(depth: 0)
    }

    private mutating func scanValue(depth: Int) throws {
        skipWhitespace()
        guard index < bytes.count else { return }

        valueCount += 1
        guard valueCount <= Self.maximumValues else {
            throw MicrobenchmarkError.malformedReport("report JSON exceeds the 65,536 value limit")
        }

        switch bytes[index] {
        case 0x7B: // {
            try scanObject(depth: depth + 1)
        case 0x5B: // [
            try scanArray(depth: depth + 1)
        case 0x22: // "
            _ = scanStringToken()
        default:
            scanPrimitive()
        }
    }

    private mutating func scanObject(depth: Int) throws {
        try checkDepth(depth)
        index += 1
        skipWhitespace()
        if consume(0x7D) { return } // }

        var keys = Set<Data>()
        while index < bytes.count {
            skipWhitespace()
            guard let token = scanStringToken() else { return }
            let rawKey = Data(bytes[token])
            let decodedKey: Data
            if let cached = decodedKeyCache[rawKey] {
                decodedKey = cached
            } else {
                let value = try JSONDecoder().decode(String.self, from: rawKey)
                decodedKey = Data(value.utf8)
                decodedKeyCache[rawKey] = decodedKey
            }
            guard keys.insert(decodedKey).inserted else {
                let name = String(decoding: decodedKey, as: UTF8.self)
                throw MicrobenchmarkError.malformedReport("duplicate report field: \(name)")
            }

            skipWhitespace()
            guard consume(0x3A) else { return } // :
            try scanValue(depth: depth)
            skipWhitespace()
            if consume(0x2C) { continue } // ,
            if consume(0x7D) { return } // }
            return
        }
    }

    private mutating func scanArray(depth: Int) throws {
        try checkDepth(depth)
        index += 1
        skipWhitespace()
        if consume(0x5D) { return } // ]

        while index < bytes.count {
            try scanValue(depth: depth)
            skipWhitespace()
            if consume(0x2C) { continue } // ,
            if consume(0x5D) { return } // ]
            return
        }
    }

    private func checkDepth(_ depth: Int) throws {
        guard depth <= Self.maximumDepth else {
            throw MicrobenchmarkError.malformedReport("report JSON exceeds the 128-container depth limit")
        }
    }

    private mutating func scanStringToken() -> Range<Int>? {
        guard index < bytes.count, bytes[index] == 0x22 else { return nil }
        let start = index
        index += 1
        while index < bytes.count {
            if bytes[index] == 0x5C { // \
                index += 2
            } else if bytes[index] == 0x22 { // "
                index += 1
                return start..<index
            } else {
                index += 1
            }
        }
        return nil
    }

    private mutating func scanPrimitive() {
        while index < bytes.count, !isDelimiter(bytes[index]) {
            index += 1
        }
    }

    private mutating func skipWhitespace() {
        while index < bytes.count, [0x20, 0x09, 0x0A, 0x0D].contains(bytes[index]) {
            index += 1
        }
    }

    private mutating func consume(_ byte: UInt8) -> Bool {
        guard index < bytes.count, bytes[index] == byte else { return false }
        index += 1
        return true
    }

    private func isDelimiter(_ byte: UInt8) -> Bool {
        byte == 0x2C || byte == 0x5D || byte == 0x7D ||
            byte == 0x20 || byte == 0x09 || byte == 0x0A || byte == 0x0D
    }
}

private func rejectDuplicateObjectKeys(in data: Data) throws {
    var scanner = DuplicateKeyScanner(data: data)
    try scanner.scan()
}

public enum ArrayWorkload {
    public static let identifier = "indexed-array-sum-4096"
    public static let elementCount = 4_096
    public static let checksum: Int64 = 511_533

    public static func makeValues() -> [Int64] {
        (0..<elementCount).map { Int64((17 * $0 + 3) % 251) }
    }

    public static func indexedBatch(_ values: [Int64], operations: Int64) throws -> Int64 {
        try batch(values, operations: operations, indexed: true)
    }

    public static func iteratorBatch(_ values: [Int64], operations: Int64) throws -> Int64 {
        try batch(values, operations: operations, indexed: false)
    }

    private static func batch(_ values: [Int64], operations: Int64, indexed: Bool) throws -> Int64 {
        guard operations > 0 else {
            throw MicrobenchmarkError.invalidObservation("batch size must be positive")
        }
        var total: Int64 = 0
        for _ in 0..<operations {
            var sum: Int64 = 0
            if indexed {
                for index in values.indices {
                    let (next, overflow) = sum.addingReportingOverflow(values[index])
                    guard !overflow else {
                        throw MicrobenchmarkError.invalidObservation("array operation checksum exceeds signed 64-bit range")
                    }
                    sum = next
                }
            } else {
                for value in values {
                    let (next, overflow) = sum.addingReportingOverflow(value)
                    guard !overflow else {
                        throw MicrobenchmarkError.invalidObservation("array operation checksum exceeds signed 64-bit range")
                    }
                    sum = next
                }
            }
            let (next, overflow) = total.addingReportingOverflow(sum)
            guard !overflow else {
                throw MicrobenchmarkError.invalidObservation("array batch checksum exceeds signed 64-bit range")
            }
            total = next
        }
        return total
    }
}

public enum CommandLineResult {
    case help
    case report(Report)
}

/// Parses options and creates one in-process indexed-array report.
/// Revision metadata is caller-injected and defaults to an empty environment.
public func runCommandLine(
    arguments: [String],
    environment: [String: String] = [:],
    clock: Clock? = nil
) throws -> CommandLineResult {
    var config = Configuration()
    let environmentRevision: String
    if let primary = environment["GIT_REVISION"] {
        environmentRevision = primary.isEmpty ? "unknown" : primary
    } else if let fallback = environment["GITHUB_SHA"] {
        environmentRevision = fallback.isEmpty ? "unknown" : fallback
    } else {
        environmentRevision = "unknown"
    }
    var reportRevision = environmentRevision
    var index = 0
    while index < arguments.count {
        let flag = arguments[index]
        if flag == "--help" || flag == "-h" {
            return .help
        }
        guard index + 1 < arguments.count else {
            throw MicrobenchmarkError.invalidConfiguration("\(flag) requires a value")
        }
        let value = arguments[index + 1]
        switch flag {
        case "--pairs":
            config.pairs = try parseInt64(value, flag: flag)
        case "--confidence":
            config.confidencePercent = try parseInt64(value, flag: flag)
        case "--factor":
            guard let factor = Double(value), factor.isFinite else {
                throw MicrobenchmarkError.invalidConfiguration("invalid value for --factor")
            }
            config.minimumFactor = factor
        case "--seed":
            config.seed = try parseInt64(value, flag: flag)
        case "--target-ns":
            config.targetNs = try parseInt64(value, flag: flag)
        case "--max-batch":
            config.maxBatch = try parseInt64(value, flag: flag)
        case "--revision":
            reportRevision = value
        default:
            throw MicrobenchmarkError.invalidConfiguration("unknown option: \(flag)")
        }
        index += 2
    }

    try validate(config)
    let values = ArrayWorkload.makeValues()
    guard values.reduce(0, +) == ArrayWorkload.checksum else {
        throw MicrobenchmarkError.checksumMismatch
    }
    var report = try collect(
        baseline: { try ArrayWorkload.indexedBatch(values, operations: $0) },
        candidate: { try ArrayWorkload.iteratorBatch(values, operations: $0) },
        expectedChecksum: ArrayWorkload.checksum,
        config: config,
        clock: clock
    )
    report.workload = ArrayWorkload.identifier
    report.invocation = arguments
    report.revision = reportRevision
    report.runtime = swiftRuntimeVersion
    report.os = operatingSystemName
    report.arch = architectureName
    report.profile = buildProfile
    return .report(try validated(report))
}

private func parseInt64(_ text: String, flag: String) throws -> Int64 {
    let bytes = Array(text.utf8)
    let digitsStart = bytes.first == 45 ? 1 : 0
    guard digitsStart < bytes.count,
          bytes[digitsStart...].allSatisfy({ (48...57).contains($0) }),
          let value = Int64(text),
          String(value) == text
    else {
        throw MicrobenchmarkError.invalidConfiguration("invalid value for \(flag)")
    }
    return value
}

private var architectureName: String {
    #if arch(x86_64)
        "x86_64"
    #elseif arch(arm64)
        "aarch64"
    #elseif arch(i386)
        "x86"
    #else
        "unknown"
    #endif
}

/// The host operating-system family, using the shared C benchmark labels.
private var operatingSystemName: String {
    #if os(Windows)
        "windows"
    #elseif os(macOS)
        "macos"
    #elseif os(Linux)
        "linux"
    #else
        "unknown"
    #endif
}

private var buildProfile: String {
    #if DEBUG
        "debug"
    #else
        "release"
    #endif
}

private var swiftRuntimeVersion: String {
    #if compiler(>=6.4)
        "Swift 6.4+"
    #elseif compiler(>=6.3)
        "Swift 6.3+"
    #elseif compiler(>=6.2)
        "Swift 6.2+"
    #elseif compiler(>=6.1)
        "Swift 6.1+"
    #elseif compiler(>=6.0)
        "Swift 6.0+"
    #elseif compiler(>=5.9)
        "Swift 5.9+"
    #else
        "Swift"
    #endif
}
