import { valuesEqual, toValue, encodeValue } from "./deep_equal.js";

export class AssertionError extends Error {
  readonly actual: unknown;
  readonly expected: unknown;
  readonly operator?: string;

  constructor(
    message: string,
    actual?: unknown,
    expected?: unknown,
    operator?: string
  ) {
    super(message);
    this.name = "AssertionError";
    this.actual = actual;
    this.expected = expected;
    this.operator = operator;
  }
}

export interface MatcherResult {
  pass: boolean;
  message: string;
  actual?: unknown;
  expected?: unknown;
  operator?: string;
}

function hasMatchingPrototypes(
  a: unknown,
  b: unknown,
  seen = new Set<unknown>()
): boolean {
  if (a === b) return true;
  if (
    a === null ||
    b === null ||
    (typeof a !== "object" && typeof a !== "function") ||
    (typeof b !== "object" && typeof b !== "function")
  ) {
    return true;
  }
  if (seen.has(a)) return true;
  seen.add(a);

  if (Object.getPrototypeOf(a) !== Object.getPrototypeOf(b)) {
    return false;
  }
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (!hasMatchingPrototypes(a[i], b[i], seen)) return false;
    }
    return true;
  }
  if (typeof a === "object" && typeof b === "object") {
    const keysA = Object.keys(a as object);
    const keysB = Object.keys(b as object);
    if (keysA.length !== keysB.length) return false;
    for (const k of keysA) {
      if (
        !hasMatchingPrototypes(
          (a as Record<string, unknown>)[k],
          (b as Record<string, unknown>)[k],
          seen
        )
      ) {
        return false;
      }
    }
  }
  return true;
}

export class Expectation<T = unknown> {
  protected readonly isNot: boolean;

  constructor(protected readonly actual: T, isNot = false) {
    this.isNot = isNot;
  }

  get not(): Expectation<T> {
    return new Expectation(this.actual, !this.isNot);
  }

  get resolves(): AsyncExpectation<T> {
    return new AsyncExpectation(
      Promise.resolve(this.actual),
      this.isNot,
      false
    );
  }

  get rejects(): AsyncExpectation<unknown> {
    const promise = Promise.resolve(this.actual).then(
      (val) => {
        throw new AssertionError("expected promise to reject, but it resolved", val);
      },
      (err) => err
    );
    return new AsyncExpectation(promise, this.isNot, true);
  }

  protected assert(result: MatcherResult): void {
    const passed = this.isNot ? !result.pass : result.pass;
    if (!passed) {
      const msg = this.isNot ? `not ${result.message}` : result.message;
      throw new AssertionError(
        msg,
        result.actual,
        result.expected,
        result.operator
      );
    }
  }

  protected assertLazy(
    pass: boolean,
    messageFn: () => string,
    actual?: unknown,
    expected?: unknown,
    operator?: string
  ): void {
    const passed = this.isNot ? !pass : pass;
    if (!passed) {
      const msg = this.isNot ? `not ${messageFn()}` : messageFn();
      throw new AssertionError(msg, actual, expected, operator);
    }
  }

  toBe(expected: unknown): void {
    const pass = Object.is(this.actual, expected);
    this.assertLazy(
      pass,
      () => `expected ${String(this.actual)} to be ${String(expected)}`,
      this.actual,
      expected,
      "toBe"
    );
  }

  toEqual(expected: unknown): void {
    const aVal = toValue(this.actual);
    const eVal = toValue(expected);
    const pass = valuesEqual(aVal, eVal);
    this.assertLazy(
      pass,
      () => `expected ${encodeValue(aVal)} to equal ${encodeValue(eVal)}`,
      this.actual,
      expected,
      "toEqual"
    );
  }

  toStrictEqual(expected: unknown): void {
    const protoMatch = hasMatchingPrototypes(this.actual, expected);
    if (!protoMatch) {
      this.assertLazy(
        false,
        () => `expected objects to have matching prototypes`,
        this.actual,
        expected,
        "toStrictEqual"
      );
      return;
    }
    this.toEqual(expected);
  }

  toBeTruthy(): void {
    const pass = Boolean(this.actual);
    this.assertLazy(
      pass,
      () => `expected ${String(this.actual)} to be truthy`,
      this.actual,
      undefined,
      "toBeTruthy"
    );
  }

  toBeFalsy(): void {
    const pass = !this.actual;
    this.assertLazy(
      pass,
      () => `expected ${String(this.actual)} to be falsy`,
      this.actual,
      undefined,
      "toBeFalsy"
    );
  }

  toBeNull(): void {
    const pass = this.actual === null;
    this.assertLazy(
      pass,
      () => `expected ${String(this.actual)} to be null`,
      this.actual,
      null,
      "toBeNull"
    );
  }

