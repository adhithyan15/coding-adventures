import XCTest
@testable import TestBootstrap

/// Replays the committed pbt-v1 fixture and proves the replay CAN fail.
final class CorpusConformanceTests: XCTestCase {
    override func tearDown() {
        Hooks.reset()
        super.tearDown()
    }

    // MARK: the real corpus ---------------------------------------------------

    func testFixtureIsTheCommittedPathAndHasTwentyFiveCases() throws {
        XCTAssertTrue(Support.fixtureURL().path.hasSuffix("code/specs/fixtures/pbt-v1/cases.json"))
        XCTAssertEqual(try Support.cases().count, 25)
    }

    func testWholeCorpusConformsExactlyIncludingStepCounts() throws {
        let report = Corpus.evaluate(try Support.cases())
        XCTAssertEqual(report.problems, [])
        XCTAssertEqual(report.evaluated, 25)
    }

    func testEveryOpAndAllFourFailureIdsAreInTheFixture() throws {
        let cases = try Support.cases()
        XCTAssertEqual(Set(cases.map(Support.op)).count, 11)
        let ids = cases.filter { Support.op($0) == "failure_id_valid" }.map { $0["input"] }
        XCTAssertEqual(ids.count, 5)
    }

    // MARK: per-op corruption: every one of the 25 cases ----------------------

    /// The corruptions an op defines. Each returns a corrupted copy of the case.
    private func corruptions(of c: Json) -> [(String, Json)] {
        func list(_ key: String) -> [Json] { if case .array(let a)? = c[key] { return a } else { return [] } }
        switch Support.op(c) {
        case "gen_primitive", "gen_bounded", "gen_bytes":
            var outs = list("outputs")
            if outs.isEmpty { return [] }
            switch outs[0] {
            case .bool(let b): outs[0] = .bool(!b)
            case .object:
                if case .string("bytes")? = outs[0]["kind"] {
                    var hex = ""
                    if case .string(let h)? = outs[0]["hex"] { hex = h }
                    outs[0] = Support.bytes(hex.isEmpty ? "00" : (hex.hasPrefix("00") ? "ff" : "00") + hex.dropFirst(2))
                } else { outs[0] = Support.intPlus(outs[0], 1) }
            default: break
            }
            return [("flipped output", Support.setting(c, "outputs", .array(outs)))]
        case "shrink_int", "shrink_refinement", "shrink_bytes":
            let minimal: Json
            if Support.op(c) == "shrink_bytes" { minimal = Support.bytes("00") } else { minimal = Support.intPlus(c["expected_minimal"]!, 1) }
            return [("wrong minimal", Support.setting(c, "expected_minimal", minimal)),
                    ("steps + 1", Support.setting(c, "expected_steps", Support.intPlus(c["expected_steps"]!, 1)))]
        case "shrink_list":
            return [("wrong minimal", Support.setting(c, "expected_minimal", .array(list("expected_minimal") + [Support.int("9")]))),
                    ("steps + 1", Support.setting(c, "expected_steps", Support.intPlus(c["expected_steps"]!, 1)))]
        case "replay_consistency":
            return [("wrong initial_fail", Support.setting(c, "initial_fail", Support.intPlus(c["initial_fail"]!, 1))),
                    ("wrong minimal", Support.setting(c, "expected_minimal", Support.intPlus(c["expected_minimal"]!, 1)))]
        case "filter_exhausted":
            return [("wrong failure id", Support.setting(c, "failure_id", .string("property-panic"))),
                    ("verdict pass", Support.setting(c, "verdict", .string("pass")))]
        case "failure_id_valid":
            guard case .bool(let v)? = c["valid"] else { return [] }
            return [("flipped validity", Support.setting(c, "valid", .bool(!v)))]
        case "corpus_reject":
            return [("a valid TFV in place of a reject text", Support.setting(c, "text", .string("tfv 1\n")))]
        default:
            return []
        }
    }

