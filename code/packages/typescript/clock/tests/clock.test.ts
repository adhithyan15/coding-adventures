/**
 * Tests for the clock package.
 *
 * These tests verify the fundamental clock behavior: signal toggling,
 * edge detection, cycle counting, listener notification, frequency
 * division, and multi-phase generation.
 */

import { describe, it, expect } from "vitest";
import {
  Clock,
  ClockDivider,
  MultiPhaseClock,
} from "../src/index.js";
import type { ClockEdge } from "../src/index.js";

// ---------------------------------------------------------------------------
// Basic clock behavior
// ---------------------------------------------------------------------------

describe("ClockInitialState", () => {
  it("starts at zero", () => {
    /**
     * The clock signal starts low (0), like a real oscillator
     * before it begins oscillating.
     */
    const clk = new Clock();
    expect(clk.value).toBe(0);
  });

  it("starts at cycle zero", () => {
    /** No cycles have elapsed before the first tick. */
    const clk = new Clock();
    expect(clk.cycle).toBe(0);
  });

  it("starts with zero ticks", () => {
    /** No ticks have occurred yet. */
    const clk = new Clock();
    expect(clk.totalTicks).toBe(0);
  });

  it("default frequency is 1 MHz", () => {
    const clk = new Clock();
    expect(clk.frequencyHz).toBe(1_000_000);
  });

  it("can specify a custom frequency", () => {
    const clk = new Clock(3_000_000_000);
    expect(clk.frequencyHz).toBe(3_000_000_000);
  });
});

// ---------------------------------------------------------------------------
// Tick behavior
// ---------------------------------------------------------------------------

describe("ClockTick", () => {
  it("first tick is rising", () => {
    /** First tick goes from 0 to 1 -- a rising edge. */
    const clk = new Clock();
    const edge = clk.tick();
    expect(edge.isRising).toBe(true);
    expect(edge.isFalling).toBe(false);
    expect(edge.value).toBe(1);
    expect(clk.value).toBe(1);
  });

  it("second tick is falling", () => {
    /** Second tick goes from 1 to 0 -- a falling edge. */
    const clk = new Clock();
    clk.tick(); // rising
    const edge = clk.tick(); // falling
    expect(edge.isRising).toBe(false);
    expect(edge.isFalling).toBe(true);
    expect(edge.value).toBe(0);
    expect(clk.value).toBe(0);
  });

  it("alternates correctly", () => {
    /** The clock should alternate: rise, fall, rise, fall, ... */
    const clk = new Clock();
    for (let i = 0; i < 10; i++) {
      const edge = clk.tick();
      if (i % 2 === 0) {
        expect(edge.isRising).toBe(true);
      } else {
        expect(edge.isFalling).toBe(true);
      }
    }
  });

  it("cycle increments on rising edge", () => {
    /** Cycle count goes up by 1 on each rising edge. */
    const clk = new Clock();

    const edge1 = clk.tick(); // rising
    expect(edge1.cycle).toBe(1);
    expect(clk.cycle).toBe(1);

    const edge2 = clk.tick(); // falling
    expect(edge2.cycle).toBe(1); // still cycle 1
    expect(clk.cycle).toBe(1);

    const edge3 = clk.tick(); // rising
    expect(edge3.cycle).toBe(2);
    expect(clk.cycle).toBe(2);
  });

  it("tick count increments every tick", () => {
    /** Total ticks counts every half-cycle. */
    const clk = new Clock();
    clk.tick();
    expect(clk.totalTicks).toBe(1);
    clk.tick();
    expect(clk.totalTicks).toBe(2);
    clk.tick();
    expect(clk.totalTicks).toBe(3);
  });
});

// ---------------------------------------------------------------------------
// FullCycle
// ---------------------------------------------------------------------------

