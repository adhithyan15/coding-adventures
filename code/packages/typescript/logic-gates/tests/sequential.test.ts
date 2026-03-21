/**
 * Tests for sequential logic — latches, flip-flops, registers, and counters.
 *
 * These tests verify each component's truth table, state-holding behavior,
 * and edge cases. We test from the bottom up, since higher-level components
 * (like registers) depend on lower-level ones (like flip-flops).
 */

import { describe, it, expect } from "vitest";
import {
  srLatch,
  dLatch,
  dFlipFlop,
  register,
  shiftRegister,
  counter,
  type Bit,
  type FlipFlopState,
  type CounterState,
} from "../src/index.js";

// ===========================================================================
// SR LATCH TESTS
// ===========================================================================

describe("SR Latch", () => {
  it("set: S=1, R=0 should set Q=1", () => {
    const [q, qBar] = srLatch(1, 0);
    expect(q).toBe(1);
    expect(qBar).toBe(0);
  });

  it("reset: S=0, R=1 should reset Q=0", () => {
    const [q, qBar] = srLatch(0, 1);
    expect(q).toBe(0);
    expect(qBar).toBe(1);
  });

  it("hold after set: S=0, R=0 should hold Q=1", () => {
    let [q, qBar] = srLatch(1, 0);
    expect(q).toBe(1);
    // Now hold — pass the current state back
    [q, qBar] = srLatch(0, 0, q, qBar);
    expect(q).toBe(1);
    expect(qBar).toBe(0);
  });

  it("hold after reset: S=0, R=0 should hold Q=0", () => {
    let [q, qBar] = srLatch(0, 1);
    expect(q).toBe(0);
    [q, qBar] = srLatch(0, 0, q, qBar);
    expect(q).toBe(0);
    expect(qBar).toBe(1);
  });

  it("hold default state: S=0, R=0 should hold Q=0, Q_bar=1", () => {
    const [q, qBar] = srLatch(0, 0);
    expect(q).toBe(0);
    expect(qBar).toBe(1);
  });

  it("invalid both set and reset: S=1, R=1 gives both 0", () => {
    const [q, qBar] = srLatch(1, 1);
    expect(q).toBe(0);
    expect(qBar).toBe(0);
  });

  it("set then reset then set transitions correctly", () => {
    let [q, qBar] = srLatch(1, 0);
    expect(q).toBe(1);

    [q, qBar] = srLatch(0, 1, q, qBar);
    expect(q).toBe(0);
    expect(qBar).toBe(1);

    [q, qBar] = srLatch(1, 0, q, qBar);
    expect(q).toBe(1);
    expect(qBar).toBe(0);
  });

  it("multiple holds should be stable", () => {
    let [q, qBar] = srLatch(1, 0); // Set
    for (let i = 0; i < 10; i++) {
      [q, qBar] = srLatch(0, 0, q, qBar);
    }
    expect(q).toBe(1);
    expect(qBar).toBe(0);
  });

  it("non-number input throws TypeError", () => {
    expect(() => srLatch(true as unknown as Bit, 0)).toThrow(TypeError);
  });

  it("out-of-range input throws RangeError", () => {
    expect(() => srLatch(2 as Bit, 0)).toThrow(RangeError);
  });
});

// ===========================================================================
// D LATCH TESTS
// ===========================================================================

describe("D Latch", () => {
  it("store 1 when enabled", () => {
    const [q, qBar] = dLatch(1, 1);
    expect(q).toBe(1);
    expect(qBar).toBe(0);
  });

  it("store 0 when enabled", () => {
    const [q, qBar] = dLatch(0, 1);
    expect(q).toBe(0);
    expect(qBar).toBe(1);
  });

  it("hold when disabled after set", () => {
    let [q, qBar] = dLatch(1, 1); // Store 1
    expect(q).toBe(1);
    [q, qBar] = dLatch(0, 0, q, qBar); // Disable — data is 0 but ignored
    expect(q).toBe(1);
    expect(qBar).toBe(0);
  });

  it("hold when disabled after reset", () => {
    let [q, qBar] = dLatch(0, 1); // Store 0
    [q, qBar] = dLatch(1, 0, q, qBar); // Disable — data is 1 but ignored
    expect(q).toBe(0);
    expect(qBar).toBe(1);
  });

  it("transparent mode: output follows data when enabled", () => {
    let [q, qBar] = dLatch(1, 1);
    expect(q).toBe(1);
    [q, qBar] = dLatch(0, 1, q, qBar);
    expect(q).toBe(0);
    [q, qBar] = dLatch(1, 1, q, qBar);
    expect(q).toBe(1);
  });

  it("hold default state: Enable=0 holds Q=0", () => {
    const [q, qBar] = dLatch(1, 0);
    expect(q).toBe(0);
    expect(qBar).toBe(1);
  });

  it("non-number input throws TypeError", () => {
    expect(() => dLatch(true as unknown as Bit, 1)).toThrow(TypeError);
  });

  it("out-of-range input throws RangeError", () => {
    expect(() => dLatch(0, 2 as Bit)).toThrow(RangeError);
  });
});

