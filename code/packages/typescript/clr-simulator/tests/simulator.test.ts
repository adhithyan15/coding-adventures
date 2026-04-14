/**
 * Comprehensive tests for the CLR IL Simulator.
 */

import { describe, it, expect, beforeEach } from "vitest";
import type { Simulator } from "@coding-adventures/simulator-protocol";
import {
  CEQ_BYTE,
  CGT_BYTE,
  CLROpcode,
  CLRSimulator,
  CLT_BYTE,
  assembleClr,
  encodeLdcI4,
  encodeLdloc,
  encodeStloc,
} from "../src/simulator.js";
import type { CLRState } from "../src/state.js";

// ===================================================================
// 1. Constant loading: ldc.i4.0 through ldc.i4.8
// ===================================================================

describe("TestLdcI4ShortForms", () => {
  for (let value = 0; value <= 8; value++) {
    it(`ldc_i4_${value}`, () => {
      const sim = new CLRSimulator();
      sim.load(assembleClr(encodeLdcI4(value), [CLROpcode.RET]));
      const traces = sim.run();
      expect(sim.stack).toEqual([value]);
      expect(traces[0].opcode).toBe(`ldc.i4.${value}`);
      expect(traces[0].stackAfter).toEqual([value]);
      expect(traces[0].description).toBe(`push ${value}`);
    });
  }

  it("ldc_i4_0_is_one_byte", () => { expect(encodeLdcI4(0)).toEqual(new Uint8Array([0x16])); expect(encodeLdcI4(0).length).toBe(1); });
  it("ldc_i4_8_is_one_byte", () => { expect(encodeLdcI4(8)).toEqual(new Uint8Array([0x1e])); expect(encodeLdcI4(8).length).toBe(1); });
});

// ===================================================================
// 2. Constant loading: ldc.i4.s
// ===================================================================

describe("TestLdcI4S", () => {
  it("positive", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(42), [CLROpcode.RET])); const t = sim.run(); expect(sim.stack).toEqual([42]); expect(t[0].opcode).toBe("ldc.i4.s"); });
  it("negative", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(-1), [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([-1]); });
  it("min_value", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(-128), [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([-128]); });
  it("max_value", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(127), [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([127]); });
  it("encoding", () => { const e = encodeLdcI4(42); expect(e.length).toBe(2); expect(e[0]).toBe(0x1f); expect(e[1]).toBe(42); });
  it("negative_encoding", () => { const e = encodeLdcI4(-1); expect(e.length).toBe(2); expect(e[0]).toBe(0x1f); expect(e[1]).toBe(0xff); });
});

// ===================================================================
// 3. Constant loading: ldc.i4 (int32)
// ===================================================================

describe("TestLdcI4", () => {
  it("large_positive", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(1000), [CLROpcode.RET])); const t = sim.run(); expect(sim.stack).toEqual([1000]); expect(t[0].opcode).toBe("ldc.i4"); });
  it("large_negative", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(-1000), [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([-1000]); });
  it("max_int32", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(2147483647), [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([2147483647]); });
  it("min_int32", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(-2147483648), [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([-2147483648]); });
  it("encoding", () => { const e = encodeLdcI4(1000); expect(e.length).toBe(5); expect(e[0]).toBe(0x20); });
  it("value_128", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(128), [CLROpcode.RET])); const t = sim.run(); expect(sim.stack).toEqual([128]); expect(t[0].opcode).toBe("ldc.i4"); });
  it("value_minus_129", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(-129), [CLROpcode.RET])); const t = sim.run(); expect(sim.stack).toEqual([-129]); expect(t[0].opcode).toBe("ldc.i4"); });
});

// ===================================================================
// 4-5. Local variables
// ===================================================================

describe("TestLocalVariablesShortForms", () => {
  for (let slot = 0; slot < 4; slot++) {
    it(`stloc_${slot}`, () => {
      const sim = new CLRSimulator();
      sim.load(assembleClr(encodeLdcI4(42), encodeStloc(slot), [CLROpcode.RET]));
      sim.run();
      expect(sim.locals[slot]).toBe(42);
    });
    it(`ldloc_${slot}`, () => {
      const sim = new CLRSimulator();
      sim.load(assembleClr(encodeLdcI4(99), encodeStloc(slot), encodeLdloc(slot), [CLROpcode.RET]));
      sim.run();
      expect(sim.stack).toEqual([99]);
    });
  }
  it("stloc_ldloc_roundtrip", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(encodeLdcI4(7), encodeStloc(0), encodeLdloc(0), [CLROpcode.RET]));
    const traces = sim.run();
    expect(sim.stack).toEqual([7]);
    expect(traces[0].opcode).toBe("ldc.i4.7");
    expect(traces[1].opcode).toBe("stloc.0");
    expect(traces[2].opcode).toBe("ldloc.0");
  });
});

