// ============================================================================
// ArithmeticTests.swift
// ============================================================================

import XCTest
@testable import Arithmetic

final class ArithmeticTests: XCTestCase {
    func testHalfAdder() throws {
        let (s0, c0) = try halfAdder(a: 0, b: 0)
        XCTAssertEqual(s0, 0); XCTAssertEqual(c0, 0)
        
        let (s1, c1) = try halfAdder(a: 0, b: 1)
        XCTAssertEqual(s1, 1); XCTAssertEqual(c1, 0)
        
        let (s2, c2) = try halfAdder(a: 1, b: 0)
        XCTAssertEqual(s2, 1); XCTAssertEqual(c2, 0)
        
        let (s3, c3) = try halfAdder(a: 1, b: 1)
        XCTAssertEqual(s3, 0); XCTAssertEqual(c3, 1)
    }
    
    func testFullAdder() throws {
        // [A, B, Cin, Sum, Cout]
        let truthTable = [
            [0, 0, 0, 0, 0],
            [0, 0, 1, 1, 0],
            [0, 1, 0, 1, 0],
            [0, 1, 1, 0, 1],
            [1, 0, 0, 1, 0],
            [1, 0, 1, 0, 1],
            [1, 1, 0, 0, 1],
            [1, 1, 1, 1, 1]
        ]
        
        for row in truthTable {
            let res = try fullAdder(a: row[0], b: row[1], carryIn: row[2])
            XCTAssertEqual(res.sum, row[3])
            XCTAssertEqual(res.carryOut, row[4])
        }
    }
    
    func testRippleCarryAdder() throws {
        // 5 + 3 = 8
        // 5 = 101 (LSB first: [1, 0, 1])
        // 3 = 011 (LSB: [1, 1, 0])
        let a = [1, 0, 1]
        let b = [1, 1, 0]
        let res = try rippleCarryAdder(a: a, b: b)
        // 8 = 1000 => LSB: [0, 0, 0], carryOut = 1
        XCTAssertEqual(res.sum, [0, 0, 0])
        XCTAssertEqual(res.carryOut, 1)
    }
    
    func testALUAddSub() throws {
        let alu = ALU(bitWidth: 4)
        
        // 1 + 2 = 3
        let r1 = try alu.execute(op: .add, a: [1,0,0,0], b: [0,1,0,0])
        XCTAssertEqual(r1.value, [1,1,0,0])
        XCTAssertFalse(r1.zero)
        XCTAssertFalse(r1.negative)
        XCTAssertFalse(r1.overflow)
        XCTAssertFalse(r1.carry)
        
        // 3 - 2 = 1
        let r2 = try alu.execute(op: .sub, a: [1,1,0,0], b: [0,1,0,0])
        XCTAssertEqual(r2.value, [1,0,0,0])
        XCTAssertFalse(r2.zero)
        
        // 3 - 3 = 0
        let r3 = try alu.execute(op: .sub, a: [1,1,0,0], b: [1,1,0,0])
        XCTAssertEqual(r3.value, [0,0,0,0])
        XCTAssertTrue(r3.zero)
    }

    func testALUBitwise() throws {
        let alu = ALU(bitWidth: 3)
        // 5 = 101 => [1, 0, 1]
        // 3 = 011 => [1, 1, 0]
        let a = [1, 0, 1]
        let b = [1, 1, 0]
        
        let andRes = try alu.execute(op: .and, a: a, b: b)
        XCTAssertEqual(andRes.value, [1, 0, 0])
        
        let orRes = try alu.execute(op: .or, a: a, b: b)
        XCTAssertEqual(orRes.value, [1, 1, 1])
        
        let xorRes = try alu.execute(op: .xor, a: a, b: b)
        XCTAssertEqual(xorRes.value, [0, 1, 1])
        
        let notRes = try alu.execute(op: .not, a: a, b: b)
        XCTAssertEqual(notRes.value, [0, 1, 0])
    }
}
