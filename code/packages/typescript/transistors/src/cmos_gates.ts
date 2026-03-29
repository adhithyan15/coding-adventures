/**
 * CMOS Logic Gates — building digital logic from transistor pairs.
 *
 * === What is CMOS? ===
 *
 * CMOS stands for Complementary Metal-Oxide-Semiconductor. It is the
 * technology used in virtually every digital chip made since the 1980s.
 *
 * The "complementary" refers to pairing NMOS and PMOS transistors:
 *     - PMOS transistors form the PULL-UP network (connects output to Vdd)
 *     - NMOS transistors form the PULL-DOWN network (connects output to GND)
 *
 * For any valid input combination, exactly ONE network is active:
 *     - Pull-up ON -> output = Vdd (logic HIGH)
 *     - Pull-down ON -> output = GND (logic LOW)
 *     - Never both ON simultaneously -> no DC current -> near-zero static power
 *
 * === Transistor Counts ===
 *
 *     Gate    | NMOS | PMOS | Total | Notes
 *     --------|------|------|-------|------
 *     NOT     |  1   |  1   |   2   | The simplest CMOS circuit
 *     NAND    |  2   |  2   |   4   | Natural CMOS gate
 *     NOR     |  2   |  2   |   4   | Natural CMOS gate
 *     AND     |  3   |  3   |   6   | NAND + NOT
 *     OR      |  3   |  3   |   6   | NOR + NOT
 *     XOR     |  3   |  3   |   6   | Transmission gate design
 */

import { NMOS, PMOS } from "./mosfet.js";
import {
  type CircuitParams,
  type GateOutput,
  type MOSFETParams,
  defaultCircuitParams,
} from "./types.js";

/**
 * Validate that a value is a binary digit (0 or 1).
 *
 * We enforce strict validation: reject booleans, floats, and out-of-range integers.
 *
 * @param value - The value to validate.
 * @param name - Name of the input for error messages.
 * @throws TypeError if value is not an integer.
 * @throws RangeError if value is not 0 or 1.
 */
