/**
 * @coding-adventures/lunar-lander
 *
 * Pure, deterministic simulation core and ASCII telemetry for the classic
 * discrete lunar landing flight simulation (GAME02).
 *
 * Architectural Guarantees:
 * - Strictly zero ambient system authority: capabilities: []
 * - Pure integer fixed-point physics: 64-bit and 128-bit BigInt arithmetic
 *   with zero floating-point operations in the simulation core.
 * - Exact Euclidean integer division (divEuclid) ensuring non-negative remainder.
 * - Bitwise 128-bit integer square root (isqrtU128).
 * - Complete 26-CWE weakness closure (CWE-190, CWE-369, CWE-681, CWE-1321, etc.).
 * - Immutable frozen records and scenes (VER04m state=explicit).
 */

export const VERSION = "0.1.0";

// ============================================================================
// Physical Constants & Apollo LM Parameters
// ============================================================================

/** Lunar surface gravitational acceleration in millifeet/s^2 (5.31 ft/s^2). */
export const G_LUNAR_MFT_S2 = 5310n;

/** Constant dry mass of the Apollo Lunar Module descent stage in pounds. */
export const DRY_MASS_LBS = 16500;

/** Initial usable propellant mass in pounds. */
export const INITIAL_FUEL_LBS = 16500;

/** Total initial gross mass (dry mass + propellant) in pounds. */
export const TOTAL_INITIAL_MASS_LBS = 33000;

/** Initial mission altitude in millifeet (120,000 ft). */
export const INITIAL_ALTITUDE_MFT = 120000000;

/** Initial descent velocity in millifeet/s (-3,000 ft/s, downward). */
export const INITIAL_VELOCITY_MFT_S = -3000000;

/** Descent engine exhaust velocity in millifeet/s (8,000 ft/s). */
export const EXHAUST_VELOCITY_MFT_S = 8000000n;

/** Maximum pilot-commanded engine burn rate in pounds per second. */
export const MAX_BURN_RATE_LBS_S = 200;

/** Minimum turn duration in seconds. */
export const MIN_DURATION_S = 1;

/** Maximum turn duration in seconds. */
export const MAX_DURATION_S = 10;

/** Maximum allowable mission elapsed time in seconds (24 hours). */
export const MAX_TIME_SECONDS = 86400;

/** Maximum plausible simulation altitude boundary in millifeet (200,000 ft). */
export const MAX_ALTITUDE_MFT = 200000000;

/** Minimum valid velocity boundary in millifeet/s (-10,000 ft/s). */
export const MIN_VELOCITY_MFT_S = -10000000;

/** Maximum valid velocity boundary in millifeet/s (+10,000 ft/s). */
export const MAX_VELOCITY_MFT_S = 10000000;

// ============================================================================
// Domain Types & State Enums
// ============================================================================

/** Terminal landing outcome classified by touchdown impact speed magnitude. */
export type LandingOutcome = "Perfect" | "Hard" | "Crash" | "Fatal" | "Crater";

export const LandingOutcome = {
  Perfect: "Perfect",
  Hard: "Hard",
  Crash: "Crash",
  Fatal: "Fatal",
  Crater: "Crater",
} as const;

/** Lifecycle flight status of the Lunar Module. */
export type FlightStatus = "InFlight" | `Landed(${LandingOutcome})`;

export const FlightStatus = {
  InFlight: "InFlight",
  Landed: (outcome: LandingOutcome): `Landed(${LandingOutcome})` => `Landed(${outcome})`,
} as const;

/** Closed enumeration of burn command rejection errors. */
export const LanderError = {
  FlightAlreadyEnded: "FlightAlreadyEnded",
  ExcessiveBurnRate: "ExcessiveBurnRate",
  InvalidDuration: "InvalidDuration",
  ArithmeticOverflow: "ArithmeticOverflow",
} as const;

export type LanderError = (typeof LanderError)[keyof typeof LanderError];

