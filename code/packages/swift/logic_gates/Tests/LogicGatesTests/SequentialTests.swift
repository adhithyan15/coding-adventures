import XCTest
@testable import LogicGates

// ============================================================================
// SequentialTests — Tests for SR latch, D latch, D flip-flop, register,
//                   shift register, and counter
// ============================================================================
//
// Test strategy:
//   1. SR latch: all truth table entries including hold and invalid states
//   2. D latch: transparent mode (enable=1) and opaque mode (enable=0)
//   3. D flip-flop: rising edge capture, hold during CLK=0
//   4. Register: store and retrieve N-bit words, all bits captured simultaneously
//   5. Shift register: shift in a known pattern, verify serial and parallel outputs
//   6. Counter: count up, overflow/wrap, synchronous reset
// ============================================================================

// MARK: - SR Latch Tests

final class SRLatchTests: XCTestCase {

    // S=1, R=0 → Set: Q=1, Q̄=0
    func testSet() throws {
        let s = try srLatch(set: 1, reset: 0)
        XCTAssertEqual(s.q, 1)
        XCTAssertEqual(s.qBar, 0)
    }

    // S=0, R=1 → Reset: Q=0, Q̄=1
    func testReset() throws {
        let s = try srLatch(set: 0, reset: 1)
        XCTAssertEqual(s.q, 0)
        XCTAssertEqual(s.qBar, 1)
    }

    // S=0, R=0 with previous Q=1 → Hold: Q remains 1
    func testHoldOne() throws {
        let s = try srLatch(set: 0, reset: 0, q: 1, qBar: 0)
        XCTAssertEqual(s.q, 1)
        XCTAssertEqual(s.qBar, 0)
    }

    // S=0, R=0 with previous Q=0 → Hold: Q remains 0
    func testHoldZero() throws {
        let s = try srLatch(set: 0, reset: 0, q: 0, qBar: 1)
        XCTAssertEqual(s.q, 0)
        XCTAssertEqual(s.qBar, 1)
    }

    // S=1, R=1 → Invalid: both outputs forced LOW
    func testInvalidState() throws {
        let s = try srLatch(set: 1, reset: 1)
        XCTAssertEqual(s.q, 0)
        XCTAssertEqual(s.qBar, 0)
    }

    // Set then hold: latch retains the set value
    func testSetThenHold() throws {
        let set  = try srLatch(set: 1, reset: 0)
        let hold = try srLatch(set: 0, reset: 0, q: set.q, qBar: set.qBar)
        XCTAssertEqual(hold.q, 1)
    }

    // Reset then hold: latch retains the reset value
    func testResetThenHold() throws {
        let reset = try srLatch(set: 0, reset: 1)
        let hold  = try srLatch(set: 0, reset: 0, q: reset.q, qBar: reset.qBar)
        XCTAssertEqual(hold.q, 0)
    }

    // Input validation
    func testInvalidSetInput() { XCTAssertThrowsError(try srLatch(set: 2, reset: 0)) }
    func testInvalidResetInput() { XCTAssertThrowsError(try srLatch(set: 0, reset: 2)) }
    func testInvalidQInput() { XCTAssertThrowsError(try srLatch(set: 0, reset: 0, q: 2)) }
}

// MARK: - D Latch Tests

final class DLatchTests: XCTestCase {

    // Enable=1, Data=1 → transparent, Q=1
    func testTransparentStoreOne() throws {
        let s = try dLatch(data: 1, enable: 1)
        XCTAssertEqual(s.q, 1)
        XCTAssertEqual(s.qBar, 0)
    }

    // Enable=1, Data=0 → transparent, Q=0
    func testTransparentStoreZero() throws {
        let s = try dLatch(data: 0, enable: 1)
        XCTAssertEqual(s.q, 0)
        XCTAssertEqual(s.qBar, 1)
    }

    // Enable=0, previous Q=1 → opaque, Q stays 1
    func testOpaqueHoldsOne() throws {
        let s = try dLatch(data: 0, enable: 0, q: 1, qBar: 0)
        XCTAssertEqual(s.q, 1)
    }

    // Enable=0, previous Q=0 → opaque, Q stays 0
    func testOpaqueHoldsZero() throws {
        let s = try dLatch(data: 1, enable: 0, q: 0, qBar: 1)
        XCTAssertEqual(s.q, 0)
    }

