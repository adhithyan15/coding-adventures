import Foundation
import XCTest
@testable import Rng

// ============================================================================
// ConformanceTests — the shared rng corpus (spec rng.md, "Conformance corpus")
// ============================================================================
//
// code/specs/fixtures/rng-v1/cases.txt is the flat form of the corpus every
// language's rng package must reproduce. Its results come from the
// independent reference code/scripts/rng_conformance.py. Each line is
// whitespace-separated:
//
//     case <id> <generator> <seed>
//     next_u32 <result>
//     next_u64 <result>
//     next_float <16 hex digits of the IEEE-754 bits>
//     next_int_in_range <min> <max> <result | error>
//     end

/// One generator of any of the three kinds, so the driver can be generic.
private enum AnyRng {
    case lcg(LCG), xorshift(Xorshift64), pcg(PCG32)

    init?(_ name: String, _ seed: UInt64) {
        switch name {
        case "lcg": self = .lcg(LCG(seed: seed))
        case "xorshift64": self = .xorshift(Xorshift64(seed: seed))
        case "pcg32": self = .pcg(PCG32(seed: seed))
        default: return nil
        }
    }

    mutating func u32() -> UInt32 {
        switch self {
        case .lcg(var g): defer { self = .lcg(g) }; return g.nextU32()
        case .xorshift(var g): defer { self = .xorshift(g) }; return g.nextU32()
        case .pcg(var g): defer { self = .pcg(g) }; return g.nextU32()
        }
    }

    mutating func u64() -> UInt64 {
        switch self {
        case .lcg(var g): defer { self = .lcg(g) }; return g.nextU64()
        case .xorshift(var g): defer { self = .xorshift(g) }; return g.nextU64()
        case .pcg(var g): defer { self = .pcg(g) }; return g.nextU64()
        }
    }

    mutating func f64() -> Double {
        switch self {
        case .lcg(var g): defer { self = .lcg(g) }; return g.nextFloat()
        case .xorshift(var g): defer { self = .xorshift(g) }; return g.nextFloat()
        case .pcg(var g): defer { self = .pcg(g) }; return g.nextFloat()
        }
    }

    mutating func inRange(_ min: Int64, _ max: Int64) -> Int64? {
        switch self {
        case .lcg(var g): defer { self = .lcg(g) }; return g.tryIntInRange(min: min, max: max)
        case .xorshift(var g):
            defer { self = .xorshift(g) }; return g.tryIntInRange(min: min, max: max)
        case .pcg(var g): defer { self = .pcg(g) }; return g.tryIntInRange(min: min, max: max)
        }
    }
}

final class ConformanceTests: XCTestCase {
    private func corpus() throws -> String {
        // Tests/RngTests/ConformanceTests.swift → up six levels is code/.
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<6 { url.deleteLastPathComponent() }
        url.appendPathComponent("specs/fixtures/rng-v1/cases.txt")
        return try String(contentsOf: url, encoding: .utf8)
    }

    func testEveryCorpusCase() throws {
        var cases = 0, calls = 0
        var id = "", rng: AnyRng? = nil
        for line in try corpus().split(separator: "\n") where !line.hasPrefix("#") {
            let t = line.split(separator: " ").map(String.init)
            switch t[0] {
            case "case":
                id = t[1]
                rng = AnyRng(t[2], UInt64(t[3])!)
                XCTAssertNotNil(rng, "\(id): unknown generator")
                cases += 1
            case "end":
                rng = nil
            case "next_u32":
                calls += 1
                XCTAssertEqual(rng!.u32(), UInt32(t[1])!, id)
            case "next_u64":
                calls += 1
                XCTAssertEqual(rng!.u64(), UInt64(t[1])!, id)
            case "next_float":
                calls += 1
                XCTAssertEqual(rng!.f64().bitPattern, UInt64(t[1], radix: 16)!, id)
            case "next_int_in_range":
                calls += 1
                let got = rng!.inRange(Int64(t[1])!, Int64(t[2])!)
                if t[3] == "error" {
                    XCTAssertNil(got, id)
                } else {
                    XCTAssertEqual(got, Int64(t[3])!, id)
                }
            default:
                XCTFail("malformed corpus line: \(line)")
            }
        }
        // Guard against a truncated or empty file passing vacuously.
        XCTAssertGreaterThanOrEqual(cases, 27)
        XCTAssertGreaterThanOrEqual(calls, 150)
    }

    // ── Regressions for the v1 range bug ────────────────────────────────────

    func testWholeInt64DomainNoLongerTraps() {
        var g = LCG(seed: 3)
        let values = (0..<64).map { _ in g.nextIntInRange(min: .min, max: .max) }
        XCTAssertTrue(values.contains { $0 < 0 } && values.contains { $0 > 0 })
    }

    func testWideRangesReachBeyondTwoToThe32() {
        var g = PCG32(seed: 3)
        XCTAssertTrue((0..<64).contains { _ in g.nextIntInRange(min: 0, max: 1 << 40) >= 1 << 32 })
    }

    func testHalfDomainPlusOneTerminates() {
        var g = Xorshift64(seed: 3)
        for _ in 0..<100 {
            XCTAssertLessThanOrEqual(g.nextIntInRange(min: .min, max: 0), 0)
        }
    }

    func testTryIntInRangeRejectsWithoutDrawing() {
        var g = LCG(seed: 9), fresh = LCG(seed: 9)
        XCTAssertNil(g.tryIntInRange(min: 1, max: 0))
        XCTAssertEqual(g.nextU32(), fresh.nextU32())
    }
}
