/**
 * Tests for pipelined floating-point arithmetic.
 *
 * These tests verify that the tick-driven pipelined FP units produce correct
 * results and exhibit proper pipeline behavior (latency, throughput, etc.).
 */
import { describe, it, expect } from "vitest";
import { FP32 } from "../src/formats.js";
import { floatToBits, bitsToFloat, isNaN as fpIsNaN } from "../src/ieee754.js";
import {
  PipelinedFPAdder,
  PipelinedFPMultiplier,
  PipelinedFMA,
  FPUnit,
} from "../src/pipeline.js";

function approx(expected: number, actual: number, tol = 1e-6): boolean {
  if (Number.isNaN(expected) && Number.isNaN(actual)) return true;
  if (!Number.isFinite(expected) && !Number.isFinite(actual)) return expected === actual;
  return Math.abs(expected - actual) <= tol;
}

// ===========================================================================
// PipelinedFPAdder tests
// ===========================================================================

describe("PipelinedFPAdder", () => {
  it("single addition", () => {
    const adder = new PipelinedFPAdder();
    adder.submit(floatToBits(1.0), floatToBits(2.0));
    for (let i = 0; i < 5; i++) adder.tick();
    expect(adder.results.length).toBe(1);
    expect(bitsToFloat(adder.results[0])).toBe(3.0);
  });

  it("negative result", () => {
    const adder = new PipelinedFPAdder();
    adder.submit(floatToBits(1.0), floatToBits(-3.0));
    for (let i = 0; i < 5; i++) adder.tick();
    expect(adder.results.length).toBe(1);
    expect(bitsToFloat(adder.results[0])).toBe(-2.0);
  });

  it("different exponents", () => {
    const adder = new PipelinedFPAdder();
    adder.submit(floatToBits(1.5), floatToBits(0.25));
    for (let i = 0; i < 5; i++) adder.tick();
    expect(adder.results.length).toBe(1);
    expect(approx(1.75, bitsToFloat(adder.results[0]))).toBe(true);
  });

  it("pipeline throughput", () => {
    const adder = new PipelinedFPAdder();
    const cases: [number, number, number][] = [
      [1.0, 2.0, 3.0], [3.0, 4.0, 7.0], [0.5, 0.5, 1.0],
      [10.0, -3.0, 7.0], [100.0, 200.0, 300.0],
    ];
    for (const [a, b] of cases) adder.submit(floatToBits(a), floatToBits(b));
    for (let i = 0; i < 9; i++) adder.tick();
    expect(adder.results.length).toBe(5);
    for (let i = 0; i < cases.length; i++) {
      expect(approx(cases[i][2], bitsToFloat(adder.results[i]))).toBe(true);
    }
  });

  it("pipeline latency", () => {
    const adder = new PipelinedFPAdder();
    adder.submit(floatToBits(1.0), floatToBits(2.0));
    for (let i = 0; i < 4; i++) adder.tick();
    expect(adder.results.length).toBe(0);
    adder.tick();
    expect(adder.results.length).toBe(1);
  });

  it("empty pipeline", () => {
    const adder = new PipelinedFPAdder();
    for (let i = 0; i < 10; i++) adder.tick();
    expect(adder.results.length).toBe(0);
    expect(adder.cycleCount).toBe(10);
  });

  it("NaN propagation", () => {
    const adder = new PipelinedFPAdder();
    adder.submit(floatToBits(NaN), floatToBits(1.0));
    for (let i = 0; i < 5; i++) adder.tick();
    expect(adder.results.length).toBe(1);
    expect(fpIsNaN(adder.results[0])).toBe(true);
  });

  it("Inf addition", () => {
    const adder = new PipelinedFPAdder();
    adder.submit(floatToBits(Infinity), floatToBits(1.0));
    for (let i = 0; i < 5; i++) adder.tick();
    expect(bitsToFloat(adder.results[0])).toBe(Infinity);
  });

  it("Inf + (-Inf) = NaN", () => {
    const adder = new PipelinedFPAdder();
    adder.submit(floatToBits(Infinity), floatToBits(-Infinity));
    for (let i = 0; i < 5; i++) adder.tick();
    expect(fpIsNaN(adder.results[0])).toBe(true);
  });

  it("zero addition", () => {
    const adder = new PipelinedFPAdder();
    adder.submit(floatToBits(0.0), floatToBits(5.0));
    for (let i = 0; i < 5; i++) adder.tick();
    expect(bitsToFloat(adder.results[0])).toBe(5.0);
  });

  it("subtraction to zero", () => {
    const adder = new PipelinedFPAdder();
    adder.submit(floatToBits(5.0), floatToBits(-5.0));
    for (let i = 0; i < 5; i++) adder.tick();
    expect(bitsToFloat(adder.results[0])).toBe(0.0);
  });

  it("cycle count", () => {
    const adder = new PipelinedFPAdder();
    expect(adder.cycleCount).toBe(0);
    for (let i = 0; i < 3; i++) adder.tick();
    expect(adder.cycleCount).toBe(3);
  });
});

