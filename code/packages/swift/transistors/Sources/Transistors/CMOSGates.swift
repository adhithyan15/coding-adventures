// CMOSGates.swift
// Part of coding-adventures — an educational computing stack.
//
// ============================================================================
// CMOS Gates — Complementary MOS Logic
// ============================================================================
//
// CMOS (Complementary MOS) is the dominant technology for digital logic.
// Every gate pairs an NMOS pull-down network with a PMOS pull-up network.
// The complementary arrangement guarantees:
//
//   - Exactly ONE path is conducting at any moment (no short-circuit current)
//   - Output swings fully from 0 V (GND) to Vdd
//   - Static power ≈ 0 (only leakage)
//   - Dynamic power ∝ C × Vdd² × f (switching energy)
//
// # The CMOS Inverter
//
//          Vdd
//           │
//         [PMOS] ← Gate (input)
//           │
//           ├──── Output
//           │
//         [NMOS] ← Gate (input)
//           │
//          GND
//
// When Input = LOW (0 V):
//   PMOS: Vgs = 0 − Vdd < −Vth → conducting (pull output HIGH)
//   NMOS: Vgs = 0 < Vth → off
//   Output = HIGH (Vdd)
//
// When Input = HIGH (Vdd):
//   PMOS: Vgs = Vdd − Vdd = 0 > −Vth → off
//   NMOS: Vgs = Vdd > Vth → conducting (pull output LOW)
//   Output = LOW (GND)
//
// # Gate Transistor Counts
//
//   Inverter (NOT):  2 transistors  (1 NMOS + 1 PMOS)
//   NAND 2-input:    4 transistors  (2 NMOS series + 2 PMOS parallel)
//   NOR 2-input:     4 transistors  (2 NMOS parallel + 2 PMOS series)
//   AND 2-input:     6 transistors  (NAND + inverter)
//   OR 2-input:      6 transistors  (NOR + inverter)
//   XOR 2-input:     12 transistors (CMOS XOR implementation)
//
// Note: NAND and NOR are "universal" — any logic function can be built
// from NAND gates alone (or NOR gates alone). This is why NAND is the
// most commonly synthesized primitive in real chip design.
//
// ============================================================================

// MARK: - CMOS Inverter (NOT gate)

/// A CMOS inverter: one PMOS pull-up + one NMOS pull-down.
///
/// The simplest CMOS gate. Output is the complement of the input.
/// All other CMOS gates are built by adding transistors to this structure.
public struct CMOSInverter {

    public let circuit: CircuitParams
    public let nmos: NMOS
    public let pmos: PMOS

    public init(
        circuit: CircuitParams = .cmos18,
        nmos: NMOS = NMOS(),
        pmos: PMOS = PMOS()
    ) {
        self.circuit = circuit
        self.nmos = nmos
        self.pmos = pmos
    }

    /// Evaluates the inverter at an analog input voltage.
    ///
    /// - Parameter inputVoltage: Input voltage (V), typically 0 or Vdd.
    /// - Returns: Full GateOutput including voltage, current, and timing.
    public func evaluate(inputVoltage: Double) -> GateOutput {
        let vdd = circuit.vdd
        // PMOS: source at Vdd, gate at input → Vgs_pmos = input − Vdd
        let vgsPmos = inputVoltage - vdd
        // NMOS: source at GND, gate at input → Vgs_nmos = input
        let vgsNmos = inputVoltage

        let pmosOn = pmos.isConducting(vgs: vgsPmos)
        let nmosOn = nmos.isConducting(vgs: vgsNmos)

        let outputVoltage: Double
        let current: Double

        if pmosOn && !nmosOn {
            // HIGH output: PMOS pulls to Vdd
            outputVoltage = vdd * 0.95
            current = pmos.drainCurrent(vgs: vgsPmos, vds: -vdd * 0.05)
        } else if nmosOn && !pmosOn {
            // LOW output: NMOS pulls to GND
            outputVoltage = vdd * 0.05
            current = nmos.drainCurrent(vgs: vgsNmos, vds: vdd * 0.05)
        } else {
            // Transition region — both conducting briefly
            outputVoltage = vdd / 2.0
            current = nmos.drainCurrent(vgs: vgsNmos, vds: vdd / 2.0)
        }

        let logicValue = outputVoltage > vdd / 2.0 ? 1 : 0
        let power = vdd * current
        // Propagation delay ≈ (Cload × Vdd) / Id
        let cLoad = nmos.params.cDrain + pmos.params.cDrain
        let delay = current > 0 ? (cLoad * vdd) / current : 1e-9

        return GateOutput(
            logicValue: logicValue,
            voltage: outputVoltage,
            currentDraw: current,
            powerDissipation: power,
            propagationDelay: delay,
            transistorCount: 2
        )
    }

