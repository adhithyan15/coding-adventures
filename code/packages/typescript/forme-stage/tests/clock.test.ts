/**
 * forme-stage — clock tests
 */

import { describe, it, expect, vi } from "vitest";
import { frozenClock, systemClock } from "../src/index.js";

describe("systemClock", () => {
  it("nowMs is close to Date.now", async () => {
    const c = systemClock();
    const before = Date.now();
    const v = await c.nowMs();
    const after = Date.now();
    expect(v).toBeGreaterThanOrEqual(before);
    expect(v).toBeLessThanOrEqual(after);
  });

  it("nowIso parses to the same instant as nowMs", async () => {
    const c = systemClock();
    const iso = await c.nowIso();
    const ms = await c.nowMs();
    // Parsing back should round-trip within a few ms (calls aren't exactly simultaneous).
    const parsed = new Date(iso).getTime();
    expect(Math.abs(parsed - ms)).toBeLessThan(1000);
  });

  it("monotonicMs is non-decreasing across rapid calls", () => {
    const c = systemClock();
    let last = c.monotonicMs();
    for (let i = 0; i < 100; i++) {
      const next = c.monotonicMs();
      expect(next).toBeGreaterThanOrEqual(last);
      last = next;
    }
  });

  it("falls back gracefully when performance.now is missing", () => {
    const original = (globalThis as { performance?: unknown }).performance;
    // Stub performance with no `now` method.
    vi.stubGlobal("performance", {});
    const c = systemClock();
    const v = c.monotonicMs();
    expect(typeof v).toBe("number");
    expect(v).toBeGreaterThan(0);
    // Restore so other tests aren't affected.
    vi.unstubAllGlobals();
    expect((globalThis as { performance?: unknown }).performance).toBe(original);
  });
});

describe("frozenClock", () => {
  const FIXED = Date.UTC(2026, 4, 15, 12, 0, 0);

  it("nowMs always returns the fixed timestamp", async () => {
    const c = frozenClock({ timestamp: FIXED });
    await expect(c.nowMs()).resolves.toBe(FIXED);
    await expect(c.nowMs()).resolves.toBe(FIXED);
    await expect(c.nowMs()).resolves.toBe(FIXED);
  });

  it("nowIso reflects the fixed timestamp", async () => {
    const c = frozenClock({ timestamp: FIXED });
    await expect(c.nowIso()).resolves.toBe("2026-05-15T12:00:00.000Z");
  });

  it("nowIso is cached (same string instance across calls)", async () => {
    const c = frozenClock({ timestamp: FIXED });
    expect(await c.nowIso()).toBe(await c.nowIso());
  });

  it("monotonicMs starts at 0 and is static when tick is 0", () => {
    const c = frozenClock({ timestamp: FIXED });
    expect(c.monotonicMs()).toBe(0);
    expect(c.monotonicMs()).toBe(0);
  });

  it("monotonicMs honours monotonicStart", () => {
    const c = frozenClock({ timestamp: FIXED, monotonicStart: 100 });
    expect(c.monotonicMs()).toBe(100);
  });

  it("monotonicMs advances by tick on each call", () => {
    const c = frozenClock({ timestamp: FIXED, monotonicStart: 0, monotonicTickMs: 10 });
    expect(c.monotonicMs()).toBe(0);
    expect(c.monotonicMs()).toBe(10);
    expect(c.monotonicMs()).toBe(20);
  });

  it("two clocks with the same timestamp produce byte-equal output", async () => {
    const a = frozenClock({ timestamp: FIXED });
    const b = frozenClock({ timestamp: FIXED });
    expect(await a.nowMs()).toBe(await b.nowMs());
    expect(await a.nowIso()).toBe(await b.nowIso());
  });
});
