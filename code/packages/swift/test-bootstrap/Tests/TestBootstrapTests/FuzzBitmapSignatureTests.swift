import XCTest
@testable import TestBootstrap

final class FuzzBitmapSignatureTests: XCTestCase {
    // MARK: bitmap -------------------------------------------------------------------------

    func testRecordHasMasksModuloMapSizeAndEdgesAreSortedAndCounted() {
        let m = Bitmap()
        XCTAssertEqual(m.count, 0)
        m.record(5); m.record(5); m.record(65_536 + 5); m.record(UInt64.max); m.record(3)
        XCTAssertTrue(m.has(5)); XCTAssertTrue(m.has(65_536 * 9 + 5)); XCTAssertFalse(m.has(4))
        XCTAssertEqual(m.edges(), [3, 5, 65_535])
        XCTAssertEqual(m.trace(), [5, 65_535, 3]) // first-seen order
        XCTAssertEqual(m.count, 3)
        XCTAssertEqual(m.raw().count, 8192)
        XCTAssertEqual(m.raw()[0], 0b0010_1000) // edges 3 and 5, LSB first
        XCTAssertEqual(m.raw()[8191], 0b1000_0000) // edge 65,535
        m.reset()
        XCTAssertEqual(m.count, 0)
        XCTAssertTrue(m.raw().allSatisfy { $0 == 0 })
        XCTAssertFalse(m.has(5))
    }

    func testAdmitCountsNovelEdgesAndMergeUnions() {
        let m = Bitmap()
        var a = m.admit([1, 2, 3])
        XCTAssertTrue(a.hasNew); XCTAssertEqual(a.countNew, 3)
        a = m.admit([1, 2])
        XCTAssertFalse(a.hasNew); XCTAssertEqual(a.countNew, 0)
        a = m.admit([3, 9, 9])
        XCTAssertEqual(a.countNew, 1)
        let o = Bitmap()
        o.record(100); o.record(1)
        m.merge(o)
        XCTAssertEqual(m.edges(), [1, 2, 3, 9, 100])
    }

    func testCorpusAdmitCountsUniqueNovelEdgesOverFullWords() {
        var n = Bitmap.corpusAdmit(existing: [1, 2, 3], trace: [2, 4])
        XCTAssertTrue(n.admitted); XCTAssertEqual(n.newEdges, 1)
        n = Bitmap.corpusAdmit(existing: [1, 2, 3], trace: [1, 2])
        XCTAssertFalse(n.admitted); XCTAssertEqual(n.newEdges, 0)
        n = Bitmap.corpusAdmit(existing: [1], trace: [7, 7, 7, 1])
        XCTAssertEqual(n.newEdges, 1) // a repeated novel edge counts once
        n = Bitmap.corpusAdmit(existing: [1], trace: [1 + 65_536, UInt64.max])
        XCTAssertEqual(n.newEdges, 2) // full words, not reduced mod 65,536
        XCTAssertFalse(Bitmap.corpusAdmit(existing: [], trace: []).admitted)
    }

    // MARK: the power schedule (fuzz-v1 pins no value) -----------------------------------------

    /// Rows computed by an independent Python evaluation of
    /// floor(max(1,base) * (1 + new/total or 1) / sqrt(max(1,len)) + 0.5). Rows
    /// marked TIE land exactly on .5, where round-half-even would differ.
    func testEnergyRowsIncludingHalfWayTies() {
        let rows: [(Int, Int, Int, Int, Int)] = [
            (10, 4, 3, 6, 8),   // TIE 7.5
            (5, 4, 0, 0, 3),    // TIE 2.5
            (10, 1, 5, 0, 10),
            (0, 1_000_000, 1, 1, 1),
            (10, 0, 1, 1, 20),
            (10, 100, 1, 2, 2), // TIE 1.5
            (10, 9, 2, 4, 5),
            (10, 16, 1, 3, 3),
            (7, 2, 1, 2, 7),
            (100, 25, 5, 10, 30),
            (10, 1, 6, 6, 20),
            (3, 1, 1, 2, 5),    // TIE 4.5
            (1, 1, 0, 5, 1),
            (10, 2, 1, 1, 14),
            (5, 1, 1, 2, 8),    // TIE 7.5
            (1, 1, 1, 1, 2),
        ]
        for (base, len, new, total, want) in rows {
            XCTAssertEqual(Bitmap.calculateEnergy(base: base, seedLen: len, newEdges: new, totalEdges: total), want, "\((base, len, new, total))")
        }
    }