describe("ClockFullCycle", () => {
  it("returns rising then falling", () => {
    /** full_cycle produces exactly one rising and one falling edge. */
    const clk = new Clock();
    const [rising, falling] = clk.fullCycle();
    expect(rising.isRising).toBe(true);
    expect(falling.isFalling).toBe(true);
  });

  it("ends at zero", () => {
    /** After a full cycle, the clock is back to 0. */
    const clk = new Clock();
    clk.fullCycle();
    expect(clk.value).toBe(0);
  });

  it("cycle count is one", () => {
    /** One fullCycle means one cycle elapsed. */
    const clk = new Clock();
    clk.fullCycle();
    expect(clk.cycle).toBe(1);
  });

  it("two ticks elapsed", () => {
    /** A full cycle is two half-cycles. */
    const clk = new Clock();
    clk.fullCycle();
    expect(clk.totalTicks).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

describe("ClockRun", () => {
  it("produces correct edge count", () => {
    /** N cycles = 2N edges (each cycle has rising + falling). */
    const clk = new Clock();
    const edges = clk.run(5);
    expect(edges.length).toBe(10);
  });

  it("edges alternate", () => {
    /** Edges should alternate rising/falling. */
    const clk = new Clock();
    const edges = clk.run(3);
    for (let i = 0; i < edges.length; i++) {
      if (i % 2 === 0) {
        expect(edges[i].isRising).toBe(true);
      } else {
        expect(edges[i].isFalling).toBe(true);
      }
    }
  });

  it("final cycle count matches", () => {
    /** After run(N), cycle count should be N. */
    const clk = new Clock();
    clk.run(7);
    expect(clk.cycle).toBe(7);
  });

  it("run zero cycles does nothing", () => {
    /** run(0) does nothing. */
    const clk = new Clock();
    const edges = clk.run(0);
    expect(edges.length).toBe(0);
    expect(clk.cycle).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Listeners (observer pattern)
// ---------------------------------------------------------------------------

describe("ClockListeners", () => {
  it("listener called on tick", () => {
    /** A registered listener receives every edge. */
    const clk = new Clock();
    const received: ClockEdge[] = [];
    clk.registerListener((edge) => received.push(edge));
    clk.tick();
    expect(received.length).toBe(1);
    expect(received[0].isRising).toBe(true);
  });

  it("listener sees all edges", () => {
    /** Listener is called for both rising and falling edges. */
    const clk = new Clock();
    const received: ClockEdge[] = [];
    clk.registerListener((edge) => received.push(edge));
    clk.run(3);
    expect(received.length).toBe(6);
  });

  it("multiple listeners all get notified", () => {
    const clk = new Clock();
    const a: ClockEdge[] = [];
    const b: ClockEdge[] = [];
    clk.registerListener((edge) => a.push(edge));
    clk.registerListener((edge) => b.push(edge));
    clk.tick();
    expect(a.length).toBe(1);
    expect(b.length).toBe(1);
  });

  it("unregister listener stops receiving edges", () => {
    /** After unregistering, listener stops receiving edges. */
    const clk = new Clock();
    const received: ClockEdge[] = [];
    const listener = (edge: ClockEdge) => received.push(edge);
    clk.registerListener(listener);
    clk.tick(); // 1 edge received
    clk.unregisterListener(listener);
    clk.tick(); // should NOT be received
    expect(received.length).toBe(1);
  });

  it("unregister nonexistent throws", () => {
    /** Unregistering a callback that was never registered throws an error. */
    const clk = new Clock();
    const dummy = (_edge: ClockEdge) => {};
    expect(() => clk.unregisterListener(dummy)).toThrow();
  });
});

// ---------------------------------------------------------------------------
// Reset
// ---------------------------------------------------------------------------

describe("ClockReset", () => {
  it("reset value", () => {
    /** Value goes back to 0. */
    const clk = new Clock();
    clk.tick(); // now 1
    clk.reset();
    expect(clk.value).toBe(0);
  });

  it("reset cycle", () => {
    /** Cycle count goes back to 0. */
    const clk = new Clock();
    clk.run(5);
    clk.reset();
    expect(clk.cycle).toBe(0);
  });

  it("reset ticks", () => {
    /** Tick count goes back to 0. */
    const clk = new Clock();
    clk.run(5);
    clk.reset();
    expect(clk.totalTicks).toBe(0);
  });

  it("reset preserves listeners", () => {
    /** Listeners survive a reset. */
    const clk = new Clock();
    const received: ClockEdge[] = [];
    clk.registerListener((edge) => received.push(edge));
    clk.run(3);
    clk.reset();
    clk.tick();
    // Should still receive edges after reset
    expect(received.length).toBe(7); // 6 from run(3) + 1 from tick()
  });

  it("reset preserves frequency", () => {
    /** Frequency is unchanged after reset. */
    const clk = new Clock(5_000_000);
    clk.run(10);
    clk.reset();
    expect(clk.frequencyHz).toBe(5_000_000);
  });
});

// ---------------------------------------------------------------------------
// Period calculation
// ---------------------------------------------------------------------------

describe("ClockPeriod", () => {
  it("1 MHz period is 1000 ns", () => {
    const clk = new Clock(1_000_000);
    expect(clk.periodNs).toBe(1000.0);
  });

  it("1 GHz period is 1 ns", () => {
    const clk = new Clock(1_000_000_000);
    expect(clk.periodNs).toBe(1.0);
  });

  it("3 GHz period is approximately 0.333 ns", () => {
    const clk = new Clock(3_000_000_000);
    expect(Math.abs(clk.periodNs - 1e9 / 3_000_000_000)).toBeLessThan(1e-10);
  });
});

// ---------------------------------------------------------------------------
// ClockDivider
// ---------------------------------------------------------------------------

describe("ClockDivider", () => {
  it("divide by 2", () => {
    /** Dividing by 2: every 2 source cycles = 1 output cycle. */
    const master = new Clock(1_000_000);
    const divider = new ClockDivider(master, 2);
    master.run(4); // 4 master cycles
    expect(divider.output.cycle).toBe(2);
  });

  it("divide by 4", () => {
    /** Dividing by 4: every 4 source cycles = 1 output cycle. */
    const master = new Clock(1_000_000_000);
    const divider = new ClockDivider(master, 4);
    master.run(8);
    expect(divider.output.cycle).toBe(2);
  });

  it("output frequency", () => {
    /** Output clock has the divided frequency. */
    const master = new Clock(1_000_000_000);
    const divider = new ClockDivider(master, 4);
    expect(divider.output.frequencyHz).toBe(250_000_000);
  });

  it("divisor too small throws", () => {
    /** Divisor must be >= 2. */
    const master = new Clock();
    expect(() => new ClockDivider(master, 1)).toThrow("Divisor must be >= 2");
  });

  it("divisor zero throws", () => {
    /** Divisor of 0 is invalid. */
    const master = new Clock();
    expect(() => new ClockDivider(master, 0)).toThrow("Divisor must be >= 2");
  });

  it("divisor negative throws", () => {
    /** Negative divisor is invalid. */
    const master = new Clock();
    expect(() => new ClockDivider(master, -1)).toThrow("Divisor must be >= 2");
  });

  it("output value returns to zero", () => {
    /** Output clock value returns to 0 after each output cycle. */
    const master = new Clock(1_000_000);
    const divider = new ClockDivider(master, 2);
    master.run(2); // Should trigger 1 output cycle
    expect(divider.output.value).toBe(0); // Full cycle completed
  });
});

// ---------------------------------------------------------------------------
// MultiPhaseClock
// ---------------------------------------------------------------------------

describe("MultiPhaseClock", () => {
  it("initial state all zero", () => {
    /** Before any ticks, all phases are 0. */
    const master = new Clock();
    const mpc = new MultiPhaseClock(master, 4);
    for (let i = 0; i < 4; i++) {
      expect(mpc.getPhase(i)).toBe(0);
    }
  });

  it("first rising activates phase 0", () => {
    /** After the first rising edge, phase 0 is active. */
    const master = new Clock();
    const mpc = new MultiPhaseClock(master, 4);
    master.tick(); // rising edge
    expect(mpc.getPhase(0)).toBe(1);
    expect(mpc.getPhase(1)).toBe(0);
    expect(mpc.getPhase(2)).toBe(0);
    expect(mpc.getPhase(3)).toBe(0);
  });

  it("phases rotate", () => {
    /** Each rising edge rotates to the next phase. */
    const master = new Clock();
    const mpc = new MultiPhaseClock(master, 4);

    // Cycle through all 4 phases
    for (let expectedPhase = 0; expectedPhase < 4; expectedPhase++) {
      master.tick(); // rising
      for (let p = 0; p < 4; p++) {
        if (p === expectedPhase) {
          expect(mpc.getPhase(p)).toBe(1);
        } else {
          expect(mpc.getPhase(p)).toBe(0);
        }
      }
      master.tick(); // falling (no change)
    }
  });

  it("phases wrap around", () => {
    /** After cycling through all phases, it wraps back to phase 0. */
    const master = new Clock();
    const mpc = new MultiPhaseClock(master, 3);

    // 3 rising edges cycle through phases 0, 1, 2
    for (let i = 0; i < 3; i++) {
      master.fullCycle();
    }

    // 4th rising edge should activate phase 0 again
    master.tick(); // rising
    expect(mpc.getPhase(0)).toBe(1);
    expect(mpc.getPhase(1)).toBe(0);
    expect(mpc.getPhase(2)).toBe(0);
  });

  it("only one phase active at a time", () => {
    /** At any time, at most one phase is active (non-overlapping). */
    const master = new Clock();
    const mpc = new MultiPhaseClock(master, 4);

    for (let i = 0; i < 20; i++) {
      master.tick();
      let activeCount = 0;
      for (let p = 0; p < 4; p++) {
        activeCount += mpc.getPhase(p);
      }
      expect(activeCount).toBeLessThanOrEqual(1);
    }
  });

  it("phases too small throws", () => {
    /** Phases must be >= 2. */
    const master = new Clock();
    expect(() => new MultiPhaseClock(master, 1)).toThrow("Phases must be >= 2");
  });

  it("phases zero throws", () => {
    /** Zero phases is invalid. */
    const master = new Clock();
    expect(() => new MultiPhaseClock(master, 0)).toThrow("Phases must be >= 2");
  });

  it("two phase clock", () => {
    /** A 2-phase clock alternates between two phases. */
    const master = new Clock();
    const mpc = new MultiPhaseClock(master, 2);

    master.tick(); // rising -> phase 0 active
    expect(mpc.getPhase(0)).toBe(1);
    expect(mpc.getPhase(1)).toBe(0);

    master.tick(); // falling -> no change
    master.tick(); // rising -> phase 1 active
    expect(mpc.getPhase(0)).toBe(0);
    expect(mpc.getPhase(1)).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// ClockEdge interface
// ---------------------------------------------------------------------------

describe("ClockEdge", () => {
  it("edge fields", () => {
    /** ClockEdge stores all transition information. */
    const edge: ClockEdge = {
      cycle: 3,
      value: 1,
      isRising: true,
      isFalling: false,
    };
    expect(edge.cycle).toBe(3);
    expect(edge.value).toBe(1);
    expect(edge.isRising).toBe(true);
    expect(edge.isFalling).toBe(false);
  });

  it("edge equality", () => {
    /** Two edges with the same fields are deeply equal. */
    const a: ClockEdge = {
      cycle: 1,
      value: 1,
      isRising: true,
      isFalling: false,
    };
    const b: ClockEdge = {
      cycle: 1,
      value: 1,
      isRising: true,
      isFalling: false,
    };
    expect(a).toEqual(b);
  });
});
