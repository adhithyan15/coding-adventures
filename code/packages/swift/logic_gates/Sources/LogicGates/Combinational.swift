// Combinational.swift
// Part of coding-adventures — an educational computing stack.
//
// ============================================================================
// Combinational Circuits — Building Blocks Between Gates and Arithmetic
// ============================================================================
//
// Between primitive gates and full arithmetic circuits, there is a family of
// "combinational building blocks" used everywhere in digital design — CPUs,
// FPGAs, memory controllers, bus arbiters. These circuits have no memory;
// output depends only on current inputs.
//
// Circuits implemented here:
//
//   Multiplexer (MUX)         — Route one of N inputs to output
//   Demultiplexer (DEMUX)     — Route one input to one of N outputs
//   Decoder                   — N-bit binary → 2^N one-hot
//   Encoder                   — 2^N one-hot → N-bit binary
//   Priority Encoder          — like Encoder, but handles multiple actives
//   Tri-state Buffer          — Connect or disconnect a signal (high-Z output)
//
// ============================================================================

// MARK: - Internal Helpers

// log2Ceil computes ⌈log₂(n)⌉ using bit shifting — no Foundation import needed.
// This is the number of bits required to represent n distinct values.
// For encoder/priority-encoder output widths: log2Ceil(4) = 2, log2Ceil(8) = 3.
private func log2Ceil(_ n: Int) -> Int {
    guard n > 1 else { return 0 }
    var bits = 0
    var v = n - 1
    while v > 0 {
        v >>= 1
        bits += 1
    }
    return bits
}

// MARK: - Multiplexer (MUX)

// ============================================================================
// MULTIPLEXER (MUX)
// ============================================================================
//
// A multiplexer is a "selector switch." It takes N data inputs and a set
// of select lines, and routes exactly one data input to the output.
// Think of it as a railroad switch that directs one train onto a single track.
//
//          ┌──────────┐
//   D0 ────┤          │
//   D1 ────┤   MUX    ├──── Output
//   ...    │          │
//   DN ────┤          │
//          └────┬─────┘
//               │
//   Sel ────────┘
//
// 2-to-1 MUX truth table:
//
//   Sel  D0  D1 │ Output
//   ────────────┼───────
//    0    0   X │   0       D0 selected
//    0    1   X │   1       D0 selected
//    1    X   0 │   0       D1 selected
//    1    X   1 │   1       D1 selected
//
// Gate equation: Output = OR(AND(D0, NOT(Sel)), AND(D1, Sel))
//
// Where MUX matters:
// - FPGAs: a K-input LUT is a 2^K-to-1 MUX with truth table in SRAM
// - CPUs: MUXes select ALU inputs, forwarded values, PC+4 vs branch target
// - Memory: MUXes route data to/from the correct bank

/// 2-to-1 multiplexer.
///
/// Routes D0 to output when sel=0, routes D1 to output when sel=1.
///
///     Output = sel ? d1 : d0
///
/// Gate-level: `AND(d0, NOT(sel)) OR AND(d1, sel)`
///
/// - Parameters:
///   - d0: Data input 0 (selected when sel=0).
///   - d1: Data input 1 (selected when sel=1).
///   - sel: Select signal. 0 → d0, 1 → d1.
/// - Returns: Selected data bit.
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func mux2(d0: Int, d1: Int, sel: Int) throws -> Int {
    try validateBit(d0, name: "d0")
    try validateBit(d1, name: "d1")
    try validateBit(sel, name: "sel")

    // Implement using gates: OR(AND(d0, NOT(sel)), AND(d1, sel))
    let notSel = try notGate(sel)
    let path0  = try andGate(d0, notSel)   // d0 when sel=0
    let path1  = try andGate(d1, sel)      // d1 when sel=1
    return try orGate(path0, path1)
}