// ===========================================================================
// PipelinedFPMultiplier tests
// ===========================================================================

describe("PipelinedFPMultiplier", () => {
  it("single multiplication", () => {
    const mul = new PipelinedFPMultiplier();
    mul.submit(floatToBits(2.0), floatToBits(3.0));
    for (let i = 0; i < 4; i++) mul.tick();
    expect(mul.results.length).toBe(1);
    expect(bitsToFloat(mul.results[0])).toBe(6.0);
  });

  it("negative result", () => {
    const mul = new PipelinedFPMultiplier();
    mul.submit(floatToBits(-2.0), floatToBits(3.0));
    for (let i = 0; i < 4; i++) mul.tick();
    expect(bitsToFloat(mul.results[0])).toBe(-6.0);
  });

  it("neg * neg = pos", () => {
    const mul = new PipelinedFPMultiplier();
    mul.submit(floatToBits(-2.0), floatToBits(-3.0));
    for (let i = 0; i < 4; i++) mul.tick();
    expect(bitsToFloat(mul.results[0])).toBe(6.0);
  });

  it("pipeline latency 4", () => {
    const mul = new PipelinedFPMultiplier();
    mul.submit(floatToBits(2.0), floatToBits(3.0));
    for (let i = 0; i < 3; i++) mul.tick();
    expect(mul.results.length).toBe(0);
    mul.tick();
    expect(mul.results.length).toBe(1);
  });

  it("pipeline throughput", () => {
    const mul = new PipelinedFPMultiplier();
    const cases: [number, number, number][] = [
      [2.0, 3.0, 6.0], [1.5, 4.0, 6.0], [0.5, 10.0, 5.0], [7.0, 8.0, 56.0],
    ];
    for (const [a, b] of cases) mul.submit(floatToBits(a), floatToBits(b));
    for (let i = 0; i < 7; i++) mul.tick();
    expect(mul.results.length).toBe(4);
    for (let i = 0; i < cases.length; i++) {
      expect(approx(cases[i][2], bitsToFloat(mul.results[i]))).toBe(true);
    }
  });

  it("multiply by zero", () => {
    const mul = new PipelinedFPMultiplier();
    mul.submit(floatToBits(42.0), floatToBits(0.0));
    for (let i = 0; i < 4; i++) mul.tick();
    expect(bitsToFloat(mul.results[0])).toBe(0.0);
  });

  it("multiply NaN", () => {
    const mul = new PipelinedFPMultiplier();
    mul.submit(floatToBits(NaN), floatToBits(1.0));
    for (let i = 0; i < 4; i++) mul.tick();
    expect(fpIsNaN(mul.results[0])).toBe(true);
  });

  it("multiply Inf", () => {
    const mul = new PipelinedFPMultiplier();
    mul.submit(floatToBits(Infinity), floatToBits(2.0));
    for (let i = 0; i < 4; i++) mul.tick();
    expect(bitsToFloat(mul.results[0])).toBe(Infinity);
  });

  it("Inf * 0 = NaN", () => {
    const mul = new PipelinedFPMultiplier();
    mul.submit(floatToBits(Infinity), floatToBits(0.0));
    for (let i = 0; i < 4; i++) mul.tick();
    expect(fpIsNaN(mul.results[0])).toBe(true);
  });

  it("empty pipeline", () => {
    const mul = new PipelinedFPMultiplier();
    for (let i = 0; i < 10; i++) mul.tick();
    expect(mul.results.length).toBe(0);
    expect(mul.cycleCount).toBe(10);
  });
});

