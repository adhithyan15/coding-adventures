/**
 * Tests for the Intel 4004 simulator.
 *
 * These tests verify each instruction independently, then test them together
 * in the x = 1 + 2 end-to-end program. The key constraint tested throughout:
 * all values are 4 bits (0-15), enforced by masking with & 0xF.
 */

import { describe, it, expect } from "vitest";
import { Intel4004Simulator } from "../src/simulator.js";

// ---------------------------------------------------------------------------
// LDM -- Load immediate into accumulator
// ---------------------------------------------------------------------------

describe("TestLDM", () => {
  /** LDM N (0xDN): Load a 4-bit immediate value into the accumulator. */

  it("ldm_sets_accumulator", () => {
    /** LDM 5 should set A = 5. */
    const sim = new Intel4004Simulator();
    // LDM 5 = 0xD5, HLT = 0x01
    const traces = sim.run(new Uint8Array([0xd5, 0x01]));
    expect(sim.accumulator).toBe(5);
    expect(traces[0].mnemonic).toBe("LDM 5");
    expect(traces[0].accumulatorBefore).toBe(0);
    expect(traces[0].accumulatorAfter).toBe(5);
  });

  it("ldm_zero", () => {
    /** LDM 0 should set A = 0. */
    const sim = new Intel4004Simulator();
    const traces = sim.run(new Uint8Array([0xd0, 0x01]));
    expect(sim.accumulator).toBe(0);
    expect(traces[0].mnemonic).toBe("LDM 0");
  });

  it("ldm_max_value", () => {
    /** LDM 15 should set A = 15 (the maximum 4-bit value). */
    const sim = new Intel4004Simulator();
    const traces = sim.run(new Uint8Array([0xdf, 0x01]));
    expect(sim.accumulator).toBe(15);
    expect(traces[0].mnemonic).toBe("LDM 15");
  });
});

// ---------------------------------------------------------------------------
// XCH -- Exchange accumulator with register
// ---------------------------------------------------------------------------

describe("TestXCH", () => {
  /** XCH RN (0xBN): Swap the accumulator and register N. */

  it("xch_swaps_values", () => {
    /** XCH R0 should swap A and R0. Start: A=7, R0=0. After: A=0, R0=7. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd7, 0xb0, 0x01]));
    expect(sim.accumulator).toBe(0);
    expect(sim.registers[0]).toBe(7);
  });

  it("xch_is_symmetric", () => {
    /** Two XCH operations on the same register restore original state. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd3, 0xb5, 0xb5, 0x01]));
    expect(sim.accumulator).toBe(3);
    expect(sim.registers[5]).toBe(0);
  });

  it("xch_high_register", () => {
    /** XCH R15 should work with the highest register number. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd9, 0xbf, 0x01]));
    expect(sim.registers[15]).toBe(9);
    expect(sim.accumulator).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// ADD -- Add register to accumulator
// ---------------------------------------------------------------------------

describe("TestADD", () => {
  /** ADD RN (0x8N): A = A + RN, set carry on overflow. */

  it("add_basic", () => {
    /** 2 + 3 = 5, no carry. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd3, 0xb0, 0xd2, 0x80, 0x01]));
    expect(sim.accumulator).toBe(5);
    expect(sim.carry).toBe(false);
  });

  it("add_carry_on_overflow", () => {
    /** 15 + 1 = 0 with carry. In 4 bits: 1111 + 0001 = 10000, truncated with carry. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd1, 0xb0, 0xdf, 0x80, 0x01]));
    expect(sim.accumulator).toBe(0);
    expect(sim.carry).toBe(true);
  });

  it("add_no_carry_at_boundary", () => {
    /** 8 + 7 = 15, no carry (exactly at the maximum). */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd7, 0xb0, 0xd8, 0x80, 0x01]));
    expect(sim.accumulator).toBe(15);
    expect(sim.carry).toBe(false);
  });

  it("add_both_max", () => {
    /** 15 + 15 = 14 with carry (30 in decimal, 0x1E masked to 0xE). */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xdf, 0xb0, 0xdf, 0x80, 0x01]));
    expect(sim.accumulator).toBe(14);
    expect(sim.carry).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// SUB -- Subtract register from accumulator
// ---------------------------------------------------------------------------

