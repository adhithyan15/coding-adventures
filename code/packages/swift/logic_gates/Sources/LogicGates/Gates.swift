// Gates.swift
// Part of coding-adventures — an educational computing stack.
//
// ============================================================================
// Logic Gates — The Seven Fundamental Gates and Their Derivations
// ============================================================================
//
// A logic gate takes one or two binary inputs (0 or 1) and produces one
// binary output (0 or 1). The output is determined entirely by the inputs —
// no state, no memory, no randomness. Pure combinational logic.
//
// # Physical Implementation
//
// Every gate function delegates to the Transistors module, which simulates
// the underlying CMOS circuitry. This reflects physical reality: a NOT gate
// IS a CMOS inverter — two transistors connected in complementary fashion.
// The digital 0/1 interface here is an abstraction over analog physics.
//
// Gate → CMOS Primitive → Transistor count
// ─────────────────────────────────────────
//  NOT  → CMOSInverter  → 2  (1 PMOS + 1 NMOS)
//  NAND → CMOSNand      → 4  (2 NMOS series + 2 PMOS parallel)
//  NOR  → CMOSNor       → 4  (2 NMOS parallel + 2 PMOS series)
//  AND  → CMOSAnd       → 6  (NAND + inverter)
//  OR   → CMOSOr        → 6  (NOR + inverter)
//  XOR  → CMOSXor       → 12 (4 NAND gates)
//  XNOR → CMOSXnor      → 14 (XOR + inverter)
//
// # Why NAND and NOR are the Natural Gates
//
// In CMOS technology, pull-down networks are built from NMOS transistors in
// series (for NAND) or parallel (for NOR). Pull-up networks are the dual
// arrangement of PMOS transistors. This "push-pull" structure is what makes
// CMOS so efficient: at any moment, exactly one network conducts.
//
// AND requires an extra inversion stage after NAND — 6 transistors vs. 4.
// This is why chip designers prefer NAND-based logic: fewer transistors =
// smaller die = lower cost = higher speed.
//
// ============================================================================

import Transistors

// ============================================================================
// Error Type
// ============================================================================
//
// Logic gates operate on binary values: exactly 0 or 1. Passing any other
// value (2, -1, true, etc.) is a programming error that must be caught early.
// We use a throwing API so callers must explicitly handle or propagate errors.

/// Errors that can occur when evaluating logic gates or circuits.
public enum LogicGateError: Error, CustomStringConvertible, Equatable {

    /// An input that must be 0 or 1 was given an out-of-range value.
    case invalidBit(name: String, got: Int)

    /// A multi-input gate received fewer than the required minimum inputs.
    case insufficientInputs(minimum: Int, got: Int)

    /// A multiplexer's select vector has wrong length for the given input count.
    case invalidSelectLength(expected: Int, got: Int)

    /// An encoder received an input that is not a valid one-hot pattern.
    case invalidEncoderInput(String)

    public var description: String {
        switch self {
        case .invalidBit(let name, let got):
            return "\(name) must be 0 or 1, got \(got)"
        case .insufficientInputs(let min, let got):
            return "requires at least \(min) inputs, got \(got)"
        case .invalidSelectLength(let expected, let got):
            return "select vector must have length \(expected), got \(got)"
        case .invalidEncoderInput(let msg):
            return "invalid encoder input: \(msg)"
        }
    }
}

// ============================================================================
// Input Validation
// ============================================================================
//
// Every gate validates its inputs before evaluation. We validate at the
// boundary so gate logic stays clean. This mirrors hardware: real chips have
// defined valid input ranges; supplying out-of-spec voltages causes undefined
// behavior.

/// Validates that a value is a binary bit (0 or 1).
///
/// - Parameters:
///   - value: The value to check.
///   - name: Label for the error message (e.g., "a", "b", "data").
/// - Throws: `LogicGateError.invalidBit` if value is not 0 or 1.
@inline(__always)
func validateBit(_ value: Int, name: String) throws {
    guard value == 0 || value == 1 else {
        throw LogicGateError.invalidBit(name: name, got: value)
    }
}

