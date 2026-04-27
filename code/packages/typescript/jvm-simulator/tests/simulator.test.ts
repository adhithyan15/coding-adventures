/**
 * Tests for the JVM bytecode simulator.
 */

import { describe, it, expect } from "vitest";
import type { Simulator } from "@coding-adventures/simulator-protocol";
import {
  JVMOpcode,
  JVMSimulator,
  assembleJvm,
  encodeIconst,
  encodeIload,
  encodeIstore,
} from "../src/simulator.js";
import type { JVMState } from "../src/state.js";
import type { JVMTrace } from "../src/simulator.js";

// ===========================================================================
// Helper encoding tests
// ===========================================================================

describe("TestEncodeIconst", () => {
  it("iconst_0", () => { expect(Array.from(encodeIconst(0))).toEqual([0x03]); });
  it("iconst_1", () => { expect(Array.from(encodeIconst(1))).toEqual([0x04]); });
  it("iconst_5", () => { expect(Array.from(encodeIconst(5))).toEqual([0x08]); });
  it("iconst_42_uses_bipush", () => { expect(Array.from(encodeIconst(42))).toEqual([0x10, 42]); });
  it("iconst_negative_uses_bipush", () => { expect(Array.from(encodeIconst(-1))).toEqual([0x10, 0xff]); });
  it("iconst_negative_128", () => { expect(Array.from(encodeIconst(-128))).toEqual([0x10, 0x80]); });
  it("iconst_out_of_range_raises", () => {
    expect(() => encodeIconst(128)).toThrow(/outside signed byte range/);
    expect(() => encodeIconst(-129)).toThrow(/outside signed byte range/);
  });
});

describe("TestEncodeIstore", () => {
  it("istore_0_uses_shortcut", () => { expect(Array.from(encodeIstore(0))).toEqual([0x3b]); });
  it("istore_3_uses_shortcut", () => { expect(Array.from(encodeIstore(3))).toEqual([0x3e]); });
  it("istore_5_uses_generic", () => { expect(Array.from(encodeIstore(5))).toEqual([0x36, 0x05]); });
});

describe("TestEncodeIload", () => {
  it("iload_0_uses_shortcut", () => { expect(Array.from(encodeIload(0))).toEqual([0x1a]); });
  it("iload_3_uses_shortcut", () => { expect(Array.from(encodeIload(3))).toEqual([0x1d]); });
  it("iload_5_uses_generic", () => { expect(Array.from(encodeIload(5))).toEqual([0x15, 0x05]); });
});

describe("TestAssembleJvm", () => {
  it("simple_program", () => {
    const bytecode = assembleJvm(
      [JVMOpcode.ICONST_1], [JVMOpcode.ICONST_2], [JVMOpcode.IADD],
      [JVMOpcode.ISTORE_0], [JVMOpcode.RETURN]
    );
    expect(Array.from(bytecode)).toEqual([0x04, 0x05, 0x60, 0x3b, 0xb1]);
  });
  it("bipush_assembly", () => {
    expect(Array.from(assembleJvm([JVMOpcode.BIPUSH, 42]))).toEqual([0x10, 42]);
  });
  it("bipush_negative", () => {
    expect(Array.from(assembleJvm([JVMOpcode.BIPUSH, -5]))).toEqual([0x10, 0xfb]);
  });
  it("goto_assembly", () => {
    expect(Array.from(assembleJvm([JVMOpcode.GOTO, 3]))).toEqual([0xa7, 0x00, 0x03]);
  });
  it("goto_negative_offset", () => {
    expect(Array.from(assembleJvm([JVMOpcode.GOTO, -5]))).toEqual([0xa7, 0xff, 0xfb]);
  });
  it("ldc_assembly", () => {
    expect(Array.from(assembleJvm([JVMOpcode.LDC, 3]))).toEqual([0x12, 0x03]);
  });
  it("iload_generic_assembly", () => {
    expect(Array.from(assembleJvm([JVMOpcode.ILOAD, 5]))).toEqual([0x15, 0x05]);
  });
  it("istore_generic_assembly", () => {
    expect(Array.from(assembleJvm([JVMOpcode.ISTORE, 7]))).toEqual([0x36, 0x07]);
  });
  it("missing_operand_raises", () => {
    expect(() => assembleJvm([JVMOpcode.BIPUSH])).toThrow(/requires an operand/);
  });
  it("missing_offset_raises", () => {
    expect(() => assembleJvm([JVMOpcode.GOTO])).toThrow(/requires an offset/);
  });
});

// ===========================================================================
// Instruction tests
// ===========================================================================

