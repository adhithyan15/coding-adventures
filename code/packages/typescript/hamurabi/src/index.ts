/**
 * @coding-adventures/hamurabi
 *
 * Pure, deterministic simulation core for the seminal 1968 Hamurabi mainframe game.
 *
 * Architectural Guarantees:
 * - Strictly zero ambient system authority: capabilities: []
 * - Pure state transition: step_year(state, order) -> Result<HamurabiState, HamurabiError>
 * - Explicit PCG32 pseudorandomness with stream ID 54 (inc = 109n)
 * - Complete integer arithmetic with zero floating-point operations
 * - Complete 26-CWE weakness closure (CWE-190, CWE-400, CWE-835, CWE-1321, CWE-681, etc.)
 * - Immutable frozen records (VER04m state=explicit)
 */

export const VERSION = "0.1.0";

// ── Constants ───────────────────────────────────────────────────────────────

/** Knuth / Numerical Recipes 64-bit LCG multiplier constant. */
export const PCG_MULTIPLIER = 6364136223846793005n;

/** Stream ID for the Hamurabi simulation PRNG. */
export const HAMURABI_STREAM_ID = 54n;

/** Sequence increment for PCG32: (HAMURABI_STREAM_ID << 1) | 1 = 109n. */
export const HAMURABI_INC = (HAMURABI_STREAM_ID << 1n) | 1n;

export const MASK64 = 0xffffffffffffffffn;
export const MAX_U32 = 0xffffffff;
export const MIN_I32 = -2147483648;
export const MAX_I32 = 2147483647;

// ── Domain Types ───────────────────────────────────────────────────────────

/** Rating awarded upon completing a 10-year term. */
export type Rating = "Charlemagne" | "Acre" | "Nero" | "Disaster";

export const Rating = {
  Charlemagne: "Charlemagne",
  Acre: "Acre",
  Nero: "Nero",
  Disaster: "Disaster",
} as const;

/** Lifecycle status of the game. */
export type GameStatus =
  | "Playing"
  | "GameOverExtinction"
  | `Won(${Rating})`
  | `Impeached(starved=${number}, percent=${number})`;

export const GameStatus = {
  Playing: "Playing",
  GameOverExtinction: "GameOverExtinction",
  Won: (rating: Rating): `Won(${Rating})` => `Won(${rating})`,
  Impeached: (starved: number, percent: number): `Impeached(starved=${number}, percent=${number})` =>
    `Impeached(starved=${starved}, percent=${percent})`,
} as const;

/** Closed enumeration of order rejection errors. */
export const HamurabiError = {
  GameAlreadyEnded: "GameAlreadyEnded",
  InvalidOrderBounds: "InvalidOrderBounds",
  InsufficientBushelsForLand: "InsufficientBushelsForLand",
  InsufficientAcresToSell: "InsufficientAcresToSell",
  InsufficientBushelsToFeed: "InsufficientBushelsToFeed",
  InsufficientAcresToPlant: "InsufficientAcresToPlant",
  InsufficientPopulationToPlant: "InsufficientPopulationToPlant",
  InsufficientGrainForSeed: "InsufficientGrainForSeed",
} as const;

export type HamurabiError = (typeof HamurabiError)[keyof typeof HamurabiError];

/** Immutable value record capturing the complete simulation state. */
export interface HamurabiState {
  readonly year: number;
  readonly population: number;
  readonly bushels: number;
  readonly acres: number;
  readonly land_price: number;
  readonly harvest_per_acre: number;
  readonly rats_eaten: number;
  readonly starved_last_year: number;
  readonly starved_total: number;
  readonly percent_starved_sum: number;
  readonly immigrants_last_year: number;
  readonly plague_struck: boolean;
  readonly status: GameStatus;
  readonly rng_state: bigint;
  readonly rng_inc: bigint;
}