/** Immutable state record capturing the complete flight simulation state. */
export interface LanderState {
  readonly time_seconds: number;
  readonly altitude_mft: number;
  readonly velocity_mft_s: number;
  readonly fuel_lbs: number;
  readonly dry_mass_lbs: number;
  readonly status: FlightStatus;
  readonly impact_velocity_mft_s: number | null;
}

/** Pilot burn order for a discrete simulation turn. */
export interface BurnCommand {
  readonly burn_rate_lbs_s: number;
  readonly duration_seconds: number;
}

/** Pure Result type wrapping success or typed refusal. */
export type Result<T, E = LanderError> =
  | { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly error: E };

export function ok<T>(value: T): Result<T, never> {
  return Object.freeze({ ok: true, value });
}

export function err<E>(error: E): Result<never, E> {
  return Object.freeze({ ok: false, error });
}

// ============================================================================
// Universal 2D Paint Intermediate Representation (IR) Types
// ============================================================================

export interface PaintBase {
  readonly id?: string;
  readonly metadata?: Record<string, string | number | boolean>;
}

export interface PaintText extends PaintBase {
  readonly kind: "text";
  readonly x: number;
  readonly y: number;
  readonly text: string;
  readonly font_ref?: string;
  readonly font_size: number;
  readonly fill?: string;
  readonly text_align?: "left" | "center" | "right";
}

export interface PaintRect extends PaintBase {
  readonly kind: "rect";
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
  readonly fill?: string;
  readonly stroke?: string;
  readonly stroke_width?: number;
  readonly corner_radius?: number;
  readonly stroke_dash?: readonly number[];
  readonly stroke_dash_offset?: number;
}

export type PaintInstruction = PaintText | PaintRect;

export interface PaintScene {
  readonly width: number;
  readonly height: number;
  readonly background: string;
  readonly instructions: readonly PaintInstruction[];
  readonly id?: string;
  readonly metadata?: Record<string, string | number | boolean>;
}

// ============================================================================
// Arithmetic Helpers
// ============================================================================

/**
 * Computes exact Euclidean division a div_euclid b where remainder r is always non-negative.
 * Parity with Rust's i64::div_euclid.
 */
export function divEuclid(a: bigint, b: bigint): bigint {
  const q = a / b;
  const r = a % b;
  return r < 0n ? (b > 0n ? q - 1n : q + 1n) : q;
}

/**
 * Computes exact integer square root floor(sqrt(n)) for 128-bit unsigned integers.
 * Uses bounded bitwise binary search with zero division operations (CWE-369).
 */
export function isqrtU128(n: bigint): bigint {
  if (n <= 0n) {
    return 0n;
  }
  let res = 0n;
  let bit = 1n << 126n;
  while (bit > n) {
    bit >>= 2n;
  }
  let num = n;
  while (bit !== 0n) {
    if (num >= res + bit) {
      num -= res + bit;
      res = (res >> 1n) + bit;
    } else {
      res >>= 1n;
    }
    bit >>= 2n;
  }
  return res;
}

/** Formats an integer with comma thousands separators. */
export function formatWithCommas(n: number | bigint): string {
  const bi = typeof n === "bigint" ? n : BigInt(n);
  const negative = bi < 0n;
  const absVal = negative ? -bi : bi;
  const s = absVal.toString();
  let result = negative ? "-" : "";
  const len = s.length;
  for (let i = 0; i < len; i++) {
    if (i > 0 && (len - i) % 3 === 0) {
      result += ",";
    }
    result += s[i];
  }
  return result;
}

// ============================================================================
// Validation & Security Helpers (CWE-681, CWE-190, CWE-1321)
// ============================================================================

/**
 * Rejects non-plain objects, poisoned prototypes, and prototype pollution (CWE-1321).
 */
export function isPlainObject(val: unknown): val is Record<string, unknown> {
  if (typeof val !== "object" || val === null || Array.isArray(val)) {
    return false;
  }
  if (Object.prototype.hasOwnProperty.call(val, "__proto__")) {
    return false;
  }
  const proto = Object.getPrototypeOf(val);
  return proto === Object.prototype || proto === null;
}

