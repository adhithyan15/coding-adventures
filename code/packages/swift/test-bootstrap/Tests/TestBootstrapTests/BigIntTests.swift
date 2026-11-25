import XCTest
@testable import TestBootstrap

final class BigIntTests: XCTestCase {
    private func b(_ s: String) -> BigInt { try! BigInt.parse(s) }

    func testParseAndPrintRoundTrip() throws {
        for s in ["0", "1", "-1", "4294967295", "4294967296", "18446744073709551616",
                  "-170141183460469231731687303715884105728", "340282366920938463463374607431768211455",
                  String(repeating: "9", count: 300)] {
            XCTAssertEqual(b(s).description, s)
        }
    }

    func testParseRejectsNonCanonicalAndOverlong() {
        for s in ["", "-", "+1", "01", "-0", "1a", " 1", "1.0", "--1"] {
            XCTAssertThrowsError(try BigInt.parse(s), s) { XCTAssertEqual($0 as? BigIntError, .notCanonical) }
        }
        XCTAssertThrowsError(try BigInt.parse(String(repeating: "9", count: 301))) {
            XCTAssertEqual($0 as? BigIntError, .tooLong)
        }
        XCTAssertNoThrow(try BigInt.parse("-" + String(repeating: "9", count: 300)))
    }

    func testInitFromIntAndUInt64Extremes() {
        XCTAssertEqual(BigInt(Int.min).description, "-9223372036854775808")
        XCTAssertEqual(BigInt(Int.max).description, "9223372036854775807")
        XCTAssertEqual(BigInt(UInt64.max).description, "18446744073709551615")
        XCTAssertEqual(BigInt(0), BigInt.zero)
        XCTAssertEqual(BigInt(Int.min).asInt, Int.min)
        XCTAssertEqual(BigInt(Int.max).asInt, Int.max)
        XCTAssertNil((BigInt(Int.max) + .one).asInt)
        XCTAssertNil((BigInt(Int.min) - .one).asInt)
        XCTAssertEqual(BigInt(UInt64.max).asUInt64, UInt64.max)
        XCTAssertNil((BigInt(UInt64.max) + .one).asUInt64)
        XCTAssertNil(BigInt(-1).asUInt64)
    }

    func testAddSubtractSigns() {
        XCTAssertEqual(b("5") + b("-7"), b("-2"))
        XCTAssertEqual(b("-5") + b("7"), b("2"))
        XCTAssertEqual(b("-5") + b("-7"), b("-12"))
        XCTAssertEqual(b("5") - b("5"), .zero)
        XCTAssertFalse((b("5") - b("5")).description.hasPrefix("-"))
        XCTAssertEqual(b("4294967295") + b("1"), b("4294967296"))
        XCTAssertEqual(b("18446744073709551615") + b("1"), b("18446744073709551616"))
        XCTAssertEqual(b("18446744073709551616") - b("1"), b("18446744073709551615"))
        XCTAssertEqual(-b("3"), b("-3"))
        XCTAssertEqual(-BigInt.zero, .zero)
    }

    func testMultiply() {
        XCTAssertEqual(b("4294967295") * b("4294967295"), b("18446744065119617025"))
        XCTAssertEqual(b("-12345678901234567890") * b("98765432109876543210"), b("-1219326311370217952237463801111263526900"))
        XCTAssertEqual(b("0") * b("5"), .zero)
        XCTAssertEqual(b("-3") * b("-4"), b("12"))
    }

    func testQuotientAndRemainderTruncates() {
        func qr(_ a: String, _ d: String) -> (String, String) {
            let r = b(a).quotientAndRemainder(dividingBy: b(d))
            return (r.quotient.description, r.remainder.description)
        }
        XCTAssertTrue(qr("7", "2") == ("3", "1"))
        XCTAssertTrue(qr("-7", "2") == ("-3", "-1"))
        XCTAssertTrue(qr("7", "-2") == ("-3", "1"))
        XCTAssertTrue(qr("1", "5") == ("0", "1"))
        XCTAssertEqual(qr("1000000000000000000000000000000", "1000000007").0, "999999993000000048999")
        // Multi-limb divisor: 2^200 / (2^100 + 1) = 2^100 - 1 remainder 1.
        let big = BigInt.one.shiftedLeft(200), d = BigInt.one.shiftedLeft(100) + .one
        let r = big.quotientAndRemainder(dividingBy: d)
        XCTAssertEqual(r.quotient, BigInt.one.shiftedLeft(100) - .one)
        XCTAssertEqual(r.remainder, .one)
        // Round trip: q * d + r == n.
        let n = b("123456789012345678901234567890123456789"), m = b("98765432109876543211")
        let t = n.quotientAndRemainder(dividingBy: m)
        XCTAssertEqual(t.quotient * m + t.remainder, n)
    }

