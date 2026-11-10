import nodeTest from "node:test";

export const describe = nodeTest.describe;
export const it = nodeTest.it;
export const test = nodeTest.test;
export const beforeEach = nodeTest.beforeEach;
export const afterEach = nodeTest.afterEach;
export const before = nodeTest.before;
export const after = nodeTest.after;
export const beforeAll = nodeTest.before;
export const afterAll = nodeTest.after;

export interface MockFn<Args extends unknown[], Return> {
  (...args: Args): Return;
  mock: {
    calls: Args[];
    results: { type: "return" | "throw"; value: unknown }[];
    clear(): void;
    reset(): void;
  };
}

/**
 * Lightweight in-house spy/stub implementation replacing vi.fn() from Vitest.
 */
export function fn<Args extends unknown[], Return>(
  impl?: (...args: Args) => Return
): MockFn<Args, Return> {
  const calls: Args[] = [];
  const results: { type: "return" | "throw"; value: unknown }[] = [];

  const mock = function (...args: Args): Return {
    calls.push(args);
    try {
      const res = impl ? impl(...args) : (undefined as unknown as Return);
      results.push({ type: "return", value: res });
      return res;
    } catch (err) {
      results.push({ type: "throw", value: err });
      throw err;
    }
  } as MockFn<Args, Return>;

  mock.mock = {
    calls,
    results,
    clear() {
      calls.length = 0;
      results.length = 0;
    },
    reset() {
      calls.length = 0;
      results.length = 0;
    },
  };

  return mock;
}

export const vi = {
  fn,
};
