/**
 * Shared types for the transistors package.
 *
 * === Enums and Interfaces ===
 *
 * These types define the vocabulary of transistor simulation. Every transistor
 * has an operating region (cutoff, linear, saturation), and every circuit has
 * electrical parameters (voltage, capacitance, etc.).
 *
 * We use readonly interfaces for parameters because transistor characteristics
 * are fixed once manufactured — you can't change a transistor's threshold
 * voltage after fabrication. The readonly modifier enforces this immutability.
 */

// ===========================================================================
// OPERATING REGION ENUMS
// ===========================================================================
// A transistor is an analog device that operates differently depending on
// the voltages applied to its terminals. The three "regions" describe these
// different operating modes.

/**
 * Operating region of a MOSFET transistor.
 *
 * Think of it like a water faucet with three positions:
 *
 *     CUTOFF:     Faucet is fully closed. No water flows.
 *                 (Vgs < Vth — gate voltage too low to turn on)
 *
 *     LINEAR:     Faucet is open, and water flow increases as you
 *                 turn the handle more. Flow is proportional to
 *                 both handle position AND water pressure.
 *                 (Vgs > Vth, Vds < Vgs - Vth — acts like a resistor)
 *
 *     SATURATION: Faucet is wide open, but the pipe is the bottleneck.
 *                 Adding more pressure doesn't increase flow much.
 *                 (Vgs > Vth, Vds >= Vgs - Vth — current is roughly constant)
 *
 * For digital circuits, we only use CUTOFF (OFF) and deep LINEAR (ON).
 * For analog amplifiers, we operate in SATURATION.
 */
export enum MOSFETRegion {
  CUTOFF = "cutoff",
  LINEAR = "linear",
  SATURATION = "saturation",
}

/**
 * Operating region of a BJT transistor.
 *
 * Similar to MOSFET regions but with different names and physics:
 *
 *     CUTOFF:      No base current -> no collector current. Switch OFF.
 *                  (Vbe < ~0.7V)
 *
 *     ACTIVE:      Small base current, large collector current.
 *                  Ic = beta * Ib. This is the AMPLIFIER region.
 *                  (Vbe >= ~0.7V, Vce > ~0.2V)
 *
 *     SATURATION:  Both junctions forward-biased. Collector current
 *                  is maximum — transistor is fully ON as a switch.
 *                  (Vbe >= ~0.7V, Vce <= ~0.2V)
 *
 * Confusing naming alert: MOSFET "saturation" = constant current (amplifier).
 * BJT "saturation" = fully ON (switch). These are DIFFERENT behaviors despite
 * sharing a name. Hardware engineers have been confusing students with this
 * for decades.
 */
export enum BJTRegion {
  CUTOFF = "cutoff",
  ACTIVE = "active",
  SATURATION = "saturation",
}

/**
 * Transistor polarity/type.
 */
export enum TransistorType {
  NMOS = "nmos",
  PMOS = "pmos",
  NPN = "npn",
  PNP = "pnp",
}

// ===========================================================================
// ELECTRICAL PARAMETERS
// ===========================================================================
// These interfaces hold the physical characteristics of transistors.
// Default values represent common, well-documented transistor types
// so that users can start experimenting immediately without needing
// to look up datasheets.

/**
 * Electrical parameters for a MOSFET transistor.
 *
 * Default values represent a typical 180nm CMOS process — the last
 * "large" process node that is still widely used in education and
 * analog/mixed-signal chips.
 *
 * Key parameters:
 *     vth:     Threshold voltage — the minimum Vgs to turn the transistor ON.
 *              Lower Vth = faster switching but more leakage current.
 *
 *     k:       Transconductance parameter — controls how much current flows
 *              for a given Vgs. k = mu * Cox * (W/L).
 *
 *     w, l:    Channel width and length. The W/L ratio is the main knob
 *              chip designers use to tune transistor strength.
 *
 *     cGate:   Gate capacitance — determines switching speed.
 *
 *     cDrain:  Drain junction capacitance — contributes to output load.
 */
export interface MOSFETParams {
  readonly vth: number;
  readonly k: number;
  readonly w: number;
  readonly l: number;
  readonly cGate: number;
  readonly cDrain: number;
}

/**
 * Electrical parameters for a BJT transistor.
 *
 * Default values represent a typical small-signal NPN transistor
 * like the 2N2222.
 *
 * Key parameters:
 *     beta:    Current gain (hfe) — the ratio Ic/Ib.
 *     vbeOn:   Base-emitter voltage when conducting (~0.7V for silicon).
 *     vceSat:  Collector-emitter voltage when fully saturated.
 *     is:      Reverse saturation current (named `is` since TypeScript
 *              does not reserve `is` as a keyword in this context).
 *     cBase:   Base capacitance — limits switching speed.
 */