// ===========================================================================
// D FLIP-FLOP TESTS
// ===========================================================================

describe("D Flip-Flop", () => {
  it("captures data on rising edge", () => {
    // Clock low: master absorbs data=1
    let [q, qBar, state] = dFlipFlop(1, 0);
    // Clock high: slave outputs master's stored value
    [q, qBar, state] = dFlipFlop(1, 1, state);
    expect(q).toBe(1);
    expect(qBar).toBe(0);
  });

  it("captures zero", () => {
    let [q, qBar, state] = dFlipFlop(0, 0);
    [q, qBar, state] = dFlipFlop(0, 1, state);
    expect(q).toBe(0);
    expect(qBar).toBe(1);
  });

  it("holds during clock high", () => {
    // Capture 1
    let [q, qBar, state] = dFlipFlop(1, 0);
    [q, qBar, state] = dFlipFlop(1, 1, state);
    expect(q).toBe(1);
    // Data changes to 0 while clock is still high — output should hold
    [q, qBar, state] = dFlipFlop(0, 1, state);
    expect(q).toBe(1);
  });

  it("new value on next edge", () => {
    // Capture 1
    let [q, qBar, state] = dFlipFlop(1, 0);
    [q, qBar, state] = dFlipFlop(1, 1, state);
    expect(q).toBe(1);

    // Next cycle: capture 0
    [q, qBar, state] = dFlipFlop(0, 0, state);
    [q, qBar, state] = dFlipFlop(0, 1, state);
    expect(q).toBe(0);
    expect(qBar).toBe(1);
  });

  it("multiple clock cycles with different values", () => {
    let state: FlipFlopState = {
      masterQ: 0,
      masterQBar: 1,
      slaveQ: 0,
      slaveQBar: 1,
    };
    const values: Bit[] = [1, 0, 1, 1, 0];
    for (const val of values) {
      let q: Bit;
      [, , state] = dFlipFlop(val, 0, state);
      [q, , state] = dFlipFlop(val, 1, state);
      expect(q).toBe(val);
    }
  });

  it("internal state has expected keys", () => {
    const [, , state] = dFlipFlop(1, 0);
    expect(state).toHaveProperty("masterQ");
    expect(state).toHaveProperty("masterQBar");
    expect(state).toHaveProperty("slaveQ");
    expect(state).toHaveProperty("slaveQBar");
  });

  it("non-number input throws TypeError", () => {
    expect(() => dFlipFlop(true as unknown as Bit, 0)).toThrow(TypeError);
  });

  it("out-of-range input throws RangeError", () => {
    expect(() => dFlipFlop(0, 2 as Bit)).toThrow(RangeError);
  });
});

// ===========================================================================
// REGISTER TESTS
// ===========================================================================

describe("Register", () => {
  it("stores a 4-bit value", () => {
    // Clock low: absorb data
    let [out, state] = register([1, 0, 1, 1], 0);
    // Clock high: output stored data
    [out, state] = register([1, 0, 1, 1], 1, state);
    expect(out).toEqual([1, 0, 1, 1]);
  });

  it("stores an 8-bit value", () => {
    const data: Bit[] = [1, 0, 0, 1, 1, 0, 1, 0];
    let [out, state] = register(data, 0);
    [out, state] = register(data, 1, state);
    expect(out).toEqual(data);
  });

  it("holds previous value then captures new data", () => {
    const data1: Bit[] = [1, 1, 0, 0];
    let [out, state] = register(data1, 0);
    [out, state] = register(data1, 1, state);
    expect(out).toEqual(data1);

    // Present new data, clock low then high
    const data2: Bit[] = [0, 0, 1, 1];
    [out, state] = register(data2, 0, state);
    [out, state] = register(data2, 1, state);
    expect(out).toEqual(data2);
  });

  it("width parameter enforces data length", () => {
    const [out] = register([1, 0], 0, undefined, 2);
    expect(out).toHaveLength(2);
  });

  it("width mismatch throws RangeError", () => {
    expect(() => register([1, 0, 1], 0, undefined, 2)).toThrow(
      "does not match width",
    );
  });

  it("empty data throws RangeError", () => {
    expect(() => register([], 0)).toThrow("must not be empty");
  });

  it("non-array data throws TypeError", () => {
    expect(() => register(1 as unknown as Bit[], 0)).toThrow(TypeError);
  });

  it("state length mismatch throws RangeError", () => {
    const [, state] = register([1, 0], 0);
    expect(() => register([1, 0, 1], 0, state)).toThrow(
      "does not match data length",
    );
  });

  it("single bit register", () => {
    let [out, state] = register([1], 0);
    [out, state] = register([1], 1, state);
    expect(out).toEqual([1]);
  });

  it("invalid bit in data throws", () => {
    expect(() => register([0, 2 as Bit, 1], 0)).toThrow();
  });

  it("all zeros", () => {
    const data: Bit[] = [0, 0, 0, 0];
    let [out, state] = register(data, 0);
    [out, state] = register(data, 1, state);
    expect(out).toEqual([0, 0, 0, 0]);
  });

  it("all ones", () => {
    const data: Bit[] = [1, 1, 1, 1];
    let [out, state] = register(data, 0);
    [out, state] = register(data, 1, state);
    expect(out).toEqual([1, 1, 1, 1]);
  });
});

