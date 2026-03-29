import XCTest
@testable import LogicGates

// ============================================================================
// CombinationalTests — Tests for MUX, DEMUX, decoder, encoder,
//                      priority encoder, and tri-state buffer
// ============================================================================
//
// Test strategy:
//   1. MUX2: exhaustive truth table (all 8 input combinations)
//   2. MUX4: each select combination routes the correct input
//   3. MUX8/MUX_N: select routing for all positions
//   4. DEMUX: data on correct output, all others 0
//   5. Decoder: one-hot output for all input combinations
//   6. Encoder: correct binary output for valid one-hot inputs
//   7. Priority encoder: highest-priority input wins, valid flag correct
//   8. Tri-state: output matches data when enabled, nil when disabled
//   9. Edge cases and input validation
// ============================================================================

// MARK: - MUX2 Tests

final class MUX2Tests: XCTestCase {

    // sel=0: output = d0
    func testSel0Picks_d0_0() throws { XCTAssertEqual(try mux2(d0: 0, d1: 0, sel: 0), 0) }
    func testSel0Picks_d0_1() throws { XCTAssertEqual(try mux2(d0: 1, d1: 0, sel: 0), 1) }
    func testSel0Ignores_d1_0() throws { XCTAssertEqual(try mux2(d0: 0, d1: 1, sel: 0), 0) }
    func testSel0Ignores_d1_1() throws { XCTAssertEqual(try mux2(d0: 1, d1: 1, sel: 0), 1) }

    // sel=1: output = d1
    func testSel1Picks_d1_0() throws { XCTAssertEqual(try mux2(d0: 0, d1: 0, sel: 1), 0) }
    func testSel1Picks_d1_1() throws { XCTAssertEqual(try mux2(d0: 0, d1: 1, sel: 1), 1) }
    func testSel1Ignores_d0_0() throws { XCTAssertEqual(try mux2(d0: 1, d1: 0, sel: 1), 0) }
    func testSel1Ignores_d0_1() throws { XCTAssertEqual(try mux2(d0: 1, d1: 1, sel: 1), 1) }

    func testInvalidSel() { XCTAssertThrowsError(try mux2(d0: 0, d1: 1, sel: 2)) }
    func testInvalidD0() { XCTAssertThrowsError(try mux2(d0: 2, d1: 0, sel: 0)) }
    func testInvalidD1() { XCTAssertThrowsError(try mux2(d0: 0, d1: -1, sel: 0)) }
}

// MARK: - MUX4 Tests

final class MUX4Tests: XCTestCase {

    // Each select combination picks the correct input
    func testSelect00() throws { XCTAssertEqual(try mux4(d0: 1, d1: 0, d2: 0, d3: 0, sel: [0, 0]), 1) }
    func testSelect10() throws { XCTAssertEqual(try mux4(d0: 0, d1: 1, d2: 0, d3: 0, sel: [1, 0]), 1) }
    func testSelect01() throws { XCTAssertEqual(try mux4(d0: 0, d1: 0, d2: 1, d3: 0, sel: [0, 1]), 1) }
    func testSelect11() throws { XCTAssertEqual(try mux4(d0: 0, d1: 0, d2: 0, d3: 1, sel: [1, 1]), 1) }

    // Ignored inputs don't bleed through
    func testOthersAreIgnored00() throws {
        XCTAssertEqual(try mux4(d0: 0, d1: 1, d2: 1, d3: 1, sel: [0, 0]), 0)
    }

    func testInvalidSelectLength() {
        XCTAssertThrowsError(try mux4(d0: 0, d1: 0, d2: 0, d3: 0, sel: [0])) { e in
            guard case LogicGateError.invalidSelectLength = e else {
                return XCTFail("wrong error: \(e)")
            }
        }
    }
}

// MARK: - MUX8 Tests

final class MUX8Tests: XCTestCase {

    func testEachInputReachesOutput() throws {
        for i in 0..<8 {
            var inputs = Array(repeating: 0, count: 8)
            inputs[i] = 1
            let sel = [
                (i >> 0) & 1,
                (i >> 1) & 1,
                (i >> 2) & 1
            ]
            XCTAssertEqual(try mux8(inputs: inputs, sel: sel), 1,
                "input[\(i)] should be selected by sel=\(sel)")
        }
    }

    func testInvalidInputCount() {
        XCTAssertThrowsError(try mux8(inputs: [0, 1, 0], sel: [0, 0, 0]))
    }

    func testInvalidSelectCount() {
        XCTAssertThrowsError(try mux8(inputs: Array(repeating: 0, count: 8), sel: [0, 1]))
    }
}

// MARK: - MUX_N Tests

final class MUXNTests: XCTestCase {

    // 2-input: same as mux2
    func testMuxN2() throws {
        XCTAssertEqual(try muxN(inputs: [0, 1], sel: [1]), 1)
        XCTAssertEqual(try muxN(inputs: [1, 0], sel: [0]), 1)
    }