describe("TestIconst", () => {
  it("iconst_0", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_0], [JVMOpcode.RETURN])); const traces = sim.run(); expect(sim.stack).toEqual([0]); expect(traces[0].stackAfter).toEqual([0]); expect(traces[0].opcode).toBe("iconst_0"); });
  it("iconst_1", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_1], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([1]); });
  it("iconst_2", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_2], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([2]); });
  it("iconst_3", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_3], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([3]); });
  it("iconst_4", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_4], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([4]); });
  it("iconst_5", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_5], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([5]); });
  it("iconst_trace_description", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_3], [JVMOpcode.RETURN])); const traces = sim.run(); expect(traces[0].description).toBe("push 3"); });
});

describe("TestBipush", () => {
  it("bipush_positive", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.BIPUSH, 42], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([42]); });
  it("bipush_zero", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.BIPUSH, 0], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([0]); });
  it("bipush_negative", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.BIPUSH, -1], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([-1]); });
  it("bipush_min_value", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.BIPUSH, -128], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([-128]); });
  it("bipush_max_value", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.BIPUSH, 127], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([127]); });
});

describe("TestLdc", () => {
  it("ldc_integer", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.LDC, 0], [JVMOpcode.RETURN]), [999]); sim.run(); expect(sim.stack).toEqual([999]); });
  it("ldc_multiple_constants", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.LDC, 0], [JVMOpcode.LDC, 1], [JVMOpcode.RETURN]), [100, 200]); sim.run(); expect(sim.stack).toEqual([100, 200]); });
  it("ldc_out_of_range_raises", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.LDC, 5], [JVMOpcode.RETURN]), [42]); expect(() => sim.run()).toThrow(/Constant pool index/); });
  it("ldc_trace_description", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.LDC, 0], [JVMOpcode.RETURN]), [42]); const traces = sim.run(); expect(traces[0].description).toContain("constant[0] = 42"); });
});

describe("TestIloadIstore", () => {
  it("istore_0_iload_0", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_5], [JVMOpcode.ISTORE_0], [JVMOpcode.ILOAD_0], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([5]); expect(sim.locals[0]).toBe(5); });
  it("istore_1_iload_1", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_3], [JVMOpcode.ISTORE_1], [JVMOpcode.ILOAD_1], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([3]); expect(sim.locals[1]).toBe(3); });
  it("istore_2_iload_2", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_2], [JVMOpcode.ISTORE_2], [JVMOpcode.ILOAD_2], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([2]); expect(sim.locals[2]).toBe(2); });
  it("istore_3_iload_3", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_4], [JVMOpcode.ISTORE_3], [JVMOpcode.ILOAD_3], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([4]); expect(sim.locals[3]).toBe(4); });
  it("istore_generic_iload_generic", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.BIPUSH, 99], [JVMOpcode.ISTORE, 5], [JVMOpcode.ILOAD, 5], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([99]); expect(sim.locals[5]).toBe(99); });
  it("iload_uninitialized_raises", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ILOAD_0], [JVMOpcode.RETURN])); expect(() => sim.run()).toThrow(/not been initialized/); });
  it("istore_underflow_raises", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ISTORE_0], [JVMOpcode.RETURN])); expect(() => sim.run()).toThrow(/Stack underflow/); });
});

describe("TestArithmetic", () => {
  it("iadd", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_3], [JVMOpcode.ICONST_4], [JVMOpcode.IADD], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([7]); });
  it("isub", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_5], [JVMOpcode.ICONST_3], [JVMOpcode.ISUB], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([2]); });
  it("imul", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_3], [JVMOpcode.ICONST_4], [JVMOpcode.IMUL], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([12]); });
  it("idiv", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_5], [JVMOpcode.ICONST_2], [JVMOpcode.IDIV], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([2]); });
  it("idiv_by_zero_raises", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_5], [JVMOpcode.ICONST_0], [JVMOpcode.IDIV], [JVMOpcode.RETURN])); expect(() => sim.run()).toThrow(/division by zero/); });
  it("iadd_trace", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_1], [JVMOpcode.ICONST_2], [JVMOpcode.IADD], [JVMOpcode.RETURN])); const traces = sim.run(); expect(traces[2].opcode).toBe("iadd"); expect(traces[2].stackAfter).toEqual([3]); expect(traces[2].description).toContain("pop 2 and 1, push 3"); });
  it("arithmetic_underflow_raises", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.IADD], [JVMOpcode.RETURN])); expect(() => sim.run()).toThrow(/Stack underflow/); });
  it("isub_negative_result", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_2], [JVMOpcode.ICONST_5], [JVMOpcode.ISUB], [JVMOpcode.RETURN])); sim.run(); expect(sim.stack).toEqual([-3]); });
});

