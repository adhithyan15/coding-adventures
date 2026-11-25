// RejectionMatrixTests.swift
// Part of coding-adventures — an educational computing stack.
//
// LGE-SWIFT-HARDEN: the wide-integer rejection matrix and exact-payload
// assertions for every block. Nothing here uses a bare XCTAssertThrowsError:
// every rejection is compared against the exact LogicGateError value.

import XCTest
@testable import LogicGates

/// Asserts `expr` throws exactly `expected` (case and all payload fields).
func assertThrowsExactly<T>(
    _ expr: @autoclosure () throws -> T, _ expected: LogicGateError,
    file: StaticString = #filePath, line: UInt = #line
) {
    XCTAssertThrowsError(try expr(), file: file, line: line) { e in
        XCTAssertEqual(e as? LogicGateError, expected, file: file, line: line)
    }
}

final class RejectionMatrixTests: XCTestCase {

    /// Values that would survive a narrowing conversion (to Int32/UInt32/Float
    /// mantissa) and be mistaken for 0 or 1 in a sloppy implementation, plus
    /// the extremes of Int.
    ///
    /// NOTE (32-bit Int targets): Swift's `Int` is 32 bits on armv7/wasm32.
    /// There, literals above Int32.max do not compile, so the wide values are
    /// built with `truncatingIfNeeded`; some then wrap to 0 or 1 (2^32, 2^32+1)
    /// and are filtered out below because they are legitimately valid bits
    /// there. Every other row compares against the value actually held in the
    /// Int (`got: v`), so the tests stay exact on every width.
    static let wide: [Int] = [
        Int(truncatingIfNeeded: Int64(1) << 31),
        Int(truncatingIfNeeded: (Int64(1) << 31) + 1),
        Int(truncatingIfNeeded: Int64(1) << 32),
        Int(truncatingIfNeeded: (Int64(1) << 32) + 1),
        Int(truncatingIfNeeded: (Int64(1) << 53) + 1),
        Int.max, Int.min, -1,
    ].filter { $0 != 0 && $0 != 1 }

    typealias Binary = (Int, Int) throws -> Int
    static let binary: [(String, Binary)] = [
        ("andGate", andGate), ("orGate", orGate), ("xorGate", xorGate),
        ("nandGate", nandGate), ("norGate", norGate), ("xnorGate", xnorGate),
        ("nandAnd", nandAnd), ("nandOr", nandOr), ("nandXor", nandXor),
    ]

    func testWideRejectionAllGatesBothPositions() {
        XCTAssertGreaterThanOrEqual(Self.wide.count, 5)
        for v in Self.wide {
            assertInvalidBit(try notGate(v), name: "a", got: v)
            assertInvalidBit(try nandNot(v), name: "a", got: v)
            for (_, f) in Self.binary {
                assertInvalidBit(try f(v, 0), name: "a", got: v)
                assertInvalidBit(try f(v, 1), name: "a", got: v)
                assertInvalidBit(try f(0, v), name: "b", got: v)
                assertInvalidBit(try f(1, v), name: "b", got: v)
            }
        }
    }

    /// A decisive first argument (AND/NAND: 0, OR/NOR: 1) must NOT
    /// short-circuit past validation of the second argument.
    func testNoShortCircuitPastBadSecondArgument() {
        for v in Self.wide {
            for (_, f) in Self.binary {
                assertInvalidBit(try f(0, v), name: "b", got: v)
                assertInvalidBit(try f(1, v), name: "b", got: v)
            }
        }
    }

    func testNaryWideRejectionNamesElement() {
        for v in Self.wide {
            assertThrowsExactly(try andN([1, v, 1]), .invalidBit(name: "inputs[1]", got: v))
            assertThrowsExactly(try orN([0, 0, v]), .invalidBit(name: "inputs[2]", got: v))
            // A decisive earlier element still validates the later one.
            assertThrowsExactly(try andN([0, v]), .invalidBit(name: "inputs[1]", got: v))
            assertThrowsExactly(try orN([1, v]), .invalidBit(name: "inputs[1]", got: v))
        }
    }

