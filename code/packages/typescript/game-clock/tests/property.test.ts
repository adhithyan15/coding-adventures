import assert from "node:assert/strict";
import test from "node:test";

import {
  ClockError,
  HARD_MAX_DELTA_NS,
  MAX_U64,
  advance_frame,
  new_clock,
  validate_state,
  type ClockStepResult,
  type GameClockConfig,
  type GameClockState,
  type Result,
} from "../src/index.js";
import {
  SplitMix64,
  checkProperty,
  genBoundedInt,
  genU64,
  mutateSequence,
} from "@coding-adventures/test-bootstrap";

function assertOk<T>(result: Result<T>): T {
  if (!result.ok) throw new Error(`unexpected error ${result.error}`);
  return result.value;
}

type OracleResult =
  | { ok: true; value: ClockStepResult }
  | { ok: false; error: string };

function checkedOracleAdd(left: bigint, right: bigint): bigint | undefined {
  const sum = left + right;
  return sum <= MAX_U64 ? sum : undefined;
}

/** Independent constructor oracle used to check exact refusals in the mutation campaign. */
function oracleNewClock(config: GameClockConfig): Result<GameClockState> {
  if (config.fixed_dt_ns === 0n) return { ok: false, error: ClockError.ZeroFixedTimestep };
  if (config.fixed_dt_ns > HARD_MAX_DELTA_NS) {
    return { ok: false, error: ClockError.InvalidMaxFrameDelta };
  }
  if (config.max_frame_steps === 0 || config.max_frame_steps > 1024) {
    return { ok: false, error: ClockError.InvalidMaxFrameSteps };
  }
  if (config.max_frame_delta_ns < config.fixed_dt_ns || config.max_frame_delta_ns > HARD_MAX_DELTA_NS) {
    return { ok: false, error: ClockError.InvalidMaxFrameDelta };
  }
  return {
    ok: true,
    value: {
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
    },
  };
}

/** Independent bounded-subtraction oracle; its step loop is capped by max_frame_steps. */
function oracleAdvance(state: GameClockState, raw: bigint): OracleResult {
  const clamped = raw > state.max_frame_delta_ns ? state.max_frame_delta_ns : raw;
  const wallDrop = raw - clamped;
  let remainder = state.accumulator_ns + clamped;
  if (remainder > MAX_U64) return { ok: false, error: ClockError.TimestampOverflow };

  let steps = 0;
  while (steps < state.max_frame_steps && remainder >= state.fixed_dt_ns) {
    remainder -= state.fixed_dt_ns;
    steps += 1;
  }
  const spiral = remainder >= state.fixed_dt_ns;
  let spiralDrop = 0n;
  if (spiral) {
    const excess = remainder / state.fixed_dt_ns;
    spiralDrop = excess * state.fixed_dt_ns;
    remainder -= spiralDrop;
  }

  const tick = checkedOracleAdd(state.tick_count, BigInt(steps));
  if (tick === undefined) return { ok: false, error: ClockError.TimestampOverflow };
  const frame = checkedOracleAdd(state.frame_count, 1n);
  if (frame === undefined) return { ok: false, error: ClockError.TimestampOverflow };
  const simulatedDelta = BigInt(steps) * state.fixed_dt_ns;
  if (simulatedDelta > MAX_U64) return { ok: false, error: ClockError.TimestampOverflow };
  const elapsedSim = checkedOracleAdd(state.elapsed_sim_ns, simulatedDelta);
  if (elapsedSim === undefined) return { ok: false, error: ClockError.TimestampOverflow };
  const elapsedWall = checkedOracleAdd(state.elapsed_wall_ns, clamped);
  if (elapsedWall === undefined) return { ok: false, error: ClockError.TimestampOverflow };
  const dropped = checkedOracleAdd(wallDrop, spiralDrop);
  if (dropped === undefined) return { ok: false, error: ClockError.TimestampOverflow };
  const totalDropped = checkedOracleAdd(state.total_dropped_ns, dropped);
  if (totalDropped === undefined) return { ok: false, error: ClockError.TimestampOverflow };
  const spiralFrames = checkedOracleAdd(state.spiral_clamped_frames, spiral ? 1n : 0n);
  if (spiralFrames === undefined) return { ok: false, error: ClockError.TimestampOverflow };

  return {
    ok: true,
    value: {
      state: Object.freeze({
        tick_count: tick,
        frame_count: frame,
        elapsed_sim_ns: elapsedSim,
        elapsed_wall_ns: elapsedWall,
        accumulator_ns: remainder,
        fixed_dt_ns: state.fixed_dt_ns,
        max_frame_steps: state.max_frame_steps,
        max_frame_delta_ns: state.max_frame_delta_ns,
        total_dropped_ns: totalDropped,
        spiral_clamped_frames: spiralFrames,
      }),
      steps_to_run: steps,
      alpha_milli: Number((remainder * 1000n) / state.fixed_dt_ns),
      alpha_q16: Number((remainder * 65536n) / state.fixed_dt_ns),
      spiral_of_death_clamped: spiral,
      dropped_ns: dropped,
    },
  };
}

