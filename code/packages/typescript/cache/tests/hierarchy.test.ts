/**
 * Tests for CacheHierarchy — multi-level cache system.
 *
 * Tests cover:
 * - L1 hit (fastest path)
 * - L1 miss -> L2 hit
 * - L1+L2 miss -> L3 hit
 * - All miss -> main memory
 * - Inclusive fill policy (data fills back up through all levels)
 * - Harvard architecture (separate L1I)
 * - Write through hierarchy
 * - No-cache configuration (straight to memory)
 * - Hierarchy utilities (invalidateAll, resetStats)
 */

import { describe, it, expect } from "vitest";
import { Cache } from "../src/cache.js";
import { CacheConfig } from "../src/cache-set.js";
import { CacheHierarchy } from "../src/hierarchy.js";

// ── Helper Factories ────────────────────────────────────────────────────

/** Create a small L1D cache (256B, 2-way, 1-cycle latency). */
function makeL1d(size = 256): Cache {
  return new Cache(new CacheConfig("L1D", size, 64, 2, 1));
}

/** Create a small L2 cache (1KB, 4-way, 10-cycle latency). */
function makeL2(size = 1024): Cache {
  return new Cache(new CacheConfig("L2", size, 64, 4, 10));
}

/** Create a small L3 cache (4KB, 8-way, 30-cycle latency). */
function makeL3(size = 4096): Cache {
  return new Cache(new CacheConfig("L3", size, 64, 8, 30));
}

// ── Read Through Hierarchy ──────────────────────────────────────────────

describe("Hierarchy read", () => {
  it("should go to memory on a cold cache (first read)", () => {
    const h = new CacheHierarchy({
      l1d: makeL1d(),
      l2: makeL2(),
      mainMemoryLatency: 100,
    });
    const result = h.read(0x1000, false, 0);
    expect(result.servedBy).toBe("memory");
    // Total: L1 latency (1) + L2 latency (10) + memory (100)
    expect(result.totalCycles).toBe(1 + 10 + 100);
  });

  it("should hit L1 on the second read", () => {
    const h = new CacheHierarchy({
      l1d: makeL1d(),
      l2: makeL2(),
      mainMemoryLatency: 100,
    });
    h.read(0x1000, false, 0); // miss -> fills L1
    const result = h.read(0x1000, false, 1);
    expect(result.servedBy).toBe("L1D");
    expect(result.totalCycles).toBe(1); // just L1 latency
  });

  it("should serve from L2 when L1 misses but L2 has it", () => {
    const l1d = makeL1d();
    const l2 = makeL2();
    const h = new CacheHierarchy({ l1d, l2, mainMemoryLatency: 100 });

    // Prime L2 by filling it directly
    l2.fillLine(0x1000, new Array(64).fill(0), 0);

    // Now read — L1 will miss, L2 should hit
    const result = h.read(0x1000, false, 1);
    expect(result.servedBy).toBe("L2");
    expect(result.totalCycles).toBe(1 + 10); // L1 miss + L2 hit
  });

  it("should serve from L3 when L1 and L2 miss", () => {
    const l1d = makeL1d();
    const l2 = makeL2();
    const l3 = makeL3();
    const h = new CacheHierarchy({ l1d, l2, l3, mainMemoryLatency: 100 });

    // Prime L3 directly
    l3.fillLine(0x2000, new Array(64).fill(0), 0);

    const result = h.read(0x2000, false, 1);
    expect(result.servedBy).toBe("L3");
    expect(result.totalCycles).toBe(1 + 10 + 30); // L1 + L2 + L3
  });

  it("should go to memory when all levels miss", () => {
    const l1d = makeL1d();
    const l2 = makeL2();
    const l3 = makeL3();
    const h = new CacheHierarchy({ l1d, l2, l3, mainMemoryLatency: 100 });

    const result = h.read(0x3000, false, 0);
    expect(result.servedBy).toBe("memory");
    expect(result.totalCycles).toBe(1 + 10 + 30 + 100);
  });

  it("should fill L1 inclusively after an L2 hit", () => {
    const l1d = makeL1d();
    const l2 = makeL2();
    const h = new CacheHierarchy({ l1d, l2, mainMemoryLatency: 100 });

    l2.fillLine(0x1000, new Array(64).fill(0), 0);
    h.read(0x1000, false, 1); // L1 miss, L2 hit -> fills L1

    const result = h.read(0x1000, false, 2);
    expect(result.servedBy).toBe("L1D");
  });

  it("should fill all levels inclusively after memory serves", () => {
    const l1d = makeL1d();
    const l2 = makeL2();
    const h = new CacheHierarchy({ l1d, l2, mainMemoryLatency: 100 });

    h.read(0x5000, false, 0); // all miss -> memory
    // Now L1 should have it
    const result = h.read(0x5000, false, 1);
    expect(result.servedBy).toBe("L1D");
  });
});

// ── Instruction Cache (Harvard Architecture) ────────────────────────────