    // Data doesn't matter when opaque
    func testOpaqueIgnoresData() throws {
        let s1 = try dLatch(data: 0, enable: 0, q: 1, qBar: 0)
        let s2 = try dLatch(data: 1, enable: 0, q: 1, qBar: 0)
        XCTAssertEqual(s1.q, s2.q)
    }

    // Q and Q̄ are always complements in valid states
    func testQAndQBarAreComplements() throws {
        for data in [0, 1] {
            for enable in [0, 1] {
                let s = try dLatch(data: data, enable: enable)
                if enable == 1 {
                    XCTAssertNotEqual(s.q, s.qBar)
                }
            }
        }
    }

    func testInvalidData() { XCTAssertThrowsError(try dLatch(data: 2, enable: 1)) }
    func testInvalidEnable() { XCTAssertThrowsError(try dLatch(data: 1, enable: 2)) }
}

// MARK: - D Flip-Flop Tests

final class DFlipFlopTests: XCTestCase {

    // Clock HIGH (enable slave): Q follows master (captures D from CLK=0 phase)
    func testCaptures1OnHighClock() throws {
        // First, clock LOW so master samples D=1
        let low  = try dFlipFlop(data: 1, clock: 0)
        // Then clock HIGH: slave copies master's output
        let high = try dFlipFlop(data: 1, clock: 1,
            q: low.q, qBar: low.qBar,
            masterQ: low.masterQ, masterQBar: 1 - low.masterQ)
        XCTAssertEqual(high.q, 1)
    }

    func testCaptures0OnHighClock() throws {
        let low  = try dFlipFlop(data: 0, clock: 0)
        let high = try dFlipFlop(data: 0, clock: 1,
            q: low.q, qBar: low.qBar,
            masterQ: low.masterQ, masterQBar: 1 - low.masterQ)
        XCTAssertEqual(high.q, 0)
    }

    // During CLK=0: slave is opaque (Q doesn't change even if D changes)
    func testSlaveOpaqueWhenClockLow() throws {
        // Start with Q=1 (stored)
        let hold = try dFlipFlop(data: 0, clock: 0,
            q: 1, qBar: 0,
            masterQ: 0, masterQBar: 1)
        // Slave is opaque: Q stays 1
        XCTAssertEqual(hold.q, 1)
    }

    // Q and Q̄ are always complementary
    func testQAndQBarComplement() throws {
        let s = try dFlipFlop(data: 1, clock: 1)
        XCTAssertEqual(s.q + s.qBar, 1)
    }

    // MasterQ is captured for diagnostic purposes
    func testMasterQPresent() throws {
        let s = try dFlipFlop(data: 1, clock: 0)
        XCTAssertTrue(s.masterQ == 0 || s.masterQ == 1)
    }

    func testInvalidData() { XCTAssertThrowsError(try dFlipFlop(data: 2, clock: 0)) }
    func testInvalidClock() { XCTAssertThrowsError(try dFlipFlop(data: 0, clock: 2)) }
}

// MARK: - Register Tests

final class RegisterTests: XCTestCase {

    // Load a 4-bit value: 1010
    func testLoad4BitWord() throws {
        let data = [1, 0, 1, 0]
        // Clock high: latch the value
        let (q, _) = try register(data: data, clock: 1)
        XCTAssertEqual(q, data)
    }

    // All zeros
    func testLoadAllZeros() throws {
        let (q, _) = try register(data: [0, 0, 0, 0], clock: 1)
        XCTAssertEqual(q, [0, 0, 0, 0])
    }

    // All ones
    func testLoadAllOnes() throws {
        let (q, _) = try register(data: [1, 1, 1, 1], clock: 1)
        XCTAssertEqual(q, [1, 1, 1, 1])
    }

    // 8-bit register
    func test8BitRegister() throws {
        let data = [1, 0, 1, 1, 0, 0, 1, 0]
        let (q, states) = try register(data: data, clock: 1)
        XCTAssertEqual(q, data)
        XCTAssertEqual(states.count, 8)
    }

    // All bits have corresponding flip-flop states
    func testStatesMatchBitCount() throws {
        let data = [1, 0, 1]
        let (q, states) = try register(data: data, clock: 1)
        XCTAssertEqual(q.count, 3)
        XCTAssertEqual(states.count, 3)
    }

    func testInvalidBitInData() { XCTAssertThrowsError(try register(data: [1, 2, 0], clock: 1)) }
    func testInvalidClock() { XCTAssertThrowsError(try register(data: [1, 0], clock: 2)) }
}

