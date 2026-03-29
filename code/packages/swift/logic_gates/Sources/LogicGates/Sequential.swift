// Sequential.swift
// Part of coding-adventures — an educational computing stack.
//
// ============================================================================
// Sequential Logic — Memory from Combinational Gates
// ============================================================================
//
// The gates in Gates.swift are "combinational" — their output depends only
// on the current input, with no memory. Sequential logic adds memory by
// feeding a gate's output back as its own input. This feedback creates
// "stable states" that persist even after the input changes.
//
// The memory hierarchy, from bottom to top:
//
//   SR Latch          raw 1-bit memory (2 cross-coupled NOR gates)
//       ↓
//   D Latch           controlled 1-bit memory (SR + enable signal)
//       ↓
//   D Flip-Flop       edge-triggered 1-bit memory (master-slave D latches)
//       ↓
//   Register          N-bit word storage (N flip-flops in parallel)
//       ↓
//   Shift Register    serial-to-parallel converter (chained flip-flops)
//       ↓
//   Counter           binary counter (register + incrementer)
//
// Each level builds on the previous. By the time we reach registers, we
// have the fundamental building block of every CPU: addressable storage
// for words of data.
//
// ============================================================================

// MARK: - SR Latch State

/// State held by an SR latch or D latch.
///
/// Both Q and Q̄ are stored explicitly. In a real latch, Q̄ is literally
/// derived from the cross-coupled gate whose output is the complement of Q.
/// We store both for convenience (no need to recompute the complement).
public struct LatchState: Equatable {
    /// The primary output. Conventionally the "stored" value.
    public let q: Int
    /// The complementary output. Always equals `1 - q` in valid states.
    public let qBar: Int

    public init(q: Int, qBar: Int) {
        self.q = q
        self.qBar = qBar
    }
}

// MARK: - Flip-Flop State

/// State held by a D flip-flop.
///
/// In addition to Q and Q̄, we track the internal master-slave state for
/// educational purposes — showing which latch holds the current data.
public struct FlipFlopState: Equatable {
    public let q: Int
    public let qBar: Int
    /// Snapshot of the master latch's Q output at the moment of evaluation.
    public let masterQ: Int

    public init(q: Int, qBar: Int, masterQ: Int) {
        self.q = q
        self.qBar = qBar
        self.masterQ = masterQ
    }
}

// ============================================================================
// SR LATCH
// ============================================================================
//
// The SR latch is the simplest memory element. Two NOR gates are cross-coupled:
// each gate's output feeds one input of the other gate. This creates a
// "bistable" circuit — two stable output states — that retains its state
// even after the inputs are removed.
//
//           ┌─────────────────────────────────┐
//           │                                  │
//   S ──────┤ NOR ├──── Q                    │
//           │     ├◄────────────────────────────┤
//   R ──────┤ NOR ├──── Q̄                    │
//           │                                  │
//           └─────────────────────────────────┘
//
// Truth table:
//
//   S  R │ Q     Q̄   │ Action
//   ─────┼────────────┼──────────────────────────────────────
//   0  0 │ Q    Q̄   │ Hold — remember the previous state
//   1  0 │ 1     0    │ Set — store a 1 in Q
//   0  1 │ 0     1    │ Reset — store a 0 in Q
//   1  1 │ 0     0    │ Invalid — both outputs forced LOW
//
// The "1 1" input is invalid because it violates the invariant that
// Q and Q̄ are always complementary. When both inputs return to 0,
// the circuit's final state depends on which input drops last — this
// is undefined (race condition).

