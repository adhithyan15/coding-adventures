import XCTest
@testable import Microbenchmark
import RankStatistics

final class MicrobenchmarkTests: XCTestCase {
    func testFixtureCasesAndPackageCopy() throws {
        let packageRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let localURL = packageRoot.appendingPathComponent("testdata/microbenchmark-pairs-v1.json")
        let canonicalURL = packageRoot.appendingPathComponent("../../../specs/fixtures/microbenchmark-pairs-v1.json")
        let local = try Data(contentsOf: localURL)
        XCTAssertEqual(local, try Data(contentsOf: canonicalURL))
        let fixture = try JSONDecoder().decode(Fixture.self, from: local)

        for example in fixture.cases {
            let pairs = example.candidate.map {
                Pair(baselineNs: example.baseline, candidateNs: $0, order: "baseline-first")
            }
            let analysis = try analyze(pairs, confidencePercent: fixture.confidencePercent,
                                       minimumFactor: fixture.minimumFactor)
            XCTAssertEqual(analysis.ratio, example.ratio, accuracy: 1e-15, example.name)
            if let expected = example.interval {
                guard let actual = analysis.interval else {
                    return XCTFail("expected interval for \(example.name)")
                }
                XCTAssertEqual(actual.low, expected[0], accuracy: 1e-15, example.name)
                XCTAssertEqual(actual.high, expected[1], accuracy: 1e-15, example.name)
            } else {
                XCTAssertNil(analysis.interval, example.name)
            }
            XCTAssertEqual(analysis.verdict, example.verdict, example.name)
        }
    }

    func testDefaultsAndConfigurationBounds() throws {
        XCTAssertEqual(Configuration(), Configuration(pairs: 40, confidencePercent: 95,
                                                      minimumFactor: 1.05, seed: 0,
                                                      targetNs: 100_000, maxBatch: 65_536))
        try validate(Configuration(maxBatch: Int64.max))
        let invalid = [Configuration(pairs: 0), Configuration(pairs: 10_001),
                       Configuration(confidencePercent: 0), Configuration(minimumFactor: .infinity),
                       Configuration(minimumFactor: 1), Configuration(targetNs: 0), Configuration(maxBatch: 0)]
        for config in invalid {
            XCTAssertThrowsError(try validate(config))
        }
    }

    func testErrorDescriptionsRemainActionable() {
        XCTAssertEqual(MicrobenchmarkError.invalidConfiguration("bad config").errorDescription, "bad config")
        XCTAssertEqual(MicrobenchmarkError.invalidObservation("bad sample").errorDescription, "bad sample")
        XCTAssertEqual(MicrobenchmarkError.invalidClock.errorDescription, "clock is negative or regressed")
        XCTAssertEqual(MicrobenchmarkError.nonPositiveDuration.errorDescription, "measured durations must be positive")
        XCTAssertEqual(MicrobenchmarkError.checksumMismatch.errorDescription, "workload checksum mismatch")
        XCTAssertEqual(MicrobenchmarkError.calibrationExhausted.errorDescription, "calibration limit exhausted")
        XCTAssertEqual(MicrobenchmarkError.malformedReport("bad report").errorDescription, "bad report")
    }

    func testCollectorCalibrationAndReproducibleBalancedOrder() throws {
        var tick: Int64 = 0
        var sizes: [Int64] = []
        let config = Configuration(pairs: 5, seed: -91, targetNs: 7, maxBatch: 32)
        let report = try collect(
            baseline: { sizes.append($0); tick += $0; return 11 * $0 },
            candidate: { sizes.append($0); tick += $0; return 11 * $0 },
            expectedChecksum: 11,
            config: config,
            clock: { tick }
        )
        XCTAssertEqual(report.batchSize, 8)
        XCTAssertEqual(report.pairs.count, 5)
        XCTAssertEqual(report.pairs.filter { $0.order == "baseline-first" }.count, 3)
        XCTAssertTrue(report.pairs.allSatisfy { $0.baselineNs > 0 && $0.candidateNs > 0 })
        XCTAssertEqual(Set(sizes), [1, 2, 4, 8])

        var secondTick: Int64 = 0
        let repeated = try collect(
            baseline: { secondTick += $0; return 11 * $0 },
            candidate: { secondTick += $0; return 11 * $0 }, expectedChecksum: 11,
            config: config,
            clock: { secondTick }
        )
        XCTAssertEqual(report.pairs.map(\.order), repeated.pairs.map(\.order))
        XCTAssertGreaterThanOrEqual(monotonicClock()(), 0)
    }

