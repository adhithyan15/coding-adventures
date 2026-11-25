import XCTest
@testable import TestBootstrap

final class RunnerTests: XCTestCase {
    override func tearDown() { Hooks.reset(); super.tearDown() }

    private struct Boom: Error { let secret: String }
    private let local = Options(numTests: 100, maxShrinkSteps: 1000, ci: .off)

    func testAHoldingPropertyPassesAndCountsItsCases() throws {
        let v = try Runner.check(seed: 1, gen: try Gens.int(0, 100), options: local) { $0 <= 100 }
        XCTAssertTrue(v.passed)
        XCTAssertEqual(v.numTests, 100)
        XCTAssertNil(v.failureId)
        XCTAssertNil(v.seed)
    }

    func testAFalsifiableMutationIsNeverAPass() throws {
        // x < 100 over [0, 10000] is falsified by the first large draw.
        let v = try Runner.check(seed: 7, gen: try Gens.int(0, 10_000), options: local) { $0 < 100 }
        XCTAssertFalse(v.passed)
        XCTAssertEqual(v.failureId, FailureIds.counterexample)
        XCTAssertEqual(v.minimal as? Int, 100)
        XCTAssertGreaterThan(v.shrinkSteps, 0)
        XCTAssertEqual(v.seed, 7)
        XCTAssertNotNil(v.initial)
    }

    func testZeroCasesIsExhaustedNeverAPass() throws {
        let v = try Runner.check(seed: 1, gen: Gens.bool(), options: Options(numTests: 0, ci: .off)) { _ in true }
        XCTAssertFalse(v.passed)
        XCTAssertEqual(v.failureId, FailureIds.exhausted)
    }

    func testInvalidOptionsThrowInsteadOfProducingAVerdict() {
        for bad in [Options(numTests: -1), Options(numTests: Limits.maxCases + 1),
                    Options(maxShrinkSteps: -1), Options(maxShrinkSteps: Limits.maxShrinkSteps + 1)] {
            XCTAssertThrowsError(try Runner.check(seed: 1, gen: Gens.bool(), options: bad) { _ in true })
        }
    }

    func testAGeneratorErrorIsExhausted() throws {
        let bad = Gens.map(Gens.bool()) { _ -> Int in throw Boom(secret: "s3cret") }
        let v = try Runner.check(seed: 1, gen: bad, options: local) { _ in true }
        XCTAssertEqual(v.failureId, FailureIds.exhausted)
        XCTAssertFalse(v.passed)
        let red = try Runner.check(seed: 1, gen: bad, options: Options(ci: .on)) { _ in true }
        XCTAssertFalse(red.message!.contains("s3cret"))
        XCTAssertTrue(red.message!.contains("Boom"))
        let exhausted = try Gens.filter(Gens.bool(), maxRetries: 3) { _ in false }
        XCTAssertEqual(try Runner.check(seed: 1, gen: exhausted, options: local) { _ in true }.failureId, FailureIds.exhausted)
    }

    func testAThrowingPropertyIsAPanic() throws {
        let v = try Runner.check(seed: 3, gen: Gens.bool(), options: local) { _ in throw Boom(secret: "s3cret") }
        XCTAssertEqual(v.failureId, FailureIds.panic)
        XCTAssertTrue(v.message!.contains("s3cret"))
        XCTAssertEqual(v.seed, 3)
        let red = try Runner.check(seed: 3, gen: Gens.bool(), options: Options(ci: .on)) { _ in throw Boom(secret: "s3cret") }
        XCTAssertEqual(red.failureId, FailureIds.panic)
        XCTAssertFalse(red.message!.contains("s3cret"))
        XCTAssertNil(red.seed)
    }

    func testCiRedactionCarriesNoSeedWitnessOrMinimal() throws {
        for mode in [CiMode.on, .unset] {
            let v = try Runner.check(seed: 123456789, gen: try Gens.int(0, 10_000), options: Options(ci: mode)) { $0 < 100 }
            XCTAssertEqual(v.failureId, FailureIds.counterexample)
            XCTAssertNil(v.seed)
            XCTAssertNil(v.initial)
            XCTAssertNil(v.minimal)
            XCTAssertFalse(v.message!.contains("123456789"))
            XCTAssertFalse(v.message!.contains("minimal="))
            XCTAssertFalse(v.message!.contains("initial="))
            XCTAssertFalse(v.message!.contains("seed"))
            let d = v.digest!
            XCTAssertEqual(d.count, 16)
            XCTAssertTrue(v.message!.contains("repro-fnv: \(d)"))
            // The digest is the FNV-1a of the documented preimage.
            XCTAssertEqual(d, String(format: "%016llx", fnv1a64("iter:\(v.iterations);steps:\(v.shrinkSteps);shrunk:100")))
        }
    }

    func testUnsetModeFailsClosed() throws {
        XCTAssertEqual(Options().ci, .unset)
        let v = try Runner.check(seed: 5, gen: try Gens.int(0, 1000), options: Options()) { $0 < 10 }
        XCTAssertNil(v.minimal)
    }