// ===========================================================================
// SHIFT REGISTER TESTS
// ===========================================================================

describe("Shift Register", () => {
  it("shift right single bit", () => {
    let [out, sout, state] = shiftRegister(1, 0, undefined, 4);
    [out, sout, state] = shiftRegister(1, 1, state, 4);
    expect(out).toEqual([1, 0, 0, 0]);
    expect(sout).toBe(0); // Nothing shifted out yet
  });

  it("shift right fill", () => {
    let state: FlipFlopState[] | undefined;
    let out: Bit[];
    for (let i = 0; i < 4; i++) {
      [, , state] = shiftRegister(1, 0, state, 4);
      [out, , state] = shiftRegister(1, 1, state, 4);
    }
    expect(out!).toEqual([1, 1, 1, 1]);
  });

  it("shift right serial out", () => {
    // Fill with 1s
    let state: FlipFlopState[] | undefined;
    let sout: Bit;
    let out: Bit[];
    for (let i = 0; i < 4; i++) {
      [, , state] = shiftRegister(1, 0, state, 4);
      [, sout, state] = shiftRegister(1, 1, state, 4);
    }

    // Now shift in 0 — the rightmost 1 should come out
    [, , state] = shiftRegister(0, 0, state, 4);
    [out, sout, state] = shiftRegister(0, 1, state, 4);
    expect(sout).toBe(1);
    expect(out).toEqual([0, 1, 1, 1]);
  });

  it("shift left single bit", () => {
    let [out, sout, state] = shiftRegister(1, 0, undefined, 4, "left");
    [out, sout, state] = shiftRegister(1, 1, state, 4, "left");
    expect(out).toEqual([0, 0, 0, 1]);
    expect(sout).toBe(0);
  });

  it("shift left fill", () => {
    let state: FlipFlopState[] | undefined;
    let out: Bit[];
    for (let i = 0; i < 4; i++) {
      [, , state] = shiftRegister(1, 0, state, 4, "left");
      [out, , state] = shiftRegister(1, 1, state, 4, "left");
    }
    expect(out!).toEqual([1, 1, 1, 1]);
  });

  it("shift left serial out", () => {
    // Fill with 1s
    let state: FlipFlopState[] | undefined;
    let sout: Bit;
    let out: Bit[];
    for (let i = 0; i < 4; i++) {
      [, , state] = shiftRegister(1, 0, state, 4, "left");
      [, , state] = shiftRegister(1, 1, state, 4, "left");
    }

    // Shift in 0 — leftmost 1 should come out
    [, , state] = shiftRegister(0, 0, state, 4, "left");
    [out, sout, state] = shiftRegister(0, 1, state, 4, "left");
    expect(sout).toBe(1);
    expect(out).toEqual([1, 1, 1, 0]);
  });

  it("pattern shift right", () => {
    const pattern: Bit[] = [1, 0, 1, 0];
    let state: FlipFlopState[] | undefined;
    let out: Bit[];
    for (const bit of pattern) {
      [, , state] = shiftRegister(bit, 0, state, 4);
      [out, , state] = shiftRegister(bit, 1, state, 4);
    }
    // Last bit shifted in is at index 0, first bit at index 3
    expect(out!).toEqual([0, 1, 0, 1]);
  });

  it("width=1", () => {
    let [out, sout, state] = shiftRegister(1, 0, undefined, 1);
    [out, sout, state] = shiftRegister(1, 1, state, 1);
    expect(out).toEqual([1]);
    expect(sout).toBe(0);
  });

  it("invalid direction throws", () => {
    expect(() =>
      shiftRegister(1, 0, undefined, 4, "up" as "right"),
    ).toThrow("direction must be");
  });

  it("invalid width throws", () => {
    expect(() => shiftRegister(1, 0, undefined, 0)).toThrow("width must be");
  });

  it("state length mismatch throws", () => {
    const [, , state] = shiftRegister(1, 0, undefined, 4);
    expect(() => shiftRegister(1, 0, state, 3)).toThrow(
      "does not match width",
    );
  });

  it("default width is 8", () => {
    const [out] = shiftRegister(0, 0);
    expect(out).toHaveLength(8);
  });
});

