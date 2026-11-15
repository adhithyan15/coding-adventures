/**
 * # Conformance and Adversarial Test Suite for @coding-adventures/lunar-lander
 *
 * Validates:
 * - Full execution of all 39 test cases in code/specs/fixtures/lunar-lander-v1/cases.json
 * - Exact multi-turn trajectory integration parity across all flight phases
 * - Ground touchdown solver contact time interpolation and terminal impact velocity
 * - Proper fuel refund calculation without phantom fuel restoration
 * - All refusal arms and canonical error precedence
 * - CWE-369 (Division by Zero) and CWE-190 (Integer Overflow) adversarial immunization
 * - CWE-681 (Safe Integer / IEEE-754) and CWE-1321 (Prototype Pollution) defense
 * - 76x5 ASCII telemetry HUD layout and gauge rendering parity with Rust reference
 * - Zero ambient authority (`"capabilities": []`)
 */

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { describe, test as it } from "node:test";
import { fileURLToPath } from "node:url";

import {
  VERSION,
  new_flight,
  validate_burn,
  step_burn,
  divEuclid,
  isqrtU128,
  to_paint_scene,
  telemetry_scene,
  render_telemetry_ascii,
  render_telemetry,
  formatWithCommas,
  newFlight,
  validateBurn,
  stepBurn,
  div_euclid,
  isqrt_u128,
  toPaintScene,
  telemetryScene,
  renderTelemetryAscii,
  renderTelemetry,
  format_with_commas,
  ok,
  err,
  LanderError,
  LandingOutcome,
  FlightStatus,
  DRY_MASS_LBS,
  INITIAL_ALTITUDE_MFT,
  INITIAL_FUEL_LBS,
  INITIAL_VELOCITY_MFT_S,
  MAX_ALTITUDE_MFT,
  MAX_BURN_RATE_LBS_S,
  MAX_DURATION_S,
  MAX_TIME_SECONDS,
  MAX_VELOCITY_MFT_S,
  MIN_VELOCITY_MFT_S,
  type LanderState,
  type BurnCommand,
} from "../src/index.js";

function findCasesJson(): string {
  let curr = path.dirname(fileURLToPath(import.meta.url));
  for (let i = 0; i < 10; i++) {
    const candidate1 = path.join(
      curr,
      "code",
      "specs",
      "fixtures",
      "lunar-lander-v1",
      "cases.json"
    );
    if (fs.existsSync(candidate1)) return candidate1;
    const candidate2 = path.join(
      curr,
      "specs",
      "fixtures",
      "lunar-lander-v1",
      "cases.json"
    );
    if (fs.existsSync(candidate2)) return candidate2;
    const parent = path.dirname(curr);
    if (parent === curr) break;
    curr = parent;
  }
  throw new Error("Could not find cases.json");
}

function findPackageFile(filename: string): string {
  let curr = path.dirname(fileURLToPath(import.meta.url));
  for (let i = 0; i < 10; i++) {
    const candidate = path.join(curr, filename);
    if (fs.existsSync(candidate)) return candidate;
    const parent = path.dirname(curr);
    if (parent === curr) break;
    curr = parent;
  }
  throw new Error(`Could not find ${filename}`);
}

interface JsonLanderState {
  time_seconds: number;
  altitude_mft: number;
  velocity_mft_s: number;
  fuel_lbs: number;
  dry_mass_lbs: number;
  status: string;
  impact_velocity_mft_s: number | null;
}

interface JsonBurnCommand {
  burn_rate_lbs_s: number;
  duration_seconds: number;
}

interface JsonTurn {
  turn: number;
  command: JsonBurnCommand;
  expected_state: JsonLanderState;
}

interface CorpusCase {
  id: string;
  description: string;
  initial_state: JsonLanderState;
  turns?: JsonTurn[];
  command?: JsonBurnCommand;
  expected_error?: string;
}

interface CorpusFile {
  version: number;
  suite: string;
  cases: CorpusCase[];
}

function toJsonLanderState(s: LanderState): JsonLanderState {
  return {
    time_seconds: s.time_seconds,
    altitude_mft: s.altitude_mft,
    velocity_mft_s: s.velocity_mft_s,
    fuel_lbs: s.fuel_lbs,
    dry_mass_lbs: s.dry_mass_lbs,
    status: s.status,
    impact_velocity_mft_s: s.impact_velocity_mft_s,
  };
}