    func testBlocksWideRejection() {
        for v in Self.wide {
            assertThrowsExactly(try mux2(d0: v, d1: 0, sel: 0), .invalidBit(name: "d0", got: v))
            assertThrowsExactly(try mux2(d0: 0, d1: v, sel: 0), .invalidBit(name: "d1", got: v))
            assertThrowsExactly(try mux2(d0: 0, d1: 0, sel: v), .invalidBit(name: "sel", got: v))
            assertThrowsExactly(try triState(data: v, enable: 1), .invalidBit(name: "data", got: v))
            assertThrowsExactly(try triState(data: 1, enable: v), .invalidBit(name: "enable", got: v))
            assertThrowsExactly(try decoder(inputs: [0, v]), .invalidBit(name: "inputs[1]", got: v))
            assertThrowsExactly(try demux(data: v, sel: [0]), .invalidBit(name: "data", got: v))
            assertThrowsExactly(try dLatch(data: v, enable: 1), .invalidBit(name: "data", got: v))
            assertThrowsExactly(try srLatch(set: 0, reset: v), .invalidBit(name: "reset", got: v))
            assertThrowsExactly(try dFlipFlop(data: 0, clock: v), .invalidBit(name: "clock", got: v))
        }
    }

    // MARK: exact payloads, combinational

    func testMux2ExactPayloads() {
        assertThrowsExactly(try mux2(d0: 2, d1: 0, sel: 0), .invalidBit(name: "d0", got: 2))
        assertThrowsExactly(try mux2(d0: 0, d1: -1, sel: 0), .invalidBit(name: "d1", got: -1))
        assertThrowsExactly(try mux2(d0: 0, d1: 1, sel: 2), .invalidBit(name: "sel", got: 2))
    }

    func testMux4ExactPayloads() {
        assertThrowsExactly(try mux4(d0: 0, d1: 0, d2: 0, d3: 0, sel: [0]),
                            .invalidSelectLength(expected: 2, got: 1))
        assertThrowsExactly(try mux4(d0: 0, d1: 0, d2: 0, d3: 0, sel: [0, 3]),
                            .invalidBit(name: "sel[1]", got: 3))
        assertThrowsExactly(try mux4(d0: 5, d1: 0, d2: 0, d3: 0, sel: [0, 0]),
                            .invalidBit(name: "d0", got: 5))
        assertThrowsExactly(try mux4(d0: 0, d1: 0, d2: 0, d3: -2, sel: [0, 0]),
                            .invalidBit(name: "d3", got: -2))
    }

    func testMux8ExactPayloads() {
        assertThrowsExactly(try mux8(inputs: [0, 1, 0], sel: [0, 0, 0]),
                            .invalidSelectLength(expected: 8, got: 3))
        assertThrowsExactly(try mux8(inputs: Array(repeating: 0, count: 8), sel: [0, 1]),
                            .invalidSelectLength(expected: 3, got: 2))
        var bad = Array(repeating: 0, count: 8)
        bad[5] = 9
        assertThrowsExactly(try mux8(inputs: bad, sel: [0, 0, 0]),
                            .invalidBit(name: "inputs[5]", got: 9))
        assertThrowsExactly(try mux8(inputs: Array(repeating: 0, count: 8), sel: [0, 4, 0]),
                            .invalidBit(name: "sel[1]", got: 4))
    }

    func testMuxNExactPayloads() {
        assertThrowsExactly(try muxN(inputs: [1], sel: []),
                            .insufficientInputs(minimum: 2, got: 1))
        assertThrowsExactly(try muxN(inputs: [0, 1, 0], sel: [0, 0]),
                            .invalidShape("inputs.count must be a power of two, got 3"))
        assertThrowsExactly(try muxN(inputs: [0, 1, 0, 1], sel: [0]),
                            .invalidSelectLength(expected: 2, got: 1))
        assertThrowsExactly(try muxN(inputs: [0, 7], sel: [0]),
                            .invalidBit(name: "inputs[1]", got: 7))
        assertThrowsExactly(try muxN(inputs: [0, 1], sel: [-1]),
                            .invalidBit(name: "sel[0]", got: -1))
    }

    func testDemuxExactPayloads() {
        assertThrowsExactly(try demux(data: 2, sel: [0]), .invalidBit(name: "data", got: 2))
        assertThrowsExactly(try demux(data: 1, sel: [0, 2]), .invalidBit(name: "sel[1]", got: 2))
        assertThrowsExactly(try demux(data: 1, sel: Array(repeating: 0, count: maxSelectBits + 1)),
                            .invalidWidth(name: "sel", value: maxSelectBits + 1))
    }

    func testDecoderExactPayloads() {
        assertThrowsExactly(try decoder(inputs: [0, 2]), .invalidBit(name: "inputs[1]", got: 2))
        assertThrowsExactly(try decoder(inputs: Array(repeating: 0, count: maxSelectBits + 1)),
                            .invalidWidth(name: "inputs", value: maxSelectBits + 1))
    }

