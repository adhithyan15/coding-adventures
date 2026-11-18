/**
 * A deterministic fixed-step clock for simulation code.
 *
 * The simulation lane uses integer nanoseconds throughout. JavaScript's `number`
 * cannot represent every `u64`, so time counters cross this public API as `bigint`;
 * only the deliberately bounded `u32` step count uses `number`.
 */

export const ClockError = Object.freeze({
  ZeroFixedTimestep: "ZeroFixedTimestep",
  InvalidMaxFrameSteps: "InvalidMaxFrameSteps",
  InvalidMaxFrameDelta: "InvalidMaxFrameDelta",
  CorruptedStateInvariant: "CorruptedStateInvariant",
  TimestampOverflow: "TimestampOverflow",
} as const);

export type ClockErrorCode = (typeof ClockError)[keyof typeof ClockError];

export interface GameClockConfig {
  readonly fixed_dt_ns: bigint;
  readonly max_frame_steps: number;
  readonly max_frame_delta_ns: bigint;
}

export interface GameClockState {
  readonly tick_count: bigint;
  readonly frame_count: bigint;
  readonly elapsed_sim_ns: bigint;
  readonly elapsed_wall_ns: bigint;
  readonly accumulator_ns: bigint;
  readonly fixed_dt_ns: bigint;
  readonly max_frame_steps: number;
  readonly max_frame_delta_ns: bigint;
  readonly total_dropped_ns: bigint;
  readonly spiral_clamped_frames: bigint;
}

export interface ClockStepResult {
  readonly state: GameClockState;
  readonly steps_to_run: number;
  readonly alpha_milli: number;
  readonly alpha_q16: number;
  readonly spiral_of_death_clamped: boolean;
  readonly dropped_ns: bigint;
}

export type Result<T> =
  | Readonly<{ ok: true; value: T }>
  | Readonly<{ ok: false; error: ClockErrorCode }>;

export type ValidationResult =
  | Readonly<{ ok: true }>
  | Readonly<{ ok: false; error: ClockErrorCode }>;

export const NS_PER_SECOND = 1_000_000_000n;
export const DEFAULT_60HZ_DT_NS = 16_666_667n;
export const DEFAULT_MAX_FRAME_STEPS = 8;
export const DEFAULT_MAX_FRAME_DELTA_NS = 250_000_000n;
export const HARD_MAX_FRAME_STEPS = 1024;
export const HARD_MAX_DELTA_NS = 10_000_000_000n;
export const MAX_U64 = (1n << 64n) - 1n;
export const MAX_U32 = 0xffff_ffff;

const ok = <T>(value: T): Result<T> => Object.freeze({ ok: true, value });
const err = (error: ClockErrorCode): Result<never> => Object.freeze({ ok: false, error });

function record(value: unknown, label: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeError(`${label} must be an object record`);
  }
  return value as Record<string, unknown>;
}

function u64(value: unknown, label: string): bigint {
  if (typeof value !== "bigint") {
    throw new TypeError(`${label} must be a bigint`);
  }
  if (value < 0n || value > MAX_U64) {
    throw new RangeError(`${label} must be in 0..=u64::MAX`);
  }
  return value;
}

function u32(value: unknown, label: string): number {
  if (typeof value !== "number") {
    throw new TypeError(`${label} must be a number`);
  }
  if (!Number.isFinite(value) || !Number.isInteger(value) || value < 0 || value > MAX_U32) {
    throw new RangeError(`${label} must be a finite integer in 0..=u32::MAX`);
  }
  return value;
}

function readConfig(value: unknown): GameClockConfig {
  const input = record(value, "config");
  return {
    fixed_dt_ns: u64(input.fixed_dt_ns, "config.fixed_dt_ns"),
    max_frame_steps: u32(input.max_frame_steps, "config.max_frame_steps"),
    max_frame_delta_ns: u64(input.max_frame_delta_ns, "config.max_frame_delta_ns"),
  };
}