    func testEnergyClampsAndIsMonotoneAndNeverTraps() {
        XCTAssertEqual(Bitmap.calculateEnergy(base: Int.max, seedLen: 1, newEdges: 1, totalEdges: 1), Mutate.maxIterations)
        XCTAssertEqual(Bitmap.calculateEnergy(base: Int.min, seedLen: Int.min, newEdges: Int.min, totalEdges: Int.max), 1)
        XCTAssertEqual(Bitmap.calculateEnergy(base: 10, seedLen: 4, newEdges: -100, totalEdges: 3), 1)
        XCTAssertGreaterThan(Bitmap.calculateEnergy(base: 10, seedLen: 4, newEdges: 6, totalEdges: 6), Bitmap.calculateEnergy(base: 10, seedLen: 4, newEdges: 0, totalEdges: 6))
        XCTAssertLessThan(Bitmap.calculateEnergy(base: 10, seedLen: 100, newEdges: 1, totalEdges: 2), Bitmap.calculateEnergy(base: 10, seedLen: 1, newEdges: 1, totalEdges: 2))
    }

    // MARK: signatures ------------------------------------------------------------------------------

    func testSignaturesArePinnedAgainstThePythonOracle() {
        // Computed by code/scripts/fuzz_conformance.py crash_signature (independent of this lane).
        let pins: [(String, [String], String)] = [
            ("panic", ["eval", "step"], "4eca7ae8"), ("timeout", [], "4e61ea76"), ("oom", [], "7146a410"),
            ("invalid", [], "43a76e9e"), ("k", ["f"], "35a55e82"),
            ("TestBootstrapTests.Boom", ["parse", "run"], "8ff4dc91"), ("x", ["a:b"], "557f548e"), ("x", ["a", "b"], "557f548e"),
            ("stale-budget-sentinel", [], "8a20b3bf"),
        ]
        for (k, f, want) in pins { XCTAssertEqual(FuzzSignature.crashSignature(kind: k, frames: f), want, "\(k) \(f)") }
        XCTAssertEqual(FuzzSignature.hex8(0xABCDEF), "00abcdef")
        XCTAssertEqual(FuzzSignature.hex8(0), "00000000")
    }

    func testDedupSeparatesKindsFramesAndDepth() {
        XCTAssertTrue(FuzzSignature.isDuplicateCrash(kindA: "panic", framesA: ["eval", "step"], kindB: "panic", framesB: ["eval", "step"]))
        XCTAssertFalse(FuzzSignature.isDuplicateCrash(kindA: "panic", framesA: ["eval", "step"], kindB: "segv", framesB: ["alloc", "write"]))
        XCTAssertFalse(FuzzSignature.isDuplicateCrash(kindA: "panic", framesA: ["a"], kindB: "panic", framesB: ["a", "b"]))
        XCTAssertFalse(FuzzSignature.isDuplicateCrash(kindA: "panic", framesA: ["a", "b"], kindB: "panic", framesB: ["b", "a"]))
    }