  toBeUndefined(): void {
    const pass = this.actual === undefined;
    this.assertLazy(
      pass,
      () => `expected ${String(this.actual)} to be undefined`,
      this.actual,
      undefined,
      "toBeUndefined"
    );
  }

  toBeDefined(): void {
    const pass = this.actual !== undefined;
    this.assertLazy(
      pass,
      () => `expected value to be defined`,
      this.actual,
      undefined,
      "toBeDefined"
    );
  }

  toBeInstanceOf(cls: new (...args: unknown[]) => unknown): void {
    const pass = this.actual instanceof cls;
    this.assertLazy(
      pass,
      () =>
        `expected value to be instance of ${(cls as { name?: string }).name ?? "Function"}`,
      this.actual,
      undefined,
      "toBeInstanceOf"
    );
  }

  toBeNaN(): void {
    const pass = typeof this.actual === "number" && Number.isNaN(this.actual);
    this.assertLazy(
      pass,
      () => `expected ${String(this.actual)} to be NaN`,
      this.actual,
      undefined,
      "toBeNaN"
    );
  }

  toBeGreaterThan(n: number | bigint): void {
    const pass = (this.actual as number | bigint) > n;
    this.assertLazy(
      pass,
      () => `expected ${String(this.actual)} > ${String(n)}`,
      this.actual,
      n,
      "toBeGreaterThan"
    );
  }

  toBeGreaterThanOrEqual(n: number | bigint): void {
    const pass = (this.actual as number | bigint) >= n;
    this.assertLazy(
      pass,
      () => `expected ${String(this.actual)} >= ${String(n)}`,
      this.actual,
      n,
      "toBeGreaterThanOrEqual"
    );
  }

  toBeLessThan(n: number | bigint): void {
    const pass = (this.actual as number | bigint) < n;
    this.assertLazy(
      pass,
      () => `expected ${String(this.actual)} < ${String(n)}`,
      this.actual,
      n,
      "toBeLessThan"
    );
  }

  toBeLessThanOrEqual(n: number | bigint): void {
    const pass = (this.actual as number | bigint) <= n;
    this.assertLazy(
      pass,
      () => `expected ${String(this.actual)} <= ${String(n)}`,
      this.actual,
      n,
      "toBeLessThanOrEqual"
    );
  }

  toBeCloseTo(expected: number, precision = 2): void {
    const tolerance = Math.pow(10, -precision) / 2;
    const pass = Math.abs((this.actual as number) - expected) < tolerance;
    this.assertLazy(
      pass,
      () =>
        `expected ${String(this.actual)} to be close to ${String(expected)} within tolerance ${tolerance}`,
      this.actual,
      expected,
      "toBeCloseTo"
    );
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
      pass = Array.from(this.actual).some((x) =>
        valuesEqual(toValue(x), itemVal)
      );
    } else if (this.actual instanceof Map) {
      const itemVal = toValue(item);
      pass = Array.from(this.actual.keys()).some((k) =>
        valuesEqual(toValue(k), itemVal)
      );
    }
    this.assertLazy(
      pass,
      () => `expected collection to contain ${String(item)}`,
      this.actual,
      item,
      "toContain"
    );
  }

  toHaveLength(expectedLength: number): void {
    const actualLen = (this.actual as { length?: number })?.length;
    const pass = actualLen === expectedLength;
    this.assertLazy(
      pass,
      () => `expected length ${expectedLength}, got ${actualLen}`,
      actualLen,
      expectedLength,
      "toHaveLength"
    );
  }

  toMatch(pattern: string | RegExp): void {
    if (typeof this.actual !== "string") {
      throw new AssertionError(
        "toMatch() requires actual to be a string",
        this.actual
      );
    }
    if (pattern instanceof RegExp) {
      pattern.lastIndex = 0;
    }
    const pass =
      typeof pattern === "string"
        ? this.actual.includes(pattern)
        : pattern.test(this.actual);
    this.assertLazy(
      pass,
      () => `expected ${JSON.stringify(this.actual)} to match ${String(pattern)}`,
      this.actual,
      pattern,
      "toMatch"
    );
  }

  toHaveProperty(key: string, expectedValue?: unknown): void {
    if (this.actual === null || typeof this.actual !== "object") {
      this.assertLazy(
        false,
        () => `expected object with property ${JSON.stringify(key)}, got ${String(this.actual)}`,
        this.actual,
        undefined,
        "toHaveProperty"
      );
      return;
    }
    const hasProp = Object.prototype.hasOwnProperty.call(this.actual, key);
    if (!hasProp) {
      this.assertLazy(
        false,
        () => `expected object to have own property ${JSON.stringify(key)}`,
        this.actual,
        undefined,
        "toHaveProperty"
      );
      return;
    }
    if (arguments.length >= 2) {
      const propVal = (this.actual as Record<string, unknown>)[key];
      const match = valuesEqual(toValue(propVal), toValue(expectedValue));
      this.assertLazy(
        match,
        () =>
          `expected property ${JSON.stringify(key)} to equal ${encodeValue(toValue(expectedValue))}, got ${encodeValue(toValue(propVal))}`,
        propVal,
        expectedValue,
        "toHaveProperty"
      );
      return;
    }
    this.assertLazy(
      true,
      () => `object has property ${JSON.stringify(key)}`,
      this.actual,
      undefined,
      "toHaveProperty"
    );
  }

  toThrow(
    expected?:
      | string
      | RegExp
      | Error
      | (new (...args: unknown[]) => Error)
  ): void {
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
      this.assertLazy(
        false,
        () => "expected function to throw an error, but it returned normally",
        undefined,
        undefined,
        "toThrow"
      );
      return;
    }

    if (expected === undefined) {
      this.assertLazy(
        true,
        () => "function threw an error",
        undefined,
        undefined,
        "toThrow"
      );
      return;
    }

    const errMessage =
      caught instanceof Error ? caught.message : String(caught);

    if (typeof expected === "string") {
      const pass = errMessage.includes(expected);
      this.assertLazy(
        pass,
        () =>
          `expected error message to include ${JSON.stringify(expected)}, got ${JSON.stringify(errMessage)}`,
        errMessage,
        expected,
        "toThrow"
      );
    } else if (expected instanceof RegExp) {
      expected.lastIndex = 0;
      const pass = expected.test(errMessage);
      this.assertLazy(
        pass,
        () =>
          `expected error message to match ${String(expected)}, got ${JSON.stringify(errMessage)}`,
        errMessage,
        String(expected),
        "toThrow"
      );
    } else if (expected instanceof Error) {
      const pass =
        caught instanceof expected.constructor &&
        errMessage === expected.message;
      this.assertLazy(
        pass,
        () => `expected error matching ${String(expected)}, got ${String(caught)}`,
        caught,
        expected,
        "toThrow"
      );
    } else if (typeof expected === "function") {
      const pass = caught instanceof expected;
      this.assertLazy(
        pass,
        () =>
          `expected error to be instance of ${(expected as { name?: string }).name ?? "Error"}`,
        caught,
        undefined,
        "toThrow"
      );
    }
  }
}