function readState(value: unknown): GameClockState {
  const input = record(value, "state");
  return {
    tick_count: u64(input.tick_count, "state.tick_count"),
    frame_count: u64(input.frame_count, "state.frame_count"),
    elapsed_sim_ns: u64(input.elapsed_sim_ns, "state.elapsed_sim_ns"),
    elapsed_wall_ns: u64(input.elapsed_wall_ns, "state.elapsed_wall_ns"),
    accumulator_ns: u64(input.accumulator_ns, "state.accumulator_ns"),
    fixed_dt_ns: u64(input.fixed_dt_ns, "state.fixed_dt_ns"),
    max_frame_steps: u32(input.max_frame_steps, "state.max_frame_steps"),
    max_frame_delta_ns: u64(input.max_frame_delta_ns, "state.max_frame_delta_ns"),
    total_dropped_ns: u64(input.total_dropped_ns, "state.total_dropped_ns"),
    spiral_clamped_frames: u64(input.spiral_clamped_frames, "state.spiral_clamped_frames"),
  };
}

function freezeConfig(config: GameClockConfig): GameClockConfig {
  return Object.freeze({ ...config });
}

function freezeState(state: GameClockState): GameClockState {
  return Object.freeze({ ...state });
}

function validateConfig(config: GameClockConfig): ClockErrorCode | undefined {
  if (config.fixed_dt_ns === 0n) return ClockError.ZeroFixedTimestep;
  if (config.fixed_dt_ns > HARD_MAX_DELTA_NS) return ClockError.InvalidMaxFrameDelta;
  if (config.max_frame_steps === 0 || config.max_frame_steps > HARD_MAX_FRAME_STEPS) {
    return ClockError.InvalidMaxFrameSteps;
  }
  if (
    config.max_frame_delta_ns < config.fixed_dt_ns ||
    config.max_frame_delta_ns > HARD_MAX_DELTA_NS
  ) {
    return ClockError.InvalidMaxFrameDelta;
  }
  return undefined;
}

function validateStateValues(state: GameClockState): ClockErrorCode | undefined {
  if (state.fixed_dt_ns === 0n) return ClockError.ZeroFixedTimestep;
  if (state.fixed_dt_ns > HARD_MAX_DELTA_NS) return ClockError.CorruptedStateInvariant;
  if (state.max_frame_steps === 0 || state.max_frame_steps > HARD_MAX_FRAME_STEPS) {
    return ClockError.CorruptedStateInvariant;
  }
  if (
    state.max_frame_delta_ns < state.fixed_dt_ns ||
    state.max_frame_delta_ns > HARD_MAX_DELTA_NS
  ) {
    return ClockError.CorruptedStateInvariant;
  }
  if (state.accumulator_ns >= state.fixed_dt_ns) {
    return ClockError.CorruptedStateInvariant;
  }
  const simulated = state.tick_count * state.fixed_dt_ns;
  if (simulated > MAX_U64) return ClockError.TimestampOverflow;
  if (state.elapsed_sim_ns !== simulated) return ClockError.CorruptedStateInvariant;
  return undefined;
}

function checkedAdd(left: bigint, right: bigint): bigint | undefined {
  const sum = left + right;
  return sum <= MAX_U64 ? sum : undefined;
}

/** Return a fresh frozen configuration with the GAME10 standard 60 Hz values. */
export function standard_60hz(): GameClockConfig {
  return freezeConfig({
    fixed_dt_ns: DEFAULT_60HZ_DT_NS,
    max_frame_steps: DEFAULT_MAX_FRAME_STEPS,
    max_frame_delta_ns: DEFAULT_MAX_FRAME_DELTA_NS,
  });
}

/** Construct a fixed-step configuration from a bounded frequency and step limit. */
export function from_hz(hzValue: number, maxStepsValue: number): Result<GameClockConfig> {
  const hz = u32(hzValue, "hz");
  const maxSteps = u32(maxStepsValue, "max_frame_steps");
  if (hz === 0) return err(ClockError.ZeroFixedTimestep);
  if (maxSteps === 0 || maxSteps > HARD_MAX_FRAME_STEPS) {
    return err(ClockError.InvalidMaxFrameSteps);
  }

  const fixedDt = NS_PER_SECOND / BigInt(hz);
  if (fixedDt === 0n) return err(ClockError.ZeroFixedTimestep);
  if (fixedDt > HARD_MAX_DELTA_NS) return err(ClockError.InvalidMaxFrameDelta);

  const config = freezeConfig({
    fixed_dt_ns: fixedDt,
    max_frame_steps: maxSteps,
    max_frame_delta_ns:
      DEFAULT_MAX_FRAME_DELTA_NS < fixedDt ? fixedDt : DEFAULT_MAX_FRAME_DELTA_NS,
  });
  const error = validateConfig(config);
  return error === undefined ? ok(config) : err(error);
}