    func testFailureVocabularyIsClosed() {
        XCTAssertEqual(FuzzFailureIds.all, ["fuzz-crash", "fuzz-timeout", "fuzz-oom", "fuzz-leak", "fuzz-panic"])
        for id in FuzzFailureIds.all { XCTAssertTrue(FuzzFailureIds.isFuzzFailureId(id)) }
        for bad in ["fuzz-arbitrary-error", "", "FUZZ-CRASH", "fuzz-crash ", "property-panic", "fuzz-crash\n", "fuzz-crash\u{301}"] {
            XCTAssertFalse(FuzzFailureIds.isFuzzFailureId(bad), bad)
        }
        XCTAssertFalse(FuzzFailureIds.isFuzzFailureId(nil))
    }

    // MARK: TAP -------------------------------------------------------------------------------------------

    func testTapLineIsOpaqueExactAndRefusesBadArguments() throws {
        XCTAssertEqual(try FuzzSignature.formatTapFailure(1, target: "ct_compare", failureId: "fuzz-crash", sig: "3a8f19b2"),
                       "not ok 1 - ct_compare [fuzz-crash:3a8f19b2]")
        XCTAssertEqual(try FuzzSignature.formatTapFailure(42, target: "a/b.c-d_e", failureId: "fuzz-oom", sig: "00000000"),
                       "not ok 42 - a/b.c-d_e [fuzz-oom:00000000]")
        XCTAssertEqual(try FuzzSignature.formatTapFailure(-3, target: "", failureId: "fuzz-leak", sig: "00000000"), "not ok -3 -  [fuzz-leak:00000000]")
        XCTAssertThrowsError(try FuzzSignature.formatTapFailure(1, target: "t", failureId: "fuzz-bogus", sig: "3a8f19b2"))
        for bad in ["3A8F19B2", "3a8f19b", "3a8f19b22", "", "3a8f19b\n", "zzzzzzzz", "3a8f19b2\n", "3a8f19b\u{301}"] {
            XCTAssertThrowsError(try FuzzSignature.formatTapFailure(1, target: "t", failureId: "fuzz-crash", sig: bad), bad)
        }
    }

    func testTapTargetCannotForgeALine() throws {
        let line = try FuzzSignature.formatTapFailure(1, target: "evil\nok 2 - forged [x]\r\n", failureId: "fuzz-crash", sig: "3a8f19b2")
        XCTAssertFalse(line.contains("\n")); XCTAssertFalse(line.contains("\r"))
        XCTAssertTrue(line.hasPrefix("not ok 1 - evil_ok_2_-_forged__x___"))
        XCTAssertEqual(try FuzzSignature.formatTapFailure(1, target: "a\u{e9}b", failureId: "fuzz-crash", sig: "3a8f19b2"), "not ok 1 - a__b [fuzz-crash:3a8f19b2]")
        XCTAssertEqual(try FuzzSignature.formatTapFailure(1, target: String(repeating: "x", count: 500), failureId: "fuzz-crash", sig: "3a8f19b2"),
                       "not ok 1 - " + String(repeating: "x", count: 128) + " [fuzz-crash:3a8f19b2]")
        XCTAssertEqual(try FuzzSignature.formatTapFailure(1, target: String(repeating: "\u{e9}", count: 100), failureId: "fuzz-crash", sig: "3a8f19b2"),
                       "not ok 1 - " + String(repeating: "_", count: 128) + " [fuzz-crash:3a8f19b2]")
        let huge = try FuzzSignature.formatTapFailure(1, target: String(repeating: "y", count: 4_000_000), failureId: "fuzz-crash", sig: "3a8f19b2")
        XCTAssertEqual(huge.utf8.count, "not ok 1 - ".utf8.count + 128 + " [fuzz-crash:3a8f19b2]".utf8.count)
    }