    func testCiModeLookup() {
        func mode(_ v: String?) -> CiMode { Options.ciMode(fromLookup: { $0 == "CI" ? v : nil }) }
        XCTAssertEqual(mode(nil), .off)
        XCTAssertEqual(mode(""), .off)
        XCTAssertEqual(mode("0"), .off)
        XCTAssertEqual(mode(" False "), .off)
        XCTAssertEqual(mode("true"), .on)
        XCTAssertEqual(mode("1"), .on)
        XCTAssertEqual(mode("yes"), .on)
    }

    func testReplayFromTheSameSeedGivesTheSameVerdict() throws {
        let g = Gens.tuple2(try Gens.int(0, 1000), try Gens.bytes(0, 8))
        let prop: ((Int, [UInt8])) -> Bool = { $0.0 < 500 || $0.1.count < 3 }
        let a = try Runner.check(seed: 42, gen: g, options: local, property: prop)
        let b = try Runner.check(seed: 42, gen: g, options: local, property: prop)
        XCTAssertEqual(a.message, b.message)
        XCTAssertFalse(a.passed)
        let c = try Runner.check(seed: 43, gen: g, options: local, property: prop)
        XCTAssertNotEqual(a.message, c.message)
    }

    func testCorpusReplaySeedReproducesThroughTheRunner() throws {
        // pbt/replay/seed42_le_100: bounded(1000) draws; first failure 413 -> minimal 101.
        let gen = Gen<Int>(sampler: { rng in Int(try rng.bounded(1000)) },
                           shrinker: { v, f, n in
                               let r = Shrink.shrinkIntCore(initial: BigInt(v), target: .zero, lo: .zero, hi: BigInt(1000),
                                                            falsifies: { f($0.asInt!) }, maxSteps: n)
                               return Shrunk(value: r.value.asInt!, steps: r.steps)
                           })
        let v = try Runner.check(seed: 42, gen: gen, options: local) { $0 <= 100 }
        XCTAssertEqual(v.initial as? Int, 413)
        XCTAssertEqual(v.minimal as? Int, 101)
    }

    // MARK: re-verification ----------------------------------------------------

    func testALyingShrinkerCannotChangeTheWitness() throws {
        let lying: Shrinker<Int> = { _, _, _ in Shrunk(value: 1, steps: 3) }   // 1 does not falsify x >= 100
        let v = try Runner.check(seed: 7, gen: try Gens.int(0, 10_000), options: local, shrinker: lying) { $0 < 100 }
        XCTAssertEqual(v.failureId, FailureIds.counterexample)
        XCTAssertGreaterThanOrEqual(v.minimal as! Int, 100)
        XCTAssertEqual(v.shrinkSteps, 0)
        XCTAssertEqual(v.minimal as? Int, v.initial as? Int)
    }

    func testAShrinkerReturningAnImpossibleStepCountIsIgnored() throws {
        for steps in [-1, 1_000_001] {
            let bad: Shrinker<Int> = { v, _, _ in Shrunk(value: v, steps: steps) }
            let v = try Runner.check(seed: 7, gen: try Gens.int(0, 10_000), options: local, shrinker: bad) { $0 < 100 }
            XCTAssertEqual(v.shrinkSteps, 0)
        }
    }

    func testACandidateOnWhichThePropertyThrowsIsNotAcceptedAsSmaller() throws {
        // Shrinks toward 0 but the property THROWS below 50: the result stays >= 50.
        let v = try Runner.check(seed: 9, gen: try Gens.int(0, 10_000), options: local) { x in
            if x < 50 { throw Boom(secret: "small") }
            return x < 100
        }
        XCTAssertEqual(v.failureId, FailureIds.counterexample)
        XCTAssertEqual(v.minimal as? Int, 100)
    }

    func testAShrinkerThatAdoptsAThrowingValueIsRejectedByReverification() throws {
        let bad: Shrinker<Int> = { _, _, _ in Shrunk(value: 3, steps: 1) }
        let v = try Runner.check(seed: 9, gen: try Gens.int(0, 10_000), options: local, shrinker: bad) { x in
            if x < 50 { throw Boom(secret: "small") }
            return x < 100
        }
        XCTAssertEqual(v.shrinkSteps, 0)
        XCTAssertGreaterThanOrEqual(v.minimal as! Int, 100)
    }

    func testFailureIdsAreClosed() {
        XCTAssertEqual(FailureIds.all.count, 4)
        for id in ["property-counterexample", "property-exhausted", "property-timeout", "property-panic"] {
            XCTAssertTrue(FailureIds.isPbtFailureId(id))
        }
        for id in ["", "property-", "Property-panic", "property-panic ", "unregistered-id"] {
            XCTAssertFalse(FailureIds.isPbtFailureId(id), id)
        }
    }

    func testStructuredWitnessesRenderWithoutCallingDescription() throws {
        struct Pair { let a: Int; let b: [UInt8] }
        let g = Gens.map(Gens.tuple2(try Gens.int(5, 9), try Gens.bytes(1, 2))) { Pair(a: $0.0, b: $0.1) }
        let v = try Runner.check(seed: 1, gen: g, options: local) { _ in false }
        XCTAssertTrue(v.message!.contains("minimal="))
    }
}