/** Player orders for a single year. */
export interface TurnOrder {
  readonly acres_delta: number;
  readonly bushels_to_feed: number;
  readonly acres_to_plant: number;
}

/** Pure Result type wrapping success or typed refusal. */
export type Result<T, E = HamurabiError> =
  | { readonly ok: true; readonly value: T }
  | { readonly ok: false; readonly error: E };

export function ok<T>(value: T): Result<T, never> {
  return Object.freeze({ ok: true, value });
}

export function err<E>(error: E): Result<never, E> {
  return Object.freeze({ ok: false, error });
}

// ── Status Helpers ──────────────────────────────────────────────────────────

export function isPlaying(status: GameStatus): boolean {
  return status === "Playing";
}

export function isWon(status: GameStatus): status is `Won(${Rating})` {
  return typeof status === "string" && status.startsWith("Won(");
}

export function getWonRating(status: GameStatus): Rating | null {
  if (status === "Won(Charlemagne)") return "Charlemagne";
  if (status === "Won(Acre)") return "Acre";
  if (status === "Won(Nero)") return "Nero";
  if (status === "Won(Disaster)") return "Disaster";
  return null;
}

export function isImpeached(status: GameStatus): boolean {
  return typeof status === "string" && status.startsWith("Impeached(");
}

export function parseImpeached(status: GameStatus): { starved: number; percent: number } | null {
  if (typeof status !== "string") return null;
  const match = status.match(/^Impeached\(starved=(\d+),\s*percent=(\d+)\)$/);
  if (!match) return null;
  return { starved: Number(match[1]), percent: Number(match[2]) };
}

// ── PRNG Primitives (PCG32 XSH RR 64/32) ───────────────────────────────────

function pcg32Step(rng: { state: bigint }, inc: bigint): number {
  const oldState = rng.state;
  rng.state = BigInt.asUintN(64, oldState * PCG_MULTIPLIER + inc);
  const xorshifted = Number(BigInt.asUintN(32, ((oldState >> 18n) ^ oldState) >> 27n));
  const rot = Number(oldState >> 59n);
  return ((xorshifted >>> rot) | (xorshifted << ((32 - rot) & 31))) >>> 0;
}

function pcg32NextIntInRange(
  rng: { state: bigint },
  inc: bigint,
  min: number,
  max: number
): number {
  const span = BigInt(max - min + 1);
  const threshold = ((1n << 32n) - span) % span;
  while (true) {
    const r = BigInt(pcg32Step(rng, inc));
    if (r >= threshold) {
      return min + Number(r % span);
    }
  }
}

// ── Validation Helpers (CWE-1321, CWE-681, CWE-190) ─────────────────────────

function isPlainObject(val: unknown): val is Record<string, unknown> {
  if (typeof val !== "object" || val === null || Array.isArray(val)) {
    return false;
  }
  // Reject own __proto__ property (CWE-1321 prototype pollution)
  if (Object.prototype.hasOwnProperty.call(val, "__proto__")) {
    return false;
  }
  // Reject prototype tampering
  const proto = Object.getPrototypeOf(val);
  return proto === Object.prototype || proto === null;
}

function validateU32(val: unknown): number | null {
  if (typeof val !== "number" || !Number.isSafeInteger(val)) {
    return null;
  }
  const normalized = Object.is(val, -0) ? 0 : val;
  if (normalized < 0 || normalized > MAX_U32) {
    return null;
  }
  return normalized;
}

function validateI32(val: unknown): number | null {
  if (typeof val !== "number" || !Number.isSafeInteger(val)) {
    return null;
  }
  const normalized = Object.is(val, -0) ? 0 : val;
  if (normalized < MIN_I32 || normalized > MAX_I32) {
    return null;
  }
  return normalized;
}