// ============================================================================
// THE FOUR FUNDAMENTAL GATES
// ============================================================================
//
// NOT, AND, OR, and XOR are the four gates from which all other gates can
// be constructed. Each is defined by a truth table — an exhaustive listing
// of every possible input combination and the corresponding output.

// MARK: - NOT Gate

/// The NOT gate (inverter): flips a single binary input.
///
/// NOT is the simplest gate. It has one input and one output. When the
/// input is 0, the output is 1. When the input is 1, the output is 0.
///
/// Truth table:
///
///     Input │ Output
///     ──────┼───────
///       0   │   1      ← GND turns into Vdd
///       1   │   0      ← Vdd turns into GND
///
/// # Physical Reality
///
/// A NOT gate IS a CMOS inverter — one PMOS transistor connected from
/// Vdd to the output, and one NMOS transistor connected from the output
/// to GND, with both gates driven by the input.
///
///          Vdd
///           │
///         [PMOS] ← input (gate)
///           │
///           ├──── output
///           │
///         [NMOS] ← input (gate)
///           │
///          GND
///
/// When input = LOW (0 V): PMOS conducts (Vgs = -Vdd < -Vth), output → Vdd
/// When input = HIGH (Vdd): NMOS conducts (Vgs = Vdd > Vth), output → GND
///
/// Real-world analogy: a light switch wired "backwards" — flip it UP
/// and the light turns OFF; flip it DOWN and the light turns ON.
///
/// - Parameter a: Binary input (0 or 1).
/// - Returns: The inverted bit: 1 if a=0, 0 if a=1.
/// - Throws: `LogicGateError.invalidBit` if a is not 0 or 1.
public func notGate(_ a: Int) throws -> Int {
    try validateBit(a, name: "a")
    // Delegate to the CMOS inverter simulation in the Transistors module.
    // CMOSInverter.evaluateDigital converts 0/1 to analog voltages internally,
    // runs the full PMOS/NMOS conductance model, then returns 0/1.
    return CMOSInverter().evaluateDigital(a)
}

// MARK: - AND Gate

/// The AND gate: output is 1 only when BOTH inputs are 1.
///
/// AND is the digital equivalent of multiplication. If either input is 0,
/// the product is 0. Only 1 × 1 = 1.
///
/// Truth table:
///
///     A  B │ Output
///     ─────┼───────
///     0  0 │   0
///     0  1 │   0
///     1  0 │   0
///     1  1 │   1      ← only case where output is 1
///
/// # Physical Reality
///
/// CMOS AND = NAND + Inverter (6 transistors total).
/// NAND is the natural CMOS gate (4 transistors). Adding an inverter stage
/// gives AND. This is why AND is slightly more expensive in silicon.
///
/// Real-world analogy: two switches in SERIES. Both must be closed (ON)
/// for current to flow:
///
///     Power ─── [Switch A] ─── [Switch B] ─── Light ─── GND
///                                              ↑
///                              Both closed = Light ON
///
/// - Parameters:
///   - a: First binary input (0 or 1).
///   - b: Second binary input (0 or 1).
/// - Returns: 1 if both a and b are 1, else 0.
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func andGate(_ a: Int, _ b: Int) throws -> Int {
    try validateBit(a, name: "a")
    try validateBit(b, name: "b")
    // CMOSAnd = NAND(A,B) followed by CMOSInverter. The transistors module
    // models both stages: NAND pulls down when both inputs are HIGH, then the
    // inverter flips the result to give the AND output.
    return CMOSAnd().evaluateDigital(a, b)
}

// MARK: - OR Gate

