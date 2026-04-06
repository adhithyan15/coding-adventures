// ============================================================================
// BitsetTests.swift
// ============================================================================

import XCTest
@testable import Bitset

final class BitsetTests: XCTestCase {
    func testNewZero() {
        let b = Bitset(size: 0)
        XCTAssertEqual(b.size, 0)
        XCTAssertEqual(b.capacity, 0)
        XCTAssertEqual(b.popcount(), 0)
        XCTAssertTrue(b.none())
        XCTAssertTrue(b.all())
    }

    func testNew64() {
        let b = Bitset(size: 64)
        XCTAssertEqual(b.size, 64)
        XCTAssertEqual(b.capacity, 64)
    }

    func testNew100() {
        let b = Bitset(size: 100)
        XCTAssertEqual(b.size, 100)
        XCTAssertEqual(b.capacity, 128)
    }

    func testSetAndAutoGrow() {
        let b = Bitset(size: 0)
        b.set(0)
        XCTAssertEqual(b.size, 1)
        XCTAssertEqual(b.capacity, 64)

        b.set(100)
        XCTAssertEqual(b.size, 101)
        XCTAssertEqual(b.capacity, 128)

        XCTAssertTrue(b.test(0))
        XCTAssertTrue(b.test(100))
        XCTAssertFalse(b.test(50))
    }

    func testClear() {
        let b = Bitset(size: 10)
        b.set(5)
        XCTAssertTrue(b.test(5))
        b.clear(5)
        XCTAssertFalse(b.test(5))
    }

    func testToggle() {
        let b = Bitset(size: 10)
        b.toggle(5)
        XCTAssertTrue(b.test(5))
        b.toggle(5)
        XCTAssertFalse(b.test(5))
        
        // Auto grow on toggle
        b.toggle(100)
        XCTAssertTrue(b.test(100))
        XCTAssertEqual(b.size, 101)
    }

    func testPopcount() {
        let b = Bitset(size: 100)
        b.set(0)
        b.set(10)
        b.set(64)
        XCTAssertEqual(b.popcount(), 3)
    }

    func testAllAnyNone() {
        let b = Bitset(size: 3)
        XCTAssertTrue(b.none())
        XCTAssertFalse(b.any())
        XCTAssertFalse(b.all())

        b.set(0)
        XCTAssertFalse(b.none())
        XCTAssertTrue(b.any())
        XCTAssertFalse(b.all())

        b.set(1)
        b.set(2)
        XCTAssertTrue(b.all())
    }

    func testFromInteger() {
        let b = Bitset(fromInteger: 5) // 101 in binary
        XCTAssertEqual(b.size, 3)
        XCTAssertTrue(b.test(0))
        XCTAssertFalse(b.test(1))
        XCTAssertTrue(b.test(2))
        XCTAssertEqual(b.toInteger(), 5)
    }

    func testFromBinaryStr() throws {
        let b = try Bitset(fromBinaryStr: "101")
        XCTAssertEqual(b.size, 3)
        XCTAssertTrue(b.test(0))
        XCTAssertFalse(b.test(1))
        XCTAssertTrue(b.test(2))
        XCTAssertEqual(b.toBinaryStr(), "101")
    }

    func testBitwiseAnd() {
        let b1 = Bitset(fromInteger: 5) // 101
        let b2 = Bitset(fromInteger: 3) // 011
        let b3 = b1.and(b2)
        XCTAssertEqual(b3.toInteger(), 1) // 001
        XCTAssertEqual(b3.size, 3)
    }

    func testBitwiseOr() {
        let b1 = Bitset(fromInteger: 5) // 101
        let b2 = Bitset(fromInteger: 3) // 011
        let b3 = b1.or(b2)
        XCTAssertEqual(b3.toInteger(), 7) // 111
    }

    func testBitwiseXor() {
        let b1 = Bitset(fromInteger: 5) // 101
        let b2 = Bitset(fromInteger: 3) // 011
        let b3 = b1.xor(b2)
        XCTAssertEqual(b3.toInteger(), 6) // 110
    }

    func testBitwiseNot() {
        let b = Bitset(fromInteger: 5) // 101
        let b2 = b.not()
        XCTAssertEqual(b2.toInteger(), 2) // 010 (size is 3)
        XCTAssertEqual(b2.size, 3)
    }

    func testAndNot() {
        let b1 = Bitset(fromInteger: 5) // 101
        let b2 = Bitset(fromInteger: 3) // 011
        let b3 = b1.andNot(b2)
        XCTAssertEqual(b3.toInteger(), 4) // 100
    }

    func testEquality() {
        let b1 = Bitset(fromInteger: 5)
        let b2 = Bitset(fromInteger: 5)
        XCTAssertEqual(b1, b2)
        
        let b3 = Bitset(size: 3)
        b3.set(0)
        b3.set(2)
        XCTAssertEqual(b1, b3)
    }
}