function validateU64(val: unknown): bigint | null {
  if (typeof val === "bigint") {
    if (val < 0n || val > MASK64) {
      return null;
    }
    return val;
  }
  if (typeof val === "number" && Number.isSafeInteger(val)) {
    const normalized = Object.is(val, -0) ? 0 : val;
    if (normalized < 0) {
      return null;
    }
    return BigInt(normalized);
  }
  return null;
}

// ── Simulation Engine API ───────────────────────────────────────────────────

/**
 * Initializes a new, deterministic 10-year reign for ruler Hamurabi.
 *
 * Seeds PCG32 with `(seed, stream_id = 54)` and performs the initial draw
 * for Year 1 land price (range 17..=26).
 */
export function new_game(seed: bigint | number): HamurabiState {
  let s: bigint;
  if (typeof seed === "bigint") {
    s = BigInt.asUintN(64, seed);
  } else if (typeof seed === "number" && Number.isSafeInteger(seed)) {
    const normalized = Object.is(seed, -0) ? 0 : seed;
    if (normalized < 0) {
      throw new RangeError("Seed must be non-negative");
    }
    s = BigInt.asUintN(64, BigInt(normalized));
  } else {
    throw new TypeError("Seed must be a non-negative safe integer or bigint");
  }

  const inc = HAMURABI_INC;
  const rng = { state: 0n };
  rng.state = BigInt.asUintN(64, rng.state * PCG_MULTIPLIER + inc);
  rng.state = BigInt.asUintN(64, rng.state + s);
  rng.state = BigInt.asUintN(64, rng.state * PCG_MULTIPLIER + inc);

  const land_price = pcg32NextIntInRange(rng, inc, 17, 26);

  return Object.freeze({
    year: 1,
    population: 100,
    bushels: 2800,
    acres: 1000,
    land_price,
    harvest_per_acre: 3,
    rats_eaten: 200,
    starved_last_year: 0,
    starved_total: 0,
    percent_starved_sum: 0,
    immigrants_last_year: 5,
    plague_struck: false,
    status: "Playing",
    rng_state: rng.state,
    rng_inc: inc,
  });
}

/**
 * Validates player orders against current state constraints without advancing state.
 *
 * Evaluates checks in the exact canonical order specified by GAME01 and Rust reference:
 * 1. Status check (Playing)
 * 2. State invariant and numerical bounds checks
 * 3. Land trade resource availability
 * 4. Food allocation feasibility
 * 5. Planting acreage and labor limits
 * 6. Seed grain availability
 * 7. Maximum harvest overflow guard
 */