/**
 * Validates that a value is a safe integer number, rejecting NaN, Infinity, floats,
 * strings, booleans, and normalizing negative zero to 0 (CWE-681, CWE-190).
 */
function validateSafeInt(val: unknown): number | null {
  if (typeof val !== "number" || !Number.isSafeInteger(val)) {
    return null;
  }
  return Object.is(val, -0) ? 0 : val;
}

// ============================================================================
// Core Pure Simulation Functions
// ============================================================================

/**
 * Initializes a new Lunar Module descent mission at standard initial conditions.
 * Guaranteed zero heap mutations and immutable return (VER04m).
 */
export function new_flight(): LanderState {
  return Object.freeze({
    time_seconds: 0,
    altitude_mft: INITIAL_ALTITUDE_MFT,
    velocity_mft_s: INITIAL_VELOCITY_MFT_S,
    fuel_lbs: INITIAL_FUEL_LBS,
    dry_mass_lbs: DRY_MASS_LBS,
    status: FlightStatus.InFlight,
    impact_velocity_mft_s: null,
  });
}

/**
 * Validates a candidate burn command against current state and physical invariants.
 *
 * Enforces canonical validation precedence:
 * 1. Lifecycle Check: craft must be InFlight.
 * 2. State Invariants (VER04m, CWE-190, CWE-369): dry mass integrity, fuel capacity,
 *    in-flight altitude positivity, velocity bounds, mission time limits, and absence of impact velocity.
 * 3. Command Bounds: maximum burn rate (<= 200), turn duration (1..=10), and mission time non-overflow.
 */
export function validate_burn(
  state: LanderState,
  cmd: BurnCommand
): Result<void, LanderError> {
  if (!isPlainObject(state) || !isPlainObject(cmd)) {
    return err(LanderError.ArithmeticOverflow);
  }

  // 1. Lifecycle Check
  if (state.status !== FlightStatus.InFlight) {
    return err(LanderError.FlightAlreadyEnded);
  }

  // 2. State Invariants
  const dryMass = validateSafeInt(state.dry_mass_lbs);
  if (dryMass === null || dryMass !== DRY_MASS_LBS) {
    return err(LanderError.ArithmeticOverflow);
  }

  const fuel = validateSafeInt(state.fuel_lbs);
  if (fuel === null || fuel < 0 || fuel > INITIAL_FUEL_LBS) {
    return err(LanderError.ArithmeticOverflow);
  }

  const alt = validateSafeInt(state.altitude_mft);
  if (alt === null || alt <= 0 || alt > MAX_ALTITUDE_MFT) {
    return err(LanderError.ArithmeticOverflow);
  }

  const vel = validateSafeInt(state.velocity_mft_s);
  if (vel === null || vel < MIN_VELOCITY_MFT_S || vel > MAX_VELOCITY_MFT_S) {
    return err(LanderError.ArithmeticOverflow);
  }

  const time = validateSafeInt(state.time_seconds);
  if (time === null || time < 0 || time > MAX_TIME_SECONDS) {
    return err(LanderError.ArithmeticOverflow);
  }

  if (state.impact_velocity_mft_s !== null && state.impact_velocity_mft_s !== undefined) {
    return err(LanderError.ArithmeticOverflow);
  }

  // 3. Command Bounds
  const burnRate = validateSafeInt(cmd.burn_rate_lbs_s);
  if (burnRate === null || burnRate < 0 || burnRate > 0xffffffff) {
    return err(LanderError.ArithmeticOverflow);
  }
  if (burnRate > MAX_BURN_RATE_LBS_S) {
    return err(LanderError.ExcessiveBurnRate);
  }

  const duration = validateSafeInt(cmd.duration_seconds);
  if (duration === null || duration < MIN_DURATION_S || duration > MAX_DURATION_S) {
    return err(LanderError.InvalidDuration);
  }

  const tNext = time + duration;
  if (!Number.isSafeInteger(tNext) || tNext > MAX_TIME_SECONDS) {
    return err(LanderError.ArithmeticOverflow);
  }

  return ok(undefined);
}