/// 4-to-1 multiplexer.
///
/// Selects one of four inputs using a 2-bit select vector [s0, s1].
///
///   sel[0]  sel[1] │ Output
///   ───────────────┼───────
///    0       0     │  d0
///    1       0     │  d1
///    0       1     │  d2
///    1       1     │  d3
///
/// Built from three 2-to-1 MUXes:
///   Stage 1: MUX(d0, d1, sel[0])  and  MUX(d2, d3, sel[0])
///   Stage 2: MUX(stage1a, stage1b, sel[1])
///
/// - Parameters:
///   - d0, d1, d2, d3: Data inputs.
///   - sel: 2-element select vector [s0, s1]. sel[0] is the LSB.
/// - Returns: Selected data bit.
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
///           `LogicGateError.invalidSelectLength` if sel.count ≠ 2.
public func mux4(d0: Int, d1: Int, d2: Int, d3: Int, sel: [Int]) throws -> Int {
    guard sel.count == 2 else {
        throw LogicGateError.invalidSelectLength(expected: 2, got: sel.count)
    }
    for (i, s) in sel.enumerated() { try validateBit(s, name: "sel[\(i)]") }
    try validateBit(d0, name: "d0"); try validateBit(d1, name: "d1")
    try validateBit(d2, name: "d2"); try validateBit(d3, name: "d3")

    let low  = try mux2(d0: d0, d1: d1, sel: sel[0])  // select between d0/d1
    let high = try mux2(d0: d2, d1: d3, sel: sel[0])  // select between d2/d3
    return try mux2(d0: low, d1: high, sel: sel[1])    // select which pair
}

/// 8-to-1 multiplexer.
///
/// Selects one of eight inputs using a 3-bit select vector [s0, s1, s2].
///
/// - Parameters:
///   - inputs: Array of 8 data bits.
///   - sel: 3-element select vector [s0, s1, s2]. sel[0] is the LSB.
/// - Returns: Selected data bit.
/// - Throws: `LogicGateError.invalidBit` or `LogicGateError.invalidSelectLength`.
public func mux8(inputs: [Int], sel: [Int]) throws -> Int {
    guard inputs.count == 8 else {
        throw LogicGateError.invalidSelectLength(expected: 8, got: inputs.count)
    }
    guard sel.count == 3 else {
        throw LogicGateError.invalidSelectLength(expected: 3, got: sel.count)
    }
    return try muxN(inputs: inputs, sel: sel)
}

/// N-to-1 multiplexer (N must be a power of 2).
///
/// Selects one of N inputs using ⌈log₂(N)⌉ select bits.
/// Implemented recursively using 2-to-1 MUXes:
///
///   8:1 MUX = two 4:1 MUXes feeding a 2:1 MUX
///   4:1 MUX = two 2:1 MUXes feeding a 2:1 MUX
///
/// - Parameters:
///   - inputs: Array of data bits. Length must be a power of 2.
///   - sel: Select bits, LSB first (sel[0] is the first select line).
/// - Returns: Selected data bit.
/// - Throws: `LogicGateError.invalidBit`, `LogicGateError.invalidSelectLength`.
public func muxN(inputs: [Int], sel: [Int]) throws -> Int {
    for (i, v) in inputs.enumerated() { try validateBit(v, name: "inputs[\(i)]") }
    for (i, s) in sel.enumerated()    { try validateBit(s, name: "sel[\(i)]") }

    // Base case: 2 inputs, 1 select bit
    guard inputs.count > 2 else {
        guard inputs.count == 2 else {
            throw LogicGateError.insufficientInputs(minimum: 2, got: inputs.count)
        }
        guard sel.count >= 1 else {
            throw LogicGateError.invalidSelectLength(expected: 1, got: sel.count)
        }
        return try mux2(d0: inputs[0], d1: inputs[1], sel: sel[0])
    }

    // Recursive case: split inputs in half, recurse on each half,
    // then MUX the two results using the MSB of sel.
    let half = inputs.count / 2
    let lowerSel = Array(sel.dropLast())
    let lo = try muxN(inputs: Array(inputs[0..<half]),  sel: lowerSel)
    let hi = try muxN(inputs: Array(inputs[half...]),   sel: lowerSel)
    return try mux2(d0: lo, d1: hi, sel: sel.last!)
}

// MARK: - Demultiplexer (DEMUX)