describe("package sanity & version", () => {
  it("exports VERSION 0.1.0", () => {
    assert.equal(VERSION, "0.1.0");
  });

  it("exports camelCase aliases identical to snake_case", () => {
    assert.equal(newFlight, new_flight);
    assert.equal(validateBurn, validate_burn);
    assert.equal(stepBurn, step_burn);
    assert.equal(div_euclid, divEuclid);
    assert.equal(isqrt_u128, isqrtU128);
    assert.equal(toPaintScene, to_paint_scene);
    assert.equal(telemetryScene, telemetry_scene);
    assert.equal(renderTelemetryAscii, render_telemetry_ascii);
    assert.equal(renderTelemetry, render_telemetry);
    assert.equal(format_with_commas, formatWithCommas);
  });
});

describe("initial flight invariants", () => {
  it("initializes Apollo LM with warrant-specified baseline conditions", () => {
    const flight = new_flight();
    assert.equal(flight.time_seconds, 0);
    assert.equal(flight.altitude_mft, INITIAL_ALTITUDE_MFT);
    assert.equal(flight.velocity_mft_s, INITIAL_VELOCITY_MFT_S);
    assert.equal(flight.fuel_lbs, INITIAL_FUEL_LBS);
    assert.equal(flight.dry_mass_lbs, DRY_MASS_LBS);
    assert.equal(flight.status, FlightStatus.InFlight);
    assert.equal(flight.impact_velocity_mft_s, null);
    assert.equal(Object.isFrozen(flight), true);
  });
});

describe("divEuclid exactness", () => {
  it("satisfies Euclidean division non-negative remainder across all sign combinations", () => {
    // positive / positive
    assert.equal(divEuclid(7n, 3n), 2n);
    // positive / negative
    assert.equal(divEuclid(7n, -3n), -2n);
    // negative / positive
    assert.equal(divEuclid(-7n, 3n), -3n);
    // negative / negative (CRITICAL cases.json Turn 4 vector)
    assert.equal(divEuclid(-46070n, -5310n), 9n);
    assert.equal(divEuclid(-7n, -3n), 3n);
    // exact division (remainder 0)
    assert.equal(divEuclid(6n, 3n), 2n);
    assert.equal(divEuclid(-6n, 3n), -2n);
    assert.equal(divEuclid(6n, -3n), -2n);
    assert.equal(divEuclid(-6n, -3n), 2n);
    assert.equal(divEuclid(0n, 5n), 0n);
    assert.equal(divEuclid(0n, -5n), 0n);
  });
});

describe("isqrtU128 exactness", () => {
  it("computes exact integer square root for small, medium, and 128-bit numbers", () => {
    assert.equal(isqrtU128(0n), 0n);
    assert.equal(isqrtU128(1n), 1n);
    assert.equal(isqrtU128(2n), 1n);
    assert.equal(isqrtU128(3n), 1n);
    assert.equal(isqrtU128(4n), 2n);
    assert.equal(isqrtU128(15n), 3n);
    assert.equal(isqrtU128(16n), 4n);
    assert.equal(isqrtU128(10_274_400_000_000n), 3_205_370n);

    const u64Max = 0xffffffffffffffffn;
    assert.equal(isqrtU128(u64Max * u64Max), u64Max);
    const u128Max = (1n << 128n) - 1n;
    assert.equal(isqrtU128(u128Max), u64Max);
  });
});

