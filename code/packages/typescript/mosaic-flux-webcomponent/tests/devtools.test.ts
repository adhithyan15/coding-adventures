// devtools.test.ts
//
// In jsdom env, window IS defined, so the PostMessageSink path
// runs. We mock postMessage and verify the payload format.

import { describe, it, expect, vi, beforeEach } from "vitest";
import { devToolsMiddleware } from "../src/devtools.js";
import type { MosaicAction } from "../src/action.js";

interface S {
  v: number;
}

class Bump implements MosaicAction<S> {
  apply(s: S): S {
    return { v: s.v + 1 };
  }
}

class Payloaded implements MosaicAction<S> {
  constructor(
    public readonly amount: number,
    public readonly tag: string,
  ) {}
  apply(s: S): S {
    return { v: s.v + this.amount };
  }
}

describe("devToolsMiddleware (jsdom postMessage path)", () => {
  let postMessageSpy: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    postMessageSpy = vi.fn();
    Object.defineProperty(window, "postMessage", {
      configurable: true,
      writable: true,
      value: postMessageSpy,
    });
  });

  it("returns a callable middleware", () => {
    const m = devToolsMiddleware<S>();
    expect(typeof m).toBe("function");
  });

  it("publishes action events via postMessage", () => {
    const m = devToolsMiddleware<S>();
    m(new Bump(), { v: 0 }, { v: 1 });
    expect(postMessageSpy).toHaveBeenCalledTimes(1);
    const [arg] = postMessageSpy.mock.calls[0] ?? [];
    expect(arg).toMatchObject({
      source: "mosaic-flux-devtools",
      payload: { kind: "action", actionType: "Bump" },
    });
  });

  it("extracts payload from action instance", () => {
    const m = devToolsMiddleware<S>();
    m(new Payloaded(42, "t"), { v: 0 }, { v: 42 });
    const [arg] = postMessageSpy.mock.calls[0] ?? [];
    const payload = (arg as { payload: { actionPayload: Record<string, unknown> } })
      .payload.actionPayload;
    expect(payload).toEqual({ amount: 42, tag: "t" });
  });

  it("survives postMessage failure", () => {
    postMessageSpy.mockImplementation(() => {
      throw new Error("clone");
    });
    const m = devToolsMiddleware<S>();
    expect(() => m(new Bump(), { v: 0 }, { v: 1 })).not.toThrow();
  });

  it("respects custom storeName", () => {
    const m = devToolsMiddleware<S>({ storeName: "my-store" });
    m(new Bump(), { v: 0 }, { v: 1 });
    const [arg] = postMessageSpy.mock.calls[0] ?? [];
    expect((arg as { payload: { storeName: string } }).payload.storeName).toBe(
      "my-store",
    );
  });
});
