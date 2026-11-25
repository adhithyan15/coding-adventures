import XCTest
@testable import TestBootstrap

/// Replays the committed fuzz-v1 fixture and proves the replay CAN fail.
final class FuzzCorpusConformanceTests: XCTestCase {
    override func tearDown() {
        FuzzHooks.reset()
        super.tearDown()
    }

    // MARK: the real corpus -------------------------------------------------------

    func testFixtureIsTheCommittedPathAndHasTwentyFiveCases() throws {
        XCTAssertTrue(FuzzSupport.fixtureURL().path.hasSuffix("code/specs/fixtures/fuzz-v1/cases.json"))
        XCTAssertEqual(try FuzzSupport.cases().count, 25)
    }

    func testWholeCorpusConformsExactly() throws {
        let report = FuzzCorpus.evaluate(try FuzzSupport.cases())
        XCTAssertEqual(report.problems, [])
        XCTAssertEqual(report.evaluated, 25)
    }

    func testEveryOpIsInTheFixture() throws {
        let ops = Set(try FuzzSupport.cases().map(Support.op))
        XCTAssertEqual(ops, Set(FuzzCorpus.shapes.keys))
        XCTAssertEqual(ops.count, 14)
    }

    // MARK: per-op corruption: every one of the 25 cases ------------------------------

    private func flipBytes(_ j: Json?) -> Json {
        var hex = ""
        if case .string(let h)? = j?["hex"] { hex = h }
        return Support.bytes(hex.isEmpty ? "00" : (hex.hasPrefix("00") ? "ff" : "00") + hex.dropFirst(2))
    }

    private func corruptions(of c: Json) -> [(String, Json)] {
        let op = Support.op(c)
        if op.hasPrefix("mutate_") { return [("flipped output", Support.setting(c, "output", flipBytes(c["output"])))] }
        switch op {
        case "corpus_admit":
            guard case .bool(let a)? = c["admitted"] else { return [] }
            return [("flipped admitted", Support.setting(c, "admitted", .bool(!a))),
                    ("new_edges + 1", Support.setting(c, "new_edges", Support.intPlus(c["new_edges"]!, 1)))]
        case "crash_dedup":
            guard case .bool(let d)? = c["is_duplicate"] else { return [] }
            return [("flipped is_duplicate", Support.setting(c, "is_duplicate", .bool(!d)))]
        case "failure_id_valid":
            guard case .bool(let v)? = c["valid"] else { return [] }
            return [("flipped validity", Support.setting(c, "valid", .bool(!v)))]
        case "private_reporting":
            guard case .bool(let l)? = c["contains_raw_payload"], case .string(let tap)? = c["tap_line"] else { return [] }
            return [("changed tap line", Support.setting(c, "tap_line", .string(tap + "x"))),
                    ("flipped privacy verdict", Support.setting(c, "contains_raw_payload", .bool(!l))),
                    ("wrong sig", Support.setting(c, "sig_hash", .string("00000000")))]
        case "corpus_reject":
            return [("a valid TFV in place of a reject text", Support.setting(c, "text", .string("tfv 1\n")))]
        default: return []
        }
    }

    func testEveryCorruptionOfEveryCaseFailsAndNamesExactlyThatCase() throws {
        let cases = try FuzzSupport.cases()
        var checked = 0
        for (index, c) in cases.enumerated() {
            let cid = Support.id(c)
            let corrupted = corruptions(of: c)
            XCTAssertFalse(corrupted.isEmpty, "no corruption defined for \(cid)")
            for (what, bad) in corrupted {
                var copy = cases
                copy[index] = bad
                let report = FuzzCorpus.evaluate(copy)
                XCTAssertFalse(report.problems.isEmpty, "\(cid): \(what) was not detected")
                XCTAssertTrue(report.problems.contains { $0.hasPrefix(cid + ":") }, "\(cid): \(what) not attributed: \(report.problems)")
                for p in report.problems {
                    XCTAssertTrue(p.hasPrefix(cid + ":") || p.hasPrefix("fuzz-v1:"), "\(cid): \(what) blamed another case: \(p)")
                }
                checked += 1
            }
        }
        XCTAssertGreaterThanOrEqual(checked, 25)
    }

    // MARK: dropped, truncated, padded, malformed corpora --------------------------------

    func testDroppingAnyCaseFails() throws {
        let cases = try FuzzSupport.cases()
        for i in 0..<cases.count {
            var copy = cases
            copy.remove(at: i)
            let report = FuzzCorpus.evaluate(copy)
            XCTAssertFalse(report.problems.isEmpty, "dropping \(Support.id(cases[i])) went unnoticed")
            XCTAssertTrue(report.problems.contains { $0.contains("expected 25 cases, got 24") })
        }
    }

