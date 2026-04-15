/**
 * Tests for the Intel 4004 simulator -- all 46 instructions.
 *
 * These tests verify each instruction independently, then test them together
 * in integration programs. The key constraint tested throughout: all values
 * are 4 bits (0-15), enforced by masking with & 0xF.
 *
 * Organization:
 *   1. Basic instructions (NOP, HLT, LDM, LD, XCH, INC)
 *   2. Arithmetic (ADD, SUB) with carry semantics
 *   3. Jump instructions (JUN, JCN, JIN, ISZ)
 *   4. Subroutine instructions (JMS, BBL)
 *   5. Register pair instructions (FIM, SRC, FIN)
 *   6. I/O instructions (WRM, RDM, WR0-WR3, RD0-RD3, etc.)
 *   7. Accumulator operations (CLB, CLC, IAC, CMC, CMA, RAL, RAR, etc.)
 *   8. Integration tests (end-to-end programs)
 */

import { describe, it, expect } from "vitest";
import { Intel4004Simulator } from "../src/simulator.js";
import type { Intel4004State } from "../src/state.js";
import type { Simulator } from "@coding-adventures/simulator-protocol";

// =========================================================================
// 1. Basic instructions
// =========================================================================

// ---------------------------------------------------------------------------
// NOP -- No operation
// ---------------------------------------------------------------------------

describe("TestNOP", () => {
  it("nop_does_nothing", () => {
    /** NOP (0x00) should advance PC without changing state. */
    const sim = new Intel4004Simulator();
    const traces = sim.run(new Uint8Array([0x00, 0x01]));
    expect(sim.accumulator).toBe(0);
    expect(sim.carry).toBe(false);
    expect(traces[0].mnemonic).toBe("NOP");
    expect(traces[0].address).toBe(0);
    expect(traces[1].mnemonic).toBe("HLT");
  });

  it("multiple_nops", () => {
    const sim = new Intel4004Simulator();
    const traces = sim.run(new Uint8Array([0x00, 0x00, 0x00, 0x01]));
    expect(traces.length).toBe(4);
    expect(traces[0].mnemonic).toBe("NOP");
    expect(traces[1].mnemonic).toBe("NOP");
    expect(traces[2].mnemonic).toBe("NOP");
  });
});

// ---------------------------------------------------------------------------
// HLT -- Halt execution
// ---------------------------------------------------------------------------

describe("TestHLT", () => {
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
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0x01]));
    expect(() => sim.step()).toThrow(/halted/);
  });
});

// ---------------------------------------------------------------------------
// LDM -- Load immediate into accumulator
// ---------------------------------------------------------------------------

describe("TestLDM", () => {
  it("ldm_sets_accumulator", () => {
    const sim = new Intel4004Simulator();
    const traces = sim.run(new Uint8Array([0xd5, 0x01]));
    expect(sim.accumulator).toBe(5);
    expect(traces[0].mnemonic).toBe("LDM 5");
    expect(traces[0].accumulatorBefore).toBe(0);
    expect(traces[0].accumulatorAfter).toBe(5);
  });

  it("ldm_zero", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd0, 0x01]));
    expect(sim.accumulator).toBe(0);
  });

  it("ldm_max_value", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xdf, 0x01]));
    expect(sim.accumulator).toBe(15);
  });
});

// ---------------------------------------------------------------------------
// LD -- Load register into accumulator
// ---------------------------------------------------------------------------

describe("TestLD", () => {
  it("ld_copies_register_to_accumulator", () => {
    /** LD R0: copy R0 into A (one-way, R0 keeps its value). */
    const sim = new Intel4004Simulator();
    // LDM 7, XCH R0, LDM 0, LD R0, HLT
    const traces = sim.run(new Uint8Array([0xd7, 0xb0, 0xd0, 0xa0, 0x01]));
    expect(sim.accumulator).toBe(7);
    expect(sim.registers[0]).toBe(7); // R0 unchanged
    expect(traces[3].mnemonic).toBe("LD R0");
  });

  it("ld_all_registers", () => {
    /** LD should work with every register R0-R15. */
    const sim = new Intel4004Simulator();
    // Load 5 into R5, then LD R5
    sim.run(new Uint8Array([0xd5, 0xb5, 0xd0, 0xa5, 0x01]));
    expect(sim.accumulator).toBe(5);
    expect(sim.registers[5]).toBe(5);
  });
});

// ---------------------------------------------------------------------------
// XCH -- Exchange accumulator with register
// ---------------------------------------------------------------------------

describe("TestXCH", () => {
  it("xch_swaps_values", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd7, 0xb0, 0x01]));
    expect(sim.accumulator).toBe(0);
    expect(sim.registers[0]).toBe(7);
  });

  it("xch_is_symmetric", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd3, 0xb5, 0xb5, 0x01]));
    expect(sim.accumulator).toBe(3);
    expect(sim.registers[5]).toBe(0);
  });

  it("xch_high_register", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd9, 0xbf, 0x01]));
    expect(sim.registers[15]).toBe(9);
    expect(sim.accumulator).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// INC -- Increment register
// ---------------------------------------------------------------------------