    // 4-input: same as mux4
    func testMuxN4EachSelect() throws {
        for i in 0..<4 {
            var inputs = Array(repeating: 0, count: 4)
            inputs[i] = 1
            let sel = [(i >> 0) & 1, (i >> 1) & 1]
            XCTAssertEqual(try muxN(inputs: inputs, sel: sel), 1,
                "muxN 4-input: input[\(i)] should win with sel=\(sel)")
        }
    }

    // Single input is an error
    func testMuxNSingleInput() {
        XCTAssertThrowsError(try muxN(inputs: [1], sel: []))
    }
}

// MARK: - DEMUX Tests

final class DEMUXTests: XCTestCase {

    // 1-to-2 DEMUX (1 select bit)
    func test1to2_sel0() throws {
        let out = try demux(data: 1, sel: [0])
        XCTAssertEqual(out[0], 1)
        XCTAssertEqual(out[1], 0)
    }

    func test1to2_sel1() throws {
        let out = try demux(data: 1, sel: [1])
        XCTAssertEqual(out[0], 0)
        XCTAssertEqual(out[1], 1)
    }

    // 1-to-4 DEMUX (2 select bits): each output gets data exactly once
    func test1to4_allSelects() throws {
        for i in 0..<4 {
            let sel = [(i >> 0) & 1, (i >> 1) & 1]
            let out = try demux(data: 1, sel: sel)
            XCTAssertEqual(out.count, 4)
            XCTAssertEqual(out[i], 1, "sel=\(sel) should route to output[\(i)]")
            // All other outputs must be 0
            for j in 0..<4 where j != i {
                XCTAssertEqual(out[j], 0)
            }
        }
    }

    // data=0 stays 0 on all outputs regardless of select
    func testDataZeroGivesAllZeros() throws {
        let out = try demux(data: 0, sel: [1, 0])
        XCTAssertEqual(out, [0, 0, 0, 0])
    }

    func testInvalidData() { XCTAssertThrowsError(try demux(data: 2, sel: [0])) }
    func testInvalidSel() { XCTAssertThrowsError(try demux(data: 1, sel: [0, 2])) }
}

// MARK: - Decoder Tests

final class DecoderTests: XCTestCase {

    // 1-to-2 decoder (1 input bit)
    func testDecoder1bit0() throws {
        let out = try decoder(inputs: [0])
        XCTAssertEqual(out, [1, 0])
    }

    func testDecoder1bit1() throws {
        let out = try decoder(inputs: [1])
        XCTAssertEqual(out, [0, 1])
    }

    // 2-to-4 decoder: all 4 input combinations
    func testDecoder2bit_00() throws {
        XCTAssertEqual(try decoder(inputs: [0, 0]), [1, 0, 0, 0])
    }

    func testDecoder2bit_10() throws {
        // inputs[0]=1 means bit0=1 → decimal 1
        XCTAssertEqual(try decoder(inputs: [1, 0]), [0, 1, 0, 0])
    }

    func testDecoder2bit_01() throws {
        // inputs[0]=0, inputs[1]=1 → decimal 2
        XCTAssertEqual(try decoder(inputs: [0, 1]), [0, 0, 1, 0])
    }

    func testDecoder2bit_11() throws {
        // inputs = [1,1] → decimal 3
        XCTAssertEqual(try decoder(inputs: [1, 1]), [0, 0, 0, 1])
    }

    // 3-to-8 decoder: always exactly one output is 1
    func testDecoder3bitIsOneHot() throws {
        for i in 0..<8 {
            let inputs = [(i >> 0) & 1, (i >> 1) & 1, (i >> 2) & 1]
            let out = try decoder(inputs: inputs)
            XCTAssertEqual(out.count, 8)
            XCTAssertEqual(out.reduce(0, +), 1, "decoder(\(inputs)) should be one-hot")
            XCTAssertEqual(out[i], 1, "decoder(\(inputs)) should activate output[\(i)]")
        }
    }

    // All-zero input activates output 0
    func testDecoderAllZeroInput() throws {
        let out = try decoder(inputs: [0, 0, 0])
        XCTAssertEqual(out[0], 1)
        XCTAssertEqual(out.filter { $0 == 1 }.count, 1)
    }

    func testInvalidInput() { XCTAssertThrowsError(try decoder(inputs: [0, 2])) }
}

// MARK: - Encoder Tests

final class EncoderTests: XCTestCase {

    // 4-to-2 encoder: each valid one-hot input
    func testEncode_I0() throws {
        // Active input 0 → binary [0, 0]
        XCTAssertEqual(try encoder(inputs: [1, 0, 0, 0]), [0, 0])
    }

    func testEncode_I1() throws {
        // Active input 1 → binary [1, 0] (bit0=1)
        XCTAssertEqual(try encoder(inputs: [0, 1, 0, 0]), [1, 0])
    }