    func testEveryTruncationFails() throws {
        let cases = try FuzzSupport.cases()
        for n in 0..<cases.count { XCTAssertFalse(FuzzCorpus.evaluate(Array(cases.prefix(n))).problems.isEmpty, "prefix \(n)") }
    }

    func testEmptyPaddedAndDuplicatedCorporaFail() throws {
        let r = FuzzCorpus.evaluate([])
        XCTAssertEqual(r.evaluated, 0)
        XCTAssertFalse(r.problems.isEmpty)
        let cases = try FuzzSupport.cases()
        XCTAssertFalse(FuzzCorpus.evaluate(cases + [cases[0]]).problems.isEmpty)
        var dup = cases
        dup[1] = Support.setting(cases[1], "id", .string(Support.id(cases[0])))
        XCTAssertTrue(FuzzCorpus.evaluate(dup).problems.contains { $0.contains("duplicate or missing case id") })
    }

    func testUnknownOpMissingAndExtraFieldsAreProblemsNotSkips() throws {
        let cases = try FuzzSupport.cases()
        let cid = Support.id(cases[0])
        var a = cases
        a[0] = Support.setting(cases[0], "op", .string("mutate_nothing"))
        XCTAssertTrue(FuzzCorpus.evaluate(a).problems.contains { $0.hasPrefix(cid + ": unknown op") })
        var b = cases
        b[0] = Support.setting(cases[0], "extra", .bool(true))
        XCTAssertTrue(FuzzCorpus.evaluate(b).problems.contains { $0.hasPrefix(cid + ": fields") })
        var c = cases
        if case .object(let pairs) = cases[0] { c[0] = .object(pairs.filter { $0.key != "count" }) }
        XCTAssertTrue(FuzzCorpus.evaluate(c).problems.contains { $0.hasPrefix(cid + ": fields") })
        var d = cases
        d[0] = .string("not an object")
        XCTAssertFalse(FuzzCorpus.evaluate(d).problems.isEmpty)
        var e = cases
        e[0] = Support.setting(cases[0], "id", .number("5"))
        XCTAssertFalse(FuzzCorpus.evaluate(e).problems.isEmpty)
    }

    func testMalformedValuesAreProblems() throws {
        let cases = try FuzzSupport.cases()
        let i = cases.firstIndex { Support.id($0) == "fuzz/mutate_bit_flip/bit0" }!
        let cid = Support.id(cases[i])
        for bad in [Support.setting(cases[i], "bit_offset", Support.int("01")),
                    Support.setting(cases[i], "bit_offset", .string("0")),
                    Support.setting(cases[i], "bit_offset", Support.int(String(repeating: "9", count: 301))),
                    Support.setting(cases[i], "input", Support.bytes("0G")),
                    Support.setting(cases[i], "input", Support.bytes("ABCD")),
                    Support.setting(cases[i], "input", Support.bytes("000")),
                    Support.setting(cases[i], "output", .string("0100"))] {
            var copy = cases
            copy[i] = bad
            XCTAssertTrue(FuzzCorpus.evaluate(copy).problems.contains { $0.hasPrefix(cid + ":") }, "\(bad)")
        }
        // An endian that is neither little nor big, a width that is not an int.
        let j = cases.firstIndex { Support.id($0) == "fuzz/mutate_arithmetic/u8_add1" }!
        for bad in [Support.setting(cases[j], "endian", .string("middle")), Support.setting(cases[j], "width", .string("1"))] {
            var copy = cases
            copy[j] = bad
            XCTAssertTrue(FuzzCorpus.evaluate(copy).problems.contains { $0.hasPrefix(Support.id(cases[j]) + ":") })
        }
        // An edge id outside [0, 2^64) is a problem, not a truncation.
        let k = cases.firstIndex { Support.id($0) == "fuzz/corpus_admit/novel_edge" }!
        var copy = cases
        copy[k] = Support.setting(cases[k], "trace_edges", .array([Support.int("18446744073709551616")]))
        XCTAssertTrue(FuzzCorpus.evaluate(copy).problems.contains { $0.hasPrefix(Support.id(cases[k]) + ":") })
    }