describe("TestINC", () => {
  it("inc_basic", () => {
    /** INC R0: increment R0 from 0 to 1. */
    const sim = new Intel4004Simulator();
    // LDM 5, XCH R0, INC R0, HLT
    sim.run(new Uint8Array([0xd5, 0xb0, 0x60, 0x01]));
    expect(sim.registers[0]).toBe(6);
  });

  it("inc_wraps_at_15", () => {
    /** INC on 15 should wrap to 0 (4-bit overflow). */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xdf, 0xb0, 0x60, 0x01]));
    expect(sim.registers[0]).toBe(0);
  });

  it("inc_does_not_affect_carry", () => {
    /** INC should not change the carry flag, even on wrap. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xdf, 0xb0, 0x60, 0x01]));
    expect(sim.carry).toBe(false);
  });

  it("inc_does_not_affect_accumulator", () => {
    /** INC modifies a register, not the accumulator. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd3, 0xb0, 0xd0, 0x60, 0x01]));
    expect(sim.accumulator).toBe(0);
    expect(sim.registers[0]).toBe(4);
  });
});

// =========================================================================
// 2. Arithmetic
// =========================================================================

// ---------------------------------------------------------------------------
// ADD -- Add register to accumulator
// ---------------------------------------------------------------------------

describe("TestADD", () => {
  it("add_basic", () => {
    /** 2 + 3 = 5, no carry. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd3, 0xb0, 0xd2, 0x80, 0x01]));
    expect(sim.accumulator).toBe(5);
    expect(sim.carry).toBe(false);
  });

  it("add_carry_on_overflow", () => {
    /** 15 + 1 = 0 with carry (carry is initially false, so no carry-in). */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd1, 0xb0, 0xdf, 0x80, 0x01]));
    expect(sim.accumulator).toBe(0);
    expect(sim.carry).toBe(true);
  });

  it("add_no_carry_at_boundary", () => {
    /** 8 + 7 = 15, no carry. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd7, 0xb0, 0xd8, 0x80, 0x01]));
    expect(sim.accumulator).toBe(15);
    expect(sim.carry).toBe(false);
  });

  it("add_both_max", () => {
    /** 15 + 15 = 14 with carry (30 = 0x1E, masked to 0xE). */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xdf, 0xb0, 0xdf, 0x80, 0x01]));
    expect(sim.accumulator).toBe(14);
    expect(sim.carry).toBe(true);
  });

  it("add_includes_carry_flag", () => {
    /** ADD includes carry: after overflow, next ADD includes carry=1.
     *  15 + 1 = 0 with carry, then 0 + 0 + carry = 1. */
    const sim = new Intel4004Simulator();
    // LDM 1, XCH R0, LDM 0, XCH R1, LDM 15, ADD R0, ADD R1, HLT
    sim.run(
      new Uint8Array([0xd1, 0xb0, 0xd0, 0xb1, 0xdf, 0x80, 0x81, 0x01])
    );
    // After ADD R0: 15+1+0=16, A=0, carry=true
    // After ADD R1: 0+0+1=1, A=1, carry=false
    expect(sim.accumulator).toBe(1);
    expect(sim.carry).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// SUB -- Subtract register from accumulator (complement-add)
// ---------------------------------------------------------------------------

describe("TestSUB", () => {
  it("sub_basic_no_borrow", () => {
    /** 5 - 3 = 2. Carry=true means no borrow. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd3, 0xb0, 0xd5, 0x90, 0x01]));
    expect(sim.accumulator).toBe(2);
    // carry=true means NO borrow (result was >= 0)
    expect(sim.carry).toBe(true);
  });

  it("sub_borrow_on_underflow", () => {
    /** 0 - 1 = 15 with borrow (carry=false).
     *  Complement-add: 0 + (~1 & 0xF) + 1 = 0 + 14 + 1 = 15. Not > 15, so carry=false. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd1, 0xb0, 0xd0, 0x90, 0x01]));
    expect(sim.accumulator).toBe(15);
    expect(sim.carry).toBe(false);
  });

  it("sub_equal_values", () => {
    /** 7 - 7 = 0. Carry=true (no borrow).
     *  Complement-add: 7 + (~7 & 0xF) + 1 = 7 + 8 + 1 = 16 > 15, carry=true.
     *  A = 16 & 0xF = 0. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd7, 0xb0, 0xd7, 0x90, 0x01]));
    expect(sim.accumulator).toBe(0);
    expect(sim.carry).toBe(true);
  });

  it("sub_wraps_to_4_bits", () => {
    /** 3 - 5 = 14. Carry=false (borrow occurred).
     *  Complement-add: 3 + (~5 & 0xF) + 1 = 3 + 10 + 1 = 14. Not > 15, carry=false. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd5, 0xb0, 0xd3, 0x90, 0x01]));
    expect(sim.accumulator).toBe(14);
    expect(sim.carry).toBe(false);
  });

  it("sub_with_existing_carry", () => {
    /** SUB uses inverse carry as borrow-in. If carry is already true
     *  (from a prior overflow), borrow_in=0, so the subtraction is off by 1.
     *  This tests: STC (set carry), then SUB. */
    const sim = new Intel4004Simulator();
    // LDM 3, XCH R0, LDM 5, STC, SUB R0, HLT
    // With carry=true: 5 + (~3 & 0xF) + 0 = 5 + 12 + 0 = 17 > 15, carry=true, A=1
    sim.run(new Uint8Array([0xd3, 0xb0, 0xd5, 0xfa, 0x90, 0x01]));
    expect(sim.accumulator).toBe(1);
    expect(sim.carry).toBe(true);
  });
});

// =========================================================================
// 3. Jump instructions
// =========================================================================

// ---------------------------------------------------------------------------
// JUN -- Unconditional jump
// ---------------------------------------------------------------------------

describe("TestJUN", () => {
  it("jun_jumps_forward", () => {
    /** JUN should jump to the target address. */
    const sim = new Intel4004Simulator();
    // JUN 0x004, NOP (skipped), NOP (skipped), NOP (skipped), LDM 7, HLT
    sim.run(new Uint8Array([0x40, 0x04, 0x00, 0x00, 0xd7, 0x01]));
    expect(sim.accumulator).toBe(7);
  });

  it("jun_12bit_address", () => {
    /** JUN can address 12-bit targets (0x000-0xFFF). */
    const sim = new Intel4004Simulator();
    // The lower nibble of byte 1 is the high nibble of the address
    // 0x40 0x03 = JUN 0x003
    const traces = sim.run(new Uint8Array([0x40, 0x03, 0x00, 0xd5, 0x01]));
    expect(sim.accumulator).toBe(5);
    expect(traces[0].mnemonic).toBe("JUN 0x003");
  });
});

// ---------------------------------------------------------------------------
// JCN -- Conditional jump
// ---------------------------------------------------------------------------

describe("TestJCN", () => {
  it("jcn_test_zero_taken", () => {
    /** JCN with condition 4 (test A==0): when A=0, jump is taken. */
    const sim = new Intel4004Simulator();
    // A=0, JCN 4,0x04, NOP (skipped), NOP (skipped), LDM 9, HLT
    sim.run(new Uint8Array([0x14, 0x04, 0x00, 0x00, 0xd9, 0x01]));
    expect(sim.accumulator).toBe(9);
  });

  it("jcn_test_zero_not_taken", () => {
    /** JCN with condition 4 (test A==0): when A!=0, jump is not taken. */
    const sim = new Intel4004Simulator();
    // LDM 5, JCN 4,0x06, LDM 3, HLT, LDM 9, HLT
    sim.run(new Uint8Array([0xd5, 0x14, 0x06, 0xd3, 0x01, 0xd9, 0x01]));
    expect(sim.accumulator).toBe(3);
  });

  it("jcn_test_carry_taken", () => {
    /** JCN with condition 2 (test carry): when carry=true, jump. */
    const sim = new Intel4004Simulator();
    // STC (set carry), JCN 2,0x06, LDM 1, HLT, LDM 9, HLT
    sim.run(new Uint8Array([0xfa, 0x12, 0x06, 0xd1, 0x01, 0x00, 0xd9, 0x01]));
    expect(sim.accumulator).toBe(9);
  });

  it("jcn_invert_bit", () => {
    /** JCN with bit 3 set inverts the test. Condition 0xC = invert + test_zero.
     *  When A!=0, inverted test_zero is true, so jump is taken. */
    const sim = new Intel4004Simulator();
    // LDM 5, JCN 0xC,0x06, LDM 1, HLT, LDM 9, HLT
    sim.run(new Uint8Array([0xd5, 0x1c, 0x06, 0xd1, 0x01, 0x00, 0xd9, 0x01]));
    expect(sim.accumulator).toBe(9);
  });

  it("jcn_invert_unconditional", () => {
    /** JCN with condition 8 (invert only, no tests): inverted false = true.
     *  Always jumps. */
    const sim = new Intel4004Simulator();
    // JCN 8,0x04, LDM 1, HLT, LDM 9, HLT
    sim.run(new Uint8Array([0x18, 0x04, 0xd1, 0x01, 0xd9, 0x01]));
    expect(sim.accumulator).toBe(9);
  });

  it("jcn_or_multiple_conditions", () => {
    /** JCN with condition 6 (test_zero | test_carry): if either is true, jump.
     *  Here carry=true but A!=0, so jump is taken via carry test. */
    const sim = new Intel4004Simulator();
    // LDM 5, STC, JCN 6,0x07, LDM 1, HLT, LDM 9, HLT
    sim.run(
      new Uint8Array([0xd5, 0xfa, 0x16, 0x07, 0xd1, 0x01, 0x00, 0xd9, 0x01])
    );
    expect(sim.accumulator).toBe(9);
  });
});

// ---------------------------------------------------------------------------
// JIN -- Jump indirect
// ---------------------------------------------------------------------------

describe("TestJIN", () => {
  it("jin_jumps_via_pair", () => {
    /** JIN P1: jump to address formed by current page + pair 1 value. */
    const sim = new Intel4004Simulator();
    // FIM P1,0x05 (R2=0, R3=5), JIN P1, NOP, LDM 7, HLT
    sim.run(new Uint8Array([0x22, 0x05, 0x33, 0x00, 0x00, 0xd7, 0x01]));
    expect(sim.accumulator).toBe(7);
  });
});

// ---------------------------------------------------------------------------
// ISZ -- Increment and skip if zero
// ---------------------------------------------------------------------------

describe("TestISZ", () => {
  it("isz_loops_until_zero", () => {
    /** ISZ loops: start R0=14, increment to 15, jump back.
     *  Then increment 15 to 0, fall through. */
    const sim = new Intel4004Simulator();
    // LDM 14, XCH R0, LDM 0, ISZ R0,0x04, LDM 5, HLT
    //   addr 0: LDM 14 (0xDE)
    //   addr 1: XCH R0 (0xB0)
    //   addr 2: LDM 0  (0xD0)
    //   addr 3: NOP to align
    //   addr 4: ISZ R0,0x04 (0x70, 0x04) -- loops back to addr 4
    //   addr 6: LDM 5  (0xD5)
    //   addr 7: HLT
    sim.run(new Uint8Array([0xde, 0xb0, 0xd0, 0x00, 0x70, 0x04, 0xd5, 0x01]));
    expect(sim.registers[0]).toBe(0);
    expect(sim.accumulator).toBe(5);
  });

  it("isz_does_not_jump_when_zero", () => {
    /** ISZ when register wraps to 0: falls through. */
    const sim = new Intel4004Simulator();
    // LDM 15, XCH R0, ISZ R0,0x06, LDM 3, HLT, LDM 9, HLT
    sim.run(new Uint8Array([0xdf, 0xb0, 0x70, 0x07, 0xd3, 0x01, 0x00, 0xd9, 0x01]));
    expect(sim.accumulator).toBe(3);
    expect(sim.registers[0]).toBe(0);
  });

  it("isz_jumps_when_not_zero", () => {
    /** ISZ when register != 0 after increment: jumps. */
    const sim = new Intel4004Simulator();
    // LDM 5, XCH R0, ISZ R0,0x06, LDM 3, HLT, LDM 9, HLT
    sim.run(new Uint8Array([0xd5, 0xb0, 0x70, 0x06, 0xd3, 0x01, 0xd9, 0x01]));
    expect(sim.accumulator).toBe(9);
    expect(sim.registers[0]).toBe(6);
  });
});

// =========================================================================
// 4. Subroutine instructions
// =========================================================================

// ---------------------------------------------------------------------------
// JMS and BBL -- Call and return
// ---------------------------------------------------------------------------

describe("TestJMS_BBL", () => {
  it("jms_bbl_basic", () => {
    /** JMS calls a subroutine, BBL returns with a value in A. */
    const sim = new Intel4004Simulator();
    // addr 0: JMS 0x004 (0x50, 0x04)
    // addr 2: HLT (0x01) -- return point
    // addr 3: NOP
    // addr 4: BBL 7 (0xC7) -- subroutine: return with A=7
    sim.run(new Uint8Array([0x50, 0x04, 0x01, 0x00, 0xc7]));
    expect(sim.accumulator).toBe(7);
    expect(sim.halted).toBe(true);
  });

  it("jms_pushes_return_address", () => {
    /** After JMS, the stack should contain the return address (addr+2). */
    const sim = new Intel4004Simulator();
    // addr 0: JMS 0x004
    // addr 2: HLT
    // addr 3: NOP
    // addr 4: LDM 5, HLT (subroutine doesn't return)
    sim.run(new Uint8Array([0x50, 0x04, 0x01, 0x00, 0xd5, 0x01]));
    expect(sim.accumulator).toBe(5);
    // Stack should have been pushed with return addr 2
    // After JMS, stackPointer advanced to 1
  });

  it("nested_jms_bbl", () => {
    /** Nested subroutine calls use the 3-level stack. */
    const sim = new Intel4004Simulator();
    // addr 0: JMS 0x004 (outer call)
    // addr 2: HLT
    // addr 3: NOP
    // addr 4: JMS 0x008 (inner call from subroutine)
    // addr 6: BBL 3 (return from outer)
    // addr 7: NOP
    // addr 8: BBL 5 (return from inner, A=5)
    sim.run(
      new Uint8Array([0x50, 0x04, 0x01, 0x00, 0x50, 0x08, 0xc3, 0x00, 0xc5])
    );
    // Inner BBL sets A=5, returns to addr 6
    // Outer BBL sets A=3, returns to addr 2
    expect(sim.accumulator).toBe(3);
    expect(sim.halted).toBe(true);
  });

  it("stack_wraps_mod_3", () => {
    /** The 4004 stack wraps mod 3 -- the 4th push overwrites the first. */
    const sim = new Intel4004Simulator();
    // We just verify that the stackPointer wraps
    sim.loadProgram(new Uint8Array([0x01]));
    // Manually push 3 times and check wrap
    (sim as any)._stackPush(0x100);
    (sim as any)._stackPush(0x200);
    (sim as any)._stackPush(0x300);
    expect(sim.stackPointer).toBe(0); // wrapped back to 0
    // 4th push overwrites slot 0
    (sim as any)._stackPush(0x400);
    expect(sim.stackPointer).toBe(1);
    // Pop should get 0x300 (slot 0 was overwritten with 0x400)
    const addr = (sim as any)._stackPop();
    expect(addr).toBe(0x400);
  });
});

// =========================================================================
// 5. Register pair instructions
// =========================================================================

// ---------------------------------------------------------------------------
// FIM -- Fetch immediate to register pair
// ---------------------------------------------------------------------------

describe("TestFIM", () => {
  it("fim_loads_pair", () => {
    /** FIM P0,0xAB: R0=0xA, R1=0xB. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0x20, 0xab, 0x01]));
    expect(sim.registers[0]).toBe(0xa);
    expect(sim.registers[1]).toBe(0xb);
  });

  it("fim_pair_1", () => {
    /** FIM P1,0x34: R2=3, R3=4. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0x22, 0x34, 0x01]));
    expect(sim.registers[2]).toBe(3);
    expect(sim.registers[3]).toBe(4);
  });

  it("fim_all_pairs", () => {
    /** FIM should work with all 8 register pairs. */
    const sim = new Intel4004Simulator();
    // FIM P7,0xFF: R14=0xF, R15=0xF
    sim.run(new Uint8Array([0x2e, 0xff, 0x01]));
    expect(sim.registers[14]).toBe(0xf);
    expect(sim.registers[15]).toBe(0xf);
  });
});

// ---------------------------------------------------------------------------
// SRC -- Send register control
// ---------------------------------------------------------------------------

describe("TestSRC", () => {
  it("src_sets_ram_address", () => {
    /** SRC P0: use pair 0 value as RAM address. */
    const sim = new Intel4004Simulator();
    // FIM P0,0x35 (R0=3, R1=5), SRC P0, HLT
    sim.run(new Uint8Array([0x20, 0x35, 0x21, 0x01]));
    expect(sim.ramRegister).toBe(3);
    expect(sim.ramCharacter).toBe(5);
  });
});

// ---------------------------------------------------------------------------
// FIN -- Fetch indirect from ROM
// ---------------------------------------------------------------------------

describe("TestFIN", () => {
  it("fin_reads_rom_via_p0", () => {
    /** FIN P1: read ROM[page | P0] into pair 1.
     *  FIN P1 encoding: 0x32 (upper=3, lower=2, even so FIN, pair=2>>1=1). */
    const sim = new Intel4004Simulator();
    // addr 0: FIM P0,0x06 -- set P0 (R0:R1) to point to addr 6
    // addr 2: FIN P1 (0x32) -- read ROM[0x06] into P1
    // addr 3: HLT
    // addr 4: NOP
    // addr 5: NOP
    // addr 6: 0xAB -- data byte to be read
    sim.run(new Uint8Array([0x20, 0x06, 0x32, 0x01, 0x00, 0x00, 0xab]));
    expect(sim.registers[2]).toBe(0xa); // P1 high = R2
    expect(sim.registers[3]).toBe(0xb); // P1 low = R3
  });
});

// =========================================================================
// 6. I/O instructions
// =========================================================================

// ---------------------------------------------------------------------------
// WRM / RDM -- Write/Read RAM main
// ---------------------------------------------------------------------------

describe("TestWRM_RDM", () => {
  it("wrm_rdm_roundtrip", () => {
    /** Write A to RAM, read it back. */
    const sim = new Intel4004Simulator();
    // FIM P0,0x00 (addr: reg=0, char=0), SRC P0, LDM 7, WRM, LDM 0, RDM, HLT
    sim.run(
      new Uint8Array([0x20, 0x00, 0x21, 0xd7, 0xe0, 0xd0, 0xe9, 0x01])
    );
    expect(sim.accumulator).toBe(7);
  });

  it("wrm_rdm_different_addresses", () => {
    /** Write to different RAM addresses and verify isolation. */
    const sim = new Intel4004Simulator();
    // FIM P0,0x00, SRC P0, LDM 5, WRM  -- write 5 to [0][0]
    // FIM P0,0x01, SRC P0, LDM 9, WRM  -- write 9 to [0][1]
    // FIM P0,0x00, SRC P0, RDM, HLT    -- read back [0][0]
    sim.run(
      new Uint8Array([
        0x20, 0x00, 0x21, 0xd5, 0xe0,
        0x20, 0x01, 0x21, 0xd9, 0xe0,
        0x20, 0x00, 0x21, 0xe9, 0x01,
      ])
    );
    expect(sim.accumulator).toBe(5);
  });
});

// ---------------------------------------------------------------------------
// WMP -- Write RAM output port
// ---------------------------------------------------------------------------

describe("TestWMP", () => {
  it("wmp_writes_output", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd7, 0xe1, 0x01]));
    expect(sim.ramOutput[0]).toBe(7);
  });
});