export interface BJTParams {
  readonly beta: number;
  readonly vbeOn: number;
  readonly vceSat: number;
  readonly is: number;
  readonly cBase: number;
}

/**
 * Parameters for a complete logic gate circuit.
 *
 * vdd:         Supply voltage. Modern CMOS uses 0.7-1.2V, older CMOS
 *              used 3.3V or 5V, TTL always uses 5V.
 *
 * temperature: Junction temperature in Kelvin. Room temperature is ~300K.
 */
export interface CircuitParams {
  readonly vdd: number;
  readonly temperature: number;
}

// ===========================================================================
// RESULT TYPES
// ===========================================================================
// These interfaces hold the results of transistor and circuit analysis.

/**
 * Result of evaluating a logic gate with voltage-level detail.
 *
 * Unlike the logic_gates package which only returns 0 or 1, this gives
 * you the full electrical picture: what voltage does the output actually
 * sit at? How much power is being consumed? How long did the signal
 * take to propagate?
 */
export interface GateOutput {
  readonly logicValue: number;
  readonly voltage: number;
  readonly currentDraw: number;
  readonly powerDissipation: number;
  readonly propagationDelay: number;
  readonly transistorCount: number;
}

/**
 * Results of analyzing a transistor as an amplifier.
 *
 * When a transistor operates in its linear/active region (not as a
 * digital switch), it can amplify small signals into larger ones.
 *
 * voltageGain:      How much the output voltage changes per unit input change.
 * transconductance: gm — output current change per input voltage change (A/V).
 * inputImpedance:   How much the amplifier "loads" the signal source.
 * outputImpedance:  How "stiff" the output is.
 * bandwidth:        Frequency at which gain drops to -3dB.
 */
export interface AmplifierAnalysis {
  readonly voltageGain: number;
  readonly transconductance: number;
  readonly inputImpedance: number;
  readonly outputImpedance: number;
  readonly bandwidth: number;
  readonly operatingPoint: Record<string, number>;
}

/**
 * Noise margin analysis for a logic family.
 *
 * Noise margins tell you how much electrical noise a digital signal
 * can tolerate before being misinterpreted.
 *
 *     vol: Output LOW voltage
 *     voh: Output HIGH voltage
 *     vil: Input LOW threshold
 *     vih: Input HIGH threshold
 *     nml: Noise Margin LOW = vil - vol
 *     nmh: Noise Margin HIGH = voh - vih
 */
export interface NoiseMargins {
  readonly vol: number;
  readonly voh: number;
  readonly vil: number;
  readonly vih: number;
  readonly nml: number;
  readonly nmh: number;
}

/**
 * Power consumption breakdown for a gate or circuit.
 *
 * staticPower:      Power consumed even when not switching.
 * dynamicPower:     Power consumed during switching (P = C * Vdd^2 * f * alpha).
 * totalPower:       static + dynamic.
 * energyPerSwitch:  Energy for one complete 0->1->0 transition (C * Vdd^2).
 */
export interface PowerAnalysis {
  readonly staticPower: number;
  readonly dynamicPower: number;
  readonly totalPower: number;
  readonly energyPerSwitch: number;
}

/**
 * Timing characteristics for a gate.
 *
 * tphl:          Propagation delay HIGH to LOW output.
 * tplh:          Propagation delay LOW to HIGH output.
 * tpd:           Average propagation delay = (tphl + tplh) / 2.
 * riseTime:      Time for output to go from 10% to 90% of Vdd.
 * fallTime:      Time for output to go from 90% to 10% of Vdd.
 * maxFrequency:  Maximum clock frequency = 1 / (2 * tpd).
 */
export interface TimingAnalysis {
  readonly tphl: number;
  readonly tplh: number;
  readonly tpd: number;
  readonly riseTime: number;
  readonly fallTime: number;
  readonly maxFrequency: number;
}

// ===========================================================================
// DEFAULT FACTORY FUNCTIONS
// ===========================================================================
// These provide sensible defaults for each parameter type, matching
// the Python dataclass defaults.

/** Create default MOSFET parameters (180nm CMOS process). */
export function defaultMOSFETParams(): MOSFETParams {
  return {
    vth: 0.4,
    k: 0.001,
    w: 1e-6,
    l: 180e-9,
    cGate: 1e-15,
    cDrain: 0.5e-15,
  };
}

/** Create default BJT parameters (2N2222-style NPN). */
export function defaultBJTParams(): BJTParams {
  return {
    beta: 100.0,
    vbeOn: 0.7,
    vceSat: 0.2,
    is: 1e-14,
    cBase: 5e-12,
  };
}

/** Create default circuit parameters (3.3V, 300K). */
export function defaultCircuitParams(): CircuitParams {
  return {
    vdd: 3.3,
    temperature: 300.0,
  };
}
