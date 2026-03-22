/**
 * TTL Logic Gates — historical BJT-based digital logic.
 *
 * === What is TTL? ===
 *
 * TTL stands for Transistor-Transistor Logic. It was the dominant digital
 * logic family from the mid-1960s through the 1980s, when CMOS replaced it.
 *
 * === Why TTL Lost to CMOS ===
 *
 * TTL's fatal flaw: STATIC POWER CONSUMPTION.
 * A single TTL NAND gate dissipates ~1-10 mW at rest.
 *     1 million gates x 10 mW/gate = 10,000 watts!
 * CMOS gates consume near-zero power at rest.
 *
 * === RTL: The Predecessor to TTL ===
 *
 * Before TTL came RTL (Resistor-Transistor Logic). An RTL inverter is just
 * one transistor with two resistors. It was used in the Apollo Guidance
 * Computer that landed humans on the moon in 1969.
 */

import { NPN } from "./bjt.js";
import { type BJTParams, type GateOutput, defaultBJTParams } from "./types.js";

/**
 * Validate that a value is a binary digit (0 or 1).
 */
function validateBit(value: number, name: string = "input"): void {
  if (typeof value === "boolean") {
    throw new TypeError(`${name} must be a number, got boolean`);
  }
  if (typeof value !== "number" || !Number.isInteger(value)) {
    throw new TypeError(`${name} must be an integer, got ${typeof value}`);
  }
  if (value !== 0 && value !== 1) {
    throw new RangeError(`${name} must be 0 or 1, got ${value}`);
  }
}

/**
 * TTL NAND gate using NPN transistors (7400-series style).
 *
 * === Operation ===
 *
 * Any input LOW:
 *     Q1's base-emitter junction forward-biases through the LOW input,
 *     stealing current from Q2's base -> Q2 and Q3 turn OFF ->
 *     output pulled HIGH through pull-up resistor.
 *
 * ALL inputs HIGH:
 *     Q1's base-collector junction forward-biases, driving current
 *     into Q2's base -> Q2 saturates -> Q3 saturates ->
 *     output pulled LOW (Vce_sat ~ 0.2V).
 *
 * === The Problem: Static Power ===
 *
 * When Q3 is ON: current flows Vcc -> R1 -> Q1 -> Q2 -> Q3 -> GND.
 * This current flows CONTINUOUSLY, consuming ~1-10 mW per gate.
 */
export class TTLNand {
  readonly vcc: number;
  readonly params: BJTParams;
  readonly rPullup: number;
  readonly q1: NPN;
  readonly q2: NPN;
  readonly q3: NPN;

  constructor(vcc: number = 5.0, bjtParams?: Partial<BJTParams>) {
    this.vcc = vcc;
    this.params = { ...defaultBJTParams(), ...bjtParams };
    this.rPullup = 4000.0; // 4k ohm pull-up resistor
    this.q1 = new NPN(bjtParams);
    this.q2 = new NPN(bjtParams);
    this.q3 = new NPN(bjtParams);
  }

  /**
   * Evaluate the TTL NAND gate with analog input voltages.
   *
   * @param va - Input A voltage (V). LOW < 0.8V, HIGH > 2.0V.
   * @param vb - Input B voltage (V).
   * @returns GateOutput with voltage and power details.
   */
  evaluate(va: number, vb: number): GateOutput {
    const vcc = this.vcc;
    const vbeOn = this.params.vbeOn;

    // TTL input thresholds
    const aHigh = va > 2.0;
    const bHigh = vb > 2.0;

    let outputV: number;
    let logicValue: number;
    let current: number;

    if (aHigh && bHigh) {
      // ALL inputs HIGH -> output LOW
      outputV = this.params.vceSat; // ~0.2V
      logicValue = 0;

      // Static current through resistor chain
      current = (vcc - 2 * vbeOn - this.params.vceSat) / this.rPullup;
      current = Math.max(current, 0.0);
    } else {
      // At least one input LOW -> output HIGH
      outputV = vcc - vbeOn; // ~4.3V
      logicValue = 1;

      // Small bias current through pull-up
      current = (vcc - outputV) / this.rPullup;
      current = Math.max(current, 0.0);
    }

    const power = current * vcc;

    // TTL propagation delay: typically 5-15 ns
    const delay = 10e-9; // 10 ns typical

    return {
      logicValue,
      voltage: outputV,
      currentDraw: current,
      powerDissipation: power,
      propagationDelay: delay,
      transistorCount: 3, // Simplified: Q1 + Q2 + Q3
    };
  }