// ---------------------------------------------------------------------------
// WRR / RDR -- Write/Read ROM I/O port
// ---------------------------------------------------------------------------

describe("TestWRR_RDR", () => {
  it("wrr_rdr_roundtrip", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd5, 0xe2, 0xd0, 0xea, 0x01]));
    expect(sim.accumulator).toBe(5);
    expect(sim.romPort).toBe(5);
  });
});

// ---------------------------------------------------------------------------
// WPM -- Write program RAM (NOP in simulator)
// ---------------------------------------------------------------------------

describe("TestWPM", () => {
  it("wpm_is_nop", () => {
    const sim = new Intel4004Simulator();
    const traces = sim.run(new Uint8Array([0xe3, 0x01]));
    expect(traces[0].mnemonic).toBe("WPM");
  });
});

// ---------------------------------------------------------------------------
// WR0-WR3 / RD0-RD3 -- Write/Read RAM status
// ---------------------------------------------------------------------------

describe("TestStatusRegisters", () => {
  it("wr0_rd0_roundtrip", () => {
    const sim = new Intel4004Simulator();
    // FIM P0,0x00, SRC P0, LDM 3, WR0, LDM 0, RD0, HLT
    sim.run(
      new Uint8Array([0x20, 0x00, 0x21, 0xd3, 0xe4, 0xd0, 0xec, 0x01])
    );
    expect(sim.accumulator).toBe(3);
  });

  it("wr1_rd1_roundtrip", () => {
    const sim = new Intel4004Simulator();
    sim.run(
      new Uint8Array([0x20, 0x00, 0x21, 0xd5, 0xe5, 0xd0, 0xed, 0x01])
    );
    expect(sim.accumulator).toBe(5);
  });

  it("wr2_rd2_roundtrip", () => {
    const sim = new Intel4004Simulator();
    sim.run(
      new Uint8Array([0x20, 0x00, 0x21, 0xd9, 0xe6, 0xd0, 0xee, 0x01])
    );
    expect(sim.accumulator).toBe(9);
  });

  it("wr3_rd3_roundtrip", () => {
    const sim = new Intel4004Simulator();
    sim.run(
      new Uint8Array([0x20, 0x00, 0x21, 0xdf, 0xe7, 0xd0, 0xef, 0x01])
    );
    expect(sim.accumulator).toBe(15);
  });

  it("status_registers_are_independent", () => {
    /** Each status register (0-3) should be independent. */
    const sim = new Intel4004Simulator();
    // FIM P0,0x00, SRC P0
    // LDM 1, WR0, LDM 2, WR1, LDM 3, WR2, LDM 4, WR3
    // RD0, XCH R4, RD1, XCH R5, RD2, XCH R6, RD3, HLT
    sim.run(
      new Uint8Array([
        0x20, 0x00, 0x21,
        0xd1, 0xe4, 0xd2, 0xe5, 0xd3, 0xe6, 0xd4, 0xe7,
        0xec, 0xb4, 0xed, 0xb5, 0xee, 0xb6, 0xef, 0x01,
      ])
    );
    expect(sim.registers[4]).toBe(1);
    expect(sim.registers[5]).toBe(2);
    expect(sim.registers[6]).toBe(3);
    expect(sim.accumulator).toBe(4);
  });
});

