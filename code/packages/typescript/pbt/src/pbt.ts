/**
 * Property-Based Testing Engine (VER05)
 *
 * Zero-dependency Level 2 property testing engine providing generator algebra,
 * integrated shrinking, deterministic SplitMix64 replay with isolated sub-seeds,
 * step budgets, closed failure vocabulary, and privacy-preserving failure reporting.
 */

// --- PRNG: SplitMix64 & FNV-1a Hash (VER05 §3) -----------------------------

const GOLDEN_GAMMA = 0x9e37_79b9_7f4a_7c15n;
const SCRAMBLE_1 = 0xbf58_476d_1ce4_e5b9n;
const SCRAMBLE_2 = 0x94d0_49bb_1331_11ebn;

export class SplitMix64 {
  private state: bigint;

  constructor(seed: bigint | number) {
    this.state = BigInt.asUintN(64, BigInt(seed));
  }

  nextU64(): bigint {
    this.state = BigInt.asUintN(64, this.state + GOLDEN_GAMMA);
    let z = this.state;
    z = BigInt.asUintN(64, (z ^ (z >> 30n)) * SCRAMBLE_1);
    z = BigInt.asUintN(64, (z ^ (z >> 27n)) * SCRAMBLE_2);
    return BigInt.asUintN(64, z ^ (z >> 31n));
  }

  bounded(n: number | bigint): number {
    const bn = BigInt(n);
    if (bn <= 1n) return 0;
    if (bn > 0x1_0000_0000n) {
      throw new Error(`bounded(n) needs 1 <= n <= 2^32, got ${n}`);
    }
    const threshold = BigInt.asUintN(64, -bn) % bn;
    while (true) {
      const x = this.nextU64();
      if (x >= threshold) {
        return Number(x % bn);
      }
    }
  }
}

export function fnv1a64(data: Uint8Array): bigint {
  let hash = 0xcbf29ce484222325n;
  const prime = 0x100000001b3n;
  for (let i = 0; i < data.length; i++) {
    hash ^= BigInt(data[i]);
    hash = BigInt.asUintN(64, hash * prime);
  }
  return hash;
}

export function subSeed(runSeed: bigint | number, propertyId: string): bigint {
  const encoder = new TextEncoder();
  const idHash = fnv1a64(encoder.encode(propertyId));
  const s = BigInt.asUintN(64, BigInt(runSeed));
  return BigInt.asUintN(64, s ^ idHash);
}

// --- Value & Verdict Types --------------------------------------------------

export type Value =
  | { kind: "bool"; value: boolean }
  | { kind: "int"; text: string }
  | { kind: "bytes"; value: Uint8Array };

export type Verdict =
  | { status: "pass" }
  | { status: "fail"; failureId: string; message: string };

// --- Closed Failure Vocabulary (VER05 §5.1) --------------------------------

export const FID_COUNTEREXAMPLE = "property-counterexample";
export const FID_EXHAUSTED = "property-exhausted";
export const FID_TIMEOUT = "property-timeout";
export const FID_PANIC = "property-panic";

export const ALL_PBT_FAILURE_IDS = [
  FID_COUNTEREXAMPLE,
  FID_EXHAUSTED,
  FID_TIMEOUT,
  FID_PANIC,
] as const;

export type PbtFailureId = typeof ALL_PBT_FAILURE_IDS[number];

export function isPbtFailureId(id: string): boolean {
  return (
    id === FID_COUNTEREXAMPLE ||
    id === FID_EXHAUSTED ||
    id === FID_TIMEOUT ||
    id === FID_PANIC
  );
}

export const isRegisteredFailureId = isPbtFailureId;

export class ExhaustedError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ExhaustedError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

// --- Primitive Generators (VER05 §3.1) -------------------------------------

export function genBool(rng: SplitMix64): boolean {
  return rng.bounded(2) === 1;
}