    func testShiftsFloorForNegatives() {
        XCTAssertEqual(b("-3").shiftedRight(1), b("-2"))
        XCTAssertEqual(b("3").shiftedRight(1), b("1"))
        XCTAssertEqual(b("-4").shiftedRight(1), b("-2"))
        XCTAssertEqual(b("-1").shiftedRight(1), b("-1"))
        XCTAssertEqual(b("-1").shiftedRight(100), b("-1"))
        XCTAssertEqual(b("1").shiftedRight(100), .zero)
        XCTAssertEqual(b("-5").shiftedRight(64), b("-1"))
        XCTAssertEqual(b("1").shiftedLeft(64), b("18446744073709551616"))
        XCTAssertEqual(b("3").shiftedLeft(33), b("25769803776"))
        XCTAssertEqual(BigInt.zero.shiftedLeft(500), .zero)
        XCTAssertEqual(b("18446744073709551616").shiftedRight(64), .one)
        XCTAssertEqual(BigInt.one.shiftedLeft(70).shiftedRight(5), BigInt.one.shiftedLeft(65))
    }

    func testOrderingAndEquality() {
        XCTAssertTrue(b("-5") < b("-3"))
        XCTAssertTrue(b("-3") < b("2"))
        XCTAssertTrue(b("2") < b("100000000000"))
        XCTAssertTrue(b("-100000000000") < b("-5"))
        XCTAssertEqual(BigInt.compare(b("7"), b("7")), 0)
        XCTAssertEqual(b("10"), b("10"))
        XCTAssertEqual(Set([b("1"), b("1"), b("2")]).count, 2)
    }

    func testPropertiesAndConversions() {
        XCTAssertEqual(b("-8").signum, -1)
        XCTAssertEqual(BigInt.zero.signum, 0)
        XCTAssertTrue(b("-4").isEven)
        XCTAssertFalse(b("-3").isEven)
        XCTAssertEqual(b("255").bitWidth, 8)
        XCTAssertEqual(b("256").bitWidth, 9)
        XCTAssertEqual(BigInt.zero.bitWidth, 0)
        XCTAssertEqual(b("-7").magnitude, b("7"))
        XCTAssertEqual(b("-1").lowU64, UInt64.max)
        XCTAssertEqual(b("18446744073709551617").lowU64, 1)
        XCTAssertEqual(b("-18446744073709551617").lowU64, UInt64.max)
        XCTAssertEqual(BigInt.zero.lowU64, 0)
    }

    func testCapsArePreconditionsOnlyBeyondTheHeadroom() {
        // 1100 bits is the ceiling; 1024 (the rejection term for a 1000-bit span) fits.
        let x = BigInt.one.shiftedLeft(1024)
        XCTAssertEqual(x.bitWidth, 1025)
        XCTAssertEqual(BigInt.maxBits, 1100)
        XCTAssertEqual(BigInt.apiMaxBits, 1000)
        XCTAssertEqual(x.quotientAndRemainder(dividingBy: BigInt.one.shiftedLeft(999) + BigInt(7)).remainder.bitWidth <= 1000, true)
    }

    func testDecimalPrintMatchesPythonForBigPowers() {
        XCTAssertEqual(BigInt.one.shiftedLeft(127).description, "170141183460469231731687303715884105728")
        XCTAssertEqual(BigInt.one.shiftedLeft(256).description,
                       "115792089237316195423570985008687907853269984665640564039457584007913129639936")
    }
}