describe("TestLocalVariablesGenericForms", () => {
  it("stloc_s", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(55), encodeStloc(10), [CLROpcode.RET])); sim.run(); expect(sim.locals[10]).toBe(55); });
  it("ldloc_s", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(77), encodeStloc(10), encodeLdloc(10), [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([77]); });
  it("stloc_s_encoding", () => { expect(Array.from(encodeStloc(10))).toEqual([0x13, 0x0a]); });
  it("ldloc_s_encoding", () => { expect(Array.from(encodeLdloc(10))).toEqual([0x11, 0x0a]); });
  it("stloc_trace", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(33), encodeStloc(5), [CLROpcode.RET])); const t = sim.run(); expect(t[1].opcode).toBe("stloc.s"); expect(t[1].description).toContain("locals[5]"); });
  it("ldloc_trace", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(33), encodeStloc(5), encodeLdloc(5), [CLROpcode.RET])); const t = sim.run(); expect(t[2].opcode).toBe("ldloc.s"); expect(t[2].description).toContain("locals[5]"); });
});

// ===================================================================
// 6. Arithmetic
// ===================================================================

describe("TestArithmetic", () => {
  it("add", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(3), encodeLdcI4(4), [CLROpcode.ADD], [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([7]); });
  it("sub", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(10), encodeLdcI4(3), [CLROpcode.SUB], [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([7]); });
  it("mul", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(6), encodeLdcI4(7), [CLROpcode.MUL], [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([42]); });
  it("div", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(10), encodeLdcI4(3), [CLROpcode.DIV], [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([3]); });
  it("div_by_zero", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(10), encodeLdcI4(0), [CLROpcode.DIV], [CLROpcode.RET])); expect(() => sim.run()).toThrow(/DivideByZeroException/); });
  it("div_negative_truncation", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(-7), encodeLdcI4(2), [CLROpcode.DIV], [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([-3]); });
  it("add_trace", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(encodeLdcI4(3), encodeLdcI4(4), [CLROpcode.ADD], [CLROpcode.RET]));
    const traces = sim.run();
    const addTrace = traces[2];
    expect(addTrace.opcode).toBe("add");
    expect(addTrace.stackBefore).toEqual([3, 4]);
    expect(addTrace.stackAfter).toEqual([7]);
    expect(addTrace.description).toContain("pop 4 and 3");
    expect(addTrace.description).toContain("push 7");
  });
  it("sub_negative_result", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(3), encodeLdcI4(10), [CLROpcode.SUB], [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([-7]); });
});

// ===================================================================
// 7. Comparison: ceq, cgt, clt
// ===================================================================

describe("TestComparisons", () => {
  it("ceq_equal", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(5), encodeLdcI4(5), [CLROpcode.PREFIX_FE, CEQ_BYTE], [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([1]); });
  it("ceq_not_equal", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(5), encodeLdcI4(3), [CLROpcode.PREFIX_FE, CEQ_BYTE], [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([0]); });
  it("cgt_greater", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(5), encodeLdcI4(3), [CLROpcode.PREFIX_FE, CGT_BYTE], [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([1]); });
  it("cgt_not_greater", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(3), encodeLdcI4(5), [CLROpcode.PREFIX_FE, CGT_BYTE], [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([0]); });
  it("cgt_equal", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(5), encodeLdcI4(5), [CLROpcode.PREFIX_FE, CGT_BYTE], [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([0]); });
  it("clt_less", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(3), encodeLdcI4(5), [CLROpcode.PREFIX_FE, CLT_BYTE], [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([1]); });
  it("clt_not_less", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdcI4(5), encodeLdcI4(3), [CLROpcode.PREFIX_FE, CLT_BYTE], [CLROpcode.RET])); sim.run(); expect(sim.stack).toEqual([0]); });
  it("ceq_trace", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(encodeLdcI4(5), encodeLdcI4(5), [CLROpcode.PREFIX_FE, CEQ_BYTE], [CLROpcode.RET]));
    const traces = sim.run();
    expect(traces[2].opcode).toBe("ceq");
    expect(traces[2].description).toContain("5 == 5");
    expect(traces[2].stackAfter).toEqual([1]);
  });
});