export function genU8(rng: SplitMix64): number {
  return rng.bounded(256);
}

export function genU64(rng: SplitMix64): bigint {
  return rng.nextU64();
}

export function genI128(rng: SplitMix64): bigint {
  const w1 = rng.nextU64();
  const w2 = rng.nextU64();
  let val = (w1 << 64n) | w2;
  if (val >= 1n << 127n) {
    val -= 1n << 128n;
  }
  return val;
}

export function genPrimitive(
  seed: bigint | number,
  typeName: string,
  count: number,
): Value[] {
  const rng = new SplitMix64(seed);
  const outputs: Value[] = [];
  for (let i = 0; i < count; i++) {
    switch (typeName) {
      case "bool":
        outputs.push({ kind: "bool", value: genBool(rng) });
        break;
      case "u8":
        outputs.push({ kind: "int", text: genU8(rng).toString() });
        break;
      case "u64":
        outputs.push({ kind: "int", text: genU64(rng).toString() });
        break;
      case "i128":
        outputs.push({ kind: "int", text: genI128(rng).toString() });
        break;
      default:
        throw new Error(`unknown primitive type: ${typeName}`);
    }
  }
  return outputs;
}

// --- Bounded Integer Generator (VER05 §3.1) ---------------------------------

export function boundedU64(rng: SplitMix64, n: bigint): bigint {
  if (n <= 0x1_0000_0000n) {
    return BigInt(rng.bounded(Number(n)));
  }
  const threshold = BigInt.asUintN(64, -n) % n;
  while (true) {
    const x = rng.nextU64();
    if (x >= threshold) {
      return x % n;
    }
  }
}

export function genBoundedInt(
  rng: SplitMix64,
  minVal: bigint,
  maxVal: bigint,
): bigint {
  if (minVal > maxVal) {
    throw new Error("minVal must be <= maxVal");
  }
  const span = maxVal - minVal + 1n;
  let offset: bigint;
  if (span <= 0x1_0000_0000n) {
    offset = BigInt(rng.bounded(Number(span)));
  } else if (span < 1n << 64n) {
    offset = boundedU64(rng, span);
  } else if (span === 1n << 64n) {
    offset = rng.nextU64();
  } else {
    // Multi-word BigInt span
    const wordsNeeded = Math.ceil(span.toString(2).length / 64) + 1;
    let raw = 0n;
    for (let w = 0; w < wordsNeeded; w++) {
      raw = (raw << 64n) | rng.nextU64();
    }
    offset = raw % span;
  }
  return minVal + offset;
}

export function genBounded(
  seed: bigint | number,
  minVal: bigint,
  maxVal: bigint,
  count: number,
): Value[] {
  const rng = new SplitMix64(seed);
  const outputs: Value[] = [];
  for (let i = 0; i < count; i++) {
    const val = genBoundedInt(rng, minVal, maxVal);
    outputs.push({ kind: "int", text: val.toString() });
  }
  return outputs;
}

// --- Byte Sequence Generator (VER05 §3.1) -----------------------------------

export function genBytes(
  seed: bigint | number,
  minLen: number,
  maxLen: number,
  count: number,
): Value[] {
  if (minLen > maxLen || minLen < 0) {
    throw new Error("invalid byte lengths: minLen <= maxLen and minLen >= 0");
  }
  const rng = new SplitMix64(seed);
  const span = maxLen - minLen + 1;
  const outputs: Value[] = [];
  for (let i = 0; i < count; i++) {
    const length = minLen + rng.bounded(span);
    const data = new Uint8Array(length);
    for (let j = 0; j < length; j++) {
      data[j] = rng.bounded(256);
    }
    outputs.push({ kind: "bytes", value: data });
  }
  return outputs;
}

// --- Euclidean Division for Integer Shrinking -------------------------------

