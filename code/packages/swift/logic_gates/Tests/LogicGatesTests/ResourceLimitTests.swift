// ResourceLimitTests.swift
//
// Swift runtime traps cannot be caught, so oversized widths must be rejected
// with a thrown error BEFORE any shift, allocation or index arithmetic. A
// regression here would crash the whole test process rather than fail a test.

import XCTest
@testable import LogicGates

final class ResourceLimitTests: XCTestCase {

    func assertWidth<T>(_ expr: @autoclosure () throws -> T, name: String, value: Int,
                        file: StaticString = #filePath, line: UInt = #line) {
        XCTAssertThrowsError(try expr(), file: file, line: line) { e in
            XCTAssertEqual(e as? LogicGateError,
                           .invalidWidth(name: name, value: value), file: file, line: line)
        }
    }

    func assertShape<T>(_ expr: @autoclosure () throws -> T,
                        file: StaticString = #filePath, line: UInt = #line) {
        XCTAssertThrowsError(try expr(), file: file, line: line) { e in
            guard case LogicGateError.invalidShape = e else {
                return XCTFail("wrong error: \(e)", file: file, line: line)
            }
        }
    }

    func testConstants() {
        XCTAssertEqual(maxSelectBits, 16)
        XCTAssertEqual(maxWidth, 65536)
        XCTAssertEqual(maxCounterWidth, 64)
    }

    func testDecoderBoundary() throws {
        let out = try decoder(inputs: Array(repeating: 1, count: 16))
        XCTAssertEqual(out.count, 65536)
        XCTAssertEqual(out.last, 1)
        XCTAssertEqual(out.reduce(0, +), 1)
        for n in [17, 62, 63, 64, 65, 1000] {
            assertWidth(try decoder(inputs: Array(repeating: 1, count: n)),
                        name: "inputs", value: n)
        }
    }

    func testDemuxBoundary() throws {
        let out = try demux(data: 1, sel: Array(repeating: 1, count: 16))
        XCTAssertEqual(out.count, 65536)
        XCTAssertEqual(out.last, 1)
        for n in [17, 62, 63, 64, 65] {
            assertWidth(try demux(data: 1, sel: Array(repeating: 0, count: n)),
                        name: "sel", value: n)
        }
    }

    func testMuxNBoundary() throws {
        var inputs = Array(repeating: 0, count: 65536)
        inputs[65535] = 1
        XCTAssertEqual(try muxN(inputs: inputs, sel: Array(repeating: 1, count: 16)), 1)
        for n in [17, 62, 63, 64] {
            assertWidth(try muxN(inputs: [0, 1], sel: Array(repeating: 0, count: n)),
                        name: "sel", value: n)
        }
        assertWidth(try muxN(inputs: Array(repeating: 0, count: 65537), sel: [0]),
                    name: "inputs", value: 65537)
    }

    func testMuxNShape() {
        assertShape(try muxN(inputs: [0, 1, 0], sel: [0, 0]))
        assertShape(try muxN(inputs: [0, 1, 0, 1, 0, 1], sel: [0, 0, 0]))
        // Excess select bits are no longer silently ignored.
        XCTAssertThrowsError(try muxN(inputs: [0, 1], sel: [0, 1])) { e in
            XCTAssertEqual(e as? LogicGateError, .invalidSelectLength(expected: 1, got: 2))
        }
        XCTAssertThrowsError(try muxN(inputs: [0, 1, 0, 1], sel: [0])) { e in
            XCTAssertEqual(e as? LogicGateError, .invalidSelectLength(expected: 2, got: 1))
        }
        XCTAssertThrowsError(try muxN(inputs: [1], sel: [])) { e in
            XCTAssertEqual(e as? LogicGateError, .insufficientInputs(minimum: 2, got: 1))
        }
    }

    func testEncodersRequirePowerOfTwo() throws {
        for n in [0, 3, 5, 6, 7, 9] {
            assertShape(try encoder(inputs: Array(repeating: 0, count: n)))
            assertShape(try priorityEncoder(inputs: Array(repeating: 0, count: n)))
        }
        assertWidth(try encoder(inputs: Array(repeating: 0, count: 65537)),
                    name: "inputs", value: 65537)
        assertWidth(try priorityEncoder(inputs: Array(repeating: 0, count: 131072)),
                    name: "inputs", value: 131072)
        var big = Array(repeating: 0, count: 65536)
        big[65535] = 1
        XCTAssertEqual(try encoder(inputs: big), Array(repeating: 1, count: 16))
        XCTAssertEqual(try priorityEncoder(inputs: big).output, Array(repeating: 1, count: 16))
    }

    func testRegisterRejectsMismatchedState() throws {
        assertShape(try register(data: [1, 0, 1], clock: 1, q: [0, 0]))
        assertShape(try register(data: [1, 0], clock: 1, q: [0, 0, 0]))
        assertShape(try register(data: [1, 0], clock: 1, masterQ: [0]))
        assertShape(try register(data: [1, 0], clock: 1, q: [], masterQ: nil))
    }

