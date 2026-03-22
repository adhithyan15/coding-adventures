/**
 * BJT Transistors — the original solid-state amplifier.
 *
 * === What is a BJT? ===
 *
 * BJT stands for Bipolar Junction Transistor. Invented in 1947 at Bell Labs
 * by John Bardeen, Walter Brattain, and William Shockley, the BJT replaced
 * vacuum tubes and launched the electronics revolution.
 *
 * A BJT has three terminals:
 *     Base (B):      The control terminal. Current here controls the switch.
 *     Collector (C): Current flows IN here (for NPN) or OUT here (for PNP).
 *     Emitter (E):   Current flows OUT here (for NPN) or IN here (for PNP).
 *
 * The key difference from MOSFETs: a BJT is CURRENT-controlled. You must
 * supply a continuous current to the base to keep it on. This means:
 *     - Base current = wasted power (even in steady state)
 *     - Lower input impedance than MOSFETs
 *     - But historically faster switching (before CMOS caught up)
 *
 * === The Current Gain (beta) ===
 *
 *     Ic = beta * Ib
 *
 * A tiny base current (microamps) controls a much larger collector current
 * (milliamps). This amplification made radios, televisions, and early
 * computers possible.
 *
 * === Why CMOS Replaced BJT for Digital Logic ===
 *
 * In TTL: ~1-10 mW per gate static power.
 * In CMOS: ~nanowatts per gate static power.
 * This power advantage is why CMOS completely replaced BJT for digital logic.
 */

import { type BJTParams, BJTRegion, defaultBJTParams } from "./types.js";

/**
 * NPN bipolar junction transistor.
 *
 * An NPN transistor turns ON when current flows into the base terminal
 * (Vbe > ~0.7V). A small base current controls a much larger collector
 * current through: Ic = beta * Ib.
 *
 * === Operating regions ===
 *
 *     CUTOFF:      Vbe < 0.7V -> no base current -> no collector current.
 *     ACTIVE:      Vbe >= 0.7V, Vce > 0.2V -> Ic = beta * Ib (amplifier).
 *     SATURATION:  Vbe >= 0.7V, Vce <= 0.2V -> transistor fully ON (switch).
 */
export class NPN {
  readonly params: BJTParams;

  constructor(params?: Partial<BJTParams>) {
    this.params = { ...defaultBJTParams(), ...params };
  }

  /**
   * Determine the operating region from terminal voltages.
   *
   * @param vbe - Base-to-Emitter voltage (V). Must exceed ~0.7V to turn on.
   * @param vce - Collector-to-Emitter voltage (V). Determines active vs saturated.
   * @returns BJTRegion enum value.
   */
  region(vbe: number, vce: number): BJTRegion {
    if (vbe < this.params.vbeOn) {
      return BJTRegion.CUTOFF;
    }

    if (vce <= this.params.vceSat) {
      return BJTRegion.SATURATION;
    }

    return BJTRegion.ACTIVE;
  }

  /**
   * Calculate collector current (Ic) in amperes.
   *
   * Uses the simplified Ebers-Moll model:
   *     Cutoff:     Ic = 0
   *     Active:     Ic = Is * (exp(Vbe/Vt) - 1)
   *     Saturation: Ic = Is * (exp(Vbe/Vt) - 1) (same formula, limited by external circuit)
   *
   * @param vbe - Base-to-Emitter voltage (V).
   * @param vce - Collector-to-Emitter voltage (V).
   * @returns Collector current in amperes.
   */
  collectorCurrent(vbe: number, vce: number): number {
    const reg = this.region(vbe, vce);

    if (reg === BJTRegion.CUTOFF) {
      return 0.0;
    }

    // Thermal voltage: Vt = kT/q ~ 26mV at room temperature
    const vt = 0.026;
    const exponent = Math.min(vbe / vt, 40.0); // Clamp to prevent overflow
    return this.params.is * (Math.exp(exponent) - 1.0);
  }

  /**
   * Calculate base current (Ib) in amperes.
   *
   * Ib = Ic / beta in the active region.
   * This is the "wasted" current that makes BJTs less efficient than MOSFETs.
   *
   * @param vbe - Base-to-Emitter voltage (V).
   * @param vce - Collector-to-Emitter voltage (V).
   * @returns Base current in amperes.
   */
  baseCurrent(vbe: number, vce: number): number {
    const ic = this.collectorCurrent(vbe, vce);
    if (ic === 0.0) {
      return 0.0;
    }
    return ic / this.params.beta;
  }

  /**
   * Digital abstraction: is this transistor ON?
   *
   * Returns true when Vbe >= Vbe_on (typically 0.7V).
   */
  isConducting(vbe: number): boolean {
    return vbe >= this.params.vbeOn;
  }

  /**
   * Calculate small-signal transconductance gm.
   *
   * For a BJT in the active region: gm = Ic / Vt.
   *
   * @param vbe - Base-to-Emitter voltage (V).
   * @param vce - Collector-to-Emitter voltage (V).
   * @returns Transconductance in Siemens (A/V).
   */
  transconductance(vbe: number, vce: number): number {
    const ic = this.collectorCurrent(vbe, vce);
    if (ic === 0.0) {
      return 0.0;
    }
    const vt = 0.026;
    return ic / vt;
  }
}

/**
 * PNP bipolar junction transistor.
 *
 * The complement of NPN. A PNP transistor turns ON when the base is
 * pulled LOW relative to the emitter (|Vbe| >= Vbe_on).
 * Current flows from emitter to collector.
 *
 * We use absolute values internally, same as PMOS.
 */
export class PNP {
  readonly params: BJTParams;

  constructor(params?: Partial<BJTParams>) {
    this.params = { ...defaultBJTParams(), ...params };
  }

  /**
   * Determine operating region for PNP.
   *
   * Uses absolute values of Vbe and Vce since PNP operates with
   * reversed polarities.
   */
  region(vbe: number, vce: number): BJTRegion {
    const absVbe = Math.abs(vbe);
    const absVce = Math.abs(vce);

    if (absVbe < this.params.vbeOn) {
      return BJTRegion.CUTOFF;
    }

    if (absVce <= this.params.vceSat) {
      return BJTRegion.SATURATION;
    }

    return BJTRegion.ACTIVE;
  }

  /**
   * Calculate collector current magnitude for PNP.
   *
   * Same equations as NPN but using absolute values.
   * Returns current magnitude (always >= 0).
   */
  collectorCurrent(vbe: number, vce: number): number {
    const reg = this.region(vbe, vce);

    if (reg === BJTRegion.CUTOFF) {
      return 0.0;
    }

    const absVbe = Math.abs(vbe);
    const vt = 0.026;

    const exponent = Math.min(absVbe / vt, 40.0);
    return this.params.is * (Math.exp(exponent) - 1.0);
  }

  /** Calculate base current magnitude for PNP. */
  baseCurrent(vbe: number, vce: number): number {
    const ic = this.collectorCurrent(vbe, vce);
    if (ic === 0.0) {
      return 0.0;
    }
    return ic / this.params.beta;
  }

  /**
   * Digital abstraction: is this PNP transistor ON?
   *
   * PNP turns ON when |Vbe| >= Vbe_on.
   */
  isConducting(vbe: number): boolean {
    return Math.abs(vbe) >= this.params.vbeOn;
  }

  /** Calculate small-signal transconductance gm for PNP. */
  transconductance(vbe: number, vce: number): number {
    const ic = this.collectorCurrent(vbe, vce);
    if (ic === 0.0) {
      return 0.0;
    }
    const vt = 0.026;
    return ic / vt;
  }
}