    func testAnalyzeBoundsOrdersDurationsAndVerdictBoundaries() throws {
        XCTAssertThrowsError(try analyze([]))
        XCTAssertThrowsError(try analyze(Array(repeating: Pair(baselineNs: 1, candidateNs: 1,
                                                               order: "baseline-first"), count: 10_001)))
        XCTAssertThrowsError(try analyze([Pair(baselineNs: 0, candidateNs: 1, order: "baseline-first")]))
        XCTAssertThrowsError(try analyze([Pair(baselineNs: 1, candidateNs: 1, order: "bad-order")]))
        XCTAssertThrowsError(try analyze([Pair(baselineNs: 1, candidateNs: 1, order: "baseline-first")],
                                         confidencePercent: 100))
        XCTAssertThrowsError(try analyze([Pair(baselineNs: 1, candidateNs: 1, order: "baseline-first")],
                                         minimumFactor: .nan))

        func repeated(_ candidate: Int64) -> [Pair] {
            (0..<6).map { Pair(baselineNs: 100, candidateNs: candidate,
                               order: $0.isMultiple(of: 2) ? "baseline-first" : "candidate-first") }
        }
        XCTAssertEqual(try analyze(repeated(80)).verdict, "faster")
        XCTAssertEqual(try analyze(repeated(120)).verdict, "slower")
        XCTAssertEqual(try analyze(repeated(100)).verdict, "equivalent")
        XCTAssertEqual(try analyze(Array(repeated(80).prefix(2))).verdict, "inconclusive")
    }

    func testValidationOccursBeforeCallbacksAndFailuresReturnNoReport() {
        var calls = 0
        XCTAssertThrowsError(try collect(
            baseline: { calls += 1; return $0 }, candidate: { $0 }, expectedChecksum: 1,
            config: Configuration(targetNs: 0), clock: { 0 }
        ))
        XCTAssertEqual(calls, 0)

        XCTAssertThrowsError(try collect(
            baseline: { _ in 0 }, candidate: { $0 }, expectedChecksum: 1,
            config: Configuration(pairs: 1, targetNs: 1, maxBatch: 1), clock: { 0 }
        ))

        var sequence: [Int64] = [4, 5, 4]
        XCTAssertThrowsError(try collect(
            baseline: { $0 }, candidate: { $0 }, expectedChecksum: 1,
            config: Configuration(pairs: 1, targetNs: 1, maxBatch: 1),
            clock: { sequence.removeFirst() }
        )) { XCTAssertEqual($0 as? MicrobenchmarkError, .invalidClock) }

        XCTAssertThrowsError(try collect(
            baseline: { _ in throw TestFailure.callback }, candidate: { $0 }, expectedChecksum: 1,
            config: Configuration(pairs: 1, targetNs: 1, maxBatch: 1), clock: { 0 }
        ))
    }

    func testChecksumOverflowAndZeroMeasuredDurationFail() {
        var tick: Int64 = 0
        XCTAssertThrowsError(try collect(
            baseline: { $0 == 1 ? Int64.max : Int64.max },
            candidate: { $0 == 1 ? Int64.max : Int64.max },
            expectedChecksum: Int64.max,
            config: Configuration(pairs: 1, targetNs: 2, maxBatch: 2),
            clock: { defer { tick += 1 }; return tick }
        )) { XCTAssertEqual($0 as? MicrobenchmarkError, .invalidObservation("batch checksum arithmetic exceeds signed 64-bit range")) }

        var ticks: [Int64] = [0, 1, 2, 3, 4, 4, 5, 6]
        XCTAssertThrowsError(try collect(
            baseline: { $0 }, candidate: { $0 }, expectedChecksum: 1,
            config: Configuration(pairs: 1, targetNs: 1, maxBatch: 1),
            clock: { ticks.removeFirst() }
        )) { XCTAssertEqual($0 as? MicrobenchmarkError, .nonPositiveDuration) }
    }

    func testSignedInt64JSONTokensRoundTripExactly() throws {
        let config = Configuration(pairs: 1, seed: Int64.min, targetNs: 1, maxBatch: Int64.max)
        let pair = Pair(baselineNs: Int64.max, candidateNs: Int64.max, order: "baseline-first")
        let report = Report(config: config, batchSize: Int64.max, pairs: [pair], analysis: try analyze([pair]))
        let json = try encodeReport(report)
        XCTAssertTrue(json.contains("-9223372036854775808"))
        XCTAssertTrue(json.contains("9223372036854775807"))
        XCTAssertEqual(try replayReport(json), report)
    }

