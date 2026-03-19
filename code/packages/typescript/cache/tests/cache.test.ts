/**
 * Tests for Cache — a single configurable cache level.
 *
 * Tests cover:
 * - Address decomposition (tag, setIndex, offset)
 * - Read hits and misses
 * - Write hits and misses (write-back and write-through)
 * - Dirty eviction on write
 * - Cache invalidation (flush)
 * - Direct-mapped and set-associative configurations
 * - Edge cases: 1-set cache, address 0
 */

import { describe, it, expect } from "vitest";
import { Cache } from "../src/cache.js";
import { CacheConfig } from "../src/cache-set.js";

// ── Address Decomposition ───────────────────────────────────────────────

describe("Address decomposition", () => {
  /**
   * For a 1024-byte cache with 64-byte lines and 4-way associativity:
   * - numLines = 1024 / 64 = 16
   * - numSets = 16 / 4 = 4
   * - offsetBits = log2(64) = 6
   * - setBits = log2(4) = 2
   * - tag = address >> 8
   *
   * Address layout:
   * |  tag (24+ bits)  | set (2 bits) | offset (6 bits) |
   */
  function makeCache(): Cache {
    return new Cache(new CacheConfig("test", 1024, 64, 4));
  }

  it("should decompose address 0 to tag=0, set=0, offset=0", () => {
    const cache = makeCache();
    const [tag, setIdx, offset] = cache.decomposeAddress(0);
    expect(tag).toBe(0);
    expect(setIdx).toBe(0);
    expect(offset).toBe(0);
  });

  it("should extract the low 6 bits as offset", () => {
    const cache = makeCache();
    // Address 0x1F = 31 — should be offset 31 within the first line
    const [, setIdx, offset] = cache.decomposeAddress(0x1f);
    expect(offset).toBe(31);
    expect(setIdx).toBe(0);
  });

  it("should extract bits 6-7 as the set index (for 4 sets)", () => {
    const cache = makeCache();
    // Address 0x40 = 64 -> offset=0, setIndex=1 (bit 6 is set)
    const [, setIdx] = cache.decomposeAddress(0x40);
    expect(setIdx).toBe(1);
    // Address 0x80 = 128 -> setIndex=2 (bit 7 is set)
    const [, setIdx2] = cache.decomposeAddress(0x80);
    expect(setIdx2).toBe(2);
    // Address 0xC0 = 192 -> setIndex=3
    const [, setIdx3] = cache.decomposeAddress(0xc0);
    expect(setIdx3).toBe(3);
  });

  it("should extract bits above set+offset as the tag", () => {
    const cache = makeCache();
    // Address 0x100 = 256 -> offset=0, set=0, tag=1
    const [tag, setIdx, offset] = cache.decomposeAddress(0x100);
    expect(offset).toBe(0);
    expect(setIdx).toBe(0);
    expect(tag).toBe(1);
  });

  it("should correctly decompose a known address (0x1A2B3C4D)", () => {
    const cache = makeCache();
    const [tag, setIdx, offset] = cache.decomposeAddress(0x1a2b3c4d);
    // low 6 bits of 0x4D = 0b01001101 -> 0b001101 = 13
    expect(offset).toBe(0x0d);
    expect(setIdx).toBe((0x1a2b3c4d >>> 6) & 0x3);
    expect(tag).toBe(0x1a2b3c4d >>> 8);
  });
});

// ── Read Operations ─────────────────────────────────────────────────────

