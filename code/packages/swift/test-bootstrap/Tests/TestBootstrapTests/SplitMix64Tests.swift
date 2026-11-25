import XCTest
@testable import TestBootstrap

final class SplitMix64Tests: XCTestCase {
    private static let gamma: UInt64 = 0x9e37_79b9_7f4a_7c15

    func testSeedZeroFirstOutputIsThePinnedReferenceVector() {
        var r = SplitMix64(seed: 0)
        XCTAssertEqual(r.nextU64(), 0xe220_a839_7b1d_cdaf)
    }

    func testFnv1aVectors() {
        XCTAssertEqual(fnv1a64(""), 14695981039346656037)
        XCTAssertEqual(fnv1a64("a"), 12638187200555641996)
        XCTAssertEqual(fnv1a64("foobar"), 9625390261332436968)
        XCTAssertEqual(fnv1a64([UInt8]("foobar".utf8)), 9625390261332436968)
    }

    func testSubSeedFollowsVer05Section23() {
        // Vectors from code/scripts/pbt_conformance.py.
        XCTAssertEqual(subSeed(1234, "prop/a"), 13640152828477459921)
        XCTAssertEqual(subSeed(0, ""), 14087677454934409008)
        XCTAssertEqual(subSeed(UInt64.max, "pbt/x"), 6387471972037664226)
        // And the definition spelled out.
        var r = SplitMix64(seed: 1234 ^ fnv1a64("prop/a"))
        XCTAssertEqual(subSeed(1234, "prop/a"), r.nextU64())
    }

    // The SplitMix64 finalizer is a bijection, so we can run it BACKWARD: given a
    // wanted output, compute the state whose next output is exactly that word.
    private func inverse(_ x: UInt64) -> UInt64 {
        var inv = x   // Newton iteration for the multiplicative inverse mod 2^64
        for _ in 0..<6 { inv = inv &* (2 &- x &* inv) }
        return inv
    }

    private func unxorshift(_ v: UInt64, _ s: UInt64) -> UInt64 {
        var x = v
        var shift = s
        while shift < 64 { x = v ^ (x >> s); shift += s }
        return x
    }

    /// The seed whose FIRST output is `target`.
    private func seed(forFirstOutput target: UInt64) -> UInt64 {
        var z = unxorshift(target, 31)
        z = z &* inverse(0x94d0_49bb_1331_11eb)
        z = unxorshift(z, 27)
        z = z &* inverse(0xbf58_476d_1ce4_e5b9)
        z = unxorshift(z, 30)
        return z &- SplitMix64Tests.gamma   // z is the post-increment state
    }

    func testFinalizerInversionRoundTrips() {
        for target: UInt64 in [0, 1, 0xdead_beef, UInt64.max, 1 << 63] {
            var r = SplitMix64(seed: seed(forFirstOutput: target))
            XCTAssertEqual(r.nextU64(), target)
        }
    }

    func testForcedRejectionDrawIsRejectedByBounded3() throws {
        // bounded(3): threshold t = 2^64 mod 3 = 1, so the word 0 must be rejected.
        let s = seed(forFirstOutput: 0)
        var probe = SplitMix64(seed: s)
        XCTAssertEqual(probe.nextU64(), 0)
        let second = probe.nextU64()

        var r = SplitMix64(seed: s)
        let got = try r.bounded(3)
        XCTAssertEqual(got, second % 3, "the 0 word is rejected and the second word is used")
        // The generator consumed two words, not one.
        var two = SplitMix64(seed: s); _ = two.nextU64(); _ = two.nextU64()
        XCTAssertEqual(r.state, two.state)
    }

    func testThresholdIsZeroMinusNModN() {
        // Pin the unsigned-wrap formula at several n.
        for n: UInt64 in [1, 2, 3, 7, 10, 1 << 32, (1 << 63) + 1, UInt64.max] {
            let expected = UInt64((BigInt.one.shiftedLeft(64) - BigInt(n)).quotientAndRemainder(dividingBy: BigInt(n)).remainder.asUInt64!)
            XCTAssertEqual((0 &- n) % n, expected, "n = \(n)")
        }
        XCTAssertEqual((0 &- ((UInt64(1) << 63) + 1)) % ((UInt64(1) << 63) + 1), (UInt64(1) << 63) - 1)
    }

