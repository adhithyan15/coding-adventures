/**
 * MOSFET Transistors — the building blocks of modern digital circuits.
 *
 * === What is a MOSFET? ===
 *
 * MOSFET stands for Metal-Oxide-Semiconductor Field-Effect Transistor. It is
 * the most common type of transistor in the world — every CPU, GPU, and phone
 * chip is built from billions of MOSFETs.
 *
 * A MOSFET has three terminals:
 *     Gate (G):   The control terminal. Voltage here controls the switch.
 *     Drain (D):  Current flows IN here (for NMOS) or OUT here (for PMOS).
 *     Source (S): Current flows OUT here (for NMOS) or IN here (for PMOS).
 *
 * The key insight: a MOSFET is VOLTAGE-controlled. Applying a voltage to the
 * gate creates an electric field that either allows or blocks current flow
 * between drain and source. No current flows into the gate itself (it's
 * insulated by a thin oxide layer), which means:
 *     - Near-zero input power consumption
 *     - Very high input impedance (good for amplifiers)
 *     - Can be packed extremely densely on a chip
 *
 * === NMOS vs PMOS ===
 *
 *     NMOS: Gate HIGH -> ON  (conducts drain to source)
 *     PMOS: Gate LOW  -> ON  (conducts source to drain)
 *
 * This complementary behavior is the foundation of CMOS (Complementary MOS)
 * logic. By pairing NMOS and PMOS transistors, we can build gates that consume
 * near-zero power in steady state.
 */

import {
  type MOSFETParams,
  MOSFETRegion,
  defaultMOSFETParams,
} from "./types.js";

/**
 * N-channel MOSFET transistor.
 *
 * An NMOS transistor conducts current from drain to source when the gate
 * voltage exceeds the threshold voltage (Vgs > Vth). Think of it as a
 * normally-OPEN switch that CLOSES when you apply voltage to the gate.
 *
 * === Water analogy ===
 *
 *     Imagine a water pipe with an electrically-controlled valve:
 *
 *         Water pressure (Vdd) --> [VALVE] --> Water out (Vss/ground)
 *                                    ^
 *                                Gate voltage
 *
 *     - Gate voltage HIGH: valve opens, water flows (current flows D->S)
 *     - Gate voltage LOW:  valve closed, water blocked (no current)
 *     - Gate voltage MEDIUM: valve partially open (analog amplifier mode)
 *
 * === In a digital circuit ===
 *
 *     When used as a digital switch, NMOS connects the output to GROUND:
 *
 *         Input HIGH -> NMOS ON -> output pulled to GND (LOW)
 *         Input LOW  -> NMOS OFF -> output disconnected from GND
 */
export class NMOS {
  readonly params: MOSFETParams;

  constructor(params?: Partial<MOSFETParams>) {
    this.params = { ...defaultMOSFETParams(), ...params };
  }

  /**
   * Determine the operating region given terminal voltages.
   *
   * For NMOS:
   *     Cutoff:     Vgs < Vth            (gate voltage below threshold)
   *     Linear:     Vgs >= Vth AND Vds < Vgs - Vth
   *     Saturation: Vgs >= Vth AND Vds >= Vgs - Vth
   *
   * @param vgs - Gate-to-Source voltage (V). Positive turns NMOS on.
   * @param vds - Drain-to-Source voltage (V). Positive for normal operation.
   * @returns MOSFETRegion enum value.
   */
  region(vgs: number, vds: number): MOSFETRegion {
    const vth = this.params.vth;

    if (vgs < vth) {
      return MOSFETRegion.CUTOFF;
    }

    const vov = vgs - vth; // Overdrive voltage
    if (vds < vov) {
      return MOSFETRegion.LINEAR;
    }
    return MOSFETRegion.SATURATION;
  }

  /**
   * Calculate drain-to-source current (Ids) in amperes.
   *
   * Uses the simplified MOSFET current equations (Shockley model):
   *
   *     Cutoff:     Ids = 0
   *     Linear:     Ids = k * ((Vgs - Vth) * Vds - 0.5 * Vds^2)
   *     Saturation: Ids = 0.5 * k * (Vgs - Vth)^2
   *
   * @param vgs - Gate-to-Source voltage (V).
   * @param vds - Drain-to-Source voltage (V).
   * @returns Drain current in amperes. Always >= 0 for NMOS.
   */
  drainCurrent(vgs: number, vds: number): number {
    const reg = this.region(vgs, vds);
    const k = this.params.k;
    const vth = this.params.vth;

    if (reg === MOSFETRegion.CUTOFF) {
      return 0.0;
    }

    const vov = vgs - vth; // Overdrive voltage

    if (reg === MOSFETRegion.LINEAR) {
      // Linear/ohmic region: Ids = k * ((Vgs-Vth)*Vds - 0.5*Vds^2)
      return k * (vov * vds - 0.5 * vds * vds);
    }

    // Saturation region: Ids = 0.5 * k * (Vgs-Vth)^2
    return 0.5 * k * vov * vov;
  }

  /**
   * Digital abstraction: is this transistor ON?
   *
   * Returns true when the gate voltage exceeds the threshold voltage.
   * This is the simplified view used in digital circuit analysis.
   *
   * @param vgs - Gate-to-Source voltage (V).
   * @returns True if Vgs >= Vth (transistor is ON).
   */
  isConducting(vgs: number): boolean {
    return vgs >= this.params.vth;
  }

