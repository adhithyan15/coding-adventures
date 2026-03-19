/**
 * Tests for CacheStats — verifying hit rate calculation and counter tracking.
 *
 * These tests ensure the scorecard is accurate. If the stats are wrong,
 * every performance analysis built on top will be misleading.
 */

import { describe, it, expect } from "vitest";
import { CacheStats } from "../src/stats.js";

// ── Basic counter operations and initial state ────────────────────────

describe("CacheStats basics", () => {
  it("should have all counters at zero initially", () => {
    const stats = new CacheStats();
    expect(stats.reads).toBe(0);
    expect(stats.writes).toBe(0);
    expect(stats.hits).toBe(0);
    expect(stats.misses).toBe(0);
    expect(stats.evictions).toBe(0);
    expect(stats.writebacks).toBe(0);
    expect(stats.totalAccesses).toBe(0);
  });

  it("should increment reads and hits on a read hit", () => {
    const stats = new CacheStats();
    stats.recordRead(true);
    expect(stats.reads).toBe(1);
    expect(stats.hits).toBe(1);
    expect(stats.misses).toBe(0);
  });

  it("should increment reads and misses on a read miss", () => {
    const stats = new CacheStats();
    stats.recordRead(false);
    expect(stats.reads).toBe(1);
    expect(stats.hits).toBe(0);
    expect(stats.misses).toBe(1);
  });

  it("should increment writes and hits on a write hit", () => {
    const stats = new CacheStats();
    stats.recordWrite(true);
    expect(stats.writes).toBe(1);
    expect(stats.hits).toBe(1);
  });

  it("should increment writes and misses on a write miss", () => {
    const stats = new CacheStats();
    stats.recordWrite(false);
    expect(stats.writes).toBe(1);
    expect(stats.misses).toBe(1);
  });

  it("should increment evictions but not writebacks on a clean eviction", () => {
    const stats = new CacheStats();
    stats.recordEviction(false);
    expect(stats.evictions).toBe(1);
    expect(stats.writebacks).toBe(0);
  });

  it("should increment both evictions and writebacks on a dirty eviction", () => {
    const stats = new CacheStats();
    stats.recordEviction(true);
    expect(stats.evictions).toBe(1);
    expect(stats.writebacks).toBe(1);
  });
});

// ── Hit rate and miss rate calculations ───────────────────────────────

describe("CacheStats rates", () => {
  it("should return 0.0 hit rate with no accesses", () => {
    const stats = new CacheStats();
    expect(stats.hitRate).toBe(0.0);
  });

  it("should return 0.0 miss rate with no accesses", () => {
    const stats = new CacheStats();
    expect(stats.missRate).toBe(0.0);
  });

  it("should return 1.0 hit rate when every access is a hit", () => {
    const stats = new CacheStats();
    for (let i = 0; i < 10; i++) {
      stats.recordRead(true);
    }
    expect(stats.hitRate).toBe(1.0);
    expect(stats.missRate).toBe(0.0);
  });

  it("should return 0.0 hit rate when every access is a miss", () => {
    const stats = new CacheStats();
    for (let i = 0; i < 10; i++) {
      stats.recordRead(false);
    }
    expect(stats.hitRate).toBe(0.0);
    expect(stats.missRate).toBe(1.0);
  });

  it("should return 0.5 hit rate with equal hits and misses", () => {
    const stats = new CacheStats();
    stats.recordRead(true);
    stats.recordRead(false);
    expect(stats.hitRate).toBe(0.5);
    expect(stats.missRate).toBe(0.5);
  });

  it("should include both reads and writes in hit rate", () => {
    const stats = new CacheStats();
    stats.recordRead(true);   // hit
    stats.recordWrite(true);  // hit
    stats.recordRead(false);  // miss
    stats.recordWrite(false); // miss
    expect(stats.totalAccesses).toBe(4);
    expect(stats.hitRate).toBe(0.5);
  });

  it("should have hit rate + miss rate equal to 1.0", () => {
    const stats = new CacheStats();
    stats.recordRead(true);
    stats.recordRead(true);
    stats.recordRead(false);
    expect(Math.abs(stats.hitRate + stats.missRate - 1.0)).toBeLessThan(1e-10);
  });
});

// ── Reset functionality ───────────────────────────────────────────────

describe("CacheStats reset", () => {
  it("should clear all counters on reset", () => {
    const stats = new CacheStats();
    stats.recordRead(true);
    stats.recordWrite(false);
    stats.recordEviction(true);
    stats.reset();
    expect(stats.reads).toBe(0);
    expect(stats.writes).toBe(0);
    expect(stats.hits).toBe(0);
    expect(stats.misses).toBe(0);
    expect(stats.evictions).toBe(0);
    expect(stats.writebacks).toBe(0);
    expect(stats.totalAccesses).toBe(0);
  });

  it("should work correctly after a reset", () => {
    const stats = new CacheStats();
    stats.recordRead(true);
    stats.reset();
    stats.recordRead(false);
    expect(stats.reads).toBe(1);
    expect(stats.misses).toBe(1);
    expect(stats.hitRate).toBe(0.0);
  });
});

// ── String representation ─────────────────────────────────────────────

describe("CacheStats toString", () => {
  it("should include key info in string representation", () => {
    const stats = new CacheStats();
    stats.recordRead(true);
    stats.recordRead(false);
    const r = stats.toString();
    expect(r).toContain("accesses=2");
    expect(r).toContain("hits=1");
    expect(r).toContain("misses=1");
    expect(r).toContain("50.0%");
  });
});
