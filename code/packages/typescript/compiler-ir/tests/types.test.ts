/**
 * Tests for IR type system — IrRegister, IrImmediate, IrLabel,
 * IrProgram, IDGenerator, and operand utilities.
 */

import { describe, it, expect } from "vitest";
import {
  IrProgram,
  IDGenerator,
  reg,
  imm,
  lbl,
  operandToString,
} from "../src/types.js";
import { IrOp } from "../src/opcodes.js";

// ──────────────────────────────────────────────────────────────────────────────
// IrRegister
// ──────────────────────────────────────────────────────────────────────────────

describe("IrRegister", () => {
  it("reg(0) produces kind=register, index=0", () => {
    const r = reg(0);
    expect(r.kind).toBe("register");
    expect(r.index).toBe(0);
  });

  it("reg(5) produces index=5", () => {
    expect(reg(5).index).toBe(5);
  });

  it("operandToString formats register as vN", () => {
    /**
     * v0, v1, v5, v100 — the "v" prefix followed by the index.
     */
    expect(operandToString(reg(0))).toBe("v0");
    expect(operandToString(reg(1))).toBe("v1");
    expect(operandToString(reg(5))).toBe("v5");
    expect(operandToString(reg(100))).toBe("v100");
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// IrImmediate
// ──────────────────────────────────────────────────────────────────────────────

describe("IrImmediate", () => {
  it("imm(42) produces kind=immediate, value=42", () => {
    const i = imm(42);
    expect(i.kind).toBe("immediate");
    expect(i.value).toBe(42);
  });

  it("imm supports negative values", () => {
    expect(imm(-1).value).toBe(-1);
    expect(imm(-255).value).toBe(-255);
  });

  it("imm supports zero", () => {
    expect(imm(0).value).toBe(0);
  });

  it("operandToString formats immediate as decimal", () => {
    expect(operandToString(imm(42))).toBe("42");
    expect(operandToString(imm(-1))).toBe("-1");
    expect(operandToString(imm(0))).toBe("0");
    expect(operandToString(imm(255))).toBe("255");
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// IrLabel
// ──────────────────────────────────────────────────────────────────────────────

describe("IrLabel", () => {
  it("lbl('_start') produces kind=label, name='_start'", () => {
    const l = lbl("_start");
    expect(l.kind).toBe("label");
    expect(l.name).toBe("_start");
  });

  it("operandToString returns the label name verbatim", () => {
    expect(operandToString(lbl("_start"))).toBe("_start");
    expect(operandToString(lbl("loop_0_end"))).toBe("loop_0_end");
    expect(operandToString(lbl("__trap_oob"))).toBe("__trap_oob");
    expect(operandToString(lbl("tape"))).toBe("tape");
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// IrProgram
// ──────────────────────────────────────────────────────────────────────────────

describe("IrProgram", () => {
  it("new IrProgram('_start') has correct defaults", () => {
    /**
     * The entry label is set at construction time. Version defaults to 1
     * (the Brainfuck-sufficient subset). Both instruction and data arrays
     * start empty.
     */
    const prog = new IrProgram("_start");
    expect(prog.entryLabel).toBe("_start");
    expect(prog.version).toBe(1);
    expect(prog.instructions).toHaveLength(0);
    expect(prog.data).toHaveLength(0);
  });

  it("addInstruction appends instructions in order", () => {
    const prog = new IrProgram("_start");
    const i1 = { opcode: IrOp.HALT, operands: [], id: 0 };
    const i2 = { opcode: IrOp.NOP, operands: [], id: 1 };
    prog.addInstruction(i1);
    prog.addInstruction(i2);
    expect(prog.instructions).toHaveLength(2);
    expect(prog.instructions[0]).toBe(i1);
    expect(prog.instructions[1]).toBe(i2);
  });

  it("addData appends data declarations in order", () => {
    const prog = new IrProgram("_start");
    const d1 = { label: "tape", size: 30000, init: 0 };
    const d2 = { label: "output_buf", size: 1024, init: 0 };
    prog.addData(d1);
    prog.addData(d2);
    expect(prog.data).toHaveLength(2);
    expect(prog.data[0]).toBe(d1);
    expect(prog.data[1]).toBe(d2);
  });

  it("version can be overridden", () => {
    const prog = new IrProgram("main");
    prog.version = 2;
    expect(prog.version).toBe(2);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// IDGenerator
// ──────────────────────────────────────────────────────────────────────────────

describe("IDGenerator", () => {
  it("starts at 0 by default", () => {
    const gen = new IDGenerator();
    expect(gen.current()).toBe(0);
  });

  it("next() returns sequential IDs starting at 0", () => {
    const gen = new IDGenerator();
    expect(gen.next()).toBe(0);
    expect(gen.next()).toBe(1);
    expect(gen.next()).toBe(2);
  });

  it("current() returns the NEXT value (not the last returned)", () => {
    /**
     * current() is a "peek" into the future: it tells you what next()
     * will return on the next call. This is useful for recording the
     * start ID before emitting a batch of instructions.
     *
     * After next() returns 0, current() returns 1.
     */
    const gen = new IDGenerator();
    gen.next(); // 0
    expect(gen.current()).toBe(1);
    gen.next(); // 1
    expect(gen.current()).toBe(2);
  });

  it("current() before any next() is 0", () => {
    const gen = new IDGenerator();
    expect(gen.current()).toBe(0);
  });

  it("supports a custom start value", () => {
    /**
     * When multiple compilers contribute instructions to the same program,
     * each needs its own IDGenerator starting from a non-overlapping range.
     * new IDGenerator(100) starts counting from 100.
     */
    const gen = new IDGenerator(100);
    expect(gen.next()).toBe(100);
    expect(gen.next()).toBe(101);
    expect(gen.current()).toBe(102);
  });

  it("IDs are strictly monotonically increasing", () => {
    const gen = new IDGenerator();
    const ids: number[] = [];
    for (let i = 0; i < 1000; i++) {
      ids.push(gen.next());
    }
    for (let i = 1; i < ids.length; i++) {
      expect(ids[i]).toBeGreaterThan(ids[i - 1]);
    }
  });

  it("each call to next() returns a unique ID", () => {
    const gen = new IDGenerator();
    const ids = new Set<number>();
    for (let i = 0; i < 100; i++) {
      ids.add(gen.next());
    }
    expect(ids.size).toBe(100);
  });
});
