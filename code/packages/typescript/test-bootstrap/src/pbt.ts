/**
 * Property-Based Testing Engine (VER05)
 *
 * Level 2 property testing engine providing generator algebra, integrated
 * shrinking, deterministic SplitMix64 replay with isolated sub-seeds, step
 * budgets, closed failure vocabulary, and privacy-preserving failure reporting.
 */

import { Value, Verdict } from "./types.js";
import { SplitMix64, fnv1a64 } from "./prng.js";
import { isRegisteredFailureId } from "./verdict.js";

// --- Closed Failure Vocabulary (VER05 §5.1) --------------------------------

export const FID_COUNTEREXAMPLE = "property-counterexample";
export const FID_EXHAUSTED = "property-exhausted";
export const FID_TIMEOUT = "property-timeout";
export const FID_PANIC = "property-panic";

export { isRegisteredFailureId };

export function isPbtFailureId(id: string): boolean {
  return (
    id === FID_COUNTEREXAMPLE ||
    id === FID_EXHAUSTED ||
    id === FID_TIMEOUT ||
    id === FID_PANIC
  );
}

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
    const w1 = rng.nextU64();
    const w2 = rng.nextU64();
    const raw = (w1 << 64n) | w2;
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
    if (minLen > maxLen) {
      throw new Error("minLen must be <= maxLen");
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
}

// --- Property Verification Options & Runner --------------------------------

export interface PbtOptions {
  numTests?: number;
  seed?: bigint | number;
  maxRetries?: number;
  maxShrinkSteps?: number;
  isCi?: boolean;
}

export function checkProperty<T>(
  seed: bigint | number,
  options: PbtOptions,
  genSample: (rng: SplitMix64) => T,
  prop: (val: T) => boolean,
  shrink?: (val: T, prop: (v: T) => boolean) => [T, number],
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
          [minimal, steps] = shrink(candidate, prop);
        } catch {
          // Shrink error fallback
        }
      }

      if (isCi) {
        const reproStr = `iter:${iteration};steps:${steps};shrunk:${JSON.stringify(minimal, (_, v) => typeof v === "bigint" ? v.toString() : v)}`;
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
        message: `property falsified at iteration ${iteration}: initial=${JSON.stringify(candidate, (_, v) => typeof v === "bigint" ? v.toString() : v)}, minimal=${JSON.stringify(minimal, (_, v) => typeof v === "bigint" ? v.toString() : v)} after ${steps} shrink steps`,
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