    func testReplayRejectsForgedAnalysisUnbalancedOrdersUnknownAndFractionalFields() throws {
        let config = Configuration(pairs: 2, targetNs: 1, maxBatch: 1)
        let pairs = [Pair(baselineNs: 10, candidateNs: 10, order: "baseline-first"),
                     Pair(baselineNs: 10, candidateNs: 10, order: "candidate-first")]
        let analysis = try analyze(pairs)
        let report = Report(config: config, batchSize: 1, pairs: pairs, analysis: analysis)
        let valid = try encodeReport(report)

        let forged = Report(config: config, batchSize: 1, pairs: pairs,
                            analysis: Analysis(ratio: 1.5, interval: analysis.interval, verdict: analysis.verdict))
        let forgedJSON = String(decoding: try JSONEncoder().encode(forged), as: UTF8.self)
        XCTAssertThrowsError(try replayReport(forgedJSON))
        XCTAssertThrowsError(try replayReport(valid.replacingOccurrences(of: "candidate-first", with: "baseline-first")))
        XCTAssertThrowsError(try replayReport(String(valid.dropLast()) + ",\"unexpected\":1}"))
        XCTAssertThrowsError(try replayReport(valid.replacingOccurrences(of: "\"seed\":0", with: "\"seed\":0.5")))
        XCTAssertThrowsError(try replayReport("{not-json"))
        XCTAssertThrowsError(try replayReport(String(repeating: " ", count: 4 * 1024 * 1024 + 1)))

        let oversized = Report(config: config, batchSize: 1, pairs: pairs, analysis: analysis,
                               revision: String(repeating: "r", count: 4 * 1024 * 1024))
        XCTAssertThrowsError(try encodeReport(oversized))
    }

    func testReplayRejectsDuplicateKeysAtEveryObjectDepth() throws {
        let config = Configuration(pairs: 2, targetNs: 1, maxBatch: 1)
        let pairs = [Pair(baselineNs: 10, candidateNs: 10, order: "baseline-first"),
                     Pair(baselineNs: 10, candidateNs: 10, order: "candidate-first")]
        let analysis = try analyze(pairs)
        let report = Report(config: config, batchSize: 1, pairs: pairs, analysis: analysis)
        let valid = try encodeReport(report)

        func assertDuplicate(_ json: String, _ key: String, file: StaticString = #filePath, line: UInt = #line) {
            XCTAssertThrowsError(try replayReport(json), file: file, line: line) {
                XCTAssertEqual($0 as? MicrobenchmarkError, .malformedReport("duplicate report field: \(key)"), file: file, line: line)
            }
        }

        assertDuplicate(valid.replacingOccurrences(of: "\"seed\":0", with: "\"seed\":0,\"seed\":1"), "seed")
        assertDuplicate(valid.replacingOccurrences(of: "\"seed\":0", with: "\"seed\":0,\"\\u0073eed\":1"), "seed")
        assertDuplicate(valid.replacingOccurrences(of: "\"baseline_ns\":10", with: "\"baseline_ns\":10,\"baseline_ns\":11"), "baseline_ns")
        assertDuplicate(valid.replacingOccurrences(of: "\"verdict\":\"inconclusive\"", with: "\"verdict\":\"inconclusive\",\"verdict\":\"faster\""), "verdict")

        let metadata = Report(config: config, batchSize: 1, pairs: pairs, analysis: analysis,
                              workload: "metadata-test",
                              invocation: ["same-value", "same-value"], revision: "repeat",
                              runtime: "swift", os: "test", arch: "test")
        let metadataJSON = try encodeReport(metadata)
        assertDuplicate(metadataJSON.replacingOccurrences(of: "\"revision\":\"repeat\"",
                                                          with: "\"revision\":\"repeat\",\"revision\":\"other\""), "revision")
        XCTAssertEqual(try replayReport(metadataJSON), metadata, "repeated string values are legal")

        let nestedDuplicate = String(valid.dropLast()) + ",\"extra\":{\"items\":[{\"same\":1,\"same\":2}]}}"
        assertDuplicate(nestedDuplicate, "same")
    }

