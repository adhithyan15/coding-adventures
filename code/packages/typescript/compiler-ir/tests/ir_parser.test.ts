/**
 * Tests for the IR parser — parseIr()
 *
 * The parser reads the canonical text format produced by printIr() and
 * reconstructs an IrProgram. Tests cover:
 *   1. Basic directives (.version, .data, .entry)
 *   2. Labels
 *   3. All operand types (register, immediate, label)
 *   4. Roundtrip: parseIr(printIr(prog)) structurally equals prog
 *   5. Error cases (unknown opcodes, bad directives, too many operands)
 */

import { describe, it, expect } from "vitest";
import { parseIr } from "../src/ir_parser.js";
import { printIr } from "../src/printer.js";
import { IrProgram, reg, imm, lbl } from "../src/types.js";
import { IrOp } from "../src/opcodes.js";

// ──────────────────────────────────────────────────────────────────────────────
// Basic directive parsing
// ──────────────────────────────────────────────────────────────────────────────

describe("parseIr: directives", () => {
  it("parses .version directive", () => {
    const prog = parseIr(".version 1\n.entry _start\n");
    expect(prog.version).toBe(1);
  });

  it("parses .version 2", () => {
    const prog = parseIr(".version 2\n.entry _start\n");
    expect(prog.version).toBe(2);
  });

  it("parses .entry directive", () => {
    const prog = parseIr(".version 1\n.entry main\n");
    expect(prog.entryLabel).toBe("main");
  });

  it("parses .data declaration", () => {
    const prog = parseIr(".version 1\n.data tape 30000 0\n.entry _start\n");
    expect(prog.data).toHaveLength(1);
    expect(prog.data[0].label).toBe("tape");
    expect(prog.data[0].size).toBe(30000);
    expect(prog.data[0].init).toBe(0);
  });

  it("parses multiple .data declarations", () => {
    const text = ".version 1\n.data tape 30000 0\n.data buf 1024 255\n.entry _start\n";
    const prog = parseIr(text);
    expect(prog.data).toHaveLength(2);
    expect(prog.data[1].label).toBe("buf");
    expect(prog.data[1].size).toBe(1024);
    expect(prog.data[1].init).toBe(255);
  });

  it("skips blank lines", () => {
    const text = "\n\n.version 1\n\n\n.entry _start\n\n";
    const prog = parseIr(text);
    expect(prog.version).toBe(1);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Label parsing
// ──────────────────────────────────────────────────────────────────────────────

describe("parseIr: labels", () => {
  it("parses a label instruction", () => {
    const text = ".version 1\n.entry _start\n\n_start:\n";
    const prog = parseIr(text);
    expect(prog.instructions).toHaveLength(1);
    const instr = prog.instructions[0];
    expect(instr.opcode).toBe(IrOp.LABEL);
    expect(instr.operands[0]).toEqual(lbl("_start"));
  });

  it("assigns ID -1 to label instructions", () => {
    const text = ".version 1\n.entry _start\n\n_start:\n";
    const prog = parseIr(text);
    expect(prog.instructions[0].id).toBe(-1);
  });

  it("parses loop labels", () => {
    const text = ".version 1\n.entry _start\n\nloop_0_start:\nloop_0_end:\n";
    const prog = parseIr(text);
    expect(prog.instructions).toHaveLength(2);
    expect(prog.instructions[0].operands[0]).toEqual(lbl("loop_0_start"));
    expect(prog.instructions[1].operands[0]).toEqual(lbl("loop_0_end"));
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Operand parsing
// ──────────────────────────────────────────────────────────────────────────────

describe("parseIr: operand parsing", () => {
  it("parses register operands (v0, v1, ...)", () => {
    const text = ".version 1\n.entry _start\n  LOAD_IMM    v0, 42  ; #0\n";
    const prog = parseIr(text);
    expect(prog.instructions).toHaveLength(1);
    const op = prog.instructions[0].operands[0];
    expect(op).toEqual(reg(0));
  });

  it("parses immediate operands (integers)", () => {
    const text = ".version 1\n.entry _start\n  LOAD_IMM    v0, 42  ; #0\n";
    const prog = parseIr(text);
    const op = prog.instructions[0].operands[1];
    expect(op).toEqual(imm(42));
  });

  it("parses negative immediate operands", () => {
    const text = ".version 1\n.entry _start\n  ADD_IMM     v1, v1, -1  ; #0\n";
    const prog = parseIr(text);
    const op = prog.instructions[0].operands[2];
    expect(op).toEqual(imm(-1));
  });

  it("parses label operands", () => {
    const text = ".version 1\n.entry _start\n  JUMP        loop_0_start  ; #0\n";
    const prog = parseIr(text);
    const op = prog.instructions[0].operands[0];
    expect(op).toEqual(lbl("loop_0_start"));
  });

  it("parses the instruction ID from ; #N comment", () => {
    const text = ".version 1\n.entry _start\n  HALT  ; #7\n";
    const prog = parseIr(text);
    expect(prog.instructions[0].id).toBe(7);
  });

  it("sets ID to -1 when no ; #N comment", () => {
    const text = ".version 1\n.entry _start\n  HALT\n";
    const prog = parseIr(text);
    expect(prog.instructions[0].id).toBe(-1);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// All opcodes
// ──────────────────────────────────────────────────────────────────────────────

describe("parseIr: all opcodes", () => {
  const opcodeLines: Array<[string, IrOp]> = [
    ["LOAD_IMM    v0, 42  ; #0", IrOp.LOAD_IMM],
    ["LOAD_ADDR   v0, tape  ; #0", IrOp.LOAD_ADDR],
    ["LOAD_BYTE   v2, v0, v1  ; #0", IrOp.LOAD_BYTE],
    ["STORE_BYTE  v2, v0, v1  ; #0", IrOp.STORE_BYTE],
    ["LOAD_WORD   v2, v0, v1  ; #0", IrOp.LOAD_WORD],
    ["STORE_WORD  v2, v0, v1  ; #0", IrOp.STORE_WORD],
    ["ADD         v3, v1, v2  ; #0", IrOp.ADD],
    ["ADD_IMM     v1, v1, 1  ; #0", IrOp.ADD_IMM],
    ["SUB         v3, v1, v2  ; #0", IrOp.SUB],
    ["AND         v3, v1, v2  ; #0", IrOp.AND],
    ["AND_IMM     v2, v2, 255  ; #0", IrOp.AND_IMM],
    ["CMP_EQ      v4, v1, v2  ; #0", IrOp.CMP_EQ],
    ["CMP_NE      v4, v1, v2  ; #0", IrOp.CMP_NE],
    ["CMP_LT      v4, v1, v2  ; #0", IrOp.CMP_LT],
    ["CMP_GT      v4, v1, v2  ; #0", IrOp.CMP_GT],
    ["JUMP        loop_0_start  ; #0", IrOp.JUMP],
    ["BRANCH_Z    v2, loop_0_end  ; #0", IrOp.BRANCH_Z],
    ["BRANCH_NZ   v2, loop_0_end  ; #0", IrOp.BRANCH_NZ],
    ["CALL        my_func  ; #0", IrOp.CALL],
    ["RET  ; #0", IrOp.RET],
    ["SYSCALL     1  ; #0", IrOp.SYSCALL],
    ["HALT  ; #0", IrOp.HALT],
    ["NOP  ; #0", IrOp.NOP],
  ];

  for (const [line, expectedOp] of opcodeLines) {
    it(`parses ${line.split(" ")[0]}`, () => {
      const text = `.version 1\n.entry _start\n  ${line}\n`;
      const prog = parseIr(text);
      expect(prog.instructions[0].opcode).toBe(expectedOp);
    });
  }
});

// ──────────────────────────────────────────────────────────────────────────────
// Roundtrip tests
// ──────────────────────────────────────────────────────────────────────────────

describe("parseIr: roundtrip with printIr", () => {
  it("roundtrip preserves version", () => {
    const prog = new IrProgram("_start");
    prog.addInstruction({ opcode: IrOp.HALT, operands: [], id: 0 });
    const text = printIr(prog);
    const reparsed = parseIr(text);
    expect(reparsed.version).toBe(prog.version);
  });

  it("roundtrip preserves entry label", () => {
    const prog = new IrProgram("main");
    const text = printIr(prog);
    const reparsed = parseIr(text);
    expect(reparsed.entryLabel).toBe("main");
  });

  it("roundtrip preserves data declarations", () => {
    const prog = new IrProgram("_start");
    prog.addData({ label: "tape", size: 30000, init: 0 });
    const text = printIr(prog);
    const reparsed = parseIr(text);
    expect(reparsed.data).toHaveLength(1);
    expect(reparsed.data[0]).toEqual({ label: "tape", size: 30000, init: 0 });
  });

  it("roundtrip preserves instruction count", () => {
    const prog = new IrProgram("_start");
    prog.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
    prog.addInstruction({ opcode: IrOp.LOAD_ADDR, operands: [reg(0), lbl("tape")], id: 0 });
    prog.addInstruction({ opcode: IrOp.LOAD_IMM, operands: [reg(1), imm(0)], id: 1 });
    prog.addInstruction({ opcode: IrOp.HALT, operands: [], id: 2 });
    const text = printIr(prog);
    const reparsed = parseIr(text);
    expect(reparsed.instructions).toHaveLength(prog.instructions.length);
  });

  it("roundtrip preserves opcodes", () => {
    const prog = new IrProgram("_start");
    prog.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
    prog.addInstruction({ opcode: IrOp.LOAD_IMM, operands: [reg(1), imm(0)], id: 0 });
    prog.addInstruction({ opcode: IrOp.HALT, operands: [], id: 1 });
    const text = printIr(prog);
    const reparsed = parseIr(text);
    for (let i = 0; i < prog.instructions.length; i++) {
      expect(reparsed.instructions[i].opcode).toBe(prog.instructions[i].opcode);
    }
  });

  it("roundtrip preserves instruction IDs", () => {
    const prog = new IrProgram("_start");
    prog.addInstruction({ opcode: IrOp.LOAD_IMM, operands: [reg(0), imm(42)], id: 7 });
    prog.addInstruction({ opcode: IrOp.HALT, operands: [], id: 8 });
    const text = printIr(prog);
    const reparsed = parseIr(text);
    expect(reparsed.instructions[0].id).toBe(7);
    expect(reparsed.instructions[1].id).toBe(8);
  });

  it("roundtrip for a complex program with loops", () => {
    const prog = new IrProgram("_start");
    prog.addData({ label: "tape", size: 30000, init: 0 });
    prog.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
    prog.addInstruction({ opcode: IrOp.LOAD_ADDR, operands: [reg(0), lbl("tape")], id: 0 });
    prog.addInstruction({ opcode: IrOp.LOAD_IMM, operands: [reg(1), imm(0)], id: 1 });
    prog.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("loop_0_start")], id: -1 });
    prog.addInstruction({ opcode: IrOp.LOAD_BYTE, operands: [reg(2), reg(0), reg(1)], id: 2 });
    prog.addInstruction({ opcode: IrOp.BRANCH_Z, operands: [reg(2), lbl("loop_0_end")], id: 3 });
    prog.addInstruction({ opcode: IrOp.JUMP, operands: [lbl("loop_0_start")], id: 4 });
    prog.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("loop_0_end")], id: -1 });
    prog.addInstruction({ opcode: IrOp.HALT, operands: [], id: 5 });

    const text = printIr(prog);
    const reparsed = parseIr(text);
    expect(reparsed.instructions).toHaveLength(prog.instructions.length);
    expect(reparsed.data).toHaveLength(1);
  });
});

// ──────────────────────────────────────────────────────────────────────────────
// Error cases
// ──────────────────────────────────────────────────────────────────────────────

describe("parseIr: error cases", () => {
  it("throws on unknown opcode", () => {
    const text = ".version 1\n.entry _start\n  BOGUS_OP  v0  ; #0\n";
    expect(() => parseIr(text)).toThrow(/unknown opcode/i);
  });

  it("throws on invalid .version directive (missing number)", () => {
    const text = ".version\n.entry _start\n";
    expect(() => parseIr(text)).toThrow();
  });

  it("throws on invalid .data directive (wrong field count)", () => {
    const text = ".version 1\n.data tape 30000\n.entry _start\n";
    expect(() => parseIr(text)).toThrow();
  });

  it("throws on invalid .entry directive (no label)", () => {
    const text = ".version 1\n.entry\n";
    expect(() => parseIr(text)).toThrow();
  });

  it("throws on register index out of range", () => {
    const text = ".version 1\n.entry _start\n  HALT  ; #0\n";
    // Parse valid text first to ensure the parser works
    expect(() => parseIr(text)).not.toThrow();
    // Now test out-of-range register
    const badText = `.version 1\n.entry _start\n  LOAD_IMM    v99999, 0  ; #0\n`;
    expect(() => parseIr(badText)).toThrow(/out of range/i);
  });
});
