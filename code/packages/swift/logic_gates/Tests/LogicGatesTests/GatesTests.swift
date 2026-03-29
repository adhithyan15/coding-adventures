import XCTest
@testable import LogicGates

// ============================================================================
// GatesTests — Comprehensive tests for primitive and derived gate functions
// ============================================================================
//
// Test strategy:
//   1. Exhaustive truth-table tests for all 7 primitive gates
//   2. NAND-derived implementations match their primitive counterparts
//   3. Multi-input (andN / orN) for 2, 3, and 4 inputs
//   4. Input validation: out-of-range values throw the correct error
//   5. Version string is defined
// ============================================================================

// MARK: - NOT Gate

final class NOTGateTests: XCTestCase {

    func testNOT0() throws { XCTAssertEqual(try notGate(0), 1) }
    func testNOT1() throws { XCTAssertEqual(try notGate(1), 0) }

    func testNOTInvalidPositive() {
        XCTAssertThrowsError(try notGate(2)) { e in
            guard case LogicGateError.invalidBit(let name, let got) = e else {
                return XCTFail("wrong error type: \(e)")
            }
            XCTAssertEqual(name, "a")
            XCTAssertEqual(got, 2)
        }
    }

    func testNOTInvalidNegative() {
        XCTAssertThrowsError(try notGate(-1))
    }
}

// MARK: - AND Gate

final class ANDGateTests: XCTestCase {

    func testAND00() throws { XCTAssertEqual(try andGate(0, 0), 0) }
    func testAND01() throws { XCTAssertEqual(try andGate(0, 1), 0) }
    func testAND10() throws { XCTAssertEqual(try andGate(1, 0), 0) }
    func testAND11() throws { XCTAssertEqual(try andGate(1, 1), 1) }

    func testANDInvalidFirst()  { XCTAssertThrowsError(try andGate(2, 0)) }
    func testANDInvalidSecond() { XCTAssertThrowsError(try andGate(0, 2)) }
}

// MARK: - OR Gate

final class ORGateTests: XCTestCase {

    func testOR00() throws { XCTAssertEqual(try orGate(0, 0), 0) }
    func testOR01() throws { XCTAssertEqual(try orGate(0, 1), 1) }
    func testOR10() throws { XCTAssertEqual(try orGate(1, 0), 1) }
    func testOR11() throws { XCTAssertEqual(try orGate(1, 1), 1) }

    func testORInvalidFirst()  { XCTAssertThrowsError(try orGate(-1, 0)) }
    func testORInvalidSecond() { XCTAssertThrowsError(try orGate(1, 3)) }
}

// MARK: - XOR Gate

final class XORGateTests: XCTestCase {

    func testXOR00() throws { XCTAssertEqual(try xorGate(0, 0), 0) }
    func testXOR01() throws { XCTAssertEqual(try xorGate(0, 1), 1) }
    func testXOR10() throws { XCTAssertEqual(try xorGate(1, 0), 1) }
    func testXOR11() throws { XCTAssertEqual(try xorGate(1, 1), 0) }

    func testXORInvalidFirst() { XCTAssertThrowsError(try xorGate(5, 0)) }
}

// MARK: - NAND Gate

final class NANDGateTests: XCTestCase {

    func testNAND00() throws { XCTAssertEqual(try nandGate(0, 0), 1) }
    func testNAND01() throws { XCTAssertEqual(try nandGate(0, 1), 1) }
    func testNAND10() throws { XCTAssertEqual(try nandGate(1, 0), 1) }
    func testNAND11() throws { XCTAssertEqual(try nandGate(1, 1), 0) }

    func testNANDInvalidInput() { XCTAssertThrowsError(try nandGate(1, 2)) }
}

// MARK: - NOR Gate

final class NORGateTests: XCTestCase {

    func testNOR00() throws { XCTAssertEqual(try norGate(0, 0), 1) }
    func testNOR01() throws { XCTAssertEqual(try norGate(0, 1), 0) }
    func testNOR10() throws { XCTAssertEqual(try norGate(1, 0), 0) }
    func testNOR11() throws { XCTAssertEqual(try norGate(1, 1), 0) }

    func testNORInvalidInput() { XCTAssertThrowsError(try norGate(0, -1)) }
}

// MARK: - XNOR Gate

final class XNORGateTests: XCTestCase {

    func testXNOR00() throws { XCTAssertEqual(try xnorGate(0, 0), 1) }
    func testXNOR01() throws { XCTAssertEqual(try xnorGate(0, 1), 0) }
    func testXNOR10() throws { XCTAssertEqual(try xnorGate(1, 0), 0) }
    func testXNOR11() throws { XCTAssertEqual(try xnorGate(1, 1), 1) }

    func testXNORInvalidInput() { XCTAssertThrowsError(try xnorGate(2, 1)) }
}

// MARK: - NAND-derived NOT

final class NANDNotTests: XCTestCase {

    func testNandNOT0() throws { XCTAssertEqual(try nandNot(0), 1) }
    func testNandNOT1() throws { XCTAssertEqual(try nandNot(1), 0) }

    // Must agree with the primitive NOT gate
    func testMatchesPrimitive() throws {
        for a in [0, 1] {
            XCTAssertEqual(try nandNot(a), try notGate(a),
                "nandNot(\(a)) should match notGate(\(a))")
        }
    }

    func testNandNOTInvalid() { XCTAssertThrowsError(try nandNot(2)) }
}

// MARK: - NAND-derived AND

final class NANDAndTests: XCTestCase {