    func testDuplicateKeyScannerBoundsNestingWithoutReplacingJSONDecoder() throws {
        let config = Configuration(pairs: 1, targetNs: 1, maxBatch: 1)
        let pair = Pair(baselineNs: 10, candidateNs: 10, order: "baseline-first")
        let report = Report(config: config, batchSize: 1, pairs: [pair], analysis: try analyze([pair]))
        let valid = try encodeReport(report)
        let nested = String(repeating: "[", count: 129) + "0" + String(repeating: "]", count: 129)
        XCTAssertThrowsError(try replayReport(String(valid.dropLast()) + ",\"extra\":\(nested)}"))
        let excessValues = String(repeating: "0,", count: 65_536) + "0"
        XCTAssertThrowsError(try replayReport(String(valid.dropLast()) + ",\"extra\":[\(excessValues)]}"))
        XCTAssertThrowsError(try replayReport(String(valid.dropLast()) + ",\"extra\":[1,,2]}"))
        XCTAssertEqual(try replayReport(valid), report, "valid nested report objects and arrays still decode")
    }

    func testMaximumCollectedReportFitsReplayValueLimit() throws {
        let config = Configuration(pairs: 10_000, targetNs: 1, maxBatch: 1)
        let pairs = (0..<10_000).map {
            Pair(baselineNs: 1_000, candidateNs: 1_001,
                 order: $0.isMultiple(of: 2) ? "baseline-first" : "candidate-first")
        }
        let report = Report(config: config, batchSize: 1, pairs: pairs,
                            analysis: try analyze(pairs))
        let json = try encodeReport(report)
        XCTAssertLessThan(json.utf8.count, 4 * 1024 * 1024)
        XCTAssertEqual(try replayReport(json), report)
    }

    func testIntervalJSONShapeAndReportValidation() throws {
        let config = Configuration(pairs: 6, targetNs: 1, maxBatch: 1)
        let pairs = (0..<6).map { Pair(baselineNs: 10, candidateNs: 20,
                                      order: $0.isMultiple(of: 2) ? "baseline-first" : "candidate-first") }
        let analysis = try analyze(pairs)
        XCTAssertNotNil(analysis.interval)
        let report = Report(config: config, batchSize: 1, pairs: pairs, analysis: analysis,
                            workload: ArrayWorkload.identifier, invocation: [], revision: "test",
                            runtime: "swift", os: "test", arch: "test")
        let json = try encodeReport(report)
        XCTAssertEqual(try replayReport(json), report)
        let generic = Report(config: config, batchSize: 1, pairs: pairs, analysis: analysis,
                             workload: "dependent-link-order", invocation: ["--seed", "19"],
                             revision: "experiment", runtime: "go", os: "windows", arch: "amd64")
        let genericJSON = try encodeReport(generic)
        XCTAssertEqual(try replayReport(genericJSON), generic)
        for invalid in ["", " \t\r\n"] {
            let quoted = try JSONEncoder().encode(invalid)
            let token = String(decoding: quoted, as: UTF8.self)
            XCTAssertThrowsError(try replayReport(genericJSON.replacingOccurrences(
                of: "\"workload\":\"dependent-link-order\"", with: "\"workload\":\(token)"
            )))
        }
        XCTAssertThrowsError(try replayReport(genericJSON.replacingOccurrences(
            of: "\"workload\":\"dependent-link-order\"", with: "\"workload\":\"bad\\u0000id\""
        )))
        for invalid in ["null", "17", "true", "[]", "{}"] {
            XCTAssertThrowsError(try replayReport(genericJSON.replacingOccurrences(
                of: "\"workload\":\"dependent-link-order\"", with: "\"workload\":\(invalid)"
            )))
        }
        XCTAssertThrowsError(try replayReport(genericJSON.replacingOccurrences(
            of: "\"workload\":\"dependent-link-order\"", with: "\"workload\":\"\\uD800\""
        )))
        let extendedInterval = json.replacingOccurrences(of: "\"interval\":[2,2]",
                                                         with: "\"interval\":[2,2,2]")
        XCTAssertNotEqual(extendedInterval, json)
        XCTAssertThrowsError(try replayReport(extendedInterval))

        XCTAssertThrowsError(try validated(Report(config: config, batchSize: 0,
                                                   pairs: pairs, analysis: analysis)))
        XCTAssertThrowsError(try validated(Report(config: config, batchSize: 1,
                                                   pairs: Array(pairs.dropLast()), analysis: analysis)))
        XCTAssertThrowsError(try validated(Report(config: config, batchSize: 1, pairs: pairs,
                                                   analysis: analysis, workload: "other-workload")))
        for invalid in ["", " \t\r\n", "bad\0id"] {
            XCTAssertThrowsError(try validated(Report(config: config, batchSize: 1, pairs: pairs,
                                                       analysis: analysis, workload: invalid)))
        }
        XCTAssertThrowsError(try validated(Report(config: config, batchSize: 1, pairs: pairs,
                                                   analysis: Analysis(ratio: 3, interval: analysis.interval,
                                                                     verdict: analysis.verdict))))
    }

