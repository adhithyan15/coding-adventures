/**
 * Tests for PredictionStats — the branch prediction scoreboard.
 *
 * These tests verify the accuracy tracking, edge cases, and reset behavior
 * of the PredictionStats class.
 */

import { describe, expect, it } from "vitest";
import { PredictionStats } from "../src/stats.js";

// ─── Creation ───────────────────────────────────────────────────────────────

describe("PredictionStats creation", () => {
  it("default counters are zero", () => {
    const stats = new PredictionStats();
    expect(stats.predictions).toBe(0);
    expect(stats.correct).toBe(0);
    expect(stats.incorrect).toBe(0);
  });

  it("custom initial values", () => {
    const stats = new PredictionStats(10, 7, 3);
    expect(stats.predictions).toBe(10);
    expect(stats.correct).toBe(7);
    expect(stats.incorrect).toBe(3);
  });
});

// ─── Accuracy Calculation ───────────────────────────────────────────────────

describe("accuracy calculation", () => {
  it("accuracy with no predictions returns 0.0", () => {
    /** 0 predictions -> 0.0% accuracy (not a division error). */
    const stats = new PredictionStats();
    expect(stats.accuracy).toBe(0.0);
  });

  it("misprediction rate with no predictions returns 0.0", () => {
    /** 0 predictions -> 0.0% misprediction rate. */
    const stats = new PredictionStats();
    expect(stats.mispredictionRate).toBe(0.0);
  });

  it("perfect accuracy", () => {
    const stats = new PredictionStats(100, 100, 0);
    expect(stats.accuracy).toBe(100.0);
    expect(stats.mispredictionRate).toBe(0.0);
  });

  it("zero accuracy", () => {
    const stats = new PredictionStats(100, 0, 100);
    expect(stats.accuracy).toBe(0.0);
    expect(stats.mispredictionRate).toBe(100.0);
  });

  it("mixed accuracy", () => {
    const stats = new PredictionStats(200, 150, 50);
    expect(stats.accuracy).toBe(75.0);
    expect(stats.mispredictionRate).toBe(25.0);
  });

  it("accuracy and misprediction rate sum to 100", () => {
    const stats = new PredictionStats(37, 23, 14);
    expect(Math.abs(stats.accuracy + stats.mispredictionRate - 100.0)).toBeLessThan(1e-10);
  });
});

// ─── Record ─────────────────────────────────────────────────────────────────

describe("record", () => {
  it("record correct", () => {
    const stats = new PredictionStats();
    stats.record(true);
    expect(stats.predictions).toBe(1);
    expect(stats.correct).toBe(1);
    expect(stats.incorrect).toBe(0);
  });

  it("record incorrect", () => {
    const stats = new PredictionStats();
    stats.record(false);
    expect(stats.predictions).toBe(1);
    expect(stats.correct).toBe(0);
    expect(stats.incorrect).toBe(1);
  });

  it("record sequence", () => {
    /** Record a mixed sequence and verify counts. */
    const stats = new PredictionStats();
    const outcomes = [true, true, false, true, false, true, true, true, true, false];
    for (const outcome of outcomes) {
      stats.record(outcome);
    }
    expect(stats.predictions).toBe(10);
    expect(stats.correct).toBe(7);
    expect(stats.incorrect).toBe(3);
    expect(stats.accuracy).toBe(70.0);
  });
});

// ─── Reset ──────────────────────────────────────────────────────────────────

describe("reset", () => {
  it("reset clears all counters", () => {
    const stats = new PredictionStats(50, 40, 10);
    stats.reset();
    expect(stats.predictions).toBe(0);
    expect(stats.correct).toBe(0);
    expect(stats.incorrect).toBe(0);
  });

  it("reset after recording", () => {
    const stats = new PredictionStats();
    for (let i = 0; i < 20; i++) {
      stats.record(true);
    }
    stats.reset();
    expect(stats.predictions).toBe(0);
    expect(stats.accuracy).toBe(0.0);
  });

  it("record after reset", () => {
    /** Verify stats work correctly after a reset. */
    const stats = new PredictionStats();
    stats.record(true);
    stats.record(false);
    stats.reset();
    stats.record(true);
    expect(stats.predictions).toBe(1);
    expect(stats.correct).toBe(1);
    expect(stats.accuracy).toBe(100.0);
  });
});
