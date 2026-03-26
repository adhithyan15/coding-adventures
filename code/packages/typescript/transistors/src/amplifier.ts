/**
 * Analog Amplifier Analysis — transistors as signal amplifiers.
 *
 * === Beyond Digital: Transistors as Amplifiers ===
 *
 * A transistor used as a digital switch operates in only two states: ON and OFF.
 * But transistors are fundamentally ANALOG devices. When biased in the right
 * operating region (saturation for MOSFET, active for BJT), they can amplify
 * small signals into larger ones.
 *
 * === Common-Source Amplifier (MOSFET) ===
 *
 *     Voltage gain: Av = -gm x Rd (inverting amplifier)
 *
 * === Common-Emitter Amplifier (BJT) ===
 *
 *     Voltage gain: Av = -gm x Rc = -(Ic/Vt) x Rc
 */

import { NPN } from "./bjt.js";
import { NMOS } from "./mosfet.js";
import type { AmplifierAnalysis } from "./types.js";

/**
 * Analyze an NMOS common-source amplifier configuration.
 *
 * The common-source amplifier is the most basic MOSFET amplifier topology.
 * The input signal is applied to the gate, and the output is taken from
 * the drain. A drain resistor (Rd) converts the drain current variation
 * into a voltage swing.
 *
 * @param transistor - NMOS transistor instance with desired parameters.
 * @param vgs - DC gate-to-source bias voltage (V). Must be > Vth.
 * @param vdd - Supply voltage (V).
 * @param rDrain - Drain resistor value (ohms).
 * @param cLoad - Output load capacitance (F). Default 1 pF.
 * @returns AmplifierAnalysis with gain, impedance, and bandwidth.
 */
export function analyzeCommonSourceAmp(
  transistor: NMOS,
  vgs: number,
  vdd: number,
  rDrain: number,
  cLoad: number = 1e-12,
): AmplifierAnalysis {
  // Calculate DC operating point
  let ids = transistor.drainCurrent(vgs, vdd); // Approximate: Vds ~ Vdd
  let vds = vdd - ids * rDrain;

  // Recalculate with correct Vds
  ids = transistor.drainCurrent(vgs, Math.max(vds, 0.0));
  vds = vdd - ids * rDrain;

  // Transconductance
  const gm = transistor.transconductance(vgs, Math.max(vds, 0.0));

  // Voltage gain: Av = -gm x Rd (inverting)
  const voltageGain = -gm * rDrain;

  // Input impedance: essentially infinite for MOSFET
  const inputImpedance = 1e12; // 1 T-ohm

  // Output impedance: approximately Rd
  const outputImpedance = rDrain;

  // Bandwidth: f_3dB = 1 / (2*pi * Rd * C_load)
  const bandwidth = 1.0 / (2.0 * Math.PI * rDrain * cLoad);

  const operatingPoint: Record<string, number> = {
    vgs,
    vds,
    ids,
    gm,
  };

  return {
    voltageGain,
    transconductance: gm,
    inputImpedance,
    outputImpedance,
    bandwidth,
    operatingPoint,
  };
}

/**
 * Analyze an NPN common-emitter amplifier configuration.
 *
 * The BJT equivalent of the common-source amplifier. Input is applied
 * to the base, output taken from the collector.
 *
 * BJT amplifiers typically have higher voltage gain than MOSFET amplifiers
 * at the same current, because BJT transconductance (gm = Ic/Vt) is
 * higher than MOSFET transconductance for the same bias current.
 *
 * However, BJT amplifiers have lower input impedance because base
 * current flows continuously.
 *
 * @param transistor - NPN transistor instance.
 * @param vbe - DC base-emitter bias voltage (V). Should be ~0.7V.
 * @param vcc - Supply voltage (V).
 * @param rCollector - Collector resistor value (ohms).
 * @param cLoad - Output load capacitance (F).
 * @returns AmplifierAnalysis with gain, impedance, and bandwidth.
 */
export function analyzeCommonEmitterAmp(
  transistor: NPN,
  vbe: number,
  vcc: number,
  rCollector: number,
  cLoad: number = 1e-12,
): AmplifierAnalysis {
  // Calculate DC operating point
  let vce = vcc; // Initial approximation
  let ic = transistor.collectorCurrent(vbe, vce);
  vce = vcc - ic * rCollector;
  vce = Math.max(vce, 0.0);

  // Recalculate with correct Vce
  ic = transistor.collectorCurrent(vbe, vce);

  // Transconductance
  const gm = transistor.transconductance(vbe, vce);

  // Voltage gain: Av = -gm x Rc
  const voltageGain = -gm * rCollector;

  // Input impedance: r_pi = beta / gm = beta * Vt / Ic
  const beta = transistor.params.beta;
  const vt = 0.026;
  let rPi: number;
  if (ic > 0) {
    rPi = (beta * vt) / ic;
  } else {
    rPi = 1e12; // Very high when no current flows
  }

  const inputImpedance = rPi;
  const outputImpedance = rCollector;

  // Bandwidth
  const bandwidth = 1.0 / (2.0 * Math.PI * rCollector * cLoad);

  const operatingPoint: Record<string, number> = {
    vbe,
    vce,
    ic,
    ib: transistor.baseCurrent(vbe, vce),
    gm,
  };

  return {
    voltageGain,
    transconductance: gm,
    inputImpedance,
    outputImpedance,
    bandwidth,
    operatingPoint,
  };
}