    func testEncode_I2() throws {
        // Active input 2 → binary [0, 1] (bit1=1)
        XCTAssertEqual(try encoder(inputs: [0, 0, 1, 0]), [0, 1])
    }

    func testEncode_I3() throws {
        // Active input 3 → binary [1, 1]
        XCTAssertEqual(try encoder(inputs: [0, 0, 0, 1]), [1, 1])
    }

    // 8-to-3 encoder
    func testEncode8to3() throws {
        for i in 0..<8 {
            var inputs = Array(repeating: 0, count: 8)
            inputs[i] = 1
            let out = try encoder(inputs: inputs)
            XCTAssertEqual(out.count, 3)
            // Reconstruct integer from LSB-first binary
            let reconstructed = out.enumerated().reduce(0) { $0 + $1.element * (1 << $1.offset) }
            XCTAssertEqual(reconstructed, i, "encoder(input[\(i)]) should produce \(i)")
        }
    }

    // Zero inputs → invalid (no active input)
    func testZeroInputsIsInvalid() {
        XCTAssertThrowsError(try encoder(inputs: [0, 0, 0, 0])) { e in
            guard case LogicGateError.invalidEncoderInput = e else {
                return XCTFail("wrong error: \(e)")
            }
        }
    }

    // Multiple active inputs → invalid
    func testMultipleActiveIsInvalid() {
        XCTAssertThrowsError(try encoder(inputs: [1, 1, 0, 0]))
    }

    func testInvalidBitValue() {
        XCTAssertThrowsError(try encoder(inputs: [1, 2, 0, 0]))
    }
}

// MARK: - Priority Encoder Tests

final class PriorityEncoderTests: XCTestCase {

    // No active inputs → valid=0
    func testNoActiveInput() throws {
        let (out, valid) = try priorityEncoder(inputs: [0, 0, 0, 0])
        XCTAssertEqual(valid, 0)
        XCTAssertEqual(out, [0, 0])
    }

    // Single active input: correct index
    func testSingleActiveI0() throws {
        let (out, valid) = try priorityEncoder(inputs: [1, 0, 0, 0])
        XCTAssertEqual(valid, 1)
        // index 0 = [0, 0]
        let idx = out.enumerated().reduce(0) { $0 + $1.element * (1 << $1.offset) }
        XCTAssertEqual(idx, 0)
    }

    func testSingleActiveI3() throws {
        let (out, valid) = try priorityEncoder(inputs: [0, 0, 0, 1])
        XCTAssertEqual(valid, 1)
        let idx = out.enumerated().reduce(0) { $0 + $1.element * (1 << $1.offset) }
        XCTAssertEqual(idx, 3)
    }

    // Multiple active: highest-index wins
    func testHighestWins_I3_over_I0() throws {
        let (out, valid) = try priorityEncoder(inputs: [1, 0, 0, 1])
        XCTAssertEqual(valid, 1)
        let idx = out.enumerated().reduce(0) { $0 + $1.element * (1 << $1.offset) }
        XCTAssertEqual(idx, 3)
    }

    func testHighestWins_I2_over_I0_I1() throws {
        let (out, valid) = try priorityEncoder(inputs: [1, 1, 1, 0])
        XCTAssertEqual(valid, 1)
        let idx = out.enumerated().reduce(0) { $0 + $1.element * (1 << $1.offset) }
        XCTAssertEqual(idx, 2)
    }

    // All active: highest (I3) wins
    func testAllActive() throws {
        let (out, valid) = try priorityEncoder(inputs: [1, 1, 1, 1])
        XCTAssertEqual(valid, 1)
        let idx = out.enumerated().reduce(0) { $0 + $1.element * (1 << $1.offset) }
        XCTAssertEqual(idx, 3)
    }

    func testInvalidInput() { XCTAssertThrowsError(try priorityEncoder(inputs: [0, 2, 0, 0])) }
}

// MARK: - Tri-State Buffer Tests

final class TriStateBufferTests: XCTestCase {

    // enable=1: output equals data
    func testEnabled0() throws { XCTAssertEqual(try triState(data: 0, enable: 1), 0) }
    func testEnabled1() throws { XCTAssertEqual(try triState(data: 1, enable: 1), 1) }

    // enable=0: output is nil (high-Z)
    func testDisabledData0() throws { XCTAssertNil(try triState(data: 0, enable: 0)) }
    func testDisabledData1() throws { XCTAssertNil(try triState(data: 1, enable: 0)) }

    // Data doesn't matter when disabled
    func testBothDisabledAreNil() throws {
        XCTAssertNil(try triState(data: 0, enable: 0))
        XCTAssertNil(try triState(data: 1, enable: 0))
    }

    func testInvalidData() { XCTAssertThrowsError(try triState(data: 2, enable: 1)) }
    func testInvalidEnable() { XCTAssertThrowsError(try triState(data: 0, enable: 3)) }
}