/// SR latch built from two cross-coupled NOR gates.
///
/// - Parameters:
///   - set: The Set input (S). HIGH → stores 1 in Q.
///   - reset: The Reset input (R). HIGH → stores 0 in Q (Q=0).
///   - q: Current Q state (previous output). Defaults to 0.
///   - qBar: Current Q̄ state (previous output). Defaults to 1.
/// - Returns: New `LatchState` with (Q, Q̄) after applying inputs.
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
///
/// - Note: When S=1, R=1 (invalid state), the output is (0, 0). This
///   models the electrical behavior where both NOR outputs are forced LOW.
///   In real hardware this creates a race condition on the S→R and R→S
///   transitions; software models the quiescent state while both are active.
public func srLatch(set: Int, reset: Int, q: Int = 0, qBar: Int = 1) throws -> LatchState {
    try validateBit(set, name: "set")
    try validateBit(reset, name: "reset")
    try validateBit(q, name: "q")
    try validateBit(qBar, name: "qBar")

    // The cross-coupled NOR equations:
    //   Q    = NOR(R, Q̄)
    //   Q̄   = NOR(S, Q)
    //
    // In the real circuit both gates settle simultaneously via feedback.
    // A single-pass evaluation is not enough: starting from Q=0, Q̄=1
    // with S=1, R=0 gives NOR(0,1)=0 on the first pass — wrong — because
    // Q̄ has not yet been updated by S. We therefore iterate to convergence
    // (at most 3 passes for any valid SR latch input combination).
    var curQ    = q
    var curQBar = qBar
    for _ in 0..<3 {
        let newQ    = try norGate(reset, curQBar)
        let newQBar = try norGate(set, newQ)
        if newQ == curQ && newQBar == curQBar { break }
        curQ    = newQ
        curQBar = newQBar
    }

    return LatchState(q: curQ, qBar: curQBar)
}

// ============================================================================
// D LATCH
// ============================================================================
//
// The SR latch has a problem: the "1 1" input is undefined. The D latch
// solves this by deriving S and R from a single data input D. An enable
// signal E controls whether the latch is "transparent" (E=1, follows D)
// or "opaque" (E=0, holds previous state).
//
//                                       SR Latch
//   D ──────┬──── [AND] ─────────────── S
//           │       ↑ E                 │      Q
//           │                           ▼
//           └──── [NOT] ── [AND] ──── R     Q̄
//                              ↑ E
//
// Truth table:
//
//   D  E │ Q    Q̄  │ Action
//   ─────┼──────────┼──────────────────────────────
//   X  0 │ Q    Q̄ │ Hold — latch is opaque/closed
//   0  1 │ 0     1  │ Store 0 — transparent/open
//   1  1 │ 1     0  │ Store 1 — transparent/open

/// D latch (transparent latch) built from SR latch + gating logic.
///
/// When enable=1 (transparent mode), Q follows D immediately.
/// When enable=0 (opaque mode), Q retains its last value.
///
/// - Parameters:
///   - data: The data input (D). Value to store when enable=1.
///   - enable: The enable input (E). 1 = transparent, 0 = opaque/hold.
///   - q: Current Q state. Defaults to 0.
///   - qBar: Current Q̄ state. Defaults to 1.
/// - Returns: New `LatchState` after applying inputs.
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func dLatch(data: Int, enable: Int, q: Int = 0, qBar: Int = 1) throws -> LatchState {
    try validateBit(data, name: "data")
    try validateBit(enable, name: "enable")
    try validateBit(q, name: "q")
    try validateBit(qBar, name: "qBar")

    // Derive S and R from D and E:
    //   S = D AND E    (set only when data=1 and enabled)
    //   R = NOT(D) AND E  (reset only when data=0 and enabled)
    let s = try andGate(data, enable)
    let notD = try notGate(data)
    let r = try andGate(notD, enable)

    return try srLatch(set: s, reset: r, q: q, qBar: qBar)
}

// ============================================================================
// D FLIP-FLOP
// ============================================================================
//
// The D latch has a problem called "transparency": when enable=1, Q follows
// D at any moment, which can cause data to ripple through multiple latches
// in a pipeline before the clock edge ends. The solution is the D flip-flop,
// which captures data ONLY at the clock edge.
//
// Architecture: Two D latches in series (master-slave), with complementary
// enable signals.
//
//          ┌────────────┐          ┌────────────┐
//  D ──────┤ D Latch    ├──────────┤ D Latch    ├──── Q
//          │ (Master)   │          │ (Slave)    │
// CLK' ────┤ Enable     │   CLK ───┤ Enable     │──── Q̄
//          └────────────┘          └────────────┘
//
// When CLK=0: Master is transparent (CLK'=1), Slave is opaque (CLK=0)
//   → Master samples D, Slave holds previous Q
//
// When CLK=1: Master is opaque (CLK'=0), Slave is transparent (CLK=1)
//   → Master locks in D, Slave copies Master's output to Q
//
// Result: Q updates only at the rising edge (0→1 transition) of CLK.
// This "edge triggering" prevents data from rippling through a pipeline.