    func testBounded2Pow63Plus1RejectsTheLastRejectedWordAndAcceptsTheThreshold() throws {
        let n = (UInt64(1) << 63) + 1
        let t = (UInt64(1) << 63) - 1
        // First output t-1: rejected. A threshold that is off by one (t-1) would accept it.
        let s = seed(forFirstOutput: t - 1)
        var probe = SplitMix64(seed: s)
        XCTAssertEqual(probe.nextU64(), t - 1)
        let second = probe.nextU64()
        var r = SplitMix64(seed: s)
        let got = try r.bounded(n)
        XCTAssertGreaterThanOrEqual(second, t, "fixture sanity: the second word is accepted")
        XCTAssertEqual(got, second % n)
        XCTAssertNotEqual(got, (t - 1) % n, "t-1 must be rejected")
        var two = SplitMix64(seed: s); _ = two.nextU64(); _ = two.nextU64()
        XCTAssertEqual(r.state, two.state, "two words consumed")
        // The word t itself is ACCEPTED (x >= threshold).
        let s2 = seed(forFirstOutput: t)
        var r2 = SplitMix64(seed: s2)
        XCTAssertEqual(try r2.bounded(n), t % n)
        var one = SplitMix64(seed: s2); _ = one.nextU64()
        XCTAssertEqual(r2.state, one.state, "the word t consumed exactly one draw")
    }

    func testWideSpanVectorsFromTheReference() throws {
        var r = SplitMix64(seed: 7)
        let n = (UInt64(1) << 63) + 1
        XCTAssertEqual([try r.bounded(n), try r.bounded(n), try r.bounded(n), try r.bounded(n)],
                       [7392729709960833537, 1529793891446696394, 8483179396677329707, 7711100304988943181])
    }

    func testBoundedOneConsumesAWord() throws {
        var r = SplitMix64(seed: 3)
        XCTAssertEqual(try r.bounded(1), 0)
        XCTAssertEqual(try r.bounded(1), 0)
        XCTAssertEqual(r.nextU64(), 11307387092600937729)
    }

    func testBoundedZeroThrows() {
        var r = SplitMix64(seed: 1)
        XCTAssertThrowsError(try r.bounded(0))
        XCTAssertThrowsError(try r.boundedBig(.zero))
        XCTAssertThrowsError(try r.boundedBig(BigInt(-5)))
        XCTAssertThrowsError(try r.boundedBig(BigInt.one.shiftedLeft(1001)))
    }

    func testBoundedBigMatchesReferenceAcrossTheThreeRegimes() throws {
        var a = SplitMix64(seed: 5)
        let span3 = BigInt(3).shiftedLeft(126)
        XCTAssertEqual((0..<4).map { _ in try! a.boundedBig(span3).description },
                       ["131610346136799443493804746066869303032", "80158113459267648879119848685637857067",
                        "145113871033734923922661941654886930275", "153288730334245147741767570423313442620"])
        var b = SplitMix64(seed: 9)
        let span200 = BigInt.one.shiftedLeft(200) + BigInt(12345)
        XCTAssertEqual((0..<3).map { _ in try! b.boundedBig(span200).description },
                       ["632422361754002406606296911583165654248235444844938038631168",
                        "1011332786024869500889722626479820132968022351165286095514414",
                        "664049325962279076961615521405294680030542777518922691047552"])
        var c = SplitMix64(seed: 9)
        XCTAssertEqual((0..<2).map { _ in try! c.boundedBig(BigInt.one.shiftedLeft(64)).description },
                       ["12587370737594032228", "13847876567842155106"])
    }

    func testThreeThirdsOfA3Times2Pow126SpanAreBalanced() throws {
        // The first Rust lane used `raw % span` above 2^64 and drew the low third
        // with probability 1/2 instead of 1/3 (VER05 section 7.2).
        var r = SplitMix64(seed: 2024)
        let span = BigInt(3).shiftedLeft(126)
        let third = BigInt.one.shiftedLeft(126)
        var counts = [0, 0, 0]
        let n = 3000
        for _ in 0..<n {
            let v = try r.boundedBig(span)
            counts[v < third ? 0 : (v < third + third ? 1 : 2)] += 1
        }
        for c in counts { XCTAssertTrue(c > 850 && c < 1150, "third counts \(counts)") }
    }

    func testBoundedIsRoughlyUniform() throws {
        var r = SplitMix64(seed: 77)
        var hist = [Int](repeating: 0, count: 10)
        for _ in 0..<10_000 { hist[Int(try r.bounded(10))] += 1 }
        for h in hist { XCTAssertTrue(h > 850 && h < 1150, "\(hist)") }
    }
}