/**
 * Integrates a single discrete flight step under the commanded burn rate and duration.
 *
 * Implements second-order Euler-Verlet numerical kinematics in pure fixed-point millifeet arithmetic.
 * When projected altitude reaches or penetrates the lunar surface (Y <= 0), solves for the exact
 * touchdown contact time delta_t and impact velocity, automatically refunding unburned propellant.
 */
export function step_burn(
  state: LanderState,
  cmd: BurnCommand
): Result<LanderState, LanderError> {
  const validation = validate_burn(state, cmd);
  if (!validation.ok) {
    return err(validation.error);
  }

  const dt = BigInt(cmd.duration_seconds);
  const fuelReq = BigInt(cmd.burn_rate_lbs_s) * dt;
  const stateFuel = BigInt(state.fuel_lbs);
  const deltaF = stateFuel < fuelReq ? stateFuel : fuelReq;
  const fuelNext = stateFuel - deltaF;

  const avgMass = BigInt(state.dry_mass_lbs) + stateFuel - (deltaF / 2n);
  const impulse = deltaF * EXHAUST_VELOCITY_MFT_S;
  const aThrust = impulse / (dt * avgMass);
  const aNet = aThrust - G_LUNAR_MFT_S2;

  const vNext = BigInt(state.velocity_mft_s) + aNet * dt;
  const yNext =
    BigInt(state.altitude_mft) +
    BigInt(state.velocity_mft_s) * dt +
    divEuclid(aNet * dt * dt, 2n);
  const tNext = BigInt(state.time_seconds) + dt;

  if (
    yNext > BigInt(MAX_ALTITUDE_MFT) ||
    vNext < BigInt(MIN_VELOCITY_MFT_S) ||
    vNext > BigInt(MAX_VELOCITY_MFT_S)
  ) {
    return err(LanderError.ArithmeticOverflow);
  }

  if (yNext > 0n) {
    return ok(
      Object.freeze({
        time_seconds: Number(tNext),
        altitude_mft: Number(yNext),
        velocity_mft_s: Number(vNext),
        fuel_lbs: Number(fuelNext),
        dry_mass_lbs: state.dry_mass_lbs,
        status: FlightStatus.InFlight,
        impact_velocity_mft_s: null,
      })
    );
  } else {
    // Ground touchdown solver
    const aHalf = divEuclid(aNet, 2n);
    let deltaTRaw: bigint;
    if (aHalf === 0n) {
      if (state.velocity_mft_s === 0) {
        return err(LanderError.ArithmeticOverflow);
      }
      deltaTRaw = divEuclid(-BigInt(state.altitude_mft), BigInt(state.velocity_mft_s));
    } else {
      const v128 = BigInt(state.velocity_mft_s);
      const y128 = BigInt(state.altitude_mft);
      const a128 = aHalf;
      const disc = v128 * v128 - 4n * a128 * y128;
      const discPos = disc > 0n ? disc : 0n;
      const root = isqrtU128(discPos);
      deltaTRaw = divEuclid(-v128 - root, 2n * aHalf);
    }

    let deltaT = deltaTRaw;
    if (deltaT < 1n) {
      deltaT = 1n;
    }
    if (deltaT > dt) {
      deltaT = dt;
    }

    const deltaTU32 = deltaT;
    const vTouchdown = BigInt(state.velocity_mft_s) + aNet * deltaT;
    const tFinal = BigInt(state.time_seconds) + deltaTU32;

    const fuelBurnReqTouchdown = deltaTU32 * BigInt(cmd.burn_rate_lbs_s);
    const fuelBurnedTouchdown =
      stateFuel < fuelBurnReqTouchdown ? stateFuel : fuelBurnReqTouchdown;
    const fuelFinal = stateFuel - fuelBurnedTouchdown;

    const speed = vTouchdown < 0n ? -vTouchdown : vTouchdown;
    let outcome: LandingOutcome;
    if (speed <= 5000n) {
      outcome = LandingOutcome.Perfect;
    } else if (speed <= 15000n) {
      outcome = LandingOutcome.Hard;
    } else if (speed <= 30000n) {
      outcome = LandingOutcome.Crash;
    } else if (speed <= 60000n) {
      outcome = LandingOutcome.Fatal;
    } else {
      outcome = LandingOutcome.Crater;
    }

    return ok(
      Object.freeze({
        time_seconds: Number(tFinal),
        altitude_mft: 0,
        velocity_mft_s: Number(vTouchdown),
        fuel_lbs: Number(fuelFinal),
        dry_mass_lbs: state.dry_mass_lbs,
        status: FlightStatus.Landed(outcome),
        impact_velocity_mft_s: Number(vTouchdown),
      })
    );
  }
}