// ---------------------------------------------------------------------------
// SBM -- Subtract RAM from accumulator
// ---------------------------------------------------------------------------

describe("TestSBM", () => {
  it("sbm_subtracts_ram", () => {
    /** SBM: A = A + ~RAM + (carry ? 0 : 1). Same as SUB but with RAM. */
    const sim = new Intel4004Simulator();
    // FIM P0,0x00, SRC P0, LDM 3, WRM, LDM 5, SBM, HLT
    // 5 - 3 = 2, carry=true (no borrow)
    sim.run(
      new Uint8Array([0x20, 0x00, 0x21, 0xd3, 0xe0, 0xd5, 0xe8, 0x01])
    );
    expect(sim.accumulator).toBe(2);
    expect(sim.carry).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// ADM -- Add RAM to accumulator
// ---------------------------------------------------------------------------

describe("TestADM", () => {
  it("adm_adds_ram", () => {
    /** ADM: A = A + RAM + carry. Same as ADD but with RAM. */
    const sim = new Intel4004Simulator();
    // FIM P0,0x00, SRC P0, LDM 3, WRM, LDM 5, ADM, HLT
    sim.run(
      new Uint8Array([0x20, 0x00, 0x21, 0xd3, 0xe0, 0xd5, 0xeb, 0x01])
    );
    expect(sim.accumulator).toBe(8);
    expect(sim.carry).toBe(false);
  });
});

// =========================================================================
// 7. Accumulator operations
// =========================================================================

// ---------------------------------------------------------------------------
// CLB -- Clear both
// ---------------------------------------------------------------------------

describe("TestCLB", () => {
  it("clb_clears_both", () => {
    const sim = new Intel4004Simulator();
    // LDM 5, STC, CLB, HLT
    sim.run(new Uint8Array([0xd5, 0xfa, 0xf0, 0x01]));
    expect(sim.accumulator).toBe(0);
    expect(sim.carry).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// CLC -- Clear carry
// ---------------------------------------------------------------------------

describe("TestCLC", () => {
  it("clc_clears_carry", () => {
    const sim = new Intel4004Simulator();
    // STC, CLC, HLT
    sim.run(new Uint8Array([0xfa, 0xf1, 0x01]));
    expect(sim.carry).toBe(false);
  });

  it("clc_preserves_accumulator", () => {
    const sim = new Intel4004Simulator();
    // LDM 7, STC, CLC, HLT
    sim.run(new Uint8Array([0xd7, 0xfa, 0xf1, 0x01]));
    expect(sim.accumulator).toBe(7);
    expect(sim.carry).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// IAC -- Increment accumulator
// ---------------------------------------------------------------------------

describe("TestIAC", () => {
  it("iac_increments", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd5, 0xf2, 0x01]));
    expect(sim.accumulator).toBe(6);
    expect(sim.carry).toBe(false);
  });

  it("iac_wraps_and_sets_carry", () => {
    /** IAC on 15 wraps to 0 and sets carry. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xdf, 0xf2, 0x01]));
    expect(sim.accumulator).toBe(0);
    expect(sim.carry).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// CMC -- Complement carry
// ---------------------------------------------------------------------------

describe("TestCMC", () => {
  it("cmc_toggles_carry", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xf3, 0x01])); // CMC when carry=false -> true
    expect(sim.carry).toBe(true);
  });

  it("cmc_toggles_back", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xfa, 0xf3, 0x01])); // STC, CMC -> false
    expect(sim.carry).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// CMA -- Complement accumulator
// ---------------------------------------------------------------------------

describe("TestCMA", () => {
  it("cma_complements", () => {
    /** CMA: A = ~A & 0xF. 5 (0101) -> 10 (1010). */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd5, 0xf4, 0x01]));
    expect(sim.accumulator).toBe(10);
  });

  it("cma_zero_to_fifteen", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd0, 0xf4, 0x01]));
    expect(sim.accumulator).toBe(15);
  });

  it("cma_fifteen_to_zero", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xdf, 0xf4, 0x01]));
    expect(sim.accumulator).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// RAL -- Rotate left through carry
// ---------------------------------------------------------------------------

describe("TestRAL", () => {
  it("ral_basic", () => {
    /** RAL: A=0001 (1), carry=0. After: A=0010 (2), carry=0. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd1, 0xf5, 0x01]));
    expect(sim.accumulator).toBe(2);
    expect(sim.carry).toBe(false);
  });

  it("ral_high_bit_to_carry", () => {
    /** RAL: A=1000 (8), carry=0. After: A=0000 (0), carry=1. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd8, 0xf5, 0x01]));
    expect(sim.accumulator).toBe(0);
    expect(sim.carry).toBe(true);
  });

  it("ral_carry_to_low_bit", () => {
    /** RAL: A=0000 (0), carry=1. After: A=0001 (1), carry=0. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd0, 0xfa, 0xf5, 0x01])); // LDM 0, STC, RAL
    expect(sim.accumulator).toBe(1);
    expect(sim.carry).toBe(false);
  });

  it("ral_full_rotation", () => {
    /** RAL: A=1010 (10), carry=1. After: A=0101 (5), carry=1. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xda, 0xfa, 0xf5, 0x01])); // LDM 10, STC, RAL
    expect(sim.accumulator).toBe(5);
    expect(sim.carry).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// RAR -- Rotate right through carry
// ---------------------------------------------------------------------------

describe("TestRAR", () => {
  it("rar_basic", () => {
    /** RAR: A=0010 (2), carry=0. After: A=0001 (1), carry=0. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd2, 0xf6, 0x01]));
    expect(sim.accumulator).toBe(1);
    expect(sim.carry).toBe(false);
  });

  it("rar_low_bit_to_carry", () => {
    /** RAR: A=0001 (1), carry=0. After: A=0000 (0), carry=1. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd1, 0xf6, 0x01]));
    expect(sim.accumulator).toBe(0);
    expect(sim.carry).toBe(true);
  });

  it("rar_carry_to_high_bit", () => {
    /** RAR: A=0000 (0), carry=1. After: A=1000 (8), carry=0. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd0, 0xfa, 0xf6, 0x01])); // LDM 0, STC, RAR
    expect(sim.accumulator).toBe(8);
    expect(sim.carry).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// TCC -- Transfer carry to accumulator
// ---------------------------------------------------------------------------

describe("TestTCC", () => {
  it("tcc_carry_set", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xfa, 0xf7, 0x01])); // STC, TCC
    expect(sim.accumulator).toBe(1);
    expect(sim.carry).toBe(false);
  });

  it("tcc_carry_clear", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xf7, 0x01])); // TCC
    expect(sim.accumulator).toBe(0);
    expect(sim.carry).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// DAC -- Decrement accumulator
// ---------------------------------------------------------------------------

describe("TestDAC", () => {
  it("dac_decrements", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd5, 0xf8, 0x01])); // LDM 5, DAC
    expect(sim.accumulator).toBe(4);
    expect(sim.carry).toBe(true); // no borrow
  });

  it("dac_wraps_and_clears_carry", () => {
    /** DAC on 0: wraps to 15, carry=false (borrow occurred). */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd0, 0xf8, 0x01])); // LDM 0, DAC
    expect(sim.accumulator).toBe(15);
    expect(sim.carry).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// TCS -- Transfer carry subtract
// ---------------------------------------------------------------------------

describe("TestTCS", () => {
  it("tcs_carry_set", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xfa, 0xf9, 0x01])); // STC, TCS
    expect(sim.accumulator).toBe(10);
    expect(sim.carry).toBe(false);
  });

  it("tcs_carry_clear", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xf9, 0x01])); // TCS
    expect(sim.accumulator).toBe(9);
    expect(sim.carry).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// STC -- Set carry
