import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import {
  advance_frame,
  from_hz,
  new_clock,
  standard_60hz,
  validate_state,
  type ClockStepResult,
  type GameClockConfig,
  type GameClockState,
  type Result,
} from "../src/index.js";

export interface FixtureCase {
  readonly id: string;
  readonly op: string;
  readonly input: Record<string, unknown>;
  readonly expected: Record<string, unknown>;
}

export interface GameClockFixture {
  readonly version: number;
  readonly suite: string;
  readonly cases: FixtureCase[];
}

const fixtureRelative = path.join("code", "specs", "fixtures", "game-clock-v1");

const canonicalV1CaseIds = new Set([
  "standard_60hz_rounding",
  "from_hz_60_rounding",
  "from_hz_zero",
  "new_clock_success",
  "new_clock_zero_precedes_other_errors",
  "new_clock_step_error_precedes_delta_error",
  "validate_zero_precedence",
  "validate_tick_product_overflow",
  "validate_accumulator_precedes_tick_product_overflow",
  "validate_elapsed_sim_mismatch",
  "validate_arbitrary_counters_at_u64_max",
  "advance_fractional_alpha_floors",
  "advance_exact_step_limit_is_not_clamped",
  "advance_wall_and_spiral_drops",
  "advance_u64_max_raw_delta",
  "advance_zero_delta_preserves_nonincremented_max_counters",
  "advance_validates_state_before_raw_delta",
  "advance_frame_count_overflow",
  "advance_tick_count_overflow",
  "advance_elapsed_sim_overflow",
  "advance_elapsed_wall_overflow",
  "advance_total_dropped_overflow",
  "advance_spiral_counter_overflow",
]);

export function findRepositoryRoot(start = path.dirname(fileURLToPath(import.meta.url))): string {
  let current = start;
  for (let depth = 0; depth < 12; depth += 1) {
    if (fs.existsSync(path.join(current, fixtureRelative, "cases.json"))) return current;
    const parent = path.dirname(current);
    if (parent === current) break;
    current = parent;
  }
  throw new Error(`could not find repository root above ${start}`);
}

function pythonCommand(): { executable: string; prefix: string[] } {
  const candidates = process.platform === "win32"
    ? [{ executable: "python", prefix: [] }, { executable: "py", prefix: ["-3"] }]
    : [{ executable: "python3", prefix: [] }, { executable: "python", prefix: [] }];
  for (const candidate of candidates) {
    const probe = spawnSync(candidate.executable, [...candidate.prefix, "--version"], {
      encoding: "utf8",
    });
    if (probe.error === undefined && probe.status === 0) return candidate;
  }
  throw new Error("GAME10 tests require the repository's Python 3 fixture validator");
}

export interface ValidatorRun {
  readonly status: number | null;
  readonly stdout: string;
  readonly stderr: string;
}

/** Run the actual Python validator against a byte-identical adjacent cases.json. */
export function runPythonValidator(rawCases: Buffer): ValidatorRun {
  const root = findRepositoryRoot();
  const fixtureDir = path.join(root, fixtureRelative);
  const scratch = fs.mkdtempSync(path.join(os.tmpdir(), "game-clock-validator-"));
  const validatorPath = path.join(fixtureDir, "validate.py");
  const localValidator = path.join(scratch, "validate.py");
  try {
    fs.copyFileSync(validatorPath, localValidator);
    fs.writeFileSync(path.join(scratch, "cases.json"), rawCases);
    const command = pythonCommand();
    const run = spawnSync(command.executable, [...command.prefix, localValidator], {
      cwd: scratch,
      encoding: "utf8",
    });
    if (run.error !== undefined) throw run.error;
    return {
      status: run.status,
      stdout: run.stdout,
      stderr: run.stderr,
    };
  } finally {
    fs.rmSync(scratch, { recursive: true, force: true });
  }
}

export function loadValidatedFixture(): { fixture: GameClockFixture; raw: Buffer } {
  const fixturePath = path.join(findRepositoryRoot(), fixtureRelative, "cases.json");
  const raw = fs.readFileSync(fixturePath);
  const validation = runPythonValidator(raw);
  assert.equal(validation.status, 0, validation.stderr);
  assert.match(validation.stdout, /23 cases, 9 accepted, 14 rejected/);

  // The original bytes have passed Python's duplicate-key, canonical-u64, and
  // integer-kind checks before JavaScript's parser ever sees the document.
  const fixture = JSON.parse(raw.toString("utf8")) as GameClockFixture;
  return { fixture, raw };
}

function asU64(input: Record<string, unknown>, key: string): bigint {
  const raw = input[key];
  assert.equal(typeof raw, "string", `${key} is a canonical u64 string`);
  assert.match(raw as string, /^(0|[1-9][0-9]*)$/);
  return BigInt(raw as string);
}

function configFromFixture(value: unknown): GameClockConfig {
  const config = value as Record<string, unknown>;
  return {
    fixed_dt_ns: asU64(config, "fixed_dt_ns"),
    max_frame_steps: config.max_frame_steps as number,
    max_frame_delta_ns: asU64(config, "max_frame_delta_ns"),
  };
}