    /// Digital evaluation: takes 0/1 input, returns 0/1 output.
    ///
    /// This is the interface `logic-gates` calls to implement NOT.
    public func evaluateDigital(_ a: Int) -> Int {
        let vin = a == 1 ? circuit.vdd : 0.0
        return evaluate(inputVoltage: vin).logicValue
    }

    /// Static (leakage) power (W). Essentially zero for CMOS.
    public func staticPower() -> Double {
        return 1e-9 * circuit.vdd  // sub-nW leakage
    }

    /// Dynamic power (W) = C × Vdd² × f.
    public func dynamicPower(frequency: Double, cLoad: Double) -> Double {
        return cLoad * circuit.vdd * circuit.vdd * frequency
    }

    /// Voltage transfer characteristic (VTC): output voltage vs. input voltage.
    ///
    /// Sweeps the input from 0 to Vdd in `steps` increments and records
    /// the output voltage. The resulting S-curve shows the switching threshold
    /// and the gain in the transition region.
    ///
    /// - Parameter steps: Number of sample points (default: 100).
    /// - Returns: Array of (Vin, Vout) pairs.
    public func voltageTransferCharacteristic(steps: Int = 100) -> [(vin: Double, vout: Double)] {
        let vdd = circuit.vdd
        return (0..<steps).map { i in
            let vin = vdd * Double(i) / Double(steps - 1)
            let vout = evaluate(inputVoltage: vin).voltage
            return (vin: vin, vout: vout)
        }
    }
}

// MARK: - CMOS NAND

/// CMOS NAND gate: 2 NMOS in series (pull-down) + 2 PMOS in parallel (pull-up).
///
/// Output = NOT(A AND B).
/// NAND is the universal gate — any Boolean function can be implemented
/// using NAND gates alone.
///
/// Truth table:
///   A B │ Out
///   ────┼────
///   0 0 │  1
///   0 1 │  1
///   1 0 │  1
///   1 1 │  0
public struct CMOSNand {

    public let circuit: CircuitParams
    public let nmos: NMOS
    public let pmos: PMOS

    public init(
        circuit: CircuitParams = .cmos18,
        nmos: NMOS = NMOS(),
        pmos: PMOS = PMOS()
    ) {
        self.circuit = circuit
        self.nmos = nmos
        self.pmos = pmos
    }

    public func evaluate(va: Double, vb: Double) -> GateOutput {
        let vdd = circuit.vdd
        let nmosA = nmos.isConducting(vgs: va)
        let nmosB = nmos.isConducting(vgs: vb)
        let pmosA = pmos.isConducting(vgs: va - vdd)
        let pmosB = pmos.isConducting(vgs: vb - vdd)

        // NAND: output LOW only when BOTH NMOS are ON (series pull-down)
        let pullDown = nmosA && nmosB
        // NAND: output HIGH when EITHER PMOS is ON (parallel pull-up)
        let pullUp = pmosA || pmosB

        let outputVoltage: Double
        let current: Double
        if pullUp && !pullDown {
            outputVoltage = vdd * 0.95
            current = pmos.drainCurrent(vgs: va - vdd, vds: -vdd * 0.05)
        } else if pullDown && !pullUp {
            outputVoltage = vdd * 0.05
            current = nmos.drainCurrent(vgs: va, vds: vdd * 0.05)
        } else {
            outputVoltage = vdd / 2.0
            current = nmos.drainCurrent(vgs: va, vds: vdd / 2.0) * 0.5
        }

        let logicValue = outputVoltage > vdd / 2.0 ? 1 : 0
        return GateOutput(
            logicValue: logicValue,
            voltage: outputVoltage,
            currentDraw: current,
            powerDissipation: vdd * current,
            propagationDelay: 1.5e-10,
            transistorCount: 4
        )
    }

    /// Digital evaluation: takes 0/1 inputs, returns 0/1 output.
    ///
    /// This is the interface `logic-gates` calls to implement NAND.
    public func evaluateDigital(_ a: Int, _ b: Int) -> Int {
        let va = a == 1 ? circuit.vdd : 0.0
        let vb = b == 1 ? circuit.vdd : 0.0
        return evaluate(va: va, vb: vb).logicValue
    }
}

// MARK: - CMOS NOR

/// CMOS NOR gate: 2 NMOS in parallel (pull-down) + 2 PMOS in series (pull-up).
///
/// Output = NOT(A OR B).
/// NOR is also universal — the entire Boolean algebra can be expressed in NOR.
///
/// Truth table:
///   A B │ Out
///   ────┼────
///   0 0 │  1
///   0 1 │  0
///   1 0 │  0
///   1 1 │  0
public struct CMOSNor {

