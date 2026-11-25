import XCTest
@testable import TestBootstrap

final class FuzzMutateTests: XCTestCase {
    private func b(_ s: String) -> [UInt8] { FuzzSupport.hex(s) }
    private func big(_ s: String) -> BigInt { try! BigInt.parse(s) }

    // MARK: pinned constants (the corpus does not pin them) ---------------------------

    func testEveryInterestAndDeltaConstantIsPinned() {
        XCTAssertEqual(Mutate.interestValues(width: 1), [0, 1, 127, 128, 255])
        XCTAssertEqual(Mutate.interestValues(width: 2), [0, 1, 255, 256, 32767, 32768, 65535])
        XCTAssertEqual(Mutate.interestValues(width: 4), [0, 1, 65535, 65536, 2147483647, 2147483648, 4294967295])
        XCTAssertEqual(Mutate.interestValues(width: 8), [0, 1, 4294967295, 4294967296, 9223372036854775807, 9223372036854775808, 18446744073709551615])
        for w in [0, 3, 5, 16, -1] { XCTAssertNil(Mutate.interestValues(width: w)) }
        XCTAssertEqual(Mutate.sequenceDeltas(), [-35, -8, -1, 1, 8, 35])
        XCTAssertEqual(Mutate.maxIterations, 1_000_000)
        XCTAssertEqual(Mutate.maxInputBytes, 1 << 20)
        XCTAssertEqual(Mutate.maxSequenceWork, 1 << 28)
    }

    func testEveryInterestValueIsWrittenExactlyAtEveryWidthAndEndian() {
        for w in [1, 2, 4, 8] {
            for v in Mutate.interestValues(width: w)! {
                let le = Mutate.boundaryInsert([UInt8](repeating: 0xAA, count: w), byteOffset: 0, width: w, val: v, endian: .little)
                let be = Mutate.boundaryInsert([UInt8](repeating: 0xAA, count: w), byteOffset: 0, width: w, val: v, endian: .big)
                var want: [UInt8] = []
                for i in 0..<w { want.append(UInt8(truncatingIfNeeded: v >> UInt64(8 * i))) }
                XCTAssertEqual(le, want, "w=\(w) v=\(v)")
                XCTAssertEqual(be, want.reversed(), "w=\(w) v=\(v)")
            }
        }
    }

    // MARK: flips ---------------------------------------------------------------------------

    func testFlipExamplesAndParity() {
        XCTAssertEqual(Mutate.bitFlip(b("0000"), bitOffset: 9, count: 1), b("0002"))
        XCTAssertEqual(Mutate.bitFlip(b("0000"), bitOffset: 0, count: 17), b("feff"))
        XCTAssertEqual(Mutate.bitFlip(b("0000"), bitOffset: 0, count: UInt64.max), b("00" + "80"))
        XCTAssertEqual(Mutate.byteFlip(b("000000"), byteOffset: 2, count: 2), b("ff00ff"))
        XCTAssertEqual(Mutate.byteFlip(b("0000"), byteOffset: 1, count: UInt64.max), b("ff00"))
        XCTAssertEqual(Mutate.bitFlip([], bitOffset: 3, count: 3), [])
        XCTAssertEqual(Mutate.byteFlip(b("12"), byteOffset: 3, count: 0), b("12"))
    }

    func testFlipsAreInvolutionsAndKeepLengthForAwkwardOffsetsAndCounts() {
        var inputs: [[UInt8]] = [[]]
        for x in 0..<256 { inputs.append([UInt8(x)]) }
        for x in stride(from: 0, to: 65536, by: 251) { inputs.append([UInt8(x >> 8), UInt8(x & 255)]) }
        for data in inputs {
            for off in [UInt64(0), 1, 9, UInt64.max] {
                for cnt in [UInt64(0), 1, 4, 17, UInt64.max] {
                    for bits in [true, false] {
                        let once = bits ? Mutate.bitFlip(data, bitOffset: off, count: cnt) : Mutate.byteFlip(data, byteOffset: off, count: cnt)
                        XCTAssertEqual(once.count, data.count)
                        // Same offset and count applied twice: every unit is back.
                        let twice = bits ? Mutate.bitFlip(once, bitOffset: off, count: cnt) : Mutate.byteFlip(once, byteOffset: off, count: cnt)
                        XCTAssertEqual(twice, data)
                    }
                }
            }
        }
    }