interface GeneratedFrame {
  readonly stratum: number;
  readonly config: GameClockConfig;
  readonly raw: bigint;
  readonly traceLength: number;
  readonly traceRaw: readonly bigint[];
}

const PROPERTY_SEED = 0x47434c4b5f545350n;
const PROPERTY_CASES = 1024;

function randomConfig(rng: SplitMix64): GameClockConfig {
  const dt = genBoundedInt(rng, 1n, HARD_MAX_DELTA_NS);
  const maxDelta = genBoundedInt(rng, dt, HARD_MAX_DELTA_NS);
  const steps = Number(genBoundedInt(rng, 1n, 1024n));
  return { fixed_dt_ns: dt, max_frame_steps: steps, max_frame_delta_ns: maxDelta };
}

test(`seeded GAME10 properties: seed=0x${PROPERTY_SEED.toString(16)}, cases=${PROPERTY_CASES}, strata=8`, (t) => {
  const stratumCounts = Array<number>(8).fill(0);
  let generated = 0;
  const verdict = checkProperty<GeneratedFrame>(
    PROPERTY_SEED,
    { numTests: PROPERTY_CASES, maxShrinkSteps: 128, isCi: false },
    (rng) => {
      const stratum = generated % 8;
      generated += 1;
      stratumCounts[stratum] += 1;
      let cfg = randomConfig(rng);
      let raw: bigint;
      if (stratum === 2 || stratum === 3) {
        const dt = genBoundedInt(rng, 1n, 1000n);
        const steps = Number(genBoundedInt(rng, 1n, 8n));
        cfg = { fixed_dt_ns: dt, max_frame_steps: steps, max_frame_delta_ns: HARD_MAX_DELTA_NS };
        raw = dt * BigInt(steps + (stratum === 3 ? 1 : 0));
      } else if (stratum === 4) {
        cfg = { fixed_dt_ns: 1n, max_frame_steps: 1, max_frame_delta_ns: 10_000_000_000n };
        raw = MAX_U64;
      } else if (stratum === 6) {
        cfg = { fixed_dt_ns: HARD_MAX_DELTA_NS, max_frame_steps: 1024, max_frame_delta_ns: HARD_MAX_DELTA_NS };
        raw = genU64(rng);
      } else {
        raw = stratum === 0 ? genU64(rng) : genBoundedInt(rng, 0n, cfg.max_frame_delta_ns + 1n);
      }
      const traceLength = stratum >= 2 && stratum <= 5 ? 1 : 1 + rng.bounded(4);
      const traceRaw = stratum === 2 || stratum === 3 || stratum === 4 || stratum === 5
        ? [raw]
        : Array.from({ length: traceLength }, () => {
          return stratum === 0 || stratum === 6
            ? genU64(rng)
            : genBoundedInt(rng, 0n, cfg.max_frame_delta_ns + 1n);
        });
      return { stratum, config: cfg, raw, traceLength, traceRaw };
    },
    (sample) => {
      const initial = assertOk(new_clock(sample.config));
      assert.equal(validate_state(initial).ok, true);
      assert.equal(sample.traceRaw.length, sample.traceLength);

      let current = initial;
      let cumulativeRaw = 0n;
      for (const raw of sample.traceRaw) {
        const before = structuredClone(current);
        const expected = oracleAdvance(current, raw);
        const actual = advance_frame(current, raw);
        if (!expected.ok) {
          assert.deepStrictEqual(actual, { ok: false, error: expected.error });
          assert.deepStrictEqual(current, before, "a refusal leaves the input state unchanged");
          break;
        }
        if (!actual.ok) throw new Error(`native result refused with ${actual.error}`);
        assert.deepStrictEqual(actual.value, expected.value);
        assert.equal(
          before.accumulator_ns + raw,
          BigInt(actual.value.steps_to_run) * before.fixed_dt_ns +
            actual.value.dropped_ns +
            actual.value.state.accumulator_ns,
        );
        cumulativeRaw += raw;
        current = actual.value.state;
        assert.equal(validate_state(current).ok, true);
        assert.equal(
          current.elapsed_sim_ns + current.accumulator_ns + current.total_dropped_ns,
          cumulativeRaw,
          "cumulative conservation holds from a state created by new_clock",
        );
      }
      return true;
    },
  );
  assert.equal(verdict.status, "pass", JSON.stringify(verdict));
  assert.equal(generated, PROPERTY_CASES);
  assert.deepStrictEqual(stratumCounts, Array<number>(8).fill(PROPERTY_CASES / 8));
  t.diagnostic(`seed=${PROPERTY_SEED}; generated=${generated}; stratum_counts=${stratumCounts.join(",")}; verdict=${verdict.status}`);
});

