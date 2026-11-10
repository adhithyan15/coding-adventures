import Foundation
import XCTest

@testable import CtCompare

// ============================================================================
// Tests for CtCompare (CT01).
//
// Part 1 (CorpusTests) replays every case of the shared CT01 corpus,
// code/specs/fixtures/ct-compare-v1/cases.json. Part 2 adds the checks the
// corpus only samples: every byte position, all 256x256 byte pairs through
// selection, length mismatches, all 64 single-bit flips of a UInt64, and
// invalid choices.
// ============================================================================

private func fromHex(_ hex: String) -> [UInt8] {
    var out: [UInt8] = []
    var index = hex.startIndex
    while index < hex.endIndex {
        let next = hex.index(index, offsetBy: 2)
        out.append(UInt8(hex[index..<next], radix: 16)!)
        index = next
    }
    return out
}

private func toHex(_ bytes: [UInt8]) -> String {
    return bytes.map { String(format: "%02x", UInt32($0)) }.joined()
}

// Corpus values are public test data, so mapping a Bool to a bit with a
// conditional is fine here (tests are never shipped).
private func bitOf(_ b: Bool) -> UInt8 { return b ? 1 : 0 }

final class CorpusTests: XCTestCase {
    private func loadCorpus() throws -> [String: Any] {
        // Tests/CtCompareTests/CtCompareTests.swift -> up six levels is code/.
        var url = URL(fileURLWithPath: #filePath)
        for _ in 0..<6 { url.deleteLastPathComponent() }
        url.appendPathComponent("specs/fixtures/ct-compare-v1/cases.json")
        let data = try Data(contentsOf: url)
        return try JSONSerialization.jsonObject(with: data) as! [String: Any]
    }

    func testEveryCorpusCase() throws {
        let corpus = try loadCorpus()
        XCTAssertEqual(corpus["profile"] as? String, "ct-compare-v1")
        let cases = corpus["cases"] as! [[String: Any]]
        XCTAssertEqual(cases.count, 21)

        for c in cases {
            let id = c["id"] as! String
            let op = c["operation"] as! String
            let expected = c["expected"] as! [String: Any]
            switch op {
            case "eq", "eq_fixed":
                let l = fromHex(c["left_hex"] as! String)
                let r = fromHex(c["right_hex"] as! String)
                let want = expected["equal"] as! Bool
                if op == "eq" {
                    XCTAssertEqual(ctEq(l, r), want, id)
                    XCTAssertEqual(ctEqBit(l, r), bitOf(want), id)
                } else {
                    XCTAssertEqual(l.count, c["fixed_size_bytes"] as! Int, id)
                    XCTAssertEqual(ctEqFixed(l, r), want, id)
                    XCTAssertEqual(ctEqFixedBit(l, r), bitOf(want), id)
                }
            case "select_bytes":
                let l = fromHex(c["left_hex"] as! String)
                let r = fromHex(c["right_hex"] as! String)
                let choice = bitOf(c["choice"] as! Bool)
                if let errorId = expected["error_id"] as? String {
                    XCTAssertEqual(errorId, "CT_SELECT_LENGTH_MISMATCH", id)
                    XCTAssertThrowsError(try ctSelectBytes(l, r, choice: choice), id) { error in
                        XCTAssertEqual(error as? CtCompareError, .selectLengthMismatch, id)
                    }
                } else {
                    var out = try ctSelectBytes(l, r, choice: choice)
                    XCTAssertEqual(toHex(out), expected["output_hex"] as? String, id)
                    // Fresh storage: mutating the result leaves both inputs alone.
                    if !out.isEmpty { out[0] ^= 0xFF }
                    XCTAssertEqual(toHex(l), c["left_hex"] as? String, id)
                    XCTAssertEqual(toHex(r), c["right_hex"] as? String, id)
                }
            case "eq_u64":
                let l = UInt64(c["left_u64"] as! String, radix: 16)!
                let r = UInt64(c["right_u64"] as! String, radix: 16)!
                let want = expected["equal"] as! Bool
                XCTAssertEqual(ctEqU64(l, r), want, id)
                XCTAssertEqual(ctEqU64Bit(l, r), bitOf(want), id)
            default:
                XCTFail("unknown corpus operation \(op)")
            }
        }
    }
}

final class EqualityTests: XCTestCase {
    func testEqualAndEmpty() {
        XCTAssertTrue(ctEq([], []))
        XCTAssertTrue(ctEq([1, 2, 3], [1, 2, 3]))
        XCTAssertEqual(ctEqBit([9], [9]), 1)
        XCTAssertEqual(ctEqFixedBit([], []), 1)
    }

    func testMismatchAtEveryBytePositionOf64Bytes() {
        let base: [UInt8] = (1...64).map { UInt8(($0 * 7) % 256) }
        for pos in 0..<64 {
            var other = base
            other[pos] ^= 1
            XCTAssertFalse(ctEq(base, other), "position \(pos)")
            XCTAssertEqual(ctEqBit(base, other), 0)
            XCTAssertFalse(ctEqFixed(base, other))
        }
    }