    // MARK: arithmetic and boundary ------------------------------------------------------------

    func testArithmeticWrapsEndianAndRefusesBadWidths() {
        XCTAssertEqual(Mutate.byteArithmetic(b("ff00"), byteOffset: 0, delta: 1, width: 2, endian: .little), b("0001"))
        XCTAssertEqual(Mutate.byteArithmetic(b("ff00"), byteOffset: 0, delta: 1, width: 2, endian: .big), b("ff01"))
        XCTAssertEqual(Mutate.byteArithmetic(b("ff"), byteOffset: 0, delta: 1, width: 1, endian: .little), b("00"))
        XCTAssertEqual(Mutate.byteArithmetic(b("00"), byteOffset: 0, delta: -1, width: 1, endian: .little), b("ff"))
        XCTAssertEqual(Mutate.byteArithmetic(b("0000000000000000"), byteOffset: 0, delta: Int64.min, width: 8, endian: .big), b("8000000000000000"))
        for w in [0, 3, 5, 7, 9, 16, -1] { XCTAssertEqual(Mutate.byteArithmetic(b("0102030405060708"), byteOffset: 0, delta: 5, width: w, endian: .little), b("0102030405060708")) }
        XCTAssertEqual(Mutate.byteArithmetic(b("01"), byteOffset: 0, delta: 5, width: 2, endian: .little), b("01"))
        XCTAssertEqual(Mutate.byteArithmetic([], byteOffset: 0, delta: 5, width: 1, endian: .little), [])
        // The offset wraps over len - width + 1 positions: here 2.
        XCTAssertEqual(Mutate.byteArithmetic(b("000000"), byteOffset: 3, delta: 1, width: 2, endian: .little), b("000100"))
        XCTAssertEqual(Mutate.byteArithmeticBig(b("000000"), byteOffset: big("-1"), delta: big("1"), width: 2, endian: .little), b("000100"))
    }

    func testBoundaryReducesValuesModuloTheWidthAndTwosComplementForNegatives() {
        XCTAssertEqual(Mutate.boundaryInsert(b("0000"), byteOffset: 0, width: 1, val: 0x1FF, endian: .little), b("ff00"))
        XCTAssertEqual(Mutate.boundaryInsertBig(b("0000"), byteOffset: 0, width: 2, val: big("-1"), endian: .little), b("ffff"))
        XCTAssertEqual(Mutate.boundaryInsertBig(b("0000"), byteOffset: 0, width: 2, val: big("65536"), endian: .big), b("0000"))
        XCTAssertEqual(Mutate.boundaryInsert(b("00000000"), byteOffset: 0, width: 4, val: 0x8000_0000, endian: .little), b("00000080"))
        XCTAssertEqual(Mutate.boundaryInsert(b("0000"), byteOffset: 0, width: 3, val: 1, endian: .little), b("0000"))
    }

    // MARK: blocks and splice ----------------------------------------------------------------------

    func testBlocksClampWrapAndNeverTrap() {
        XCTAssertEqual(Mutate.blockDelete(b("0102030405"), offset: 3, length: UInt64.max), b("010203"))
        XCTAssertEqual(Mutate.blockDelete(b("0102030405"), offset: UInt64.max, length: 1), b("02030405")) // 2^64-1 mod 5 == 0
        XCTAssertEqual(Mutate.blockDelete([], offset: 1, length: 1), [])
        XCTAssertEqual(Mutate.blockDelete(b("01"), offset: 0, length: 0), b("01"))
        XCTAssertEqual(Mutate.blockInsert([], offset: 7, insert: b("aabb")), b("aabb"))
        XCTAssertEqual(Mutate.blockInsert(b("0102"), offset: 5, insert: b("aa")), b("0102aa")) // 5 mod 3 == 2: the end
        XCTAssertEqual(Mutate.blockInsert(b("0102"), offset: 2, insert: []), b("0102"))
        XCTAssertEqual(Mutate.blockDuplicate(b("010203"), srcOffset: 2, length: UInt64.max, dstOffset: 0), b("03010203"))
        XCTAssertEqual(Mutate.blockDuplicate(b("010203"), srcOffset: 0, length: 2, dstOffset: UInt64.max), b("0102030102")) // 2^64-1 mod 4 == 3: the end
        XCTAssertEqual(Mutate.blockDuplicate([], srcOffset: 0, length: 3, dstOffset: 0), [])
        XCTAssertEqual(Mutate.blockDeleteBig(b("0102030405"), offset: big("-1"), length: big("1")), b("01020304"))
        XCTAssertEqual(Mutate.blockDeleteBig(b("0102030405"), offset: big("0"), length: big("-3")), b("0102030405"))
    }