describe("TestControlFlow", () => {
  it("goto_forward", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm([JVMOpcode.ICONST_1], [JVMOpcode.GOTO, 5], [JVMOpcode.ICONST_2], [JVMOpcode.RETURN], [JVMOpcode.ICONST_3], [JVMOpcode.RETURN]));
    sim.run();
    expect(sim.stack).toEqual([1, 3]);
  });
  it("if_icmpeq_taken", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm([JVMOpcode.ICONST_3], [JVMOpcode.ICONST_3], [JVMOpcode.IF_ICMPEQ, 6], [JVMOpcode.ICONST_1], [JVMOpcode.RETURN], [JVMOpcode.RETURN], [JVMOpcode.ICONST_5], [JVMOpcode.RETURN]));
    sim.run();
    expect(sim.stack).toEqual([5]);
  });
  it("if_icmpeq_not_taken", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm([JVMOpcode.ICONST_3], [JVMOpcode.ICONST_4], [JVMOpcode.IF_ICMPEQ, 6], [JVMOpcode.ICONST_1], [JVMOpcode.RETURN]));
    sim.run();
    expect(sim.stack).toEqual([1]);
  });
  it("if_icmpgt_taken", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm([JVMOpcode.ICONST_5], [JVMOpcode.ICONST_3], [JVMOpcode.IF_ICMPGT, 6], [JVMOpcode.ICONST_0], [JVMOpcode.RETURN], [JVMOpcode.RETURN], [JVMOpcode.ICONST_1], [JVMOpcode.RETURN]));
    sim.run();
    expect(sim.stack).toEqual([1]);
  });
  it("if_icmpgt_not_taken", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm([JVMOpcode.ICONST_3], [JVMOpcode.ICONST_5], [JVMOpcode.IF_ICMPGT, 6], [JVMOpcode.ICONST_0], [JVMOpcode.RETURN]));
    sim.run();
    expect(sim.stack).toEqual([0]);
  });
  it("if_icmpgt_equal_not_taken", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm([JVMOpcode.ICONST_3], [JVMOpcode.ICONST_3], [JVMOpcode.IF_ICMPGT, 6], [JVMOpcode.ICONST_0], [JVMOpcode.RETURN]));
    sim.run();
    expect(sim.stack).toEqual([0]);
  });
});

describe("TestReturn", () => {
  it("return_halts", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.RETURN])); sim.run(); expect(sim.halted).toBe(true); expect(sim.returnValue).toBeNull(); });
  it("ireturn_halts_with_value", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_5], [JVMOpcode.IRETURN])); sim.run(); expect(sim.halted).toBe(true); expect(sim.returnValue).toBe(5); });
  it("ireturn_pops_stack", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_1], [JVMOpcode.ICONST_5], [JVMOpcode.IRETURN])); sim.run(); expect(sim.stack).toEqual([1]); expect(sim.returnValue).toBe(5); });
  it("return_trace_description", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.RETURN])); const traces = sim.run(); expect(traces[0].description).toBe("return void"); });
  it("ireturn_trace_description", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_3], [JVMOpcode.IRETURN])); const traces = sim.run(); expect(traces[1].description).toBe("return 3"); });
});

describe("TestEndToEnd", () => {
  it("x_equals_1_plus_2", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm([JVMOpcode.ICONST_1], [JVMOpcode.ICONST_2], [JVMOpcode.IADD], [JVMOpcode.ISTORE_0], [JVMOpcode.RETURN]));
    const traces = sim.run();
    expect(sim.locals[0]).toBe(3);
    expect(traces.length).toBe(5);
  });
  it("x_equals_3_plus_4_times_2", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm([JVMOpcode.ICONST_3], [JVMOpcode.ICONST_4], [JVMOpcode.IADD], [JVMOpcode.ICONST_2], [JVMOpcode.IMUL], [JVMOpcode.ISTORE_0], [JVMOpcode.RETURN]));
    sim.run();
    expect(sim.locals[0]).toBe(14);
  });
  it("swap_two_variables", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm(
      [JVMOpcode.ICONST_3], [JVMOpcode.ISTORE_0],
      [JVMOpcode.ICONST_5], [JVMOpcode.ISTORE_1],
      [JVMOpcode.ILOAD_0], [JVMOpcode.ISTORE_2],
      [JVMOpcode.ILOAD_1], [JVMOpcode.ISTORE_0],
      [JVMOpcode.ILOAD_2], [JVMOpcode.ISTORE_1],
      [JVMOpcode.RETURN]
    ));
    sim.run();
    expect(sim.locals[0]).toBe(5);
    expect(sim.locals[1]).toBe(3);
  });
  it("ireturn_value", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm([JVMOpcode.ICONST_1], [JVMOpcode.ICONST_2], [JVMOpcode.IADD], [JVMOpcode.IRETURN]));
    sim.run();
    expect(sim.returnValue).toBe(3);
  });
  it("trace_stack_states", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm([JVMOpcode.ICONST_1], [JVMOpcode.ICONST_2], [JVMOpcode.IADD], [JVMOpcode.RETURN]));
    const traces = sim.run();
    expect(traces[0].stackBefore).toEqual([]);
    expect(traces[0].stackAfter).toEqual([1]);
    expect(traces[1].stackBefore).toEqual([1]);
    expect(traces[1].stackAfter).toEqual([1, 2]);
    expect(traces[2].stackAfter).toEqual([3]);
  });
  it("trace_locals_snapshot", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm([JVMOpcode.ICONST_5], [JVMOpcode.ISTORE_0], [JVMOpcode.RETURN]));
    const traces = sim.run();
    expect(traces[1].localsSnapshot[0]).toBe(5);
    expect(traces[1].opcode).toBe("istore_0");
  });
});