// ===================================================================
// 8. Branching
// ===================================================================

describe("TestBranching", () => {
  it("br_s_forward", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(encodeLdcI4(1), new Uint8Array([CLROpcode.BR_S, 1]), encodeLdcI4(2), [CLROpcode.RET]));
    sim.run();
    expect(sim.stack).toEqual([1]);
  });
  it("br_s_zero_offset", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(encodeLdcI4(1), new Uint8Array([CLROpcode.BR_S, 0]), [CLROpcode.RET]));
    sim.run();
    expect(sim.stack).toEqual([1]);
  });
  it("brfalse_s_taken", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(encodeLdcI4(0), new Uint8Array([CLROpcode.BRFALSE_S, 1]), encodeLdcI4(1), [CLROpcode.RET]));
    sim.run();
    expect(sim.stack).toEqual([]);
  });
  it("brfalse_s_not_taken", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(encodeLdcI4(1), new Uint8Array([CLROpcode.BRFALSE_S, 1]), encodeLdcI4(2), [CLROpcode.RET]));
    sim.run();
    expect(sim.stack).toEqual([2]);
  });
  it("brtrue_s_taken", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(encodeLdcI4(1), new Uint8Array([CLROpcode.BRTRUE_S, 1]), encodeLdcI4(2), [CLROpcode.RET]));
    sim.run();
    expect(sim.stack).toEqual([]);
  });
  it("brtrue_s_not_taken", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(encodeLdcI4(0), new Uint8Array([CLROpcode.BRTRUE_S, 1]), encodeLdcI4(2), [CLROpcode.RET]));
    sim.run();
    expect(sim.stack).toEqual([2]);
  });
  it("br_s_backward_loop", () => {
    // Count from 0 to 3 with a loop
    const brfalseByte = (256 - 10) & 0xff; // -10 as unsigned byte
    const sim = new CLRSimulator();
    sim.load(assembleClr(
      encodeLdcI4(0), encodeStloc(0),
      // Loop start (PC=2)
      encodeLdloc(0), encodeLdcI4(1), [CLROpcode.ADD], encodeStloc(0),
      encodeLdloc(0), encodeLdcI4(3),
      [CLROpcode.PREFIX_FE, CEQ_BYTE],
      new Uint8Array([CLROpcode.BRFALSE_S, brfalseByte]),
      [CLROpcode.RET]
    ));
    sim.run();
    expect(sim.locals[0]).toBe(3);
  });
  it("brfalse_s_trace_taken", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(encodeLdcI4(0), new Uint8Array([CLROpcode.BRFALSE_S, 0]), [CLROpcode.RET]));
    const traces = sim.run();
    expect(traces[1].description).toContain("branch taken");
  });
  it("brtrue_s_trace_not_taken", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(encodeLdcI4(0), new Uint8Array([CLROpcode.BRTRUE_S, 0]), [CLROpcode.RET]));
    const traces = sim.run();
    expect(traces[1].description).toContain("branch not taken");
  });
});

// ===================================================================
// 9. Miscellaneous
// ===================================================================

describe("TestMiscellaneous", () => {
  it("nop", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr([CLROpcode.NOP], encodeLdcI4(1), [CLROpcode.RET]));
    const traces = sim.run();
    expect(traces[0].opcode).toBe("nop");
    expect(traces[0].stackBefore).toEqual([]);
    expect(traces[0].stackAfter).toEqual([]);
    expect(sim.stack).toEqual([1]);
  });
  it("ldnull", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr([CLROpcode.LDNULL], [CLROpcode.RET]));
    sim.run();
    expect(sim.stack).toEqual([null]);
  });
  it("ret_halts", () => { const sim = new CLRSimulator(); sim.load(assembleClr([CLROpcode.RET])); sim.run(); expect(sim.halted).toBe(true); });
  it("ret_trace", () => { const sim = new CLRSimulator(); sim.load(assembleClr([CLROpcode.RET])); const t = sim.run(); expect(t[0].opcode).toBe("ret"); expect(t[0].description).toBe("return"); });
});

// ===================================================================
// 10. Helper function tests
// ===================================================================

