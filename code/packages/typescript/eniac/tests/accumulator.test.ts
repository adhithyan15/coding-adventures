/**
 * Tests for the ENIAC accumulator model.
 *
 * Verifies:
 * - Accumulator creation and value reading
 * - Single-digit addition (no carry)
 * - Multi-digit addition with carry propagation
 * - Overflow detection
 * - Per-digit trace data accuracy
 * - Edge cases
 */

import { describe, it, expect } from "vitest";
import {
  createAccumulator,
  accumulatorValue,
  accumulatorAdd,
} from "../src/index.js";

describe("createAccumulator", () => {
  it("defaults to 0 with 4 digits", () => {
    const acc = createAccumulator();
    expect(accumulatorValue(acc)).toBe(0);
    expect(acc.digitCount).toBe(4);
  });

  it("creates accumulator with specified value", () => {
    const acc = createAccumulator(42);
    expect(accumulatorValue(acc)).toBe(42);
  });

  it("creates accumulator with specified digit count", () => {
    const acc = createAccumulator(0, 6);
    expect(acc.digitCount).toBe(6);
    expect(acc.decades.length).toBe(6);
  });

  it("decomposes value into correct per-digit ring counters", () => {
    const acc = createAccumulator(1234, 4);
    expect(acc.decades[0].currentDigit).toBe(4); // ones
    expect(acc.decades[1].currentDigit).toBe(3); // tens
    expect(acc.decades[2].currentDigit).toBe(2); // hundreds
    expect(acc.decades[3].currentDigit).toBe(1); // thousands
  });

  it("throws for negative value", () => {
    expect(() => createAccumulator(-1)).toThrow();
  });

  it("throws for value exceeding digit count", () => {
    expect(() => createAccumulator(10000, 4)).toThrow();
  });

  it("allows max value for digit count", () => {
    const acc = createAccumulator(9999, 4);
    expect(accumulatorValue(acc)).toBe(9999);
  });
});

describe("accumulatorAdd", () => {
  it("simple addition without carry: 42 + 35 = 77", () => {
    const acc = createAccumulator(42, 4);
    const trace = accumulatorAdd(acc, 35);
    expect(accumulatorValue(trace.accumulator)).toBe(77);
    expect(trace.overflow).toBe(false);
  });

  it("addition with carry in ones: 42 + 75 = 117", () => {
    const acc = createAccumulator(42, 4);
    const trace = accumulatorAdd(acc, 75);
    expect(accumulatorValue(trace.accumulator)).toBe(117);
    // Tens digit wraps: 4+7=11, carry out
    expect(trace.carries[1]).toBe(true);
    // Ones digit does not carry: 2+5=7
    expect(trace.carries[0]).toBe(false);
  });

  it("addition with cascading carries: 999 + 1 = 1000", () => {
    const acc = createAccumulator(999, 4);
    const trace = accumulatorAdd(acc, 1);
    expect(accumulatorValue(trace.accumulator)).toBe(1000);
    // All lower 3 digits carry
    expect(trace.carries[0]).toBe(true);
    expect(trace.carries[1]).toBe(true);
    expect(trace.carries[2]).toBe(true);
    expect(trace.carries[3]).toBe(false);
  });

  it("adding 0 doesn't change the accumulator", () => {
    const acc = createAccumulator(42, 4);
    const trace = accumulatorAdd(acc, 0);
    expect(accumulatorValue(trace.accumulator)).toBe(42);
    expect(trace.carries.every((c) => !c)).toBe(true);
  });

  it("overflow when result exceeds digit count", () => {
    const acc = createAccumulator(9999, 4);
    const trace = accumulatorAdd(acc, 1);
    // Wraps to 0000 with overflow
    expect(accumulatorValue(trace.accumulator)).toBe(0);
    expect(trace.overflow).toBe(true);
  });

  it("per-digit trace records correct pulse counts", () => {
    const acc = createAccumulator(42, 4);
    const trace = accumulatorAdd(acc, 75);

    // Ones: 2 + 5 = 7 (5 pulses, no carry in)
    expect(trace.digitTraces[0].digitBefore).toBe(2);
    expect(trace.digitTraces[0].pulsesReceived).toBe(5);
    expect(trace.digitTraces[0].digitAfter).toBe(7);
    expect(trace.digitTraces[0].carryOut).toBe(false);

    // Tens: 4 + 7 = 11 → digit=1, carry (7 pulses, no carry in)
    expect(trace.digitTraces[1].digitBefore).toBe(4);
    expect(trace.digitTraces[1].pulsesReceived).toBe(7);
    expect(trace.digitTraces[1].digitAfter).toBe(1);
    expect(trace.digitTraces[1].carryOut).toBe(true);

    // Hundreds: 0 + 0 + carry = 1 (1 pulse from carry)
    expect(trace.digitTraces[2].digitBefore).toBe(0);
    expect(trace.digitTraces[2].pulsesReceived).toBe(1);
    expect(trace.digitTraces[2].digitAfter).toBe(1);
    expect(trace.digitTraces[2].carryOut).toBe(false);
  });

  it("per-digit trace includes step-by-step positions", () => {
    const acc = createAccumulator(7, 4);
    const trace = accumulatorAdd(acc, 5);

    // Ones: 7→8→9→0→1→2
    expect(trace.digitTraces[0].pulseResult.stepsTraced).toEqual([8, 9, 0, 1, 2]);
  });

  it("records the addend", () => {
    const acc = createAccumulator(0, 4);
    const trace = accumulatorAdd(acc, 123);
    expect(trace.addend).toBe(123);
  });

  it("100 + 200 = 300 (only hundreds digit changes)", () => {
    const acc = createAccumulator(100, 4);
    const trace = accumulatorAdd(acc, 200);
    expect(accumulatorValue(trace.accumulator)).toBe(300);
    expect(trace.digitTraces[0].pulsesReceived).toBe(0);
    expect(trace.digitTraces[1].pulsesReceived).toBe(0);
    expect(trace.digitTraces[2].pulsesReceived).toBe(2);
  });

  it("throws for negative addend", () => {
    const acc = createAccumulator(0, 4);
    expect(() => accumulatorAdd(acc, -1)).toThrow();
  });
});