// ============================================================================
// DEMULTIPLEXER (DEMUX)
// ============================================================================
//
// A DEMUX is the inverse of a MUX: one data input, N outputs. Select lines
// choose which output receives the data. All other outputs are 0.
//
//   1-to-4 DEMUX truth table:
//
//   Sel  │ Y0  Y1  Y2  Y3
//   ─────┼─────────────────
//   00   │  D   0   0   0
//   01   │  0   D   0   0
//   10   │  0   0   D   0
//   11   │  0   0   0   D
//
// Real use: memory address decoding. The address bits select which memory
// chip/bank receives the read/write signal. Only one chip is enabled at
// a time — all others see 0 on their chip-select input.

/// 1-to-N demultiplexer.
///
/// Routes `data` to the output selected by `sel`, setting all other outputs to 0.
///
/// The number of outputs is 2^(sel.count).
///
/// - Parameters:
///   - data: Data bit to route.
///   - sel: Select bits, LSB first. Selects which output receives `data`.
/// - Returns: Array of output bits. Exactly one is equal to `data`; others are 0.
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func demux(data: Int, sel: [Int]) throws -> [Int] {
    try validateBit(data, name: "data")
    for (i, s) in sel.enumerated() { try validateBit(s, name: "sel[\(i)]") }

    let nOutputs = 1 << sel.count  // 2^(number of select bits)
    var outputs = Array(repeating: 0, count: nOutputs)

    // Compute the selected index from select bits (sel[0] is LSB)
    let idx = sel.enumerated().reduce(0) { acc, pair in
        acc + pair.element * (1 << pair.offset)
    }
    outputs[idx] = data
    return outputs
}

// MARK: - Decoder

// ============================================================================
// DECODER
// ============================================================================
//
// A decoder converts an N-bit binary input into a "one-hot" output — exactly
// one of 2^N output lines is HIGH, the rest are LOW. It is conceptually a
// DEMUX with the data input hardwired to 1.
//
// 2-to-4 decoder:
//
//   A1  A0  │ Y0  Y1  Y2  Y3
//   ────────┼─────────────────
//    0   0  │  1   0   0   0   ← Y0 = NOT(A1) AND NOT(A0)
//    0   1  │  0   1   0   0   ← Y1 = NOT(A1) AND A0
//    1   0  │  0   0   1   0   ← Y2 = A1 AND NOT(A0)
//    1   1  │  0   0   0   1   ← Y3 = A1 AND A0
//
// Real use: instruction decoding in CPUs (opcode → control signals),
// memory chip-select, interrupt priority encoding.

/// N-to-2^N binary decoder.
///
/// Converts an N-bit binary input to a one-hot output of 2^N bits.
/// Exactly one output bit is 1; all others are 0.
///
/// - Parameter inputs: Array of binary input bits (LSB first, i.e., inputs[0] is bit 0).
/// - Returns: Array of 2^N output bits. outputs[i] = 1 iff inputs encodes i.
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func decoder(inputs: [Int]) throws -> [Int] {
    for (i, v) in inputs.enumerated() { try validateBit(v, name: "inputs[\(i)]") }

    let n = inputs.count
    let numOutputs = 1 << n  // 2^n outputs
    var outputs = Array(repeating: 0, count: numOutputs)

    // Compute which output to activate:
    // The selected index is the binary number represented by inputs (LSB first)
    let idx = inputs.enumerated().reduce(0) { acc, pair in
        acc + pair.element * (1 << pair.offset)
    }
    outputs[idx] = 1
    return outputs
}

// MARK: - Encoder

// ============================================================================
// ENCODER
// ============================================================================
//
// An encoder is the inverse of a decoder: 2^N one-hot input lines, N-bit
// binary output. The active input determines the output code.
//
// 4-to-2 encoder:
//
//   I0  I1  I2  I3  │ A1  A0
//   ────────────────┼────────
//    1   0   0   0  │  0   0
//    0   1   0   0  │  0   1
//    0   0   1   0  │  1   0
//    0   0   0   1  │  1   1
//
// Limitation: exactly ONE input must be active (one-hot). If zero or
// multiple inputs are active, the input is invalid.