    func testHostileIdsAndOpsCannotForgeOrSmuggleLines() throws {
        let cases = try FuzzSupport.cases()
        var copy = cases
        let hostile = "evil\nok 2 - forged\u{1b}[31m\r" + String(repeating: "z", count: 500)
        copy[0] = Support.setting(Support.setting(cases[0], "id", .string(hostile)), "output", Support.bytes("ffff"))
        let problems = FuzzCorpus.evaluate(copy).problems
        XCTAssertFalse(problems.isEmpty)
        for p in problems {
            XCTAssertFalse(p.contains("\n") || p.contains("\r") || p.contains("\u{1b}"), p)
        }
        XCTAssertTrue(problems.contains { $0.hasPrefix("evil?ok 2 - forged?[31m?") })
        XCTAssertEqual(FuzzCorpus.escapeId(String(repeating: "a", count: 300)).count, 103)
        var op = cases
        op[0] = Support.setting(cases[0], "op", .string("x\ny"))
        for p in FuzzCorpus.evaluate(op).problems { XCTAssertFalse(p.contains("\n"), p) }
    }

    func testRealFixtureBytesParseButMutatedBytesAreRefused() throws {
        let raw = try FuzzSupport.corpusBytes()
        XCTAssertNoThrow(try FuzzCorpus.parse(raw))
        XCTAssertThrowsError(try FuzzCorpus.parse(Array("{\"schema_version\": 2, \"cases\": []}".utf8)))
        XCTAssertThrowsError(try FuzzCorpus.parse(Array("{\"schema_version\": 1}".utf8)))
        XCTAssertThrowsError(try FuzzCorpus.parse(Array("{\"schema_version\": 1, \"cases\": {}}".utf8)))
        XCTAssertThrowsError(try FuzzCorpus.parse(Array("[1]".utf8)))
        XCTAssertThrowsError(try FuzzCorpus.parse(raw + [0x20, 0x7b]))
        XCTAssertNoThrow(try FuzzCorpus.parse(Array("{\"cases\": [], \"schema_version\": 1}".utf8)))
    }

    // MARK: the aggregate work cap --------------------------------------------------------

    private func sequenceCase(_ id: String, bytes: Int, steps: Int) -> Json {
        .object([("id", .string(id)), ("op", .string("mutate_sequence")),
                 ("input", Support.bytes(String(repeating: "00", count: bytes))), ("seed", Support.int("1")),
                 ("steps", Support.int(String(steps))), ("output", Support.bytes("00"))])
    }

    func testBreachedAggregateIsZeroedSoLaterCasesFailWithoutRunning() {
        // The first call "fails" having really copied 16,000,000 bytes: a call that
        // hits its cap is charged too, and the 777,216 bytes left are ZEROED, so
        // the tiny second case (which would fit) fails at once and never runs.
        var calls = 0
        FuzzHooks.modify { t in
            t.sequence = { _, _, _, _, _ in calls += 1; throw FuzzError.workExhausted(workDone: 16_000_000) }
        }
        let report = FuzzCorpus.evaluate([sequenceCase("fuzz/seq/one", bytes: 1, steps: 1),
                                          sequenceCase("fuzz/seq/two", bytes: 1, steps: 1),
                                          sequenceCase("fuzz/seq/three", bytes: 1, steps: 1)], expected: -1)
        XCTAssertEqual(calls, 1, "only the first case may run")
        for id in ["one", "two", "three"] {
            XCTAssertTrue(report.problems.contains { $0.hasPrefix("fuzz/seq/\(id):") && $0.contains("budget") }, "\(report.problems)")
        }
        XCTAssertEqual(report.evaluated, 0)
    }

    func testEachCallIsChargedItsRealWorkAndHandedWhatIsLeft() {
        var caps: [Int] = []
        FuzzHooks.modify { t in
            t.sequence = { _, _, _, _, cap in caps.append(cap); return Mutate.Counted(data: [0], work: 10_000_000) }
        }
        let report = FuzzCorpus.evaluate([sequenceCase("fuzz/seq/a", bytes: 1, steps: 1),
                                          sequenceCase("fuzz/seq/b", bytes: 1, steps: 1),
                                          sequenceCase("fuzz/seq/c", bytes: 1, steps: 1)], expected: -1)
        XCTAssertEqual(caps, [FuzzCorpus.maxAggregateSequenceWork, FuzzCorpus.maxAggregateSequenceWork - 10_000_000])
        XCTAssertFalse(report.problems.contains { $0.hasPrefix("fuzz/seq/a:") || $0.hasPrefix("fuzz/seq/b:") }, "\(report.problems)")
        XCTAssertTrue(report.problems.contains { $0.hasPrefix("fuzz/seq/c:") && $0.contains("budget") })
    }