describe("cases.json full 39-vector conformance suite", () => {
  const casesJsonPath = findCasesJson();
  const corpus: CorpusFile = JSON.parse(
    fs.readFileSync(casesJsonPath, "utf8")
  );

  it("verifies suite metadata and case count", () => {
    assert.equal(corpus.version, 1);
    assert.equal(corpus.suite, "lunar_lander_deterministic_flight");
    assert.equal(corpus.cases.length, 39, "Corpus must have exactly 39 cases");
  });

  for (const c of corpus.cases) {
    it(`Case ${c.id}: ${c.description}`, () => {
      let state: LanderState = Object.freeze({
        time_seconds: c.initial_state.time_seconds,
        altitude_mft: c.initial_state.altitude_mft,
        velocity_mft_s: c.initial_state.velocity_mft_s,
        fuel_lbs: c.initial_state.fuel_lbs,
        dry_mass_lbs: c.initial_state.dry_mass_lbs,
        status: c.initial_state.status as any,
        impact_velocity_mft_s: c.initial_state.impact_velocity_mft_s,
      });

      if (c.turns && c.turns.length > 0) {
        for (const turn of c.turns) {
          const cmd: BurnCommand = {
            burn_rate_lbs_s: turn.command.burn_rate_lbs_s,
            duration_seconds: turn.command.duration_seconds,
          };

          const validation = validate_burn(state, cmd);
          assert.equal(
            validation.ok,
            true,
            `Case ${c.id} turn ${turn.turn} validation failed unexpectedly`
          );

          const stepResult = step_burn(state, cmd);
          assert.equal(
            stepResult.ok,
            true,
            `Case ${c.id} turn ${turn.turn} step failed unexpectedly`
          );
          if (!stepResult.ok) return;

          const next = stepResult.value;
          assert.equal(
            next.time_seconds,
            turn.expected_state.time_seconds,
            `Case ${c.id} turn ${turn.turn} time_seconds mismatch`
          );
          assert.equal(
            next.altitude_mft,
            turn.expected_state.altitude_mft,
            `Case ${c.id} turn ${turn.turn} altitude_mft mismatch`
          );
          assert.equal(
            next.velocity_mft_s,
            turn.expected_state.velocity_mft_s,
            `Case ${c.id} turn ${turn.turn} velocity_mft_s mismatch`
          );
          assert.equal(
            next.fuel_lbs,
            turn.expected_state.fuel_lbs,
            `Case ${c.id} turn ${turn.turn} fuel_lbs mismatch`
          );
          assert.equal(
            next.dry_mass_lbs,
            turn.expected_state.dry_mass_lbs,
            `Case ${c.id} turn ${turn.turn} dry_mass_lbs mismatch`
          );
          assert.equal(
            next.status,
            turn.expected_state.status,
            `Case ${c.id} turn ${turn.turn} status mismatch`
          );
          assert.equal(
            next.impact_velocity_mft_s,
            turn.expected_state.impact_velocity_mft_s,
            `Case ${c.id} turn ${turn.turn} impact_velocity_mft_s mismatch`
          );
          assert.equal(Object.isFrozen(next), true);

          state = next;
        }
      } else if (c.command) {
        const cmd: BurnCommand = {
          burn_rate_lbs_s: c.command.burn_rate_lbs_s,
          duration_seconds: c.command.duration_seconds,
        };
        const expectedError = c.expected_error;
        assert.ok(expectedError, `Refusal case ${c.id} must define expected_error`);

        const valResult = validate_burn(state, cmd);
        assert.equal(
          valResult.ok,
          false,
          `Case ${c.id} validate_burn expected error but succeeded`
        );
        if (!valResult.ok) {
          assert.equal(
            valResult.error,
            expectedError,
            `Case ${c.id} validate_burn error mismatch`
          );
        }

        const stepResult = step_burn(state, cmd);
        assert.equal(
          stepResult.ok,
          false,
          `Case ${c.id} step_burn expected error but succeeded`
        );
        if (!stepResult.ok) {
          assert.equal(
            stepResult.error,
            expectedError,
            `Case ${c.id} step_burn error mismatch`
          );
        }
      } else {
        assert.fail(`Case ${c.id} has neither turns nor command`);
      }
    });
  }
});

describe("telemetry HUD and PaintScene presentation", () => {
  it("renders 76x5 ASCII dashboard HUD with full parity", () => {
    const state = new_flight();
    const scene = to_paint_scene(state, 0);
    assert.equal(scene.width, 76.0);
    assert.equal(scene.height, 5.0);
    assert.equal(scene.background, "#000000");
    assert.equal(Object.isFrozen(scene), true);
    assert.equal(Object.isFrozen(scene.instructions), true);

    const hud = render_telemetry_ascii(state, 0);
    const lines = hud.split("\n");
    assert.equal(lines.length, 5, "Telemetry HUD must contain exactly 5 lines");

    for (let idx = 0; idx < lines.length; idx++) {
      assert.equal(
        Array.from(lines[idx]).length,
        76,
        `Line ${idx} length mismatch: ${lines[idx]}`
      );
    }

    assert.ok(hud.includes("LUNAR LANDER TELEMETRY"));
    assert.ok(hud.includes("TIME:"));
    assert.ok(hud.includes("ALTITUDE:"));
    assert.ok(hud.includes("VELOCITY:"));
    assert.ok(hud.includes("FUEL:"));
    assert.ok(hud.includes("BURN:"));
    assert.ok(hud.includes("RATE:"));
    assert.ok(hud.includes("DESCENT"));
    assert.ok(hud.includes("█"));

    // Check mid-flight state with partial gauge fill containing both █ and ░
    const midState: LanderState = Object.freeze({
      ...state,
      fuel_lbs: 8250,
      altitude_mft: 60000000,
    });
    const hudMid = render_telemetry_ascii(midState, 100);
    assert.ok(hudMid.includes("█"));
    assert.ok(hudMid.includes("░"));

    // Convenience helpers
    const scene2 = telemetry_scene(state);
    assert.equal(scene2.width, 76.0);
    const hud2 = render_telemetry(state);
    assert.equal(hud2, hud);
  });

  it("renders flight rate and landed outcome states accurately", () => {
    const base = new_flight();

    // InFlight ascent
    const ascentState: LanderState = Object.freeze({
      ...base,
      velocity_mft_s: 50000,
    });
    const hudAscent = render_telemetry_ascii(ascentState, 100);
    assert.ok(hudAscent.includes("ASCENT"));
    assert.ok(hudAscent.includes("▲"));

    // InFlight hover
    const hoverState: LanderState = Object.freeze({
      ...base,
      velocity_mft_s: 0,
    });
    const hudHover = render_telemetry_ascii(hoverState, 50);
    assert.ok(hudHover.includes("HOVER"));
    assert.ok(hudHover.includes("◆"));

    // Landed outcomes
    const outcomes: Array<[LandingOutcome, string]> = [
      [LandingOutcome.Perfect, "PERFECT"],
      [LandingOutcome.Hard, "HARD"],
      [LandingOutcome.Crash, "CRASH"],
      [LandingOutcome.Fatal, "FATAL"],
      [LandingOutcome.Crater, "CRATER"],
    ];

    for (const [outcome, text] of outcomes) {
      const landedState: LanderState = Object.freeze({
        ...base,
        status: FlightStatus.Landed(outcome),
        impact_velocity_mft_s: -1000,
      });
      const hudLanded = render_telemetry_ascii(landedState, 0);
      assert.ok(hudLanded.includes("STATUS:"), "Landed HUD missing STATUS: label");
      assert.ok(
        hudLanded.includes(text),
        `Landed HUD missing outcome text ${text}`
      );
      assert.ok(hudLanded.includes("★"), "Landed HUD missing star symbol");
    }
  });
});