/// 2^N-to-N encoder.
///
/// Converts a one-hot input of 2^N bits to an N-bit binary output.
/// Exactly one input bit must be 1.
///
/// - Parameter inputs: One-hot input array. Length must be a power of 2.
/// - Returns: Array of ⌈log₂(inputs.count)⌉ binary output bits (LSB first).
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
///           `LogicGateError.invalidEncoderInput` if input is not one-hot.
public func encoder(inputs: [Int]) throws -> [Int] {
    for (i, v) in inputs.enumerated() { try validateBit(v, name: "inputs[\(i)]") }

    let activeCount = inputs.reduce(0, +)
    guard activeCount == 1 else {
        throw LogicGateError.invalidEncoderInput(
            "exactly one input must be 1, got \(activeCount) active inputs"
        )
    }

    let activeIndex = inputs.firstIndex(of: 1)!
    let n = log2Ceil(inputs.count)  // number of output bits

    // Convert activeIndex to binary (LSB first)
    return (0..<n).map { bit in (activeIndex >> bit) & 1 }
}

// MARK: - Priority Encoder

// ============================================================================
// PRIORITY ENCODER
// ============================================================================
//
// A priority encoder is like a regular encoder, but handles the case where
// multiple inputs are active simultaneously. The highest-priority (highest-
// index) active input "wins" and its index is encoded.
//
// 4-to-2 Priority Encoder (I3 = highest priority):
//
//   I0  I1  I2  I3  │ A1  A0  Valid
//   ────────────────┼─────────────
//    0   0   0   0  │  X   X    0    No input active
//    1   0   0   0  │  0   0    1    I0 wins
//    X   1   0   0  │  0   1    1    I1 wins over I0
//    X   X   1   0  │  1   0    1    I2 wins over I0,I1
//    X   X   X   1  │  1   1    1    I3 always wins
//
// The "valid" output tells downstream logic whether any input was active.
//
// Real use: interrupt controllers (which interrupt fires when multiple
// arrive simultaneously?), FPGA carry-chain priority logic.

/// Priority encoder: highest-index active input wins.
///
/// - Parameter inputs: Input bits. Higher index = higher priority.
/// - Returns: Tuple of (binary output bits LSB-first, valid flag: 1 if any input active).
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func priorityEncoder(inputs: [Int]) throws -> (output: [Int], valid: Int) {
    for (i, v) in inputs.enumerated() { try validateBit(v, name: "inputs[\(i)]") }

    // Find the highest-priority (highest-index) active input
    guard let activeIndex = inputs.indices.reversed().first(where: { inputs[$0] == 1 }) else {
        // No input active: valid=0, output is all zeros
        let n = log2Ceil(inputs.count)
        return (output: Array(repeating: 0, count: n), valid: 0)
    }

    let n = log2Ceil(inputs.count)
    let output = (0..<n).map { bit in (activeIndex >> bit) & 1 }
    return (output: output, valid: 1)
}

// MARK: - Tri-State Buffer

// ============================================================================
// TRI-STATE BUFFER
// ============================================================================
//
// A tri-state buffer has three possible output states: 0, 1, or
// high-impedance (Z). High-impedance means the output is electrically
// disconnected — as if the wire were cut. We model this as nil in Swift.
//
//   Data  Enable │ Output
//   ─────────────┼───────
//     0      0   │  nil     Disconnected (high-Z)
//     1      0   │  nil     Disconnected (high-Z)
//     0      1   │  0       Active low
//     1      1   │  1       Active high
//
// Why this matters: in a shared bus (memory data bus, I²C, SPI), multiple
// devices connect to the same wires. Only ONE device can drive the bus at
// a time. Tri-state buffers let each device disconnect when idle, preventing
// electrical conflicts (two drivers fighting over a wire = bus contention =
// damaged hardware).
//
// In FPGAs, tri-state buffers appear in I/O blocks where pins can be
// configured as inputs (high-Z) or outputs (driven).

/// Tri-state buffer: either drives the bus or disconnects.
///
/// - Parameters:
///   - data: The data bit to drive on the bus (0 or 1).
///   - enable: Output enable. 1 → drive `data`; 0 → high-Z (nil).
/// - Returns: The data bit if enabled, `nil` if high-impedance.
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func triState(data: Int, enable: Int) throws -> Int? {
    try validateBit(data, name: "data")
    try validateBit(enable, name: "enable")
    return enable == 1 ? data : nil
}