    func testEncoderExactPayloads() {
        assertThrowsExactly(try encoder(inputs: [0, 0, 0, 0]),
            .invalidEncoderInput("exactly one input must be 1, got 0 active inputs"))
        assertThrowsExactly(try encoder(inputs: [1, 1, 0, 0]),
            .invalidEncoderInput("exactly one input must be 1, got 2 active inputs"))
        assertThrowsExactly(try encoder(inputs: [1, 2, 0, 0]),
            .invalidBit(name: "inputs[1]", got: 2))
        assertThrowsExactly(try encoder(inputs: [0, 1, 0]),
            .invalidShape("inputs.count must be a power of two, got 3"))
        assertThrowsExactly(try encoder(inputs: []),
            .invalidShape("inputs.count must be a power of two, got 0"))
    }

    func testPriorityEncoderExactPayloads() {
        assertThrowsExactly(try priorityEncoder(inputs: [0, 2, 0, 0]),
            .invalidBit(name: "inputs[1]", got: 2))
        assertThrowsExactly(try priorityEncoder(inputs: [0, 1, 0]),
            .invalidShape("inputs.count must be a power of two, got 3"))
        assertThrowsExactly(try priorityEncoder(inputs: []),
            .invalidShape("inputs.count must be a power of two, got 0"))
    }

    func testTriStateExactPayloads() {
        assertThrowsExactly(try triState(data: 2, enable: 1), .invalidBit(name: "data", got: 2))
        assertThrowsExactly(try triState(data: 0, enable: 3), .invalidBit(name: "enable", got: 3))
    }

    // MARK: exact payloads, sequential

    func testLatchAndFlipFlopExactPayloads() {
        assertThrowsExactly(try srLatch(set: 2, reset: 0), .invalidBit(name: "set", got: 2))
        assertThrowsExactly(try srLatch(set: 0, reset: 2), .invalidBit(name: "reset", got: 2))
        assertThrowsExactly(try srLatch(set: 0, reset: 0, q: 2), .invalidBit(name: "q", got: 2))
        assertThrowsExactly(try srLatch(set: 0, reset: 0, qBar: 2), .invalidBit(name: "qBar", got: 2))
        assertThrowsExactly(try dLatch(data: 2, enable: 1), .invalidBit(name: "data", got: 2))
        assertThrowsExactly(try dLatch(data: 1, enable: 2), .invalidBit(name: "enable", got: 2))
        assertThrowsExactly(try dFlipFlop(data: 2, clock: 0), .invalidBit(name: "data", got: 2))
        assertThrowsExactly(try dFlipFlop(data: 0, clock: 2), .invalidBit(name: "clock", got: 2))
        assertThrowsExactly(try dFlipFlop(data: 0, clock: 0, masterQ: 2),
                            .invalidBit(name: "masterQ", got: 2))
        assertThrowsExactly(try dFlipFlop(data: 0, clock: 0, masterQBar: -1),
                            .invalidBit(name: "masterQBar", got: -1))
    }

    func testRegisterExactPayloads() {
        assertThrowsExactly(try register(data: [1, 2, 0], clock: 1),
                            .invalidBit(name: "data[1]", got: 2))
        assertThrowsExactly(try register(data: [1, 0], clock: 2),
                            .invalidBit(name: "clock", got: 2))
        assertThrowsExactly(try register(data: [1, 0], clock: 1, q: [0]),
                            .invalidShape("q.count 1 != data.count 2"))
        assertThrowsExactly(try register(data: [1, 0], clock: 1, masterQ: [0, 0, 0]),
                            .invalidShape("masterQ.count 3 != data.count 2"))
    }

    func testShiftRegisterExactPayloads() {
        assertThrowsExactly(try shiftRegister(serialIn: 2, clock: 1),
                            .invalidBit(name: "serialIn", got: 2))
        assertThrowsExactly(try shiftRegister(serialIn: 0, clock: 2),
                            .invalidBit(name: "clock", got: 2))
        assertThrowsExactly(try shiftRegister(serialIn: 0, clock: 0, q: [0, 5]),
                            .invalidBit(name: "q[1]", got: 5))
    }

    func testCounterExactPayloads() {
        assertThrowsExactly(try counter(clock: 2), .invalidBit(name: "clock", got: 2))
        assertThrowsExactly(try counter(clock: 1, reset: 3), .invalidBit(name: "reset", got: 3))
        assertThrowsExactly(try counter(clock: 1, q: []), .invalidWidth(name: "q", value: 0))
        assertThrowsExactly(try counter(clock: 1, q: [0, 0, 4, 0]),
                            .invalidBit(name: "q[2]", got: 4))
    }

    func testDescriptionCarriesName() {
        XCTAssertEqual(LogicGateError.invalidBit(name: "inputs[3]", got: 9).description,
                       "inputs[3] must be 0 or 1, got 9")
    }
}