export function validate_order(
  state: HamurabiState,
  order: TurnOrder
): Result<void, HamurabiError> {
  // Reject non-plain objects, poisoned prototypes, or missing inputs
  if (!isPlainObject(state) || !isPlainObject(order)) {
    return err(HamurabiError.InvalidOrderBounds);
  }

  // 1. Status Check
  if (state.status !== "Playing") {
    return err(HamurabiError.GameAlreadyEnded);
  }

  // 2. State Invariant Checks (VER04m state_inv & CWE-400 / CWE-835)
  // Check rng_inc (must be odd 64-bit integer)
  const rngInc = validateU64(state.rng_inc);
  if (rngInc === null || (rngInc & 1n) !== 1n) {
    return err(HamurabiError.InvalidOrderBounds);
  }

  // Check rng_state
  const rngState = validateU64(state.rng_state);
  if (rngState === null) {
    return err(HamurabiError.InvalidOrderBounds);
  }

  // Check year (playable reign is 1..=10)
  const year = validateU32(state.year);
  if (year === null || year < 1 || year > 10) {
    return err(HamurabiError.InvalidOrderBounds);
  }

  // Check land_price [17, 26]
  const landPrice = validateU32(state.land_price);
  if (landPrice === null || landPrice < 17 || landPrice > 26) {
    return err(HamurabiError.InvalidOrderBounds);
  }

  // Check harvest_per_acre [1, 5]
  const harvestPerAcre = validateU32(state.harvest_per_acre);
  if (harvestPerAcre === null || harvestPerAcre < 1 || harvestPerAcre > 5) {
    return err(HamurabiError.InvalidOrderBounds);
  }

  // Validate other state fields
  const population = validateU32(state.population);
  const bushels = validateU32(state.bushels);
  const acres = validateU32(state.acres);
  const ratsEaten = validateU32(state.rats_eaten);
  const starvedLastYear = validateU32(state.starved_last_year);
  const starvedTotal = validateU32(state.starved_total);
  const percentStarvedSum = validateU32(state.percent_starved_sum);
  const immigrantsLastYear = validateU32(state.immigrants_last_year);
  if (
    population === null ||
    bushels === null ||
    acres === null ||
    ratsEaten === null ||
    starvedLastYear === null ||
    starvedTotal === null ||
    percentStarvedSum === null ||
    immigrantsLastYear === null ||
    typeof state.plague_struck !== "boolean"
  ) {
    return err(HamurabiError.InvalidOrderBounds);
  }

  // Validate order fields
  const acresDelta = validateI32(order.acres_delta);
  const bushelsToFeed = validateU32(order.bushels_to_feed);
  const acresToPlant = validateU32(order.acres_to_plant);
  if (acresDelta === null || bushelsToFeed === null || acresToPlant === null) {
    return err(HamurabiError.InvalidOrderBounds);
  }

  // 3. Bounds & Overflow Guard on Land Trade
  if (acresDelta > 0) {
    const buyCount = BigInt(acresDelta);
    const cost = buyCount * BigInt(landPrice);
    if (cost > BigInt(bushels)) {
      return err(HamurabiError.InsufficientBushelsForLand);
    }
  } else if (acresDelta < 0) {
    const sellCount = BigInt(-acresDelta);
    if (sellCount > BigInt(acres)) {
      return err(HamurabiError.InsufficientAcresToSell);
    }
  }

  // 4. Land Trade Balances
  let acresAfterTrade: number;
  if (acresDelta >= 0) {
    const nextAcresBig = BigInt(acres) + BigInt(acresDelta);
    if (nextAcresBig > BigInt(MAX_U32)) {
      return err(HamurabiError.InvalidOrderBounds);
    }
    acresAfterTrade = Number(nextAcresBig);
  } else {
    const sellCount = BigInt(-acresDelta);
    acresAfterTrade = Number(BigInt(acres) - sellCount);
  }

  let bushelsAfterTrade: number;
  if (acresDelta > 0) {
    const cost = BigInt(acresDelta) * BigInt(landPrice);
    bushelsAfterTrade = Number(BigInt(bushels) - cost);
  } else if (acresDelta < 0) {
    const sellCount = BigInt(-acresDelta);
    const revenue = sellCount * BigInt(landPrice);
    const nextBushelsBig = BigInt(bushels) + revenue;
    if (nextBushelsBig > BigInt(MAX_U32)) {
      return err(HamurabiError.InvalidOrderBounds);
    }
    bushelsAfterTrade = Number(nextBushelsBig);
  } else {
    bushelsAfterTrade = bushels;
  }

  // 5. Food Allocation Check
  if (bushelsToFeed > bushelsAfterTrade) {
    return err(HamurabiError.InsufficientBushelsToFeed);
  }
  const bushelsAfterFeed = bushelsAfterTrade - bushelsToFeed;

  // 6. Planting Land & Labor Checks
  if (acresToPlant > acresAfterTrade) {
    return err(HamurabiError.InsufficientAcresToPlant);
  }
  const maxLaborAcres = BigInt(population) * 10n;
  if (BigInt(acresToPlant) > maxLaborAcres) {
    return err(HamurabiError.InsufficientPopulationToPlant);
  }

  // 7. Planting Seed Check: each bushel seeds 2 acres (ceil(acres / 2))
  const seedNeeded = Math.floor((acresToPlant + 1) / 2);
  if (seedNeeded > bushelsAfterFeed) {
    return err(HamurabiError.InsufficientGrainForSeed);
  }
  const bushelsAfterSeed = bushelsAfterFeed - seedNeeded;

  // 8. Maximum Harvest Overflow Guard (CWE-190)
  // Maximum harvest per acre is 5. Enforce bushelsAfterSeed + acresToPlant * 5 <= MAX_U32
  const maxHarvest = BigInt(acresToPlant) * 5n;
  const maxStored = BigInt(bushelsAfterSeed) + maxHarvest;
  if (maxStored > BigInt(MAX_U32)) {
    return err(HamurabiError.InvalidOrderBounds);
  }

  return ok(undefined);
}