/** Create a new zeroed state, copying the valid configuration into a fresh frozen record. */
export function new_clock(configValue: GameClockConfig): Result<GameClockState> {
  const config = readConfig(configValue);
  const error = validateConfig(config);
  if (error !== undefined) return err(error);
  return ok(
    freezeState({
      tick_count: 0n,
      frame_count: 0n,
      elapsed_sim_ns: 0n,
      elapsed_wall_ns: 0n,
      accumulator_ns: 0n,
      fixed_dt_ns: config.fixed_dt_ns,
      max_frame_steps: config.max_frame_steps,
      max_frame_delta_ns: config.max_frame_delta_ns,
      total_dropped_ns: 0n,
      spiral_clamped_frames: 0n,
    }),
  );
}

/** Validate a state using GAME10's exact first-error precedence. */
export function validate_state(stateValue: GameClockState): ValidationResult {
  const state = readState(stateValue);
  const error = validateStateValues(state);
  return error === undefined ? Object.freeze({ ok: true }) : err(error);
}

/** Advance one fixed-step frame without mutating the caller-owned state. */
export function advance_frame(
  stateValue: GameClockState,
  rawDeltaValue: bigint,
): Result<ClockStepResult> {
  const state = readState(stateValue);
  const rawDelta = u64(rawDeltaValue, "raw_frame_delta_ns");
  const stateError = validateStateValues(state);
  if (stateError !== undefined) return err(stateError);

  const clamped = rawDelta > state.max_frame_delta_ns ? state.max_frame_delta_ns : rawDelta;
  const wallDrop = rawDelta - clamped;
  const accumulated = state.accumulator_ns + clamped;
  if (accumulated > MAX_U64) return err(ClockError.TimestampOverflow);

  // Quotient/remainder gives a constant-operation step decision even when the
  // configured limit is large. Only the admitted prefix becomes simulation;
  // every excess whole tick is drained and accounted as dropped time.
  const wholeSteps = accumulated / state.fixed_dt_ns;
  const remaining = accumulated % state.fixed_dt_ns;
  const spiral = wholeSteps > BigInt(state.max_frame_steps);
  const steps = spiral ? state.max_frame_steps : Number(wholeSteps);
  const spiralDrop = spiral
    ? (wholeSteps - BigInt(state.max_frame_steps)) * state.fixed_dt_ns
    : 0n;

  const tickCount = checkedAdd(state.tick_count, BigInt(steps));
  if (tickCount === undefined) return err(ClockError.TimestampOverflow);
  const frameCount = checkedAdd(state.frame_count, 1n);
  if (frameCount === undefined) return err(ClockError.TimestampOverflow);
  const simAdvance = BigInt(steps) * state.fixed_dt_ns;
  if (simAdvance > MAX_U64) return err(ClockError.TimestampOverflow);
  const elapsedSim = checkedAdd(state.elapsed_sim_ns, simAdvance);
  if (elapsedSim === undefined) return err(ClockError.TimestampOverflow);
  const elapsedWall = checkedAdd(state.elapsed_wall_ns, clamped);
  if (elapsedWall === undefined) return err(ClockError.TimestampOverflow);
  const dropped = checkedAdd(wallDrop, spiralDrop);
  if (dropped === undefined) return err(ClockError.TimestampOverflow);
  const totalDropped = checkedAdd(state.total_dropped_ns, dropped);
  if (totalDropped === undefined) return err(ClockError.TimestampOverflow);
  const spiralFrames = checkedAdd(state.spiral_clamped_frames, spiral ? 1n : 0n);
  if (spiralFrames === undefined) return err(ClockError.TimestampOverflow);

  const nextState = freezeState({
    tick_count: tickCount,
    frame_count: frameCount,
    elapsed_sim_ns: elapsedSim,
    elapsed_wall_ns: elapsedWall,
    accumulator_ns: remaining,
    fixed_dt_ns: state.fixed_dt_ns,
    max_frame_steps: state.max_frame_steps,
    max_frame_delta_ns: state.max_frame_delta_ns,
    total_dropped_ns: totalDropped,
    spiral_clamped_frames: spiralFrames,
  });
  const step = Object.freeze({
    state: nextState,
    steps_to_run: steps,
    alpha_milli: Number((remaining * 1000n) / state.fixed_dt_ns),
    alpha_q16: Number((remaining * 65536n) / state.fixed_dt_ns),
    spiral_of_death_clamped: spiral,
    dropped_ns: dropped,
  });
  return ok(step);
}