/// The OR gate: output is 1 if EITHER input (or both) is 1.
///
/// OR is the digital equivalent of addition, clamped at 1. The only way
/// to get 0 is if both inputs are 0.
///
/// Truth table:
///
///     A  B │ Output
///     ─────┼───────
///     0  0 │   0      ← only case where output is 0
///     0  1 │   1
///     1  0 │   1
///     1  1 │   1
///
/// # Physical Reality
///
/// CMOS OR = NOR + Inverter (6 transistors total).
/// NOR is the other natural CMOS gate (4 transistors). Adding an inverter
/// stage gives OR.
///
/// Real-world analogy: two switches in PARALLEL. Either switch being
/// closed (ON) completes the circuit:
///
///     Power ──┬── [Switch A] ──┬── Light ─── GND
///             │                │
///             └── [Switch B] ──┘
///             ↑
///     Either closed = Light ON
///
/// - Parameters:
///   - a: First binary input (0 or 1).
///   - b: Second binary input (0 or 1).
/// - Returns: 1 if either a or b is 1, else 0.
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func orGate(_ a: Int, _ b: Int) throws -> Int {
    try validateBit(a, name: "a")
    try validateBit(b, name: "b")
    // CMOSOr = NOR(A,B) followed by CMOSInverter.
    return CMOSOr().evaluateDigital(a, b)
}

// MARK: - XOR Gate

/// The XOR gate (exclusive OR): output is 1 if inputs are DIFFERENT.
///
/// XOR is the "inequality detector." If both inputs are the same (both 0
/// or both 1), the output is 0. If they differ, output is 1.
///
/// Truth table:
///
///     A  B │ Output
///     ─────┼───────
///     0  0 │   0      ← same → 0
///     0  1 │   1      ← different → 1
///     1  0 │   1      ← different → 1
///     1  1 │   0      ← same → 0
///
/// # Why XOR matters for arithmetic
///
/// In binary addition: 1 + 1 = 10 (decimal 2). The sum digit is 0,
/// the carry is 1. Notice: XOR(1, 1) = 0, which is exactly the sum
/// digit! This is no coincidence — XOR is "addition without carry,"
/// which is why the half-adder uses XOR for its sum output.
///
/// # Physical Reality
///
/// CMOS XOR requires 12 transistors (4 NAND gates in a specific
/// feedback arrangement). It is one of the most complex "single gates":
///
///   Let C = NAND(A, B)
///   XOR(A,B) = NAND(NAND(A, C), NAND(B, C))
///
/// - Parameters:
///   - a: First binary input (0 or 1).
///   - b: Second binary input (0 or 1).
/// - Returns: 1 if a ≠ b, else 0.
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func xorGate(_ a: Int, _ b: Int) throws -> Int {
    try validateBit(a, name: "a")
    try validateBit(b, name: "b")
    // CMOSXor uses 4 NAND gates wired in the classical CMOS XOR topology.
    return CMOSXor().evaluateDigital(a, b)
}

// ============================================================================
// THE THREE COMPOSITE GATES
// ============================================================================
//
// These gates are each the NOT of a fundamental two-input gate. They are
// important because NAND and NOR are individually "functionally complete"
// — every Boolean function can be built from NAND gates alone (or NOR alone).

// MARK: - NAND Gate

/// The NAND gate (NOT-AND): output is 0 only when BOTH inputs are 1.
///
/// NAND is the opposite of AND. It is also one of the two "universal" gates —
/// every other Boolean function can be built from NAND gates alone.
///
/// Truth table:
///
///     A  B │ Output
///     ─────┼───────
///     0  0 │   1
///     0  1 │   1
///     1  0 │   1
///     1  1 │   0      ← only case where output is 0
///
/// # Why NAND is the Most Important Gate
///
/// In CMOS technology, NAND is the cheapest gate to build (4 transistors,
/// no extra inversion stage). Real chip design tools synthesize most logic
/// as NAND-based standard cells. ASIC libraries often consist almost entirely
/// of NAND gates. Knowing that NAND is universal, chip designers can implement
/// ANY Boolean function using only NAND gates — which minimizes manufacturing
/// cost and maximizes yield.
///
/// - Parameters:
///   - a: First binary input (0 or 1).
///   - b: Second binary input (0 or 1).
/// - Returns: 0 if both a and b are 1, else 1.
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func nandGate(_ a: Int, _ b: Int) throws -> Int {
    try validateBit(a, name: "a")
    try validateBit(b, name: "b")
    // CMOSNand is the natural CMOS gate: 2 NMOS in series (pull-down)
    // + 2 PMOS in parallel (pull-up). No extra inversion needed.
    return CMOSNand().evaluateDigital(a, b)
}

