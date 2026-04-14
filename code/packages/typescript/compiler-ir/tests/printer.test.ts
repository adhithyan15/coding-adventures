/**
 * Tests for the IR printer — printIr()
 *
 * The printer converts an IrProgram to its canonical text format.
 * These tests verify:
 *   1. The .version directive is always first
 *   2. .data declarations appear before .entry
 *   3. Labels are unindented and followed by ":"
 *   4. Instructions are indented and include "; #ID" comments
 *   5. COMMENT instructions emit as "; text" lines
 *   6. Operand formatting (registers, immediates, labels)
 */

import { describe, it, expect } from "vitest";
import { printIr } from "../src/printer.js";
import { IrProgram, reg, imm, lbl } from "../src/types.js";
import { IrOp } from "../src/opcodes.js";

// Helper: build a minimal "empty" program for testing
function emptyProgram(): IrProgram {
  return new IrProgram("_start");
}

describe("printIr: directives", () => {
  it("always starts with .version", () => {
    const prog = emptyProgram();
    const text = printIr(prog);
    expect(text.startsWith(".version 1")).toBe(true);
  });

  it("includes .entry directive", () => {
    const prog = emptyProgram();
    const text = printIr(prog);
    expect(text).toContain(".entry _start");
  });

  it("includes .data before .entry when data is present", () => {
    const prog = emptyProgram();
    prog.addData({ label: "tape", size: 30000, init: 0 });
    const text = printIr(prog);
    const dataIdx = text.indexOf(".data tape");
    const entryIdx = text.indexOf(".entry");
    expect(dataIdx).toBeGreaterThan(-1);
    expect(entryIdx).toBeGreaterThan(-1);
    expect(dataIdx).toBeLessThan(entryIdx);
  });

  it("formats .data with label, size, and init", () => {
    const prog = emptyProgram();
    prog.addData({ label: "tape", size: 30000, init: 0 });
    const text = printIr(prog);
    expect(text).toContain(".data tape 30000 0");
  });

  it("handles multiple .data declarations", () => {
    const prog = emptyProgram();
    prog.addData({ label: "tape", size: 30000, init: 0 });
    prog.addData({ label: "buf", size: 1024, init: 255 });
    const text = printIr(prog);
    expect(text).toContain(".data tape 30000 0");
    expect(text).toContain(".data buf 1024 255");
  });

  it("supports version 2", () => {
    const prog = emptyProgram();
    prog.version = 2;
    const text = printIr(prog);
    expect(text).toContain(".version 2");
  });
});

describe("printIr: labels", () => {
  it("prints labels as 'name:' on their own unindented line", () => {
    const prog = emptyProgram();
    prog.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
    const text = printIr(prog);
    // Label line should match "_start:" with no leading spaces
    const lines = text.split("\n");
    const labelLine = lines.find((l) => l.trim().endsWith(":") && l.includes("_start"));
    expect(labelLine).toBeDefined();
    expect(labelLine!.trimStart()).toMatch(/^_start:$/);
  });

  it("prints loop labels", () => {
    const prog = emptyProgram();
    prog.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("loop_0_start")], id: -1 });
    prog.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("loop_0_end")], id: -1 });
    const text = printIr(prog);
    expect(text).toContain("loop_0_start:");
    expect(text).toContain("loop_0_end:");
  });
});

describe("printIr: instructions", () => {
  it("indents instructions with two spaces", () => {
    const prog = emptyProgram();
    prog.addInstruction({ opcode: IrOp.HALT, operands: [], id: 0 });
    const text = printIr(prog);
    const lines = text.split("\n");
    const haltLine = lines.find((l) => l.includes("HALT"));
    expect(haltLine).toBeDefined();
    expect(haltLine!.startsWith("  ")).toBe(true);
  });

  it("includes the ; #ID comment", () => {
    const prog = emptyProgram();
    prog.addInstruction({ opcode: IrOp.HALT, operands: [], id: 42 });
    const text = printIr(prog);
    expect(text).toContain("; #42");
  });

  it("formats instructions with register operands", () => {
    const prog = emptyProgram();
    prog.addInstruction({
      opcode: IrOp.LOAD_IMM,
      operands: [reg(0), imm(42)],
      id: 0,
    });
    const text = printIr(prog);
    expect(text).toContain("LOAD_IMM");
    expect(text).toContain("v0");
    expect(text).toContain("42");
  });

  it("formats instructions with label operands", () => {
    const prog = emptyProgram();
    prog.addInstruction({
      opcode: IrOp.JUMP,
      operands: [lbl("loop_0_start")],
      id: 5,
    });
    const text = printIr(prog);
    expect(text).toContain("JUMP");
    expect(text).toContain("loop_0_start");
    expect(text).toContain("; #5");
  });

  it("formats COMMENT instructions as '; text' lines", () => {
    const prog = emptyProgram();
    prog.addInstruction({
      opcode: IrOp.COMMENT,
      operands: [lbl("this is a comment")],
      id: -1,
    });
    const text = printIr(prog);
    const lines = text.split("\n");
    const commentLine = lines.find((l) => l.includes("this is a comment"));
    expect(commentLine).toBeDefined();
    expect(commentLine!.trim().startsWith(";")).toBe(true);
    // Should NOT contain the word COMMENT as an opcode
    expect(commentLine).not.toContain("COMMENT");
  });

  it("formats a complete ADD_IMM instruction", () => {
    const prog = emptyProgram();
    prog.addInstruction({
      opcode: IrOp.ADD_IMM,
      operands: [reg(1), reg(1), imm(1)],
      id: 3,
    });
    const text = printIr(prog);
    expect(text).toContain("ADD_IMM");
    expect(text).toContain("v1");
    expect(text).toContain("1");
    expect(text).toContain("; #3");
  });
});

describe("printIr: full program", () => {
  it("prints a complete Brainfuck empty-program IR", () => {
    /**
     * The canonical empty BF program IR:
     *
     *   .version 1
     *
     *   .data tape 30000 0
     *
     *   .entry _start
     *
     *   _start:
     *     LOAD_ADDR   v0, tape  ; #0
     *     LOAD_IMM    v1, 0     ; #1
     *     HALT                  ; #2
     */
    const prog = new IrProgram("_start");
    prog.addData({ label: "tape", size: 30000, init: 0 });
    prog.addInstruction({ opcode: IrOp.LABEL, operands: [lbl("_start")], id: -1 });
    prog.addInstruction({ opcode: IrOp.LOAD_ADDR, operands: [reg(0), lbl("tape")], id: 0 });
    prog.addInstruction({ opcode: IrOp.LOAD_IMM, operands: [reg(1), imm(0)], id: 1 });
    prog.addInstruction({ opcode: IrOp.HALT, operands: [], id: 2 });

    const text = printIr(prog);
    expect(text).toContain(".version 1");
    expect(text).toContain(".data tape 30000 0");
    expect(text).toContain(".entry _start");
    expect(text).toContain("_start:");
    expect(text).toContain("LOAD_ADDR");
    expect(text).toContain("LOAD_IMM");
    expect(text).toContain("HALT");
  });
});
