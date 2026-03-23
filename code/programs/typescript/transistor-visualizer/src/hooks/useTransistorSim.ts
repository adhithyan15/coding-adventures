/**
 * Simulation hooks — React wrappers around the transistors package.
 *
 * These hooks encapsulate the simulation state for each transistor era,
 * providing a clean interface for the visualization components. Each hook
 * manages its own slider state and computes derived values (region, current,
 * conducting state) reactively.
 *
 * === Why hooks instead of raw state? ===
 *
 * 1. Encapsulation: each era's simulation logic lives in one place
 * 2. Memoization: transistor instances are created once, not on every render
 * 3. Derived values: region and current are always in sync with voltage
 */

import { useState, useMemo } from "react";
import {
  NPN,
  NMOS,
  CMOSInverter,
  demonstrateCmosScaling,
} from "@coding-adventures/transistors";

// ---------------------------------------------------------------------------
// BJT Simulation Hook
// ---------------------------------------------------------------------------

/**
 * Hook for the BJT (Bipolar Junction Transistor) era.
 *
 * Simulates an NPN transistor with adjustable base-emitter voltage.
 * The collector-emitter voltage is fixed at 5V — a typical supply voltage
 * for TTL-era circuits.
 *
 * @returns Reactive state for BJT visualization
 */
export function useBjtSim() {
  const [vbe, setVbe] = useState(0);
  const vce = 5.0;

  // Create the NPN transistor once and reuse it across renders
  const npn = useMemo(() => new NPN(), []);

  // Derive all values from the current voltage setting
  const region = npn.region(vbe, vce);
  const ic = npn.collectorCurrent(vbe, vce);
  const ib = npn.baseCurrent(vbe, vce);
  const conducting = npn.isConducting(vbe);

  return { vbe, setVbe, vce, region, ic, ib, conducting };
}

// ---------------------------------------------------------------------------
// MOSFET Simulation Hook
// ---------------------------------------------------------------------------

/**
 * Hook for the MOSFET era.
 *
 * Simulates an NMOS transistor with adjustable gate-source voltage.
 * The drain-source voltage is fixed at 3.3V — the standard CMOS supply
 * voltage that dominated the 1990s-2000s era.
 *
 * @returns Reactive state for MOSFET visualization
 */
export function useMosfetSim() {
  const [vgs, setVgs] = useState(0);
  const vds = 3.3;

  const nmos = useMemo(() => new NMOS(), []);

  const region = nmos.region(vgs, vds);
  const ids = nmos.drainCurrent(vgs, vds);
  const conducting = nmos.isConducting(vgs);

  return { vgs, setVgs, vds, region, ids, conducting };
}

// ---------------------------------------------------------------------------
// CMOS Simulation Hook
// ---------------------------------------------------------------------------

/**
 * Hook for the CMOS era.
 *
 * Simulates a CMOS inverter with a digital toggle input (0 or 1).
 * Also pre-computes the voltage transfer characteristic curve and
 * technology scaling data for the supplementary visualizations.
 *
 * The VTC and scaling data are computed once (they don't change with input)
 * and memoized for performance.
 *
 * @returns Reactive state for CMOS visualization
 */
export function useCmosSim() {
  const [inputDigital, setInputDigital] = useState<0 | 1>(0);

  const inverter = useMemo(() => new CMOSInverter(), []);

  // Map digital input to analog voltage (0V or 1.8V for modern CMOS)
  const inputV = inputDigital === 1 ? 1.8 : 0;
  const output = inverter.evaluate(inputV);

  // Voltage Transfer Characteristic — 50 points from 0 to Vdd
  // Shows the sharp switching threshold that makes CMOS ideal for digital
  const vtc = useMemo(() => inverter.voltageTranferCharacteristic(50), [inverter]);

  // Technology scaling data — Moore's Law in numbers
  const scaling = useMemo(() => demonstrateCmosScaling(), []);

  return { inputDigital, setInputDigital, output, vtc, scaling };
}