    func testEverySingleBitDifferenceOfEveryByteValue() {
        for v in 0...255 {
            let byte = UInt8(v)
            XCTAssertTrue(ctEq([byte], [byte]))
            for bit in 0..<8 {
                XCTAssertFalse(ctEq([byte], [byte ^ UInt8(1 << bit)]))
            }
        }
    }

    func testEveryNonZeroAccumulatorIsDifferent() {
        for v in 1...255 {
            XCTAssertEqual(ctEqBit([0, 0], [0, UInt8(v)]), 0)
        }
    }

    func testLengthMismatch() {
        XCTAssertFalse(ctEq([1], [1, 2]))
        XCTAssertFalse(ctEq([1, 2], [1]))
        XCTAssertFalse(ctEq([], [0]))
        XCTAssertEqual(ctEqBit([], [0]), 0)
        XCTAssertFalse(ctEqFixed([1, 2, 3], [1, 2, 3, 4]))
    }

    func testNulAndHighBitBytesAreData() {
        XCTAssertFalse(ctEq([0x61, 0, 0x62], [0x61, 0, 0x63]))
        XCTAssertTrue(ctEq([0xff, 0x80], [0xff, 0x80]))
        XCTAssertFalse(ctEq([0xff], [0x7f]))
    }
}

final class SelectionTests: XCTestCase {
    func testPicksLeftForOneAndRightForZero() throws {
        XCTAssertEqual(try ctSelectBytes([65, 66], [67, 68], choice: 1), [65, 66])
        XCTAssertEqual(try ctSelectBytes([65, 66], [67, 68], choice: 0), [67, 68])
    }

    func testAll256ByteValuesAgainstAll256ByteValues() throws {
        let right = (0...255).map { UInt8($0) }
        for lv in 0...255 {
            let left = [UInt8](repeating: UInt8(lv), count: 256)
            XCTAssertEqual(try ctSelectBytes(left, right, choice: 1), left)
            XCTAssertEqual(try ctSelectBytes(left, right, choice: 0), right)
        }
    }

    func testEmptyInputs() throws {
        XCTAssertEqual(try ctSelectBytes([], [], choice: 1), [])
        XCTAssertEqual(try ctSelectBytes([], [], choice: 0), [])
    }

    func testLengthMismatchThrows() {
        for (l, r) in [([1], [1, 2]), ([1, 2], [1]), ([], [1])] as [([UInt8], [UInt8])] {
            XCTAssertThrowsError(try ctSelectBytes(l, r, choice: 1)) { error in
                XCTAssertEqual(error as? CtCompareError, .selectLengthMismatch)
            }
        }
    }

    func testLengthIsCheckedBeforeTheChoice() {
        XCTAssertThrowsError(try ctSelectBytes([1], [1, 2], choice: 7)) { error in
            XCTAssertEqual(error as? CtCompareError, .selectLengthMismatch)
        }
    }

    func testEveryChoiceOtherThanZeroAndOneIsRejected() {
        for choice in 2...255 {
            XCTAssertThrowsError(try ctSelectBytes([1], [2], choice: UInt8(choice))) { error in
                XCTAssertEqual(error as? CtCompareError, .invalidChoice)
            }
        }
    }

    func testComposesWithTheBitReturningEqualities() throws {
        XCTAssertEqual(try ctSelectBytes([1], [2], choice: ctEqBit([7], [7])), [1])
        XCTAssertEqual(try ctSelectBytes([1], [2], choice: ctEqBit([7], [8])), [2])
        XCTAssertEqual(try ctSelectBytes([1], [2], choice: ctEqU64Bit(5, 6)), [2])
    }

    func testResultIsIndependentOfTheInputs() throws {
        var l: [UInt8] = [1, 2, 3]
        let r: [UInt8] = [4, 5, 6]
        let out = try ctSelectBytes(l, r, choice: 1)
        l[0] = 99  // arrays are values: mutating the input cannot change the result
        XCTAssertEqual(out, [1, 2, 3])
    }
}

final class U64Tests: XCTestCase {
    func testEqualValuesIncludingZeroMaxAndSignBit() {
        for v: UInt64 in [0, 1, .max, 1 << 63, (1 << 63) - 1, 0x0123_4567_89AB_CDEF] {
            XCTAssertTrue(ctEqU64(v, v))
            XCTAssertEqual(ctEqU64Bit(v, v), 1)
        }
    }

    func testEachOf64SingleBitFlipsFromSeveralBases() {
        let bases: [UInt64] = [0, .max, 0x5555_5555_5555_5555, 0x0123_4567_89AB_CDEF, 1 << 63]
        for base in bases {
            for bit in 0..<64 {
                let flipped = base ^ (UInt64(1) << UInt64(bit))
                XCTAssertFalse(ctEqU64(base, flipped), "bit \(bit)")
                XCTAssertFalse(ctEqU64(flipped, base))
                XCTAssertEqual(ctEqU64Bit(base, flipped), 0)
            }
        }
    }

    func testExtremesAreDistinct() {
        XCTAssertFalse(ctEqU64(0, 1 << 63))
        XCTAssertFalse(ctEqU64(0, .max))
        XCTAssertFalse(ctEqU64((1 << 63) - 1, .max))
    }
}
