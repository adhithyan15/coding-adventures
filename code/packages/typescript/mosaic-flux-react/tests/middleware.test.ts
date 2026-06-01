// middleware.test.ts

import { describe, it, expect, vi } from "vitest";
import {
  composeMiddleware,
  loggerMiddleware,
  type Middleware,
} from "../src/middleware.js";
import type { MosaicAction } from "../src/action.js";

interface S {
  v: number;
}

class Bump implements MosaicAction<S> {
  apply(s: S): S {
    return { v: s.v + 1 };
  }
}

describe("composeMiddleware", () => {
  it("returns a no-op when the input array is empty", () => {
    const composed = composeMiddleware<S>([]);
    // Should not throw
    expect(() => composed(new Bump(), { v: 0 }, { v: 1 })).not.toThrow();
  });

  it("returns the single middleware verbatim when only one is provided", () => {
    const m: Middleware<S> = () => {};
    const composed = composeMiddleware<S>([m]);
    expect(composed).toBe(m);
  });

  it("runs middleware in registration order", () => {
    const calls: string[] = [];
    const composed = composeMiddleware<S>([
      () => calls.push("first"),
      () => calls.push("second"),
      () => calls.push("third"),
    ]);
    composed(new Bump(), { v: 0 }, { v: 1 });
    expect(calls).toEqual(["first", "second", "third"]);
  });

  it("isolates throwing middleware so subsequent middleware still run", () => {
    const calls: string[] = [];
    const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    const composed = composeMiddleware<S>([
      () => calls.push("first"),
      () => {
        throw new Error("boom");
      },
      () => calls.push("third"),
    ]);
    composed(new Bump(), { v: 0 }, { v: 1 });
    expect(calls).toEqual(["first", "third"]);
    expect(consoleErrorSpy).toHaveBeenCalled();
    consoleErrorSpy.mockRestore();
  });
});

describe("loggerMiddleware", () => {
  it("logs action class name + changed keys", () => {
    const groupSpy = vi
      .spyOn(console, "groupCollapsed")
      .mockImplementation(() => {});
    const groupEndSpy = vi.spyOn(console, "groupEnd").mockImplementation(() => {});
    const logSpy = vi.spyOn(console, "log").mockImplementation(() => {});

    const m = loggerMiddleware<S>();
    m(new Bump(), { v: 0 }, { v: 1 });

    expect(groupSpy).toHaveBeenCalledTimes(1);
    expect(groupSpy.mock.calls[0]?.[0]).toContain("Bump");
    expect(groupSpy.mock.calls[0]?.[0]).toContain("v"); // the changed key
    expect(logSpy).toHaveBeenCalled();
    expect(groupEndSpy).toHaveBeenCalled();

    groupSpy.mockRestore();
    groupEndSpy.mockRestore();
    logSpy.mockRestore();
  });

  it("reports '(none)' for no-op dispatches", () => {
    const groupSpy = vi
      .spyOn(console, "groupCollapsed")
      .mockImplementation(() => {});
    vi.spyOn(console, "groupEnd").mockImplementation(() => {});
    vi.spyOn(console, "log").mockImplementation(() => {});

    const m = loggerMiddleware<S>();
    m(new Bump(), { v: 0 }, { v: 0 });

    expect(groupSpy.mock.calls[0]?.[0]).toContain("(none)");
    vi.restoreAllMocks();
  });

  it("treats non-object state as atomic", () => {
    const groupSpy = vi
      .spyOn(console, "groupCollapsed")
      .mockImplementation(() => {});
    vi.spyOn(console, "groupEnd").mockImplementation(() => {});
    vi.spyOn(console, "log").mockImplementation(() => {});

    const m = loggerMiddleware<number>();
    class NumberAction implements MosaicAction<number> {
      apply(n: number): number {
        return n + 1;
      }
    }
    m(new NumberAction(), 0, 1);

    expect(groupSpy.mock.calls[0]?.[0]).toContain("<root>");
    vi.restoreAllMocks();
  });
});