function divEuclid(a: bigint, b: bigint): bigint {
  if (b === 0n) throw new Error("division by zero");
  const q = a / b;
  const r = a % b;
  if (r !== 0n && ((a < 0n && b > 0n) || (a > 0n && b < 0n))) {
    return q - 1n;
  }
  return q;
}

// --- Integrated Shrinking (VER05 §4) ----------------------------------------

export function shrinkInt(
  initial: bigint,
  target: bigint,
  minVal: bigint,
  maxVal: bigint,
  prop: (x: bigint) => boolean,
  maxSteps: number = 1000,
): [bigint, number] {
  if (prop(initial)) {
    throw new Error(`initial value ${initial} must falsify property`);
  }
  let current = initial;
  let steps = 0;

  if (minVal <= target && target <= maxVal && target !== current && !prop(target)) {
    return [target, 1];
  }

  while (steps < maxSteps) {
    if (current === target) break;
    const diff = current - target;
    let progress = false;
    const absDiff = diff < 0n ? -diff : diff;

    if (absDiff > 1n) {
      const step = divEuclid(diff, 2n);
      const candidate = current - step;
      if (
        minVal <= candidate &&
        candidate <= maxVal &&
        !prop(candidate) &&
        candidate !== current
      ) {
        current = candidate;
        steps++;
        progress = true;
      }
    }

    if (!progress) {
      const step = diff > 0n ? 1n : -1n;
      const candidate = current - step;
      if (
        minVal <= candidate &&
        candidate <= maxVal &&
        !prop(candidate) &&
        candidate !== current
      ) {
        current = candidate;
        steps++;
        progress = true;
      }
    }

    if (!progress) break;
  }

  return [current, steps];
}

export function shrinkList(
  initial: bigint[],
  elemTarget: bigint,
  prop: (slice: bigint[]) => boolean,
  maxSteps: number = 1000,
): [bigint[], number] {
  if (prop(initial)) {
    throw new Error("initial list must falsify property");
  }
  let current = [...initial];
  let steps = 0;

  if (!prop([]) && current.length > 0) {
    return [[], 1];
  }

  // Phase 1: Bisection
  while (current.length > 1 && steps < maxSteps) {
    const mid = Math.floor(current.length / 2);
    const left = current.slice(0, mid);
    if (!prop(left)) {
      current = left;
      steps++;
      continue;
    }
    const right = current.slice(mid);
    if (!prop(right)) {
      current = right;
      steps++;
      continue;
    }
    break;
  }

  // Phase 2: Single-element deletion from right to left
  let i = current.length - 1;
  while (i >= 0 && current.length > 1 && steps < maxSteps) {
    const cand = [...current.slice(0, i), ...current.slice(i + 1)];
    if (!prop(cand)) {
      current = cand;
      steps++;
      i = Math.min(i, current.length - 1);
    } else {
      i--;
    }
  }

  // Phase 3: Element-wise reduction from left to right
  for (let idx = 0; idx < current.length; idx++) {
    while (steps < maxSteps) {
      if (current[idx] === elemTarget) break;
      const diff = current[idx] - elemTarget;
      let progress = false;
      const absDiff = diff < 0n ? -diff : diff;

      if (absDiff > 1n) {
        const step = divEuclid(diff, 2n);
        const candVal = current[idx] - step;
        const candList = [
          ...current.slice(0, idx),
          candVal,
          ...current.slice(idx + 1),
        ];
        if (!prop(candList)) {
          current = candList;
          steps++;
          progress = true;
        }
      }

      if (!progress) {
        const step = diff > 0n ? 1n : -1n;
        const candVal = current[idx] - step;
        const candList = [
          ...current.slice(0, idx),
          candVal,
          ...current.slice(idx + 1),
        ];
        if (!prop(candList)) {
          current = candList;
          steps++;
          progress = true;
        }
      }

      if (!progress) break;
    }
  }

  return [current, steps];
}