describe("TestHelperFunctions", () => {
  it("encode_ldc_i4_short_forms", () => { for (let n = 0; n <= 8; n++) { const e = encodeLdcI4(n); expect(e.length).toBe(1); expect(e[0]).toBe(CLROpcode.LDC_I4_0 + n); } });
  it("encode_ldc_i4_medium_form", () => { for (const n of [9, 10, 50, 100, 127, -1, -50, -128]) { const e = encodeLdcI4(n); expect(e.length).toBe(2); expect(e[0]).toBe(CLROpcode.LDC_I4_S); } });
  it("encode_ldc_i4_general_form", () => { for (const n of [128, 256, 1000, -129, -1000]) { const e = encodeLdcI4(n); expect(e.length).toBe(5); expect(e[0]).toBe(CLROpcode.LDC_I4); } });
  it("encode_stloc_short_forms", () => { for (let slot = 0; slot < 4; slot++) { const e = encodeStloc(slot); expect(e.length).toBe(1); expect(e[0]).toBe(CLROpcode.STLOC_0 + slot); } });
  it("encode_stloc_generic_form", () => { const e = encodeStloc(10); expect(e.length).toBe(2); expect(e[0]).toBe(CLROpcode.STLOC_S); expect(e[1]).toBe(10); });
  it("encode_ldloc_short_forms", () => { for (let slot = 0; slot < 4; slot++) { const e = encodeLdloc(slot); expect(e.length).toBe(1); expect(e[0]).toBe(CLROpcode.LDLOC_0 + slot); } });
  it("encode_ldloc_generic_form", () => { const e = encodeLdloc(10); expect(e.length).toBe(2); expect(e[0]).toBe(CLROpcode.LDLOC_S); expect(e[1]).toBe(10); });
  it("assemble_clr_tuples", () => { expect(Array.from(assembleClr([CLROpcode.LDC_I4_1], [CLROpcode.RET]))).toEqual([0x17, 0x2a]); });
  it("assemble_clr_bytes", () => { expect(Array.from(assembleClr(encodeLdcI4(42), new Uint8Array([CLROpcode.RET])))).toEqual([0x1f, 42, 0x2a]); });
  it("assemble_clr_mixed", () => { expect(Array.from(assembleClr(encodeLdcI4(1), [CLROpcode.RET]))).toEqual([0x17, 0x2a]); });
});

// ===================================================================
// 11. End-to-end programs
// ===================================================================

describe("TestEndToEnd", () => {
  it("x_equals_1_plus_2", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(encodeLdcI4(1), encodeLdcI4(2), [CLROpcode.ADD], encodeStloc(0), [CLROpcode.RET]));
    const traces = sim.run();
    expect(sim.locals[0]).toBe(3);
    expect(traces.length).toBe(5);
    expect(sim.halted).toBe(true);
  });
  it("x_equals_3_plus_4_times_2", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(encodeLdcI4(3), encodeLdcI4(4), [CLROpcode.ADD], encodeLdcI4(2), [CLROpcode.MUL], encodeStloc(0), [CLROpcode.RET]));
    sim.run();
    expect(sim.locals[0]).toBe(14);
  });
  it("swap_two_locals", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(
      encodeLdcI4(5), encodeStloc(0),
      encodeLdcI4(8), encodeStloc(1),
      encodeLdloc(0), encodeLdloc(1),
      encodeStloc(0), encodeStloc(1),
      [CLROpcode.RET]
    ));
    sim.run();
    expect(sim.locals[0]).toBe(8);
    expect(sim.locals[1]).toBe(5);
  });
  it("conditional_max", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(
      encodeLdcI4(5), encodeStloc(0),       // PC=0,1
      encodeLdcI4(3), encodeStloc(1),       // PC=2,3
      encodeLdloc(0), encodeLdloc(1),       // PC=4,5
      [CLROpcode.PREFIX_FE, CGT_BYTE],      // PC=6 (2 bytes)
      new Uint8Array([CLROpcode.BRFALSE_S, 3]), // PC=8 (2 bytes)
      encodeLdloc(0),                        // PC=10 (1 byte)
      new Uint8Array([CLROpcode.BR_S, 1]),   // PC=11 (2 bytes)
      encodeLdloc(1),                        // PC=13 (1 byte)
      encodeStloc(2),                        // PC=14 (1 byte)
      [CLROpcode.RET]                        // PC=15 (1 byte)
    ));
    sim.run();
    expect(sim.locals[2]).toBe(5);
  });
  it("trace_verification", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(encodeLdcI4(1), encodeLdcI4(2), [CLROpcode.ADD], encodeStloc(0), [CLROpcode.RET]));
    const traces = sim.run();
    expect(traces[0].pc).toBe(0); expect(traces[0].opcode).toBe("ldc.i4.1"); expect(traces[0].stackBefore).toEqual([]); expect(traces[0].stackAfter).toEqual([1]);
    expect(traces[1].pc).toBe(1); expect(traces[1].opcode).toBe("ldc.i4.2"); expect(traces[1].stackBefore).toEqual([1]); expect(traces[1].stackAfter).toEqual([1, 2]);
    expect(traces[2].pc).toBe(2); expect(traces[2].opcode).toBe("add"); expect(traces[2].stackBefore).toEqual([1, 2]); expect(traces[2].stackAfter).toEqual([3]);
    expect(traces[3].pc).toBe(3); expect(traces[3].opcode).toBe("stloc.0"); expect(traces[3].stackBefore).toEqual([3]); expect(traces[3].stackAfter).toEqual([]); expect(traces[3].localsSnapshot[0]).toBe(3);
    expect(traces[4].pc).toBe(4); expect(traces[4].opcode).toBe("ret");
  });
});