// MARK: - NOR Gate

/// The NOR gate (NOT-OR): output is 1 only when BOTH inputs are 0.
///
/// NOR is the opposite of OR. Like NAND, NOR is also "functionally complete."
/// NOR gates are the foundation of SR latches — the simplest memory elements.
///
/// Truth table:
///
///     A  B │ Output
///     ─────┼───────
///     0  0 │   1      ← only case where output is 1
///     0  1 │   0
///     1  0 │   0
///     1  1 │   0
///
/// # NOR in Sequential Logic
///
/// Two NOR gates cross-coupled (each output feeds the other's input) form
/// an SR latch — the simplest bistable circuit. This is how RAM worked
/// before SRAM; even today, flip-flops are built from cross-coupled gates.
///
/// - Parameters:
///   - a: First binary input (0 or 1).
///   - b: Second binary input (0 or 1).
/// - Returns: 1 if both a and b are 0, else 0.
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func norGate(_ a: Int, _ b: Int) throws -> Int {
    try validateBit(a, name: "a")
    try validateBit(b, name: "b")
    // CMOSNor is the other natural CMOS gate: 2 NMOS in parallel (pull-down)
    // + 2 PMOS in series (pull-up).
    return CMOSNor().evaluateDigital(a, b)
}

// MARK: - XNOR Gate

/// The XNOR gate (NOT-XOR, equivalence gate): output is 1 when inputs are EQUAL.
///
/// XNOR is the "equality detector." It outputs 1 when A equals B,
/// and 0 when they differ. Sometimes called the "coincidence gate."
///
/// Truth table:
///
///     A  B │ Output
///     ─────┼───────
///     0  0 │   1      ← same (both 0) → 1
///     0  1 │   0      ← different → 0
///     1  0 │   0      ← different → 0
///     1  1 │   1      ← same (both 1) → 1
///
/// # Application: N-bit Comparator
///
/// To check if two N-bit numbers are equal, XOR each pair of bits and NOR
/// the results. If all bit pairs match, all XOR outputs are 0, and NOR
/// gives 1. Equivalently, XNOR each pair and AND the results.
///
/// - Parameters:
///   - a: First binary input (0 or 1).
///   - b: Second binary input (0 or 1).
/// - Returns: 1 if a equals b, else 0.
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func xnorGate(_ a: Int, _ b: Int) throws -> Int {
    try validateBit(a, name: "a")
    try validateBit(b, name: "b")
    // CMOSXnor = CMOSXor followed by CMOSInverter. The transistors module
    // chains the XOR (12 transistors) with an inverter (2 transistors).
    return CMOSXnor().evaluateDigital(a, b)
}

// ============================================================================
// NAND-DERIVED GATES — Proving Functional Completeness
// ============================================================================
//
// The following functions prove that NAND is "functionally complete" —
// every logic function can be built from NAND gates alone. This is not
// just theoretical: real ASIC synthesis tools often use NAND-only standard
// cell libraries because NAND gates are cheaper (4 transistors vs. 6 for AND)
// and faster (no extra inversion stage).
//
// De Morgan's Laws provide the algebraic foundation:
//   NOT(A AND B) = NOT(A) OR NOT(B)  ← NAND = OR of NOTs
//   NOT(A OR B)  = NOT(A) AND NOT(B) ← NOR = AND of NOTs
//
// From these, any gate can be expressed in terms of NAND.

