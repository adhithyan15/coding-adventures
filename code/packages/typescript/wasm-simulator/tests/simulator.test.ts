/**
 * Tests for the WASM stack-based virtual machine simulator.
 */

import { describe, it, expect } from "vitest";
import type { Simulator } from "@coding-adventures/simulator-protocol";
import {
  WasmDecoder,
  WasmSimulator,
  assembleWasm,
  encodeEnd,
  encodeI32Add,
  encodeI32Const,
  encodeI32Sub,
  encodeLocalGet,
  encodeLocalSet,
} from "../src/simulator.js";
import type { WasmState } from "../src/state.js";

describe("TestEncoding", () => {
  /** Verify instruction encoding produces correct byte sequences. */

  it("encode_i32_const_1", () => {
    /** i32.const 1 should be [0x41, 0x01, 0x00, 0x00, 0x00]. */
    expect(Array.from(encodeI32Const(1))).toEqual([0x41, 0x01, 0x00, 0x00, 0x00]);
  });

  it("encode_i32_const_negative", () => {
    /** i32.const -1 should encode as signed little-endian (0xFFFFFFFF). */
    expect(Array.from(encodeI32Const(-1))).toEqual([0x41, 0xff, 0xff, 0xff, 0xff]);
  });

  it("encode_i32_const_large", () => {
    /** i32.const 256 should encode correctly in little-endian. */
    expect(Array.from(encodeI32Const(256))).toEqual([0x41, 0x00, 0x01, 0x00, 0x00]);
  });

  it("encode_i32_add", () => {
    /** i32.add is a single byte: 0x6A. */
    expect(Array.from(encodeI32Add())).toEqual([0x6a]);
  });

  it("encode_i32_sub", () => {
    /** i32.sub is a single byte: 0x6B. */
    expect(Array.from(encodeI32Sub())).toEqual([0x6b]);
  });

  it("encode_local_get", () => {
    /** local.get 0 should be [0x20, 0x00]. */
    expect(Array.from(encodeLocalGet(0))).toEqual([0x20, 0x00]);
  });

  it("encode_local_set", () => {
    /** local.set 2 should be [0x21, 0x02]. */
    expect(Array.from(encodeLocalSet(2))).toEqual([0x21, 0x02]);
  });

  it("encode_end", () => {
    /** end should be a single byte: 0x0B. */
    expect(Array.from(encodeEnd())).toEqual([0x0b]);
  });
});

describe("TestDecoder", () => {
  /** Verify the decoder correctly reads variable-width instructions. */

  it("decode_i32_const", () => {
    const decoder = new WasmDecoder();
    const bytecode = encodeI32Const(42);
    const result = decoder.decode(bytecode, 0);
    expect(result.mnemonic).toBe("i32.const");
    expect(result.operand).toBe(42);
    expect(result.size).toBe(5);
  });

  it("decode_i32_const_negative", () => {
    const decoder = new WasmDecoder();
    const bytecode = encodeI32Const(-5);
    const result = decoder.decode(bytecode, 0);
    expect(result.mnemonic).toBe("i32.const");
    expect(result.operand).toBe(-5);
    expect(result.size).toBe(5);
  });

  it("decode_i32_add", () => {
    const decoder = new WasmDecoder();
    const bytecode = encodeI32Add();
    const result = decoder.decode(bytecode, 0);
    expect(result.mnemonic).toBe("i32.add");
    expect(result.operand).toBeNull();
    expect(result.size).toBe(1);
  });

  it("decode_i32_sub", () => {
    const decoder = new WasmDecoder();
    const bytecode = encodeI32Sub();
    const result = decoder.decode(bytecode, 0);
    expect(result.mnemonic).toBe("i32.sub");
    expect(result.operand).toBeNull();
    expect(result.size).toBe(1);
  });

  it("decode_local_get", () => {
    const decoder = new WasmDecoder();
    const bytecode = encodeLocalGet(3);
    const result = decoder.decode(bytecode, 0);
    expect(result.mnemonic).toBe("local.get");
    expect(result.operand).toBe(3);
    expect(result.size).toBe(2);
  });

  it("decode_local_set", () => {
    const decoder = new WasmDecoder();
    const bytecode = encodeLocalSet(1);
    const result = decoder.decode(bytecode, 0);
    expect(result.mnemonic).toBe("local.set");
    expect(result.operand).toBe(1);
    expect(result.size).toBe(2);
  });

  it("decode_end", () => {
    const decoder = new WasmDecoder();
    const bytecode = encodeEnd();
    const result = decoder.decode(bytecode, 0);
    expect(result.mnemonic).toBe("end");
    expect(result.operand).toBeNull();
    expect(result.size).toBe(1);
  });

  it("decode_at_offset", () => {
    /** Decoder should handle non-zero PC offsets correctly. */
    const decoder = new WasmDecoder();
    // Put i32.add (0x6A) at offset 5, preceded by 5 bytes of i32.const
    const bytecode = assembleWasm([encodeI32Const(99), encodeI32Add()]);
    const result = decoder.decode(bytecode, 5);
    expect(result.mnemonic).toBe("i32.add");
  });

  it("decode_unknown_opcode", () => {
    /** Unknown opcodes should throw an Error. */
    const decoder = new WasmDecoder();
    expect(() => decoder.decode(new Uint8Array([0xff]), 0)).toThrow(
      /Unknown WASM opcode/
    );
  });
});