/// D flip-flop with master-slave architecture (rising-edge triggered).
///
/// Q captures the value of D at the rising clock edge (CLK: 0→1).
/// During CLK=0, the master latch is transparent and samples D.
/// During CLK=1, the slave latch is transparent and outputs D's value.
///
/// - Parameters:
///   - data: The data input (D). Value captured at the rising edge.
///   - clock: The clock input (CLK). Rising edge (0→1) triggers capture.
///   - q: Current Q state. Defaults to 0.
///   - qBar: Current Q̄ state. Defaults to 1.
///   - masterQ: Current master latch Q. Defaults to 0.
///   - masterQBar: Current master latch Q̄. Defaults to 1.
/// - Returns: `FlipFlopState` with Q, Q̄, and master latch Q.
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func dFlipFlop(
    data: Int, clock: Int,
    q: Int = 0, qBar: Int = 1,
    masterQ: Int = 0, masterQBar: Int = 1
) throws -> FlipFlopState {
    try validateBit(data, name: "data")
    try validateBit(clock, name: "clock")
    try validateBit(q, name: "q")
    try validateBit(qBar, name: "qBar")
    try validateBit(masterQ, name: "masterQ")
    try validateBit(masterQBar, name: "masterQBar")

    // Master latch: enabled by CLK' (inverted clock)
    let clockBar = try notGate(clock)
    let master = try dLatch(data: data, enable: clockBar, q: masterQ, qBar: masterQBar)

    // Slave latch: enabled by CLK (non-inverted)
    let slave = try dLatch(data: master.q, enable: clock, q: q, qBar: qBar)

    return FlipFlopState(q: slave.q, qBar: slave.qBar, masterQ: master.q)
}

// ============================================================================
// REGISTER
// ============================================================================
//
// A register is N flip-flops sharing a common clock signal. On each rising
// edge of the clock, all N bits are captured simultaneously. This is the
// fundamental unit of storage in every CPU.
//
//   D[0] ── DFF ── Q[0]    ┐
//   D[1] ── DFF ── Q[1]    │  All share the same CLK input
//   D[2] ── DFF ── Q[2]    │
//   D[3] ── DFF ── Q[3]    ┘
//           ↑
//          CLK (shared)
//
// A CPU register file is an array of registers. The 16 general-purpose
// registers in ARM (x0-x15) are 16 parallel 64-bit registers, each
// consisting of 64 D flip-flops sharing a write-enable signal.

/// N-bit register: N D flip-flops sharing a common clock.
///
/// All bits are captured simultaneously on the rising clock edge.
///
/// - Parameters:
///   - data: Array of data bits (each 0 or 1). Length determines register width.
///   - clock: The clock input (CLK). Rising edge captures all bits.
///   - q: Current output state. Defaults to all zeros.
///   - masterQ: Current master latch states. Defaults to all zeros.
/// - Returns: Tuple of (new Q bits, array of `FlipFlopState` for each bit).
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func register(
    data: [Int], clock: Int,
    q: [Int]? = nil, masterQ: [Int]? = nil
) throws -> (q: [Int], states: [FlipFlopState]) {
    let n = data.count
    let currentQ   = q       ?? Array(repeating: 0, count: n)
    let currentMQ  = masterQ ?? Array(repeating: 0, count: n)

    try validateBit(clock, name: "clock")
    for (i, d) in data.enumerated() { try validateBit(d, name: "data[\(i)]") }

    var newQ: [Int] = []
    var states: [FlipFlopState] = []

    for i in 0..<n {
        let cq  = i < currentQ.count  ? currentQ[i]  : 0
        let cmq = i < currentMQ.count ? currentMQ[i] : 0
        let s = try dFlipFlop(
            data: data[i], clock: clock,
            q: cq, qBar: 1 - cq,
            masterQ: cmq, masterQBar: 1 - cmq
        )
        newQ.append(s.q)
        states.append(s)
    }

    return (q: newQ, states: states)
}

// ============================================================================
// SHIFT REGISTER
// ============================================================================
//
// A shift register is a chain of D flip-flops where each flip-flop's Q
// output drives the next flip-flop's D input. On each clock edge, each
// bit shifts one position to the right. A new bit enters from the left
// (serial input), and the rightmost bit exits (serial output).
//
//   SerialIn → [DFF] → [DFF] → [DFF] → [DFF] → SerialOut
//                ↑       ↑       ↑       ↑
//               CLK     CLK     CLK     CLK
//
// Parallel output: all DFF Q outputs simultaneously = Q[0..N-1]
//
// Applications:
// - Serial-to-parallel conversion (receive data bit-by-bit, output word)
// - Parallel-to-serial conversion (read word, transmit bit-by-bit)
// - Data delays (N-bit delay line)
// - LFSR (Linear Feedback Shift Register) for pseudo-random number generation

