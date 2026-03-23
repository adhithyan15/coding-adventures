/**
 * Electrical Analysis — noise margins, power, timing, and technology comparison.
 *
 * === Why Electrical Analysis Matters ===
 *
 * Digital logic designers don't just care about truth tables — they care about:
 *
 * 1. NOISE MARGINS: Can the circuit tolerate voltage fluctuations?
 * 2. POWER: How much energy does the chip consume?
 * 3. TIMING: How fast can the circuit switch?
 * 4. SCALING: How do properties change as we shrink transistors?
 */

import { CMOSInverter, CMOSNand, CMOSNor } from "./cmos_gates.js";
import { TTLNand } from "./ttl_gates.js";
import type {
  NoiseMargins,
  PowerAnalysis,
  TimingAnalysis,
} from "./types.js";

/**
 * Analyze noise margins for a gate.
 *
 * Noise margins tell you how much electrical noise a digital signal
 * can tolerate before being misinterpreted by the next gate in the chain.
 *
 * For CMOS: VOL ~ 0V, VOH ~ Vdd -> large noise margins
 * For TTL: VOL ~ 0.2V, VOH ~ 3.5V -> smaller margins
 *
 * @param gate - CMOSInverter or TTLNand gate to analyze.
 * @returns NoiseMargins with vol, voh, vil, vih, nml, nmh.
 */
export function computeNoiseMargins(
  gate: CMOSInverter | TTLNand,
): NoiseMargins {
  let vol: number;
  let voh: number;
  let vil: number;
  let vih: number;

  if (gate instanceof CMOSInverter) {
    const vdd = gate.circuit.vdd;
    // CMOS has nearly ideal rail-to-rail output
    vol = 0.0;
    voh = vdd;
    // Input thresholds at ~40% and ~60% of Vdd (symmetric CMOS)
    vil = 0.4 * vdd;
    vih = 0.6 * vdd;
  } else if (gate instanceof TTLNand) {
    // TTL specifications (standard 74xx series)
    vol = 0.2; // Vce_sat of output transistor
    voh = gate.vcc - 0.7; // Vcc minus one diode drop
    vil = 0.8; // Standard TTL input LOW threshold
    vih = 2.0; // Standard TTL input HIGH threshold
  } else {
    throw new TypeError(`Unsupported gate type`);
  }

  const nml = vil - vol;
  const nmh = voh - vih;

  return { vol, voh, vil, vih, nml, nmh };
}

/**
 * Compute power consumption for a gate at a given operating frequency.
 *
 * === Power in CMOS ===
 *     P_total = P_static + P_dynamic
 *     P_static ~ negligible (nanowatts)
 *     P_dynamic = C_load * Vdd^2 * f * alpha
 *
 * === Power in TTL ===
 *     P_static ~ milliwatts (DOMINATES!)
 *     P_dynamic = similar formula but static power is so large it barely matters.
 *
 * @param gate - The gate to analyze.
 * @param frequency - Operating frequency in Hz (default 1 GHz).
 * @param cLoad - Load capacitance in Farads (default 1 pF).
 * @param activityFactor - Fraction of cycles with output transition (0-1).
 */
export function analyzePower(
  gate: CMOSInverter | CMOSNand | CMOSNor | TTLNand,
  frequency: number = 1e9,
  cLoad: number = 1e-12,
  activityFactor: number = 0.5,
): PowerAnalysis {
  let staticPow: number;
  let vdd: number;

  if (gate instanceof TTLNand) {
    staticPow = gate.staticPower;
    vdd = gate.vcc;
  } else if (
    gate instanceof CMOSInverter ||
    gate instanceof CMOSNand ||
    gate instanceof CMOSNor
  ) {
    staticPow = 0.0; // Ideal CMOS has zero static power
    vdd = gate.circuit.vdd;
  } else {
    throw new TypeError(`Unsupported gate type`);
  }

  // Dynamic power: P = C * V^2 * f * alpha
  const dynamicPow = cLoad * vdd * vdd * frequency * activityFactor;
  const totalPow = staticPow + dynamicPow;

  // Energy per switching event: E = C * V^2
  const energyPerSwitch = cLoad * vdd * vdd;

  return {
    staticPower: staticPow,
    dynamicPower: dynamicPow,
    totalPower: totalPow,
    energyPerSwitch,
  };
}

/**
 * Compute timing characteristics for a gate.
 *
 * For CMOS: t_pd ~ (C_load * Vdd) / (2 * I_sat)
 * For TTL: t_pd ~ 5-15 ns (fixed by transistor switching speed)
 *
 * @param gate - The gate to analyze.
 * @param cLoad - Load capacitance in Farads (default 1 pF).
 */