describe("Cache read", () => {
  /** Small 256-byte, 2-way cache with 64-byte lines (2 sets). */
  function makeCache(): Cache {
    return new Cache(new CacheConfig("test", 256, 64, 2, 3));
  }

  it("should miss on the first read (compulsory miss)", () => {
    const cache = makeCache();
    const access = cache.read(0x100, 1, 0);
    expect(access.hit).toBe(false);
    expect(access.cycles).toBe(3);
  });

  it("should hit on the second read to the same address", () => {
    const cache = makeCache();
    cache.read(0x100, 1, 0); // miss — fills the line
    const access = cache.read(0x100, 1, 1); // should hit
    expect(access.hit).toBe(true);
    expect(access.cycles).toBe(3);
  });

  it("should hit for different addresses within the same cache line", () => {
    const cache = makeCache();
    cache.read(0x100, 1, 0); // miss — fills line for block starting at 0x100
    // 0x110 = 0x100 + 16, same 64-byte block
    const access = cache.read(0x110, 1, 1);
    expect(access.hit).toBe(true); // same line!
  });

  it("should track read misses in statistics", () => {
    const cache = makeCache();
    cache.read(0x100, 1, 0);
    expect(cache.stats.reads).toBe(1);
    expect(cache.stats.misses).toBe(1);
    expect(cache.stats.hits).toBe(0);
  });

  it("should track read hits in statistics", () => {
    const cache = makeCache();
    cache.read(0x100, 1, 0);
    cache.read(0x100, 1, 1);
    expect(cache.stats.reads).toBe(2);
    expect(cache.stats.hits).toBe(1);
    expect(cache.stats.misses).toBe(1);
  });

  it("should return correct address decomposition in the access record", () => {
    const cache = makeCache();
    const access = cache.read(0x100, 1, 0);
    expect(access.address).toBe(0x100);
    expect(typeof access.tag).toBe("number");
    expect(typeof access.offset).toBe("number");
  });
});

// ── Write Operations ────────────────────────────────────────────────────

describe("Cache write", () => {
  /** Small write-back cache. */
  function makeWbCache(): Cache {
    return new Cache(
      new CacheConfig("test", 256, 64, 2, 1, "write-back"),
    );
  }

  /** Small write-through cache. */
  function makeWtCache(): Cache {
    return new Cache(
      new CacheConfig("test", 256, 64, 2, 1, "write-through"),
    );
  }

  it("should allocate a line on a write miss (write-allocate)", () => {
    const cache = makeWbCache();
    const access = cache.write(0x100, [0xab], 0);
    expect(access.hit).toBe(false);
    // Now reading should hit
    const readAccess = cache.read(0x100, 1, 1);
    expect(readAccess.hit).toBe(true);
  });

  it("should mark the line dirty on a write hit in write-back mode", () => {
    const cache = makeWbCache();
    cache.read(0x100, 1, 0); // bring line in
    cache.write(0x100, [0xab], 1); // write hit
    // Check the line is dirty
    const [tag, setIdx] = cache.decomposeAddress(0x100);
    const [hit, way] = cache.sets[setIdx].lookup(tag);
    expect(hit).toBe(true);
    expect(way).not.toBeNull();
    expect(cache.sets[setIdx].lines[way!].dirty).toBe(true);
  });

  it("should NOT mark the line dirty in write-through mode", () => {
    const cache = makeWtCache();
    cache.read(0x100, 1, 0);
    cache.write(0x100, [0xab], 1);
    const [tag, setIdx] = cache.decomposeAddress(0x100);
    const [hit, way] = cache.sets[setIdx].lookup(tag);
    expect(hit).toBe(true);
    expect(way).not.toBeNull();
    expect(cache.sets[setIdx].lines[way!].dirty).toBe(false);
  });

  it("should store written data in the cache line", () => {
    const cache = makeWbCache();
    // Write to address 0x100, offset 0
    cache.write(0x100, [0xde, 0xad], 0);
    const [tag, setIdx, offset] = cache.decomposeAddress(0x100);
    const [, way] = cache.sets[setIdx].lookup(tag);
    expect(way).not.toBeNull();
    const line = cache.sets[setIdx].lines[way!];
    expect(line.data[offset]).toBe(0xde);
    expect(line.data[offset + 1]).toBe(0xad);
  });

  it("should track write operations in stats", () => {
    const cache = makeWbCache();
    cache.write(0x100, null, 0); // miss
    cache.write(0x100, null, 1); // hit
    expect(cache.stats.writes).toBe(2);
    expect(cache.stats.misses).toBe(1);
    expect(cache.stats.hits).toBe(1);
  });
});

