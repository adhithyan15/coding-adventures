/**
 * forme-stage — cancellation tests
 */

import { describe, it, expect } from "vitest";
import { CancellationError } from "@coding-adventures/forme-errors";
import {
  createCancellationTokenSource,
  neverCancelledToken,
} from "../src/index.js";

describe("createCancellationTokenSource", () => {
  it("starts with cancelled = false and reason = null", () => {
    const { token } = createCancellationTokenSource();
    expect(token.cancelled).toBe(false);
    expect(token.reason).toBeNull();
  });

  it("cancel() trips the token", () => {
    const { token, cancel } = createCancellationTokenSource();
    cancel("user pressed Ctrl-C");
    expect(token.cancelled).toBe(true);
    expect(token.reason).toBe("user pressed Ctrl-C");
  });

  it("cancel() without reason sets reason to null", () => {
    const { token, cancel } = createCancellationTokenSource();
    cancel();
    expect(token.cancelled).toBe(true);
    expect(token.reason).toBeNull();
  });

  it("cancel() is idempotent — second call is silent", () => {
    const { token, cancel } = createCancellationTokenSource();
    cancel("first");
    cancel("second");
    expect(token.reason).toBe("first");
  });

  it("throwIfCancelled is a no-op while not cancelled", () => {
    const { token } = createCancellationTokenSource();
    expect(() => token.throwIfCancelled()).not.toThrow();
  });

  it("throwIfCancelled throws CancellationError after cancel", () => {
    const { token, cancel } = createCancellationTokenSource();
    cancel("deadline");
    expect(() => token.throwIfCancelled()).toThrow(CancellationError);
    try {
      token.throwIfCancelled();
    } catch (e) {
      expect((e as CancellationError).reason).toBe("deadline");
    }
  });

  it("throwIfCancelled with no reason still throws CancellationError", () => {
    const { token, cancel } = createCancellationTokenSource();
    cancel();
    let caught: CancellationError | null = null;
    try { token.throwIfCancelled(); } catch (e) { caught = e as CancellationError; }
    expect(caught).toBeInstanceOf(CancellationError);
    expect(caught?.reason).toBeNull();
  });
});

describe("onCancel callbacks", () => {
  it("fires callbacks once on cancel", () => {
    const { token, cancel } = createCancellationTokenSource();
    const calls: string[] = [];
    token.onCancel(() => calls.push("a"));
    token.onCancel(() => calls.push("b"));
    cancel();
    expect(calls).toEqual(["a", "b"]);
  });

  it("does not fire callbacks twice on a second cancel", () => {
    const { token, cancel } = createCancellationTokenSource();
    let count = 0;
    token.onCancel(() => count++);
    cancel(); cancel();
    expect(count).toBe(1);
  });

  it("fires immediately when registered after cancellation", () => {
    const { token, cancel } = createCancellationTokenSource();
    cancel("late");
    let called = false;
    token.onCancel(() => { called = true; });
    expect(called).toBe(true);
  });

  it("swallows errors from callbacks (other callbacks still fire)", () => {
    const { token, cancel } = createCancellationTokenSource();
    let bRan = false;
    token.onCancel(() => { throw new Error("a failed"); });
    token.onCancel(() => { bRan = true; });
    expect(() => cancel()).not.toThrow();
    expect(bRan).toBe(true);
  });

  it("swallows errors from immediate (already-cancelled) callbacks", () => {
    const { token, cancel } = createCancellationTokenSource();
    cancel();
    expect(() => {
      token.onCancel(() => { throw new Error("nope"); });
    }).not.toThrow();
  });
});

describe("AbortSignal interop", () => {
  it("token.signal aborts when cancel fires", () => {
    const { token, cancel } = createCancellationTokenSource();
    expect(token.signal.aborted).toBe(false);
    cancel();
    expect(token.signal.aborted).toBe(true);
  });

  it("signal can be passed to fetch-style APIs", () => {
    const { token } = createCancellationTokenSource();
    // Just check the type and that it's an AbortSignal — no fetch needed.
    expect(token.signal).toBeInstanceOf(AbortSignal);
  });
});

describe("neverCancelledToken", () => {
  it("is permanently uncancelled", () => {
    const t = neverCancelledToken();
    expect(t.cancelled).toBe(false);
    expect(t.reason).toBeNull();
    expect(() => t.throwIfCancelled()).not.toThrow();
  });

  it("onCancel is a silent no-op", () => {
    const t = neverCancelledToken();
    expect(() => t.onCancel(() => { throw new Error("never fires"); })).not.toThrow();
  });

  it("returns a stable AbortSignal", () => {
    expect(neverCancelledToken().signal).toBeInstanceOf(AbortSignal);
  });
});