    public let circuit: CircuitParams
    public let nmos: NMOS
    public let pmos: PMOS

    public init(
        circuit: CircuitParams = .cmos18,
        nmos: NMOS = NMOS(),
        pmos: PMOS = PMOS()
    ) {
        self.circuit = circuit
        self.nmos = nmos
        self.pmos = pmos
    }

    public func evaluate(va: Double, vb: Double) -> GateOutput {
        let vdd = circuit.vdd
        let nmosA = nmos.isConducting(vgs: va)
        let nmosB = nmos.isConducting(vgs: vb)
        let pmosA = pmos.isConducting(vgs: va - vdd)
        let pmosB = pmos.isConducting(vgs: vb - vdd)

        // NOR: output LOW when EITHER NMOS is ON (parallel pull-down)
        let pullDown = nmosA || nmosB
        // NOR: output HIGH only when BOTH PMOS are ON (series pull-up)
        let pullUp = pmosA && pmosB

        let outputVoltage: Double
        let current: Double
        if pullUp && !pullDown {
            outputVoltage = circuit.vdd * 0.95
            current = pmos.drainCurrent(vgs: va - vdd, vds: -vdd * 0.05)
        } else if pullDown && !pullUp {
            outputVoltage = circuit.vdd * 0.05
            current = nmos.drainCurrent(vgs: va, vds: vdd * 0.05)
        } else {
            outputVoltage = circuit.vdd / 2.0
            current = 0.0
        }

        let logicValue = outputVoltage > vdd / 2.0 ? 1 : 0
        return GateOutput(
            logicValue: logicValue,
            voltage: outputVoltage,
            currentDraw: current,
            powerDissipation: vdd * current,
            propagationDelay: 2.0e-10,
            transistorCount: 4
        )
    }

    /// Digital evaluation: takes 0/1 inputs, returns 0/1 output.
    ///
    /// This is the interface `logic-gates` calls to implement NOR.
    public func evaluateDigital(_ a: Int, _ b: Int) -> Int {
        let va = a == 1 ? circuit.vdd : 0.0
        let vb = b == 1 ? circuit.vdd : 0.0
        return evaluate(va: va, vb: vb).logicValue
    }
}

// MARK: - Derived CMOS Gates (AND, OR, XOR)

/// CMOS AND gate: NAND followed by an inverter. 6 transistors total.
///
/// AND = NOT(NAND(A, B))
public struct CMOSAnd {

    private let nand: CMOSNand
    private let inverter: CMOSInverter

    public init(circuit: CircuitParams = .cmos18) {
        self.nand = CMOSNand(circuit: circuit)
        self.inverter = CMOSInverter(circuit: circuit)
    }

    public var circuit: CircuitParams { nand.circuit }

    public func evaluate(va: Double, vb: Double) -> GateOutput {
        let nandOut = nand.evaluate(va: va, vb: vb)
        let invOut = inverter.evaluate(inputVoltage: nandOut.voltage)
        return GateOutput(
            logicValue: invOut.logicValue,
            voltage: invOut.voltage,
            currentDraw: nandOut.currentDraw + invOut.currentDraw,
            powerDissipation: nandOut.powerDissipation + invOut.powerDissipation,
            propagationDelay: nandOut.propagationDelay + invOut.propagationDelay,
            transistorCount: 6
        )
    }

    /// Digital evaluation — interface for `logic-gates` AND implementation.
    public func evaluateDigital(_ a: Int, _ b: Int) -> Int {
        let va = a == 1 ? circuit.vdd : 0.0
        let vb = b == 1 ? circuit.vdd : 0.0
        return evaluate(va: va, vb: vb).logicValue
    }
}

/// CMOS OR gate: NOR followed by an inverter. 6 transistors total.
///
/// OR = NOT(NOR(A, B))
public struct CMOSOr {

    private let nor: CMOSNor
    private let inverter: CMOSInverter

    public init(circuit: CircuitParams = .cmos18) {
        self.nor = CMOSNor(circuit: circuit)
        self.inverter = CMOSInverter(circuit: circuit)
    }

    public var circuit: CircuitParams { nor.circuit }

    public func evaluate(va: Double, vb: Double) -> GateOutput {
        let norOut = nor.evaluate(va: va, vb: vb)
        let invOut = inverter.evaluate(inputVoltage: norOut.voltage)
        return GateOutput(
            logicValue: invOut.logicValue,
            voltage: invOut.voltage,
            currentDraw: norOut.currentDraw + invOut.currentDraw,
            powerDissipation: norOut.powerDissipation + invOut.powerDissipation,
            propagationDelay: norOut.propagationDelay + invOut.propagationDelay,
            transistorCount: 6
        )
    }

