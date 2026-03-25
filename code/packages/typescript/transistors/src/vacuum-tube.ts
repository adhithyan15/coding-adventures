/**
 * Vacuum Tube (Triode) Model — simplified Child-Langmuir law.
 *
 * === Historical Context ===
 *
 * Before transistors existed, vacuum tubes were the only way to amplify
 * electrical signals. Lee De Forest invented the triode in 1906 by adding
 * a wire grid between the cathode and anode of a vacuum tube diode.
 *
 * The triode is the historical predecessor to the MOSFET and BJT transistors
 * in this package. All three devices share the same fundamental principle:
 * a small control signal (grid voltage / gate voltage / base current)
 * modulates a larger current between two other terminals.
 *
 * === How a Triode Works ===
 *
 * A triode has three electrodes inside a glass vacuum:
 *
 *     Cathode (K):  Heated filament that emits electrons via thermionic emission.
 *     Grid (G):     Wire mesh between cathode and anode. Controls electron flow.
 *     Anode/Plate (P): Positive electrode that collects electrons.
 *
 * The grid voltage controls the flow of electrons:
 *   - Positive grid: electrons pass through the mesh to the anode
 *   - Negative grid: electric field repels electrons back to cathode
 *   - The grid draws almost no current itself (high impedance)
 *
 * === The Child-Langmuir Law ===
 *
 * The plate current follows a 3/2 power law:
 *
 *     Ip = K * (Vg + Vp/mu)^(3/2)
 *
 * Where:
 *   K   = perveance (geometry-dependent constant)
 *   Vg  = grid voltage
 *   Vp  = plate voltage
 *   mu  = amplification factor (how effectively the grid controls current
 *          relative to the plate voltage)
 *
 * The term (Vg + Vp/mu) is the "effective voltage" — it combines the
 * grid's direct control with the plate's weaker influence. When this
 * effective voltage goes negative, no current flows (cutoff).
 *
 * === Comparison to Modern Transistors ===
 *
 * | Property        | Triode (1906)     | MOSFET (1960)     |
 * |-----------------|-------------------|-------------------|
 * | Control signal  | Grid voltage      | Gate voltage      |
 * | Current law     | 3/2 power (Child) | Square (Shockley) |
 * | Size            | ~5 cm             | ~5 nm             |
 * | Power           | ~1 W per tube     | ~1 nW per FET     |
 * | Speed           | ~1 MHz            | ~5 GHz            |
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Parameters for a triode vacuum tube. */
export interface TriodeParams {
  /** Amplification factor — how many volts on the grid equals one volt on the plate. */
  mu: number;
  /** Perveance — geometry-dependent constant (A/V^1.5). */
  K: number;
  /** Fixed plate (anode) voltage in volts. */
  plateVoltage: number;
}

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

/**
 * Default triode parameters representing a typical small-signal triode
 * like the 12AX7 (the tube that made guitar amplifiers famous).
 *
 *   mu = 20:    Grid has 20x the influence of the plate on current flow.
 *   K = 0.001:  Moderate perveance for a small-signal tube.
 *   Vp = 250V:  Typical plate supply voltage for vacuum tube circuits.
 */
export function defaultTriodeParams(): TriodeParams {
  return {
    mu: 20,
    K: 0.001,
    plateVoltage: 250,
  };
}

// ---------------------------------------------------------------------------
// Model Functions
// ---------------------------------------------------------------------------

/**
 * Calculate the plate current (Ip) for a given grid voltage.
 *
 * Uses the Child-Langmuir equation: Ip = K * (Vg + Vp/mu)^(3/2)
 *
 * The 3/2 power law comes from the physics of electron emission in a
 * vacuum — the space-charge-limited regime where emitted electrons
 * form a cloud near the cathode that partially shields it from the
 * anode's electric field.
 *
 * @param gridVoltage - Grid voltage in volts (typically -20V to +5V).
 * @param params - Optional partial triode parameters.
 * @returns Plate current in amperes.
 */
export function triodePlateCurrent(
  gridVoltage: number,
  params?: Partial<TriodeParams>,
): number {
  const p = { ...defaultTriodeParams(), ...params };

  // Effective voltage: the combined influence of grid and plate
  // The grid has mu times more influence than the plate
  const effectiveV = gridVoltage + p.plateVoltage / p.mu;

  // When effective voltage is negative or zero, no electrons reach the anode
  // — the grid's negative field repels all electrons back to the cathode
  if (effectiveV <= 0) return 0;

  // Child-Langmuir 3/2 power law
  return p.K * Math.pow(effectiveV, 1.5);
}

/**
 * Digital abstraction: is the tube conducting current?
 *
 * Returns true when the plate current is greater than zero, meaning
 * the grid voltage is above the cutoff point.
 *
 * This is the same abstraction used by ENIAC — each vacuum tube in a
 * decade ring counter is either conducting (ON) or not conducting (OFF).
 *
 * @param gridVoltage - Grid voltage in volts.
 * @param params - Optional partial triode parameters.
 * @returns True if current is flowing through the tube.
 */
export function isConducting(
  gridVoltage: number,
  params?: Partial<TriodeParams>,
): boolean {
  return triodePlateCurrent(gridVoltage, params) > 0;
}