export function shrinkBytes(
  initialBytes: Uint8Array,
  prop: (b: Uint8Array) => boolean,
  maxSteps: number = 1000,
): [Uint8Array, number] {
  const initial = Array.from(initialBytes, (b) => BigInt(b));
  const [shrunkList, steps] = shrinkList(
    initial,
    0n,
    (slice) => {
      const arr = new Uint8Array(slice.map((x) => Number(x)));
      return prop(arr);
    },
    maxSteps,
  );
  return [new Uint8Array(shrunkList.map((x) => Number(x))), steps];
}

// --- Replay Consistency Simulation (VER05 §5) -------------------------------

export function simulateReplay(
  seed: bigint | number,
  prop: (x: bigint) => boolean,
): [bigint, bigint] {
  const rng = new SplitMix64(seed);
  let initialFail: bigint | null = null;
  for (let i = 0; i < 1000; i++) {
    const val = BigInt(rng.bounded(1000));
    if (!prop(val)) {
      initialFail = val;
      break;
    }
  }
  if (initialFail === null) {
    throw new Error("property was not falsified within 1000 draws");
  }
  const [minimal] = shrinkInt(initialFail, 0n, 0n, 1000n, prop);
  return [initialFail, minimal];
}

// --- Generator Combinator Algebra (`Gen<T>`) --------------------------------

export class Gen<T> {
  constructor(public readonly sample: (rng: SplitMix64) => T) {}

  map<U>(fn: (val: T) => U): Gen<U> {
    return new Gen((rng) => fn(this.sample(rng)));
  }

  filter(predicate: (val: T) => boolean, maxRetries: number = 100): Gen<T> {
    return new Gen((rng) => {
      for (let i = 0; i < maxRetries; i++) {
        const val = this.sample(rng);
        if (predicate(val)) {
          return val;
        }
      }
      throw new ExhaustedError(
        `Filter precondition rejected all candidates after ${maxRetries} attempts`,
      );
    });
  }

  flatMap<U>(fn: (val: T) => Gen<U>): Gen<U> {
    return new Gen((rng) => {
      const intermediate = this.sample(rng);
      return fn(intermediate).sample(rng);
    });
  }

  static constant<T>(val: T): Gen<T> {
    return new Gen(() => val);
  }

  static bool(): Gen<boolean> {
    return new Gen(genBool);
  }

  static u8(): Gen<number> {
    return new Gen(genU8);
  }

  static u64(): Gen<bigint> {
    return new Gen(genU64);
  }

  static integer(min: bigint | number, max: bigint | number): Gen<bigint> {
    const bmin = BigInt(min);
    const bmax = BigInt(max);
    return new Gen((rng) => genBoundedInt(rng, bmin, bmax));
  }

  static bytes(minLen: number, maxLen: number): Gen<Uint8Array> {
    if (minLen > maxLen || minLen < 0) {
      throw new Error("invalid byte lengths: minLen <= maxLen and minLen >= 0");
    }
    return new Gen((rng) => {
      const span = maxLen - minLen + 1;
      const length = minLen + rng.bounded(span);
      const buf = new Uint8Array(length);
      for (let i = 0; i < length; i++) {
        buf[i] = rng.bounded(256);
      }
      return buf;
    });
  }

  static list<T>(minLen: number, maxLen: number, elemGen: Gen<T>): Gen<T[]> {
    if (minLen > maxLen || minLen < 0) {
      throw new Error("invalid list lengths: minLen <= maxLen and minLen >= 0");
    }
    return new Gen((rng) => {
      const span = maxLen - minLen + 1;
      const length = minLen + rng.bounded(span);
      const arr: T[] = [];
      for (let i = 0; i < length; i++) {
        arr.push(elemGen.sample(rng));
      }
      return arr;
    });
  }

  static oneOf<T>(...gens: Gen<T>[]): Gen<T> {
    if (gens.length === 0) {
      throw new Error("oneOf requires at least one generator");
    }
    return new Gen((rng) => {
      const idx = rng.bounded(gens.length);
      return gens[idx].sample(rng);
    });
  }