// ===========================================================================
// PipelinedFMA tests
// ===========================================================================

describe("PipelinedFMA", () => {
  it("basic fma", () => {
    const fma = new PipelinedFMA();
    fma.submit(floatToBits(2.0), floatToBits(3.0), floatToBits(1.0));
    for (let i = 0; i < 6; i++) fma.tick();
    expect(fma.results.length).toBe(1);
    expect(approx(7.0, bitsToFloat(fma.results[0]))).toBe(true);
  });

  it("fma latency", () => {
    const fma = new PipelinedFMA();
    fma.submit(floatToBits(1.0), floatToBits(1.0), floatToBits(1.0));
    for (let i = 0; i < 5; i++) fma.tick();
    expect(fma.results.length).toBe(0);
    fma.tick();
    expect(fma.results.length).toBe(1);
  });

  it("fma throughput", () => {
    const fma = new PipelinedFMA();
    const cases: [number, number, number, number][] = [
      [2.0, 3.0, 1.0, 7.0], [1.5, 2.0, 0.5, 3.5], [4.0, 0.5, 1.0, 3.0],
    ];
    for (const [a, b, c] of cases) fma.submit(floatToBits(a), floatToBits(b), floatToBits(c));
    for (let i = 0; i < 8; i++) fma.tick();
    expect(fma.results.length).toBe(3);
    for (let i = 0; i < cases.length; i++) {
      expect(approx(cases[i][3], bitsToFloat(fma.results[i]))).toBe(true);
    }
  });

  it("fma NaN", () => {
    const fma = new PipelinedFMA();
    fma.submit(floatToBits(NaN), floatToBits(1.0), floatToBits(1.0));
    for (let i = 0; i < 6; i++) fma.tick();
    expect(fpIsNaN(fma.results[0])).toBe(true);
  });

  it("fma Inf * 0 = NaN", () => {
    const fma = new PipelinedFMA();
    fma.submit(floatToBits(Infinity), floatToBits(0.0), floatToBits(1.0));
    for (let i = 0; i < 6; i++) fma.tick();
    expect(fpIsNaN(fma.results[0])).toBe(true);
  });

  it("fma 0 * finite = c", () => {
    const fma = new PipelinedFMA();
    fma.submit(floatToBits(0.0), floatToBits(5.0), floatToBits(3.0));
    for (let i = 0; i < 6; i++) fma.tick();
    expect(bitsToFloat(fma.results[0])).toBe(3.0);
  });

  it("fma Inf operand", () => {
    const fma = new PipelinedFMA();
    fma.submit(floatToBits(Infinity), floatToBits(2.0), floatToBits(1.0));
    for (let i = 0; i < 6; i++) fma.tick();
    expect(bitsToFloat(fma.results[0])).toBe(Infinity);
  });

  it("fma c Inf", () => {
    const fma = new PipelinedFMA();
    fma.submit(floatToBits(2.0), floatToBits(3.0), floatToBits(Infinity));
    for (let i = 0; i < 6; i++) fma.tick();
    expect(bitsToFloat(fma.results[0])).toBe(Infinity);
  });

  it("fma Inf + (-Inf) = NaN", () => {
    const fma = new PipelinedFMA();
    fma.submit(floatToBits(Infinity), floatToBits(1.0), floatToBits(-Infinity));
    for (let i = 0; i < 6; i++) fma.tick();
    expect(fpIsNaN(fma.results[0])).toBe(true);
  });

  it("fma 0 + 0 = 0", () => {
    const fma = new PipelinedFMA();
    fma.submit(floatToBits(0.0), floatToBits(0.0), floatToBits(0.0));
    for (let i = 0; i < 6; i++) fma.tick();
    expect(bitsToFloat(fma.results[0])).toBe(0.0);
  });

  it("fma empty pipeline", () => {
    const fma = new PipelinedFMA();
    for (let i = 0; i < 10; i++) fma.tick();
    expect(fma.results.length).toBe(0);
    expect(fma.cycleCount).toBe(10);
  });

  it("fma cancellation", () => {
    const fma = new PipelinedFMA();
    fma.submit(floatToBits(2.0), floatToBits(3.0), floatToBits(-6.0));
    for (let i = 0; i < 6; i++) fma.tick();
    expect(bitsToFloat(fma.results[0])).toBe(0.0);
  });
});