    func testArrayWorkloadAndCommandLineParsing() throws {
        let values = ArrayWorkload.makeValues()
        XCTAssertEqual(values.count, 4_096)
        XCTAssertEqual(Array(values.prefix(4)), [3, 20, 37, 54])
        XCTAssertEqual(values.reduce(0, +), ArrayWorkload.checksum)
        XCTAssertEqual(try ArrayWorkload.indexedBatch(values, operations: 3), 3 * ArrayWorkload.checksum)
        XCTAssertEqual(try ArrayWorkload.iteratorBatch(values, operations: 3), 3 * ArrayWorkload.checksum)
        XCTAssertThrowsError(try ArrayWorkload.indexedBatch([Int64.max, 1], operations: 1))
        XCTAssertThrowsError(try ArrayWorkload.iteratorBatch([Int64.min, -1], operations: 1))

        var tick: Int64 = 0
        guard case .report(let report) = try runCommandLine(
            arguments: ["--pairs", "3", "--target-ns", "1", "--max-batch", "1",
                        "--seed", "-9223372036854775808", "--revision", "test"],
            environment: [:], clock: { defer { tick += 1 }; return tick }
        ) else {
            return XCTFail("expected a report")
        }
        XCTAssertEqual(report.config.seed, Int64.min)
        XCTAssertEqual(report.revision, "test")
        XCTAssertEqual(report.workload, ArrayWorkload.identifier)
        XCTAssertEqual(report.pairs.count, 3)
        XCTAssertEqual(report.os, expectedOperatingSystemName)
        XCTAssertEqual(try runCommandLine(arguments: ["--help"], environment: [:], clock: { 0 }).isHelp, true)

        let invalid = [["--unknown", "1"], ["--pairs"], ["--pairs", "9223372036854775808"],
                       ["--revision", ""],
                       ["--seed", "1_000"], ["--factor", "infinity"]]
        for args in invalid {
            XCTAssertThrowsError(try runCommandLine(arguments: args, environment: [:], clock: { 0 }))
        }
    }

    func testCommandLineEnvironmentInjectionAndEmptyDefault() throws {
        func report(environment: [String: String]? = nil) throws -> Report {
            var tick: Int64 = 0
            let arguments = ["--pairs", "1", "--target-ns", "1", "--max-batch", "1"]
            let result: CommandLineResult
            if let environment {
                result = try runCommandLine(
                    arguments: arguments,
                    environment: environment,
                    clock: { defer { tick += 1 }; return tick }
                )
            } else {
                result = try runCommandLine(
                    arguments: arguments,
                    clock: { defer { tick += 1 }; return tick }
                )
            }
            guard case .report(let value) = result else {
                throw MicrobenchmarkError.invalidConfiguration("expected a report")
            }
            return value
        }

        XCTAssertEqual(try report().revision, "unknown")
        XCTAssertEqual(
            try report(environment: ["GIT_REVISION": "git-revision", "GITHUB_SHA": "github-sha"]).revision,
            "git-revision"
        )
        XCTAssertEqual(try report(environment: ["GITHUB_SHA": "github-sha"]).revision, "github-sha")
        XCTAssertEqual(try report(environment: ["GIT_REVISION": "", "GITHUB_SHA": "github-sha"]).revision,
                       "unknown", "a present empty primary suppresses the fallback")
        XCTAssertEqual(try report(environment: ["GITHUB_SHA": ""]).revision, "unknown")
    }