    func testEveryCorruptionOfEveryCaseFailsAndNamesExactlyThatCase() throws {
        let cases = try Support.cases()
        var checked = 0
        for (index, c) in cases.enumerated() {
            let cid = Support.id(c)
            let corrupted = corruptions(of: c)
            XCTAssertFalse(corrupted.isEmpty, "no corruption defined for \(cid)")
            for (what, bad) in corrupted {
                var copy = cases
                copy[index] = bad
                let report = Corpus.evaluate(copy)
                XCTAssertFalse(report.problems.isEmpty, "\(cid): \(what) was not detected")
                XCTAssertTrue(report.problems.contains { $0.hasPrefix(cid + ":") }, "\(cid): \(what) not attributed: \(report.problems)")
                for p in report.problems {
                    XCTAssertTrue(p.hasPrefix(cid + ":") || p.hasPrefix("pbt-v1:"), "\(cid): \(what) blamed another case: \(p)")
                }
                checked += 1
            }
        }
        XCTAssertGreaterThanOrEqual(checked, 25)
    }

    // MARK: dropped, truncated, padded, malformed corpora ---------------------

    func testDroppingAnyCaseFails() throws {
        let cases = try Support.cases()
        for i in 0..<cases.count {
            var copy = cases
            copy.remove(at: i)
            let report = Corpus.evaluate(copy)
            XCTAssertFalse(report.problems.isEmpty, "dropping \(Support.id(cases[i])) went unnoticed")
            XCTAssertTrue(report.problems.contains { $0.contains("expected 25 cases, got 24") })
        }
    }

    func testEveryTruncationFails() throws {
        let cases = try Support.cases()
        for n in 0..<cases.count {
            XCTAssertFalse(Corpus.evaluate(Array(cases.prefix(n))).problems.isEmpty, "prefix \(n)")
        }
    }

    func testEmptyCorpusFails() {
        let r = Corpus.evaluate([])
        XCTAssertEqual(r.evaluated, 0)
        XCTAssertFalse(r.problems.isEmpty)
    }

    func testPaddedAndDuplicatedCorporaFail() throws {
        let cases = try Support.cases()
        XCTAssertFalse(Corpus.evaluate(cases + [cases[0]]).problems.isEmpty)
        var dup = cases
        dup[1] = Support.setting(cases[1], "id", .string(Support.id(cases[0])))
        XCTAssertTrue(Corpus.evaluate(dup).problems.contains { $0.contains("duplicate or missing case id") })
    }

    func testUnknownOpMissingFieldAndExtraFieldAreProblemsNotSkips() throws {
        let cases = try Support.cases()
        let cid = Support.id(cases[0])
        var a = cases
        a[0] = Support.setting(cases[0], "op", .string("gen_nothing"))
        XCTAssertTrue(Corpus.evaluate(a).problems.contains { $0.hasPrefix(cid + ": unknown op") })
        var b = cases
        b[0] = Support.setting(cases[0], "extra", .bool(true))
        XCTAssertTrue(Corpus.evaluate(b).problems.contains { $0.hasPrefix(cid + ": fields") })
        var c = cases
        if case .object(let pairs) = cases[0] { c[0] = .object(pairs.filter { $0.key != "count" }) }
        XCTAssertTrue(Corpus.evaluate(c).problems.contains { $0.hasPrefix(cid + ": fields") })
        var d = cases
        d[0] = .string("not an object")
        XCTAssertFalse(Corpus.evaluate(d).problems.isEmpty)
        var e = cases
        e[0] = Support.setting(cases[0], "id", .number("5"))
        XCTAssertFalse(Corpus.evaluate(e).problems.isEmpty)
    }

    func testMalformedValuesAndUnknownPropertiesAreProblems() throws {
        let cases = try Support.cases()
        let i = cases.firstIndex { Support.id($0) == "pbt/shrink_int/54321_le_100" }!
        let cid = Support.id(cases[i])
        for bad in [Support.setting(cases[i], "property", .string("int_is_prime")),
                    Support.setting(cases[i], "initial", Support.int("01")),
                    Support.setting(cases[i], "initial", .string("54321")),
                    Support.setting(cases[i], "initial", Support.int(String(repeating: "9", count: 301))),
                    Support.setting(cases[i], "min", Support.int("200000")),   // initial outside [min, max]
                    Support.setting(cases[i], "initial", Support.int("5"))] {   // does not falsify
            var copy = cases
            copy[i] = bad
            XCTAssertTrue(Corpus.evaluate(copy).problems.contains { $0.hasPrefix(cid + ":") }, "\(bad)")
        }
    }