    func testARealOversizedSequenceCaseIsAProblemNotAHang() {
        // Real engine: the corpus's own per-case caps (steps <= 10,000) hold, and
        // a step count past them is a reported problem.
        let report = FuzzCorpus.evaluate([sequenceCase("fuzz/seq/huge", bytes: 1, steps: 1_000_000_000)], expected: -1)
        XCTAssertTrue(report.problems.contains { $0.hasPrefix("fuzz/seq/huge:") })
    }

    // MARK: sabotage hooks ----------------------------------------------------------------------

    private func caught(_ what: String, prefix: String, _ change: (inout FuzzHooks.Table) -> Void, file: StaticString = #filePath, line: UInt = #line) throws {
        FuzzHooks.modify(change)
        defer { FuzzHooks.reset() }
        let problems = FuzzCorpus.evaluate(try FuzzSupport.cases()).problems
        XCTAssertTrue(problems.contains { $0.hasPrefix(prefix) }, "\(what) was not caught: \(problems)", file: file, line: line)
    }

    func testEverySabotagedEntryPointIsCaught() throws {
        try caught("bit flip", prefix: "fuzz/mutate_bit_flip/") { $0.bitFlip = { d, _, _ in d } }
        try caught("byte flip", prefix: "fuzz/mutate_byte_flip/") { $0.byteFlip = { d, _, _ in d } }
        try caught("arithmetic", prefix: "fuzz/mutate_arithmetic/") { $0.arithmetic = { d, _, _, _, _ in d } }
        try caught("boundary", prefix: "fuzz/mutate_boundary/") { $0.boundary = { d, _, _, _, _ in d } }
        try caught("block delete", prefix: "fuzz/mutate_block_delete/") { $0.blockDelete = { d, _, _ in d } }
        try caught("block insert", prefix: "fuzz/mutate_block_insert/") { $0.blockInsert = { d, _, _ in d } }
        try caught("block duplicate", prefix: "fuzz/mutate_block_duplicate/") { $0.blockDuplicate = { d, _, _, _ in d } }
        try caught("splice", prefix: "fuzz/mutate_splice/") { $0.splice = { t, _, _, _ in t } }
        try caught("sequence", prefix: "fuzz/mutate_sequence/") { $0.sequence = { d, _, _, _, _ in Mutate.Counted(data: d, work: 0) } }
        try caught("admit", prefix: "fuzz/corpus_admit/") { $0.corpusAdmit = { _, _ in (false, 0) } }
        try caught("admit always", prefix: "fuzz/corpus_admit/redundant") { $0.corpusAdmit = { _, t in (true, t.count) } }
        try caught("dedup always", prefix: "fuzz/crash_dedup/divergent") { $0.isDuplicate = { _, _, _, _ in true } }
        try caught("dedup never", prefix: "fuzz/crash_dedup/same") { $0.isDuplicate = { _, _, _, _ in false } }
        try caught("failure ids accept all", prefix: "fuzz/failure_id/unregistered") { $0.isFailureId = { _ in true } }
        try caught("failure ids reject all", prefix: "fuzz/failure_id/crash_valid") { $0.isFailureId = { _ in false } }
        try caught("tap", prefix: "fuzz/private_reporting/") { $0.formatTap = { _, _, _, _ in "not ok 1 - x" } }
        try caught("tap leak", prefix: "fuzz/private_reporting/") { $0.formatTap = { n, t, f, s in try FuzzSignature.formatTapFailure(n, target: t, failureId: f, sig: s) + "\ndeadbeef" } }
        try caught("tfv accepts", prefix: "fuzz/corpus_reject/") { $0.tfvValid = { _ in true } }
    }

    func testSabotageThatThrowsIsAProblemAndResetRestoresACleanRun() throws {
        FuzzHooks.modify { $0.formatTap = { _, _, _, _ in throw FuzzError.invalidArgument("boom") } }
        XCTAssertTrue(FuzzCorpus.evaluate(try FuzzSupport.cases()).problems.contains { $0.hasPrefix("fuzz/private_reporting/") })
        FuzzHooks.reset()
        XCTAssertEqual(FuzzCorpus.evaluate(try FuzzSupport.cases()).problems, [])
    }

    func testIndependentReferenceScriptsAreWhereTheDocsSayThey() {
        XCTAssertTrue(FileManager.default.fileExists(atPath: FuzzSupport.repoFile("code/scripts/fuzz_conformance.py").path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: FuzzSupport.repoFile("code/scripts/pbt_conformance.py").path))
    }
}