    func testMetadataGroupAndPortableStringRules() throws {
        let config = Configuration(pairs: 1, targetNs: 1, maxBatch: 1)
        let pairs = [Pair(baselineNs: 10, candidateNs: 10, order: "baseline-first")]
        let analysis = try analyze(pairs)
        let complete = Report(config: config, batchSize: 1, pairs: pairs, analysis: analysis,
                              workload: "guest", invocation: [], revision: "r",
                              runtime: "\u{2003}", os: " ", arch: "test", profile: " ")
        let whitespaceJSON = try encodeReport(complete)
        XCTAssertEqual(try replayReport(whitespaceJSON), complete,
                       "empty argv and nonempty descriptive whitespace are valid")

        let argument = Report(config: config, batchSize: 1, pairs: pairs, analysis: analysis,
                              workload: "guest", invocation: [""], revision: "r",
                              runtime: "swift", os: "test", arch: "test")
        XCTAssertEqual(try replayReport(encodeReport(argument)), argument,
                       "empty argument values preserve argv faithfully")

        XCTAssertThrowsError(try validated(Report(config: config, batchSize: 1, pairs: pairs,
                                                   analysis: analysis, workload: "guest")))
        XCTAssertThrowsError(try validated(Report(config: config, batchSize: 1, pairs: pairs,
                                                   analysis: analysis, profile: "release")))
        let strictMetadata = try encodeReport(Report(
            config: config, batchSize: 1, pairs: pairs, analysis: analysis,
            workload: "guest", invocation: [], revision: "r", runtime: "swift",
            os: "test", arch: "test", profile: "release"
        ))
        for invalid in [
            strictMetadata.replacingOccurrences(of: "\"workload\":\"guest\"", with: "\"workload\":\"\""),
            strictMetadata.replacingOccurrences(of: "\"workload\":\"guest\"", with: "\"workload\":\"bad\\u0000id\""),
            strictMetadata.replacingOccurrences(of: "\"revision\":\"r\"", with: "\"revision\":\"\""),
            strictMetadata.replacingOccurrences(of: "\"revision\":\"r\"", with: "\"revision\":\"bad\\u0000id\""),
            strictMetadata.replacingOccurrences(of: "\"runtime\":\"swift\"", with: "\"runtime\":\"\""),
            strictMetadata.replacingOccurrences(of: "\"runtime\":\"swift\"", with: "\"runtime\":\"bad\\u0000id\""),
            strictMetadata.replacingOccurrences(of: "\"os\":\"test\"", with: "\"os\":\"\""),
            strictMetadata.replacingOccurrences(of: "\"os\":\"test\"", with: "\"os\":\"bad\\u0000id\""),
            strictMetadata.replacingOccurrences(of: "\"arch\":\"test\"", with: "\"arch\":\"\""),
            strictMetadata.replacingOccurrences(of: "\"arch\":\"test\"", with: "\"arch\":\"bad\\u0000id\""),
            strictMetadata.replacingOccurrences(of: "\"invocation\":[]", with: "\"invocation\":null"),
            strictMetadata.replacingOccurrences(of: "\"invocation\":[]", with: "\"invocation\":[\"bad\\u0000arg\"]"),
            strictMetadata.replacingOccurrences(of: "\"profile\":\"release\"", with: "\"profile\":null"),
            strictMetadata.replacingOccurrences(of: "\"profile\":\"release\"", with: "\"profile\":\"\""),
            strictMetadata.replacingOccurrences(of: "\"profile\":\"release\"", with: "\"profile\":\"bad\\u0000id\"")
        ] {
            XCTAssertThrowsError(try replayReport(invalid))
        }
        XCTAssertThrowsError(try replayReport(strictMetadata.replacingOccurrences(
            of: "\"arch\":\"test\"", with: "\"arch\":null"
        )))
        XCTAssertThrowsError(try replayReport(strictMetadata.replacingOccurrences(
            of: ",\"arch\":\"test\"", with: ""
        )), "missing one of the six required metadata keys must fail")
    }

    private struct Fixture: Decodable {
        let confidencePercent: Int64
        let minimumFactor: Double
        let cases: [FixtureCase]
        enum CodingKeys: String, CodingKey {
            case confidencePercent = "confidence_percent"
            case minimumFactor = "minimum_factor"
            case cases
        }
    }

    private struct FixtureCase: Decodable {
        let name: String
        let baseline: Int64
        let candidate: [Int64]
        let ratio: Double
        let interval: [Double]?
        let verdict: String
        enum CodingKeys: String, CodingKey {
            case name
            case baseline = "baseline_ns"
            case candidate = "candidate_ns"
            case ratio
            case interval
            case verdict
        }
    }

    private enum TestFailure: Error {
        case callback
    }
}

private var expectedOperatingSystemName: String {
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

private extension CommandLineResult {
    var isHelp: Bool {
        if case .help = self { return true }
        return false
    }
}