// MARK: - NAND-derived NOT

/// NOT built exclusively from NAND gates.
///
///     NOT(a) = NAND(a, a)
///
/// Connecting both inputs of a NAND gate to the same signal:
///   NAND(0, 0) = 1  → NOT(0) = 1 ✓
///   NAND(1, 1) = 0  → NOT(1) = 0 ✓
///
/// This works because NAND(a, a) = NOT(a AND a) = NOT(a).
/// Uses 1 NAND gate (4 transistors) vs. the standard inverter (2 transistors).
///
/// - Parameter a: Binary input (0 or 1).
/// - Returns: 1 if a=0, 0 if a=1.
/// - Throws: `LogicGateError.invalidBit` if a is not 0 or 1.
public func nandNot(_ a: Int) throws -> Int {
    try validateBit(a, name: "a")
    return try nandGate(a, a)
}

// MARK: - NAND-derived AND

/// AND built exclusively from NAND gates.
///
///     AND(a, b) = NOT(NAND(a, b)) = NAND(NAND(a, b), NAND(a, b))
///
/// First compute NAND(a,b), then invert it using the NAND-NOT trick.
/// Uses 2 NAND gates (8 transistors) vs. the standard AND (6 transistors).
///
/// Gate diagram:
///                ┌──────────┐    ┌──────────┐
///   A ──────────┤          │    │          │
///               │   NAND   ├────┤   NAND   ├─── Output
///   B ──────────┤          │    │ (same    │
///                └──────────┘    │ input)   │
///                           └───┘          │
///                                          └─── (tied)
///
/// - Parameters:
///   - a: First binary input (0 or 1).
///   - b: Second binary input (0 or 1).
/// - Returns: 1 if both a and b are 1, else 0.
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func nandAnd(_ a: Int, _ b: Int) throws -> Int {
    try validateBit(a, name: "a")
    try validateBit(b, name: "b")
    let nab = try nandGate(a, b)
    return try nandGate(nab, nab)  // = NOT(NAND(a,b)) = AND(a,b)
}

// MARK: - NAND-derived OR

/// OR built exclusively from NAND gates.
///
///     OR(a, b) = NAND(NOT(a), NOT(b)) = NAND(NAND(a,a), NAND(b,b))
///
/// By De Morgan's Law: NOT(NOT(a) AND NOT(b)) = a OR b
/// Since NAND(x,x) = NOT(x), we get:
///   NAND(NOT(a), NOT(b)) = NOT(NOT(a) AND NOT(b)) = a OR b
///
/// Uses 3 NAND gates (12 transistors) vs. the standard OR (6 transistors).
///
/// This is the bridge between NAND and OR. In early TTL chip design,
/// building OR gates from NAND was extremely common.
///
/// - Parameters:
///   - a: First binary input (0 or 1).
///   - b: Second binary input (0 or 1).
/// - Returns: 1 if either a or b is 1, else 0.
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func nandOr(_ a: Int, _ b: Int) throws -> Int {
    try validateBit(a, name: "a")
    try validateBit(b, name: "b")
    let na = try nandGate(a, a)  // NOT(a)
    let nb = try nandGate(b, b)  // NOT(b)
    return try nandGate(na, nb)  // NAND(NOT(a), NOT(b)) = OR(a,b)
}

// MARK: - NAND-derived XOR

