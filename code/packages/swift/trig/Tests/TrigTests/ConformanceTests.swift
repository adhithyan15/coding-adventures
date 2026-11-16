// ============================================================================
// ConformanceTests.swift — bit-exact conformance against TRIG-REF-1
// ============================================================================
//
// code/specs/fixtures/trig-reference-v1 holds every vector as one row
//
//     op <TAB> x <TAB> y <TAB> cr <TAB> flags
//
// where x and y are IEEE binary64 BIT PATTERNS (16 lowercase hex digits), never
// decimals (PHY00a 6.2).  For each row this test
//
//   (a) decodes x by a bit cast (`Double(bitPattern:)`), never by parsing text,
//   (b) calls the PUBLIC function,
//   (c) encodes the result back to bits and compares the two integers, except
//       that y = 7ff8000000000000 matches ANY NaN (VER04p's `same` relation),
//   (d) then checks accuracy against `cr`, the correctly rounded value.
//
// Printed floats are never compared: they hide the last digits.
//
// How the `expect = error` rows are handled (the Swift decision)
// ---------------------------------------------------------------
// The vector file lists `sqrt` of a negative number (and of -inf) with the
// result word `error`.  `Trig.sqrt` signals that with `preconditionFailure`,
// which TRAPS the whole process: XCTest has no way to catch it (survey
// PHY00a 8.1.5 and the Swift row of 8.1.7).  So those rows cannot be asserted
// against the trapping function.  We do NOT skip them.  The package gains a
// non-trapping twin, `Trig.sqrtChecked(_:) throws`, which throws
// `TrigError.negativeSquareRoot` for exactly the inputs where `Trig.sqrt`
// traps, and the `error` rows are asserted against it.  `Trig.sqrt` itself is
// unchanged (it still traps).  The test counts the error rows and requires
// that the count is nonzero, so the rows can never silently disappear.

import XCTest
@testable import Trig

/// One parsed row of a vector file.
private struct Vector {
    let group: String
    let op: String
    let x: String
    let y: String
    let cr: String
    let flags: String
}

private let groups = ["edge", "sweep-binade", "sweep-cw", "sweep-prng"]
private let qnanBits: UInt64 = 0x7FF8_0000_0000_0000