// ---------------------------------------------------------------------------

describe("TestSTC", () => {
  it("stc_sets_carry", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xfa, 0x01])); // STC, HLT
    expect(sim.carry).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// DAA -- Decimal adjust accumulator
// ---------------------------------------------------------------------------

describe("TestDAA", () => {
  it("daa_no_adjustment_needed", () => {
    /** DAA with A=5, carry=false: no change (5 <= 9). */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd5, 0xfb, 0x01]));
    expect(sim.accumulator).toBe(5);
    expect(sim.carry).toBe(false);
  });

  it("daa_adjusts_high_digit", () => {
    /** DAA with A=12, carry=false: add 6 -> 18 & 0xF = 2, carry=true. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xdc, 0xfb, 0x01]));
    expect(sim.accumulator).toBe(2);
    expect(sim.carry).toBe(true);
  });

  it("daa_with_carry", () => {
    /** DAA with A=5, carry=true: add 6 -> 11, carry stays true? No -- 11 <= 15. */
    const sim = new Intel4004Simulator();
    // STC, LDM 5 clears carry... need to set carry after LDM
    // LDM 5, STC, DAA, HLT
    sim.run(new Uint8Array([0xd5, 0xfa, 0xfb, 0x01]));
    expect(sim.accumulator).toBe(11);
    // carry was true, DAA condition met, result 11 <= 15, so carry set only if result > 15
    // But the Python code only sets carry to true if result > 0xF, it doesn't clear it
    // So carry remains true from STC
    expect(sim.carry).toBe(true);
  });

  it("daa_9_with_carry", () => {
    /** DAA with A=9, carry=true: add 6 -> 15. carry stays true (not cleared). */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd9, 0xfa, 0xfb, 0x01]));
    expect(sim.accumulator).toBe(15);
    expect(sim.carry).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// KBP -- Keyboard process
