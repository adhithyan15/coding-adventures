/**
 * Tests for the decade ring counter model.
 *
 * Verifies:
 * - Counter creation with correct initial digit
 * - Exactly one tube conducting at any time
 * - Pulse advancing increments the digit
 * - 9→0 wraparound generates carry
 * - Step trace records intermediate positions
 * - Edge cases (0 pulses, 10 pulses = full cycle)
 */

import { describe, it, expect } from "vitest";
import { createDecadeCounter, pulseDecadeCounter } from "../src/index.js";

describe("createDecadeCounter", () => {
  it("defaults to digit 0", () => {
    const counter = createDecadeCounter();
    expect(counter.currentDigit).toBe(0);
  });

  it("creates counter at specified digit", () => {
    const counter = createDecadeCounter(7);
    expect(counter.currentDigit).toBe(7);
  });

  it("has exactly 10 tubes", () => {
    const counter = createDecadeCounter();
    expect(counter.tubes.length).toBe(10);
  });

  it("exactly one tube is conducting", () => {
    for (let d = 0; d <= 9; d++) {
      const counter = createDecadeCounter(d);
      const conducting = counter.tubes.filter((t) => t.conducting);
      expect(conducting.length).toBe(1);
      expect(conducting[0].position).toBe(d);
    }
  });

  it("throws for invalid digit", () => {
    expect(() => createDecadeCounter(-1)).toThrow();
    expect(() => createDecadeCounter(10)).toThrow();
    expect(() => createDecadeCounter(3.5)).toThrow();
  });
});

describe("pulseDecadeCounter", () => {
  it("one pulse advances digit by 1", () => {
    const counter = createDecadeCounter(3);
    const result = pulseDecadeCounter(counter, 1);
    expect(result.counter.currentDigit).toBe(4);
    expect(result.carry).toBe(false);
  });

  it("multiple pulses advance digit correctly", () => {
    const counter = createDecadeCounter(2);
    const result = pulseDecadeCounter(counter, 5);
    expect(result.counter.currentDigit).toBe(7);
    expect(result.carry).toBe(false);
  });

  it("wrapping from 9→0 generates carry", () => {
    const counter = createDecadeCounter(9);
    const result = pulseDecadeCounter(counter, 1);
    expect(result.counter.currentDigit).toBe(0);
    expect(result.carry).toBe(true);
  });

  it("wrapping mid-sequence generates carry", () => {
    // 7 + 5 pulses: 7→8→9→0→1→2, carry at 9→0
    const counter = createDecadeCounter(7);
    const result = pulseDecadeCounter(counter, 5);
    expect(result.counter.currentDigit).toBe(2);
    expect(result.carry).toBe(true);
  });

  it("0 pulses does nothing", () => {
    const counter = createDecadeCounter(5);
    const result = pulseDecadeCounter(counter, 0);
    expect(result.counter.currentDigit).toBe(5);
    expect(result.carry).toBe(false);
    expect(result.stepsTraced.length).toBe(0);
  });

  it("10 pulses = full cycle, returns to same digit with carry", () => {
    const counter = createDecadeCounter(3);
    const result = pulseDecadeCounter(counter, 10);
    expect(result.counter.currentDigit).toBe(3);
    expect(result.carry).toBe(true);
  });

  it("stepsTraced records each intermediate position", () => {
    const counter = createDecadeCounter(7);
    const result = pulseDecadeCounter(counter, 5);
    expect(result.stepsTraced).toEqual([8, 9, 0, 1, 2]);
  });

  it("stepsTraced for single pulse", () => {
    const counter = createDecadeCounter(4);
    const result = pulseDecadeCounter(counter, 1);
    expect(result.stepsTraced).toEqual([5]);
  });

  it("default pulses is 1", () => {
    const counter = createDecadeCounter(0);
    const result = pulseDecadeCounter(counter);
    expect(result.counter.currentDigit).toBe(1);
  });

  it("throws for negative pulses", () => {
    const counter = createDecadeCounter(0);
    expect(() => pulseDecadeCounter(counter, -1)).toThrow();
  });

  it("exactly one tube conducting after pulsing", () => {
    const counter = createDecadeCounter(5);
    const result = pulseDecadeCounter(counter, 7);
    const conducting = result.counter.tubes.filter((t) => t.conducting);
    expect(conducting.length).toBe(1);
  });
});