export class AsyncExpectation<T = unknown> {
  constructor(
    private readonly promise: Promise<T>,
    private readonly isNot: boolean,
    private readonly isRejection: boolean
  ) {}

  get not(): AsyncExpectation<T> {
    return new AsyncExpectation(this.promise, !this.isNot, this.isRejection);
  }

  async toBe(expected: unknown): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toBe(expected);
  }

  async toEqual(expected: unknown): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toEqual(expected);
  }

  async toStrictEqual(expected: unknown): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toStrictEqual(expected);
  }

  async toBeTruthy(): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toBeTruthy();
  }

  async toBeFalsy(): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toBeFalsy();
  }

  async toBeNull(): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toBeNull();
  }

  async toBeUndefined(): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toBeUndefined();
  }

  async toBeDefined(): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toBeDefined();
  }

  async toBeInstanceOf(cls: new (...args: unknown[]) => unknown): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toBeInstanceOf(cls);
  }

  async toBeNaN(): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toBeNaN();
  }

  async toBeGreaterThan(n: number | bigint): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toBeGreaterThan(n);
  }

  async toBeGreaterThanOrEqual(n: number | bigint): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toBeGreaterThanOrEqual(n);
  }

  async toBeLessThan(n: number | bigint): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toBeLessThan(n);
  }

  async toBeLessThanOrEqual(n: number | bigint): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toBeLessThanOrEqual(n);
  }

  async toBeCloseTo(expected: number, precision = 2): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toBeCloseTo(expected, precision);
  }

  async toContain(item: unknown): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toContain(item);
  }

  async toHaveLength(expectedLength: number): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toHaveLength(expectedLength);
  }

  async toMatch(pattern: string | RegExp): Promise<void> {
    const val = await this.promise;
    new Expectation(val, this.isNot).toMatch(pattern);
  }

  async toHaveProperty(key: string, expectedValue?: unknown): Promise<void> {
    const val = await this.promise;
    if (arguments.length >= 2) {
      new Expectation(val, this.isNot).toHaveProperty(key, expectedValue);
    } else {
      new Expectation(val, this.isNot).toHaveProperty(key);
    }
  }

  async toThrow(
    expected?:
      | string
      | RegExp
      | Error
      | (new (...args: unknown[]) => Error)
  ): Promise<void> {
    if (!this.isRejection) {
      throw new AssertionError(
        "toThrow() must be called on rejects, not resolves"
      );
    }
    const val = await this.promise;
    new Expectation(() => {
      throw val;
    }, this.isNot).toThrow(expected);
  }
}

export function expect<T>(actual: T): Expectation<T> {
  return new Expectation(actual);
}