describe("TestExecutor", () => {
  /** Verify executor operations on the stack and locals. */

  it("i32_const_pushes", () => {
    /** i32.const should push its operand onto the stack. */
    const sim = new WasmSimulator(4);
    const program = assembleWasm([encodeI32Const(7), encodeEnd()]);
    sim.load(program);
    const trace = sim.step();
    expect(trace.stackBefore).toEqual([]);
    expect(trace.stackAfter).toEqual([7]);
    expect(sim.stack).toEqual([7]);
  });

  it("i32_add_pops_two_pushes_sum", () => {
    /** i32.add should pop two values and push their sum. */
    const sim = new WasmSimulator(4);
    const program = assembleWasm([
      encodeI32Const(10),
      encodeI32Const(20),
      encodeI32Add(),
      encodeEnd(),
    ]);
    sim.load(program);
    sim.step(); // push 10
    sim.step(); // push 20
    const trace = sim.step(); // add
    expect(trace.stackBefore).toEqual([10, 20]);
    expect(trace.stackAfter).toEqual([30]);
    expect(sim.stack).toEqual([30]);
  });

  it("i32_sub_pops_two_pushes_difference", () => {
    /** i32.sub should compute second-to-top minus top. */
    const sim = new WasmSimulator(4);
    const program = assembleWasm([
      encodeI32Const(10),
      encodeI32Const(3),
      encodeI32Sub(),
      encodeEnd(),
    ]);
    sim.load(program);
    sim.step(); // push 10
    sim.step(); // push 3
    const trace = sim.step(); // sub: 10 - 3 = 7
    expect(trace.stackBefore).toEqual([10, 3]);
    expect(trace.stackAfter).toEqual([7]);
  });

  it("local_set_get_roundtrip", () => {
    /** local.set followed by local.get should round-trip a value. */
    const sim = new WasmSimulator(4);
    const program = assembleWasm([
      encodeI32Const(42), // push 42
      encodeLocalSet(1), // pop 42, store in locals[1]
      encodeLocalGet(1), // push locals[1] = 42
      encodeEnd(),
    ]);
    sim.load(program);
    sim.step(); // push 42
    sim.step(); // local.set 1
    expect(sim.locals[1]).toBe(42);
    expect(sim.stack).toEqual([]);
    const trace = sim.step(); // local.get 1
    expect(trace.stackAfter).toEqual([42]);
    expect(sim.stack).toEqual([42]);
  });
});

