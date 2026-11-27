import XCTest
@testable import TestBootstrap

final class GensTests: XCTestCase {
    private func draw<T>(_ g: Gen<T>, seed: UInt64 = 1, n: Int) throws -> [T] {
        var rng = SplitMix64(seed: seed)
        return try (0..<n).map { _ in try g.sample(&rng) }
    }

    func testPrimitivesMatchTheDocumentedDrawRules() throws {
        var rng = SplitMix64(seed: 0)
        XCTAssertEqual(try Gens.bool().sample(&rng), true)
        XCTAssertEqual(try draw(Gens.u8(), seed: 0, n: 3), [175, 244, 79])
        XCTAssertEqual(try draw(Gens.u64(), seed: 0, n: 2), [16294208416658607535, 7960286522194355700])
        let i = try draw(Gens.i128(), seed: 42, n: 1)
        XCTAssertEqual(i[0].description, "-87940914747023602177814525588822101757")
    }

    func testIntegerAndIntAreInclusiveAndAgree() throws {
        let a = try draw(try Gens.integer(BigInt(10), BigInt(20)), seed: 7, n: 8).map { $0.asInt! }
        XCTAssertEqual(a, [12, 10, 10, 10, 17, 17, 11, 19])
        let b = try draw(try Gens.int(10, 20), seed: 7, n: 8)
        XCTAssertEqual(a, b)
        let fullInt = try draw(try Gens.int(Int.min, Int.max), seed: 3, n: 50)
        XCTAssertEqual(fullInt.count, 50)
        let extremes = try draw(try Gens.int(5, 5), n: 3)
        XCTAssertEqual(extremes, [5, 5, 5])
        XCTAssertThrowsError(try Gens.integer(BigInt(2), BigInt(1)))
        XCTAssertThrowsError(try Gens.int(2, 1))
        XCTAssertThrowsError(try Gens.integer(-BigInt.one.shiftedLeft(1001), .zero))
        let wide = try Gens.integer(-BigInt.one.shiftedLeft(500), BigInt.one.shiftedLeft(500))
        for v in try draw(wide, n: 20) { XCTAssertTrue(v.bitWidth <= 501) }
    }

    func testBytesLengthsAreInclusive() throws {
        let lens = Set(try draw(try Gens.bytes(2, 4), n: 200).map { $0.count })
        XCTAssertEqual(lens, [2, 3, 4])
        XCTAssertEqual(try draw(try Gens.bytes(0, 0), n: 3).map { $0.count }, [0, 0, 0])
        XCTAssertThrowsError(try Gens.bytes(5, 4))
        XCTAssertThrowsError(try Gens.bytes(-1, 4))
        XCTAssertThrowsError(try Gens.bytes(0, Limits.maxInputBytes + 1))
    }

    func testOneOfIsUniformAndEmptyThrows() throws {
        let g = try Gens.oneOf([Gens.constant(0), Gens.constant(1), Gens.constant(2)])
        var counts = [0, 0, 0]
        for v in try draw(g, n: 3000) { counts[v] += 1 }
        for c in counts { XCTAssertTrue(c > 850 && c < 1150, "\(counts)") }
        XCTAssertThrowsError(try Gens.oneOf([Gen<Int>]()))
    }

    func testFrequencyFollowsWeightsAndNeverPicksZeroWeight() throws {
        let g = try Gens.frequency([.init(weight: 1, gen: Gens.constant("a")), .init(weight: 0, gen: Gens.constant("never")),
                                    .init(weight: 3, gen: Gens.constant("b"))])
        let xs = try draw(g, n: 4000)
        XCTAssertFalse(xs.contains("never"))
        let a = xs.filter { $0 == "a" }.count
        XCTAssertTrue(a > 850 && a < 1150, "a=\(a)")
        XCTAssertThrowsError(try Gens.frequency([Gens.Arm<Int>]()))
        XCTAssertThrowsError(try Gens.frequency([Gens.Arm(weight: 0, gen: Gens.constant(1))]))
        XCTAssertThrowsError(try Gens.frequency([Gens.Arm(weight: (1 << 32) + 1, gen: Gens.constant(1))]))
        XCTAssertThrowsError(try Gens.frequency([Gens.Arm(weight: 1 << 32, gen: Gens.constant(1)), Gens.Arm(weight: 1, gen: Gens.constant(1))]))
        XCTAssertNoThrow(try Gens.frequency([Gens.Arm(weight: 1 << 32, gen: Gens.constant(1))]))
    }

