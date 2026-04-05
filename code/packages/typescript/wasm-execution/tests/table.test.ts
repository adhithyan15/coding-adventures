/**
 * table.test.ts --- Tests for WASM function reference table.
 */

import { describe, it, expect } from "vitest";
import { Table } from "../src/table.js";
import { TrapError } from "../src/host_interface.js";

// ===========================================================================
// Construction
// ===========================================================================

describe("Table construction", () => {
  it("should create a table with the given size", () => {
    const t = new Table(5);
    expect(t.size()).toBe(5);
  });

  it("should initialize all entries to null", () => {
    const t = new Table(3);
    expect(t.get(0)).toBeNull();
    expect(t.get(1)).toBeNull();
    expect(t.get(2)).toBeNull();
  });

  it("should create a zero-sized table", () => {
    const t = new Table(0);
    expect(t.size()).toBe(0);
  });
});

// ===========================================================================
// get / set
// ===========================================================================

describe("get and set", () => {
  it("should store and retrieve a function index", () => {
    const t = new Table(5);
    t.set(2, 42);
    expect(t.get(2)).toBe(42);
  });

  it("should allow setting to null (clearing an entry)", () => {
    const t = new Table(5);
    t.set(0, 10);
    expect(t.get(0)).toBe(10);
    t.set(0, null);
    expect(t.get(0)).toBeNull();
  });

  it("should allow multiple entries to be set independently", () => {
    const t = new Table(3);
    t.set(0, 100);
    t.set(1, 200);
    t.set(2, 300);
    expect(t.get(0)).toBe(100);
    expect(t.get(1)).toBe(200);
    expect(t.get(2)).toBe(300);
  });

  it("should throw TrapError on get with out-of-bounds index", () => {
    const t = new Table(3);
    expect(() => t.get(3)).toThrow(TrapError);
    expect(() => t.get(-1)).toThrow(TrapError);
    expect(() => t.get(100)).toThrow(TrapError);
  });

  it("should throw TrapError on set with out-of-bounds index", () => {
    const t = new Table(3);
    expect(() => t.set(3, 42)).toThrow(TrapError);
    expect(() => t.set(-1, 42)).toThrow(TrapError);
  });

  it("should throw TrapError on get from zero-size table", () => {
    const t = new Table(0);
    expect(() => t.get(0)).toThrow(TrapError);
  });
});

// ===========================================================================
// grow
// ===========================================================================

describe("grow", () => {
  it("should return old size on success", () => {
    const t = new Table(3);
    const oldSize = t.grow(2);
    expect(oldSize).toBe(3);
    expect(t.size()).toBe(5);
  });

  it("should initialize new entries to null", () => {
    const t = new Table(1);
    t.set(0, 42);
    t.grow(2);
    expect(t.get(0)).toBe(42);
    expect(t.get(1)).toBeNull();
    expect(t.get(2)).toBeNull();
  });

  it("should return -1 when exceeding max size", () => {
    const t = new Table(3, 5);
    expect(t.grow(2)).toBe(3); // OK: 3 + 2 = 5 = max
    expect(t.grow(1)).toBe(-1); // Fail: 5 + 1 = 6 > max
    expect(t.size()).toBe(5); // unchanged
  });

  it("should handle grow(0) as a no-op", () => {
    const t = new Table(3);
    expect(t.grow(0)).toBe(3);
    expect(t.size()).toBe(3);
  });

  it("should allow growth when no max is set", () => {
    const t = new Table(1);
    expect(t.grow(10)).toBe(1);
    expect(t.size()).toBe(11);
  });
});