describe("TestSUB", () => {
  /** SUB RN (0x9N): A = A - RN, set carry (borrow) on underflow. */

  it("sub_basic", () => {
    /** 5 - 3 = 2, no borrow. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd3, 0xb0, 0xd5, 0x90, 0x01]));
    expect(sim.accumulator).toBe(2);
    expect(sim.carry).toBe(false);
  });

  it("sub_borrow_on_underflow", () => {
    /** 0 - 1 = 15 with borrow. In 4 bits: 0000 - 0001 = 1111 (15) with borrow. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd1, 0xb0, 0xd0, 0x90, 0x01]));
    expect(sim.accumulator).toBe(15);
    expect(sim.carry).toBe(true);
  });

  it("sub_equal_values", () => {
    /** 7 - 7 = 0, no borrow. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd7, 0xb0, 0xd7, 0x90, 0x01]));
    expect(sim.accumulator).toBe(0);
    expect(sim.carry).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// 4-bit masking -- the fundamental constraint
// ---------------------------------------------------------------------------

describe("TestFourBitMasking", () => {
  /** All values must be masked to 4 bits (0-15). */

  it("accumulator_never_exceeds_15", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xdf, 0xb0, 0xdf, 0x80, 0x01]));
    expect(sim.accumulator).toBeGreaterThanOrEqual(0);
    expect(sim.accumulator).toBeLessThanOrEqual(15);
  });

  it("registers_never_exceed_15", () => {
    const sim = new Intel4004Simulator();
    sim.run(
      new Uint8Array([
        0xdf, 0xb0, // LDM 15, XCH R0
        0xda, 0xb1, // LDM 10, XCH R1
        0xd0, 0xb2, // LDM 0, XCH R2
        0x01, // HLT
      ])
    );
    for (let i = 0; i < sim.registers.length; i++) {
      expect(sim.registers[i]).toBeGreaterThanOrEqual(0);
      expect(sim.registers[i]).toBeLessThanOrEqual(15);
    }
  });

  it("sub_wraps_to_4_bits", () => {
    /** Subtraction wraps around in 4 bits: 3 - 5 = 14. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd5, 0xb0, 0xd3, 0x90, 0x01]));
    expect(sim.accumulator).toBe(14);
    expect(sim.carry).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// HLT -- Halt execution
// ---------------------------------------------------------------------------

describe("TestHLT", () => {
  /** HLT (0x01): Stop the CPU. */

  it("hlt_stops_execution", () => {
    const sim = new Intel4004Simulator();
    const traces = sim.run(new Uint8Array([0x01]));
    expect(sim.halted).toBe(true);
    expect(traces.length).toBe(1);
    expect(traces[0].mnemonic).toBe("HLT");
  });

  it("hlt_mid_program", () => {
    /** Instructions after HLT should not execute. */
    const sim = new Intel4004Simulator();
    const traces = sim.run(new Uint8Array([0x01, 0xd5]));
    expect(sim.halted).toBe(true);
    expect(sim.accumulator).toBe(0);
    expect(traces.length).toBe(1);
  });

  it("step_after_halt_raises", () => {
    /** Stepping after HLT should throw an error. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0x01]));
    expect(() => sim.step()).toThrow(/halted/);
  });
});

// ---------------------------------------------------------------------------
// End-to-end: x = 1 + 2
// ---------------------------------------------------------------------------

describe("TestEndToEnd", () => {
  /** The canonical x = 1 + 2 program, testing the full instruction flow. */

  it("x_equals_1_plus_2", () => {
    const sim = new Intel4004Simulator();
    const program = new Uint8Array([0xd1, 0xb0, 0xd2, 0x80, 0xb1, 0x01]);
    const traces = sim.run(program);

    expect(sim.registers[1]).toBe(3);
    expect(sim.registers[0]).toBe(1);
    expect(sim.accumulator).toBe(0);
    expect(sim.carry).toBe(false);
    expect(sim.halted).toBe(true);

    expect(traces.length).toBe(6);
    expect(traces[0].mnemonic).toBe("LDM 1");
    expect(traces[1].mnemonic).toBe("XCH R0");
    expect(traces[2].mnemonic).toBe("LDM 2");
    expect(traces[3].mnemonic).toBe("ADD R0");
    expect(traces[4].mnemonic).toBe("XCH R1");
    expect(traces[5].mnemonic).toBe("HLT");
  });

  it("trace_accumulator_flow", () => {
    /** Verify the accumulator values through each step of x = 1 + 2. */
    const sim = new Intel4004Simulator();
    const program = new Uint8Array([0xd1, 0xb0, 0xd2, 0x80, 0xb1, 0x01]);
    const traces = sim.run(program);

    const expectedAcc: [number, number][] = [
      [0, 1], // LDM 1
      [1, 0], // XCH R0
      [0, 2], // LDM 2
      [2, 3], // ADD R0
      [3, 0], // XCH R1
      [0, 0], // HLT
    ];

    for (let i = 0; i < traces.length; i++) {
      const [before, after] = expectedAcc[i];
      expect(traces[i].accumulatorBefore).toBe(before);
      expect(traces[i].accumulatorAfter).toBe(after);
    }
  });
});