describe("TestWasmSimulator", () => {
  /** End-to-end tests running actual WASM programs. */

  it("x_equals_1_plus_2", () => {
    const sim = new WasmSimulator(4);
    const program = assembleWasm([
      encodeI32Const(1),
      encodeI32Const(2),
      encodeI32Add(),
      encodeLocalSet(0),
      encodeEnd(),
    ]);
    const traces = sim.run(program);

    expect(traces.length).toBe(5);
    expect(sim.locals[0]).toBe(3);
    expect(sim.halted).toBe(true);
    expect(sim.stack).toEqual([]); // Stack should be empty after local.set
  });

  it("stack_state_at_each_step", () => {
    const sim = new WasmSimulator(4);
    const program = assembleWasm([
      encodeI32Const(1),
      encodeI32Const(2),
      encodeI32Add(),
      encodeLocalSet(0),
      encodeEnd(),
    ]);
    const traces = sim.run(program);

    // Step 0: push 1
    expect(traces[0].stackBefore).toEqual([]);
    expect(traces[0].stackAfter).toEqual([1]);
    expect(traces[0].instruction.mnemonic).toBe("i32.const");

    // Step 1: push 2
    expect(traces[1].stackBefore).toEqual([1]);
    expect(traces[1].stackAfter).toEqual([1, 2]);
    expect(traces[1].instruction.mnemonic).toBe("i32.const");

    // Step 2: add -> pop 2 and 1, push 3
    expect(traces[2].stackBefore).toEqual([1, 2]);
    expect(traces[2].stackAfter).toEqual([3]);
    expect(traces[2].instruction.mnemonic).toBe("i32.add");

    // Step 3: local.set 0 -> pop 3
    expect(traces[3].stackBefore).toEqual([3]);
    expect(traces[3].stackAfter).toEqual([]);
    expect(traces[3].instruction.mnemonic).toBe("local.set");
    expect(traces[3].localsSnapshot[0]).toBe(3);

    // Step 4: end -> halt
    expect(traces[4].instruction.mnemonic).toBe("end");
    expect(traces[4].halted).toBe(true);
  });

  it("halt_raises_on_extra_step", () => {
    /** Stepping after halt should throw an Error. */
    const sim = new WasmSimulator(4);
    const program = assembleWasm([encodeEnd()]);
    sim.run(program);
    expect(() => sim.step()).toThrow(/halted/);
  });

  it("subtraction_program", () => {
    /** x = 10 - 3 -> locals[0] should be 7. */
    const sim = new WasmSimulator(4);
    const program = assembleWasm([
      encodeI32Const(10),
      encodeI32Const(3),
      encodeI32Sub(),
      encodeLocalSet(0),
      encodeEnd(),
    ]);
    sim.run(program);
    expect(sim.locals[0]).toBe(7);
  });

  it("multiple_locals", () => {
    /** Store different values in different locals, then retrieve them. */
    const sim = new WasmSimulator(4);
    const program = assembleWasm([
      encodeI32Const(10), // push 10
      encodeLocalSet(0), // locals[0] = 10
      encodeI32Const(20), // push 20
      encodeLocalSet(1), // locals[1] = 20
      encodeLocalGet(0), // push 10
      encodeLocalGet(1), // push 20
      encodeI32Add(), // push 30
      encodeLocalSet(2), // locals[2] = 30
      encodeEnd(),
    ]);
    sim.run(program);
    expect(sim.locals[0]).toBe(10);
    expect(sim.locals[1]).toBe(20);
    expect(sim.locals[2]).toBe(30);
  });

  it("pc_advances_correctly", () => {
    /**
     * PC should advance by each instruction's byte width.
     *
     * i32.const = 5 bytes, i32.add = 1 byte, local.set = 2 bytes, end = 1 byte
     * PCs: 0, 5, 10, 11, 13
     */
    const sim = new WasmSimulator(4);
    const program = assembleWasm([
      encodeI32Const(1), // 5 bytes, PC 0->5
      encodeI32Const(2), // 5 bytes, PC 5->10
      encodeI32Add(), // 1 byte,  PC 10->11
      encodeLocalSet(0), // 2 bytes, PC 11->13
      encodeEnd(), // 1 byte,  PC 13->14
    ]);
    const traces = sim.run(program);

    expect(traces[0].pc).toBe(0);
    expect(traces[1].pc).toBe(5);
    expect(traces[2].pc).toBe(10);
    expect(traces[3].pc).toBe(11);
    expect(traces[4].pc).toBe(13);
  });

  it("add_large_numbers", () => {
    /** 100 + 200 = 300. */
    const sim = new WasmSimulator(4);
    const program = assembleWasm([
      encodeI32Const(100),
      encodeI32Const(200),
      encodeI32Add(),
      encodeLocalSet(0),
      encodeEnd(),
    ]);
    sim.run(program);
    expect(sim.locals[0]).toBe(300);
  });
});

describe("TestSimulatorProtocol", () => {
  it("supports_structural_protocol_typing", () => {
    const sim: Simulator<WasmState> = new WasmSimulator(4);
    const result = sim.execute(
      assembleWasm([
        encodeI32Const(1),
        encodeI32Const(2),
        encodeI32Add(),
        encodeLocalSet(0),
        encodeEnd(),
      ])
    );

    expect(result.ok).toBe(true);
    expect(result.finalState.locals[0]).toBe(3);
  });

  it("get_state_returns_immutable_snapshot", () => {
    const sim = new WasmSimulator(4);
    sim.run(
      assembleWasm([encodeI32Const(7), encodeLocalSet(0), encodeEnd()])
    );

    const state = sim.getState();
    expect(state.locals[0]).toBe(7);
    expect(Object.isFrozen(state)).toBe(true);
    expect(Object.isFrozen(state.locals)).toBe(true);
    expect(Object.isFrozen(state.stack)).toBe(true);
  });

  it("execute_reports_max_steps_failures", () => {
    const sim = new WasmSimulator(4);
    const result = sim.execute(
      assembleWasm([encodeI32Const(1), encodeI32Const(2), encodeI32Add()]),
      2
    );

    expect(result.ok).toBe(false);
    expect(result.error).toMatch(/max_steps/);
    expect(result.steps).toBe(2);
  });
});
