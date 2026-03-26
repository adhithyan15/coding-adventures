/**
 * Tests for sleep -- delay for a specified amount of time.
 *
 * We test the exported `parseDuration` function, which converts a
 * duration string (like "30", "2m", "1.5h") into seconds. This is
 * the pure logic of the sleep utility -- the actual sleeping is just
 * a setTimeout wrapper and doesn't need unit testing.
 */

import { describe, it, expect } from "vitest";
import { parseDuration } from "../src/sleep.js";

describe("parseDuration", () => {
  // -------------------------------------------------------------------------
  // No suffix (defaults to seconds)
  // -------------------------------------------------------------------------

  it("should parse plain integers as seconds", () => {
    expect(parseDuration("30")).toBe(30);
    expect(parseDuration("1")).toBe(1);
    expect(parseDuration("0")).toBe(0);
    expect(parseDuration("100")).toBe(100);
  });

  it("should parse floating-point numbers as seconds", () => {
    expect(parseDuration("0.5")).toBe(0.5);
    expect(parseDuration("1.5")).toBe(1.5);
    expect(parseDuration("0.001")).toBe(0.001);
  });

  // -------------------------------------------------------------------------
  // Seconds suffix (s)
  // -------------------------------------------------------------------------

  it("should parse 's' suffix as seconds", () => {
    expect(parseDuration("30s")).toBe(30);
    expect(parseDuration("1s")).toBe(1);
    expect(parseDuration("0.5s")).toBe(0.5);
  });

  it("should handle uppercase S suffix", () => {
    expect(parseDuration("30S")).toBe(30);
  });

  // -------------------------------------------------------------------------
  // Minutes suffix (m)
  // -------------------------------------------------------------------------

  it("should parse 'm' suffix as minutes (multiply by 60)", () => {
    expect(parseDuration("1m")).toBe(60);
    expect(parseDuration("2m")).toBe(120);
    expect(parseDuration("0.5m")).toBe(30);
    expect(parseDuration("1.5m")).toBe(90);
  });

  // -------------------------------------------------------------------------
  // Hours suffix (h)
  // -------------------------------------------------------------------------

  it("should parse 'h' suffix as hours (multiply by 3600)", () => {
    expect(parseDuration("1h")).toBe(3600);
    expect(parseDuration("2h")).toBe(7200);
    expect(parseDuration("0.5h")).toBe(1800);
    expect(parseDuration("1.5h")).toBe(5400);
  });

  // -------------------------------------------------------------------------
  // Days suffix (d)
  // -------------------------------------------------------------------------

  it("should parse 'd' suffix as days (multiply by 86400)", () => {
    expect(parseDuration("1d")).toBe(86400);
    expect(parseDuration("2d")).toBe(172800);
    expect(parseDuration("0.5d")).toBe(43200);
  });

  // -------------------------------------------------------------------------
  // Zero values
  // -------------------------------------------------------------------------

  it("should handle zero with any suffix", () => {
    expect(parseDuration("0")).toBe(0);
    expect(parseDuration("0s")).toBe(0);
    expect(parseDuration("0m")).toBe(0);
    expect(parseDuration("0h")).toBe(0);
    expect(parseDuration("0d")).toBe(0);
  });

  // -------------------------------------------------------------------------
  // Error cases
  // -------------------------------------------------------------------------

  it("should throw on empty string", () => {
    expect(() => parseDuration("")).toThrow("invalid time interval");
  });

  it("should throw on non-numeric input", () => {
    expect(() => parseDuration("abc")).toThrow("invalid time interval");
    expect(() => parseDuration("xyz")).toThrow("invalid time interval");
  });

  it("should throw on suffix-only input", () => {
    expect(() => parseDuration("s")).toThrow("invalid time interval");
    expect(() => parseDuration("m")).toThrow("invalid time interval");
    expect(() => parseDuration("h")).toThrow("invalid time interval");
    expect(() => parseDuration("d")).toThrow("invalid time interval");
  });

  it("should throw on negative values", () => {
    expect(() => parseDuration("-1")).toThrow("invalid time interval");
    expect(() => parseDuration("-5s")).toThrow("invalid time interval");
  });

  // -------------------------------------------------------------------------
  // Edge cases
  // -------------------------------------------------------------------------

  it("should handle very small values", () => {
    expect(parseDuration("0.001")).toBe(0.001);
    expect(parseDuration("0.001s")).toBe(0.001);
  });

  it("should handle very large values", () => {
    expect(parseDuration("999999")).toBe(999999);
    expect(parseDuration("365d")).toBe(365 * 86400);
  });

  it("should include the original string in error messages", () => {
    try {
      parseDuration("bad");
      expect.fail("should have thrown");
    } catch (err: unknown) {
      expect((err as Error).message).toContain("bad");
    }
  });
});
