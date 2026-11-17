/**
 * Hamurabi Conformance & Security Test Suite
 *
 * Verifies:
 * 1. Conformance with code/specs/fixtures/hamurabi-v1/cases.json preserving 64-bit PRNG precision
 * 2. All 8 HamurabiError error arms with exact precedence
 * 3. JS Number / IEEE-754 Hardening (CWE-681 / CWE-190)
 * 4. Prototype Pollution & Immutability / Fresh Record Hardening (CWE-1321 & VER04m)
 * 5. PRNG state invariant & DOS loop protections (CWE-400 / CWE-835)
 * 6. Zero-Capability AST Audit (VER08a)
 */

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { describe, test as it } from "node:test";
import { fileURLToPath } from "node:url";

import {
  VERSION,
  new_game,
  validate_order,
  step_year,
  format_report,
  newGame,
  stepYear,
  validateOrder,
  formatReport,
  isPlaying,
  isWon,
  getWonRating,
  isImpeached,
  parseImpeached,
  HamurabiError,
  GameStatus,
  Rating,
  type HamurabiState,
  type TurnOrder,
} from "../src/index.js";

function findCasesJson(): string {
  let curr = path.dirname(fileURLToPath(import.meta.url));
  for (let i = 0; i < 10; i++) {
    const candidate1 = path.join(curr, "code", "specs", "fixtures", "hamurabi-v1", "cases.json");
    if (fs.existsSync(candidate1)) return candidate1;
    const candidate2 = path.join(curr, "specs", "fixtures", "hamurabi-v1", "cases.json");
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

function parseState(rawState: Record<string, unknown>): HamurabiState {
  return {
    year: Number(rawState.year),
    population: Number(rawState.population),
    bushels: Number(rawState.bushels),
    acres: Number(rawState.acres),
    land_price: Number(rawState.land_price),
    harvest_per_acre: Number(rawState.harvest_per_acre),
    rats_eaten: Number(rawState.rats_eaten),
    starved_last_year: Number(rawState.starved_last_year),
    starved_total: Number(rawState.starved_total),
    percent_starved_sum: Number(rawState.percent_starved_sum),
    immigrants_last_year: Number(rawState.immigrants_last_year),
    plague_struck: Boolean(rawState.plague_struck),
    status: rawState.status as HamurabiState["status"],
    rng_state: BigInt(rawState.rng_state as string | number | bigint),
    rng_inc: BigInt(rawState.rng_inc as string | number | bigint),
  };
}

// ── Package Sanity ─────────────────────────────────────────────────────────

describe("package metadata", () => {
  it("exports VERSION 0.1.0", () => {
    assert.equal(VERSION, "0.1.0");
  });
});

// ── Shared Corpus Conformance (cases.json) ──────────────────────────────────

describe("cases.json shared corpus conformance", () => {
  const casesPath = findCasesJson();
  const rawText = fs.readFileSync(casesPath, "utf8");

  // Regex quote 64-bit integer values to prevent IEEE-754 precision loss
  const quotedText = rawText.replace(
    /"(rng_state|rng_inc|seed)":\s*(\d+)/g,
    '"$1":"$2"'
  );
  const suiteData = JSON.parse(quotedText);

  it("suite contains all 31 conformance cases", () => {
    assert.equal(suiteData.cases.length, 31);
  });

  for (const testCase of suiteData.cases) {
    it(`conformance case: ${testCase.id}`, () => {
      const parsedInitial = parseState(testCase.initial_state);

      if (
        testCase.seed !== undefined &&
        Array.isArray(testCase.turns) &&
        parsedInitial.year === 1 &&
        parsedInitial.acres === 1000 &&
        parsedInitial.population === 100
      ) {
        const seedVal = BigInt(testCase.seed);
        const fromSeed = new_game(seedVal);
        assert.deepEqual(fromSeed, parsedInitial);
      }

      if (Array.isArray(testCase.turns)) {
        let currentState = parsedInitial;
        for (const turn of testCase.turns) {
          const order: TurnOrder = {
            acres_delta: Number(turn.order.acres_delta),
            bushels_to_feed: Number(turn.order.bushels_to_feed),
            acres_to_plant: Number(turn.order.acres_to_plant),
          };

          const valRes = validate_order(currentState, order);
          assert.equal(valRes.ok, true, `validate_order failed on turn ${turn.turn}`);

          const stepRes = step_year(currentState, order);
          assert.equal(stepRes.ok, true, `step_year failed on turn ${turn.turn}`);

          const expectedState = parseState(turn.expected_state);
          assert.deepEqual(stepRes.value, expectedState, `State mismatch on turn ${turn.turn}`);

          if (turn.expected_report) {
            const report = format_report(stepRes.value);
            assert.equal(report, turn.expected_report, `Report mismatch on turn ${turn.turn}`);
          }

          currentState = stepRes.value;
        }
      } else if (testCase.order && testCase.expected_error) {
        const order: TurnOrder = {
          acres_delta: Number(testCase.order.acres_delta),
          bushels_to_feed: Number(testCase.order.bushels_to_feed),
          acres_to_plant: Number(testCase.order.acres_to_plant),
        };

        const valRes = validate_order(parsedInitial, order);
        assert.equal(valRes.ok, false);
        assert.equal(valRes.error, testCase.expected_error);

        const stepRes = step_year(parsedInitial, order);
        assert.equal(stepRes.ok, false);
        assert.equal(stepRes.error, testCase.expected_error);
      }
    });
  }
});

// ── All 8 Error Arms & Canonical Precedence ─────────────────────────────────

describe("HamurabiError all 8 arms & canonical evaluation order", () => {
  const baseState = new_game(42n);

  it("Arm 1: GameAlreadyEnded", () => {
    const ended = { ...baseState, status: "Won(Charlemagne)" as const };
    const order: TurnOrder = { acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 100 };
    assert.deepEqual(validate_order(ended, order), { ok: false, error: HamurabiError.GameAlreadyEnded });
    assert.deepEqual(step_year(ended, order), { ok: false, error: HamurabiError.GameAlreadyEnded });
  });

  it("Arm 2: InvalidOrderBounds (corrupt land price)", () => {
    const badState = { ...baseState, land_price: 10 };
    const order: TurnOrder = { acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 100 };
    assert.deepEqual(validate_order(badState, order), { ok: false, error: HamurabiError.InvalidOrderBounds });
  });

  it("Arm 2: InvalidOrderBounds (revenue overflow in land sale)", () => {
    const richState = { ...baseState, acres: 2_000_000_000, bushels: 4_000_000_000 };
    const overflowSell: TurnOrder = { acres_delta: -1_000_000_000, bushels_to_feed: 0, acres_to_plant: 0 };
    assert.deepEqual(validate_order(richState, overflowSell), { ok: false, error: HamurabiError.InvalidOrderBounds });
  });

  it("Arm 3: InsufficientBushelsForLand", () => {
    const buyTooMuch: TurnOrder = { acres_delta: 200, bushels_to_feed: 0, acres_to_plant: 0 };
    assert.deepEqual(validate_order(baseState, buyTooMuch), { ok: false, error: HamurabiError.InsufficientBushelsForLand });
  });

  it("Arm 4: InsufficientAcresToSell", () => {
    const sellTooMuch: TurnOrder = { acres_delta: -1001, bushels_to_feed: 0, acres_to_plant: 0 };
    assert.deepEqual(validate_order(baseState, sellTooMuch), { ok: false, error: HamurabiError.InsufficientAcresToSell });
  });

  it("Arm 5: InsufficientBushelsToFeed", () => {
    const feedTooMuch: TurnOrder = { acres_delta: 0, bushels_to_feed: 2801, acres_to_plant: 0 };
    assert.deepEqual(validate_order(baseState, feedTooMuch), { ok: false, error: HamurabiError.InsufficientBushelsToFeed });
  });

  it("Arm 6: InsufficientAcresToPlant", () => {
    const plantTooMuch: TurnOrder = { acres_delta: 0, bushels_to_feed: 1000, acres_to_plant: 1001 };
    assert.deepEqual(validate_order(baseState, plantTooMuch), { ok: false, error: HamurabiError.InsufficientAcresToPlant });
  });

  it("Arm 7: InsufficientPopulationToPlant", () => {
    const smallPop = { ...baseState, population: 50 }; // 50 people tend max 500 acres
    const plantTooMuch: TurnOrder = { acres_delta: 0, bushels_to_feed: 1000, acres_to_plant: 501 };
    assert.deepEqual(validate_order(smallPop, plantTooMuch), { ok: false, error: HamurabiError.InsufficientPopulationToPlant });
  });

  it("Arm 8: InsufficientGrainForSeed", () => {
    // 800 acres needs 400 bushels seed. Bushels left after feed: 2800 - 2500 = 300
    const seedStarved: TurnOrder = { acres_delta: 0, bushels_to_feed: 2500, acres_to_plant: 800 };
    assert.deepEqual(validate_order(baseState, seedStarved), { ok: false, error: HamurabiError.InsufficientGrainForSeed });
  });
});

// ── JS Number / IEEE-754 Hardening (CWE-681 / CWE-190) ──────────────────────

describe("JS number / IEEE-754 hardening (CWE-681 / CWE-190)", () => {
  const baseState = new_game(1337n);
  const validOrder: TurnOrder = { acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 500 };

  const hostileValues = [
    NaN,
    Infinity,
    -Infinity,
    10.5,
    Number.MAX_SAFE_INTEGER + 10,
    "100",
    null,
    undefined,
    true,
    false,
    {},
    [],
  ];

  it("rejects non-safe-integers and hostile types on order fields", () => {
    for (const bad of hostileValues) {
      assert.deepEqual(
        validate_order(baseState, { ...validOrder, acres_delta: bad as unknown as number }),
        { ok: false, error: HamurabiError.InvalidOrderBounds },
        `Failed to reject bad acres_delta: ${bad}`
      );
      assert.deepEqual(
        validate_order(baseState, { ...validOrder, bushels_to_feed: bad as unknown as number }),
        { ok: false, error: HamurabiError.InvalidOrderBounds },
        `Failed to reject bad bushels_to_feed: ${bad}`
      );
      assert.deepEqual(
        validate_order(baseState, { ...validOrder, acres_to_plant: bad as unknown as number }),
        { ok: false, error: HamurabiError.InvalidOrderBounds },
        `Failed to reject bad acres_to_plant: ${bad}`
      );
    }
  });

  it("normalizes -0 to 0 on order and state", () => {
    const minusZeroOrder: TurnOrder = {
      acres_delta: -0,
      bushels_to_feed: 2000,
      acres_to_plant: 500,
    };
    const res = step_year(baseState, minusZeroOrder);
    assert.equal(res.ok, true);
    assert.equal(Object.is(res.value.acres, 1000), true);
  });
});

// ── Prototype Pollution & Immutability Hardening (CWE-1321 & VER04m) ────────

describe("prototype pollution & immutability (CWE-1321 & VER04m)", () => {
  const baseState = new_game(100n);

  it("rejects orders with own __proto__ property", () => {
    const poisonedOrder = { acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 100 };
    Object.defineProperty(poisonedOrder, "__proto__", {
      value: { polluted: true },
      enumerable: true,
      configurable: true,
    });
    assert.deepEqual(
      validate_order(baseState, poisonedOrder),
      { ok: false, error: HamurabiError.InvalidOrderBounds }
    );
  });

  it("rejects state with own __proto__ property", () => {
    const poisonedState = { ...baseState };
    Object.defineProperty(poisonedState, "__proto__", {
      value: { injected: true },
      enumerable: true,
      configurable: true,
    });
    const order: TurnOrder = { acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 100 };
    assert.deepEqual(
      validate_order(poisonedState, order),
      { ok: false, error: HamurabiError.InvalidOrderBounds }
    );
  });

  it("proves state objects are frozen and immutable", () => {
    assert.equal(Object.isFrozen(baseState), true);

    const order: TurnOrder = { acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 800 };
    const stepRes = step_year(baseState, order);
    assert.equal(stepRes.ok, true);
    assert.equal(Object.isFrozen(stepRes.value), true);
    assert.notEqual(stepRes.value, baseState);

    // Attempting mutation in strict mode throws TypeError
    assert.throws(() => {
      // @ts-expect-error intentionally mutating read-only field
      stepRes.value.population = 999999;
    }, TypeError);
  });
});

// ── PRNG & Invariant Security Guards (CWE-400 / CWE-835 / CWE-190) ─────────

describe("PRNG & invariant security guards", () => {
  const validOrder: TurnOrder = { acres_delta: 0, bushels_to_feed: 2000, acres_to_plant: 800 };

  it("rejects even rng_inc (prevents LCG period collapse / infinite loop)", () => {
    const state0 = { ...new_game(1n), rng_inc: 0n };
    assert.deepEqual(validate_order(state0, validOrder), { ok: false, error: HamurabiError.InvalidOrderBounds });

    const state2 = { ...new_game(1n), rng_inc: 2n };
    assert.deepEqual(validate_order(state2, validOrder), { ok: false, error: HamurabiError.InvalidOrderBounds });

    const state108 = { ...new_game(1n), rng_inc: 108n };
    assert.deepEqual(validate_order(state108, validOrder), { ok: false, error: HamurabiError.InvalidOrderBounds });
  });

  it("rejects out of bounds state invariants", () => {
    // year bounds [1, 10]
    assert.deepEqual(validate_order({ ...new_game(1n), year: 0 }, validOrder), { ok: false, error: HamurabiError.InvalidOrderBounds });
    assert.deepEqual(validate_order({ ...new_game(1n), year: 11 }, validOrder), { ok: false, error: HamurabiError.InvalidOrderBounds });
    assert.deepEqual(validate_order({ ...new_game(1n), year: 255 }, validOrder), { ok: false, error: HamurabiError.InvalidOrderBounds });

    // land price bounds [17, 26]
    assert.deepEqual(validate_order({ ...new_game(1n), land_price: 16 }, validOrder), { ok: false, error: HamurabiError.InvalidOrderBounds });
    assert.deepEqual(validate_order({ ...new_game(1n), land_price: 27 }, validOrder), { ok: false, error: HamurabiError.InvalidOrderBounds });

    // harvest yield bounds [1, 5]
    assert.deepEqual(validate_order({ ...new_game(1n), harvest_per_acre: 0 }, validOrder), { ok: false, error: HamurabiError.InvalidOrderBounds });
    assert.deepEqual(validate_order({ ...new_game(1n), harvest_per_acre: 6 }, validOrder), { ok: false, error: HamurabiError.InvalidOrderBounds });
  });

  it("rejects harvest overflow (acres_to_plant * 5 > MAX_U32)", () => {
    const richState = { ...new_game(1n), population: 500_000_000, acres: 1_000_000_000, bushels: 2_000_000_000 };
    const overflowOrder: TurnOrder = { acres_delta: 0, bushels_to_feed: 0, acres_to_plant: 900_000_000 };
    assert.deepEqual(validate_order(richState, overflowOrder), { ok: false, error: HamurabiError.InvalidOrderBounds });
    assert.deepEqual(step_year(richState, overflowOrder), { ok: false, error: HamurabiError.InvalidOrderBounds });
  });

  it("handles starved_total and percent_starved_sum saturating addition", () => {
    const saturatedState = {
      ...new_game(1n),
      starved_total: 0xffffffff,
      percent_starved_sum: 0xffffffff - 10,
    };
    const starveOrder: TurnOrder = { acres_delta: 0, bushels_to_feed: 1000, acres_to_plant: 0 };
    const res = step_year(saturatedState, starveOrder);
    assert.equal(res.ok, true);
    assert.equal(res.value.starved_total, 0xffffffff);
    assert.equal(res.value.percent_starved_sum, 0xffffffff);
  });
});

// ── Zero-Capability AST Audit (VER08a) ──────────────────────────────────────

describe("zero-capability AST audit (VER08a)", () => {
  it("verifies src/index.ts has zero forbidden imports or calls", () => {
    const srcPath = findPackageFile(path.join("src", "index.ts"));
    const src = fs.readFileSync(srcPath, "utf8");

    const forbidden = [
      /from\s+["']node:fs["']/,
      /from\s+["']fs["']/,
      /from\s+["']node:net["']/,
      /from\s+["']net["']/,
      /from\s+["']node:child_process["']/,
      /from\s+["']child_process["']/,
      /from\s+["']node:http["']/,
      /from\s+["']http["']/,
      /from\s+["']node:process["']/,
      /\beval\s*\(/,
      /\bFunction\s*\(/,
    ];

    for (const pat of forbidden) {
      assert.equal(
        pat.test(src),
        false,
        `Forbidden pattern detected in src/index.ts: ${pat}`
      );
    }
  });

  it("verifies required_capabilities.json declares empty capabilities", () => {
    const capPath = findPackageFile("required_capabilities.json");
    const caps = JSON.parse(fs.readFileSync(capPath, "utf8"));
    assert.equal(caps.version, 1);
    assert.equal(caps.package, "typescript/hamurabi");
    assert.deepEqual(caps.capabilities, []);
  });
});

// ── Status Helpers & Additional Formatting Branches ────────────────────────

describe("status helpers and formatting edge cases", () => {
  it("exercises status helpers and rating extractors", () => {
    const playing = new_game(1n);
    assert.equal(isPlaying(playing.status), true);
    assert.equal(isWon(playing.status), false);
    assert.equal(isImpeached(playing.status), false);
    assert.equal(getWonRating(playing.status), null);
    assert.equal(parseImpeached(playing.status), null);

    const wonChar = { ...playing, status: GameStatus.Won("Charlemagne") };
    assert.equal(isPlaying(wonChar.status), false);
    assert.equal(isWon(wonChar.status), true);
    assert.equal(getWonRating(wonChar.status), "Charlemagne");

    const wonAcre = { ...playing, status: GameStatus.Won("Acre") };
    assert.equal(getWonRating(wonAcre.status), "Acre");

    const wonNero = { ...playing, status: GameStatus.Won("Nero") };
    assert.equal(getWonRating(wonNero.status), "Nero");

    const wonDisasterState = { ...playing, status: GameStatus.Won("Disaster") };
    assert.equal(getWonRating(wonDisasterState.status), "Disaster");

    const impeached = {
      ...playing,
      status: GameStatus.Impeached(50, 50),
    };
    assert.equal(isImpeached(impeached.status), true);
    assert.deepEqual(parseImpeached(impeached.status), { starved: 50, percent: 50 });

    const extinctionState = {
      ...playing,
      status: "GameOverExtinction" as const,
    };
    assert.equal(
      format_report(extinctionState),
      "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n"
    );

    const wonDisaster = {
      ...playing,
      year: 11,
      population: 10,
      acres: 50,
      percent_starved_sum: 500,
      status: "Won(Disaster)" as const,
    };
    const disasterReport = format_report(wonDisaster);
    assert.match(disasterReport, /OFFICIAL NATIONAL DISASTER/);

    // Aliases
    assert.equal(newGame, new_game);
    assert.equal(stepYear, step_year);
    assert.equal(validateOrder, validate_order);
    assert.equal(formatReport, format_report);
  });

  it("exercises seed validation edge cases", () => {
    // Valid number seed
    const s1 = new_game(42);
    assert.equal(s1.year, 1);

    // Negative number throws RangeError
    assert.throws(() => {
      new_game(-1);
    }, RangeError);

    // Non-integer throws TypeError
    assert.throws(() => {
      // @ts-expect-error test non-integer
      new_game("invalid");
    }, TypeError);
  });
});