// ---------------------------------------------------------------------------

describe("TestKBP", () => {
  it("kbp_no_key", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd0, 0xfc, 0x01]));
    expect(sim.accumulator).toBe(0);
  });

  it("kbp_key_1", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd1, 0xfc, 0x01]));
    expect(sim.accumulator).toBe(1);
  });

  it("kbp_key_2", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd2, 0xfc, 0x01]));
    expect(sim.accumulator).toBe(2);
  });

  it("kbp_key_3", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd4, 0xfc, 0x01]));
    expect(sim.accumulator).toBe(3);
  });

  it("kbp_key_4", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd8, 0xfc, 0x01]));
    expect(sim.accumulator).toBe(4);
  });

  it("kbp_multiple_keys_error", () => {
    /** Multiple keys pressed -> 15 (error). */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd3, 0xfc, 0x01])); // A=3 (0011, two bits set)
    expect(sim.accumulator).toBe(15);
  });

  it("kbp_all_invalid_values_return_15", () => {
    /** Any value not in {0,1,2,4,8} maps to 15. */
    const invalidValues = [3, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15];
    for (const v of invalidValues) {
      const sim = new Intel4004Simulator();
      sim.run(new Uint8Array([0xd0 | v, 0xfc, 0x01]));
      expect(sim.accumulator).toBe(15);
    }
  });
});

