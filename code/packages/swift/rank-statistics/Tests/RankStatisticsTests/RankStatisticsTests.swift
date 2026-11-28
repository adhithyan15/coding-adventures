import XCTest
@testable import RankStatistics

final class RankStatisticsTests: XCTestCase {
    func testMedianAndInputPreservation() throws {
        let values = [9.0, 1.0, 4.0, 7.0]
        XCTAssertEqual(try median(values), 5.5)
        XCTAssertEqual(values, [9.0, 1.0, 4.0, 7.0])
        XCTAssertEqual(try median([7, 2, 4]), 4)
    }

    func testMidpointHandlesExtremeAndSubnormalValues() throws {
        let tiny = Double.leastNonzeroMagnitude
        XCTAssertEqual(try median([tiny, 2 * tiny]), 2 * tiny)
        XCTAssertEqual(try median([-Double.greatestFiniteMagnitude, Double.greatestFiniteMagnitude]), 0)
        XCTAssertEqual(try median([Double.greatestFiniteMagnitude, Double.greatestFiniteMagnitude]), Double.greatestFiniteMagnitude)
    }

    func testRejectsEmptyNonFiniteAndOverLimitSamples() {
        XCTAssertThrowsError(try median([])) { XCTAssertEqual($0 as? RankStatisticsError, .emptySample) }
        XCTAssertThrowsError(try median([1, .infinity])) { XCTAssertEqual($0 as? RankStatisticsError, .nonFiniteSample(1)) }
        XCTAssertThrowsError(try medianInterval([], confidencePercent: 95)) {
            XCTAssertEqual($0 as? RankStatisticsError, .emptySample)
        }
        XCTAssertThrowsError(try medianInterval([1], confidencePercent: 0)) {
            XCTAssertEqual($0 as? RankStatisticsError, .invalidConfidence(0))
        }
        XCTAssertThrowsError(try medianInterval(Array(repeating: 1, count: 10_001), confidencePercent: 95)) {
            XCTAssertEqual($0 as? RankStatisticsError, .tooManyObservations(10_001))
        }
    }

    func testErrorDescriptionsRemainActionable() {
        XCTAssertEqual(RankStatisticsError.emptySample.errorDescription, "values must not be empty")
        XCTAssertEqual(RankStatisticsError.tooManyObservations(10_001).errorDescription,
                       "sample has 10001 observations; maximum is 10000")
        XCTAssertEqual(RankStatisticsError.invalidConfidence(0).errorDescription,
                       "confidence must be from 1 through 99, got 0")
        XCTAssertEqual(RankStatisticsError.nonFiniteSample(2).errorDescription,
                       "sample at index 2 must be finite")
        XCTAssertEqual(RankStatisticsError.arithmeticInvariant.errorDescription,
                       "exact binomial recurrence lost divisibility")
    }

    func testExactIntervalsMatchExhaustiveSignAssignments() throws {
        for count in 1...10 {
            let values = (0..<count).map(Double.init)
            for confidence in 1...99 {
                let expected = exhaustiveInterval(count: count, confidence: confidence)
                let actual = try medianInterval(values, confidencePercent: confidence)
                XCTAssertEqual(actual, expected.map { MedianInterval(low: values[$0 - 1], high: values[count - $0]) },
                               "n=\(count), confidence=\(confidence)")
            }
        }
    }

    func testMultiLimbCutoffsMatchFrozenIndependentVectors() throws {
        let vectors: [(Int, Int, Int)] = [
            (64, 95, 24), (129, 90, 55), (129, 99, 50),
            (257, 95, 113), (1024, 50, 501), (1024, 99, 471),
            (10_000, 95, 4_902),
        ]
        for (count, confidence, rank) in vectors {
            let values = (0..<count).map(Double.init)
            XCTAssertEqual(try medianInterval(values, confidencePercent: confidence),
                           MedianInterval(low: values[rank - 1], high: values[count - rank]))
        }
    }

    func testSharedFixtureAndPackageCopy() throws {
        let packageRoot = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .deletingLastPathComponent()
        let localURL = packageRoot.appendingPathComponent("testdata/microbenchmark-pairs-v1.json")
        let canonicalURL = packageRoot.appendingPathComponent("../../../specs/fixtures/microbenchmark-pairs-v1.json")
        let localData = try Data(contentsOf: localURL)
        XCTAssertEqual(localData, try Data(contentsOf: canonicalURL))
        let fixture = try JSONDecoder().decode(Fixture.self, from: localData)

        for example in fixture.cases {
            let ratios = example.candidate.map { Double($0) / Double(example.baseline) }
            XCTAssertEqual(try median(ratios), example.ratio, example.name)
            let interval = try medianInterval(ratios, confidencePercent: fixture.confidencePercent)
            XCTAssertEqual(interval?.low, example.interval?.first, example.name)
            XCTAssertEqual(interval?.high, example.interval?.last, example.name)
        }
    }

    func testSeededPropertiesForMedianAndInterval() throws {
        var state: UInt64 = 0x4d454449414e
        for _ in 0..<300 {
            state ^= state << 13
            state ^= state >> 7
            state ^= state << 17
            let count = Int(state % 48) + 1
            var values: [Double] = []
            for _ in 0..<count {
                state ^= state << 13
                state ^= state >> 7
                state ^= state << 17
                values.append(Double(Int(state % 20_001) - 10_000))
            }
            let original = values
            let confidence = Int(state % 99) + 1
            let interval = try medianInterval(values, confidencePercent: confidence)
            values.reverse()
            XCTAssertEqual(try median(values), try median(original))
            XCTAssertEqual(Array(values.reversed()), original)
            if let interval {
                XCTAssertTrue(original.contains(interval.low))
                XCTAssertTrue(original.contains(interval.high))
                XCTAssertLessThanOrEqual(interval.low, interval.high)
            }
        }
    }

    private func exhaustiveInterval(count: Int, confidence: Int) -> Int? {
        let assignmentCount = 1 << count
        let cutoff = assignmentCount * (100 - confidence)
        var bestRank: Int?
        for rank in 1...count {
            var tailAssignments = 0
            for assignment in 0..<assignmentCount where assignment.nonzeroBitCount < rank {
                tailAssignments += 1
            }
            if 200 * tailAssignments <= cutoff {
                bestRank = rank
            } else {
                break
            }
        }
        return bestRank
    }

    private struct Fixture: Decodable {
        let confidencePercent: Int
        let cases: [FixtureCase]

        enum CodingKeys: String, CodingKey {
            case confidencePercent = "confidence_percent"
            case cases
        }
    }

    private struct FixtureCase: Decodable {
        let name: String
        let baseline: Int64
        let candidate: [Int64]
        let ratio: Double
        let interval: [Double]?

        enum CodingKeys: String, CodingKey {
            case name
            case baseline = "baseline_ns"
            case candidate = "candidate_ns"
            case ratio
            case interval
        }
    }
}