// ===========================================================================
// FPUnit tests
// ===========================================================================

describe("FPUnit", () => {
  it("all pipelines simultaneously", () => {
    const unit = new FPUnit();
    unit.adder.submit(floatToBits(1.0), floatToBits(2.0));
    unit.multiplier.submit(floatToBits(3.0), floatToBits(4.0));
    unit.fma.submit(floatToBits(2.0), floatToBits(3.0), floatToBits(1.0));
    unit.tick(6);
    expect(unit.adder.results.length).toBe(1);
    expect(unit.multiplier.results.length).toBe(1);
    expect(unit.fma.results.length).toBe(1);
    expect(bitsToFloat(unit.adder.results[0])).toBe(3.0);
    expect(bitsToFloat(unit.multiplier.results[0])).toBe(12.0);
    expect(approx(7.0, bitsToFloat(unit.fma.results[0]))).toBe(true);
  });

  it("tick method", () => {
    const unit = new FPUnit();
    unit.adder.submit(floatToBits(10.0), floatToBits(20.0));
    unit.tick(5);
    expect(bitsToFloat(unit.adder.results[0])).toBe(30.0);
  });

  it("empty tick", () => {
    const unit = new FPUnit();
    unit.tick(10);
    expect(unit.adder.results.length).toBe(0);
    expect(unit.multiplier.results.length).toBe(0);
    expect(unit.fma.results.length).toBe(0);
  });

  it("unit format", () => {
    const unit = new FPUnit();
    expect(unit.fmt).toBe(FP32);
    expect(unit.adder.fmt).toBe(FP32);
    expect(unit.multiplier.fmt).toBe(FP32);
    expect(unit.fma.fmt).toBe(FP32);
  });

  it("interleaved add and multiply", () => {
    const unit = new FPUnit();
    unit.adder.submit(floatToBits(1.0), floatToBits(2.0));
    unit.multiplier.submit(floatToBits(3.0), floatToBits(4.0));
    unit.adder.submit(floatToBits(5.0), floatToBits(6.0));
    unit.multiplier.submit(floatToBits(7.0), floatToBits(8.0));
    unit.tick(7);
    expect(unit.adder.results.length).toBe(2);
    expect(unit.multiplier.results.length).toBe(2);
    expect(bitsToFloat(unit.adder.results[0])).toBe(3.0);
    expect(bitsToFloat(unit.adder.results[1])).toBe(11.0);
    expect(bitsToFloat(unit.multiplier.results[0])).toBe(12.0);
    expect(bitsToFloat(unit.multiplier.results[1])).toBe(56.0);
  });

  it("heavy throughput", () => {
    const adder = new PipelinedFPAdder();
    for (let i = 0; i < 10; i++) {
      adder.submit(floatToBits(i), floatToBits(i + 1));
    }
    for (let i = 0; i < 14; i++) adder.tick();
    expect(adder.results.length).toBe(10);
    for (let i = 0; i < 10; i++) {
      const expected = i + (i + 1);
      expect(approx(expected, bitsToFloat(adder.results[i]))).toBe(true);
    }
  });
});
