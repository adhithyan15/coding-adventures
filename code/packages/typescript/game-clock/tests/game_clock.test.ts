import assert from "node:assert/strict";
import test from "node:test";

import {
  ClockError,
  DEFAULT_60HZ_DT_NS,
  DEFAULT_MAX_FRAME_DELTA_NS,
  DEFAULT_MAX_FRAME_STEPS,
  HARD_MAX_DELTA_NS,
  HARD_MAX_FRAME_STEPS,
  MAX_U32,
  MAX_U64,
  advance_frame,
  from_hz,
  new_clock,
  standard_60hz,
  validate_state,
  type GameClockConfig,
  type GameClockState,
  type Result,
} from "../src/index.js";

function unwrap<T>(result: Result<T>): T {
  if (!result.ok) throw new Error(`unexpected ClockError ${result.error}`);
  return result.value;
}

function config(
  fixed_dt_ns = 2n,
  max_frame_steps = 1,
  max_frame_delta_ns = 2n,
): GameClockConfig {
  return { fixed_dt_ns, max_frame_steps, max_frame_delta_ns };
}

function state(overrides: Partial<GameClockState> = {}): GameClockState {
  return {
    tick_count: 0n,
    frame_count: 0n,
    elapsed_sim_ns: 0n,
    elapsed_wall_ns: 0n,
    accumulator_ns: 0n,
    fixed_dt_ns: 2n,
    max_frame_steps: 1,
    max_frame_delta_ns: 2n,
    total_dropped_ns: 0n,
    spiral_clamped_frames: 0n,
    ...overrides,
  };
}

test("standard and frequency constructors preserve exact integer rounding and freeze fresh records", () => {
  const standardA = standard_60hz();
  const standardB = standard_60hz();
  assert.deepStrictEqual(standardA, {
    fixed_dt_ns: DEFAULT_60HZ_DT_NS,
    max_frame_steps: DEFAULT_MAX_FRAME_STEPS,
    max_frame_delta_ns: DEFAULT_MAX_FRAME_DELTA_NS,
  });
  assert.notStrictEqual(standardA, standardB);
  assert.ok(Object.isFrozen(standardA));

  const fromHz60 = unwrap(from_hz(60, 8));
  assert.equal(fromHz60.fixed_dt_ns, 16_666_666n);
  assert.equal(fromHz60.max_frame_steps, 8);
  assert.equal(fromHz60.max_frame_delta_ns, 250_000_000n);
  assert.ok(Object.isFrozen(fromHz60));

  const fromHzOneBillion = unwrap(from_hz(1_000_000_000, 1));
  assert.equal(fromHzOneBillion.fixed_dt_ns, 1n);
  assert.equal(fromHzOneBillion.max_frame_delta_ns, 250_000_000n);
});

test("frequency constructor validates zero, step range, then integer quotient", () => {
  assert.deepStrictEqual(from_hz(0, 0), { ok: false, error: ClockError.ZeroFixedTimestep });
  assert.deepStrictEqual(
    from_hz(1_000_000_001, 0),
    { ok: false, error: ClockError.InvalidMaxFrameSteps },
  );
  assert.deepStrictEqual(
    from_hz(1_000_000_001, 1),
    { ok: false, error: ClockError.ZeroFixedTimestep },
  );
  assert.deepStrictEqual(
    from_hz(60, HARD_MAX_FRAME_STEPS + 1),
    { ok: false, error: ClockError.InvalidMaxFrameSteps },
  );
  assert.equal(from_hz(60, MAX_U32).ok, false);
  assert.deepStrictEqual(from_hz(-0, 0), { ok: false, error: ClockError.ZeroFixedTimestep });
});

test("host widths are checked without coercion, and host faults precede semantic refusals", () => {
  assert.throws(() => from_hz("60" as unknown as number, 8), TypeError);
  assert.throws(() => from_hz(60, 8n as unknown as number), TypeError);
  assert.throws(() => from_hz(1.5, 1), RangeError);
  assert.throws(() => from_hz(Number.POSITIVE_INFINITY, 1), RangeError);
  assert.throws(() => from_hz(-1, 1), RangeError);
  assert.throws(() => from_hz(1, MAX_U32 + 1), RangeError);
  assert.throws(() => new_clock(config(-1n)), RangeError);
  assert.throws(() => new_clock(config(1n, 1.25)), RangeError);
  assert.throws(() => new_clock(config(1n, 1, Number.MAX_SAFE_INTEGER as unknown as bigint)), TypeError);
  assert.throws(() => advance_frame(state(), 1 as unknown as bigint), TypeError);
  assert.throws(() => advance_frame(state(), MAX_U64 + 1n), RangeError);

  const semanticallyBad = state({ accumulator_ns: 2n });
  assert.throws(() => advance_frame(semanticallyBad, -1n), RangeError);
  assert.deepStrictEqual(validate_state(semanticallyBad), {
    ok: false,
    error: ClockError.CorruptedStateInvariant,
  });
});