    func testListLengthsAreInclusiveAndElementsInOrder() throws {
        let g = try Gens.list(Gens.u8(), 1, 3)
        XCTAssertEqual(Set(try draw(g, n: 200).map { $0.count }), [1, 2, 3])
        XCTAssertThrowsError(try Gens.list(Gens.u8(), 3, 1))
        XCTAssertThrowsError(try Gens.list(Gens.u8(), 0, Limits.maxListLen + 1))
    }

    func testMapAndAndThenCompose() throws {
        let sized = Gens.andThen(try Gens.int(1, 3)) { n in try Gens.list(Gens.constant(n), n, n) }
        for v in try draw(sized, n: 30) { XCTAssertEqual(v, Array(repeating: v.count, count: v.count)) }
        let m = Gens.map(Gens.constant(4)) { $0 + 1 }
        XCTAssertEqual(try draw(m, n: 1), [5])
        let thrower = Gens.map(Gens.constant(0)) { _ -> Int in throw PbtError.invalidArgument("boom") }
        XCTAssertThrowsError(try draw(thrower, n: 1))
    }

    func testFilterSamplesExactlyMaxRetriesThenExhausts() throws {
        var calls = 0
        let g = try Gens.filter(try Gens.int(0, 10), maxRetries: 37) { _ in calls += 1; return false }
        var rng = SplitMix64(seed: 1)
        XCTAssertThrowsError(try g.sample(&rng)) { XCTAssertTrue($0 is ExhaustedError) }
        XCTAssertEqual(calls, 37)
        XCTAssertThrowsError(try Gens.filter(Gens.bool(), maxRetries: 0) { _ in true })
        XCTAssertThrowsError(try Gens.filter(Gens.bool(), maxRetries: Limits.maxFilterRetries + 1) { _ in true })
        XCTAssertNoThrow(try Gens.filter(Gens.bool(), maxRetries: Limits.maxFilterRetries) { _ in true })
    }

    func testAnOrdinaryFilterStillPassesAtTheDefaultAllowance() throws {
        let g = try Gens.filter(try Gens.int(0, 99)) { $0 % 10 == 0 }
        let v = try Runner.check(seed: 3, gen: g, options: Options(numTests: 200, ci: .on)) { $0 % 10 == 0 }
        XCTAssertTrue(v.passed)
    }

    func testDrawsAreReproducibleFromTheSeed() throws {
        let g = Gens.tuple2(try Gens.int(0, 1_000_000), try Gens.bytes(0, 6))
        let a = try draw(g, seed: 99, n: 20), b = try draw(g, seed: 99, n: 20)
        XCTAssertTrue(zip(a, b).allSatisfy { $0.0 == $1.0 && $0.1 == $1.1 })
        let c = try draw(g, seed: 100, n: 20)
        XCTAssertFalse(zip(a, c).allSatisfy { $0.0 == $1.0 && $0.1 == $1.1 })
    }

    func testCorpusEntryPointsValidateTheirArguments() {
        XCTAssertThrowsError(try Gens.genPrimitive(seed: 0, typeName: "f32", count: 1))
        XCTAssertThrowsError(try Gens.genPrimitive(seed: 0, typeName: "u8", count: -1))
        XCTAssertThrowsError(try Gens.genPrimitive(seed: 0, typeName: "u8", count: Limits.maxGenCount + 1))
        XCTAssertThrowsError(try Gens.genBounded(seed: 0, lo: BigInt(5), hi: BigInt(1), count: 1))
        XCTAssertThrowsError(try Gens.genBounded(seed: 0, lo: .zero, hi: BigInt.one.shiftedLeft(900), count: 100_000))
        XCTAssertThrowsError(try Gens.genBytes(seed: 0, minLen: 0, maxLen: 1000, count: 100_000))
        XCTAssertThrowsError(try Gens.genBytes(seed: 0, minLen: 0, maxLen: 0, count: Limits.maxGenCount + 1))
        XCTAssertThrowsError(try Gens.genBytes(seed: 0, minLen: 4, maxLen: 2, count: 1))
        XCTAssertEqual(try Gens.genBytes(seed: 1, minLen: 0, maxLen: 0, count: 2), [[], []])
    }

    func testI128CoversTheFullRangeAndBothSigns() throws {
        let vs = try draw(Gens.i128(), seed: 5, n: 200)
        XCTAssertTrue(vs.contains { $0.signum < 0 })
        XCTAssertTrue(vs.contains { $0.signum > 0 })
        let lo = -BigInt.one.shiftedLeft(127), hi = BigInt.one.shiftedLeft(127) - .one
        XCTAssertTrue(vs.allSatisfy { $0 >= lo && $0 <= hi })
    }
}
