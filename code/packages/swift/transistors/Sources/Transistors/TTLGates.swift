// TTLGates.swift
// Part of coding-adventures — an educational computing stack.
//
// ============================================================================
// TTL Gates — Transistor-Transistor Logic
// ============================================================================
//
// TTL was the dominant logic family from the late 1960s through the early
// 1990s, before CMOS took over. The 7400 series (7400 = quad 2-input NAND)
// is the archetype — billions of these chips were produced.
//
// # Why Study TTL?
//
// 1. Historical significance: The Intel 4004 (1971), 8080 (1974), and Z80
//    (1976) were all built with or interfaced to TTL logic.
// 2. Educational clarity: TTL's BJT-based structure makes the transistor
//    physics more visible than CMOS.
// 3. Still in use: TTL-compatible voltage levels (0/5 V) are found in
//    microcontroller I/O pins and legacy industrial equipment.
//
// # TTL NAND Gate Structure
//
//       Vcc (5 V)
//         │
//        [R1] 4kΩ
//         │
//    A ───┤ Q1 (multi-emitter)
//    B ───┘  │
//            │ collector
//           [R2] 1.6kΩ
//            │
//            ├───┤ Q3 (output driver)
//            │   │
//           [R3] │ output
//           1kΩ  │
//            │  [R4] 130Ω
//            │   │
//           Q2   └───── OUT
//            │
//           GND
//
// When BOTH inputs are HIGH:
//   Q1 operates in reverse-active mode, Q2 and Q3 both saturate.
//   Output = LOW (≈ 0.2 V)
//
// When ANY input is LOW:
//   Q1 saturates, stealing base drive from Q2/Q3.
//   Output = HIGH (≈ 3.4 V)
//
// # RTL (Resistor-Transistor Logic)
//
// RTL is even simpler — a single resistor and NPN transistor form an inverter.
// RTL was used in the Apollo Guidance Computer (1966). It's the simplest
// possible digital logic implementation.
//
//   Vcc
//    │
//   [Rc] collector resistor
//    │
//    └─── OUT
//    │
//   [Q] NPN
//    │
//   [Rb] base resistor ──── IN
//    │
//   GND
//
// ============================================================================

// MARK: - TTL NAND Gate

/// A TTL 2-input NAND gate.
///
/// Models the classic 7400-series NAND cell using NPN BJTs.
/// Output = NOT(A AND B).
public struct TTLNand {

    /// Supply voltage — standard TTL uses 5 V.
    public let vcc: Double
    public let bjt: BJTParams

    /// Base resistor (Ω). Limits base current into Q2.
    public let rBase: Double = 4000.0

    /// Collector resistor (Ω). Pull-up to Vcc.
    public let rCollector: Double = 1600.0

    public init(vcc: Double = 5.0, bjt: BJTParams = .defaultNPN) {
        self.vcc = vcc
        self.bjt = bjt
    }

    /// Evaluates the TTL NAND at analog input voltages.
    ///
    /// - Parameters:
    ///   - va: Input A voltage (V). HIGH ≈ 3.4 V, LOW ≈ 0.2 V.
    ///   - vb: Input B voltage (V).
    public func evaluate(va: Double, vb: Double) -> GateOutput {
        let npn = NPN(params: bjt)

        // TTL NAND: output LOW only when BOTH inputs are HIGH
        let aHigh = va > 2.0   // TTL HIGH threshold ≈ 2.0 V
        let bHigh = vb > 2.0

        let outputVoltage: Double
        let current: Double

        if aHigh && bHigh {
            // Both HIGH → output transistors saturate → OUT = VceSat ≈ 0.2 V
            outputVoltage = bjt.vceSat
            let vbe = bjt.vbeOn
            current = npn.collectorCurrent(vbe: vbe, vce: bjt.vceSat)
        } else {
            // At least one LOW → output transistors off → OUT = Vcc − drop
            outputVoltage = vcc - 1.4  // Vcc minus two diode drops ≈ 3.4 V at 5 V
            current = (vcc - outputVoltage) / rCollector
        }

        let logicValue = outputVoltage > vcc / 2.0 ? 1 : 0
        let power = vcc * current

        // TTL propagation delay ≈ 10 ns (much slower than CMOS)
        return GateOutput(
            logicValue: logicValue,
            voltage: outputVoltage,
            currentDraw: current,
            powerDissipation: power,
            propagationDelay: 10e-9,
            transistorCount: 4
        )
    }