    func testNandAND00() throws { XCTAssertEqual(try nandAnd(0, 0), 0) }
    func testNandAND01() throws { XCTAssertEqual(try nandAnd(0, 1), 0) }
    func testNandAND10() throws { XCTAssertEqual(try nandAnd(1, 0), 0) }
    func testNandAND11() throws { XCTAssertEqual(try nandAnd(1, 1), 1) }

    func testMatchesPrimitive() throws {
        for a in [0, 1] {
            for b in [0, 1] {
                XCTAssertEqual(try nandAnd(a, b), try andGate(a, b),
                    "nandAnd(\(a),\(b)) should match andGate(\(a),\(b))")
            }
        }
    }

    func testNandANDInvalid() { XCTAssertThrowsError(try nandAnd(3, 1)) }
}

// MARK: - NAND-derived OR

final class NANDOrTests: XCTestCase {

    func testNandOR00() throws { XCTAssertEqual(try nandOr(0, 0), 0) }
    func testNandOR01() throws { XCTAssertEqual(try nandOr(0, 1), 1) }
    func testNandOR10() throws { XCTAssertEqual(try nandOr(1, 0), 1) }
    func testNandOR11() throws { XCTAssertEqual(try nandOr(1, 1), 1) }

    func testMatchesPrimitive() throws {
        for a in [0, 1] {
            for b in [0, 1] {
                XCTAssertEqual(try nandOr(a, b), try orGate(a, b),
                    "nandOr(\(a),\(b)) should match orGate(\(a),\(b))")
            }
        }
    }

    func testNandORInvalid() { XCTAssertThrowsError(try nandOr(1, 9)) }
}

// MARK: - NAND-derived XOR

final class NANDXorTests: XCTestCase {

    func testNandXOR00() throws { XCTAssertEqual(try nandXor(0, 0), 0) }
    func testNandXOR01() throws { XCTAssertEqual(try nandXor(0, 1), 1) }
    func testNandXOR10() throws { XCTAssertEqual(try nandXor(1, 0), 1) }
    func testNandXOR11() throws { XCTAssertEqual(try nandXor(1, 1), 0) }

    func testMatchesPrimitive() throws {
        for a in [0, 1] {
            for b in [0, 1] {
                XCTAssertEqual(try nandXor(a, b), try xorGate(a, b),
                    "nandXor(\(a),\(b)) should match xorGate(\(a),\(b))")
            }
        }
    }

    func testNandXORInvalid() { XCTAssertThrowsError(try nandXor(-2, 1)) }
}

// MARK: - Multi-input AND

final class AndNTests: XCTestCase {

    func testAndNAllOnes2() throws { XCTAssertEqual(try andN([1, 1]), 1) }
    func testAndNOneFalse2() throws { XCTAssertEqual(try andN([1, 0]), 0) }

    func testAndN3AllOnes() throws { XCTAssertEqual(try andN([1, 1, 1]), 1) }
    func testAndN3OneFalse() throws { XCTAssertEqual(try andN([1, 0, 1]), 0) }

    func testAndN4AllOnes() throws { XCTAssertEqual(try andN([1, 1, 1, 1]), 1) }
    func testAndN4AllFalse() throws { XCTAssertEqual(try andN([0, 0, 0, 0]), 0) }
    func testAndN4Mixed() throws { XCTAssertEqual(try andN([1, 1, 0, 1]), 0) }

    func testAndNTooFewInputs() {
        XCTAssertThrowsError(try andN([1])) { e in
            guard case LogicGateError.insufficientInputs = e else {
                return XCTFail("wrong error: \(e)")
            }
        }
    }

    func testAndNEmptyArray() {
        XCTAssertThrowsError(try andN([]))
    }

    func testAndNInvalidBit() {
        XCTAssertThrowsError(try andN([1, 2]))
    }
}

// MARK: - Multi-input OR

final class OrNTests: XCTestCase {

    func testOrNAllZeros2() throws { XCTAssertEqual(try orN([0, 0]), 0) }
    func testOrNOneTrue2() throws { XCTAssertEqual(try orN([0, 1]), 1) }

    func testOrN3AllZeros() throws { XCTAssertEqual(try orN([0, 0, 0]), 0) }
    func testOrN3OneTrue() throws { XCTAssertEqual(try orN([0, 0, 1]), 1) }

    func testOrN4AllOnes() throws { XCTAssertEqual(try orN([1, 1, 1, 1]), 1) }
    func testOrN4Mixed() throws { XCTAssertEqual(try orN([0, 1, 0, 0]), 1) }

    func testOrNTooFewInputs() {
        XCTAssertThrowsError(try orN([0])) { e in
            guard case LogicGateError.insufficientInputs = e else {
                return XCTFail("wrong error: \(e)")
            }
        }
    }

    func testOrNInvalidBit() {
        XCTAssertThrowsError(try orN([0, -1]))
    }
}

// MARK: - Error Description

final class ErrorDescriptionTests: XCTestCase {

    func testInvalidBitDescription() {
        let e = LogicGateError.invalidBit(name: "a", got: 5)
        XCTAssertTrue(e.description.contains("a"))
        XCTAssertTrue(e.description.contains("5"))
    }

    func testInsufficientInputsDescription() {
        let e = LogicGateError.insufficientInputs(minimum: 2, got: 1)
        XCTAssertTrue(e.description.contains("2"))
        XCTAssertTrue(e.description.contains("1"))
    }
}

// MARK: - Version

final class VersionTests: XCTestCase {
    func testVersionIsDefined() {
        XCTAssertFalse(LogicGates.version.isEmpty)
    }
}
