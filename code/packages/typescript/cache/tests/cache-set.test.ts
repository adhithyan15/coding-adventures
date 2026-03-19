/**
 * Tests for CacheSet and CacheConfig — set-associative lookup with LRU.
 *
 * Tests cover:
 * - CacheConfig validation (powers of 2, valid policies)
 * - CacheSet lookup (hit/miss), access (LRU update), allocation, eviction
 * - LRU replacement ordering
 * - Direct-mapped (1-way) behavior
 */

import { describe, it, expect } from "vitest";
import { CacheConfig, CacheSet } from "../src/cache-set.js";

// ── CacheConfig Validation ──────────────────────────────────────────────

describe("CacheConfig validation", () => {
  it("should accept a typical L1 config", () => {
    const config = new CacheConfig("L1D", 65536, 64, 4, 1);
    expect(config.numSets).toBe(256);
    expect(config.numLines).toBe(1024);
  });

  it("should reject non-positive total size", () => {
    expect(() => new CacheConfig("bad", 0)).toThrow(
      "total_size must be positive",
    );
  });

  it("should reject line size that is not a power of 2", () => {
    expect(() => new CacheConfig("bad", 256, 48)).toThrow(
      "line_size must be a positive power of 2",
    );
  });

  it("should reject non-positive associativity", () => {
    expect(() => new CacheConfig("bad", 256, 64, 0)).toThrow(
      "associativity must be positive",
    );
  });

  it("should reject total_size not divisible by line_size * associativity", () => {
    expect(() => new CacheConfig("bad", 100, 64, 4)).toThrow(
      "must be divisible",
    );
  });

  it("should reject invalid write policy", () => {
    expect(
      () => new CacheConfig("bad", 256, 64, 1, 1, "write-around" as any),
    ).toThrow("write_policy must be");
  });

  it("should reject negative latency", () => {
    expect(() => new CacheConfig("bad", 256, 64, 1, -1)).toThrow(
      "access_latency must be non-negative",
    );
  });

  it("should accept write-through as a valid policy", () => {
    const config = new CacheConfig("L1D", 256, 64, 1, 1, "write-through");
    expect(config.writePolicy).toBe("write-through");
  });

  it("should be immutable after creation (readonly fields)", () => {
    const config = new CacheConfig("L1D", 256, 64, 1);
    // TypeScript readonly prevents assignment at compile time.
    // At runtime we verify the properties exist and have correct values.
    expect(config.totalSize).toBe(256);
    expect(config.name).toBe("L1D");
  });
});

// ── CacheSet Lookup ─────────────────────────────────────────────────────

describe("CacheSet lookup", () => {
  it("should miss on an empty set (all lines invalid)", () => {
    const cs = new CacheSet(4, 64);
    const [hit, way] = cs.lookup(42);
    expect(hit).toBe(false);
    expect(way).toBeNull();
  });

  it("should hit after filling a line", () => {
    const cs = new CacheSet(4, 8);
    cs.lines[0].fill(42, new Array(8).fill(0), 0);
    const [hit, way] = cs.lookup(42);
    expect(hit).toBe(true);
    expect(way).toBe(0);
  });

  it("should miss with a different tag", () => {
    const cs = new CacheSet(4, 8);
    cs.lines[0].fill(42, new Array(8).fill(0), 0);
    const [hit, way] = cs.lookup(99);
    expect(hit).toBe(false);
    expect(way).toBeNull();
  });

  it("should find the correct way among multiple valid lines", () => {
    const cs = new CacheSet(4, 8);
    cs.lines[0].fill(10, new Array(8).fill(0), 0);
    cs.lines[1].fill(20, new Array(8).fill(0), 0);
    cs.lines[2].fill(30, new Array(8).fill(0), 0);
    const [hit, way] = cs.lookup(20);
    expect(hit).toBe(true);
    expect(way).toBe(1);
  });
});

// ── CacheSet Access ─────────────────────────────────────────────────────

describe("CacheSet access", () => {
  it("should update LRU timestamp on a hit", () => {
    const cs = new CacheSet(2, 8);
    cs.lines[0].fill(10, new Array(8).fill(0), 5);
    const [hit, line] = cs.access(10, 100);
    expect(hit).toBe(true);
    expect(line.lastAccess).toBe(100);
  });

  it("should return the LRU victim on a miss with all ways full", () => {
    const cs = new CacheSet(2, 8);
    cs.lines[0].fill(10, new Array(8).fill(0), 1);
    cs.lines[1].fill(20, new Array(8).fill(0), 5);
    const [hit, victim] = cs.access(99, 10);
    expect(hit).toBe(false);
    // Victim should be lines[0] (accessed at cycle 1, older than cycle 5)
    expect(victim.tag).toBe(10);
  });
});