export function validateBit(value: number, name: string = "input"): void {
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
 * CMOS NOT gate: 1 PMOS + 1 NMOS = 2 transistors.
 *
 * The simplest and most important CMOS circuit. Every other CMOS gate
 * is a variation of this fundamental pattern.
 *
 * === How it works ===
 *
 *     Input A = HIGH (Vdd):
 *         NMOS: Vgs = Vdd > Vth -> ON -> pulls output to GND
 *         PMOS: Vgs = 0 -> OFF -> disconnected from Vdd
 *         Output = LOW (GND) = NOT HIGH
 *
 *     Input A = LOW (0V):
 *         NMOS: Vgs = 0 < Vth -> OFF -> disconnected from GND
 *         PMOS: Vgs = -Vdd -> ON -> pulls output to Vdd
 *         Output = HIGH (Vdd) = NOT LOW
 *
 * Static power: ZERO. In both states, one transistor is OFF, breaking
 * the current path from Vdd to GND.
 */
export class CMOSInverter {
  static readonly TRANSISTOR_COUNT = 2;

  readonly circuit: CircuitParams;
  readonly nmos: NMOS;
  readonly pmos: PMOS;

  constructor(
    circuitParams?: Partial<CircuitParams>,
    nmosParams?: Partial<MOSFETParams>,
    pmosParams?: Partial<MOSFETParams>,
  ) {
    this.circuit = { ...defaultCircuitParams(), ...circuitParams };
    this.nmos = new NMOS(nmosParams);
    this.pmos = new PMOS(pmosParams);
  }

  /**
   * Evaluate the inverter with an analog input voltage.
   *
   * Maps the input voltage through the CMOS transfer characteristic
   * to produce an output voltage.
   *
   * @param inputVoltage - Input voltage in volts (0 to Vdd).
   * @returns GateOutput with voltage, current, power, and timing details.
   */
  evaluate(inputVoltage: number): GateOutput {
    const vdd = this.circuit.vdd;

    // NMOS: gate = input, source = GND -> Vgs_n = Vin
    const vgsN = inputVoltage;
    // PMOS: gate = input, source = Vdd -> Vgs_p = Vin - Vdd
    const vgsP = inputVoltage - vdd;

    const nmosOn = this.nmos.isConducting(vgsN);
    const pmosOn = this.pmos.isConducting(vgsP);

    // Determine output voltage
    let outputV: number;
    if (pmosOn && !nmosOn) {
      outputV = vdd; // PMOS pulls to Vdd
    } else if (nmosOn && !pmosOn) {
      outputV = 0.0; // NMOS pulls to GND
    } else if (nmosOn && pmosOn) {
      // Both on (transition region) — voltage divider
      outputV = vdd / 2.0;
    } else {
      // Both off (shouldn't happen in normal operation)
      outputV = vdd / 2.0;
    }

    // Digital interpretation
    const logicValue = outputV > vdd / 2.0 ? 1 : 0;

    // Current draw: only significant during transition
    let current: number;
    if (nmosOn && pmosOn) {
      // Short-circuit current during transition
      const vdsN = vdd / 2.0;
      current = this.nmos.drainCurrent(vgsN, vdsN);
    } else {
      current = 0.0; // Static: no current path
    }

    const power = current * vdd;

    // Propagation delay estimate
    const cLoad = this.nmos.params.cDrain + this.pmos.params.cDrain;
    let delay: number;
    if (current > 0) {
      delay = (cLoad * vdd) / (2.0 * current);
    } else {
      const idsSat = this.nmos.drainCurrent(vdd, vdd);
      delay = idsSat > 0 ? (cLoad * vdd) / (2.0 * idsSat) : 1e-9;
    }

    return {
      logicValue,
      voltage: outputV,
      currentDraw: current,
      powerDissipation: power,
      propagationDelay: delay,
      transistorCount: CMOSInverter.TRANSISTOR_COUNT,
    };
  }

  /**
   * Evaluate with digital input (0 or 1), returns 0 or 1.
   *
   * Convenience method that maps 0 -> 0V, 1 -> Vdd.
   */
  evaluateDigital(a: number): number {
    validateBit(a, "a");
    const vin = a === 1 ? this.circuit.vdd : 0.0;
    return this.evaluate(vin).logicValue;
  }

  /**
   * Generate the VTC curve: list of [Vin, Vout] points.
   *
   * The VTC shows the sharp switching threshold of CMOS — the output
   * snaps from HIGH to LOW over a very narrow input range.
   */
  voltageTranferCharacteristic(steps: number = 100): [number, number][] {
    const vdd = this.circuit.vdd;
    const points: [number, number][] = [];
    for (let i = 0; i <= steps; i++) {
      const vin = (vdd * i) / steps;
      const result = this.evaluate(vin);
      points.push([vin, result.voltage]);
    }
    return points;
  }

  /**
   * Static power dissipation (ideally ~0 for CMOS).
   */
  get staticPower(): number {
    return 0.0;
  }

  /**
   * Dynamic power: P = C_load * Vdd^2 * f.
   *
   * @param frequency - Switching frequency in Hz.
   * @param cLoad - Load capacitance in Farads.
   * @returns Dynamic power in Watts.
   */
  dynamicPower(frequency: number, cLoad: number): number {
    const vdd = this.circuit.vdd;
    return cLoad * vdd * vdd * frequency;
  }
}

/**
 * CMOS NAND gate: 2 PMOS parallel + 2 NMOS series = 4 transistors.
 *
 * NAND is a "natural" CMOS gate — it requires only 4 transistors.
 * AND requires 6 (NAND + inverter). This is because the CMOS structure
 * naturally produces an inverted output.
 *
 * Pull-down: NMOS in SERIES — BOTH must be ON to pull low.
 * Pull-up: PMOS in PARALLEL — EITHER can pull up.
 */
export class CMOSNand {
  static readonly TRANSISTOR_COUNT = 4;

  readonly circuit: CircuitParams;
  readonly nmos1: NMOS;
  readonly nmos2: NMOS;
  readonly pmos1: PMOS;
  readonly pmos2: PMOS;

  constructor(
    circuitParams?: Partial<CircuitParams>,
    nmosParams?: Partial<MOSFETParams>,
    pmosParams?: Partial<MOSFETParams>,
  ) {
    this.circuit = { ...defaultCircuitParams(), ...circuitParams };
    this.nmos1 = new NMOS(nmosParams);
    this.nmos2 = new NMOS(nmosParams);
    this.pmos1 = new PMOS(pmosParams);
    this.pmos2 = new PMOS(pmosParams);
  }

  /** Evaluate the NAND gate with analog input voltages. */
  evaluate(va: number, vb: number): GateOutput {
    const vdd = this.circuit.vdd;

    const vgsN1 = va;
    const vgsN2 = vb;
    const vgsP1 = va - vdd;
    const vgsP2 = vb - vdd;

    const nmos1On = this.nmos1.isConducting(vgsN1);
    const nmos2On = this.nmos2.isConducting(vgsN2);
    const pmos1On = this.pmos1.isConducting(vgsP1);
    const pmos2On = this.pmos2.isConducting(vgsP2);

    // Pull-down: NMOS in SERIES — BOTH must be ON
    const pulldownOn = nmos1On && nmos2On;
    // Pull-up: PMOS in PARALLEL — EITHER can pull up
    const pullupOn = pmos1On || pmos2On;

    let outputV: number;
    if (pullupOn && !pulldownOn) {
      outputV = vdd;
    } else if (pulldownOn && !pullupOn) {
      outputV = 0.0;
    } else {
      outputV = vdd / 2.0;
    }

    const logicValue = outputV > vdd / 2.0 ? 1 : 0;
    const current = pulldownOn && pullupOn ? 0.001 : 0.0;

    const cLoad = this.nmos1.params.cDrain + this.pmos1.params.cDrain;
    const idsSat = this.nmos1.drainCurrent(vdd, vdd);
    const delay = idsSat > 0 ? (cLoad * vdd) / (2.0 * idsSat) : 1e-9;

    return {
      logicValue,
      voltage: outputV,
      currentDraw: current,
      powerDissipation: current * vdd,
      propagationDelay: delay,
      transistorCount: CMOSNand.TRANSISTOR_COUNT,
    };
  }

  /** Evaluate with digital inputs (0 or 1). */
  evaluateDigital(a: number, b: number): number {
    validateBit(a, "a");
    validateBit(b, "b");
    const vdd = this.circuit.vdd;
    const va = a === 1 ? vdd : 0.0;
    const vb = b === 1 ? vdd : 0.0;
    return this.evaluate(va, vb).logicValue;
  }

  /** Returns 4. */
  get transistorCount(): number {
    return CMOSNand.TRANSISTOR_COUNT;
  }
}

/**
 * CMOS NOR gate: 2 PMOS series + 2 NMOS parallel = 4 transistors.
 *
 * Pull-down: NMOS in PARALLEL — EITHER ON pulls low.
 * Pull-up: PMOS in SERIES — BOTH must be ON.
 */
export class CMOSNor {
  static readonly TRANSISTOR_COUNT = 4;

  readonly circuit: CircuitParams;
  readonly nmos1: NMOS;
  readonly nmos2: NMOS;
  readonly pmos1: PMOS;
  readonly pmos2: PMOS;

  constructor(
    circuitParams?: Partial<CircuitParams>,
    nmosParams?: Partial<MOSFETParams>,
    pmosParams?: Partial<MOSFETParams>,
  ) {
    this.circuit = { ...defaultCircuitParams(), ...circuitParams };
    this.nmos1 = new NMOS(nmosParams);
    this.nmos2 = new NMOS(nmosParams);
    this.pmos1 = new PMOS(pmosParams);
    this.pmos2 = new PMOS(pmosParams);
  }

  /** Evaluate the NOR gate with analog input voltages. */
  evaluate(va: number, vb: number): GateOutput {
    const vdd = this.circuit.vdd;

    const vgsN1 = va;
    const vgsN2 = vb;
    const vgsP1 = va - vdd;
    const vgsP2 = vb - vdd;

    const nmos1On = this.nmos1.isConducting(vgsN1);
    const nmos2On = this.nmos2.isConducting(vgsN2);
    const pmos1On = this.pmos1.isConducting(vgsP1);
    const pmos2On = this.pmos2.isConducting(vgsP2);

    // Pull-down: NMOS in PARALLEL — EITHER ON pulls low
    const pulldownOn = nmos1On || nmos2On;
    // Pull-up: PMOS in SERIES — BOTH must be ON
    const pullupOn = pmos1On && pmos2On;

    let outputV: number;
    if (pullupOn && !pulldownOn) {
      outputV = vdd;
    } else if (pulldownOn && !pullupOn) {
      outputV = 0.0;
    } else {
      outputV = vdd / 2.0;
    }

    const logicValue = outputV > vdd / 2.0 ? 1 : 0;
    const current = pulldownOn && pullupOn ? 0.001 : 0.0;

    const cLoad = this.nmos1.params.cDrain + this.pmos1.params.cDrain;
    const idsSat = this.nmos1.drainCurrent(vdd, vdd);
    const delay = idsSat > 0 ? (cLoad * vdd) / (2.0 * idsSat) : 1e-9;

    return {
      logicValue,
      voltage: outputV,
      currentDraw: current,
      powerDissipation: current * vdd,
      propagationDelay: delay,
      transistorCount: CMOSNor.TRANSISTOR_COUNT,
    };
  }

  /** Evaluate with digital inputs (0 or 1). */
  evaluateDigital(a: number, b: number): number {
    validateBit(a, "a");
    validateBit(b, "b");
    const vdd = this.circuit.vdd;
    const va = a === 1 ? vdd : 0.0;
    const vb = b === 1 ? vdd : 0.0;
    return this.evaluate(va, vb).logicValue;
  }
}

/**
 * CMOS AND gate: NAND + Inverter = 6 transistors.
 *
 * There is no "direct" CMOS AND gate. The CMOS topology naturally
 * produces inverted outputs, so to get AND we must add an inverter
 * after the NAND.
 */
export class CMOSAnd {
  static readonly TRANSISTOR_COUNT = 6;

  readonly circuit: CircuitParams;
  private readonly _nand: CMOSNand;
  private readonly _inv: CMOSInverter;

  constructor(circuitParams?: Partial<CircuitParams>) {
    this.circuit = { ...defaultCircuitParams(), ...circuitParams };
    this._nand = new CMOSNand(circuitParams);
    this._inv = new CMOSInverter(circuitParams);
  }

  /** AND = NOT(NAND(A, B)). */
  evaluate(va: number, vb: number): GateOutput {
    const nandOut = this._nand.evaluate(va, vb);
    const invOut = this._inv.evaluate(nandOut.voltage);
    return {
      logicValue: invOut.logicValue,
      voltage: invOut.voltage,
      currentDraw: nandOut.currentDraw + invOut.currentDraw,
      powerDissipation: nandOut.powerDissipation + invOut.powerDissipation,
      propagationDelay: nandOut.propagationDelay + invOut.propagationDelay,
      transistorCount: CMOSAnd.TRANSISTOR_COUNT,
    };
  }

  /** Evaluate with digital inputs. */
  evaluateDigital(a: number, b: number): number {
    validateBit(a, "a");
    validateBit(b, "b");
    const vdd = this.circuit.vdd;
    const va = a === 1 ? vdd : 0.0;
    const vb = b === 1 ? vdd : 0.0;
    return this.evaluate(va, vb).logicValue;
  }
}

/**
 * CMOS OR gate: NOR + Inverter = 6 transistors.
 */
export class CMOSOr {
  static readonly TRANSISTOR_COUNT = 6;

  readonly circuit: CircuitParams;
  private readonly _nor: CMOSNor;
  private readonly _inv: CMOSInverter;

  constructor(circuitParams?: Partial<CircuitParams>) {
    this.circuit = { ...defaultCircuitParams(), ...circuitParams };
    this._nor = new CMOSNor(circuitParams);
    this._inv = new CMOSInverter(circuitParams);
  }

  /** OR = NOT(NOR(A, B)). */
  evaluate(va: number, vb: number): GateOutput {
    const norOut = this._nor.evaluate(va, vb);
    const invOut = this._inv.evaluate(norOut.voltage);
    return {
      logicValue: invOut.logicValue,
      voltage: invOut.voltage,
      currentDraw: norOut.currentDraw + invOut.currentDraw,
      powerDissipation: norOut.powerDissipation + invOut.powerDissipation,
      propagationDelay: norOut.propagationDelay + invOut.propagationDelay,
      transistorCount: CMOSOr.TRANSISTOR_COUNT,
    };
  }

  /** Evaluate with digital inputs. */
  evaluateDigital(a: number, b: number): number {
    validateBit(a, "a");
    validateBit(b, "b");
    const vdd = this.circuit.vdd;
    const va = a === 1 ? vdd : 0.0;
    const vb = b === 1 ? vdd : 0.0;
    return this.evaluate(va, vb).logicValue;
  }
}

/**
 * CMOS XOR gate using NAND-based construction = 6 transistors.
 *
 * XOR from 4 NANDs:
 *     XOR(A, B) = NAND(NAND(A, NAND(A,B)), NAND(B, NAND(A,B)))
 *
 * This construction proves that XOR can be built from the universal
 * NAND gate alone.
 */
export class CMOSXor {
  static readonly TRANSISTOR_COUNT = 6;

  readonly circuit: CircuitParams;
  private readonly _nand1: CMOSNand;
  private readonly _nand2: CMOSNand;
  private readonly _nand3: CMOSNand;
  private readonly _nand4: CMOSNand;

  constructor(circuitParams?: Partial<CircuitParams>) {
    this.circuit = { ...defaultCircuitParams(), ...circuitParams };
    this._nand1 = new CMOSNand(circuitParams);
    this._nand2 = new CMOSNand(circuitParams);
    this._nand3 = new CMOSNand(circuitParams);
    this._nand4 = new CMOSNand(circuitParams);
  }

  /** XOR using 4 NAND gates. */
  evaluate(va: number, vb: number): GateOutput {
    const vdd = this.circuit.vdd;

    // Step 1: NAND(A, B)
    const nandAb = this._nand1.evaluate(va, vb);
    // Step 2: NAND(A, NAND(A,B))
    const nandANab = this._nand2.evaluate(va, nandAb.voltage);
    // Step 3: NAND(B, NAND(A,B))
    const nandBNab = this._nand3.evaluate(vb, nandAb.voltage);
    // Step 4: NAND(step2, step3)
    const result = this._nand4.evaluate(nandANab.voltage, nandBNab.voltage);

    const totalCurrent =
      nandAb.currentDraw +
      nandANab.currentDraw +
      nandBNab.currentDraw +
      result.currentDraw;
    const totalDelay =
      nandAb.propagationDelay +
      Math.max(nandANab.propagationDelay, nandBNab.propagationDelay) +
      result.propagationDelay;

    return {
      logicValue: result.logicValue,
      voltage: result.voltage,
      currentDraw: totalCurrent,
      powerDissipation: totalCurrent * vdd,
      propagationDelay: totalDelay,
      transistorCount: CMOSXor.TRANSISTOR_COUNT,
    };
  }

  /** Evaluate with digital inputs. */
  evaluateDigital(a: number, b: number): number {
    validateBit(a, "a");
    validateBit(b, "b");
    const vdd = this.circuit.vdd;
    const va = a === 1 ? vdd : 0.0;
    const vb = b === 1 ? vdd : 0.0;
    return this.evaluate(va, vb).logicValue;
  }

  /**
   * Build XOR from 4 NAND gates to demonstrate universality.
   *
   * Same as evaluateDigital but makes the NAND construction explicit.
   */
  evaluateFromNands(a: number, b: number): number {
    return this.evaluateDigital(a, b);
  }
}

/**
 * CMOS XNOR gate: XOR + Inverter = 8 transistors.
 *
 * XNOR(A, B) = NOT(XOR(A, B))
 *
 * XNOR is the "equivalence" gate — it outputs 1 when both inputs are the
 * same value (both 0 or both 1), and 0 when they differ.
 *
 * Truth table:
 *
 *     A | B | XNOR
 *     --|---|-----
 *     0 | 0 |  1    (same)
 *     0 | 1 |  0    (different)
 *     1 | 0 |  0    (different)
 *     1 | 1 |  1    (same)
 *
 * Transistor count: 8 (6 for XOR + 2 for Inverter).
 *
 * Use case: equality comparators — if XNOR(a, b) = 1 then a === b.
 */
export class CMOSXnor {
  static readonly TRANSISTOR_COUNT = 8;

  readonly circuit: CircuitParams;
  private readonly _xorGate: CMOSXor;
  private readonly _inv: CMOSInverter;

  constructor(circuitParams?: Partial<CircuitParams>) {
    this.circuit = { ...defaultCircuitParams(), ...circuitParams };
    this._xorGate = new CMOSXor(circuitParams);
    this._inv = new CMOSInverter(circuitParams);
  }

  /** XNOR = NOT(XOR(A, B)). */
  evaluate(va: number, vb: number): GateOutput {
    const xorOut = this._xorGate.evaluate(va, vb);
    const invOut = this._inv.evaluate(xorOut.voltage);
    return {
      logicValue: invOut.logicValue,
      voltage: invOut.voltage,
      currentDraw: xorOut.currentDraw + invOut.currentDraw,
      powerDissipation:
        (xorOut.currentDraw + invOut.currentDraw) * this.circuit.vdd,
      propagationDelay: xorOut.propagationDelay + invOut.propagationDelay,
      transistorCount: CMOSXnor.TRANSISTOR_COUNT,
    };
  }

  /** Evaluate with digital inputs. */
  evaluateDigital(a: number, b: number): number {
    validateBit(a, "a");
    validateBit(b, "b");
    const vdd = this.circuit.vdd;
    const va = a === 1 ? vdd : 0.0;
    const vb = b === 1 ? vdd : 0.0;
    return this.evaluate(va, vb).logicValue;
  }
}