// ===========================================================================
// COUNTER TESTS
// ===========================================================================

describe("Counter", () => {
  it("counts to three", () => {
    /**
     * Counter should count 1, 2, 3, 4 after successive clock edges.
     *
     * The counter increments on each rising clock edge, so the first
     * tick produces 1 (not 0). This matches hardware behavior — a real
     * counter starts at 0 and the first edge gives 1.
     */
    let state: CounterState | undefined;
    const expectedValues: Bit[][] = [
      [1, 0, 0, 0], // 1 (first tick)
      [0, 1, 0, 0], // 2
      [1, 1, 0, 0], // 3
      [0, 0, 1, 0], // 4
    ];

    for (let i = 0; i < 4; i++) {
      let bits: Bit[];
      [bits, state] = counter(0, 0, state, 4);
      [bits, state] = counter(1, 0, state, 4);
      expect(bits).toEqual(expectedValues[i]);
    }
  });

  it("reset forces counter to zero", () => {
    let state: CounterState | undefined;
    // Count up a couple times
    for (let i = 0; i < 3; i++) {
      [, state] = counter(0, 0, state, 4);
      [, state] = counter(1, 0, state, 4);
    }

    // Reset
    let bits: Bit[];
    [bits, state] = counter(0, 1, state, 4);
    [bits, state] = counter(1, 1, state, 4);
    expect(bits).toEqual([0, 0, 0, 0]);
  });

  it("counts normally after reset", () => {
    let state: CounterState | undefined;
    // Count to 3
    for (let i = 0; i < 3; i++) {
      [, state] = counter(0, 0, state, 4);
      [, state] = counter(1, 0, state, 4);
    }

    // Reset
    [, state] = counter(0, 1, state, 4);
    [, state] = counter(1, 1, state, 4);

    // Count again
    let bits: Bit[];
    [bits, state] = counter(0, 0, state, 4);
    [bits, state] = counter(1, 0, state, 4);
    expect(bits).toEqual([1, 0, 0, 0]); // 1
  });

  it("overflow wraps to zero", () => {
    let state: CounterState | undefined;
    const width = 3; // Max value = 7

    // Count to 7 (7 ticks since each tick increments)
    let bits: Bit[];
    for (let i = 0; i < 7; i++) {
      [, state] = counter(0, 0, state, width);
      [bits, state] = counter(1, 0, state, width);
    }
    expect(bits!).toEqual([1, 1, 1]); // 7

    // One more tick should wrap to 0
    [, state] = counter(0, 0, state, width);
    [bits, state] = counter(1, 0, state, width);
    expect(bits).toEqual([0, 0, 0]); // 0 (overflow)
  });

  it("1-bit counter toggles", () => {
    let state: CounterState | undefined;
    let bits: Bit[];

    // Tick 1 — starts at 0, increments to 1
    [, state] = counter(0, 0, state, 1);
    [bits, state] = counter(1, 0, state, 1);
    expect(bits).toEqual([1]);

    // Tick 2 — wraps to 0
    [, state] = counter(0, 0, state, 1);
    [bits, state] = counter(1, 0, state, 1);
    expect(bits).toEqual([0]);

    // Tick 3 — back to 1
    [, state] = counter(0, 0, state, 1);
    [bits, state] = counter(1, 0, state, 1);
    expect(bits).toEqual([1]);
  });

  it("invalid width throws", () => {
    expect(() => counter(0, 0, undefined, 0)).toThrow("width must be");
  });

  it("default width is 8", () => {
    const [bits] = counter(0);
    expect(bits).toHaveLength(8);
  });

  it("state has value and ffState keys", () => {
    const [, state] = counter(0, 0, undefined, 4);
    expect(state).toHaveProperty("value");
    expect(state).toHaveProperty("ffState");
  });
});