    func testTapFailuresRenderEveryBucketWithoutPayload() {
        let a = CrashRecord(signature: "aaaaaaaa", kind: "k", frames: ["f"], minimal: [0xde, 0xad], minimized: true, inputDropped: false)
        let b = CrashRecord(signature: "bbbbbbbb", kind: "k", frames: ["g"], minimal: [1], minimized: true, inputDropped: false)
        let r = FuzzReport(iterations: 5, admittedSeeds: 1, totalEdges: 1, crashes: [a, b], timeouts: 2, ooms: 3, crashesDropped: 0,
                           admissionsDropped: 0, coverageTier: FuzzReport.tierReported, corpus: [[1]])
        let lines = FuzzSignature.tapFailures(r, target: "tgt", firstTest: 10)
        XCTAssertEqual(lines, ["not ok 10 - tgt [fuzz-crash:aaaaaaaa]", "not ok 11 - tgt [fuzz-crash:bbbbbbbb]",
                               "not ok 12 - tgt [fuzz-timeout:4e61ea76]", "not ok 13 - tgt [fuzz-oom:7146a410]"])
        for l in lines { XCTAssertFalse(l.contains("dead")) }
        let clean = FuzzReport(iterations: 5, admittedSeeds: 1, totalEdges: 1, crashes: [], timeouts: 0, ooms: 0, crashesDropped: 0,
                               admissionsDropped: 0, coverageTier: FuzzReport.tierReported, corpus: [])
        XCTAssertEqual(FuzzSignature.tapFailures(clean, target: "tgt", firstTest: 1), [])
        // A hostile first-test number cannot trap the counter.
        XCTAssertEqual(FuzzSignature.tapFailures(r, target: "t", firstTest: Int.max).count, 4)
    }

    func testTapFailuresAlwaysEmitALineEvenForAMalformedSignature() {
        let bad = CrashRecord(signature: "NOT-HEX!", kind: "k", frames: ["f"], minimal: [], minimized: true, inputDropped: false)
        let r = FuzzReport(iterations: 1, admittedSeeds: 1, totalEdges: 1, crashes: [bad], timeouts: 0, ooms: 0, crashesDropped: 0,
                           admissionsDropped: 0, coverageTier: FuzzReport.tierBlackBox, corpus: [])
        XCTAssertEqual(FuzzSignature.tapFailures(r, target: "t", firstTest: 1), ["not ok 1 - t [fuzz-crash:43a76e9e]"])
    }

    // MARK: kind and frame normalisation ------------------------------------------------------------------

    func testKindNormalisationStripsAddressesAndSanitises() {
        XCTAssertEqual(FuzzSignature.normaliseKind("TestBootstrapTests.Boom"), "TestBootstrapTests.Boom")
        XCTAssertEqual(FuzzSignature.normaliseKind("Mod.(unknown context at $104a3c1f8).Boom"), "Mod._local_.Boom")
        XCTAssertEqual(FuzzSignature.normaliseKind("A.(unknown context at $1).B.(unknown context at $2).C"), "A._local_.B._local_.C")
        XCTAssertEqual(FuzzSignature.normaliseKind("Mod.(unknown context at $dead"), "Mod._local_")
        XCTAssertEqual(FuzzSignature.normaliseKind("a b:c\nd"), "a_b_c_d")
        XCTAssertEqual(FuzzSignature.normaliseKind(String(repeating: "x", count: 200)).utf8.count, 64)
        XCTAssertEqual(FuzzSignature.normaliseKind(String(repeating: "x", count: 100_000)).utf8.count, 64)
        XCTAssertFalse(FuzzSignature.normaliseKind("Box<Int>:x").contains(":"))
        XCTAssertEqual(FuzzSignature.normaliseFrame(""), "_")
        XCTAssertEqual(FuzzSignature.normaliseFrame("a:b\nc"), "a_b_c")
        XCTAssertEqual(FuzzSignature.normaliseFrame(String(repeating: "f", count: 500)).utf8.count, 128)
        XCTAssertEqual(FuzzSignature.collapse(["a", "a", "b", "b", "b", "a"]), ["a", "b", "a"])
        XCTAssertEqual(FuzzSignature.collapse((0..<500).map { "f\($0)" }).count, 64)
    }
}