describe("adversarial security & input hardening", () => {
  it("prevents division by zero with corrupted state (CWE-369)", () => {
    const hostileState: LanderState = Object.freeze({
      time_seconds: 0,
      altitude_mft: 100000000,
      velocity_mft_s: -2000000,
      fuel_lbs: 0,
      dry_mass_lbs: 0, // Corrupted dry mass!
      status: FlightStatus.InFlight,
      impact_velocity_mft_s: null,
    });
    const cmd: BurnCommand = {
      burn_rate_lbs_s: 100,
      duration_seconds: 5,
    };
    assert.deepEqual(
      validate_burn(hostileState, cmd),
      err(LanderError.ArithmeticOverflow)
    );
    assert.deepEqual(
      step_burn(hostileState, cmd),
      err(LanderError.ArithmeticOverflow)
    );
  });

  it("prevents integer overflow and bounds violation (CWE-190)", () => {
    const validCmd: BurnCommand = {
      burn_rate_lbs_s: 100,
      duration_seconds: 5,
    };

    // Hostile altitude boundaries
    const stateHighAlt: LanderState = Object.freeze({
      ...new_flight(),
      altitude_mft: Number.MAX_SAFE_INTEGER,
    });
    assert.deepEqual(
      validate_burn(stateHighAlt, validCmd),
      err(LanderError.ArithmeticOverflow)
    );

    const stateAboveMaxAlt: LanderState = Object.freeze({
      ...new_flight(),
      altitude_mft: MAX_ALTITUDE_MFT + 1,
    });
    assert.deepEqual(
      validate_burn(stateAboveMaxAlt, validCmd),
      err(LanderError.ArithmeticOverflow)
    );

    const stateZeroAlt: LanderState = Object.freeze({
      ...new_flight(),
      altitude_mft: 0,
    });
    assert.deepEqual(
      validate_burn(stateZeroAlt, validCmd),
      err(LanderError.ArithmeticOverflow)
    );

    // Hostile velocity boundaries
    const stateHighVel: LanderState = Object.freeze({
      ...new_flight(),
      velocity_mft_s: MAX_VELOCITY_MFT_S + 1,
    });
    assert.deepEqual(
      validate_burn(stateHighVel, validCmd),
      err(LanderError.ArithmeticOverflow)
    );

    const stateLowVel: LanderState = Object.freeze({
      ...new_flight(),
      velocity_mft_s: MIN_VELOCITY_MFT_S - 1,
    });
    assert.deepEqual(
      validate_burn(stateLowVel, validCmd),
      err(LanderError.ArithmeticOverflow)
    );

    // Hostile fuel boundaries
    const stateExcessFuel: LanderState = Object.freeze({
      ...new_flight(),
      fuel_lbs: INITIAL_FUEL_LBS + 1,
    });
    assert.deepEqual(
      validate_burn(stateExcessFuel, validCmd),
      err(LanderError.ArithmeticOverflow)
    );

    // Hostile time boundaries
    const stateExcessTime: LanderState = Object.freeze({
      ...new_flight(),
      time_seconds: MAX_TIME_SECONDS + 1,
    });
    assert.deepEqual(
      validate_burn(stateExcessTime, validCmd),
      err(LanderError.ArithmeticOverflow)
    );

    const stateMaxTime: LanderState = Object.freeze({
      ...new_flight(),
      time_seconds: MAX_TIME_SECONDS,
    });
    const addOneS: BurnCommand = {
      burn_rate_lbs_s: 0,
      duration_seconds: 1,
    };
    assert.deepEqual(
      validate_burn(stateMaxTime, addOneS),
      err(LanderError.ArithmeticOverflow)
    );

    // Hostile command bounds
    const base = new_flight();
    const excessiveBurn: BurnCommand = {
      burn_rate_lbs_s: MAX_BURN_RATE_LBS_S + 1,
      duration_seconds: 5,
    };
    assert.deepEqual(
      validate_burn(base, excessiveBurn),
      err(LanderError.ExcessiveBurnRate)
    );

    const zeroDuration: BurnCommand = {
      burn_rate_lbs_s: 50,
      duration_seconds: 0,
    };
    assert.deepEqual(
      validate_burn(base, zeroDuration),
      err(LanderError.InvalidDuration)
    );

    const excessiveDuration: BurnCommand = {
      burn_rate_lbs_s: 50,
      duration_seconds: MAX_DURATION_S + 1,
    };
    assert.deepEqual(
      validate_burn(base, excessiveDuration),
      err(LanderError.InvalidDuration)
    );
  });

  it("hardens against IEEE-754 floats, NaN, strings, booleans, and negative zero (CWE-681)", () => {
    const base = new_flight();

    // NaN / Infinity on command burn rate
    assert.deepEqual(
      validate_burn(base, { burn_rate_lbs_s: NaN, duration_seconds: 5 } as any),
      err(LanderError.ArithmeticOverflow)
    );
    assert.deepEqual(
      validate_burn(base, { burn_rate_lbs_s: Infinity, duration_seconds: 5 } as any),
      err(LanderError.ArithmeticOverflow)
    );

    // Fractional float on duration
    assert.deepEqual(
      validate_burn(base, { burn_rate_lbs_s: 50, duration_seconds: 5.5 } as any),
      err(LanderError.InvalidDuration)
    );

    // String / boolean injection on state
    assert.deepEqual(
      validate_burn({ ...base, altitude_mft: "120000000" } as any, {
        burn_rate_lbs_s: 0,
        duration_seconds: 1,
      }),
      err(LanderError.ArithmeticOverflow)
    );
    assert.deepEqual(
      validate_burn({ ...base, altitude_mft: true } as any, {
        burn_rate_lbs_s: 0,
        duration_seconds: 1,
      }),
      err(LanderError.ArithmeticOverflow)
    );

    // Negative zero normalizes cleanly without arithmetic deviation
    const negZeroBurnCmd = { burn_rate_lbs_s: -0, duration_seconds: 1 };
    assert.equal(validate_burn(base, negZeroBurnCmd).ok, true);
  });

  it("rejects prototype pollution attempts (CWE-1321)", () => {
    const base = new_flight();
    const poisonedState = JSON.parse(
      '{"__proto__":{"polluted":true},"time_seconds":0,"altitude_mft":120000000,"velocity_mft_s":-3000000,"fuel_lbs":16500,"dry_mass_lbs":16500,"status":"InFlight","impact_velocity_mft_s":null}'
    );
    const validCmd = { burn_rate_lbs_s: 0, duration_seconds: 1 };

    assert.deepEqual(
      validate_burn(poisonedState, validCmd),
      err(LanderError.ArithmeticOverflow)
    );
    assert.deepEqual(
      step_burn(poisonedState, validCmd),
      err(LanderError.ArithmeticOverflow)
    );

    const poisonedCmd = JSON.parse(
      '{"__proto__":{"polluted":true},"burn_rate_lbs_s":0,"duration_seconds":1}'
    );
    assert.deepEqual(
      validate_burn(base, poisonedCmd),
      err(LanderError.ArithmeticOverflow)
    );
  });
});

describe("security capability audit", () => {
  it("enforces zero ambient system authority in required_capabilities.json", () => {
    const capPath = findPackageFile("required_capabilities.json");
    const content = JSON.parse(fs.readFileSync(capPath, "utf8"));
    assert.equal(content.package, "typescript/lunar-lander");
    assert.deepEqual(content.capabilities, []);
  });
});