test("new_clock keeps configuration refusal precedence and copies a successful config", () => {
  assert.deepStrictEqual(
    new_clock(config(0n, 0, 0n)),
    { ok: false, error: ClockError.ZeroFixedTimestep },
  );
  assert.deepStrictEqual(
    new_clock(config(HARD_MAX_DELTA_NS + 1n, 0, 0n)),
    { ok: false, error: ClockError.InvalidMaxFrameDelta },
  );
  assert.deepStrictEqual(
    new_clock(config(2n, 0, 1n)),
    { ok: false, error: ClockError.InvalidMaxFrameSteps },
  );
  assert.deepStrictEqual(
    new_clock(config(2n, 1, 1n)),
    { ok: false, error: ClockError.InvalidMaxFrameDelta },
  );

  const input = config(3n, 2, 9n);
  const before = structuredClone(input);
  const first = unwrap(new_clock(input));
  const second = unwrap(new_clock(input));
  assert.equal(input.fixed_dt_ns, before.fixed_dt_ns);
  assert.deepStrictEqual(first, {
    tick_count: 0n,
    frame_count: 0n,
    elapsed_sim_ns: 0n,
    elapsed_wall_ns: 0n,
    accumulator_ns: 0n,
    fixed_dt_ns: 3n,
    max_frame_steps: 2,
    max_frame_delta_ns: 9n,
    total_dropped_ns: 0n,
    spiral_clamped_frames: 0n,
  });
  assert.notStrictEqual(first, second);
  assert.ok(Object.isFrozen(first));
});

test("state validation preserves exact precedence and accepts documented unconstrained counters", () => {
  assert.deepStrictEqual(validate_state(state({ fixed_dt_ns: 0n, max_frame_steps: 0 })), {
    ok: false,
    error: ClockError.ZeroFixedTimestep,
  });
  assert.deepStrictEqual(validate_state(state({ fixed_dt_ns: HARD_MAX_DELTA_NS + 1n, max_frame_steps: 0 })), {
    ok: false,
    error: ClockError.CorruptedStateInvariant,
  });
  assert.deepStrictEqual(validate_state(state({ max_frame_steps: 0, max_frame_delta_ns: 1n })), {
    ok: false,
    error: ClockError.CorruptedStateInvariant,
  });
  assert.deepStrictEqual(
    validate_state(state({ accumulator_ns: 2n, tick_count: MAX_U64, fixed_dt_ns: 2n })),
    { ok: false, error: ClockError.CorruptedStateInvariant },
  );
  assert.deepStrictEqual(validate_state(state({ tick_count: MAX_U64, fixed_dt_ns: 2n })), {
    ok: false,
    error: ClockError.TimestampOverflow,
  });
  assert.deepStrictEqual(validate_state(state({ tick_count: 1n, elapsed_sim_ns: 1n })), {
    ok: false,
    error: ClockError.CorruptedStateInvariant,
  });

  const arbitraryCounters = state({
    tick_count: 1n,
    elapsed_sim_ns: 2n,
    fixed_dt_ns: 2n,
    elapsed_wall_ns: MAX_U64,
    total_dropped_ns: MAX_U64,
    spiral_clamped_frames: MAX_U64,
  });
  assert.deepStrictEqual(validate_state(arbitraryCounters), { ok: true });
});