// ---------------------------------------------------------------------------
// DCL -- Designate command line (select RAM bank)
// ---------------------------------------------------------------------------

describe("TestDCL", () => {
  it("dcl_selects_bank", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd2, 0xfd, 0x01])); // LDM 2, DCL
    expect(sim.ramBank).toBe(2);
  });

  it("dcl_masks_to_3_bits", () => {
    /** DCL uses only lower 3 bits of A, clamped to 0-3. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd3, 0xfd, 0x01])); // LDM 3, DCL
    expect(sim.ramBank).toBe(3);
  });
});

// =========================================================================
// 8. Integration tests
// =========================================================================

// ---------------------------------------------------------------------------
// End-to-end: x = 1 + 2
// ---------------------------------------------------------------------------

describe("TestEndToEnd", () => {
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

// ---------------------------------------------------------------------------
// Subroutine with RAM
// ---------------------------------------------------------------------------

describe("TestSubroutineWithRAM", () => {
  it("subroutine_stores_to_ram", () => {
    /** A program that calls a subroutine to store a value in RAM. */
    const sim = new Intel4004Simulator();
    // addr 0: FIM P0,0x00 (set RAM address)
    // addr 2: SRC P0
    // addr 3: LDM 9
    // addr 4: JMS 0x008 (call subroutine at addr 8)
    // addr 6: HLT
    // addr 7: NOP
    // addr 8: WRM (subroutine: write A to RAM)
    // addr 9: BBL 0 (return)
    sim.run(
      new Uint8Array([0x20, 0x00, 0x21, 0xd9, 0x50, 0x08, 0x01, 0x00, 0xe0, 0xc0])
    );
    expect(sim.ram[0][0][0]).toBe(9);
    expect(sim.accumulator).toBe(0); // BBL 0
    expect(sim.halted).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Loop with counter
// ---------------------------------------------------------------------------

describe("TestLoopCounter", () => {
  it("isz_counts_from_13_to_0", () => {
    /** Use ISZ to loop 3 times (R0 starts at 13, wraps at 16->0). */
    const sim = new Intel4004Simulator();
    // addr 0: LDM 13 (0xDD)
    // addr 1: XCH R0 (0xB0) -- R0 = 13
    // addr 2: LDM 0
    // addr 3: NOP
    // addr 4: IAC (A = A + 1 each iteration)
    // addr 5: ISZ R0,0x04 -- loop back to addr 4
    // addr 7: HLT
    sim.run(new Uint8Array([0xdd, 0xb0, 0xd0, 0x00, 0xf2, 0x70, 0x04, 0x01]));
    // R0: 13->14->15->0. Three iterations.
    expect(sim.registers[0]).toBe(0);
    // A incremented 3 times from 0: 1, 2, 3
    expect(sim.accumulator).toBe(3);
  });
});

// ---------------------------------------------------------------------------
// 4-bit masking
// ---------------------------------------------------------------------------

describe("TestFourBitMasking", () => {
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
        0xdf, 0xb0,
        0xda, 0xb1,
        0xd0, 0xb2,
        0x01,
      ])
    );
    for (let i = 0; i < sim.registers.length; i++) {
      expect(sim.registers[i]).toBeGreaterThanOrEqual(0);
      expect(sim.registers[i]).toBeLessThanOrEqual(15);
    }
  });
});