// ============================================================================
// ASCII Telemetry & PaintScene Presentation Bridge
// ============================================================================

/**
 * Translates a flight state into a declarative 76x5 PaintScene.
 *
 * Telemetry HUD visual layout:
 * - Outer border drawn via multiline box characters.
 * - Numerical instruments for flight time, altitude, velocity, fuel, burn rate, and descent rate.
 * - 20-character proportional bar gauges for altitude and usable fuel.
 */
export function to_paint_scene(
  state: LanderState,
  last_burn_rate_lbs_s: number = 0
): PaintScene {
  const altClamped = Math.max(0, Math.min(state.altitude_mft, INITIAL_ALTITUDE_MFT));
  const altPct = Math.floor((altClamped * 100) / INITIAL_ALTITUDE_MFT);
  const altBars = Math.floor((altClamped * 20) / INITIAL_ALTITUDE_MFT);

  const fuelClamped = Math.max(0, Math.min(state.fuel_lbs, INITIAL_FUEL_LBS));
  const fuelPct = Math.floor((fuelClamped * 100) / INITIAL_FUEL_LBS);
  const fuelBars = Math.floor((fuelClamped * 20) / INITIAL_FUEL_LBS);

  const altFt = Math.trunc(state.altitude_mft / 1000);
  const velFps = Math.trunc(state.velocity_mft_s / 1000);
  const fuelLbs = state.fuel_lbs;

  const altStr = formatWithCommas(altFt);
  const fuelStr = formatWithCommas(fuelLbs);

  let rateLabel: string;
  let rateText: string;
  let rateSymbol: string;

  if (state.status === FlightStatus.InFlight) {
    if (state.velocity_mft_s < 0) {
      rateLabel = "RATE:";
      rateText = "DESCENT";
      rateSymbol = "▼";
    } else if (state.velocity_mft_s > 0) {
      rateLabel = "RATE:";
      rateText = "ASCENT";
      rateSymbol = "▲";
    } else {
      rateLabel = "RATE:";
      rateText = "HOVER";
      rateSymbol = "◆";
    }
  } else {
    rateLabel = "STATUS:";
    rateSymbol = "★";
    if (state.status === "Landed(Perfect)") rateText = "PERFECT";
    else if (state.status === "Landed(Hard)") rateText = "HARD";
    else if (state.status === "Landed(Crash)") rateText = "CRASH";
    else if (state.status === "Landed(Fatal)") rateText = "FATAL";
    else rateText = "CRATER";
  }

  const row0 = "┌───────────────────────── LUNAR LANDER TELEMETRY ─────────────────────────┐";
  const left1 = " TIME: " + String(state.time_seconds).padStart(6, " ") + " SEC    ";
  const right1 =
    " ALTITUDE: " +
    altStr.padStart(7, " ") +
    " FT [░░░░░░░░░░░░░░░░░░░░] " +
    String(altPct).padStart(3, " ") +
    "%   ";
  const row1 = "│" + left1 + "│" + right1 + "│";

  const left2 = " VELOCITY: " + String(velFps).padStart(5, " ") + " FT/S";
  const right2 =
    " FUEL:    " +
    fuelStr.padStart(7, " ") +
    " LBS [░░░░░░░░░░░░░░░░░░░░] " +
    String(fuelPct).padStart(3, " ") +
    "%   ";
  const row2 = "│" + left2 + "│" + right2 + "│";

  const burnStr = String(last_burn_rate_lbs_s).padStart(4, " ");
  const left3 = " BURN:     " + burnStr + " LB/S ";
  const rateCol =
    rateLabel.padEnd(7, " ") + " " + rateText.padEnd(8, " ") + " " + rateSymbol.padEnd(1, " ");
  const row3 = "│" + left3 + "│ " + rateCol.padEnd(51, " ") + "│";
  const row4 = "└──────────────────────────────────────────────────────────────────────────┘";

  const fullText = [row0, row1, row2, row3, row4].join("\n");

  const instructions: PaintInstruction[] = [
    Object.freeze({
      kind: "text",
      x: 0,
      y: 0,
      text: fullText,
      font_size: 1,
      fill: "#ffffff",
      text_align: "left",
    }),
  ];

  if (altBars > 0) {
    instructions.push(
      Object.freeze({
        kind: "rect",
        x: 46,
        y: 1,
        width: altBars - 1,
        height: 0,
        fill: "#00ff00",
      })
    );
  }

  if (fuelBars > 0) {
    instructions.push(
      Object.freeze({
        kind: "rect",
        x: 46,
        y: 2,
        width: fuelBars - 1,
        height: 0,
        fill: "#00ff00",
      })
    );
  }

  return Object.freeze({
    width: 76,
    height: 5,
    background: "#000000",
    instructions: Object.freeze(instructions),
  });
}