/// N-bit serial shift register.
///
/// Shifts stored bits one position to the right on each clock edge,
/// inserting `serialIn` at the leftmost position.
///
/// - Parameters:
///   - serialIn: New bit to shift in at position 0.
///   - clock: Clock signal. Rising edge causes all bits to shift.
///   - q: Current stored bits (MSB first). Defaults to all zeros.
/// - Returns: Tuple of (new Q bits, serial output from rightmost stage, flip-flop states).
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func shiftRegister(
    serialIn: Int, clock: Int,
    q: [Int] = [0, 0, 0, 0]  // default: 4-bit shift register
) throws -> (q: [Int], serialOut: Int, states: [FlipFlopState]) {
    try validateBit(serialIn, name: "serialIn")
    try validateBit(clock, name: "clock")
    for (i, v) in q.enumerated() { try validateBit(v, name: "q[\(i)]") }

    let n = q.count
    // Build the input chain: serialIn → q[0]'s D, q[0] → q[1]'s D, ...
    var data = [serialIn] + q.dropLast()

    var newQ: [Int] = []
    var states: [FlipFlopState] = []

    for i in 0..<n {
        let s = try dFlipFlop(
            data: data[i], clock: clock,
            q: q[i], qBar: 1 - q[i]
        )
        newQ.append(s.q)
        states.append(s)
    }

    return (q: newQ, serialOut: newQ.last ?? 0, states: states)
}

// ============================================================================
// COUNTER
// ============================================================================
//
// A counter is a register that increments its stored value on each clock
// edge. It is built from a register plus a binary incrementer (which is a
// chain of half-adders with carry-in=1).
//
// Simple 4-bit ripple counter counting from 0000 to 1111 (0 to 15):
//
//   Q = [0,0,0,0] → [0,0,0,1] → [0,0,1,0] → ... → [1,1,1,1] → [0,0,0,0]
//
// On overflow (0xF + 1 = 0x0), the carry-out is lost and the counter wraps.
//
// Real CPUs contain multiple counters:
//   - Program Counter (PC): tracks the current instruction address
//   - Performance counters: measure cycles, cache misses, branch mispredictions
//   - Timer registers: generate periodic interrupts

/// N-bit binary counter with synchronous reset.
///
/// Increments the stored count on each rising clock edge. Wraps to 0 on
/// overflow. When reset=1 and clock edge occurs, count resets to 0.
///
/// - Parameters:
///   - clock: Clock signal. Rising edge increments the counter.
///   - reset: Synchronous reset. When 1, counter clears to 0 on next clock.
///   - q: Current count as binary array (MSB first). Defaults to 4-bit zero.
/// - Returns: Tuple of (new count bits, overflow flag, flip-flop states).
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func counter(
    clock: Int, reset: Int = 0,
    q: [Int] = [0, 0, 0, 0]  // default: 4-bit counter
) throws -> (q: [Int], overflow: Int, states: [FlipFlopState]) {
    try validateBit(clock, name: "clock")
    try validateBit(reset, name: "reset")
    for (i, v) in q.enumerated() { try validateBit(v, name: "q[\(i)]") }

    let n = q.count

    // If reset is asserted, load all zeros; otherwise increment.
    var nextCount: [Int]
    var overflow = 0

    if reset == 1 {
        nextCount = Array(repeating: 0, count: n)
    } else {
        // Binary increment: add 1 to the current count using ripple carry.
        // Carry propagates from LSB (last element) to MSB (first element).
        var carry = 1
        nextCount = Array(repeating: 0, count: n)
        for i in stride(from: n - 1, through: 0, by: -1) {
            let sum = q[i] + carry
            nextCount[i] = sum % 2
            carry = sum / 2
        }
        overflow = carry  // 1 if the counter wrapped around
    }

    // Load next count into register on the clock edge.
    let (newQ, states) = try register(data: nextCount, clock: clock, q: q)

    return (q: newQ, overflow: overflow, states: states)
}