    func testUnequalSpliceSplitsWithTargetAndDonorSwapped() {
        // The corpus splice has equal splits (2, 2), which cannot tell the two
        // splits apart; these can.
        let a = b("01020304"), d = b("0506070809")
        XCTAssertEqual(Mutate.splice(a, d, targetSplit: 1, donorSplit: 3), b("010809"))
        XCTAssertEqual(Mutate.splice(d, a, targetSplit: 1, donorSplit: 3), b("0504"))
        XCTAssertEqual(Mutate.splice(a, d, targetSplit: 3, donorSplit: 1), b("010203060708" + "09"))
        XCTAssertEqual(Mutate.splice(d, a, targetSplit: 3, donorSplit: 1), b("050607" + "020304"))
        XCTAssertEqual(Mutate.splice(a, d, targetSplit: UInt64.max, donorSplit: UInt64.max), a)
        XCTAssertEqual(Mutate.splice(a, d, targetSplit: 0, donorSplit: 0), d)
        XCTAssertEqual(Mutate.spliceBig(a, d, targetSplit: big("-5"), donorSplit: big("99999999999999999999999")), [])
        XCTAssertEqual(Mutate.spliceLength(targetLen: 4, donorLen: 5, targetSplit: 1, donorSplit: 3), 3)
    }

    // MARK: mutateSequence --------------------------------------------------------------------------

    func testSequenceCapsAndBasicShape() throws {
        XCTAssertThrowsError(try Mutate.mutateSequence([0], seed: 1, steps: Mutate.maxIterations + 1, maxLen: 10)) {
            XCTAssertEqual($0 as? FuzzError, .invalidArgument("steps exceeds maxIterations"))
        }
        XCTAssertThrowsError(try Mutate.mutateSequence([UInt8](repeating: 0, count: Mutate.maxInputBytes + 1), seed: 1, steps: 1, maxLen: 10))
        XCTAssertEqual(try Mutate.mutateSequence(b("0102"), seed: 9, steps: -5, maxLen: 10), b("0102"))
        XCTAssertEqual(try Mutate.mutateSequence(b("0102"), seed: 9, steps: 0, maxLen: 10), b("0102"))
        XCTAssertEqual(try Mutate.mutateSequence(b("0102"), seed: 9, steps: 5, maxLen: 10), try Mutate.mutateSequence(b("0102"), seed: 9, steps: 5, maxLen: 10))
        XCTAssertNotEqual(try Mutate.mutateSequence(b("0102030405"), seed: 9, steps: 40, maxLen: 10), try Mutate.mutateSequence(b("0102030405"), seed: 10, steps: 40, maxLen: 10))
        // maxLen stops growth (after the draws) but never shrinks an input.
        for seed in UInt64(0)..<20 {
            XCTAssertLessThanOrEqual(try Mutate.mutateSequence(b("0102030405060708"), seed: seed, steps: 200, maxLen: 10).count, 10)
        }
        XCTAssertEqual(try Mutate.mutateSequence([], seed: 3, steps: 0, maxLen: 0), [])
        XCTAssertNoThrow(try Mutate.mutateSequence(b("00"), seed: 3, steps: 50, maxLen: Int.min))
        XCTAssertNoThrow(try Mutate.mutateSequence(b("00"), seed: 3, steps: 50, maxLen: Int.max))
    }

