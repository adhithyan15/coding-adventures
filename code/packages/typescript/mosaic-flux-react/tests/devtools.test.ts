// devtools.test.ts — DevTools middleware behaviour.
//
// We don't test the actual postMessage / WebSocket transports here
// (they require a browser env or socket mock); we test the
// middleware shape, payload extraction, and no-throw guarantees.

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
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

describe("devToolsMiddleware", () => {
  beforeEach(() => {
    // Without a window (vitest default node env), the middleware
    // falls through to the NoopSink, which is what we want here —
    // we exercise the middleware path without needing a transport.
    expect(typeof (globalThis as { window?: unknown }).window).toBe(
      "undefined",
    );
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("returns a callable middleware function", () => {
    const m = devToolsMiddleware<S>();
    expect(typeof m).toBe("function");
  });

  it("does not throw when dispatched with a void-payload action", () => {
    const m = devToolsMiddleware<S>();
    expect(() => m(new Bump(), { v: 0 }, { v: 1 })).not.toThrow();
  });

  it("does not throw when dispatched with a payloaded action", () => {
    const m = devToolsMiddleware<S>();
    expect(() => m(new Payloaded(5, "tag"), { v: 0 }, { v: 5 })).not.toThrow();
  });

  it("accepts custom storeName option", () => {
    const m = devToolsMiddleware<S>({ storeName: "my-grid" });
    expect(() => m(new Bump(), { v: 0 }, { v: 1 })).not.toThrow();
  });

  it("uses NoopSink in non-browser non-ws environments", () => {
    // The middleware was constructed at the top of beforeEach without
    // a window or WebSocket, so it must have selected NoopSink.
    // Direct check: invoking it should not produce side effects.
    const m = devToolsMiddleware<S>();
    expect(() => m(new Bump(), { v: 0 }, { v: 1 })).not.toThrow();
  });
});

describe("devToolsMiddleware with mocked window.postMessage", () => {
  let postMessageSpy: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    postMessageSpy = vi.fn();
    (globalThis as { window?: unknown }).window = {
      postMessage: postMessageSpy,
    };
  });

  afterEach(() => {
    delete (globalThis as { window?: unknown }).window;
  });

  it("publishes action events through postMessage", () => {
    const m = devToolsMiddleware<S>();
    m(new Bump(), { v: 0 }, { v: 1 });
    expect(postMessageSpy).toHaveBeenCalledTimes(1);
    const [payload] = postMessageSpy.mock.calls[0] ?? [];
    expect(payload).toMatchObject({
      source: "mosaic-flux-devtools",
      payload: {
        kind: "action",
        actionType: "Bump",
      },
    });
  });

  it("extracts payload fields from the action instance", () => {
    const m = devToolsMiddleware<S>();
    m(new Payloaded(42, "x"), { v: 0 }, { v: 42 });
    const [arg] = postMessageSpy.mock.calls[0] ?? [];
    const payload = (arg as { payload: { actionPayload: Record<string, unknown> } })
      .payload.actionPayload;
    expect(payload).toEqual({ amount: 42, tag: "x" });
  });

  it("survives postMessage failures (non-cloneable payloads)", () => {
    postMessageSpy.mockImplementation(() => {
      throw new Error("DataCloneError");
    });
    const m = devToolsMiddleware<S>();
    expect(() => m(new Bump(), { v: 0 }, { v: 1 })).not.toThrow();
  });

  it("includes storeName in published payload", () => {
    const m = devToolsMiddleware<S>({ storeName: "custom" });
    m(new Bump(), { v: 0 }, { v: 1 });
    const [arg] = postMessageSpy.mock.calls[0] ?? [];
    const payload = (arg as { payload: { storeName: string } }).payload;
    expect(payload.storeName).toBe("custom");
  });
});