    func testRegisterValidatesStateBits() {
        XCTAssertThrowsError(try register(data: [1, 0], clock: 1, q: [0, 2])) { e in
            XCTAssertEqual(e as? LogicGateError, .invalidBit(name: "q[1]", got: 2))
        }
        XCTAssertThrowsError(try register(data: [1, 0], clock: 1, masterQ: [-1, 0])) { e in
            XCTAssertEqual(e as? LogicGateError, .invalidBit(name: "masterQ[0]", got: -1))
        }
    }

    func testRegisterWidthCapAndMatchingState() throws {
        assertWidth(try register(data: Array(repeating: 0, count: 65537), clock: 0),
                    name: "data", value: 65537)
        let r = try register(data: [1, 0], clock: 1, q: [0, 0], masterQ: [0, 0])
        XCTAssertEqual(r.q, [1, 0])
    }

    func testCounterWidthLimits() throws {
        assertWidth(try counter(clock: 1, q: []), name: "q", value: 0)
        assertWidth(try counter(clock: 1, q: Array(repeating: 0, count: 65)),
                    name: "q", value: 65)
        let r = try counter(clock: 1, q: Array(repeating: 1, count: 64))
        XCTAssertEqual(r.q, Array(repeating: 0, count: 64))
        XCTAssertEqual(r.overflow, 1)
    }

    // ---- LGE-SWIFT-CAPS: n-ary input cap and shift-register width ----

    func testMaxInputsConstant() {
        XCTAssertEqual(maxInputs, 65536)
    }

    func testAndNOrNBoundary() throws {
        let ones = Array(repeating: 1, count: maxInputs)
        XCTAssertEqual(try andN(ones), 1)
        XCTAssertEqual(try orN(ones), 1)
        let zeros = Array(repeating: 0, count: maxInputs)
        XCTAssertEqual(try andN(zeros), 0)
        XCTAssertEqual(try orN(zeros), 0)
        let over = Array(repeating: 1, count: maxInputs + 1)
        assertWidth(try andN(over), name: "inputs", value: maxInputs + 1)
        assertWidth(try orN(over), name: "inputs", value: maxInputs + 1)
    }

    func testNAryCapPrecedesBitValidation() {
        // An oversized array containing an invalid bit reports the size error,
        // proving the cap is checked before the per-element loop.
        var bad = Array(repeating: 1, count: maxInputs + 1)
        bad[0] = Int.max
        assertWidth(try andN(bad), name: "inputs", value: maxInputs + 1)
        assertWidth(try orN(bad), name: "inputs", value: maxInputs + 1)
        // At the cap, invalid bits still report invalidBit.
        var atCap = Array(repeating: 1, count: maxInputs)
        atCap[7] = 2
        XCTAssertThrowsError(try andN(atCap)) { e in
            XCTAssertEqual(e as? LogicGateError, .invalidBit(name: "inputs[7]", got: 2))
        }
    }

    func testShiftRegisterWidthBoundary() throws {
        let atCap = Array(repeating: 0, count: maxWidth)
        let r = try shiftRegister(serialIn: 1, clock: 1, q: atCap)
        XCTAssertEqual(r.q.count, maxWidth)
        XCTAssertEqual(r.q[0], 1)
        assertWidth(try shiftRegister(serialIn: 1, clock: 1,
                                      q: Array(repeating: 0, count: maxWidth + 1)),
                    name: "q", value: maxWidth + 1)
    }

    func testBoundaryCapsAtMaxValuesDoNotCrash() {
        // Int.max / negative / huge values never reach a shift or allocation.
        assertWidth(try decoder(inputs: Array(repeating: 1, count: maxSelectBits + 1)),
                    name: "inputs", value: maxSelectBits + 1)
        XCTAssertThrowsError(try decoder(inputs: [Int.max])) { e in
            XCTAssertEqual(e as? LogicGateError, .invalidBit(name: "inputs[0]", got: Int.max))
        }
        XCTAssertThrowsError(try demux(data: 1, sel: [Int.min])) { e in
            XCTAssertEqual(e as? LogicGateError, .invalidBit(name: "sel[0]", got: Int.min))
        }
        XCTAssertThrowsError(try counter(clock: 1, q: [-1])) { e in
            XCTAssertEqual(e as? LogicGateError, .invalidBit(name: "q[0]", got: -1))
        }
        XCTAssertThrowsError(try shiftRegister(serialIn: Int.max, clock: 0, q: [0])) { e in
            XCTAssertEqual(e as? LogicGateError, .invalidBit(name: "serialIn", got: Int.max))
        }
        assertWidth(try counter(clock: 1, q: Array(repeating: 0, count: maxCounterWidth + 1)),
                    name: "q", value: maxCounterWidth + 1)
        assertWidth(try counter(clock: 1, q: []), name: "q", value: 0)
    }

    func testShiftRegisterSizeWinsOverBadBit() {
        var q = Array(repeating: 0, count: maxWidth + 1)
        q[0] = 2
        assertWidth(try shiftRegister(serialIn: 0, clock: 0, q: q),
                    name: "q", value: maxWidth + 1)
    }

    func testShiftRegisterEmptyState() throws {
        // Documented behaviour: no stages, nothing shifts, serialOut is 0.
        let r = try shiftRegister(serialIn: 0, clock: 0, q: [])
        XCTAssertEqual(r.q, [])
        XCTAssertEqual(r.serialOut, 0)
        XCTAssertEqual(r.states.count, 0)
    }
}