/// XOR built exclusively from NAND gates.
///
///     Let C = NAND(a, b)
///     XOR(a, b) = NAND(NAND(a, C), NAND(b, C))
///
/// Uses 4 NAND gates (16 transistors) vs. the standard XOR (12 transistors).
/// This is the classic 4-NAND XOR circuit found in textbooks and silicon.
///
/// Verification:
///   A=0, B=0: C=1; NAND(0,1)=1; NAND(0,1)=1; NAND(1,1)=0 ✓
///   A=0, B=1: C=1; NAND(0,1)=1; NAND(1,1)=0; NAND(1,0)=1 ✓
///   A=1, B=0: C=1; NAND(1,1)=0; NAND(0,1)=1; NAND(0,1)=1 ✓
///   A=1, B=1: C=0; NAND(1,0)=1; NAND(1,0)=1; NAND(1,1)=0 ✓
///
/// - Parameters:
///   - a: First binary input (0 or 1).
///   - b: Second binary input (0 or 1).
/// - Returns: 1 if a ≠ b, else 0.
/// - Throws: `LogicGateError.invalidBit` if any input is not 0 or 1.
public func nandXor(_ a: Int, _ b: Int) throws -> Int {
    try validateBit(a, name: "a")
    try validateBit(b, name: "b")
    let c  = try nandGate(a, b)        // C = NAND(A, B)
    let l  = try nandGate(a, c)        // NAND(A, C)
    let r  = try nandGate(b, c)        // NAND(B, C)
    return try nandGate(l, r)          // NAND(NAND(A,C), NAND(B,C)) = XOR(A,B)
}

// ============================================================================
// MULTI-INPUT VARIANTS
// ============================================================================
//
// Real circuits often need to AND or OR more than two signals simultaneously.
// For example, detecting whether all four flag bits are clear requires a
// 4-input AND gate. We implement these by chaining two-input gates, which
// is exactly how hardware implements N-input gates: as a tree of 2-input gates.
//
// N-input AND is equivalent to a balanced binary tree of 2-input AND gates.
// For 4 inputs: AND(AND(a,b), AND(c,d)). For 3 inputs: AND(AND(a,b), c).

// MARK: - N-input AND

/// AND gate with N inputs (N ≥ 2).
///
/// Returns 1 only if ALL inputs are 1. Equivalent to chaining two-input
/// AND gates:
///
///     andN([a, b, c]) = andGate(andGate(a, b), c)
///
/// Hardware analogy: N switches in series — all must be closed for
/// current to flow.
///
/// - Parameter inputs: Array of binary values (each 0 or 1). Minimum 2 inputs.
/// - Returns: 1 if all inputs are 1, else 0.
/// - Throws: `LogicGateError.insufficientInputs` if fewer than 2 inputs.
///           `LogicGateError.invalidBit` if any input is not 0 or 1.
public func andN(_ inputs: [Int]) throws -> Int {
    guard inputs.count >= 2 else {
        throw LogicGateError.insufficientInputs(minimum: 2, got: inputs.count)
    }
    for (i, v) in inputs.enumerated() {
        try validateBit(v, name: "input[\(i)]")
    }
    return try inputs.dropFirst().reduce(inputs[0]) { acc, x in
        try andGate(acc, x)
    }
}

// MARK: - N-input OR

/// OR gate with N inputs (N ≥ 2).
///
/// Returns 1 if ANY input is 1. Equivalent to chaining two-input OR gates:
///
///     orN([a, b, c]) = orGate(orGate(a, b), c)
///
/// Hardware analogy: N switches in parallel — any one being closed
/// allows current to flow.
///
/// - Parameter inputs: Array of binary values (each 0 or 1). Minimum 2 inputs.
/// - Returns: 1 if any input is 1, else 0.
/// - Throws: `LogicGateError.insufficientInputs` if fewer than 2 inputs.
///           `LogicGateError.invalidBit` if any input is not 0 or 1.
public func orN(_ inputs: [Int]) throws -> Int {
    guard inputs.count >= 2 else {
        throw LogicGateError.insufficientInputs(minimum: 2, got: inputs.count)
    }
    for (i, v) in inputs.enumerated() {
        try validateBit(v, name: "input[\(i)]")
    }
    return try inputs.dropFirst().reduce(inputs[0]) { acc, x in
        try orGate(acc, x)
    }
}