/**
 * Advances the simulation by exactly one year according to player orders.
 *
 * Executes the strict two-phase transition:
 * - Phase A: Validation & constraint checking in canonical order.
 * - Phase B: Strictly locked PRNG draw sequence (harvest, rats, plague, starvation, immigration, next land price).
 *
 * Returns the updated immutable HamurabiState on success, or HamurabiError on refusal.
 * The original simulation state is never modified on error.
 */
export function step_year(
  state: HamurabiState,
  order: TurnOrder
): Result<HamurabiState, HamurabiError> {
  const valResult = validate_order(state, order);
  if (!valResult.ok) {
    return err(valResult.error);
  }

  const acresDelta = Object.is(order.acres_delta, -0) ? 0 : order.acres_delta;
  const bushelsToFeed = Object.is(order.bushels_to_feed, -0) ? 0 : order.bushels_to_feed;
  const acresToPlant = Object.is(order.acres_to_plant, -0) ? 0 : order.acres_to_plant;

  const landPrice = state.land_price;
  const acresAfterTrade = acresDelta >= 0
    ? state.acres + acresDelta
    : state.acres - (-acresDelta);

  const bushelsAfterTrade = acresDelta > 0
    ? state.bushels - Number(BigInt(acresDelta) * BigInt(landPrice))
    : acresDelta < 0
    ? state.bushels + Number(BigInt(-acresDelta) * BigInt(landPrice))
    : state.bushels;

  const bushelsAfterFeed = bushelsAfterTrade - bushelsToFeed;
  const seedNeeded = Math.floor((acresToPlant + 1) / 2);
  const bushelsAfterSeed = bushelsAfterFeed - seedNeeded;

  const rng = { state: BigInt.asUintN(64, state.rng_state) };
  const inc = BigInt.asUintN(64, state.rng_inc);

  // Step 1: Harvest Yield Draw
  // Draw 1: pcg.next_int_in_range(1, 5) -> harvest_per_acre
  const harvestPerAcre = pcg32NextIntInRange(rng, inc, 1, 5);
  const totalHarvest = BigInt(acresToPlant) * BigInt(harvestPerAcre);
  const bushelsStored = BigInt(bushelsAfterSeed) + totalHarvest;

  // Step 2: Rat Infestation Draws
  // Draw 2: pcg.next_int_in_range(1, 100) -> rat_roll
  const ratRoll = pcg32NextIntInRange(rng, inc, 1, 100);
  let ratsEaten = 0;
  if (ratRoll <= 40) {
    // Draw 3: pcg.next_int_in_range(1, 5) -> rat_factor
    const ratFactor = pcg32NextIntInRange(rng, inc, 1, 5);
    const eatenBig = bushelsStored / BigInt(ratFactor);
    ratsEaten = Number(eatenBig > BigInt(MAX_U32) ? BigInt(MAX_U32) : eatenBig);
  }
  const bushelsFinalBig = bushelsStored > BigInt(ratsEaten) ? bushelsStored - BigInt(ratsEaten) : 0n;
  const bushelsFinal = Number(bushelsFinalBig > BigInt(MAX_U32) ? BigInt(MAX_U32) : bushelsFinalBig);

  // Step 3: Plague Draw
  // Draw 4: pcg.next_int_in_range(1, 100) -> plague_roll
  const plagueRoll = pcg32NextIntInRange(rng, inc, 1, 100);
  const plagueStruck = plagueRoll <= 15;

  // Step 4: Starvation & Impeachment Evaluation
  const peopleFed = Math.floor(bushelsToFeed / 20);
  let starved: number;
  let survivors: number;
  if (peopleFed >= state.population) {
    starved = 0;
    survivors = state.population;
  } else {
    starved = state.population - peopleFed;
    survivors = peopleFed;
  }

  const percentStarved = state.population > 0
    ? Math.min(100, Math.floor((starved * 100) / state.population))
    : 0;

  const newStarvedTotalBig = BigInt(state.starved_total) + BigInt(starved);
  const newStarvedTotal = Number(
    newStarvedTotalBig > BigInt(MAX_U32) ? BigInt(MAX_U32) : newStarvedTotalBig
  );

  const newPercentSumBig = BigInt(state.percent_starved_sum) + BigInt(percentStarved);
  const newPercentSum = Number(
    newPercentSumBig > BigInt(MAX_U32) ? BigInt(MAX_U32) : newPercentSumBig
  );

  let newStatus: GameStatus;
  if (BigInt(starved) * 100n > BigInt(state.population) * 45n) {
    newStatus = `Impeached(starved=${starved}, percent=${percentStarved})`;
  } else if (survivors === 0) {
    newStatus = "GameOverExtinction";
  } else {
    newStatus = "Playing";
  }

  // Step 5: Plague Resolution
  if (plagueStruck && newStatus === "Playing") {
    survivors = Math.floor(survivors / 2);
  }

  // Step 6: Immigration Draw
  // Draw 5: pcg.next_int_in_range(1, 5) -> imm_factor
  const immFactor = pcg32NextIntInRange(rng, inc, 1, 5);
  let immigrants = 0;
  if (starved === 0 && newStatus === "Playing") {
    const acresTerm = BigInt(acresAfterTrade) * 20n;
    const inner = acresTerm + BigInt(bushelsFinal);
    const numerator = BigInt(immFactor) * inner;
    const denominator = 100n * BigInt(state.population);
    const immBig = numerator / denominator + 1n;
    immigrants = Number(immBig > BigInt(MAX_U32) ? BigInt(MAX_U32) : immBig);
  }

  const finalPopBig = BigInt(survivors) + BigInt(immigrants);
  const finalPopulation = Number(finalPopBig > BigInt(MAX_U32) ? BigInt(MAX_U32) : finalPopBig);

  // Step 7: Next Year Land Price Draw
  // Draw 6: pcg.next_int_in_range(17, 26) -> next_land_price
  const nextLandPrice = pcg32NextIntInRange(rng, inc, 17, 26);

  // Step 8: Turn Advance & Rating
  const nextYear = Math.min(255, state.year + 1);
  if (nextYear === 11 && newStatus === "Playing") {
    const avgStarved = Math.floor(newPercentSum / 10);
    const acresPerPerson =
      finalPopulation > 0 ? Math.floor(acresAfterTrade / finalPopulation) : 0;
    let rating: Rating;
    if (avgStarved <= 3 && acresPerPerson >= 10) {
      rating = "Charlemagne";
    } else if (avgStarved <= 10 && acresPerPerson >= 9) {
      rating = "Acre";
    } else if (avgStarved <= 33 && acresPerPerson >= 7) {
      rating = "Nero";
    } else {
      rating = "Disaster";
    }
    newStatus = `Won(${rating})`;
  }

  return ok(
    Object.freeze({
      year: nextYear,
      population: finalPopulation,
      bushels: bushelsFinal,
      acres: acresAfterTrade,
      land_price: nextLandPrice,
      harvest_per_acre: harvestPerAcre,
      rats_eaten: ratsEaten,
      starved_last_year: starved,
      starved_total: newStarvedTotal,
      percent_starved_sum: newPercentSum,
      immigrants_last_year: immigrants,
      plague_struck: plagueStruck,
      status: newStatus,
      rng_state: rng.state,
      rng_inc: inc,
    })
  );
}