// ── Dirty Eviction ──────────────────────────────────────────────────────

describe("Dirty eviction", () => {
  it("should return the evicted dirty line in the access record", () => {
    // 1-way, 1 set (fully conflicts everything)
    const cache = new Cache(
      new CacheConfig("test", 64, 64, 1, 1, "write-back"),
    );
    // Write to address 0 — miss, allocate, mark dirty
    cache.write(0, [0xff], 0);
    // Write to address 64 — different tag, same set -> evict address 0
    const access = cache.read(64, 1, 1);
    expect(access.hit).toBe(false);
    expect(access.evicted).not.toBeNull();
    expect(access.evicted!.dirty).toBe(true);
  });

  it("should track evictions and writebacks in stats", () => {
    const cache = new Cache(
      new CacheConfig("test", 64, 64, 1, 1, "write-back"),
    );
    cache.write(0, [0xff], 0);
    cache.read(64, 1, 1); // evicts dirty line
    expect(cache.stats.evictions).toBeGreaterThanOrEqual(1);
    expect(cache.stats.writebacks).toBeGreaterThanOrEqual(1);
  });
});

// ── Cache Invalidation ──────────────────────────────────────────────────

describe("Cache invalidation", () => {
  it("should cause all subsequent reads to miss", () => {
    const cache = new Cache(new CacheConfig("test", 256, 64, 2, 1));
    cache.read(0x100, 1, 0);
    cache.read(0x100, 1, 1); // should hit
    expect(cache.stats.hits).toBe(1);

    cache.invalidate();
    const access = cache.read(0x100, 1, 2);
    expect(access.hit).toBe(false); // cold miss after flush
  });
});

// ── Edge Cases ──────────────────────────────────────────────────────────

describe("Cache edge cases", () => {
  it("should work with a single-set cache (fully associative for its size)", () => {
    const cache = new Cache(new CacheConfig("tiny", 128, 64, 2, 1));
    // Both addresses map to set 0 (only set)
    cache.read(0, 1, 0);
    cache.read(64, 1, 1);
    // Both should be cached (2-way, 1 set)
    expect(cache.read(0, 1, 2).hit).toBe(true);
    expect(cache.read(64, 1, 3).hit).toBe(true);
  });

  it("should thrash with direct-mapped conflict eviction", () => {
    /**
     * Classic pathological case: alternating between two addresses
     * that map to the same set results in 100% miss rate.
     */
    const cache = new Cache(new CacheConfig("dm", 256, 64, 1, 1));
    // Two addresses that map to the same set (same set index, different tag)
    // With 4 sets (256/64/1), setBits=2, addresses 0x000 and 0x100 both map to set 0
    const addrA = 0x000;
    const addrB = 0x100; // different tag, same set
    cache.read(addrA, 1, 0); // miss, fill
    cache.read(addrB, 1, 1); // miss, evict a, fill b
    cache.read(addrA, 1, 2); // miss, evict b, fill a
    cache.read(addrB, 1, 3); // miss, evict a, fill b
    // All misses after the initial cold miss
    expect(cache.stats.hits).toBe(0);
    expect(cache.stats.misses).toBe(4);
  });

  it("should support fillLine() to install data without stats", () => {
    const cache = new Cache(new CacheConfig("test", 256, 64, 2, 1));
    cache.fillLine(0x100, new Array(64).fill(0xab), 0);
    // Should hit on a subsequent read
    const access = cache.read(0x100, 1, 1);
    expect(access.hit).toBe(true);
  });

  it("should show configuration in toString()", () => {
    const cache = new Cache(new CacheConfig("L1D", 65536, 64, 4, 1));
    const r = cache.toString();
    expect(r).toContain("L1D");
    expect(r).toContain("64KB");
    expect(r).toContain("4-way");
  });
});
