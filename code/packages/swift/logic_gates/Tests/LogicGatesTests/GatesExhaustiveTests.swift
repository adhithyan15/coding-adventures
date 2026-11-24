// GatesExhaustiveTests.swift
// Part of coding-adventures — an educational computing stack.
//
// ============================================================================
// Exhaustive validation tests for the eleven basic gates (LG02 sections 7, 9)
// ============================================================================
//
// Contract under test:
//   1. For every valid input combination the result is exactly 0 or 1 and
//      matches a literal truth table written out below (not recomputed).
//   2. Every gate validates ITS OWN arguments, left to right, before computing
//      or delegating. An invalid argument raises exactly
//          LogicGateError.invalidBit(name: <that gate's parameter>, got: <value>)
//      where the name is the first invalid parameter ("a" then "b"). The
//      NAND-built gates (nandNot, nandAnd, nandOr, nandXor) must never leak
//      the parameter names of an inner nandGate call.
//
// A note on Swift's type system (the "type confusion" lane): Swift has no
// implicit numeric conversion, so `andGate(true, 0)`, `andGate(1.0, 0)`,
// `andGate(Int8(1), 0)` or `andGate(UInt(1), 0)` do not compile — the
// parameters are `Int`. A bare integer literal such as `1` is accepted because
// the literal adapts to Int; a floating literal `1.0` is rejected at compile
// time. Callers holding other integer widths must convert explicitly with
// `Int(x)`, after which an out-of-range value is rejected at runtime by the
// checks exercised here. Hence no runtime test for Bool/Double is possible or
// necessary: the compiler is the validator for those.

import XCTest
@testable import LogicGates

/// Asserts that `expr` throws exactly `.invalidBit(name:got:)` with the
/// given payload (pattern-matching the case and both fields).
func assertInvalidBit(
    _ expr: @autoclosure () throws -> Int,
    name: String, got: Int,
    file: StaticString = #filePath, line: UInt = #line
) {
    XCTAssertThrowsError(try expr(), file: file, line: line) { e in
        guard case LogicGateError.invalidBit(let n, let g) = e else {
            return XCTFail("wrong error: \(e)", file: file, line: line)
        }
        XCTAssertEqual(n, name, file: file, line: line)
        XCTAssertEqual(g, got, file: file, line: line)
    }
}

final class GatesExhaustiveTests: XCTestCase {

    typealias Binary = (Int, Int) throws -> Int

    /// Literal truth tables, rows ordered (0,0) (0,1) (1,0) (1,1).
    let binaryTables: [(String, Binary, [Int])] = [
        ("andGate", andGate, [0, 0, 0, 1]),
        ("orGate", orGate, [0, 1, 1, 1]),
        ("xorGate", xorGate, [0, 1, 1, 0]),
        ("nandGate", nandGate, [1, 1, 1, 0]),
        ("norGate", norGate, [1, 0, 0, 0]),
        ("xnorGate", xnorGate, [1, 0, 0, 1]),
        ("nandAnd", nandAnd, [0, 0, 0, 1]),
        ("nandOr", nandOr, [0, 1, 1, 1]),
        ("nandXor", nandXor, [0, 1, 1, 0]),
    ]

    /// Every out-of-range value we probe: -1024...1024 minus {0,1}, plus the
    /// extremes of Int and Int32.
    var badValues: [Int] {
        var v = Array(-1024...1024).filter { $0 != 0 && $0 != 1 }
        v += [Int.min, Int.min + 1, Int.max - 1, Int.max,
              Int(Int32.min), Int(Int32.max)]
        return v
    }

    func testUnaryTruthTables() throws {
        XCTAssertEqual(try notGate(0), 1)
        XCTAssertEqual(try notGate(1), 0)
        XCTAssertEqual(try nandNot(0), 1)
        XCTAssertEqual(try nandNot(1), 0)
    }

    func testBinaryTruthTables() throws {
        let inputs = [(0, 0), (0, 1), (1, 0), (1, 1)]
        for (name, f, expected) in binaryTables {
            for (i, (a, b)) in inputs.enumerated() {
                let r = try f(a, b)
                XCTAssertEqual(r, expected[i], "\(name)(\(a),\(b))")
                XCTAssertTrue(r == 0 || r == 1)
            }
        }
    }

    func testNandBuiltEqualsDirect() throws {
        for a in 0...1 {
            XCTAssertEqual(try nandNot(a), try notGate(a))
            for b in 0...1 {
                XCTAssertEqual(try nandAnd(a, b), try andGate(a, b))
                XCTAssertEqual(try nandOr(a, b), try orGate(a, b))
                XCTAssertEqual(try nandXor(a, b), try xorGate(a, b))
            }
        }
    }

    func testUnaryRejectsEveryBadValue() {
        for v in badValues {
            assertInvalidBit(try notGate(v), name: "a", got: v)
            assertInvalidBit(try nandNot(v), name: "a", got: v)
        }
    }

    func testBinaryRejectsEveryBadValueInEveryPosition() {
        for (_, f, _) in binaryTables {
            for v in badValues {
                // Bad first argument, valid or invalid second: "a" wins.
                assertInvalidBit(try f(v, 0), name: "a", got: v)
                assertInvalidBit(try f(v, 1), name: "a", got: v)
                assertInvalidBit(try f(v, 7), name: "a", got: v)
                // Valid first argument, bad second: "b".
                assertInvalidBit(try f(0, v), name: "b", got: v)
                assertInvalidBit(try f(1, v), name: "b", got: v)
            }
        }
    }

    func testFirstInvalidArgumentReportedWhenBothBad() {
        for (_, f, _) in binaryTables {
            assertInvalidBit(try f(Int.min, Int.max), name: "a", got: Int.min)
            assertInvalidBit(try f(2, -1), name: "a", got: 2)
        }
    }

    func testNoValidInputThrows() {
        for a in 0...1 {
            XCTAssertNoThrow(try notGate(a))
            XCTAssertNoThrow(try nandNot(a))
            for b in 0...1 {
                for (_, f, _) in binaryTables {
                    XCTAssertNoThrow(try f(a, b))
                }
            }
        }
    }
}