// ---------------------------------------------------------------------------
// BCD addition with DAA
// ---------------------------------------------------------------------------

describe("TestBCDAddition", () => {
  it("bcd_7_plus_8", () => {
    /** BCD addition: 7 + 8 = 15, DAA corrects to 5 with carry (tens digit).
     *  After ADD: A=15, carry=false. After DAA: 15+6=21, A=5, carry=true. */
    const sim = new Intel4004Simulator();
    // LDM 8, XCH R0, LDM 7, ADD R0, DAA, HLT
    sim.run(new Uint8Array([0xd8, 0xb0, 0xd7, 0x80, 0xfb, 0x01]));
    expect(sim.accumulator).toBe(5);
    expect(sim.carry).toBe(true);
  });

  it("bcd_3_plus_4", () => {
    /** BCD: 3 + 4 = 7, no correction needed. */
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd4, 0xb0, 0xd3, 0x80, 0xfb, 0x01]));
    expect(sim.accumulator).toBe(7);
    expect(sim.carry).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Reset
// ---------------------------------------------------------------------------

describe("TestReset", () => {
  it("reset_clears_state", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd5, 0xb0, 0xfa, 0x01]));
    expect(sim.accumulator).toBe(0);
    expect(sim.carry).toBe(true);

    // Run again (run() calls reset internally)
    sim.run(new Uint8Array([0x01]));
    expect(sim.accumulator).toBe(0);
    expect(sim.carry).toBe(false);
    expect(sim.registers[0]).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Trace includes raw2 for 2-byte instructions
// ---------------------------------------------------------------------------

describe("TestTraceRaw2", () => {
  it("two_byte_instruction_has_raw2", () => {
    const sim = new Intel4004Simulator();
    const traces = sim.run(new Uint8Array([0x40, 0x02, 0x01])); // JUN 0x002, HLT
    expect(traces[0].raw).toBe(0x40);
    expect(traces[0].raw2).toBe(0x02);
  });

  it("one_byte_instruction_has_no_raw2", () => {
    const sim = new Intel4004Simulator();
    const traces = sim.run(new Uint8Array([0xd5, 0x01])); // LDM 5, HLT
    expect(traces[0].raw2).toBeUndefined();
  });
});

describe("TestSimulatorProtocol", () => {
  it("supports_structural_protocol_typing", () => {
    const sim: Simulator<Intel4004State> = new Intel4004Simulator();
    const result = sim.execute(new Uint8Array([0xd7, 0x01]));
    expect(result.ok).toBe(true);
    expect(result.finalState.accumulator).toBe(7);
  });

  it("get_state_returns_immutable_snapshot", () => {
    const sim = new Intel4004Simulator();
    sim.run(new Uint8Array([0xd7, 0xb0, 0x01]));

    const state = sim.getState();
    expect(state.registers[0]).toBe(7);
    expect(Object.isFrozen(state)).toBe(true);
    expect(Object.isFrozen(state.registers)).toBe(true);
    expect(Object.isFrozen(state.ram)).toBe(true);
    expect(() => ((state as { accumulator: number }).accumulator = 99)).toThrow();
  });

  it("execute_returns_normalized_traces", () => {
    const sim = new Intel4004Simulator();
    const result = sim.execute(new Uint8Array([0xd1, 0xb0, 0x01]));

    expect(result.ok).toBe(true);
    expect(result.steps).toBe(3);
    expect(result.traces.map((trace) => trace.mnemonic)).toEqual([
      "LDM 1",
      "XCH R0",
      "HLT",
    ]);
    expect(result.finalState.registers[0]).toBe(1);
  });
});