/// The directory that holds the shared vectors, found relative to THIS file
/// (`#filePath`), so the test works from any checkout and any build directory.
private func fixtureDirectory() throws -> URL {
    var directory = URL(fileURLWithPath: #filePath).deletingLastPathComponent()
    let manager = FileManager.default
    while directory.path != "/" {
        let candidate = directory.appendingPathComponent(
            "code/specs/fixtures/trig-reference-v1")
        if manager.fileExists(atPath: candidate.appendingPathComponent("edge.tsv").path) {
            return candidate
        }
        directory.deleteLastPathComponent()
    }
    throw NSError(domain: "TrigConformance", code: 1, userInfo: [
        NSLocalizedDescriptionKey: "could not locate code/specs/fixtures/trig-reference-v1"])
}

private func loadVectors() throws -> [Vector] {
    let dir = try fixtureDirectory()
    var rows: [Vector] = []
    for group in groups {
        let text = try String(contentsOf: dir.appendingPathComponent("\(group).tsv"),
                              encoding: .utf8)
        precondition(text.hasSuffix("\n"), "\(group).tsv must end with a newline")
        for line in text.split(separator: "\n", omittingEmptySubsequences: true) {
            let f = line.split(separator: "\t", maxSplits: 4, omittingEmptySubsequences: false)
            precondition(f.count == 5, "bad row in \(group).tsv: \(line)")
            rows.append(Vector(group: group, op: String(f[0]), x: String(f[1]),
                               y: String(f[2]), cr: String(f[3]), flags: String(f[4])))
        }
    }
    return rows
}

/// Hex digits to a Double by a bit cast.
private func fromBits(_ hex: Substring) -> Double {
    Double(bitPattern: UInt64(hex, radix: 16)!)
}

/// Run one vector through the public API.  `atan2` has two inputs: "<y>,<x>".
private func call(_ op: String, _ x: String) -> Double {
    if op == "atan2" {
        let parts = x.split(separator: ",")
        return Trig.atan2(fromBits(parts[0]), fromBits(parts[1]))
    }
    let v = fromBits(Substring(x))
    switch op {
    case "sin": return Trig.sin(v)
    case "cos": return Trig.cos(v)
    case "tan": return Trig.tan(v)
    case "radians": return Trig.radians(v)
    case "degrees": return Trig.degrees(v)
    case "sqrt": return Trig.sqrt(v)
    case "atan": return Trig.atan(v)
    default: fatalError("unknown op \(op)")
    }
}

/// Map a double's bits to an integer whose order is the numeric order, so the
/// distance between two results is a count of representable steps (VER04p 5.4).
private func ordered(_ bits: UInt64) -> Int64 {
    let magnitude = Int64(bits & 0x7FFF_FFFF_FFFF_FFFF)
    return (bits >> 63) != 0 ? -magnitude : magnitude
}

final class ConformanceTests: XCTestCase {

    private static let vectors: [Vector] = {
        do { return try loadVectors() } catch { fatalError("\(error)") }
    }()

    /// Compare one group bit for bit; report failing rows per op.
    private func checkGroup(_ group: String) throws {
        var checked = 0
        var errorRows = 0
        var failuresByOp: [String: Int] = [:]
        var first: [String] = []

        for v in Self.vectors where v.group == group {
            checked += 1
            var ok = true
            var detail = ""

            if v.y == "error" {
                // `sqrt` of a negative: asserted against the non-trapping twin.
                errorRows += 1
                precondition(v.op == "sqrt", "only sqrt has error rows")
                do {
                    let r = try Trig.sqrtChecked(fromBits(Substring(v.x)))
                    ok = false
                    detail = "no error, got \(String(r.bitPattern, radix: 16))"
                } catch is TrigError {
                    ok = true
                } catch {
                    ok = false
                    detail = "wrong error kind \(error)"
                }
            } else {
                let got = call(v.op, v.x)
                let want = UInt64(v.y, radix: 16)!
                if want == qnanBits {
                    ok = got.isNaN
                    detail = "want NaN, got \(String(got.bitPattern, radix: 16))"
                } else {
                    ok = got.bitPattern == want
                    detail = "want \(v.y), got \(String(got.bitPattern, radix: 16))"
                }
            }
            if !ok {
                failuresByOp[v.op, default: 0] += 1
                if first.count < 8 { first.append("\(v.op) \(v.x): \(detail)") }
            }
        }

        XCTAssertGreaterThan(checked, 0)
        if group == "edge" {
            // Five error rows today (four negatives and -inf); never zero.
            XCTAssertGreaterThan(errorRows, 0, "the expect=error rows must be exercised")
        }
        let summary = failuresByOp.sorted { $0.key < $1.key }
            .map { "\($0.key)=\($0.value)" }.joined(separator: " ")
        print("CONFORMANCE \(group): rows=\(checked) errorRows=\(errorRows) failing[\(summary)]")
        XCTAssertTrue(failuresByOp.isEmpty,
                      "\(group): failing rows per op: \(summary); first: \(first)")
    }

    func testEdgeVectorsMatchBitForBit() throws { try checkGroup("edge") }
    func testSweepBinadeVectorsMatchBitForBit() throws { try checkGroup("sweep-binade") }
    func testSweepCwVectorsMatchBitForBit() throws { try checkGroup("sweep-cw") }
    func testSweepPrngVectorsMatchBitForBit() throws { try checkGroup("sweep-prng") }

    /// Every row with a `cr` is within 3 steps (sin, cos) or 1 step (radians,
    /// degrees) of the correctly rounded value.  `tan` is a no-claim op (PHY00a
    /// 3.8: sin/cos plus a division and a pinned pole sentinel); it carries a
    /// `cr` on most rows but is skipped BY NAME.  Any other op that has a `cr`
    /// and no stated limit fails closed, and so does an unknown op.
    func testAccuracyAgainstTheCorrectlyRoundedColumn() throws {
        let limit: [String: Int64] = ["sin": 3, "cos": 3, "radians": 1, "degrees": 1]
        let noClaim: Set<String> = ["tan"]
        let known: Set<String> = ["sin", "cos", "tan", "radians", "degrees",
                                  "sqrt", "atan", "atan2"]
        var worst: [String: Int64] = [:]
        var measured = 0
        for v in Self.vectors {
            XCTAssertTrue(known.contains(v.op), "unknown op \(v.op)")
            if v.cr == "-" || noClaim.contains(v.op) { continue }
            guard limit[v.op] != nil else {
                XCTFail("op \(v.op) has a cr value but no accuracy limit")
                continue
            }
            let got = call(v.op, v.x).bitPattern
            // Subtraction can overflow Int64 for a NaN-vs-number distance; treat
            // that as "infinitely far".
            let (diff, overflow) = ordered(got).subtractingReportingOverflow(
                ordered(UInt64(v.cr, radix: 16)!))
            let steps = overflow ? Int64.max : (diff == Int64.min ? Int64.max : abs(diff))
            worst[v.op] = max(worst[v.op] ?? 0, steps)
            measured += 1
        }
        XCTAssertGreaterThan(measured, 0)
        for (op, bound) in limit {
            XCTAssertLessThanOrEqual(worst[op] ?? 0, bound, "\(op) worst steps \(worst[op] ?? -1)")
        }
        print("ACCURACY worst steps vs cr: \(worst.sorted { $0.key < $1.key })")
    }
}