/**
 * Formats the annual teletype report or final evaluation in authentic 1970s teletype styling.
 */
export function format_report(state: HamurabiState): string {
  if (typeof state.status === "string" && state.status.startsWith("Impeached(")) {
    const match = state.status.match(/starved=(\d+)/);
    const starved = match ? match[1] : String(state.starved_last_year);
    return (
      `YOU STARVED ${starved} PEOPLE IN ONE YEAR!!!\n` +
      `DUE TO THIS EXTREME MISMANAGEMENT YOU HAVE BEEN IMPEACHED AND THROWN OUT OF OFFICE!\n`
    );
  }

  if (state.status === "GameOverExtinction") {
    return "EVERYONE HAS DIED. THE CITY OF LAGASH IS EXTINCT!\n";
  }

  if (typeof state.status === "string" && state.status.startsWith("Won(")) {
    const avgStarved = Math.floor(state.percent_starved_sum / 10);
    const acresPerPerson = state.population > 0 ? Math.floor(state.acres / state.population) : 0;
    let ratingMsg = "";
    if (state.status === "Won(Charlemagne)") {
      ratingMsg =
        "A FANTASTIC PERFORMANCE! CHARLEMAGNE, DISRAELI, AND JEFFERSON COMBINED COULD NOT HAVE DONE BETTER!";
    } else if (state.status === "Won(Acre)") {
      ratingMsg =
        "YOUR PERFORMANCE COULD HAVE BEEN WORSE, BUT YOU LEFT THEM REASONABLE ACREAGE.";
    } else if (state.status === "Won(Nero)") {
      ratingMsg =
        "YOUR PERFORMANCE LEAVES MUCH TO BE DESIRED. NERO WOULD HAVE BEEN PROUD OF YOU.";
    } else {
      ratingMsg = "YOU DID SO POORLY YOU HAVE BEEN DECLARED AN OFFICIAL NATIONAL DISASTER!";
    }
    return (
      `IN YOUR 10-YEAR TERM OF OFFICE, ${avgStarved} PERCENT OF THE\n` +
      `POPULATION STARVED PER YEAR ON AVERAGE.\n` +
      `A TOTAL OF ${state.starved_total} PEOPLE DIED!!\n` +
      `YOU STARTED WITH 10 ACRES PER PERSON AND ENDED WITH\n` +
      `${acresPerPerson} ACRES PER PERSON.\n\n` +
      `${ratingMsg}\n`
    );
  }

  let out =
    `HAMURABI: I BEG TO REPORT TO YOU,\n` +
    `IN YEAR ${state.year}, ${state.starved_last_year} PEOPLE STARVED, ${state.immigrants_last_year} CAME TO THE CITY.\n`;
  if (state.plague_struck) {
    out += "A HORRIBLE PLAGUE STRUCK! HALF THE PEOPLE DIED.\n";
  }
  out +=
    `POPULATION IS NOW ${state.population}.\n` +
    `THE CITY NOW OWNS ${state.acres} ACRES.\n` +
    `YOU HARVESTED ${state.harvest_per_acre} BUSHELS PER ACRE.\n` +
    `RATS ATE ${state.rats_eaten} BUSHELS.\n` +
    `YOU NOW HAVE ${state.bushels} BUSHELS IN STORE.\n` +
    `LAND IS TRADING AT ${state.land_price} BUSHELS PER ACRE.\n`;
  return out;
}

// ── Aliases ─────────────────────────────────────────────────────────────────

export const newGame = new_game;
export const stepYear = step_year;
export const validateOrder = validate_order;
export const formatReport = format_report;