    func testSequenceWorkIsTheBytesReallyCopiedAndAFailedCallReportsIt() throws {
        let data = b("0102030405060708")
        var expected = data.count
        for k in 1...30 {
            let out = try Mutate.mutateSequence(data, seed: 77, steps: k, maxLen: Mutate.maxInputBytes)
            expected += max(out.count, 1)
            let counted = try Mutate.counted(data, seed: 77, steps: k, maxLen: Mutate.maxInputBytes, workCap: Mutate.maxSequenceWork)
            XCTAssertEqual(counted.work, expected, "step \(k)")
            XCTAssertEqual(counted.data, out)
        }
        // A cap just under the real work of 30 steps fails, and reports real work.
        do {
            _ = try Mutate.counted(data, seed: 77, steps: 30, maxLen: Mutate.maxInputBytes, workCap: expected - 1)
            XCTFail("must exhaust")
        } catch FuzzError.workExhausted(let done) {
            XCTAssertGreaterThanOrEqual(done, data.count)
            XCTAssertLessThanOrEqual(done, expected + Mutate.maxInputBytes)
        }
        // The initial copy alone counts: a cap below the input length fails with 0 done.
        do {
            _ = try Mutate.counted(data, seed: 1, steps: 1, maxLen: 10, workCap: 3)
            XCTFail("must exhaust")
        } catch FuzzError.workExhausted(let done) { XCTAssertEqual(done, 0) }
        // A cap of exactly the real work passes.
        XCTAssertNoThrow(try Mutate.counted(data, seed: 77, steps: 30, maxLen: Mutate.maxInputBytes, workCap: expected))
    }

    // MARK: the two integer paths cannot drift, and nothing traps -------------------------------------

    func testExtremeBigIntegersNeverTrapAndAgreeWithTheMachinePathWhereTheyFit() {
        let huge = big(String(repeating: "9", count: 300))
        let negHuge = big("-" + String(repeating: "9", count: 300))
        let data = b("0102030405060708090a")
        for v in [huge, negHuge, big("0"), big("-1"), big("18446744073709551616"), big("-18446744073709551617")] {
            _ = Mutate.bitFlipBig(data, bitOffset: v, count: v)
            _ = Mutate.byteFlipBig(data, byteOffset: v, count: huge)
            _ = Mutate.byteArithmeticBig(data, byteOffset: v, delta: v, width: 4, endian: .big)
            _ = Mutate.boundaryInsertBig(data, byteOffset: v, width: 8, val: v, endian: .little)
            _ = Mutate.blockDeleteBig(data, offset: v, length: v)
            _ = Mutate.blockInsertBig(data, offset: v, insert: b("aa"))
            _ = Mutate.blockDuplicateBig(data, srcOffset: v, length: v, dstOffset: v)
            _ = Mutate.spliceBig(data, data, targetSplit: v, donorSplit: v)
        }
        var rng = SplitMix64(seed: 5)
        for _ in 0..<400 {
            let len = Int(rng.nextU64() % 12)
            let d = (0..<len).map { _ in UInt8(truncatingIfNeeded: rng.nextU64()) }
            let o = rng.nextU64(), c = rng.nextU64() % 100, l = rng.nextU64() % 20
            XCTAssertEqual(Mutate.bitFlip(d, bitOffset: o, count: c), Mutate.bitFlipBig(d, bitOffset: BigInt(o), count: BigInt(c)))
            XCTAssertEqual(Mutate.byteFlip(d, byteOffset: o, count: c), Mutate.byteFlipBig(d, byteOffset: BigInt(o), count: BigInt(c)))
            XCTAssertEqual(Mutate.blockDelete(d, offset: o, length: l), Mutate.blockDeleteBig(d, offset: BigInt(o), length: BigInt(l)))
            XCTAssertEqual(Mutate.blockDuplicate(d, srcOffset: o, length: l, dstOffset: c), Mutate.blockDuplicateBig(d, srcOffset: BigInt(o), length: BigInt(l), dstOffset: BigInt(c)))
            XCTAssertEqual(Mutate.splice(d, d, targetSplit: o, donorSplit: c), Mutate.spliceBig(d, d, targetSplit: BigInt(o), donorSplit: BigInt(c)))
        }
    }
}