describe("Harvard architecture", () => {
  it("should use L1I for instruction reads", () => {
    const l1i = new Cache(new CacheConfig("L1I", 256, 64, 2, 1));
    const l1d = makeL1d();
    const l2 = makeL2();
    const h = new CacheHierarchy({ l1i, l1d, l2, mainMemoryLatency: 100 });

    // Prime L1I directly
    l1i.fillLine(0x1000, new Array(64).fill(0), 0);

    const result = h.read(0x1000, true, 1);
    expect(result.servedBy).toBe("L1I");
    expect(result.totalCycles).toBe(1);
  });

  it("should NOT use L1I for data reads", () => {
    const l1i = new Cache(new CacheConfig("L1I", 256, 64, 2, 1));
    const l1d = makeL1d();
    const h = new CacheHierarchy({ l1i, l1d, mainMemoryLatency: 100 });

    l1i.fillLine(0x1000, new Array(64).fill(0), 0);

    const result = h.read(0x1000, false, 1);
    // L1D doesn't have it — goes to memory
    expect(result.servedBy).toBe("memory");
  });
});

// ── Write Through Hierarchy ─────────────────────────────────────────────

describe("Hierarchy write", () => {
  it("should hit at L1 when the data is already there", () => {
    const l1d = makeL1d();
    const h = new CacheHierarchy({ l1d, mainMemoryLatency: 100 });

    h.read(0x1000, false, 0); // fill L1
    const result = h.write(0x1000, [0xab], 1);
    expect(result.servedBy).toBe("L1D");
    expect(result.totalCycles).toBe(1);
  });

  it("should walk down to lower levels on a write miss", () => {
    const l1d = makeL1d();
    const l2 = makeL2();
    const h = new CacheHierarchy({ l1d, l2, mainMemoryLatency: 100 });

    const result = h.write(0x2000, [0xff], 0);
    // L1 misses, L2 misses -> memory
    expect(result.servedBy).toBe("memory");
  });

  it("should serve from L2 on a write miss when L2 has the data", () => {
    const l1d = makeL1d();
    const l2 = makeL2();
    const h = new CacheHierarchy({ l1d, l2, mainMemoryLatency: 100 });

    l2.fillLine(0x1000, new Array(64).fill(0), 0);
    const result = h.write(0x1000, [0xab], 1);
    expect(result.servedBy).toBe("L2");
  });
});

// ── No-Cache Configuration ──────────────────────────────────────────────

describe("No-cache hierarchy", () => {
  it("should go straight to memory on read", () => {
    const h = new CacheHierarchy({ mainMemoryLatency: 200 });
    const result = h.read(0x1000, false, 0);
    expect(result.servedBy).toBe("memory");
    expect(result.totalCycles).toBe(200);
  });

  it("should go straight to memory on write", () => {
    const h = new CacheHierarchy({ mainMemoryLatency: 200 });
    const result = h.write(0x1000, [0xab], 0);
    expect(result.servedBy).toBe("memory");
    expect(result.totalCycles).toBe(200);
  });
});

// ── Utilities ───────────────────────────────────────────────────────────

describe("Hierarchy utilities", () => {
  it("should cause all reads to miss after invalidateAll()", () => {
    const l1d = makeL1d();
    const l2 = makeL2();
    const h = new CacheHierarchy({ l1d, l2, mainMemoryLatency: 100 });

    h.read(0x1000, false, 0);
    h.read(0x1000, false, 1); // L1 hit
    h.invalidateAll();
    const result = h.read(0x1000, false, 2);
    expect(result.servedBy).toBe("memory"); // cold miss after flush
  });

  it("should zero all stats after resetStats()", () => {
    const l1d = makeL1d();
    const l2 = makeL2();
    const h = new CacheHierarchy({ l1d, l2, mainMemoryLatency: 100 });

    h.read(0x1000, false, 0);
    h.resetStats();
    expect(l1d.stats.totalAccesses).toBe(0);
    expect(l2.stats.totalAccesses).toBe(0);
  });

  it("should summarize configuration in toString()", () => {
    const h = new CacheHierarchy({
      l1d: makeL1d(),
      l2: makeL2(),
      l3: makeL3(),
      mainMemoryLatency: 100,
    });
    const r = h.toString();
    expect(r).toContain("L1D");
    expect(r).toContain("L2");
    expect(r).toContain("L3");
    expect(r).toContain("mem=100cyc");
  });

  it("should track hitAtLevel correctly", () => {
    const h = new CacheHierarchy({
      l1d: makeL1d(),
      l2: makeL2(),
      mainMemoryLatency: 100,
    });
    // First read -> memory (level index = 2, beyond all caches)
    const result = h.read(0x1000, false, 0);
    expect(result.hitAtLevel).toBe(2); // past L1D (0) and L2 (1)
    // Second read -> L1D (level index = 0)
    const result2 = h.read(0x1000, false, 1);
    expect(result2.hitAtLevel).toBe(0);
  });
});