describe("TestErrors", () => {
  it("invalid_opcode_raises", () => { const sim = new JVMSimulator(); sim.load(new Uint8Array([0xff])); expect(() => sim.step()).toThrow(/Unknown JVM opcode/); });
  it("step_after_halt_raises", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.RETURN])); sim.run(); expect(() => sim.step()).toThrow(/halted/); });
  it("pc_past_end_raises", () => { const sim = new JVMSimulator(); sim.load(new Uint8Array(0)); expect(() => sim.step()).toThrow(/past end of bytecode/); });
  it("ireturn_empty_stack_raises", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.IRETURN])); expect(() => sim.run()).toThrow(/Stack underflow/); });
  it("if_icmpeq_underflow_raises", () => { const sim = new JVMSimulator(); sim.load(assembleJvm([JVMOpcode.ICONST_1], [JVMOpcode.IF_ICMPEQ, 3])); expect(() => sim.run()).toThrow(/Stack underflow/); });
  it("load_resets_state", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm([JVMOpcode.ICONST_5], [JVMOpcode.ISTORE_0], [JVMOpcode.RETURN]));
    sim.run();
    expect(sim.locals[0]).toBe(5);
    expect(sim.halted).toBe(true);
    sim.load(assembleJvm([JVMOpcode.RETURN]));
    expect(sim.locals[0]).toBeNull();
    expect(sim.halted).toBe(false);
    expect(sim.stack).toEqual([]);
    expect(sim.pc).toBe(0);
  });
  it("max_steps_safety", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm([JVMOpcode.GOTO, 0]));
    const traces = sim.run(5);
    expect(traces.length).toBe(5);
    expect(sim.halted).toBe(false);
  });
});

describe("TestStep", () => {
  it("step_returns_trace", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm([JVMOpcode.ICONST_1], [JVMOpcode.RETURN]));
    const trace = sim.step();
    expect(trace.pc).toBe(0);
    expect(trace.opcode).toBe("iconst_1");
  });
  it("step_advances_pc", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm([JVMOpcode.ICONST_1], [JVMOpcode.ICONST_2], [JVMOpcode.RETURN]));
    sim.step();
    expect(sim.pc).toBe(1);
    sim.step();
    expect(sim.pc).toBe(2);
  });
  it("step_bipush_advances_pc_by_2", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm([JVMOpcode.BIPUSH, 42], [JVMOpcode.RETURN]));
    sim.step();
    expect(sim.pc).toBe(2);
  });
});

describe("TestSimulatorProtocol", () => {
  it("supports_structural_protocol_typing", () => {
    const sim: Simulator<JVMState, JVMTrace> = new JVMSimulator();
    const result = sim.execute(
      assembleJvm([JVMOpcode.ICONST_1], [JVMOpcode.ICONST_2], [JVMOpcode.IADD], [JVMOpcode.IRETURN])
    );

    expect(result.ok).toBe(true);
    expect(result.finalState.returnValue).toBe(3);
  });

  it("get_state_returns_immutable_snapshot", () => {
    const sim = new JVMSimulator();
    sim.load(assembleJvm([JVMOpcode.ICONST_5], [JVMOpcode.ISTORE_0], [JVMOpcode.RETURN]));
    sim.run();

    const state = sim.getState();
    expect(state.locals[0]).toBe(5);
    expect(Object.isFrozen(state)).toBe(true);
    expect(Object.isFrozen(state.locals)).toBe(true);
  });

  it("execute_reports_max_steps_failures", () => {
    const sim = new JVMSimulator();
    const result = sim.execute(assembleJvm([JVMOpcode.GOTO, 0]), 3);

    expect(result.ok).toBe(false);
    expect(result.error).toMatch(/max_steps/);
    expect(result.steps).toBe(3);
  });
});
