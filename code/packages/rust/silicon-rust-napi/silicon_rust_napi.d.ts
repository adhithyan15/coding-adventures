/**
 * silicon_rust_napi — TypeScript declarations for the silicon simulation stack
 * Node.js N-API addon.
 *
 * Load via:
 *   const srp = require('./silicon_rust_napi.node') as typeof import('./silicon_rust_napi');
 */

// ─── Physical constants (no arguments) ──────────────────────────────────────

/** Boltzmann constant k [J/K]. */
export declare function kBoltzmann(): number;
/** Elementary charge q [C]. */
export declare function qElectron(): number;
/** Vacuum permittivity ε₀ [F/m]. */
export declare function eps0(): number;
/** Silicon permittivity ε_Si [F/m]. */
export declare function epsSi(): number;
/** SiO₂ permittivity ε_ox [F/m]. */
export declare function epsOx(): number;
/** Intrinsic carrier concentration n_i at 300 K [/m³]. */
export declare function niAt300k(): number;
/** Silicon bandgap E_g at 300 K [eV]. */
export declare function egSiAt300k(): number;
/** Electron mobility μ_n at 300 K [m²/V·s]. */
export declare function muN300k(): number;
/** Hole mobility μ_p at 300 K [m²/V·s]. */
export declare function muP300k(): number;

// ─── Device-physics functions ────────────────────────────────────────────────

/** Thermal voltage V_T = kT/q [V]. At 300 K → 0.02585 V. */
export declare function thermalVoltage(tKelvin: number): number;

/** Intrinsic carrier concentration n_i(T) [/m³]. Throws below 100 K. */
export declare function intrinsicConcentration(tKelvin: number): number;

/**
 * Fermi potential φ_F [V].
 * - kind `'p'`: returns +|φ_F|
 * - kind `'n'`: returns −|φ_F|
 */
export declare function fermiPotential(
  nDoping: number,
  kind: 'p' | 'n',
  tKelvin: number,
): number;

/** Built-in voltage V_bi [V] for an abrupt p-n junction. */
export declare function pnJunctionBuiltInVoltage(
  na: number,
  nd: number,
  t: number,
): number;

/**
 * Total depletion-region width W [m].
 * Positive `vApplied` = forward bias; negative = reverse bias.
 */
export declare function pnJunctionDepletionWidth(
  na: number,
  nd: number,
  t: number,
  vApplied: number,
): number;

/**
 * Shockley saturation current I_S [A].
 * @param a  Junction area [m²]
 * @param tauN  Electron minority-carrier lifetime [s]
 * @param tauP  Hole minority-carrier lifetime [s]
 */
export declare function pnJunctionSaturationCurrent(
  na: number,
  nd: number,
  a: number,
  t: number,
  tauN: number,
  tauP: number,
): number;

/** Shockley diode current I [A] at applied voltage `v` [V]. */
export declare function pnJunctionCurrent(
  na: number,
  nd: number,
  a: number,
  t: number,
  tauN: number,
  tauP: number,
  v: number,
): number;

/**
 * Threshold voltage V_t [V] with body effect.
 * @param deviceType  `'NMOS'` or `'PMOS'`
 * @param vSb  Source-to-body reverse bias [V] (≥ 0)
 */
export declare function mosfetThresholdVoltage(
  deviceType: 'NMOS' | 'PMOS',
  l: number,
  w: number,
  tOx: number,
  nBody: number,
  phiMs: number,
  qOx: number,
  t: number,
  vSb: number,
): number;

// ─── MOSFET Level-1 model ───────────────────────────────────────────────────

/** DC operating-point result from the Level-1 MOSFET model. */
export interface MosResult {
  /** Drain current [A] */
  id: number;
  /** Transconductance g_m [S] */
  gm: number;
  /** Output conductance g_ds [S] */
  gds: number;
  /** Body transconductance g_mb [S] */
  gmb: number;
  /** Gate-source capacitance C_gs [F] */
  cgs: number;
  /** Gate-drain capacitance C_gd [F] */
  cgd: number;
  /** Gate-body capacitance C_gb [F] */
  cgb: number;
  /** Body-source capacitance C_bs [F] */
  cbs: number;
  /** Body-drain capacitance C_bd [F] */
  cbd: number;
  /** Operating region */
  region: 'cutoff' | 'subthreshold' | 'triode' | 'saturation';
}

/**
 * Evaluate the SPICE Level-1 MOSFET model at a given operating point.
 * @param lambda  Channel-length modulation parameter [V⁻¹]
 * @param gamma   Body-effect coefficient [√V]
 * @param phi     Surface potential 2|φ_F| [V]
 */
export declare function evaluateLevel1(
  vt0: number,
  kp: number,
  lambda: number,
  gamma: number,
  phi: number,
  w: number,
  l: number,
  nSub: number,
  vGs: number,
  vDs: number,
  vBs: number,
  t: number,
): MosResult;

/**
 * Evaluate the Level-1 MOSFET using the default 130 nm NMOS parameter set.
 * Equivalent to `evaluateLevel1` with the default `Level1Params`.
 */
export declare function evaluateLevel1Defaults(
  vGs: number,
  vDs: number,
  vBs: number,
  t: number,
): MosResult;

// ─── Fab-process simulation ──────────────────────────────────────────────────

/**
 * Grow thermal SiO₂ via the Deal-Grove model.
 * Optional `aUm` / `bUm2PerHr` default to dry-O₂ 1000 °C values.
 * @returns Updated cross-section wire string
 */
export declare function dealGroveOxidation(
  csStr: string,
  timeMin: number,
  aUm?: number,
  bUm2PerHr?: number,
): string;

/**
 * Deposit a uniform film on top of the cross-section.
 * Throws if `material` contains `|` or `:` (wire-format injection guard).
 * @returns Updated cross-section wire string
 */
export declare function deposit(
  csStr: string,
  material: string,
  thicknessNm: number,
): string;

/**
 * Remove `depthNm` nm of `targetMaterial` from the top.
 * Stops when the budget is exhausted or a different material is reached.
 * @returns Updated cross-section wire string
 */
export declare function etch(
  csStr: string,
  targetMaterial: string,
  depthNm: number,
): string;

/**
 * Add a Gaussian ion-implant profile to the topmost Si layer.
 * @param species  `'B'`, `'P'`, `'As'`, or `'BF2'`
 * @returns Updated cross-section wire string
 */
export declare function implant(
  csStr: string,
  species: 'B' | 'P' | 'As' | 'BF2',
  energyKev: number,
  doseCm2: number,
): string;

/**
 * Broaden all Gaussian doping profiles via Fick's law.
 * `temperatureC` defaults to 1000 °C when omitted.
 * @returns Updated cross-section wire string
 */
export declare function diffuse(
  csStr: string,
  timeMin: number,
  temperatureC?: number,
): string;

/** Result of a SRIM table lookup. Both values are in nm. */
export interface ImplantRangeResult {
  /** Projected range R_p [nm] */
  rp: number;
  /** Straggle ΔR_p [nm] */
  straggle: number;
}

/**
 * Return projected range and straggle from the SRIM table (linear
 * interpolation).  Throws for unknown species.
 */
export declare function implantRange(
  species: string,
  energyKev: number,
): ImplantRangeResult;

/**
 * Arrhenius-scaled diffusivity D(T) [cm²/s].
 * Uses the Arrhenius equation scaled from the 1000 °C reference.
 */
export declare function diffusivityCm2PerS(
  species: string,
  temperatureC: number,
): number;
