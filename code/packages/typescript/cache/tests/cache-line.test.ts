/**
 * Tests for CacheLine — the smallest unit of data in a cache.
 *
 * Verifies the lifecycle of a cache line: creation (invalid), filling
 * (valid + data), touching (LRU update), modification (dirty), and
 * invalidation.
 */

import { describe, it, expect } from "vitest";
import { CacheLine } from "../src/cache-line.js";

// ── Initial state of a newly created cache line ───────────────────────

describe("CacheLine creation", () => {
  it("should be invalid by default (empty box)", () => {
    const line = new CacheLine();
    expect(line.valid).toBe(false);
    expect(line.dirty).toBe(false);
    expect(line.tag).toBe(0);
    expect(line.lastAccess).toBe(0);
  });

  it("should default to 64 bytes (standard on modern CPUs)", () => {
    const line = new CacheLine();
    expect(line.data.length).toBe(64);
    expect(line.lineSize).toBe(64);
  });

  it("should support custom line sizes (e.g., 32 bytes)", () => {
    const line = new CacheLine(32);
    expect(line.data.length).toBe(32);
    expect(line.lineSize).toBe(32);
  });

  it("should initialize all bytes to zero", () => {
    const line = new CacheLine(8);
    expect(line.data).toEqual([0, 0, 0, 0, 0, 0, 0, 0]);
  });
});

// ── Filling a cache line with data from memory ───────────────────────

describe("CacheLine fill", () => {
  it("should make the line valid with the correct tag", () => {
    const line = new CacheLine(8);
    line.fill(42, [1, 2, 3, 4, 5, 6, 7, 8], 100);
    expect(line.valid).toBe(true);
    expect(line.tag).toBe(42);
    expect(line.lastAccess).toBe(100);
  });

  it("should store the provided data bytes", () => {
    const line = new CacheLine(4);
    line.fill(7, [0xaa, 0xbb, 0xcc, 0xdd], 0);
    expect(line.data).toEqual([0xaa, 0xbb, 0xcc, 0xdd]);
  });

  it("should clear the dirty bit (freshly loaded data is clean)", () => {
    const line = new CacheLine(4);
    line.dirty = true; // simulate a prior dirty state
    line.fill(1, [0, 0, 0, 0], 0);
    expect(line.dirty).toBe(false);
  });

  it("should make a defensive copy of the data", () => {
    const line = new CacheLine(4);
    const original = [1, 2, 3, 4];
    line.fill(1, original, 0);
    original[0] = 99; // mutate the original
    expect(line.data[0]).toBe(1); // line's data should be unchanged
  });
});

// ── LRU tracking via touch() ──────────────────────────────────────────

describe("CacheLine touch", () => {
  it("should update the last access timestamp", () => {
    const line = new CacheLine();
    line.fill(1, new Array(64).fill(0), 10);
    expect(line.lastAccess).toBe(10);
    line.touch(50);
    expect(line.lastAccess).toBe(50);
  });
});

// ── Invalidation (cache flush / coherence) ────────────────────────────

describe("CacheLine invalidate", () => {
  it("should clear valid and dirty flags", () => {
    const line = new CacheLine(4);
    line.fill(5, [1, 2, 3, 4], 10);
    line.dirty = true;
    line.invalidate();
    expect(line.valid).toBe(false);
    expect(line.dirty).toBe(false);
  });

  it("should not zero out the data (just marks invalid)", () => {
    const line = new CacheLine(4);
    line.fill(5, [0xaa, 0xbb, 0xcc, 0xdd], 0);
    line.invalidate();
    // Data still physically present (like a file in a recycle bin)
    expect(line.data).toEqual([0xaa, 0xbb, 0xcc, 0xdd]);
  });
});

// ── String representation for debugging ───────────────────────────────

describe("CacheLine toString", () => {
  it("should show '--' for an invalid line", () => {
    const line = new CacheLine();
    const r = line.toString();
    expect(r).toContain("--");
  });

  it("should show 'V-' for a valid clean line", () => {
    const line = new CacheLine(4);
    line.fill(0xff, [0, 0, 0, 0], 0);
    const r = line.toString();
    expect(r).toContain("V-");
    expect(r.toLowerCase()).toContain("0xff");
  });

  it("should show 'VD' for a valid dirty line", () => {
    const line = new CacheLine(4);
    line.fill(1, [0, 0, 0, 0], 0);
    line.dirty = true;
    const r = line.toString();
    expect(r).toContain("VD");
  });
});