    /// Digital evaluation: takes 0/1 inputs, returns 0/1 output.
    public func evaluateDigital(_ a: Int, _ b: Int) -> Int {
        let va = a == 1 ? vcc * 0.7 : vcc * 0.04  // 3.5 V = HIGH, 0.2 V = LOW
        let vb = b == 1 ? vcc * 0.7 : vcc * 0.04
        return evaluate(va: va, vb: vb).logicValue
    }

    /// Static power dissipation (W).
    ///
    /// TTL draws significant static current even when not switching —
    /// a major disadvantage vs. CMOS.
    public func staticPower() -> Double {
        // TTL 7400: ≈ 10 mW per gate at 5 V
        return vcc * (vcc / rBase + vcc / rCollector) * 0.5
    }
}

// MARK: - RTL Inverter

/// A Resistor-Transistor Logic (RTL) inverter.
///
/// The simplest possible digital inverter: one resistor and one NPN transistor.
/// Output = NOT(Input).
///
/// Used in the Apollo Guidance Computer (1966) — RTL ICs were the first
/// practical integrated circuit logic family.
///
/// # Circuit
///
///   Vcc
///    │
///   [Rc] 640Ω  ← collector resistor (pull-up)
///    │
///    └──── OUT
///    │
///   [NPN transistor]
///    │
///   [Rb] 450Ω  ← base resistor (input coupling)
///    │
///   IN
///
public struct RTLInverter {

    public let vcc: Double
    public let bjt: BJTParams
    public let rBase: Double      // base resistor (Ω)
    public let rCollector: Double // collector resistor (Ω)

    public init(
        vcc: Double = 3.6,        // Apollo AGC used 3.6 V supply
        bjt: BJTParams = .defaultNPN,
        rBase: Double = 450.0,
        rCollector: Double = 640.0
    ) {
        self.vcc = vcc
        self.bjt = bjt
        self.rBase = rBase
        self.rCollector = rCollector
    }

    /// Evaluates the RTL inverter at the given input voltage.
    public func evaluate(inputVoltage: Double) -> GateOutput {
        let npn = NPN(params: bjt)

        // Base current: Ib = (Vin − Vbe) / Rb
        let vbe = min(inputVoltage, bjt.vbeOn)
        let ib = max(0.0, (inputVoltage - vbe) / rBase)

        let outputVoltage: Double
        let current: Double

        if npn.isConducting(vbe: vbe) && ib > 0 {
            // Transistor ON → output pulled LOW through Vce(sat)
            outputVoltage = bjt.vceSat
            current = (vcc - outputVoltage) / rCollector
        } else {
            // Transistor OFF → output pulled HIGH through Rc
            outputVoltage = vcc - ib * rCollector
            current = ib
        }

        let logicValue = outputVoltage > vcc / 2.0 ? 1 : 0

        return GateOutput(
            logicValue: logicValue,
            voltage: outputVoltage,
            currentDraw: current,
            powerDissipation: vcc * current,
            propagationDelay: 20e-9,  // RTL is slower than TTL
            transistorCount: 1
        )
    }

    /// Digital evaluation: takes 0/1 input, returns 0/1 output.
    public func evaluateDigital(_ a: Int) -> Int {
        let vin = a == 1 ? vcc * 0.8 : 0.1
        return evaluate(inputVoltage: vin).logicValue
    }
}