// ===================================================================
// 12. Error cases
// ===================================================================

describe("TestErrorCases", () => {
  it("step_after_halt", () => { const sim = new CLRSimulator(); sim.load(assembleClr([CLROpcode.RET])); sim.run(); expect(() => sim.step()).toThrow(/halted/); });
  it("unknown_opcode", () => { const sim = new CLRSimulator(); sim.load(new Uint8Array([0xff])); expect(() => sim.step()).toThrow(/Unknown CLR opcode/); });
  it("unknown_two_byte_opcode", () => { const sim = new CLRSimulator(); sim.load(new Uint8Array([0xfe, 0xff])); expect(() => sim.step()).toThrow(/Unknown two-byte opcode/); });
  it("pc_beyond_bytecode", () => { const sim = new CLRSimulator(); sim.load(new Uint8Array(0)); expect(() => sim.step()).toThrow(/beyond the end/); });
  it("ldloc_uninitialized", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdloc(0), [CLROpcode.RET])); expect(() => sim.step()).toThrow(/uninitialized/); });
  it("ldloc_s_uninitialized", () => { const sim = new CLRSimulator(); sim.load(assembleClr(encodeLdloc(10), [CLROpcode.RET])); expect(() => sim.step()).toThrow(/uninitialized/); });
  it("load_resets_state", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(encodeLdcI4(42), encodeStloc(0), [CLROpcode.RET]));
    sim.run();
    expect(sim.locals[0]).toBe(42);
    expect(sim.halted).toBe(true);
    sim.load(assembleClr([CLROpcode.RET]));
    expect(sim.locals[0]).toBeNull();
    expect(sim.halted).toBe(false);
    expect(sim.stack).toEqual([]);
    expect(sim.pc).toBe(0);
  });
  it("incomplete_two_byte_opcode", () => { const sim = new CLRSimulator(); sim.load(new Uint8Array([0xfe])); expect(() => sim.step()).toThrow(/Incomplete two-byte opcode/); });
  it("brfalse_with_null", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr([CLROpcode.LDNULL], new Uint8Array([CLROpcode.BRFALSE_S, 0]), [CLROpcode.RET]));
    const traces = sim.run();
    expect(traces[1].description).toContain("branch taken");
  });
  it("num_locals_parameter", () => { const sim = new CLRSimulator(); sim.load(assembleClr([CLROpcode.RET]), 4); expect(sim.locals.length).toBe(4); });
});

describe("TestSimulatorProtocol", () => {
  it("supports_structural_protocol_typing", () => {
    const sim: Simulator<CLRState> = new CLRSimulator();
    const result = sim.execute(
      assembleClr(encodeLdcI4(1), encodeLdcI4(2), [CLROpcode.ADD], encodeStloc(0), [CLROpcode.RET])
    );

    expect(result.ok).toBe(true);
    expect(result.finalState.locals[0]).toBe(3);
  });

  it("get_state_returns_immutable_snapshot", () => {
    const sim = new CLRSimulator();
    sim.load(assembleClr(encodeLdcI4(42), encodeStloc(0), [CLROpcode.RET]));
    sim.run();

    const state = sim.getState();
    expect(state.locals[0]).toBe(42);
    expect(Object.isFrozen(state)).toBe(true);
    expect(Object.isFrozen(state.stack)).toBe(true);
    expect(Object.isFrozen(state.locals)).toBe(true);
  });

  it("execute_reports_max_steps_failures", () => {
    const sim = new CLRSimulator();
    const result = sim.execute(assembleClr(new Uint8Array([CLROpcode.BR_S, 0xfe])), 3);

    expect(result.ok).toBe(false);
    expect(result.error).toMatch(/max_steps/);
    expect(result.steps).toBe(3);
  });
});