// MARK: - Shift Register Tests

final class ShiftRegisterTests: XCTestCase {

    // Shift in a 1: [0,0,0,0] → [1,0,0,0]
    func testShiftIn1() throws {
        let (q, serialOut, _) = try shiftRegister(serialIn: 1, clock: 1, q: [0, 0, 0, 0])
        XCTAssertEqual(q[0], 1)
        XCTAssertEqual(serialOut, 0)
    }

    // Shift in a 0 to all-ones: [1,1,1,1] → [0,1,1,1], serial out = 1
    func testShiftOutRightmost() throws {
        let (q, serialOut, _) = try shiftRegister(serialIn: 0, clock: 1, q: [1, 1, 1, 1])
        XCTAssertEqual(q[0], 0)
        XCTAssertEqual(serialOut, 1)
    }

    // Three shifts of 1 into zeros:
    // [0,0,0,0] → [1,0,0,0] → [1,1,0,0] → [1,1,1,0]
    func testThreeShifts() throws {
        var q = [0, 0, 0, 0]
        for _ in 0..<3 {
            (q, _, _) = try shiftRegister(serialIn: 1, clock: 1, q: q)
        }
        XCTAssertEqual(q, [1, 1, 1, 0])
    }

    // After 4 shifts of 1: [1,1,1,1], serial out = 0 at each step until the last
    func testFourShiftsAllOnes() throws {
        var q = [0, 0, 0, 0]
        for _ in 0..<4 {
            (q, _, _) = try shiftRegister(serialIn: 1, clock: 1, q: q)
        }
        XCTAssertEqual(q, [1, 1, 1, 1])
    }

    // Serial output on the 5th shift is 1 (the first 1 we shifted in)
    func testSerialOutAfterFourShifts() throws {
        var q = [0, 0, 0, 0]
        var serialOut = 0
        for _ in 0..<4 {
            (q, _, _) = try shiftRegister(serialIn: 1, clock: 1, q: q)
        }
        (_, serialOut, _) = try shiftRegister(serialIn: 0, clock: 1, q: q)
        XCTAssertEqual(serialOut, 1)
    }

    func testInvalidSerialIn() { XCTAssertThrowsError(try shiftRegister(serialIn: 2, clock: 1)) }
    func testInvalidClock() { XCTAssertThrowsError(try shiftRegister(serialIn: 0, clock: 2)) }
}

// MARK: - Counter Tests

final class CounterTests: XCTestCase {

    // Increment from 0: [0,0,0,0] → [0,0,0,1]
    func testIncrementFromZero() throws {
        let (q, overflow, _) = try counter(clock: 1, q: [0, 0, 0, 0])
        XCTAssertEqual(q, [0, 0, 0, 1])
        XCTAssertEqual(overflow, 0)
    }

    // Count sequence: 0 → 1 → 2 → 3
    func testCountSequence() throws {
        var q = [0, 0, 0, 0]
        var expected = [[0,0,0,1], [0,0,1,0], [0,0,1,1]]
        for exp in expected {
            (q, _, _) = try counter(clock: 1, q: q)
            XCTAssertEqual(q, exp)
        }
    }

    // Overflow: [1,1,1,1] + 1 wraps to [0,0,0,0] with overflow=1
    func testOverflow() throws {
        let (q, overflow, _) = try counter(clock: 1, q: [1, 1, 1, 1])
        XCTAssertEqual(q, [0, 0, 0, 0])
        XCTAssertEqual(overflow, 1)
    }

    // No overflow on non-max value
    func testNoOverflowMidCount() throws {
        let (_, overflow, _) = try counter(clock: 1, q: [0, 1, 0, 1])
        XCTAssertEqual(overflow, 0)
    }

    // Synchronous reset: regardless of current count, reset=1 → [0,0,0,0]
    func testSynchronousReset() throws {
        let (q, overflow, _) = try counter(clock: 1, reset: 1, q: [1, 0, 1, 0])
        XCTAssertEqual(q, [0, 0, 0, 0])
        XCTAssertEqual(overflow, 0)
    }

    // States array has correct length
    func testStatesCount() throws {
        let (_, _, states) = try counter(clock: 1, q: [0, 0, 0, 0])
        XCTAssertEqual(states.count, 4)
    }

    func testInvalidClock() { XCTAssertThrowsError(try counter(clock: 2)) }
    func testInvalidReset() { XCTAssertThrowsError(try counter(clock: 1, reset: 3)) }
}