    func testGenCaseWithWrongOutputCountOrOversizeCountIsRefusedBeforeGenerating() throws {
        let cases = try Support.cases()
        let i = cases.firstIndex { Support.id($0) == "pbt/gen_bytes/len4_8_seed13" }!
        let cid = Support.id(cases[i])
        var wrongCount = cases
        wrongCount[i] = Support.setting(cases[i], "count", Support.int("4"))
        XCTAssertTrue(Corpus.evaluate(wrongCount).problems.contains { $0.hasPrefix(cid + ":") && $0.contains("exactly count") })
        var huge = cases
        huge[i] = Support.setting(cases[i], "count", Support.int("9999999"))
        XCTAssertTrue(Corpus.evaluate(huge).problems.contains { $0.hasPrefix(cid + ":") })
        var wide = cases
        wide[i] = Support.setting(Support.setting(cases[i], "max_len", Support.int("10000")), "count", Support.int("3"))
        XCTAssertFalse(Corpus.evaluate(wide).problems.isEmpty)
    }

    func testRealFixtureBytesParseButMutatedBytesAreRefused() throws {
        let raw = try Support.corpusBytes()
        XCTAssertNoThrow(try Corpus.parse(raw))
        XCTAssertThrowsError(try Corpus.parse(Array("{\"schema_version\": 2, \"cases\": []}".utf8)))
        XCTAssertThrowsError(try Corpus.parse(Array("{\"schema_version\": 1}".utf8)))
        XCTAssertThrowsError(try Corpus.parse(Array("{\"schema_version\": 1, \"cases\": {}}".utf8)))
        XCTAssertThrowsError(try Corpus.parse(Array("[1]".utf8)))
        XCTAssertThrowsError(try Corpus.parse(raw + [0x20, 0x7b]))
        XCTAssertNoThrow(try Corpus.parse(Array("{\"cases\": [], \"schema_version\": 1}".utf8)))
    }

    // MARK: sabotage hooks ----------------------------------------------------

    func testSabotagedUnitStepIsCaught() throws {
        Hooks.unitCandidate = { current, diff in current - BigInt(diff.signum * 2) }
        XCTAssertFalse(Corpus.evaluate(try Support.cases()).problems.isEmpty)
    }

    func testSabotagedBisectionIsCaught() throws {
        // A quarter step instead of a half step.
        Hooks.bisectCandidate = { current, diff in current - diff.shiftedRight(2) }
        XCTAssertFalse(Corpus.evaluate(try Support.cases()).problems.isEmpty)
    }

    func testSabotagedDeletionPhaseIsCaught() throws {
        Hooks.skipDeletionPhase = true
        XCTAssertFalse(Corpus.evaluate(try Support.cases()).problems.isEmpty)
    }

    func testShiftedRngIsCaught() throws {
        Hooks.newRng = { SplitMix64(seed: $0 &+ 1) }
        let problems = Corpus.evaluate(try Support.cases()).problems
        XCTAssertFalse(problems.isEmpty)
        XCTAssertTrue(problems.contains { $0.hasPrefix("pbt/gen_primitive/") })
        XCTAssertTrue(problems.contains { $0.hasPrefix("pbt/replay/") })
    }

    func testResetRestoresACleanRun() throws {
        Hooks.skipDeletionPhase = true
        Hooks.reset()
        XCTAssertEqual(Corpus.evaluate(try Support.cases()).problems, [])
    }

    // MARK: the independent reference ------------------------------------------

    func testIndependentReferenceScriptIsWhereTheDocsSayItIs() throws {
        // Located relative to the fixture: <repo>/code/scripts/pbt_conformance.py
        let script = Support.fixtureURL().deletingLastPathComponent().deletingLastPathComponent()
            .deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("scripts/pbt_conformance.py")
        XCTAssertTrue(FileManager.default.fileExists(atPath: script.path))
    }
}