  /** Evaluate with digital inputs (0 or 1). */
  evaluateDigital(a: number, b: number): number {
    validateBit(a, "a");
    validateBit(b, "b");
    const va = a === 1 ? this.vcc : 0.0;
    const vb = b === 1 ? this.vcc : 0.0;
    return this.evaluate(va, vb).logicValue;
  }

  /**
   * Static power dissipation — significantly higher than CMOS.
   *
   * TTL gates consume power continuously due to resistor-based biasing.
   * The worst case is when the output is LOW (all inputs HIGH).
   *
   * @returns Static power in watts. Typically ~1-10 mW for a single gate.
   */
  get staticPower(): number {
    const current =
      (this.vcc - 2 * this.params.vbeOn - this.params.vceSat) / this.rPullup;
    return Math.max(current, 0.0) * this.vcc;
  }
}

/**
 * Resistor-Transistor Logic inverter — the earliest IC logic family.
 *
 * === Operation ===
 *
 * Input HIGH (Vcc):
 *     Current flows through Rb into Q1's base -> Q1 saturates ->
 *     output pulled LOW through Q1 (Vce_sat ~ 0.2V).
 *
 * Input LOW (0V):
 *     No base current -> Q1 in cutoff -> output pulled HIGH
 *     through Rc to Vcc.
 *
 * === Historical Note ===
 *
 * RTL was used in the Apollo Guidance Computer (AGC), which navigated
 * Apollo 11 to the moon in 1969.
 */
export class RTLInverter {
  readonly vcc: number;
  readonly rBase: number;
  readonly rCollector: number;
  readonly params: BJTParams;
  readonly q1: NPN;

  constructor(
    vcc: number = 5.0,
    rBase: number = 10_000.0,
    rCollector: number = 1_000.0,
    bjtParams?: Partial<BJTParams>,
  ) {
    this.vcc = vcc;
    this.rBase = rBase;
    this.rCollector = rCollector;
    this.params = { ...defaultBJTParams(), ...bjtParams };
    this.q1 = new NPN(bjtParams);
  }

  /** Evaluate the RTL inverter with an analog input voltage. */
  evaluate(vInput: number): GateOutput {
    const vcc = this.vcc;
    const vbeOn = this.params.vbeOn;

    let outputV: number;
    let logicValue: number;
    let current: number;

    if (vInput > vbeOn) {
      // Q1 is ON
      const ib = (vInput - vbeOn) / this.rBase;
      const ic = Math.min(
        ib * this.params.beta,
        (vcc - this.params.vceSat) / this.rCollector,
      );
      outputV = vcc - ic * this.rCollector;
      outputV = Math.max(outputV, this.params.vceSat);
      logicValue = outputV < vcc / 2.0 ? 0 : 1;
      current = ic + ib;
    } else {
      // Q1 is OFF — output pulled to Vcc through Rc
      outputV = vcc;
      logicValue = 1;
      current = 0.0;
    }

    const power = current * vcc;
    const delay = 50e-9; // RTL is slow: ~50 ns typical

    return {
      logicValue,
      voltage: outputV,
      currentDraw: current,
      powerDissipation: power,
      propagationDelay: delay,
      transistorCount: 1,
    };
  }

  /** Evaluate with digital input (0 or 1). */
  evaluateDigital(a: number): number {
    validateBit(a, "a");
    const vInput = a === 1 ? this.vcc : 0.0;
    return this.evaluate(vInput).logicValue;
  }
}