export function analyzeTiming(
  gate: CMOSInverter | CMOSNand | CMOSNor | TTLNand,
  cLoad: number = 1e-12,
): TimingAnalysis {
  let tphl: number;
  let tplh: number;
  let riseTime: number;
  let fallTime: number;

  if (gate instanceof TTLNand) {
    // TTL has relatively fixed timing characteristics
    tphl = 7e-9; // HIGH to LOW: ~7 ns
    tplh = 11e-9; // LOW to HIGH: ~11 ns (slower pull-up)
    riseTime = 15e-9;
    fallTime = 10e-9;
  } else if (
    gate instanceof CMOSInverter ||
    gate instanceof CMOSNand ||
    gate instanceof CMOSNor
  ) {
    const vdd = gate.circuit.vdd;

    // Get NMOS/PMOS parameters for timing
    let nmos: { params: { k: number; vth: number } };
    let pmos: { params: { k: number; vth: number } };

    if (gate instanceof CMOSInverter) {
      nmos = gate.nmos;
      pmos = gate.pmos;
    } else {
      nmos = gate.nmos1;
      pmos = gate.pmos1;
    }

    // Saturation current (approximation for timing)
    const k = nmos.params.k;
    const vth = nmos.params.vth;
    const idsSatN =
      vdd > vth ? 0.5 * k * (vdd - vth) ** 2 : 1e-12;
    const idsSatP =
      vdd > pmos.params.vth
        ? 0.5 * pmos.params.k * (vdd - pmos.params.vth) ** 2
        : 1e-12;

    // Propagation delays
    tphl = (cLoad * vdd) / (2.0 * idsSatN); // Pull-down (NMOS)
    tplh = (cLoad * vdd) / (2.0 * idsSatP); // Pull-up (PMOS)

    // Rise and fall times (2.2 RC time constants)
    const rOnN = idsSatN > 0 ? vdd / (2.0 * idsSatN) : 1e6;
    const rOnP = idsSatP > 0 ? vdd / (2.0 * idsSatP) : 1e6;
    riseTime = 2.2 * rOnP * cLoad;
    fallTime = 2.2 * rOnN * cLoad;
  } else {
    throw new TypeError(`Unsupported gate type`);
  }

  const tpd = (tphl + tplh) / 2.0;
  const maxFrequency = tpd > 0 ? 1.0 / (2.0 * tpd) : Infinity;

  return { tphl, tplh, tpd, riseTime, fallTime, maxFrequency };
}

/**
 * Compare CMOS and TTL NAND gates across all metrics.
 *
 * This function demonstrates WHY CMOS replaced TTL:
 * - CMOS has ~1000x less static power
 * - CMOS has better noise margins (relative to Vdd)
 * - CMOS can operate at lower voltages
 *
 * @param frequency - Operating frequency in Hz. Default 1 MHz.
 * @param cLoad - Load capacitance in Farads. Default 1 pF.
 */
export function compareCmosVsTtl(
  frequency: number = 1e6,
  cLoad: number = 1e-12,
): Record<string, Record<string, number>> {
  const cmosNand = new CMOSNand();
  const ttlNand = new TTLNand();

  const cmosPower = analyzePower(cmosNand, frequency, cLoad);
  const ttlPower = analyzePower(ttlNand, frequency, cLoad);

  const cmosTiming = analyzeTiming(cmosNand, cLoad);
  const ttlTiming = analyzeTiming(ttlNand, cLoad);

  const cmosNm = computeNoiseMargins(new CMOSInverter());
  const ttlNm = computeNoiseMargins(ttlNand);

  return {
    cmos: {
      transistor_count: 4,
      supply_voltage: cmosNand.circuit.vdd,
      static_power_w: cmosPower.staticPower,
      dynamic_power_w: cmosPower.dynamicPower,
      total_power_w: cmosPower.totalPower,
      propagation_delay_s: cmosTiming.tpd,
      max_frequency_hz: cmosTiming.maxFrequency,
      noise_margin_low_v: cmosNm.nml,
      noise_margin_high_v: cmosNm.nmh,
    },
    ttl: {
      transistor_count: 3,
      supply_voltage: ttlNand.vcc,
      static_power_w: ttlPower.staticPower,
      dynamic_power_w: ttlPower.dynamicPower,
      total_power_w: ttlPower.totalPower,
      propagation_delay_s: ttlTiming.tpd,
      max_frequency_hz: ttlTiming.maxFrequency,
      noise_margin_low_v: ttlNm.nml,
      noise_margin_high_v: ttlNm.nmh,
    },
  };
}

/**
 * Show how CMOS performance changes with technology scaling.
 *
 * As transistors shrink (Moore's Law):
 * - Gate length decreases -> faster switching
 * - Supply voltage decreases -> less power per switch
 * - Gate capacitance decreases -> less energy per transition
 * - BUT leakage current INCREASES -> more static power (the "leakage wall")
 *
 * @param technologyNodes - Array of gate lengths in meters.
 *     Defaults to [180nm, 90nm, 45nm, 22nm, 7nm, 3nm].
 */
export function demonstrateCmosScaling(
  technologyNodes?: number[],
): Record<string, number>[] {
  const nodes = technologyNodes ?? [180e-9, 90e-9, 45e-9, 22e-9, 7e-9, 3e-9];

  const results: Record<string, number>[] = [];

  for (const node of nodes) {
    // Empirical scaling relationships (simplified)
    const scale = node / 180e-9; // Relative to 180nm baseline

    const vdd = Math.max(0.7, 3.3 * Math.pow(scale, 0.5));
    const vth = Math.max(0.15, 0.4 * Math.pow(scale, 0.3));
    const cGate = 1e-15 * scale;
    const k = 0.001 / Math.pow(scale, 0.5);

    // Create transistor and circuit with scaled parameters
    const inv = new CMOSInverter(
      { vdd, temperature: 300.0 },
      { vth, k, l: node, cGate, w: 1e-6, cDrain: 0.5e-15 },
      { vth, k, l: node, cGate, w: 1e-6, cDrain: 0.5e-15 },
    );

    const timing = analyzeTiming(inv, cGate * 10);
    const power = analyzePower(inv, 1e9, cGate * 10);

    // Leakage current increases exponentially as Vth decreases
    const leakage = 1e-12 * Math.exp((0.4 - vth) / 0.052);

    results.push({
      node_nm: node * 1e9,
      vdd_v: vdd,
      vth_v: vth,
      c_gate_f: cGate,
      propagation_delay_s: timing.tpd,
      dynamic_power_w: power.dynamicPower,
      leakage_current_a: leakage,
      max_frequency_hz: timing.maxFrequency,
    });
  }

  return results;
}
