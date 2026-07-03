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
  it("no-op when empty", () => {
    const c = composeMiddleware<S>([]);
    expect(() => c(new Bump(), { v: 0 }, { v: 1 })).not.toThrow();
  });

  it("returns the single middleware verbatim", () => {
    const m: Middleware<S> = () => {};
    expect(composeMiddleware<S>([m])).toBe(m);
  });

  it("runs in order", () => {
    const calls: string[] = [];
    const c = composeMiddleware<S>([
      () => calls.push("a"),
      () => calls.push("b"),
      () => calls.push("c"),
    ]);
    c(new Bump(), { v: 0 }, { v: 1 });
    expect(calls).toEqual(["a", "b", "c"]);
  });

  it("isolates throws", () => {
    const calls: string[] = [];
    const spy = vi.spyOn(console, "error").mockImplementation(() => {});
    const c = composeMiddleware<S>([
      () => calls.push("a"),
      () => {
        throw new Error("boom");
      },
      () => calls.push("c"),
    ]);
    c(new Bump(), { v: 0 }, { v: 1 });
    expect(calls).toEqual(["a", "c"]);
    expect(spy).toHaveBeenCalled();
    spy.mockRestore();
  });
});

describe("loggerMiddleware", () => {
  it("logs action name + changed keys", () => {
    const g = vi.spyOn(console, "groupCollapsed").mockImplementation(() => {});
    vi.spyOn(console, "groupEnd").mockImplementation(() => {});
    vi.spyOn(console, "log").mockImplementation(() => {});
    loggerMiddleware<S>()(new Bump(), { v: 0 }, { v: 1 });
    expect(g.mock.calls[0]?.[0]).toContain("Bump");
    expect(g.mock.calls[0]?.[0]).toContain("v");
    vi.restoreAllMocks();
  });

  it("'(none)' for no-op", () => {
    const g = vi.spyOn(console, "groupCollapsed").mockImplementation(() => {});
    vi.spyOn(console, "groupEnd").mockImplementation(() => {});
    vi.spyOn(console, "log").mockImplementation(() => {});
    loggerMiddleware<S>()(new Bump(), { v: 0 }, { v: 0 });
    expect(g.mock.calls[0]?.[0]).toContain("(none)");
    vi.restoreAllMocks();
  });

  it("atomic for non-object state", () => {
    const g = vi.spyOn(console, "groupCollapsed").mockImplementation(() => {});
    vi.spyOn(console, "groupEnd").mockImplementation(() => {});
    vi.spyOn(console, "log").mockImplementation(() => {});
    class NA implements MosaicAction<number> {
      apply(n: number): number {
        return n + 1;
      }
    }
    loggerMiddleware<number>()(new NA(), 0, 1);
    expect(g.mock.calls[0]?.[0]).toContain("<root>");
    vi.restoreAllMocks();
  });
});