function stateFromFixture(value: unknown): GameClockState {
  const state = value as Record<string, unknown>;
  return {
    tick_count: asU64(state, "tick_count"),
    frame_count: asU64(state, "frame_count"),
    elapsed_sim_ns: asU64(state, "elapsed_sim_ns"),
    elapsed_wall_ns: asU64(state, "elapsed_wall_ns"),
    accumulator_ns: asU64(state, "accumulator_ns"),
    fixed_dt_ns: asU64(state, "fixed_dt_ns"),
    max_frame_steps: state.max_frame_steps as number,
    max_frame_delta_ns: asU64(state, "max_frame_delta_ns"),
    total_dropped_ns: asU64(state, "total_dropped_ns"),
    spiral_clamped_frames: asU64(state, "spiral_clamped_frames"),
  };
}

function configToFixture(config: GameClockConfig): Record<string, unknown> {
  return {
    fixed_dt_ns: config.fixed_dt_ns.toString(),
    max_frame_steps: config.max_frame_steps,
    max_frame_delta_ns: config.max_frame_delta_ns.toString(),
  };
}

function stateToFixture(state: GameClockState): Record<string, unknown> {
  return {
    tick_count: state.tick_count.toString(),
    frame_count: state.frame_count.toString(),
    elapsed_sim_ns: state.elapsed_sim_ns.toString(),
    elapsed_wall_ns: state.elapsed_wall_ns.toString(),
    accumulator_ns: state.accumulator_ns.toString(),
    fixed_dt_ns: state.fixed_dt_ns.toString(),
    max_frame_steps: state.max_frame_steps,
    max_frame_delta_ns: state.max_frame_delta_ns.toString(),
    total_dropped_ns: state.total_dropped_ns.toString(),
    spiral_clamped_frames: state.spiral_clamped_frames.toString(),
  };
}

function resultToFixture<T>(result: Result<T>, encode: (value: T) => unknown): Record<string, unknown> {
  return result.ok ? { ok: encode(result.value) } : { error: result.error };
}

export function dispatchPublic(op: string, input: Record<string, unknown>): Record<string, unknown> {
  switch (op) {
    case "standard_60hz":
      return { ok: { config: configToFixture(standard_60hz()) } };
    case "from_hz":
      return resultToFixture(
        from_hz(input.hz as number, input.max_frame_steps as number),
        (config) => ({ config: configToFixture(config) }),
      );
    case "new_clock":
      return resultToFixture(new_clock(configFromFixture(input.config)), (state) => ({
        state: stateToFixture(state),
      }));
    case "validate_state": {
      const result = validate_state(stateFromFixture(input.state));
      return result.ok ? { ok: true } : { error: result.error };
    }
    case "advance_frame":
      return resultToFixture(
        advance_frame(
          stateFromFixture(input.state),
          asU64(input, "raw_frame_delta_ns"),
        ),
        (step) => ({
          state: stateToFixture((step as ClockStepResult).state),
          steps_to_run: (step as ClockStepResult).steps_to_run,
          alpha_milli: (step as ClockStepResult).alpha_milli,
          alpha_q16: (step as ClockStepResult).alpha_q16,
          spiral_of_death_clamped: (step as ClockStepResult).spiral_of_death_clamped,
          dropped_ns: (step as ClockStepResult).dropped_ns.toString(),
        }),
      );
    default:
      throw new Error(`unknown GAME10 operation ${JSON.stringify(op)}`);
  }
}

export function runCorpus(
  cases: readonly FixtureCase[],
  dispatch: (op: string, input: Record<string, unknown>) => Record<string, unknown> = dispatchPublic,
): void {
  assert.equal(cases.length, 23, "GAME10 v1 requires all 23 fixture cases");
  const seen = new Set<string>();
  const operations = new Set<string>();
  let accepted = 0;
  let refused = 0;
  for (const fixtureCase of cases) {
    assert.ok(fixtureCase.id.length > 0, "case IDs are non-empty");
    assert.ok(!seen.has(fixtureCase.id), `duplicate case ID ${fixtureCase.id}`);
    assert.ok(canonicalV1CaseIds.has(fixtureCase.id), `unknown canonical GAME10 v1 case ID ${fixtureCase.id}`);
    seen.add(fixtureCase.id);
    operations.add(fixtureCase.op);
    const actual = dispatch(fixtureCase.op, fixtureCase.input);
    assert.deepStrictEqual(actual, fixtureCase.expected, fixtureCase.id);
    if ("error" in actual) refused += 1;
    else accepted += 1;
  }
  assert.deepStrictEqual(seen, canonicalV1CaseIds, "the complete canonical GAME10 v1 ID set is required");
  assert.deepStrictEqual(
    [...operations].sort(),
    ["advance_frame", "from_hz", "new_clock", "standard_60hz", "validate_state"],
  );
  assert.equal(accepted, 9);
  assert.equal(refused, 14);
}
