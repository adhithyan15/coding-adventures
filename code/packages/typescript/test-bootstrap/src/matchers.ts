import { valuesEqual, toValue, encodeValue } from "./deep_equal.js";

export class AssertionError extends Error {
  readonly actual: unknown;
  readonly expected: unknown;

  constructor(message: string, actual?: unknown, expected?: unknown) {
    super(message);
    this.name = "AssertionError";
    this.actual = actual;
    this.expected = expected;
  }
}

export interface MatcherResult {
  pass: boolean;
  message: string;
  actual?: unknown;
  expected?: unknown;
}

export class Expectation<T = unknown> {
  private readonly isNot: boolean;

  constructor(protected readonly actual: T, isNot = false) {
    this.isNot = isNot;
  }

  get not(): Expectation<T> {
    return new Expectation(this.actual, !this.isNot);
  }

  private assert(result: MatcherResult): void {
    const passed = this.isNot ? !result.pass : result.pass;
    if (!passed) {
      const msg = this.isNot
        ? `not ${result.message}`
        : result.message;
      throw new AssertionError(msg, result.actual, result.expected);
    }
  }

  toBe(expected: unknown): void {
    const pass = Object.is(this.actual, expected);
    this.assert({
      pass,
      message: `expected ${String(this.actual)} to be ${String(expected)}`,
      actual: this.actual,
      expected,
    });
  }

  toEqual(expected: unknown): void {
    const aVal = toValue(this.actual);
    const eVal = toValue(expected);
    const pass = valuesEqual(aVal, eVal);
    this.assert({
      pass,
      message: `expected ${encodeValue(aVal)} to equal ${encodeValue(eVal)}`,
      actual: this.actual,
      expected,
    });
  }

  toStrictEqual(expected: unknown): void {
    this.toEqual(expected);
  }

  toBeTruthy(): void {
    const pass = Boolean(this.actual);
    this.assert({
      pass,
      message: `expected ${String(this.actual)} to be truthy`,
      actual: this.actual,
    });
  }

  toBeFalsy(): void {
    const pass = !this.actual;
    this.assert({
      pass,
      message: `expected ${String(this.actual)} to be falsy`,
      actual: this.actual,
    });
  }

  toBeNull(): void {
    const pass = this.actual === null;
    this.assert({
      pass,
      message: `expected ${String(this.actual)} to be null`,
      actual: this.actual,
      expected: null,
    });
  }

  toBeUndefined(): void {
    const pass = this.actual === undefined;
    this.assert({
      pass,
      message: `expected ${String(this.actual)} to be undefined`,
      actual: this.actual,
      expected: undefined,
    });
  }

  toBeDefined(): void {
    const pass = this.actual !== undefined;
    this.assert({
      pass,
      message: `expected value to be defined`,
      actual: this.actual,
    });
  }

  toBeInstanceOf(cls: new (...args: unknown[]) => unknown): void {
    const pass = this.actual instanceof cls;
    this.assert({
      pass,
      message: `expected value to be instance of ${(cls as { name?: string }).name ?? "Function"}`,
      actual: this.actual,
    });
  }

  toBeNaN(): void {
    const pass = typeof this.actual === "number" && Number.isNaN(this.actual);
    this.assert({
      pass,
      message: `expected ${String(this.actual)} to be NaN`,
      actual: this.actual,
    });
  }

  toBeGreaterThan(n: number | bigint): void {
    const pass = (this.actual as number | bigint) > n;
    this.assert({
      pass,
      message: `expected ${String(this.actual)} > ${String(n)}`,
      actual: this.actual,
      expected: n,
    });
  }

  toBeGreaterThanOrEqual(n: number | bigint): void {
    const pass = (this.actual as number | bigint) >= n;
    this.assert({
      pass,
      message: `expected ${String(this.actual)} >= ${String(n)}`,
      actual: this.actual,
      expected: n,
    });
  }

  toBeLessThan(n: number | bigint): void {
    const pass = (this.actual as number | bigint) < n;
    this.assert({
      pass,
      message: `expected ${String(this.actual)} < ${String(n)}`,
      actual: this.actual,
      expected: n,
    });
  }

  toBeLessThanOrEqual(n: number | bigint): void {
    const pass = (this.actual as number | bigint) <= n;
    this.assert({
      pass,
      message: `expected ${String(this.actual)} <= ${String(n)}`,
      actual: this.actual,
      expected: n,
    });
  }

  toBeCloseTo(expected: number, precision = 2): void {
    const tolerance = Math.pow(10, -precision) / 2;
    const pass = Math.abs((this.actual as number) - expected) < tolerance;
    this.assert({
      pass,
      message: `expected ${String(this.actual)} to be close to ${String(expected)} within tolerance ${tolerance}`,
      actual: this.actual,
      expected,
    });
  }

  toContain(item: unknown): void {
    let pass = false;
    if (typeof this.actual === "string" && typeof item === "string") {
      pass = this.actual.includes(item);
    } else if (Array.isArray(this.actual)) {
      const itemVal = toValue(item);
      pass = this.actual.some((x) => valuesEqual(toValue(x), itemVal));
    } else if (this.actual instanceof Set) {
      const itemVal = toValue(item);
      pass = Array.from(this.actual).some((x) => valuesEqual(toValue(x), itemVal));
    } else if (this.actual instanceof Map) {
      const itemVal = toValue(item);
      pass = Array.from(this.actual.keys()).some((k) => valuesEqual(toValue(k), itemVal));
    }
    this.assert({
      pass,
      message: `expected collection to contain ${String(item)}`,
      actual: this.actual,
      expected: item,
    });
  }

  toHaveLength(expectedLength: number): void {
    const actualLen = (this.actual as { length?: number })?.length;
    const pass = actualLen === expectedLength;
    this.assert({
      pass,
      message: `expected length ${expectedLength}, got ${actualLen}`,
      actual: actualLen,
      expected: expectedLength,
    });
  }

  toThrow(expected?: string | RegExp | Error | (new (...args: unknown[]) => Error)): void {
    if (typeof this.actual !== "function") {
      throw new AssertionError("toThrow() requires a function", this.actual);
    }
    let threw = false;
    let caught: unknown = null;
    try {
      (this.actual as () => void)();
    } catch (err) {
      threw = true;
      caught = err;
    }

    if (!threw) {
      this.assert({
        pass: false,
        message: "expected function to throw an error, but it returned normally",
      });
      return;
    }

    if (expected === undefined) {
      this.assert({ pass: true, message: "function threw an error" });
      return;
    }

    const errMessage = caught instanceof Error ? caught.message : String(caught);

    if (typeof expected === "string") {
      const pass = errMessage.includes(expected);
      this.assert({
        pass,
        message: `expected error message to include ${JSON.stringify(expected)}, got ${JSON.stringify(errMessage)}`,
        actual: errMessage,
        expected,
      });
    } else if (expected instanceof RegExp) {
      const pass = expected.test(errMessage);
      this.assert({
        pass,
        message: `expected error message to match ${String(expected)}, got ${JSON.stringify(errMessage)}`,
        actual: errMessage,
        expected: String(expected),
      });
    } else if (typeof expected === "function") {
      const pass = caught instanceof expected;
      this.assert({
        pass,
        message: `expected error to be instance of ${(expected as { name?: string }).name ?? "Error"}`,
        actual: caught,
      });
    }
  }
}

export function expect<T>(actual: T): Expectation<T> {
  return new Expectation(actual);
}