test("advance is atomic, uses strict step-limit clamping, and returns frozen complete records", () => {
  const initial = state({ fixed_dt_ns: 2n, max_frame_steps: 1, max_frame_delta_ns: 5n });
  const before = structuredClone(initial);
  const spirals = unwrap(advance_frame(initial, 5n));
  assert.deepStrictEqual(initial, before);
  assert.deepStrictEqual(spirals, {
    state: {
      tick_count: 1n,
      frame_count: 1n,
      elapsed_sim_ns: 2n,
      elapsed_wall_ns: 5n,
      accumulator_ns: 1n,
      fixed_dt_ns: 2n,
      max_frame_steps: 1,
      max_frame_delta_ns: 5n,
      total_dropped_ns: 2n,
      spiral_clamped_frames: 1n,
    },
    steps_to_run: 1,
    alpha_milli: 500,
    alpha_q16: 32768,
    spiral_of_death_clamped: true,
    dropped_ns: 2n,
  });
  assert.ok(Object.isFrozen(spirals));
  assert.ok(Object.isFrozen(spirals.state));

  const exactLimit = unwrap(advance_frame(initial, 3n));
  assert.equal(exactLimit.spiral_of_death_clamped, false);
  assert.equal(exactLimit.state.accumulator_ns, 1n);
  assert.equal(exactLimit.dropped_ns, 0n);

  const invalid = state({ accumulator_ns: 2n });
  const invalidBefore = structuredClone(invalid);
  assert.deepStrictEqual(advance_frame(invalid, 1n), {
    ok: false,
    error: ClockError.CorruptedStateInvariant,
  });
  assert.deepStrictEqual(invalid, invalidBefore);
});

test("full-width conservation and counter overflow match the native edge cases", () => {
  const first = unwrap(advance_frame(state({ fixed_dt_ns: 2n, max_frame_delta_ns: 2n }), 1n));
  assert.equal(first.state.accumulator_ns, 1n);
  const reachable = first.state;
  const before = structuredClone(reachable);
  const maxDelta = unwrap(advance_frame(reachable, MAX_U64));
  assert.deepStrictEqual(reachable, before);
  assert.deepStrictEqual(maxDelta.state, {
    tick_count: 1n,
    frame_count: 2n,
    elapsed_sim_ns: 2n,
    elapsed_wall_ns: 3n,
    accumulator_ns: 1n,
    fixed_dt_ns: 2n,
    max_frame_steps: 1,
    max_frame_delta_ns: 2n,
    total_dropped_ns: MAX_U64 - 2n,
    spiral_clamped_frames: 0n,
  });
  assert.equal(maxDelta.dropped_ns, MAX_U64 - 2n);
  assert.equal(
    before.accumulator_ns + MAX_U64,
    BigInt(maxDelta.steps_to_run) * before.fixed_dt_ns + maxDelta.dropped_ns + maxDelta.state.accumulator_ns,
  );

  const atDropBoundary = state({
    accumulator_ns: 1n,
    total_dropped_ns: 2n,
    frame_count: 1n,
    elapsed_wall_ns: 1n,
  });
  const exactMaxTotal = unwrap(advance_frame(atDropBoundary, MAX_U64));
  assert.equal(exactMaxTotal.state.total_dropped_ns, MAX_U64);
  const overflowingDrop = state({ ...atDropBoundary, total_dropped_ns: 3n });
  const overflowBefore = structuredClone(overflowingDrop);
  assert.deepStrictEqual(advance_frame(overflowingDrop, MAX_U64), {
    ok: false,
    error: ClockError.TimestampOverflow,
  });
  assert.deepStrictEqual(overflowingDrop, overflowBefore);

  const countersAtMax = state({
    tick_count: MAX_U64,
    frame_count: 0n,
    elapsed_sim_ns: MAX_U64,
    elapsed_wall_ns: MAX_U64,
    total_dropped_ns: MAX_U64,
    spiral_clamped_frames: MAX_U64,
    fixed_dt_ns: 1n,
    max_frame_delta_ns: 1n,
  });
  const zeroFrame = unwrap(advance_frame(countersAtMax, 0n));
  assert.equal(zeroFrame.state.frame_count, 1n);
  assert.equal(zeroFrame.state.tick_count, MAX_U64);
  assert.equal(zeroFrame.state.elapsed_wall_ns, MAX_U64);
  assert.equal(zeroFrame.state.total_dropped_ns, MAX_U64);
  assert.equal(zeroFrame.state.spiral_clamped_frames, MAX_U64);
  assert.deepStrictEqual(
    advance_frame(state({ frame_count: MAX_U64, fixed_dt_ns: 1n, max_frame_delta_ns: 1n }), 0n),
    { ok: false, error: ClockError.TimestampOverflow },
  );
});