/** Convenience helper translating a flight state into a declarative PaintScene with zero initial burn. */
export function telemetry_scene(state: LanderState): PaintScene {
  return to_paint_scene(state, 0);
}

/**
 * Renders a flight state telemetry dashboard into a deterministic ASCII string.
 */
export function render_telemetry_ascii(
  state: LanderState,
  last_burn_rate_lbs_s: number = 0
): string {
  const scene = to_paint_scene(state, last_burn_rate_lbs_s);
  const rows = 5;
  const cols = 76;
  const buf: string[][] = Array.from({ length: rows }, () => Array(cols).fill(" "));

  for (const inst of scene.instructions) {
    if (inst.kind === "text") {
      const lines = inst.text.split("\n");
      for (let r = 0; r < lines.length; r++) {
        const line = Array.from(lines[r]);
        for (let c = 0; c < line.length; c++) {
          if (r < rows && c < cols) {
            buf[r][c] = line[c];
          }
        }
      }
    } else if (inst.kind === "rect") {
      const c1 = Math.round(inst.x);
      const r1 = Math.round(inst.y);
      const c2 = Math.round(inst.x + inst.width);
      const r2 = Math.round(inst.y + inst.height);
      for (let r = r1; r <= r2; r++) {
        for (let c = c1; c <= c2; c++) {
          if (r >= 0 && r < rows && c >= 0 && c < cols) {
            buf[r][c] = "█";
          }
        }
      }
    }
  }

  return buf.map((row) => row.join("").trimEnd()).join("\n").trimEnd();
}

/** Convenience helper rendering a flight state telemetry dashboard with zero burn rate. */
export function render_telemetry(state: LanderState): string {
  return render_telemetry_ascii(state, 0);
}

// ============================================================================
// CamelCase Aliases for Ergonomics & Interoperability
// ============================================================================

export const newFlight = new_flight;
export const validateBurn = validate_burn;
export const stepBurn = step_burn;
export const div_euclid = divEuclid;
export const isqrt_u128 = isqrtU128;
export const toPaintScene = to_paint_scene;
export const telemetryScene = telemetry_scene;
export const renderTelemetryAscii = render_telemetry_ascii;
export const renderTelemetry = render_telemetry;
export const format_with_commas = formatWithCommas;