test("seeded invalid-config and invalid-state strata keep exact first-error precedence", () => {
  const expectedConfigErrors = [
    ClockError.ZeroFixedTimestep,
    ClockError.InvalidMaxFrameDelta,
    ClockError.InvalidMaxFrameSteps,
    ClockError.InvalidMaxFrameDelta,
  ];
  const configSamples: GameClockConfig[] = [
    { fixed_dt_ns: 0n, max_frame_steps: 0, max_frame_delta_ns: 0n },
    { fixed_dt_ns: HARD_MAX_DELTA_NS + 1n, max_frame_steps: 0, max_frame_delta_ns: 0n },
    { fixed_dt_ns: 2n, max_frame_steps: 0, max_frame_delta_ns: 1n },
    { fixed_dt_ns: 2n, max_frame_steps: 1, max_frame_delta_ns: 1n },
  ];
  configSamples.forEach((sample, index) => {
    const before = structuredClone(sample);
    assert.deepStrictEqual(new_clock(sample), { ok: false, error: expectedConfigErrors[index] });
    assert.deepStrictEqual(sample, before);
  });

  const invalidState: GameClockState = {
    tick_count: MAX_U64,
    frame_count: 0n,
    elapsed_sim_ns: 0n,
    elapsed_wall_ns: 0n,
    accumulator_ns: 2n,
    fixed_dt_ns: 2n,
    max_frame_steps: 0,
    max_frame_delta_ns: 1n,
    total_dropped_ns: 0n,
    spiral_clamped_frames: 0n,
  };
  assert.deepStrictEqual(validate_state(invalidState), {
    ok: false,
    error: ClockError.CorruptedStateInvariant,
  });
  assert.deepStrictEqual(advance_frame(invalidState, MAX_U64), {
    ok: false,
    error: ClockError.CorruptedStateInvariant,
  });
});

function readWord(bytes: Uint8Array, offset: number): bigint {
  const padded = new Uint8Array(32);
  padded.set(bytes.subarray(0, 32));
  return new DataView(padded.buffer).getBigUint64(offset, false);
}

function readStepCount(bytes: Uint8Array): number {
  const padded = new Uint8Array(32);
  padded.set(bytes.subarray(0, 32));
  return new DataView(padded.buffer).getUint32(8, false);
}

test("bounded seeded mutation campaign: 256 cases, 8 mutations, 32-byte interpreted input", (t) => {
  const seed = 0x47434c4b5f46555an;
  const rng = new SplitMix64(seed);
  const seedCorpus = Array.from({ length: 8 }, () => {
    const bytes = new Uint8Array(32);
    const view = new DataView(bytes.buffer);
    view.setBigUint64(0, rng.nextU64(), false);
    view.setUint32(8, Number(rng.nextU64() & 0xffff_ffffn), false);
    view.setBigUint64(12, rng.nextU64(), false);
    view.setBigUint64(20, rng.nextU64(), false);
    return bytes;
  });
  const refusalCounts = { constructor: 0, advance: 0 };
  const acceptCounts = { constructor: 0, advance: 0 };

  for (let caseIndex = 0; caseIndex < 256; caseIndex += 1) {
    const base = seedCorpus[caseIndex % seedCorpus.length];
    const mutated = mutateSequence(base, seed + BigInt(caseIndex), 8);
    let dt = readWord(mutated, 0);
    let steps = readStepCount(mutated);
    let maxDelta = readWord(mutated, 12);
    if (caseIndex % 2 === 0) {
      dt = 1n + (dt % HARD_MAX_DELTA_NS);
      maxDelta = dt + (maxDelta % (HARD_MAX_DELTA_NS - dt + 1n));
      steps = 1 + (steps % 1024);
    }
    const cfg: GameClockConfig = {
      fixed_dt_ns: dt,
      max_frame_steps: steps,
      max_frame_delta_ns: maxDelta,
    };
    const beforeConfig = structuredClone(cfg);
    const expectedStart = oracleNewClock(cfg);
    const start = new_clock(cfg);
    assert.deepStrictEqual(start, expectedStart, `constructor mutation case ${caseIndex}`);
    assert.deepStrictEqual(cfg, beforeConfig);
    if (!start.ok) {
      refusalCounts.constructor += 1;
      continue;
    }
    acceptCounts.constructor += 1;
    const raw = readWord(mutated, 20);
    const prior = structuredClone(start.value);
    const expected = oracleAdvance(start.value, raw);
    const actual = advance_frame(start.value, raw);
    if (!expected.ok) {
      refusalCounts.advance += 1;
      assert.deepStrictEqual(actual, { ok: false, error: expected.error });
      assert.deepStrictEqual(start.value, prior);
    } else {
      acceptCounts.advance += 1;
      assert.deepStrictEqual(actual, { ok: true, value: expected.value });
      assert.deepStrictEqual(start.value, prior);
    }
  }

  assert.equal(acceptCounts.constructor + refusalCounts.constructor, 256);
  assert.deepStrictEqual(acceptCounts, { constructor: 128, advance: 128 });
  assert.deepStrictEqual(refusalCounts, { constructor: 128, advance: 0 });
  t.diagnostic(`seed=${seed}; cases=256; bytes_interpreted<=32; mutations_per_case=8; constructor=${JSON.stringify({ accept: acceptCounts.constructor, refuse: refusalCounts.constructor })}; advance=${JSON.stringify({ accept: acceptCounts.advance, refuse: refusalCounts.advance })}`);
});