    /// Digital evaluation — interface for `logic-gates` OR implementation.
    public func evaluateDigital(_ a: Int, _ b: Int) -> Int {
        let va = a == 1 ? circuit.vdd : 0.0
        let vb = b == 1 ? circuit.vdd : 0.0
        return evaluate(va: va, vb: vb).logicValue
    }
}

/// CMOS XOR gate: 12 transistors (standard CMOS XOR implementation).
///
/// XOR = (A AND NOT_B) OR (NOT_A AND B) = A ⊕ B
/// Equivalently: XOR(A,B) = NAND(NAND(A, NAND(A,B)), NAND(B, NAND(A,B)))
///
/// The 12-transistor count comes from building XOR from NAND primitives:
///   - 1 NAND (first level):  4 transistors
///   - 2 NAND (second level): 8 transistors
///   Total: 12 transistors
public struct CMOSXor {

    private let nand1: CMOSNand
    private let nand2: CMOSNand
    private let nand3: CMOSNand
    private let nand4: CMOSNand

    public init(circuit: CircuitParams = .cmos18) {
        nand1 = CMOSNand(circuit: circuit)
        nand2 = CMOSNand(circuit: circuit)
        nand3 = CMOSNand(circuit: circuit)
        nand4 = CMOSNand(circuit: circuit)
    }

    public var circuit: CircuitParams { nand1.circuit }

    public func evaluate(va: Double, vb: Double) -> GateOutput {
        let vdd = circuit.vdd
        // Build XOR from NAND: XOR(A,B) = NAND(NAND(A, NAND(A,B)), NAND(B, NAND(A,B)))
        let abNand = nand1.evaluate(va: va, vb: vb)
        let v_ab = abNand.voltage
        let out1 = nand2.evaluate(va: va, vb: v_ab)
        let out2 = nand3.evaluate(va: vb, vb: v_ab)
        let finalOut = nand4.evaluate(va: out1.voltage, vb: out2.voltage)

        return GateOutput(
            logicValue: finalOut.logicValue,
            voltage: finalOut.voltage,
            currentDraw: finalOut.currentDraw,
            powerDissipation: finalOut.powerDissipation,
            propagationDelay: abNand.propagationDelay * 3,
            transistorCount: 12
        )
    }

    /// Digital evaluation — interface for `logic-gates` XOR implementation.
    public func evaluateDigital(_ a: Int, _ b: Int) -> Int {
        let va = a == 1 ? circuit.vdd : 0.0
        let vb = b == 1 ? circuit.vdd : 0.0
        return evaluate(va: va, vb: vb).logicValue
    }
}

// ============================================================================
// CMOS XNOR Gate — XOR followed by an Inverter
// ============================================================================
//
// XNOR(A, B) = NOT(XOR(A, B))
//
// Truth table:
//
//   A | B | XNOR
//   --|---|-----
//   0 | 0 |  1    (same — equal)
//   0 | 1 |  0    (different)
//   1 | 0 |  0    (different)
//   1 | 1 |  1    (same — equal)
//
// Transistor count: CMOSXor transistorCount + 2 (XOR + Inverter).
// XNOR is the "equivalence" gate — it answers "are A and B equal?"
//
public struct CMOSXnor {

    let circuit: CircuitParams
    private let xorGate: CMOSXor
    private let inverter: CMOSInverter

    public init(circuit: CircuitParams = .cmos18) {
        self.circuit = circuit
        self.xorGate = CMOSXor(circuit: circuit)
        self.inverter = CMOSInverter(circuit: circuit)
    }

    /// Evaluates XNOR using XOR followed by an Inverter.
    public func evaluate(va: Double, vb: Double) -> GateOutput {
        let xorOut = xorGate.evaluate(va: va, vb: vb)
        let result = inverter.evaluate(inputVoltage: xorOut.voltage)
        let totalCurrent = xorOut.currentDraw + result.currentDraw
        return GateOutput(
            logicValue: result.logicValue,
            voltage: result.voltage,
            currentDraw: totalCurrent,
            powerDissipation: totalCurrent * circuit.vdd,
            propagationDelay: xorOut.propagationDelay + result.propagationDelay,
            transistorCount: xorOut.transistorCount + 2
        )
    }

    /// Evaluates XNOR with digital (0/1) inputs.
    public func evaluateDigital(_ a: Int, _ b: Int) -> Int {
        let va = a == 1 ? circuit.vdd : 0.0
        let vb = b == 1 ? circuit.vdd : 0.0
        return evaluate(va: va, vb: vb).logicValue
    }
}