  static tuple<T extends unknown[]>(
    ...gens: { [K in keyof T]: Gen<T[K]> }
  ): Gen<T> {
    return new Gen((rng) => {
      return gens.map((g) => g.sample(rng)) as T;
    });
  }
}

// --- Property Verification Options & Runner --------------------------------

export interface PbtOptions {
  numTests?: number;
  seed?: bigint | number;
  maxRetries?: number;
  maxShrinkSteps?: number;
  isCi?: boolean;
}

function safeStringify(val: unknown): string {
  try {
    return JSON.stringify(val, (_, v) =>
      typeof v === "bigint" ? v.toString() : v,
    );
  } catch {
    return String(val);
  }
}

export function checkProperty<T>(
  seed: bigint | number,
  options: PbtOptions,
  genSample: (rng: SplitMix64) => T,
  prop: (val: T) => boolean,
  shrink?: (val: T, prop: (v: T) => boolean, maxSteps?: number) => [T, number],
): Verdict {
  const rng = new SplitMix64(seed);
  const numTests = options.numTests ?? 100;
  const envCi = (globalThis as unknown as { process?: { env?: Record<string, string> } })
    .process?.env?.CI;
  const isCi = options.isCi ?? Boolean(envCi);

  for (let iteration = 1; iteration <= numTests; iteration++) {
    let candidate: T;
    try {
      candidate = genSample(rng);
    } catch (err) {
      if (err instanceof ExhaustedError) {
        return {
          status: "fail",
          failureId: FID_EXHAUSTED,
          message: `filter precondition rejected all candidates at iteration ${iteration}`,
        };
      }
      return {
        status: "fail",
        failureId: FID_EXHAUSTED,
        message: `generator error at iteration ${iteration}: ${err instanceof Error ? err.message : String(err)}`,
      };
    }

    let pass: boolean;
    try {
      pass = prop(candidate);
    } catch (err) {
      return {
        status: "fail",
        failureId: FID_PANIC,
        message: `property evaluation panicked at iteration ${iteration}: ${err instanceof Error ? err.message : String(err)}`,
      };
    }

    if (!pass) {
      let minimal = candidate;
      let steps = 0;
      if (shrink) {
        try {
          [minimal, steps] = shrink(
            candidate,
            prop,
            options.maxShrinkSteps ?? 1000,
          );
        } catch {
          // Shrink error fallback
        }
      }

      if (isCi) {
        const reproStr = `iter:${iteration};steps:${steps};shrunk:${safeStringify(minimal)}`;
        const encoder = new TextEncoder();
        const digest = fnv1a64(encoder.encode(reproStr));
        return {
          status: "fail",
          failureId: FID_COUNTEREXAMPLE,
          message: `property falsified after ${iteration} iterations (shrunk in ${steps} steps, repro-fnv: ${digest.toString(16).padStart(16, "0")})`,
        };
      }

      return {
        status: "fail",
        failureId: FID_COUNTEREXAMPLE,
        message: `property falsified at iteration ${iteration}: initial=${safeStringify(candidate)}, minimal=${safeStringify(minimal)} after ${steps} shrink steps`,
      };
    }
  }

  return { status: "pass" };
}

export function assertProperty<T>(
  gen: Gen<T>,
  prop: (val: T) => boolean | void,
  options?: PbtOptions,
): void {
  const seed = options?.seed ?? 42n;
  const verdict = checkProperty(
    seed,
    options ?? {},
    (rng) => gen.sample(rng),
    (val) => {
      const res = prop(val);
      return res !== false;
    },
  );

  if (verdict.status === "fail") {
    const err = new Error(`[${verdict.failureId}] ${verdict.message}`);
    (err as unknown as { failureId: string }).failureId = verdict.failureId;
    throw err;
  }
}