// ── CacheSet Allocate ───────────────────────────────────────────────────

describe("CacheSet allocate", () => {
  it("should fill an empty slot without eviction", () => {
    const cs = new CacheSet(4, 8);
    const evicted = cs.allocate(42, new Array(8).fill(0xaa), 10);
    expect(evicted).toBeNull(); // no eviction needed
    const [hit] = cs.lookup(42);
    expect(hit).toBe(true);
  });

  it("should evict the LRU line when all ways are full", () => {
    const cs = new CacheSet(2, 8);
    // Fill both ways
    cs.allocate(10, new Array(8).fill(0), 1);
    cs.allocate(20, new Array(8).fill(0), 2);
    // Now allocate a third — should evict tag=10 (cycle=1 is older)
    const evicted = cs.allocate(30, new Array(8).fill(0), 3);
    // tag=10 was not dirty, so evicted should be null
    expect(evicted).toBeNull();
    // tag=10 should be gone, tag=30 should be present
    const [hit10] = cs.lookup(10);
    const [hit30] = cs.lookup(30);
    expect(hit10).toBe(false);
    expect(hit30).toBe(true);
  });

  it("should return the dirty evicted line for writeback", () => {
    const cs = new CacheSet(2, 8);
    cs.allocate(10, new Array(8).fill(0xaa), 1);
    cs.lines[0].dirty = true; // mark first line as dirty
    cs.allocate(20, new Array(8).fill(0), 2);
    // Now allocate a third — should evict dirty tag=10
    const evicted = cs.allocate(30, new Array(8).fill(0), 3);
    expect(evicted).not.toBeNull();
    expect(evicted!.dirty).toBe(true);
    expect(evicted!.tag).toBe(10);
    expect(evicted!.data).toEqual(new Array(8).fill(0xaa));
  });

  it("should fill all empty slots before any eviction", () => {
    const cs = new CacheSet(4, 8);
    for (let i = 0; i < 4; i++) {
      const evicted = cs.allocate(i, new Array(8).fill(0), i);
      expect(evicted).toBeNull(); // still had empty slots
    }
    // 5th allocation must evict
    cs.allocate(99, new Array(8).fill(0), 10);
    // tag=0 (cycle=0) should have been evicted as LRU
    const [hit0] = cs.lookup(0);
    expect(hit0).toBe(false);
  });
});

// ── LRU Ordering ────────────────────────────────────────────────────────

describe("LRU replacement", () => {
  it("should prefer invalid lines over valid ones", () => {
    const cs = new CacheSet(4, 8);
    cs.lines[0].fill(1, new Array(8).fill(0), 100);
    // lines[1], [2], [3] are invalid — should pick one of them
    const lru = cs.findLru();
    expect([1, 2, 3]).toContain(lru);
  });

  it("should pick the line with the smallest last_access", () => {
    const cs = new CacheSet(4, 8);
    cs.lines[0].fill(1, new Array(8).fill(0), 10);
    cs.lines[1].fill(2, new Array(8).fill(0), 5);   // oldest
    cs.lines[2].fill(3, new Array(8).fill(0), 20);
    cs.lines[3].fill(4, new Array(8).fill(0), 15);
    const lru = cs.findLru();
    expect(lru).toBe(1); // cycle=5 is the oldest
  });

  it("should prevent eviction of a recently accessed line", () => {
    const cs = new CacheSet(2, 8);
    cs.lines[0].fill(10, new Array(8).fill(0), 1);
    cs.lines[1].fill(20, new Array(8).fill(0), 2);
    // Access tag=10 at a later cycle — it becomes most recent
    cs.access(10, 100);
    // Now tag=20 (cycle=2) is older than tag=10 (cycle=100)
    const lru = cs.findLru();
    expect(lru).toBe(1); // tag=20 is now LRU
  });
});

// ── Direct-Mapped (1-way) ───────────────────────────────────────────────

describe("Direct-mapped set (1-way)", () => {
  it("should cause a conflict miss when two addresses map to the same set", () => {
    const cs = new CacheSet(1, 8);
    cs.allocate(10, new Array(8).fill(0), 1);
    // Allocating a different tag to the same set evicts the first
    cs.allocate(20, new Array(8).fill(0), 2);
    const [hit10] = cs.lookup(10);
    const [hit20] = cs.lookup(20);
    expect(hit10).toBe(false);
    expect(hit20).toBe(true);
  });
});