  /**
   * Output voltage when used as a pull-down switch.
   *
   * In a CMOS circuit, NMOS transistors form the pull-down network.
   * When the NMOS is ON, it pulls the output to ~0V. When OFF, the
   * output floats at Vdd.
   *
   * @param vgs - Gate-to-Source voltage (V).
   * @param vdd - Supply voltage (V).
   * @returns Output voltage in volts.
   */
  outputVoltage(vgs: number, vdd: number): number {
    if (this.isConducting(vgs)) {
      return 0.0;
    }
    return vdd;
  }

  /**
   * Calculate small-signal transconductance gm.
   *
   * gm = dIds / dVgs = k * (Vgs - Vth) in saturation and linear.
   *
   * @param vgs - Gate-to-Source voltage (V).
   * @param vds - Drain-to-Source voltage (V).
   * @returns Transconductance in Siemens (A/V). Returns 0 in cutoff.
   */
  transconductance(vgs: number, vds: number): number {
    const reg = this.region(vgs, vds);
    if (reg === MOSFETRegion.CUTOFF) {
      return 0.0;
    }

    const vov = vgs - this.params.vth;
    return this.params.k * vov;
  }
}

/**
 * P-channel MOSFET transistor.
 *
 * A PMOS transistor is the complement of NMOS. It conducts current from
 * source to drain when the gate voltage is LOW (below the source voltage
 * by more than |Vth|). Think of it as a normally-CLOSED switch that OPENS
 * when you apply voltage.
 *
 * PMOS transistors form the pull-UP network in CMOS gates. When we need
 * to connect the output to Vdd (logic HIGH), PMOS transistors do the job.
 *
 * PMOS uses the same equations as NMOS, but with reversed voltage
 * polarities. For PMOS, Vgs and Vds are typically negative.
 */
export class PMOS {
  readonly params: MOSFETParams;

  constructor(params?: Partial<MOSFETParams>) {
    this.params = { ...defaultMOSFETParams(), ...params };
  }

  /**
   * Determine operating region for PMOS.
   *
   * For PMOS, we use the magnitudes of Vgs and Vds (which are typically
   * negative in a circuit). The regions are:
   *
   *     Cutoff:     |Vgs| < Vth
   *     Linear:     |Vgs| >= Vth AND |Vds| < |Vgs| - Vth
   *     Saturation: |Vgs| >= Vth AND |Vds| >= |Vgs| - Vth
   *
   * @param vgs - Gate-to-Source voltage (V). Typically negative for PMOS.
   * @param vds - Drain-to-Source voltage (V). Typically negative for PMOS.
   */
  region(vgs: number, vds: number): MOSFETRegion {
    const vth = this.params.vth;
    const absVgs = Math.abs(vgs);
    const absVds = Math.abs(vds);

    if (absVgs < vth) {
      return MOSFETRegion.CUTOFF;
    }

    const vov = absVgs - vth;
    if (absVds < vov) {
      return MOSFETRegion.LINEAR;
    }
    return MOSFETRegion.SATURATION;
  }

  /**
   * Calculate source-to-drain current for PMOS.
   *
   * Same equations as NMOS but using absolute values of voltages.
   * Current magnitude is returned (always >= 0).
   */
  drainCurrent(vgs: number, vds: number): number {
    const reg = this.region(vgs, vds);
    const k = this.params.k;
    const vth = this.params.vth;

    if (reg === MOSFETRegion.CUTOFF) {
      return 0.0;
    }

    const absVgs = Math.abs(vgs);
    const absVds = Math.abs(vds);
    const vov = absVgs - vth;

    if (reg === MOSFETRegion.LINEAR) {
      return k * (vov * absVds - 0.5 * absVds * absVds);
    }

    return 0.5 * k * vov * vov;
  }

  /**
   * Digital abstraction: is this PMOS transistor ON?
   *
   * PMOS turns ON when Vgs is sufficiently negative (gate pulled
   * below the source). Returns true when |Vgs| >= Vth.
   *
   * @param vgs - Gate-to-Source voltage (V). Typically negative for PMOS.
   */
  isConducting(vgs: number): boolean {
    return Math.abs(vgs) >= this.params.vth;
  }

  /**
   * Output voltage when used as a pull-up switch.
   *
   * PMOS forms the pull-up network in CMOS:
   *     ON:  output = Vdd
   *     OFF: output = 0V
   *
   * @param vgs - Gate-to-Source voltage (V).
   * @param vdd - Supply voltage (V).
   */
  outputVoltage(vgs: number, vdd: number): number {
    if (this.isConducting(vgs)) {
      return vdd;
    }
    return 0.0;
  }

  /**
   * Calculate small-signal transconductance gm for PMOS.
   *
   * Same formula as NMOS but using absolute values.
   */
  transconductance(vgs: number, vds: number): number {
    const reg = this.region(vgs, vds);
    if (reg === MOSFETRegion.CUTOFF) {
      return 0.0;
    }

    const vov = Math.abs(vgs) - this.params.vth;
    return this.params.k * vov;
  }
}
